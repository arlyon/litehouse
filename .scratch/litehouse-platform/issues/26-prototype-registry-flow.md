# Prototype registry publish, install, and rollback

Type: prototype
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 23, 24, 25
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Does a disposable R2-only registry prototype prove content-addressed object-
first publication, immutable FlatBuffer shard upload, concurrent ETag-root
compare-and-swap, first-party signature/provenance verification, byte-identical
native Rust/Wasm queries, conditional/resumable persistent cache update, exact
installation-lock activation, permission/update-plan review, offline cache hit,
malformed/digest/conflict partial-failure safety, and known-good rollback without
a long-running registry database service?

## Answer

Yes. Keep the R2-only, static FlatBuffer architecture and implement it from the
isolated prototype in
[`../prototypes/registry-flow`](../prototypes/registry-flow/). Do not add a
registry database, S3 fallback, separate JavaScript query implementation, or
custom package manager.

### Protocol and trust evidence

The prototype implements one versioned mutable FlatBuffer root, seven immutable
alphabetical FlatBuffer shards, complete signed release records, and SHA-256
content-addressed blobs. Publication uploads immutable objects with create-only
writes and switches only the root with ETag `If-Match`. A bounded retry reloads
and reapplies after a precondition conflict, so two concurrent publishers remain
visible rather than replacing one another.

Release verification fails closed on payload substitution, signature/key
mismatch, malformed schemas, root-selected shard length/digest mismatch, blob
length/digest mismatch, unknown schema/partition metadata, and yanked releases.
Exact package/version lookup reads only the selected shard. Verified root and
shard bytes remain sufficient for an offline query.

Interrupted blob download persists a digest-named partial file, resumes through
bounded byte ranges, validates final length and SHA-256, and promotes only the
complete bytes. The activation model rejects permission growth and failed
bounded startup without changing the active release, retains exactly one
successfully activated predecessor, and rolls back to it offline.

### Native/Wasm parity

The same `registry-flow-core` FlatBuffer decoder and Ed25519 verifier runs
natively and inside a `wasm32-wasip2` component hosted by Wasmtime. The parity
fixture passes identical root, shard, package, version, and trusted key bytes to
both paths and asserts the same verified release digest. There is no handwritten
JavaScript query path.

### Live R2 evidence

A uniquely named temporary Cloudflare R2 bucket proved the actual S3-compatible
R2 operations required by the protocol:

- create-only `PutObject` with `If-None-Match: *` and conflict rejection;
- root replacement with `If-Match` and stale-ETag rejection;
- conditional root `GET` returning not-modified;
- concurrent object-first publication with root-CAS retry;
- immutable shard/blob reads, exact query, and signature verification;
- byte-range reads plus interrupted/resumed final digest verification; and
- prefix cleanup on completion.

The smoke completed successfully and its temporary bucket was deleted. R2 is
the sole registry backend. Custom-domain CDN headers, browser CORS, and cache
propagation are deployment configuration checks, not reasons to retain a second
backend or duplicate the registry protocol.

### Reproducible checks

From the prototype directory, `./check.sh` passes:

- five core tests covering concurrency, fail-closed verification, offline cache
  and yank behavior, resume, and activation/rollback;
- a direct `wasm32-wasip2` component build;
- one native/Wasmtime byte-parity test; and
- workspace Clippy with warnings denied.

`./check.sh --r2` is the separately credentialed disposable live smoke. Its
adapter reads only a regular non-symlink `0600` runtime target file, zeroizes the
parsed credentials on drop, avoids printing endpoint or credential details,
uses a random object prefix, and attempts prefix cleanup on success or failure.
Bucket and token lifecycle remain explicit operator actions.

### Implementation handoff

Port the proven state machine into the production registry crate rather than
shipping the spike directly. Use typed AWS SDK/R2 service errors instead of the
prototype's intentionally narrow HTTP-status mapping. Add the full release,
lock, cache-manifest, review-diff, revocation, and activation-set fields already
resolved by **Define registry publication and installation locks** and **Define
signing and trust**. Keep I/O in native/browser adapters and keep deterministic
FlatBuffer parsing, signature checks, partitioning, and queries in one shared
Rust core.
