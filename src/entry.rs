use std::{
    cmp,
    convert::{TryFrom, TryInto},
};

use super::Score;
use anyhow::{anyhow, Context, Result};
use futures::stream::{self, StreamExt};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use nvim_meta::value;
use nvim_rs::Value;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entry {
    pub contents: String,
    pub score: Score,
}

impl Entry {
    pub fn new(contents: String, score: Score) -> Entry {
        Entry { contents, score }
    }

    pub async fn serialize(entries: Vec<Entry>) -> Value {
        Value::Array(stream::iter(entries).map(|e| e.into()).collect().await)
    }

    pub fn score(self, text: &str) -> Option<Entry> {
        let matcher = SkimMatcherV2::default();
        matcher
            .fuzzy_match(&self.contents, text)
            .map(|score| Entry::new(self.contents, Score::new(score)))
    }

    pub fn score_multiple(entries: Vec<Entry>, text: &str) -> Vec<Entry> {
        let mut scored: Vec<_> = entries.into_iter().filter_map(|e| e.score(text)).collect();
        scored.sort_unstable_by(|e1, e2| e1.score.cmp(&e2.score));
        scored
    }
}

impl From<Entry> for Value {
    fn from(entry: Entry) -> Value {
        value!([
            "word" => "",
            "empty" => 1, // we just set word to be empty
            "dup" => 1, // this match will be added even even if duplicate
            "equal" => 1, // do not filter, reduces flickering because we are filtering
            "abbr" => entry.contents,
            "menu" => "[B]",
        ])
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TryFrom<Value> for Entry {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> Result<Entry> {
        let s = value.try_into().map_err(|_| anyhow!("Failed to tryfrom"))?;
        Ok(Entry::new(s, Score::new(0)))
    }
}
