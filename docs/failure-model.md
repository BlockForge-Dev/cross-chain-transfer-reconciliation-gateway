# Failure Model Draft

This project is built around failure-heavy cross-chain execution reality.

## Primary failure classes

### 1. Duplicate request
The same transfer request is submitted more than once.

### 2. Missing source evidence
The transfer intent exists, but no source-chain evidence is observed.

### 3. Relay timeout / ambiguous relay outcome
A relay attempt was made, but the system cannot prove whether downstream execution happened.

### 4. Source confirmed, destination missing
The source side succeeded, but destination settlement is absent.

### 5. Duplicate relay attempt
A transfer is replayed or relayed more than once.

### 6. Destination mismatch
Destination evidence exists, but it does not match the intended chain, recipient, asset, or amount.

### 7. Partial visibility
One truth surface updates while others remain stale or missing.

### 8. Reconciliation mismatch
Internal truth and observed source/destination truth diverge.

### 9. Stale pending transfer
A transfer remains unresolved beyond the acceptable time window.

---

## Failure posture

This system should not:
- hide ambiguity
- assume missing evidence means failure
- assume source confirmation means completion
- replay blindly
- silently patch mismatch

This system should:
- preserve ambiguity explicitly
- store evidence durably
- compare truth surfaces carefully
- expose mismatch and exception cases
- support safe recovery paths