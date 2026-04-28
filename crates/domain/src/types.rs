use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

pub type TransferId = Uuid;
pub type AttemptNumber = u32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientTransferReference(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxHash(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayReference(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetAmount {
    pub quantity: String,
    pub asset: AssetId,
}

impl AssetAmount {
    pub fn new(quantity: impl Into<String>, asset: impl Into<String>) -> Self {
        Self {
            quantity: quantity.into(),
            asset: AssetId(asset.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceSource {
    SourceChainObservation {
        tx_hash: String,
    },
    DestinationChainObservation {
        tx_hash: String,
    },
    RelayStatusCheck {
        checked_at: DateTime<Utc>,
    },
    ManualOperatorDecision {
        operator_id: String,
        note: String,
    },
    InternalValidation,
}
