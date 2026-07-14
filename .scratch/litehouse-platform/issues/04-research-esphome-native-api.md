# Research ESPHome Native API constraints

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

What current Native API behavior constrains a multi-node provider exposing many
independent vendor-neutral facets?

## Answer

Partial research snapshot against ESPHome 2026.6.5, current protocol sources,
and `aioesphomeapi` 45.6.0:

- Every node is an independent TCP server, normally port 6053; one session cannot
  multiplex nodes. Noise uses a PSK and should be required for controllable
  production nodes rather than silently falling back to plaintext.
- Discovery is a bounded device-info/entity-list snapshot followed by state
  subscription. There is no entity-added/removed event or resume cursor;
  topology changes require reconnect, re-list, diff, and atomic republish.
- Switch routing uses `(device_id, key)`. The key derives from a hash of the
  object identity and can change after rename; it is not collision-proof or a
  durable Litehouse identity.
- Switch commands have no protocol acknowledgement. A successful write means
  accepted for delivery, not applied. A later state update is observation, not
  causal proof. State messages have no timestamp, revision, sequence, or replay
  token.
- Use one bounded session actor/cache/reconnect lifecycle per node, shared by all
  its facets. Persist Litehouse-owned device/facet identities separately from
  current ESPHome routing locators. Invalidate old generations on disconnect or
  republish.
- Deterministic tests should pin official protobufs, golden framing vectors,
  fixed non-secret Noise fixtures, and scripted multi-node fake servers; retain
  a separately approved real-node proof.

Primary sources: [protocol details](https://developers.esphome.io/architecture/api/protocol_details/),
[`api.proto`](https://github.com/esphome/esphome/blob/5e3e2f82c9800bc232b0c9c9d962418a1b4f7d44/esphome/components/api/api.proto),
and [`aioesphomeapi`](https://github.com/esphome/aioesphomeapi/tree/v45.6.0).

Litehouse must decide durable identity reconciliation, command semantics,
revision/timestamp behavior, reconnect policy, and the broader contract catalog;
the wire protocol does not supply those guarantees.
