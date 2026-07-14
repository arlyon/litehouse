# Define the shared error model

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: 11, 15
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 4

## Question

Which failures are shared transport/runtime errors versus contract-domain
errors, and how should invalid arguments, unavailable/removed entities, denial,
unsupported operations, stale handles, deadlines, caller/provider cancellation,
overload, traps, disconnects, ambiguous command outcomes, and retryability
propagate across guest, proxy, rule, API/MCP, and WebRTC boundaries?

## Answer

Publish exact-major `litehouse:errors@1` as the operational classification that
informs Litehouse and generic clients. Use grouped typed variants rather than a
flat enum:

- `binding`: not-found, ambiguous, stale-handle, endpoint-removed;
- `authorization`: denied, expired-grant, insufficient-scope;
- `availability`: unavailable, reconnecting, shutting-down;
- `timing`: deadline-exceeded and cancelled with structured caller, provider,
  host, or shutdown origin;
- `capacity`: overloaded with optional retry-after;
- `execution`: provider-trapped, protocol-failed, unsupported;
- `outcome-unknown`: a deadline, cancellation, or disconnect after a side effect
  may have occurred; and
- `contract`: generic classification for an accompanying domain detail.

Every exact-major service contract defines its own complete error record using
that core type, conceptually:

```wit
record error {
  inner: litehouse:errors/core-error,
  detail: option<contract-error-detail>,
  retry: retry-advice,
  message: option<string>,
  error-id: string,
}
```

The core `inner` exists for Litehouse routing, retry/rebind behavior, telemetry,
and generic API handling. `detail` is owned by the service contract—not the
concrete provider—so dependent plugins can exhaustively match the full typed
error while providers remain substitutable. Litehouse may inspect `inner` but
forwards the complete envelope unchanged. Host-synthesized failures use the
same contract shape with no domain detail.

Messages are optional, bounded, safe/redacted, and treated as untrusted display
text; they never replace typed handling. `error-id` correlates protected host
diagnostics without exposing internal causal chains or secrets.

Retry advice is machine-readable (`never`, reconnect/rebind guidance, or
optional retry-after), but automatic retry additionally requires the contract
operation to declare safe idempotency. Error kind alone never authorizes retry.

If a side-effecting call loses confirmation after dispatch, return
`outcome-unknown` with its structured cause and idempotency context; never claim
definite failure or retry blindly. Provider traps return `provider-failed`,
withdraw the generation, fail pending work, retire the store, and restart under
bounded policy. Queue-full admission fails immediately as `overloaded` and
never drops other work.

Discovery/binding uses the same grouped core WIT. Web/API/MCP/WebRTC serialize
the complete core group, contract detail, retry advice, safe message, and error
ID. Transport status codes are secondary and never replace the typed envelope.
