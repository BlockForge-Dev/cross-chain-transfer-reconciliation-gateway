use chrono::{ DateTime, Utc };
use domain::{
    Address,
    AssetAmount,
    ChainId,
    ClientTransferReference,
    DestinationEvidence,
    EvidenceSource,
    ExceptionClassification,
    FailureClassification,
    IdempotencyKey,
    ReceiptTimelineEntry,
    ReconciliationComparison,
    ReconciliationDecision,
    ReconciliationResult,
    RelayAttempt,
    RelayAttemptOutcome,
    RelayReference,
    SourceEvidence,
    TransferId,
    TransferIntent,
    TransferReceipt,
    TransferState,
    TxHash,
};
use serde::{ Deserialize, Serialize };
use serde_json::{ json, Value };
use sqlx::{ PgPool, Postgres, Transaction };

use crate::error::PersistenceError;
use crate::rows::{
    DbAuditEventRow,
    DbDestinationEvidenceRow,
    DbExceptionCaseRow,
    DbIdempotencyKeyRow,
    DbReconciliationRunRow,
    DbRelayAttemptRow,
    DbSourceEvidenceRow,
    DbTransferIntentRow,
};

#[derive(Debug, Clone)]
pub struct PostgresPersistence {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub enum CreateTransferResult {
    Created(TransferIntent),
    Existing(TransferIntent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveExceptionCaseInput {
    pub transfer_id: TransferId,
    pub classification: ExceptionClassification,
    pub case_status: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredExceptionCase {
    pub transfer_id: TransferId,
    pub classification: ExceptionClassification,
    pub case_status: String,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAuditEvent {
    pub transfer_id: Option<TransferId>,
    pub event_type: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedTransferReceipt {
    pub core: TransferReceipt,
    pub exception_cases: Vec<StoredExceptionCase>,
    pub audit_events: Vec<StoredAuditEvent>,
}

impl PostgresPersistence {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn create_transfer_with_idempotency(
        &self,
        transfer: &TransferIntent,
        scope: &str,
        request_fingerprint: &str
    ) -> Result<CreateTransferResult, PersistenceError> {
        let mut tx = self.pool.begin().await?;

        let existing = sqlx
            ::query_as::<_, DbIdempotencyKeyRow>(
                r#"
            SELECT scope, idempotency_key, transfer_id, request_fingerprint, created_at
            FROM idempotency_keys
            WHERE scope = $1 AND idempotency_key = $2
            "#
            )
            .bind(scope)
            .bind(transfer.idempotency_key.0.as_str())
            .fetch_optional(&mut *tx).await?;

        if let Some(existing) = existing {
            if existing.request_fingerprint != request_fingerprint {
                return Err(PersistenceError::IdempotencyConflict {
                    scope: scope.to_string(),
                    key: existing.idempotency_key,
                });
            }

            let existing_transfer = self.load_transfer_by_id_tx(
                &mut tx,
                existing.transfer_id
            ).await?;
            return Ok(CreateTransferResult::Existing(existing_transfer));
        }

        sqlx
            ::query(
                r#"
            INSERT INTO transfer_intents (
                id,
                client_transfer_reference,
                source_chain,
                destination_chain,
                source_address,
                destination_recipient,
                asset_id,
                quantity,
                state,
                latest_failure_classification,
                latest_exception_classification,
                created_at,
                updated_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#
            )
            .bind(transfer.id)
            .bind(transfer.client_transfer_reference.0.as_str())
            .bind(transfer.source_chain.0.as_str())
            .bind(transfer.destination_chain.0.as_str())
            .bind(transfer.source_address.0.as_str())
            .bind(transfer.destination_recipient.0.as_str())
            .bind(transfer.asset_amount.asset.0.as_str())
            .bind(transfer.asset_amount.quantity.as_str())
            .bind(state_to_db(transfer.state))
            .bind(transfer.latest_failure.as_ref().map(failure_to_db))
            .bind(transfer.latest_exception.as_ref().map(exception_to_db))
            .bind(transfer.created_at)
            .bind(transfer.updated_at)
            .execute(&mut *tx).await?;

        sqlx
            ::query(
                r#"
            INSERT INTO idempotency_keys (
                scope,
                idempotency_key,
                transfer_id,
                request_fingerprint,
                created_at
            )
            VALUES ($1,$2,$3,$4,$5)
            "#
            )
            .bind(scope)
            .bind(transfer.idempotency_key.0.as_str())
            .bind(transfer.id)
            .bind(request_fingerprint)
            .bind(transfer.created_at)
            .execute(&mut *tx).await?;

        for entry in &transfer.timeline {
            insert_state_transition_audit_tx(&mut tx, transfer.id, entry).await?;
        }

        insert_audit_event_tx(
            &mut tx,
            Some(transfer.id),
            "transfer_created",
            json!({
                "client_transfer_reference": transfer.client_transfer_reference.0,
                "source_chain": transfer.source_chain.0,
                "destination_chain": transfer.destination_chain.0,
                "asset": transfer.asset_amount.asset.0,
                "quantity": transfer.asset_amount.quantity,
            }),
            transfer.created_at
        ).await?;

        tx.commit().await?;
        Ok(CreateTransferResult::Created(transfer.clone()))
    }

    pub async fn get_transfer_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError> {
        let mut tx = self.pool.begin().await?;
        let transfer = self.load_transfer_by_id_tx(&mut tx, transfer_id).await?;
        tx.commit().await?;
        Ok(transfer)
    }

    pub async fn save_source_evidence(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        let source = transfer.source_evidence
            .as_ref()
            .ok_or_else(||
                PersistenceError::InvariantViolation(
                    "source evidence missing on aggregate".to_string()
                )
            )?;

        let mut tx = self.pool.begin().await?;

        self.update_transfer_header_tx(&mut tx, transfer).await?;

        sqlx
            ::query(
                r#"
            INSERT INTO source_evidence (
                transfer_id,
                source_tx_hash,
                observed_at,
                confirmed_at,
                note
            )
            VALUES ($1,$2,$3,$4,$5)
            ON CONFLICT (transfer_id)
            DO UPDATE SET
                source_tx_hash = EXCLUDED.source_tx_hash,
                observed_at = EXCLUDED.observed_at,
                confirmed_at = EXCLUDED.confirmed_at,
                note = EXCLUDED.note
            "#
            )
            .bind(transfer.id)
            .bind(source.source_tx_hash.0.as_str())
            .bind(source.observed_at)
            .bind(source.confirmed_at)
            .bind(source.note.as_deref())
            .execute(&mut *tx).await?;

        maybe_insert_latest_state_transition_tx(&mut tx, transfer).await?;

        insert_audit_event_tx(
            &mut tx,
            Some(transfer.id),
            "source_evidence_recorded",
            json!({
                "source_tx_hash": source.source_tx_hash.0,
                "observed_at": source.observed_at,
                "confirmed_at": source.confirmed_at,
            }),
            transfer.updated_at
        ).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn save_relay_attempt_started(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        let attempt = transfer.relay_attempts
            .last()
            .ok_or_else(||
                PersistenceError::InvariantViolation(
                    "relay attempt start requested but no current attempt exists".to_string()
                )
            )?;

        let mut tx = self.pool.begin().await?;
        self.update_transfer_header_tx(&mut tx, transfer).await?;

        sqlx
            ::query(
                r#"
            INSERT INTO relay_attempts (
                transfer_id,
                attempt_no,
                started_at,
                ended_at,
                outcome_kind,
                error_category,
                result_reason,
                relay_reference,
                note
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#
            )
            .bind(transfer.id)
            .bind(
                i32
                    ::try_from(attempt.attempt_no)
                    .map_err(|_|
                        PersistenceError::InvariantViolation("attempt number overflow".to_string())
                    )?
            )
            .bind(attempt.started_at)
            .bind(attempt.ended_at)
            .bind(Option::<String>::None)
            .bind(Option::<String>::None)
            .bind(Option::<String>::None)
            .bind(attempt.relay_reference.as_ref().map(|r| r.0.as_str()))
            .bind(attempt.note.as_deref())
            .execute(&mut *tx).await?;

        maybe_insert_latest_state_transition_tx(&mut tx, transfer).await?;

        insert_audit_event_tx(
            &mut tx,
            Some(transfer.id),
            "relay_attempt_started",
            json!({
                "attempt_no": attempt.attempt_no,
                "started_at": attempt.started_at,
            }),
            attempt.started_at
        ).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn save_relay_attempt_finished(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        let attempt = transfer.relay_attempts
            .last()
            .ok_or_else(||
                PersistenceError::InvariantViolation(
                    "relay attempt finish requested but no current attempt exists".to_string()
                )
            )?;

        let outcome = attempt.outcome
            .as_ref()
            .ok_or_else(||
                PersistenceError::InvariantViolation(
                    "relay attempt finish requested but attempt has no outcome".to_string()
                )
            )?;

        let (outcome_kind, error_category, result_reason) = relay_attempt_outcome_to_db(outcome);

        let mut tx = self.pool.begin().await?;
        self.update_transfer_header_tx(&mut tx, transfer).await?;

        sqlx
            ::query(
                r#"
            UPDATE relay_attempts
            SET
                ended_at = $3,
                outcome_kind = $4,
                error_category = $5,
                result_reason = $6,
                relay_reference = $7,
                note = $8
            WHERE transfer_id = $1 AND attempt_no = $2
            "#
            )
            .bind(transfer.id)
            .bind(
                i32
                    ::try_from(attempt.attempt_no)
                    .map_err(|_|
                        PersistenceError::InvariantViolation("attempt number overflow".to_string())
                    )?
            )
            .bind(attempt.ended_at)
            .bind(outcome_kind)
            .bind(error_category)
            .bind(result_reason)
            .bind(attempt.relay_reference.as_ref().map(|r| r.0.as_str()))
            .bind(attempt.note.as_deref())
            .execute(&mut *tx).await?;

        maybe_insert_latest_state_transition_tx(&mut tx, transfer).await?;

        insert_audit_event_tx(
            &mut tx,
            Some(transfer.id),
            "relay_attempt_finished",
            json!({
                "attempt_no": attempt.attempt_no,
                "outcome_kind": outcome_kind,
                "relay_reference": attempt.relay_reference.as_ref().map(|r| r.0.clone()),
                "ended_at": attempt.ended_at,
            }),
            attempt.ended_at.unwrap_or(transfer.updated_at)
        ).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn save_destination_evidence(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        let destination = transfer.destination_evidence
            .as_ref()
            .ok_or_else(||
                PersistenceError::InvariantViolation(
                    "destination evidence missing on aggregate".to_string()
                )
            )?;

        let mut tx = self.pool.begin().await?;
        self.update_transfer_header_tx(&mut tx, transfer).await?;

        sqlx
            ::query(
                r#"
            INSERT INTO destination_evidence (
                transfer_id,
                destination_tx_hash,
                destination_chain,
                recipient,
                asset,
                quantity,
                observed_at,
                confirmed_at,
                note
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (transfer_id)
            DO UPDATE SET
                destination_tx_hash = EXCLUDED.destination_tx_hash,
                destination_chain = EXCLUDED.destination_chain,
                recipient = EXCLUDED.recipient,
                asset = EXCLUDED.asset,
                quantity = EXCLUDED.quantity,
                observed_at = EXCLUDED.observed_at,
                confirmed_at = EXCLUDED.confirmed_at,
                note = EXCLUDED.note
            "#
            )
            .bind(transfer.id)
            .bind(destination.destination_tx_hash.0.as_str())
            .bind(destination.destination_chain.0.as_str())
            .bind(destination.recipient.0.as_str())
            .bind(destination.asset.0.as_str())
            .bind(destination.quantity.as_str())
            .bind(destination.observed_at)
            .bind(destination.confirmed_at)
            .bind(destination.note.as_deref())
            .execute(&mut *tx).await?;

        maybe_insert_latest_state_transition_tx(&mut tx, transfer).await?;

        insert_audit_event_tx(
            &mut tx,
            Some(transfer.id),
            "destination_evidence_recorded",
            json!({
                "destination_tx_hash": destination.destination_tx_hash.0,
                "destination_chain": destination.destination_chain.0,
                "recipient": destination.recipient.0,
                "asset": destination.asset.0,
                "quantity": destination.quantity,
                "observed_at": destination.observed_at,
                "confirmed_at": destination.confirmed_at,
            }),
            transfer.updated_at
        ).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn save_reconciliation_run(
        &self,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        let recon = transfer.reconciliation
            .as_ref()
            .ok_or_else(||
                PersistenceError::InvariantViolation(
                    "reconciliation result missing on aggregate".to_string()
                )
            )?;

        let mut tx = self.pool.begin().await?;
        self.update_transfer_header_tx(&mut tx, transfer).await?;

        sqlx
            ::query(
                r#"
            INSERT INTO reconciliation_runs (
                transfer_id,
                compared_at,
                internal_state,
                source_status,
                relay_status,
                destination_status,
                comparison_result,
                decision,
                evidence,
                notes
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#
            )
            .bind(transfer.id)
            .bind(recon.compared_at)
            .bind(state_to_db(recon.internal_state))
            .bind(recon.source_status.as_str())
            .bind(recon.relay_status.as_str())
            .bind(recon.destination_status.as_str())
            .bind(reconciliation_comparison_to_db(recon.comparison))
            .bind(reconciliation_decision_to_db(recon.decision))
            .bind(sqlx::types::Json(serde_json::to_value(&recon.evidence)?))
            .bind(recon.note.as_deref())
            .execute(&mut *tx).await?;

        maybe_insert_latest_state_transition_tx(&mut tx, transfer).await?;

        insert_audit_event_tx(
            &mut tx,
            Some(transfer.id),
            "reconciliation_run_recorded",
            json!({
                "compared_at": recon.compared_at,
                "comparison": reconciliation_comparison_to_db(recon.comparison),
                "decision": reconciliation_decision_to_db(recon.decision),
            }),
            transfer.updated_at
        ).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn save_exception_case(
        &self,
        input: SaveExceptionCaseInput
    ) -> Result<(), PersistenceError> {
        let mut tx = self.pool.begin().await?;

        sqlx
            ::query(
                r#"
            INSERT INTO exception_cases (
                transfer_id,
                exception_classification,
                case_status,
                note,
                created_at,
                resolved_at
            )
            VALUES ($1,$2,$3,$4,$5,$6)
            "#
            )
            .bind(input.transfer_id)
            .bind(exception_to_db(&input.classification))
            .bind(input.case_status.as_str())
            .bind(input.note.as_deref())
            .bind(input.created_at)
            .bind(input.resolved_at)
            .execute(&mut *tx).await?;

        insert_audit_event_tx(
            &mut tx,
            Some(input.transfer_id),
            "exception_case_recorded",
            json!({
                "classification": exception_to_db(&input.classification),
                "case_status": input.case_status,
                "resolved_at": input.resolved_at,
            }),
            input.created_at
        ).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_receipt_by_id(
        &self,
        transfer_id: TransferId
    ) -> Result<ComputedTransferReceipt, PersistenceError> {
        let transfer = self.get_transfer_by_id(transfer_id).await?;
        let core = transfer.to_receipt();

        let exception_rows = sqlx
            ::query_as::<_, DbExceptionCaseRow>(
                r#"
            SELECT
                transfer_id,
                exception_classification,
                case_status,
                note,
                created_at,
                resolved_at
            FROM exception_cases
            WHERE transfer_id = $1
            ORDER BY created_at ASC
            "#
            )
            .bind(transfer_id)
            .fetch_all(&self.pool).await?;

        let audit_rows = sqlx
            ::query_as::<_, DbAuditEventRow>(
                r#"
            SELECT transfer_id, event_type, payload, created_at
            FROM audit_events
            WHERE transfer_id = $1
            ORDER BY created_at ASC
            "#
            )
            .bind(transfer_id)
            .fetch_all(&self.pool).await?;

        Ok(ComputedTransferReceipt {
            core,
            exception_cases: exception_rows
                .into_iter()
                .map(|row| {
                    Ok(StoredExceptionCase {
                        transfer_id: row.transfer_id,
                        classification: exception_from_db(&row.exception_classification)?,
                        case_status: row.case_status,
                        note: row.note,
                        created_at: row.created_at,
                        resolved_at: row.resolved_at,
                    })
                })
                .collect::<Result<Vec<_>, PersistenceError>>()?,
            audit_events: audit_rows
                .into_iter()
                .map(|row| StoredAuditEvent {
                    transfer_id: row.transfer_id,
                    event_type: row.event_type,
                    payload: row.payload.0,
                    created_at: row.created_at,
                })
                .collect(),
        })
    }

    async fn load_transfer_by_id_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        transfer_id: TransferId
    ) -> Result<TransferIntent, PersistenceError> {
        let transfer_row = sqlx
            ::query_as::<_, DbTransferIntentRow>(
                r#"
            SELECT
                id,
                client_transfer_reference,
                source_chain,
                destination_chain,
                source_address,
                destination_recipient,
                asset_id,
                quantity,
                state,
                latest_failure_classification,
                latest_exception_classification,
                created_at,
                updated_at
            FROM transfer_intents
            WHERE id = $1
            "#
            )
            .bind(transfer_id)
            .fetch_optional(&mut **tx).await?
            .ok_or(PersistenceError::TransferNotFound(transfer_id))?;

        let idempotency_key = self.load_idempotency_key_tx(tx, transfer_id).await?;

        let source_row = sqlx
            ::query_as::<_, DbSourceEvidenceRow>(
                r#"
            SELECT transfer_id, source_tx_hash, observed_at, confirmed_at, note
            FROM source_evidence
            WHERE transfer_id = $1
            "#
            )
            .bind(transfer_id)
            .fetch_optional(&mut **tx).await?;

        let relay_rows = sqlx
            ::query_as::<_, DbRelayAttemptRow>(
                r#"
            SELECT
                transfer_id,
                attempt_no,
                started_at,
                ended_at,
                outcome_kind,
                error_category,
                result_reason,
                relay_reference,
                note
            FROM relay_attempts
            WHERE transfer_id = $1
            ORDER BY attempt_no ASC
            "#
            )
            .bind(transfer_id)
            .fetch_all(&mut **tx).await?;

        let destination_row = sqlx
            ::query_as::<_, DbDestinationEvidenceRow>(
                r#"
            SELECT
                transfer_id,
                destination_tx_hash,
                destination_chain,
                recipient,
                asset,
                quantity,
                observed_at,
                confirmed_at,
                note
            FROM destination_evidence
            WHERE transfer_id = $1
            "#
            )
            .bind(transfer_id)
            .fetch_optional(&mut **tx).await?;

        let recon_row = sqlx
            ::query_as::<_, DbReconciliationRunRow>(
                r#"
            SELECT
                transfer_id,
                compared_at,
                internal_state,
                source_status,
                relay_status,
                destination_status,
                comparison_result,
                decision,
                evidence,
                notes
            FROM reconciliation_runs
            WHERE transfer_id = $1
            ORDER BY compared_at DESC
            LIMIT 1
            "#
            )
            .bind(transfer_id)
            .fetch_optional(&mut **tx).await?;

        let audit_rows = sqlx
            ::query_as::<_, DbAuditEventRow>(
                r#"
            SELECT transfer_id, event_type, payload, created_at
            FROM audit_events
            WHERE transfer_id = $1
            ORDER BY created_at ASC
            "#
            )
            .bind(transfer_id)
            .fetch_all(&mut **tx).await?;

        let source_evidence = source_row.map(map_source_evidence_row);
        let relay_attempts = relay_rows
            .into_iter()
            .map(map_relay_attempt_row)
            .collect::<Result<Vec<_>, _>>()?;
        let destination_evidence = destination_row.map(map_destination_evidence_row);
        let reconciliation = match recon_row {
            Some(row) => Some(map_reconciliation_row(row)?),
            None => None,
        };
        let timeline = audit_rows
            .iter()
            .filter(|row| row.event_type == "state_transition")
            .map(map_timeline_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(TransferIntent {
            id: transfer_row.id,
            client_transfer_reference: ClientTransferReference(
                transfer_row.client_transfer_reference
            ),
            idempotency_key,
            source_chain: ChainId(transfer_row.source_chain),
            destination_chain: ChainId(transfer_row.destination_chain),
            source_address: Address(transfer_row.source_address),
            destination_recipient: Address(transfer_row.destination_recipient),
            asset_amount: AssetAmount::new(transfer_row.quantity, transfer_row.asset_id),
            state: state_from_db(&transfer_row.state)?,
            latest_failure: match transfer_row.latest_failure_classification {
                Some(value) => Some(failure_from_db(&value)?),
                None => None,
            },
            latest_exception: match transfer_row.latest_exception_classification {
                Some(value) => Some(exception_from_db(&value)?),
                None => None,
            },
            source_evidence,
            relay_attempts,
            destination_evidence,
            reconciliation,
            timeline,
            created_at: transfer_row.created_at,
            updated_at: transfer_row.updated_at,
        })
    }

    async fn load_idempotency_key_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        transfer_id: TransferId
    ) -> Result<IdempotencyKey, PersistenceError> {
        let row = sqlx
            ::query_as::<_, DbIdempotencyKeyRow>(
                r#"
            SELECT scope, idempotency_key, transfer_id, request_fingerprint, created_at
            FROM idempotency_keys
            WHERE transfer_id = $1
            ORDER BY created_at ASC
            LIMIT 1
            "#
            )
            .bind(transfer_id)
            .fetch_one(&mut **tx).await?;

        Ok(IdempotencyKey(row.idempotency_key))
    }

    async fn update_transfer_header_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        transfer: &TransferIntent
    ) -> Result<(), PersistenceError> {
        sqlx
            ::query(
                r#"
            UPDATE transfer_intents
            SET
                state = $2,
                latest_failure_classification = $3,
                latest_exception_classification = $4,
                updated_at = $5
            WHERE id = $1
            "#
            )
            .bind(transfer.id)
            .bind(state_to_db(transfer.state))
            .bind(transfer.latest_failure.as_ref().map(failure_to_db))
            .bind(transfer.latest_exception.as_ref().map(exception_to_db))
            .bind(transfer.updated_at)
            .execute(&mut **tx).await?;

        Ok(())
    }
}

async fn maybe_insert_latest_state_transition_tx(
    tx: &mut Transaction<'_, Postgres>,
    transfer: &TransferIntent
) -> Result<(), PersistenceError> {
    if let Some(last) = transfer.timeline.last() {
        if last.at == transfer.updated_at && last.state == transfer.state {
            insert_state_transition_audit_tx(tx, transfer.id, last).await?;
        }
    }
    Ok(())
}

async fn insert_state_transition_audit_tx(
    tx: &mut Transaction<'_, Postgres>,
    transfer_id: TransferId,
    entry: &ReceiptTimelineEntry
) -> Result<(), PersistenceError> {
    insert_audit_event_tx(
        tx,
        Some(transfer_id),
        "state_transition",
        json!({
            "state": state_to_db(entry.state),
            "note": entry.note
        }),
        entry.at
    ).await
}

async fn insert_audit_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    transfer_id: Option<TransferId>,
    event_type: &str,
    payload: Value,
    created_at: DateTime<Utc>
) -> Result<(), PersistenceError> {
    sqlx
        ::query(
            r#"
        INSERT INTO audit_events (transfer_id, event_type, payload, created_at)
        VALUES ($1,$2,$3,$4)
        "#
        )
        .bind(transfer_id)
        .bind(event_type)
        .bind(sqlx::types::Json(payload))
        .bind(created_at)
        .execute(&mut **tx).await?;

    Ok(())
}

fn map_source_evidence_row(row: DbSourceEvidenceRow) -> SourceEvidence {
    SourceEvidence {
        source_tx_hash: TxHash(row.source_tx_hash),
        observed_at: row.observed_at,
        confirmed_at: row.confirmed_at,
        note: row.note,
    }
}

fn map_relay_attempt_row(row: DbRelayAttemptRow) -> Result<RelayAttempt, PersistenceError> {
    let outcome = match row.outcome_kind {
        None => None,
        Some(kind) =>
            Some(
                relay_attempt_outcome_from_db(
                    &kind,
                    row.error_category.as_deref(),
                    row.result_reason.as_deref()
                )?
            ),
    };

    Ok(RelayAttempt {
        attempt_no: row.attempt_no as u32,
        started_at: row.started_at,
        ended_at: row.ended_at,
        outcome,
        relay_reference: row.relay_reference.map(RelayReference),
        note: row.note,
    })
}

fn map_destination_evidence_row(row: DbDestinationEvidenceRow) -> DestinationEvidence {
    DestinationEvidence {
        destination_tx_hash: TxHash(row.destination_tx_hash),
        destination_chain: ChainId(row.destination_chain),
        recipient: Address(row.recipient),
        asset: domain::AssetId(row.asset),
        quantity: row.quantity,
        observed_at: row.observed_at,
        confirmed_at: row.confirmed_at,
        note: row.note,
    }
}

fn map_reconciliation_row(
    row: DbReconciliationRunRow
) -> Result<ReconciliationResult, PersistenceError> {
    let evidence: EvidenceSource = serde_json::from_value(row.evidence.0)?;

    Ok(ReconciliationResult {
        compared_at: row.compared_at,
        internal_state: state_from_db(&row.internal_state)?,
        source_status: row.source_status,
        relay_status: row.relay_status,
        destination_status: row.destination_status,
        comparison: reconciliation_comparison_from_db(&row.comparison_result)?,
        decision: reconciliation_decision_from_db(&row.decision)?,
        evidence,
        note: row.notes,
    })
}

fn map_timeline_row(row: &DbAuditEventRow) -> Result<ReceiptTimelineEntry, PersistenceError> {
    let payload = &row.payload.0;

    let state = payload
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            PersistenceError::InvariantViolation(
                "state_transition audit event missing state".to_string()
            )
        })?;

    let note = payload
        .get("note")
        .and_then(Value::as_str)
        .map(|value| value.to_string());

    Ok(ReceiptTimelineEntry {
        state: state_from_db(state)?,
        at: row.created_at,
        note,
    })
}

fn state_to_db(state: TransferState) -> &'static str {
    match state {
        TransferState::Received => "received",
        TransferState::Validated => "validated",
        TransferState::Rejected => "rejected",
        TransferState::Queued => "queued",
        TransferState::SourceObserved => "source_observed",
        TransferState::SourceConfirmed => "source_confirmed",
        TransferState::RelayInProgress => "relay_in_progress",
        TransferState::RelayUnknown => "relay_unknown",
        TransferState::DestinationPending => "destination_pending",
        TransferState::Settled => "settled",
        TransferState::MismatchDetected => "mismatch_detected",
        TransferState::Reconciling => "reconciling",
        TransferState::ManualReview => "manual_review",
        TransferState::FailedTerminal => "failed_terminal",
        TransferState::DeadLettered => "dead_lettered",
    }
}

fn state_from_db(value: &str) -> Result<TransferState, PersistenceError> {
    match value {
        "received" => Ok(TransferState::Received),
        "validated" => Ok(TransferState::Validated),
        "rejected" => Ok(TransferState::Rejected),
        "queued" => Ok(TransferState::Queued),
        "source_observed" => Ok(TransferState::SourceObserved),
        "source_confirmed" => Ok(TransferState::SourceConfirmed),
        "relay_in_progress" => Ok(TransferState::RelayInProgress),
        "relay_unknown" => Ok(TransferState::RelayUnknown),
        "destination_pending" => Ok(TransferState::DestinationPending),
        "settled" => Ok(TransferState::Settled),
        "mismatch_detected" => Ok(TransferState::MismatchDetected),
        "reconciling" => Ok(TransferState::Reconciling),
        "manual_review" => Ok(TransferState::ManualReview),
        "failed_terminal" => Ok(TransferState::FailedTerminal),
        "dead_lettered" => Ok(TransferState::DeadLettered),
        other => Err(PersistenceError::InvalidPersistedState(other.to_string())),
    }
}

fn failure_to_db(failure: &FailureClassification) -> &'static str {
    match failure {
        FailureClassification::Validation => "validation",
        FailureClassification::DuplicateRequest => "duplicate_request",
        FailureClassification::RetryableRelayInfrastructure => "retryable_relay_infrastructure",
        FailureClassification::TerminalRelayFailure => "terminal_relay_failure",
        FailureClassification::UnknownRelayOutcome => "unknown_relay_outcome",
        FailureClassification::SourceEvidenceMissing => "source_evidence_missing",
        FailureClassification::DestinationEvidenceMissing => "destination_evidence_missing",
        FailureClassification::DestinationMismatch => "destination_mismatch",
        FailureClassification::ReconciliationMismatch => "reconciliation_mismatch",
    }
}

fn failure_from_db(value: &str) -> Result<FailureClassification, PersistenceError> {
    match value {
        "validation" => Ok(FailureClassification::Validation),
        "duplicate_request" => Ok(FailureClassification::DuplicateRequest),
        "retryable_relay_infrastructure" => Ok(FailureClassification::RetryableRelayInfrastructure),
        "terminal_relay_failure" => Ok(FailureClassification::TerminalRelayFailure),
        "unknown_relay_outcome" => Ok(FailureClassification::UnknownRelayOutcome),
        "source_evidence_missing" => Ok(FailureClassification::SourceEvidenceMissing),
        "destination_evidence_missing" => Ok(FailureClassification::DestinationEvidenceMissing),
        "destination_mismatch" => Ok(FailureClassification::DestinationMismatch),
        "reconciliation_mismatch" => Ok(FailureClassification::ReconciliationMismatch),
        other => Err(PersistenceError::InvalidFailureClassification(other.to_string())),
    }
}

fn exception_to_db(exception: &ExceptionClassification) -> &'static str {
    match exception {
        ExceptionClassification::DestinationMissing => "destination_missing",
        ExceptionClassification::DestinationMismatch => "destination_mismatch",
        ExceptionClassification::AmbiguousRelayOutcome => "ambiguous_relay_outcome",
        ExceptionClassification::DuplicateRelayAttempt => "duplicate_relay_attempt",
        ExceptionClassification::StalePendingTransfer => "stale_pending_transfer",
        ExceptionClassification::SourceMissing => "source_missing",
        ExceptionClassification::ManualReviewRequired => "manual_review_required",
    }
}

fn exception_from_db(value: &str) -> Result<ExceptionClassification, PersistenceError> {
    match value {
        "destination_missing" => Ok(ExceptionClassification::DestinationMissing),
        "destination_mismatch" => Ok(ExceptionClassification::DestinationMismatch),
        "ambiguous_relay_outcome" => Ok(ExceptionClassification::AmbiguousRelayOutcome),
        "duplicate_relay_attempt" => Ok(ExceptionClassification::DuplicateRelayAttempt),
        "stale_pending_transfer" => Ok(ExceptionClassification::StalePendingTransfer),
        "source_missing" => Ok(ExceptionClassification::SourceMissing),
        "manual_review_required" => Ok(ExceptionClassification::ManualReviewRequired),
        other => Err(PersistenceError::InvalidExceptionClassification(other.to_string())),
    }
}

fn relay_attempt_outcome_to_db(
    outcome: &RelayAttemptOutcome
) -> (&'static str, Option<&'static str>, Option<String>) {
    match outcome {
        RelayAttemptOutcome::Accepted => ("accepted", None, None),
        RelayAttemptOutcome::RetryableFailure { classification, reason } =>
            ("retryable_failure", Some(failure_to_db(classification)), Some(reason.clone())),
        RelayAttemptOutcome::TerminalFailure { classification, reason } =>
            ("terminal_failure", Some(failure_to_db(classification)), Some(reason.clone())),
        RelayAttemptOutcome::UnknownOutcome { classification, reason } =>
            ("unknown_outcome", Some(failure_to_db(classification)), Some(reason.clone())),
    }
}

fn relay_attempt_outcome_from_db(
    outcome_kind: &str,
    error_category: Option<&str>,
    result_reason: Option<&str>
) -> Result<RelayAttemptOutcome, PersistenceError> {
    match outcome_kind {
        "accepted" => Ok(RelayAttemptOutcome::Accepted),
        "retryable_failure" =>
            Ok(RelayAttemptOutcome::RetryableFailure {
                classification: failure_from_db(
                    error_category.ok_or_else(|| {
                        PersistenceError::InvariantViolation(
                            "retryable relay outcome missing error_category".to_string()
                        )
                    })?
                )?,
                reason: result_reason.unwrap_or_default().to_string(),
            }),
        "terminal_failure" =>
            Ok(RelayAttemptOutcome::TerminalFailure {
                classification: failure_from_db(
                    error_category.ok_or_else(|| {
                        PersistenceError::InvariantViolation(
                            "terminal relay outcome missing error_category".to_string()
                        )
                    })?
                )?,
                reason: result_reason.unwrap_or_default().to_string(),
            }),
        "unknown_outcome" =>
            Ok(RelayAttemptOutcome::UnknownOutcome {
                classification: failure_from_db(
                    error_category.ok_or_else(|| {
                        PersistenceError::InvariantViolation(
                            "unknown relay outcome missing error_category".to_string()
                        )
                    })?
                )?,
                reason: result_reason.unwrap_or_default().to_string(),
            }),
        other => Err(PersistenceError::InvalidRelayAttemptOutcome(other.to_string())),
    }
}

fn reconciliation_comparison_to_db(comparison: ReconciliationComparison) -> &'static str {
    match comparison {
        ReconciliationComparison::Matched => "matched",
        ReconciliationComparison::Mismatch => "mismatch",
        ReconciliationComparison::Unresolved => "unresolved",
    }
}

fn reconciliation_comparison_from_db(
    value: &str
) -> Result<ReconciliationComparison, PersistenceError> {
    match value {
        "matched" => Ok(ReconciliationComparison::Matched),
        "mismatch" => Ok(ReconciliationComparison::Mismatch),
        "unresolved" => Ok(ReconciliationComparison::Unresolved),
        other => Err(PersistenceError::InvalidReconciliationComparison(other.to_string())),
    }
}

fn reconciliation_decision_to_db(decision: ReconciliationDecision) -> &'static str {
    match decision {
        ReconciliationDecision::ConfirmSettled => "confirm_settled",
        ReconciliationDecision::KeepPending => "keep_pending",
        ReconciliationDecision::MarkMismatch => "mark_mismatch",
        ReconciliationDecision::EscalateManualReview => "escalate_manual_review",
    }
}

fn reconciliation_decision_from_db(
    value: &str
) -> Result<ReconciliationDecision, PersistenceError> {
    match value {
        "confirm_settled" => Ok(ReconciliationDecision::ConfirmSettled),
        "keep_pending" => Ok(ReconciliationDecision::KeepPending),
        "mark_mismatch" => Ok(ReconciliationDecision::MarkMismatch),
        "escalate_manual_review" => Ok(ReconciliationDecision::EscalateManualReview),
        other => Err(PersistenceError::InvalidReconciliationDecision(other.to_string())),
    }
}
