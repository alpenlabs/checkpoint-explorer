use super::utils::resolve_order;
use crate::services::pagination::PaginatedData;
use model::{
    block::Entity as Block,
    checkpoint::{
        ActiveModel, Entity as Checkpoint, RpcCheckpointConfStatus, RpcCheckpointInfo,
        RpcCheckpointInfoCheckpointExp,
    },
};
use sea_orm::{
    prelude::*, ColumnTrait, DatabaseConnection, EntityTrait, Order, QueryFilter, QueryOrder,
    QuerySelect,
};
use tracing::{debug, error, info};
pub struct CheckpointService<'a> {
    pub db: &'a DatabaseConnection,
}

impl<'a> CheckpointService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn checkpoint_exists(&self, idx: u64) -> bool {
        Checkpoint::find()
            .filter(model::checkpoint::Column::Idx.eq(idx))
            .one(self.db)
            .await
            .map(|result| result.is_some())
            .unwrap_or(false)
    }

    /// Insert a new checkpoint into the database
    pub async fn insert_checkpoint(&self, checkpoint: RpcCheckpointInfo) {
        let idx: u64 = checkpoint.idx;

        // for the first checkpoint (idx=0), no need to check the previous checkpoint
        if let Some(previous_idx) = idx.checked_sub(1) {
            let previous_checkpoint_exists = self.checkpoint_exists(previous_idx).await;

            // checkpoints must be continuous, better to restart to re-sync from a valid checkpoint
            if !previous_checkpoint_exists {
                error!(
                    idx,
                    previous_idx, "Cannot insert checkpoint: previous does not exist"
                );
                return;
            }
        }

        // Insert the checkpoint
        let active_model: ActiveModel = checkpoint.into();
        match Checkpoint::insert(active_model).exec(self.db).await {
            Ok(_) => info!(idx, "Checkpoint inserted"),
            Err(err) => error!(idx, ?err, "Failed to insert checkpoint"),
        }
    }

    /// Fetch a checkpoint by its index
    pub async fn get_checkpoint_by_idx(&self, idx: u64) -> Option<RpcCheckpointInfoCheckpointExp> {
        match Checkpoint::find()
            .filter(model::checkpoint::Column::Idx.eq(idx))
            .one(self.db)
            .await
        {
            Ok(Some(checkpoint)) => Some(checkpoint.into()),
            Ok(None) => None,
            Err(err) => {
                error!(?err, "Failed to fetch checkpoint");
                None
            }
        }
    }

    /// Fetch a checkpoint by its L2 block ID
    pub async fn get_checkpoint_idx_by_block_hash(
        &self,
        block_hash: &str,
    ) -> Result<Option<u64>, DbErr> {
        match Block::find()
            .filter(model::block::Column::BlockHash.eq(block_hash))
            .one(self.db)
            .await
        {
            Ok(Some(block)) => {
                debug!(?block, "Block found");
                Ok(Some(block.checkpoint_idx))
            }
            Ok(None) => {
                debug!(%block_hash, "No block found");
                Ok(None)
            }
            Err(err) => {
                error!(?err, "Query failed");
                Err(err)
            }
        }
    }

    /// Fetch a checkpoint by its L2 block height
    pub async fn get_checkpoint_idx_by_block_height(
        &self,
        block_height: u64,
    ) -> Result<Option<u64>, DbErr> {
        debug!(block_height, "Searching for block");

        match Block::find()
            .filter(model::block::Column::Height.eq(block_height))
            .one(self.db)
            .await
        {
            Ok(Some(block)) => {
                debug!(?block, "Block found");
                Ok(Some(block.checkpoint_idx))
            }
            Ok(None) => {
                debug!(block_height, "No block found");
                Ok(None)
            }
            Err(err) => {
                error!(?err, "Query failed");
                Err(err)
            }
        }
    }
    // TODO: move this out of db and have a separate pagination wrapper module
    pub async fn get_paginated_checkpoints(
        &self,
        current_page: u64,
        page_size: u64,
        absolute_first_page: u64,
        order: Option<&str>,
    ) -> PaginatedData<RpcCheckpointInfoCheckpointExp> {
        let total_checkpoints = self.get_total_checkpoint_count().await;
        let total_pages = (total_checkpoints as f64 / page_size as f64).ceil() as u64;
        let offset = (current_page - absolute_first_page) * page_size; // Adjust based on the first page
        let order = resolve_order(order);
        let offset = Some(offset);
        let limit = Some(page_size);

        let items = match Checkpoint::find()
            .filter(Expr::col(model::checkpoint::Column::Idx).is_not_null()) // Ensure idx is not NULL
            .order_by(model::checkpoint::Column::Idx, order) // Sort numerically
            .offset(offset)
            .limit(limit)
            .all(self.db)
            .await
        {
            Ok(checkpoints) => checkpoints.into_iter().map(Into::into).collect(),
            Err(err) => {
                error!(?err, "Failed to fetch paginated checkpoints");
                vec![]
            }
        };

        PaginatedData {
            current_page,
            total_pages,
            absolute_first_page,
            items,
        }
    }

    /// Get the total count of checkpoints in the database
    pub async fn get_total_checkpoint_count(&self) -> u64 {
        use sea_orm::entity::prelude::*;

        match Checkpoint::find().count(self.db).await {
            Ok(count) => count,
            Err(err) => {
                error!(?err, "Failed to count checkpoints");
                0
            }
        }
    }

    /// Get the latest checkpoint index stored in the database
    pub async fn get_latest_checkpoint_index(&self) -> Option<u64> {
        use sea_orm::entity::prelude::*;

        match Checkpoint::find()
            .select_only()
            .column_as(model::checkpoint::Column::Idx.max(), "max_idx")
            .into_tuple::<Option<u64>>()
            .one(self.db)
            .await
        {
            Ok(Some(max_idx)) => max_idx,
            Ok(_) => None, // If no checkpoints exist, return None
            Err(err) => {
                error!(?err, "Failed to fetch latest checkpoint index");
                None
            }
        }
    }

    /// Get the earliest checkpoint index whose status is either `Pending` or `Confirmed`
    pub async fn get_earliest_unfinalized_checkpoint_idx(&self) -> Option<u64> {
        // add the condition to check no checkpoint at all
        self.get_latest_checkpoint_index().await?;
        match Checkpoint::find()
            .filter(model::checkpoint::Column::Status.ne(RpcCheckpointConfStatus::Finalized))
            .order_by(model::checkpoint::Column::Idx, Order::Asc)
            .one(self.db)
            .await
        {
            Ok(Some(checkpoint)) => Some(checkpoint.idx),
            Ok(None) => None,
            Err(err) => {
                error!(?err, "Failed to fetch earliest unfinalized checkpoint");
                None
            }
        }
    }
    /// Get the earliest checkpoint index whose status is `Pending`
    pub async fn get_earliest_pending_checkpoint_idx(&self) -> Option<u64> {
        // add the condition to check no checkpoint at all
        self.get_latest_checkpoint_index().await?;
        match Checkpoint::find()
            .filter(model::checkpoint::Column::Status.eq(RpcCheckpointConfStatus::Pending))
            .order_by(model::checkpoint::Column::Idx, Order::Asc)
            .one(self.db)
            .await
        {
            Ok(Some(checkpoint)) => Some(checkpoint.idx),
            Ok(None) => None,
            Err(err) => {
                error!(?err, "Failed to fetch earliest pending checkpoint");
                None
            }
        }
    }
    /// Get the earliest checkpoint index whose status is `Confirmed`
    pub async fn get_earliest_confirmed_checkpoint_idx(&self) -> Option<u64> {
        // add the condition to check no checkpoint at all
        self.get_latest_checkpoint_index().await?;
        match Checkpoint::find()
            .filter(model::checkpoint::Column::Status.eq(RpcCheckpointConfStatus::Confirmed))
            .order_by(model::checkpoint::Column::Idx, Order::Asc)
            .one(self.db)
            .await
        {
            Ok(Some(checkpoint)) => Some(checkpoint.idx),
            Ok(None) => None,
            Err(err) => {
                error!(?err, "Failed to fetch earliest confirmed checkpoint");
                None
            }
        }
    }
    /// Get the last checkpoint index whose status is `Finalized`
    pub async fn get_last_finalized_checkpoint_idx(&self) -> Option<u64> {
        // add the condition to check no checkpoint at all
        self.get_latest_checkpoint_index().await?;
        match Checkpoint::find()
            .filter(model::checkpoint::Column::Status.eq(RpcCheckpointConfStatus::Finalized))
            .order_by(model::checkpoint::Column::Idx, Order::Desc)
            .one(self.db)
            .await
        {
            Ok(Some(checkpoint)) => Some(checkpoint.idx),
            Ok(None) => None,
            Err(err) => {
                error!(?err, "Failed to fetch last finalized checkpoint");
                None
            }
        }
    }

    /// Update the status of a checkpoint
    pub async fn update_checkpoint(
        &self,
        checkpoint_idx: u64,
        updated_checkpoint: RpcCheckpointInfo,
    ) -> Result<(), DbErr> {
        match Checkpoint::find()
            .filter(model::checkpoint::Column::Idx.eq(checkpoint_idx))
            .one(self.db)
            .await
        {
            Ok(Some(checkpoint)) => {
                let mut active_model: ActiveModel = checkpoint.into();
                let updated_checkpoint: ActiveModel = updated_checkpoint.into();
                let status = updated_checkpoint.status.clone();
                active_model.status = status;
                active_model.checkpoint_txid = updated_checkpoint.checkpoint_txid;

                match active_model.update(self.db).await {
                    Ok(_) => {
                        info!(checkpoint_idx, "Checkpoint updated");
                        Ok(())
                    }
                    Err(err) => {
                        error!(checkpoint_idx, ?err, "Failed to update checkpoint");
                        Err(err)
                    }
                }
            }
            // idx is omitted from the error string — the caller logs it as a structured field
            // before invoking this function, so it will appear in the surrounding log context.
            Ok(None) => Err(DbErr::RecordNotFound("checkpoint not found".into())),
            Err(err) => {
                error!(checkpoint_idx, ?err, "Failed to query checkpoint");
                Err(err)
            }
        }
    }
}
