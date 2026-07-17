# Choose central control-plane hosting

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-15
Blocked by: 14, 27, 28, 45, 46, 48, 49, 50
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Should Litehouse replace Vercel with a Cloudflare-centered topology, and which
uncached gateway/auth versus Workers Cache entrypoints, tag-purged public
registry rendering through the Rust/Wasm core, static-assets/R2 delivery,
Durable Objects, database, Better Auth/session, Cockpit WebSocket/WebRTC
signaling, secrets, custom-domain, preview-build, observability, cost,
portability, deployment, and rollback boundaries host the central website and
transient control plane without caching private live management data or moving
routine home traffic/local runtime into the cloud?

## Answer

Replace Vercel with a Cloudflare-centered topology. Use D1 as the sole
authoritative durable database for central identity, OAuth, pairing, and server
inventory. Use a separate Cockpit Worker with one Durable Object per paired
server only for bounded live presence/signaling coordination. Use R2 only for the
static registry, Workers Static Assets for deployment-static web content, and
Workers Cache only for the public/per-user-derived classes already decided.
Direct WebRTC with ordinary ICE is the first-release path. TURN/relay fallback is
deferred and requires a later explicit selection based on observed connectivity.

Use direct, thin Cloudflare adapters at infrastructure boundaries. Do not build a
provider-neutral database/cache/object/actor framework, but do keep auth, domain,
registry-core, and management behavior independent from Worker bindings so they
remain testable and are not duplicated between web, Cockpit, and local runtime.

### Deployment topology

Deploy independently versioned units:

1. **`litehouse-web` Worker** — exact-pinned TanStack Start application, public
   route gateway/SSR, Better Auth sessions/passkeys, OAuth/OIDC issuer, pairing/
   recovery/account/server-inventory APIs, protected-resource/authorization-
   server metadata, short-lived management/signaling grant issuance, and the
   authenticated durable shell. It binds D1, Workers Static Assets/Cache,
   outbound email, and a service binding to
   Cockpit. It never proxies routine home management traffic.
2. **`litehouse-cockpit` Worker** — WebSocket/HTTP signaling gateway, strict
   request/rate/auth bounds, per-server Durable Object routing, and TURN
   credential adapter. It has no Better Auth cookie handling, registry writer,
   general management API, user interface, or D1 write access.
3. **`ServerSession` Durable Object namespace** — one object keyed through
   `idFromName` from a stable random 256-bit routing ID created/stored in D1 and
   never accepted from request authority. It serializes that server's presence and
   signaling attempts, manages hibernatable WebSockets, nonces/replay markers,
   answer/fingerprint commitments, and short-lived connection state. It does not
   own account, OAuth, owner, key, pair-epoch, or server-inventory truth.
4. **Registry R2/custom domain** — immutable blobs/shards and conditionally
   revalidated root. Only the protected first-party CI/CLI publisher writes it.
   Web and Cockpit receive no bucket write credentials.
5. **D1** — one clean production database and isolated clean staging database.
   D1 is the durable source of truth described below; legacy Turso/libSQL state
   is neither read, copied, dual-written, nor used as rollback authority.

Static assets may be served by the web Worker/Static Assets deployment. Do not
add Pages, KV, Queues, Hyperdrive, Vectorize, Workflows, or another database
without a measured requirement and separate decision.

### Stable origins and route classes

Use stable configured production origins for website/passkey RP, OAuth issuer,
logical per-server resource URIs, Cockpit, and registry. Exact hostnames are
release configuration, but issuer/RP/resource identity never follows random
preview URLs and never changes during framework/database/Cockpit migrations.
Staging has separate stable origins/RP ID, D1 database, R2 bucket, DO namespace,
TURN/email behavior, OAuth clients/keys, and secrets.

Preserve the four route classes from **Choose Cloudflare web framework and cache
model**:

- deployment-static marketing/docs/legal/assets through Static Assets;
- public registry-derived pages through read-only R2 + Rust/Wasm registry core +
  Workers Cache/tag purge;
- authenticated durable account/pairing/inventory routes through the uncached
  auth gateway, with only explicitly keyed inner read-model caching; and
- private live management/signaling routes that are never shared-cached.

Any request that reads cookies, Authorization, OAuth/session/recovery/pairing
state, owner inventory, signaling, WebRTC descriptions, TURN credentials, or
management data sends `private, no-store` and bypasses shared cache. Cache output
never authorizes. Public registry rendering receives verified bytes from the
same Rust/Wasm core proved by the registry prototype; R2 data and cache errors do
not fall back to hard-coded package data.

### D1 durable authority

D1 owns normalized, versioned relational records for:

- Better Auth users, verified emails, passkeys, credentials, sessions, and
  delayed recovery holds;
- OAuth/OIDC clients, exact redirects/resources, authorization codes, PKCE,
  consents/grants, refresh-token families/replay state, revocation, and bounded
  token metadata;
- accepted JWT signing-key metadata/public JWKS and rotation state, while private
  signer material remains in the approved Worker secret/signing boundary;
- paired servers, owner subject, public identity key/fingerprint, pair epoch,
  display metadata, status/tombstone, and config-safe inventory fields;
- pairing intents, keyed code/secret verifiers, signed receipts,
  acknowledgement/activation/cancellation state, attempts, expiry, and replay
  prevention;
- registered first-party web/CLI/MCP clients and fixed scope grants;
- RFC 8628 CLI device/user codes, exact client/resource/scopes, keyed verifier,
  polling interval/last poll, attempts, owner approval, expiry, status, and
  atomic consumed/replay state; and
- schema/migration metadata and bounded safe operational counters required for
  correctness, not a duplicate audit log.

Use foreign keys, unique constraints, bounded columns, exact timestamps/counters,
and D1 transactional batches/CAS around code consumption, passkey counters,
refresh-family rotation/replay, owner/server/key/epoch binding, recovery, and
pairing activation. Exact-pin and qualify Better Auth's device-authorization
plugin for callback-less CLI; if it cannot meet RFC 8628/resource/poll/replay
requirements, implement only that small standards endpoint over the same D1
records—not a second token issuer. It is user login, never server pairing.

Every tenant-owned table carries immutable `owner_id` and uses composite
owner/resource foreign keys and uniqueness where applicable. Repository methods
receive owner context only from the validated session/token principal and always
predicate CRUD/list operations on it; request bodies cannot choose owner
authority. Every mutation is idempotent or revision-checked. Store
neither device credentials/config/rules/state, raw home telemetry, WebRTC SDP/
ICE, TURN passwords, plugin secrets, nor routine management traffic in D1.

A single global Durable Object is not the database. It would serialize all
accounts in one location/failure domain, fit Better Auth/relational queries and
migrations poorly, and eventually require a directory database when partitioned.
Per-user Durable Object storage would similarly duplicate D1 inventory and
complicate cross-account uniqueness/recovery. D1 is durable truth; DOs serialize
only one server's live session.

### Cockpit and Durable Object contract

The Cockpit edge verifies a central signed signaling grant before routing to the
server object. The grant is 60–120 seconds, single-use, and bound to owner,
client, server, pair epoch, scopes needed for connection, nonce, and browser
offer/DTLS fingerprint. Server presence and answers use the Ed25519 key-proof
protocol from **Define authentication and pairing**.

A `ServerSession` object may hold only:

- current authenticated server WebSocket attachment and liveness;
- bounded pending browser attempts/offers and one-shot response channels;
- consumed grant IDs/nonces until expiry;
- short-lived signed inventory-cache entry;
- bounded rate/queue/timeout counters; and
- TURN issuance correlation/expiry without the credential after return.

It never persists SDP, ICE candidates, management JWTs, bearer/cookie values,
TURN passwords, or management payloads. Hibernation attachments contain only
safe opaque IDs and expiry/version metadata. Before forwarding an offer,
answer, or TURN issuance, the object atomically persists the consumed signaling-
grant ID/nonce in its own storage; eviction/restart cannot make it reusable.
Replay markers and bounded counters expire/GC after the grant/token overlap but
are mandatory, not an optional cache.

Cockpit obtains an active inventory/key/epoch binding through a narrow signed
web-Worker service binding. The binding is a renewable presence lease no longer
than 60 seconds. Sockets close unless a fresh equal-or-newer lease is verified.
Pair/unpair/epoch/key transactions also write a durable D1 invalidation-outbox
row; the web Worker sends it to the stable routing ID and retries from scheduled/
subsequent work until the DO acknowledges, then clears the row. Immediate ack
closes stale sockets; lease expiry bounds missed delivery. An absent/stale/
conflicting binding fails closed. Cockpit has no generic D1 query/mutation
binding.

Limit one active server presence socket, bounded concurrent browser attempts and
queues, complete signaling timeouts, WebSocket frame/message size, and per-
account/server/IP rates. A replacement server socket requires a newer signed
presence nonce/epoch and closes the prior socket explicitly. Deployments and
hibernation never imply authorization or preserve expired grants.

### Direct ICE; TURN deferred

The first connected release uses ordinary WebRTC ICE with configured STUN as
needed. Cockpit facilitates signaling only and does not relay routine management
traffic.

TURN provider selection, credential issuance, quotas, abuse controls, telemetry,
and relay-specific ICE restart are deferred. If observed networks later show that
direct connectivity is insufficient, define and approve that operational feature
from measured failure evidence. Until then the UI reports direct-connection
failure rather than silently routing management through central infrastructure.

### Better Auth and OAuth issuer boundary

Exact-pin and qualify Better Auth's current OAuth Provider/JWT/passkey plugins
against D1 and Cloudflare Workers before cutover. Better Auth owns browser
sessions, passkey ceremonies, client/grant/token-family persistence, and code
exchange plumbing. Litehouse owns exact issuer/resource/audience/server/epoch/
scope claims, pairing key proof, protected-resource metadata, authorization
checks, recovery hold, token lifetimes, and redacted behavior. Generic bearer or
session shortcuts never authorize home resources.

The web BFF stores refresh families/session state server-side and gives browsers
only Secure/HttpOnly/SameSite cookies plus in-memory short management tokens.
All auth and mutation responses are uncached. Cookie mutations require CSRF and
exact-Origin controls. The issuer has exactly two versioned Ed25519 private-key
slots (current/next) in encrypted production Worker secret bindings, accessible
only to the issuer module; D1 stores public JWKS/key status only. Protected key-
rotation deployment publishes both public keys, switches signing, waits beyond
maximum token/skew/cache lifetime, then removes the old key. Keep one encrypted
offline recovery copy under separate custody; staging has unrelated keys. Better
Auth secret, mail credential, TURN provider credential, bindings, and environment
IDs are likewise secrets—not source, D1 plaintext config, public env, or preview
fallback. Production cutover remains gated on the complete custody/rotation/
remediation contract in **Define secret lifecycle and credential remediation**.

### R2 and public registry rendering

`litehouse-web` reads the registry only with anonymous GET/HEAD through its
custom HTTPS registry domain and a small Fetch byte adapter. It has no R2 bucket
binding, S3 credential, purge credential, or write-capable origin path. The pure
Rust/Wasm registry core verifies root,
shard, release, and blob metadata. Public package pages cache derived output with
canonical deployment/schema/representation keys and bounded registry/package/
release tags. Successful publisher root CAS triggers tag purge; purge failure
retries and TTL bounds staleness. Auth/private routes never share that cache.

The protected publisher remains the only root/blob writer and cache-purge
initiator. Web compromise must not yield registry write/signing credentials;
registry publisher compromise must not yield D1/auth/Cockpit credentials.

### Clean D1 bootstrap and recovery

The later greenfield decision supersedes Turso migration. Do not combine
framework, issuer/passkey origin, auth schema, D1, Cockpit, or DNS cutovers. The
bounded sequence is:

1. create expand-only schemas in a clean isolated staging D1 database;
2. validate new Better Auth/OAuth/pairing behavior with staging-only identities,
   passkey origin, email behavior, and bindings;
3. create a clean production D1 database and apply the exact verified migration
   set before enabling account creation;
4. cut the greenfield web/auth routes to Workers only after the D1/schema/auth
   gate passes; existing legacy accounts/sessions/passkeys are not imported;
5. soak D1 auth/pairing/inventory before enabling the separate Cockpit binding;
   and
6. perform DNS cutover separately, retaining prior Worker versions and the legacy
   deployment only as independent route rollback—not data authority.

There is no Turso binding, read, snapshot copy, dual-write, read-repair, reverse
migration, or account/state compatibility layer. Once production D1 accepts
writes, rollback uses a schema-compatible previous Worker or forward fix; it
never points at legacy state. D1 schema changes remain expand/contract and Worker
versions tolerate the overlap during gradual deploys.

After D1 production begins, require provider-native point-in-time recovery with at
least 30 days retention and target RPO 15 minutes/RTO four hours. Restore never
overwrites production in place: freeze writes, restore/copy to an isolated D1
database, verify schema/constraints/row counts/hashes and auth/pairing invariants,
then atomically rebind a reviewed Worker version. Rehearse this quarterly in
staging and alert on backup/Time Travel unavailability. If the selected
Cloudflare plan cannot meet retention/RPO/RTO, production cutover is blocked.

### Deployment, environments, and rollback

Web and Cockpit have separate Worker projects, bindings, secrets, DO migrations,
version histories, tests, and deployment permissions. Web can deploy without
restarting live server sockets; Cockpit can deploy without changing issuer/web
assets. Durable Object class/schema migrations are additive and old/new Worker
versions interoperate through explicit message versions during rollout.

Production uses separately approved jobs and non-overlapping credentials for web
code deploy, Cockpit code deploy, D1 schema/data migration, DO class migration,
secret/key rotation, route/DNS change, TURN provisioning, and registry/cache
purge. No ordinary code-deploy job can mutate D1 schema/data, secrets, routes,
TURN account settings, or registry. Preview builds use static/public routes and
isolated ephemeral resources only; they never bind production D1/R2/DO/secrets,
send production mail, issue trusted OAuth tokens, or register production
passkeys. Stable staging—not random preview URLs—runs auth/signaling browser
tests.

Use Workers gradual versions for code. Cloudflare code rollback does not roll
back D1, DO, or R2 state. Every release declares compatible schema/message ranges
and rollback gate. Migration/pairing/token changes require expand-first rollout,
verification, then contract cleanup in a later release. Keep the previous Vercel
build only during the explicit route-by-route migration window; delete Vercel
bindings and duplicate private routes after final soak rather than maintaining a
permanent fallback stack.

### Availability, cost, and portability

Routine device state, commands, rules, logs, and plugin traffic remain local and
flow over direct/relayed end-to-end WebRTC only while a user is connected. D1 is
hit for auth, account/inventory, pairing/recovery, grant issuance, and bounded
read models—not device polling. Durable Objects hibernate with no active server/
browser socket and retain no unbounded session history. Workers Cache/R2 absorb
public registry traffic. TURN incurs traffic only when direct ICE fails, with
per-account/server quotas and visible usage/failure.

A central outage never stops local plugins/rules or erases local trust. Cached
keys validate otherwise-valid short JWTs until expiry under the auth outage
rules; no new login, pairing, inventory lookup, signaling, TURN credential, or
reconnect is promised. Standalone mode remains independent.

Cloudflare integration is direct behind narrow adapters: D1 repositories,
registry-domain Fetch bytes, Workers Cache, service bindings, signer/mail/TURN
clients, and Cockpit DO router. Domain records/state machines and the Rust/Wasm
registry core never import Cloudflare bindings. Do not implement generic SQL/object/cache/actor portability,
shadow stores, multi-cloud failover, or a second signaling implementation.

### Observability and validation

Use bounded redacted Worker/DO structured logs and metrics for route class/cache,
authn/authz, D1 transaction/retry, pairing/recovery, grant/epoch, presence,
signaling, TURN issuance/use category, deployment version, and safe correlation
IDs. Authentication, authorization denial, recovery, key/epoch, refresh-replay,
pair/unpair, migration, secret/route, and TURN circuit-breaker events are
unsampled and delivered through Cloudflare Logpush to a protected retention sink
with restricted RBAC, at least 30 days retention, delivery-lag/failure alerts,
and no application mutation/query interface. This remains the configured tracing
sink, not a second audit database. Never log cookies, tokens, magic links,
authorization codes, PKCE values, passkey data, pairing codes/secrets, private
keys, email bodies, SDP/ICE, TURN credentials, D1 row payloads, home data, or
unrestricted errors. Existing structured tracing remains the audit source; do not add a duplicate audit DB.

Required implementation gates include D1 transaction/failure injection for
codes/counters/refresh replay/pairing and composite owner isolation on every CRUD/
list path; Better Auth Worker/D1 and RFC 8628 polling/expiry/replay/resource
qualification; two-key JWT rotation/outage; passkey/recovery; cross-account/
server/epoch/resource replay; private-cache isolation; anonymous GET/HEAD-only
registry access and Rust/Wasm parity/purge; durable DO grant consumption, lease/
invalidation retry, hibernation/replacement/rate/timeout behavior; signaling/
DTLS replay; no SDP/token persistence; and, **only if TURN is selected/enabled
for the release**, Cloudflare Realtime TURN API/endpoints, abuse-destination
denial, allocation/refresh/traffic quota, telemetry/circuit breaker, credential
expiry, and direct-to-relay ICE restart. With TURN disabled, require configuration
and UI/protocol fixtures proving no credential request plus explicit relay-
unavailable failure; clean staging/production D1 bootstrap with no legacy binding/import and quarterly
D1 restore/RPO/RTO rehearsals; separate CI permission denial tests; Logpush delivery/
alert/retention tests; staging/preview isolation; gradual web/Cockpit/DO/D1 schema
compatibility; custom-domain/DNS rollback; and proof routine management payloads
never reach web, Cockpit, D1, cache, logs, or registry storage.
