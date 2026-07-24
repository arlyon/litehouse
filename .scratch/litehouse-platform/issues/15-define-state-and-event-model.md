# Define the state and event model

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: 04, 10
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 3

## Question

Which v1 guarantees should exact-major service contracts expose for queries,
observed-state hydration, immutable events, and streams—covering revision scope,
ordering, duplicate handling, timestamps, availability/quality, lag, replay,
resume, reconnect, topology changes, and provider generations—so device
contracts, rules, WebRTC, UI, and stale proxies share one coherent model?

## Answer

V1 uses a generation-qualified, per-entity observation model:

1. Revisions are opaque monotonic counters scoped to one endpoint/facet within
   one provider generation. They are not comparable across entities or
   generations; Litehouse promises no global or cross-device causal order.
2. Subscriptions begin with current state, then an explicit hydration/caught-up
   barrier, then race-free live updates at that entity scope.
3. State separates last value from availability and quality. A disconnect keeps
   the last observation for display/history but marks it unavailable or stale;
   rules cannot mistake it for current data.
4. Every observation has mandatory Litehouse `received-at`, used for freshness.
   Optional provider/device source time is preserved separately with provenance
   and is never assumed synchronized or authoritative.
5. Every accepted observation advances revision and receive time, including an
   equal value. Rules distinguish value-change triggers from any-observation
   triggers; equality alone does not prove duplication.
6. Recovery is best-effort. A bounded in-memory buffer may replay updates while
   a cursor remains valid. Otherwise Litehouse reports a gap, rehydrates current
   state, and continues. V1 promises no durable event replay.
7. A transient device transport reconnect preserves bindings and generation
   when the provider process, endpoint identity, topology, and state continuity
   still match. It exposes temporary unavailability and rehydrates before
   resuming.
8. Provider restart, authoritative continuity loss, or endpoint identity/topology
   replacement creates a new generation. Old handles and cursors become
   explicitly stale; same-identity rebinding creates new consumer proxies.
9. Discrete events are delivered at least once. Retransmissions retain a stable
   Litehouse event ID plus source identity and generation so consumers can
   deduplicate. Litehouse does not promise exactly-once side effects.
10. Lag, overflow, expired cursors, and history loss are explicit typed gaps;
    they never silently continue as if delivery were complete.

Exact buffer limits, staleness thresholds, and quality vocabulary remain
measurable policy details for provider/error/SLO tickets, not different state
semantics.
