use async_trait::async_trait;
use chrono::{ DateTime, Utc };
use domain::{ FailureClassification, RelayAttemptOutcome, TransferId, TransferIntent };
use persistence::{ PersistenceError, PostgresPersistence };

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct BeginRelayAttemptCommand {
    pub transfer_id: TransferId,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct FinishRelayAttemptCommand {
    pub transfer_id: TransferId,
    pub outcome: String,
    pub classification: Option<String>,
    pub reason: Option<String>,
    pub relay_reference: Option<String>,
    pub note: Option<String>,
    pub finished_at: DateTime<Utc>,
}

#[async_trait]
pub trait RelayAttemptRepo: Clone + Send + Sync + 'static {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError>;

    async fn save_relay_attempt_started(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError>;

    async fn save_relay_attempt_finished(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError>;
}

#[async_trait]
impl RelayAttemptRepo for PostgresPersistence {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError> {
        PostgresPersistence::get_transfer_by_id(self, transfer_id).await
    }

    async fn save_relay_attempt_started(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        PostgresPersistence::save_relay_attempt_started(self, transfer).await
    }

    async fn save_relay_attempt_finished(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        PostgresPersistence::save_relay_attempt_finished(self, transfer).await
    }
}

#[derive(Debug, Clone)]
pub struct RelayAttemptService<R> where R: RelayAttemptRepo {
    repo: R,
}

impl<R> RelayAttemptService<R> where R: RelayAttemptRepo {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn begin_attempt(
        &self,
        command: BeginRelayAttemptCommand
    ) -> Result<TransferIntent, ApplicationError> {
        let mut transfer = self.repo.get_transfer_by_id(command.transfer_id).await?;

        transfer.begin_relay_attempt(command.started_at)?;
        self.repo.save_relay_attempt_started(&transfer).await?;

        Ok(transfer)
    }

    pub async fn finish_attempt(
        &self,
        command: FinishRelayAttemptCommand
    ) -> Result<TransferIntent, ApplicationError> {
        let mut transfer = self.repo.get_transfer_by_id(command.transfer_id).await?;

        let outcome = parse_relay_attempt_outcome(
            &command.outcome,
            command.classification.as_deref(),
            command.reason.as_deref()
        )?;

        transfer.finish_current_relay_attempt(
            command.finished_at,
            outcome,
            command.relay_reference,
            command.note
        )?;

        self.repo.save_relay_attempt_finished(&transfer).await?;
        Ok(transfer)
    }
}

fn parse_relay_attempt_outcome(
    outcome: &str,
    classification: Option<&str>,
    reason: Option<&str>
) -> Result<RelayAttemptOutcome, ApplicationError> {
    let normalized = outcome.trim().to_lowercase();

    match normalized.as_str() {
        "accepted" => Ok(RelayAttemptOutcome::Accepted),

        "retryable_failure" =>
            Ok(RelayAttemptOutcome::RetryableFailure {
                classification: parse_failure_classification(
                    classification.ok_or_else(|| {
                        ApplicationError::Validation(
                            "classification is required for retryable_failure".to_string()
                        )
                    })?
                )?,
                reason: required_reason("retryable_failure", reason)?,
            }),

        "terminal_failure" =>
            Ok(RelayAttemptOutcome::TerminalFailure {
                classification: parse_failure_classification(
                    classification.ok_or_else(|| {
                        ApplicationError::Validation(
                            "classification is required for terminal_failure".to_string()
                        )
                    })?
                )?,
                reason: required_reason("terminal_failure", reason)?,
            }),

        "unknown_outcome" =>
            Ok(RelayAttemptOutcome::UnknownOutcome {
                classification: parse_failure_classification(
                    classification.ok_or_else(|| {
                        ApplicationError::Validation(
                            "classification is required for unknown_outcome".to_string()
                        )
                    })?
                )?,
                reason: required_reason("unknown_outcome", reason)?,
            }),

        other =>
            Err(
                ApplicationError::Validation(format!("unsupported relay attempt outcome: {other}"))
            ),
    }
}

fn parse_failure_classification(
    classification: &str
) -> Result<FailureClassification, ApplicationError> {
    match classification.trim().to_lowercase().as_str() {
        "validation" => Ok(FailureClassification::Validation),
        "duplicate_request" => Ok(FailureClassification::DuplicateRequest),
        "retryable_relay_infrastructure" => {
            Ok(FailureClassification::RetryableRelayInfrastructure)
        }
        "terminal_relay_failure" => Ok(FailureClassification::TerminalRelayFailure),
        "unknown_relay_outcome" => Ok(FailureClassification::UnknownRelayOutcome),
        "source_evidence_missing" => Ok(FailureClassification::SourceEvidenceMissing),
        "destination_evidence_missing" => Ok(FailureClassification::DestinationEvidenceMissing),
        "destination_mismatch" => Ok(FailureClassification::DestinationMismatch),
        "reconciliation_mismatch" => Ok(FailureClassification::ReconciliationMismatch),
        other =>
            Err(
                ApplicationError::Validation(format!("unsupported failure classification: {other}"))
            ),
    }
}

fn required_reason(kind: &str, reason: Option<&str>) -> Result<String, ApplicationError> {
    let value = reason.unwrap_or_default().trim().to_string();
    if value.is_empty() {
        return Err(ApplicationError::Validation(format!("reason is required for {kind}")));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use domain::TransferState;
    use std::collections::HashMap;
    use std::sync::{ Arc, Mutex };

    #[derive(Debug, Clone, Default)]
    struct FakeRepo {
        transfers: Arc<Mutex<HashMap<TransferId, TransferIntent>>>,
    }

    #[async_trait]
    impl RelayAttemptRepo for FakeRepo {
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

        async fn save_relay_attempt_started(
            &self,
            transfer: &TransferIntent
        ) -> Result<(), PersistenceError> {
            let mut transfers = self.transfers.lock().unwrap();
            transfers.insert(transfer.id, transfer.clone());
            Ok(())
        }

        async fn save_relay_attempt_finished(
            &self,
            transfer: &TransferIntent
        ) -> Result<(), PersistenceError> {
            let mut transfers = self.transfers.lock().unwrap();
            transfers.insert(transfer.id, transfer.clone());
            Ok(())
        }
    }

    fn source_confirmed_transfer() -> TransferIntent {
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
        transfer
    }

    #[tokio::test]
    async fn relay_attempt_start_moves_transfer_to_in_progress() {
        let repo = FakeRepo::default();
        let service = RelayAttemptService::new(repo.clone());

        let transfer = source_confirmed_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let updated = service
            .begin_attempt(BeginRelayAttemptCommand {
                transfer_id,
                started_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::RelayInProgress);
        assert_eq!(updated.relay_attempts.len(), 1);
    }

    #[tokio::test]
    async fn relay_attempt_accepted_moves_transfer_to_destination_pending() {
        let repo = FakeRepo::default();
        let service = RelayAttemptService::new(repo.clone());

        let transfer = source_confirmed_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        service
            .begin_attempt(BeginRelayAttemptCommand {
                transfer_id,
                started_at: Utc::now(),
            }).await
            .unwrap();

        let updated = service
            .finish_attempt(FinishRelayAttemptCommand {
                transfer_id,
                outcome: "accepted".into(),
                classification: None,
                reason: None,
                relay_reference: Some("relay_ref_1".into()),
                note: Some("relay accepted".into()),
                finished_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::DestinationPending);
    }

    #[tokio::test]
    async fn retryable_failure_returns_transfer_to_source_confirmed() {
        let repo = FakeRepo::default();
        let service = RelayAttemptService::new(repo.clone());

        let transfer = source_confirmed_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        service
            .begin_attempt(BeginRelayAttemptCommand {
                transfer_id,
                started_at: Utc::now(),
            }).await
            .unwrap();

        let updated = service
            .finish_attempt(FinishRelayAttemptCommand {
                transfer_id,
                outcome: "retryable_failure".into(),
                classification: Some("retryable_relay_infrastructure".into()),
                reason: Some("temporary relayer outage".into()),
                relay_reference: Some("relay_ref_2".into()),
                note: Some("safe to retry later".into()),
                finished_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::SourceConfirmed);
    }

    #[tokio::test]
    async fn unknown_outcome_moves_transfer_to_relay_unknown() {
        let repo = FakeRepo::default();
        let service = RelayAttemptService::new(repo.clone());

        let transfer = source_confirmed_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        service
            .begin_attempt(BeginRelayAttemptCommand {
                transfer_id,
                started_at: Utc::now(),
            }).await
            .unwrap();

        let updated = service
            .finish_attempt(FinishRelayAttemptCommand {
                transfer_id,
                outcome: "unknown_outcome".into(),
                classification: Some("unknown_relay_outcome".into()),
                reason: Some("timeout after relay submit".into()),
                relay_reference: Some("relay_ref_3".into()),
                note: Some("relay result ambiguous".into()),
                finished_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::RelayUnknown);
    }
}
