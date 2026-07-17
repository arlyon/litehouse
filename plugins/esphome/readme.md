# ESPHome provider

`esphome` is a `wasm32-wasip2` Litehouse provider for one explicitly configured ESPHome Native API node. One component instance owns one encrypted Noise session and state cache shared by every facet.

Initial mapping:

- normal ESPHome switch → `litehouse:on-off@1`
- normal ESPHome binary sensor → `litehouse:boolean-state@1`
- diagnostic and unsupported entities → not published as facets

Connector configuration contains a literal socket address and a runtime-injected `noise_key`. The key must remain null/absent in persisted configuration and be resolved by the host from protected environment input before component start. Host authority is restricted independently to the same exact TCP socket; mDNS and hostname discovery are not supported.

Switch control uses only absolute `set(bool)`. A successful protocol write reports `accepted-for-delivery`; it does not update cached state or claim device confirmation. A later state observation is independent. A write failure is `outcome-unknown` and is never blindly retried.
