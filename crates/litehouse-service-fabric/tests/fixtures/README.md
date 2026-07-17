# Service-fabric Component Model fixtures

These WIT worlds and components are **test-only**. They exist solely to prove
Litehouse's generated provider-resource/consumer-proxy partition against real
Wasmtime stores.

They are not published contracts and are not a generic service API. Every call
is statically typed (`query`, `set`, `rebind`); there is no reflected contract,
operation-name dispatch, generic payload, provider-selected target, or dynamic
resource handle. The copied `litehouse:errors@1.0.0` dependency is byte-checked
against the T1 source by the T0 baseline checker.
