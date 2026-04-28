use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

use crate::{ Address, AssetId, ChainId, TxHash };

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestinationEvidence {
    pub destination_tx_hash: TxHash,
    pub destination_chain: ChainId,
    pub recipient: Address,
    pub asset: AssetId,
    pub quantity: String,
    pub observed_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

impl DestinationEvidence {
    pub fn observed(
        destination_tx_hash: TxHash,
        destination_chain: ChainId,
        recipient: Address,
        asset: AssetId,
        quantity: String,
        observed_at: DateTime<Utc>,
        note: Option<String>
    ) -> Self {
        Self {
            destination_tx_hash,
            destination_chain,
            recipient,
            asset,
            quantity,
            observed_at,
            confirmed_at: None,
            note,
        }
    }

    pub fn confirmed(
        destination_tx_hash: TxHash,
        destination_chain: ChainId,
        recipient: Address,
        asset: AssetId,
        quantity: String,
        observed_at: DateTime<Utc>,
        confirmed_at: DateTime<Utc>,
        note: Option<String>
    ) -> Self {
        Self {
            destination_tx_hash,
            destination_chain,
            recipient,
            asset,
            quantity,
            observed_at,
            confirmed_at: Some(confirmed_at),
            note,
        }
    }
}
