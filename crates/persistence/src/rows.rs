use chrono::{ DateTime, Utc };
use serde_json::Value;
use sqlx::types::Json;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct DbTransferIntentRow {
    pub id: Uuid,
    pub client_transfer_reference: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub source_address: String,
    pub destination_recipient: String,
    pub asset_id: String,
    pub quantity: String,
    pub state: String,
    pub latest_failure_classification: Option<String>,
    pub latest_exception_classification: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbIdempotencyKeyRow {
    pub scope: String,
    pub idempotency_key: String,
    pub transfer_id: Uuid,
    pub request_fingerprint: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbSourceEvidenceRow {
    pub transfer_id: Uuid,
    pub source_tx_hash: String,
    pub observed_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbRelayAttemptRow {
    pub transfer_id: Uuid,
    pub attempt_no: i32,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub outcome_kind: Option<String>,
    pub error_category: Option<String>,
    pub result_reason: Option<String>,
    pub relay_reference: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbDestinationEvidenceRow {
    pub transfer_id: Uuid,
    pub destination_tx_hash: String,
    pub destination_chain: String,
    pub recipient: String,
    pub asset: String,
    pub quantity: String,
    pub observed_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbReconciliationRunRow {
    pub transfer_id: Uuid,
    pub compared_at: DateTime<Utc>,
    pub internal_state: String,
    pub source_status: String,
    pub relay_status: String,
    pub destination_status: String,
    pub comparison_result: String,
    pub decision: String,
    pub evidence: Json<Value>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DbExceptionCaseRow {
    pub id: i64,
    pub transfer_id: Uuid,
    pub exception_classification: String,
    pub case_status: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, FromRow)]
pub struct DbAuditEventRow {
    pub transfer_id: Option<Uuid>,
    pub event_type: String,
    pub payload: Json<Value>,
    pub created_at: DateTime<Utc>,
}
