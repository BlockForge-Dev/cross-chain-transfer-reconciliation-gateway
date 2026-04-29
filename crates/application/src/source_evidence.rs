use async_trait::async_trait;
use chrono::{ DateTime, Utc };
use domain::{ TransferId, TransferIntent };
use persistence::{ PersistenceError, PostgresPersistence };

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct RecordSourceEvidenceCommand {
    pub transfer_id: TransferId,
    pub source_tx_hash: String,
    pub note: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

#[async_trait]
pub trait SourceEvidenceRepo: Clone + Send + Sync + 'static {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError>;

    async fn save_source_evidence(&self, transfer: &TransferIntent) -> Result<(), PersistenceError>;
}

#[async_trait]
impl SourceEvidenceRepo for PostgresPersistence {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError> {
        PostgresPersistence::get_transfer_by_id(self, transfer_id).await
    }

    async fn save_source_evidence(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        PostgresPersistence::save_source_evidence(self, transfer).await
    }
}

#[derive(Debug, Clone)]
pub struct SourceEvidenceService<R> where R: SourceEvidenceRepo {
    repo: R,
}

impl<R> SourceEvidenceService<R> where R: SourceEvidenceRepo {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn record_observed(
        &self,
        command: RecordSourceEvidenceCommand
    ) -> Result<TransferIntent, ApplicationError> {
        let tx_hash = command.source_tx_hash.trim().to_string();
        if tx_hash.is_empty() {
            return Err(ApplicationError::Validation("source_tx_hash is required".to_string()));
        }

        let mut transfer = self.repo.get_transfer_by_id(command.transfer_id).await?;

        transfer.record_source_observed(tx_hash, command.recorded_at, command.note)?;

        self.repo.save_source_evidence(&transfer).await?;
        Ok(transfer)
    }

    pub async fn record_confirmed(
        &self,
        command: RecordSourceEvidenceCommand
    ) -> Result<TransferIntent, ApplicationError> {
        let tx_hash = command.source_tx_hash.trim().to_string();
        if tx_hash.is_empty() {
            return Err(ApplicationError::Validation("source_tx_hash is required".to_string()));
        }

        let mut transfer = self.repo.get_transfer_by_id(command.transfer_id).await?;

        transfer.confirm_source(tx_hash, command.recorded_at, command.note)?;

        self.repo.save_source_evidence(&transfer).await?;
        Ok(transfer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::{ Arc, Mutex };

    #[derive(Debug, Clone, Default)]
    struct FakeRepo {
        transfers: Arc<Mutex<HashMap<TransferId, TransferIntent>>>,
    }

    #[async_trait]
    impl SourceEvidenceRepo for FakeRepo {
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

        async fn save_source_evidence(
            &self,
            transfer: &TransferIntent
        ) -> Result<(), PersistenceError> {
            let mut transfers = self.transfers.lock().unwrap();
            transfers.insert(transfer.id, transfer.clone());
            Ok(())
        }
    }

    fn queued_transfer() -> TransferIntent {
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
        transfer
    }

    #[tokio::test]
    async fn source_observed_updates_transfer_state() {
        let repo = FakeRepo::default();
        let service = SourceEvidenceService::new(repo.clone());

        let transfer = queued_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let updated = service
            .record_observed(RecordSourceEvidenceCommand {
                transfer_id,
                source_tx_hash: "0xsourcehash".into(),
                note: Some("source event observed".into()),
                recorded_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, domain::TransferState::SourceObserved);
        assert!(updated.source_evidence.is_some());
    }

    #[tokio::test]
    async fn source_confirmed_updates_transfer_state_but_not_settled() {
        let repo = FakeRepo::default();
        let service = SourceEvidenceService::new(repo.clone());

        let transfer = queued_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let updated = service
            .record_confirmed(RecordSourceEvidenceCommand {
                transfer_id,
                source_tx_hash: "0xsourcehash".into(),
                note: Some("source confirmation observed".into()),
                recorded_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, domain::TransferState::SourceConfirmed);
        assert_ne!(updated.state, domain::TransferState::Settled);
        assert!(updated.source_evidence.is_some());
    }
}
