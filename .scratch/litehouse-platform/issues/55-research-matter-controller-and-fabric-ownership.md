# Research Matter controller and fabric ownership

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-16
Blocked by: 17, 30, 35, 42, 44
Map: [Find the route to Litehouse's first connected release](../map.md)

## Decision

Use a **separate, long-running Matter controller sidecar built on
`connectedhomeip` v1.5.1.0**, with one logical Litehouse plugin instance per
physical device (or per bridged-device endpoint composition), not one plugin
instance for the whole fabric.

The sidecar is the sole owner of the Matter fabric, dynamic operational
credentials, node-ID allocator, CASE sessions, mDNS, BLE/BTP, commissioning,
node caches, and subscriptions. Litehouse receives only a mutually authenticated,
non-secret API view of configured physical targets and their facets. The main
Litehouse container keeps its current no-device/no-multicast/no-host-network and
no-secret-persistence boundaries.

This requires a new **external controller-service contract**, not a relaxation
that lets ordinary Wasm plugins browse the LAN. If commissioning must appear in
Litehouse's own UI, [issue 44](./44-define-plugin-driven-instance-discovery.md)
must be narrowly revised to permit a first-party, sidecar-owned commissioning
transaction; otherwise commission through the sidecar's operator surface and
create each Litehouse instance explicitly afterward.

## Evidence baseline

- Matter behavior is based on released `connectedhomeip` `v1.5.1.0`, commit
  [`abcc720b`](https://github.com/project-chip/connectedhomeip/tree/abcc720b48c5e59c0edcfe65c516f76ca9448aa3).
  Public specification cross-checks use the CSA's official public
  [Matter 1.2 Core Specification](https://csa-iot.org/wp-content/uploads/2023/10/Matter-1.2-Core-Specification.pdf);
  where versions differ, the 1.5.1 source is authoritative for implementation.
- Current first-party Rust evidence is `project-chip/matter-rs` commit
  [`6b06fc5`](https://github.com/project-chip/matter-rs/tree/6b06fc505ced48fd3001a824de6ffb22b807ff9b),
  inspected 2026-07-16; its first release is `0.2.0` and its README still calls it
  a toolkit with unstable APIs
  ([README](https://github.com/project-chip/matter-rs/blob/6b06fc505ced48fd3001a824de6ffb22b807ff9b/README.md),
  [changelog](https://github.com/project-chip/matter-rs/blob/6b06fc505ced48fd3001a824de6ffb22b807ff9b/CHANGELOG.md)).
- The current OHF Matter Server is `matterjs-server` commit
  [`f3e05e8`](https://github.com/matter-js/matterjs-server/tree/f3e05e89cd5358f262f269f133ad5da88eabfb96).
  Its own README says it is beta, not yet re-certified, and implements Matter
  1.4.2; the previously certified Python server is frozen at 8.1.2 and no longer
  supported
  ([current README](https://github.com/matter-js/matterjs-server/blob/f3e05e89cd5358f262f269f133ad5da88eabfb96/README.md),
  [Python-server README](https://github.com/matter-js/python-matter-server/blob/4c820ed12bac349dda372031e9757c67b9fd9048/README.md)).

## Protocol obligations

### Commissioning, discovery, and Thread

The CSA flow is not merely “pair then connect”: discover over BLE/Wi-Fi/Ethernet,
establish PASE, arm the fail-safe, attest the device, obtain a CSR, install the
root/NOC/IPK and administrator ACL, provision the operational network, discover
the operational identity, establish CASE, and invoke `CommissioningComplete`
([CSA Core §5.5](https://csa-iot.org/wp-content/uploads/2023/10/Matter-1.2-Core-Specification.pdf)).
The 1.5.1 controller API implements both BLE and on-network commissioning and
accepts an explicit Thread Operational Dataset
([Python controller API](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/controller/python/matter/ChipDeviceCtrl.py),
[`CommissioningParameters`](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/controller/CommissioningDelegate.h)).
The auto-commissioner sends the dataset in `AddOrUpdateThreadNetwork`, extracts
its Extended PAN ID for `ConnectNetwork`, then completes over CASE
([controller source](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/controller/CHIPDeviceController.cpp)).

Therefore:

- The sidecar needs direct LAN IPv6 and multicast/mDNS access. It browses
  commissionable `_matterc._udp` records and resolves operational
  `_matter._tcp` records; Litehouse never receives a multicast grant.
- BLE commissioning belongs only in the sidecar, through its own BlueZ/D-Bus or
  reviewed BLE proxy. On-network commissioning remains available when a device
  is already on Ethernet/Wi-Fi/Thread.
- A Thread device still requires an existing Thread Border Router and routable
  IPv6 path. The operator supplies the complete Active Operational Dataset to a
  one-shot commissioning request. Do not expose the dataset to a plugin or store
  it in Litehouse.
- A PASE setup code is commissioning input and its nonces are transaction state;
  neither is a steady-state controller credential. Normal control is
  CASE-authenticated after commissioning.

### Fabric, ACL, and credentials

`AddNOC` installs an initial fabric-scoped CASE `Administer` ACL; later reads,
writes, invokes, and subscriptions are checked against the target's fabric ACL
([1.5.1 access-control guide](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/docs/guides/access-control-guide.md)).
The controller must retain its operational private key, NOC/ICAC/RCAC chain,
IPK, fabric metadata, and CA/NOC-signing state across restart. The 1.5.1 factory
makes storage, operational keystore, operational certificate store, and session
resumption storage explicit dependencies
([factory](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/controller/CHIPDeviceControllerFactory.h),
[fabric table](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/credentials/FabricTable.h)).
Its example plaintext operational keystore explicitly warns that raw keys may be
copied and is only a baseline, while `FabricTable` supports externally owned
keypairs
([keystore](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/crypto/PersistentStorageOperationalKeystore.h)).

**Ownership consequence:** [issue 35](./35-define-secret-lifecycle-and-remediation.md)
cannot supply these generated, long-lived values as startup-only environment
variables. The external controller sidecar must own a dedicated secret-bearing
store or an HSM/OS-keystore-backed `OperationalKeystore`; its backup, encryption,
rotation, and access control are operator responsibilities. Litehouse stores only
the sidecar's public fabric identity digest, Matter node/endpoint locators, facet
catalog, and receipts. The sidecar's data directory is not mounted into or backed
up with `/var/lib/litehouse`.

Prefer an offline/HSM RCAC, a runtime ICAC signer, and a non-exportable controller
operational key. A simpler software-keystore deployment is possible but must be
documented as secret-bearing. Wi-Fi credentials and Thread datasets are separate
operator secrets: fetch or inject them just in time and erase them after
commissioning rather than treating them as reusable fabric state.

### Subscriptions and recovery

A cluster data version is per cluster instance, randomly initialized, incremented
on attribute changes, and wraps; it is an optimization hint, not a durable global
revision ([CSA Core §7.10.3](https://csa-iot.org/wp-content/uploads/2023/10/Matter-1.2-Core-Specification.pdf)).
Data-version filters affect subscription priming only; later reports carry current
cluster data versions ([CSA Core §§8.5.3.4, 10.6.4](https://csa-iot.org/wp-content/uploads/2023/10/Matter-1.2-Core-Specification.pdf)).
The 1.5.1 `ReadClient` can establish CASE itself and auto-resubscribe with backoff;
`ClusterStateCache` commits versions only after coherent reports
([`ReadClient`](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/ReadClient.h),
[`ClusterStateCache`](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/ClusterStateCache.h)).

Implement one bounded wildcard subscription/cache per Matter **node** in the
sidecar and fan out filtered updates to physical-device Litehouse instances.
On restart or loss:

1. publish the persisted catalog as unavailable, never as current;
2. operationally resolve the node, establish CASE, and subscribe using only the
   last coherently committed cluster versions/event number as priming filters;
3. mark facets hydrated only after the complete priming report commits;
4. re-interview Descriptor/Basic Information on first attach, device restart,
   topology/configuration-version change, data-version inconsistency, or an
   incomplete priming report; and
5. persist no subscription ID as identity. A replacement subscription receives a
   new protocol ID while the Litehouse instance/facet identities remain stable.

### Node, endpoint, and physical identity

A Matter Node ID is unique only within a fabric; CASE proves the tuple of fabric
and operational Node ID. Local fabric indexes can differ between peers, so they
must never be used as durable identity ([CSA Core §§2.5.5.1, 4.10.2](https://csa-iot.org/wp-content/uploads/2023/10/Matter-1.2-Core-Specification.pdf)).
Use a full digest of `(RCAC public key, Fabric ID)` as the sidecar's public fabric
identity and `(fabric identity digest, operational Node ID)` as authenticated
node identity.

One Matter node is not necessarily one physical device: a bridge composes
multiple bridged devices from endpoint subtrees. Descriptor exposes device types,
server/client cluster lists, parts, and semantic tags
([1.5.1 Descriptor model](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/descriptor-cluster.xml)).
Accordingly:

- a native single-device node gets one Litehouse instance scoped to its device
  composition;
- a bridge gets one Litehouse instance per bridged-device composition root, all
  sharing the sidecar's node subscription/session; and
- a facet locator is `(node ID, device-root endpoint, endpoint, cluster,
  direction)`, but the Litehouse facet ID remains host-owned.

Basic Information `UniqueID` may be used as additional duplicate evidence but is
manufacturer-defined and changes on factory reset; the 1.5.1
`EndpointUniqueID` is optional/provisional
([Basic Information model](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/basic-information-cluster.xml),
[Descriptor model](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/zap-templates/zcl/data-model/chip/descriptor-cluster.xml)).
Thus endpoint numbers are locators, not sufficient physical identity. Preserve a
facet only when the authenticated node and exact composition/descriptor evidence
still match; otherwise remove/add and require operator reconciliation rather than
silently retargeting.

### Commands, idempotency, and ambiguity

MRP retransmits one reliable message and the receiver rejects duplicate message
counters, but a new Invoke after an application timeout is a new interaction
([CSA Core §4.11.2](https://csa-iot.org/wp-content/uploads/2023/10/Matter-1.2-Core-Specification.pdf),
[1.5.1 duplicate window](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/transport/PeerMessageCounter.h)).
`InvokeRequestMessage` has no general idempotency key; command references only
correlate batched requests with responses
([invoke schema](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/MessageDef/InvokeRequestMessage.h),
[`CommandSender`](https://github.com/project-chip/connectedhomeip/blob/abcc720b48c5e59c0edcfe65c516f76ca9448aa3/src/app/CommandSender.h)).

The sidecar API therefore carries Litehouse `request_id`, exact target/path,
payload digest, and deadline. Keep a bounded durable receipt journal: same ID and
payload returns the prior terminal receipt; same ID with another payload rejects.
This deduplicates Litehouse-to-sidecar retries, not a crash after the device acted
but before the receipt committed. Such a window returns `outcome-unknown`.
Never automatically retry toggle, step, move, press/action, or notification.
Absolute setters may be reconciled by a read-back and reported
`observed-completed`; a Matter success response means protocol/target
acknowledgement, not universal physical completion. This matches the receipt
strength already chosen in [issue 42](./42-research-matter-capability-mapping.md).

## Mapping into the nine facets

The sidecar retains the complete endpoint graph and raw feature/data-version
metadata; each physical-device plugin instance publishes only its scoped facets:

| Litehouse facet | Matter projection |
|---|---|
| On/Off | On/Off attributes plus absolute On/Off; Toggle remains explicitly non-idempotent. |
| Level | Level Control current level and move-to/step/move/stop, preserving transition, quantization, and On/Off coupling. |
| Color | Color Control HS, xy, and color-temperature models without lossy implicit RGB conversion. |
| Boolean state | Boolean State value/events; unknown/unavailable are not false. |
| Numeric measurement | Quantity-specific measurement clusters with source range, scale, tolerance, nullability, and representation retained. |
| Thermostat | Thermostat mode, running state, heat/cool setpoints, bounds/steps, and supported optional features. |
| Window covering | Window Covering lift/tilt axes, target/current/movement, open/close/stop, and safety/fault state. |
| Momentary action | Switch/gesture event sources and command sinks, separate from Boolean state; press/action is non-idempotent. |
| Notification sink | Messages command subset only when the endpoint supports it; preserve expiry/priority and strongest available receipt. |

Repeated facets are grouped by the Matter device composition root and semantic
tags, never by labels alone. Unsupported, nullable/unknown, unavailable, and
error remain distinct. Numeric Matter IDs stay private locators rather than
becoming portable contract identity, as established by
[issue 42](./42-research-matter-capability-mapping.md).

## OTA exclusion

OTA is out of scope. Do not expose OTA Requestor/Provider clusters, BDX transfer,
DCL update lookup, image import, or update commands through the sidecar API or any
facet. Device `SoftwareVersion` remains ordinary observed metadata. Build the
controller without an OTA provider and add a conformance test proving OTA paths
are rejected. If Matter Server is used for a prototype, require
`DISABLE_OTA=true`; its first-party Docker contract exposes that switch
([Matter.js Docker docs](https://github.com/matter-js/matterjs-server/blob/f3e05e89cd5358f262f269f133ad5da88eabfb96/docs/docker.md)).
Device-attestation certificate/DCL validation during commissioning is not OTA and
must remain enabled.

## Design comparison

| Design | Benefits | Required boundary changes / risks | Verdict |
|---|---|---|---|
| Litehouse-native controller | One process and direct typed integration; can use 1.5.1 C++ SDK. | Main container needs multicast/IPv6 interface access and BLE/D-Bus or a proxy, contradicting [issue 30](./30-define-container-and-butane-contract.md). Dynamic fabric keys contradict [issue 35](./35-define-secret-lifecycle-and-remediation.md). A native subsystem also bypasses the ordinary Wasm instance isolation model. | Reject unless issues 30 and 35 are deliberately reopened. |
| Separate controller sidecar | Correct place for host networking, BLE, shared fabric/session/cache, and generated credentials; Litehouse remains unprivileged and secretless; one logical instance per physical target remains possible. | Adds a secure local API, separate secret-bearing backup/upgrade lifecycle, and correlated failure domain for all Matter nodes. Integrated commissioning narrowly reopens issue 44. | **Recommended.** |
| One multi-node plugin instance | Minimal instance count and naturally shares one controller. | Directly violates [issues 17](./17-define-provider-execution-and-esphome-lifecycle.md) and [44](./44-define-plugin-driven-instance-discovery.md): one actor/store would own a device collection, one trap/update would withdraw every Matter device, and bindings could not be retired per physical device. It still cannot obtain BLE/mDNS or persist fabric keys under issues 30/35. | Reject. |

The recommended shape slightly clarifies, but does not reverse, issue 17: a
Matter instance owns one **logical physical-target lifecycle**, while the
external sidecar shares protocol sessions/subscriptions where Matter requires
node-level aggregation. No Litehouse instance owns a nested device collection.

## Library and service alternatives

1. **`connectedhomeip` 1.5.1 sidecar — recommendation.** It has the complete
   commissioning state machine, BLE/on-network APIs, Thread dataset handoff,
   operational discovery, credential-store abstractions, CASE, ACL support, and
   mature auto-resubscription/cache machinery cited above. Wrap a deliberately
   small API rather than exposing CHIP objects.
2. **`matter-rs` native sidecar — future Rust alternative.** Current head now has
   PASE/CASE initiators, mDNS resolve/browse, controller-mode BTP, IM read/write/
   invoke/subscribe establishment, fabric persistence, and NOC generation
   ([PASE](https://github.com/project-chip/matter-rs/blob/6b06fc505ced48fd3001a824de6ffb22b807ff9b/rs-matter/src/sc/pase/initiator.rs),
   [exchange initiation](https://github.com/project-chip/matter-rs/blob/6b06fc505ced48fd3001a824de6ffb22b807ff9b/rs-matter/src/transport/exchange.rs),
   [commissioner](https://github.com/project-chip/matter-rs/blob/6b06fc505ced48fd3001a824de6ffb22b807ff9b/rs-matter/src/onboard.rs)).
   It is not implementation-ready here: device attestation validation is
   explicitly deferred, the commissioner does not provision Wi-Fi/Thread, and
   IM subscribe supports establishment only while active reports require a new
   listener abstraction
   ([commissioner source](https://github.com/project-chip/matter-rs/blob/6b06fc505ced48fd3001a824de6ffb22b807ff9b/rs-matter/src/onboard.rs),
   [IM client](https://github.com/project-chip/matter-rs/blob/6b06fc505ced48fd3001a824de6ffb22b807ff9b/rs-matter/src/im/client.rs)).
3. **OHF Matter Server — prototype/bridge alternative only.** Its process split,
   host networking, persistent fabric, and WebSocket API prove the sidecar shape;
   its storage explicitly contains fabric IDs, keys, and certificates
   ([migration/storage notes](https://github.com/matter-js/matterjs-server/blob/f3e05e89cd5358f262f269f133ad5da88eabfb96/ALPHABETATESTS.md)).
   Current Matter.js Server is beta/1.4.2, stores Wi-Fi and Thread credentials in
   its application storage, and places API security on the operator
   ([config storage](https://github.com/matter-js/matterjs-server/blob/f3e05e89cd5358f262f269f133ad5da88eabfb96/packages/ws-controller/src/server/ConfigStorage.ts),
   [Docker security warning](https://github.com/matter-js/matterjs-server/blob/f3e05e89cd5358f262f269f133ad5da88eabfb96/docs/docker.md)).
   The certified Python predecessor is EOL. Neither is the v1.5 production
   baseline without a separate acceptance decision.

## Implementation gates and blockers

1. **Sidecar API/trust contract:** define authenticated transport, authorization,
   request/receipt schema, deadlines, bounded fan-out/backpressure, versioning,
   health, and redaction. Do not accept the Matter Server's unauthenticated
   all-interface WebSocket default. Prefer a runtime-only Unix socket where the
   deployment can share one safely; otherwise use exact-unicast mutually
   authenticated TLS with operator-injected keys.
2. **Secret-bearing sidecar lifecycle:** choose HSM/OS keystore or encrypted
   software store; specify atomic fabric/keystore/certificate commits, backup and
   restore as one unit, rotation/removal, permissions, and disaster recovery.
   This is outside Litehouse's backup and secret APIs.
3. **Commissioning ownership:** either keep commissioning external and preserve
   issue 44 unchanged, or approve a narrow issue-44 amendment for a sidecar-owned
   `commission-with-code`/on-network transaction followed by explicit per-device
   instance creation. No ambient candidate browser or auto-adoption.
4. **Bridge identity:** prototype Descriptor composition on representative native
   and bridged nodes. Devices lacking stable bridged `UniqueID` evidence cannot
   receive a silent duplicate/retarget guarantee.
5. **Recovery and ambiguity tests:** cover sidecar/Litehouse crashes at every
   commission, credential commit, subscription prime, command send/response, and
   receipt-journal boundary; data-version wrap/mismatch; endpoint add/remove;
   CASE re-establishment; Thread route loss; BLE unavailable; and non-idempotent
   command timeout.
6. **Dependency acceptance:** pin and patch `connectedhomeip` 1.5.x, define its
   build/update/CVE process, and run certification-oriented controller tests.
   Reconsider `matter-rs` only after attestation, network commissioning, and
   active subscription recovery are first-party complete.
7. **Explicit OTA negative gate:** no OTA API, provider, file store, BDX handler,
   or DCL update fetch in the shipping sidecar.

No hardware or external system is needed to settle the architecture. The main
remaining product decision is whether commissioning is deliberately external in
v1 or whether issue 44 is reopened for the narrow sidecar-owned flow.
