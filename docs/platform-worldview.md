# Litehouse Platform Worldview

- **Status:** Current pre-Wayfinder snapshot
- **Date:** 2026-07-13
- **Purpose:** Preserve the complete shared worldview before compaction so a
  fresh session can chart the work without reconstructing intent. This
  document does not grant implementation or external-mutation authority.

## 1. Source-of-truth order

Load these documents in order:

1. `docs/platform-worldview.md` — current context, status, and handoff.
2. `docs/platform-architecture-prd.md` — architecture requirements, decisions,
   acceptance scenarios, rollout, and ordered decision queue.
3. `PRODUCT.md` — users, product character, design principles, and
   accessibility requirements.
4. Current source and tests — evidence of what is implemented, not a substitute
   for the decisions above.

The worldview does not replace the PRD. It records what the current sessions
know, including implementation reality, unresolved work, dirty-worktree
boundaries, and the intended Wayfinder destination.

**Compaction precondition:** satisfied. Commit `1484940` preserves this reviewed
worldview, the project-local Wayfinder files/symlink, and the single-skill
lockfile without absorbing unrelated dirty work.

## 2. Destination

Deliver and verify the entire Litehouse platform, not only the contract kernel:

- WASI 0.3/native-async plugin runtime;
- exact-major typed service contracts and host-owned cross-store proxies;
- a real ESPHome integration proving multiple switches per node and multiple
  nodes per plugin instance;
- native and no-code automation with the same contract semantics;
- independent contract/plugin distribution and deterministic installation
  locks;
- durable account/server inventory and direct authenticated WebRTC management;
- a recoverable Fedora CoreOS appliance;
- structured, least-privilege operations for people and AI agents;
- the shared public/product design system and accessible control surfaces; and
- inventory and explicit disposition of every source-owned TODO, blocker,
  placeholder, and migration residual relevant to the approved platform
  destination. Disposition may be implement, delete, defer, or mark out of
  scope; inventory does not imply implementation.

The immediate product proof is an ESPHome multi-switch vertical slice, but the
Wayfinder map must cover the whole destination and preserve the dependency
chain between planes.

## 3. Product truths

- Litehouse is for home operators, advanced multi-home operators, plugin and
  contract authors, and authorized AI agents.
- It should feel quietly capable, direct, trustworthy, and technically honest.
- Public pages and operational surfaces use one visual language at different
  densities.
- Avoid generic enterprise SaaS dashboards, decorative analytics, nested card
  grids, and complexity without operational value.
- Meet WCAG 2.2 AA: keyboard operation, visible focus, semantic structure,
  reduced motion, non-color status cues, adequate targets, zoom, and
  screen-reader navigation.
- Local operation must remain useful without the cloud. The central control
  plane must not become the routine home data path.

## 4. Confirmed architecture decisions

Do not reopen these without concrete contradictory evidence.

### Runtime and compatibility

- Wasmtime is pinned to `46.0.1`.
- Guest-visible long-running work uses native Component Model async.
- Constructors, metadata, and subscriptions remain bounded and synchronous in
  WIT; `update` and transitional `outputs` are native async.
- Rust guests currently compile for `wasm32-wasip1` and use Wasmtime's pinned
  reactor adapter because usable Rust `wasm32-wasip3` `std` is unavailable.
- The native-async ABI is deliberately incompatible with stale pre-migration
  components. Host and components must roll out atomically; no partial HTTP 0.2
  linkage can restore the old runner ABI.

### Contracts and plugins

- Contract identity is exact namespace/name/major, such as `switch@1`.
- The host never implicitly adapts contract majors. Explicit adapter plugins
  may bridge them.
- Use generated typed host adapters (S1). A new provider implementation does
  not require a host rebuild; a wholly new contract initially does.
- Plugins are symmetric: any plugin may provide services, consume services, or
  do both. Device, automation, adapter, and rule-executor plugins use the same
  fabric.
- Contracts and plugins publish independently.

### Facet ownership

- Physical-device state is a private provider aggregate containing connection,
  cache, synchronization, and lifecycle state.
- The aggregate is not a public WIT resource and never crosses stores.
- Each capability is a detached, exact-major typed provider endpoint.
- The host retains provider implementation handles only in the provider store
  and creates consumer-store-local typed proxy resources.
- Raw `ResourceAny`, guest resources, borrows, instances, and resource-table
  indices never cross stores.
- Endpoint identity is conceptually:

  ```text
  (provider instance, device-key, facet-key, namespace, contract name, major)
  ```

- Repeated same-contract facets are mandatory. A provider may expose any number
  of `switch@1` endpoints. Stable source entity IDs, not labels or list
  positions, become facet keys.
- Provider restart changes generation. Old proxies become explicitly stale and
  are never silently retargeted.

### Binding

- Persist stable exact-field selectors over provider instance, device key, and
  facet key. Omitted fields are wildcards.
- Contract namespace/name/exact-major come from the typed requirement and are
  never wildcarded.
- Cardinality is explicit:
  - `one`: exactly one match;
  - `optional`: zero or one, with ambiguity rejected;
  - `many`: the complete deterministic match snapshot.
- Rebinding is `never` or `same-identity` only.
- `same-identity` may replace only the same provider/device/facet/contract in a
  new provider generation.
- V1 never silently substitutes another endpoint or adds a newly matching
  identity to an active `many` binding. Membership changes require explicit
  re-resolution.

### Distribution and control plane

- Registry reads must be cheap, immutable-object/CDN friendly, and queryable
  from supported JavaScript server environments. Node/N-API is acceptable.
- Contract releases, plugin releases, and installation locks are separate
  entities.
- The website owns durable users, paired-server inventory, ownership, sharing,
  and last-known metadata.
- Cockpit owns transient presence and signaling.
- After signaling, normal management traffic is direct WebRTC.
- WebRTC carries a versioned structured protocol. Logs are one stream, not the
  protocol itself.
- No-code rules and native automation plugins consume the same public typed
  contracts.

### Appliance and AI direction

- Prototype Fedora CoreOS with predefined services/containers,
  health-gated updates, rollback, backup, restore, and recovery.
- AI operation means inspect/validate/plan/apply contracts, dry runs, explicit
  permissions, idempotency, revisions, audit, and human gates—not unrestricted
  shell access.

## 5. ESPHome is the first vertical proof

ESPHome support is an immediate priority and must be real product capability,
not merely an architecture example.

The proving scenario is:

1. One ESPHome provider instance manages one or more ESPHome nodes.
2. Each node owns one shared Native API connection, cache, reconnect lifecycle,
   and topology view.
3. One node may expose several switches with the same `switch@1` contract.
4. Every switch is independently discoverable, permissionable, bindable,
   observable, and controllable.
5. Device keys and facet keys remain stable across reconnects and provider
   generations.
6. Separate consumers can bind separate switches concurrently without creating
   duplicate node connections.
7. A deterministic protocol fixture is required for tests, but it does not
   replace a real Native API integration as the destination.

Before implementation, Wayfinder must resolve or research:

- ESPHome Native API framing, authentication/encryption, entity discovery,
  command and subscription semantics, reconnect behavior, and topology change;
- the first `switch@1` query/command/observation contract;
- state, event, revision, timestamp, lag, replay, and hydration semantics;
- shared transport/domain errors, deadlines, cancellation, and overload;
- provider queue/reentrancy/restart behavior; and
- the executable provider-endpoint/consumer-proxy WIT and host routing proof.

## 6. Completed work and historical evidence

### Unpublished branch boundary

After commit `1484940`, `main` is nine commits ahead of `origin/main` and zero
behind. Preserve all unpublished commits; do not push, rewrite, or publish them
without explicit approval.

- `1f41c19` — new auth flow.
- `2419814` — add passkeys.
- `36e8865` — format.
- `310c6ec` — upgrade Wasmtime runtime to 46.
- `c210c65` — migrate the plugin runtime and eight tracked plugins to WASI 0.3
  and native async; add exact-address TCP capability support and operational
  Samsung transport.
- `53543d2` — add product direction and architecture PRD.
- `8b4e759` — normalize the PRD for Markdown quality checks.
- `34cf442` — record detached-facet ownership and stable binding semantics.

### Historical runtime validation

The following results were recorded by the migration sessions for the
`c210c65`-era worktree. This worldview review did not reproduce them, they are
not current validation of the dirty worktree, and durable raw logs are not
attached. Every implementation ticket must rerun its relevant checks.

- `cargo test -p litehouse` passed.
- `cargo test -p litehouse-config` passed.
- `cargo test -p samsung` passed.
- All nine locally present plugin packages passed
  `cargo check --target wasm32-wasip1`.
- Real packaged components validated with `wasm-tools` and exposed native async
  runner exports plus WASI HTTP/sockets 0.3 imports.
- The migration patch was tested in an isolated worktree under the committed
  workspace edition 2024. This found and fixed Rust 2024 unsafe-attribute
  generation.
- A hosted Samsung smoke used a self-signed local TLS WebSocket server,
  exact-address TCP authorization, the remote-control endpoint, required
  `ms.channel.connect`, sent `ed.apps.launch`, completed transport futures, and
  verified that configuration/response tokens were absent from debug logs.
- Independent correctness and risk reviewers returned PASS after fixes.

### Samsung's deliberate boundary

- Samsung currently accepts any server certificate because TVs use self-signed
  TLS.
- Source and README comments explicitly document LAN impersonation and token
  interception risk.
- Exact destination authorization limits TCP to the configured TV address and
  port.
- Certificate enrollment/pinning is deferred by explicit user choice.

## 7. Implementation reality versus target

- The current plugin WIT still has fixed update variants and placeholder `a`/`b`
  output resource types. The service fabric is designed but not implemented.
- The host logs transitional outputs; it does not yet register typed endpoints,
  create consumer proxies, route cross-store calls, or enforce generations.
- Registry FlatBuffers and partition code represent the old plugin-only model.
- Cockpit currently provides experimental in-memory presence/pairing and log
  streaming, not the durable inventory or management protocol in the PRD.
- The site flow editor is primarily a static prototype, not a rule IR editor or
  executor.
- CoreOS files are experimental infrastructure, not a defined Litehouse
  appliance/update/recovery topology.
- The locally present Twilio package is an untracked ABI shell and does not
  implement Twilio behavior.
- MQTT, Presence, and Sonoff are honest no-op component skeletons, not completed
  integrations.

## 8. Open architecture decisions

The PRD's queue remains ordered. PRD items 1 and 2 are complete. Wayfinder must
preserve the original numbering and resolve items 3 through 13:

- **PRD 3 — State/event model:** revisions, ordering, duplicate handling,
  timestamps, hydration, lag, replay, resume, reconnect, and
  provider-generation effects.
- **PRD 4 — Shared error model:** transport versus domain errors, stale handles,
  denial, overload, deadlines, cancellation, and retryability.
- **PRD 5 — Contract artifact format:** normalized WIT, digests, docs, generated
  artifacts, conformance suites, and within-major compatibility checks.
- **PRD 6 — Registry publication/query:** immutable layout, partitions, cache
  manifests, atomic visibility, conditional reads, and JavaScript APIs.
- **PRD 7 — Signing/trust:** publisher identity, trust roots, rotation,
  revocation, and local policy.
- **PRD 8 — Pairing/sharing:** proof binding a physical server key to an account
  plus owner/member/support roles and scoped grants.
- **PRD 9 — WebRTC management protocol:** envelopes, negotiation, streams,
  cancellation, revisions, backpressure, resume, and audit.
- **PRD 10 — Rule IR:** typed triggers/conditions/actions, durability,
  simulation, traces, parity, and bounded execution.
- **PRD 11 — Appliance topology:** service/container boundaries, persistent
  data, secrets, update unit, rollback, backup, restore, and recovery.
- **PRD 12 — AI authorization:** read-only, plan-gated, confirmation-gated, and
  forbidden operations.
- **PRD 13 — Performance SLOs:** measured startup, memory, latency, overload,
  restart, and recovery targets.

Additional sharp decisions include:

- canonical workspace Rust edition and treatment of generated registry output;
- whether the untracked Twilio shell becomes a tracked plugin;
- exact HTTP authority matching across schemes, hostnames, IPv4, bracketed
  IPv6, explicit/default ports, absent authority, and fail-closed behavior;
- provider actor/store, queue, reentrancy, and restart/backoff policy;
- service/WASI authorization at declaration, bind, and invocation time;
- atomic native-async host/component rollout and stale-cache invalidation;
- how the Open Design package is committed/published; and
- whether project-local Wayfinder files and its local Markdown map are tracked.

## 9. Concrete blockers and TODO inventory

### Security and credentials

- Plaintext AWS credentials already exist in tracked `.mise.toml` at `HEAD`;
  they are not introduced by the current dirty hunk. Human-authorized work must
  rotate/revoke them, remove plaintext storage, and assess history-aware
  remediation. Never print, diff, or copy their values.
- The current dirty `.mise.toml` hunk adds a wasm-tools declaration. Keep that
  tooling change logically separate from credential remediation.
- Audit and remove the tracked hard-coded mail credential fallback without
  exposing its value.
- Define one secret injection, storage, rotation, and redaction policy spanning
  local development, CI, registry publication, website, Cockpit, appliance, and
  AI context.
- `crates/cockpit/src/client.rs` still has `TODO: validate password` in anonymous
  pairing.
- `site/components/toolbar.tsx` still has `TODO: Add admin role check to Better
  Auth session`.

External credential rotation or mutation requires explicit human approval and
must not be performed automatically by Wayfinder.

### Runtime resilience and regression evidence

- Add explicit overall/phase deadlines and cancellation tests for Samsung TCP,
  TLS, WebSocket handshake, first frame, send, and shutdown.
- Add bounded provider restart backoff and remove panic-prone restart paths.
- Add fail-closed HTTP authority tests, including IPv6 and port semantics.
- Add regression coverage for actual broadcast-event payload forwarding,
  output-discovery rejection, authority matching, restart behavior, stale
  generations, queue bounds, and cancellation.
- Define and test atomic rollout of the new host and rebuilt components,
  including stale cache invalidation and rollback.
- Preserve the temporary Preview 1/2 compatibility bridge until upstream Rust
  WASI 0.3 support is usable.

### Source markers requiring disposition

Verified tracked Rust/TypeScript markers:

- `crates/litehouse/src/util.rs`: `resolve_pointer` panics on JSON Schema keyword
  path chunks.
- `crates/registry/src/partitioned_store.rs`: `Index::insert` is `todo!()`.
- `crates/registry/src/partition.rs`: insertion uses a collision-prone fixed
  temporary path with `todo: use a real temp dir`.
- `crates/opendal-fs-cache/src/lib.rs`: `blocking_read` is `todo!()`.
- `crates/plugin/src/http.rs`: legacy `ureq` middleware `handle` is `todo!()`;
  decide whether to delete it or revive it under the current HTTP model.
- `crates/plugin-macro/src/lib.rs`: plugin metadata still needs a post-build
  injection design.
- `crates/cockpit/src/client.rs`: anonymous pairing password validation is
  missing.
- `site/components/toolbar.tsx`: admin-role authorization is missing.
- `crates/litehouse/src/runtime.rs`: component compilation notes an estimated
  7 MB allocation.
- `crates/litehouse/src/server.rs`: server setup notes an estimated 6 MB
  allocation.
- `crates/litehouse/src/cache.rs`: cache encoding notes an estimated 8 MB
  allocation.

The memory markers are optimization fog until baselines and SLOs prove their
priority. Generated `plugins/tasmota/out.rs` contains upstream TODO prose and is
not a source-owned task.

### Workspace and publication hygiene

- Preserve the dirty root `Cargo.toml` edition 2024-to-2021 change until the
  user explicitly resolves it. It can make workspace formatting differ from
  committed edition 2024.
- Preserve local explicit registry-edition and generated FlatBuffers changes
  until their ownership is resolved.
- Decide whether to track Twilio; do not silently absorb its pre-existing work.
- Keep unrelated untracked Expo app, benchmark files, game-server
  infrastructure, generated Tasmota output, `deny.toml`, and rustc crash reports
  out of platform commits unless explicitly brought into scope.
- Quarantine `.mise.toml`: do not stage it and do not emit any value-bearing
  diff into logs, issues, commits, or AI context until tracked credentials are
  removed through an approved remediation.
- `infra/coreos/factorio-backup.container` and
  `infra/coreos/wow/podman-login.service` are untracked, unrelated, and
  credential-bearing. Never print or stage them. Their object-store and
  registry credentials require separate human-authorized rotation/revocation.
- `app/android/app/debug.keystore` is untracked signing material. Never bulk-add
  `app/`, never expose or stage the keystore, and never use debug signing for a
  release artifact.

## 10. Design system and tooling status

### Open Design

- Open Design is checked out separately at
  `/home/arlyon/Programming/open-design`.
- The Litehouse package is at
  `/home/arlyon/Programming/open-design/design-systems/litehouse/`.
- Historical session reports say its manifest, 56 required tokens, Tailwind
  bindings, components, previews, and evidence passed targeted validators and
  `pnpm guard`. This review did not reproduce those checks.
- Historical session reports say the daemon was healthy at `/api/health` and
  the web UI returned HTTP 200. Recheck health and MCP discovery after reload
  before relying on them.
- The package remains untracked in the separate repository pending an explicit
  commit/publication decision.
- Pi has an absolute-path MCP configuration entry, but a fresh/reloaded session
  must verify discovery.

### Wayfinder

- Project-local Wayfinder is installed at `.agents/skills/wayfinder/`.
- `.claude/skills/wayfinder` is a relative symlink.
- `skills-lock.json` contains only `wayfinder`.
- The reviewed source revision was
  `66898f60e8c744e269f8ce06c2b2b99ce7660d5f`; the skill contains Markdown/YAML
  and no executable files.
- The installer briefly selected all repository skills because the
  `--skill=wayfinder` form was ignored in noninteractive mode. The other 38
  skills were removed and final CLI inventory reports exactly one skill.
- Commit `1484940` tracks the project-local skill files, symlink, and lockfile.
- No Wayfinder map has been created yet. That is intentional: first compact
  this worldview, then run Wayfinder in the fresh context.

## 11. Out-of-scope boundaries

Unless the destination is explicitly redrawn, do not include:

- arbitrary runtime reflection over unknown WIT contracts;
- implicit major-version adaptation;
- a central relay for routine home state or commands;
- a universal device ontology or full Matter model before small contracts prove
  the need;
- a public parent-device aggregate without a demonstrated device-wide
  transaction requirement;
- distributed multi-node runtime consensus;
- public marketplace moderation before signing/provenance exists;
- browser/edge WASM as a mandatory registry-query target;
- complex loops, sagas, or rich state machines in the first no-code rule IR;
- implementing every inventoried TODO merely because it exists. Every
  source-owned marker must receive an explicit disposition, but generated,
  upstream, and section-11 exclusions remain out of implementation scope unless
  separately approved;
- completing every skeletal integration as a prerequisite for the ESPHome
  switch slice;
- the untracked Expo starter without a separate mobile-product decision;
- unrelated Factorio/WoW infrastructure; or
- rustc crash reports as product artifacts.

## 12. Workspace safety contract

- Preserve all pre-existing dirty and untracked work.
- Never reset, clean, stash, overwrite, or stage unrelated files.
- Stage explicit approved paths only. Do not stage `.mise.toml`, credential-
  bearing infrastructure, signing material, or bulk directories. Never emit
  value-bearing secret diffs for review.
- Do not push, publish, deploy, restart services, mutate external trackers,
  rotate credentials, or alter auth without explicit approval.
- Use isolated worktrees for risky or exact-commit validation.
- Keep implementation commits cohesive and unsigned unless the user changes
  that policy.
- Current project hooks run Rust and Taplo formatting through Mise. The invalid
  local cargo-tool declaration was removed in `c210c65`; the dirty workspace
  edition may still require exact-commit validation under edition 2024.

## 13. Validation contract

For every implementation ticket:

1. State the observable behavior and highest stable test seam.
2. Run diagnostics before expensive builds.
3. Add behavioral and meaningful error-case coverage.
4. Run targeted checks while iterating.
5. Run the repository-native completion check or state why it cannot run.
6. Build and validate real components when ABI or guest behavior changes.
7. Inspect final status/diff for generated artifacts, secrets, and scope drift.
8. Report exact commands, outcomes, warnings, and residual risks.

For the contract kernel and ESPHome slice, tests must prove:

- multiple same-contract switch facets on one shared connection;
- exact-major routing and no accidental major adaptation;
- provider-store targets and consumer-store-local proxies;
- deterministic `one`/`optional`/`many` resolution;
- same-identity-only rebinding and stale old handles;
- bounded dispatch, cancellation, overload, and trap isolation;
- real Native API behavior plus deterministic fixture coverage; and
- no token/credential leakage in normal diagnostics.

## 14. Post-compaction Wayfinder handoff

After this document is preserved according to the compaction precondition and
the conversation is compacted:

1. Reload Pi if needed so project-local `wayfinder` is discoverable.
2. Invoke Wayfinder explicitly for the entire Litehouse platform.
3. Use the local Markdown tracker default unless the user chooses an external
   tracker:

   ```text
   .scratch/litehouse-platform/map.md
   .scratch/litehouse-platform/issues/NN-<ticket>.md
   ```

4. Set the map destination to section 2: explicitly inventory and disposition
   every relevant source-owned TODO, and include the real ESPHome multi-switch
   integration. Inventory does not authorize implementation.
5. Import the decisions in section 4 and the PRD as decisions-so-far. Do not
   reopen them without contradictory evidence.
6. Map sharp decisions and prerequisite research/tasks breadth-first. Keep
   downstream implementation in fog until blockers are resolved.
7. Record the workspace-safety and validation contracts in map Notes.
8. Chart only in the first Wayfinder session. Then compact or hand off before
   resolving the first human decision ticket.
9. Research tickets may run in parallel. Human decision tickets proceed one at
   a time with a recommendation.
10. Wayfinder remains planning-only by default. A cleared route does not
    authorize implementation, commits, deployments, restarts, credential
    changes, tracker mutations, or other external effects. Obtain explicit
    approval for each implementation scope and retain all workspace-safety
    gates.

Likely initial frontier:

- define state/event semantics;
- research ESPHome Native API multi-switch behavior;
- research HTTP authority normalization;
- decide the canonical workspace edition/baseline;
- decide secret storage/rotation policy, with credential rotation as a separate
  human-authorized task; and
- decide whether Wayfinder/Open Design/Twilio artifacts are tracked.

## 15. Compact checkpoint

Commit `1484940` satisfies the worldview durability precondition. Compaction
may proceed.

At compaction, preserve at minimum:

- this document's path and source-of-truth order;
- the destination and ESPHome priority;
- all nine unpublished commits from `1f41c19` through `1484940`;
- confirmed detached-facet and stable-binding decisions;
- the fact that no Wayfinder map exists yet;
- the instruction to run Wayfinder only after compaction;
- project-local Wayfinder installation paths and untracked status;
- Open Design package/MCP status;
- credential and dirty-worktree safety boundaries; and
- active task-list intent: preserve or explicitly accept the worldview,
  compact, then chart the planning-only Wayfinder frontier. Implementation
  requires separate explicit approval.
