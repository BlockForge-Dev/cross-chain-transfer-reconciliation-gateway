# Reconciliation Draft

## Why reconciliation exists

Cross-chain transfer truth is not single-sourced.

A transfer can look different depending on where you observe it:
- internal transfer record
- source chain
- relay layer
- destination chain

Reconciliation exists to compare those truth surfaces deliberately.

---

## Reconciliation questions

For every transfer, the system should be able to ask:

- what was intended?
- what source-side evidence exists?
- what relay attempts were made?
- what destination-side evidence exists?
- do those truths agree?
- if they disagree, what class of mismatch is this?
- what action is safe next?

---

## Inputs to reconciliation

- transfer intent
- source evidence
- relay attempt history
- destination evidence
- prior reconciliation runs
- optional operator notes

---

## Outputs of reconciliation

A reconciliation run should produce one of these broad outcomes:

- matched
- unresolved
- mismatch
- manual review required

---

## Reconciliation principle

The system must never invent certainty.

If truth is incomplete or conflicting, reconciliation should preserve ambiguity and create a durable case instead of pretending the transfer is resolved.