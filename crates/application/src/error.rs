use domain::DomainError;
use persistence::PersistenceError;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("validation error: {0}")] Validation(String),

    #[error("unsupported chain: {0}")] UnsupportedChain(String),

    #[error("idempotency conflict for scope={scope} key={key}")] IdempotencyConflict {
        scope: String,
        key: String,
    },

    #[error("transfer intent not found: {0}")] TransferNotFound(Uuid),

    #[error(transparent)] Domain(#[from] DomainError),

    #[error(transparent)] Persistence(PersistenceError),
}

impl From<PersistenceError> for ApplicationError {
    fn from(value: PersistenceError) -> Self {
        match value {
            PersistenceError::TransferNotFound(id) => Self::TransferNotFound(id),
            PersistenceError::IdempotencyConflict { scope, key } => {
                Self::IdempotencyConflict { scope, key }
            }
            other => Self::Persistence(other),
        }
    }
}
