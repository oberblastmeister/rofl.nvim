use std::convert::TryFrom;

use super::{EntrySender, Source};
use crate::{Entry, Score, SharedNvim};
use anyhow::Result;
use async_trait::async_trait;

use log::info;
use nvim_rs::{
    call_args,
    rpc::{model::IntoVal, unpack::TryUnpack},
    Value,
};

#[derive(Debug, Clone)]
pub struct LuaRequest {
    pub name: String,
}

#[async_trait]
impl Source for LuaFn {
    async fn get(&mut self, nvim: SharedNvim, sender: EntrySender) -> Result<()> {
        info!("Calling lua source with name: {}", self.name);
        let entries: Vec<Value> = nvim
            .call_function(
                "luaeval",
                call_args!(format!(r#"require'rofl'.call_source('{}')"#, self.name),),
            )
            .await?
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
