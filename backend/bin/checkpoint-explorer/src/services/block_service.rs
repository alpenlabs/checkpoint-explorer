use database::connection::DatabaseWrapper;
use database::services::block_service::BlockService;
use fullnode_client::fetcher::StrataFetcher;
use model::checkpoint::L2BlockFetchTarget;
use std::sync::Arc;
use tokio::sync::watch::Receiver;
use tracing::{error, info};

const MAX_HEADERS_RANGE: u64 = 5000;

pub async fn run_block_fetcher(
    fetcher: Arc<StrataFetcher>,
    database: Arc<DatabaseWrapper>,
    mut rx: Receiver<L2BlockFetchTarget>,
) {
    info!("Starting block fetcher...");
    loop {
        if rx.changed().await.is_err() {
            break;
        }
        let target = *rx.borrow_and_update();
        fetch_blocks(fetcher.clone(), database.clone(), target).await;
    }
}

async fn fetch_blocks(fetcher: Arc<StrataFetcher>, database: Arc<DatabaseWrapper>, target: u64) {
    let block_db = BlockService::new(&database.db);
    let start = block_db
        .get_latest_block_index()
        .await
        .map(|h| h + 1)
        .unwrap_or(0);

    if start > target {
        info!("No blocks to fetch");
        return;
    }

    let mut cursor = start;
    info!(cursor, target, "Fetching blocks");
    while cursor <= target {
        let chunk_end = (cursor + MAX_HEADERS_RANGE - 1).min(target);
        match fetcher.fetch_headers_in_range(cursor, chunk_end).await {
            Ok(headers) => {
                for header in headers {
                    let checkpoint_idx = header.epoch;
                    block_db.insert_block(header, checkpoint_idx).await;
                }
                cursor = chunk_end + 1;
            }
            Err(e) => {
                error!(?e, cursor, chunk_end, "Failed to fetch block headers");
                return;
            }
        }
    }
}
