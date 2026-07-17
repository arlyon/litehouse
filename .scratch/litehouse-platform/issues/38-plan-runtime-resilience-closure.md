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
event was lost. Lag matters only to an instance with matching update
subscriptions. Disable that affected actor with an explicit continuity-loss error
rather than guessing that constructor restart equals state refresh. Timer-only
instances ignore unrelated broadcast lag. A later provider/management seam may
add explicit snapshot refresh when it exists; T3 does not add durable replay,
cursors, or a delivery state machine.

### Trap isolation

The default per-instance store strategy may reconstruct only the trapped
instance. Global and per-plugin shared-store strategies remain valid legacy
configuration, but an individual actor cannot safely replace their shared store;
on trap that actor becomes inactive while siblings keep their existing store.
T3 does not invent coordinated shared-store restart or runtime generations.

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
- production service-fabric endpoint publication before T6 supplies a real
  provider endpoint;
- arbitrary fixed queue, memory, PID, timer, or diagnostic limits;
- process-per-plugin isolation;
- a new diagnostics subsystem; or
- provider, registry, rule, web, cloud, deployment, or secret-manager behavior.

## Acceptance

T3 passes focused tests proving:

1. corrupt/truncated cache bytes cause a rebuild rather than panic;
2. yielding I/O and non-yielding guest CPU calls terminate at their deadline;
3. subscribed broadcast lag disables only the affected actor instead of
   continuing with incomplete state, while timer-only actors ignore unrelated
   lag;
4. a trap restarts only a default per-instance store; shared-store actor failure
   does not replace sibling state or terminate the whole runtime;
5. per-instance capabilities do not leak, duplicate instance IDs are rejected,
   and legacy global configuration remains compatible;
6. protected environment values reach only the declared instance and never enter
   persisted config or logs;
7. invalid component/config/output/lifecycle input returns an error instead of a
   host panic; and
8. the existing cache, in-process compiler, event payload forwarding, output
   validation, and ordinary `run_concurrent` behavior remain intact.

Run focused Litehouse/runtime tests, the T2 service-fabric regression harness, release build,
Wasm component validation, formatting, diagnostics, and the repository's T0
checks. T3 remains one independently reviewed commit.
