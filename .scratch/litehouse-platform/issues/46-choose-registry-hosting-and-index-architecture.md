# Choose registry hosting and index architecture

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 06, 45
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which R2 object-store/CDN contract, partitioned
FlatBuffers index, content-addressed artifact layout, origin trust model, cache
policy, publication writer boundary, and unified Rust/JavaScript client form the
canonical ultra-cheap v1 registry architecture before detailed release objects
and installation locks are specified?

## Answer

Keep and complete Litehouse's existing static object-store/FlatBuffers
direction on Cloudflare R2 rather than replacing it with Cargo JSON, OCI
Distribution, TUF, an always-on registry service, or a second storage backend.

### Hosting and origin contract

The canonical read path is Cloudflare R2 behind a custom-domain CDN. The
registry prototype proved R2's exact S3-client conditional-write, ETag,
conditional-read, and range behavior. V1 has no S3/CloudFront fallback and no
backend-portability layer; the small internal object-I/O seam exists only to
separate the shared Rust core from native, browser, cache, and test adapters.

The required origin surface is intentionally small: anonymous `GET`/`HEAD`, byte
ranges, strongly consistent reads, create-only immutable writes, ETag
compare-and-swap for one mutable root, CORS, and CDN cache headers. No object
listing, database, Git checkout, OCI registry API, or server-side query endpoint
is part of the client contract.

### Root and partitioned index

`registry/index.bin` becomes a real FlatBuffer `RegistryRoot`. It contains:

- format/schema and key-normalization versions;
- a diagnostic publication generation;
- one bounded, data-only partition-scheme variant and parameters;
- immutable shard descriptors with partition identity/range, object path,
  SHA-256, byte length, and schema version; and
- no executable registry-provided logic.

V1 publishes the existing alphabetical seven-way scheme (`[Split::Seven]`). The
scheme is selected from root metadata so a later supported generation can
repartition without another root-format migration. Readers fail closed on
unknown schemes or normalization versions. Repartitioning publishes a complete
new immutable shard set and switches the root; it never mutates the active
partition map.

Each sorted FlatBuffer shard stores one structured record per release. A record
has two logical regions:

1. an immutable, deterministically encoded publisher-signed payload containing
   publisher namespace/identity, package/version, component/WIT/docs/provenance/
   SBOM blob descriptors, provided/required contracts, compatibility,
   permissions, and signature context; and
2. separately mutable registry state such as yank, withdrawal, review, and
   eventual revocation references, excluded from the publisher signature.

The signed-payload digest is the stable release identity cached by installation
locks. Updating registry state rewrites the affected immutable shard and root
but does not change release identity. FlatBuffer signing bytes require one
specified deterministic encoder plus native/Wasm conformance vectors; semantic
re-encoding is never guessed.

Artifact bytes use content-addressed storage such as
`blobs/sha256/<digest>`. Publication verifies SHA-256 and length and uses
create-only writes. Identical artifacts deduplicate naturally. OCI descriptors,
tags, referrers, and Distribution APIs are not canonical; a deterministic OCI
projection/mirror may be added later for `wkg`/ORAS interoperability.

### Trust and publisher scope

V1 deliberately uses a Cargo-like origin-trust model rather than TUF:

- HTTPS/CDN origin trust for the current root and mutable registry state;
- publisher signatures over immutable release payloads;
- SHA-256/length verification for root-selected shards and blobs;
- immutable package/version release identity;
- exact digest installation locks, reviewed updates, yanks, and known-good
  rollback; and
- explicit acceptance of registry-origin freeze/rollback risk.

ETags answer cache equality and root publication concurrency; they are not
content hashes. Clients do not persist a security minimum generation. A root
generation remains useful for diagnostics and cache grouping but the client
trusts the current ETag-revalidated root. Stronger rollback/freeze resistance
requires a complete future TUF decision, not a custom TUF-like signature.

The record and writer interfaces are third-party-ready, but central v1 policy
accepts only Litehouse first-party publishers. Public namespace enrollment,
publisher intake, moderation, abuse handling, and key recovery remain deferred.
Third parties never receive bucket credentials; a future intake validates their
signed records and invokes the same writer.

### Publication and caching

A short-lived CLI/CI publisher is the only v1 writer. It:

1. validates and signs the release payload;
2. uploads missing content-addressed blobs with create-only writes;
3. conditionally fetches the current root and affected shard;
4. builds and validates a new immutable shard;
5. uploads the shard and verifies stored length/hash; then
6. replaces `registry/index.bin` with `If-Match` against the prior ETag.

A precondition failure discards no live data: the writer reloads the new root,
reapplies its release, and retries. Only the short-lived job receives narrowly
scoped R2 write/purge credentials.

Roots and verified shards persist in local cache indefinitely for offline reads.
An intentional registry refresh—startup asynchronously, explicit refresh,
opening install/update views, or resolving a new install/update—coalesces and
rate-limits one conditional root request using the cached ETag. `304` keeps the
cache; `200` verifies and fetches only referenced missing/changed immutable
shards. Network failure continues from the last validated cache with visible
last-refresh time.

Content-addressed blobs and immutable shards use long-lived immutable CDN
caching. The mutable root is stored locally but conditionally revalidated on an
intentional refresh; ordinary cache reads do not hit the network. No fixed
freshness interval is required merely to save CDN requests.

Exact package lookup fetches one alphabetical shard. Browse/prefix/free-text
search fetches and verifies all seven v1 shards and searches client-side. A
separate search artifact or query service is deferred until representative
measurements justify it.

### Unified client and Cloudflare rendering

One pure Rust `registry-core` owns FlatBuffer decode/verification, key
normalization, dynamic bounded partition selection, release queries, and signed
payload/lock digest logic. Native publisher/server tooling links it directly.
JavaScript uses that same core compiled to Wasm; thin environment adapters own
`fetch` and persistent cache bytes. N-API and independently generated TypeScript
query logic are not v1 requirements.

Cloudflare Workers Cache may cache derived public registry HTML/JSON rendered
through the Wasm core, with package/registry tags purged after successful root
publication. It is not part of the registry protocol. Content-addressed blobs
and static assets remain direct immutable CDN/R2 responses; auth, sessions,
signaling, and live management data remain uncached. The broader Vercel versus
Cloudflare control-plane topology is a separate decision.

### Prototype and migration requirements

The disposable registry prototype must prove R2 origin behavior for create-only
uploads, root ETag CAS/conflicts, conditional reads, ranges,
interrupted/resumed downloads, concurrent publishers, offline cache,
malformed/unsupported roots/schemes/shards, digest failures, yanks,
deterministic locks, and rollback to known-good artifacts. Custom-domain CDN
headers, browser CORS, and propagation remain bounded deployment checks; they do
not justify a fallback backend or duplicate protocol implementation.

The implementation tranche preserves the generic partition-store concept but
replaces its prototype hazards: hardcoded endpoint credentials/configuration,
unversioned root, stale path cache, unchecked I/O, global temp filename,
incomplete index methods, unsafe lifetime assumptions without bounded ownership,
and mismatched tracked generated output. Pin `flatc` with its runtime, generate
Rust/Wasm bindings reproducibly into build output, and provide deterministic
human-readable dumps/fixtures for review.
