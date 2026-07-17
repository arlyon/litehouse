# `litehouse:errors@1.0.0`

This package is the normative operational failure classification shared by Litehouse contracts. A capability error envelope MUST contain one `core-error`; capability-owned domain detail is carried separately by that capability.

## Classification

- `binding` distinguishes missing, ambiguous, stale, and removed endpoints.
- `authorization`, `availability`, `timing`, `capacity`, and `execution` describe host-visible failure boundaries without exposing provider internals.
- `contract-error` MUST accompany capability detail when the provider has a domain-specific rejection.
- `outcome-unknown` MUST be returned after a side effect may have been dispatched but confirmation was lost. It MUST retain request, attempt, cause, and any idempotency key and MUST NOT be presented as definite failure.

`unsupported` is not `not-found`, and an unavailable endpoint is not an unknown state value. Hosts MUST forward the complete capability envelope unchanged except when synthesizing a failure, in which case domain detail is absent.

## Retry and diagnostics

Retry advice is guidance, not permission. Automatic retry additionally requires operation semantics and a matching advertised deduplication guarantee. Relative, continuous, toggle, momentary, and notification effects are never automatically retried. Messages MUST be bounded, redacted, safe for display, and treated as untrusted text. `error-id` correlates protected diagnostics and MUST NOT encode secrets or causal chains.
