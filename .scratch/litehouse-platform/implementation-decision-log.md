# Implementation decision log

Updated: 2026-07-17
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

### D-018 — Empty optional/many binding ownership

- **Status:** resolved
- **Blocks:** nothing
- **Question:** Successful `optional` or `many` resolution may select no endpoint,
  leaving no guest proxy whose drop hook could release a retained binding.
- **Resolution:** empty snapshots carry no `BindingId` and create no binding or
  cycle-graph record. Only non-empty snapshots retain host binding state. A
  repeated-miss regression test proves optional/many resolution leaves the
  active-binding count at zero.

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
