# Implementation decision log

Updated: 2026-07-18
Owner: T1–T15 implementation route
Status: active; review requested in the morning

This log records implementation decisions that are unclear, inferred, newly
resolved, or deliberately deferred while executing the approved tranches. It is
not authority to publish, deploy, access credentials, mutate external services,
or operate servers/devices. It must never contain secrets or protected local
configuration.

## How this log is maintained

- Add an entry when repository evidence and resolved planning do not determine
  one safe implementation.
- Mark whether the item blocks the active tranche.
- Record the chosen resolution and evidence before implementing a consequential
  assumption.
- Keep external-authority decisions unresolved until separately approved.
- Update this file before completing each tranche.

## Open decisions

### D-001 — T1 production package topology

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Should the nine exact-major contracts and shared packages be
  represented as separate source-package directories immediately, or begin in
  one workspace crate that emits independently versioned WIT packages?
- **Known constraints:** releases are independently versioned; no separately
  published generated helper-crate matrix; code generation reads a locked local
  WIT graph; runtime routing keys on namespace/name/major.
- **Resolution:** use one `litehouse-contracts` Rust tooling/core crate with 15
  independently versioned WIT source-package roots. Each root emits its own
  deterministic binary WIT package and ABI digest; generated Rust remains in
  `OUT_DIR`. This preserves publication boundaries without a helper-crate matrix.

### D-002 — Canonical management-operation manifest format

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Which tracked source format should own
  `management-operations-v1` so Rust, HTTP, WebRTC, web, and MCP adapters can be
  generated without introducing a second behavioral authority?
- **Known constraints:** the manifest is closed and versioned; each operation
  fixes scope, method/path, mutation/read class, idempotency, revision,
  deadlines, errors, redaction, projection, and streaming shape; no transport-
  only operation is allowed.
- **Resolution:** checked-in strict JSON is the sole 74-row operation inventory.
  It carries closed schemas and digests; build-time validation generates typed
  Rust and transport projection metadata only. No generated adapter performs I/O
  or registers a route in T1.

### D-003 — Canonical decimal bounds

- **Status:** resolved
- **Blocks:** nothing
- **Question:** What exact signed-coefficient width, scale range, input length,
  token count, nesting, exponent, precision, and diagnostic-span limits become
  the immutable quantity-profile v1 contract?
- **Known constraints:** values are canonical decimal strings; no binary float,
  implicit rounding, NaN/infinity, unknown result profile, or unbounded input;
  recurring results require explicit round-half-even quantization.
- **Resolution:** signed checked `i128`, at most 38 significant digits, scale
  0–18, 96-byte magnitude input, 256-byte UCUM input, 64 tokens, nesting depth
  eight, and exponent magnitude 12. Diagnostics expose stable codes and bounded
  spans only. Ratio profile bounds are enforced on parse, deserialization, and
  every arithmetic result. Canonical dBm source quantities may normalize and
  compare, but all arithmetic on logarithmic quantities fails with
  `semantic-mismatch`. These are correctness/abuse bounds, not performance SLOs.

### D-004 — Per-operation default deadlines in T1

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Which bounded default deadline is assigned to each generated
  operation while preserving the runtime hard ceiling?
- **Known constraints:** queue time counts; callers may shorten but not lengthen;
  ordinary provider operations have a 30-second host ceiling; exact numeric
  defaults were previously deferred pending evidence.
- **Resolution:** ordinary reads and bounded stream batches default to 10 seconds;
  validation, planning, and ordinary mutations default to 30 seconds. The normal
  maximum is 30 seconds. Explicit registry/install jobs may declare a 300-second
  maximum while retaining a 30-second admission default. These define timeout
  behavior, not responsiveness targets.

### D-005 — UCUM/QUDT source custody and licensing

- **Status:** source identity resolved; redistribution review deferred
- **Blocks:** vendoring or shipping upstream datasets, not the bounded T1 engine
- **Question:** Where and how should verbatim UCUM 2.2 essence and QUDT 3.4.0
  reference inputs, checksums, notices, and attribution be stored/generated?
- **Known constraints:** UCUM is runtime authority; QUDT is design/generation
  reference only; UCUM redistribution requires preserved notices and legal
  review; QUDT is CC BY 4.0.
- **Resolution:** pin UCUM 2.2 commit
  `d060c6d3dfe3b0deb196c190b8024d95385e0b92`, essence SHA-256
  `dfccea1b5dc284245ebae97edd1dc03c45864da4e87df55bc9851797b4fd0b61`,
  and license SHA-256
  `b7989c84c68b7a80d42aad28cfdd824d756c2f70f26eaf1347d2e49f7838bbdf`.
  Pin QUDT 3.4.0 commit `1137205617d03d3d5c8351ea58105b6719c5d6f0`
  and release-asset SHA-256
  `26aab21644a6531abec1e0285a3907a33a553da7d78d0675ffa323cbab800a95`.
  Record both in the quantity package but vendor neither. Legal/attribution review
  remains mandatory before any upstream data enters a release bundle.

### D-006 — T1 signed-release metadata boundary

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Which typed metadata is produced in T1 versus merely accepted as
  signed release inputs owned by T5/T5b?
- **Known constraints:** T1 must remove zero-length/post-build placeholders and
  expose typed runtime metadata; T1 cannot implement registry publication,
  signing custody, or real signatures.
- **Resolution:** T1 owns bounded synchronous `RuntimeMetadataV1` and an explicit
  `UnsignedReleaseInputV1`. The latter has no signature, publication target, key,
  or verification claim. Final blob size/digest verification, provenance/SBOM
  binding, signing, and publication remain T5/T5b.

### D-007 — Commit-hook execution without protected project config

- **Status:** resolved
- **Blocks:** nothing
- **Question:** How should repository hooks run canonical `mise exec` checks
  without loading protected `.mise.toml`?
- **Known constraints:** commits are now explicitly requested; hooks must not be
  bypassed silently; Rust nightly and `cargo:taplo-cli@0.10.0` are already
  installed through Mise; protected project config must not be inspected or
  used.
- **Resolution:** run the hook-equivalent Rust/Taplo commands through
  `mise --no-config` in the isolated clean tranche worktree, inspect for formatter
  drift, then commit with hooks skipped only to prevent whole-workspace formatters
  from mutating unrelated dirty work. The user separately authorized disabling
  GPG signing, so tranche commits use `git -c commit.gpgsign=false commit`.

### D-008 — Local WIT lock registry label

- **Status:** resolved for T1; replaced by T5 publication locks
- **Blocks:** nothing
- **Question:** Standard `wkg.lock` requires a registry field even though T1 uses
  only reviewed local path overrides and has no publication authority.
- **Resolution:** use the reserved non-resolving label
  `local.litehouse.invalid` while each `wkg.toml` points to an exact local WIT
  path. Lock entries bind the generated ABI SHA-256 and never trigger network
  resolution. T5 must regenerate release locks against the approved R2 registry
  identity before publication.

### D-009 — Same-major additive WIT evolution

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Adding a required function to a provider-exported WIT interface
  breaks existing providers even when prose calls the operation optional.
- **Resolution:** every v1 base-interface operation is declared up front and
  feature-gated. A future same-major minor may add a separate extension interface
  and extension world while leaving the baseline provider/consumer worlds
  structurally compatible. It may not add a required field/function to a baseline
  interface. The compatibility fixture proves the unchanged baseline world passes
  and a changed required signature fails.

### D-010 — Pre-1.0 plugin package identity

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Existing fixture and provider crates use SemVer 0.x, while published
  contract requirements must route on a non-zero exact major.
- **Resolution:** runtime artifact identity may faithfully carry a 0.x plugin
  package version. Contract and runtime-profile requirements still reject major
  zero, and every T1 contract artifact remains exactly 1.0.0. Plugin package
  version is not a capability-contract routing identity.

### D-011 — Management-to-service projection authority

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Safe management JSON is not byte-for-byte identical to WIT
  service records where it flattens variants, injects facet generation, converts
  Celsius to canonical quantities, expands notification templates, or maps
  provider-trapped to a safe public error.
- **Resolution:** each capability row now carries one closed `service_binding`
  naming its exact package, interface, function, and reviewed projection. The
  manifest validator freezes all 29 mappings and a resolver-backed test proves
  every target function exists. The manifest remains the management adapter
  authority; WIT remains the service ABI authority. Unnamed or generic
  projection is forbidden.

### D-012 — Runtime metadata transport and admission

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Returning an unconstrained typed WIT record let an untrusted
  component allocate nested metadata before host validation.
- **Resolution:** `get-metadata` returns the versioned `LHMETA01` byte envelope.
  Hosts enforce a ten-second call deadline, a 64 KiB decode ceiling, nested
  identity/name/origin/config bounds, exact protected-field tokens, and schema
  pointer existence before consuming fields. Component imports are structurally
  decoded and checked against the exact runtime-profile package/version/interface
  allowlist in CI, `litehouse build`, and runtime admission. T3 still owns the
  general per-store memory/resource limiter and cancellation model.

### D-013 — Contract source-bundle trust root

- **Status:** resolved
- **Blocks:** nothing
- **Question:** A reproducible tar stream is insufficient if package discovery
  can follow a symlink outside the reviewed contract tree.
- **Resolution:** package and version discovery reject symlinks and non-directory
  entries. Bundling receives the separately trusted `packages/` root, rejects
  symlinked roots and ancestors, proves canonical containment, and rejects every
  descendant symlink and non-regular entry. Tests cover package-root and
  version-root escapes, descendant escapes, and a Unix socket entry.

### D-014 — T2 kernel and store-resource boundary

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Which production layer owns binding policy without retaining a
  provider guest resource in consumer or registry state?
- **Resolution:** `litehouse-service-fabric` is a Wasmtime-independent kernel.
  Its registry and proxy records contain stable endpoint metadata and host-owned
  numeric proxy IDs only. Generated adapters retain provider `ResourceAny`
  values inside the single provider-store actor and consumer resource tables
  retain only a typed `ProxyId`; actor messages carry typed dispatch records and
  never a Wasmtime resource, table index, instance, or store.

### D-015 — T2 selector ordering and snapshot semantics

- **Status:** resolved
- **Blocks:** nothing
- **Question:** What ordering is canonical for `many`, and when may wildcard
  membership change?
- **Resolution:** order by stable endpoint identity: provider-instance ID, then
  facet key, then exact contract namespace/name/major. `one` and `optional`
  reject more than one authorized match; `many` is the complete authorized
  ordered snapshot. Wildcard grants authorize a later explicit resolution but
  never mutate an existing binding.

### D-016 — T2 authorization, cycle, and rebind failures

- **Status:** resolved
- **Blocks:** nothing
- **Question:** How should the kernel preserve the closed T1 shared error model
  where it has no cycle-specific variant?
- **Resolution:** one concrete default-deny `ServiceAuthorizer::authorize` seam
  checks every bind, rebind, and invocation at exact operation and grant
  revision. Each grant set is associated with one principal and every service
  grant explicitly names its permitted actions; a missing set or principal,
  action, resource, or operation mismatch cannot inherit authority. Cycle
  rejection is a host-synthesized `execution(unsupported)`/`never` error; it
  does not extend `litehouse:errors@1`. Never-rebind returns stale/never.
  Same-identity rebind looks up the original identity only, but retains and
  rechecks the complete original selector and every bound operation against the
  current descriptor before grant checks or proxy allocation. It never reruns a
  wildcard selector against other identities.

### D-017 — T2 provider lifecycle and fixture ABI

- **Status:** resolved
- **Blocks:** nothing
- **Question:** How much lifecycle and WIT surface belongs in T2 rather than T3?
- **Resolution:** T2 fences active provider generation, facet incarnation, and
  grant revision; an explicit endpoint withdrawal must present the withdrawing
  `InstanceGeneration` and expected facet incarnation, so stale lifecycle work
  returns a `LifecycleError` without removing a newer descriptor. A trap
  withdraws endpoints and reports provider-trapped until a higher generation
  activates, after which old proxies are stale. Restart policy, deadlines,
  cancellation, admission/resource limits, and durable generation allocation
  remain T3. The real provider/consumer fixture WIT is explicitly test-only and
  contains fixed typed query/set/rebind functions—no operation strings, generic
  payload, reflection, or dynamic call escape hatch.

### D-018 — T2 live cycle-edge bookkeeping

- **Status:** resolved
- **Blocks:** nothing
- **Question:** How should a `many` binding contribute dependency edges after
  individual proxies are dropped, including repeated facets from one provider?
- **Resolution:** each binding maintains a live proxy refcount per provider.
  Dropping a proxy decrements only its provider; the dependency edge disappears
  when that provider's last proxy drops, while other provider edges in the same
  binding remain live. Cycle reachability uses an explicit iterative worklist and
  visited set rather than recursion, preserving behavior for deeply nested
  service graphs without consuming call stack per edge.

### D-019 — Empty optional/many binding ownership

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Successful `optional` or `many` resolution may select no endpoint,
  leaving no guest proxy whose drop hook could release a retained binding.
- **Resolution:** empty snapshots carry no `BindingId` and create no binding or
  cycle-graph record. Only non-empty snapshots retain host binding state. A
  repeated-miss regression test proves optional/many resolution leaves the
  active-binding count at zero.

### D-020 — T3 preserves the existing runtime

- **Status:** user-selected
- **Blocks:** nothing
- **Question:** Should T3 replace Litehouse's runtime/compiler/cache with a new
  policy kernel and compiler sandbox?
- **Resolution:** no. Preserve the existing Wasmtime engine, in-process
  compilation, incremental cache, store strategies, capability hooks, event loop,
  and restart path. T3 applies only the focused defect fixes in issue 38. There is
  no compiler sandbox, custom cache/DNS, component-size ceiling, global activation
  journal, host-restore protocol, or arbitrary resource-limit system. T3 does not
  create inert T2 lifecycle bookkeeping; real endpoint publication and proxy
  fencing begin with a real T6 provider.

### D-021 — T9 first-release container boundary

- **Status:** user-selected
- **Blocks:** nothing
- **Question:** Which portability and rollback machinery belongs in the first
  container tranche?
- **Resolution:** keep a conventional non-root amd64/arm64 image, one data
  directory, health/readiness/shutdown, basic digest/smoke evidence, and a small
  Butane/Quadlet generator. Remove cross-version restore, receipt watermarks,
  N-1 migration rollback, and prescribed CPU/memory/PID/tmpfs limits. Protected
  signing/publication qualification remains later T15 work.

### D-022 — T11 basic direct WebRTC RPC

- **Status:** user-selected
- **Blocks:** nothing
- **Question:** What transport does the web UI need over broker-facilitated
  WebRTC?
- **Resolution:** the broker performs signaling only. Browser and Litehouse use
  one direct reliable ordered data channel carrying standard JSON-RPC 2.0. The
  first `session.open` authenticates a short-lived T10 grant; selected methods
  call T8 handlers; notifications update/invalidate UI state; reconnect creates a
  fresh session and reloads state. No custom binary/credit/transfer/resume/
  reauthorization protocol or TURN belongs to T11.

### D-023 — T13/T14/T15 release scope

- **Status:** user-selected
- **Blocks:** exact T13 tool list only
- **Question:** How much MCP and external release qualification belongs in the
  first connected release?
- **Resolution:** T13 exposes a small explicitly approved subset of safe
  inspection, state, and essential-control tools rather than one tool per
  operation. T14 remains the isolated local release-candidate proof. T15
  provider-backed staging/signing/cloud qualification is later work requiring a
  separate decision and per-action external approval; it does not block T14.

### D-024 — First server-side Rule JSON tranche

- **Status:** user-authorized
- **Blocks:** nothing
- **Question:** How much of T7 can establish a useful authoritative server seam
  without prematurely selecting CEL/JCS/browser/runtime dependencies?
- **Resolution:** publish a closed Draft 2020-12 Rust/schema subset for one state-
  observation trigger, typed immutable snapshots, Boolean/string/i64/quantity
  comparisons, explicit four-outcome branching, exact On/Off set intents, typed
  ends, deterministic DAG compilation, and pure transition planning. Use the
  existing quantity core. Defer browser/Wasm, CEL, JCS semantic digests, selector
  resolution, persistence, dispatch, receipts, and timers. In particular, do not
  claim sorted JSON is RFC 8785 when no audited JCS dependency is already present.

### D-025 — Restart-clean management and automation

- **Status:** user-authorized
- **Blocks:** T8
- **Question:** Which state must survive a Litehouse restart, and which workflow
  guarantees are proportionate for a home automation server rather than a
  business-critical workflow engine?
- **Resolution:** persist authored instance configuration, Rule JSON, immutable
  revisions, active pointers, grants, and enabled state atomically in Fjall.
  Rule runs, delays, debounce/throttle windows, trigger queues, pending effects,
  receipt continuations, stream cursors, and short-lived idempotency results are
  process-epoch-local. A crash discards unfinished work; Litehouse never resumes,
  catches up, compensates, or replays it. Time schedules continue at their next
  future occurrence. Rules arm only after their own dependencies rehydrate or
  become explicitly unavailable. An ambiguous provider call is outcome-unknown
  and is never automatically retried. Absolute commands remain preferred where
  supported, and accepted-for-delivery never becomes invented observed state.
  T8 still completes the designed Rule JSON V1 language, uses typed provider push
  plus bounded polling, and adopts one canonical management core, but it does not
  add a durable workflow journal, outbox, or lifetime idempotency tombstones.

### D-026 — T8 implementation and dependency locks

- **Status:** user-authorized
- **Blocks:** nothing
- **Question:** Which concrete storage, canonicalization, CEL, timezone, provider-
  observation, and management-contract seams implement D-025 without reopening
  the provider ABI or publishing misleading V1 operations?
- **Resolution:** revise the unreleased management V1 catalog in place and remove
  top-level CEL/manual triggering. Rule source is JSON-only and bounded to 128
  KiB; provider-safe config is bounded JSON checked against exact component
  metadata with closed authority grants; run/result/stream cursors carry a fresh
  process epoch. Standardize on Fjall 3.1.7 `SingleWriterTxDatabase` in a fresh
  V3 directory, one `SyncAll` cross-keyspace transaction per authored mutation,
  its exclusive path lock, and a separate Litehouse application-format marker.
  Use `serde_json_canonicalizer` 0.3.2 for JCS after bounded JSON validation.
  Litehouse-authored JSON has unique keys; unsupported external duplicates use
  `serde_json`'s deterministic last-value behavior rather than a second parser.
  Use the `cel` 0.14.0 bounded parser and interpreter with immutable variables,
  a Litehouse-owned AST/type/reference allowlist and static cost cap, and no I/O
  functions. Use Jiff 0.2.32 plus bundled `jiff-tzdb` 0.1.8 (`2026c`) for explicit
  DST policy. Keep `litehouse:provider@1.0.0` unchanged:
  the actor performs bounded typed query sweeps, coalescing ESPHome's pushed
  Native API frames and allowing Tasmota/Open-Meteo to poll only when due. Guest
  handles remain confined to their actor/store and seven stateful families use
  seven exact typed ingress paths.
- **Evidence:** Fjall 3.1.7 committed `SyncAll` writes across two keyspaces
  survived immediate abort, uncommitted writes did not, and a second process was
  rejected with `Locked`. CEL-parser/JCS/Jiff probes compiled natively, for
  `wasm32-wasip2`, and for `wasm32-unknown-unknown` using the installed Rust
  sources; the RFC 8785 number/string vector and New York DST gap/fold rejection
  passed. The disposable release browser-Wasm probe was 1,765,990 bytes before
  application-specific stripping or binding optimization.

## Deferred external-authority decisions

### X-001 — T15 staging/signing/cloud/DNS operations

- **Status:** deliberately deferred
- **Blocks:** external portions of T15 only
- **Decision needed:** separately approve each named signing, staging R2/OCI,
  D1/Worker/DO, IAM, Logpush, email/TURN, route/DNS, rollback, and cleanup action
  after T14 passes.
- **Current authority:** local implementation and fixtures only. No generic
  instruction to work through tranches grants these external mutations.

### X-002 — Real provider/server/hardware evidence

- **Status:** deliberately deferred
- **Blocks:** any tranche acceptance item that explicitly requires supervised
  external evidence
- **Decision needed:** approve the exact endpoint/device/server command and its
  rollback before execution.
- **Current authority:** deterministic local fixtures and previously captured
  evidence only.

## Resolved during implementation

### R-001 — T0 FlatBuffers generation ownership

- **Resolution:** generate `registry_generated.rs` into Cargo `OUT_DIR`; delete
  tracked nested-target output; pin and checksum-verify `flatc 25.12.19`.
- **Evidence:** deterministic full/source baseline checks, clean registry build,
  release build, and final correctness/risk PASS.

### R-002 — T0 CI authority

- **Resolution:** ordinary workflows are read-only and non-publishing; release
  and legacy N-API workflows are manual guards; all external actions use locked
  full commit SHAs; workflow, policy, and installer bytes are digest-locked.
- **Evidence:** T0 baseline checks and final correctness/risk PASS.

### R-003 — T4 isolated frontend capability gate

- **Resolution:** keep `web/` as a self-contained TanStack Start and Cloudflare
  Worker package with its own pnpm workspace and lockfile. Render the nine V1
  exact-major capability facets from local fixtures only, and keep legacy
  `site/`, authentication, transport, persistence, and deployment integration
  outside T4. Consume Open Design only through the reviewed static bundle whose
  verified digest is
  `d9e80a366af1b97f58bbb79be34990f739b4292b66df0d500c01670282cb2c62`.
- **Evidence:** bundle-integrity verification, TypeScript checking, 13 jsdom
  behavior tests, a built-Worker workerd test, production build/prerender, and a
  Wrangler dry run under the pinned Node 24 and pnpm 10 toolchain.

### R-004 — ESPHome typed subresource feasibility

- **Resolution:** use the T2 Component Model resource pattern for provider
  facets. One configured provider component/store may return arbitrarily many
  instances of statically declared typed resources. Provider handles remain in
  that store; only handle-free endpoint descriptors and host-owned proxy IDs may
  enter the service fabric or consumer stores.
- **Evidence:** the disposable typed ESPHome provider spike returns two switch
  On/Off resources plus LED On/Off, Level, and Color resources from one guest.
  Wasmtime invokes all five opaque handles, observes shared LED state across
  facets, and publishes five handle-free service-fabric descriptors. The
  component validates with Component Model async features and the workspace
  passes Clippy with warnings denied.

### R-005 — Promote a three-tool MCP ESPHome proving slice

- **Resolution:** leave the existing partially working registry untouched and
  defer T5/T5b. Combine the first ESPHome production slice with an internal MCP
  proving adapter exposing exactly `litehouse_inventory`, `litehouse_query`, and
  On/Off-only `litehouse_interact`. MCP calls canonical handlers, which dispatch
  through the production provider host adapter to store-local typed guest
  resources. The proof uses in-process `rmcp` transport; authenticated stateful
  Streamable HTTP remains T13 after T9/T10. The supervised device mutation is
  limited to an absolute Relay 1 set, observation, and restoration.
- **Evidence required:** deterministic provider/Noise/host/MCP tests, exact
  three-tool inventory, real component validation, release Litehouse build, and
  a redacted supervised inventory/query/set/query/restore/query transcript with
  successful restoration.

### R-006 — Keep provider execution imports type-only

- **Resolution:** split the On/Off and Boolean packages into execution-free
  `types` interfaces plus their existing `capability` interfaces. The provider
  runner reuses only those exact records, so a provider component does not gain
  consumer capability imports or a reverse invocation path. The production host
  retains all guest resources in one bounded actor/store, supplies connect-only
  exact TCP grants, and treats host generation as authoritative.
- **Evidence:** contract tests reject executable functions in the two `types`
  interfaces; a real `wasm32-wasip2` fixture passes provider admission and the
  production Wasmtime adapter proves typed discovery, query, absolute set,
  generation fencing, accepted-delivery receipt projection, and endpoint
  withdrawal on actor shutdown.

### R-007 — Run ESPHome client work on one guest-local runtime

- **Resolution:** one ESPHome component runner owns one current-thread Tokio
  runtime, encrypted Native API client, topology, and observation cache. Every
  facet shares it, while the host provider actor remains the external ordered
  queue and hard deadline boundary. Reconnect preserves numeric source-locator
  facet keys; absolute writes do not update cached state and a failed write is
  outcome-unknown without replay.
- **Evidence:** the compiled `wasm32-wasip2` component passes runtime-profile and
  Component Model validation. A fixed-key Noise server exercised the actual
  component through Wasmtime and exact TCP grants: discovery returned one
  On/Off and one Boolean resource, query observed the initial relay state,
  absolute set returned accepted-for-delivery, and a later query observed the
  server-emitted state.

### R-008 — Keep MCP behind three canonical management handlers

- **Resolution:** the proving principal carries only `litehouse:read` and
  `litehouse:operate`. One fixed-scope authorization seam guards the typed
  `inventory_snapshot`, `facet_query`, and `on_off_set` handlers before lookup
  or provider invocation. A bounded, provider-generation-local result cache
  supplies the set operation's required host idempotency, including stable
  replay of outcome-unknown errors without reissuing a device command. The
  in-process `rmcp` adapter publishes exactly `litehouse_inventory`,
  `litehouse_query`, and `litehouse_interact`, using the checked contract
  projection schemas and safe structured errors. It has no
  generic operation dispatcher and no independent policy.
- **Evidence:** an in-process MCP client listed exactly three tools with schemas
  equal to the checked manifest, then exercised inventory, query, absolute set,
  and later observation through the production provider actor. A full fixed-key
  Noise fixture separately proved ESPHome Native API → Wasm component →
  store-local host → canonical handlers → MCP, including absolute restore and
  restoration observation.

### R-009 — Accept the supervised Relay 1 vertical proof

- **Resolution:** the approved live mutation remained limited to the unique
  `Relay 1` On/Off facet. The proof used only the three in-process MCP tools,
  changed the observed absolute state from Off to On, observed that state, sent
  the absolute original Off state, and observed restoration. Smart Bulb Mode,
  Relay 2, Relay 3, and every other entity were excluded by exact label and
  contract selection.
- **Evidence:** the production component and runtime profile were revalidated
  immediately before the proof. Inventory selected facet
  `switch-0-1878979320` at generation 1; the initial observation was Off,
  accepted-for-delivery change and restore receipts were returned, the changed
  observation reached revision 2, and the restored Off observation reached
  revision 3. The proof process exited successfully and retained no credential
  file or child process.

### R-010 — Complete provider v1 in place before external release

- **Resolution:** complete the unreleased `litehouse:provider@1.0.0` execution
  ABI in bounded additive phases rather than maintaining a second pre-release
  ABI. Measurement and Notification Sink now use execution-free `types`
  interfaces and repeated typed guest resources; no generic facet or operation
  envelope is introduced. Existing ESPHome and fixture components are rebuilt
  against the completed world. The host retains tags as structured metadata and
  grants HTTP only by exact scheme, host, and port.
- **Evidence:** deterministic component tests return On/Off, Boolean,
  Measurement, and Notification Sink resources from one store; the production
  host queries typed quantities, sends a typed notification, fences stale
  generations, withdraws endpoints, and still passes the full encrypted
  ESPHome Noise → Wasmtime → canonical handler → MCP regression path.

### R-011 — Close the nine-family provider ABI without protocol expansion

- **Resolution:** complete the same unreleased `litehouse:provider@1.0.0` ABI
  with repeated Level Control, Color Control, Thermostat, Window Covering, and
  Momentary Action resources. Each canonical capability now separates
  execution-free `types` from executable `capability`; the provider imports only
  `types`, exposes every typed command except subscriptions, and has no generic
  payload or dispatch envelope. ESPHome supplies uninhabited placeholders only;
  protocol mappings remain deferred.
- **Evidence:** deterministic fixture resources cover query and representative
  commands for all five families through the production store-local Wasmtime
  actor. Both fixture and ESPHome components pass Component Model and runtime
  profile admission, while the encrypted ESPHome set/observe/restore and MCP
  regressions remain green without hardware or secret access.

### R-012 — Establish the closed Rule JSON planner seam

- **Resolution:** `litehouse-rule-json` owns the strict wire structs, generated
  exact-release schema, generated port catalog, semantic DAG compiler, immutable
  evaluation input, typed effect-intent plan, and assumption recheck. Unknown
  fields/kinds/versions and binary floats fail closed. Unknown and evaluator
  error remain separate required branch ports. Transition planning has no
  provider handle, mutation API, persistence, timer, or arbitrary-code seam.
- **Evidence:** checked schema/catalog regeneration and Draft 2020-12 validation;
  deterministic compile/plan tests for Boolean/string/i64/quantity comparison;
  explicit false/unknown/stale/error routes; graph, cycle, type, order, bound, and
  unknown-field rejection; immutable-input proof; and stale-assumption rejection.

### R-013 — Keep interim MCP HTTP explicitly reachability-authorized

- **Resolution:** T13a mounts only the existing three-tool `McpAdapter` at
  `/mcp` over stateful Streamable HTTP. The bind address, allowed hosts, and
  exact origins are explicit; wildcard authority is rejected. There is no
  authentication in this tranche: any reachable client receives the fixed
  adapter principal. One adapter factory remains the sole later-auth seam, and
  startup emits a prominent structured security warning.
- **Evidence:** deterministic network transport tests initialize an MCP client,
  list exactly the three closed tools, call one tool, reject an unknown tool,
  enforce session capacity and origin policy, and prove bounded graceful
  shutdown.

### R-014 — Enforce the four-provider first-party release portfolio

- **Resolution:** the published provider group is exactly ESPHome, Tasmota HTTP,
  ntfy, and Open-Meteo. `noop` and `persistent-task` remain development assets;
  Samsung remains deferred outside the release group. Empty MQTT, Presence,
  Sonoff, and legacy Weather shells are removed rather than remapped to broader
  authority.
- **Evidence:** Cargo workspace metadata names only the four production provider
  paths in the release group. Tasmota, ntfy, and Open-Meteo pass deterministic
  protocol tests and Component Model/runtime-profile admission without live
  service or hardware access.

### R-015 — Map ESPHome by advertised Native API features

- **Resolution:** one encrypted API 1.14 session now projects switches, binary
  sensors, whitelisted numeric sensors, lights, fans, sirens, climates, covers,
  and buttons only into contracts their discovery metadata can support. Facets
  from one entity share a stable source locator. Commands remain absolute;
  unsupported step/move/toggle semantics and unknown units/entities become
  bounded diagnostics rather than invented capabilities.
- **Evidence:** deterministic API 1.14 Noise fixtures cover discovery, hydration,
  feature-gated descriptors, typed queries, every mapped command family, bounded
  unsupported diagnostics, disconnect gaps, topology reincarnation, and the
  existing switch set/observe/restore regression. The expanded component passes
  Clippy, Component Model validation, and provider runtime-profile admission.

### R-016 — Complete unreleased Rule JSON V1 before executor work

- **Resolution:** replace the T7 wire subset in place with the exact three
  triggers and eleven post-trigger V1 families. Rust owns closed typed selectors,
  WIT events/state fields, values, schedules, effects, desired paths, exact
  operation-parameterized receipts, editor data, policies, contract/WIT and other artifact locks,
  limits, and all 29 capability operations. Effect planning suspends before a
  typed receipt; desired path/value/target and retry/deduplication/deadline rules
  fail closed before compatibility lowering. The T7 planner survives only for
  supported comparisons and literal absolute On/Off set intents; other valid
  families return precise subset diagnostics until T8.1b/c. Concrete provider
  generations and facet/grant revisions remain activation-record data, not
  authored JSON.
- **Evidence:** the generator byte-checks the Draft 2020-12 schema, 14 catalog
  entries, all manifest argument schemas and WIT service projections, exact
  event/state/desired mappings, process-epoch state, and coherent minimal,
  maximal, family, operation, schedule, and invalid fixtures. Targeted tests
  cover receipt wiring, selector/contract ordering, all three retained trigger
  contexts, fail-closed retry, and narrow model bounds. RFC 8785 semantic digest
  and the exact 64-KiB canonical bound remain T8.1b #64; executor receipts/timers
  and persisted activation revisions remain T8.1c/T8.2.
  Native checks pass without changing provider WIT. The immediately preceding
  repair passed `wasm32-wasip2`; the final local rerun is blocked because the
  synchronized toolchain no longer has that optional target component installed.

### R-017 — Compile Rule JSON semantics through pinned engines

- **Resolution:** T8.1b hashes only the RFC 8785 JCS projection of schema locks
  plus semantic content, sorts non-semantic arrays by stable identity, and
  enforces its 64-KiB bound. `cel` 0.14 supplies the bounded dynamic interpreter;
  Litehouse admits a closed AST/reference/type profile, precharges static cost,
  accounts checked/runtime-value memory, checks elapsed deadline, injects only
  declared immutable values and `evaluation_time`, and exposes no I/O functions.
  Chrono is confined to CEL timestamps. All quantity
  node operations are validated and closed through the existing exact quantity
  engine. Jiff 0.2.32 and bundled tzdb 2026c compile one-shot, fixed-interval,
  and weekly civil schedules while retaining explicit gap/fold policy.
- **Evidence:** digest tests prove editor/revision exclusion and semantic change
  sensitivity; CEL tests cover timestamp evaluation, undeclared references,
  dynamic-language rejection, type-faithful integers, and quantity exclusion;
  quantity unit/arithmetic/result-profile tests call the shared engine; schedule
  tests compile all three forms and reject an unavailable timezone.

### R-018 — Replace planner compatibility forms in place

- **Resolution:** T8.1c replaces the unreleased planner API rather than preserving
  a wrapper. Its first bounded phase removes compiler subset diagnostics and
  retains closed compiled forms for all three triggers, all eleven node families,
  all 29 typed effects, schedules, quantities, desired patches, admission,
  fork/join, delays, receipts, retry, and every terminal outcome. Persistence,
  provider dispatch, and workflow recovery remain outside the pure planner.
- **Evidence:** every checked valid fixture now compiles completely; selector,
  graph, port, retry, receipt, deadline, quantity, CEL, and schedule failures
  remain typed compile diagnostics.

### R-019 — Complete pure restart-clean planner parity

- **Resolution:** one deterministic `plan_microstep` interpreter now evaluates
  every V1 trigger and post-trigger family from immutable `initial`, `timer`, or
  `effect_receipt` input. Opaque constructor-validated process state retains exact
  continuations, trigger queues, admission windows, fork arrivals, effect history,
  and timer/receipt identities. Admission serializes debounce/throttle winners.
  Forks execute two to four branches in declared order; joins preserve arrivals
  across microsteps, reject same-target parallel effects before dispatch, and
  merge desired projections with the authored conflict policy. Nested fork regions
  fail compilation in V1. Plans may contain typed desired projections, effects,
  timers, assumptions, traces, and next state, but never persistence, dispatch,
  live guest handles, restart recovery, or observed-state mutation.
- **Evidence:** deterministic tests cover sequential execution, all operation
  argument families, exact quantities and schedules, receipt/timer mismatch,
  admission queues, debounce/throttle, immediate and suspended forks, terminal
  joins, desired/effect conflicts, state forgery, and configured bounds. Repeated
  adversarial reviews drove exact saved-state validation, reachability-based nested
  fork rejection, selector-identity effect checks, and pre-fork history scoping;
  the final review passed. Package/dependency/management tests, generator parity,
  native all-target checking, Rustfmt, Taplo, diagnostics, full T0 validation, and
  secret scanning complete the local evidence seam.
