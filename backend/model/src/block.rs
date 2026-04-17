use crate::checkpoint::{HexBytes32, L2BlockId};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};

/// Represents the Block model for the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blocks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub block_hash: String,
    pub height: u64,
    pub checkpoint_idx: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Implements conversion from `RpcBlockHeader` to `ActiveModel` for the `blocks` table
impl From<RpcBlockHeader> for ActiveModel {
    fn from(header: RpcBlockHeader) -> Self {
        Self {
            block_hash: Set(header.blkid),
            height: Set(header.slot),
            checkpoint_idx: Set(header.epoch),
        }
    }
}

/// Represents a block header as returned by strata_getHeadersInRange
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RpcBlockHeader {
    pub slot: u64,
    pub epoch: u32,
    pub blkid: L2BlockId,
    pub timestamp: u64,
    pub parent_blkid: L2BlockId,
    pub state_root: HexBytes32,
    pub body_root: HexBytes32,
    pub logs_root: HexBytes32,
    pub is_terminal: bool,
}
