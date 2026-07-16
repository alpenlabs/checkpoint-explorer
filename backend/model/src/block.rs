use crate::utils::DisplayHashHex;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use strata_identifiers::{Buf32, L2BlockId};

// TODO(STR-3792): Rename block model/table abstractions to L2 block-header terminology.
/// Represents the Block model for the database
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blocks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
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
            block_hash: Set(header.blkid.to_display_hex()),
            height: Set(header.slot),
            checkpoint_idx: Set(u64::from(header.epoch)),
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
    pub state_root: Buf32,
    pub body_root: Buf32,
    pub logs_root: Buf32,
    pub is_terminal: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ActiveValue::Set;

    fn ascending_hex32() -> String {
        (0u8..32).map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn active_model_preserves_l2_block_id_byte_order() {
        let blkid = ascending_hex32();
        let header: RpcBlockHeader = serde_json::from_value(serde_json::json!({
            "slot": 42,
            "epoch": 7,
            "blkid": blkid,
            "timestamp": 1_700_000_000u64,
            "parent_blkid": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "state_root": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "body_root": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "logs_root": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "is_terminal": true,
        }))
        .expect("test block header should deserialize");

        let active_model: ActiveModel = header.into();

        assert_eq!(active_model.block_hash, Set(blkid));
        assert_eq!(active_model.checkpoint_idx, Set(7));
    }
}
