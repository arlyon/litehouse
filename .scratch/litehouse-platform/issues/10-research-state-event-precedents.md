# Research state and event precedents

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which established local-first automation, event-log, streaming, and device-state
models provide useful precedents for query hydration, revision scope, ordering,
duplicates, timestamps, lag signals, replay/resume, reconnect, and generation
changes—and which semantics would be unsafe or misleading for Litehouse v1?

## Answer

Partial research compared Kubernetes list/watch, MQTT 5, Kafka, OPC UA, Home
Assistant, CloudEvents, and ESPHome. No one model supplies Litehouse's complete
contract, but several narrow precedents are useful:

- Kubernetes demonstrates snapshot hydration followed by an explicit caught-up
  cursor and mandatory re-list after history expires; its resource versions are
  opaque and scope-bound, not global revisions.
- MQTT demonstrates retained latest values and at-least-once duplicate delivery,
  but retained topics are not an atomic snapshot and QoS is not device command
  acknowledgement.
- Kafka demonstrates ordered per-partition cursors, bounded replay, and visible
  lag, but no total cross-partition order or exactly-once external effects.
- OPC UA usefully separates value, availability/quality, source time, receive
  time, sequence gaps, and bounded republish.
- CloudEvents provides event `(source, id)` and optional occurrence time, but no
  ordering, replay, or delivery guarantee.
- Home Assistant and ESPHome expose useful current-state/subscription patterns
  but no race-free revisions, cursors, replay, or trustworthy source time.

Unsafe claims include treating equal payloads as duplicates, cached values as
available, source clocks as authoritative, transport acknowledgements as device
actuation, cursors as globally comparable, or exactly-once delivery as
exactly-once effects.

Primary sources: [Kubernetes API concepts](https://kubernetes.io/docs/reference/using-api/api-concepts/),
[MQTT 5](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html),
[Kafka design](https://kafka.apache.org/43/design/design/),
[OPC UA subscriptions](https://reference.opcfoundation.org/Core/Part4/v105/docs/5.14.1),
[CloudEvents 1.0.2](https://github.com/cloudevents/spec/blob/v1.0.2/cloudevents/spec.md),
[Home Assistant WebSocket API](https://developers.home-assistant.io/docs/api/websocket/),
and [ESPHome Native API](https://developers.esphome.io/architecture/api/protocol_details/).
