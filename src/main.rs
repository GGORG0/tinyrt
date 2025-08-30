use std::{
    io::Error,
    net::{Ipv4Addr, SocketAddr},
};

use axum::Router;
use tokio::net::TcpListener;
use tracing::info;

mod db;
mod interfaces;
mod topic;

pub use serde_json::Value;

use crate::db::ArcDb;

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_tracing();

    info!(
        "Starting {} {}...",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let db: ArcDb = Default::default();

    let router = Router::new();

    let router = router.merge(interfaces::http::router());

    let router = router.layer(interfaces::socketio::layer(db.clone()));

    let router = router.with_state(db);

    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080));
    let listener = TcpListener::bind(address).await?;

    if let Ok(local_addr) = listener.local_addr() {
        info!("Listening on {}.", local_addr);
    }

    axum::serve(listener, router.into_make_service()).await
}

#[cfg(feature = "tracing-subscriber")]
fn setup_tracing() {
    use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE))
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(
                    format!("{}=info", env!("CARGO_PKG_NAME"))
                        .parse()
                        .expect("hardcoded default directive should be valid"),
                )
                .from_env()
                .expect("Failed to parse RUST_LOG environment variable"),
        )
        .init();
}

#[cfg(not(feature = "tracing-subscriber"))]
fn setup_tracing() {}
