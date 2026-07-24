# Research Cloudflare React framework and cache options

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-14
Blocked by: 45, 46
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

How do current React Router framework mode and TanStack Start compare on
Cloudflare Workers for Litehouse's Next.js replacement, including official
support/maturity, SSR and streaming, static prerendering, route loaders/actions,
server functions, Workers Cache entrypoints and tag invalidation, R2/D1/Durable
Objects/service bindings, Better Auth/passkeys, Rust/Wasm loading, MDX/docs,
WebSockets, Node compatibility, type safety, testing, observability, preview and
rollback workflows, bundle/CPU limits, and migration risk—while aggressively
caching public data and never caching private live management traffic?

## Answer

Research against React Router 8.2.0, TanStack Start 1.168, current Cloudflare
Workers/Cache/Static Assets, Better Auth 1.6, Fumadocs, D1, Durable Objects, and
Workers deployment documentation favors **TanStack Start**, gated by a deployed
prototype because TanStack still labels Start release-candidate software.

TanStack Start has first-party Cloudflare Vite support, official static
prerendering, selective per-route SSR, streaming/deferred data, typed validated
server functions/search/params, lazy route splitting, and official Better Auth
and Fumadocs integrations. These align directly with Litehouse's static public,
cached derived-public, authenticated durable, and private live route classes.
Pin exact versions and rehearse upgrades.

React Router framework mode 8.2 is the GA, lower-churn fallback with mature
Remix-derived loaders/actions/forms and stronger testing history. However,
Cloudflare's current official Vite guide explicitly does not support SPA mode or
prerendering, forcing a separate static build/workaround for Litehouse's public
site and docs. Switch to React Router if TanStack's prototype, RC lifecycle,
server-function version affinity, auth behavior, or cache integration fails.
Do not depend on TanStack Nitro, React Router RSC, or community Cloudflare
adapters while first-party paths exist.

### Target topology

Use two independently deployable Workers:

- `litehouse-web`: TanStack Start public site/docs/registry, Better Auth,
  account/server inventory, short-lived signaling grants, D1, R2, static assets,
  Rust/Wasm registry core, and a private service binding to Cockpit.
- `litehouse-cockpit`: framework-independent Worker with one Durable Object per
  opaque server ID, hibernatable signaling WebSockets, bounded presence and
  offer/answer/ICE relay, and no routine management payloads or home secrets.

Separating Cockpit matters because Durable Object migrations and preview/rollback
behavior differ from ordinary framework Workers. Routine management remains
direct WebRTC after signaling.

### Rendering and caching

Classify every route explicitly:

1. **Public deployment-static:** marketing, committed docs/MDX, legal/help,
   static shell, hashed assets. Prerender with TanStack and serve asset-first
   through Workers Static Assets. Hashed assets are year-long immutable; builds
   fail if static routes depend on production bindings or secrets.
2. **Public registry-derived:** browse/search/package/version HTML/JSON and OG
   data. SSR lazily from R2 through the Rust/Wasm registry core, then use Workers
   Cache in front of the renderer with canonical path/query/representation keys,
   bounded package/root tags, short TTL plus stale-while-revalidate, and purge
   only after successful R2 root ETag CAS. Immutable blobs/shards are direct CDN
   responses and never purged.
3. **Authenticated durable:** auth/passkeys, profile, pairing/device approval,
   inventory, grant issuance, and every mutation. Always invoke the Worker with
   `private, no-store`; perform server-side session/authorization/revision/
   idempotency checks. TanStack's client cache is UX state, never authority.
4. **Private live:** presence, signaling, live state/config/rules/commands/logs,
   and WebRTC. Never shared-cache. SSR only a stable authorized shell/inventory,
   lazy-load React Flow, WebRTC, editors, and diagnostics, then connect from the
   client.

Workers Cache/response-entry caching is distinct from `caches.default`. Use the
former so hits avoid Worker execution and support tiering/SWR/tag purge. Do not
enable cache indiscriminately for auth gateways, static assets, signaling, or
private management. Cache keys must be deployment/schema-versioned so rollback
never consumes incompatible cached responses.

### Data, auth, and platform boundaries

D1 is the recommended final durable store for Better Auth and account/server
inventory. Better Auth accepts D1 bindings and Drizzle supports D1, while the
current data is SQLite-shaped. Keep auth/authorization reads on the primary path
until D1 Sessions/read-replication semantics are proven.

Do not combine framework and database cutovers. First run TanStack/Workers
against existing Turso through a fetch-compatible client. Migrate to D1 in a
separate expand/contract phase with export/import checks, passkey representation
and counter validation, session-expiry verification, bounded maintenance or
dual-write window, and read-only Turso rollback. `@libsql/client` cannot simply
point at D1; final code uses the D1 Drizzle adapter. Hyperdrive is irrelevant
unless Litehouse later adopts external Postgres/MySQL.

Pin Better Auth 1.6.x initially and prove password/magic-link, passkeys,
device authorization, bearer sessions, cookie integration, D1 plugin tables,
session invalidation, and secret rotation. Passkeys need stable HTTPS RP IDs;
random preview URLs cannot reuse production credentials. Use isolated staging
origins/secrets. Prefer an HTTP email provider if Nodemailer/SMTP compatibility
is incomplete.

Durable Objects are one-per-server serialized signaling coordinators using
WebSocket Hibernation and signed short-lived grants from `litehouse-web`.
Application message limits must be far below Cloudflare's platform maximum.
Clients reconnect/renegotiate after deployment; ordinary non-hibernating
WebSockets are not acceptable for sparse long-lived presence.

The Rust registry query core must be pure browser-style Wasm, not WASI 0.3.
Cloudflare imports a `WebAssembly.Module`; it does not support streaming
instantiation, threads, or the local Component Model runtime. Prototype bundle
size, one-second startup, 128 MiB isolate memory, malformed FlatBuffers, and
concurrent isolate behavior.

Fumadocs supports official TanStack/Vite integration, so preserve content and
build-time MDX/Shiki compilation while replacing Next-specific source, routing,
metadata, image, OG, and middleware APIs. Lazy-load only interactive examples.

### Validation and rollout

Mandatory framework gates are:

- public Fumadocs prerenders while private routes stay dynamic;
- bundle/startup/Wasm memory fit Cloudflare limits with margin;
- Better Auth passkey, magic-link, device, bearer, and cookie flows pass;
- no response that reads session/cookie/authorization can enter shared cache;
- cache tags purge globally after publication and TTL bounds purge failure;
- native/Wasm registry fixtures are byte-identical and fail closed;
- Turso-to-D1 migration/rollback preserves auth and plugin records;
- per-server Durable Objects hibernate, authorize, overload safely, and reconnect;
- previews/gradual deployment preserve server-function and cache-version affinity;
  and
- independent lint, typecheck, Worker Vitest integration, browser journey tests,
  production build, staging cache/purge, and rollback checks pass.

Migrate reversibly: prototype; public static routes; public registry rendering;
framework auth against Turso; separate D1 migration; private profile/pairing/
inventory; separate Cockpit Worker/DO; final DNS cutover. Keep Vercel and prior
Worker versions through soak. Worker rollback does not roll back D1/R2/DO state,
so schemas are expand/contract and Durable Object migrations must preserve the
rollback window.

Primary sources:

- [React Router 8.2](https://github.com/remix-run/react-router/releases/tag/react-router%408.2.0)
  and [Cloudflare guide](https://developers.cloudflare.com/workers/framework-guides/web-apps/react-router/)
- [TanStack Start Cloudflare guide](https://developers.cloudflare.com/workers/framework-guides/web-apps/tanstack-start/)
  and [hosting](https://tanstack.com/start/latest/docs/framework/react/guide/hosting)
- [Workers Cache](https://developers.cloudflare.com/workers/cache/),
  [Static Assets](https://developers.cloudflare.com/workers/static-assets/),
  and [versions/rollback](https://developers.cloudflare.com/workers/configuration/versions-and-deployments/)
- [D1](https://developers.cloudflare.com/d1/),
  [Durable Objects](https://developers.cloudflare.com/durable-objects/), and
  [service bindings](https://developers.cloudflare.com/workers/runtime-apis/bindings/service-bindings/)
- [Better Auth TanStack integration](https://www.better-auth.com/docs/integrations/tanstack)
  and [passkeys](https://www.better-auth.com/docs/plugins/passkey)
- [Cloudflare Wasm](https://developers.cloudflare.com/workers/runtime-apis/webassembly/)
- [Fumadocs TanStack installation](https://fumadocs.dev/docs/manual-installation/tanstack-start)
