use std::fmt;

use super::SharedNvim;
use anyhow::Result;
use futures::Future;
use nvim_meta::{api, function};
use nvim_rs::{call_args, rpc::model::IntoVal, Value};
use tokio::task;

// function! {
//     pub async fn line(nvim: SharedNvim, mark: &str) -> Result<i64>;

//     pub async fn col(nvim: SharedNvim, mark: &str) -> Result<i64>;

//     pub async fn line2byte(nvim: SharedNvim, line: i64) -> Result<i64>;
// }

// api! {
//     pub async fn nvim_complete(nvim: SharedNvim, col: i64, entries: Vec<Value>, opts: Vec<(Value, Value)>) -> Result<()>;
// }

// pub async fn get_byte_offset(nvim: SharedNvim) -> Result<i64> {
//     let line = line(nvim.clone(), ".").await?;
//     let col = col(nvim.clone(), ".").await?;
//     let res = line2byte(nvim.clone(), line).await? + col - 2;
//     Ok(res)
// }
//

pub async fn panic_task<T, U, E>(task: T)
where
    T: Future<Output = std::result::Result<U, E>> + Send + 'static,
    T::Output: Send + 'static,
    U: Send + 'static,
    E: fmt::Debug + Send + 'static,
{
    task::spawn(async move { task.await.expect("Task panicked!") });
}
