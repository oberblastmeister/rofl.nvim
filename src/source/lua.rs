use super::Source;
use crate::{Entry, Score, SharedNvim};
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
    async fn get(&mut self, nvim: SharedNvim, _user_match: &str) -> Vec<Entry> {
        nvim.call_function(
            "luaeval",
            call_args!(
                r#"require("rofl").get_source("_A[1]")()"#,
                vec![self.name.clone()]
            ),
        )
        .await
        .unwrap();
        vec![]
    }
}

impl LuaFn {
    pub fn new(name: String) -> LuaFn {
        LuaFn { name }
    }
}
