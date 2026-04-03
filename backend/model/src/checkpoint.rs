use anyhow::Error;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::result::Result;
use std::str::FromStr;
/// Represents an L2 Block ID.
pub type L2BlockId = String;
pub type L1BlockId = String;
pub type Txid = String;

/// Represents the checkpoint information returned by the RPC.
/// Name for this struct comes from the Strata RPC endpoint.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RpcCheckpointInfo {
    /// The index of the checkpoint
    pub idx: u64,
    /// The L1 height range that the checkpoint covers (start, end)
    pub l1_range: (L1BlockCommitment, L1BlockCommitment),
    /// The L2 height range that the checkpoint covers (start, end)
    pub l2_range: (L2BlockCommitment, L2BlockCommitment),
    /// Info on txn where checkpoint is committed on chain
    pub l1_reference: Option<RpcCheckpointL1Ref>,
    /// Confirmation status of checkpoint
    pub confirmation_status: Option<RpcCheckpointConfStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct L1BlockCommitment {
    pub height: u64,
    pub blkid: L1BlockId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Buf32(pub [u8; 32]);
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcCheckpointL1Ref {
    pub block_height: u64,
    pub block_id: String,
    pub txid: String,
    pub wtxid: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct L2BlockCommitment {
    slot: u64,
    blkid: L2BlockId,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[serde(rename_all = "lowercase")]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum RpcCheckpointConfStatus {
    /// Pending to be posted on L1
    #[sea_orm(string_value = "Pending")]
    Pending,
    /// Confirmed on L1
    #[sea_orm(string_value = "Confirmed")]
    Confirmed,
    /// Finalized on L1
    #[sea_orm(string_value = "Finalized")]
    Finalized,
}

impl FromStr for RpcCheckpointConfStatus {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(RpcCheckpointConfStatus::Pending),
            "confirmed" => Ok(RpcCheckpointConfStatus::Confirmed),
            "finalized" => Ok(RpcCheckpointConfStatus::Finalized),
            _ => Err(Error::msg(format!("Invalid status: {s}"))),
        }
    }
}

impl Display for RpcCheckpointConfStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let status_str = match self {
            RpcCheckpointConfStatus::Pending => "pending",
            RpcCheckpointConfStatus::Confirmed => "confirmed",
            RpcCheckpointConfStatus::Finalized => "finalized",
        };
        write!(f, "{status_str}")
    }
}

#[derive(
    Clone, Debug, PartialEq, DeriveEntityModel, DeriveActiveModelBehavior, Serialize, Deserialize,
)]
#[sea_orm(table_name = "checkpoints")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub idx: u64,
    pub l1_start: u64,
    pub l1_end: u64,
    pub l2_start: u64,
    pub l2_end: u64,
    pub checkpoint_txid: Option<Txid>,
    pub status: RpcCheckpointConfStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl From<RpcCheckpointInfo> for ActiveModel {
    fn from(info: RpcCheckpointInfo) -> Self {
        Self {
            idx: Set(info.idx),
            l1_start: Set(info.l1_range.0.height),
            l1_end: Set(info.l1_range.1.height),
            l2_start: Set(info.l2_range.0.slot),
            l2_end: Set(info.l2_range.1.slot),
            checkpoint_txid: Set(info.l1_reference.as_ref().map(|c| c.txid.clone())),
            status: Set(info
                .confirmation_status
                .unwrap_or(RpcCheckpointConfStatus::Pending)),
        }
    }
}

/// Minimal L1 reference for the explorer response — only the txid is stored in the DB.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExplorerL1Ref {
    pub txid: Txid,
}

/// Represents the checkpoint information returned by the RPC to the frontend.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RpcCheckpointInfoCheckpointExp {
    /// The index of the checkpoint
    pub idx: u64,
    /// The L1 height range that the checkpoint covers (start, end)
    pub l1_range: (u64, u64),
    /// The L2 height range that the checkpoint covers (start, end)
    pub l2_range: (u64, u64),
    /// Txid of the L1 transaction where the checkpoint was committed (None if not yet committed)
    pub l1_reference: Option<ExplorerL1Ref>,
    /// Confirmation status of checkpoint
    pub confirmation_status: Option<RpcCheckpointConfStatus>,
}

impl From<Model> for RpcCheckpointInfoCheckpointExp {
    fn from(model: Model) -> Self {
        Self {
            idx: model.idx,
            l1_range: (model.l1_start, model.l1_end),
            l2_range: (model.l2_start, model.l2_end),
            l1_reference: model.checkpoint_txid.map(|txid| ExplorerL1Ref { txid }),
            confirmation_status: Some(model.status),
        }
    }
}
