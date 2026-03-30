mod services;
mod utils;
// mod cors;

use axum::{routing::get, Router};
use clap::Parser;
use database::connection::DatabaseWrapper;
use migration::{Migrator, MigratorTrait};
use dotenvy::dotenv;
use fullnode_client::fetcher::StrataFetcher;
use reqwest::Method;
use services::{
    block_service::run_block_fetcher,
    checkpoint_service::{start_checkpoint_fetcher, start_checkpoint_status_updater_task},
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::FmtSubscriber;
use utils::config::Config;

use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = Config::parse();

    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()) // Uses RUST_LOG
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set logging subscriber");

    // Initialize database and fetcher
    let database = Arc::new(DatabaseWrapper::new(&config.database_url).await);
    let fetcher = Arc::new(StrataFetcher::new(config.strata_fullnode));

    Migrator::up(&database.db, None)
        .await
        .expect("Failed to run database migrations");

    // Channels for communication between checkpoint fetcher and block fetcher
    let (tx, rx) = mpsc::channel(100);

    // Start block fetcher task
    let fetcher_clone = fetcher.clone();
    let database_clone = database.clone();
    tokio::spawn(async move {
        run_block_fetcher(fetcher_clone, database_clone, rx).await;
    });

    // Start checkpoint fetcher task
    let fetcher_clone = fetcher.clone();
    let database_clone = database.clone();
    tokio::spawn(async move {
        start_checkpoint_fetcher(fetcher_clone, database_clone, tx, config.fetch_interval).await;
    });

    // Start checkpoint status updater task
    let fetcher_clone = fetcher.clone();
    let database_clone = database.clone();
    tokio::spawn(async move {
        start_checkpoint_status_updater_task(
            fetcher_clone,
            database_clone,
            config.status_update_interval,
        )
        .await;
    });

    // api routes
    let api_routes = Router::new()
        .route("/checkpoints", get(services::api_service::checkpoints))
        .route("/checkpoint", get(services::api_service::checkpoint))
        .route("/search", get(services::api_service::search));

    // Add Cors layer for Allow cross origin request
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    // Setup Axum router
    let app: Router = Router::new()
        .nest("/api", api_routes)
        .with_state(database.clone());

    // Start the server
    let addr = format!("0.0.0.0:{}", config.server_port).parse().unwrap();
    info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.layer(cors).into_make_service())
        .await
        .unwrap();
}
