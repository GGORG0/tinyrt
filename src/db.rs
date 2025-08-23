use std::sync::Arc;

use dashmap::DashMap;

use crate::topic::Topic;

pub type ArcDb<T> = Arc<Db<T>>;

pub struct Db<T>
where
    T: Clone + Send + 'static,
{
    topics: DashMap<String, Topic<T>>,
}

impl<T> Db<T>
where
    T: Clone + Send + 'static,
{
    pub fn new() -> Self {
        Self {
            topics: DashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Topic<T> {
        self.topics.entry(name.to_string()).or_default().clone()
    }
}

impl<T> Default for Db<T>
where
    T: Clone + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
