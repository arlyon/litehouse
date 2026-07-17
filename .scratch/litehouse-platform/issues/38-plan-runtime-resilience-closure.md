# Plan minimal runtime resilience closure

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-17
Blocked by: 05, 16, 17, 24, 28, 30, 33, 37
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which concrete defects in the existing Litehouse runtime must T3 fix before the
connected provider work begins, while preserving the runtime and avoiding a
replacement supervisor platform?

## Answer

Harden the runtime already in `crates/litehouse`. Preserve its Wasmtime engine,
in-process `Component::new` compilation, incremental module cache, store
strategies, capability hooks, `run_concurrent` calls, event loop, and trap restart
path. T3 is a focused reliability and T2-integration tranche, not a new runtime
kernel.

This decision supersedes the former actor-policy kernel, compiler sandbox,
custom cache, custom DNS resolver, global activation transaction, host-restore
protocol, and arbitrary component/resource limits previously specified here.

## Scope

### Cache corruption

Preserve `ModuleCache`, bitcode/LZ4 storage, and Wasmtime incremental compilation.
A missing, corrupt, truncated, or incompatible cache is a cache miss: log a safe
warning, discard/rebuild it, and continue startup. Cache writes should not leave a
partially written authoritative file after an ordinary interruption.

### Deadlines and interruption

Prevent one plugin call from hanging its instance forever:

- lifecycle and update calls have ordinary host deadlines;
- queue/wait time is not hidden from the caller;
- host I/O obeys the remaining call deadline; and
- Wasmtime epoch interruption stops a guest CPU loop that never yields.

Use the smallest integration with the existing runtime. Do not build a general
admission scheduler, global memory ledger, compiler supervisor, or durable call
journal. Deadline defaults remain normal implementation configuration rather
than immutable profile policy.

### Event continuity

The existing broadcast lag path must not log and silently continue as though no
event was lost. Surface an explicit continuity-loss condition to the affected
instance/UI projection and refresh current state before treating later updates as
continuous. Do not add durable event replay, cursor rehydration, or a delivery-
epoch state machine in T3.

### Restart generation fencing

When a plugin store traps and restarts, allocate a fresh in-process runtime
generation, withdraw the old generation's T2 endpoints, and publish endpoints for
the fresh generation only after normal initialization succeeds. Old T2 proxies
must become stale. No cross-process durable generation database is needed because
store resources and proxies do not survive process restart.

### Capability scope

Allow capabilities to be assigned per configured plugin instance while retaining
backward compatibility with the existing global capability list. Preserve the
existing WASI HTTP/TCP hooks. Fix concrete authority-matching defects, including
retaining the configured port, without replacing Wasmtime's sender or implementing
DNS.

### Protected environment values

Resolve only configuration fields explicitly declared as environment-backed.
Inject their values into the owning instance at store construction without
persisting, logging, enumerating, or forwarding the ambient environment. Preserve
existing ordinary configuration behavior.

### Panic paths

Replace relevant runtime/cache `unwrap` and `expect` paths reached by component,
configuration, cache, lifecycle, or output errors with normal typed/reportable
errors. Do not convert unrelated internal invariants into a new error framework.

### T2 production seam

Connect the existing `litehouse-service-fabric` generation and endpoint lifecycle
to one production provider path. Do not add a second routing or authorization
authority. Treat the current service-fabric harness as evidence, not product
integration.

## Explicitly not scope

T3 does not add:

- out-of-process or sandboxed compilation;
- serialized compiled-code loading;
- a replacement/custom compatibility cache;
- a hard component-size ceiling;
- custom DNS parsing or vendored IANA policy;
- global plugin-set activation/rollback transactions;
- runtime or host restore journals;
- durable event replay/rehydration;
- arbitrary fixed queue, memory, PID, timer, or diagnostic limits;
- process-per-plugin isolation;
- a new diagnostics subsystem; or
- provider, registry, rule, web, cloud, deployment, or secret-manager behavior.

## Acceptance

T3 passes focused tests proving:

1. corrupt/truncated cache bytes cause a rebuild rather than panic;
2. yielding I/O and non-yielding guest CPU calls terminate at their deadline;
3. broadcast lag becomes explicit continuity loss followed by state refresh;
4. a trap restarts only the affected instance and fences old T2 proxies;
5. per-instance capabilities do not leak while legacy global configuration still
   behaves compatibly;
6. protected environment values reach only the declared instance and never enter
   persisted config or logs;
7. invalid component/config/output/lifecycle input returns an error instead of a
   host panic; and
8. the existing cache, in-process compiler, event payload forwarding, output
   validation, and ordinary `run_concurrent` behavior remain intact.

Run focused Litehouse/runtime tests, the T2 service-fabric harness, release build,
Wasm component validation, formatting, diagnostics, and the repository's T0
checks. T3 remains one independently reviewed commit.
