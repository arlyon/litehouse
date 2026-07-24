# Establish representative performance baselines

Type: task
Mode: AFK
Status: resolved
Resolved: 2026-07-15
Blocked by: 21, 22, 26
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Using isolated exact-commit builds plus the three accepted disposable
prototypes, what reproducible measurements capture compile/cache stages, cold
and warm startup, instantiation/discovery/ready time, service latency, queue
overload, memory, ESPHome reconnect, registry fetch/cache/activation, and
rollback without optimizing before evidence exists or absorbing dirty work?

## Answer

Do not create a v1 performance-baseline tranche or numeric SLO. The operator has
previously observed Litehouse startup well below one second, and the runtime
caches Cranelift compilation artifacts for fast incremental startup. There is no
current user-visible performance problem or evidence that compile, startup,
service, ESPHome, registry, memory, activation, or rollback cost threatens the
first connected release.

Treat Home Assistant-class startup latency or another obvious slowdown as a
regression/pathology to diagnose, not a competitive target that justifies
benchmark infrastructure now. Preserve the existing compatibility-aware
Cranelift cache and its invalidation rules, but do not optimize, add synthetic
microbenchmarks, retain a benchmark runner, tune the host, access real hardware/
R2, or invent thresholds without demonstrated pressure.

The three accepted prototypes remain correctness evidence only. Existing tests,
structured timing spans, bounded queues/resources, health/readiness, and normal
release checks are sufficient for v1. If a user reports latency, startup visibly
regresses, cache behavior changes, or a production path acquires a concrete
budget, first measure that exact implemented seam on representative amd64 and
arm64 hardware; only then set a numeric gate. Historical sub-second observation
is directional evidence, not a reproducible release SLO.
