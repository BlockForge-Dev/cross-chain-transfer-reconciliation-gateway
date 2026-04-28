use async_trait::async_trait;
use chrono::{ DateTime, Utc };
use domain::{ TransferId, TransferIntent };
use persistence::{ CreateTransferResult, PostgresPersistence };
use serde::{ Deserialize, Serialize };
use serde_json::json;
use sha2::{ Digest, Sha256 };

use crate::ApplicationError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransferIntentCommand {
    pub client_transfer_reference: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub source_address: String,
    pub destination_recipient: String,
    pub asset: String,
    pub quantity: String,
    pub idempotency_key: String,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum CreateTransferIntentResult {
    Created(TransferIntent),
    Existing(TransferIntent),
}

#[async_trait]
pub trait TransferIntentGatewayRepo: Clone + Send + Sync + 'static {
    async fn create_transfer_with_idempotency(
        &self,
        transfer: &TransferIntent,
        scope: &str,
        request_fingerprint: &str
    ) -> Result<CreateTransferResult, persistence::PersistenceError>;

    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, persistence::PersistenceError>;
}

#[async_trait]
impl TransferIntentGatewayRepo for PostgresPersistence {
    async fn create_transfer_with_idempotency(
        &self,
        transfer: &TransferIntent,
        scope: &str,
        request_fingerprint: &str
    ) -> Result<CreateTransferResult, persistence::PersistenceError> {
        PostgresPersistence::create_transfer_with_idempotency(
            self,
            transfer,
            scope,
            request_fingerprint
        ).await
    }

    async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, persistence::PersistenceError> {
        PostgresPersistence::get_transfer_by_id(self, transfer_id).await
    }
}

#[derive(Debug, Clone)]
pub struct TransferIntentService<R> where R: TransferIntentGatewayRepo {
    repo: R,
    supported_chains: Vec<String>,
    idempotency_scope: String,
}

impl<R> TransferIntentService<R> where R: TransferIntentGatewayRepo {
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            supported_chains: vec![
                "ethereum".to_string(),
                "solana".to_string(),
                "base".to_string(),
                "polygon".to_string(),
                "arbitrum".to_string()
            ],
            idempotency_scope: "transfer_intents:create".to_string(),
        }
    }

    pub fn with_supported_chains(mut self, chains: Vec<String>) -> Self {
        self.supported_chains = chains
            .into_iter()
            .map(|c| normalize_chain(&c))
            .collect();
        self
    }

    pub async fn create_transfer(
        &self,
        command: CreateTransferIntentCommand
    ) -> Result<CreateTransferIntentResult, ApplicationError> {
        let client_transfer_reference = command.client_transfer_reference.trim().to_string();
        let source_chain = normalize_chain(&command.source_chain);
        let destination_chain = normalize_chain(&command.destination_chain);
        let source_address = command.source_address.trim().to_string();
        let destination_recipient = command.destination_recipient.trim().to_string();
        let asset = command.asset.trim().to_uppercase();
        let quantity = command.quantity.trim().to_string();
        let idempotency_key = command.idempotency_key.trim().to_string();

        if client_transfer_reference.is_empty() {
            return Err(
                ApplicationError::Validation("client_transfer_reference is required".to_string())
            );
        }

        if idempotency_key.is_empty() {
            return Err(
                ApplicationError::Validation("Idempotency-Key header is required".to_string())
            );
        }

        if source_address.is_empty() {
            return Err(ApplicationError::Validation("source_address is required".to_string()));
        }

        if destination_recipient.is_empty() {
            return Err(
                ApplicationError::Validation("destination_recipient is required".to_string())
            );
        }

        if asset.is_empty() {
            return Err(ApplicationError::Validation("asset is required".to_string()));
        }

        if quantity.is_empty() {
            return Err(ApplicationError::Validation("quantity is required".to_string()));
        }

        if !self.supported_chains.iter().any(|c| c == &source_chain) {
            return Err(ApplicationError::UnsupportedChain(source_chain));
        }

        if !self.supported_chains.iter().any(|c| c == &destination_chain) {
            return Err(ApplicationError::UnsupportedChain(destination_chain));
        }

        let fingerprint = fingerprint_create_transfer_request(
            &client_transfer_reference,
            &source_chain,
            &destination_chain,
            &source_address,
            &destination_recipient,
            &asset,
            &quantity
        )?;

        let mut transfer = TransferIntent::new(
            client_transfer_reference,
            idempotency_key,
            source_chain,
            destination_chain,
            source_address,
            destination_recipient,
            asset,
            quantity,
            command.received_at
        )?;

        transfer.validate(command.received_at)?;
        transfer.queue(command.received_at)?;

        let result = self.repo.create_transfer_with_idempotency(
            &transfer,
            &self.idempotency_scope,
            &fingerprint
        ).await?;

        match result {
            CreateTransferResult::Created(transfer) => {
                Ok(CreateTransferIntentResult::Created(transfer))
            }
            CreateTransferResult::Existing(transfer) => {
                Ok(CreateTransferIntentResult::Existing(transfer))
            }
        }
    }

    pub async fn get_transfer(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, ApplicationError> {
        self.repo.get_transfer_by_id(transfer_id).await.map_err(Into::into)
    }
}

fn normalize_chain(chain: &str) -> String {
    chain.trim().to_lowercase()
}

pub fn fingerprint_create_transfer_request(
    client_transfer_reference: &str,
    source_chain: &str,
    destination_chain: &str,
    source_address: &str,
    destination_recipient: &str,
    asset: &str,
    quantity: &str
) -> Result<String, ApplicationError> {
    let canonical =
        json!({
        "client_transfer_reference": client_transfer_reference.trim(),
        "source_chain": source_chain.trim().to_lowercase(),
        "destination_chain": destination_chain.trim().to_lowercase(),
        "source_address": source_address.trim(),
        "destination_recipient": destination_recipient.trim(),
        "asset": asset.trim().to_uppercase(),
        "quantity": quantity.trim(),
    });

    let bytes = serde_json
        ::to_vec(&canonical)
        .map_err(|e| ApplicationError::Validation(format!("failed to canonicalize request: {e}")))?;

    let digest = Sha256::digest(bytes);
    Ok(hex::encode(digest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use persistence::{ CreateTransferResult, PersistenceError };
    use std::collections::HashMap;
    use std::sync::{ Arc, Mutex };

    #[derive(Debug, Clone, Default)]
    struct FakeRepo {
        store: Arc<Mutex<HashMap<(String, String), (String, TransferIntent)>>>,
        transfers: Arc<Mutex<HashMap<TransferId, TransferIntent>>>,
    }

    #[async_trait]
    impl TransferIntentGatewayRepo for FakeRepo {
        async fn create_transfer_with_idempotency(
            &self,
            transfer: &TransferIntent,
            scope: &str,
            request_fingerprint: &str
        ) -> Result<CreateTransferResult, PersistenceError> {
            let mut store = self.store.lock().unwrap();
            let mut transfers = self.transfers.lock().unwrap();

            let key = (scope.to_string(), transfer.idempotency_key.0.clone());

            if let Some((existing_fingerprint, existing_transfer)) = store.get(&key) {
                if existing_fingerprint != request_fingerprint {
                    return Err(PersistenceError::IdempotencyConflict {
                        scope: scope.to_string(),
                        key: transfer.idempotency_key.0.clone(),
                    });
                }

                return Ok(CreateTransferResult::Existing(existing_transfer.clone()));
            }

            store.insert(key, (request_fingerprint.to_string(), transfer.clone()));
            transfers.insert(transfer.id, transfer.clone());

            Ok(CreateTransferResult::Created(transfer.clone()))
        }

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
    }

    fn service() -> TransferIntentService<FakeRepo> {
        TransferIntentService::new(FakeRepo::default()).with_supported_chains(
            vec!["ethereum".into(), "solana".into(), "base".into()]
        )
    }

    #[tokio::test]
    async fn same_idempotency_key_same_payload_returns_existing_lineage() {
        let svc = service();
        let now = Utc::now();

        let cmd = CreateTransferIntentCommand {
            client_transfer_reference: "transfer_123".into(),
            source_chain: "ethereum".into(),
            destination_chain: "solana".into(),
            source_address: "0xabc123".into(),
            destination_recipient: "So1Recipient111".into(),
            asset: "USDC".into(),
            quantity: "1000000".into(),
            idempotency_key: "idem_123".into(),
            received_at: now,
        };

        let first = svc.create_transfer(cmd.clone()).await.unwrap();
        let second = svc.create_transfer(cmd.clone()).await.unwrap();

        let first_id = match first {
            CreateTransferIntentResult::Created(transfer) => transfer.id,
            CreateTransferIntentResult::Existing(_) => panic!("first call should create"),
        };

        let second_id = match second {
            CreateTransferIntentResult::Created(_) => panic!("second call should return existing"),
            CreateTransferIntentResult::Existing(transfer) => transfer.id,
        };

        assert_eq!(first_id, second_id);
    }

    #[tokio::test]
    async fn same_idempotency_key_different_payload_is_rejected() {
        let svc = service();
        let now = Utc::now();

        let first = CreateTransferIntentCommand {
            client_transfer_reference: "transfer_123".into(),
            source_chain: "ethereum".into(),
            destination_chain: "solana".into(),
            source_address: "0xabc123".into(),
            destination_recipient: "So1Recipient111".into(),
            asset: "USDC".into(),
            quantity: "1000000".into(),
            idempotency_key: "idem_123".into(),
            received_at: now,
        };

        let second = CreateTransferIntentCommand {
            client_transfer_reference: "transfer_123".into(),
            source_chain: "ethereum".into(),
            destination_chain: "solana".into(),
            source_address: "0xabc123".into(),
            destination_recipient: "So1Recipient111".into(),
            asset: "USDC".into(),
            quantity: "999999".into(),
            idempotency_key: "idem_123".into(),
            received_at: now,
        };

        svc.create_transfer(first).await.unwrap();
        let result = svc.create_transfer(second).await;

        assert!(matches!(result, Err(ApplicationError::IdempotencyConflict { .. })));
    }

    #[tokio::test]
    async fn unsupported_chain_is_rejected() {
        let svc = service();
        let now = Utc::now();

        let cmd = CreateTransferIntentCommand {
            client_transfer_reference: "transfer_123".into(),
            source_chain: "unknown_chain".into(),
            destination_chain: "solana".into(),
            source_address: "0xabc123".into(),
            destination_recipient: "So1Recipient111".into(),
            asset: "USDC".into(),
            quantity: "1000000".into(),
            idempotency_key: "idem_123".into(),
            received_at: now,
        };

        let result = svc.create_transfer(cmd).await;
        assert!(matches!(result, Err(ApplicationError::UnsupportedChain(_))));
    }
}
