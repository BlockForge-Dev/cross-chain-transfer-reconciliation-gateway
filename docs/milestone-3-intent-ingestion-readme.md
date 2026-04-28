# Milestone 3 — Intent Ingestion API

## Goal

Allow external systems to create transfer intents safely.

This milestone is the first trust boundary of the Cross-Chain Transfer Reconciliation Gateway.

The purpose of this milestone is not just to expose HTTP endpoints.

The purpose is to ensure that external systems can submit cross-chain transfer intents in a way that is:

- authenticated
- validated
- idempotent
- durable
- queryable
- safe under HTTP retries

If this boundary is weak, the rest of the system can be correct and still produce dangerous duplicate lineages.

---

## What this milestone owns

This milestone owns:

- `POST /transfer-intents`
- `GET /transfer-intents/:id`

It is responsible for:

- authenticating the caller
- validating incoming requests
- requiring an idempotency key
- fingerprinting the normalized request payload
- creating a durable transfer lineage
- returning the same lineage on safe duplicate replay
- rejecting conflicting idempotency reuse
- exposing the current transfer state

It is explicitly **not** responsible for:

- source chain execution
- relay execution
- destination settlement
- reconciliation runs

No relay or execution happens inline in this milestone.

That is intentional.

---

## Why this milestone exists

In money-critical and chain-critical systems, a retried HTTP request must not become a duplicated business lineage.

A caller may retry because:

- the network was unstable
- the caller timed out waiting for a response
- a service crashed and retried
- an upstream system retried automatically
- an operator retried manually

If the ingestion boundary is not idempotent, those retries create duplicate transfer lineages.

That is exactly what this milestone is designed to prevent.

---

## Design decisions in this milestone

### We require `Idempotency-Key`
For this version, `POST /transfer-intents` requires the `Idempotency-Key` header.

This is a money-critical trust boundary, so required idempotency is the stronger default.

### We fingerprint the normalized request
The system computes a request fingerprint from normalized transfer fields:

- client transfer reference
- source chain
- destination chain
- source address
- destination recipient
- asset
- quantity

That lets us distinguish:

- same idempotency key + same payload → safe replay, return existing lineage
- same idempotency key + different payload → reject as conflict

### We do not execute relay flow inline
This milestone only:
- validates
- creates the transfer intent
- moves domain state to `Validated` then `Queued`
- persists the durable lineage

Relay logic comes later.

This keeps the ingestion boundary safe and replay-resilient.

---

## Endpoint behavior

## `POST /transfer-intents`

Creates a transfer lineage safely.

### Request body
- `client_transfer_reference`
- `source_chain`
- `destination_chain`
- `source_address`
- `destination_recipient`
- `asset`
- `quantity`

### Required headers
- `Authorization: Bearer <token>`
- `Idempotency-Key: <key>`

### Successful responses
- `201 Created` when a new transfer lineage is created
- `200 OK` when the same idempotency key and same normalized payload are replayed and the existing lineage is returned

### Conflict response
- `409 Conflict` when the same idempotency key is reused with a different payload

### Important behavior
This endpoint **never** performs relay execution inline.

It only creates durable transfer truth.

---

## `GET /transfer-intents/:id`

Returns the current durable header state of the transfer lineage.

This allows external systems and operators to query current truth safely without inferring from logs.

---

## Validation behavior

This milestone validates:

- `client_transfer_reference` must be present
- `source_chain` must be supported
- `destination_chain` must be supported
- `source_address` must be present
- `destination_recipient` must be present
- `asset` must be present
- `quantity` must be present
- `Idempotency-Key` must be present
- caller must be authenticated

At this stage, supported chains are intentionally limited, for example:
- `ethereum`
- `solana`
- `base`
- `polygon`
- `arbitrum`

That list can be adjusted later.

---

## What happens on create

The `POST /transfer-intents` flow is:

1. authenticate caller
2. require `Idempotency-Key`
3. parse JSON body
4. normalize chains and asset fields
5. validate request correctness
6. compute request fingerprint
7. build domain `TransferIntent`
8. move domain state:
   - `Received`
   - `Validated`
   - `Queued`
9. persist transfer lineage + idempotency record transactionally
10. return:
   - `201 Created` for new lineage
   - `200 OK` for safe replay of the same lineage

That is the safe ingestion flow.

---

## Duplicate request behavior

This milestone must handle two critical cases.

### Case 1 — same idempotency key, same payload
Example:
- first request creates the transfer lineage
- second request is a safe retry of the same business command

Result:
- do not create a second lineage
- return the existing lineage safely

### Case 2 — same idempotency key, different payload
Example:
- first request used quantity `1000000`
- second request reuses the same key with quantity `999999`

Result:
- reject as conflict
- do not overwrite or silently merge the lineage

This is one of the core trust guarantees in the system.

---

## Why we do not execute relay logic here

This milestone is the ingestion boundary, not the execution boundary.

If relay execution happened inline here, then:
- HTTP retry risk and execution risk would be mixed
- transport-layer issues could leak into business-layer execution
- replay safety would become weaker

So this milestone intentionally stops at:
- validated intent
- durable lineage
- queued state

That separation is part of the architecture.

---

## Project goals this milestone supports

This milestone directly supports:

### Trust
The caller gets a stable lineage for the same business command.

### Reliability
HTTP retries do not create duplicate transfer lineages.

### Execution safety
No relay execution is triggered inline.

### Operational truth
The transfer lineage is durably queryable.

### Coordination
The API prepares the lineage for later source / relay / destination workflows without conflating them.

---

## What done means for this milestone

Milestone 3 is done when:

- valid requests persist a new transfer intent
- duplicate idempotent requests return the same lineage safely
- conflicting idempotency reuse is rejected
- current state can be queried
- no relay execution happens inline
- tests exist for duplicate submission scenarios
- the API boundary feels deliberate and safety-aware

Done means:
**the first trust boundary behaves safely under retries.**

---

## Files introduced in this milestone

### `crates/application`
Contains the use case for transfer intent ingestion.

It owns:
- supported chain validation
- request normalization
- request fingerprinting
- use-case orchestration
- mapping domain + persistence into ingestion behavior

### `apps/api`
Contains the HTTP boundary.

It owns:
- auth check
- header extraction
- JSON request/response handling
- HTTP status mapping

This separation matters because it keeps HTTP concerns out of the domain layer.

---

## Testing focus in this milestone

This milestone includes tests for:

- same idempotency key + same payload → returns existing lineage
- same idempotency key + different payload → conflict
- unsupported chain → rejected

That is intentional.

This milestone is about correctness at the ingestion boundary.

---

## Summary

Milestone 3 gives the Cross-Chain Transfer Reconciliation Gateway its first safe external interface.

It allows callers to:
- submit transfer intents
- rely on idempotent replay safety
- query current transfer state

And it ensures that:
- duplicate HTTP retries do not create duplicate transfer lineage
- conflicting reuse of idempotency keys is rejected
- relay execution is not mixed into request-thread truth

That is exactly the kind of trust boundary a reliability-first cross-chain system should have.






s