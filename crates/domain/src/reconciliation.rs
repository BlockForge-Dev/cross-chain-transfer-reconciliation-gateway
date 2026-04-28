use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

use crate::{ EvidenceSource, TransferState };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationComparison {
    Matched,
    Mismatch,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationDecision {
    ConfirmSettled,
    KeepPending,
    MarkMismatch,
    EscalateManualReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationResult {
    pub compared_at: DateTime<Utc>,
    pub internal_state: TransferState,
    pub source_status: String,
    pub relay_status: String,
    pub destination_status: String,
    pub comparison: ReconciliationComparison,
    pub decision: ReconciliationDecision,
    pub evidence: EvidenceSource,
    pub note: Option<String>,
}
