use axum::{
    Router,
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
};
use bytes::Bytes;

use crate::db::ArcDb;

mod sse;

pub fn router() -> Router<ArcDb> {
    Router::<ArcDb>::new().route("/{topic}", get(handle_get).post(post_value))
}

async fn handle_get(
    path: Path<String>,
    state: State<ArcDb>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if sse::is_sse_request(&headers) {
        return sse::sse_stream(path, state).await.into_response();
    }

    get_latest_value(path, state).await.into_response()
}

async fn get_latest_value(Path(topic): Path<String>, State(db): State<ArcDb>) -> Bytes {
    db.get(&topic).latest().await.unwrap_or_default()
}

async fn post_value(Path(topic): Path<String>, State(db): State<ArcDb>, body: Bytes) -> String {
    db.get(&topic).send(body).await.unwrap_or(0).to_string()
}
