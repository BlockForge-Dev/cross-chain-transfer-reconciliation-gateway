# Architecture Draft

## High-level flow

Upstream App
→ Ingress API
→ Durable Transfer Intent Store
→ Source Evidence Tracking
→ Relay Attempt Tracking
→ Destination Evidence Tracking
→ Reconciliation Engine
→ Exception Classification
→ Receipt / Operator Query Surface

---

## Layer responsibilities

### apps/api
Responsible for:
- transfer intent ingestion
- query endpoints
- auth boundary
- request validation boundary

### apps/worker
Responsible for:
- background relay-side work
- scheduled tracking jobs
- evidence ingestion helpers

### apps/reconciler
Responsible for:
- comparing intended truth with observed truth
- classifying mismatches and unresolved states
- opening or updating exception cases

### apps/operator
Responsible for:
- operator-facing views
- timeline inspection
- case inspection
- safe operator actions

---

## crate responsibilities

### crates/domain
Owns:
- domain language
- lifecycle states
- invariants
- transition rules

### crates/application
Owns:
- use cases
- orchestration
- service boundaries

### crates/persistence
Owns:
- Postgres schema interaction
- repositories
- transaction boundaries
- durable truth storage

### crates/source_tracking
Owns:
- source-side evidence models and workflows

### crates/relay_tracking
Owns:
- relay attempt models and workflows

### crates/destination_tracking
Owns:
- destination evidence models and workflows

### crates/reconciliation
Owns:
- intended vs observed truth comparison
- reconciliation decisions

### crates/exceptions
Owns:
- exception case models
- classification
- manual review state

### crates/receipts
Owns:
- receipt assembly
- transfer timeline view
- operator-visible truth presentation

### crates/shared
Owns:
- shared primitives that do not belong to the core domain

### crates/config
Owns:
- environment config loading
- runtime settings

---

## Dependency direction

The desired dependency direction is:

apps → application → domain  
apps → persistence  
application → domain  
tracking / reconciliation / exceptions / receipts → domain + persistence  
shared and config stay low-level and reusable

The important rule:

**domain must stay clean and independent of transport, framework, and database details.**

---

## Durability principle

Durable truth lives in Postgres.

Not in logs.  
Not in worker memory.  
Not in chain explorer output alone.  
Not in temporary relayer state.

---

## Architectural posture

This repo should feel:
- deliberate
- evidence-heavy
- reconciliation-first
- operator-aware
- reliability-focused

If the code starts to feel like a generic blockchain app, the architecture has drifted.