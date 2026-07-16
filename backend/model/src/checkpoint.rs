use crate::utils::DisplayHashHex;
use anyhow::Error;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::result::Result;
use std::str::FromStr;
use strata_identifiers::{EpochCommitment, L1BlockCommitment, L2BlockCommitment, RBuf32};

pub type Txid = String;

/// The L2 slot number up to which the block fetcher should fetch blocks.
/// Sent over the watch channel from the checkpoint fetcher to the block fetcher.
pub type L2BlockFetchTarget = u64;

/// Represents the checkpoint information returned by the RPC.
/// Name for this struct comes from the Strata RPC endpoint.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RpcCheckpointInfo {
    /// The index of the checkpoint
    pub idx: u64,

    /// The L1 height range that the checkpoint covers (start, end)
    pub l1_range: (L1BlockCommitment, L1BlockCommitment),

    /// The first L2 block that the checkpoint covers, if known.
    pub l2_start: Option<L2BlockCommitment>,

    /// The last L2 block that the checkpoint covers.
    pub l2_end: L2BlockCommitment,

    confirmation_status: ConfStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCheckpointL1Ref {
    pub l1_block: L1BlockCommitment,
    pub txid: RBuf32,
    pub wtxid: RBuf32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ConfStatus {
    Pending,
    Confirmed { l1_reference: RpcCheckpointL1Ref },
    Finalized { l1_reference: RpcCheckpointL1Ref },
}

/// Wire type for strata_getChainStatus (only fields we use).
#[derive(Debug, Deserialize)]
pub struct RpcOLChainStatus {
    pub latest: EpochCommitment,
    pub confirmed: EpochCommitment,
    pub finalized: EpochCommitment,
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
    pub l2_start: Option<u64>,
    pub l2_end: u64,
    pub checkpoint_txid: Option<Txid>,
    pub status: RpcCheckpointConfStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl RpcCheckpointInfo {
    pub fn status(&self) -> RpcCheckpointConfStatus {
        match &self.confirmation_status {
            ConfStatus::Pending => RpcCheckpointConfStatus::Pending,
            ConfStatus::Confirmed { .. } => RpcCheckpointConfStatus::Confirmed,
            ConfStatus::Finalized { .. } => RpcCheckpointConfStatus::Finalized,
        }
    }

    pub fn checkpoint_txid(&self) -> Option<Txid> {
        match &self.confirmation_status {
            ConfStatus::Pending => None,
            ConfStatus::Confirmed { l1_reference } | ConfStatus::Finalized { l1_reference } => {
                Some(l1_reference.txid.to_display_hex())
            }
        }
    }
}

impl From<RpcCheckpointInfo> for ActiveModel {
    fn from(info: RpcCheckpointInfo) -> Self {
        let (status, txid) = match &info.confirmation_status {
            ConfStatus::Pending => (RpcCheckpointConfStatus::Pending, None),
            ConfStatus::Confirmed { l1_reference } => (
                RpcCheckpointConfStatus::Confirmed,
                Some(l1_reference.txid.to_display_hex()),
            ),
            ConfStatus::Finalized { l1_reference } => (
                RpcCheckpointConfStatus::Finalized,
                Some(l1_reference.txid.to_display_hex()),
            ),
        };
        Self {
            idx: Set(info.idx),
            l1_start: Set(u64::from(info.l1_range.0.height)),
            l1_end: Set(u64::from(info.l1_range.1.height)),
            l2_start: Set(info.l2_start.map(|commitment| commitment.slot)),
            l2_end: Set(info.l2_end.slot),
            checkpoint_txid: Set(txid),
            status: Set(status),
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

    /// The first L2 block that the checkpoint covers, if known.
    pub l2_start: Option<u64>,

    /// The last L2 block that the checkpoint covers.
    pub l2_end: u64,

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
            l2_start: model.l2_start,
            l2_end: model.l2_end,
            l1_reference: model.checkpoint_txid.map(|txid| ExplorerL1Ref { txid }),
            confirmation_status: Some(model.status),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ActiveValue::Set;

    fn hex32(byte: u8) -> String {
        format!("{byte:02x}").repeat(32)
    }

    fn ascending_hex32() -> String {
        (0u8..32).map(|byte| format!("{byte:02x}")).collect()
    }

    fn descending_hex32() -> String {
        (0u8..32).rev().map(|byte| format!("{byte:02x}")).collect()
    }

    fn checkpoint_json(status: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "idx": 7,
            "l1_range": [
                {"height": 70, "blkid": hex32(1)},
                {"height": 79, "blkid": hex32(2)}
            ],
            "l2_start": {"slot": 700, "blkid": hex32(3)},
            "l2_end": {"slot": 799, "blkid": hex32(4)},
            "confirmation_status": status
        })
    }

    fn confirmed_status(txid: &str) -> serde_json::Value {
        serde_json::json!({
            "status": "confirmed",
            "l1_reference": {
                "l1_block": {"height": 79, "blkid": hex32(2)},
                "txid": txid,
                "wtxid": hex32(0x33)
            }
        })
    }

    fn finalized_status(txid: &str) -> serde_json::Value {
        serde_json::json!({
            "status": "finalized",
            "l1_reference": {
                "l1_block": {"height": 79, "blkid": hex32(2)},
                "txid": txid,
                "wtxid": hex32(0x33)
            }
        })
    }

    #[test]
    fn checkpoint_txid_is_none_for_pending() {
        let checkpoint: RpcCheckpointInfo =
            serde_json::from_value(checkpoint_json(serde_json::json!({"status": "pending"})))
                .expect("pending checkpoint should deserialize");

        assert_eq!(checkpoint.status(), RpcCheckpointConfStatus::Pending);
        assert_eq!(checkpoint.checkpoint_txid(), None);
    }

    #[test]
    fn checkpoint_txid_is_exposed_for_confirmed_and_finalized() {
        let confirmed_txid = ascending_hex32();
        let finalized_txid = descending_hex32();
        let confirmed: RpcCheckpointInfo =
            serde_json::from_value(checkpoint_json(confirmed_status(&confirmed_txid)))
                .expect("confirmed checkpoint should deserialize");
        let finalized: RpcCheckpointInfo =
            serde_json::from_value(checkpoint_json(finalized_status(&finalized_txid)))
                .expect("finalized checkpoint should deserialize");

        assert_eq!(confirmed.status(), RpcCheckpointConfStatus::Confirmed);
        assert_eq!(
            confirmed.checkpoint_txid().as_deref(),
            Some(confirmed_txid.as_str())
        );
        assert_eq!(finalized.status(), RpcCheckpointConfStatus::Finalized);
        assert_eq!(
            finalized.checkpoint_txid().as_deref(),
            Some(finalized_txid.as_str())
        );
    }

    #[test]
    fn active_model_sets_checkpoint_txid_for_confirmed_checkpoint() {
        let confirmed_txid = ascending_hex32();
        let checkpoint: RpcCheckpointInfo =
            serde_json::from_value(checkpoint_json(confirmed_status(&confirmed_txid)))
                .expect("confirmed checkpoint should deserialize");

        let active_model: ActiveModel = checkpoint.into();

        assert_eq!(active_model.status, Set(RpcCheckpointConfStatus::Confirmed));
        assert_eq!(active_model.checkpoint_txid, Set(Some(confirmed_txid)));
    }

    #[test]
    fn checkpoint_l2_start_can_be_absent() {
        let mut json = checkpoint_json(serde_json::json!({"status": "pending"}));
        json["l2_start"] = serde_json::Value::Null;

        let checkpoint: RpcCheckpointInfo =
            serde_json::from_value(json).expect("checkpoint without l2_start should deserialize");

        assert_eq!(checkpoint.l2_start, None);

        let active_model: ActiveModel = checkpoint.into();
        assert_eq!(active_model.l2_start, Set(None));
        assert_eq!(active_model.l2_end, Set(799));
    }
}
