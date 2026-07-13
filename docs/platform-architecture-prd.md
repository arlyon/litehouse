# Litehouse Platform Architecture PRD

- **Status:** Draft for sequential decision-making
- **Date:** 2026-07-13
- **Classification:** Full PRD—this changes public contracts, concurrency,
  package schemas, trust boundaries, remote workflows, and appliance
  operations.

## 1. Executive summary

Litehouse is a lightweight home-automation platform built around WebAssembly components. Its differentiator is a small, fast core where device integrations, reusable services, stateful automations, and user-authored rules all use the same typed service fabric.

The target has six planes:

1. **Runtime:** WASI 0.3 and native Component Model async on Wasmtime.
2. **Service fabric:** exact-major typed contracts resolved independently from concrete plugin types.
3. **Automation:** no-code rules and native orchestration plugins with equivalent semantics.
4. **Distribution:** independent contract and plugin artifacts in a cheap, S3-backed, JavaScript-queryable registry.
5. **Control plane:** a central website owning account/server inventory, with Cockpit handling authenticated presence and WebRTC signaling.
6. **Appliance and operations:** a Fedora CoreOS-based appliance with machine-readable operations for people and AI agents.

Immediate sequence:

1. Upgrade Wasmtime to 46. **Completed in `310c6ec`.**
2. Migrate host and guests to WASI 0.3/native async. **Completed in `c210c65`.**
3. Resolve service facet/handle contracts using WIT and Matter-inspired spikes.
4. Implement the smallest typed service registry using generated host adapters (S1).
5. Build automation, distribution, control-plane, appliance, and AI-operability layers over that model.

## 2. Confirmed decisions

- **Exact major contracts:** a consumer compiled for `litehouse:light@1` resolves `light@1`, not `light@2`. It may explicitly support both. No implicit host major-version adaptation.
- **Generated typed host adapters (S1):** the host has generated bindings for known service contracts. A new implementation needs no host rebuild; a wholly new contract initially does. Reflective dispatch is deferred.
- **Symmetric plugins:** device, automation, and adapter plugins may provide and consume services, publish events, and run state machines. The no-code rule executor uses the same fabric.
- **Native async:** long-running guest operations use WIT `async func`, `future<T>`, and `stream<T>`, not merely host wrappers. Construction and metadata remain bounded and synchronous in WIT; `update` and transitional `outputs` are native async. Future lifecycle I/O belongs in explicit async initialization, not constructors.
- **Independent publishing:** contracts are first-class registry artifacts with identity, major, WIT, digest, docs, and compatibility policy. Plugin releases reference exact provided and required contract majors.
- **JavaScript registry querying:** Node/N-API is acceptable. Browser/edge WASM is desirable, not mandatory. Reads must not require a long-running registry database service.
- **Website-owned inventory:** the website owns durable users, server identities, ownership/sharing, names, and last-known metadata. Cockpit owns short-lived presence and signaling.
- **Versioned WebRTC protocol:** data channels carry structured management messages; logs are one stream, not the entire protocol. Normal home traffic stays direct after signaling.
- **Shared automation semantics:** anything available to no-code rules is available to native orchestration plugins through public contracts. Rules get no privileged concrete-plugin path.

## 3. Problem

Home Assistant is powerful but accumulates startup, dependency, integration, deployment, and operational complexity. Litehouse should preserve composability while making integration loading, plugin composition, remote access, upgrades, and recovery substantially simpler.

Current pieces do not yet form one coherent contract:

- The runner exposes fixed update variants and experimental hard-coded output resources.
- Runtime routing is a fixed broadcast/timer loop, despite useful store isolation modes.
- Registry metadata covers artifact name/version/path/size/hash, but not contracts, requirements, signatures, or provenance.
- Cockpit stores online connections in memory; durable account/server inventory is absent.
- The data channel primarily streams logs; the flow editor is mostly static.

## 4. Goals and success evidence

1. **Fast deterministic startup:** load only configured artifacts and declared metadata. Benchmark cold compile, warm cache, instantiation, discovery, and ready time separately.
2. **Typed composition:** one physical light exposes power, brightness, and color-temperature facets backed by shared state; independent consumers use different facets concurrently.
3. **Major coexistence:** `light@1` and `light@2` operate in one process without global migration.
4. **Async-native execution:** guests await I/O and expose async operations without blocking executor threads or Litehouse polling loops.
5. **Simple distribution:** JavaScript can cheaply query contract/plugin metadata while immutable objects remain CDN-cacheable.
6. **Useful remote control:** authenticated users see all paired servers and inspect services, state, automations, plugins, config, health, and logs over WebRTC.
7. **Appliance operation:** non-experts can install, pair, update, back up, restore, and recover without administering Fedora or containers.
8. **AI-first operation:** agents can inspect typed inventory, explain failures, dry-run changes, validate outcomes, and execute only authorized operations with auditability.

## 5. Users and critical use cases

- **Home operator:** installs and pairs an appliance, adds integrations, creates rules, sees actionable failures, and rolls back updates.
- **Advanced operator:** pins versions, manages several homes, writes stateful automation plugins, and controls permissions/bindings.
- **Plugin author:** declares provided/required contracts and permissions, uses async networking and typed handles, and tests locally.
- **Contract author:** publishes documented WIT independently and evolves compatibility deliberately.
- **AI agent:** reads machine-generated inventories/errors, creates bounded plans, uses idempotency and audit trails, and reproduces builds from declared tools.

## 6. Scope and non-goals

### Included

WASI 0.3/native async; typed service discovery/routing; plugin composition; shared automation semantics; contract/plugin registry redesign; durable server inventory; WebRTC management protocol; CoreOS appliance planning; AI-readable operations.

### Initially excluded

- Arbitrary unknown WIT reflection in the host.
- Implicit adaptation across contract majors.
- Central relay of routine home state or commands.
- A universal semantic model for every device category before validating a small contract set.
- Multi-node distributed Litehouse runtime consensus.
- A public plugin marketplace moderation program before signing/provenance foundations exist.

## 7. Architecture

### 7.1 Runtime and WASI transition

Wasmtime 46 provides WASI 0.3, native async, concurrent stores, and generated component bindings. Litehouse enables Component Model async and concurrency, and invokes native async exports through `StoreContextMut::run_concurrent`.

Rust `std` cannot yet target `wasm32-wasip3` in the selected nightly: the target is unavailable and a build-std attempt fails because Rust's WASI sysroot currently accepts only p1/p2 environments. Therefore the transition is intentionally hybrid:

- **Litehouse contract:** WASI 0.3/native async now.
- **Rust standard library imports:** Preview 1/2 compatibility adapter temporarily.
- **Packaging:** compile guest core modules for `wasm32-wasip1`, componentize with Wasmtime 46's versioned reactor adapter, and validate resulting components.
- **Exit condition:** move to direct p3 Rust targets when upstream `std` support is usable and all required guest dependencies support it.

Async host call wrappers for synchronous WIT exports are an embedding requirement of Wasmtime's async-enabled store; they do not make guest constructors or metadata semantically async.

### 7.2 Contract identity and compatibility

Canonical identity is `(namespace, name, major)`, for example `litehouse:light@1`. Registry releases may have semver, but runtime resolution is exact-major. Every contract release contains canonical WIT, a normalized digest, generated language artifacts where useful, documentation, examples, and conformance tests.

Within a major, compatibility is enforced by policy and automated WIT comparison. Breaking shape or semantic changes require a new major. A separately published adapter plugin may bridge majors when useful; the host never invents that bridge.

### 7.3 Provider endpoints and host-owned proxies

Plugins must not exchange raw `ResourceAny` values across stores. Wasmtime resources belong to their defining component/store and cannot safely become ambient cross-plugin references.

Instead, a provider registers one or more **endpoint descriptors**. The host creates typed **proxy resources** in the consumer's store. A proxy contains opaque host-owned routing identity such as endpoint ID, generation, contract major, and authorization context. Calls travel through bounded host queues to the provider's store/executor.

Required properties:

- stale generations fail explicitly after provider restart/reload;
- queue limits, deadlines, cancellation, and backpressure are part of dispatch;
- provider traps do not corrupt consumer stores;
- authorization is checked when binding and when invoking sensitive operations;
- tracing spans preserve consumer, provider, contract, endpoint, and correlation IDs;
- endpoint identity is stable enough for configuration but does not expose memory/store handles.

### 7.4 Multi-facet devices and shared state

A physical device may expose several capability facets while sharing one connection, cache, and lifecycle. The likely shape is a provider-owned device aggregate plus independently bindable typed facet endpoints. One Samsung television, for example, may expose power, media transport, volume, and application-launcher contracts backed by one session.

The exact WIT ownership shape is still open. The decision must prove:

- several handles can refer to one provider state owner;
- facets are independently discoverable and permissionable;
- teardown/restart has deterministic stale-handle behavior;
- no cross-store raw resource leaks;
- a plugin may expose multiple instances of the same facet;
- Matter-inspired endpoint/cluster concepts help without forcing Matter's full data model.

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
9. On failure, withdraw endpoints, increment generation, fail pending calls, restart according to policy, and rebind eligible consumers.

Startup discovery must not call arbitrary placeholder implementations. Current `outputs()` methods containing `todo!()` are a known panic hazard and must be removed before making discovery mandatory.

### 7.6 Discovery, selection, and binding

Discovery returns typed descriptors, not concrete plugin types. A descriptor minimally includes endpoint ID, contract major, provider/plugin instance, human label, device/area identity where available, health, generation, tags, and permission requirements.

Configuration stores a selector or explicit endpoint binding. Resolution must be deterministic. Ambiguity is an error unless the consumer declares a multi-binding cardinality. Automatic rebinding may occur only within declared policy and must be observable.

The first implementation should support:

- explicit endpoint ID;
- provider instance plus facet key;
- constrained selector with expected cardinality (`one`, `optional`, or `many`);
- dry-run resolution explaining matches and rejections.

### 7.7 Calls, state, events, and streams

Commands, queries, state observations, and event streams are distinct contract concepts:

- **Command:** causes an effect and returns an acknowledged result.
- **Query:** reads current provider state without subscribing.
- **Observed state:** revisioned snapshot suitable for rule evaluation and UI hydration.
- **Event:** immutable occurrence with source, sequence/revision, and timestamp metadata.
- **Stream:** ordered asynchronous delivery with explicit lag/reconnect semantics.

Contracts must define idempotency, timeout expectations, and error variants. A transport error, stale handle, denied operation, unsupported operation, provider-domain error, and invalid argument must remain distinguishable.

### 7.8 Automation architecture

A no-code rule compiles to a versioned, declarative intermediate representation. It references service selectors and contract operations rather than plugin names. The runtime validates bindings and types before activation.

Minimum rule model:

- triggers from state/event/time streams;
- typed conditions over current state and trigger context;
- ordered actions with explicit timeout/retry policy;
- optional debounce, throttle, delay, and bounded parallelism;
- durable enabled/version state;
- dry-run/simulation against recorded inputs;
- trace showing trigger, evaluated values, branch decisions, service calls, and outcomes.

Complex loops, long-lived sagas, and rich state machines belong in native automation plugins initially. Native plugins consume the same public contracts and may expose management/status contracts.

### 7.9 Permissions and trust

Plugin manifests declare requested WASI capabilities and service permissions separately:

- network destinations/protocols;
- filesystem/preopen access;
- clocks/randomness where relevant;
- contracts and operations provided/required;
- service selection scope (specific endpoint, area, or class);
- remote-management operations exposed.

The host grants a subset. Default deny applies to undeclared sensitive capabilities. Registry metadata is not itself trusted: artifacts require digest verification, and the signing/provenance policy must be chosen before a public marketplace.

The central website, Cockpit, and home runtime use separate credentials and narrowly scoped tokens. Pairing must bind a local server-generated key to a user-authorized server record. Secrets never transit logs, registry metadata, AI context, or diagnostic bundles.

### 7.10 Registry object model

The registry separates three entities:

1. **Contract release:** identity, semver/major, WIT object and digest, docs, compatibility metadata, conformance assets, deprecation/replacement links.
2. **Plugin release:** identity/version/platform, component object and digest, metadata, requested permissions, provided/required contracts, config schema, provenance/signatures, compatibility bounds.
3. **Installation manifest/lock:** selected plugin releases, exact digests, contract bindings, granted permissions, rollout channel, and optional known-good rollback set.

The current immutable S3-friendly index remains a valuable constraint. The recommended read architecture is immutable content-addressed objects plus compact partitioned indexes/manifests that can be range-fetched and CDN-cached. Writers may build indexes in JS land and use N-API for the existing Rust/FlatBuffers implementation. Reads must support conditional requests, resumable cache updates, integrity verification, and bounded memory.

The exact partitioning and publication transaction remain open. Publishing must never expose an index entry before referenced immutable objects are available.

### 7.11 Website, Cockpit, and WebRTC

Logical responsibilities:

- **Website/database:** account identity, durable server records, ownership/sharing/roles, pairing state, display metadata, audit history, and last-known presence/health.
- **Cockpit/signaling:** authenticated online connection map, presence heartbeats, offer/answer/ICE relay, and short-lived connection authorization.
- **Home server:** authoritative live configuration, service graph, state, events, logs, and operations.
- **Client:** fetches durable inventory from the website, requests a scoped connection grant, signals through Cockpit, then talks directly to the server.

A versioned data-channel envelope should include protocol version, message kind, request/stream ID, sequence, correlation ID, optional deadline, and typed payload. Initial domains:

- handshake/capability negotiation;
- service and plugin inventory;
- state snapshot and subscriptions;
- command/query calls;
- configuration read/validate/plan/apply;
- rule graph read/validate/simulate/apply;
- logs, traces, metrics, and health;
- package/update plan and progress;
- cancellation, errors, acknowledgments, and resume markers.

Large snapshots need pagination/chunking. Streams need bounded buffers and explicit dropped/resume behavior. Mutations need authorization, optimistic revision checks, idempotency keys, audit records, and preferably plan/apply separation.

### 7.12 Fedora CoreOS appliance

The appliance candidate is Fedora CoreOS with immutable OS updates and predefined Podman/systemd units for Litehouse and supporting services. It competes with Home Assistant OS on lifecycle quality, not merely installation.

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

Do not finalize the image until service topology, persistent data ownership, update atomicity, secret storage, and rollback boundaries have been prototyped.

### 7.13 AI-first operability

AI-first does not mean unrestricted shell access. It means every important operation has a discoverable structured contract:

- schema-described resources and actions;
- stable error codes plus causal chains and remediation metadata;
- inspect/validate/plan/apply phases;
- dry-run and machine-readable diffs;
- explicit required permissions and blast radius;
- idempotency and optimistic revisions;
- event/audit streams with correlation IDs;
- replayable rule traces and bounded diagnostic bundles;
- reproducible toolchain and artifact digests.

High-risk operations—permission grants, credential changes, destructive resets, production updates, and external mutations—require explicit human authorization. An agent should be able to explain why an action is needed and what evidence will prove success before execution.

## 8. Functional requirements

- **FR1:** Host supports WASI 0.3 interfaces and native async guest exports while temporarily hosting Preview 1/2 imports needed by Rust `std`.
- **FR2:** Runtime identifies contracts by exact namespace/name/major and permits concurrent majors.
- **FR3:** Providers register typed endpoint descriptors independent of concrete plugin Rust types.
- **FR4:** Consumers receive host-owned typed proxies; raw Wasmtime resources never cross stores.
- **FR5:** One provider instance can expose multiple facets sharing one state owner and lifecycle.
- **FR6:** Dispatch enforces queue bounds, deadlines, cancellation, stale generations, isolation, and authorization.
- **FR7:** Plugin manifests declare provided/required contracts and requested WASI/service permissions.
- **FR8:** Binding resolution is deterministic, typed, cardinality-aware, explainable, and validatable before activation.
- **FR9:** Rules compile to a versioned typed IR using the same service contracts as native plugins.
- **FR10:** Registry publishes separate contract, plugin release, and installation-lock entities with immutable digests.
- **FR11:** Registry queries run in supported JavaScript server environments with S3/CDN-friendly caching.
- **FR12:** Website returns durable authorized server inventory independent of current Cockpit presence.
- **FR13:** WebRTC management protocol is version-negotiated, structured, cancellable, resumable where applicable, and backpressured.
- **FR14:** Mutating remote operations use scoped authorization, revision checks, idempotency, and audit records.
- **FR15:** Appliance supports install, pair, update, health-gated rollback, backup/restore, local fallback, and redacted diagnostics.
- **FR16:** Structured inspection and plan/apply APIs support safe AI operation without requiring shell scraping.

## 9. Non-functional requirements

- **Performance:** establish baselines before numeric SLOs; instrument startup stages and service-call latency. No unbounded queue, stream, or retry loop.
- **Reliability:** provider traps are isolated; restart invalidates generations and fails in-flight calls explicitly. Known-good config/artifact sets are recoverable.
- **Security:** least privilege, authenticated pairing, encrypted transport, digest verification, secret redaction, and auditable mutations.
- **Compatibility:** exact majors coexist. Protocol and rule IR negotiate versions. Registry objects are immutable once published.
- **Observability:** structured logs, traces, metrics, health, endpoint inventory, binding decisions, queue pressure, and restart causes share correlation IDs.
- **Operability:** every mutation has validation and rollback/compensation guidance. Offline local operation remains possible.
- **Accessibility:** visual automation and control surfaces must be keyboard accessible, screen-reader meaningful, and not depend on color alone.

## 10. Acceptance scenarios

1. Build a Rust Preview 1 core module, adapt it with the Wasmtime 46 reactor adapter, validate it, inspect metadata, instantiate it, and execute native async `update` and `outputs` through Litehouse.
2. Expose three typed light facets from one provider state owner; bind separate consumers; restart the provider; observe explicit stale proxies and successful policy-driven rebind.
3. Run `light@1` and `light@2` providers/consumers simultaneously and prove no accidental cross-resolution.
4. Publish a contract, a provider plugin, and a consumer plugin; query metadata from JS; install from immutable digests; verify an offline cache hit.
5. Pair two servers to one account, show both in durable inventory, mark one offline, and connect directly to the online server via a scoped WebRTC grant.
6. Build a rule using a typed trigger/condition/action, reject an invalid binding before activation, simulate it, activate it, and inspect its execution trace.
7. Trigger a plugin trap and a queue-overload condition; show isolation, bounded failure, structured diagnostics, restart behavior, and no lost server control.
8. Stage an appliance update, fail health checks, roll back automatically, and restore backed-up configuration/artifact locks.
9. Have an AI client inspect a failure, create a dry-run remediation plan, request only required authorization, apply with an idempotency key, and report validation evidence.

## 11. Validation plan

### Stable seams

- WIT validation and component metadata for ABI semantics.
- Runtime integration tests around proxy routing, generations, queues, cancellation, and traps.
- Contract conformance suites for providers/consumers.
- Registry fixture/object tests plus JS query tests.
- Protocol codec/state-machine tests independent of WebRTC transport.
- Rule IR validation/simulation tests independent of UI.
- Ignition/Butane and VM tests for the appliance lifecycle.

### Current migration checks

```bash
wasm-tools component wit crates/plugin-macro/wit -t
cargo check -p litehouse
cargo test -p litehouse
cargo build -Zbuild-std=panic_abort,std --target wasm32-wasip1 -p noop
wasm-tools component new target/wasm32-wasip1/debug/noop.wasm \
  --adapt <wasmtime-46-preview1-reactor-adapter> -o noop.component.wasm
wasm-tools validate noop.component.wasm
litehouse inspect noop.component.wasm --no-cache
litehouse build noop <temp-dir> --debug --no-optimise
```

The generated component WIT must show native `async func` for `runner.update` and `runner.outputs`. A real host invocation, not only compilation, is required.

### Completion checks by workstream

- Rust: diagnostics on changed files, targeted package tests, then repository canonical Rust workflow.
- Site/control plane: `bun --filter site typecheck`, `bun --filter site lint`, targeted tests, and production build when environment dependencies are available.
- Registry JS: package-native typecheck/tests and N-API workflow.
- Protocol: cross-version golden fixtures, fuzz/property tests for envelopes and sequencing, and simulated disconnect/backpressure.
- Appliance: reproducible image build, clean VM first boot, upgrade/rollback, backup/restore, offline control, and secret-redaction test.
- Final: inspect diff/status for unrelated work, generated artifacts, binary drift, and credentials.

## 12. Delivery phases and gates

### Phase 0 — Runtime foundation (active)

- Wasmtime 46 upgrade.
- WASI 0.3 WIT dependency graph.
- Native async runner exports.
- Preview 1 compatibility adapter pinned to Wasmtime 46.
- Real component build/validate/instantiate/invoke test.

**Gate:** representative plugins compile and run; migration is isolated in a reviewed commit.

### Phase 1 — Contract kernel

- Resolve multi-facet WIT ownership.
- Define contract package format and generated adapter catalog.
- Implement endpoint registry and host proxy dispatch.
- Add generations, bounds, deadlines, cancellation, authorization, and tracing.
- Prove `light@1`/`light@2` coexistence.

**Gate:** end-to-end provider/consumer tests pass under restart, trap, and overload.

### Phase 2 — Registry and installation locks

- Add contract/plugin/lock schemas.
- Implement immutable object publication and partitioned query indexes.
- Add JS/N-API APIs, local cache, digest verification, and provenance fields.

**Gate:** publish/query/install/offline/rollback scenario passes without a registry database service.

### Phase 3 — Automation

- Versioned rule IR, validation, selector binding, simulation, execution traces.
- Minimal visual editor driven by contract schemas.
- Native automation plugin parity tests.

**Gate:** typed rule acceptance scenario and invalid-binding/error cases pass.

### Phase 4 — Control plane

- Durable server inventory, pairing, sharing/roles, scoped connection grants.
- Versioned WebRTC management protocol and direct service/state/config/rule channels.
- Backpressure, resume, audit, and offline UX.

**Gate:** multi-server inventory and direct authenticated management pass under disconnect/reconnect.

### Phase 5 — Appliance and AI operations

- CoreOS image/service prototype, update/rollback, backup/restore, recovery.
- Structured inspect/validate/plan/apply APIs and redacted support bundles.

**Gate:** fresh VM-to-paired-home journey, failed-update rollback, restore drill, and bounded AI remediation pass.

## 13. Risks and mitigations

| Risk | Consequence | Mitigation |
|---|---|---|
| WASI 0.3/Rust support is moving | toolchain breakage | pin nightly, Wasmtime, adapter, and wasm-tools; retain temporary p1/p2 compatibility; test real components |
| Shared facet ownership is modeled incorrectly | leaky API or duplicated device state | decide from executable WIT spike before service-kernel implementation |
| Async concurrency enables reentrancy surprises | deadlocks/state corruption | explicit per-provider actor model, bounded queues, cancellation tests, no locks held across guest calls |
| Generated S1 catalog grows host build surface | slower releases | start with few contracts; automate generation; measure; revisit reflection only with concrete pressure |
| Contract semantics drift despite shape compatibility | runtime incompatibility | semantic docs, conformance suites, normalized digests, deliberate major policy |
| Central control plane becomes data-path dependency | home outage when cloud is down | direct WebRTC after signaling and fully functional local control |
| Registry publication is partially visible | broken installs | immutable objects first, atomic manifest/index pointer last, digest verification |
| Rules become a second privileged runtime | inconsistency/security gaps | compile against the same contract/proxy/permission layer as plugins |
| Appliance appears simple but cannot recover | operator lockout/data loss | prototype rollback/restore/recovery before branding it appliance-ready |
| AI surface exposes destructive power | unsafe mutations | plan/apply, least privilege, human gates, revisions, idempotency, audit |
| Existing placeholder `outputs()` panics | startup failure | replace all `todo!()` implementations; discovery failure is typed and isolated |
| Credentials are present in local Mise config | credential compromise | rotate immediately, remove from tracked/local plaintext config, use secret manager/environment; never print values |

## 14. Ordered decision queue

Resolve one item at a time. Each answer should update this document's decision log before implementation.

1. **Facet ownership:** What exact WIT shape represents one provider-owned device with multiple independently bindable capability handles?
2. **Binding model:** Which selectors and cardinalities are supported in v1, and when may automatic rebinding occur?
3. **State/event model:** What revision, ordering, lag, replay, and timestamp guarantees do service contracts receive?
4. **Error model:** Which shared transport errors exist versus contract-domain errors, and how do cancellation/deadlines propagate?
5. **Contract artifact format:** How is WIT normalized, hashed, documented, tested, and checked for within-major compatibility?
6. **Registry publication/query:** Which immutable objects, partitions, cache manifests, and atomic publish protocol form v1?
7. **Signing and trust:** Which identities sign contracts/plugins, how are keys rotated/revoked, and what is the local trust policy?
8. **Pairing and sharing:** What proof binds a physical server key to an account, and which owner/member/support roles exist?
9. **WebRTC protocol:** Which envelope encoding, negotiation, stream semantics, and resume behavior form protocol v1?
10. **Rule IR:** Which trigger/condition/action primitives, durability guarantees, and simulation semantics form v1?
11. **Appliance topology:** Which containers/services exist, who owns persistent data/secrets, and what is the update/rollback unit?
12. **AI authorization:** Which operations are always read-only, plan-gated, confirmation-gated, or unavailable to agents?
13. **Performance SLOs:** What measured startup, call latency, memory, and recovery targets distinguish Litehouse?

## 15. Migration checkpoint (2026-07-13)

### Completed and committed

Commit `310c6ec` (`upgrade wasmtime runtime to 46`) includes Wasmtime
`46.0.1`, Mise-managed Rust nightly, mold, `-Zpolonius=next`, a Mise-aware
Taplo hook, and removal of `rust-toolchain.toml`.

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
  under the committed workspace edition 2024. This caught and fixed Rust
  2024's unsafe-attribute requirement in generated metadata.
- A hosted Samsung smoke test used a self-signed local TLS WebSocket server,
  exercised exact-address authorization, required `ms.channel.connect`, sent
  `ed.apps.launch`, completed transport futures, and confirmed tokens were not
  emitted at debug level.
- Independent correctness and risk reviewers both returned PASS after fixes.

### Deliberate compatibility boundary

Native async changes the component ABI, so stale pre-migration components are
not compatible with the new host. Adding only WASI HTTP 0.2 linkage would not
restore compatibility. Rollout must publish the rebuilt host and rebuilt
components atomically, invalidate stale component caches, and retain
`310c6ec` as the host rollback point.

### Remaining blockers and hazards

- Direct `wasm32-wasip3` Rust `std` remains unavailable; retain the documented
  compatibility bridge.
- Samsung connect, TLS, WebSocket, first-frame, and shutdown operations still
  need explicit deadlines and cancellation tests.
- Plugin restart policy still needs bounded backoff.
- HTTP authority matching still needs explicit IPv6/port semantics and
  fail-closed tests.
- The dirty root `Cargo.toml` changes workspace edition 2024 to 2021 and can
  cause Cockpit rustfmt failures; preserve it unless the user explicitly
  chooses a resolution.
- Plaintext AWS credentials in the local `.mise.toml` require rotation and
  removal. Their values must never enter logs, issues, commits, or this
  document.

### Parallel setup status

The Open Design checkout and portable `design-systems/litehouse` package are
complete and pass the upstream package validators and `pnpm guard`. The
package remains untracked in that separate repository pending an explicit
commit decision. This Pi session still requires `/reload` before it discovers
the new `open-design` MCP entry.

## 16. Rollback strategy

- Runtime migration remains isolated from the preceding Wasmtime upgrade
  commit and can be reverted independently.
- Registry schema evolution uses new immutable object versions; old readers and
  indexes remain available during transition.
- Installation locks retain exact artifact digests and a last-known-good set.
- Rule IR and WebRTC envelopes are explicitly versioned; unsupported versions
  fail negotiation rather than being guessed.
- Appliance updates preserve a bootable previous deployment and do not migrate
  durable data irreversibly without a tested backup/restore path.

## 17. Definition of done

The architecture is not done when diagrams or APIs exist. It is done when
acceptance scenarios pass with observable evidence, adversarial validation
finds no blocking correctness/security issues, canonical repository checks
pass, rollback paths are demonstrated, and the user-facing
install-to-automation journey is measurably simpler than the baseline being
replaced.
