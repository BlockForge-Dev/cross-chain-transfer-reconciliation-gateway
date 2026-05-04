use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

use crate::{
    Address,
    AssetAmount,
    ChainId,
    ClientTransferReference,
    DestinationEvidence,
    DomainError,
    EvidenceSource,
    ExceptionClassification,
    FailureClassification,
    IdempotencyKey,
    ReceiptTimelineEntry,
    ReconciliationDecision,
    ReconciliationResult,
    RelayAttempt,
    RelayAttemptOutcome,
    RelayReference,
    SourceEvidence,
    TransferId,
    TransferReceipt,
    TransferState,
    TxHash,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferIntent {
    pub id: TransferId,
    pub client_transfer_reference: ClientTransferReference,
    pub idempotency_key: IdempotencyKey,
    pub source_chain: ChainId,
    pub destination_chain: ChainId,
    pub source_address: Address,
    pub destination_recipient: Address,
    pub asset_amount: AssetAmount,
    pub state: TransferState,
    pub latest_failure: Option<FailureClassification>,
    pub latest_exception: Option<ExceptionClassification>,
    pub source_evidence: Option<SourceEvidence>,
    pub relay_attempts: Vec<RelayAttempt>,
    pub destination_evidence: Option<DestinationEvidence>,
    pub reconciliation: Option<ReconciliationResult>,
    pub timeline: Vec<ReceiptTimelineEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TransferIntent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client_transfer_reference: impl Into<String>,
        idempotency_key: impl Into<String>,
        source_chain: impl Into<String>,
        destination_chain: impl Into<String>,
        source_address: impl Into<String>,
        destination_recipient: impl Into<String>,
        asset: impl Into<String>,
        quantity: impl Into<String>,
        now: DateTime<Utc>
    ) -> Result<Self, DomainError> {
        let client_transfer_reference = client_transfer_reference.into();
        let idempotency_key = idempotency_key.into();
        let source_chain = source_chain.into();
        let destination_chain = destination_chain.into();
        let source_address = source_address.into();
        let destination_recipient = destination_recipient.into();
        let asset = asset.into();
        let quantity = quantity.into();

        if client_transfer_reference.trim().is_empty() {
            return Err(DomainError::EmptyClientTransferReference);
        }
        if idempotency_key.trim().is_empty() {
            return Err(DomainError::EmptyIdempotencyKey);
        }
        if source_chain.trim().is_empty() {
            return Err(DomainError::EmptySourceChain);
        }
        if destination_chain.trim().is_empty() {
            return Err(DomainError::EmptyDestinationChain);
        }
        if source_chain.trim() == destination_chain.trim() {
            return Err(DomainError::SameSourceAndDestinationChain);
        }
        if source_address.trim().is_empty() {
            return Err(DomainError::EmptySourceAddress);
        }
        if destination_recipient.trim().is_empty() {
            return Err(DomainError::EmptyDestinationRecipient);
        }
        if asset.trim().is_empty() {
            return Err(DomainError::EmptyAsset);
        }
        if quantity.trim().is_empty() {
            return Err(DomainError::EmptyQuantity);
        }

        Ok(Self {
            id: Uuid::new_v4(),
            client_transfer_reference: ClientTransferReference(client_transfer_reference),
            idempotency_key: IdempotencyKey(idempotency_key),
            source_chain: ChainId(source_chain),
            destination_chain: ChainId(destination_chain),
            source_address: Address(source_address),
            destination_recipient: Address(destination_recipient),
            asset_amount: AssetAmount::new(quantity, asset),
            state: TransferState::Received,
            latest_failure: None,
            latest_exception: None,
            source_evidence: None,
            relay_attempts: Vec::new(),
            destination_evidence: None,
            reconciliation: None,
            timeline: vec![ReceiptTimelineEntry {
                state: TransferState::Received,
                at: now,
                note: Some("transfer intent durably accepted".to_string()),
            }],
            created_at: now,
            updated_at: now,
        })
    }

    pub fn validate(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition_to(
            TransferState::Validated,
            now,
            Some("transfer intent validated".to_string())
        )
    }

    pub fn reject(&mut self, now: DateTime<Utc>, reason: String) -> Result<(), DomainError> {
        self.latest_failure = Some(FailureClassification::Validation);
        self.transition_to(TransferState::Rejected, now, Some(reason))
    }

    pub fn queue(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition_to(
            TransferState::Queued,
            now,
            Some("transfer queued for source tracking / relay flow".to_string())
        )
    }

    pub fn record_source_observed(
        &mut self,
        source_tx_hash: impl Into<String>,
        now: DateTime<Utc>,
        note: Option<String>
    ) -> Result<(), DomainError> {
        let source_tx_hash = source_tx_hash.into();
        if source_tx_hash.trim().is_empty() {
            return Err(DomainError::EmptyTransactionHash);
        }

        self.source_evidence = Some(
            SourceEvidence::observed(TxHash(source_tx_hash), now, note.clone())
        );

        self.transition_to(TransferState::SourceObserved, now, note)
    }

    pub fn confirm_source(
        &mut self,
        source_tx_hash: impl Into<String>,
        now: DateTime<Utc>,
        note: Option<String>
    ) -> Result<(), DomainError> {
        let source_tx_hash = source_tx_hash.into();
        if source_tx_hash.trim().is_empty() {
            return Err(DomainError::EmptyTransactionHash);
        }

        let tx_hash = TxHash(source_tx_hash);

        self.source_evidence = Some(SourceEvidence::confirmed(tx_hash, now, now, note.clone()));

        self.latest_failure = None;
        self.latest_exception = None;

        self.transition_to(TransferState::SourceConfirmed, now, note)
    }

    pub fn begin_relay_attempt(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.state.is_terminal() {
            return Err(DomainError::TerminalStateNotRelayable(self.state));
        }

        if !self.state.can_begin_relay() {
            return Err(DomainError::SourceEvidenceRequiredBeforeRelay);
        }

        self.transition_to(
            TransferState::RelayInProgress,
            now,
            Some("relay attempt started".to_string())
        )?;

        let next_attempt_no = (self.relay_attempts.len() as u32) + 1;
        self.relay_attempts.push(RelayAttempt::started(next_attempt_no, now));
        Ok(())
    }

    pub fn finish_current_relay_attempt(
        &mut self,
        now: DateTime<Utc>,
        outcome: RelayAttemptOutcome,
        relay_reference: Option<String>,
        note: Option<String>
    ) -> Result<(), DomainError> {
        let last = self.relay_attempts.pop().ok_or(DomainError::InvalidAttemptNumber)?;

        let relay_reference = match relay_reference {
            Some(value) => {
                if value.trim().is_empty() {
                    return Err(DomainError::EmptyRelayReference);
                }
                Some(RelayReference(value))
            }
            None => None,
        };

        let updated_attempt = last.finish(now, outcome.clone(), relay_reference, note.clone());
        self.relay_attempts.push(updated_attempt);

        match outcome {
            RelayAttemptOutcome::Accepted => {
                self.latest_failure = None;
                self.latest_exception = None;
                self.transition_to(
                    TransferState::DestinationPending,
                    now,
                    note.or_else(||
                        Some("relay accepted; awaiting destination evidence".to_string())
                    )
                )?;
            }
            RelayAttemptOutcome::RetryableFailure { classification, .. } => {
                self.latest_failure = Some(classification);
                self.transition_to(
                    TransferState::SourceConfirmed,
                    now,
                    note.or_else(||
                        Some("retryable relay failure; transfer remains relayable".to_string())
                    )
                )?;
            }
            RelayAttemptOutcome::TerminalFailure { classification, .. } => {
                self.latest_failure = Some(classification);
                self.transition_to(
                    TransferState::FailedTerminal,
                    now,
                    note.or_else(|| Some("terminal relay failure".to_string()))
                )?;
            }
            RelayAttemptOutcome::UnknownOutcome { classification, .. } => {
                self.latest_failure = Some(classification);
                self.latest_exception = Some(ExceptionClassification::AmbiguousRelayOutcome);
                self.transition_to(
                    TransferState::RelayUnknown,
                    now,
                    note.or_else(|| Some("relay outcome is ambiguous".to_string()))
                )?;
            }
        }

        Ok(())
    }

    pub fn record_destination_evidence(
        &mut self,
        evidence: DestinationEvidence,
        now: DateTime<Utc>
    ) -> Result<(), DomainError> {
        if self.destination_matches_intent(&evidence) {
            self.destination_evidence = Some(evidence);
            self.latest_exception = None;
            self.updated_at = now;

            // Recording matching destination evidence is not always a state transition.
            // If we are already waiting on destination-side confirmation / reconciliation,
            // we should preserve the current state and let reconciliation decide final settlement.
            if self.state != TransferState::DestinationPending {
                self.transition_to(
                    TransferState::DestinationPending,
                    now,
                    Some("destination evidence recorded; awaiting reconciliation".to_string())
                )?;
            }
        } else {
            self.destination_evidence = Some(evidence);
            self.latest_failure = Some(FailureClassification::DestinationMismatch);
            self.latest_exception = Some(ExceptionClassification::DestinationMismatch);

            self.transition_to(
                TransferState::MismatchDetected,
                now,
                Some("destination evidence does not match intended transfer".to_string())
            )?;
        }

        Ok(())
    }

    pub fn begin_reconciliation(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if !self.state.needs_reconciliation() {
            return Err(DomainError::InvalidStateTransition {
                from: self.state,
                to: TransferState::Reconciling,
            });
        }

        self.transition_to(
            TransferState::Reconciling,
            now,
            Some("reconciliation started".to_string())
        )
    }

    pub fn apply_reconciliation(
        &mut self,
        result: ReconciliationResult,
        now: DateTime<Utc>
    ) -> Result<(), DomainError> {
        self.reconciliation = Some(result.clone());

        match result.decision {
            ReconciliationDecision::ConfirmSettled => {
                self.latest_failure = None;
                self.latest_exception = None;
                self.transition_to(
                    TransferState::Settled,
                    now,
                    Some("reconciliation confirmed settlement".to_string())
                )?;
            }
            ReconciliationDecision::KeepPending => {
                self.latest_failure = Some(FailureClassification::DestinationEvidenceMissing);
                self.transition_to(
                    TransferState::DestinationPending,
                    now,
                    Some("reconciliation kept transfer pending".to_string())
                )?;
            }
            ReconciliationDecision::MarkMismatch => {
                self.latest_failure = Some(FailureClassification::ReconciliationMismatch);
                self.latest_exception = Some(ExceptionClassification::DestinationMismatch);
                self.transition_to(
                    TransferState::MismatchDetected,
                    now,
                    Some("reconciliation marked transfer as mismatch".to_string())
                )?;
            }
            ReconciliationDecision::EscalateManualReview => {
                self.latest_exception = Some(ExceptionClassification::ManualReviewRequired);
                self.transition_to(
                    TransferState::ManualReview,
                    now,
                    Some("reconciliation escalated transfer to manual review".to_string())
                )?;
            }
        }

        Ok(())
    }

    pub fn resolve_relay_unknown_with_evidence(
        &mut self,
        now: DateTime<Utc>,
        to_state: TransferState,
        evidence: EvidenceSource,
        note: Option<String>
    ) -> Result<(), DomainError> {
        if self.state != TransferState::RelayUnknown {
            return Err(DomainError::RelayUnknownResolutionRequiresEvidence);
        }

        match evidence {
            | EvidenceSource::SourceChainObservation { .. }
            | EvidenceSource::DestinationChainObservation { .. }
            | EvidenceSource::RelayStatusCheck { .. }
            | EvidenceSource::ManualOperatorDecision { .. } => {}
            EvidenceSource::InternalValidation => {
                return Err(DomainError::RelayUnknownResolutionRequiresEvidence);
            }
        }

        match to_state {
            TransferState::DestinationPending => {
                self.latest_exception = None;
                self.transition_to(to_state, now, note)?;
            }
            TransferState::Settled => {
                self.latest_failure = None;
                self.latest_exception = None;
                self.transition_to(to_state, now, note)?;
            }
            TransferState::MismatchDetected => {
                self.latest_failure = Some(FailureClassification::DestinationMismatch);
                self.latest_exception = Some(ExceptionClassification::DestinationMismatch);
                self.transition_to(to_state, now, note)?;
            }
            TransferState::ManualReview => {
                self.latest_exception = Some(ExceptionClassification::ManualReviewRequired);
                self.transition_to(to_state, now, note)?;
            }
            _ => {
                return Err(DomainError::InvalidStateTransition {
                    from: self.state,
                    to: to_state,
                });
            }
        }

        Ok(())
    }

    pub fn to_receipt(&self) -> TransferReceipt {
        TransferReceipt {
            transfer_id: self.id,
            client_transfer_reference: self.client_transfer_reference.clone(),
            idempotency_key: self.idempotency_key.clone(),
            current_state: self.state,
            latest_failure: self.latest_failure.clone(),
            latest_exception: self.latest_exception.clone(),
            source_evidence: self.source_evidence.clone(),
            relay_attempts: self.relay_attempts.clone(),
            destination_evidence: self.destination_evidence.clone(),
            reconciliation: self.reconciliation.clone(),
            timeline: self.timeline.clone(),
        }
    }

    fn destination_matches_intent(&self, evidence: &DestinationEvidence) -> bool {
        evidence.destination_chain == self.destination_chain &&
            evidence.recipient == self.destination_recipient &&
            evidence.asset == self.asset_amount.asset &&
            evidence.quantity == self.asset_amount.quantity
    }

    fn transition_to(
        &mut self,
        to: TransferState,
        now: DateTime<Utc>,
        note: Option<String>
    ) -> Result<(), DomainError> {
        if !Self::is_valid_transition(self.state, to) {
            return Err(DomainError::InvalidStateTransition {
                from: self.state,
                to,
            });
        }

        self.state = to;
        self.updated_at = now;
        self.timeline.push(ReceiptTimelineEntry { state: to, at: now, note });
        Ok(())
    }

    fn is_valid_transition(from: TransferState, to: TransferState) -> bool {
        use TransferState::*;

        match (from, to) {
            (Received, Validated) => true,
            (Received, Rejected) => true,

            (Validated, Queued) => true,
            (Validated, Rejected) => true,

            (Queued, SourceObserved) => true,
            (Queued, SourceConfirmed) => true,
            (Queued, Rejected) => true,

            (SourceObserved, SourceConfirmed) => true,

            (SourceConfirmed, RelayInProgress) => true,
            (SourceConfirmed, DestinationPending) => true,
            (DestinationPending, Reconciling) => true,
            (DestinationPending, MismatchDetected) => true,
            (DestinationPending, ManualReview) => true,

            (RelayInProgress, SourceConfirmed) => true,
            (RelayInProgress, DestinationPending) => true,
            (RelayInProgress, RelayUnknown) => true,
            (RelayInProgress, FailedTerminal) => true,

            (RelayUnknown, DestinationPending) => true,
            (RelayUnknown, Settled) => true,
            (RelayUnknown, MismatchDetected) => true,
            (RelayUnknown, Reconciling) => true,
            (RelayUnknown, ManualReview) => true,

            (MismatchDetected, Reconciling) => true,
            (MismatchDetected, ManualReview) => true,

            (Reconciling, Settled) => true,
            (Reconciling, DestinationPending) => true,
            (Reconciling, MismatchDetected) => true,
            (Reconciling, ManualReview) => true,

            (ManualReview, Reconciling) => true,
            (ManualReview, Settled) => true,
            (ManualReview, FailedTerminal) => true,

            (_, DeadLettered) if !from.is_terminal() => true,

            _ => false,
        }
    }
}
