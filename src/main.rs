mod errors;
mod routes;
mod instrumentation;
mod models;
mod repository;
mod tests;
mod utilities;
mod config;
mod extractors;

#[cfg(not(unix))]
use std::future;

use axum::{
    extract::DefaultBodyLimit, http::{HeaderValue, Method}, routing::{delete, get, post}, Extension, Json, Router
};
use config::Config;
use dotenvy_macro::dotenv;
use errors::AppResult;
use routes::{delete::delete_endpoint, download::download_endpoint, info::info_endpoint, preview::preview_endpoint, stats::service_stats, upload::upload_endpoint};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::{net::TcpListener, signal};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
};

use crate::config::load_config;

const CONFIG_PATH: &str = "Config.toml";
const DATABASE_URL: &str = dotenv!("DATABASE_URL");

#[derive(Debug, Clone)]
struct AppContext {
    cfg: Config,
    db: PgPool,
}

fn router(cfg: Config, db: PgPool) -> Router {
    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_origin(AllowOrigin::exact(
            HeaderValue::from_str(&cfg.general.cors_origin).unwrap(),
        ))
        .allow_credentials(true);

    let router = Router::new()
        .route("/health", get(health_check))
        .route("/upload", post(upload_endpoint))
        .route("/delete/:upload_id", delete(delete_endpoint))
        .route("/download/:upload_id", get(download_endpoint))
        .route("/info/:upload_id", get(info_endpoint))
        .route("/preview/:upload_id", get(preview_endpoint))
        .route("/stats", get(service_stats))
        .layer((
            DefaultBodyLimit::disable(),
            RequestBodyLimitLayer::new(1024 * 1024 * 1024 + 1024),
            Extension(AppContext { cfg, db }),
            cors_layer,
        ));

    instrumentation::add_layer(router)
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let config = load_config(CONFIG_PATH).await?;

    instrumentation::setup(&config.instrumentation.directives)?;

    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect(DATABASE_URL)
        .await?;

    let listener = TcpListener::bind(&config.general.bind_address).await?;
    tracing::info!("api is available on http://{}", config.general.bind_address);

    axum::serve(listener, router(config, db))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn health_check() -> Json<String> {
    Json(String::from("im alive!"))
}
