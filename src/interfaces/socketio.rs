use socketioxide::{SocketIoBuilder, layer::SocketIoLayer};

use crate::db::ArcDb;

#[cfg(not(any(feature = "socketio_pub", feature = "socketio_sub")))]
compile_error!(
    "The \"socketio\" feature must be enabled along with \"socketio_pub\" or \"socketio_sub\" (or both via \"socketio_full\")."
);

pub fn layer(db: ArcDb) -> SocketIoLayer {
    let (layer, io) = SocketIoBuilder::new().with_state(db).build_layer();

    io.ns("/", on_connect);

    layer
}

#[cfg(any(feature = "socketio_pub", feature = "socketio_sub"))]
async fn on_connect(socket: socketioxide::extract::SocketRef) {
    #[cfg(feature = "socketio_pub")]
    socket.on("message", r#pub::message);

    #[cfg(feature = "socketio_sub")]
    {
        socket
            .extensions
            .insert::<sub::AbortMap>(Default::default());

        socket.on("subscribe", sub::subscribe);
        socket.on("unsubscribe", sub::unsubscribe);

        socket.on_disconnect(sub::on_disconnect);
    }
}

#[cfg(not(any(feature = "socketio_pub", feature = "socketio_sub")))]
async fn on_connect() {}

#[cfg(feature = "socketio_pub")]
mod r#pub {
    use bytes::Bytes;
    use cfg_if::cfg_if;
    use rmpv::Value;
    use socketioxide::extract::{Data, State};

    use crate::db::ArcDb;

    pub async fn message(State(db): State<ArcDb>, Data((topic, value)): Data<(String, Value)>) {
        let value = match value {
            Value::Binary(bin) => Bytes::from(bin),
            Value::String(s) => Bytes::from(s.into_str().unwrap_or_default()),
            _ => Bytes::from({
                cfg_if! {
                    if #[cfg(feature = "socketio_json")] {
                        serde_json::to_string(&value).unwrap_or_else(|_| value.to_string())
                    } else {
                        value.to_string()
                    }
                }
            }),
        };

        let _ = db.get(&topic).send(value).await;
    }
}

#[cfg(feature = "socketio_sub")]
mod sub {
    use std::sync::Arc;

    use bytes::Bytes;

    use dashmap::DashMap;
    use serde::Deserialize;
    use socketioxide::extract::{Data, Extension, SocketRef, State};
    use tokio::sync::broadcast::Receiver;
    use tracing::instrument;

    use crate::db::ArcDb;

    pub type AbortMap = Arc<DashMap<String, tokio::task::AbortHandle>>;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "lowercase")]
    enum SubscriptionMode {
        String,
        Binary,

        #[cfg(feature = "socketio_json")]
        Json,
    }

    impl Default for SubscriptionMode {
        fn default() -> Self {
            Self::String
        }
    }

    #[derive(Deserialize)]
    pub struct SubscribeData(String, #[serde(default)] SubscriptionMode);

    pub async fn subscribe(
        socket: SocketRef,
        State(db): State<ArcDb>,
        Extension(abortmap): Extension<AbortMap>,
        Data(SubscribeData(topic, mode)): Data<SubscribeData>,
    ) {
        if let Some((_, handle)) = abortmap.remove(&topic) {
            handle.abort();
        }

        let receiver = db.get(&topic).subscribe();

        let handle =
            tokio::spawn(socketio_tx(socket, topic.clone(), receiver, mode)).abort_handle();

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

                #[cfg(feature = "socketio_json")]
                SubscriptionMode::Json => {
                    if let Ok(json) = serde_json::from_slice::<rmpv::Value>(&value) {
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

    pub async fn unsubscribe(Extension(abortmap): Extension<AbortMap>, Data(topic): Data<String>) {
        if let Some((_, handle)) = abortmap.remove(&topic) {
            handle.abort();
        }
    }

    pub async fn on_disconnect(Extension(abortmap): Extension<AbortMap>) {
        for handle_ref in abortmap.iter() {
            handle_ref.value().abort();
        }
    }
}
