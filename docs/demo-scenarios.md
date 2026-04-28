# Demo Scenarios Draft

These are the core scenarios the project should eventually demonstrate.

## Scenario 1
Duplicate transfer request with same idempotency key and same payload.

Expected outcome:
- one transfer lineage
- safe replay response

## Scenario 2
Duplicate transfer request with same idempotency key and different payload.

Expected outcome:
- conflict
- no second lineage created

## Scenario 3
Source confirmed, destination still missing.

Expected outcome:
- not marked settled
- unresolved or pending state remains explicit

## Scenario 4
Relay timeout / unknown outcome.

Expected outcome:
- ambiguity preserved
- no blind replay

## Scenario 5
Late destination settlement after ambiguity.

Expected outcome:
- reconciliation resolves to settled with evidence trail

## Scenario 6
Destination mismatch.

Expected outcome:
- mismatch state or exception case
- operator-visible evidence

## Scenario 7
Duplicate relay attempt.

Expected outcome:
- durable attempt history
- no silent corruption of truth

## Scenario 8
Stale pending transfer.

Expected outcome:
- manual review or exception case

## Scenario 9
Successful fully reconciled transfer.

Expected outcome:
- source evidence
- relay evidence
- destination evidence
- clear receipt timeline

These scenarios exist to prove the repo is about reliability, not just blockchain activity.