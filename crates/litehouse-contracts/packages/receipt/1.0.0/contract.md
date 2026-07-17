# `litehouse:receipt@1.0.0`

Every query and command carries a host-issued request ID, optional correlation ID, absolute monotonic deadline, attempt number, and optional idempotency context. Queue wait and nested calls consume the same deadline. A provider MUST NOT lengthen it.

Operation metadata declares whether intent is absolute/convergent, relative, continuous, or momentary and advertises any deduplication guarantee. An idempotency key authorizes retry only when the specific operation advertises a matching guarantee. Presence of a key alone changes no semantics.

A successful command returns `command-receipt` with one of these ordered confidence claims:

1. `accepted-for-delivery` — admitted or sent;
2. `protocol-or-target-acknowledged` — a protocol or target acknowledged it;
3. `completed` — the provider has completion evidence; or
4. `observed-completed` — a later authoritative observation matched the requested result.

A provider MUST NOT claim stronger confidence than its evidence. Matching later state is not silently causal confirmation. Loss after possible dispatch is an `outcome-unknown` error, not a receipt. Receipts preserve request, attempt, generation, timing, confidence, and idempotency identity across restarts.
