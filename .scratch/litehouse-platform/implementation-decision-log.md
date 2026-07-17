# Implementation decision log

Updated: 2026-07-15
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

- **Status:** investigating
- **Blocks:** T1 implementation plan
- **Question:** Should the nine exact-major contracts and shared packages be
  represented as separate source-package directories immediately, or begin in
  one workspace crate that emits independently versioned WIT packages?
- **Known constraints:** releases are independently versioned; no separately
  published generated helper-crate matrix; code generation reads a locked local
  WIT graph; runtime routing keys on namespace/name/major.
- **Resolution evidence needed:** existing plugin-macro generation boundaries,
  wasm-pkg/wkg integration seams, and the smallest layout that can prove
  independent ABI digests without duplicating code.

### D-002 — Canonical management-operation manifest format

- **Status:** investigating
- **Blocks:** T1 implementation plan
- **Question:** Which tracked source format should own
  `management-operations-v1` so Rust, HTTP, WebRTC, web, and MCP adapters can be
  generated without introducing a second behavioral authority?
- **Known constraints:** the manifest is closed and versioned; each operation
  fixes scope, method/path, mutation/read class, idempotency, revision,
  deadlines, errors, redaction, projection, and streaming shape; no transport-
  only operation is allowed.
- **Resolution evidence needed:** current build-script/config-schema patterns and
  whether a typed Rust source, JSON Schema-validated JSON, or WIT-adjacent source
  produces the least duplicated authority.

### D-003 — Canonical decimal bounds

- **Status:** unresolved
- **Blocks:** quantity API finalization, not initial parser prototyping
- **Question:** What exact signed-coefficient width, scale range, input length,
  token count, nesting, exponent, precision, and diagnostic-span limits become
  the immutable quantity-profile v1 contract?
- **Known constraints:** values are canonical decimal strings; no binary float,
  implicit rounding, NaN/infinity, unknown result profile, or unbounded input;
  recurring results require explicit round-half-even quantization.
- **Resolution evidence needed:** representative home-quantity ranges, existing
  decimal dependencies, Wasm parity, denial-of-service fixtures, and compatibility
  consequences. These are correctness bounds, not performance SLOs.

### D-004 — Per-operation default deadlines in T1

- **Status:** unresolved
- **Blocks:** final operation metadata vectors
- **Question:** Which bounded default deadline is assigned to each generated
  operation while preserving the runtime hard ceiling?
- **Known constraints:** queue time counts; callers may shorten but not lengthen;
  ordinary provider operations have a 30-second host ceiling; exact numeric
  defaults were previously deferred pending evidence.
- **Resolution evidence needed:** choose conservative contract defaults from
  protocol semantics and fixtures without creating unsupported responsiveness
  SLOs.

### D-005 — UCUM/QUDT source custody and licensing

- **Status:** unresolved
- **Blocks:** shipping a quantity-profile source bundle, not local semantic tests
- **Question:** Where and how should verbatim UCUM 2.2 essence and QUDT 3.4.0
  reference inputs, checksums, notices, and attribution be stored/generated?
- **Known constraints:** UCUM is runtime authority; QUDT is design/generation
  reference only; UCUM redistribution requires preserved notices and legal
  review; QUDT is CC BY 4.0.
- **Resolution evidence needed:** reviewed upstream artifacts/checksums and an
  explicit license review before vendoring or release.

### D-006 — T1 signed-release metadata boundary

- **Status:** investigating
- **Blocks:** metadata placeholder replacement design
- **Question:** Which typed metadata is produced in T1 versus merely accepted as
  signed release inputs owned by T5/T5b?
- **Known constraints:** T1 must remove zero-length/post-build placeholders and
  expose typed runtime metadata; T1 cannot implement registry publication,
  signing custody, or real signatures.
- **Resolution evidence needed:** current plugin metadata embedding path and the
  minimum unsigned/test fixture structure that T5 can later bind without format
  duplication.

### D-007 — Commit-hook execution without protected project config

- **Status:** unresolved
- **Blocks:** committing T0 and subsequent tranches
- **Question:** How should repository hooks run canonical `mise exec` checks
  without loading protected `.mise.toml`?
- **Known constraints:** commits are now explicitly requested; hooks must not be
  bypassed silently; Rust nightly and `cargo:taplo-cli@0.10.0` are already
  installed through Mise; protected project config must not be inspected or
  used.
- **Proposed resolution:** invoke `git commit` with a temporary PATH-local `mise`
  wrapper that adds `--no-config` and activates the already-installed pinned
  Rust/Taplo tools, preserving the real hooks and checks.

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
