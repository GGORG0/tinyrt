use std::sync::Arc;

use dashmap::DashMap;

use crate::topic::Topic;

pub type ArcDb = Arc<Db>;

pub struct Db {
    topics: DashMap<String, Topic>,
}

impl Db {
    pub fn new() -> Self {
        Self {
            topics: DashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Topic {
        self.topics.entry(name.to_string()).or_default().clone()
    }
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}
