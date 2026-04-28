use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

use crate::{
    ClientTransferReference,
    DestinationEvidence,
    ExceptionClassification,
    FailureClassification,
    IdempotencyKey,
    RelayAttempt,
    ReconciliationResult,
    SourceEvidence,
    TransferId,
    TransferState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptTimelineEntry {
    pub state: TransferState,
    pub at: DateTime<Utc>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferReceipt {
    pub transfer_id: TransferId,
    pub client_transfer_reference: ClientTransferReference,
    pub idempotency_key: IdempotencyKey,
    pub current_state: TransferState,
    pub latest_failure: Option<FailureClassification>,
    pub latest_exception: Option<ExceptionClassification>,
    pub source_evidence: Option<SourceEvidence>,
    pub relay_attempts: Vec<RelayAttempt>,
    pub destination_evidence: Option<DestinationEvidence>,
    pub reconciliation: Option<ReconciliationResult>,
    pub timeline: Vec<ReceiptTimelineEntry>,
}
