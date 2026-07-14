# Secure representative ESPHome proof access

Type: task
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: 04
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which disposable ESPHome node(s), firmware/API versions, entity topology,
network boundary, non-production identifiers, secret-safe Noise-key injection,
rollback, and test window will the human explicitly approve for the required
real proof across multiple one-node plugin instances and repeated facets without
putting credentials in the tracker,
logs, fixtures, commits, or AI context?

## Answer

The human approved a bounded future proof using two classes of existing,
disposable ESPHome devices. Record only abstract aliases in planning, fixtures,
and test output:

- **Node S — three-channel switch:** one ESPHome node with three independent
  relay controls and three button/input entities. It proves repeated On/Off and
  Boolean facets plus metadata-only composition groups. Existing update, time,
  text/select, and other non-catalog entities remain diagnostic; the provider
  must not force them into the nine v1 contracts.
- **Node A — air-quality monitor:** a separate ESPHome node exposing
  temperature, humidity, carbon-dioxide, and available particulate/VOC
  measurements. It proves a second configured plugin instance, common-home
  profile mappings, and diagnostic-only handling for any unknown class/unit.

Do not preserve current hostnames, object IDs, addresses, MACs, device names, or
other local identifiers in the tracker or fixtures. The harness uses local
aliases only and asserts stable Litehouse identities against privately supplied
source locators.

### Access and secret boundary

- Pin the protocol fixture/client baseline to ESPHome 2026.6.5. Read each real
  node's reported ESPHome/API version during preflight and record only the
  version. If incompatible, stop; do not upgrade or alter firmware.
- The user creates a credential file under
  `$XDG_RUNTIME_DIR/litehouse-esphome-proof/` on tmpfs with mode `0600`. It holds
  only the exact addresses/API ports and Noise keys for Node S and Node A. The
  path may be supplied to the harness; values never appear in command arguments,
  environment dumps, logs, traces, errors, fixtures, snapshots, commits, or AI
  context.
- The harness rejects missing or overly broad file permissions, redacts all
  connection fields, avoids debug dumps of protobuf/config structures, and
  releases secret material after use. The user removes the runtime directory at
  the end of the window.
- Network grants allow direct local connections only to those two configured
  address/port tuples. No subnet scan, mDNS sweep, multicast grant, cloud relay,
  firewall change, or unrelated network access is approved for this proof.

### Approved real-device behavior

Use one supervised window of at most one hour. Preflight confirms the user is
present, both targets are still disposable/safe, credentials are runtime-only,
and no unexpected loads or automations make relay control hazardous.

The real proof may:

1. build/load the ESPHome provider Wasm component;
2. connect and complete Noise/API negotiation;
3. read device/entity metadata and subscribe to state;
4. publish one configured Litehouse instance per node and independently bind
   repeated facets;
5. snapshot Node S relay state, issue bounded absolute On/Off commands to
   selected relays, observe later state independently, and restore the snapshot;
6. disconnect and reconnect the **client** to prove unavailable/rehydration and
   stable identity behavior; and
7. verify Node A measurement/profile mappings without changing the monitor.

Never use Toggle in the real proof, retry an ambiguous command, claim a later
state update causally acknowledges a command, or intentionally overload a real
node. Commands are serialized in the test plan with explicit deadlines and a
`finally` restoration path. Before ending, the human verifies relay state
manually; any uncertainty is reported rather than hidden.

No firmware flash/upgrade, device reboot, factory reset, topology/configuration
change, Noise-key change, packet-secret capture, or destructive failure injection
is needed or approved. Reconnect, topology mutation, malformed framing,
overload, cancellation, and failure cases use deterministic secret-free fake
servers. Rollback is restoring the relay snapshot, disconnecting the client, and
removing the runtime credential file.

This ticket approves the target classes and safety envelope for planning. It is
not standing authority to access hardware. The real prototype ticket still
requires a separate execution proposal and explicit approval immediately before
the supervised window.
