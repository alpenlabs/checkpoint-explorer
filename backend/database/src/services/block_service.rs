use model::block::{ActiveModel as BlockActiveModel, Entity as Block, RpcBlockHeader};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, Set};
use tracing::{debug, error};

/// Wrapper around the database connection
pub struct BlockService<'a> {
    pub db: &'a DatabaseConnection,
}

impl<'a> BlockService<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn insert_block(&self, rpc_block_header: RpcBlockHeader, checkpoint_idx: u64) {
        // Use `From` to convert `RpcBlockHeader` into an `ActiveModel`
        let mut active_model: BlockActiveModel = rpc_block_header.into();

        let height = active_model.height.clone().unwrap();
        let block_id = active_model.block_hash.clone().unwrap();

        // ensure that blocks exist incrementally and continuously
        let can_insert_block = self.can_insert_block(height).await;
        if !can_insert_block {
            // TODO(STR-1454): handle this gracefully using service framework
            panic!("last_block_height does not match the expected height!");
        }

        active_model.checkpoint_idx = Set(checkpoint_idx);

        // Insert the block using the Entity::insert() method
        match Block::insert(active_model).exec(self.db).await {
            Ok(_) => {
                debug!(height, %block_id, "Block inserted and indexed");
            }
            Err(err) if is_duplicate_entry(&err) => {
                debug!(height, "Block already exists, skipping");
            }
            Err(err) => {
                error!(height, ?err, "Failed to insert block");
            }
        }
    }

    /// Get the latest block height stored in the database
    pub async fn get_latest_block_index(&self) -> Option<u64> {
        match Block::find()
            .select_only()
            .column_as(model::block::Column::Height.max(), "max_height")
            .into_tuple::<Option<u64>>()
            .one(self.db)
            .await
        {
            Ok(Some(max_height)) => max_height,
            Ok(_) => None, // If no blocks exist, return None
            Err(err) => {
                error!(?err, "Failed to fetch latest block index");
                None
            }
        }
    }

    async fn block_exists(&self, height: u64) -> bool {
        Block::find()
            .filter(model::block::Column::Height.eq(height))
            .one(self.db)
            .await
            .map(|result| result.is_some())
            .unwrap_or(false)
    }

    async fn prev_block_exists(&self, height: u64) -> bool {
        if height == 0 {
            // genesis block has no predecessor
            return true;
        }
        self.block_exists(height - 1).await
    }

    /// Check if a block can be inserted at the given height
    /// The conditions it should meet are:
    ///     1. The block table should be empty
    ///     2. or, the block table should have the previous block
    pub async fn can_insert_block(&self, height: u64) -> bool {
        if self.get_latest_block_index().await.is_none() {
            return true;
        }
        self.prev_block_exists(height).await
    }
}

fn is_duplicate_entry(err: &DbErr) -> bool {
    err.to_string().contains("1062")
}
