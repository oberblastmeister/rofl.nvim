use super::Source;
use crate::{Entry, Score, SharedNvim};
use super::EntrySender;
use async_trait::async_trait;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Counter(pub u64);

#[async_trait]
impl Source for Counter {
    async fn get(&mut self, nvim: SharedNvim, sender: EntrySender) -> Result<()> {
        let entry = Entry::new(format!("The counter is {}", self.0), Score::new(0));
        self.0 += 1;
        sender.send(entry);
        Ok(())
    }
}
