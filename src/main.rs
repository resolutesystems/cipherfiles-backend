mod delete;
mod download;
mod errors;
mod info;
mod instrumentation;
mod models;
mod repository;
mod stats;
mod tests;
mod upload;
mod utilities;

#[cfg(not(unix))]
use std::future;

use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method},
    routing::{delete, get, post},
    Extension, Router,
};
use dotenvy_macro::dotenv;
use download::download_endpoint;
use errors::AppResult;
use info::info_endpoint;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::{net::TcpListener, signal};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
};
use upload::upload_endpoint;

use crate::{delete::delete_endpoint, stats::service_stats};

const DATABASE_URL: &str = dotenv!("DATABASE_URL");
const WEBSITE_ADDRESS: &str = dotenv!("WEBSITE_ADDRESS");
const ADDRESS: &str = dotenv!("ADDRESS");

#[cfg(not(test))]
const STORAGE_PATH: &str = dotenv!("STORAGE_PATH");
#[cfg(test)]
const STORAGE_PATH: &str = "./src/tests/storage/";

#[derive(Clone)]
struct AppContext {
    db: PgPool,
}

fn router(db: PgPool) -> Router {
    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_origin(AllowOrigin::exact(
            HeaderValue::from_str(WEBSITE_ADDRESS).unwrap(),
        ))
        .allow_credentials(true);

    let router = Router::new()
        .route("/upload", post(upload_endpoint))
        .route("/delete/:upload_id", delete(delete_endpoint))
        .route("/download/:upload_id", get(download_endpoint))
        .route("/info/:upload_id", get(info_endpoint))
        .route("/stats", get(service_stats))
        .layer((
            DefaultBodyLimit::disable(),
            RequestBodyLimitLayer::new(1024 * 1024 * 1024 + 1024),
            Extension(AppContext { db }),
            cors_layer,
        ));

    instrumentation::add_layer(router)
}

#[tokio::main]
async fn main() -> AppResult<()> {
    instrumentation::setup()?;

    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect(DATABASE_URL)
        .await?;

    let listener = TcpListener::bind(ADDRESS).await?;
    tracing::info!("api is available on http://{ADDRESS}");

    axum::serve(listener, router(db))
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
