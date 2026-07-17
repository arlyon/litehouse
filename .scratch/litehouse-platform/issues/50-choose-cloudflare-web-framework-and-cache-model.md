# Choose Cloudflare web framework and cache model

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 46, 48, 49
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Should Litehouse replace Next.js/Vercel with gated TanStack Start or React Router
on Cloudflare Workers, and which public-static, Workers-Cache-rendered,
authenticated-durable, and private-live route classes, lazy loading boundaries,
R2/Rust-Wasm registry integration, framework fallback gates, test seams, phased
cutover, and rollback rules form the web migration contract before database,
auth, and Cockpit hosting details are finalized?

## Answer

Replace Next.js/Vercel with exact-pinned **TanStack Start on Cloudflare
Workers**, subject to objective first-tranche gates. React Router 8 is the
preselected GA fallback if TanStack's release-candidate maturity, Cloudflare
prerendering, Better Auth, Workers Cache, server-function deployment affinity,
or production build behavior fails those gates. Do not retain a permanent dual-
framework architecture.

TanStack is preferred because its first-party Cloudflare Vite path supports
static prerendering, selective SSR, streaming/deferred data, validated typed
server functions/search/params, route splitting, Better Auth, and Fumadocs.
Pin the framework, router, Cloudflare Vite plugin, Better Auth, and build tooling
as one tested compatibility set and rehearse upgrades before changing it.

### Route and rendering contract

Every route belongs to one explicit class. Shared layouts may not silently
promote a public route into a session-dependent dynamic route.

1. **Public deployment-static:** marketing, committed docs/MDX, legal/help,
   static shell, and hashed assets. Prerender at build and serve asset-first
   through Workers Static Assets. Hashed assets use year-long immutable caching;
   builds fail if a static route depends on runtime secrets, sessions, D1, or
   production-only bindings. Preserve Fumadocs content through its Vite/TanStack
   integration and compile MDX/Shiki at build time.
2. **Public data-derived:** registry browse/search/package/version pages, public
   registry JSON, and derived OG data. SSR lazily only on a cache miss from R2
   FlatBuffer root/shards through the Rust/Wasm registry core. Workers Cache sits
   in front of this renderer with canonical path/query/representation keys,
   deployment/schema namespacing, bounded `registry`/package/release tags, short
   freshness plus stale-while-revalidate, and purge only after successful R2
   root ETag CAS. Immutable shards/blobs remain direct CDN objects and are never
   purged. A purge failure retries and TTL bounds stale derived output.
3. **Authenticated durable:** auth entry pages, profile, pairing/device approval,
   account/server inventory, and signaling-grant issuance. The outer gateway
   always runs and validates the session/authorization; auth handlers and every
   mutation are `private, no-store`. Stable per-user inventory read models may
   use a gated inner Workers Cache entrypoint keyed by opaque user identity in
   `ctx.props`, schema/deployment version, and canonical request. Writes purge
   the user-scoped tag and a short TTL bounds failure. Cached read models never
   authorize a request or contain tokens/secrets/live presence.
4. **Private live:** presence, signaling, live home state/config/rules/commands/
   logs, WebRTC, and editor sessions. Never shared-cache. Render only a stable
   authorized shell and durable inventory snapshot, then lazy-load React Flow,
   WebRTC, editors, traces, and diagnostics after authorization. Routine
   management travels directly over WebRTC after signaling.

Do not rely on `Vary: Cookie` for tenant isolation. Prove that any route reading
a session, cookie, bearer token, or tenant state either bypasses shared cache or
uses the explicitly reviewed authenticated gateway/`ctx.props` read-model path.
Workers Cache response-entry caching is the public/private-derived cache;
`caches.default` is not substituted because it lacks the same tiering/SWR/purge
semantics.

### Laziness and dependency boundaries

Route modules split automatically. Public static routes ship no auth, D1,
WebRTC, React Flow, or registry-query code unless used. Registry pages fetch only
the root and required shard for exact lookup; browse/search may load all seven
verified shards on first cached render. Heavy authenticated modules load only
after route authorization and user intent. Deferred data must render stable
loading/error boundaries and cannot defer an authorization check.

One pure Rust registry core compiles to browser-style Wasm and runs identically
in Worker/server and browser-compatible build contexts; thin Fetch/R2/cache
adapters own bytes. Do not load WASI 0.3 components into Workers. Native local
runtime remains separate.

### Greenfield cutover and fallback

The later approved production boundary supersedes source/state migration: build
`web/` greenfield and never import, copy, or migrate legacy `site/` routes,
components, auth implementation, transport, client state, or Turso/libSQL state.
D1 begins as a clean authority. The legacy deployment may remain only as an
independent rollback origin during bounded cutover; it is never a route/runtime
dependency of the Worker.

1. pass the local route/cache/token/Worker gate without external mutation;
2. build fresh public docs/registry routes and lazy R2/Rust-Wasm rendering;
3. build fresh Better Auth/OAuth/passkey flows and D1 inventory on isolated new
   bindings;
4. add connected private inventory, then the separate signaling-only Cockpit
   Worker;
5. validate the complete greenfield journey on a separately approved staging
   origin; and
6. cut over routes/DNS only after evidence review, retaining previous Worker
   versions and the independent legacy deployment for the explicit rollback
   window.

Do not combine framework, auth-origin, D1, Cockpit, or DNS cutovers. Preview
and staging use isolated databases/buckets/secrets/email behavior and stable
staging passkey credentials; random preview URLs never bind production auth.
Cloudflare code rollback does not roll back D1/R2/Durable Object state, so later
schemas use expand/contract compatibility.

Framework qualification is two-stage. T4 is only the local build/workerd/
prerender/accessibility candidate gate. It does not finalize TanStack. T12 adds
integrated auth, D1, R2/Wasm, private-cache, and connected-journey evidence, and
T14 repeats the complete local candidate journey/build. TanStack becomes final
when those selected local gates pass; later external T15 staging is not a
framework-selection prerequisite.

Switch the implementation plan to React Router 8 rather than extending the
TanStack experiment if any of these framework gates fail:

- official Cloudflare build cannot prerender bounded Fumadocs/public routes while
  leaving private routes dynamic;
- the required Better Auth magic-link/passkey/device/bearer/cookie journeys fail;
- public/private cache segregation or tag purge cannot be proven remotely;
- gradual deployment serves incompatible TanStack server-function IDs/assets;
- Rust/Wasm registry behavior fails, workerd cannot start the built Worker, or
  provider validation rejects it against the selected Cloudflare plan's
  published hard CPU/memory/bundle limits (no custom performance threshold);
- exact-pinned lint/typecheck/tests/production build is unstable; or
- at T12 lock, any required TanStack/Cloudflare integration package uses a SemVer
  prerelease, declares incompatible peer/runtime ranges, or has an applicable
  unmitigated security advisory.

React Router fallback keeps the same Fetch, route-class, auth, cache, R2/Wasm,
D1, and Cockpit contracts, but uses a separately generated static surface until
Cloudflare prerender support is adequate.

### Validation contract

The first approved greenfield tranche must add stable tests before route cutover:

- pure cache-key/tag/route-class and authorization-boundary tests;
- TanStack loader/server-function/redirect/error/serialization tests;
- `@cloudflare/vitest-pool-workers` integration tests for bindings, R2 and cache
  adapters;
- native-versus-Wasm registry golden/malformed fixtures;
- browser journeys for sign-in, passkeys, device approval, registry, and lazy
  private-shell loading;
- deployed staging evidence for Workers Cache hit/bypass, cross-user isolation,
  purge/TTL failure bounds, Static Assets routing, previews, gradual deployment,
  and rollback;
- WCAG 2.2 AA keyboard/focus/semantic/reduced-motion checks; and
- independent lint, typecheck, Worker tests, browser tests, and production build
  with no ignored TypeScript errors.

The legacy journeys and content inventory are requirements evidence only, not
source or state migration inputs. The approved Open Design package is the visual
source and behavior remains local to greenfield React. Delete the broken registry
PUT stub, duplicate search paths, and Vercel/Next tooling with legacy retirement
rather than porting them. N-API is superseded by the Rust/Wasm registry client.
