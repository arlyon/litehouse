# Prototype real ESPHome capabilities

Type: prototype
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 17, 18, 20, 21
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Does a disposable real Native API prototype prove one configured provider
instance per approved node, one plugin-owned shared session/cache per instance,
stable Litehouse facet identities without a nested device key, metadata-only UI
groups, several independently bindable capabilities represented by the approved
hardware (including On/Off plus feature-gated Level/Color), conservative
measurement/profile mapping, typed command receipts and observed-state quality,
reconnect/topology handling, isolation across multiple instances, and equivalent
deterministic secret-free fixtures without claiming hardware atomicity or
confirmation?

## Answer

The disposable [ESPHome capability spike](../prototypes/esphome-capabilities/)
and one bounded real Node S proof establish the simple provider route.

Use one configured plugin instance, encrypted Native API connection, actor, and
shared entity/state cache per node. Entity listing creates independently
bindable facets keyed internally by the current `(device_id, key)` route while a
Litehouse-owned identity table preserves stable public facet IDs. State
subscription hydrates current observed values; disconnect marks the instance
unavailable and increments its generation; reconnect re-lists atomically and
reconciles the same source routes. Metadata groups never participate in identity
or binding.

The supervised read-only proof connected twice to Node S over Noise. The node
reported ESPHome 2026.3.2, four switch entities, three binary sensors, all seven
initial states, and identical stable Litehouse identities after reconnect. No
address, key, hostname, object ID, device name, or state value entered the
planning artifacts or logs.

The confirmed local YAML explains the four switches: one template plus three
GPIO switches. Native API presents all four as ordinary switch entities and does
not expose their source platform. The provider must therefore publish four valid
On/Off facets by default; explicit adoption/configuration may hide the template
facet, but the provider must not guess that only GPIO entities are physical.

Wire mapping is direct:

- advertised switch → On/Off facet;
- advertised binary sensor → Boolean State facet;
- light → On/Off plus Level and/or Color only from advertised support;
- sensor → Measurement only for an exact built-in device-class/unit profile;
- unsupported or unknown entities → diagnostic inventory only;
- absolute switch command → `SwitchCommandRequest(device_id, key, state)`; and
- successful write → accepted-for-delivery receipt with `confirmed=false`.

A later state update remains an observation, not command acknowledgement or
physical completion. Do not use Toggle, retry ambiguous writes, claim hardware
atomicity, or infer features/units/ranges absent from discovery.

Fixtures prove three repeated relay facets plus three Boolean facets, metadata-
only groups, exact stable identity across reconnect, facet/store staleness,
two-instance isolation, conservative temperature/humidity/carbon-dioxide/
particulate profiles, unknown VOC rejection, feature-gated Level/Color, typed
observed/unknown/unavailable quality, and non-confirming receipts. The user
chose to stop real access after Node S, so Node A measurements and Level/Color
remain fixture-proven rather than hardware-proven; no real relay command was
sent.

Library evidence:

- `esphome-client` 0.2.0 is the usable Rust client. Pin API 1.14 and disable its
  mDNS discovery feature; it interoperated with the real 2026.3.2 node.
- `esphome-native-api` 2.1.0 supplies current 2026.6.2 protobufs and a useful
  deterministic Noise fixture server, but `EspHomeApi::start` behaves as the
  server-side responder despite client-oriented documentation. Do not base the
  provider client on that API without an upstream fix.
- A fixed-key encrypted fixture proved selected API 1.14 client messages decode
  against the 2026.6.2 server/protobuf source. Real compatibility still fails
  closed on a protocol-major mismatch.

Validation passed:

```text
cargo test --manifest-path \
  .scratch/litehouse-platform/prototypes/esphome-capabilities/Cargo.toml
  7 capability/identity tests passed
  1 encrypted cross-version fixture test passed

cargo clippy --manifest-path \
  .scratch/litehouse-platform/prototypes/esphome-capabilities/Cargo.toml \
  --all-targets -- -D warnings
  passed

real Node S preflight
  2 encrypted sessions; topology/state/reconnect proof passed
```

Do not extend this spike into a production component before **Define the
contract artifact format**. Implementation should move this small mapping into
the generated plugin interface, retain the per-instance session actor, and keep
wire fixtures equivalent to the native model—without adding reflection,
platform heuristics, or another device abstraction layer.
