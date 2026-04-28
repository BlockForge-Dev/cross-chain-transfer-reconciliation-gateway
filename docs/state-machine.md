# Lifecycle Draft

This is the early lifecycle draft for the transfer aggregate.

The exact names may evolve, but the distinction between phases matters.

## Proposed lifecycle

- `received`
- `validated`
- `rejected`
- `queued`
- `source_observed`
- `source_confirmed`
- `relay_in_progress`
- `relay_unknown`
- `destination_pending`
- `settled`
- `mismatch_detected`
- `reconciling`
- `manual_review`
- `failed_terminal`
- `dead_lettered`

---

## Meaning of key states

### received
The system accepted the transfer request into durable storage.

### validated
The transfer request passed validation and is eligible to proceed.

### rejected
The transfer request is invalid or unsupported.

### queued
The transfer is ready for tracking or execution workflow.

### source_observed
Some source-side evidence was seen, but final source confirmation may not yet be strong enough.

### source_confirmed
Source-side action is confirmed.

Important:
This is **not** the same as destination completion.

### relay_in_progress
A relay attempt or cross-system transfer action is underway.

### relay_unknown
Relay outcome is ambiguous. The system cannot safely determine whether the downstream step happened.

### destination_pending
The system expects destination evidence but has not yet confirmed final settlement.

### settled
Destination evidence is present and correlated correctly to the intended transfer.

### mismatch_detected
Observed evidence does not match intended truth strongly enough.

### reconciling
The system is actively comparing truth surfaces.

### manual_review
The case needs operator attention.

### failed_terminal
The transfer cannot proceed safely without starting a new lineage or explicit operator action.

### dead_lettered
The transfer has been pushed out of normal automation and preserved for investigation.

---

## Early lifecycle principle

Source confirmation is not destination completion.

That distinction must remain explicit in both code and docs.