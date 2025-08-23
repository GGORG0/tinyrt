use std::sync::Arc;

use tokio::sync::{
    RwLock,
    broadcast::{Receiver, Sender, error::SendError},
};

#[derive(Clone)]
pub struct Topic<T>
where
    T: Clone + Send + 'static,
{
    channel: Sender<T>,
    latest_value: Arc<RwLock<Option<T>>>,
}

impl<T> Topic<T>
where
    T: Clone + Send + 'static,
{
    pub fn new() -> Self {
        Self {
            channel: Sender::new(32),
            latest_value: Default::default(),
        }
    }

    pub fn subscribe(&self) -> Receiver<T> {
        self.channel.subscribe()
    }

    pub async fn send(&self, value: T) -> Result<usize, SendError<T>> {
        {
            let mut latest_value = self.latest_value.write().await;
            *latest_value = Some(value.clone());
        }

        self.channel.send(value)
    }

    pub async fn latest(&self) -> Option<T> {
        self.latest_value.read().await.clone()
    }
}

impl<T> Default for Topic<T>
where
    T: Clone + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
