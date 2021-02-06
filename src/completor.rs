use crate::SharedNvim;

use anyhow::Result;
use futures::future::join_all;
use futures::stream::StreamExt;
use log::{debug, info};
use nvim_rs::Value;
use std::{
    collections::{BTreeSet, HashMap},
    panic,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{mpsc::unbounded_channel, Mutex, RwLock},
    task,
    time::{timeout, Instant},
};

use super::{Entry, SharedSource, Source};

const PUM_HEIGHT: usize = 6;

#[derive(Debug, Clone)]
pub struct Completor {
    pub v_char: Option<char>,
    pub user_match: Arc<RwLock<String>>,
    pub sources: HashMap<String, SharedSource>,
    pub instant: Instant,
    pub previous_complete: Option<Value>,
}

impl Completor {
    pub fn new() -> Completor {
        Completor {
            v_char: None,
            user_match: Arc::new(RwLock::new(String::new())),
            sources: HashMap::new(),
            instant: Instant::now(),
            previous_complete: Some(Value::Array(Vec::new())),
        }
    }

    pub fn set_v_char(&mut self, c: Value) {
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

    pub async fn update_user_match(&mut self, nvim: SharedNvim) {
        if let Some(' ') = self.v_char {
            self.user_match.write().await.clear();
        } else if let Some(c) = self.v_char {
            self.user_match.write().await.push(c);
        }
    }

    pub fn quicker_than(&mut self, duration: Duration) -> bool {
        let earlier = self.instant;
        let now = Instant::now();
        self.instant = now;
        now.duration_since(earlier) < duration
    }

    // async fn byte_difference(&mut self, nvim: SharedNvim) -> Result<Option<i64>> {
    //     let new_offset = get_byte_offset(nvim.clone()).await?;

    //     if self.byte_offset_history.is_empty() {
    //         self.byte_offset_history.push_back(new_offset);
    //         Ok(None)
    //     } else if self.byte_offset_history.len() == 1 {
    //         let previous_offset = self.byte_offset_history[0];
    //         self.byte_offset_history.push_back(new_offset);
    //         Ok(Some(previous_offset - new_offset))
    //     } else if self.byte_offset_history.len() == 2 {
    //         self.byte_offset_history.pop_front();
    //         let previous_offset = self.byte_offset_history[0];
    //         self.byte_offset_history.push_back(new_offset);
    //         Ok(Some(previous_offset - new_offset))
    //     } else {
    //         unreachable!()
    //     }
    // }
    pub async fn agregate_sources(&mut self, nvim: SharedNvim) -> Result<Value> {
        // if self.quicker_than(Duration::from_millis(100)) {
        //     return Ok(());
        // }
        // let mode = nvim.get_mode().await?.swap_remove(0).1;
        // let mode = mode.as_str().unwrap();
        // debug!("mode: {:?}", mode);
        // if mode != "i" || mode != "ic" {
        //     return Ok(());
        // }
        let mut futs = Vec::with_capacity(self.sources.len());

        let (tx, mut rx) = unbounded_channel();
        for source in self.sources.values() {
            let nvim = nvim.clone();
            let source = source.clone();
            let tx_clone = tx.clone();

            let fut = async move {
                let mut source = source.lock().await;
                source.get(nvim, tx_clone).await?;
                Ok::<_, anyhow::Error>(())
            };

            let timeout_source = timeout(Duration::from_millis(200), fut);
            let handle = task::spawn(timeout_source);
            futs.push(handle);
        }
        drop(tx); // very important

        let user_match = self.user_match.read().await;
        let mut entries = BTreeSet::new();
        while let Some(entry) = rx.recv().await {
            let entry = entry.score(&user_match);
            if let Some(entry) = entry {
                entries.insert(entry);
            }
        }

        join_all(futs)
            .await
            .into_iter()
            .map(|res| res.expect("Failed to join"))
            .filter_map(|timeout_res| match timeout_res {
                Err(e) => {
                    info!("A source timed out: {:?}", e);
                    None
                }
                Ok(res) => Some(res),
            })
            .map(|source_res| source_res.expect("Source errored"))
            .for_each(|_| ());

        let entries: Vec<_> = entries
            .into_iter()
            .take(PUM_HEIGHT)
            .map(|e| e.into())
            .collect();

        Ok(Value::Array(entries))
    }

    pub fn register<S: Source>(&mut self, name: &str, source: S) {
        self.sources
            .insert(name.to_string(), Arc::new(Mutex::new(Box::new(source))));
    }
}
