use super::SharedNvim;
use anyhow::Result;
use nvim_rs::{call_args, rpc::model::IntoVal, Value};
use nvim_meta::{function, api};

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
