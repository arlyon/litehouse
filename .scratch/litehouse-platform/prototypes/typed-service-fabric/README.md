# Typed service-fabric spike

Disposable planning evidence for
[Prototype the typed service fabric](../../issues/21-prototype-typed-service-fabric.md).
It is a standalone Cargo workspace and does not modify Litehouse production WIT
or crates.

## What it proves

- Rust `wasm32-wasip2` produces valid components containing custom native-async
  WIT exports.
- Official `wasip3` 0.7 bindings produce a component that imports and exports
  WASI 0.3 async interfaces while Rust `std` temporarily retains WASI 0.2
  imports. The old P1 reactor adapter is not needed for this route.
- The existing production `noop` plugin also builds directly for
  `wasm32-wasip2`; `wasm-tools` validates it and reports its current `update`
  and `outputs` exports as native async.
- Provider endpoint resources and consumer proxy resources are separate generated
  types in separate Wasmtime binding modules.
- `on-off` v1 and v2 are distinct generated interfaces and host proxy types.
- One configured provider instance can expose repeated same-contract facets
  without a nested device identity.
- Wildcard grants cover later explicit re-resolution while an active binding
  remains a deterministic snapshot.
- Operation grants, store generations, and facet incarnations produce distinct
  denial/staleness behavior.
- Host admission can remain ordered while the provider controls completion
  concurrency.
- The handwritten host model contains no dynamic cross-store resource handle.

## What it deliberately does not prove

This is not a production service runtime. It does not instantiate and forward
calls between live provider and consumer stores, exercise real trap teardown,
or implement production subscriptions, cancellation, tracing, persistence,
configuration, code generation, or all nine capability contracts. Those belong
in the implementation and real ESPHome prototype. The spike establishes that
the generated type partition and host-owned proxy design compile without a
reflection layer.

## Run

```bash
rustup target add wasm32-wasip2
./check.sh
```

`check.sh` builds the provider, consumer, and WASI 0.3 smoke components;
validates Component Model async encoding; inspects their WIT; rejects dynamic
resource handles in handwritten Rust; and runs the host model tests.
