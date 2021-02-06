use std::convert::TryFrom;

use super::{EntrySender, Source};
use crate::{Entry, Score, SharedNvim};
use anyhow::Result;
use async_trait::async_trait;

use log::info;
use nvim_meta::value;
use nvim_rs::{
    call_args,
    rpc::{model::IntoVal, unpack::TryUnpack},
    Value,
};

#[derive(Debug, Clone)]
pub struct LuaFn {
    pub name: String,
}

#[async_trait]
impl Source for LuaFn {
    async fn get(&mut self, nvim: SharedNvim, sender: EntrySender) -> Result<()> {
        info!("Calling lua source with name: {}", self.name);
        let entries: Vec<Value> = nvim
            .call(
                "nvim_exec_lua",
                call_args!(
                    format!(r#"return require'rofl'.call_source('{}')"#, self.name),
                    value!([]),
                ),
            )
            .await?
            .unwrap()
            .try_unpack()
            .expect("Failed to unpack");

        for entry in entries {
            sender.send(Entry::try_from(entry)?)?;
        }
        Ok(())
    }
}

impl LuaFn {
    pub fn new(name: String) -> LuaFn {
        LuaFn { name }
    }
}
