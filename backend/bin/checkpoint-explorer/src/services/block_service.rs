use database::connection::DatabaseWrapper;
use database::services::{block_service::BlockService, checkpoint_service::CheckpointService};
use fullnode_client::fetcher::StrataFetcher;
use model::block::RpcBlockHeader;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tracing::{debug, info};

/// Event sent to block fetcher to request fetching of blocks for the checkpoint
#[derive(Debug, Clone)]
pub struct CheckpointFetch {
    pub idx: u64,
}
impl CheckpointFetch {
    pub fn new(idx: u64) -> Self {
        Self { idx }
    }
}
pub async fn run_block_fetcher(
    fetcher: Arc<StrataFetcher>,
    database: Arc<DatabaseWrapper>,
    mut rx: Receiver<CheckpointFetch>,
) {
    info!("Starting block fetcher...");
    while let Some(CheckpointFetch { idx }) = rx.recv().await {
        debug!("Received checkpoint: {:?}", idx);
        fetch_blocks_in_checkpoint(fetcher.clone(), database.clone(), idx).await;
    }
}

async fn fetch_blocks_in_checkpoint(
    fetcher: Arc<StrataFetcher>,
    database: Arc<DatabaseWrapper>,
    checkpoint_idx: u64,
) {
    let checkpoint_db = CheckpointService::new(&database.db);
    let block_db = BlockService::new(&database.db);
    let checkpoint = checkpoint_db.get_checkpoint_by_idx(checkpoint_idx).await;
    if let Some(c) = checkpoint {
        let mut start = c.l2_range.0;
        let end = c.l2_range.1;

        // we will reach this point only when we are sure that we must fetch from particular
        // checkpoint. So having the highest among the blocks must give us the shortcut
        // to determine the most optimal starting point.
        let last_block = block_db.get_latest_block_index().await;
        if let Some(last_block_height) = last_block {
            // start from the next block
            if last_block_height >= start {
                start = last_block_height + 1;
            }
        }
        if start > end {
            info!("No blocks to fetch for checkpoint {}", checkpoint_idx);
            return;
        }
        info!(
            "Fetching blocks from {} to {} for checkpoint {}",
            start, end, checkpoint_idx
        );
        for block_height in start..=end {
            if let Ok(block_headers) = fetcher
                .fetch_data::<Vec<RpcBlockHeader>>("strata_getHeadersAtIdx", block_height)
                .await
            {
                for block_header in block_headers {
                    block_db
                        .insert_block(block_header.clone(), checkpoint_idx)
                        .await;
                }
            }
        }
    }
}
