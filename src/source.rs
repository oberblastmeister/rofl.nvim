mod buffer;
mod counter;
mod r#static;
mod lua;

use std::{fmt, sync::Arc};

use async_trait::async_trait;
use dyn_clone::DynClone;
use futures::Stream;
use nvim_rs::{compat::tokio::Compat, Neovim};
use tokio::{io::Stdout, sync::{Mutex, RwLock, mpsc::{Sender, UnboundedSender}}};

use super::{Entry, Score, SharedNvim};

pub use buffer::BufferWords;
pub use counter::Counter;
pub use r#static::Static;
pub use lua::LuaFn;

type EntrySender = UnboundedSender<Entry>;

#[async_trait]
pub trait Source: 'static + Sync + Send + DynClone + fmt::Debug {
    async fn get(&mut self, nvim: SharedNvim, sender: EntrySender) -> anyhow::Result<()>;

    async fn update(&mut self, _nvim: SharedNvim) -> anyhow::Result<()> {
        Ok(())
    }
}

dyn_clone::clone_trait_object!(Source);

pub type SharedSource = Arc<Mutex<Box<dyn Source>>>;
