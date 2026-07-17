# Research the Home Assistant Matter Server adapter boundary

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-16
Blocked by: 30, 35, 42, 44, 54, 55
Map: [Find the route to Litehouse's first connected release](../map.md)
Related: [Matter controller and fabric ownership](./55-research-matter-controller-and-fabric-ownership.md)

## Recommendation

Support an **external, operator-owned Python Matter Server 8.1.x compatibility
adapter**, not a bundled controller and not a second fabric. Litehouse should be
one additional WebSocket API client of the already-running server. The server
continues to own its CA, fabric administrator, operational controller key and
certificates, CASE sessions, node IDs, interviews, cache, and Matter
subscriptions. Litehouse must not mount `/data`, see `chip.json`, accept setup
codes or network credentials, commission/remove/interview nodes, manage fabrics,
or control OTBR.

The supplied live evidence is compatible with this boundary: Python Matter
Server 8.1.0 is already running with host networking and no Bluetooth adapter;
Home Assistant 2026.2.2 and a separate OTBR are already its operators. However,
the present unauthenticated `0.0.0.0/[::]:5580` exposure is **not an acceptable
Litehouse connection boundary**. Rebind the Matter Server to host loopback and
put a separately managed, bounded transport in front of it:

1. **Preferred:** a host-side WebSocket policy relay exposed as a Unix-domain
   socket whose directory is bind-mounted only into the unprivileged Litehouse
   container. Socket ownership/mode and the mount are the authorization boundary;
   the relay connects upstream to `ws://127.0.0.1:5580/ws`.
2. **If an IP transport is operationally necessary:** expose the relay only on
   the dedicated container-bridge gateway, require mTLS (preferred) or TLS plus a
   per-instance bearer credential, and firewall it to the Litehouse container's
   bridge identity/address. Never publish it on the LAN.
3. The relay should parse commands, allocate its own upstream `message_id`, and
   enforce the adapter allowlist below. A generic authenticated reverse proxy is
   better than today's exposure, but it still gives a compromised Litehouse
   process the server's full fabric authority; application-layer command/path
   filtering is the stronger boundary.

Home Assistant can continue to use loopback directly if it shares the host
network namespace; otherwise give HA a separate operator-only relay route. Verify
that route before changing the bind. Do not solve bridge reachability by leaving
port 5580 open on all host interfaces.

Python Matter Server is an **existing-operator compatibility target only**. Its
current first-party README says 8.1.2 is the final Python release and is no longer
updated or supported; the repository is archived. The active successor is
`matterjs-server`, which currently describes itself as beta and API-compatible
with 8.1.2.[LIFE-PY][LIFE-JS] This warrants an adapter boundary and contract tests,
not adding Python Matter Server to Litehouse's dependency or deployment graph.

## Exact integration boundary

### Commissioning and Thread: Home Assistant remains the only path

Official HA documentation says the Companion app is used for commissioning: the
phone is brought near the device, sends Wi-Fi or Thread network credentials over
BLE, and control subsequently uses Wi-Fi/Thread. It explicitly says HA does not
use the server host's Bluetooth adapter for its normal flow.[HA-MATTER-DOC] For
Thread, credentials must be synchronized to the phone; current HA docs also warn
that Companion commissioning uses the **mobile device's preferred Thread
network**, not necessarily HA's preferred network.[HA-THREAD-DOC][HA-THREAD-PREFERRED]

The source matches that two-stage design. HA's authenticated/admin WebSocket API
passes the setup code to `commission_with_code` with `network_only` defaulting to
`true`.[HA-API] Python Matter Server rejects non-network-only commissioning when
it was not started with `--bluetooth-adapter`; its on-network path then allocates
the node ID, commissions into the server's fabric, interviews, and starts the
node subscription.[PMS-CONTROLLER] With the supplied command line,
`bluetooth_enabled` is therefore false. The separate OTBR only routes Thread
IPv6; it neither owns this Matter fabric nor changes the Litehouse boundary.

Consequently Litehouse exposes **none** of:

- `commission_with_code`, `commission_on_network`, `discover`;
- `set_wifi_credentials`, `set_thread_dataset`;
- `open_commissioning_window`, `remove_node`, `interview_node`;
- `set_default_fabric_label`, ACL/binding/fabric management, OTA, diagnostics, or
  test-node import.

It consumes already-commissioned nodes only. Adding/removing/sharing a device
continues through HA/operator workflows.

### One server fabric, many API clients

Multiple API clients are an intentional server architecture: the development
documentation says multiple consumers can communicate with the same Matter
fabric while the server keeps the fabric running independently.[PMS-DEV] The
implementation creates one `WebsocketClientHandler` per connection and tracks a
set of clients for shutdown.[PMS-SERVER] Each connection may call
`start_listening` once and gets its own copy of every server event.[PMS-HANDLER]
Thus HA and Litehouse can remain connected concurrently.

These are **not** separate Matter controllers or fabrics. `MatterStack` loads the
CA state from `<storage>/chip.json`, selects/creates one fabric administrator, and
creates the controller from it.[PMS-STACK] All WebSocket clients therefore share
that controller's device ACL authority and global node/cache state. Node cache
metadata is stored by compressed fabric ID in a separate JSON file; the API
clients do not need either file.[PMS-STORAGE] Wi-Fi and Thread provisioning
setters update the controller and expose only `*_credentials_set` booleans in
server info, never secret values; those flags initialize false with each 8.1.x
process. Litehouse must neither call the setters nor infer durable network-secret
state from the booleans.[PMS-CONTROLLER]

### Handshake and version contract

Transport is plain JSON text messages over `GET /ws`; there is no WebSocket
subprotocol negotiation. Immediately after upgrade, the server sends an
**unsolicited** `ServerInfoMessage`:

```text
fabric_id, compressed_fabric_id,
schema_version, min_supported_schema_version, sdk_version,
wifi_credentials_set, thread_credentials_set, bluetooth_enabled
```

There is no Python Matter Server package version in the handshake. The reference
client identifies this message by the presence of `sdk_version`, then accepts its
own schema `C` only when `server.schema_version >= C` and
`server.min_supported_schema_version <= C`.[PMS-MODELS][PMS-CONNECTION]

For 8.1.0 and 8.1.2, `schema_version=11` and
`min_supported_schema_version=9`. HA 2026.2.2 pins the 8.1.2 client library, which
also speaks schema 11.[HA-MANIFEST][PMS-CONST] Litehouse should advertise
internally that it implements **client schema 11**, apply the same range check,
and ignore unknown additive fields/events. Record the fabric ID pair and SDK
version for diagnostics, but do not treat either integer alone as a portable
fabric identity.

The 8.1.0 and 8.1.2 client, message, schema, and error protocol files are
byte-identical in the tagged first-party source. 8.1.2's release changes are
custom-cluster/dashboard fixes and maintenance; 8.1.1 adds fabric-label
truncation and minor discovery behavior.[REL-810][REL-811][REL-812] Therefore the
live 8.1.0 server is wire-compatible with HA's 8.1.2 client for this narrow
profile, but it is not HA's exactly pinned/supported server release and cannot be
distinguished through the handshake. Treat that as an explicit operator-owned
compatibility risk.

### Node, endpoint, state, event, read, and invoke API

The minimum profile is:

| Need | Wire command/event | Adapter behavior |
|---|---|---|
| Initial hydration + push stream | `start_listening` | Required once per connection. Its correlated success result is the full `MatterNodeData[]`; all later server events arrive on the same socket. |
| Reconciliation | `get_nodes`; optionally `get_node` | Allowed read-only. Run `get_nodes` once after listener activation to close the initial snapshot/subscriber-registration race and after detected cache inconsistency. |
| Point read | `read_attribute` (schema 9+) | Allowed only for mapped node/path allowlists. Path is `endpoint/cluster/attribute`; `*` is supported by the server, but the Litehouse boundary should reject broad wildcards after hydration. |
| Invoke | `device_command` | Allowed only for closed capability mappings: exact node, endpoint, cluster ID, command name, payload schema, and timeouts. Never provide generic invoke passthrough. |
| Attribute write | `write_attribute` | Deny by default; admit only an exact mapped writable attribute when a capability cannot use a command. |
| Updates | `attribute_updated` | Data is `[node_id, attribute_path, new_value]`; replace that cached value. |
| Matter events | `node_event` | Data includes node/endpoint/cluster/event IDs, event number, priority, timestamp/type, and payload. Preserve source identity and ordering metadata. |
| Topology/lifecycle | `node_added`, `node_updated`, `node_removed`, `endpoint_added`, `endpoint_removed` | Rebuild/reconcile the affected node or endpoint composition. `server_shutdown` and socket loss make all projected facets unavailable. |

The complete command/event enums and message shapes are first-party dataclasses.[PMS-MODELS]
`MatterNodeData` contains node ID, commissioning/interview timestamps and version,
availability, bridge flag, a raw attribute map, and legacy subscription metadata.
There is no separate endpoint-list RPC: endpoints are derived from attribute keys,
and device composition comes from Descriptor cluster attributes. The reference
client groups attributes by the numeric endpoint prefix and builds endpoint and
cluster objects from those values.[PMS-NODE]

The server, not either API client, owns the actual Matter subscription. It creates
one wildcard attribute and event subscription per node with automatic
resubscription, updates its cache, persists changed attributes, and fans out
`attribute_updated`/`node_event` to every listening API client.[PMS-SUBSCRIBE]
`subscribe_events` in the Python client is only an in-process filter over that
single all-events WebSocket stream; there is no per-client Matter subscription
RPC.[PMS-CLIENT]

For a non-Python Litehouse adapter, keep unknown attribute values as opaque JSON
and implement only the closed cluster/command schemas needed by Litehouse's
published facets. Wire bytes are base64 strings and SDK enums/structs are JSON
numbers/objects; the protocol does not carry a self-describing type schema.[PMS-JSON]
Do not import or bundle the Python client merely to decode the stream.

## RPC, acknowledgement, concurrency, and recovery

### Request IDs and errors

A command is:

```json
{"message_id":"unique-string","command":"read_attribute","args":{}}
```

A terminal response repeats `message_id` and contains either `result` or
`error_code` plus optional `details`. Codes 0–11 map to generic/unknown,
commission, interview, not-ready, not-resolving, nonexistent-node, version,
SDK-stack, invalid-arguments, invalid-command, update-check, and update errors.[PMS-ERRORS]
Use cryptographically random, never-concurrently-reused IDs, bound pending
requests, validate the response type, map unknown future codes to a generic
remote error, and set an adapter-side deadline: the 8.1 client has no general RPC
timeout.[PMS-CLIENT]

The server launches every non-listen command in its own asyncio task, so replies
may arrive out of request order. Matter reads/writes/invokes are serialized by a
per-node SDK lock, while different nodes can progress concurrently.[PMS-HANDLER][PMS-SDK]
Litehouse should still use one bounded queue per node and a small global
in-flight cap so it neither races HA unnecessarily nor overloads Thread. There
is no cross-client transaction or ownership arbitration.

### Acknowledgement and idempotency

A success result means the server handler and underlying SDK call returned
success; for an invoke it is the Matter interaction response, **not** proof of
physical completion or later stable state. Confirm absolute state changes from a
subsequent attribute report/read where the capability requires it.

`message_id` is correlation only. The server has no duplicate-request cache,
idempotency key, or receipt journal; reusing an ID executes another command. If
the socket closes after transmission but before the response, outcome is
unknown. Therefore:

- retry handshake, hydration, `get_*`, and safe reads;
- never automatically retry toggle/step/move/press or another non-idempotent
  invoke after timeout/disconnect;
- for an absolute setter, reconcile by observed state before deciding whether a
  new invoke is safe; and
- retain Litehouse's higher-level request/receipt deduplication, while reporting
  the crash/transport ambiguity honestly.

### Reconnect and hydration

The Python client does not reconnect itself. On disconnect it cancels pending
futures; HA responds by reloading the integration.[PMS-CLIENT][HA-SETUP] A native
Litehouse adapter should instead use bounded exponential backoff with jitter and,
on every new socket:

1. mark all prior nodes/facets unavailable and fail pending mutations as
   `outcome-unknown` where appropriate;
2. receive and validate the unsolicited server-info message;
3. send no other request until the correlated `start_listening` snapshot is
   accepted and staged;
4. start the sole reader/demultiplexer, issue `get_nodes`, reconcile atomically,
   then publish hydrated state; and
5. process events and command results through that one reader until loss.

There is no stream cursor or replay protocol. `start_listening` snapshots nodes,
enqueues the result, and only then registers that connection's event callback,
leaving a narrow initial race; the post-listen `get_nodes` barrier is therefore
intentional.[PMS-HANDLER] Matter node events carry an event number, but ordinary
attribute/topology updates have no global sequence. A slow client also has a
512-message output queue and is disconnected when it fills, so the adapter must
consume continuously and hand bounded work to downstream actors.[PMS-HANDLER]

## Security assessment and acceptance profile

Python Matter Server 8.1.x has no API authentication, authorization, TLS, Origin
check, or per-client command policy in its WebSocket server. It mounts `GET /ws`
and `GET /info` directly on a bare `aiohttp.web.Application`; the CLI defaults
`--listen-address` to all IPv4/IPv6 addresses.[PMS-SERVER][PMS-MAIN] Anyone who
can reach port 5580 can use the same fabric controller authority as HA, including
credential-setting, generic commands, fabric/ACL changes, node removal, and
commissioning operations. Host-network placement does not make that safe.

Minimum acceptance criteria for the compatibility adapter:

- Matter Server listens only on host loopback; no LAN or container-bridge listener
  on 5580.
- Litehouse receives only the policy relay socket/URL, never the Matter Server
  data directory, host network, multicast, OTBR API, Bluetooth/D-Bus, or hardware.
- The relay authenticates/bounds the caller and enforces the exact RPC and
  node/endpoint/cluster/command/path allowlists above.
- The adapter implements schema-11 handshake tests against tagged 8.1.0 and
  8.1.2 fixtures; unknown fields/events/error codes do not crash it.
- Contract tests cover out-of-order responses, duplicate IDs, 512-message
  backpressure/loss, hydration reconciliation, endpoint add/remove, malformed or
  oversized JSON, reconnect, and ambiguous in-flight invokes.
- No commissioning, fabric, credential, remove, interview, OTA, diagnostics,
  wildcard post-hydration read, arbitrary command name, or arbitrary attribute
  write can cross the relay.
- HA and Litehouse simultaneous listeners are exercised, including concurrent
  same-node activity and server restart, without changing HA's fabric ownership.

## Compatibility risks

1. **Lifecycle:** 8.1.2 is final and unsupported. Keep this as an external
   compatibility target and plan a separately tested schema-compatible
   `matterjs-server` route; do not silently migrate operator storage.
2. **Supported-version mismatch:** HA 2026.2.2 pins client 8.1.2 while the live
   server is 8.1.0. The narrow wire contract matches, but the handshake cannot
   prove the package version and 8.1.0 misses later fixes.
3. **Shared authority:** multiple API clients share one controller with no
   authorization or arbitration. Authentication alone does not provide least
   privilege; command-aware filtering is required.
4. **No replay/idempotency:** disconnects can lose updates and make invokes
   ambiguous. Rehydrate and reconcile; do not blind-retry mutations.
5. **Dynamic JSON/data model:** command payload names and typed attribute values
   are coupled to the server's Matter SDK (`2025.7.0` in 8.1.0). Keep a closed
   mapping and opaque fallback rather than claiming arbitrary-cluster support.
6. **Thread/network timing:** the server serializes per node but multiple clients
   and nodes remain concurrent. Bound work, especially for sleepy Thread nodes.
7. **Exposure during transition:** adding an authenticated proxy while leaving
   `0.0.0.0:5580` reachable does not fix the vulnerability; loopback rebinding or
   an equivalent host firewall denial is part of the change.

## Exact primary sources

All source was inspected at immutable tags/commits; no credentials, hardware, or
live API were accessed.

- **Python Matter Server 8.1.0**, commit
  [`606cfee`](https://github.com/matter-js/python-matter-server/tree/606cfee7cabdcceb3a91ec1bce058fb4cb2eb2f7):
  [wire models and enums][PMS-MODELS], [client connection/version check][PMS-CONNECTION],
  [client hydration/RPC/event handling][PMS-CLIENT], [node/endpoint projection][PMS-NODE],
  [errors][PMS-ERRORS], [JSON encoding][PMS-JSON], [WebSocket handler][PMS-HANDLER],
  [server/routes/multi-client handling][PMS-SERVER], [stack/fabric storage][PMS-STACK],
  [node controller/read/invoke/subscription][PMS-CONTROLLER], [SDK locks][PMS-SDK],
  [storage][PMS-STORAGE], [CLI bind default][PMS-MAIN], [protocol documentation][PMS-WS-DOC],
  and [multi-consumer design][PMS-DEV].
- **Python Matter Server 8.1.2**, commit
  [`8dd49d5`](https://github.com/matter-js/python-matter-server/tree/8dd49d5e5b0767203602a5dd8eae6a00ad2dc730),
  plus first-party releases [8.1.0][REL-810], [8.1.1][REL-811], and [8.1.2][REL-812].
- **Home Assistant Core 2026.2.2**, commit
  [`3e6bc29`](https://github.com/home-assistant/core/tree/3e6bc29a6a3e24ae687e8705cad8d9e82a034e79):
  [Matter dependency manifest][HA-MANIFEST], [Matter setup/reload path][HA-SETUP],
  and [commissioning API][HA-API].
- **Official Home Assistant documentation**, commit
  [`6b247f8`](https://github.com/home-assistant/home-assistant.io/tree/6b247f85f4f9dff375e4093ee5a4b64d5d9c6e58):
  [Matter commissioning/BLE documentation][HA-MATTER-DOC],
  [Thread credential synchronization][HA-THREAD-DOC], and
  [mobile preferred-network behavior][HA-THREAD-PREFERRED].
- **Lifecycle only:** archived Python repository current commit
  [`4c820ed`][LIFE-PY] and active successor current commit
  [`f3e05e8`][LIFE-JS].

[PMS-MODELS]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/common/models.py#L14-L180
[PMS-CONNECTION]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/client/connection.py#L60-L108
[PMS-CLIENT]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/client/client.py#L445-L789
[PMS-NODE]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/client/models/node.py#L45-L395
[PMS-ERRORS]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/common/errors.py
[PMS-JSON]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/common/helpers/json.py
[PMS-HANDLER]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/server/client_handler.py#L38-L255
[PMS-SERVER]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/server/server.py#L75-L264
[PMS-STACK]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/server/stack.py#L90-L154
[PMS-CONTROLLER]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/server/device_controller.py#L145-L791
[PMS-SUBSCRIBE]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/server/device_controller.py#L1146-L1377
[PMS-SDK]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/server/sdk.py#L54-L365
[PMS-STORAGE]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/server/storage.py
[PMS-MAIN]: https://github.com/matter-js/python-matter-server/blob/8.1.0/matter_server/server/__main__.py#L23-L75
[PMS-CONST]: https://github.com/matter-js/python-matter-server/blob/8.1.2/matter_server/common/const.py
[PMS-WS-DOC]: https://github.com/matter-js/python-matter-server/blob/8.1.0/docs/websockets_api.md
[PMS-DEV]: https://github.com/matter-js/python-matter-server/blob/8.1.0/DEVELOPMENT.md#L20-L38
[HA-MANIFEST]: https://github.com/home-assistant/core/blob/2026.2.2/homeassistant/components/matter/manifest.json
[HA-SETUP]: https://github.com/home-assistant/core/blob/2026.2.2/homeassistant/components/matter/__init__.py#L77-L199
[HA-API]: https://github.com/home-assistant/core/blob/2026.2.2/homeassistant/components/matter/api.py#L119-L146
[HA-MATTER-DOC]: https://github.com/home-assistant/home-assistant.io/blob/6b247f85f4f9dff375e4093ee5a4b64d5d9c6e58/source/_integrations/matter.markdown#L96-L106
[HA-THREAD-DOC]: https://github.com/home-assistant/home-assistant.io/blob/6b247f85f4f9dff375e4093ee5a4b64d5d9c6e58/source/_integrations/thread.markdown#L159-L175
[HA-THREAD-PREFERRED]: https://github.com/home-assistant/home-assistant.io/blob/6b247f85f4f9dff375e4093ee5a4b64d5d9c6e58/source/_integrations/thread.markdown#L309-L315
[REL-810]: https://github.com/matter-js/python-matter-server/releases/tag/8.1.0
[REL-811]: https://github.com/matter-js/python-matter-server/releases/tag/8.1.1
[REL-812]: https://github.com/matter-js/python-matter-server/releases/tag/8.1.2
[LIFE-PY]: https://github.com/matter-js/python-matter-server/blob/4c820ed12bac349dda372031e9757c67b9fd9048/README.md#L10-L15
[LIFE-JS]: https://github.com/matter-js/matterjs-server/blob/f3e05e89cd5358f262f269f133ad5da88eabfb96/README.md#L4-L25
