use std::collections::HashMap;

use axum::{
    extract::{
        FromRequestParts, Path, Query, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    http::{header::UPGRADE, request::Parts},
    response::Response,
};
use axum_extra::{
    TypedHeader,
    headers::{Connection, Upgrade},
};
use bytes::Bytes;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};

use crate::db::ArcDb;

pub fn is_ws_request(
    connection: &Option<TypedHeader<Connection>>,
    upgrade: &Option<TypedHeader<Upgrade>>,
) -> bool {
    let Some(TypedHeader(connection)) = connection else {
        return false;
    };

    let Some(TypedHeader(upgrade)) = upgrade else {
        return false;
    };

    connection.contains(UPGRADE) && *upgrade == Upgrade::websocket()
}

pub async fn upgrade_handler(
    parts: &mut Parts,
    Path(topic): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(db): State<ArcDb>,
) -> axum::response::Result<Response> {
    let ws = WebSocketUpgrade::from_request_parts(parts, &db).await?;
    Ok(ws.on_upgrade(|socket| ws_handler(socket, topic, params, db)))
}

async fn ws_handler(socket: WebSocket, topic: String, params: HashMap<String, String>, db: ArcDb) {
    let (sender, receiver) = socket.split();

    let tx_binary = params
        .get("binary")
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    tokio::select! {
        _ = rx(receiver, topic.clone(), db.clone()) => {},
        _ = tx(sender, topic,tx_binary, db ) => {},
    }
}

async fn rx(mut receiver: SplitStream<WebSocket>, topic: String, db: ArcDb) {
    while let Some(Ok(msg)) = receiver.next().await {
        let data = match msg {
            Message::Binary(data) => data,
            Message::Text(data) => Bytes::from(data),
            _ => continue,
        };

        let _ = db.get(&topic).send(data).await;
    }
}

async fn tx(mut sender: SplitSink<WebSocket, Message>, topic: String, tx_binary: bool, db: ArcDb) {
    let mut receiver = db.get(&topic).subscribe();

    while let Ok(value) = receiver.recv().await {
        let msg = if tx_binary {
            Message::Binary(value)
        } else {
            Message::Text(Utf8Bytes::from(String::from_utf8_lossy(&value).to_string()))
        };

        if sender.send(msg).await.is_err() {
            break;
        }
    }
}
