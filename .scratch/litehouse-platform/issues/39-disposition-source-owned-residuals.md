# Disposition source-owned residuals

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-16
Blocked by: 24, 27, 33, 38
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which verified source-owned residuals are implemented, deleted, explicitly
deferred with an owner/trigger, or ruled out—and what evidence closes each
choice? The inventory must include:

- JSON-Schema keyword paths in `crates/litehouse/src/util.rs`;
- registry insertion and temp-file safety in
  `crates/registry/src/partitioned_store.rs` and
  `crates/registry/src/partition.rs`;
- blocking reads in `crates/opendal-fs-cache/src/lib.rs`;
- legacy HTTP middleware in `crates/plugin/src/http.rs`;
- metadata injection in `crates/plugin-macro/src/lib.rs`;
- pairing validation in `crates/cockpit/src/client.rs`;
- the admin check in `site/components/toolbar.tsx`;
- memory markers in `crates/litehouse/src/runtime.rs`,
  `crates/litehouse/src/server.rs`, and `crates/litehouse/src/cache.rs`;
- the WASI compatibility bridge, plugin skeletons, and migration residuals.

## Answer

Do not complete placeholders merely because they exist. Assign each residual to
one approved production tranche, delete superseded paths with their replacement,
and keep performance estimates, legacy UI/auth, alternate registry clients, and
deferred providers out of V1. No residual may survive as a reachable `todo!()`,
ambient authority, panic path, fake implementation, or undocumented release
exception.

This inventory is planning disposition only. It authorizes no code deletion,
credential access/remediation, publication, deployment, server/hardware access,
or commit.

## Required implementation or replacement

| Residual | Disposition | Owning tranche and exact closure evidence |
|---|---|---|
| `util.rs` `PathChunk::Keyword(_) => todo!()` | **Implement fail-closed.** `resolve_pointer` returns `None` for a keyword in an instance path; it must not invent a source location or panic. | **T0 only.** Unit/property fixtures cover root, escaped property, valid/invalid index, missing node, keyword in every position, and malformed/deep bounded input. CLI schema diagnostics remain typed and safe when no span exists. |
| Current runtime event payload forwarding and output-discovery rejection | **Retain, do not reimplement.** | **T3 only.** Regression fixtures prove subscribed payload/type forwarding, nonzero event identity/time, explicit lag failure rather than silent continuation, and output-discovery rejection. |
| Runtime/store/server/cache resilience seams | **Preserve the existing runtime and apply issue 38's focused cache, deadline, continuity, trap-isolation, capability, protected-value, and panic-path fixes.** | **T3 only.** Focused regressions prove corrupt cache rebuild, CPU/I/O deadline interruption, subscribed continuity loss, shared-store sibling preservation, per-instance grants with unique IDs, protected-value non-persistence, and ordinary errors without replacing the compiler/cache/runtime. Real T2 endpoint publication waits for T6. |
| Plugin metadata placeholder/static zero-length section | **Replace, not inject post-build.** Runtime/config metadata comes from generated typed contracts; immutable release metadata, permissions, provenance, and exact locks come from the signed release record. | **T1 only.** Reproducible component bytes, WIT/import validation, signed payload vectors, native/Wasm metadata parity, and a scan proving no mutable post-build metadata injection. |
| Current anonymous pairing password path | **Delete with the legacy Cockpit pairing route.** Do not validate or preserve the static Basic/password protocol. | **T10 only.** Key-proof pairing, pair epochs, scoped short grants, crash/replay fixtures, and a route/import scan prove the old handler is unreachable/removed before T11 adds new signaling. |

## Delete with the owning replacement

### Registry implementation

Delete the old local mutable registry implementation—including
`Index::insert`/unfinished lookup, fixed `index-new.bin`, in-place mmap rewrite,
unsafe lifetime extension, legacy FlatBuffer schema/generated bindings, and
hard-coded partition assumptions—in **T5 only**, when the issue-24/46 R2 registry
replaces it.
Do not repair that code into a second backend.

The replacement owns immutable object-first publication, seven versioned shards,
signed complete release records, ETag root CAS, bounded verified caches, one
Rust/native-Wasm query core, explicit activation plans, and retained predecessor
rollback. Closure requires malformed/concurrent/offline/crash vectors and a source
scan proving the old writer/index are absent from production dependency paths.

### `opendal-fs-cache`

The cache layer is currently used only by the legacy registry paths and contains
an unimplemented blocking read. Delete the crate and both current dependency/
layer call sites in **T5 only** with the registry rewrite. The production registry cache has its
own exact verified-object and compatibility-cache contracts; it must not retain a
generic fallback layer, blocking adapter, S3 mode, or unreachable `todo!()`.

### Guest HTTP helper

Delete `crates/plugin/src/http.rs`, its legacy client/middleware dependency, and
all exports/callers in **T3 only**. Generated WASI HTTP bindings
plus issue 38's exact-origin host connector are the only V1 guest HTTP path. A
compiled malicious guest proves there is no environment, DNS, raw-socket,
redirect, proxy, or alternate-sender bypass, and a first-party HTTP fixture
completes a request without panic.

### Legacy web/admin source

Do not implement the toolbar admin TODO. **T12b only** retires the complete
legacy `site/` package after T12 greenfield parity; no earlier tranche edits or
imports it.
No legacy route, Better Auth implementation, state, transport, component, CSS,
or Turso/libSQL data is copied or migrated into `web/`. D1 starts clean. Closure
is a dependency/import/route/data-migration scan plus greenfield Worker and
browser journey evidence.

## Memory markers

The `TODO(MEM)` comments in runtime, server, and cache are neither V1 performance
requirements nor authorization for optimization work.

- Implement only issue 38's demonstrated correctness bounds: component/input,
  dynamic ledger, isolated compiler reservation, guest/resource, queue/I/O,
  cache decode/disk, activation, and shutdown limits.
- Remove each marker when its owning old allocation seam is replaced or covered
  by the bounded implementation and fixture. Do not leave stale numeric prose.
- Add no benchmark harness, startup/call-latency target, heap optimization,
  custom allocator, or product SLO.
- Future performance owner: the runtime maintainer. Trigger: a reproducible
  amd64/arm64 production-path regression or inability to satisfy the existing
  actual release behavior. Evidence may measure the implemented
  seam before proposing a threshold or optimization.

## WASI and generated-artifact migration

- Build Rust guests directly for `wasm32-wasip2` and validate native Component
  Model async exports/imports.
- Retain only the exact official WASI 0.2.12 `std` imports and WASI 0.3 interfaces
  declared by the signed runtime profile.
- Remove the Preview-1 reactor packaging path, stale adapter comments/assets, and
  any requirement to build plugins for `wasm32-wasip1` or wait for direct Rust
  `wasip3` `std`.
- Keep generated WIT/FlatBuffer files reproducible and untracked unless an
  approved generator contract explicitly requires checked-in bytes. Generated
  drift fails validation; hand-editing generated output is forbidden.
- The root Rust 2024 workspace baseline is implemented only from a clean reviewed
  worktree. Unrelated dirty edition/tooling changes are never absorbed.

Closure requires direct component build/validation/invocation, exact runtime-
profile import rejection, clean regeneration parity, and absence of Preview-1
adapter packaging from build/release paths.

## Plugin packages

| Package/path | Disposition |
|---|---|
| ESPHome | Implement as a separately signed V1 provider after contract/runtime/registry seams. |
| Tasmota HTTP | Replace the current shell with the narrow literal-private-IPv4 provider from issues 37/53. |
| Weather/Open-Meteo | Replace/rename deliberately with the four-measurement non-commercial attributed provider; withhold release unless every exposing surface carries adjacent attribution. |
| ntfy | Add as the only V1 notification sink; no generic webhook or Twilio fallback. |
| `noop`, `persistent-task` | Keep only as non-published conformance/development fixtures; release manifests reject them. |
| Samsung | Keep as deferred source evidence, excluded from V1 build/image/registry/readiness; re-entry requires its separately approved media contract and phase/cancellation proof. |
| MQTT, Presence, Sonoff, Twilio shells | Delete in a separately approved cleanup tranche after confirming no production consumer; do not publish, complete, or use them as generic escape hatches. |

A package's disposition is enforced by workspace/release-manifest tests, not only
documentation.

## Other migration residuals

- Retire the legacy registry N-API/JS path with the Rust/Wasm query wrapper; V1
  has no parallel query implementation.
- Preserve `.scratch` prototypes, historical hardware/R2 evidence, and the
  untracked greenfield `web/` gate as evidence only until their production
  tranches are approved. Production code must be reviewed rather than copied
  wholesale from a prototype.
- Keep Matter/Thread/Matter Server implementation outside the first connected
  release. Issues 54–57 are post-V1 planning, not hidden provider scope.
- The separate credential/history-remediation incident in issue 35 remains a
  release prerequisite and separate explicit authorization; no implementation
  tranche may inspect, print, migrate, or silently repair secret values.

## Validation and evidence

Before closing each owning implementation tranche:

1. run AST/source scans across production members proving no reachable `todo!()`/
   `unimplemented!()` or superseded import remains;
2. run diagnostics and targeted tests at the stable seam named above;
3. regenerate and compare contract/registry artifacts from pinned tools;
4. run malicious/invalid/crash fixtures for every removed panic or authority
   boundary;
5. verify package/release manifests include only the four V1 providers and omit
   development/deferred/deleted packages;
6. run the repository's canonical Rust checks and, for greenfield web changes,
   `pnpm --dir web check`; and
7. inspect final diff/status for unrelated work, generated blobs, binary drift,
   credentials, and accidental legacy-source reuse.

Implementation completion—not this disposition—requires zero blocking
correctness/security findings. A deferred item is closed only by its owner/trigger
above, not by leaving an executable placeholder.
