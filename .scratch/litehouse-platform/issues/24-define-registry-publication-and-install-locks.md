# Define registry publication and installation locks

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 06, 23, 45, 46
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 6

## Question

Which exact FlatBuffer root/shard/single-release-record and installation-lock
schemas, content-addressed R2 blob layout, dynamic bounded partition metadata,
cache manifests, Rust/Wasm APIs, object-first shard plus ETag-root publication
protocol, digest verification, conditional/resumable reads, exact bindings and
grants, reviewed update plan, activation transaction, offline cache, known-good
set, and rollback rules form v1?

## Answer

Keep v1 to three immutable FlatBuffer objects, one mutable root, the existing
combined JSON desired configuration, and one server-local activation record. Do
not build a second package manager: Cargo owns Rust dependencies; standard
`wasm-pkg`/`wkg.toml`/`wkg.lock` own WIT dependencies; Litehouse owns only its
static-registry adapter, release verification, and runtime activation.

### Registry schemas

`RegistryRoot` at `registry/index.bin` contains:

- root format, FlatBuffer schema, package-key normalization, and partition-
  scheme versions;
- diagnostic publication generation;
- a bounded data-only `AlphabeticalV1` scheme (`[Split::Seven]`);
- sorted shard descriptors containing partition ID and key bounds, immutable
  object path, SHA-256, byte length, and shard schema version; and
- no package records, signatures, URLs outside the configured origin, or
  executable registry-provided rules.

Package keys are canonical lowercase WIT-style `namespace:name`; SemVer remains
a canonical string so prerelease/build syntax does not require a schema change.
V1 partition selection uses the first canonical package-key byte and seven fixed
alphabet ranges. Readers validate descriptor bounds/coverage and fail closed on
unknown normalization, scheme, or schema versions. A future repartition emits a
complete new immutable descriptor set before switching the root.

Each immutable `ReleaseShard` is sorted by `(package-key, semantic-version)` and
contains complete `ReleaseRecord` entries. A record has:

1. deterministic signed `ReleasePayload` bytes plus SHA-256 release identity;
2. one or more signature envelopes over the payload digest; and
3. unsigned registry state.

The signed payload contains schema/signature-context versions, artifact kind
(`component`, `wit-package`, or `runtime-profile`), package/version/publisher,
primary and attachment blob descriptors, exact standard WIT dependency lock,
required runtime profile, provided/required contract requirements, maximum
requested WASI/service permissions, environment-secret field declarations,
license/source/provenance/SBOM references, and build metadata. Each blob
descriptor contains media type, SHA-256, and `u64` byte length. A V1 `component`
primary blob over issue 38's 16 MiB compile/input ceiling is rejected at
publication and install before transfer; attachments keep their separately
bounded media contracts and are never compiler input.

Registry state contains first-party review status, yank flag/reason/time,
withdrawal state, and optional revocation/tombstone references. Rewriting that
state produces a new immutable shard/root but never changes signed release
identity. Exact deterministic FlatBuffer payload bytes are the only signing
input; readers never reconstruct signing bytes from semantic fields.

The old `Summaries`, `Summary`, `Entry`, and fixed numeric partition filenames
are replaced rather than migrated in place. Generated bindings are reproducible
build output pinned to the same `flatc`/runtime version; deterministic JSON dumps
and vectors make every root/shard/release reviewable.

### Object layout and publication

Cloudflare R2 is the sole v1 storage backend; do not add an S3 fallback or
backend portability layer. Use only:

```text
registry/index.bin                         mutable root
registry/shards/sha256/<64-lower-hex>.bin immutable shards
blobs/sha256/<64-lower-hex>               immutable payloads
```

Immutable objects receive long-lived `immutable` CDN caching. The root requires
conditional revalidation and must not receive immutable caching. ETags are only
HTTP validators/CAS tokens, never content digests.

The first-party CI/CLI publisher:

1. builds twice and validates the release, standard `wkg.lock`, runtime imports,
   conformance assets, deterministic signed payload, and publisher signature;
2. uploads every missing blob with create-only `If-None-Match: *`; on an
   existing object it verifies length and SHA-256 rather than overwriting;
3. conditionally reads the current root and affected shard;
4. rejects an existing package/version unless its signed payload is identical;
5. deterministically inserts the record, validates the new shard, uploads it
   create-only under its SHA-256 path, and verifies stored bytes;
6. builds and validates the complete new root; and
7. writes `registry/index.bin` with `If-Match` against the prior ETag (or
   create-only for the first root).

A root precondition failure reloads the new head, reapplies the release, and
retries with a small bound/backoff. Uploaded losing shards/blobs are harmless
orphans collected later. Only successful root CAS makes a release visible.
Purge/revalidate the root and derived public web cache after CAS; never purge
immutable objects. Third parties never receive bucket credentials.

### Reads, cache, and Rust/Wasm core

One pure Rust core owns FlatBuffer verification, strict key/SemVer
normalization, partition lookup, deterministic release-payload digesting,
signature input, exact/caret resolution, standard-lock conversion, registry
state rules, and human-readable dumps. Native tools link it directly; browser
code uses the same core as Wasm.

I/O remains outside the core behind small environment adapters supporting
`HEAD`, conditional root `GET`, byte-range `GET`, and atomic cache put. Native
and browser adapters coalesce refreshes. The cache is content-addressed; its
small local index tracks origin/root ETag and digest, verified shard/blob
lengths, last refresh/access, and activation pin counts. It is GC bookkeeping,
not a trust authority.

Root refresh uses `If-None-Match`; `304` reuses the validated root, while `200`
validates the new root and fetches only missing shard digests. Exact lookup
loads one shard; browse/search loads all seven. Shards are small whole-object
reads. Large blobs download to a digest-named partial file using validated
`Range`/`Content-Range`; a `200` response restarts rather than appends. Only a
complete length and SHA-256 match is atomically promoted to the verified cache.

Offline operation continues from the last validated root/shards. Existing active
plugins always run. Exact install, restart, or rollback succeeds offline only
when every signed record and blob is verified locally; otherwise it fails
without altering the active set. Cached registry age is visible. A yank excludes
a release from new resolution, while an already active/locked release remains
restartable with a warning. Withdrawal/revocation activation policy is finalized
by **Define signing and trust**.

### Development and runtime locks

The existing `litehouse build` stays the one developer command. Before its
existing Cargo step, it uses standard wasm-pkg resolver/lock/materialization
libraries with the Litehouse registry-core adapter to update `wkg.lock` and
populate `wit/deps`. `generate!` consumes that local graph; Cargo builds directly
for `wasm32-wasip2`; `wasm-tools` validates the completed component and runtime-
profile imports. Do not implement custom `add`, SemVer solver, lock format, or
binding generator.

The server's combined JSON remains the user-visible desired configuration and
stores exact direct plugin versions. It does not need another user-managed
package lock. A published plugin's signed release already carries its exact
transitive standard lock graph.

The server stores one internal FlatBuffer `ActivationSet` containing:

- deterministic activation ID and predecessor ID;
- desired-config revision/digest;
- every configured instance ID and exact plugin release/component/runtime-
  profile digests;
- config digest (not raw config) and deterministic environment-secret
  requirement digest/status (never values and never a grant); and
- exact granted WASI/service operations/resources;
- resolved stable facet/binding identities and **activation-time generation**;
  and
- activation status/time plus safe failure IDs.

The activation-time generation is immutable historical evidence for that pointer,
not current routing authority. Routine trap/reconnect/process restart generations
are allocated from issue 38's separate durable per-instance runtime-generation
counter/table and do not rewrite `ActivationSet`. Current endpoint/proxy/event
routing always consults that runtime table.

`ActivationSet` represents the last successfully committed configured set, not
another configuration source or a live-process snapshot. Canonical ordering makes
equivalent validated input byte-identical.

### Explicit updates and rollback

The registry may display compatible updates, but never selects or activates one
automatically. A user/API update changes the exact version in combined JSON and
produces an ordinary review diff of release/dependency changes, requested-
permission deltas, runtime-profile requirement, config-schema compatibility,
yank/withdrawal status, and affected bindings. This is UI/API information—not a
second approval-token protocol.

Permissions never broaden from a version update. New requested authority must
already be present in the configured per-instance grant, otherwise activation
fails with a typed diff. V1 runs no arbitrary guest config migration; an
incompatible schema requires an explicit config edit.

After all bytes verify, Litehouse updates the complete affected-instance diff
using issue 38's one batch journal and exact no-grant-preflight, predecessor-
quiesce, granted-candidate, and single atomic `ActivationSet` pointer ordering.
There is no partial/mixed set. A disposable preflight may coexist with a
predecessor because it has no real grants, but old/candidate granted sessions for
the same instance never overlap. Any candidate failure or pre-commit crash drops
all candidates and restores every journaled predecessor while the old global
pointer remains authoritative.

Unrelated plugins continue running. In-flight operations retain the shared
durable dispatch/ambiguous-outcome semantics and are never blindly retried.
Failure leaves desired/active state visibly unresolved rather than claiming
success.

Pin the active activation's blobs plus exactly one successfully activated
predecessor. An explicit rollback performs the same affected-plugin restart in
reverse and works offline. Failed staging automatically keeps/restarts the old
version; that is transaction abort, not an unrequested rollback. Later runtime
health problems are reported/restarted under normal resilience policy and never
silently change the exact configured version. After the next successful update,
unreferenced history beyond the one predecessor is eligible for cache GC.
