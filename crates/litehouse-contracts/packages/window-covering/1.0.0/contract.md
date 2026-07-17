# `litehouse:window-covering@1.0.0`

A covering independently advertises lift and tilt axes, position awareness, native step, state fields, and operations. Portable positions use the quantity ratio profile (`0` closed, `1` open) only when the source semantics support that mapping; they never imply Matter installed-unit absolute positioning.

Each axis keeps optional current and target separately. Unknown position, unavailable/stale transport, and source-assumed quality are distinct. Movement `idle` or `stopped` is not proof that a target completed. Safety/fault strings are source-supported bounded identifiers, not inferred conditions.

Open/close are absolute intents; stop, continuous physical movement, and ambiguous requests are never automatically replayed. Target positioning and stop require their advertised features. Every command carries shared request/deadline/idempotency context and returns a typed intent plus evidence-bounded receipt.

Unsupported axis/position/stop, invalid position, obstruction, safety lockout, not calibrated, conflicting stop/movement, and unsupported operation are exhaustive domain details. Streams hydrate before live observations and expose all gaps.
