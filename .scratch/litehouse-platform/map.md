# Find the route to Litehouse's first connected release

Label: wayfinder:map
Status: superseded-after-t8.2
Tracker: local-markdown

> Downstream implementation planning after completed T8.2 is superseded by
> [Ship Litehouse through local MCP and remote UI](../litehouse-release/map.md).
> Resolved research and decisions here remain historical evidence; the former
> T8.3–T15 route is no longer implementation authority.

## Destination

Produce a decision-complete, evidence-backed, implementation-ready route for
Litehouse's first connected release: log in, pair a portable local container,
install independent signed ESPHome, Tasmota HTTP, ntfy, and non-commercial
Open-Meteo providers from the central registry, discover and control
vendor-neutral typed capabilities, build basic React Flow automations, and
operate the management API through the web UI and an explicitly selected small
subset through a thin MCP adapter.
ESPHome remains the primary physical-device proof; ntfy owns notifications.
Implementation remains separately approved.

## Notes

### Working mode

- This map plans; it does not implement. It grants no standing authority for
  code edits, commits, publication, deployment, credential changes, service
  restarts, hardware access, or external mutation.
- Research tickets may proceed independently when explicitly launched. HITL
  decisions resolve one at a time. No new background research is launched merely
  by creating this map.
- Every implementation tranche requires a separate proposal, user approval,
  stable behavioral seam, targeted checks, canonical checks, rollback plan, and
  final scope/secret review.
- Keep it simple: fixed scopes, typed records, one obvious decision seam, and
  ordinary structured logs. Do not add policy DSLs, approval-token ceremonies,
  duplicate audit stores, parallel implementations, or speculative abstraction
  without a demonstrated current requirement.

### Canonical context

Read in order:

1. [`docs/platform-worldview.md`](../../docs/platform-worldview.md)
2. [`docs/platform-architecture-prd.md`](../../docs/platform-architecture-prd.md)
3. [`PRODUCT.md`](../../PRODUCT.md)
4. Current source/tests as evidence of implementation reality

The existing WIT/Component Model pipeline is fixed. Current
`crates/plugin-macro/wit/plugin.wit` outputs are transitional placeholders;
service decisions extend that pipeline rather than inventing another plugin ABI.
PRD items 1 and 2 remain decided. Items 3 through 13 keep their order, with item
11 narrowed here from a full CoreOS appliance to a portable-container contract
plus a very thin optional Butane configurator.

### First-release boundary

- One portable Litehouse container with one persistent data volume.
- Rust-first plugin authoring; WIT remains language-neutral.
- Signed first-party registry releases plus explicitly trusted local developer
  artifacts; installs and updates remain digest-pinned and reviewed.
- Broad vendor-neutral ESPHome capability contracts for switches, lights with
  brightness/color, sensors, climate, covers, and buttons; independent narrow
  Tasmota HTTP, ntfy `notification-sink@1`, and acknowledged non-commercial
  Open-Meteo providers complete the selected V1 portfolio.
- Versioned Litehouse Rule JSON is the authored and persisted rule language;
  React Flow and its synchronized semantic outline are projections over that one
  state, while bounded CEL remains only a pure condition leaf.
- Web UI and structured management API have functional parity.
- AI may use the explicitly approved small subset of authenticated Litehouse
  operations exposed through a thin MCP adapter, but receives no host shell,
  arbitrary filesystem/network access, raw secrets,
  ownership/trust bypass, or permission to weaken safeguards.
- Owner-only accounts initially; multi-user sharing follows later.

### Identity and connectivity

A server chooses exactly one mode:

1. **Cloud-connected:** central Litehouse OAuth, short-lived pairing code,
   durable website inventory, Cockpit signaling, and direct WebRTC management.
2. **Standalone:** one manually configured external OIDC issuer and direct local
   HTTPS API/MCP access, without central inventory or WebRTC signaling.

Authenticated OAuth/OIDC requires HTTPS. Plain HTTP exists only behind an
explicit unauthenticated `unsecure-local` flag. ESPHome addresses and Noise keys
remain local to the server and never persist in the website, registry, Cockpit,
logs, or AI context.

### Evidence bar

Before the route is complete, require focused research where external standards
or ecosystem behavior can invalidate a choice. Require disposable prototypes
only for:

1. typed cross-store service fabric;
2. real ESPHome shared-node/multi-facet behavior plus deterministic fixtures;
3. registry publish/query/install/offline/rollback behavior.

WebRTC, CoreOS, AI, and full UI prototypes are not mandatory Wayfinder gates.

### Safety boundary

- Preserve all pre-existing dirty/untracked work and all unpublished commits.
- Never reset, clean, stash, overwrite, bulk-stage, or expose secret-bearing
  files/diffs. The Wayfinder directory is tracked from planning baseline
  `4ebaee6`; future commits still require explicit approval and path-scoped
  review.
- External credentials, production hardware, Open Design publication, and
  repository commits require separate explicit approval.
- Imported research answers are snapshots; partial results explicitly label
  their limits, recommendations, and remaining decisions.

## Decisions so far

- [Use WASI 0.3 native async and exact-major generated contracts](../../docs/platform-worldview.md#runtime-and-compatibility)
  — preserve native async, S1 adapters, symmetric plugins, and no implicit major
  adaptation; the former P1 packaging bridge is superseded by the prototype
  evidence below.
- [Keep device aggregates private behind detached typed facets](../../docs/platform-worldview.md#facet-ownership)
  — provider resources stay provider-local; consumer proxies are host-owned;
  repeated facets are first-class.
- [Use stable cardinality-aware binding](../../docs/platform-worldview.md#binding)
  — exact stable selectors, deterministic snapshots, and same-identity-only
  rebinding.
- [Set the first connected release](./issues/01-set-first-connected-release.md)
  — central registry, broad ESPHome contracts, React Flow rules, web/API parity,
  and reviewed pinned updates.
- [Set identity and AI access boundaries](./issues/02-set-identity-and-ai-boundaries.md)
  — exclusive central-versus-standalone OAuth modes and full Litehouse-API
  access without host escape.
- [Set deployment and evidence boundaries](./issues/03-set-deployment-and-evidence-boundaries.md)
  — portable container, single volume, thin Butane configurator, and three
  risk-based prototypes.
- [Research ESPHome Native API constraints](./issues/04-research-esphome-native-api.md)
  — one Noise session per node, no command acknowledgement/revision stream, and
  provider-owned durable identity are required.
- [Research HTTP authority normalization](./issues/05-research-http-authority-normalization.md)
  — compare strict origin tuples and treat DNS-rebinding resistance as a
  connector decision.
- [Research component distribution and trust](./issues/06-research-component-distribution-and-trust.md)
  — reuse WIT/OCI/wasm-tools/Sigstore while owning release identity,
  compatibility, withdrawal, and revocation policy.
- [Research WebRTC management constraints](./issues/07-research-webrtc-management-constraints.md)
  — application-level negotiation, bounds, cancellation, resume, and
  authorization binding are mandatory.
- [Research thin Butane/CoreOS deployment](./issues/08-research-thin-butane-deployment.md)
  — reuse provisioning primitives but do not make a CoreOS appliance lifecycle
  part of v1.
- [Research safe AI operation patterns](./issues/09-research-safe-ai-operations.md)
  — one typed API with server-enforced scopes, revisions, idempotency,
  redaction, and audit; no general-purpose host tools.
- [Research state and event precedents](./issues/10-research-state-event-precedents.md)
  — borrow scoped cursors, explicit hydration/gaps, quality, and at-least-once
  identities without claiming global order or exactly-once effects.
- [Define the state and event model](./issues/15-define-state-and-event-model.md)
  — per-entity generation-qualified revisions, current-state hydration,
  separate availability, receive-time freshness, best-effort replay, and stable
  event IDs.
- [Research Wasmtime concurrency and cancellation](./issues/11-research-wasmtime-concurrency-cancellation.md)
  — stores are cooperatively driven; dropped call futures do not cancel guests,
  so Litehouse needs provider actors, bounded queues, soft cancellation, and
  generation-fatal hard teardown.
- [Define the shared error model](./issues/16-define-shared-error-model.md)
  — exact-major grouped core errors inform Litehouse while each contract owns
  exhaustive typed detail; full envelopes preserve retry advice, safe messages,
  error IDs, and explicit ambiguous outcomes.
- [Define provider execution and ESPHome lifecycle](./issues/17-define-provider-execution-and-esphome-lifecycle.md)
  — one configured instance represents one physical device with no nested
  device key; bounded ordered dispatch feeds plugin-controlled concurrent work,
  while facet-local incarnations and store-wide generations isolate failure.
- [Research unit mathematics standards and libraries](./issues/41-research-unit-math-standards.md)
  — use bounded UCUM 2.2 input over canonical decimals and built-in profiles,
  with one Rust engine shared to TypeScript through Wasm; current libraries are
  differential oracles rather than production authorities.
- [Research ESPHome capability semantics for the v1 catalog](./issues/43-research-esphome-capability-semantics.md)
  — map only discovery-advertised features, treat multi-field commands as
  grouped rather than transactional, and never fabricate acknowledgements,
  initialized state, ranges, units, or physical completion.
- [Research Matter capability mapping for the v1 catalog](./issues/42-research-matter-capability-mapping.md)
  — preserve cluster boundaries, named features, source representations,
  nullability, events, and endpoint-like composition while translating away
  Matter wire IDs, fabrics, sentinels, and interaction machinery.
- [Define the vendor-neutral contract catalog](./issues/18-define-vendor-neutral-contract-catalog.md)
  — ship nine Matter-informed exact-major capabilities with metadata-only UI
  groups, canonical typed quantities, explicit feature/receipt semantics, and
  no fabricated conversion, confirmation, or weak fallback.
- [Secure representative ESPHome proof access](./issues/20-secure-representative-esphome-access.md)
  — use one disposable three-channel switch and one air-quality monitor through
  exact local targets, runtime-only credentials, supervised absolute commands,
  client-only reconnect, and verified relay restoration.
- [Decide the workspace and map baseline](./issues/34-decide-workspace-and-map-baseline.md)
  — standardize all workspace crates on Rust 2024, keep generated registry code
  untracked and pinned if retained, validate approved patches in clean
  worktrees, and track only the reviewed Litehouse Wayfinder directory.
- [Research static sparse registry index precedents](./issues/45-research-static-registry-index-precedents.md)
  — use CDN-backed object storage, immutable OCI-shaped artifacts, signed TUF
  snapshots, and deterministic JSON Lines shards; binary indexes wait for
  measured size, latency, memory, or update pressure.
- [Choose registry hosting and index architecture](./issues/46-choose-registry-hosting-and-index-architecture.md)
  — retain dynamic partitioned FlatBuffers on R2 only with ETag root CAS,
  content-addressed blobs, signed single release records, a Rust/Wasm client,
  first-party CLI publication, and Cargo-like reviewed origin trust.
- [Inventory the Next.js and Vercel migration surface](./issues/49-inventory-nextjs-vercel-migration-surface.md)
  — preserve React/content/auth journeys while replacing Next/Vercel bindings,
  hard-coded registry data, weak migration/auth assumptions, and mixed public,
  durable authenticated, and live-private runtime boundaries.
- [Research Cloudflare React framework and cache options](./issues/48-research-cloudflare-react-frameworks.md)
  — prefer gated TanStack Start for official prerender/selective SSR and typed
  server boundaries, with React Router 8 as the GA fallback if RC, auth, cache,
  or deployment-affinity prototypes fail.
- [Choose Cloudflare web framework and cache model](./issues/50-choose-cloudflare-web-framework-and-cache-model.md)
  — migrate in phases to exact-pinned TanStack Start with objective React Router
  fallback gates, static/public-derived/per-user-durable/live-private route
  classes, lazy heavy modules, and tested Workers Cache isolation/invalidation.
- [Research standardized rule DSL options](./issues/12-research-rule-dsl-options.md)
  — use a bounded statically checked CEL profile only for Boolean conditions;
  exact-major inputs, UCUM math, time/state semantics, control flow, simulation,
  and traces remain in the Litehouse-owned typed graph and Rust/Wasm core.
- [Research open event-rule and state-transition standards](./issues/51-research-open-event-rule-standards.md)
  — no single ECA/workflow standard fits a small typed planner: compose
  CloudEvents, strict CEL, JSON Patch, JSON Schema/JCS, UCUM, and selected SCXML/
  IEC graph semantics around an owned pure pre-state-to-plan IR; physical post-
  state remains receipt-reconciled rather than optimistically asserted.
- [Research OAuth and OIDC architecture](./issues/14-research-oauth-oidc-architecture.md)
  — use exclusive central JWT-issuer or standalone external-OIDC modes, exact
  resource/server-bound tokens, separate Litehouse key-proof pairing, offline
  JWKS validation, short grants, and standards-discoverable API/MCP resources.
- [Define service and management authorization](./issues/19-define-service-and-management-authorization.md)
  — use per-instance typed grants, operation-level service permissions, fixed
  OAuth scopes for every external client, direct authorized execution, strict
  permission-plane separation, and filterable tracing instead of policy,
  approval, AI-specific, or duplicate audit systems.
- [Research lightweight Rust MCP libraries](./issues/13-research-rust-mcp-libraries.md)
  — use official `rmcp` 2.2 with bounded stateful Streamable HTTP as a thin
  Tower adapter; Litehouse keeps all canonical API, OAuth, authorization,
  limits, redaction, and domain behavior.
- [Prototype the typed service fabric](./issues/21-prototype-typed-service-fabric.md)
  — generated provider resources and consumer proxies, typed exact majors,
  stable IDs/generations/incarnations, deterministic snapshots, operation
  grants, and ordered actor admission fit without reflection; build guests
  directly for `wasm32-wasip2` with native P3 bindings instead of a P1 adapter.
- [Prototype real ESPHome capabilities](./issues/22-prototype-real-esphome-capabilities.md)
  — one Noise session/cache per configured node maps advertised entities
  directly to independent typed facets with stable host identity, conservative
  profiles and non-confirming receipts; real Node S proved topology, state, and
  reconnect, while fixtures cover isolation, measurements, and Level/Color.
- [Define the contract artifact format](./issues/23-define-contract-artifact-format.md)
  — publish type-only binary WIT and deterministic source bundles through the
  same signed package/lock graph as plugins; use additive same-major minimums,
  standard wkg resolution plus build-time bindings, structural checks, and a
  separately versioned official-WASI runtime profile.
- [Define registry publication and installation locks](./issues/24-define-registry-publication-and-install-locks.md)
  — replace prototype schemas with versioned FlatBuffer roots, immutable
  content-addressed shards/blobs, signed complete release records, root ETag
  CAS, verified offline caches, existing exact-version JSON configuration, and
  per-plugin restart activation retaining one offline predecessor.
- [Define signing and trust](./issues/25-define-signing-and-trust.md)
  — verify deterministic release payloads with a pinned first-party Ed25519 key,
  protected CI custody and product-shipped rotation; keep origin-owned
  yank/withdraw/revoke states advisory for running code, allow only digest-
  pinned local development, and use structured tracing rather than audit DBs.
- [Prototype registry publish, install, and rollback](./issues/26-prototype-registry-flow.md)
  — an isolated FlatBuffer/Ed25519 core proved object-first concurrent
  publication, ETag root CAS, verified offline/resumable reads, guarded
  activation and one-predecessor rollback; native and `wasm32-wasip2` queries
  are byte-identical, and a disposable R2 bucket proved the exact conditional
  and range operations before successful prefix and bucket cleanup.
- [Define authentication and pairing](./issues/27-define-authentication-and-pairing.md)
  — use exclusive central or standalone issuers, passkey plus delayed magic-link
  recovery, exact server-bound JWTs, and one owner; preserve today's nearby/name/
  six-digit pairing UX while replacing its static password and bearer stubs with
  short-lived key-proof pairing, pair epochs, separate signaling grants, pinned
  standalone bootstrap, operator TLS, loopback-only unsafe mode, and local-only
  destructive trust migration.
- [Define the WebRTC management protocol](./issues/28-define-webrtc-management-protocol.md)
  — use broker-facilitated signaling followed by one direct reliable ordered
  WebRTC channel carrying standard JSON-RPC 2.0; authenticate with a short-lived
  grant, dispatch the web UI's selected methods to canonical handlers, emit
  notifications, and reconnect with a fresh session/state reload rather than a
  custom binary, credit, replay, resume, or TURN protocol.
- [Define the rule IR and editor contract](./issues/29-define-rule-ir-and-editor-contract.md)
  — publish a closed, versioned Rule JSON Schema and typed port catalog; React
  Flow edits that JSON projection while bounded CEL remains a pure condition
  leaf, and serialized run-to-completion microsteps produce hypothetical desired
  patches plus durable typed effects whose receipts reconcile physical state.
- [Define plugin-driven instance discovery and adoption](./issues/44-define-plugin-driven-instance-discovery.md)
  — defer device discovery entirely: v1 explicitly configures one artifact/
  physical-device instance with non-secret connector config, deterministic
  per-field environment requirements, and exact grants; pure validation runs no
  guest, hidden activation uses the existing pointer transaction, and future
  discovery is a separate artifact class with no reserved ABI or LAN authority.
- [Define the container and thin Butane contract](./issues/30-define-container-and-butane-contract.md)
  — build conventional non-root read-only amd64/arm64 OCI images with one locked
  data directory, health/readiness, graceful shutdown, recorded digest and basic
  smoke evidence; emit a small optional validated FCOS Butane/Quadlet config and
  defer cross-version restore, fixed resource envelopes, signing, and staging.
- [Choose central control-plane hosting](./issues/47-choose-central-control-plane-hosting.md)
  — replace Vercel/Turso with staged Cloudflare Workers and D1 authority; a
  separate Cockpit Worker plus per-server Durable Object owns only bounded live
  signaling; direct WebRTC uses ordinary ICE while TURN remains deferred, R2
  stays GET/HEAD-only to web, and thin adapters avoid multi-cloud
  abstractions without coupling domain behavior to bindings.
- [Define the AI and MCP management surface](./issues/31-define-ai-and-mcp-management-surface.md)
  — expose local-HTTPS-only `rmcp` Streamable HTTP with a small explicitly
  approved subset of canonical inspection, state, and essential-control tools;
  reuse ordinary handlers/auth/redaction and provide no host, cloud-control,
  secret-value, ownership, or generic API escape hatch.
- [Define secret lifecycle and credential remediation](./issues/35-define-secret-lifecycle-and-remediation.md)
  — keep long-lived runtime values entirely in Podman/orchestrator-injected
  environment variables with deterministic per-instance field names and no
  Litehouse secret store, persistence, rotation, encryption, vault, or CRUD;
  rotate by provider update plus container recreation, and require a separately
  approved revoke-first redacted history-remediation incident for existing leaks.
- [Defer representative performance baselines](./issues/32-establish-representative-baselines.md)
  — prior operation observed startup well below one second and Cranelift artifacts
  are cached; add no benchmark harness or numeric SLO without a demonstrated
  regression, and measure the exact implemented amd64/arm64 seam before any
  future threshold or optimization.
- [Define no first-release performance SLOs](./issues/33-define-performance-slos.md)
  — retain existing queue/session/resource/deadline limits as mandatory safety
  and resilience bounds, not latency promises; current performance is accepted
  and any future numeric gate follows a concrete regression plus exact production
  measurement.
- [Define Open Design and control-surface adoption](./issues/36-define-open-design-and-control-surface-plan.md)
  — remediate and separately release the Open Design candidate as a closed,
  Git-versioned vendored visual source, keep React behavior local, and use one
  `litehouse-web` generated-operation UI across cloud WebRTC and standalone
  HTTPS while `litehouse-cockpit` stays signaling-only; fixed routes and
  densities, complete journeys, honest state/ambiguity, keyboard-equivalent Rule
  JSON editing, manifest-bounded activity/diagnostics, and WCAG 2.2 AA validation
  define the release control surface.
- [Research the first-party notification provider](./issues/52-research-first-party-notification-provider.md)
  — use one product-specific ntfy adapter with a fixed origin/topic, injected
  token, bounded body/title/priority, acceptance-only receipts, and no ambiguous
  retry; defer Twilio and forbid a generic webhook escape hatch.
- [Research Tasmota and Open-Meteo provider contracts](./issues/53-research-tasmota-and-weather-provider-contracts.md)
  — constrain Tasmota to explicit private-LAN HTTP nodes exposing absolute power
  and whitelisted measurements, and Open-Meteo to four attributed current-model
  measurements on its acknowledged non-commercial free tier.
- [Decide the V1 plugin portfolio](./issues/37-decide-plugin-portfolio.md)
  — ship independent signed ESPHome, Tasmota HTTP, ntfy, and non-commercial
  Open-Meteo providers; keep persistent-task/no-op as non-published development
  fixtures, defer Samsung, and remove empty MQTT, Presence, Sonoff, and Twilio
  shells in a separately approved tranche.
- [Research the OTBR and Thread-radio boundary](./issues/54-research-otbr-and-thread-radio-topology.md)
  — keep OTBR, Thread routing, privileged host networking, persistence, RCP,
  firmware, radio, and antenna entirely operator-owned and invisible to
  Litehouse; OTBR is not a Matter controller.
- [Research Matter controller and fabric ownership](./issues/55-research-matter-controller-and-fabric-ownership.md)
  — keep fabric credentials, commissioning, CASE, subscriptions, and controller
  storage outside Litehouse; reject a native or multi-node plugin controller.
- [Research the Home Assistant Matter Server adapter](./issues/56-research-home-assistant-matter-server-adapter.md)
  — reuse external Python Matter Server schema 11 as an operator-owned
  compatibility target, with Home Assistant retaining commissioning and a
  filtered loopback/local transport denying unrestricted fabric authority.
- [Define Matter adapter v1](./issues/57-define-home-assistant-matter-adapter-v1.md)
  — after the first connected release, configure one plugin instance per native
  Matter node, each with a dedicated filtered relay/upstream connection to one
  external Matter Server; defer bridges, discovery, commissioning, OTBR, shared
  dispatch, fabric credentials, OTA, and generic Matter operations.
- [Plan runtime resilience closure](./issues/38-plan-runtime-resilience-closure.md)
  — preserve the existing runtime/compiler/cache and apply focused fixes for
  corrupt cache handling, queue-inclusive guest deadlines, explicit subscribed
  event continuity loss, safe per-instance versus shared-store trap behavior,
  per-instance capabilities with unique IDs, protected environment values, and
  panic paths; real T2 endpoint publication waits for a T6 provider.
- [Disposition source-owned residuals](./issues/39-disposition-source-owned-residuals.md)
  — implement only the schema/runtime safety seams owned by resolved contracts;
  replace the old registry/pairing/admin placeholders in their production
  tranches, harden rather than replace the existing runtime cache/HTTP seams,
  enforce the four-provider portfolio, and defer
  optimization until measured pressure rather than leaving executable TODOs.
- [Prepare the implementation-ready handoff](./issues/40-prepare-implementation-ready-handoff.md)
  — deliver the greenfield first connected release through separately approved
  workspace, contract, service, resilience, web, registry, publisher/signing,
  provider, rule, server, identity, basic WebRTC JSON-RPC, UI, small MCP subset,
  and local candidate tranches, with later separately selected provider-backed
  staging, FR1–FR16 traceability, generated-artifact
  ownership, rollback, and no implicit external authority.

## Not yet specified

- No first-connected-release architecture item. Implementation evidence and any
  post-V1 shared dispatcher or Matter delivery require separately approved work.

## Out of scope

- Implementing the route during Wayfinder.
- First-release benchmark infrastructure and numeric performance SLOs; current
  performance is accepted until demonstrated pressure requires exact measurement.
- Baking or maintaining a Litehouse OS image, full CoreOS lifecycle controller,
  VM appliance proof, automatic local PKI, or orchestrator-specific runtime.
- Public third-party publisher onboarding/moderation, multi-user sharing roles,
  TypeScript plugin SDK parity, and a built-in AI chat assistant in v1.
- Host-level AI shell/filesystem/container/network access, raw-secret retrieval,
  self-granted permissions, or safeguard bypass.
- Arbitrary WIT reflection, implicit contract-major adaptation, universal device
  ontology, full Matter model, central relay of routine home traffic, and
  distributed runtime consensus.
- Rich workflow loops/sagas/state machines in rule IR v1.
- Automatic weaker capability emulation, cross-capability physical planning, or
  a general pushdown optimizer in v1; providers may perform only observationally
  exact private optimization.
- Completing every plugin skeleton or source marker merely because it exists;
  each receives an explicit implement/delete/defer/out-of-scope disposition.
- The unrelated Expo starter, Factorio/WoW infrastructure, generated Tasmota
  upstream TODO prose, rustc crash reports, and credential/signing artifacts.
