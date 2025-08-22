use std::net::{Ipv4Addr, SocketAddr};

use axum::response::Html;
use color_eyre::eyre::{Context, Result};
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable as RedocServable};
use utoipa_scalar::{Scalar, Servable as ScalarServable};
use utoipa_swagger_ui::SwaggerUi;

mod db;
mod topic;

pub use serde_json::Value;

use crate::db::ArcDb;

#[derive(OpenApi)]
#[openapi()]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE))
        .with(ErrorLayer::default())
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()?,
        )
        .try_init()?;

    info!(
        "Starting {} {}...",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi()).split_for_parts();

    let db: ArcDb = Default::default();

    let router = router.with_state(db);

    let router = router
        .merge(SwaggerUi::new("/api-docs/swagger-ui").url("/api-docs/openapi.json", api.clone()))
        .merge(Redoc::with_url("/api-docs/redoc", api.clone()))
        .merge(RapiDoc::new("/api-docs/openapi.json").path("/api-docs/rapidoc"))
        .merge(Scalar::with_url("/api-docs/scalar", api))
        .route(
            "/api-docs",
            axum::routing::get(|| async {
                Html(
                    [
                        ("Swagger UI", "/api-docs/swagger-ui"),
                        ("Redoc", "/api-docs/redoc"),
                        ("RapiDoc", "/api-docs/rapidoc"),
                        ("Scalar", "/api-docs/scalar"),
                    ]
                    .map(|(name, url)| format!("<a href=\"{}\">{}</a>", url, name))
                    .join("\n"),
                )
            }),
        );

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
