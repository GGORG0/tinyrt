use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use bytes::Bytes;

use crate::db::ArcDb;

pub fn router() -> Router<ArcDb> {
    Router::<ArcDb>::new().route("/{topic}", get(get_latest_value).post(post_value))
}

async fn get_latest_value(Path(topic): Path<String>, State(db): State<ArcDb>) -> Bytes {
    db.get(&topic).latest().await.unwrap_or_default()
}

async fn post_value(Path(topic): Path<String>, State(db): State<ArcDb>, body: Bytes) -> String {
    db.get(&topic).send(body).await.unwrap_or(0).to_string()
}
