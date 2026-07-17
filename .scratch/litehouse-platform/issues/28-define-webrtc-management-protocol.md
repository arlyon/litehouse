# Define the WebRTC management protocol

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-17
Blocked by: 07, 16, 27
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 9

## Question

What is the smallest authenticated RPC protocol the web UI needs over a broker-facilitated WebRTC connection?

## Answer

Use the broker for authenticated signaling only. After offer/answer and ICE setup,
the browser and Litehouse server communicate directly over one reliable, ordered
WebRTC data channel. Routine management payloads do not pass through or persist in
the broker.

Carry standard JSON-RPC 2.0 text messages on that channel. Do not define a second
custom management envelope or transport stack. RPC method implementations call
the same canonical T8 handlers as local HTTP, with the same authorization,
validation, revisions, idempotency where the operation requires it, and typed
Litehouse errors.

This decision supersedes the earlier custom framing, transcript, flow-control,
resume, and transfer protocol in this issue.

## Connection and authentication

- T10 authenticates the owner, pairs the server, and issues a short-lived grant
  for the exact owner/client/server/pair epoch.
- The broker authorizes and correlates signaling but does not become a management
  relay.
- The browser opens one reliable, ordered data channel named
  `litehouse-management`.
- The first RPC is `session.open`, carrying protocol version `1` and the
  short-lived T10 grant. No management method is accepted before it succeeds.
- Litehouse validates the grant with the normal T10/T8 authorization seam and
  binds the resulting principal to the peer connection.
- Expiry or trust/pair-epoch change closes the channel. V1 does not reauthorize an
  existing channel in place; the client reconnects with a fresh grant.

WebRTC's DTLS transport and the broker-authorized peer/session provide the
connection security boundary. V1 does not hash raw SDP or define a separate
application transcript protocol.

## JSON-RPC surface

Use JSON-RPC 2.0 requests, responses, errors, and notifications:

```json
{"jsonrpc":"2.0","id":"17","method":"device.set","params":{"id":"lamp","on":true}}
{"jsonrpc":"2.0","id":"17","result":{"accepted":true}}
{"jsonrpc":"2.0","method":"state.changed","params":{"id":"lamp","on":true}}
```

Rules:

- Method names and parameter/result schemas are generated projections of the
  selected canonical T8 operations used by the web UI.
- Request IDs follow JSON-RPC semantics and correlate only one connection's
  request and response.
- Litehouse domain failures use JSON-RPC's `error` object with a stable Litehouse
  code and typed safe data.
- Server notifications carry live UI invalidation/state events. They are not a
  durable event log.
- JSON-RPC batch requests are not required in V1.
- Binary messages and arbitrary HTTP tunneling are not supported.

The transport does not add its own replay, mutation journal, idempotency store,
or authorization model. Those are canonical-handler concerns when an operation
actually needs them.

## Reconnect and state

A disconnected client opens a new peer connection, obtains a fresh grant, calls
`session.open`, and fetches current state again. V1 does not migrate requests,
resume streams, replay transport notifications, or preserve transfer state across
connections. Clients never automatically replay an uncertain mutation.

Large registry artifacts use the registry or local HTTP path. They are not split
into custom WebRTC binary chunks.

## Broker and TURN boundary

The broker exchanges signaling needed to establish the direct WebRTC connection.
It does not relay routine RPC traffic, write management state, or inspect RPC
payloads.

TURN/relay fallback is deferred. If real connectivity evidence later requires
it, TURN is a separately selected operational feature rather than part of the V1
RPC protocol.

## Validation

Required tests cover:

- broker-facilitated direct connection establishment;
- rejection of RPC before `session.open`;
- valid, expired, wrong-server, wrong-owner, and stale-pair grants;
- JSON-RPC request/result/error and server-notification fixtures;
- parity between representative HTTP and WebRTC calls into the same T8 handler;
- malformed JSON-RPC and unknown-method errors without channel-wide crashes;
- disconnect followed by fresh authentication and state reload;
- no automatic replay of an uncertain mutation; and
- proof that routine management payloads are not stored or relayed by the broker.

No custom binary framing, credit windows, SCTP threshold policy, in-place
reauthorization, durable stream cursor, reconnect/resume protocol, transfer
digest, per-frame token bucket, transport result store, or mandatory TURN
infrastructure belongs to T11.
