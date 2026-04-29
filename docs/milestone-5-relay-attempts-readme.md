# Milestone 5 — Relay Attempt Tracking

## Goal

Track relay execution attempts durably.

This milestone adds the failure-heavy middle layer of the Cross-Chain Transfer Reconciliation Gateway.

The purpose of this milestone is to make the relay step explicit instead of hiding it behind vague status changes.

The system now needs to answer:

- when did relay start?
- how many relay attempts happened?
- did relay get accepted?
- did relay fail in a retryable way?
- did relay fail terminally?
- did relay become ambiguous?
- what is safe to do next?

That is what this milestone builds.

---

## Why this milestone exists

Cross-chain transfer systems become dangerous in the middle.

A transfer can be:
- source confirmed
- relay submitted
- response delayed
- timeout ambiguous
- retried unsafely
- destination truth still missing

If the relay step is not tracked explicitly, operators and downstream systems cannot understand what really happened.

So this milestone makes relay execution attempts durable and queryable.

---

## What this milestone owns

This milestone owns:
- relay attempt start
- relay attempt finish
- relay attempt outcome classification
- persistence of relay attempts
- API endpoints for relay attempt progression

It is responsible for:
- beginning a relay attempt only when the transfer is relayable
- persisting each attempt durably
- classifying relay outcomes explicitly
- moving the transfer state accordingly

It is explicitly **not** responsible for:
- destination evidence tracking
- final settlement
- reconciliation resolution

Those come later.

---

## New endpoints

### `POST /transfer-intents/:id/relay-attempts/start`
Begins a new relay attempt.

Behavior:
- loads the transfer
- checks domain preconditions
- moves state into `RelayInProgress`
- appends a new relay attempt
- persists the updated truth

### `POST /transfer-intents/:id/relay-attempts/finish`
Finishes the current relay attempt.

Request body:
- `outcome`
- `classification` when needed
- `reason` when needed
- `relay_reference` optional
- `note` optional

Supported outcomes:
- `accepted`
- `retryable_failure`
- `terminal_failure`
- `unknown_outcome`

---

## Relay outcome meaning

### `accepted`
Relay progression was accepted.

Result:
- transfer moves to `DestinationPending`

### `retryable_failure`
Relay failed in a way that may safely be retried later.

Result:
- transfer returns to `SourceConfirmed`

### `terminal_failure`
Relay failed in a way that should not be auto-retried.

Result:
- transfer moves to `FailedTerminal`

### `unknown_outcome`
Relay outcome is ambiguous.

Result:
- transfer moves to `RelayUnknown`
- ambiguity remains explicit

---

## Core invariant preserved here

The most important invariant in this milestone is:

**Ambiguous relay outcome must remain explicit.**

That means:
- a timeout is not treated as proof of failure
- a missing response is not treated as proof of acceptance
- the system must preserve uncertainty until later evidence resolves it

That is one of the strongest signals in this whole project.

---

## What done means for this milestone

Milestone 5 is done when:

- relay attempts can be started and finished
- each relay attempt is recorded durably
- accepted relay moves the transfer to `DestinationPending`
- retryable failure returns the transfer to `SourceConfirmed`
- terminal failure moves the transfer to `FailedTerminal`
- unknown outcome moves the transfer to `RelayUnknown`
- application tests cover the important outcome paths

Done means:
**the system now models the ambiguous middle explicitly instead of hiding it.**

---

## Why this milestone matters

This is the part of cross-chain execution where many systems become unsafe.

A weak system treats the relay step like a black box.

A stronger system records:
- each attempt
- each outcome
- each ambiguity
- each resulting lifecycle change

That is what this milestone proves.

---

## Summary

Milestone 5 gives the Cross-Chain Transfer Reconciliation Gateway an explicit relay execution history.

It allows the system to:
- begin relay attempts safely
- finish relay attempts with explicit classification
- preserve unknown outcomes as unknown
- keep retryable and terminal failures distinct
- maintain durable relay attempt history

This is the milestone where the project starts to feel deeply failure-aware.