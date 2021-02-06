mod completor;
mod entry;
mod handle;
mod score;
mod source;
mod utils;

use std::{
    error::Error,
    panic,
    sync::{atomic::AtomicBool, Arc},
};

use log::{error, info, trace, LevelFilter};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;

use nvim_rs::{
    call_args,
    compat::tokio::Compat,
    create::tokio as create,
    rpc::{model::IntoVal, unpack::TryUnpack},
    Handler, Neovim, Value,
};

use simplelog::WriteLogger;
use tokio::{
    io::Stdout,
    runtime,
    sync::{oneshot, Mutex},
    task::{self, JoinHandle},
};

use completor::Completor;

pub use crate::entry::Entry;
pub use crate::score::Score;
pub use crate::source::{SharedSource, Source};

type Nvim = Neovim<Compat<Stdout>>;
type SharedNvim = Arc<Nvim>;

#[derive(Debug, Clone)]
pub struct NeovimHandler {
    completor: Arc<Mutex<Completor>>,
    abort_handle: Arc<Mutex<Option<JoinHandle<Result<()>>>>>,
    complete_started: Arc<AtomicBool>,
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Compat<Stdout>;

    async fn handle_request(
        &self,
        name: String,
        args: Vec<Value>,
        neovim: Neovim<Compat<Stdout>>,
    ) -> Result<Value, Value> {
        info!("Request: {}, {:?}", name, args);

        Ok(Value::from(true))
    }

    async fn handle_notify(&self, name: String, args: Vec<Value>, neovim: Neovim<Self::Writer>) {
        handle::notify(&self, name, args, neovim).await.unwrap();
    }
}

impl NeovimHandler {
    fn completor(&self) -> Arc<Mutex<Completor>> {
        self.completor.clone()
    }
}

async fn run() {
    let cache_path = dirs_next::cache_dir()
        .expect("Failed to get cache dir")
        .join("nvim");

    // should be okay to be synchronous
    std::fs::create_dir_all(&cache_path).expect("Failed to create cache dir");

    WriteLogger::init(
        LevelFilter::Debug,
        simplelog::Config::default(),
        std::fs::File::create(cache_path.join("rofl.log")).expect("Failed to create log file"),
    )
    .expect("Failed to start logger");

    // we do not want to crash when panicking, instead log it
    panic::set_hook(Box::new(move |panic| {
        error!("----- Panic -----");
        error!("{}", panic);
    }));

    let mut completor = Completor::new();
    completor.register("counter", source::Counter(0));
    completor.register(
        "static",
        source::Static::new(&[
            "This is just a test".to_owned(),
            "This is another test from static source".to_owned(),
        ]),
    );
    completor.register("buffer_words", source::BufferWords::new());

    let neovim_handler = NeovimHandler {
        completor: Arc::new(Mutex::new(completor.clone())),
        abort_handle: Arc::new(Mutex::new(None)),
        complete_started: Arc::new(AtomicBool::new(false)),
    };

    loop {
        let (nvim, io_handler) = create::new_parent(neovim_handler.clone()).await;

        info!("Connected to parent...");
        // TODO: Any error should probably be logged, as stderr is not visible to users.
        match io_handler.await {
            Err(joinerr) => error!("Error joining IO loop: {}", joinerr),
            Ok(Err(err)) => {
                let err = *err;
                error!("{}", anyhow!("{:?}", err));
                error!("There was an error: {}", err);
                error!("Debug error: {:?}", err);
                error!("Channel closed: {}", err.is_channel_closed());
                error!("Is this a reader error: {}", err.is_reader_error());
            }
            Ok(Ok(())) => {
                info!("got okay")
            }
        }
    }
}

fn main() {
    let runtime = runtime::Builder::new_multi_thread()
        .enable_time()
        .build()
        .expect("Failed to build runtime");
    runtime.block_on(run())
}
