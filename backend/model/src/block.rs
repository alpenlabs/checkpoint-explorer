use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::{NotSet, Set};
use serde::{Deserialize, Serialize};

/// Represents the Block model for the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blocks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub block_hash: String,
    pub height: u64,
    pub checkpoint_idx: u64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Implements conversion from `RpcBlockHeader` to `ActiveModel` for the `blocks` table
impl From<RpcBlockHeader> for ActiveModel {
    fn from(header: RpcBlockHeader) -> Self {
        Self {
            block_hash: Set(header.block_id),
            height: Set(header.block_idx),
            checkpoint_idx: NotSet,
        }
    }
}

/// Represents a block header as returned by Strata fullnode
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RpcBlockHeader {
    /// The index of the block representing height.
    pub block_idx: u64,

    /// The timestamp of when the block was created in UNIX epoch format.
    pub timestamp: u64,

    /// hash of the block's contents.
    pub block_id: String,

    /// previous block
    pub prev_block: String,

    /// L1 segment hash
    pub l1_segment_hash: String,

    /// Hash of the execution segment
    pub exec_segment_hash: String,

    /// The root hash of the state tree
    pub state_root: String,
}
