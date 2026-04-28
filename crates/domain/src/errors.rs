use crate::state::TransferState;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid state transition from {from:?} to {to:?}")] InvalidStateTransition {
        from: TransferState,
        to: TransferState,
    },

    #[error("client transfer reference cannot be empty")]
    EmptyClientTransferReference,

    #[error("idempotency key cannot be empty")]
    EmptyIdempotencyKey,

    #[error("source chain cannot be empty")]
    EmptySourceChain,

    #[error("destination chain cannot be empty")]
    EmptyDestinationChain,

    #[error("source chain and destination chain must differ")]
    SameSourceAndDestinationChain,

    #[error("source address cannot be empty")]
    EmptySourceAddress,

    #[error("destination recipient cannot be empty")]
    EmptyDestinationRecipient,

    #[error("asset cannot be empty")]
    EmptyAsset,

    #[error("quantity cannot be empty")]
    EmptyQuantity,

    #[error("transaction hash cannot be empty")]
    EmptyTransactionHash,

    #[error("relay reference cannot be empty")]
    EmptyRelayReference,

    #[error("source confirmation is required before relay can begin")]
    SourceEvidenceRequiredBeforeRelay,

    #[error("attempt number must increase monotonically")]
    InvalidAttemptNumber,

    #[error("unknown relay outcome cannot be resolved without external evidence")]
    RelayUnknownResolutionRequiresEvidence,

    #[error("terminal state cannot accept a new relay attempt: {0:?}")] TerminalStateNotRelayable(
        TransferState,
    ),
}
