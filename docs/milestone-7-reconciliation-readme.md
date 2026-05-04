# Milestone 7 — Reconciliation Engine

## Goal

Compare intended truth, source evidence, relay history, and destination evidence.

This milestone turns the Cross-Chain Transfer Reconciliation Gateway from a tracked lifecycle system into a real truth comparison system.

The purpose of this milestone is to answer:

- do the truth surfaces align?
- is the transfer still unresolved?
- is there a mismatch?
- should the transfer be settled, kept pending, or escalated?

That is what reconciliation exists to do.

---

## Why this milestone exists

Cross-chain systems naturally accumulate multiple truth surfaces:

- intended transfer truth
- source-chain truth
- relay execution truth
- destination-chain truth

Those surfaces do not always agree.

A system that only stores events but never compares them is still incomplete.

This milestone closes that gap by adding explicit reconciliation.

---

## What this milestone owns

This milestone owns:
- reconciliation trigger
- reconciliation comparison logic
- reconciliation decision logic
- persistence of reconciliation runs
- lifecycle movement based on reconciliation outcome

It is responsible for producing durable outcomes such as:
- matched
- unresolved
- mismatch
- manual review

---

## New endpoint

## `POST /transfer-intents/:id/reconcile`

Triggers reconciliation for a transfer.

### Optional request body
- `note`

### Required headers
- `Authorization: Bearer <token>`

### Behavior
- loads the transfer
- moves into `Reconciling`
- compares intended/source/relay/destination truth
- computes reconciliation result
- persists reconciliation run
- updates lifecycle accordingly

---

## Current reconciliation rules in this milestone

### Rule 1 — matching destination evidence + confirmed source truth
If:
- source truth is confirmed
- destination evidence matches intended transfer

then:
- `comparison = Matched`
- `decision = ConfirmSettled`
- lifecycle becomes `Settled`

### Rule 2 — explicit destination mismatch
If:
- destination evidence exists but does not match intended truth
or
- transfer is already in `MismatchDetected`

then:
- `comparison =MismatchDetected`

then:
 Mismatch`
- `decision = MarkMismatch`
- lifecycle stays or becomes `MismatchDetected`

### Rule 3 — relay unknown with no conclusive destination truth
If:
- relay outcome is ambiguous

then:
- `comparison = Unresolved`
- `decision = EscalateManualReview`
- lifecycle becomes `ManualReview`

### Rule 4 — otherwise unresolved
If truth is incomplete but not explicitly mismatched, then:
- `comparison = Unresolved`
- `decision = KeepPending`
- lifecycle becomes or remains `DestinationPending`

---

## Core invariant preserved here

The most important invariant in this milestone is:

**The system must not invent certainty when truth surfaces do not fully align.**

That means:
- missing truth should stay unresolved
- relay ambiguity should not be silently finalized
- mismatch should remain explicit
- only aligned truth should settle the transfer

---

## What done means for this milestone

Milestone 7 is done when:

- reconciliation can be triggered through the API
- reconciliation runs are persisted durably
- matching truth settles the transfer
- relay ambiguity can escalate to manual review
- mismatch remains explicit
- tests cover matched, unresolved, and mismatching cases

Done means:
**the system can now compare truth, not just collect it.**

---

## Why this milestone matters

This is the maturity layer of the project.

Before this, the system could:
- accept transfer intents
- store source evidence
- track relay attempts
- store destination evidence

Now it can actually say:
- these truths match
- these truths do not match
- this transfer is still unresolved
- this case needs manual review

That is a much stronger system.

---

## Summary

Milestone 7 gives the Cross-Chain Transfer Reconciliation Gateway a real reconciliation engine.

It allows the system to:
- compare truth surfaces deliberately
- settle when truth aligns
- preserve mismatch when truth diverges
- escalate ambiguity to manual review
- store reconciliation decisions durably

This is the milestone that makes the gateway feel like a true reconciliation-first system.