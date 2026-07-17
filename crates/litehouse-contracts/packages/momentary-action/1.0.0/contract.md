# `litehouse:momentary-action@1.0.0`

Momentary Action is stateless and parameterless. Its stable action identity and risk classification describe one provider action; it never invents pressed/released state or a physical-switch event stream.

`invoke` is a momentary side effect. It is non-idempotent unless the provider advertises a durable matching deduplication guarantee, and it MUST NOT be automatically retried by default. The shared request context supplies request/correlation identity, absolute deadline, attempt, and optional idempotency key.

Success returns an action-specific typed receipt whose confidence is no stronger than available evidence. Loss after possible dispatch is `outcome-unknown`; a button-like request with no response may claim at most accepted-for-delivery. Rejected, busy, safety lockout, and unsupported action are the exhaustive domain details. Provider and consumer worlds grant no WASI facilities.
