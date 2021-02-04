mod entry;
mod score;
mod source;
mod utils;

use std::{
    collections::{HashMap, VecDeque},
    panic,
    sync::Arc,
    time::Duration,
};

use log::{debug, error, info, trace, LevelFilter};

use anyhow::Result;
use async_trait::async_trait;
use futures::future::AbortHandle;
use futures::{future::abortable, future::join_all};
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
    sync::{Mutex, RwLock},
    task,
    time::Instant,
};

use utils::*;

pub use entry::Entry;
pub use score::Score;
pub use source::{SharedSource, Source};

const PUM_HEIGHT: usize = 5;

type SharedNvim = Arc<Neovim<Compat<Stdout>>>;

#[derive(Debug, Clone)]
struct Completor {
    v_char: Option<char>,
    user_match: Arc<RwLock<String>>,
    sources: HashMap<String, SharedSource>,
    instant: Instant,
    complete_abort: Option<AbortHandle>,
    previous_complete: Option<Value>,
    byte_offset_history: VecDeque<i64>,
}

impl Completor {
    fn new() -> Completor {
        Completor {
            v_char: None,
            user_match: Arc::new(RwLock::new(String::new())),
            sources: HashMap::new(),
            instant: Instant::now(),
            complete_abort: None,
            previous_complete: Some(Value::Array(Vec::new())),
            byte_offset_history: VecDeque::with_capacity(2),
        }
    }

    fn set_v_char(&mut self, c: Value) {
        let s: String = match c {
            Value::String(utf_s) => utf_s.into_str().expect("Couldn't convert to rust String"),
            _ => panic!("The value must be a string"),
        };
        let mut chars = s.chars();
        let maybe_c = chars.next().expect("String is empty");

        if let Some(c) = chars.next() {
            panic!("String is not only one char");
        }

        debug!("Setting v_char to {}", maybe_c);

        self.v_char = Some(maybe_c);
    }

    async fn update_user_match(&mut self, nvim: SharedNvim) -> Result<()> {
        if let Some(' ') = self.v_char {
            self.user_match.write().await.clear();
        } else if let Some(c) = self.v_char {
            self.user_match.write().await.push(c);
        }

        debug!("The byte difference is {:?}", self.byte_difference(nvim).await?);
        // match self.byte_difference(nvim).await? {
        //     Some(byte_diff) if byte_diff < 0 => {
        //         let mut user_match = self.user_match.write().await;

        //         let up_to = (user_match.len())
        //             .checked_sub(byte_diff.checked_abs().unwrap_or(0) as usize)
        //             .unwrap_or_else(|| user_match.len());
        //         user_match.drain(..up_to);
        //     }
        //     _ => (),
        // }

        debug!("the user match is now: {}", self.user_match.read().await);
        Ok(())
    }

    fn quicker_than(&mut self, duration: Duration) -> bool {
        let earlier = self.instant;
        let now = Instant::now();
        self.instant = now;
        now.duration_since(earlier) < duration
    }

    async fn byte_difference(&mut self, nvim: SharedNvim) -> Result<Option<i64>> {
        let new_offset = get_byte_offset(nvim.clone()).await?;

        if self.byte_offset_history.is_empty() {
            self.byte_offset_history.push_back(new_offset);
            Ok(None)
        } else if self.byte_offset_history.len() == 1 {
            let previous_offset = self.byte_offset_history[0];
            self.byte_offset_history.push_back(new_offset);
            Ok(Some(previous_offset - new_offset))
        } else if self.byte_offset_history.len() == 2 {
            self.byte_offset_history.pop_front();
            let previous_offset = self.byte_offset_history[0];
            self.byte_offset_history.push_back(new_offset);
            Ok(Some(previous_offset - new_offset))
        } else {
            unreachable!()
        }
    }

    async fn complete(&mut self, nvim: SharedNvim) -> Result<()> {
        // let mode = nvim.get_mode().await?.swap_remove(0).1;
        // let mode = mode.as_str().unwrap();
        // debug!("mode: {:?}", mode);
        // if mode != "i" || mode != "ic" {
        //     return Ok(());
        // }
        self.update_user_match(nvim.clone()).await?;

        let mut futs = Vec::with_capacity(self.sources.len());

        for source in self.sources.values() {
            let nvim = nvim.clone();
            let source = source.clone();
            let user_match = self.user_match.clone();

            let handle = task::spawn(async move {
                let mut source = source.lock().await;
                source.get(nvim, &user_match.read().await).await
            });
            futs.push(handle);
        }

        let user_match = self.user_match.read().await;
        let mut entries: Vec<Entry> = join_all(futs)
            .await
            .into_iter()
            .map(|res| res.expect("Failed to join"))
            .flatten()
            .filter_map(|e| e.score(&user_match))
            .collect();

        entries.sort_unstable_by(|e1, e2| e1.score.cmp(&e2.score));

        let get = entries.len().saturating_sub(PUM_HEIGHT);

        drop(entries.drain(0..get));

        let entries = Entry::serialize(entries).await;

        nvim_complete(nvim.clone(), col(nvim, ".").await?, entries, Vec::new()).await?;

        Ok(())
    }

    fn register<S: Source>(&mut self, name: &str, source: S) {
        self.sources
            .insert(name.to_string(), Arc::new(Mutex::new(Box::new(source))));
    }
}

#[derive(Debug, Clone)]
struct NeovimHandler {
    completor: Arc<RwLock<Completor>>,
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

    async fn handle_notify(
        &self,
        name: String,
        mut args: Vec<Value>,
        neovim: Neovim<Self::Writer>,
    ) {
        trace!("Notification: {}, {:?}", name, args);

        let nvim = SharedNvim::new(neovim);
        let completor = self.completor();

        match name.as_ref() {
            "complete" => {
                if let Some(previous_complete) = self.completor.write().await.complete_abort.take()
                {
                    previous_complete.abort();
                }

                let fut = task::spawn(async move {
                    info!("completing");
                    let mut completor = completor.write().await;
                    completor.complete(nvim).await.expect("Failed to complete");
                });

                let (_fut, handle) = abortable(fut);
                self.completor.write().await.complete_abort.replace(handle);
            }
            "v_char" => {
                task::spawn(async move {
                    let mut completor_handle = completor.write().await;
                    completor_handle.set_v_char(args.remove(0));
                    drop(args);
                });
            }
            "insert_leave" => {
                task::spawn(async move {
                    info!("Clearing user match");
                    completor.write().await.user_match.write().await.clear();
                });
            }
            "update_buffer_words" => {
                task::spawn(async move {
                    let completor = completor.read().await;
                    let mut source = completor.sources.get("buffer_words").unwrap().lock().await;
                    source.update(nvim).await.unwrap();
                });
            }
            "add_lua_source" => {
                task::spawn(async move {
                    let mut completor = completor.write().await;
                    let name: String = args.remove(0).try_unpack().unwrap();
                    completor.register(&name.clone(), source::LuaFn::new(name));
                });
            }
            _ => (),
        }
    }
}

impl NeovimHandler {
    fn completor(&self) -> Arc<RwLock<Completor>> {
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

    let (nvim, io_handler) = create::new_parent(NeovimHandler {
        completor: Arc::new(RwLock::new(completor)),
    })
    .await;
    info!("Connected to parent...");

    // TODO: Any error should probably be logged, as stderr is not visible to users.
    match io_handler.await {
        Ok(res) => {
            trace!("OK Result: {:?}", res);
        }
        Err(err) => {
            nvim.err_writeln(&format!("Error: '{}'", err))
                .await
                .unwrap_or_else(|e| {
                    // We could inspect this error to see what was happening, and
                    // maybe retry, but at this point it's probably best
                    // to assume the worst and print a friendly and
                    // supportive message to our users
                    eprintln!("Well, dang... '{}'", e);
                });
        }
    }
}

fn main() {
    let mut runtime = runtime::Builder::new()
        .threaded_scheduler()
        .build()
        .expect("Failed to build runtime");
    runtime.block_on(run())
}
