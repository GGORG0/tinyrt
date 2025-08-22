use std::sync::Arc;

use crate::Value;
use tokio::sync::{
    RwLock,
    broadcast::{Receiver, Sender, error::SendError},
};

#[derive(Clone)]
pub struct Topic {
    channel: Sender<Value>,
    latest_value: Arc<RwLock<Option<Value>>>,
}

impl Topic {
    pub fn new() -> Self {
        Self {
            channel: Sender::new(32),
            latest_value: Default::default(),
        }
    }

    pub fn subscribe(&self) -> Receiver<Value> {
        self.channel.subscribe()
    }

    pub async fn send(&self, value: Value) -> Result<usize, SendError<Value>> {
        {
            let mut latest_value = self.latest_value.write().await;
            *latest_value = Some(value.clone());
        }

        self.channel.send(value)
    }

    pub async fn latest(&self) -> Option<Value> {
        self.latest_value.read().await.clone()
    }
}

impl Default for Topic {
    fn default() -> Self {
        Self::new()
    }
}
