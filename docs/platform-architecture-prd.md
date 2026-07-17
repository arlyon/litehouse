# Litehouse Platform Architecture PRD

- **Status:** Historical architecture input; superseded for implementation
- **Date:** 2026-07-13 (authority note updated 2026-07-16)
- **Classification:** Full PRD—this changes public contracts, concurrency,
  package schemas, trust boundaries, remote workflows, and appliance operations.

> **Authority note:** all architecture/topology/technology/delivery prose in this
> document is historical unless repeated by a resolved Wayfinder decision. The
> current authority is `.scratch/litehouse-platform/map.md` and issue 40. Section
> 8's FR1–FR16 wording is aligned for traceability; old S3/N-API, installation-
> lock entity, sharing-role, Preview-1, legacy-site, VM/appliance, and delivery-
> phase language authorizes no implementation.

## 1. Executive summary

Litehouse is a lightweight home-automation platform built around WebAssembly
components. Its differentiator is a small, fast core where device integrations,
reusable services, stateful automations, and user-authored rules all use the
same typed service fabric.

The target has six planes:

1. **Runtime:** WASI 0.3 and native Component Model async on Wasmtime.
2. **Service fabric:** exact-major typed contracts resolved independently from
    concrete plugin types.
3. **Automation:** no-code rules and native orchestration plugins with
    equivalent semantics.
4. **Distribution:** independent contract/plugin artifacts in an R2-only
    registry queried by one Rust/native-Wasm core.
5. **Control plane:** an owner-only greenfield Cloudflare/D1 website, with a
    separate UI-free Cockpit Worker handling bounded WebRTC signaling.
6. **Deployment and operations:** one portable non-root container plus optional
    thin Butane/Quadlet output and machine-readable operations for people/agents.

Immediate sequence:

1. Upgrade Wasmtime to 46. **Completed in `310c6ec`.**
2. Migrate host and guests to WASI 0.3/native async. **Completed in
    `c210c65`.**
3. Resolve service facet/handle contracts using WIT and Matter-inspired spikes.
4. Implement the smallest typed service registry using generated host adapters
    (S1).
5. Build automation, distribution, control-plane, appliance, and AI-operability
    layers over that model.

## 2. Confirmed decisions

- **Exact major contracts:** a consumer compiled for `litehouse:light@1`
  resolves `light@1`, not `light@2`. It may explicitly support both. No implicit
  host major-version adaptation.
- **Generated typed host adapters (S1):** the host has generated bindings for
  known service contracts. A new implementation needs no host rebuild; a wholly
  new contract initially does. Reflective dispatch is deferred.
- **Symmetric plugins:** device, automation, and adapter plugins may provide and
  consume services, publish events, and run state machines. The no-code rule
  executor uses the same fabric.
- **Detached typed facets:** physical-device state remains private to its
  provider. Each exact-major capability is registered as an independently
  bindable typed endpoint, and each consumer receives a host-owned typed proxy.
  A provider may expose any number of endpoints for the same contract; stable
  `device-key` and `facet-key` values distinguish them.
- **Stable binding selectors:** v1 selectors match stable provider, device, and
  facet keys with explicit `one`, `optional`, or `many` cardinality. Automatic
  rebinding may target only the same logical endpoint in a new provider
  generation; v1 never silently substitutes a different matching endpoint.
- **Native async:** long-running guest operations use WIT `async func`,
  `future<T>`, and `stream<T>`, not merely host wrappers. Construction and
  metadata remain bounded and synchronous in WIT; `update` and transitional
  `outputs` are native async. Future lifecycle I/O belongs in explicit async
  initialization, not constructors.
- **Independent publishing:** contracts are first-class registry artifacts with
  identity, major, WIT, digest, docs, and compatibility policy. Plugin releases
  reference exact provided and required contract majors.
- **JavaScript registry querying:** one Rust query core runs natively and through
  Wasm in supported Worker/browser JavaScript environments. N-API is retired and
  reads require no registry database service.
- **Website-owned inventory:** clean D1 owns durable users, owner-only server
  identities, names, and last-known metadata. Cockpit owns
  short-lived presence and signaling.
- **Versioned WebRTC protocol:** data channels carry structured management
  messages; logs are one stream, not the entire protocol. Normal home traffic
  stays direct after signaling.
- **Shared automation semantics:** anything available to no-code rules is
  available to native orchestration plugins through public contracts. Rules get
  no privileged concrete-plugin path.

## 3. Problem

Home Assistant is powerful but accumulates startup, dependency, integration,
deployment, and operational complexity. Litehouse should preserve composability
while making integration loading, plugin composition, remote access, upgrades,
and recovery substantially simpler.

Current pieces do not yet form one coherent contract:

- The runner exposes fixed update variants and experimental hard-coded output
  resources.
- Runtime routing is a fixed broadcast/timer loop, despite useful store
  isolation modes.
- Registry metadata covers artifact name/version/path/size/hash, but not
  contracts, requirements, signatures, or provenance.
- Cockpit stores online connections in memory; durable account/server inventory
  is absent.
- The data channel primarily streams logs; the flow editor is mostly static.

## 4. Goals and success evidence

1. **Fast deterministic startup:** load only configured artifacts and declared
    metadata. Benchmark cold compile, warm cache, instantiation, discovery, and
    ready time separately.
2. **Typed composition:** one physical light exposes power, brightness, and
    color-temperature facets backed by shared state; independent consumers use
    different facets concurrently. One provider may also expose multiple
    independently bindable instances of the same contract, such as several
    switches on one ESPHome node.
3. **Major coexistence:** `light@1` and `light@2` operate in one process
    without global migration.
4. **Async-native execution:** guests await I/O and expose async operations
    without blocking executor threads or Litehouse polling loops.
5. **Simple distribution:** JavaScript can cheaply query contract/plugin
    metadata while immutable objects remain CDN-cacheable.
6. **Useful remote control:** authenticated users see all paired servers and
    inspect services, state, automations, plugins, config, health, and logs over
    WebRTC.
7. **Appliance operation:** non-experts can install, pair, update, back up,
    restore, and recover without administering Fedora or containers.
8. **AI-first operation:** agents can inspect typed inventory, explain
    failures, dry-run changes, validate outcomes, and execute only authorized
    operations with auditability.

## 5. Users and critical use cases

- **Home operator:** installs and pairs an appliance, adds integrations, creates
  rules, sees actionable failures, and rolls back updates.
- **Advanced operator:** pins versions, manages several homes, writes stateful
  automation plugins, and controls permissions/bindings.
- **Plugin author:** declares provided/required contracts and permissions, uses
  async networking and typed handles, and tests locally.
- **Contract author:** publishes documented WIT independently and evolves
  compatibility deliberately.
- **AI agent:** reads machine-generated inventories/errors, creates bounded
  plans, uses idempotency and audit trails, and reproduces builds from declared
  tools.

## 6. Scope and non-goals

### Included

WASI 0.3/native async; typed service discovery/routing; plugin composition;
shared automation semantics; contract/plugin registry redesign; durable server
inventory; WebRTC management protocol; CoreOS appliance planning; AI-readable
operations.

### Initially excluded

- Arbitrary unknown WIT reflection in the host.
- Implicit adaptation across contract majors.
- Central relay of routine home state or commands.
- A universal semantic model for every device category before validating a small
  contract set.
- Multi-node distributed Litehouse runtime consensus.
- A public plugin marketplace moderation program before signing/provenance
  foundations exist.

## 7. Architecture

### 7.1 Runtime and WASI transition

Wasmtime 46 provides WASI 0.3, native async, concurrent stores, and generated
component bindings. Litehouse enables Component Model async and concurrency, and
invokes native async exports through `StoreContextMut::run_concurrent`.

V1 uses the signed runtime profile defined by issue 23 rather than a temporary
Preview-1 packaging bridge:

- **Litehouse contract:** official WASI 0.3/native async plus Litehouse typed host
  interfaces.
- **Rust standard library imports:** exact official WASI 0.2.12 imports produced
  by `wasm32-wasip2`/`std` and linked by the host only under explicit grants.
- **Packaging:** build guest components directly for `wasm32-wasip2` and validate
  their exact imports and Component Model async exports.
- **Evolution:** additive profile minors retain prior imports; removing an
  interface requires a new profile major. Direct Rust `wasip3` `std` is not a V1
  gate.

Async host call wrappers for synchronous WIT exports are an embedding
requirement of Wasmtime's async-enabled store; they do not make guest
constructors or metadata semantically async.

### 7.2 Contract identity and compatibility

Canonical identity is `(namespace, name, major)`, for example
`litehouse:light@1`. Registry releases may have semver, but runtime resolution
is exact-major. Every contract release contains canonical WIT, a normalized
digest, generated language artifacts where useful, documentation, examples, and
conformance tests.

Within a major, compatibility is enforced by policy and automated WIT
comparison. Breaking shape or semantic changes require a new major. A separately
published adapter plugin may bridge majors when useful; the host never invents
that bridge.

### 7.3 Provider endpoints and host-owned proxies

Plugins must not exchange raw `ResourceAny` values across stores. Wasmtime
resources belong to their defining component/store and cannot safely become
ambient cross-plugin references.

Instead, a provider registers one or more **endpoint descriptors**. The host
creates typed **proxy resources** in the consumer's store. A proxy contains
opaque host-owned routing identity such as endpoint ID, generation, contract
major, and authorization context. Calls travel through bounded host queues to
the provider's store/executor.

Required properties:

- stale generations fail explicitly after provider restart/reload;
- queue limits, deadlines, cancellation, and backpressure are part of dispatch;
- provider traps do not corrupt consumer stores;
- authorization is checked when binding and when invoking sensitive operations;
- tracing spans preserve consumer, provider, contract, endpoint, and correlation
  IDs;
- endpoint identity is stable enough for configuration but does not expose
  memory/store handles.

### 7.4 Multi-facet devices and shared state

**Decision (2026-07-13): use detached typed facets.** A provider privately owns
one or more device aggregates containing each physical connection, cache,
lifecycle, and synchronization boundary. The aggregate is not a public WIT
resource. It never crosses into the host or a consumer.

Each exact-major contract defines a resource interface. A generated provider
surface returns a list of typed endpoint records. Conceptually, a switch
contract has this ownership shape; binding and error details remain decisions
2 and 4:

```wit
interface provider {
  use service.{switch};

  record endpoint {
    device-key: string,
    facet-key: string,
    label: string,
    implementation: own<switch>,
  }

  endpoints: func() -> list<endpoint>;
}
```

The returned `own<switch>` is retained only in the provider's store. The host
creates a different resource implementing the same exact-major interface in
each consumer store. That consumer-local resource contains only host-owned
routing state: endpoint ID, provider generation, contract major, authorization
context, and bounded dispatch sender. No raw `ResourceAny`, guest resource,
borrow, instance, or resource-table index crosses stores.

Endpoint identity is:

```text
(provider instance, device-key, facet-key, namespace, contract name, major)
```

Repeated facets are first-class, not an edge case. An ESPHome provider may
manage several nodes, and one node may expose many `switch@1` endpoints. Those
endpoints share the node's private connection and cache, use the same stable
`device-key`, and have distinct source-stable `facet-key` values such as entity
IDs. Labels and list positions are not identities. The provider surface returns
all of them; it is not limited to one endpoint per contract.

Dropping a consumer proxy releases only that binding. It does not drop the
provider endpoint or close the shared connection. Provider restart withdraws
all endpoints from the old aggregate generation; old proxies remain stale and
are never silently retargeted. Rebinding creates new proxies. A topology change
republishes the affected aggregate as a new generation in v1.

This shape preserves independent discovery and permissions, supports multiple
same-contract facets, and keeps major versions independent. Matter contributes
only useful vocabulary—device, endpoint, and capability/cluster occurrence—not
a mandatory parent-device API or full Matter data model. A public aggregate
resource can be introduced later only if device-wide transactions become a
real contract requirement.

### 7.5 Provider and consumer lifecycle

Proposed phases:

1. Read artifact metadata without constructing configured instances.
2. Validate configuration and contract requirements.
3. Instantiate plugin store/component.
4. Run bounded synchronous construction.
5. Run explicit asynchronous initialization if required.
6. Register endpoint descriptors atomically.
7. Resolve required bindings and create proxies.
8. Start event streams and scheduled work.
9. On failure, withdraw endpoints, increment generation, fail pending calls,
    restart according to policy, and rebind eligible consumers.

Startup discovery must not call arbitrary placeholder implementations. The
migration removed the tracked plugins' discovery-time `todo!()` traps; future
provider registration must preserve that invariant.

### 7.6 Discovery, selection, and binding

Discovery returns typed descriptors, not concrete plugin types. A descriptor
minimally includes endpoint ID, contract major, provider/plugin instance, human
label, device/area identity where available, health, generation, tags, and
permission requirements.

**Decision (2026-07-13): use stable exact-field selectors with explicit
cardinality.** A persisted selector may constrain `provider-instance`,
`device-key`, and `facet-key`; omitted fields act as wildcards. Contract
namespace, name, and exact major come from the typed requirement and cannot be
wildcarded. Labels, list positions, areas, and mutable tags are discovery
metadata, not v1 binding identity.

Every requirement declares one cardinality:

- `one` succeeds only with exactly one match;
- `optional` succeeds with zero or one match and rejects ambiguity; and
- `many` binds the complete match set in canonical endpoint-identity order.

Resolution occurs at activation and produces a snapshot plus an explanation of
matches and rejections. A `many` binding does not silently add a newly appearing
identity. Membership changes require explicit re-resolution, preserving
predictable automations.

Every requirement also declares `rebind: never | same-identity`.
`same-identity` may replace a stale proxy only when provider instance, device
key, facet key, contract identity, and major all match and only the provider
generation changed. `never` leaves the requirement stale until explicit
activation. Neither mode may select a different logical endpoint, even if that
endpoint matches the original wildcard selector.

Host endpoint IDs and generations remain useful in diagnostics and transient
APIs, but persisted configuration uses the stable selector fields. Dry-run
resolution must report cardinality, deterministic ordering, matches,
rejections, ambiguity, and the exact rebind policy.

### 7.7 Calls, state, events, and streams

Commands, queries, state observations, and event streams are distinct contract
concepts:

- **Command:** causes an effect and returns an acknowledged result.
- **Query:** reads current provider state without subscribing.
- **Observed state:** revisioned snapshot suitable for rule evaluation and UI
  hydration.
- **Event:** immutable occurrence with source, sequence/revision, and timestamp
  metadata.
- **Stream:** ordered asynchronous delivery with explicit lag/reconnect
  semantics.

Contracts must define idempotency, timeout expectations, and error variants. A
transport error, stale handle, denied operation, unsupported operation,
provider-domain error, and invalid argument must remain distinguishable.

### 7.8 Automation architecture

A no-code rule compiles to a versioned, declarative intermediate representation.
It references service selectors and contract operations rather than plugin
names. The runtime validates bindings and types before activation.

Minimum rule model:

- triggers from state/event/time streams;
- typed conditions over current state and trigger context;
- ordered actions with explicit timeout/retry policy;
- optional debounce, throttle, delay, and bounded parallelism;
- durable enabled/version state;
- dry-run/simulation against recorded inputs;
- trace showing trigger, evaluated values, branch decisions, service calls, and
  outcomes.

Complex loops, long-lived sagas, and rich state machines belong in native
automation plugins initially. Native plugins consume the same public contracts
and may expose management/status contracts.

### 7.9 Permissions and trust

Plugin manifests declare requested WASI capabilities and service permissions
separately:

- network destinations/protocols;
- filesystem/preopen access;
- clocks/randomness where relevant;
- contracts and operations provided/required;
- service selection scope (specific endpoint, area, or class);
- remote-management operations exposed.

The host grants a subset. Default deny applies to undeclared sensitive
capabilities. Registry metadata is not itself trusted: artifacts require digest
verification, and the signing/provenance policy must be chosen before a public
marketplace.

The central website, Cockpit, and home runtime use separate credentials and
narrowly scoped tokens. Pairing must bind a local server-generated key to a
user-authorized server record. Secrets never transit logs, registry metadata, AI
context, or diagnostic bundles.

### 7.10 Registry object model

The registry separates three entities:

1. **Contract release:** identity, semver/major, WIT object and digest, docs,
    compatibility metadata, conformance assets, deprecation/replacement links.
2. **Plugin release:** identity/version/platform, component object and digest,
    metadata, requested permissions, provided/required contracts, config schema,
    provenance/signatures, compatibility bounds.
3. **Installation manifest/lock:** selected plugin releases, exact digests,
    contract bindings, granted permissions, rollout channel, and optional
    known-good rollback set.

The current immutable S3-friendly index remains a valuable constraint. The
recommended read architecture is immutable content-addressed objects plus
compact partitioned indexes/manifests that can be range-fetched and CDN-cached.
Writers may build indexes in JS land and use N-API for the existing
Rust/FlatBuffers implementation. Reads must support conditional requests,
resumable cache updates, integrity verification, and bounded memory.

The exact partitioning and publication transaction remain open. Publishing must
never expose an index entry before referenced immutable objects are available.

### 7.11 Website, Cockpit, and WebRTC

Logical responsibilities:

- **Website/database:** account identity, durable server records,
  ownership/sharing/roles, pairing state, display metadata, audit history, and
  last-known presence/health.
- **Cockpit/signaling:** authenticated online connection map, presence
  heartbeats, offer/answer/ICE relay, and short-lived connection authorization.
- **Home server:** authoritative live configuration, service graph, state,
  events, logs, and operations.
- **Client:** fetches durable inventory from the website, requests a scoped
  connection grant, signals through Cockpit, then talks directly to the server.

A versioned data-channel envelope should include protocol version, message kind,
request/stream ID, sequence, correlation ID, optional deadline, and typed
payload. Initial domains:

- handshake/capability negotiation;
- service and plugin inventory;
- state snapshot and subscriptions;
- command/query calls;
- configuration read/validate/plan/apply;
- rule graph read/validate/simulate/apply;
- logs, traces, metrics, and health;
- package/update plan and progress;
- cancellation, errors, acknowledgments, and resume markers.

Large snapshots need pagination/chunking. Streams need bounded buffers and
explicit dropped/resume behavior. Mutations need authorization, optimistic
revision checks, idempotency keys, audit records, and preferably plan/apply
separation.

### 7.12 Fedora CoreOS appliance

The appliance candidate is Fedora CoreOS with immutable OS updates and
predefined Podman/systemd units for Litehouse and supporting services. It
competes with Home Assistant OS on lifecycle quality, not merely installation.

Required operator flows:

- image installation and first-boot discovery;
- secure local proof and account pairing;
- automatic signed OS/application updates with staged health checks;
- rollback to last known good deployment;
- encrypted backup/export and tested restore;
- storage expansion and hardware discovery;
- local access when cloud/signaling is unavailable;
- support bundle creation with automatic secret redaction;
- documented recovery mode.

Do not finalize the image until service topology, persistent data ownership,
update atomicity, secret storage, and rollback boundaries have been prototyped.

### 7.13 AI-first operability

AI-first does not mean unrestricted shell access. It means every important
operation has a discoverable structured contract:

- schema-described resources and actions;
- stable error codes plus causal chains and remediation metadata;
- inspect/validate/plan/apply phases;
- dry-run and machine-readable diffs;
- explicit required permissions and blast radius;
- idempotency and optimistic revisions;
- event/audit streams with correlation IDs;
- replayable rule traces and bounded diagnostic bundles;
- reproducible toolchain and artifact digests.

High-risk operations—permission grants, credential changes, destructive resets,
production updates, and external mutations—require explicit human authorization.
An agent should be able to explain why an action is needed and what evidence
will prove success before execution.

## 8. Functional requirements

- **FR1:** Host supports Wasmtime Component Model async/concurrency, official
  WASI 0.3 native-async interfaces, and native async guest exports while the
  pinned runtime profile also supplies exact WASI 0.2.12 imports produced by
  Rust `wasm32-wasip2`/`std`; no Preview-1 reactor packaging path remains.
- **FR2:** Runtime identifies contracts by exact namespace/name/major and
  permits concurrent majors.
- **FR3:** Providers register typed endpoint descriptors independent of concrete
  plugin Rust types.
- **FR4:** Consumers receive host-owned typed proxies; raw Wasmtime resources
  never cross stores.
- **FR5:** One configured provider instance represents one physical device and
  can expose any number of independently bindable facets, including repeated
  instances of the same exact-major contract, while sharing its private device
  connection/cache/state owner and lifecycle.
- **FR6:** Dispatch enforces queue bounds, deadlines, cancellation, stale
  generations, isolation, and authorization.
- **FR7:** Plugin manifests declare provided/required contracts and requested
  WASI/service permissions.
- **FR8:** Binding resolution is deterministic, typed, cardinality-aware,
  explainable, and validatable before activation.
- **FR9:** Rules compile to a versioned typed IR using the same service
  contracts as native plugins.
- **FR10:** The R2 registry publishes separate immutable contract packages and
  signed complete plugin release records; local `ActivationSet` records pin exact
  installed artifacts and one predecessor rather than creating a registry lock
  entity.
- **FR11:** One Rust registry query core runs natively and through Wasm in
  supported JavaScript Worker/browser environments using R2/CDN GET/HEAD cache
  semantics; V1 has no N-API or S3 backend.
- **FR12:** Website returns durable authorized server inventory independent of
  current Cockpit presence.
- **FR13:** WebRTC management protocol is version-negotiated, structured,
  cancellable, resumable where applicable, and backpressured.
- **FR14:** Mutating remote operations use scoped authorization, revision
  checks, idempotency, durable mutation receipts, and bounded structured tracing;
  V1 adds no separate audit database.
- **FR15:** The portable container supports install, pair, update, health-gated
  rollback, local fallback, and redacted diagnostics; backup/restore is an
  operator-managed stopped-directory procedure whose receipt guard blocks
  destructive restore while post-copy ambiguity remains.
- **FR16:** Structured inspection and plan/apply APIs support safe AI operation
  without requiring shell scraping.

## 9. Non-functional requirements

- **Performance:** establish baselines before numeric SLOs; instrument startup
  stages and service-call latency. No unbounded queue, stream, or retry loop.
- **Reliability:** provider traps are isolated; restart invalidates generations
  and fails in-flight calls explicitly. Known-good config/artifact sets are
  recoverable.
- **Security:** least privilege, authenticated pairing, encrypted transport,
  digest verification, secret redaction, and auditable mutations.
- **Compatibility:** exact majors coexist. Protocol and rule IR negotiate
  versions. Registry objects are immutable once published.
- **Observability:** structured logs, traces, metrics, health, endpoint
  inventory, binding decisions, queue pressure, and restart causes share
  correlation IDs.
- **Operability:** every mutation has validation and rollback/compensation
  guidance. Offline local operation remains possible.
- **Accessibility:** visual automation and control surfaces must be keyboard
  accessible, screen-reader meaningful, and not depend on color alone.

## 10. Acceptance scenarios

1. Build a Rust plugin directly for `wasm32-wasip2`, validate its exact runtime-
    profile imports and Component Model async exports, inspect metadata,
    instantiate it, and execute native async lifecycle/service calls through
    Litehouse without a Preview-1 reactor adapter.
2. Expose three typed light facets from one provider state owner; bind separate
    consumers; restart the provider; observe explicit stale proxies and
    successful policy-driven rebind.
3. Expose at least two `switch@1` endpoints from one ESPHome-style device using
    one shared connection, one stable device key, and distinct stable facet
    keys; bind and operate both independently.
4. Resolve those switches with `one`, `optional`, and `many` requirements;
    reject ambiguity, preserve canonical ordering, and prove that
    `same-identity` rebinding never substitutes or silently adds another
    switch.
5. Run `light@1` and `light@2` providers/consumers simultaneously and prove no
    accidental cross-resolution.
6. Publish a contract, a provider plugin, and a consumer plugin; query metadata
    from JS; install from immutable digests; verify an offline cache hit.
7. Pair two servers to one account, show both in durable inventory, mark one
    offline, and connect directly to the online server via a scoped WebRTC
    grant.
8. Build a rule using a typed trigger/condition/action, reject an invalid
    binding before activation, simulate it, activate it, and inspect its
    execution trace.
9. Trigger a plugin trap and a queue-overload condition; show isolation,
    bounded failure, structured diagnostics, restart behavior, and no lost
    server control.
10. Stage a digest-pinned container update, fail health checks, select the
    declared N-1 rollback path, and prove that stopped-copy restore fails closed
    while any post-copy `outcome-unknown` receipt remains.
11. Have an AI client inspect a failure, create a dry-run remediation plan,
    request only required authorization, apply with an idempotency key, and
    report validation evidence.

## 11. Validation plan

### Stable seams

- WIT validation and component metadata for ABI semantics.
- Runtime integration tests around proxy routing, generations, queues,
  cancellation, and traps.
- Contract conformance suites for providers/consumers.
- Registry fixture/object tests plus JS query tests.
- Protocol codec/state-machine tests independent of WebRTC transport.
- Rule IR validation/simulation tests independent of UI.
- Ignition/Butane and VM tests for the appliance lifecycle.

### Current migration checks

``` bash
wasm-tools component wit crates/plugin-macro/wit -t
cargo check -p litehouse
cargo test -p litehouse
cargo build --target wasm32-wasip2 -p noop
wasm-tools validate --features cm-async target/wasm32-wasip2/debug/noop.wasm
litehouse inspect target/wasm32-wasip2/debug/noop.wasm --no-cache
litehouse build noop <temp-dir> --debug --no-optimise
```

The generated component WIT must show native `async func` for `runner.update`
and `runner.outputs`. A real host invocation, not only compilation, is required.

### Completion checks by workstream

- Rust: diagnostics on changed files, targeted package tests, then repository
  canonical Rust workflow.
- Web/control plane: greenfield `pnpm --dir web check`, targeted Worker/D1/
  browser tests, and production build when environment dependencies are
  available; legacy `site/` code is not a migration input.
- Registry: native/Wasm byte-parity, R2 fixture/cache tests, and supported
  JavaScript-runtime wrapper tests; no N-API workflow.
- Protocol: cross-version golden fixtures, fuzz/property tests for envelopes and
  sequencing, and simulated disconnect/backpressure.
- Appliance: reproducible image build, clean VM first boot, upgrade/rollback,
  backup/restore, offline control, and secret-redaction test.
- Final: inspect diff/status for unrelated work, generated artifacts, binary
  drift, and credentials.

## 12. Delivery phases and gates

> Historical planning only. The resolved implementation tranches in Wayfinder
> issue 40 supersede this section; these phases authorize no legacy `site/`,
> N-API/S3 registry, Preview-1 adapter, VM appliance, sharing-role, or deployment
> work.

### Phase 0 — Runtime foundation (historical)

- Wasmtime 46 upgrade.
- WASI 0.3 WIT dependency graph.
- Native async runner exports.
- Preview 1 compatibility adapter pinned to Wasmtime 46.
- Real component build/validate/instantiate/invoke test.

**Gate:** representative plugins compile and run; migration is isolated in a
reviewed commit.

### Phase 1 — Contract kernel

- Implement the decided detached-facet WIT ownership model.
- Define contract package format and generated adapter catalog.
- Implement endpoint registry and host proxy dispatch.
- Make `switch@1` the first vertical contract and use an ESPHome-style provider
  fixture to prove multiple switches on one shared device connection.
- Add generations, bounds, deadlines, cancellation, authorization, and tracing.
- Prove exact-major coexistence with two versions of one contract.

**Gate:** end-to-end provider/consumer tests pass under restart, trap, and
overload.

### Phase 2 — Registry and installation locks

- Add contract/plugin/lock schemas.
- Implement immutable object publication and partitioned query indexes.
- Add JS/N-API APIs, local cache, digest verification, and provenance fields.

**Gate:** publish/query/install/offline/rollback scenario passes without a
registry database service.

### Phase 3 — Automation

- Versioned rule IR, validation, selector binding, simulation, execution traces.
- Minimal visual editor driven by contract schemas.
- Native automation plugin parity tests.

**Gate:** typed rule acceptance scenario and invalid-binding/error cases pass.

### Phase 4 — Control plane

- Durable server inventory, pairing, sharing/roles, scoped connection grants.
- Versioned WebRTC management protocol and direct service/state/config/rule
  channels.
- Backpressure, resume, audit, and offline UX.

**Gate:** multi-server inventory and direct authenticated management pass under
disconnect/reconnect.

### Phase 5 — Appliance and AI operations

- CoreOS image/service prototype, update/rollback, backup/restore, recovery.
- Structured inspect/validate/plan/apply APIs and redacted support bundles.

**Gate:** fresh VM-to-paired-home journey, failed-update rollback, restore
drill, and bounded AI remediation pass.

## 13. Risks and mitigations

- **WASI 0.3/Rust support is moving.** Toolchain breakage is mitigated by
  pinning nightly, Wasmtime, the adapter, and wasm-tools; retaining temporary
  p1/p2 compatibility; and testing real components.
- **Shared facet ownership may be modeled incorrectly.** A leaky API or
  duplicated device state is mitigated by deciding from an executable WIT
  spike before service-kernel implementation.
- **Async concurrency enables reentrancy surprises.** Deadlocks and state
  corruption are mitigated by an explicit per-provider actor model, bounded
  queues, cancellation tests, and no locks held across guest calls.
- **The generated S1 catalog grows host build surface.** Slower releases are
  mitigated by starting with few contracts, automating generation, measuring,
  and revisiting reflection only with concrete pressure.
- **Contract semantics may drift despite shape compatibility.** Runtime
  incompatibility is mitigated with semantic documentation, conformance
  suites, normalized digests, and deliberate major-version policy.
- **The central control plane may become a data-path dependency.** Home outage
  when the cloud is down is mitigated by direct WebRTC after signaling and
  fully functional local control.
- **Registry publication may become partially visible.** Broken installs are
  mitigated by writing immutable objects first, atomically publishing the
  manifest/index pointer last, and verifying digests.
- **Rules may become a second privileged runtime.** Inconsistency and security
  gaps are mitigated by compiling against the same contract, proxy, and
  permission layer as plugins.
- **The appliance may appear simple but remain unrecoverable.** Operator
  lockout and data loss are mitigated by prototyping rollback, restore, and
  recovery before branding it appliance-ready.
- **The AI surface exposes destructive power.** Unsafe mutations are mitigated
  by plan/apply separation, least privilege, human gates, revisions,
  idempotency, and audit.
- **Placeholder `outputs()` methods may panic.** Startup failure is mitigated
  by replacing every `todo!()` implementation and making discovery failure
  typed and isolated.
- **Credentials are present in local Mise configuration.** Compromise is
  mitigated by immediate rotation, removal from tracked or local plaintext
  configuration, secret-manager or environment injection, and never printing
  values.

## 14. Ordered decision queue

Resolve one item at a time. Each answer should update this document's decision
log before implementation.

1. **Facet ownership — decided 2026-07-13:** private device aggregates,
    detached exact-major typed provider endpoints, and consumer-store-local
    host proxies. Repeated same-contract facets use stable device and facet
    keys.
2. **Binding model — decided 2026-07-13:** stable exact-field selectors,
    explicit `one`/`optional`/`many` cardinality, and `never` or
    `same-identity` rebinding. No silent endpoint substitution or dynamic
    membership in v1.
3. **State/event model — next:** What revision, ordering, lag, replay, and timestamp
    guarantees do service contracts receive?
4. **Error model:** Which shared transport errors exist versus contract-domain
    errors, and how do cancellation/deadlines propagate?
5. **Contract artifact format:** How is WIT normalized, hashed, documented,
    tested, and checked for within-major compatibility?
6. **Registry publication/query:** Which immutable objects, partitions, cache
    manifests, and atomic publish protocol form v1?
7. **Signing and trust:** Which identities sign contracts/plugins, how are keys
    rotated/revoked, and what is the local trust policy?
8. **Pairing and sharing:** What proof binds a physical server key to an
    account, and which owner/member/support roles exist?
9. **WebRTC protocol:** Which envelope encoding, negotiation, stream semantics,
    and resume behavior form protocol v1?
10. **Rule IR:** Which trigger/condition/action primitives, durability
    guarantees, and simulation semantics form v1?
11. **Appliance topology:** Which containers/services exist, who owns persistent
    data/secrets, and what is the update/rollback unit?
12. **AI authorization:** Which operations are always read-only, plan-gated,
    confirmation-gated, or unavailable to agents?
13. **Performance SLOs:** What measured startup, call latency, memory, and
    recovery targets distinguish Litehouse?

## 15. Migration checkpoint (2026-07-13)

### Completed and committed

Commit `310c6ec` (`upgrade wasmtime runtime to 46`) includes Wasmtime `46.0.1`,
Mise-managed Rust nightly, mold, `-Zpolonius=next`, a Mise-aware Taplo hook, and
removal of `rust-toolchain.toml`.

Commit `c210c65` (`migrate plugin runtime to WASI 0.3`) is the separate,
unsigned ABI migration. It:

- vendors Wasmtime 46's WASI 0.3 WIT dependency graph;
- makes runner `update` and `outputs` native async exports;
- migrates guests to WASI HTTP/client and sockets/types 0.3;
- upgrades guest `wit-bindgen` to 0.59;
- enables Component Model async, concurrency, and `run_concurrent`;
- uses Wasmtime's versioned Preview 1 reactor adapter provider instead of a
  drifting repository binary;
- migrates all eight tracked plugin packages and removes discovery-time
  `todo!()` traps;
- forwards real broadcast payloads and propagates output-discovery rejection;
- adds exact-address `tcp-client:<ip>:<port>` authorization; and
- enables Samsung's TLS WebSocket transport on port 8002 with an explicit,
  intentionally unpinned certificate policy and documented LAN MITM risk.

The locally present, untracked Twilio package was built and validated with the
same ABI but was deliberately not absorbed into the commit.

### Validation evidence

- `cargo test -p litehouse`, `cargo test -p litehouse-config`, and
  `cargo test -p samsung` pass.
- All nine local plugin packages pass `cargo check --target wasm32-wasip1`.
- Real packaged components validate with `wasm-tools`; extracted WIT shows
  native async runner exports plus WASI HTTP/sockets 0.3 imports.
- The staged patch was applied to an isolated `310c6ec` worktree and validated
  under the committed workspace edition 2024. This caught and fixed Rust 2024's
  unsafe-attribute requirement in generated metadata.
- A hosted Samsung smoke test used a self-signed local TLS WebSocket server,
  exercised exact-address authorization, required `ms.channel.connect`, sent
  `ed.apps.launch`, completed transport futures, and confirmed tokens were not
  emitted at debug level.
- Independent correctness and risk reviewers both returned PASS after fixes.

### Deliberate compatibility boundary

Native async changes the component ABI, so stale pre-migration components are
not compatible with the new host. Adding only WASI HTTP 0.2 linkage would not
restore compatibility. Rollout must publish the rebuilt host and rebuilt
components atomically, invalidate stale component caches, and retain `310c6ec`
as the host rollback point.

### Remaining blockers and hazards

- Direct `wasm32-wasip3` Rust `std` remains unavailable; retain the documented
  compatibility bridge.
- Samsung connect, TLS, WebSocket, first-frame, and shutdown operations still
  need explicit deadlines and cancellation tests.
- Plugin restart policy still needs bounded backoff.
- HTTP authority matching still needs explicit IPv6/port semantics and
  fail-closed tests.
- The dirty root `Cargo.toml` changes workspace edition 2024 to 2021 and can
  cause Cockpit rustfmt failures; preserve it unless the user explicitly chooses
  a resolution.
- Plaintext AWS credentials in the local `.mise.toml` require rotation and
  removal. Their values must never enter logs, issues, commits, or this
  document.

### Parallel setup status

The Open Design checkout and portable `design-systems/litehouse` package are
complete and pass the upstream package validators and `pnpm guard`. The package
remains untracked in that separate repository pending an explicit commit
decision. This Pi session still requires `/reload` before it discovers the new
`open-design` MCP entry.

## 16. Rollback strategy

- Runtime migration remains isolated from the preceding Wasmtime upgrade commit
  and can be reverted independently.
- Registry schema evolution uses new immutable object versions; old readers and
  indexes remain available during transition.
- Installation locks retain exact artifact digests and a last-known-good set.
- Rule IR and WebRTC envelopes are explicitly versioned; unsupported versions
  fail negotiation rather than being guessed.
- Appliance updates preserve a bootable previous deployment and do not migrate
  durable data irreversibly without a tested backup/restore path.

## 17. Definition of done

The architecture is not done when diagrams or APIs exist. It is done when
acceptance scenarios pass with observable evidence, adversarial validation finds
no blocking correctness/security issues, canonical repository checks pass,
rollback paths are demonstrated, and the user-facing install-to-automation
journey is measurably simpler than the baseline being replaced.
