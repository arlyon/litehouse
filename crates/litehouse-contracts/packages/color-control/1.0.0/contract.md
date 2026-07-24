# `litehouse:color-control@1.0.0`

Color is one feature-gated facet with distinct models: hue/saturation, CIE xy, correlated color temperature, calibrated RGB with declared color space, explicitly uncalibrated normalized RGB, and provider-native white/cold/warm channels. Supported models, operations, ranges, gamut, and quantization are advertised explicitly.

Current color, target color, and active model are independent optional fields. Unknown is not unsupported; either may be retained while unavailable. `source-assumed` is not physical verification.

Providers MUST NOT silently present RGB↔xy/HS, quantized Kelvin↔mired, or white-channel↔temperature conversion as lossless. A requested model is handled natively with declared semantics or rejected as unsupported, out-of-range/gamut, unrepresentable, or lossy. Effects and color-loop are outside basic v1.

Set is absolute/convergent. Step, move, and stop are not automatically retried. Every call carries shared request/deadline/idempotency context and returns a typed intent plus receipt; ambiguous dispatch returns `outcome-unknown`. Subscription ordering and gap recovery follow `litehouse:state@1`.
