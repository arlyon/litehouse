# `litehouse:thermostat@1.0.0`

Thermostat owns mode, action/running state, setpoint model and bounds, deadband, and advertised fan/swing/preset choices. Modes are stable known tokens or provider-qualified custom tokens. Providers MUST advertise either single or heat/cool setpoints and MUST accept a complete dual range atomically; partial dual writes are invalid.

Current mode and target mode are distinct optional fields. Unknown state, unavailable transport, unsupported choice, and source-assumed quality are never collapsed. Schedules, setbacks, and atomic list writes are outside v1.

Ambient temperature and humidity are separate Measurement facets in the same composition group, not duplicated fields. Composition affects presentation only.

Every operation is feature-gated, deadline-bounded, and returns its typed intent with shared receipt confidence. Absolute mode/target operations may be retried only with matching deduplication; optional choices are not assumed safe. Unsupported mode/choice, wrong setpoint model, incomplete/inverted range, deadband violation, out-of-range target, safety lockout, and unsupported operation are exhaustive domain details. Provider visual ranges are guidance, not fabricated hardware guarantees.
