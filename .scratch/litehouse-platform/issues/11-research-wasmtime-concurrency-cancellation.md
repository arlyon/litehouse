# Research Wasmtime concurrency and cancellation

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

What does Wasmtime 46 guarantee for Component Model async resources, concurrent
stores, `run_concurrent`, cancellation, traps, resource destruction, and
reentrancy, and which queueing, deadline, isolation, and shutdown semantics must
Litehouse add rather than infer?

## Answer

Partial source-level research against Wasmtime 46.0.1 found:

- A store owns its instances/runtime state. Its Component Model concurrency is
  cooperative: one event loop polls eligible work, not parallel mutable store
  access. `call_concurrent` allows overlapping calls but defines no Litehouse
  ordering, fairness, queue capacity, or overload behavior.
- Calls progress only while `run_concurrent` drives the store. A decisive timeout
  must wrap the whole `run_concurrent` call; a timeout inside its closure can
  stall when the store blocks.
- Dropping a `call_concurrent` future does **not** cancel the guest task. It only
  discards the result handle. Hard cancellation of a retained top-level guest
  call requires retiring/dropping the store.
- Epoch/fuel interruption can preempt CPU-bound Wasm but cannot wake blocked host
  I/O; every host operation needs its own deadline/cancellation.
- `ResourceAny` has no RAII destructor. Retained owned and borrowed handles need
  explicit `resource_drop_async`; destructors can trap and canonical-ABI
  reentrancy checks apply.
- Synchronous recursive entry is rejected, but asynchronous call cycles remain
  possible and are the embedder's responsibility.

Recommended Litehouse policy:

1. One actor and fresh store per provider instance/generation; share only the
   engine and never move guest resources across actors/stores.
2. Admit one guest call at a time per provider by default. Higher concurrency is
   an explicit tested capability; same-provider routing cycles are rejected.
3. Use bounded queues and absolute monotonic deadlines including queue wait.
4. Cancel queued work directly; propagate soft cancellation to running child
   calls/I/O; after bounded grace, drop the provider store, fail its generation,
   and make side-effect outcomes explicitly ambiguous rather than retrying.
5. Treat traps as generation-fatal. Restart from a fresh store with bounded
   backoff, jitter, budget/circuit breaker, and health gate.
6. Shutdown stops admission, drains/cancels work, explicitly drops resources
   while driving the event loop, then hard-drops the store after a bound.

Primary sources: Wasmtime 46.0.1
[`concurrent.rs`](https://github.com/bytecodealliance/wasmtime/blob/v46.0.1/crates/wasmtime/src/runtime/component/concurrent.rs),
[`call_concurrent`](https://github.com/bytecodealliance/wasmtime/blob/v46.0.1/crates/wasmtime/src/runtime/component/concurrent/func.rs),
[`ResourceAny`](https://github.com/bytecodealliance/wasmtime/blob/v46.0.1/crates/wasmtime/src/runtime/component/resources/any.rs),
and [Component concurrency design](https://github.com/WebAssembly/component-model/blob/c940736c4c6c1e9f0d6c67381b7ae0f6766bb120/design/mvp/Concurrency.md).

Queue sizes, deadlines, cancellation grace, restart budgets, concurrency opt-in,
idempotency, process-level isolation, and destructor-failure reporting remain
Litehouse decisions.
