# `litehouse:level-control@1.0.0`

Portable levels use the quantity ratio profile with canonical unit `1` and magnitude in `[0,1]`. Native range, step, and quantization metadata preserve exact source projection. Current and target are separate optional fields; absent means unknown, and availability remains independent.

Providers advertise state fields, operations, transitions, and coupling explicitly. Basic `set-level`, `step`, and `move` never secretly alter On/Off. `set-level-with-on-off` is available only with named coupled support. A provider MUST NOT synthesize step/move/transition from weaker primitives or silently quantize an inexact request.

Absolute set is convergent. Step and continuous move are not automatically retried; stop is also not replayed after ambiguous dispatch. Every operation carries request, deadline, attempt, and optional idempotency context and returns its typed intent with receipt confidence no stronger than evidence.

Out-of-range, unsupported operation/coupling/transition, invalid rate, and invalid step are the exhaustive domain details. Unknown value, unavailable facet, unsupported operation, and stale generation remain distinct typed states/errors.
