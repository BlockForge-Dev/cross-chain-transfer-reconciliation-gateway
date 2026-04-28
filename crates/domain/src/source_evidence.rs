use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

use crate::TxHash;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEvidence {
    pub source_tx_hash: TxHash,
    pub observed_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

impl SourceEvidence {
    pub fn observed(
        source_tx_hash: TxHash,
        observed_at: DateTime<Utc>,
        note: Option<String>
    ) -> Self {
        Self {
            source_tx_hash,
            observed_at,
            confirmed_at: None,
            note,
        }
    }

    pub fn confirmed(
        source_tx_hash: TxHash,
        observed_at: DateTime<Utc>,
        confirmed_at: DateTime<Utc>,
        note: Option<String>
    ) -> Self {
        Self {
            source_tx_hash,
            observed_at,
            confirmed_at: Some(confirmed_at),
            note,
        }
    }
}
