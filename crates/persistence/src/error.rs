use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("database error: {0}")] Sqlx(#[from] sqlx::Error),

    #[error("serialization error: {0}")] Serde(#[from] serde_json::Error),

    #[error("transfer intent not found: {0}")] TransferNotFound(Uuid),

    #[error("idempotency conflict for scope={scope} key={key}")] IdempotencyConflict {
        scope: String,
        key: String,
    },

    #[error("no unresolved exception case found for transfer: {0}")] ExceptionCaseNotFound(Uuid),

    #[error("invalid persisted transfer state: {0}")] InvalidPersistedState(String),

    #[error("invalid persisted failure classification: {0}")] InvalidFailureClassification(String),

    #[error("invalid persisted exception classification: {0}")] InvalidExceptionClassification(
        String,
    ),

    #[error("invalid persisted relay attempt outcome: {0}")] InvalidRelayAttemptOutcome(String),

    #[error("invalid persisted reconciliation comparison: {0}")] InvalidReconciliationComparison(
        String,
    ),

    #[error("invalid persisted reconciliation decision: {0}")] InvalidReconciliationDecision(
        String,
    ),

    #[error("invariant violation: {0}")] InvariantViolation(String),
}
