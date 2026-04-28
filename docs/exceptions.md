# Exception Model Draft

## Purpose

Exception cases are the durable operator-facing representation of unresolved or mismatched transfer truth.

Not every failure should become an exception case immediately.

But when automation is no longer enough, the system should create a durable case.

---

## Early exception classes

- `destination_missing`
- `destination_mismatch`
- `ambiguous_relay_outcome`
- `duplicate_relay_attempt`
- `stale_pending_transfer`
- `source_missing`
- `manual_review_required`

---

## What an exception case should capture later

- transfer id
- exception class
- current lifecycle state
- evidence summary
- reconciliation summary
- severity
- operator notes
- resolution status
- created_at / updated_at

---

## Operator posture

Exception handling is not about hiding failure.

It is about making unresolved truth actionable and safe for a human to inspect.