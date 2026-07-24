# Research static sparse registry index precedents

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-14
Blocked by: 06
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

How do current Cargo sparse/git registries, OCI/Wasm registries, TUF-style
metadata, GitHub Pages/releases, S3/R2/CDN object stores, static SQLite, JSON
Lines, CBOR, and FlatBuffers compare for Litehouse's ultra-cheap read path,
including immutable artifact separation, partitioning, cache/ETag behavior,
atomic publication, signed snapshots, yanks/revocation, offline use, JavaScript
querying/search, repository reviewability, hosting/rate limits, and realistic
size/performance thresholds for choosing binary indexes?

## Answer

This research originally recommended an S3-compatible object store plus CDN and
text shards. The later **Choose registry hosting and index architecture**
decision and **Prototype registry publish, install, and rollback** evidence
supersede that recommendation with R2-only hosting and versioned FlatBuffers.
The precedent findings below remain evidence, not an active fallback plan. Do
not make GitHub, an OCI Distribution API, or a long-running registry service an
operational dependency.

Recommended separation:

1. **Immutable artifact plane:** content-addressed blobs and separate
   OCI-1.1-shaped manifests for contract releases, plugin releases, signatures,
   provenance, and SBOM referrers. Wasm components use the current CNCF Wasm OCI
   media layout where interoperable. Names/tags are aliases; installation
   identity is always manifest/blob digest.
2. **Mutable catalog plane:** deterministically serialized UTF-8 JSON/JSON Lines
   shards with separately generated disposable search summaries. A compact
   signed snapshot lists each shard's URL, SHA-256, length, and schema version.
   Start coarse and repartition by stable hash prefix only when measured size
   requires it.
3. **Trust plane:** use TUF 1.0.34 or a conforming implementation—offline root,
   delegated targets, versioned snapshot, short-lived timestamp, and consistent
   snapshot names—rather than inventing similar metadata/signatures.

Publication is object-first:

1. build and validate locally;
2. upload immutable blobs/manifests with create-only conditional writes;
3. verify stored length and SHA-256 rather than treating ETag as a digest;
4. upload catalog/search shards, targets, and versioned snapshot; then
5. publish `timestamp.json` last with generation/ETag compare-and-swap.

A CAS conflict rebuilds from the new head. S3's strong object consistency does
not create a multi-object transaction; immutable versioned objects plus the
TUF timestamp-last pointer provide atomic visibility.

Cache digested/versioned objects for a year as immutable. Keep timestamp short-
lived with conditional revalidation. Clients cache by SHA-256, retain the last
validated snapshot and known-good installation lock, and activate only after
all referenced objects/signatures verify. Metadata expiry blocks new resolution
or activation, not execution of an already activated known-good set; UI/API
must expose snapshot age. Range/resume large components, then verify the whole
SHA-256 before activation.

Yank, revocation, and deletion stay distinct:

- a signed yank overlay excludes a release from new resolution while exact
  existing locks remain reproducible;
- separately signed urgent revocation blocks new activation/update, while
  already-running policy belongs to signing/trust;
- deletion is exceptional legal/abuse handling with a signed tombstone where
  possible, never mutation of an immutable release manifest.

Installation locks canonically order plugin identities, exact release/artifact
and contract digests, bindings, runtime profile, granted permissions, selected
snapshot digest/version, and known-good predecessor. Identical validated inputs
against one snapshot produce byte-identical locks. URLs, ETags, names, and OCI
mutable tags are not lock identities.

Precedent conclusions:

- Borrow Cargo sparse index's per-name deterministic records, checksums, HTTP
  caching, and yank behavior, but add signed cross-shard snapshots.
- Use OCI descriptors/referrers for immutable packaging, not tags/listing as the
  catalog or search protocol.
- GitHub Pages/releases are suitable for prototypes but have documented site,
  bandwidth/build, and asset constraints without a registry SLA; do not make
  them the production contract.
- Static SQLite can be a locally built cache keyed by snapshot digest. Remote
  HTTP-range SQLite adds request amplification, CORS/worker/VFS complexity, and
  opaque updates.
- Compressed JSON Lines is reviewable, streamable, and native to JavaScript.
  CBOR and FlatBuffers provide no search by themselves and add bindings/debug
  cost before measurements justify them.

Do not reopen a binary-index decision from release counts alone. Benchmark
identical compressed JSON Lines, CBOR, FlatBuffers, and SQLite data only after a
normal operation exceeds roughly 5 MiB compressed, one hot shard exceeds 1–2
MiB compressed after repartitioning, parsing exceeds 100 ms p95 in Node or 250
ms p95 in the slowest supported browser/device, peak parse memory exceeds 64
MiB, or routine updates redownload over 1 MiB p50 unchanged metadata. These are
investigation triggers for the baseline/SLO tickets, not v1 promises.

At first-party v1 scale, engineering and review complexity dominate storage
cost. R2's current published model is low storage/request cost with free Internet
egress, while S3/CloudFront pricing varies by region; selected-provider
conditional-write, ETag, CDN propagation, purge, and range behavior still need
the required disposable registry prototype.

Primary sources:

- [Cargo registry index](https://doc.rust-lang.org/cargo/reference/registry-index.html)
- [OCI Image 1.1.1](https://github.com/opencontainers/image-spec/releases/tag/v1.1.1)
  and [Distribution 1.1.1](https://github.com/opencontainers/distribution-spec/releases/tag/v1.1.1)
- [CNCF Wasm OCI artifact layout](https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/)
- [TUF 1.0.34](https://github.com/theupdateframework/specification/releases/tag/v1.0.34)
- [Amazon S3 consistency](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html#ConsistencyModel)
  and [conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)
- [Cloudflare R2 pricing](https://developers.cloudflare.com/r2/pricing/)
  and [conditional operations](https://developers.cloudflare.com/r2/api/workers/workers-api-reference/#conditional-operations)
- [GitHub Pages limits](https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits)
- [SQLite Wasm](https://sqlite.org/wasm/doc/trunk/index.md),
  [JSON Lines](https://jsonlines.org/),
  [CBOR RFC 8949](https://www.rfc-editor.org/rfc/rfc8949), and
  [FlatBuffers](https://flatbuffers.dev/)
