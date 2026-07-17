# `litehouse:on-off@1.0.0`

On/Off is an actuator facet. Boolean sensors MUST use `boolean-state` instead.

`current: none` means unknown and is distinct from `false`; availability is independent and a disconnected facet may retain its last current value. `source-assumed` means the source reports assumed state, not physical verification. A subscription MUST emit snapshot, caught-up barrier, then live observations or an explicit gap.

Providers advertise every supported feature and operation. `set` is absolute/convergent; `toggle` is non-idempotent and MUST NOT be automatically retried. Timed, startup, effect, dead-front, and off-only semantics exist only when named. Unsupported is never emulated or interpreted as unknown state. Timed/effect requests without their feature return typed contract detail.

All calls use shared request/deadline context. Successful operations return the requested intent plus a typed receipt. A provider such as ESPHome that lacks acknowledgement MUST claim at most `accepted-for-delivery`. Loss after possible dispatch is `outcome-unknown`. The listed domain-detail variants are exhaustive for this major; host-synthesized errors carry no detail.
