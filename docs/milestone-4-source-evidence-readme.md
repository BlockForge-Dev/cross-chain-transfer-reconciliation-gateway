# Milestone 4 — Source Evidence Tracking

## Goal

Track source-chain proof separately from transfer intent.

This milestone adds the first real on-chain truth surface to the system.

The purpose of this milestone is not to mark transfers complete.

The purpose is to let the system observe and confirm source-side evidence durably while preserving the core invariant:

**source confirmation is not destination completion.**

---

## Why this milestone exists

A cross-chain transfer starts with intent, but intent alone is not enough.

At some point, the system must record that something actually happened on the source side:
- lock
- burn
- escrow
- submit
- or any source-chain action that proves transfer progression began

That proof matters, but it must not be confused with final completion.

If the system treats source confirmation as full settlement, it will produce false certainty.

This milestone prevents that.

---

## What this milestone owns

This milestone owns:
- source evidence recording workflow
- source observed vs source confirmed state transitions
- durable source evidence persistence
- source evidence API endpoint

It is responsible for:
- loading a transfer lineage
- recording source-side tx evidence
- moving the state into `SourceObserved` or `SourceConfirmed`
- persisting that evidence durably
- returning the updated transfer truth

It is explicitly **not** responsible for:
- relay execution
- destination settlement
- reconciliation completion
- marking the transfer settled

---

## New endpoint

## `POST /transfer-intents/:id/source-evidence`

Records source-side evidence for a transfer.

### Request body
- `source_tx_hash`
- `status` → `observed` or `confirmed`
- `note` → optional

### Required headers
- `Authorization: Bearer <token>`

### Behavior
- loads the transfer by id
- validates the request
- applies the domain transition
- persists source evidence and updated state
- returns the updated transfer

---

## State behavior

### `observed`
Used when source-side evidence is seen but not yet considered fully confirmed.

Result:
- source evidence stored
- transfer moves to `SourceObserved`

### `confirmed`
Used when source-side proof is strong enough to confirm source progression.

Result:
- source evidence stored
- transfer moves to `SourceConfirmed`

Important:
`SourceConfirmed` still does **not** mean `Settled`.

That distinction remains central to the system.

---

## Core invariant preserved here

The most important invariant in this milestone is:

**Source confirmation is not destination completion.**

This means:
- recording source proof is valid progress
- but it does not finalize the transfer
- the system must still wait for relay and destination truth later

That is why the transfer moves to `SourceObserved` or `SourceConfirmed`, not `Settled`.

---

## What done means for this milestone

Milestone 4 is done when:

- source evidence can be recorded durably
- `SourceObserved` and `SourceConfirmed` are real lifecycle states
- source evidence updates the transfer correctly
- source confirmation is visible through the API
- source confirmation does not falsely imply destination completion
- application tests cover observed and confirmed flows

Done means:
**the system can now represent source-side truth without inventing finality.**

---

## Why this milestone matters

This is the first point where the project becomes visibly cross-chain aware.

The system now distinguishes between:
- requested transfer
- source-side proof
- later relay truth
- later destination truth

That is a major step toward the full reconciliation story.

---

## Summary

Milestone 4 gives the Cross-Chain Transfer Reconciliation Gateway its first real evidence boundary.

It allows the system to:
- record source-chain proof
- move the transfer lifecycle forward safely
- persist source evidence durably
- keep final settlement explicitly unresolved

That is exactly the right behavior for a reliability-first cross-chain system.