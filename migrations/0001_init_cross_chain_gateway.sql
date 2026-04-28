BEGIN;

CREATE TABLE IF NOT EXISTS transfer_intents (
    id UUID PRIMARY KEY,
    client_transfer_reference TEXT NOT NULL,
    source_chain TEXT NOT NULL CHECK (length(trim(source_chain)) > 0),
    destination_chain TEXT NOT NULL CHECK (length(trim(destination_chain)) > 0),
    source_address TEXT NOT NULL CHECK (length(trim(source_address)) > 0),
    destination_recipient TEXT NOT NULL CHECK (length(trim(destination_recipient)) > 0),
    asset_id TEXT NOT NULL CHECK (length(trim(asset_id)) > 0),
    quantity TEXT NOT NULL CHECK (length(trim(quantity)) > 0),
    state TEXT NOT NULL,
    latest_failure_classification TEXT NULL,
    latest_exception_classification TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transfer_intents_state
    ON transfer_intents (state);

CREATE INDEX IF NOT EXISTS idx_transfer_intents_client_reference
    ON transfer_intents (client_transfer_reference);

CREATE TABLE IF NOT EXISTS idempotency_keys (
    id BIGSERIAL PRIMARY KEY,
    scope TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    transfer_id UUID NOT NULL REFERENCES transfer_intents(id) ON DELETE RESTRICT,
    request_fingerprint TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE (scope, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_transfer_id
    ON idempotency_keys (transfer_id);

CREATE TABLE IF NOT EXISTS source_evidence (
    transfer_id UUID PRIMARY KEY REFERENCES transfer_intents(id) ON DELETE CASCADE,
    source_tx_hash TEXT NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL,
    confirmed_at TIMESTAMPTZ NULL,
    note TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_source_evidence_source_tx_hash
    ON source_evidence (source_tx_hash);

CREATE TABLE IF NOT EXISTS relay_attempts (
    id BIGSERIAL PRIMARY KEY,
    transfer_id UUID NOT NULL REFERENCES transfer_intents(id) ON DELETE CASCADE,
    attempt_no INTEGER NOT NULL CHECK (attempt_no > 0),
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ NULL,
    outcome_kind TEXT NULL,
    error_category TEXT NULL,
    result_reason TEXT NULL,
    relay_reference TEXT NULL,
    note TEXT NULL,
    UNIQUE (transfer_id, attempt_no)
);

CREATE INDEX IF NOT EXISTS idx_relay_attempts_transfer_id
    ON relay_attempts (transfer_id);

CREATE TABLE IF NOT EXISTS destination_evidence (
    transfer_id UUID PRIMARY KEY REFERENCES transfer_intents(id) ON DELETE CASCADE,
    destination_tx_hash TEXT NOT NULL,
    destination_chain TEXT NOT NULL,
    recipient TEXT NOT NULL,
    asset TEXT NOT NULL,
    quantity TEXT NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL,
    confirmed_at TIMESTAMPTZ NULL,
    note TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_destination_evidence_destination_tx_hash
    ON destination_evidence (destination_tx_hash);

CREATE TABLE IF NOT EXISTS reconciliation_runs (
    id BIGSERIAL PRIMARY KEY,
    transfer_id UUID NOT NULL REFERENCES transfer_intents(id) ON DELETE CASCADE,
    compared_at TIMESTAMPTZ NOT NULL,
    internal_state TEXT NOT NULL,
    source_status TEXT NOT NULL,
    relay_status TEXT NOT NULL,
    destination_status TEXT NOT NULL,
    comparison_result TEXT NOT NULL,
    decision TEXT NOT NULL,
    evidence JSONB NOT NULL,
    notes TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_reconciliation_runs_transfer_id
    ON reconciliation_runs (transfer_id);

CREATE TABLE IF NOT EXISTS exception_cases (
    id BIGSERIAL PRIMARY KEY,
    transfer_id UUID NOT NULL REFERENCES transfer_intents(id) ON DELETE CASCADE,
    exception_classification TEXT NOT NULL,
    case_status TEXT NOT NULL,
    note TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    resolved_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_exception_cases_transfer_id
    ON exception_cases (transfer_id);

CREATE INDEX IF NOT EXISTS idx_exception_cases_status
    ON exception_cases (case_status);

CREATE TABLE IF NOT EXISTS audit_events (
    id BIGSERIAL PRIMARY KEY,
    transfer_id UUID NULL REFERENCES transfer_intents(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_events_transfer_id
    ON audit_events (transfer_id);

CREATE INDEX IF NOT EXISTS idx_audit_events_event_type
    ON audit_events (event_type);

COMMIT;