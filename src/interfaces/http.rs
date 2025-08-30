use std::collections::HashMap;

use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, request::Parts},
    response::IntoResponse,
    routing::get,
};
use axum_extra::{
    TypedHeader,
    headers::{Connection, Upgrade},
};
use bytes::Bytes;

use crate::db::ArcDb;

mod sse;
mod ws;

pub fn router() -> Router<ArcDb> {
    Router::<ArcDb>::new().route(
        "/{*topic}",
        get(handle_get).post(post_value).connect(ws::handle_connect),
    )
}

async fn handle_get(
    path: Path<String>,
    state: State<ArcDb>,
    connection: Option<TypedHeader<Connection>>,
    upgrade: Option<TypedHeader<Upgrade>>,
    headers: HeaderMap,
    query: Query<HashMap<String, String>>,
    mut parts: Parts,
) -> impl IntoResponse {
    if sse::is_sse_request(&headers) {
        return sse::sse_stream(path, state).await.into_response();
    }

    if ws::is_ws_request(&connection, &upgrade) {
        return ws::get_upgrade_handler(&mut parts, path, query, state)
            .await
            .into_response();
    }

    get_latest_value(path, state).await.into_response()
}

async fn get_latest_value(Path(topic): Path<String>, State(db): State<ArcDb>) -> Bytes {
    db.get(&topic).latest().await.unwrap_or_default()
}

async fn post_value(Path(topic): Path<String>, State(db): State<ArcDb>, body: Bytes) -> String {
    db.get(&topic).send(body).await.unwrap_or(0).to_string()
}
