# ESPHome provider

`esphome` is a `wasm32-wasip2` Litehouse provider for one explicitly configured ESPHome Native API 1.14 node. One component instance owns one encrypted Noise session and cache shared by every facet.

Conservative mappings:

- switch, light, fan, and binary siren output → `litehouse:on-off@1`
- binary sensor → `litehouse:boolean-state@1`
- numeric sensor → `litehouse:measurement@1` only for the built-in quantity catalog's exact device-class/unit whitelist
- light → `litehouse:level-control@1` and losslessly representable advertised RGB, color-temperature, or white `litehouse:color-control@1` models
- speed-capable fan → `litehouse:level-control@1`
- climate → `litehouse:thermostat@1` with only advertised modes, target model, fan, swing, and preset choices
- cover → `litehouse:window-covering@1` with only advertised position, tilt, and stop support
- button → `litehouse:momentary-action@1`

All facets from one entity share its stable `(entity type, device_id, key)` group key. Descriptor topology changes advance generation and incarnation; reconnect hydration preserves confirmed cache only as explicit stale/gap state until a fresh source observation arrives. Unknown units and unsupported text, select, number, lock, media, alarm, update, camera, event, date/time, valve, service, infrared/radio, and other entities produce bounded internal diagnostics and never fabricated facets. Combined light channel modes that cannot be represented losslessly are not advertised as color models. The deterministic Noise fixture cannot exercise siren frames because its pinned server parser omits protocol IDs 55–57; siren topology and command construction remain unit-covered.

Connector configuration contains a literal socket address and a runtime-injected `noise_key`. The key must remain null/absent in persisted configuration and be resolved by the host from protected environment input before component start. Host authority is restricted independently to the same exact TCP socket; mDNS and hostname discovery are not supported.

Commands use only absolute component writes (or a momentary button invocation); no Toggle, relative, continuous, effect, or implicit transition operation is exposed. A successful socket write reports `accepted-for-delivery` and does not mutate cached state or claim device confirmation. A possible dispatch failure reports `outcome-unknown` and is never blindly retried.
