# Define the vendor-neutral contract catalog

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: 04, 15, 16, 17, 41, 42, 43
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which exact-major WIT interfaces, capability composition, operations, state,
metadata, units, idempotency, deadlines, and domain errors form the first-party
catalog for switch, light (on/off, brightness, color), binary/numeric sensors,
climate, cover, button, notifier, and other approved ESPHome domains without
leaking ESPHome routing details or inventing a universal ontology?

## Answer

Publish nine independent exact-major capability contracts, plus shared state,
error, receipt, quantity, and descriptor dependencies:

1. `on-off@1`
2. `level-control@1`
3. `color-control@1`
4. `boolean-state@1`
5. `measurement@1`
6. `thermostat@1`
7. `window-covering@1`
8. `momentary-action@1`
9. `notification-sink@1`

The catalog is deliberately Matter-informed but not a Matter wire model. It
preserves useful cluster boundaries, feature negotiation, source
representations, nullability, occurrence events, and endpoint-like composition
while replacing numeric IDs, bitmaps, sentinels, fabrics, and writable-attribute
mechanics with ergonomic generated WIT.

### Common capability shape

Every facet descriptor carries provider-instance identity, stable facet key,
host-tracked incarnation/generation, exact contract identity, one stable primary
composition-group key, direction, bounded human label, built-in semantic
role/profile IDs, named features/operations/events, and namespaced semantic
tags. Labels, tags, source routing keys, and list position are not binding
identity. There is no nested `device-key`.

Composition groups are presentation metadata only. A provider uses them to say,
for example, that On/Off, Level, and Color facets form one light while repeated
relay facets remain separate. Each facet has exactly one primary group; UI views
may reference it elsewhere without duplicating identity. Plugins supply semantic
hints only—no renderer names, HTML, components, or executable UI. Litehouse
chooses accessible renderers from the exact contract/feature set.

Supported features, operations, state fields, and events are explicit named
sets. Unsupported is distinct from a supported-but-unknown value. State uses the
shared generation-qualified revision, hydration, availability, freshness, gap,
and replay model. Capability fields distinguish current from target where that
matters. Quality is narrowly `source-reported` or `source-assumed`; neither means
physically verified. Unknown and unavailable remain separate.

Commands carry the host request/correlation context, absolute deadline, and
optional idempotency context through generated proxies. Every successful command
returns a typed receipt whose confidence is one of accepted-for-delivery,
protocol/target-acknowledged, completed, or observed-completed. Loss after
possible dispatch returns `outcome-unknown`; later matching state is never
silently treated as causal confirmation.

Operation metadata distinguishes absolute/convergent intent from relative,
continuous, or momentary effects. Idempotency keys authorize retry only when the
provider actually offers the matching deduplication guarantee. Toggle, move,
step, press, action, and notification never auto-retry. Numeric deadline defaults
and maxima wait for baselines/SLOs, but no operation is unbounded and queue wait
counts against the caller deadline.

Providers may privately fuse, coalesce, or push down work only when every
observable contract guarantee is preserved. V1 has no weaker emulation, generic
optimizer, or silent fallback. Contract documentation reserves a future
plan/lowering seam that must expose native versus emulated provenance and any
weakened atomicity, timing, cancellation, or confirmation guarantee.

### Capability contracts

#### On/Off

State is optional current Boolean plus availability/quality. Operations are
absolute `set` and optional non-idempotent `toggle`. Timed/startup/effect,
dead-front, and off-only behavior are named optional features rather than hidden
semantics. Switches and lights may provide On/Off; Boolean sensors never do.

Domain detail covers unsupported operation/feature and invalid timed options.
ESPHome `assumed-state` maps to source-assumed, and successful commands report at
most accepted-for-delivery.

#### Level Control

State preserves optional current and target levels, native min/max/step or
quantization, transition remaining when genuinely available, and named role such
as brightness. Portable levels use canonical decimal ratio semantics while
retaining native quantization metadata for exact Matter projection.

Operations may include absolute set, relative step, continuous move, stop, and
transition. Basic operations never secretly affect On/Off. A provider may expose
explicit Matter-style coupled variants; composition UI selects them deliberately
and the provider may fuse them exactly. ESPHome exposes the operations its
advertised light modes can faithfully perform, not synthetic move/step behavior.

Domain detail covers out-of-range, unsupported operation/coupling/transition,
and invalid rate or step.

#### Color Control

One feature-gated capability preserves model-specific current and target values,
active model, physical ranges/gamut where known, and optional absolute,
move/step/stop/transition operations. Models remain distinct:

- hue/saturation;
- CIE xy chromaticity;
- correlated color temperature;
- calibrated RGB with declared color space/gamut;
- explicitly uncalibrated normalized RGB channels; and
- provider-native white/cold-warm channel models needed for lossless control.

No model is universal. RGB↔xy/HS, mired↔Kelvin after quantization, and
white-channel↔temperature conversions are never silently presented as lossless.
ESPHome's uncalibrated RGB channels remain controllable without being mislabeled
sRGB; effects and Matter color-loop stay outside the basic v1 operations.

Domain detail covers unsupported model, out-of-range, out-of-gamut,
unrepresentable/lossy conversion, and invalid transition.

#### Boolean State

A read-only optional Boolean has a built-in semantic profile/role and may expose
an occurrence event for source-provided state changes. Unknown is not false.
Contact, occupancy, smoke, status, and other meanings do not collapse into an
unlabeled Boolean, but no actuator command is implied. Unknown source classes use
a generic built-in Boolean profile only when polarity/meaning remain honest;
otherwise they stay diagnostic.

#### Measurement

A measurement contains an optional canonical quantity plus source/native
representation metadata, measurement kind (`instantaneous`, `total`, or
`total-increasing`), and optional min/max, resolution/step, tolerance, and native
quantization. Display accuracy is not asserted as sensor resolution.

The quantity wire uses canonical decimal strings and a unit fixed by an
immutable exact-major built-in profile. Bounded case-sensitive UCUM 2.2 input is
normalized by one Rust engine shared to TypeScript through Wasm. Addition and
subtraction require the same semantic profile; multiplication/division/powers
must resolve to another built-in profile. Absolute temperature and temperature
difference are distinct. Custom profiles/units are not accepted in v1.

The initial common-home profile manifest covers temperature and temperature
difference, relative humidity, pressure, illuminance, power, energy, voltage,
electric current, frequency, battery ratio, signal strength, carbon dioxide,
carbon monoxide, volatile organic compounds, PM1/PM2.5/PM10, distance, speed,
volume, and volumetric flow. Profiles are versioned catalog IDs rather than a
closed WIT enum, so additive catalog releases can expand coverage without
changing `measurement@1`; an existing profile's meaning never changes in place.

An ESPHome sensor must match an approved device-class/unit/profile mapping.
Unknown or ambiguous mappings remain visible in plugin diagnostics but do not
publish a misleading vendor-neutral Measurement facet. Matter logarithmic or
quantized source encodings stay private alongside exact projection metadata.

Domain detail covers unsupported profile/unit, invalid/non-finite value,
dimension/profile mismatch, affine misuse, overflow, inexact result requiring
quantization, and out-of-range source data.

#### Thermostat

Thermostat owns control semantics: device-declared stable mode choices (portable
known tokens plus provider-qualified custom tokens), current mode, current
action/running state, single versus heat/cool dual targets, target bounds/steps,
deadband rules, and optional fan/swing/preset choices. Typed operations set mode,
single target, complete dual-target range, and advertised optional choices.
Schedules, setbacks, atomic list writes, and provisional Matter features are
deferred.

Current ambient temperature/humidity are reusable Measurement facets with roles
in the same composition group, not duplicate fields inside Thermostat. The UI
renders one thermostat while generic consumers bind the measurements normally.

Domain detail covers unsupported mode/choice, wrong setpoint model, incomplete or
inverted range, deadband violation, out-of-range target, and safety lockout.
ESPHome visual ranges are treated as provider guidance, not fabricated hardware
guarantees.

#### Window Covering

One capability has independently optional lift and tilt axes, each with optional
current/target ratio, position-awareness and native quantization. It also exposes
movement state, open/close, optional stop, optional target positioning, and
source-supported safety/fault metadata. Generic normalized position never
implies Matter installed-unit absolute positioning.

Domain detail covers unsupported axis/position/stop, invalid position,
obstruction/safety lockout, not calibrated, and conflicting stop/movement.
ESPHome assumed state remains source-assumed; idle is not proof of completed
movement.

#### Momentary Action

A stateless parameterless `invoke` capability has semantic action identity, risk
metadata, deadline, and typed receipt. It is non-idempotent unless the provider
supplies durable deduplication. It never invents pressed/released state or a
physical-switch event stream.

Domain detail covers rejected, busy, safety lockout, and unsupported action.

#### Notification Sink

V1 sends bounded plain text with optional title, priority, expiry, and caller
idempotency context. The facet advertises exactly which fields and receipt levels
it supports. Domain detail covers invalid/oversized content, unsupported field,
rate limit, expired message, and provider rejection.

The contract is intentionally weaker than Matter Messages: response choices,
replies, scheduling, cancellation, presentation lifecycle, and fabric scoping
are optional future extensions. ESPHome has no uniform Native API notifier;
provider selection belongs to the plugin-portfolio decision. Accepted delivery
never implies presentation.

### Conformance and validation

Each contract release includes generated Rust provider/consumer helpers,
descriptor and feature validators, canonical state/error/receipt fixtures, and
provider conformance tests. The service-fabric prototype must exercise repeated
facets/groups, exact-major routing, unsupported versus unknown fields, current
versus target, source-assumed quality, operation negotiation, acknowledgement
strength, deadline/cancellation, and no cross-store guest resources.

The ESPHome prototype/fixtures must cover every exposed color-mode family,
partial grouped commands, assumed/missing/default-valued state, measurement
mapping failures, single/dual climate setpoints, cover axes/stop, and button
requests with no response. It proves grouped provider calls, not hardware
transactions or confirmation. Future Matter validation must preserve source
range/scale/color/nullability and test data-version rollover/resubscription,
acknowledgement categories, logarithmic illuminance, and endpoint composition.
