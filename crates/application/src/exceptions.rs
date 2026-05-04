use async_trait::async_trait;
use chrono::{ DateTime, Utc };
use domain::{ ExceptionClassification, TransferId, TransferIntent, TransferState };
use persistence::{
    PersistenceError,
    PostgresPersistence,
    SaveExceptionCaseInput,
    StoredExceptionCase,
};

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct OpenExceptionCaseCommand {
    pub transfer_id: TransferId,
    pub classification: Option<String>,
    pub case_status: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ResolveExceptionCaseCommand {
    pub transfer_id: TransferId,
    pub note: Option<String>,
    pub resolved_at: DateTime<Utc>,
}

#[async_trait]
pub trait ExceptionCaseRepo: Clone + Send + Sync + 'static {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError>;

    async fn save_exception_case(
        &self,
        input: SaveExceptionCaseInput
    ) -> Result<StoredExceptionCase, PersistenceError>;

    async fn list_exception_cases_by_transfer(
        &self,
        transfer_id: TransferId
    ) -> Result<Vec<StoredExceptionCase>, PersistenceError>;

    async fn resolve_latest_open_exception_case(
        &self,
        transfer_id: TransferId,
        resolution_note: Option<String>,
        resolved_at: DateTime<Utc>
    ) -> Result<StoredExceptionCase, PersistenceError>;
}

#[async_trait]
impl ExceptionCaseRepo for PostgresPersistence {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError> {
        PostgresPersistence::get_transfer_by_id(self, transfer_id).await
    }

    async fn save_exception_case(
        &self,
        input: SaveExceptionCaseInput
    ) -> Result<StoredExceptionCase, PersistenceError> {
        PostgresPersistence::save_exception_case(self, input).await
    }

    async fn list_exception_cases_by_transfer(
        &self,
        transfer_id: TransferId
    ) -> Result<Vec<StoredExceptionCase>, PersistenceError> {
        PostgresPersistence::list_exception_cases_by_transfer(self, transfer_id).await
    }

    async fn resolve_latest_open_exception_case(
        &self,
        transfer_id: TransferId,
        resolution_note: Option<String>,
        resolved_at: DateTime<Utc>
    ) -> Result<StoredExceptionCase, PersistenceError> {
        PostgresPersistence::resolve_latest_open_exception_case(
            self,
            transfer_id,
            resolution_note,
            resolved_at
        ).await
    }
}

#[derive(Debug, Clone)]
pub struct ExceptionCaseService<R> where R: ExceptionCaseRepo {
    repo: R,
}

impl<R> ExceptionCaseService<R> where R: ExceptionCaseRepo {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn open_case(
        &self,
        command: OpenExceptionCaseCommand
    ) -> Result<StoredExceptionCase, ApplicationError> {
        let transfer = self.repo.get_transfer_by_id(command.transfer_id).await?;

        let classification = match command.classification {
            Some(value) => parse_exception_classification(&value)?,
            None => infer_exception_classification(&transfer)?,
        };

        let case_status = normalize_case_status(command.case_status.as_deref())?;
        if case_status == "resolved" {
            return Err(
                ApplicationError::Validation(
                    "use the resolve endpoint to resolve an exception case".to_string()
                )
            );
        }

        self.repo
            .save_exception_case(SaveExceptionCaseInput {
                transfer_id: command.transfer_id,
                classification,
                case_status,
                note: command.note,
                created_at: command.created_at,
                resolved_at: None,
            }).await
            .map_err(Into::into)
    }

    pub async fn list_cases(
        &self,
        transfer_id: TransferId
    ) -> Result<Vec<StoredExceptionCase>, ApplicationError> {
        self.repo.list_exception_cases_by_transfer(transfer_id).await.map_err(Into::into)
    }

    pub async fn resolve_latest_case(
        &self,
        command: ResolveExceptionCaseCommand
    ) -> Result<StoredExceptionCase, ApplicationError> {
        self.repo
            .resolve_latest_open_exception_case(
                command.transfer_id,
                command.note,
                command.resolved_at
            ).await
            .map_err(Into::into)
    }
}

fn infer_exception_classification(
    transfer: &TransferIntent
) -> Result<ExceptionClassification, ApplicationError> {
    if let Some(existing) = &transfer.latest_exception {
        return Ok(existing.clone());
    }

    match transfer.state {
        TransferState::RelayUnknown => Ok(ExceptionClassification::AmbiguousRelayOutcome),
        TransferState::MismatchDetected => Ok(ExceptionClassification::DestinationMismatch),
        TransferState::ManualReview => Ok(ExceptionClassification::ManualReviewRequired),
        TransferState::DestinationPending => Ok(ExceptionClassification::DestinationMissing),
        _ =>
            Err(
                ApplicationError::Validation(
                    "transfer is not currently in an exception-worthy state; provide an explicit classification if needed".to_string()
                )
            ),
    }
}

fn parse_exception_classification(
    value: &str
) -> Result<ExceptionClassification, ApplicationError> {
    match value.trim().to_lowercase().as_str() {
        "destination_missing" => Ok(ExceptionClassification::DestinationMissing),
        "destination_mismatch" => Ok(ExceptionClassification::DestinationMismatch),
        "ambiguous_relay_outcome" => Ok(ExceptionClassification::AmbiguousRelayOutcome),
        "duplicate_relay_attempt" => Ok(ExceptionClassification::DuplicateRelayAttempt),
        "stale_pending_transfer" => Ok(ExceptionClassification::StalePendingTransfer),
        "source_missing" => Ok(ExceptionClassification::SourceMissing),
        "manual_review_required" => Ok(ExceptionClassification::ManualReviewRequired),
        other =>
            Err(
                ApplicationError::Validation(
                    format!("unsupported exception classification: {other}")
                )
            ),
    }
}

fn normalize_case_status(value: Option<&str>) -> Result<String, ApplicationError> {
    let normalized = value.unwrap_or("open").trim().to_lowercase();
    match normalized.as_str() {
        "open" | "investigating" | "resolved" => Ok(normalized),
        other => Err(ApplicationError::Validation(format!("unsupported case status: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use domain::{ FailureClassification, RelayAttemptOutcome, TransferIntent };
    use std::collections::HashMap;
    use std::sync::{ Arc, Mutex };

    #[derive(Debug, Clone, Default)]
    struct FakeRepo {
        transfers: Arc<Mutex<HashMap<TransferId, TransferIntent>>>,
        cases: Arc<Mutex<Vec<StoredExceptionCase>>>,
        next_case_id: Arc<Mutex<i64>>,
    }

    #[async_trait]
    impl ExceptionCaseRepo for FakeRepo {
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

        async fn save_exception_case(
            &self,
            input: SaveExceptionCaseInput
        ) -> Result<StoredExceptionCase, PersistenceError> {
            let mut next = self.next_case_id.lock().unwrap();
            *next += 1;

            let case = StoredExceptionCase {
                case_id: *next,
                transfer_id: input.transfer_id,
                classification: input.classification,
                case_status: input.case_status,
                note: input.note,
                created_at: input.created_at,
                resolved_at: input.resolved_at,
            };

            self.cases.lock().unwrap().push(case.clone());
            Ok(case)
        }

        async fn list_exception_cases_by_transfer(
            &self,
            transfer_id: TransferId
        ) -> Result<Vec<StoredExceptionCase>, PersistenceError> {
            let cases = self.cases.lock().unwrap();
            Ok(
                cases
                    .iter()
                    .filter(|c| c.transfer_id == transfer_id)
                    .cloned()
                    .collect()
            )
        }

        async fn resolve_latest_open_exception_case(
            &self,
            transfer_id: TransferId,
            resolution_note: Option<String>,
            resolved_at: DateTime<Utc>
        ) -> Result<StoredExceptionCase, PersistenceError> {
            let mut cases = self.cases.lock().unwrap();
            let case = cases
                .iter_mut()
                .rev()
                .find(|c| c.transfer_id == transfer_id && c.case_status != "resolved")
                .ok_or(PersistenceError::ExceptionCaseNotFound(transfer_id))?;

            case.case_status = "resolved".to_string();
            case.note = resolution_note;
            case.resolved_at = Some(resolved_at);

            Ok(case.clone())
        }
    }

    fn mismatch_transfer() -> TransferIntent {
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

        transfer.latest_failure = Some(FailureClassification::DestinationMismatch);
        transfer.latest_exception = Some(ExceptionClassification::DestinationMismatch);
        transfer.state = TransferState::MismatchDetected;
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
                    classification: FailureClassification::UnknownRelayOutcome,
                    reason: "timeout after relay submit".into(),
                },
                Some("relay_ref_2".into()),
                Some("ambiguous relay result".into())
            )
            .unwrap();

        transfer
    }

    #[tokio::test]
    async fn open_case_infers_destination_mismatch() {
        let repo = FakeRepo::default();
        let service = ExceptionCaseService::new(repo.clone());

        let transfer = mismatch_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let case = service
            .open_case(OpenExceptionCaseCommand {
                transfer_id,
                classification: None,
                case_status: None,
                note: Some("operator opened mismatch case".into()),
                created_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(case.classification, ExceptionClassification::DestinationMismatch);
        assert_eq!(case.case_status, "open");
    }

    #[tokio::test]
    async fn open_case_infers_ambiguous_relay_outcome() {
        let repo = FakeRepo::default();
        let service = ExceptionCaseService::new(repo.clone());

        let transfer = relay_unknown_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let case = service
            .open_case(OpenExceptionCaseCommand {
                transfer_id,
                classification: None,
                case_status: Some("investigating".into()),
                note: Some("operator investigating ambiguous relay".into()),
                created_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(case.classification, ExceptionClassification::AmbiguousRelayOutcome);
        assert_eq!(case.case_status, "investigating");
    }

    #[tokio::test]
    async fn resolve_latest_case_marks_it_resolved() {
        let repo = FakeRepo::default();
        let service = ExceptionCaseService::new(repo.clone());

        let transfer = mismatch_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        service
            .open_case(OpenExceptionCaseCommand {
                transfer_id,
                classification: None,
                case_status: None,
                note: Some("open case".into()),
                created_at: Utc::now(),
            }).await
            .unwrap();

        let resolved = service
            .resolve_latest_case(ResolveExceptionCaseCommand {
                transfer_id,
                note: Some("resolved after operator review".into()),
                resolved_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(resolved.case_status, "resolved");
        assert!(resolved.resolved_at.is_some());
    }
}
