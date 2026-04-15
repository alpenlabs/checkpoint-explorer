mod services;
mod utils;
// mod cors;

use axum::{routing::get, Router};
use clap::Parser;
use database::connection::DatabaseWrapper;
use dotenvy::dotenv;
use fullnode_client::fetcher::StrataFetcher;
use migration::{Migrator, MigratorTrait};
use reqwest::Method;
use services::{
    block_service::run_block_fetcher,
    checkpoint_service::{start_checkpoint_fetcher, start_checkpoint_status_updater_task},
};
use model::checkpoint::L2BlockFetchTarget;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, info};
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

    // Signals the block fetcher how far to fetch: carries the L2 end slot of
    // the latest checkpoint processed by the checkpoint fetcher.
    let (tx, rx) = watch::channel::<L2BlockFetchTarget>(0);

    // Start block fetcher task
    let fetcher_clone = fetcher.clone();
    let database_clone = database.clone();
    let block_fetcher_handle = tokio::spawn(async move {
        run_block_fetcher(fetcher_clone, database_clone, rx).await;
    });

    // Start checkpoint fetcher task
    let fetcher_clone = fetcher.clone();
    let database_clone = database.clone();
    let checkpoint_fetcher_handle = tokio::spawn(async move {
        start_checkpoint_fetcher(fetcher_clone, database_clone, tx, config.fetch_interval).await;
    });

    // Start checkpoint status updater task
    let fetcher_clone = fetcher.clone();
    let database_clone = database.clone();
    let status_updater_handle = tokio::spawn(async move {
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
    info!(%addr, "Server started");
    let server = axum::Server::bind(&addr).serve(app.layer(cors).into_make_service());

    // TODO: ideally use a service framework for lifecycle management in a follow up pr
    tokio::select! {
        res = block_fetcher_handle => { error!(?res, "block fetcher exited unexpectedly"); }
        res = checkpoint_fetcher_handle => { error!(?res, "checkpoint fetcher exited unexpectedly"); }
        res = status_updater_handle => { error!(?res, "status updater exited unexpectedly"); }
        res = server => { error!(?res, "server exited unexpectedly"); }
    }
    std::process::exit(1);
}
