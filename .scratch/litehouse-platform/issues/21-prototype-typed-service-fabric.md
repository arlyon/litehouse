# Prototype the typed service fabric

Type: prototype
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 16, 17, 18, 19
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Does a minimal disposable WIT/host spike prove provider-store-local endpoint
resources, consumer-store-local generated proxies, one configured instance as
the physical-device identity, repeated same-contract facets without a nested
device key, exact-major coexistence, deterministic cardinality/binding,
facet-local incarnations, store generations/stale handles, metadata-only
composition groups, named feature/operation negotiation, current/target state,
typed receipts, bounded ordered dispatch with plugin-controlled concurrent
execution, typed subscriptions, cancellation, authorization, trap isolation,
tracing, and zero cross-store `ResourceAny`?

## Answer

The disposable [typed service-fabric spike](../prototypes/typed-service-fabric/)
proves the architecture-critical type and identity shape without implementing a
second runtime.

Two generated component worlds define provider-owned endpoint resources and
consumer-local proxy resources. `on-off` v1 and v2 are separate WIT interfaces,
separate generated Wasmtime modules, and separate Rust proxy types. One provider
instance exposes two deterministic v1 facets with independent incarnations and
one v2 facet; there is no nested device identity. The handwritten host registry
contains only stable provider-instance/endpoint/generation/incarnation data and
contains no dynamic cross-store resource handle.

The host model tests prove deterministic sorted `many` snapshots, explicit
re-resolution for new facets covered by a wildcard grant, operation-level
authorization, exact-major type separation, facet-local staleness, store-wide
generation invalidation, typed current/target receipts, and ordered host
admission with provider-controlled completion order. Metadata groups and named
features remain ordinary typed data with no binding authority.

The spike also supersedes the old packaging assumption. Official `wasip3`
0.7.0 bindings compile on the distributed `wasm32-wasip2` target into a valid
component with native WASI 0.3 async imports/exports while Rust `std` temporarily
retains WASI 0.2 imports. The existing production `noop` plugin likewise builds
directly for `wasm32-wasip2`; `wasm-tools` validates it and reports its current
`update` and `outputs` exports as native async. Implementation should therefore
use `wasm32-wasip2` directly and remove the P1 reactor-adapter packaging step;
do not build the Tier-3 `wasm32-wasip3` compiler target locally.

Validation passed:

```text
./.scratch/litehouse-platform/prototypes/typed-service-fabric/check.sh
  3 components built and Component Model async validated
  WASI 0.3 run interface and custom async exports found
  4 host model tests passed
  0 handwritten ResourceAny occurrences

cargo check --target wasm32-wasip2 -p noop
  passed

cargo build --target wasm32-wasip2 -p noop
wasm-tools validate --features cm-async target/wasm32-wasip2/debug/noop.wasm
  passed; update and outputs remain async
```

Keep the conclusion narrow. This spike does not implement or prove live
cross-store forwarding, real trap teardown, guest cancellation, tracing,
persistent subscriptions, all capability contracts, or ESPHome behavior. Those
are direct implementation and **Prototype real ESPHome capabilities** checks,
not justification for a policy/reflection framework. Generated typed adapters,
host-owned IDs/proxies, the existing actor/queue decision, and ordinary
structured tracing remain sufficient.
