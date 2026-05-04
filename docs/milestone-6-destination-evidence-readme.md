# Milestone 6 — Destination Evidence Tracking

## Goal

Track destination settlement evidence separately and correlate it carefully.

This milestone adds the destination-side truth surface of the Cross-Chain Transfer Reconciliation Gateway.

The purpose of this milestone is not to blindly finalize transfers.

The purpose is to let the system record destination-side evidence durably and compare it against intended truth carefully.

---

## Why this milestone exists

In cross-chain systems, the destination side is where false certainty can become dangerous.

The system may observe:
- a destination tx hash
- a recipient
- an asset
- a quantity
- a chain

But destination evidence is only useful if it actually matches the intended transfer lineage.

That means the system must not just record destination events.

It must correlate them.

---

## What this milestone owns

This milestone owns:
- destination evidence recording workflow
- destination evidence persistence
- matching vs mismatching destination evidence handling
- destination evidence API endpoint

It is responsible for:
- loading the transfer lineage
- accepting destination evidence
- storing it durably
- moving the lifecycle to the right state based on whether the evidence matches the intended transfer

It is explicitly **not** responsible for final settlement decision through reconciliation.

That comes next.

---

## New endpoint

## `POST /transfer-intents/:id/destination-evidence`

Records destination-side evidence for a transfer.

### Request body
- `destination_tx_hash`
- `destination_chain`
- `recipient`
- `asset`
- `quantity`
- `status` → `observed` or `confirmed`
- `note` optional

### Required headers
- `Authorization: Bearer <token>`

### Behavior
- loads the transfer
- validates request
- builds destination evidence
- compares it against intended transfer details
- persists evidence durably
- returns updated transfer state

---

## Matching behavior

If destination evidence matches the intended transfer:
- correct destination chain
- correct recipient
- correct asset
- correct quantity

then the system records it and keeps the transfer in `DestinationPending` until reconciliation formally confirms settlement later.

This is important:

**destination evidence is not the same as reconciliation-complete settlement.**

---

## Mismatch behavior

If destination evidence does not match the intended transfer, the system must not silently accept it.

Instead it moves the transfer into:

- `MismatchDetected`

and records the mismatch explicitly.

This makes operator review and later reconciliation possible.

---

## Core invariant preserved here

The key invariant in this milestone is:

**Destination settlement evidence must correlate to the intended transfer.**

A destination tx hash by itself is not enough.

The evidence must align with:
- intended destination chain
- intended recipient
- intended asset
- intended quantity

If it does not, mismatch becomes explicit.

---

## What done means for this milestone

Milestone 6 is done when:

- destination evidence can be recorded durably
- matching destination evidence stays explicit without inventing finality
- mismatched destination evidence becomes `MismatchDetected`
- destination evidence is queryable through the API
- tests cover both matching and mismatching cases

Done means:
**destination-side truth is now explicit and correlated, not guessed.**

---

## Why this milestone matters

This milestone completes another important truth surface.

The system now has:
- intended truth
- source truth
- relay truth
- destination truth

That makes the next milestone, reconciliation, much more meaningful.

---

## Summary

Milestone 6 gives the Cross-Chain Transfer Reconciliation Gateway the ability to record and correlate destination-side evidence.

It allows the system to:
- store destination proof
- distinguish matching from mismatching evidence
- preserve explicit `DestinationPending` state
- move to `MismatchDetected` when truth diverges

This is the milestone where the destination side stops being assumed and starts being modeled properly.