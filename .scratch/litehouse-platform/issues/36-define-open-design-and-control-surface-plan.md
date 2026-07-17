# Define Open Design and control-surface adoption

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-15
Blocked by: 01, 28, 29, 31, 44, 47
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

How is the separate Litehouse Open Design package owned/versioned/consumed, and
how should login/pairing, registry install/update review, server inventory,
services/state, React Flow rules, AI/API audit, health, errors, offline modes,
and advanced diagnostics form one WCAG 2.2 AA web journey with structured API
parity and two deliberate information densities?

## Answer

Use the reviewed Litehouse Open Design bundle as the candidate visual source,
but keep it a static design input rather than a runtime component library. Build
one control-surface model with two route-selected densities and two transport
shells: the `litehouse-web` authenticated shell uses central inventory plus
direct WebRTC management; standalone uses direct local HTTPS against its one
implicit server. Both shells render the same server-scoped React components and
call the same generated typed management client. `litehouse-cockpit` remains a
signaling-only Worker with no HTML routes. Do not duplicate domain behavior,
invent a central management proxy, or generate generic forms/screens from
arbitrary plugin UI.

The reviewed bundle establishes the approved visual direction, not an accessible
release or every product interaction. Its existing buttons, fields, badges,
notices, tables, package rows, loading states, warm-neutral palette, Litehouse
green, typography, and focus/motion direction are the baseline. Navigation,
capability controls, pairing, install review, rule editing, operation outcomes,
and diagnostics remain application-owned patterns governed by this contract.

### Package ownership, versioning, and consumption

`open-design/design-systems/litehouse/` is the canonical portable source package.
The Open Design repository owns its package schema, validators, fixtures, and
previews; Litehouse product review owns whether a visual or semantic change is
accepted; the Litehouse web source owns accessible React/Radix implementations.
`components.html` is a review fixture, never code copied into production and
never a second component implementation.

The present package is untracked and therefore not yet a release. Tracking,
tagging, modifying, or publishing it remains a separately approved Open Design
repository change. Its first release is blocked until it:

- adds a distinct underlined text-link role using `#008138` on the light canvas
  and `#00C758` on the dark canvas, each verified at 4.5:1 or better, while the
  brighter accent remains a fill/selection role;
- makes the fixture consume canonical `tokens.css` instead of duplicating light
  tokens, demonstrates light and dark states, and provides a visible native
  outline under forced colors rather than relying on box shadow; and
- records automated contrast/structure checks plus manual keyboard, dark-mode,
  and forced-colors evidence without claiming that token coverage alone proves
  accessibility.

Once those gates pass, version the design release by an Open Design semantic Git
tag and immutable source commit. Its generated `bundle-files.json` lists one
release version and every other bundle file as a sorted safe POSIX relative path,
byte length, and SHA-256 over exact bytes. It includes only regular files, rejects
symlinks and unlisted files, and is canonicalized with RFC 8785 JCS; the bundle
SHA-256 is the digest of those canonical manifest bytes. Litehouse vendors only
that closed file set under `design-system/litehouse/` and records repository
identity, tag, commit, package path, and bundle digest in one checked-in
`design-system.lock.json`.

Git plus the file manifest and lock is the update mechanism; there is no npm
publication, live daemon dependency, Git submodule, runtime fetch, automatic
synchronization, or custom package manager. Release validation also bounds files
and sizes and scans the closed bundle for credentials before vendoring.

The application imports only the vendored `tokens.css` through a small
Litehouse-owned semantic alias layer. `tailwind-v4.css`, `design-tokens.json`,
`DESIGN.md`, previews, and evidence remain review/build inputs. React primitives
stay local and map their existing background/foreground/primary/status/link
aliases to the canonical tokens. An update is deliberate: validate and visually
review the Open Design package, tag it, update the vendored snapshot and lock in
one Litehouse change, inspect the complete token/component diff, run UI and
accessibility checks, and retain the previous Git revision for rollback. Never
silently merge raw token values or let generated files become a second source of
truth.

The approved visual direction does not require the Open Design fixture to grow
into a full product component library. Reusable brand-level patterns may be
upstreamed in separately reviewed Open Design changes; product-specific behavior
and tests remain in Litehouse.

### Information architecture and route ownership

Keep public browsing, durable account work, and private live management visibly
separate. `litehouse-web` owns every central HTML route and the authenticated
server shell:

```text
/
/docs/*
/registry
/registry/:package
/registry/:package/:version
/sign-in
/recovery
/device                                     RFC 8628 CLI verification
/servers                                    durable owner inventory
/servers/pair                               cloud pairing
/account/security                           passkeys, sessions, recovery
/servers/:server-id                         authorized live shell/overview
/servers/:server-id/devices                 configured physical devices
/servers/:server-id/devices/:instance-id    config, facets, state, health
/servers/:server-id/automations             rule inventory
/servers/:server-id/automations/:rule-id    draft/editor/runs
/servers/:server-id/plugins                 packages and updates
/servers/:server-id/activity                runs/current-session operations
/servers/:server-id/settings                server-safe settings
/servers/:server-id/settings/diagnostics    bounded advanced diagnostics
```

`litehouse-cockpit` owns only signaling and presence routing through its
per-server Durable Object. TURN is deferred. It has no browser route, Better Auth cookie,
HTML renderer, app assets, or management UI. After `litehouse-web` authorizes and
renders the stable private shell, lazy leaf components obtain the short-lived
grant and communicate directly with the server over WebRTC. Routine management
bytes never pass through either central Worker.

These are product route identities, not permission boundaries. Every loader and
mutation still authorizes through the canonical operation seam. Public registry
pages remain anonymous and cacheable; selecting **Install** authenticates, asks
for one owned server, and enters that server's reviewed install plan. No token,
private address, desired config, or plan authority travels in a URL.

Standalone local HTTPS serves a static client build from the same UI source and
container release, with one implicit server and direct transport. It presents
`/`, `/devices`, `/automations`, `/plugins`, `/activity`, and `/settings` without
a redundant selector or Cloudflare/D1 route logic. Shared components and the
generated client are one implementation; only route bootstrap, auth, and
transport selection differ. Cloud-only account, inventory, pairing, presence,
and signaling routes are absent. `unsecure-local` uses that local shell with a
persistent unsafe-mode banner and never exposes sign-in, pairing, Cockpit,
central inventory, or MCP.

Use **Devices** for configured one-physical-device instances and show their typed
capability facets within the device detail. Use **Plugins** for installed package
artifacts and release lifecycle. Do not introduce a second integration-level
device container, a discovery page, or a generic dashboard full of KPIs. Health
summaries belong on overview/device rows; privileged rawer detail belongs under
Diagnostics.

### Two deliberate information densities

Density follows task context and is not a user preference, global toggle, or
separate theme:

1. **Guided density** — public pages, authentication, recovery, pairing,
   install/update review, destructive confirmation, empty states, and settings.
   Use 44-pixel controls, 52-pixel-or-larger primary rows, 16/24-pixel grouping,
   plain-language explanation, and one obvious next action.
2. **Operational density** — device/capability rows, automation inventories,
   operation history, rule traces, and diagnostics. Use 40-pixel controls/rows,
   8/12-pixel grouping, stable columns, compact status text, and progressive
   disclosure. Metadata may use the package's 12-pixel scale; actionable labels
   do not.

Both densities use the same color, type, radius, elevation, focus, and semantic
component tokens. Compactness never reduces the interactive target below the
component row, hides required state, or turns a complex control into an icon-only
mystery action. Touch layouts use at least the guided 44-pixel target. A guided
page may contain an operational table, but nested arbitrary density overrides
are forbidden.

### Critical journey matrix

- **Central sign-in:** A verified magic link may bootstrap or recover an
  account; a new owner must enroll a user-verifying passkey before
  ownership-sensitive work. Existing owners prefer passkey sign-in. Restricted
  and recovery-hold states explain unavailable actions without leaking account
  existence.
- **Standalone sign-in:** Redirect through the one configured external issuer
  and return to the local BFF cookie session. First ownership remains pending
  until the local operator confirms the exact subject and one-use bootstrap
  code. No cloud inventory or pairing UI appears.
- **Pair a cloud server:** From empty or inventory state, instruct the operator
  to run `litehouse pair`; accept nearby convenience or a manual six-digit code;
  require recent passkey assertion; and show the proposed name plus grouped
  server fingerprint for human comparison. Proof, server acknowledgement,
  activation, cancellation, expiry, attempt exhaustion, and restart
  reconciliation are distinct states. Completion means central inventory and
  server both report the signed active binding, not merely that the code matched.
- **Browse and install:** Anonymous registry detail shows exact version, origin,
  signature/trust, compatibility, contract/runtime requirements, withdrawal or
  revocation, and release notes. Authenticated install selects one server, calls
  the canonical plan operation, shows exact digest, permissions/grants, affected
  instances, impact, and blockers, then explicitly applies the unchanged plan
  and revision. Completion means verified bytes are installed; it does not
  create a physical-device instance.
- **Configure a device:** Start from an installed exact plugin release; create a
  host-ID draft for one physical device; render only closed schema annotations;
  collect bounded non-secret connector config and exact grants; and show each
  environment field's deterministic variable name and status without accepting
  a value. Shape validation executes no guest. Explain that activation performs
  real connector work, then activate the hidden generation. Completion means the
  active pointer commits and typed facets become available. Offline config may
  be saved disabled and unverified.
- **Observe and control:** Hydrate current snapshots before enabling controls;
  group independently bindable facets under their configured device; show
  availability, freshness/gap, feature support, generation, and safe units; and
  issue exactly one typed operation with generated idempotency/revision metadata.
  Present accepted, acknowledged, completed, observed-completed, rejected,
  cancelled, and ambiguous outcomes honestly. A successful request never alone
  claims that physical state changed.
- **Author an automation:** Create or branch an immutable draft; edit the same
  Rule JSON through canvas plus synchronized outline/inspector; validate shape,
  types, bindings, features, grants, units, and CEL; simulate with explicit
  hypothetical state/receipts; show a semantic diff excluding layout; save
  without activating; and explicitly activate against the expected base
  revision. Completion means one immutable revision is active. Enabled, device
  health, binding health, and run health remain separate.
- **Update or roll back:** Plan the exact release/digest change, trust and
  compatibility result, semantic config/grant diff, and only affected instances
  and stores. Apply with expected desired/active revisions, preserve the last
  active version on failure, and show pending versus active. Rollback revalidates
  the retained predecessor and never claims to undo physical effects or state.
- **Activity and run investigation:** Compose only data available through the
  closed manifest: durable rule runs from `rules.runs.page@1`, operation cards
  whose idempotency keys this browser session already holds using
  `operations.result.get@1`, and—only with `litehouse:diagnostics`—allowlisted
  events from `diagnostics.events.page@1`. Filters apply only to fields exposed
  by each source. Do not enumerate the result store, imply durable operation
  history after the browser forgets a key, or create an audit database. Safe
  diagnostics may identify an MCP/API client, but no AI prompt, transcript,
  token, raw arguments, or secret-bearing payload is stored or displayed.
- **Health and diagnostics:** Overview gives actionable summaries and links to
  the affected device, plugin, or rule. Diagnostics requires
  `litehouse:diagnostics`, uses bounded allowlisted queries, and displays typed
  error/correlation IDs and truncation. It never becomes raw log tail, SQL,
  filesystem, environment, process, stack-dump, or arbitrary tracing access.
- **Recovery or trust change:** Passkey recovery follows the delayed, notified
  hold. Unpairing and credential changes require recent passkey proof. Issuer or
  trust-mode switching, identity-key reset, and forced local cleanup remain
  explicit host commands and are not web/API/MCP buttons.

### State, error, and offline vocabulary

Every data-bearing surface must distinguish these states rather than collapsing
them into loading, success, or failure:

- **loading** — bounded skeleton or progress label; preserve layout and announce
  completion once, not every streamed update;
- **empty** — explain why no resource exists and offer only the authorized next
  action;
- **unknown/uninitialized** — no valid observation has arrived; never render a
  false zero/off value;
- **current** — show generation/revision and freshness only where operationally
  useful;
- **stale** — retain the last safe value with its age/reason; do not present it
  as current;
- **gap/resync required** — stop applying incremental events and perform complete
  hydration;
- **offline** — distinguish central service unavailable, server unreachable, and
  provider/device unavailable;
- **reconnecting** — show which link is being restored and keep unsafe mutations
  disabled until authorization plus hydration complete;
- **degraded/partial** — identify the unavailable facet/plugin/rule without
  treating the whole server as healthy or failed;
- **desired pending versus active** — show unapplied config/plugin/rule revisions
  and the last successfully active revision together;
- **operation running** — retain correlation/idempotency identity and provide
  result lookup rather than a second dispatch;
- **terminal known** — preserve typed receipt confidence and observed-state
  reconciliation separately;
- **terminal unknown** — state that an effect may have occurred, disable blind
  retry, and offer result lookup plus safe observation/reconciliation guidance;
- **invalid, unsupported, unauthorized, forbidden, conflict, not found,
  capacity-limited, deadline, cancelled, and internal fault** — map the shared
  safe error envelope to stable headings, field/node spans, retry advice, error
  ID, and one appropriate recovery action.

Use a persistent page banner for transport/auth mode and route-wide failures,
inline state adjacent to the affected control for validation or operation
results, and a dialog only for an interruptive destructive decision. Toasts may
confirm a noncritical background transition but are never the sole error,
ambiguity, or completion record. Color always has text plus icon/shape. Safe
user/plugin labels are untrusted text and never render as HTML/Markdown.

Cloud Cockpit may show durable server inventory while a server is offline, but it
must not fabricate live device state or queue management mutations. If an
already-authorized WebRTC channel survives a central outage, it may continue only
until its current token lease; no new signaling, grants, pairing, or refresh
starts. Standalone issuer outage follows cached-key/token-expiry rules. Reconnect
always authenticates a fresh JSON-RPC session and reloads the current snapshots.
No UI automatically replays an ambiguous mutation.

Public/static pages and immutable registry assets may work from their declared
cache. Private management responses, Rule JSON, instance config, diagnostics,
and operation results remain `no-store` and memory-bounded in the browser. Do not
add offline/PWA mutation queues or persist access tokens, private addresses,
protected diagnostics, environment status, or sensitive payloads in local
browser storage. Non-sensitive theme and editor-presentation preferences may be
local.

### Component and interaction contract

Retain the bundle's current primitive direction and implement the missing product
patterns locally with semantic HTML and existing Radix-class primitives where
they solve focus/overlay behavior. The required inventory is:

- public header, authenticated app shell, breadcrumbs, responsive navigation,
  server switcher, tabs, and skip link;
- button/link, text field, textarea, select/combobox, checkbox, radio, switch,
  slider, segmented choice, search, and field-level diagnostics;
- dialog, alert dialog, popover, menu, tooltip, drawer/sheet, and disclosure;
- step/progress treatment for pairing/install/activation and persistent notices;
- empty, loading, stale, offline, reconnecting, partial, forbidden, not-found,
  and fatal route states;
- device/facet rows and typed On/Off, Level, Color, Thermostat, Covering,
  Momentary, Notification, Boolean State, and Measurement presentations that
  appear only when advertised features support them;
- package/release rows, trust/signature/compatibility status, permission and
  semantic diff, affected-instance list, and destructive cascade review;
- rule node, typed port, edge, node palette/search, outline, inspector,
  diagnostics summary, semantic diff, simulation input/result, run timeline,
  and trace table;
- operation result, activity timeline, bounded diagnostics table, cursor/gap,
  truncation, and copy-safe error/correlation ID patterns.

Do not use plugin-provided HTML, JavaScript, React, executable validation, or
open-ended UI schemas. Closed first-party config/capability/schema annotations
may select one reviewed host renderer. Unknown fields/kinds fail closed or use a
safe read-only structured view; they never execute arbitrary presentation code.
Avoid decorative charts, card grids for ordinary lists, nested cards, icon-only
primary actions, and globe/search animations as substitutes for real inventory.

### Accessible Rule JSON editing

React Flow remains the spatial projection, not the only editor. One editor state
and command model drives both canvas and a synchronized semantic outline/list.
Every operation—insert node, configure, connect typed ports, choose branch order,
reorder, disconnect, delete, inspect diagnostics, simulate, save, and activate—
must be possible without dragging or precise pointer movement. This satisfies
WCAG 2.2 dragging alternatives without maintaining a second persistence format.

Canvas nodes expose meaningful names, kind, status, connected inputs/outputs, and
selection. Keyboard commands move focus predictably among palette, graph,
outline, inspector, and diagnostics. Creating/removing a node or edge moves focus
to the resulting object or safe predecessor; validation links focus the exact
node/field/port. Zoom and pan have labeled controls and never trap page zoom.
Animation is nonessential and disabled under reduced motion.

At narrow widths, the outline plus inspector is the primary editor and the canvas
is an optional full-screen view. The same Rule JSON, compiler, undo/redo command
history, draft revision, and diagnostics back both. No mobile-only rule format,
React Flow object persistence, or silent semantic merge is allowed.

### Structured API and MCP parity

`management-operations-v1.json` from **Define the AI and MCP management surface**
is the closed parity authority. Generated HTTP/WebRTC clients and UI operation
hooks carry canonical input/output/error types, scopes, revisions, deadlines,
idempotency, result lookup, and redaction. Components never call plugin stores,
R2, D1, Durable Objects, WebRTC frames, or ad hoc REST paths directly.

- **System and health:** Server overview, connection banner, and schema/help
  details use `system.get@1`, `system.health.get@1`, and `schemas.get@1`, with
  safe bounded summaries only.
- **Instances/devices:** Device list/detail, config draft, activation, disable,
  and delete use exact `instances.*@1` operations. Schema validation is pure,
  activation is real, cascade is explicit, and private/environment fields follow
  their projections.
- **Capabilities:** Facet rows and controls use `facets.*@1` snapshots/streams and
  one exact feature-gated capability operation. Generation, deadline,
  idempotency, receipt, and ambiguity are preserved.
- **Rules:** Automation list/editor/simulation/runs use exact `rules.*@1`
  operations. Immutable revisions, base conflicts, simulation non-dispatch,
  activation, and run semantics remain visible.
- **Registry/plugins:** Public browse stays separate from local
  `registry.*@1` and `plugins.*.plan/apply@1`, which own the verified cache and
  affected-store lifecycle.
- **Results/activity:** A known operation card uses
  `operations.result.get@1`; durable lists use `rules.runs.page@1`; privileged
  structured events use `diagnostics.events.page@1`. There is no general
  operation-results listing. Reconnect queries known results rather than
  replaying mutations.
- **Diagnostics:** Advanced diagnostics uses `diagnostics.summary@1` and
  `diagnostics.events.page@1` with allowlisted filters, cursors, bounds,
  redaction, and truncation.

Central account/passkey/recovery/pairing/inventory/signaling actions remain their
separately typed central operations; they are not smuggled into the local
management manifest or exposed through MCP. Conversely, the central Worker does
not proxy routine local management operations.

Every visible mutation generates one random 128-bit idempotency key, supplies the
manifest-declared revision/generation preconditions, and retains the key for
result lookup for the life of that UI operation. Repeating with different input
is a conflict. The UI may offer validate/plan/simulate first, but apply always
rechecks and a plan digest is only a stale-plan precondition, never authority.
Hiding a control for missing scope/feature improves clarity; direct invocation
still reauthorizes and fails closed.

A build/test parity check must account for every manifest row as intentionally
`visible`, `contextual`, or `non-UI`, with a route/component and reason. It also
asserts that every UI mutation names one manifest operation. This is metadata
coverage, not automatic screen generation. T13 selects a small generated MCP
subset from these same operations, with explicit private-field projections and no
generic escape hatch.

### Security and privacy presentation

The browser receives only redacted, scope-appropriate views. Central inventory
never receives ESPHome addresses, Noise keys, device config/state, Rule JSON,
operation payloads, diagnostics, or injected environment values. Local/Cockpit
management may show authorized bounded non-secret config, but environment fields
show only deterministic variable name plus configured status and MCP omits
private addresses as already decided. Copy actions are explicit and never include
hidden neighboring fields.

Ownership-sensitive central actions require a recent passkey assertion and say
why. Ordinary capability/install/rule authorization remains the fixed scope plus
canonical operation seam; do not invent an AI approval queue or UI-only policy.
The active pairing comparison is the sole exception for identity evidence: it
renders the bounded grouped public-key fingerprint required by the pairing
protocol as text for human comparison. It is not copied by default, retained
past the transaction, or sent to analytics, error reporting, or tracing. No
secret, token, cookie, code, raw proof transcript, private identity evidence,
private key, notification body, unrestricted state/config, guest diagnostic
text, or raw protocol message appears in markup, URL, browser logs, analytics,
error reporting, traces, or clipboard defaults.

Activity is a safe composition of closed operation reads, not a compliance claim
or duplicate audit store. Current-session operation cards may identify result
confidence; rule pages expose declared run metadata; diagnostics may identify a
safe transport/client. None stores AI prompts or reconstructs user intent beyond
the canonical operation.

### WCAG 2.2 AA and responsive contract

WCAG 2.2 AA is a release requirement for complete journeys, not a token claim.
Use native landmarks, headings, lists, forms, tables, buttons, links, labels, and
status semantics before ARIA. All functionality works by keyboard with visible,
unobscured focus; sticky headers, banners, dialogs, drawers, and inspectors may
not cover the focused element. Modal focus is trapped only while truly modal and
returns to its invoker.

Meet contrast for text, controls, focus, status, and graph edges in light/dark
modes. Never rely on color, position, hover, animation, or icon shape alone.
Async operation changes use restrained `status`/`alert` announcements; live
state/log streams do not continuously steal focus or flood assistive technology.
Errors identify the field/node, explain correction, preserve entered non-secret
input, and provide a summary with links when multiple fields fail. Authentication
supports password-manager/passkey semantics and does not impose cognitive tests.

Support text zoom to 200%, browser zoom/reflow to 400% at 320 CSS pixels, reduced
motion, forced-colors/high-contrast behavior, and coarse pointer input. Content
reflows without page-level two-dimensional scrolling; bounded tables, code,
graphs, and logs may scroll within labeled regions with equivalent non-spatial
access. On phones, navigation becomes a labeled menu, secondary metadata uses
disclosure, device controls remain direct, dialogs become sheets only when
semantics/focus remain correct, and diagnostics/rules prefer list-plus-inspector.

English is the v1 authored language, but UI strings are centralized and layouts
do not assume fixed label widths. Locale selection, translation infrastructure,
and right-to-left certification are deferred rather than partially claimed.

### Implementation sequence and validation

The greenfield web cutover remains phased under the framework/cache and central-
hosting decisions. Legacy `site/` source/state is requirements evidence only and
is not copied or migrated. Do not combine framework, auth origin, D1, Cockpit,
DNS, or credential cutovers. Within the approved UI implementation, proceed at stable seams:

1. remediate and separately release the Open Design candidate, then vendor/lock
   its closed bundle; map tokens and build primitives, two densities, route/error
   states, and accessibility tests;
2. establish public/auth shells, account security, durable inventory, pairing,
   and the shared generated operation client boundary;
3. add server overview, hydration/connection state, devices/facets, and typed
   controls over direct HTTPS/WebRTC;
4. add registry handoff, plugin install/update/rollback review, instance config,
   environment requirement presentation, and activation lifecycle;
5. add the one-state React Flow/outline rule editor, validation, simulation,
   activation, runs, and traces; and
6. add activity/diagnostics, finish responsive/assistive-technology coverage,
   then remove superseded prototype UI and routes.

The current `site/components/flow-editor.tsx`, animated searching globe, polling
Cockpit shell, bearer stubs, hard-coded registry data, generic input/output nodes,
and memory-only graph are prototypes, not migration formats or source inputs.
Implement their required journeys afresh in `web/`; do not preserve legacy
components.

Required evidence before release:

- Open Design closed-file/digest, manifest/component/token drift, symlink/size,
  and credential-scan guards plus visual review of the exact vendored bundle;
- measured link/text/control/focus contrast and token alias/primitive snapshots
  in light/dark and guided/operational contexts, including a native forced-colors
  focus outline and reduced motion;
- pure tests for route class, density, state/error mapping, scope/feature
  visibility, operation-to-UI manifest coverage, idempotency retention, and
  redaction/copy projections;
- component tests for keyboard/focus, overlays, forms, async announcements,
  capability controls, semantic diffs, and graph/outline command parity;
- browser journeys for central passkey enrollment/recovery, standalone login/
  bootstrap, pair success and every terminal state, public registry-to-reviewed
  install, environment requirement/recreation, device activation/control,
  ambiguous-result recovery, rule keyboard authoring/simulation/activation,
  update/rollback, offline/reconnect hydration, unsafe-mode warning, and bounded
  diagnostics;
- automated accessibility analysis plus manual keyboard and representative
  screen-reader checks at desktop/mobile, 200% text, 400% reflow, light/dark,
  reduced motion, and high contrast;
- isolated candidate proof of public/private cache isolation, no-store
  management, the `litehouse-web` UI versus signaling-only `litehouse-cockpit`
  boundary, direct WebRTC JSON-RPC, fresh-session state reload after reconnect,
  central outage behavior, and no cross-owner leakage; TURN/relayed-connectivity
  validation is deferred to a later separately selected release; and
- independent lint, typecheck, Worker integration tests, browser tests, and
  production builds with no ignored TypeScript errors, followed by final diff,
  secret, generated-artifact, and scope review.

No visual snapshot alone proves accessibility, no hidden button proves security,
and no successful transport response proves a physical device changed.
