use async_trait::async_trait;
use chrono::{ DateTime, Utc };
use domain::{
    EvidenceSource,
    ReconciliationComparison,
    ReconciliationDecision,
    ReconciliationResult,
    RelayAttemptOutcome,
    TransferId,
    TransferIntent,
    TransferState,
};
use persistence::{ PersistenceError, PostgresPersistence };

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct ReconcileTransferCommand {
    pub transfer_id: TransferId,
    pub note: Option<String>,
    pub reconciled_at: DateTime<Utc>,
}

#[async_trait]
pub trait ReconciliationRepo: Clone + Send + Sync + 'static {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError>;

    async fn save_reconciliation_run(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError>;
}

#[async_trait]
impl ReconciliationRepo for PostgresPersistence {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError> {
        PostgresPersistence::get_transfer_by_id(self, transfer_id).await
    }

    async fn save_reconciliation_run(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        PostgresPersistence::save_reconciliation_run(self, transfer).await
    }
}

#[derive(Debug, Clone)]
pub struct ReconciliationService<R> where R: ReconciliationRepo {
    repo: R,
}

impl<R> ReconciliationService<R> where R: ReconciliationRepo {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn reconcile(
        &self,
        command: ReconcileTransferCommand
    ) -> Result<TransferIntent, ApplicationError> {
        let mut transfer = self.repo.get_transfer_by_id(command.transfer_id).await?;

        let pre_reconciliation_state = transfer.state;
        transfer.begin_reconciliation(command.reconciled_at)?;

        let source_status = source_status(&transfer);
        let relay_status = relay_status(&transfer);
        let destination_status = destination_status(&transfer);

        let result = build_reconciliation_result(
            &transfer,
            pre_reconciliation_state,
            source_status,
            relay_status,
            destination_status,
            command.reconciled_at,
            command.note
        );

        transfer.apply_reconciliation(result, command.reconciled_at)?;
        self.repo.save_reconciliation_run(&transfer).await?;

        Ok(transfer)
    }
}

fn build_reconciliation_result(
    transfer: &TransferIntent,
    internal_state: TransferState,
    source_status: String,
    relay_status: String,
    destination_status: String,
    compared_at: DateTime<Utc>,
    note: Option<String>
) -> ReconciliationResult {
    let (comparison, decision, evidence, default_note) = if
        has_matching_destination_evidence(transfer) &&
        has_source_confirmation(transfer)
    {
        (
            ReconciliationComparison::Matched,
            ReconciliationDecision::ConfirmSettled,
            destination_evidence_source(transfer),
            "reconciliation confirmed matching destination settlement".to_string(),
        )
    } else if
        transfer.state == TransferState::MismatchDetected ||
        has_mismatching_destination_evidence(transfer)
    {
        (
            ReconciliationComparison::Mismatch,
            ReconciliationDecision::MarkMismatch,
            destination_evidence_source(transfer),
            "reconciliation detected destination mismatch".to_string(),
        )
    } else if internal_state == TransferState::RelayUnknown {
        (
            ReconciliationComparison::Unresolved,
            ReconciliationDecision::EscalateManualReview,
            EvidenceSource::RelayStatusCheck { checked_at: compared_at },
            "relay outcome remains ambiguous; escalating to manual review".to_string(),
        )
    } else {
        (
            ReconciliationComparison::Unresolved,
            ReconciliationDecision::KeepPending,
            fallback_reconciliation_evidence(transfer, compared_at),
            "reconciliation kept transfer pending".to_string(),
        )
    };

    ReconciliationResult {
        compared_at,
        internal_state,
        source_status,
        relay_status,
        destination_status,
        comparison,
        decision,
        evidence,
        note: note.or(Some(default_note)),
    }
}

fn has_source_confirmation(transfer: &TransferIntent) -> bool {
    transfer.source_evidence
        .as_ref()
        .and_then(|e| e.confirmed_at)
        .is_some()
}

fn has_matching_destination_evidence(transfer: &TransferIntent) -> bool {
    let Some(destination) = &transfer.destination_evidence else {
        return false;
    };

    destination.destination_chain == transfer.destination_chain &&
        destination.recipient == transfer.destination_recipient &&
        destination.asset == transfer.asset_amount.asset &&
        destination.quantity == transfer.asset_amount.quantity
}

fn has_mismatching_destination_evidence(transfer: &TransferIntent) -> bool {
    transfer.destination_evidence.is_some() && !has_matching_destination_evidence(transfer)
}

fn source_status(transfer: &TransferIntent) -> String {
    match &transfer.source_evidence {
        None => "missing".to_string(),
        Some(evidence) if evidence.confirmed_at.is_some() => "confirmed".to_string(),
        Some(_) => "observed".to_string(),
    }
}

fn relay_status(transfer: &TransferIntent) -> String {
    match transfer.relay_attempts.last() {
        None => "not_started".to_string(),
        Some(attempt) =>
            match &attempt.outcome {
                None => "in_progress".to_string(),
                Some(RelayAttemptOutcome::Accepted) => "accepted".to_string(),
                Some(RelayAttemptOutcome::RetryableFailure { .. }) =>
                    "retryable_failure".to_string(),
                Some(RelayAttemptOutcome::TerminalFailure { .. }) => "terminal_failure".to_string(),
                Some(RelayAttemptOutcome::UnknownOutcome { .. }) => "unknown_outcome".to_string(),
            }
    }
}

fn destination_status(transfer: &TransferIntent) -> String {
    match &transfer.destination_evidence {
        None => "missing".to_string(),
        Some(evidence) if evidence.confirmed_at.is_some() => "confirmed".to_string(),
        Some(_) => "observed".to_string(),
    }
}

fn destination_evidence_source(transfer: &TransferIntent) -> EvidenceSource {
    match &transfer.destination_evidence {
        Some(evidence) =>
            EvidenceSource::DestinationChainObservation {
                tx_hash: evidence.destination_tx_hash.0.clone(),
            },
        None =>
            EvidenceSource::RelayStatusCheck {
                checked_at: Utc::now(),
            },
    }
}

fn fallback_reconciliation_evidence(
    transfer: &TransferIntent,
    compared_at: DateTime<Utc>
) -> EvidenceSource {
    if let Some(evidence) = &transfer.destination_evidence {
        return EvidenceSource::DestinationChainObservation {
            tx_hash: evidence.destination_tx_hash.0.clone(),
        };
    }

    if let Some(evidence) = &transfer.source_evidence {
        return EvidenceSource::SourceChainObservation {
            tx_hash: evidence.source_tx_hash.0.clone(),
        };
    }

    EvidenceSource::RelayStatusCheck { checked_at: compared_at }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use domain::{
        DestinationEvidence,
        RelayAttemptOutcome,
        TransferState,
        TxHash,
        ChainId,
        Address,
        AssetId,
    };
    use std::collections::HashMap;
    use std::sync::{ Arc, Mutex };

    #[derive(Debug, Clone, Default)]
    struct FakeRepo {
        transfers: Arc<Mutex<HashMap<TransferId, TransferIntent>>>,
    }

    #[async_trait]
    impl ReconciliationRepo for FakeRepo {
        async fn get_transfer_by_id(
            &self,
            transfer_id: TransferId
        ) -> Result<TransferIntent, PersistenceError> {
            let transfers = self.transfers.lock().unwrap();
            transfers
                .get(&transfer_id)
                .cloned()
                .ok_or(PersistenceError::TransferNotFound(transfer_id))
        }

        async fn save_reconciliation_run(
            &self,
            transfer: &TransferIntent
        ) -> Result<(), PersistenceError> {
            let mut transfers = self.transfers.lock().unwrap();
            transfers.insert(transfer.id, transfer.clone());
            Ok(())
        }
    }

    fn destination_pending_transfer_with_matching_evidence() -> TransferIntent {
        let now = Utc::now();
        let mut transfer = TransferIntent::new(
            "transfer_123",
            "idem_123",
            "ethereum",
            "solana",
            "0xabc123",
            "So1Recipient111",
            "USDC",
            "1000000",
            now
        ).unwrap();

        transfer.validate(now).unwrap();
        transfer.queue(now).unwrap();
        transfer.confirm_source("0xsourcehash", now, None).unwrap();
        transfer.begin_relay_attempt(now).unwrap();
        transfer
            .finish_current_relay_attempt(
                now,
                RelayAttemptOutcome::Accepted,
                Some("relay_ref_1".into()),
                Some("relay accepted".into())
            )
            .unwrap();

        let evidence = DestinationEvidence::confirmed(
            TxHash("0xdestinationhash".into()),
            ChainId("solana".into()),
            Address("So1Recipient111".into()),
            AssetId("USDC".into()),
            "1000000".to_string(),
            now,
            now,
            Some("destination confirmed".into())
        );

        transfer.record_destination_evidence(evidence, now).unwrap();
        transfer
    }

    fn relay_unknown_transfer() -> TransferIntent {
        let now = Utc::now();
        let mut transfer = TransferIntent::new(
            "transfer_123",
            "idem_123",
            "ethereum",
            "solana",
            "0xabc123",
            "So1Recipient111",
            "USDC",
            "1000000",
            now
        ).unwrap();

        transfer.validate(now).unwrap();
        transfer.queue(now).unwrap();
        transfer.confirm_source("0xsourcehash", now, None).unwrap();
        transfer.begin_relay_attempt(now).unwrap();
        transfer
            .finish_current_relay_attempt(
                now,
                RelayAttemptOutcome::UnknownOutcome {
                    classification: domain::FailureClassification::UnknownRelayOutcome,
                    reason: "timeout after relay submit".into(),
                },
                Some("relay_ref_2".into()),
                Some("ambiguous relay result".into())
            )
            .unwrap();

        transfer
    }

    fn mismatched_transfer() -> TransferIntent {
        let now = Utc::now();
        let mut transfer = TransferIntent::new(
            "transfer_123",
            "idem_123",
            "ethereum",
            "solana",
            "0xabc123",
            "So1Recipient111",
            "USDC",
            "1000000",
            now
        ).unwrap();

        transfer.validate(now).unwrap();
        transfer.queue(now).unwrap();
        transfer.confirm_source("0xsourcehash", now, None).unwrap();
        transfer.begin_relay_attempt(now).unwrap();
        transfer
            .finish_current_relay_attempt(
                now,
                RelayAttemptOutcome::Accepted,
                Some("relay_ref_1".into()),
                Some("relay accepted".into())
            )
            .unwrap();

        let evidence = DestinationEvidence::observed(
            TxHash("0xbaddestinationhash".into()),
            ChainId("solana".into()),
            Address("WrongRecipient999".into()),
            AssetId("USDC".into()),
            "1000000".to_string(),
            now,
            Some("recipient mismatch".into())
        );

        transfer.record_destination_evidence(evidence, now).unwrap();
        transfer
    }

    #[tokio::test]
    async fn matching_truth_reconciles_to_settled() {
        let repo = FakeRepo::default();
        let service = ReconciliationService::new(repo.clone());

        let transfer = destination_pending_transfer_with_matching_evidence();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let updated = service
            .reconcile(ReconcileTransferCommand {
                transfer_id,
                note: Some("all truth surfaces align".into()),
                reconciled_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::Settled);
        assert!(updated.reconciliation.is_some());
    }

    #[tokio::test]
    async fn relay_unknown_reconciles_to_manual_review() {
        let repo = FakeRepo::default();
        let service = ReconciliationService::new(repo.clone());

        let transfer = relay_unknown_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let updated = service
            .reconcile(ReconcileTransferCommand {
                transfer_id,
                note: Some("relay ambiguity unresolved".into()),
                reconciled_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::ManualReview);
        assert!(updated.reconciliation.is_some());
    }

    #[tokio::test]
    async fn mismatched_truth_reconciles_to_mismatch_detected() {
        let repo = FakeRepo::default();
        let service = ReconciliationService::new(repo.clone());

        let transfer = mismatched_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let updated = service
            .reconcile(ReconcileTransferCommand {
                transfer_id,
                note: Some("destination mismatch confirmed".into()),
                reconciled_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::MismatchDetected);
        assert!(updated.reconciliation.is_some());
    }
}
