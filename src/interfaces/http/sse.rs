use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream};

use crate::db::ArcDb;

pub fn is_sse_request(headers: &HeaderMap) -> bool {
    if let Some(accept) = headers.get("accept")
        && accept
            .to_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("text/event-stream")
    {
        true
    } else {
        false
    }
}

pub async fn sse_stream(
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
