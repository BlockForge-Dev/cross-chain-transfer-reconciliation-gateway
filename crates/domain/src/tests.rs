use chrono::Utc;

use crate::{
    DestinationEvidence,
    EvidenceSource,
    FailureClassification,
    ReconciliationComparison,
    ReconciliationDecision,
    ReconciliationResult,
    RelayAttemptOutcome,
    TransferIntent,
    TransferState,
};

fn sample_intent() -> TransferIntent {
    TransferIntent::new(
        "transfer_123",
        "idem_123",
        "ethereum",
        "solana",
        "0xabc123",
        "So1anaRecipient111",
        "USDC",
        "1000000",
        Utc::now()
    ).unwrap()
}

#[test]
fn new_transfer_starts_in_received_state() {
    let intent = sample_intent();
    assert_eq!(intent.state, TransferState::Received);
    assert_eq!(intent.timeline.len(), 1);
}

#[test]
fn queue_before_validate_is_rejected() {
    let mut intent = sample_intent();
    let now = Utc::now();

    let result = intent.queue(now);
    assert!(result.is_err());
}

#[test]
fn source_confirmed_is_not_the_same_as_settled() {
    let mut intent = sample_intent();
    let now = Utc::now();

    intent.validate(now).unwrap();
    intent.queue(now).unwrap();
    intent.confirm_source("0xsourcehash", now, Some("source confirmed".into())).unwrap();

    assert_eq!(intent.state, TransferState::SourceConfirmed);
    assert_ne!(intent.state, TransferState::Settled);
}

#[test]
fn relay_cannot_begin_before_source_confirmation() {
    let mut intent = sample_intent();
    let now = Utc::now();

    intent.validate(now).unwrap();
    intent.queue(now).unwrap();

    let result = intent.begin_relay_attempt(now);
    assert!(result.is_err());
}

#[test]
fn happy_path_to_settlement_via_reconciliation_works() {
    let mut intent = sample_intent();
    let now = Utc::now();

    intent.validate(now).unwrap();
    intent.queue(now).unwrap();
    intent.confirm_source("0xsourcehash", now, Some("source confirmed".into())).unwrap();

    intent.begin_relay_attempt(now).unwrap();
    intent
        .finish_current_relay_attempt(
            now,
            RelayAttemptOutcome::Accepted,
            Some("relay_ref_1".into()),
            Some("relay accepted".into())
        )
        .unwrap();

    let destination = DestinationEvidence::confirmed(
        crate::TxHash("0xdestinationhash".into()),
        crate::ChainId("solana".into()),
        crate::Address("So1anaRecipient111".into()),
        crate::AssetId("USDC".into()),
        "1000000".to_string(),
        now,
        now,
        Some("destination observed".into())
    );

    intent.record_destination_evidence(destination, now).unwrap();
    intent.begin_reconciliation(now).unwrap();

    let recon = ReconciliationResult {
        compared_at: now,
        internal_state: intent.state,
        source_status: "confirmed".into(),
        relay_status: "accepted".into(),
        destination_status: "confirmed".into(),
        comparison: ReconciliationComparison::Matched,
        decision: ReconciliationDecision::ConfirmSettled,
        evidence: EvidenceSource::DestinationChainObservation {
            tx_hash: "0xdestinationhash".into(),
        },
        note: Some("all truth surfaces align".into()),
    };

    intent.apply_reconciliation(recon, now).unwrap();

    assert_eq!(intent.state, TransferState::Settled);
}

#[test]
fn relay_unknown_requires_external_evidence_to_resolve() {
    let mut intent = sample_intent();
    let now = Utc::now();

    intent.validate(now).unwrap();
    intent.queue(now).unwrap();
    intent.confirm_source("0xsourcehash", now, None).unwrap();
    intent.begin_relay_attempt(now).unwrap();

    intent
        .finish_current_relay_attempt(
            now,
            RelayAttemptOutcome::UnknownOutcome {
                classification: FailureClassification::UnknownRelayOutcome,
                reason: "timeout after relay submit".into(),
            },
            Some("relay_ref_1".into()),
            Some("ambiguous relay result".into())
        )
        .unwrap();

    let result = intent.resolve_relay_unknown_with_evidence(
        now,
        TransferState::Settled,
        EvidenceSource::InternalValidation,
        Some("should fail".into())
    );

    assert!(result.is_err());
}

#[test]
fn destination_mismatch_becomes_explicit_state() {
    let mut intent = sample_intent();
    let now = Utc::now();

    intent.validate(now).unwrap();
    intent.queue(now).unwrap();
    intent.confirm_source("0xsourcehash", now, None).unwrap();

    let mismatched_destination = DestinationEvidence::observed(
        crate::TxHash("0xbadtx".into()),
        crate::ChainId("solana".into()),
        crate::Address("WrongRecipient999".into()),
        crate::AssetId("USDC".into()),
        "1000000".to_string(),
        now,
        Some("mismatched recipient".into())
    );

    intent.record_destination_evidence(mismatched_destination, now).unwrap();

    assert_eq!(intent.state, TransferState::MismatchDetected);
}

#[test]
fn retryable_relay_failure_returns_to_source_confirmed() {
    let mut intent = sample_intent();
    let now = Utc::now();

    intent.validate(now).unwrap();
    intent.queue(now).unwrap();
    intent.confirm_source("0xsourcehash", now, None).unwrap();

    intent.begin_relay_attempt(now).unwrap();
    intent
        .finish_current_relay_attempt(
            now,
            RelayAttemptOutcome::RetryableFailure {
                classification: FailureClassification::RetryableRelayInfrastructure,
                reason: "temporary relayer outage".into(),
            },
            Some("relay_ref_1".into()),
            Some("safe to retry".into())
        )
        .unwrap();

    assert_eq!(intent.state, TransferState::SourceConfirmed);
}
