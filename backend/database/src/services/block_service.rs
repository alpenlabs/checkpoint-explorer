use model::{block::{RpcBlockHeader, ActiveModel as BlockActiveModel, Entity as Block}};
use sea_orm::{ ColumnTrait, DatabaseConnection,  EntityTrait, QueryFilter, QuerySelect, Set};
use tracing::error;
use model::pgu64::PgU64;

/// Wrapper around the database connection
pub struct BlockService<'a> {
    pub db: &'a DatabaseConnection,
}

impl<'a> BlockService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn insert_block(&self, rpc_block_header: RpcBlockHeader, checkpoint_idx: i64)   {
        // Use `From` to convert `RpcBlockHeader` into an `ActiveModel`
        let mut active_model: BlockActiveModel = rpc_block_header.into();
 
        let height = active_model.height.clone().unwrap();
        let block_id = active_model.block_hash.clone().unwrap();

        // If block already exists locally do nothing
        if self.block_exists(height).await{
            tracing::debug!("Block already exists, height={}", PgU64::i64_to_u64(height));
            return;
        }
        // ensure that blocks exist incrementally and continuously
        let can_insert_block = self.can_insert_block(height).await;
        if !can_insert_block {
            panic!("last_block_height does not match the expected height!"); 
        }

        active_model.checkpoint_idx = Set(checkpoint_idx);

        // Insert the block using the Entity::insert() method
        match Block::insert(active_model).exec(self.db).await {
            Ok(_) => {
                tracing::debug!(
                    "Block inserted & indexed successfully: height={}, block_hash={}",
                    PgU64::i64_to_u64(height),
                    block_id
                );
            }
            Err(err) => {
                tracing::error!(
                    "Error inserting block with height {}: {:?}",
                    PgU64::i64_to_u64(height), err
                );
            }
        }
    }
    
    /// Get the latest checkpoint index stored in the database
    pub async fn get_latest_block_index(&self) -> Option<i64> {
        // use sea_orm::entity::prelude::*;

        match Block::find()
            .select_only()
            .column_as(model::block::Column::Height.max(), "max_height")
            .into_tuple::<Option<i64>>() // Fetch the max value as a tuple
            .one(self.db)
            .await
        {
            Ok(Some(max_height)) => max_height,
            Ok(_) => None, // If no block exist, return None
            Err(err) => {
                error!("Failed to fetch the latest  block index: {:?}", err);
                None
            }
        }
    }
    
    async fn block_exists(&self, height: i64) -> bool {
        Block::find()
            .filter(model::block::Column::Height.eq(height))
            .one(self.db)
            .await
            .map(|result| result.is_some())
            .unwrap_or(false)
    }

    async fn prev_block_exists(&self, height: i64) -> bool {
        if height == i64::MIN {
            // return true for the genesis block
            return true;
        }
        self.block_exists(height-1).await
    }

    /// Check if a block can be inserted at the given height
    /// The conditions it should meet are:
    ///     1. The block table should be empty 
    ///     2. or, the block table should have the previous block
    pub async fn can_insert_block(&self, height: i64) -> bool {
        if self.get_latest_block_index().await.is_none() {
            return true;
        }
        self.prev_block_exists(height).await
    }
}

