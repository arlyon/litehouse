# Research ESPHome capability semantics for the v1 catalog

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

What do current ESPHome Native API protobufs and first-party implementations
actually guarantee for switch, light, binary sensor, numeric sensor, climate,
cover, and button entities—including discovery metadata, supported-feature and
mode negotiation, units/device classes, state and command fields, ranges,
scaling, transition behavior, acknowledgement, missing/unknown values, and
cross-field atomicity—and how can the nine approved Litehouse v1 capabilities
map without fabricating semantics?

## Answer

Source-level research used ESPHome 2026.6.5 and verified that the relevant
entity protobufs were unchanged at `dev` commit `65d6c028`. The Native API can
support the approved catalog conservatively, but its discovery feature flags
must gate every state field and command. Proto3 defaults are not evidence that
an unsupported field is present.

Protocol-wide constraints:

- Discovery emits type-specific entity records followed by a done marker; state
  subscription is separate. Common names, object IDs, icons, device classes,
  and units are opaque metadata rather than standardized identities.
- Switch, light, climate, cover, and button commands all return `void`. There is
  no application acknowledgement, request ID, rejection response, or causal
  state revision. Litehouse can report accepted-for-dispatch or ambiguous
  outcome, never device confirmation.
- Only binary and numeric sensors have explicit `missing-state`. Other state
  messages require discovery-gated interpretation; unsupported fields decode
  to scalar defaults. Non-finite numeric values must be rejected.

Capability mappings:

- **On/Off:** switches expose Boolean state and set-state. Preserve
  `assumed-state` as observation quality; it is not availability or confirmed
  feedback. Switch state has no explicit unknown marker.
- **Level and Color Control:** light `supported-color-modes` is authoritative.
  Brightness/RGB/white channels are normalized floats; color temperature is in
  mireds with advertised bounds. Commands are partial grouped desired-state
  updates with optional transition/flash/effect fields. One message becomes one
  `LightCall`, but validation may drop individual fields and hardware atomicity
  or transition progress is not guaranteed. Feature-gate every model and never
  silently convert or claim physical completion.
- **Boolean State:** binary sensors provide Boolean plus `missing-state`.
  Device class and polarity remain opaque unless they map to an approved
  built-in profile. A status sensor is not availability for unrelated facets.
- **Measurement:** numeric sensors provide one `float`, `missing-state`, opaque
  unit/device-class strings, display accuracy, `force-update`, and state class
  (`measurement`, `total`, `total-increasing`, or angle). They provide no range,
  resolution, scale, or offset. Only approved unit/profile mappings may enter
  canonical quantity math; display precision is not sensor resolution.
- **Thermostat:** discovery advertises supported modes, current/target
  temperature and humidity features, one-point versus two-point setpoints,
  action, fan/swing/preset choices, and visual ranges/steps. Those ranges are UI
  guidance rather than enforced device bounds. Commands are partial and may
  drop invalid fields individually. The current implementation does not
  reliably populate its temperature-unit field; transport values must be
  treated as Celsius unless a tested implementation proves otherwise.
- **Window Covering:** discovery gates proportional lift, tilt, and stop.
  Position is normalized closed `0` to open `1`; binary open/close remains
  available without proportional support. Preserve assumed-state quality.
  Stop is exclusive and wins over movement fields. Do not infer obstruction,
  calibration, or completed movement from idle state.
- **Momentary Action:** a button has a parameterless press command with no state,
  release, duration, result, or acknowledgement. Completion means dispatch
  only.

ESPHome cannot supply entity revisions, replay identities, universal initialized
state, hardware acknowledgement, transactional multi-field application,
reliable transition progress, cover safety/completion, or exactly-once effects.
The shared Litehouse state/error/receipt model must expose those limitations
rather than fabricate them.

Deterministic fixtures must pair discovery with state and cover every feature
family, missing/default-valued fields, unknown units/classes, assumed state,
partial and conflicting commands, non-finite values, grouped light/climate/cover
calls, disconnects, and buttons with no response. Real-node proof records
outbound commands and later state independently; acceptance stops at one decoded
protobuf becoming one ESPHome call, not hardware atomicity or acknowledgement.

Primary sources:

- [ESPHome `api.proto`](https://github.com/esphome/esphome/blob/65d6c028cea339b1e8baa1fcb456afbe76f3d210/esphome/components/api/api.proto)
- [`api_connection.cpp`](https://github.com/esphome/esphome/blob/65d6c028cea339b1e8baa1fcb456afbe76f3d210/esphome/components/api/api_connection.cpp)
- [`light_call.cpp`](https://github.com/esphome/esphome/blob/65d6c028cea339b1e8baa1fcb456afbe76f3d210/esphome/components/light/light_call.cpp)
- [`climate.cpp`](https://github.com/esphome/esphome/blob/65d6c028cea339b1e8baa1fcb456afbe76f3d210/esphome/components/climate/climate.cpp)
- [`cover.cpp`](https://github.com/esphome/esphome/blob/65d6c028cea339b1e8baa1fcb456afbe76f3d210/esphome/components/cover/cover.cpp)
