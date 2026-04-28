use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureClassification {
    Validation,
    DuplicateRequest,
    RetryableRelayInfrastructure,
    TerminalRelayFailure,
    UnknownRelayOutcome,
    SourceEvidenceMissing,
    DestinationEvidenceMissing,
    DestinationMismatch,
    ReconciliationMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExceptionClassification {
    DestinationMissing,
    DestinationMismatch,
    AmbiguousRelayOutcome,
    DuplicateRelayAttempt,
    StalePendingTransfer,
    SourceMissing,
    ManualReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayAttemptOutcome {
    Accepted,
    RetryableFailure {
        classification: FailureClassification,
        reason: String,
    },
    TerminalFailure {
        classification: FailureClassification,
        reason: String,
    },
    UnknownOutcome {
        classification: FailureClassification,
        reason: String,
    },
}
