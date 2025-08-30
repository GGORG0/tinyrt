use std::net::{Ipv4Addr, SocketAddr};

use axum::Router;
use color_eyre::eyre::{Context, Result};
use tokio::net::TcpListener;
use tracing::info;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod interfaces;
mod topic;

pub use serde_json::Value;

use crate::db::ArcDb;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE))
        .with(ErrorLayer::default())
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(format!("{}=info", env!("CARGO_PKG_NAME")).parse()?)
                .from_env()?,
        )
        .try_init()?;

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
    info!(
        "Listening on {}.",
        listener
            .local_addr()
            .wrap_err("failed to get local address")?,
    );

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}
