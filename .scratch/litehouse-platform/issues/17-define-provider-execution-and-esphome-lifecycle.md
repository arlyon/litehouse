# Define provider execution and ESPHome lifecycle

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: 04, 11, 15, 16
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

What generic provider actor/store, dispatch, lifecycle, publication, failure,
and shutdown policy safely supports repeated facets, while leaving the
first-party ESPHome plugin responsible for one physical node's connection,
cache, discovery, and reconnect behavior?

## Answer

Use one configured plugin instance per physical device. Installing the ESPHome
artifact makes that plugin type available; each ESPHome node is then one
separately configured instance exposing any number of capability facets. There
is no nested provider-owned device collection and no separate `device-key` in
binding identity. A facet is identified by provider instance, stable facet key,
exact contract identity, and its host-tracked incarnation/generation.

### Generic host boundary

- One host actor owns one fresh Wasmtime store and provider generation for each
  configured instance. Stores and guest resources never cross actors.
- Each instance receives one bounded, ordered stream of typed calls and events
  addressed to it. FIFO admission and dispatch do not serialize execution:
  after dispatch, the plugin controls mapping, fast-forwarding, and concurrent
  handling inside its cooperatively driven store.
- Direct calls are always addressed. Other events are delivered only through
  declared bindings and active typed subscriptions; plugins do not receive an
  ambient global event stream. Later subscription APIs may let a plugin refine
  host-side delivery without changing this boundary.
- Every request carries identity, reply context, authorization context, and an
  absolute monotonic deadline that includes queue wait. Full queues reject new
  work explicitly as overload; they never discard earlier commands silently.
- Queued cancellation removes the request immediately. Dispatched cancellation
  becomes an ordered control message to the plugin. If work survives a bounded
  grace period, the host retires the entire store generation. A side-effecting
  request then reports `outcome-unknown` rather than definite failure.
- Plugin traps, unrecoverable resource-drop failures, and hard cancellation are
  generation-fatal. The host withdraws that instance's endpoints, fails pending
  work with typed errors, drops the store after a bound, and restarts from a
  fresh store.
- Restart uses bounded exponential backoff with jitter and a fast-retry budget,
  then sparse probes rather than a permanent circuit. Health and manual retry
  remain visible. Exact queue, deadline, grace, and backoff values wait for
  representative baselines and performance SLOs.
- Shutdown stops admission, expires or cancels queued work, sends cooperative
  shutdown, drives bounded resource destruction, and finally drops the store.
  No lock may be held across a guest call or guest-controlled await.

The host does not standardize device connectors, keepalive, cache, discovery,
or reconnect mechanics. Those belong to each plugin. A plugin reports typed
lifecycle, availability, endpoint, state, and error information through the
shared contracts; Litehouse supervises rather than impersonates its transport.

### ESPHome instance policy

- The first-party ESPHome instance owns one Native API session/cache/reconnect
  lifecycle for its one configured node and shares it across all facets.
- The plugin may publish facets incrementally. Discovery need not hide all
  useful capabilities until a complete node snapshot is available.
- The plugin durably preserves its last confirmed facet catalog and
  Litehouse-owned identity/routing map. On offline startup it may restore known
  facets as explicitly unavailable, then hydrate and reconcile them after a
  live connection.
- Adding a facet does not alter existing handles. Removing or replacing one
  facet stales only that facet's incarnation; unaffected facets continue.
  Restarting the plugin store stales every facet in the instance.
- ESPHome routing keys are locators, not Litehouse identities. Exact persisted
  evidence may preserve a facet identity, but a changed key that cannot be
  proven equivalent is removal plus a new facet. Labels and topology heuristics
  may suggest operator reconciliation but never redirect bindings silently.

This deliberately borrows Home Assistant's useful shape—one ESPHome config
entry per node, shared integration state, ordered routing followed by async
work—without copying its weak bounds. Home Assistant Core `4c2b232` routes event
listeners in registration order but starts coroutine listeners as independent
tasks, runs targeted entity calls concurrently, and has integration/platform
semaphores rather than a general bounded mailbox. Litehouse therefore makes
mailbox limits, overload, deadlines, cancellation, generations, and ambiguous
outcomes explicit at the plugin boundary.

The typed-service prototype must prove ordered dispatch with concurrent plugin
execution, bounded overload, typed subscriptions, cancellation escalation,
facet-local replacement, store-wide trap isolation, offline restored facets,
and zero cross-store guest resources.
