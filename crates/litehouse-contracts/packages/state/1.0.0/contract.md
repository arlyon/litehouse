# `litehouse:state@1.0.0`

This package defines the normative generation-qualified observation and recovery vocabulary.

- A revision is an opaque monotonic counter scoped to one facet and one provider generation. Revisions are not comparable across facets or generations; no global causal order is promised.
- Every accepted observation advances revision and `received-at`, including an equal value. `received-at` is Litehouse time and is authoritative for freshness. Optional source time retains its device/provider provenance and is never assumed synchronized.
- Value presence, availability, and quality are independent. An absent value is unknown, not false or zero. `unavailable` and `stale` retain the last value where a capability state permits it. Quality is only `source-reported` or `source-assumed`; neither proves physical completion.
- A subscription emits a current snapshot, then `caught-up`, then race-free live observations. A cursor is valid only for its generation.
- Replay is bounded and best effort. Lag, overflow, expiry, history loss, and generation change produce an explicit scoped `gap`. Gap and resume carry subscription ID, delivery epoch, and source generation. Recovery rehydrates state and resumes with a typed continuity receipt, or explicitly uses `at-now`; stale identities fail and `at-now` never claims recovered continuity.
- Provider restart, identity/topology replacement, or authoritative continuity loss creates a new generation. Old handles and cursors become stale. A transport reconnect may preserve generation only when identity, topology, and continuity still match.

Discrete capability events are at least once. Retransmission MUST retain its event ID, source identity, and generation so consumers can deduplicate. V1 provides no durable event replay or exactly-once side effects.
