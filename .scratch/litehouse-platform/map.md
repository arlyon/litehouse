# Find the route to Litehouse's first connected release

Label: wayfinder:map
Status: active
Tracker: local-markdown

## Destination

Produce a decision-complete, evidence-backed, implementation-ready route for
Litehouse's first connected release: log in, pair a portable local container,
install the signed ESPHome plugin from the central registry, discover and
control vendor-neutral typed capabilities, build basic React Flow automations,
and operate the same management API through the web UI or a thin MCP adapter.
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
- Broad vendor-neutral ESPHome capability contracts, including switches,
  lights with brightness/color, sensors, climate, covers, buttons, and notifier.
- React Flow is the editor, not the persisted format. It compiles to a versioned,
  validated rule IR after standardized DSL options are researched.
- Web UI and structured management API have functional parity.
- AI may use every authenticated Litehouse API through a thin MCP adapter, but
  receives no host shell, arbitrary filesystem/network access, raw secrets,
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
  files/diffs. The map itself remains untracked until its tracking ticket is
  resolved.
- External credentials, production hardware, Open Design publication, and
  repository commits require separate explicit approval.
- Imported research answers are snapshots; partial results explicitly label
  their limits, recommendations, and remaining decisions.

## Decisions so far

- [Use WASI 0.3 native async and exact-major generated contracts](../../docs/platform-worldview.md#runtime-and-compatibility)
  — preserve the pinned bridge, S1 adapters, symmetric plugins, and no implicit
  major adaptation.
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

## Not yet specified

- Which standardized condition DSL—such as CEL, JSON Logic, DMN/FEEL, or a
  workflow standard—fits inside the small Litehouse-owned typed rule graph.
- Which maintained lightweight Rust MCP library and transport profile can expose
  the canonical management API without creating a second authorization model.
- Exact OAuth/OIDC discovery, audience, token, JWKS-cache, pairing-grant, and
  standalone issuer configuration after standards research.
- Exact implementation workstreams, migrations, generated artifacts, and UI
  decomposition until their upstream decisions and prototypes resolve.
- Numeric performance targets until representative measurements exist.

## Out of scope

- Implementing the route during Wayfinder.
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
