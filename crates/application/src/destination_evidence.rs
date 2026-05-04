use async_trait::async_trait;
use chrono::{ DateTime, Utc };
use domain::{ Address, AssetId, ChainId, DestinationEvidence, TransferId, TransferIntent, TxHash };
use persistence::{ PersistenceError, PostgresPersistence };

use crate::ApplicationError;

#[derive(Debug, Clone)]
pub struct RecordDestinationEvidenceCommand {
    pub transfer_id: TransferId,
    pub destination_tx_hash: String,
    pub destination_chain: String,
    pub recipient: String,
    pub asset: String,
    pub quantity: String,
    pub status: String,
    pub note: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

#[async_trait]
pub trait DestinationEvidenceRepo: Clone + Send + Sync + 'static {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError>;

    async fn save_destination_evidence(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError>;
}

#[async_trait]
impl DestinationEvidenceRepo for PostgresPersistence {
    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError> {
        PostgresPersistence::get_transfer_by_id(self, transfer_id).await
    }

    async fn save_destination_evidence(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        PostgresPersistence::save_destination_evidence(self, transfer).await
    }
}

#[derive(Debug, Clone)]
pub struct DestinationEvidenceService<R> where R: DestinationEvidenceRepo {
    repo: R,
}

impl<R> DestinationEvidenceService<R> where R: DestinationEvidenceRepo {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn record_evidence(
        &self,
        command: RecordDestinationEvidenceCommand
    ) -> Result<TransferIntent, ApplicationError> {
        let destination_tx_hash = command.destination_tx_hash.trim().to_string();
        let destination_chain = command.destination_chain.trim().to_lowercase();
        let recipient = command.recipient.trim().to_string();
        let asset = command.asset.trim().to_uppercase();
        let quantity = command.quantity.trim().to_string();
        let status = command.status.trim().to_lowercase();

        if destination_tx_hash.is_empty() {
            return Err(ApplicationError::Validation("destination_tx_hash is required".to_string()));
        }
        if destination_chain.is_empty() {
            return Err(ApplicationError::Validation("destination_chain is required".to_string()));
        }
        if recipient.is_empty() {
            return Err(ApplicationError::Validation("recipient is required".to_string()));
        }
        if asset.is_empty() {
            return Err(ApplicationError::Validation("asset is required".to_string()));
        }
        if quantity.is_empty() {
            return Err(ApplicationError::Validation("quantity is required".to_string()));
        }

        let evidence = match status.as_str() {
            "observed" =>
                DestinationEvidence::observed(
                    TxHash(destination_tx_hash),
                    ChainId(destination_chain),
                    Address(recipient),
                    AssetId(asset),
                    quantity,
                    command.recorded_at,
                    command.note
                ),
            "confirmed" =>
                DestinationEvidence::confirmed(
                    TxHash(destination_tx_hash),
                    ChainId(destination_chain),
                    Address(recipient),
                    AssetId(asset),
                    quantity,
                    command.recorded_at,
                    command.recorded_at,
                    command.note
                ),
            _ => {
                return Err(
                    ApplicationError::Validation(
                        "status must be either 'observed' or 'confirmed'".to_string()
                    )
                );
            }
        };

        let mut transfer = self.repo.get_transfer_by_id(command.transfer_id).await?;
        transfer.record_destination_evidence(evidence, command.recorded_at)?;
        self.repo.save_destination_evidence(&transfer).await?;

        Ok(transfer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use domain::{ RelayAttemptOutcome, TransferState };
    use std::collections::HashMap;
    use std::sync::{ Arc, Mutex };

    #[derive(Debug, Clone, Default)]
    struct FakeRepo {
        transfers: Arc<Mutex<HashMap<TransferId, TransferIntent>>>,
    }

    #[async_trait]
    impl DestinationEvidenceRepo for FakeRepo {
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

        async fn save_destination_evidence(
            &self,
            transfer: &TransferIntent
        ) -> Result<(), PersistenceError> {
            let mut transfers = self.transfers.lock().unwrap();
            transfers.insert(transfer.id, transfer.clone());
            Ok(())
        }
    }

    fn destination_pending_transfer() -> TransferIntent {
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

        transfer
    }

    #[tokio::test]
    async fn matching_destination_evidence_keeps_transfer_pending_not_settled() {
        let repo = FakeRepo::default();
        let service = DestinationEvidenceService::new(repo.clone());

        let transfer = destination_pending_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let updated = service
            .record_evidence(RecordDestinationEvidenceCommand {
                transfer_id,
                destination_tx_hash: "0xdestinationhash".into(),
                destination_chain: "solana".into(),
                recipient: "So1Recipient111".into(),
                asset: "USDC".into(),
                quantity: "1000000".into(),
                status: "confirmed".into(),
                note: Some("destination mint confirmed".into()),
                recorded_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::DestinationPending);
        assert_ne!(updated.state, TransferState::Settled);
        assert!(updated.destination_evidence.is_some());
    }

    #[tokio::test]
    async fn mismatched_destination_evidence_moves_transfer_to_mismatch_detected() {
        let repo = FakeRepo::default();
        let service = DestinationEvidenceService::new(repo.clone());

        let transfer = destination_pending_transfer();
        let transfer_id = transfer.id;
        repo.transfers.lock().unwrap().insert(transfer_id, transfer);

        let updated = service
            .record_evidence(RecordDestinationEvidenceCommand {
                transfer_id,
                destination_tx_hash: "0xbaddestinationhash".into(),
                destination_chain: "solana".into(),
                recipient: "WrongRecipient999".into(),
                asset: "USDC".into(),
                quantity: "1000000".into(),
                status: "observed".into(),
                note: Some("destination recipient mismatch".into()),
                recorded_at: Utc::now(),
            }).await
            .unwrap();

        assert_eq!(updated.state, TransferState::MismatchDetected);
        assert!(updated.destination_evidence.is_some());
    }
}
