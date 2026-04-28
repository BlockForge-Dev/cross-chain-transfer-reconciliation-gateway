# Milestone 1 Invariants

These invariants define the core truth rules of the Cross-Chain Transfer Reconciliation Gateway.

---

## Invariant 1
**One idempotency key maps to one transfer lineage.**

Meaning:
- the same business request must not create multiple transfer lineages
- persistence will enforce this later
- milestone 1 defines the rule in the domain language

---

## Invariant 2
**Source confirmation is not destination completion.**

Meaning:
- source-side proof is not enough to mark a transfer as settled
- source truth and destination truth are separate states
- the lifecycle must preserve that distinction

---

## Invariant 3
**Unknown relay outcome cannot be finalized without external evidence.**

Meaning:
- relay timeout or ambiguous relay outcome must remain explicit
- accepted resolution evidence includes:
  - destination chain observation
  - source chain observation
  - relay status check
  - manual operator decision
- internal assumption alone is not enough

---

## Invariant 4
**Destination settlement must correlate to intended transfer details.**

Meaning:
- destination evidence must match:
  - destination chain
  - recipient
  - asset
  - quantity
- if not, the system should move into mismatch handling rather than silently accepting the evidence

---

## Invariant 5
**Every relay attempt must be durably recorded.**

Meaning:
- every relay try is part of the business truth
- no hidden relay execution should exist
- persistence will enforce this later

---

## Invariant 6
**Mismatch must be explicit, not silently patched.**

Meaning:
- if observed truth and intended truth diverge, the system should enter mismatch or manual review states explicitly
- operators should be able to inspect the case later

---

## Invariant 7
**State transitions must be explicit and guarded.**

Meaning:
- lifecycle state should not be assigned freely across the codebase
- state changes should happen through domain-owned methods

---

## Invariant 8
**Terminal states should not silently re-enter relay flow.**

Meaning:
- once a transfer is settled, failed terminal, or dead-lettered, new relay attempts should not begin automatically