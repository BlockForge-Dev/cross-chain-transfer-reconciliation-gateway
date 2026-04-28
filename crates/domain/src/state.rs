use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferState {
    Received,
    Validated,
    Rejected,
    Queued,
    SourceObserved,
    SourceConfirmed,
    RelayInProgress,
    RelayUnknown,
    DestinationPending,
    Settled,
    MismatchDetected,
    Reconciling,
    ManualReview,
    FailedTerminal,
    DeadLettered,
}

impl TransferState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Settled | Self::FailedTerminal | Self::DeadLettered)
    }

    pub fn can_begin_relay(self) -> bool {
        matches!(self, Self::SourceConfirmed)
    }

    pub fn needs_reconciliation(self) -> bool {
        matches!(
            self,
            Self::SourceConfirmed |
                Self::RelayUnknown |
                Self::DestinationPending |
                Self::MismatchDetected |
                Self::ManualReview
        )
    }
}
