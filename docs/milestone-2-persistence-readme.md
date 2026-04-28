# Milestone 2 — Database Schema and Persistence Layer

## Goal

Create the durable state model in Postgres for the Cross-Chain Transfer Reconciliation Gateway.

This milestone is where the system stops being only a domain model and becomes a durable truth system.

The purpose of this milestone is not just to save transfer records.

The purpose is to preserve:

- transfer lineage
- idempotency truth
- source-chain evidence
- relay attempt history
- destination-chain evidence
- reconciliation history
- exception case history
- auditability of lifecycle transitions

This is where truth lives.

---

## Why this milestone exists

This project is not a generic blockchain app.

It is a reliability-first truth and reconciliation system around cross-chain transfer execution.

That means the persistence layer must support hard questions such as:

- was this transfer already accepted?
- is this a replay of the same business command or a conflicting duplicate?
- what source evidence do we have?
- how many relay attempts happened?
- did the relay outcome become ambiguous?
- what destination evidence exists?
- do intended truth and observed truth match?
- did reconciliation run?
- is there an explicit exception case?
- can an operator reconstruct what happened?

This milestone exists to make those answers durable.

---

## What we are building in this milestone

We are building the Postgres-backed truth model for:

- `transfer_intents`
- `idempotency_keys`
- `source_evidence`
- `relay_attempts`
- `destination_evidence`
- `reconciliation_runs`
- `exception_cases`
- `audit_events`

We are also building the Rust persistence layer that:

- wraps SQLx/Postgres
- uses transactions where consistency matters
- persists transfer lineage safely with idempotency
- stores source and destination evidence durably
- stores relay attempt history durably
- stores reconciliation runs durably
- stores exception cases durably
- computes a receipt read model from normalized truth tables

---

## Design choice for Milestone 2

For this milestone we use a **computed receipt read model**, not stored receipt snapshots.

Why?

Because the execution and reconciliation flows are still evolving.

At this stage, correctness of writes matters more than snapshot optimization.

So we keep the truth normalized and compute the transfer receipt by loading:

- transfer header
- idempotency key
- source evidence
- relay attempts
- destination evidence
- latest reconciliation result
- state transition timeline from audit events
- exception cases
- audit history

Snapshotting can come later when the write paths are stable.

---

## Core persistence principles

### 1. `transfer_intents` is the current header, not the whole story
This table stores the current durable identity and current state of the transfer lineage.

It is not the entire truth by itself.

The full story lives across:
- source evidence
- relay attempts
- destination evidence
- reconciliation runs
- exception cases
- audit events

### 2. `idempotency_keys` owns duplicate-request truth
One idempotency key within one scope must map to one transfer lineage.

This is how retried HTTP requests do not become duplicate cross-chain transfer lineages.

### 3. source, relay, and destination evidence stay separate
This is one of the most important design choices.

The system should not collapse:
- source truth
- relay truth
- destination truth

into one vague status row.

### 4. reconciliation is first-class
This system exists partly because truth surfaces can diverge.

So reconciliation history must be durable.

### 5. exception cases are durable operator artifacts
When automation is no longer enough, the system should create durable operator-facing exception records.

### 6. audit trail is part of truth
State transitions and major events are stored in `audit_events` so the operator story can be reconstructed.

---

## Table-by-table purpose

### `transfer_intents`
Stores the current durable identity and state of the transfer lineage.

Important fields:
- internal transfer id
- client transfer reference
- source chain
- destination chain
- source address
- destination recipient
- asset id
- quantity
- current state
- latest failure classification
- latest exception classification
- created_at
- updated_at

### `idempotency_keys`
Stores duplicate-request truth.

Important fields:
- scope
- idempotency key
- linked transfer id
- request fingerprint
- created_at

### `source_evidence`
Stores the current source-side proof for a transfer.

Important fields:
- source tx hash
- observed_at
- confirmed_at
- note

### `relay_attempts`
Stores each relay attempt separately.

Important fields:
- transfer id
- attempt number
- started_at
- ended_at
- outcome kind
- error category
- reason
- relay reference
- note

This is where relay ambiguity and retry history remain visible.

### `destination_evidence`
Stores the current destination-side proof for a transfer.

Important fields:
- destination tx hash
- destination chain
- recipient
- asset
- quantity
- observed_at
- confirmed_at
- note

### `reconciliation_runs`
Stores reconciliation history.

Important fields:
- transfer id
- compared_at
- internal state
- source status
- relay status
- destination status
- comparison result
- decision
- evidence
- notes

### `exception_cases`
Stores durable operator-facing unresolved or mismatched cases.

Important fields:
- transfer id
- exception classification
- case status
- note
- created_at
- resolved_at

### `audit_events`
Stores the durable timeline and system event trail.

For now this includes:
- state transitions
- transfer created events
- source evidence recorded
- relay attempt started
- relay attempt finished
- destination evidence recorded
- reconciliation run recorded
- exception case recorded

---

## What done means for this milestone

Milestone 2 is done when:

- migrations run cleanly
- all core tables exist
- repository methods are defined
- idempotent create is transaction-safe
- multi-table writes use transactions
- source evidence can be persisted durably
- relay attempt history can be persisted durably
- destination evidence can be persisted durably
- reconciliation runs can be persisted durably
- exception cases can be persisted durably
- a computed receipt read model can be loaded from normalized truth tables

That is what “done” means here.

Not just “database works.”

Done means:
**cross-chain transfer truth has a durable home.**

---

## Repository methods in this milestone

### `create_transfer_with_idempotency`
Creates a new transfer lineage and idempotency record in one transaction.

Behavior:
- checks `(scope, idempotency_key)`
- returns existing lineage when fingerprint matches
- rejects conflicting idempotency reuse
- inserts transfer header
- inserts idempotency record
- inserts initial state transition audits

### `get_transfer_by_id`
Loads:
- current transfer header
- idempotency key
- source evidence
- relay attempts
- destination evidence
- latest reconciliation result
- state transition timeline

### `save_source_evidence`
Persists source proof and syncs the aggregate header.

### `save_relay_attempt_started`
Persists a newly started relay attempt and syncs lifecycle state.

### `save_relay_attempt_finished`
Persists the relay outcome and syncs lifecycle state.

### `save_destination_evidence`
Persists destination proof and syncs lifecycle state.

### `save_reconciliation_run`
Persists a reconciliation result and syncs lifecycle state.

### `save_exception_case`
Persists an explicit operator-facing exception case.

### `get_receipt_by_id`
Computes a transfer receipt plus exception cases and audit events from normalized storage.

---

## Important invariants supported here

### One idempotency key maps to one transfer lineage
Supported by:
- unique constraint on `(scope, idempotency_key)`
- conflict handling in repository code

### Source confirmation is not destination completion
Supported by:
- separate `source_evidence`
- separate `destination_evidence`
- current header state in `transfer_intents`

### Every relay attempt must be durably recorded
Supported by:
- `relay_attempts`
- unique `(transfer_id, attempt_no)`

### Mismatch must be explicit
Supported by:
- `latest_exception_classification`
- `exception_cases`
- `reconciliation_runs`

### State transitions must be reconstructable
Supported by:
- `audit_events`
- `state_transition` event type

---

## Why this structure matches our project goal

Our goal is not to build a generic blockchain CRUD backend.

Our goal is to make one thing obvious:

this system is built for truth, reliability, execution safety, reconciliation, coordination, and operator understanding under cross-chain ambiguity.

This persistence model supports that directly.

### Durable lineage truth
`transfer_intents` + `idempotency_keys`

### Source truth
`source_evidence`

### Relay truth
`relay_attempts`

### Destination truth
`destination_evidence`

### Reconciliation truth
`reconciliation_runs`

### Exception truth
`exception_cases`

### Operator timeline
`audit_events`

This is not accidental schema design.

This is reliability architecture.

---

## What we are not doing in this milestone

We are not yet building:

- HTTP handlers
- relay worker orchestration
- source/destination chain watchers
- reconciliation scheduler
- exception workflow UI
- receipt snapshots
- retry scheduler

Those come after this durable truth layer.

This milestone is focused on:
**schema, consistency, evidence, and repository boundaries.**

---

## Summary

Milestone 2 is the durable truth layer of the Cross-Chain Transfer Reconciliation Gateway.

It exists so that:

- accepted transfer intent is durable
- duplicate requests are handled safely
- source evidence is stored
- relay attempts are visible
- destination evidence is stored
- reconciliation is first-class
- exception cases are durable
- a receipt can be computed from normalized evidence

That is exactly aligned with the project’s niche:

**trust, reliability, execution safety, reconciliation, coordination, and operational truth for cross-chain transfer execution.**