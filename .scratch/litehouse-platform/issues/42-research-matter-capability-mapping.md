# Research Matter capability mapping for the v1 catalog

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which current Matter data-model boundaries, feature maps, attributes, commands,
events, semantic tags, units, quality/nullability rules, and composition patterns
should Litehouse preserve for its ESPHome-first On/Off, Level, Color,
Boolean/Numeric Measurement, Thermostat, Window Covering, button/action, and
notification capabilities so a later full Matter plugin maps without semantic
loss—while exact-major ergonomic WIT deliberately omits Matter transport,
fabric, numeric-ID, sentinel, and code-generation complexity?

## Answer

Research used released `connectedhomeip` v1.5.1.0 at commit `abcc720b` as
the data-model baseline. Its cluster boundaries are a strong compatibility
shape for Litehouse, but its wire encodings and fabric machinery are not an
ergonomic plugin contract.

Preserve these Matter semantics:

- independently bindable, repeatable capabilities composed into endpoint-like
  groups with server/source versus client/sink direction and namespaced semantic
  tags;
- distinct current state/configuration, commands, and occurrence events;
- named feature-dependent attributes, operations, and events;
- separate unsupported, nullable/unknown, unavailable, and erroneous states;
- source ranges, steps, tolerance, scale/quantization, color model, and
  nullability when projecting values;
- protocol acknowledgement strength without treating success as universal proof
  of physical completion; and
- cluster data versions and priming subscriptions as inputs to Litehouse's
  generation-qualified revisions/hydration, never as durable global cursors.

Translate or omit numeric node/endpoint/cluster/field IDs, raw feature bitmaps,
TLV and sentinel encodings, fabrics/ACL privileges, interaction framing,
subscription IDs, raw data versions, retransmission machinery, and generated
conformance expressions. A Matter provider retains its complete endpoint graph
privately and may expose separate Matter-specific exact-major interfaces later;
portable facets do not pretend to be the full Matter ontology.

Capability implications:

- **On/Off:** preserve absolute set and optional non-idempotent toggle. Timed,
  startup, lighting-effect, dead-front, and off-only behavior are explicit
  optional features rather than hidden basic semantics.
- **Level Control:** preserve absolute target, relative step, continuous move,
  stop, native range/quantization, transition, and whether an operation couples
  to On/Off. Cover position is not generic Level Control.
- **Color Control:** one feature-gated capability preserves hue/saturation, CIE
  xy, color temperature, active model, physical temperature range, and optional
  move/step/transition operations. Provider-native RGB/white-channel models
  remain distinct. RGB↔xy/HS and channel-white↔temperature are not assumed
  lossless; v1 performs no weaker emulation or silent conversion.
- **Boolean State:** keep a read-only Boolean distinct from On/Off. Meaning comes
  from a built-in semantic profile/group role; unknown is not false, and state
  transitions remain distinct from occurrence events.
- **Measurement:** Matter's quantity-specific nullable values, ranges,
  tolerance, scaling, and special encodings can project into Litehouse's
  canonical decimal plus built-in quantity profile if native metadata is
  retained. Logarithmic illuminance, mired/Kelvin conversion, unknown source
  units, and exact IEEE bit patterns need explicit adapters/private retention;
  they are not silently normalized.
- **Thermostat:** preserve supported mode set, current mode versus current
  action/running state, single versus heat/cool dual setpoints, bounds/steps,
  nullable current values, and optional fan/swing/preset features. Use typed
  operations rather than exposing writable-attribute mechanics. Schedules,
  atomic list writes, setbacks, and provisional Matter features are deferred.
- **Window Covering:** one capability has independently optional lift and tilt
  axes, current/target values, movement, open/close/stop, position-awareness,
  and safety/fault metadata. Generic position never implies installed-unit
  absolute positioning.
- **Momentary Action:** keep command sinks separate from Matter-style physical
  switch/gesture event sources and Boolean state. Press and relative/action
  operations are non-idempotent by default.
- **Notification Sink:** Litehouse v1 is deliberately weaker than Matter
  Messages: bounded text, optional title/priority/expiry, caller idempotency
  context, and the strongest provider receipt available. Matter's response,
  reply, confirmation, scheduling, cancellation, and fabric-scoped lifecycle
  may be projected only when supported; basic notifier success never implies
  presentation.

Composition metadata should retain one primary provider-scoped group identity,
capability direction, built-in semantic roles/profile IDs, and namespaced tags.
It remains metadata: bindings, permissions, state, and operations target exact
facets. Litehouse chooses UI renderers; plugins provide no executable UI.

Matter Invoke can return a stronger path-specific acknowledgement than ESPHome,
but neither guarantees physical completion universally. Receipts therefore
separate rejected-before-delivery, accepted-for-delivery, protocol/target
acknowledged, completed, observed-completed, and ambiguous outcome. Toggle,
step, move, press, action, and notification are never automatically retried;
absolute setters are retryable only when their operation and provider actually
supply the required idempotency/deduplication semantics.

There is no blocker to later full Matter support if v1 retains exact feature,
range, quantization, color-model, nullability, event, and group metadata. The
main compatibility hazards are RGB-only color, generic percentages for
unrelated domains, closed climate enums, null conflated with unsupported or
unavailable, events collapsed into state, notification success presented as
confirmed, automatic command retries, and discarded source representations.

Focused implementation tests must cover Matter data-version
rollover/resubscription, acknowledgement categories, logarithmic illuminance,
color gamut/channel projections, dual-setpoint/custom-mode climate, cover
position/tilt/stop, and the approved notifier transport. These are validation
cases, not additional mandatory Wayfinder prototypes.

Primary sources:

- [`connectedhomeip` v1.5.1.0](https://github.com/project-chip/connectedhomeip/tree/v1.5.1.0)
- [On/Off](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/onoff-cluster.xml),
  [Level Control](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/level-control-cluster.xml),
  and [Color Control](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/color-control-cluster.xml)
- [Thermostat](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/thermostat-cluster.xml)
  and [Window Covering](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/window-covering.xml)
- [Boolean State](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/boolean-state-cluster.xml),
  [Switch](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/switch-cluster.xml),
  and [Messages](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/messages-cluster.xml)
- [Descriptor and semantic composition](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/descriptor-cluster.xml)
