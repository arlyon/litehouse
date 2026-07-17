# Define authentication and pairing

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 14, 19, 25
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 8

## Question

What exact OAuth/OIDC flows, issuer/audience/scope model, central account and
short-lived pairing proof, server-key binding, owner identity, standalone issuer
configuration, JWKS caching/rotation, token expiry/revocation, local HTTPS,
`unsecure-local` guardrails, recovery, and mode-switch migration implement the
exclusive cloud-connected and standalone modes?

## Answer

Use one owner, one issuer, one management authorization seam, and one local
Ed25519 server identity at a time. Cloud-connected and standalone are exclusive
trust modes, not two simultaneously accepted issuers. Keep RFC 8628 device
authorization only for callback-less CLI user login. Replace the current static
six-digit Cockpit password and placeholder certificate with a short-lived
pairing transaction that preserves the familiar discovery/name/code UI while
proving the server key and owner binding.

### Stable identities and mode records

The local server persists one versioned, tagged `AuthState` in its protected data
volume. Its variants are `UnsecureLocal`, `CloudUnpaired`,
`CloudPendingActivation`, `CloudPendingCancellation`, `CloudActive`, `StandaloneUnowned`,
`StandalonePendingOwner`, `StandaloneActive`, and `Switching`. Every variant has
a monotonic local trust generation. Authenticated variants also carry the exact
issuer, management resource, allowed asymmetric algorithms, validated discovery/
JWKS metadata, and an authorization-not-before instant. Cloud pending/active
states carry opaque server ID, public-key fingerprint, pair epoch, signed
pairing receipt, and proposed/active owner `(issuer, subject)`. Standalone active
state carries the exact owner. Pending states are never authorized states.

Pairing/bootstrap codes, transaction secrets, browser cookies, and OAuth tokens
are not fields of `AuthState`. Short-lived protocol secrets live only in their
bounded flow/session state; they do not create a generic secret store. Persist
every required state transition through a write-fsync-atomic-rename-directory-
fsync sequence before exposing its effects.

The Ed25519 private identity seed is operator-generated with a CSPRNG and
injected at process start as `LITEHOUSE_SERVER_IDENTITY_SEED`; Litehouse persists
only the derived public key/identifier and never the seed. It signs pairing,
presence, signaling, and key-proof requests; it never signs OAuth tokens, acts as
a client secret, or grants management access. Missing, malformed, or changed seed
versus active `AuthState` keeps authenticated listeners closed.
The central account ID/OIDC `sub`, not email, display name, passkey credential,
or server name, is owner identity. One account may own multiple servers; one
server has one owner in v1. Ownership transfer and shared-user roles are out of
scope. A different owner requires local trust reset and fresh pairing.

### Central account and OAuth profile

Cloud-connected mode uses the central Litehouse issuer and Better Auth database.
Target passkeys plus verified email magic links; migrate password users and then
disable password authentication rather than retaining three permanent login
systems. A magic link bootstraps a new account and can recover one, while a
user-verifying passkey is the preferred login and sensitive-operation step-up.
The stable Better Auth user ID is the issuer subject.

A new magic-link account remains restricted until it enrolls a user-verifying
passkey. Pairing, unpairing, credential changes, recovery, and every other
ownership-sensitive operation always require a user-verifying passkey assertion
no more than five minutes old; there is no pre-enrollment exception. Losing
every passkey starts a 24-hour notified recovery hold from a fresh, single-use,
ten-minute magic link. Starting recovery revokes ordinary web sessions and OAuth
refresh families; the recovery session cannot pair, unpair, issue management or
signaling grants, or change trust. The old passkey or notified cancellation link
cancels the hold. After the hold, another fresh magic link may register one
replacement passkey and atomically consumes the recovery record. Never use
security questions or support-visible recovery secrets.

Human browser/public clients use authorization code with S256 PKCE. The central
web BFF owns its rotating refresh family and Secure/HttpOnly/SameSite session;
the browser receives only short-lived server-bound access tokens in memory.
Callback-less first-party CLI uses RFC 8628 with the same resource/scopes, not as
server pairing. V1 requests no offline access/refresh token and keeps its short-
lived access token only in memory for the current command/session; automation may
inject `LITEHOUSE_ACCESS_TOKEN` through its own environment/credential runner.
The CLI never writes tokens, prints them, or implements a credential vault/file.
Remove the current hard-coded development URLs, ordinary JSON bearer-token
persistence, token/body/header debug output, and verbose secret serialization.
Logout/revocation is server-side; there is no local token material to remove.

Central management resource identifiers are stable logical HTTPS URIs of the
form `https://api.litehouse.dev/servers/<opaque-server-id>`. The exact URI is
sent as RFC 8707 `resource` during authorization and token exchange and is the
single JWT audience. Standalone uses its configured canonical external HTTPS
origin as both resource and audience. API and `/mcp` share that one management
resource and the fixed scopes from **Define service and management
authorization**; clients do not obtain a broader token merely by choosing MCP.

All management JWTs require an allowed asymmetric algorithm and known `kid`,
exact `iss` and one exact matching `aud`, stable `sub`, authorized client ID,
`iat`/`nbf`/`exp`, `jti`, scopes, and a bounded lifetime. The owner subject must
equal active local `AuthState`. Central tokens additionally require
`typ=at+jwt`, exact Litehouse server ID, and pair epoch. Qualified standalone
tokens do not require claims an external issuer cannot mint; they require an
exact configured access-token `typ` allowlist (`at+jwt` preferred) and `iat` not
before the current standalone authorization-not-before instant. ID-token types
are forbidden. Read/operate-only central tokens last at most ten minutes; any
token containing configure, install, diagnostics, or admin lasts at most five
minutes. Standalone qualification sets an equal or shorter maximum. Use at most
60 seconds of clock skew. Scope is necessary but the normal typed authorization
seam still checks every operation, resource, revision, grant, and safeguard.

### Pairing while preserving the current UX

Pairing exists only while an operator runs an explicit local `litehouse pair`
command. It generates a five-minute intent, a random single-use six-digit human
code, a 256-bit transaction secret, and fresh nonces. The command displays the
code and a grouped fingerprint derived from the server public key and intent; it
does not write either code or secret to configuration or logs. An unpaired
server otherwise does not advertise indefinitely.

The server registers over authenticated TLS a random transaction ID, public key/
fingerprint, code, transaction secret, server nonce, expiry, protocol version,
and a self-signed request transcript. Central computes HMAC lookup/verifier tags
under a pairing-service key, persists only those tags, and discards the plaintext
code/secret before responding; the server never receives or computes the tags.
Ordinary unkeyed hashes of the six-digit code are forbidden. The service
atomically enforces uniqueness among live code tags; a collision tells the
server to regenerate before display. Cockpit may show a pending server to a
recently authenticated browser from the same apparent source IP, preserving
today's nearby discovery. Source IP is only a convenience filter and never
identity or authorization. The user may enter the code directly when nearby
discovery fails.

The authenticated, recently reauthenticated owner chooses the display name,
enters the code, and compares the grouped fingerprint with the local command.
Before any response, the service atomically consumes one attempt. Five wrong
attempts close the intent globally; also limit lookup to ten attempts per account
and twenty per source IP per hour, returning one generic error. It then sends an
issuer-signed challenge containing the transaction, proposed owner subject,
central nonce, server nonce, server ID, pair epoch, expiry, and protocol version.
The server verifies issuer/signature/times and returns the transaction secret
plus an Ed25519 signature over the domain-separated complete transcript,
including method, target, body digest, and both nonces. Central verifies the
secret tag, key proof, code, owner, and unused intent.

Completion is an idempotent two-sided state machine, not an assumed distributed
transaction:

1. central consumes the five-minute proof intent, creates a distinct
   `pending-server-ack` binding retained for 24 hours, and returns a signed
   receipt;
2. the server verifies and durably enters `CloudPendingActivation` with that
   receipt, then sends the same signed acknowledgement until accepted;
3. central atomically marks inventory active, durably retains the consumed
   transcript for replay detection, and returns a signed active confirmation;
4. only after durably verifying that confirmation does the server enter
   `CloudActive` and enable authenticated management.

On restart, a pending server presents its receipt and reconciles before it opens
a management listener. Central returns the same active confirmation, a signed
pending response, or a signed terminal cancellation. An unreachable central
leaves the server pending and unauthorized. A 24-hour expiry or explicit local
cancel durably enters `CloudPendingCancellation` with a key-signed idempotent
cancellation statement; it does not discard the receipt, open management, or
start another pairing. Central must tombstone either a pending or already-active
binding and return a signed cancellation acknowledgement. Only after durably
verifying that acknowledgement does the server enter `CloudUnpaired`. Central
expiry performs the same tombstone, so restart reconciliation returns its signed
cancellation even if the prior active confirmation was lost. Conflicting keys,
owner, epoch, receipt, acknowledgement, or consumed intent fail closed. Pairing
codes and secrets are never management credentials. Cockpit
accepts only active inventory bindings obtained from the central inventory
service or a signed cache entry no older than 60 seconds; epoch/unpair changes
invalidate that cache immediately.

After pairing, server presence and Cockpit signaling requests use replay-bounded
Ed25519 HTTP message signatures tied to method, target, body digest, nonce, and
timestamp. Cockpit resolves the active inventory key instead of trusting the
current bearer/string stubs. A 60–120-second single-use signaling grant binds
owner, server, client, pair epoch, nonce, and the browser offer/DTLS fingerprint;
it authorizes signaling only. The server validates that grant, creates its
answer, and signs the answer body and newly selected server DTLS fingerprint
with its inventory Ed25519 key. Cockpit verifies the answer signature before
forwarding it, and the browser verifies the inventory key and committed
fingerprint before accepting the answer. This removes any need to know the
server's ephemeral fingerprint before it creates the answer.

The data channel rejects management calls until JSON-RPC `session.open`
validates the separate short-lived management grant. Channel authorization ends
at grant expiry; V1 closes and reconnects rather than reauthorizing a live
channel. Pair-epoch, owner, or trust-generation change closes all channels
immediately. Every RPC operation still passes through the normal typed
authorization seam; opening a session never grants indefinite or additional
authority.

### Standalone external issuer

Standalone is a resource server/BFF, never a Litehouse issuer. Configuration
pins one exact HTTPS issuer, discovered endpoint set, canonical public origin and
management resource, a bounded allowlist of at most eight client IDs with exact
redirects, allowed asymmetric algorithms, requested scopes, and one declared
public-PKCE or client-auth method per client. Discovery issuer equality is
exact; endpoint-host changes require local reapproval. Apply the SSRF and JWT
qualification gates from **Research OAuth and OIDC architecture** before
enabling the mode.

The local BFF performs authorization code/S256 exchange and keeps refresh/access
tokens server-side. The browser receives only a Secure/HttpOnly/SameSite cookie;
OAuth `state`, nonce, PKCE verifier, response issuer, code, redirect, and
resource are one-time and exact. Cookie-authenticated mutations retain separate
CSRF and exact-Origin checks. Direct API/CLI/MCP clients present qualified JWT
access tokens in the Authorization header. ID tokens never authorize APIs.

A fresh standalone server exposes only health, discovery, and login until owner
bootstrap completes. One valid external OIDC login creates a ten-minute pending
record bound to its session nonce, exact issuer, subject, client, resource, and
trust generation; another login cannot replace it. The local command displays
that pending subject, requires confirmation, and generates a ten-minute one-use
bootstrap code keyed to that exact pending-record ID. Each attempt is atomically
counted before comparison; five failures or any concurrent successful consumer
closes the record. One atomic commit pins `(iss, sub)`, advances authorization-
not-before, invalidates every pending login/code/session, and enters
`StandaloneActive`. First-login-wins and group-claim ownership are forbidden.
The code never appears in startup logs or configuration.

Loss or reassignment of the external subject uses an explicit local
`reset-standalone-owner` command. It journals the reset, invalidates all local
sessions/tokens and old issuer acceptance, advances trust generation and
authorization-not-before, and returns to `StandaloneUnowned` without deleting
runtime configuration. A new owner then completes the same bootstrap; there is
no remote recovery or implicit first login.

### JWKS, rotation, revocation, and outage

Persist a last-known-good JWKS under the exact issuer. Disable redirects; allow
only the already approved HTTPS endpoint. Use five-second connect/read and ten-
second overall timeouts, a 256-KiB decompressed limit, at most 16 supported
asymmetric verification keys, and bounded field/string lengths. Honor cache
metadata within a five-minute minimum and 24-hour maximum. Coalesce one unknown-
`kid` refresh per issuer per minute, then use bounded exponential backoff. Never
replace good keys with malformed, empty, wrong-issuer, symmetric, or unsupported
material. A known cached key may validate an otherwise valid unexpired token
during outage; an unknown key fails. Retain every still-needed old key beyond
maximum token lifetime, skew, and cache overlap rather than assuming exactly one
previous key.

During issuer/JWKS outage, permit only cached-key validation of already issued
unexpired management tokens. Do not start login/token refresh, pairing,
bootstrap, recovery, inventory lookup, presence reconnect, signaling, or new
WebRTC negotiation. An already authorized channel remains bounded by its current
token expiry and operation checks.

Central refresh families rotate and detect replay. Ordinary session/token
revocation blocks refresh and new grants, but already issued offline JWTs remain
valid until their short expiry. Unpairing also advances/tombstones the pair epoch
and stops new presence/signaling. A reachable server receives and durably applies
the new epoch/trust state through signed key-proof exchange; an offline server
may accept only already-issued, otherwise-valid old tokens until expiry. Never
claim instantaneous offline revocation.

If the local identity key is lost or suspected compromised, do not recover or
escrow it. Local access explicitly resets cloud trust and revokes/tombstones the
old binding when reachable. The operator then replaces the injected identity
seed and recreates the container; reset state rejects the old public key, accepts
the fresh derived identity, and pairing starts again. Home non-secret
configuration, plugins, rules, states, and grants remain local; operator-injected
values remain in the deployment, while old central identity/session material
does not. The supported data-volume restore
procedure always starts authentication suspended: cloud mode reconciles key/
server/epoch with active central inventory; standalone requires local owner
reconfirmation and advances authorization-not-before. Only then may management
listen. Blind filesystem rollback of live auth state is unsupported and never
silently restores trust.

### HTTPS and explicit unsafe local mode

Authenticated OAuth/OIDC management requires HTTPS. Support two mutually
exclusive operator-managed termination forms:

- Litehouse terminates rustls for the exact configured origin using
  `LITEHOUSE_TLS_CERTIFICATE_PEM` and `LITEHOUSE_TLS_PRIVATE_KEY_PEM`, injected
  by the operator/orchestrator environment and never persisted; or
- an operator TLS proxy exposes the exact configured origin while Litehouse
  binds its plaintext, still application-authenticated backend only to a host-
  loopback published port (the proxy must run in that reachable host/network
  namespace). Litehouse derives security decisions from static origin
  configuration, not arbitrary forwarded headers, and the proxy must replace
  rather than append forwarding headers.

Certificate issuance/renewal and automatic local PKI remain out of scope. Startup
fails on missing/malformed material, certificate/private-key mismatch, origin/
certificate mismatch, non-loopback proxy backend, or an HTTP callback/resource
in authenticated mode.

The explicit `unsecure-local` mode is unauthenticated HTTP on a loopback socket
only. Startup rejects wildcard, private-LAN, ULA, public, or proxy exposure and
does not honor forwarding headers. Accept only literal `localhost`, `127.0.0.1`,
or `[::1]` Host values with the configured port; reject DNS aliases. Disable
CORS, reject every foreign/`null` Origin and cross-origin WebSocket handshake,
and require a fixed non-simple `Litehouse-Local-Client` header on every unsafe
method so form posts and cross-origin scripts fail before mutation. Same-origin
browser requests use exact Origin/Host checks. It may expose the ordinary API to
local processes, but the later **Define the AI and MCP management surface**
decision supersedes this profile by disabling `/mcp` in `unsecure-local`. The API
still cannot create host shell/arbitrary I/O/raw-secret export or bypass fixed
safeguards. Emit a persistent UI banner,
startup warning, response marker, and structured event. It has no OAuth cookies,
issuer, central inventory, pairing, Cockpit, or signaling and is never an
implicit fallback when TLS/auth fails.

### Mode switching and migration

Only an explicit local host/container command may switch cloud-connected,
standalone, or `unsecure-local` trust modes. Web, OAuth, MCP, plugins, and rules
cannot invoke it. The command prints a redacted plan, requires the exact target
mode and server name as typed confirmation, and validates target TLS/issuer
configuration before writing a durable `Switching` journal.

The journal advances idempotently through `Prepared`, `ListenerQuiesced`,
`RevocationRecorded`, `OldTrustCleared`, and `TargetInitialized`. Each transition
uses the durable atomic state-write sequence. Startup that finds any nonterminal
switch exposes no management, login, pairing, Cockpit, or MCP listener and
resumes cleanup. Before retiring the active injected key binding, Litehouse
obtains central unpair acknowledgement or creates a signed, idempotent revocation
statement and retains
it in a retry outbox until central acknowledgement. A forced offline switch is
allowed only with an extra typed warning and that durable outbox; it never claims
remote inventory is already gone. External-issuer refresh revocation is best
effort, while local acceptance is removed synchronously.

`OldTrustCleared` invalidates server-side browser sessions and refresh families,
pending pair/bootstrap/recovery records, owner binding, JWKS/discovery cache,
old issuer/resource acceptance, and the old public-key identifier/acceptance. It
cannot erase tokens
already copied to remote browsers or CLIs; short expiry, revocation, pair epoch,
and local trust state make them unusable. The next transition advances trust
generation and authorization-not-before and requires a fresh operator-injected
identity seed before creating target-mode bootstrap state. Runtime non-secret
configuration, registry cache, plugins, rules, state, service/WASI grants, and
operator-managed environment values remain. There is no dual-issuer
grace period and no rollback that restores old trust; returning is a new switch
and authorization. A terminal local recovery command may only finish cleanup or
initialize the already journaled target, never reactivate the source state.

Migrate the current implementation expand-first: add stable user/credential,
OAuth grant/family, server inventory/key/epoch, pairing transaction, and recovery
records; qualify Better Auth provider/JWT/passkey behavior; and migrate existing
users and passkeys. Authenticated networking cannot launch until tests prove all
current raw-string bearer identities, static pairing passwords, unauthenticated
Cockpit finalization/rejection routes, hard-coded development endpoints, token/
header/body debug output, ordinary JSON token storage, and plaintext credential
fallbacks are removed or unreachable. Cut password and prototype bearer/device
endpoints only after clients use standards endpoints. Do not combine this with
framework, passkey origin, D1, Cockpit, or DNS cutovers. History-aware credential
remediation remains governed by **Define secret lifecycle and credential
remediation** without reproducing values.

### Validation and observability

Required tests cover authorization-code/PKCE/resource binding, RFC 8628 CLI,
refresh replay, separate central/standalone claim profiles, exact issuer/
audience/server/epoch/scope rejection, authorization-not-before, token/channel
expiry, CSRF and redirect/origin checks, passkey enrollment/step-up and delayed
recovery, pair/bootstrap attempt atomicity, expiry, replay, collision and
concurrency, key substitution, every pairing crash/restart/reconcile edge,
nearby-discovery non-authority, signed presence, two-stage DTLS commitment,
signaling replay, standalone qualification/bootstrap/reset, JWKS rotation/
outage/storms/redirects/bounds, clock skew, TLS/proxy boundaries, loopback Host/
Origin/CORS/WebSocket/DNS-rebinding defenses, identity loss, stale backup,
unpairing, and failure injection at every fsynced mode-switch transition. Launch
gates also assert every legacy bypass/secret-output route is unreachable.

Use bounded redacted `tracing` targets for authn, pairing, token/JWKS decisions,
recovery, unpairing, and mode changes. Record safe user/server/key/epoch/client/
transaction IDs, decision reasons, and correlation IDs; never log passwords,
magic links, codes, transaction secrets, private keys, cookies, tokens,
authorization headers, OAuth response bodies, raw config, or recovery material.
No second audit database is introduced.
