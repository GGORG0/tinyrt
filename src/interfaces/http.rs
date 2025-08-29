use std::convert::Infallible;

use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    response::{
        IntoResponse, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use bytes::Bytes;
use tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream};

use crate::db::ArcDb;

pub fn router() -> Router<ArcDb> {
    Router::<ArcDb>::new().route("/{topic}", get(handle_get).post(post_value))
}

async fn handle_get(
    path: Path<String>,
    state: State<ArcDb>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(accept) = headers.get("accept")
        && accept
            .to_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("text/event-stream")
    {
        return sse_stream(path, state).await.into_response();
    }

    get_latest_value(path, state).await.into_response()
}

async fn get_latest_value(Path(topic): Path<String>, State(db): State<ArcDb>) -> Bytes {
    db.get(&topic).latest().await.unwrap_or_default()
}

async fn sse_stream(
    Path(topic): Path<String>,
    State(db): State<ArcDb>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = db.get(&topic).subscribe();

    let stream = BroadcastStream::new(receiver)
        .filter_map(|value| value.ok())
        .filter_map(|value| String::from_utf8(value.to_vec()).ok())
        .map(|value| {
            #[cfg(feature = "sse_dos_newlines")]
            let value = value.replace("\r\n", "\n");

            value.replace('\r', "\n")
        })
        .map(|value| Ok(Event::default().data(value)));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn post_value(Path(topic): Path<String>, State(db): State<ArcDb>, body: Bytes) -> String {
    db.get(&topic).send(body).await.unwrap_or(0).to_string()
}
