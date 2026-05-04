# Milestone 8 — Exception Case Management

## Goal

Turn unresolved or mismatched transfers into explicit operator cases.

This milestone adds the operator-facing exception layer of the Cross-Chain Transfer Reconciliation Gateway.

The purpose of this milestone is to move from:
- “this transfer is unresolved”
to
- “this transfer now has a durable operator case with status, notes, and resolution trail”

---

## Why this milestone exists

A reliability-first system should not stop at detecting ambiguity or mismatch.

It should also give operators a structured way to inspect, track, and resolve those cases.

This milestone exists to make unresolved truth actionable.

---

## What this milestone owns

This milestone owns:
- opening exception cases
- listing exception cases for a transfer
- resolving the latest open case
- recording exception case audit trail

It is responsible for:
- turning ambiguous or mismatched transfer states into explicit cases
- preserving case status
- preserving operator notes
- recording resolution

---

## New endpoints

### `POST /transfer-intents/:id/exception-cases`
Opens a new exception case for the transfer.

Request body:
- `classification` optional
- `case_status` optional (`open` or `investigating`)
- `note` optional

If classification is omitted, the service tries to infer it from transfer state.

### `GET /transfer-intents/:id/exception-cases`
Lists all exception cases for the transfer.

### `POST /transfer-intents/:id/exception-cases/resolve`
Resolves the latest non-resolved case for the transfer.

Request body:
- `note` optional

---

## Inference rules

If no explicit classification is given, the system infers from current transfer truth:

- `RelayUnknown` -> `AmbiguousRelayOutcome`
- `MismatchDetected` -> `DestinationMismatch`
- `ManualReview` -> `ManualReviewRequired`
- `DestinationPending` -> `DestinationMissing`
- existing `latest_exception` is preferred when present

---

## What done means for this milestone

Milestone 8 is done when:

- explicit exception cases can be opened
- exception cases can be listed
- unresolved cases can be resolved
- operator notes are stored durably
- exception audit trail exists
- tests cover open and resolve flows

Done means:
**unresolved truth is now manageable as an operator workflow, not just a status.**

---

## Why this milestone matters

This is what turns mismatch and ambiguity into something operational.

Before this milestone, the system could detect:
- mismatch
- unresolved state
- manual review need

After this milestone, it can also preserve:
- case history
- case status
- notes
- resolution trail

That is a big step toward a real reliability platform.

---

## Summary

Milestone 8 gives the Cross-Chain Transfer Reconciliation Gateway durable exception case management.

It allows the system to:
- create operator cases from unresolved or mismatched transfers
- query those cases
- resolve them safely
- preserve a durable trail of case activity

This is where truth becomes actionable for humans.