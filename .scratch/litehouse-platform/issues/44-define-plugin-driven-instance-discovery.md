# Define plugin-driven instance discovery and adoption

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-15
Blocked by: 17, 18, 19
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Should v1 let an installed plugin type expose a bounded discovery source that
uses separately granted protocols such as mDNS to publish expiring,
deduplicated device candidates, and if so which evidence, network permissions,
stable candidate identity, conflict handling, secret/config collection,
operator review, authorization, audit, and adoption transaction safely turn one
candidate into one configured physical-device plugin instance without automatic
trust or an integration-level device container?

## Answer

No. Defer device-discovery workers, candidates, mDNS/SSDP browsing grants, and an
adoption protocol. V1 creates configured instances explicitly. One instance has
all reviewed connector configuration plus deterministic environment-secret
requirements and per-instance grants needed to connect to exactly one physical
device. This preserves the one-instance-
per-device model and avoids inventing a second pre-instance principal, candidate
identity system, LAN permission plane, and adoption transaction before there is
measured need.

Discovery of facets **inside an already configured physical device** remains
normal provider behavior. For example, after connecting to its configured node,
an ESPHome instance lists that node's entities and publishes independent typed
facets. It never turns those entities or other mDNS records into additional
configured plugin instances.

### Explicit instance configuration

Creating an instance starts from exactly one artifact source: a verified signed
registry release, or an explicitly selected local-developer artifact pinned by
path and SHA-256 under **Define signing and trust**. The host generates the stable
instance ID; neither plugin nor user supplies/reuses it. Desired config stores:

- exact plugin package/version/release digest, or local path/artifact digest;
- host-generated instance ID and optimistic config revision;
- complete bounded connector configuration such as canonical address, port,
  protocol options, and device-specific settings; address fields persist locally
  but are redacted from AI/MCP output;
- no value or caller-selected reference for schema-marked environment-secret
  fields such as credentials; runtime names derive from instance ID/field token;
- exact per-instance WASI network/resource grants;
- exact service-operation and selector grants;
- enabled/disabled desired state; and
- no candidate, discovery run, ambient LAN scope, nested device key, or
  provider-owned child-instance list.

The signed release includes one closed, versioned config JSON Schema and bounded
declarative annotations for environment-secret fields, private address/
authority fields, safe display, and restart impact. A local artifact carries the
same files, all covered by its explicit artifact digest.

The host validates schema/annotations identically in Rust and browser Wasm.
Plugins provide no HTML, JavaScript, React component, executable validator, or
arbitrary UI schema.

Shape and cross-field validation is pure host/Wasm validation with no plugin
instantiation, environment lookup, network/service/subscription grant, or
endpoint publication. Connector semantics that cannot be expressed declaratively
are checked only during actual activation and are not represented as harmless dry-
run validation. Guest diagnostics at that boundary are closed typed codes and
field paths; unrestricted guest text is neither returned nor traced.

An environment-secret schema field declares one immutable reviewed uppercase
field token.
The host derives `LITEHOUSE_INSTANCE_<UUID32_UPPER>_<FIELD_TOKEN>`, where
`UUID32_UPPER` is exactly 32 uppercase hexadecimal digits encoding the host-owned
UUID bytes without hyphens; neither plugin nor API caller can select/rebind it.

Litehouse snapshots only those required variables at process startup. It passes
each value inside the exact generated typed config—not as ambient guest
environment. It never
enumerates/forwards other variables, persists a value, or offers secret CRUD,
sharing, lookup, rotation, or export. Values are UTF-8 or schema-declared base64,
at most 64 KiB each; missing/empty/oversized/invalid values fail that instance's
activation with a redacted field error. Config APIs/forms may return the derived
variable name and configured status, never its value. Creating a draft that adds
an environment-secret field returns required names; the operator injects them
and recreates the container before activation.

### Validation and activation

The management API exposes validate, create/update draft, activate, disable, and
delete under fixed OAuth scopes and optimistic revisions. There is no adoption
API. Shape validation never runs guest code. Activation follows the existing
desired-versus-active transaction rather than pretending process start and
storage commit are atomic:

1. validate artifact source/digest, schema, runtime profile, bounds, environment-
   secret field tokens/derived-name uniqueness, and requested service/WASI grants;
2. normalize connector authority and resolve it through the host; reject any
   multicast, broadcast, wildcard, CIDR/range, port range, raw-socket, DNS-SD,
   mDNS, SSDP, or guest-controlled arbitrary resolution authority;
3. atomically persist the new desired config/grant revision, visibly pending and
   distinct from the last active set;
4. for a new/disabled instance with no predecessor, resolve only that instance's
   protected environment fields and prepare one hidden provider generation with
   exact real runtime grants; for replacement, use issue 38's no-grant preflight,
   quiesce/drop the predecessor, and only then start one granted candidate;
5. connect and return proposed facets plus a protocol-scoped identity proof where
   available;
6. under one local activation transaction/lock, compare a unique key of plugin
   package plus proof kind/version/digest, reject a duplicate, persist the proof
   digest on the instance and candidate `ActivationSet`, and switch the active-
   set pointer; and
7. only after pointer commit expose that generation and facets.

A granted hidden generation is real trusted plugin execution, not a read-only
sandbox. Network capability confinement cannot prove arbitrary protocol bytes
harmless. Issue 38's replacement preflight is different: it receives no
network/service/environment/timer/event grants and cannot prove connector
identity. First-party conformance requires no startup control actions; a local-
developer artifact is explicitly trusted to the same risk. Failure, side effects,
and ambiguous outcomes are reported honestly rather than claiming capability-
enforced read-only validation.

The identity proof is protocol/plugin-scoped and host-owned. It may use an
authenticated device identifier returned inside the secured session; address,
hostname, label, list position, or user name alone is never proof. Persist only
bounded proof kind/version/digest and safe provenance in the existing instance/
ActivationSet records, not a device container or unrestricted identifier.
Protocols without trustworthy stable identity leave it absent and receive no
duplicate or silent-retarget guarantee.

Every activation phase is journaled and idempotently reconciled after crash. For
a new install, a prepared-but-uncommitted generation remains hidden and is
terminated/retried; a committed pointer is authoritative and startup completes
publication. For replacement, issue 38's journal is authoritative: a crash after
predecessor drop but before candidate commit restarts the prior digest, so two
real sessions never overlap. Failure leaves desired versus active state visibly
unresolved. A schema-valid offline instance may be saved disabled/unverified; it
exposes no facets and cannot enable until activation succeeds. Connector/config/
credential changes use that replacement path and never retarget live bindings
before commit.

### Runtime and lifecycle

An active instance owns one store/actor, provider generation, connection/cache,
config revision, grant revision, and physical-device lifecycle. It may reconnect
to its one configured authority under its exact unicast host/port/protocol grant
and reconcile its facets. It cannot scan neighbors, browse multicast/broadcast,
create siblings, modify desired config, obtain OAuth tokens, or call management.

The host records normalized authority and activation-time resolved unicast
addresses. For a protocol with authenticated stable identity, changed DNS answers
may be tried only in a hidden reconnect path and become usable after proving the
same stored identity digest. Issue 38's exact-name WebPKI proof is the stable
`tls-dns-name` identity for HTTPS and additionally requires the same address-class
policy; it is not generic permission to change authority. Without stable identity,
any resolution-set change fails closed until an explicit config revision/
activation. Literal address or
hostname changes always require config revision. Resolution never grants a
subnet, interface, DNS suffix, redirect target, or same-network host. Labels and
metadata may change without identity change.

Disabling stops admission, quiesces/drops the store, withdraws facets, and keeps
desired non-secret config and derived environment requirements for restart.

Deletion defaults to reject while any binding, rule, or other dependency remains.
Explicit cascade lists exact dependency IDs/revisions and the resulting disable/
removal diff.
A durable `deleting` tombstone first blocks admission/restart, then idempotently
quiesces the actor and withdraws facets; one revision-checked transaction removes
approved dependencies, desired instance, scoped identity key, and activation-
set entry. Crash recovery resumes the tombstone.

Plugin update/restart preserves instance identity only after revalidating config,
grants, profile, DNS/connector identity, and facets through the same activation-
pointer protocol.

### Deferred discovery boundary

A future release may define **separate discovery artifacts or workers** that
produce untrusted connection hints. That work requires its own principal,
protocol-specific host services or WASI grants, bounds/expiry/deduplication,
spoofing model, candidate schema, identity verification, management API, and
adoption transaction. It must not be smuggled into ordinary provider startup,
installation hooks, config validation, or an ambient integration-level device
container.

If added, discovery remains optional convenience: a candidate can prefill an
explicit instance draft but cannot create, trust, grant, activate, or retarget an
instance. V1 reserves no discovery ABI. Manifest, config, and grant validation
categorically reject mDNS, DNS-SD, SSDP, subnet scan, multicast, broadcast,
wildcard, CIDR/range, raw-socket, or arbitrary resolver authority even when a
plugin requests it. Installation and pure validation execute no plugin code.

### Observability and validation

Use bounded redacted tracing for pure config validation, hidden activation,
identity-digest/duplicate decision, grants, pointer commit, retargeting, disable,
and deletion. Include safe plugin/instance/config/grant/generation/correlation
IDs and typed outcomes. Never log raw identity evidence, secret values, full
connection strings, unrestricted protocol metadata/config, guest diagnostic
text, or network payloads. Do not create a second audit database.

Required implementation tests cover registry and digest-pinned local artifacts,
schema/native/Wasm parity, pure-validation non-execution, environment-secret
name derivation/value redaction, caller-rebinding rejection, missing/empty/
oversized value failure, ambient-environment non-enumeration, authority
normalization, pinned DNS/rebinding, and categorical discovery-grant rejection.
Also cover exact unicast network/service grants and hidden-generation
confinement. Test first-party no-startup-control conformance, offline disabled
drafts, transactional identity uniqueness, absent-identity protocols, every activation/
pointer crash point, config conflicts, retarget rejection, facet reconciliation,
deletion reject/cascade/tombstone recovery, update compatibility, and proof that
installed or validating plugins cannot browse LAN discovery or create instances.
