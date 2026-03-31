use crate::services::block_service::CheckpointFetch;
use database::connection::DatabaseWrapper;
use database::services::{block_service::BlockService, checkpoint_service::CheckpointService};
use fullnode_client::fetcher::StrataFetcher;
use model::checkpoint::RpcCheckpointInfo;
use std::cmp::min;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{error, info, warn};

/// This function fetches the checkpoints from the fullnode and inserts them into the database
/// It will run in a loop with a delay of `fetch_interval` seconds
pub async fn start_checkpoint_fetcher(
    fetcher: Arc<StrataFetcher>,
    database: Arc<DatabaseWrapper>,
    tx: Sender<CheckpointFetch>,
    fetch_interval: u64,
) {
    info!("Starting checkpoint fetcher...");
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(fetch_interval));

    loop {
        interval.tick().await;
        match fetch_checkpoints(fetcher.clone(), database.clone(), tx.clone()).await {
            Ok(_) => (),
            Err(e) => tracing::error!("Error fetching checkpoints: {}", e),
        }
    }
}

/// This function fetches the checkpoints from the fullnode and inserts them into the database
/// It then sends the checkpoint index to the block fetcher task to fetch the corresponding block
async fn fetch_checkpoints(
    fetcher: Arc<StrataFetcher>,
    database: Arc<DatabaseWrapper>,
    tx: Sender<CheckpointFetch>,
) -> anyhow::Result<()> {
    let checkpoint_db = CheckpointService::new(&database.db);
    info!("Fetching checkpoints from fullnode...");
    let fullnode_last_checkpoint = fetcher
        .get_latest_index("strata_getLatestCheckpointIndex")
        .await?;
    // handle None case
    if fullnode_last_checkpoint.is_none() {
        warn!("Failed to fetch latest checkpoint index from fullnode or no checkpoint yet.");
        return Ok(());
    }
    let fn_chkpt = fullnode_last_checkpoint.unwrap();
    let starting_checkpoint = get_starting_checkpoint_idx(database.clone()).await?;
    info!(
        "latest checkpoint index in fullnode: {}, local checkpoint to start block indexing from: {}",
        fn_chkpt, starting_checkpoint
    );
    for idx in starting_checkpoint..=fn_chkpt {
        if !checkpoint_db.checkpoint_exists(idx).await {
            info!("Checkpoint does not exist in db, fetching checkpoint with idx {}", idx);
            if let Ok(checkpoint) = fetcher
                .fetch_data::<RpcCheckpointInfo>("strata_getCheckpointInfo", idx)
                .await
            {
                checkpoint_db.insert_checkpoint(checkpoint.clone()).await;
            }
        }
        let range = CheckpointFetch::new(idx);
        tx.send(range).await?;
    }
    Ok(())
}

/// It is a helper function that returns the starting checkpoint index to start fetching from
/// It will return the minimum of the last checkpoint in the database and the checkpoint corresponding to
/// last block in the database
async fn get_starting_checkpoint_idx(db: Arc<DatabaseWrapper>) -> anyhow::Result<u64> {
    let checkpoint_db = CheckpointService::new(&db.db);
    let block_db = BlockService::new(&db.db);

    let last_block = block_db.get_latest_block_index().await;

    let local_last_checkpoint = match checkpoint_db.get_latest_checkpoint_index().await {
        Some(idx) => idx,
        // no checkpoints in db yet — start from the beginning
        None => return Ok(0),
    };

    // we are calling it probable_* to consider some weirdest condition when
    // we have the block but no any earlier checkpoint (before where block corresponds)
    let probable_starting_checkpoint: u64 = if let Some(block_height) = last_block {
        checkpoint_db
            .get_checkpoint_idx_by_block_height(block_height)
            .await?
            .unwrap_or(0)
    } else {
        0
    };

    Ok(min(probable_starting_checkpoint, local_last_checkpoint))
}

/// This function starts the checkpoint status updater task
pub async fn start_checkpoint_status_updater_task(
    fetcher: Arc<StrataFetcher>,
    database: Arc<DatabaseWrapper>,
    update_interval: u64,
) {
    info!("Starting checkpoint status updater...");

    // Spawn the "pending" checkpoint updater loop
    let fetcher_clone = fetcher.clone();
    let database_clone = database.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(update_interval));

        loop {
            interval.tick().await;

            if let Err(e) =
                update_checkpoints_status(fetcher_clone.clone(), database_clone.clone(), "pending")
                    .await
            {
                tracing::error!("Error fetching pending checkpoints: {}", e);
            }
        }
    });

    // Spawn the "confirmed" checkpoint updater loop
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(update_interval));

        loop {
            interval.tick().await;

            if let Err(e) =
                update_checkpoints_status(fetcher.clone(), database.clone(), "confirmed").await
            {
                tracing::error!("Error fetching confirmed checkpoints: {}", e);
            }
        }
    });
}

/// This function continuously updates the status of the checkpoints which are yet to be finalized.
///
/// This algorithm works on the assumptions that the checkpoints must get finalized in the order they are created.
/// i.e. (n-1)th checkpoint gets finalized before (n)th.
///
/// ** Algorithm **
/// 1. Get the earliest checkpoint idx whose status is Either pending or Confirmed
/// 2. Fetch the checkpoint from fullnode
/// 3. If the checkpoint status is different from the one in the database,
///    a. update the status may be
///    b. increment the idx and go to step 2
/// 4. Else break the loop
async fn update_checkpoints_status(
    fetcher: Arc<StrataFetcher>,
    database: Arc<DatabaseWrapper>,
    status: &str,
) -> anyhow::Result<()> {
    let checkpoint_db = CheckpointService::new(&database.db);

    let mut idx: u64 = if status == "pending" {
        match checkpoint_db.get_earliest_pending_checkpoint_idx().await {
            Some(i) => i,
            None => {
                info!("No more pending checkpoints locally.");
                return Ok(());
            }
        }
    } else if status == "confirmed" {
        match checkpoint_db.get_earliest_confirmed_checkpoint_idx().await {
            Some(i) => i,
            None => {
                info!("No more confirmed checkpoints locally.");
                return Ok(());
            }
        }
    } else {
        return Ok(());
    };

    loop {
        // This is the stopping condition for the loop. If the checkpoint is not found in the database,
        // break the loop as we have already updated all the checkpoints.
        let Some(checkpoint_in_db) = checkpoint_db.get_checkpoint_by_idx(idx).await else {
            info!("Status of all checkpoints in db is already updated.");
            return Ok(());
        };

        let Ok(checkpoint_from_rpc) = fetcher
            .fetch_data::<RpcCheckpointInfo>("strata_getCheckpointInfo", idx)
            .await
        else {
            warn!("Checkpoint not found in fullnode for idx {}", idx);
            return Ok(());
        };

        let status = match checkpoint_from_rpc.confirmation_status {
            Some(status) => status.to_string(),
            None => {
                warn!("Checkpoint status is None for idx {}", idx);
                return Ok(()); // Simply return and continue execution instead of erroring
            }
        };

        // if there is no change in status, return by doing nothing
        if checkpoint_in_db
            .confirmation_status
            .map_or("-".to_string(), |s| s.to_string())
            == status.to_string()
        {
            // if the status is unchanged then do nothing
            return Ok(());
        }

        info!("Updating checkpoint status: idx={}, status={}", idx, status.clone());
        // update the db with the new checkpoint record instead of tweaking the existing one
        // as there could be change in both status and txid
        checkpoint_db
            .update_checkpoint(idx, checkpoint_from_rpc)
            .await
            .map_err(|e| {
                error!("Error updating checkpoint status: {:?}", e);
                anyhow::anyhow!("Failed to update checkpoint status")
            })?;

        idx = idx.saturating_add(1);
    }
}
