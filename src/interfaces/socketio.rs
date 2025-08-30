use std::sync::Arc;

use bytes::Bytes;
use dashmap::DashMap;
use rmpv::Value;
use serde::Deserialize;
use socketioxide::{
    SocketIoBuilder,
    extract::{Data, Extension, SocketRef, State},
    layer::SocketIoLayer,
};
use tokio::sync::broadcast::Receiver;
use tracing::instrument;

use crate::db::ArcDb;

type AbortMap = Arc<DashMap<String, tokio::task::AbortHandle>>;

pub fn layer(db: ArcDb) -> SocketIoLayer {
    let (layer, io) = SocketIoBuilder::new().with_state(db).build_layer();

    io.ns("/", on_connect);

    layer
}

async fn on_connect(socket: SocketRef) {
    socket.extensions.insert::<AbortMap>(Default::default());

    socket.on("subscribe", subscribe);
    socket.on("unsubscribe", unsubscribe);
    socket.on("message", message);

    socket.on_disconnect(on_disconnect);
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum SubscriptionMode {
    String,
    Binary,
    Json,
}

impl Default for SubscriptionMode {
    fn default() -> Self {
        Self::String
    }
}

#[derive(Deserialize)]
struct SubscribeData(String, #[serde(default)] SubscriptionMode);

async fn subscribe(
    socket: SocketRef,
    State(db): State<ArcDb>,
    Extension(abortmap): Extension<AbortMap>,
    Data(SubscribeData(topic, mode)): Data<SubscribeData>,
) {
    if let Some((_, handle)) = abortmap.remove(&topic) {
        handle.abort();
    }

    let receiver = db.get(&topic).subscribe();

    let handle = tokio::spawn(socketio_tx(socket, topic.clone(), receiver, mode)).abort_handle();

    abortmap.insert(topic, handle);
}

#[instrument(level = "debug", skip(socket, receiver))]
async fn socketio_tx(
    socket: SocketRef,
    topic: String,
    mut receiver: Receiver<Bytes>,
    mode: SubscriptionMode,
) {
    while let Ok(value) = receiver.recv().await {
        let res = match mode {
            SubscriptionMode::Binary => socket.emit("message", &(&topic, &value)),
            SubscriptionMode::String => {
                socket.emit("message", &(&topic, String::from_utf8_lossy(&value)))
            }
            SubscriptionMode::Json => {
                if let Ok(json) = serde_json::from_slice::<Value>(&value) {
                    socket.emit("message", &(&topic, &json))
                } else {
                    socket.emit("message", &(&topic, String::from_utf8_lossy(&value)))
                }
            }
        };

        if res.is_err() {
            break;
        }
    }
}

async fn unsubscribe(Extension(abortmap): Extension<AbortMap>, Data(topic): Data<String>) {
    if let Some((_, handle)) = abortmap.remove(&topic) {
        handle.abort();
    }
}

async fn message(State(db): State<ArcDb>, Data((topic, value)): Data<(String, Value)>) {
    let value = match value {
        Value::Binary(bin) => Bytes::from(bin),
        Value::String(s) => Bytes::from(s.into_str().unwrap_or_default()),
        _ => Bytes::from(serde_json::to_string(&value).unwrap_or_else(|_| value.to_string())),
    };

    let _ = db.get(&topic).send(value).await;
}

async fn on_disconnect(Extension(abortmap): Extension<AbortMap>) {
    for handle_ref in abortmap.iter() {
        handle_ref.value().abort();
    }
}
