# Inventory the Next.js and Vercel migration surface

Type: task
Mode: AFK
Status: resolved
Resolved: 2026-07-14
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which current site routes, rendering modes, route handlers/server actions,
middleware, Better Auth/passkey flows, database and migration code, MDX/Fumadocs
pipeline, registry integration, React Flow/editor state, Vercel/Next-specific
packages, Node-only APIs, Inngest/email/observability hooks, static assets,
tests, build/deploy assumptions, and public/private cache boundaries must be
preserved, rewritten, replaced, split, or deleted for an evidence-based
Cloudflare Workers migration without absorbing unrelated app work?

## Answer

> **Superseded implementation boundary (2026-07-16):** this remains a historical
> read-only inventory only. The approved greenfield `web/` tranche does not
> import, copy, preserve, or migrate legacy `site/` source, routes, components,
> auth implementation, transport, client state, or Turso/libSQL data. D1 starts
> clean. Every “preserve” statement below means product-journey evidence, not
> implementation or data-migration authority.

A read-only audit of tracked `site/` found a mixed Next.js 16 App Router
application whose React UI/content can largely survive, but whose routing,
rendering, auth bindings, database runtime, registry data path, and control-plane
transport need explicit Cloudflare replacements.

### Route and API inventory

- `/` is a static server-component landing page. Preserve content/components;
  replace Next metadata, navigation, image/font, and route conventions.
- `/docs/**` uses Fumadocs/MDX, forced static rendering, and
  `generateStaticParams`. Preserve authored content/source concepts if the
  chosen framework adapter supports them; rebuild the source/build integration.
- `/registry`, `/registry/[title]`, and `/[title]/[version]` mix client data,
  partial-prerender flags, server metadata, and a hard-coded in-bundle catalog.
  Replace data with the R2 FlatBuffer root/shards and Rust/Wasm client. Split
  public cacheable rendering from personalized controls.
- `/sign-in`, `/sign-up`, `/profile`, `/device`, and `/device/approve` implement
  Better Auth password, magic-link, passkey, bearer, and device-authorization
  flows. Preserve user behavior and durable tables; replace Next handlers,
  middleware, request/redirect APIs, and harden device-approval authorization.
- `/cockpit` and `/cockpit/[slug]` combine authenticated shells, five-second
  polling, and WebRTC client behavior. Split durable inventory UI from live
  private signaling/management. Neither live path is shared-cacheable.
- OpenGraph image routes use `next/og` and local fonts; replace with target
  generation or static assets.
- `GET/POST /api/auth/[...all]` is a Better Auth Next handler.
  `/api/feedback` validates with Zod but only logs to Axiom.
  `/api/search` and `/api/registry/search` duplicate Fumadocs indexes.
  `/api/registry` is an incomplete mixed Pages/App Router PUT stub and should be
  deleted rather than migrated. No server actions were found.

### Preserve, rewrite, replace, and split

Preserve React/Tailwind UI behavior, MDX content, Better Auth concepts/schema,
passkey and device flows, React Flow editor behavior, and local IndexedDB
manifest selection.

Rewrite App Router layouts/pages/metadata, middleware, `toNextJsHandler`,
`NextResponse`, `headers()`/`redirect()`, Next fetch revalidation,
`next/image`, `next/font`, `next/og`, `next.config.mjs`, and Axiom framework
bindings. The public registry layout currently invokes an authenticated Toolbar
that reads request headers/session, unnecessarily making public pages dynamic;
move personalized controls behind an uncached auth/client boundary.

Replace the Vercel toolbar, hard-coded registry catalog, target-specific OG
rendering, and `next-axiom` integration if the new observability path differs.
Split public/static UI from auth/database runtime, Cockpit UI from signaling,
and anonymous registry reads from publication writes. Consolidate duplicate
search routes and delete the broken registry PUT stub. Generated Fumadocs
`.source` output must be regenerated rather than manually migrated.

`inngest`, `resend`, legacy `reactflow` v11, and `litehouse-registry-js` appear
unused and need confirmation before deletion. The Rust/Wasm registry package
supersedes the N-API dependency.

### Durable state and hazards

Better Auth uses Drizzle with Turso/libSQL. Tracked migrations cover users,
accounts, sessions, verification, device codes, and passkeys. The current
`scripts/migrate.ts` is effectively a no-op and incorrectly claims first-use
table creation; migration ownership must be replaced with a real deploy gate.

Passkeys require stable HTTPS origin/relying-party semantics;
`NEXT_PUBLIC_APP_URL` currently controls client URL/trusted origin. SMTP uses
Nodemailer/Fastmail. A credential-like fallback exists in tracked source; its
value was not read into this answer and belongs to the separately approved
credential-remediation ticket. Node/build dependencies include filesystem MDX
configuration, Nodemailer, Drizzle tooling, environment variables, and
`next/font/google` despite bundled Inter files.

Cockpit uses a generated Fets client against a fixed remote endpoint, polling,
WebRTC data channels, and public Google STUN. Several paths use user/node IDs as
bearer values; this authorization contract must be replaced before migration.
React Flow state is memory-only, while manifest selections use
IndexedDB/BroadcastChannel.

### Cache boundaries

Public/cacheable surfaces are landing/docs pages, registry catalog/detail/
version pages, consolidated public search, OG images, and static assets. Some
docs data currently fetches GitHub release/OpenGraph content at build time and
needs bounded lazy/cache behavior.

Authenticated durable surfaces are profile, device authorization, auth/session/
passkey/verification records, and server inventory. Entry pages may be public,
but stateful responses and mutations are never shared-cacheable.

Live private/no-cache surfaces are Cockpit presence/polling, WebRTC signaling and
logs, auth mutations, device approval/denial, feedback writes, registry
publication, and future live state/config/rule/command traffic.

### Validation gap

The site has no tracked unit, component, Playwright, Vitest, or Jest suite.
Current commands are:

```bash
pnpm --filter site lint
pnpm --filter site typecheck
pnpm --filter site build
pnpm --filter site db:generate
pnpm --filter site db:migrate
```

`next.config.mjs` currently ignores TypeScript build errors, so `build` alone is
not an authoritative gate. A migration needs independent lint/typecheck/build,
route/API contract tests, auth/device/passkey integration tests, Workers local
integration tests, accessibility checks, cache-isolation/invalidation tests,
and preview/rollback evidence. No commands were run during this read-only audit.
