
---

# 4) `docs/foundation-spec.md`

**Path:** `docs/foundation-spec.md`

```md id="ew11rv"
# Foundation Spec

## Working name

Cross-Chain Transfer Reconciliation Gateway

---

## Mission

To provide a durable, auditable, reconciliation-first control layer for cross-chain transfer execution so that source evidence, relay attempts, destination settlement, and exception cases can be tracked, understood, and recovered safely.

---

## Vision

To become the trusted truth and recovery layer around cross-chain execution, where every transfer can answer:

- what was intended
- what happened on the source side
- what happened during relay
- what happened on the destination side
- what is still uncertain
- what evidence exists
- what action is safe next

---

## Core goal

The goal is not to build a full bridge.

The goal is to preserve truth and support safe recovery when cross-chain transfer execution becomes ambiguous across source chain, relay system, and destination chain.

---

## What this project is

This project is:
- a transfer intent ingestion system
- a source evidence tracker
- a relay attempt tracker
- a destination evidence tracker
- a reconciliation engine
- an exception classification system
- a receipt timeline system
- an operator-visible truth surface

---

## What this project is not

This project is not:
- a full bridge protocol
- a liquidity routing engine
- a wallet product
- a multichain UX platform
- tokenomics design
- governance system
- relayer marketplace

---

## Core truth surfaces

This system models at least four truth surfaces:

1. Intended truth  
   What the caller requested.

2. Source truth  
   What the source chain proves happened.

3. Relay truth  
   What the relay layer attempted or observed.

4. Destination truth  
   What the destination chain proves happened.

These truth surfaces are related, but they must not be treated as identical.

---

## North star sentence

Safely preserve, compare, and reconcile cross-chain transfer truth across intent, source evidence, relay attempts, and destination evidence under ambiguity and failure.