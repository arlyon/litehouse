# Define the Home Assistant Matter Server adapter v1

Type: prd
Mode: HITL
Status: resolved
Resolved: 2026-07-16
Blocked by: 17, 30, 35, 42, 44, 54, 55, 56
Map: [Find the route to Litehouse's first connected release](../map.md)

## Problem

An operator who already commissions Matter devices through Home Assistant should
be able to expose those devices as typed Litehouse capabilities without creating
a second fabric, commissioning flow, controller, Thread network, or secret
store. Litehouse must not gain access to OTBR, the Thread radio, Bluetooth,
Matter fabric credentials, or the unrestricted Matter Server API.

The verified reference deployment already runs Home Assistant, a separate OTBR,
and external Python Matter Server 8.1.0/schema 11. Matter Server owns the fabric
and listens through an unauthenticated WebSocket API. Home Assistant commissions
through its Companion app and continues to be the only commissioning/removal
surface.

This is **Matter adapter v1**, a post-first-connected-release extension. It does
not enlarge the current connected V1 release gate.

## Goals and success

1. Already-commissioned native Matter nodes can be configured as independent
   Litehouse plugin instances.
2. Each instance refers to exactly one node on exactly one configured Matter
   Server and owns its own logical and upstream connection lifecycle.
3. Supported Matter clusters project into the nine exact-major Litehouse facets
   without generic Matter passthrough or semantic loss.
4. Home Assistant remains connected to the same Matter Server and continues to
   commission, remove, and control devices independently.
5. Litehouse remains non-root, bridged, device-free, multicast-free, and unable
   to read or persist Matter/Thread credentials.
6. Removing or rolling back Litehouse changes no Matter fabric, node, OTBR, or
   Home Assistant state.

Success is fixture-proven first, then read-only against the existing external
server, then—under separate hardware approval—one supervised absolute command
whose acknowledgement and later observation remain distinct.

## Users and use cases

- A home operator configures a Matter Server profile, obtains a Matter node ID
  through Home Assistant/operator tooling, and creates one Litehouse instance
  for that native node.
- A rule or UI binds to that instance's independently typed facets.
- An operator sees explicit incompatible-server, node-missing, hydrating,
  unavailable, stale, ambiguous-command, and unsupported-bridge states.
- Home Assistant can act concurrently; Litehouse never claims exclusive node
  ownership or transaction ordering across clients.

## Scope

### Included

- External operator-owned Python Matter Server 8.1.0 through 8.1.2, protocol
  schema 11, as one explicit compatibility profile.
- One configured Matter Server profile may be referenced by many Litehouse
  plugin instances.
- One Matter plugin instance per native, non-bridge Matter node.
- One dedicated filtered relay session and one dedicated upstream Matter Server
  WebSocket per active instance.
- Closed projections for the nine existing capability contracts where the node
  advertises the required clusters/features.
- Node-scoped hydration, event filtering, commands, reconnect, topology
  reconciliation, receipts, and failure isolation.

### Non-goals

- Commissioning, setup codes, commissioning windows, discovery, guided adoption,
  or automatic import.
- Matter bridge nodes or splitting bridged endpoint compositions into instances.
- A shared/multiplexed upstream dispatcher.
- Matter fabric, ACL, binding, credential, interview, remove-node, diagnostic,
  test-node, or OTA operations.
- OTBR/Thread dataset, radio, antenna, Bluetooth, D-Bus, mDNS, host-network, or
  hardware access from Litehouse.
- Bundling, upgrading, backing up, or administering Matter Server.
- Generic cluster/attribute/command reads, writes, templates, or passthrough.
- Automatic compatibility with Matter.js Server; it requires a separately
  validated profile even if it claims schema compatibility.

A future dispatcher/discovery decision may multiplex connections and decompose
bridge nodes while preserving one Litehouse instance per physical device. It is
not smuggled into this version.

## Behavioral requirements

### Server profiles and instance identity

1. A closed top-level Matter Server profile contains a host-generated profile ID,
   canonical absolute non-symlink relay socket path, operator-configured immutable
   non-secret relay deployment UUID, compatibility profile
   `python-matter-server-8.1-schema-11`, and activation-pinned server-info
   locators. It contains no fabric key, setup code, network credential, or Matter
   Server data path.
2. A Matter plugin instance contains a host-generated instance ID, exact Matter
   Server profile ID, operational Matter node ID, config revision, grants, and
   enabled state.
3. Relay handshake establishes the observed upstream fabric locator
   `(fabric-id, compressed-fabric-id)`. Adapter v1 permits only one active server
   profile for that locator, regardless of profile UUID, socket spelling, relay
   deployment UUID, or whether alias relays operate concurrently or sequentially.
   A collision between genuinely independent servers fails closed rather than
   weakening duplicate protection.
4. The active set enforces uniqueness of `(observed-fabric-locator, node-id)`; a
   second active claimant is rejected transactionally across all profiles.
5. The relay deployment UUID and canonical socket remain transport pins on that
   single profile, not identity namespaces. Replacing either requires one atomic
   profile-replacement transaction that first quiesces all referencing instances,
   validates the new relay reports the same expected fabric locator, updates the
   profile revision, and then reactivates those same instances. A second profile
   is never created for replacement.
6. Node IDs are meaningful only with the selected connector/fabric locators. A
   changed server schema range or fabric-ID pair fails closed and requires
   explicit profile reactivation; Litehouse claims no cryptographic fabric-
   replacement detection because schema 11 exposes no root certificate.
7. Activation rejects a missing, uncommissioned, unavailable-for-interview, or
   `is_bridge=true` node. A later bridge transition withdraws the instance as
   incompatible rather than flattening child devices.

### Dedicated connection lifecycle

1. Each active plugin instance owns one typed connection resource and one
   dedicated upstream Matter Server WebSocket. Connections are not shared or
   multiplexed in adapter v1.
2. The relay receives the global snapshot/event stream required by Matter Server
   but passes only the configured node's bounded data and events to that
   instance. Cross-node reads, events, and commands are impossible at the relay
   boundary, not merely ignored by guest code.
3. At most 32 node instances may be active per Matter Server profile in adapter
   v1. Activation beyond the cap returns typed capacity failure and identifies
   the future dispatcher as the scaling path.
4. Connection startup is staggered with at most two concurrent hydrations per
   profile. A missed/restarted connection uses bounded exponential backoff with
   jitter and never forms an unbounded reconnect storm.
5. After the unsolicited schema-11 server-info message, the instance invokes
    `start_listening`, stages only its target node from the result, and executes a
    target-specific `get_node` barrier before atomically publishing hydrated
    **attribute and topology state**. Prior state is never presented as current
    during hydration.
6. Matter Server registers the connection's event callback only after creating
    the `start_listening` snapshot. The `get_node` barrier cannot recover an
    occurrence event in that interval. Every connection generation therefore
    emits an explicit scoped event-gap marker covering initial hydration; live
    occurrence events are complete only from callback registration onward and no
    replay claim is made.
7. One reader demultiplexes responses/events for the instance. The instance
    continuously drains its upstream queue; overflow or malformed ordering drops
    the connection, marks facets unavailable, records another event gap, and
    forces full rehydration.
8. Node removal, server shutdown, socket loss, schema incompatibility, or relay
    failure affects only the corresponding instance except that one server
    outage naturally makes every profile-referencing instance unavailable.

### Capability projection

1. The relay and provider retain the full bounded endpoint graph privately but
    expose only mappings approved in issue 42: On/Off, Level, Color, Boolean
    State, Measurement, Thermostat, Window Covering, Momentary Action, and
    Notification Sink when supported.
2. Numeric endpoint/cluster/field IDs remain private locators. Feature maps,
    ranges, steps, tolerance, color model, nullability, events, direction, data
    versions, semantic tags, and composition metadata are preserved where they
    affect the portable contract.
3. Unknown, unsupported, unavailable, stale, erroneous, and false/zero remain
    distinct. An unknown attribute or unapproved cluster stays opaque inventory
    or a bounded diagnostic; it never becomes a generic operation.
4. Reads after hydration are target-node and mapped-path only. Wildcard reads,
    arbitrary writes, arbitrary command names, and unrestricted payloads are
    denied.

### Commands and receipts

1. Every mutation uses a Litehouse request ID, exact instance/facet/operation,
    bounded payload digest, and deadline. Matter Server message IDs are unique
    correlation IDs only and are never treated as idempotency keys.
2. One mutation is admitted at a time per instance for adapter v1. Home
    Assistant remains a concurrent external controller, so this provides no
    cross-client transaction or exclusion claim.
3. A successful Matter interaction response is protocol/target
    acknowledgement, not physical completion. A later attribute observation may
    establish observed completion for an absolute setter but is not fabricated
    as causally certain.
4. A disconnect, cancellation, relay crash, or deadline after possible upstream
    transmission returns `outcome-unknown`. Toggle, step, move, press/action,
    notification, and all ambiguous mutations are never replayed automatically.
    An absolute setter may be reconsidered only after fresh observation under the
    existing operation contract.
5. Reads and hydration may be retried after a new connection. No mutation is
    retried merely because the server or relay reconnects.

### Forbidden operations

1. The relay categorically denies Matter Server commissioning/discovery,
    Wi-Fi/Thread credential setters, commissioning-window, remove/interview,
    fabric/ACL/binding, OTA/update, diagnostics, test-node, and unrestricted
    read/write/invoke commands.
2. Litehouse has no Matter Server data mount, host network, multicast, OTBR API,
    Bluetooth/D-Bus, serial/USB device, controller key, Thread dataset, setup
    code, or raw credential path.

## Contracts and architecture

```text
Home Assistant Companion app ── commissions ──┐
Home Assistant client ─────────────────────────┤
                                              ▼
                              Python Matter Server schema 11
                                 loopback ws://127.0.0.1:5580
                                      ▲       ▲       ▲
                                      │       │       │ dedicated upstream WS
                               relay session A│relay session B
                                      │       │
                             typed connection A/B over protected UDS
                                      │       │
                         Matter plugin A   Matter plugin B
                           node 123          node 456
```

Matter Server is rebound to host loopback before supported use. A first-party
Rust policy relay is separately operated on the host, can connect only to the
loopback endpoint, and exposes one protected Unix-domain socket directory mounted
read-only into the Litehouse container. Socket ownership/mode is the transport
authorization boundary. The relay has no Matter storage and no persistent
credential/state volume.

The Litehouse host creates a generated, node-scoped connection resource for each
plugin instance. The guest owns reconnect/hydration behavior through that typed
resource; it receives no raw UDS, WebSocket, host-network, or generic JSON
capability. The resource operations are closed server-info, hydrate/get-target,
subscribe-target-events, mapped-read, mapped-invoke/write, and close operations.

## Data and migration

Litehouse persists only profile/instance configuration, expected public
server-info locators, host-owned facet identities, bounded typed observations,
topology evidence, and Litehouse request/receipt records. It does not persist raw
Matter Server storage or credentials.

There is no migration from another Litehouse Matter implementation. Disabling or
deleting an instance withdraws only Litehouse facets and dependencies; it never
removes the Matter node. Profile deletion rejects while instances reference it.

The relay persists no application or Matter state. Its immutable deployment UUID
is operator configuration, and it holds an exclusive runtime lock for its
loopback upstream plus observed fabric-ID pair so concurrent supported relays
cannot alias the same Matter Server. Litehouse's global one-profile-per-observed-
fabric rule additionally rejects a sequential alias with another relay UUID. A
crash with an in-flight mutation causes an ambiguous Litehouse receipt; it does
not recreate or retry the request. Matter Server backup, fabric recovery, and
version migration remain operator-owned and outside `/var/lib/litehouse`.

## Security and privacy

- Port 5580 MUST listen only on host loopback; the current unauthenticated
  all-interface exposure is not a supported boundary.
- The relay MUST expose no LAN listener. Its UDS has a dedicated owner/group and
  restrictive mode; only the intended Litehouse runtime identity receives the
  mount.
- The relay validates schema, message kind, node, endpoint, cluster, field,
  command, payload, status, frame size, and deadline before forwarding.
- Unknown or oversized input fails closed. Raw upstream error text and unknown
  payloads never enter user-visible errors or logs.
- Safe diagnostics include profile/instance/node-locator hashes, schema/SDK
  versions, correlation IDs, state phase, and typed outcome—not fabric secrets,
  raw attributes, household labels, setup/network data, or command payloads.
- Rebinding Matter Server and deploying the relay are external server mutations
  requiring a separate approved, validated, reversible operation. HA's loopback
  route must be proven before changing the listener.

## Reliability and operations

- Matter Server remains ready/unready independently of Litehouse.
- One failed plugin/relay session must not terminate other instances or HA.
- Each instance has bounded pending RPC, event handoff, frame, endpoint,
  attribute, and payload limits fixed in the generated contract and tested at
  both boundaries. Limits are correctness controls, not latency SLOs.
- Attribute/event streams have no replay cursor. Every reconnect performs full
  target hydration before current status returns and emits a scoped occurrence-
  event gap for the unobservable snapshot/callback interval.
- Server output-queue overflow, unknown schema minimum, duplicate response IDs,
  out-of-order responses, target removal, and endpoint topology changes are
  explicit test cases.
- Structured redacted logs identify connection, hydration, reconciliation,
  command, ambiguity, and denial phases. No duplicate audit database is added.

## User experience

The Matter settings surface shows configured server profiles, compatibility and
connection state, and manually entered/selected exact node IDs. It says
“Commission and remove Matter devices in Home Assistant.” It has no setup-code,
commission, Thread, OTBR, radio, fabric, or credential controls.

An instance shows node missing, unsupported bridge, hydrating, current, stale,
unavailable, incompatible server, and ambiguous command states. Technical Matter
IDs are progressive detail, not primary labels. Every workflow remains keyboard
and screen-reader operable under the existing control-surface contract.

## Acceptance criteria

1. Two instances referencing one server open two independent relay/upstream
   sessions and hydrate only their respective nodes.
2. A session cannot read, receive, or command another configured node even with a
   forged downstream request.
3. Concurrent or sequential alias profiles reporting the same observed fabric
   locator are rejected even with different relay UUIDs/socket paths, and
   duplicate active `(observed-fabric-locator, node-id)` configuration is rejected
   atomically. Relay replacement succeeds only through the one-profile atomic
   replacement transaction.
4. Bridge nodes are rejected with an explicit unsupported code.
5. Home Assistant and all Litehouse sessions listen concurrently without fabric
   or subscription mutation by Litehouse.
6. Schema-11 fixtures for Python Matter Server 8.1.0 and 8.1.2 produce identical
   supported projections; incompatible ranges fail before hydration.
7. The post-listen target `get_node` barrier reconciles current attributes and
   topology, while an occurrence event injected in the snapshot/callback gap
   produces an explicit scoped gap marker and is never claimed as replayed.
8. Socket loss immediately marks the instance unavailable; reconnection stages a
   complete target snapshot before publication.
9. Every supported facet preserves issue-42 features, nullability, range,
   quantization, events, and receipt strength.
10. Every forbidden server command and cross-node path is denied at the relay.
11. Possibly transmitted mutations become `outcome-unknown` and are not replayed.
12. Matter acknowledgement never appears as physical or observed completion.
13. The 33rd active instance for one profile fails with typed capacity error and
    does not open another Matter Server client.
14. Port 5580 is unreachable from the LAN and ordinary containers; Litehouse can
    access only the relay UDS and has no host/device/OTBR/Matter-storage access.
15. Stopping/removing Litehouse and the relay changes no node, fabric, Home
    Assistant, Matter Server data, OTBR, or radio state.

## Validation matrix

| Requirement | Stable seam | Required evidence |
|---|---|---|
| Schema compatibility | Tagged 8.1.0/8.1.2 fixture servers | Server-info, hydration, snapshot/callback event-gap, unknown-field/error, out-of-order response tests |
| Connector/target identity | Relay handshake and activation transaction | Concurrent/sequential alias rejection across profile/socket/relay UUIDs, duplicate-node rejection, and atomic relay replacement |
| Node isolation | Relay policy boundary | Cross-node snapshot/event/read/invoke denial matrix |
| Dedicated lifecycle | Two-plus-instance process fixture | Separate sockets, hydration, failure, backoff, and capacity evidence |
| Capability truth | Pure cluster projection fixtures | All nine feature/nullability/range/event/receipt cases from issue 42 |
| Ambiguity | Relay/server fault fixture | Before-write failure versus after-possible-write disconnect and zero replay |
| HA coexistence | Multi-client fixture | HA-like client plus multiple Litehouse listeners and same-node concurrency |
| Container security | Disposable Podman topology | Loopback-only 5580, protected UDS, no host network/devices/data mount |
| Existing deployment | Separately approved read-only proof | Exact schema/server-info, target hydration, no commands or config mutation |
| Controlled command | Separately approved disposable device | One absolute command, acknowledgement, and later observation kept distinct |
| Repository completion | Existing Rust workspace seams | Targeted relay/plugin/host tests, diagnostics, formatting, and repository canonical checks |

No live household command, listener change, relay deployment, hardware access, or
service restart is authorized by this PRD.

## Rollout and rollback

1. Build schema-11 fake servers and denial fixtures.
2. Implement the Rust relay and typed host resource without external access.
3. Implement one-node plugin instances and issue-42 projections.
4. Prove Podman/UDS isolation in a disposable topology.
5. Under separate server approval, verify HA's loopback route, rebind Matter
   Server to loopback, deploy the relay, and perform read-only target hydration.
6. Enable explicit instances disabled-by-default, then observation-only.
7. Under separate hardware approval, perform one supervised absolute command.
8. Mark adapter v1 supported only after all criteria pass.

Rollback disables Matter instances and removes/stops the relay while preserving
Matter Server loopback and HA's validated route. It never restores an unsafe LAN
listener, rewinds Matter Server storage, or touches OTBR. Re-enable only the prior
Litehouse image/config; external Matter state is unchanged.

## Risks and assumptions

- Python Matter Server 8.1.x is frozen and unsupported. It is an operator-owned
  compatibility target, not a Litehouse dependency.
- The schema handshake cannot prove exact package patch level or cryptographic
  fabric continuity. The profile pins observable locators and fails on change but
  documents the residual same-locator replacement risk. Adapter v1 also rejects
  genuinely independent servers that collide on both exposed fabric locators; a
  future cryptographic controller identity may remove that conservative limit.
- Dedicated connections duplicate global snapshots/events and scale linearly.
  The explicit 32-instance cap is adapter-v1 correctness scope; a future shared
  dispatcher is the migration path.
- Multiple clients can issue concurrent commands to one node. Litehouse orders
  only its own instance work and never claims cross-client exclusion.
- Existing server exposure must be remediated before supported integration, but
  that external mutation is separately approved and reversible.

## Delivery plan

1. Relay protocol, policy, bounds, and tagged fixture harness.
2. Node-scoped typed host resource and dedicated connection actor.
3. Matter plugin instance config, identity, hydration, and lifecycle.
4. Nine-facet projection and command/receipt mapping.
5. Management/API/UI profile and explicit node-instance configuration.
6. Container socket topology and operational documentation.
7. Read-only existing-server proof, then separately approved command proof.

Definition of done is all acceptance criteria, validation evidence, map/handoff
updates, and no unresolved security or compatibility blocker. Implementation
requires its own approved tranche.
