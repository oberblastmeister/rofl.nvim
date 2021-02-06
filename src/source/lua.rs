use super::{EntrySender, Source};
use crate::{Entry, Score, SharedNvim};
use anyhow::Result;
use async_trait::async_trait;

use nvim_rs::{
    call_args,
    rpc::{model::IntoVal, unpack::TryUnpack},
};

#[derive(Debug, Clone)]
pub struct LuaFn {
    pub name: String,
}

#[async_trait]
impl Source for LuaFn {
    async fn get(&mut self, nvim: SharedNvim, sender: EntrySender) -> Result<()> {
        nvim.call_function(
            "luaeval",
            call_args!(
                r#"require("rofl").get_source("_A[1]")()"#,
                vec![self.name.clone()]
            ),
        )
        .await?;
        Ok(())
    }
}

impl LuaFn {
    pub fn new(name: String) -> LuaFn {
        LuaFn { name }
    }
}
