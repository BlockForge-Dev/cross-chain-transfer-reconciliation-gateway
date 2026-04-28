# Cross-Chain Transfer Reconciliation Gateway

A reliability-first truth, reconciliation, and recovery layer for cross-chain transfer execution.

This project is not a full bridge protocol.

It is a focused infrastructure slice built to answer a harder and more important question:

**When cross-chain transfer truth diverges across intent, source chain, relay attempts, and destination chain, how do we detect it, classify it, explain it, and recover safely?**

---

## Why this project exists

Cross-chain transfer flows are naturally failure-heavy.

A transfer may begin on a source chain, move through relay infrastructure, and settle on a destination chain. In between, truth can diverge.

Examples:
- source side confirms, destination side is missing
- relay attempt times out and outcome becomes ambiguous
- destination settlement happens late
- duplicate relay attempt occurs
- observed destination settlement does not match intended recipient, asset, or amount
- internal system truth and on-chain truth drift apart

This project exists to preserve durable truth and make those cases operationally understandable.

---

## Project goal

The goal is not to build a full bridge product.

The goal is to build the **failure-sensitive truth and reconciliation layer** around cross-chain transfer execution.

This repo should prove strength in:
- reliability
- execution safety
- evidence modeling
- ambiguity handling
- reconciliation
- operator truth
- distributed systems thinking

---

## Critical action

A cross-chain transfer intent is accepted and tracked from request to observed completion or unresolved exception.

---

## Main failure modes

- duplicate transfer request
- source confirmation without destination settlement
- relay timeout / unknown outcome
- duplicate relay attempt
- destination mismatch
- stale pending transfer
- internal truth vs chain truth divergence

---

## System guarantees

This system aims to guarantee:

- one idempotency key maps to one transfer lineage
- source confirmation is not treated as destination completion
- relay ambiguity is represented explicitly
- destination settlement must correlate to intended transfer details
- every relay attempt and evidence update is durable
- mismatch becomes explicit, not silently ignored
- reconciliation decisions are queryable

---

## Source of truth

Durable truth lives in Postgres.

No single log line, relayer memory, or one-off chain observation is treated as the whole truth.

The full story is composed from:
- transfer intent
- source evidence
- relay attempts
- destination evidence
- reconciliation runs
- exception cases
- audit timeline

---

## Operator visibility

A human operator should be able to inspect:
- what was requested
- what source evidence exists
- what relay attempts happened
- what destination evidence exists
- what is still ambiguous
- what reconciliation concluded
- what action is safe next

---

## Recovery path

The system should support controlled recovery paths such as:
- recheck
- replay when safe
- escalate to manual review
- mark resolved with evidence
- keep unresolved until more truth arrives

---

## Repository shape

```text
apps/
  api/           -> request ingestion and query surface
  worker/        -> relay / tracking workers
  reconciler/    -> truth comparison and case classification
  operator/      -> operator-facing read surface

crates/
  domain/                -> core domain language and invariants
  application/           -> use cases and orchestration
  persistence/           -> database truth layer
  source_tracking/       -> source-side evidence tracking
  relay_tracking/        -> relay attempt tracking
  destination_tracking/  -> destination evidence tracking
  reconciliation/        -> truth comparison logic
  exceptions/            -> exception case modeling
  receipts/              -> transfer timeline / receipt assembly
  shared/                -> shared types / utilities
  config/                -> configuration loading