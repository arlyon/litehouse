# Research static sparse registry index precedents

Type: research
Mode: AFK
Status: claimed
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
