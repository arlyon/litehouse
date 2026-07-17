# Research OAuth and OIDC architecture

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-14
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which current OAuth 2.1/OIDC standards and Rust/browser libraries support the
two exclusive Litehouse modes—central issuer with pairing/Cockpit/WebRTC versus
standalone external issuer with direct HTTPS API/MCP—including discovery, PKCE,
device/pairing grants, audiences, scopes, token/JWKS lifetimes, offline
validation, revocation, key rotation, issuer pinning, and protected-resource
metadata?

## Answer

Use one canonical management authorization model with exactly one issuer mode at
a time.

In **cloud-connected mode**, Litehouse operates a stable OAuth 2.1/OIDC issuer
on Cloudflare Workers. Better Auth stores identity, OAuth grants, refresh-token
families, passkeys, and durable server inventory in the selected durable store.
A separate Cockpit Worker/Durable Object handles only presence and WebRTC
signaling. Local servers validate short-lived, server-bound JWT access tokens
offline. Pairing and server-key authentication remain Litehouse protocols, not
OAuth device grants.

In **standalone mode**, the operator configures one exact external OIDC issuer.
The local HTTPS API and `/mcp` are resource servers only; there is no Litehouse
issuer, central account/inventory, pairing service, Cockpit, or signaling. The
issuer must pass qualification for exact discovery, authorization code with S256
PKCE, locally verifiable JWT access tokens with accepted resource/audience and
algorithms, refresh/revocation behavior, and MCP registration compatibility.
OIDC support alone is insufficient because many issuers provide opaque or fixed-
audience access tokens. ID tokens are never API bearer tokens.

Never accept both issuers simultaneously. Mode change is an explicit destructive
trust migration: revoke/delete old token families and inventory bindings, clear
issuer/JWKS state, rotate local management credentials, and require fresh
authorization.

### Standards and central profile

OAuth 2.1 remains draft 15; implement its authorization-code/S256 behavior while
using RFC 9700, RFC 8414, OIDC Core/Discovery, RFC 7636, RFC 9207, RFC 8707,
RFC 9728, RFC 9068/8725, RFC 7009, RFC 7662, RFC 8252, and RP-Initiated Logout as
the normative baseline. Do not implement implicit, password grant, PKCE plain,
tokens in URLs, wildcard redirects, or ID-token bearer use.

Prototype exact-pinned Better Auth 1.6 OAuth Provider/JWT/passkey plugins and
`jose`; do not rely on deprecated `oidcProvider`. Human/public clients use only
authorization code plus S256. Confidential `client_credentials` is separately
registered and never substitutes for server identity. Disable unauthenticated
dynamic registration by default and generic JWT/session shortcuts that bypass
resource/audience semantics.

Clients always send the same RFC 8707 `resource` at authorization and token
exchange so the issuer emits a JWT resource token. Management tokens require
exact issuer/audience, stable user subject, authorized client, scopes, short
lifetime/JTI, and namespaced server ID plus pair epoch matching the local paired
state. Audience alone never authorizes every server.

Cockpit uses a different single-use 60–120-second signaling grant bound to
signaling audience, server/user/client identity, WebRTC fingerprint, nonce, and
expiry. It cannot authorize management operations.

Initial operation scopes are `litehouse:read`, `operate`, `configure`, `install`,
`diagnostics`, and `admin`, plus OIDC scopes. Scopes are coarse; local ownership,
endpoint/resource, revision, grant, and safeguard checks remain authoritative.
Ownership transfer, unpairing, trust weakening, secret export, host shell, and
arbitrary I/O are separate forbidden or owner-only actions. MCP uses the same
scopes/checks as the normal API.

### Standalone and browser behavior

Standalone configuration pins the exact issuer and approved discovered endpoint
set, expected API/MCP resources, allowed asymmetric algorithms, client IDs/
redirects, and local HTTPS URLs. Discovery requires exact issuer equality and
HTTPS; endpoint-host changes require administrator reapproval. SSRF controls
reject unexpected loopback/link-local/private endpoints unless the operator
explicitly configured an on-premises issuer.

Prefer a local Rust BFF: it performs code exchange with PKCE and stores tokens
server-side while the browser receives a Secure/HttpOnly/SameSite session
cookie. `state`, OIDC nonce, authorization-response issuer, PKCE verifier, code,
redirect, and resource are one-time and exact. OAuth state does not replace CSRF
protection for ordinary cookie-authenticated mutations. A pure SPA is only a
fallback using in-memory access tokens and session-scoped transient state, never
localStorage refresh/access tokens.

### Pairing and server identity

RFC 8628 is only for limited-input OAuth clients such as a callback-less CLI. It
is not server ownership proof: device codes do not prove possession of a server
key or bind pair epochs/WebRTC fingerprints.

Litehouse pairing uses a locally generated Ed25519 server identity key, a short-
lived single-use human code plus high-entropy secret, authenticated owner review
of the server fingerprint, and a signed challenge containing both nonces and the
transaction. Central state atomically binds server ID, public key, owner subject,
and pair epoch. Re-pair/recovery increments the epoch and invalidates prior
grants. Pair codes expire within five minutes, are attempt-limited, and are never
management credentials.

The identity key authenticates signed pairing/presence/signaling requests, not
OAuth tokens or client secrets. Bind signatures to method, target, body digest,
nonce, and timestamp, preferably using RFC 9421 semantics. Do not force it into
Better Auth `client_credentials`; `private_key_jwt` would require a separate
prototype.

### Tokens, JWTs, JWKS, and outage

Recommended central defaults are 10-minute management tokens, five minutes for
higher-risk configure/install/admin tokens, 60–120-second signaling grants,
five-minute pairing/code lifetimes, ten-minute ID tokens, and 30-day rotating
refresh families. Refresh tokens stay in BFF/server storage. Revoke refresh
families/sessions and increment pair epoch where relevant; already issued offline
JWTs remain valid until short expiry, so do not claim instantaneous revocation.

Rust resource servers validate allowed asymmetric algorithm, known `kid`,
signature, exact issuer/audience, times with at most 60-second skew, maximum
lifetime, subject/client/scope, server ID/pair epoch, and `typ=at+jwt` where the
issuer supports RFC 9068. Never infer algorithm from token preferences or accept
symmetric/browser-secret verification.

Persist last-known-good JWKS. Honor bounded cache headers, coalesce one immediate
refresh on unknown `kid`, rate-limit storms, retain good keys on malformed/empty
responses, and never accept unknown keys or another issuer. Cached keys validate
only otherwise-valid unexpired tokens during outage; no refresh, new auth,
pairing, inventory lookup, or signaling occurs. Established WebRTC may continue
under its negotiated policy, but reconnect requires valid authorization.

Pin issuer identity, not one key. Test Ed25519/EdDSA across Cloudflare `jose`,
Rust, and browsers; ES256 is the fallback. Rotate regularly and retain old public
keys beyond maximum token lifetime, skew, and cache TTL. Emergency compromise
cannot be solved for offline servers beyond short token expiry.

Passkeys use one stable production HTTPS origin/RP ID with isolated staging and
preview credentials. Require user verification and recent reauthentication for
pairing, unpairing, recovery, and ownership-sensitive operations. Passkeys do not
replace PKCE, server keys, or bearer-token protections; do not trust unfinished
Better Auth ACR behavior without proof.

### API/MCP discovery and libraries

Every HTTPS API/MCP resource publishes RFC 9728 protected-resource metadata with
one authorization server, exact resource, supported scopes, and bearer-header
method. Missing/invalid tokens return a Bearer challenge with
`resource_metadata`; insufficient scope returns 403/challenge. Never advertise
both modes.

Target MCP authorization 2025-11-25 with Streamable HTTP, protected-resource and
authorization-server discovery, S256, RFC 8707 resource binding, bearer header,
and scope upgrades. Launch with preregistered first-party clients. Broad client
interoperability remains gated on Client ID Metadata Document or safely bounded
registration; unauthenticated DCR is abuse-prone. WebRTC is not MCP HTTP. A
connected-mode MCP client needs a local HTTPS `/mcp` endpoint or a separately
specified gateway.

Recommended Rust libraries are `openidconnect` 4.0.1, `oauth2` 5.0.0 without
PKCE plain, `jsonwebtoken` 10.4 with Litehouse-owned claim/JWKS policy,
`reqwest`/rustls, and official `rmcp` 2.2. Worker/browser standards work may use
`oauth4webapi`; Better Auth owns central browser sessions/passkeys.

Migration must not combine TanStack/Workers, issuer behavior, Turso-to-D1, or
Cockpit cutovers. Preserve stable issuer/passkey origins; add OAuth schemas
expand-only, qualify behavior against Turso, migrate users/sessions/passkeys/
clients/consents/tokens/JWKS/device rows with counter/time/foreign-key checks,
then cut D1 writes separately with read-only rollback. Never change issuer URL
in place.

Implementation security gates include JWT claim/type/resource behavior, atomic
code consumption and refresh-family replay handling on D1, two-key JWKS rotation
and outage, EdDSA/ES256 interoperability, recent passkey reauthentication,
pairing guessing/replay/key-substitution/concurrency, cross-audience/server/epoch
replay rejection, standalone qualification against representative issuers, MCP
Inspector/client registration and scope flows, shared-cache exclusion, and
account/server-key/clock recovery.

Primary sources include:

- [OAuth Security BCP, RFC 9700](https://www.rfc-editor.org/rfc/rfc9700)
- [Authorization metadata, RFC 8414](https://www.rfc-editor.org/rfc/rfc8414)
- [Protected resource metadata, RFC 9728](https://www.rfc-editor.org/rfc/rfc9728)
- [Resource indicators, RFC 8707](https://www.rfc-editor.org/rfc/rfc8707)
- [JWT access tokens, RFC 9068](https://www.rfc-editor.org/rfc/rfc9068)
- [OIDC Core](https://openid.net/specs/openid-connect-core-1_0.html)
- [MCP authorization 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
- [Better Auth OAuth Provider](https://www.better-auth.com/docs/plugins/oauth-provider)
- [Rust `openidconnect` 4.0.1](https://github.com/ramosbugs/openidconnect-rs/releases/tag/4.0.1)
- [Rust `oauth2` 5.0.0](https://github.com/ramosbugs/oauth2-rs/releases/tag/5.0.0)
