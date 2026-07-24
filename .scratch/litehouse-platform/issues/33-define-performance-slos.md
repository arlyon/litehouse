# Define performance SLOs

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-15
Blocked by: 31, 32
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 13

## Question

What measured v1 targets and overload/recovery envelopes for container startup,
warm cache, service calls, queues/memory, state lag, ESPHome reconnect, registry
operations, direct management, rule execution, and API/MCP plans distinguish
Litehouse, on which reference hardware/workloads, with what instrumentation and
failure thresholds?

## Answer

Define no numeric performance SLOs for v1. Prior operation observed startup well
below one second, Cranelift compilation artifacts are cached, and the operator
accepts current performance. Home Assistant-class startup latency or another
obvious slowdown would be treated as a regression/pathology, not as the baseline
Litehouse should target.

All already-decided queue, byte, item, session, deadline, timeout, retry,
reconnect, memory/resource, and overload limits remain mandatory correctness,
security, and resilience bounds. They prevent unbounded work and define typed
failure; they are not p50/p95 latency, throughput, startup, or recovery promises.
Do not add benchmark infrastructure, reference-hardware matrices, comparative
claims, telemetry retention, optimization work, or release thresholds without a
real problem.

If startup visibly regresses, a user reports latency, cache behavior changes, or
an implemented production seam needs a budget, measure that exact workload first
on representative `linux/amd64` and `linux/arm64` release hardware with the real
container/plugins/network path. Preserve raw evidence and define a narrow gate
only after that measurement. Until then, normal structured spans, health/
readiness, bounded admission, and canonical correctness checks are sufficient.
