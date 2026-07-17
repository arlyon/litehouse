# `litehouse:boolean-state@1.0.0`

Boolean State is read-only and never implies an actuator. The built-in semantic profile fixes polarity/meaning for contact, occupancy, smoke, status, moisture, vibration, or honest generic use. A source whose meaning is not honest enough for generic publication remains diagnostic.

`current: none` is unknown, never false. Availability and quality are independent; stale/unavailable state may retain a last value, and `source-assumed` is not verification. Descriptor event sets feature-gate source-provided occurrences.

Subscriptions emit snapshot, caught-up barrier, then observations and optional occurrences. Every occurrence is at least once and retains a stable event ID, source identity, and generation on retransmission. Gaps are explicit; no durable replay is promised. Request deadlines bound describe/query/next calls.

`unsupported-event` is the only domain detail in major 1. Binding, availability, timing, generation, and runtime failures use the shared core envelope without inventing a Boolean value.
