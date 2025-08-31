use std::error::Error as _;
use std::{
    io::Error,
    net::{Ipv4Addr, SocketAddr},
};

use axum::{Router, extract::MatchedPath, http::Request};
use tokio::net::TcpListener;
use tracing::{info, info_span};

mod db;
mod interfaces;
mod topic;

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

    #[cfg(feature = "socketio")]
    let router = router.layer(interfaces::socketio::layer(db.clone()));

    #[cfg(feature = "permissive_cors")]
    let router = router.layer(tower_http::cors::CorsLayer::permissive());

    #[cfg(feature = "logging")]
    let router = router.layer(
        tower_http::trace::TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
            let matched = request
                .extensions()
                .get::<MatchedPath>()
                .map(MatchedPath::as_str);

            info_span!(
                "http_request",
                method = ?request.method(),
                matched,
                uri = %request.uri(),
                some_other_field = tracing::field::Empty,
            )
        }),
    );

    let router = router.with_state(db);

    let address = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080));
    let listener = TcpListener::bind(address).await?;

    if let Ok(local_addr) = listener.local_addr() {
        info!("Listening on {}.", local_addr);
    }

    axum::serve(listener, router.into_make_service()).await
}

#[cfg(feature = "logging")]
fn setup_tracing() {
    use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE))
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .or_else(|err| {
                    use std::env::VarError;

                    if err.source().and_then(|e| e.downcast_ref::<VarError>())
                        == Some(&VarError::NotPresent)
                    {
                        #[cfg(debug_assertions)]
                        let default_directive = format!(
                            "warn,{}=debug,tower_http=debug,axum::rejection=trace",
                            env!("CARGO_PKG_NAME")
                        );

                        #[cfg(not(debug_assertions))]
                        let default_directive = format!("warn,{}=info", env!("CARGO_PKG_NAME"));

                        Ok(default_directive.into())
                    } else {
                        Err(err)
                    }
                })
                .expect("Failed to parse RUST_LOG environment variable"),
        )
        .init();
}

#[cfg(not(feature = "logging"))]
fn setup_tracing() {}
