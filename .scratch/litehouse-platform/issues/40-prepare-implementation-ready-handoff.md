# Prepare the implementation-ready handoff

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-16
Blocked by: 20, 21, 22, 26, 33, 34, 35, 36, 37, 38, 39, 44, 47
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Can FR1–FR16, the narrowed connected-release journey, every resolved decision,
security/workspace boundary, source residual, prototype finding, accessibility
requirement, rollout/rollback gate, and residual risk be converted into an
ordered set of bounded, separately approvable implementation tranches with
stable acceptance seams, targeted/canonical checks, and no hidden execution
authority?

## Answer

Yes. Deliver the first connected release through the separately approved
tranches below. Each tranche has one stable behavioral seam, explicit non-scope,
targeted evidence, rollback, and an authority boundary. No tranche approval
implicitly approves the next tranche or any commit, publication, credential,
external service, staging/production deployment, server mutation, container
restart, DNS change, email, or household-device access.

Later user-selected decisions supersede stale architecture-PRD language. The
PRD's FR1/FR5/FR10/FR11/FR14/FR15 and representative acceptance/check text align
to direct `wasm32-wasip2`, one physical device per instance, R2 plus one
Rust/Wasm query core, durable rule/management behavior where T8 requires it, a
basic direct WebRTC JSON-RPC path, and a portable first-release container without
cross-version restore machinery. Its old delivery phases are historical only;
this issue is the implementation ordering authority.

The web/control plane is greenfield. Legacy `site/` source, routes, components,
auth implementation, transport, state, and Turso/libSQL data are evidence only
and are not copied or migrated. D1 starts clean. The legacy deployment may remain
temporarily as an independent route rollback origin, never as a source, runtime
dependency, or data authority.

Matter/Thread/Matter Server issues 54–57 remain post-first-release planning and
are absent from every tranche below.

## Global delivery contract

Every tranche follows this sequence:

1. start from the reviewed source baseline in an isolated clean worktree;
2. carry only the approved tranche and preserve all unrelated dirty/untracked
   work;
3. restate observable acceptance, stable seam, targeted checks, canonical check,
   migration/rollback, and external authority before editing;
4. implement the smallest coherent root fix with no compatibility path or new
   dependency unless the tranche requires it;
5. run diagnostics and targeted fixtures before expensive builds;
6. inspect generated artifacts, final diff/status, credentials, scope drift, and
   rollback evidence;
7. obtain independent correctness and risk review for security/cross-cutting
   tranches; and
8. stop for separate approval before commit/publication/deployment/operation.

No implementation reads or edits protected local tool configuration, signs with
real keys, contacts R2/Cloudflare/D1/email/TURN, accesses a server/device, or
runs a publishing workflow unless that exact external action receives separate
approval. Fixtures use fake time, fake DNS/TLS, local Worker/workerd, isolated
files, dummy keys, and disposable databases/buckets only when separately allowed.

Do not invent fixed runtime, component, cache, container, queue, memory, CPU, or
PID limits without a concrete format/protocol requirement or measured failure.
Input-format bounds required for deterministic parsing remain part of their
format. No benchmark or optimization tranche exists without a reproducible
production-path regression.

## Evidence versus production inputs

### Evidence only

- `.scratch/litehouse-platform/prototypes/*` and their successful typed-service,
  ESPHome, registry, native/Wasm parity, and R2 results;
- historical supervised Node S evidence and disposable R2 proof;
- the current untracked `web/` TanStack/Open Design/capability-renderer gate;
- the vendored Open Design snapshot until its source release/tag/commit and
  bundle digest are separately approved;
- legacy `site/`, its auth/database/routes/flow editor/Cockpit polling, hard-coded
  registry data, and bearer/static-password stubs;
- current no-op/persistent-task fixtures and Samsung source; and
- current registry writer/index, N-API path, generic OpenDAL cache, plugin HTTP
  helper, and other residuals assigned for replacement/deletion by issue 39.

Evidence may inform tests and semantics but is never copied wholesale into
production without tranche review.

### Production seams to replace or extend

- `crates/plugin-macro/wit`, `crates/plugin-macro`, `crates/plugin`, generated
  contract/adapters, and first-party provider crates;
- `crates/litehouse`, `crates/litehouse-config`, auth/server/store/runtime/cache/
  rule/management modules;
- `crates/registry` and its schemas, IO/cache/query/publication paths;
- greenfield `web/`, a signaling-only Cockpit Worker/DO, clean D1 schemas, and
  central/local issuer adapters;
- container/build/release metadata and optional Butane/Quadlet generator; and
- generated HTTP/WebRTC/web/MCP operation adapters over one canonical handler
  set.

## Ordered implementation tranches

### T0 — Workspace and execution-safety baseline

**Prerequisites:** issue 34 and issue 39 disposition.

**Scope:** establish Rust 2024 and pinned project-native tools from a clean
worktree; make WIT/FlatBuffer generation reproducible; define non-publishing CI
for implementation branches; make schema diagnostics return no span rather than
panic on JSON-Schema keyword chunks; inventory exact production workspace/release
members.

**Not scope:** product behavior, protected tool config, credential remediation,
registry schema, provider cleanup, publication, or deployment.

**Acceptance:** clean `cargo metadata`; keyword/property/index/missing/deep schema
fixtures; two clean generations are byte-identical; ordinary validation cannot
publish or use real credentials; no unrelated edition/lock/generated changes.

**Rollback:** revert one baseline patch. Generated output is regenerated, never
hand-edited.

### T1 — Contract, quantity, and generated-operation kernel

**Prerequisites:** T0.

**Scope:** publish source contracts for the nine exact-major capabilities,
`litehouse:errors@1`, state/event/receipt records, UCUM quantity engine/profile,
runtime-profile package, provider/consumer bindings, config annotations, and one
closed `management-operations-v1` manifest used to generate HTTP/WebRTC/web/MCP
adapters. Replace zero-length/post-build metadata placeholders with typed runtime
metadata plus signed release metadata inputs.

**Not scope:** endpoint routing, provider networking, registry publication,
rules, UI, or external commands.

**Acceptance:** deterministic WIT/source/binary/digest vectors; exact-major
coexistence and same-major compatibility checks; canonical-decimal/UCUM valid and
invalid vectors in Rust and Wasm; generated Rust components build directly for
`wasm32-wasip2`; `wasm-tools` validates exact runtime-profile imports and native
async exports; every operation has fixed scope, idempotency, revision, deadline,
error, redaction, and projection metadata.

**Rollback:** host/contracts/adapters are one compatible release set; retain the
preceding verified set and reject mismatched profile imports before activation.

### T2 — Typed service fabric and binding

**Prerequisites:** T1.

**Scope:** provider-store resources, consumer-store-local typed proxies, endpoint
registry, exact-major resolution, deterministic `one`/`optional`/`many` selectors,
facet incarnations, state/event identities, operation grants, cycle rejection,
and same-identity-only rebinding.

**Not scope:** real provider protocols, registry, rules, web, or cloud.

**Acceptance:** real provider/consumer fixture components prove repeated facets,
independent consumers, v1/v2 coexistence, canonical many ordering, ambiguity,
never/same-identity rebinding, provider restart/stale proxies, typed errors, and
zero Wasmtime resource handles crossing stores.

**Rollback:** atomic host/component rollback; no reflection or implicit major
adapter.

### T3 — Minimal runtime resilience

**Prerequisites:** T2.

**Scope:** preserve the existing runtime, in-process Wasmtime compilation,
incremental cache, store strategies, capability hooks, event loop, and trap
restart path. Fix cache-corruption panics, add queue-inclusive lifecycle/update
deadlines with epoch interruption for non-yielding guests, disable only an
affected subscribed actor on event continuity loss, restart traps only for the
default per-instance strategy without replacing shared sibling stores, support
backward-compatible per-instance capabilities, inject only explicitly declared
protected environment values, and replace relevant input-driven panic paths with
errors.

**Not scope:** compiler sandboxing, serialized compiled-code loading, replacement
cache, hard component-size limit, custom DNS/IANA policy, durable event replay,
global activation/rollback transactions, host restore, arbitrary resource limits,
production service-fabric endpoint publication before a real T6 provider, new
diagnostics subsystem, registry, providers, rules, UI, or cloud.

**Acceptance:** focused issue-38 tests prove corrupt cache rebuild, yielding-I/O
and non-yielding-CPU deadlines, subscribed event continuity loss without silent
continuation, shared-store sibling preservation, per-instance capability
isolation with legacy compatibility and unique IDs, protected-value
non-persistence/bounds, ordinary
error handling, and preservation of the existing compiler/cache/event/output and
`run_concurrent` behavior.

**Rollback:** revert the focused T3 commit; no new durable format, activation
pointer, compiler artifact, or host restore protocol is introduced.

### T4 — Greenfield TanStack/Open Design provisional local gate

**Prerequisites:** T1 and a separately approved Open Design release identity/
bundle digest. May proceed in parallel with T2–T3.

**Scope:** review/adopt only the greenfield `web/` package as a **provisional**
TanStack candidate; exact-pinned Start/Cloudflare Worker build; digest-locked
released Open Design source; local `--lh-*` token adapter; route/cache/error
primitives; exhaustive nine-facet view models/renderers; native accessibility
foundations; workerd and browser fixtures. T4 does not make the final TanStack-
versus-React-Router decision.

**Not scope:** legacy `site/` source/state, auth, D1, WebRTC, commands, registry
writes, React Flow persistence, staging, or deployment.

**Acceptance:** `pnpm --dir web check`; built Worker executes in workerd; route-
class/private-cache fixtures; malformed view-model tests; six-width reflow,
44-pixel targets, skip/focus, keyboard, 200% text/400% zoom, light/dark, reduced-
motion, and forced-colors evidence; bundle digest drift fails closed. Use React
Router 8 only if the documented objective TanStack gate fails.

**Rollback:** remove the isolated package/adoption patch. The Open Design source
remains static vendored input, never a runtime dependency. Final framework
adoption waits for T12's integrated auth/registry/cache gate and T14's local
candidate journey/build gate; any issue-50 failure switches to
React Router 8 and repeats the affected evidence.

### T5 — R2 registry, trust, cache, and install locks

**Prerequisites:** T1 and T3.

**Scope:** replace the current registry with immutable blobs/release shards/root,
seven versioned partitions, complete signed Ed25519 release records, object-first
publication, ETag CAS, R2-only adapters, one Rust/native-Wasm query core,
resumable verified cache, reviewed install/update plans, and one offline
predecessor. Enforce the 16 MiB component ceiling.

Delete the old writer/index/temp mmap path, old schema/bindings, generic
`opendal-fs-cache`, N-API query path, and S3 fallback with this replacement.

**Not scope:** live R2 publication, public third-party onboarding, automatic
updates, alternate registry/database, or production signing-key custody/use;
T5b owns the publisher/custody implementation and T15 owns approved external
evidence.

**Acceptance:** deterministic root/shard/release/signature vectors; native/Wasm
byte parity; malformed/unknown schema, digest, signature, yank/withdraw/revoke,
range/resume, offline, cache-corruption, concurrent CAS, hidden activation, and
rollback fixtures; source/dependency scans prove old paths absent.

**Migration/rollback:** publish a new object/schema namespace; immutable objects
are never rewritten; root pointer switches last; prior root/artifacts remain
readable through the bounded rollback window. A live disposable R2 smoke is a
separate explicit action.

### T5b — Protected publisher and signing-custody integration

**Prerequisites:** T5 and issue 25. Real-key or external-service activity requires
its own explicit authorization.

**Scope:** implement the first-party CLI/CI publisher, domain-separated plugin
and OCI signing identities, trust-bundle key IDs, two-key rotation overlap,
revocation/withdrawal inputs, R2 object/root writer role, cache-purge role, release
manifest/provenance/SBOM binding, and policy-as-code for least-privilege CI.
Signing, R2 writing, cache purge, image publication, D1, and deployment roles are
separate and non-convertible.

**Not scope:** reading/importing existing private keys, creating production keys,
real signatures, live R2/cache mutation, image publication, or deployment.

**Acceptance:** dummy/HSM-emulator vectors prove domain separation, rotation,
revocation and complete payload binding; role-policy fixtures and negative CI
jobs prove each identity cannot assume another role or access D1/auth/Cockpit/
deployment; logs expose only safe key IDs/digests; compromise drills show plugin
signer cannot sign OCI and publisher cannot derive signing material. T15 owns the
separately authorized provider-backed ceremony/evidence.

**Rollback:** revoke/disable the candidate role and retain the prior trust bundle/
publisher. Running verified plugins are not remotely killed; future activation
honors revocation.

### T6a — ESPHome production provider

**Prerequisites:** T1–T3; T5 before signed install/release evidence.

**Scope:** one configured physical node/session/cache per instance; exact
advertised On/Off, Level, Color, Boolean, Measurement, Thermostat, Covering, and
Momentary facets; stable host identities; topology reconciliation; truthful
state/availability and accepted-delivery receipts.

**Not scope:** discovery/mDNS, firmware changes, nested devices, unsupported
feature inference, generic services, or a notifier facet.

**Acceptance:** deterministic Native API/Noise fixtures cover repeated facets,
all mapped families, source representations, grouped non-transactional commands,
reconnect/lag/stale/incarnation/topology changes, cancellation, malformed input,
redaction, and no fabricated acknowledgement/range/unit/state. Historical Node S
proof is supporting evidence only; fresh hardware access is separately approved.

**Rollback:** disable/revert the independent signed artifact; preserve instance
identity and ambiguous receipts.

### T6b — Tasmota HTTP production provider

**Prerequisites:** T1, T3; T5 before release.

**Scope:** explicit RFC1918 IPv4 node, exact port/profile, environment-only web
password, absolute power channels, and the issue-53 measurement whitelist.

**Not scope:** MQTT, discovery, arbitrary commands, public/hostname HTTP,
relative toggles, or firmware control.

**Acceptance:** fake request recorder and response fixtures cover exact authority,
auth redaction, multi-channel identity, topology remap, every measurement/unit,
malformed/oversized response, unavailable/stale state, before/after-write
ambiguity, and no blind replay.

**Rollback:** independent artifact predecessor; no real LAN node access.

### T6c — ntfy production provider

**Prerequisites:** T1, T3; T5 before release.

**Scope:** `notification-sink@1` with one fixed configured HTTPS origin/topic,
environment-only token, bounded body/title/priority, and acceptance-only receipt.

**Not scope:** generic webhook, arbitrary URL/header/method, delivery/read claims,
Twilio, or ambiguous retry.

**Acceptance:** local fake server covers every allowed status, redirect denial,
payload/header bounds, token redaction, timeout/disconnect before/after send,
`outcome-unknown`, and proof that callers cannot alter origin/topic or replay an
ambiguous send.

**Rollback:** independent artifact predecessor; no production ntfy endpoint.

### T6d — Open-Meteo production provider

**Prerequisites:** T1, T3; T5 and attribution consumers before release.

**Scope:** deliberate replacement/rename of the current weather shell; fixed
non-commercial Open-Meteo endpoint/profile; exactly the four issue-53 current-
model measurements; mandatory adjacent CC BY 4.0/Open-Meteo attribution.

**Not scope:** commercial profile, arbitrary forecast API, provider selection,
geocoding, forecast history, or hidden attribution.

**Acceptance:** fixed-query/response/unit/time/stale/rate/error fixtures; sibling
instance isolation; bounded cache; adjacent attribution in API/web/MCP/export
projections. Withhold the artifact if any exposing surface cannot guarantee it.
No live API is required.

**Rollback:** independent artifact predecessor; removing it also removes its
attributed surfaces.

### T6e — Plugin portfolio cleanup

**Prerequisites:** T6a–T6d implementations or explicit release-manifest stubs.

**Scope:** keep no-op/persistent-task as non-published fixtures; remove MQTT,
Presence, Sonoff, and Twilio shells after reference checks; leave Samsung source
as deferred evidence but exclude it from default workspace release/image/
registry/readiness.

**Acceptance:** workspace and release-manifest tests enforce exactly four V1
providers and reject development/deferred/deleted packages; no production
consumer/import remains before deletion.

**Rollback:** cleanup is independently revertible and never restores publication
eligibility without a new provider decision.

### T7 — Rule JSON, compiler, and pure planner

**Prerequisites:** T1–T2.

**Completed scope:** the first closed server-side Rule JSON subset: one state
observation trigger, typed immutable snapshots, Boolean/string/i64/quantity
comparisons, explicit branching, exact On/Off set intents, typed ends,
deterministic DAG compilation, and pure pre-state-to-plan simulation.

**Not scope:** the remaining V1 node/trigger vocabulary, CEL/JCS/browser-Wasm,
selector resolution, effect dispatch, restart-clean execution, React Flow
persistence, loops/sagas, or optimistic physical state. T8 completes the designed
V1 language before executing it.

**Acceptance:** strict schema/shape/bound fixtures, graph/type/cardinality and
quantity validation, deterministic planning, unknown/stale input, and proof the
planner cannot call a provider or mutate state.

**Rollback:** schema/evaluator versions are immutable/pinned; migration creates a
new draft and never rewrites an active rule.

### T8 — Persistent management core and restart-clean rule executor

**Prerequisites:** T3 and T7.

**Scope:** atomically persisted instance/rule drafts, immutable revisions,
active/predecessor pointers, grants, and enabled state in Fjall 3.1.7; the
complete bounded Rule JSON V1 language and process-local executor; typed source-
push consumption plus bounded polling through the unchanged provider ABI;
drafts/validate/plan/apply/activate/rollback; canonical operation handlers;
optimistic revisions and bounded process-local idempotency; state/event/config/
registry/rule/diagnostic projections; and fixed OAuth scope requirements. Use
single-writer cross-keyspace `SyncAll` transactions in a fresh Fjall V3 database,
RFC 8785 canonicalization via `serde_json_canonicalizer` 0.3.2, bounded CEL
parsing via default-feature-free `cel` 0.14.0 with a Litehouse-owned checker and
evaluator, and Jiff 0.2.32 with bundled tzdb 2026c.

**Not scope:** durable runs, microsteps, timers, trigger queues, effect outbox,
receipt continuation, catch-up, replay, compensation, or idempotency tombstones;
transport-specific behavior, UI, new MCP tools, external audit DB, or host escape.

**Acceptance:** authored state is atomic across crash/reopen; a second database
process is rejected; stale edits conflict; process-local duplicate requests
deduplicate; restart discards every unfinished run/timer/queue/effect and stales
old run IDs/cursors; each rule waits for its own hydration barrier before arming;
time schedules continue only at the next future occurrence; ambiguous effects
are never retried; disable/drain and predecessor rollback remain explicit;
authorization, redaction, gaps, and projections are consistent across fake
adapters. Native, `wasm32-wasip2`, and browser-Wasm rule dependencies compile,
and observed physical state never changes merely because delivery was accepted.

**Rollback:** active rule plus predecessor remain pinned; rollback revalidates and
never rewinds external effects. Removing the executor leaves persisted authored
state intact and no unfinished work is resumed.

### T9 — Portable local server and container

**Prerequisites:** T3, T5, T8.

**Scope:** one persistent data directory and instance lock, configured local
HTTPS/API, health/readiness, graceful shutdown, a conventional non-root read-only
`linux/amd64` and `linux/arm64` OCI image, recorded image digest with basic
architecture smoke tests, and a small optional Butane/Quadlet generator.

**Not scope:** cross-version backup/restore, receipt watermarks, N-1 migration
rollback, prescribed CPU/memory/PID/tmpfs limits, protected image signing or
publication, extensive OCI provenance infrastructure, Litehouse OS, VM appliance,
auto-update, PKI, secret store, arbitrary host path/device, or host mutation.

**Acceptance:** both images build and start non-root; metadata/digest and
architecture are correct; one writable data directory and exclusive lock work;
health/readiness/shutdown work; no secret is embedded; representative Butane
transpiles to Ignition and generated Quadlet syntax validates.

**Rollback:** revert the image/generator source. T9 defines no cross-version data
restore claim; the first incompatible future migration must design against its
actual predecessor.

### T10 — Clean central D1 identity, inventory, and pairing

**Prerequisites:** T4 and T9.

**Scope:** greenfield `litehouse-web` Worker; clean D1 schema; one-owner Better
Auth account/passkey/magic-link recovery; OAuth/OIDC issuer/resource metadata;
durable server inventory; key-proof pairing/pair epochs; signed short-lived
management/signaling grants; bounded email adapter. Delete the old anonymous
Basic/password Cockpit pairing handler and route in this tranche before T11 adds
new signaling.

**Not scope:** legacy site/Turso import, sharing roles, routine management relay,
Cockpit UI, D1 in Durable Objects, or production bindings.

**Acceptance:** local Worker/D1 transaction and crash injection; clean bootstrap;
owner isolation on every query/mutation; PKCE/device/recovery/passkey counters;
pair code/key/epoch/replay/reconcile; JWT key rotation/outage; private cache
isolation; route/import scan proving the anonymous password handler is absent;
no legacy import/dependency; and no secret/token/email-body logging.

**Migration/rollback:** D1 starts clean and uses expand/contract migrations.
Framework, auth origin, D1, email, DNS, and route changes are separate deployment
steps; rollback uses a compatible prior Worker, never legacy data.

### T11 — Signaling-only Cockpit and basic WebRTC RPC

**Prerequisites:** T8–T10.

**Scope:** the authenticated broker facilitates signaling only. Browser and home
server then use one direct reliable ordered WebRTC data channel carrying standard
JSON-RPC 2.0. The first `session.open` call validates a short-lived T10 grant;
selected web-UI methods dispatch to the same T8 handlers as HTTP; responses use
standard JSON-RPC results/errors and server notifications invalidate/update UI
state. Reconnect creates a fresh channel, authenticates again, and reloads state.

**Not scope:** management relay, raw SDP/DTLS transcript hashing, custom binary
frames, transfer protocol, credit windows, fixed SCTP thresholds, in-place
reauthorization, durable stream cursors/replay, transport resume, automatic
mutation replay, transport-specific idempotency/result stores, mandatory custom
heartbeat, TURN, arbitrary HTTP tunneling, or arbitrary data channels.

**Acceptance:** direct broker-facilitated connection; RPC denied before
`session.open`; valid/invalid/expired/wrong-server/stale-pair grants; JSON-RPC
request/result/error/notification fixtures; representative HTTP/WebRTC handler
parity; malformed/unknown-method handling; fresh reconnect and state reload; no
automatic mutation replay; and proof routine payloads are neither relayed nor
stored by the broker.

**Rollback:** remove/disable the JSON-RPC data-channel adapter while preserving
canonical HTTP handlers and broker signaling.

### T12 — Connected greenfield web journeys

**Prerequisites:** T4–T11 and T6a–T6d attribution/control contracts.

**Scope:** fresh account/inventory/pairing, registry review/install/update,
instance configuration/environment requirements, device/facet controls, Rule JSON
React Flow plus synchronized semantic outline, simulation/activation/runs,
activity/diagnostics, reconnect/ambiguity recovery, and generated operations only.

**Not scope:** legacy imports/state, generic API/form escape hatch, optimistic
observed state, PWA mutation queue, central management relay, or deployment.

**Acceptance:** `pnpm --dir web check`; operation-manifest coverage with no
hand-coded transport call; browser journey from clean account to paired server to
provider/rule; unknown/stale/offline/unavailable/outcome-unknown truth; blocked
replay; disconnect hydration; Open-Meteo attribution; keyboard-equivalent rule
editing; screen reader/contrast/focus/reflow/reduced-motion/forced-colors/coarse-
pointer evidence at 320–1440 pixels. Reconnect opens a fresh authenticated RPC
session and reloads current state. The integrated issue-50 gate also proves
Better Auth/D1 journeys, lazy R2/Rust-Wasm rendering, public/private cache
segregation, purge failure bounds, server-function compatibility, production
build stability, and local rollback. Failure switches to React Router 8 before
T14.

**Rollback:** route-by-route greenfield Worker versions; legacy deployment is an
independent temporary origin only. T12b owns source retirement.

### T12b — Legacy site source retirement

**Prerequisites:** T12 integrated parity and framework decision.

**Scope:** delete the complete legacy `site/` package, toolbar/admin TODO,
Next/Vercel-only dependencies/config/routes, old auth/database/transport/state
code, and root-workspace references. T10 already owns deletion of the legacy
Cockpit anonymous pairing handler; T12b verifies rather than reowns it.

**Not scope:** importing/migrating any source or Turso/libSQL state, deleting an
independently deployed rollback artifact, DNS cutover, or production mutation.

**Acceptance:** dependency/import/route/data-migration scans prove `web/` has no
legacy source/state edge; workspace manifests/locks contain no unused site-only
packages; the greenfield T12 suite remains identical; the toolbar/admin and site
TODOs are absent; T10's old pairing route remains absent.

**Rollback:** revert only the source-cleanup patch for investigation. Reversion
does not make legacy code a supported source, runtime dependency, or release
member.

### T13 — Small MCP adapter

**Prerequisites:** T8–T9.

**Scope:** official `rmcp` Streamable HTTP over authenticated local HTTPS and a
small explicitly approved subset of generated canonical operations: safe
inspection, useful state reads, and essential reversible controls already exposed
by the web UI. The exact initial list is selected before T13 implementation.

**Not scope:** automatic one-tool-per-operation exposure, generic API caller,
host/file/shell/container/network tools, secret values, ownership/trust changes,
registry publication, central gateway, prompts/chat UI, or a separate MCP policy
or quota system.

**Acceptance:** approved small tool inventory; each tool maps to exactly one T8
handler; HTTP/MCP semantic parity; normal scope/redaction enforcement; MCP SDK
smoke/conformance; and absence of generic or unselected operations.

**Rollback:** disable/remove only the thin adapter; canonical HTTP/WebRTC/web
behavior remains unchanged.

### T14 — Connected-release integration and release-candidate proof

**Prerequisites:** every prior release tranche. Matter work is excluded.

**Scope:** assemble the compatible host/runtime profile, four separately
**test-signed** provider artifacts, R2 fixtures, clean D1/Workers, container,
web/Cockpit, rules, and the approved small MCP subset; execute the complete clean
login → pair → review/install → configure → control → rule simulate/activate →
inspect/MCP journey in an isolated environment. Record the source revisions,
artifact/image digests, generated-file status, and rollback instructions needed
to identify that candidate.

**Not scope:** live publication, production/staging cutover without approval,
real credentials/email/TURN, server changes, DNS, or household hardware.

**Acceptance:** all targeted matrices; canonical checks below; generated digest
inventory; plugin-set, rule, Worker, image/data, and route rollback drills;
WCAG 2.2 AA evidence; credential/pattern scan; final diff/status/scope review;
and independent adversarial correctness/risk PASS with no waived blocker.

**Rollback:** demonstrate each implemented local rollback independently. T14 is
the first connected release-candidate gate. External staging and publisher
qualification are later T15 work.

### T15 — Deferred external staging and publication qualification

**Prerequisites:** T14 PASS and a later explicit decision to define T15.

T15 is a placeholder for real provider-backed staging, signing, publication, and
cloud deployment qualification. It is not part of first-release implementation
or the T14 local candidate gate. No provider, IAM layout, signing topology,
staging domain, observability product, retention policy, recovery drill, or TURN
service is selected by this placeholder.

When T15 is intentionally started, first present a proportionate scope based on
the exact T14 candidate and selected deployment providers. Every credential,
signing, R2/OCI/D1/Worker/DO, DNS/route, deployment, rollback, and cleanup action
requires separate explicit authorization. T15 never includes production or
household/server mutation implicitly.

## Critical path and safe parallelism

```text
T0 workspace
  -> T1 contracts/operations
  -> T2 service fabric
  -> T3 runtime/resilience
  -> T5 registry -> T5b publisher -------┐
  -> T6a ESPHome -------------------------|
  -> T7 rule planner -> T8 management     |
  -> T9 local container/server            |
  -> T10 central identity/D1 -> T11 RTC   |
  -> T12 connected web -> T12b cleanup ---+-> T14 local candidate
  -> T13 small MCP subset ----------------┘

T15 provider-backed staging/signing qualification is a later separately selected
and authorized route after T14; it does not block the local candidate.
```

T4 may proceed after T1/Open-Design approval in parallel with T2–T3 but remains
provisional through T12/T14. T6a–T6d are independent after T1/T3 and finish
signed install evidence after T5; T5b may proceed once T5's formats pass. T6e
waits for consumer scans. T7 may proceed after T2; T13 may proceed after T8/T9
without waiting for central cloud work. Parallel work never shares an activation pointer, generated artifact,
D1 schema, or deployment target without an explicit integration owner.

## FR1–FR16 traceability

| Requirement | Owning evidence |
|---|---|
| FR1 native async/runtime profile | T1, T2, T3 |
| FR2 exact majors/coexistence | T1, T2 |
| FR3 typed provider endpoints | T1, T2, T6a–T6d |
| FR4 host-owned cross-store proxies | T2 |
| FR5 one-device instance/repeated facets | T2, T6a–T6d |
| FR6 deadline/stale-generation/isolation/capability scope | T3 |
| FR7 manifest contracts/permissions | T1, T3, T5 |
| FR8 deterministic typed binding | T2, T7 |
| FR9 rules use the same contracts | T7, T8 |
| FR10 immutable contract/release/local lock | T5, T5b, T15 |
| FR11 native/Wasm JS registry queries/R2 cache | T5, T12, T15 |
| FR12 durable authorized inventory | T10, T12, T15 |
| FR13 authenticated direct WebRTC JSON-RPC for the web UI | T11, T14 |
| FR14 scoped/revisioned/idempotent durable mutations | T8, T10, T11, T13, T14 |
| FR15 install/pair/update/local diagnostics | T5, T9–T12, T14 |
| FR16 structured inspect/validate/plan/apply for AI | T8, T13 |

The architecture PRD's eleven acceptance scenarios map respectively to T1–T3,
T2/T6a, T2/T6a, T2, T1/T2, T5/T5b/T15, T10–T12/T15, T7/T8/T12,
T3/T12, T9/T15, and T8/T13/T14.
No scenario substitutes a mock for the stable seam it is meant to prove.

## Generated-artifact ownership

| Artifact | Tracked authority | Generated/validation rule |
|---|---|---|
| WIT contracts/runtime profile | canonical source packages and exact dependency lock | Rust/Wasm bindings and binary WIT are pinned generator output; regenerate and byte-compare, never hand-edit |
| Quantity/error/state fixtures | canonical typed JSON/binary vectors | Rust/Wasm/TypeScript projections must be byte/semantic identical |
| Management operations | one versioned closed manifest | HTTP/WebRTC/web bindings and the explicitly selected MCP subset project from it; no transport-only operation |
| Rule JSON/port catalog | versioned schema/catalog source | Rust/browser-Wasm validator/compiler vectors and semantic digests match |
| Registry | FlatBuffer schemas, partition/signature versions, release locks | bindings/root/shard/signature vectors generated from pinned tools; old schema never edited in place |
| Activation/config | source schemas/annotations plus signed artifact IDs | deterministic environment requirement names and `ActivationSet` bytes; values never generated/persisted |
| D1 | reviewed expand/contract migrations | generated types may be refreshed; clean bootstrap and migration hashes are tested; no legacy import |
| Web/Workers | source routes/bindings/cache classes and exact package lock | TanStack route/build output is project-generated; workerd/Wrangler dry run must consume the built Worker |
| Container/Butane | Docker/build/generator source | amd64/arm64 image metadata/digest and representative Ignition/Quadlet fixtures are reproducible and contain no secret |

Every release candidate records source revision, tool/runtime profile, artifact
and bundle digests, generated-file status, compatibility range, and predecessor.

## Validation commands and completion evidence

Run tranche-specific diagnostics/tests first. The repository canonical Rust
completion set is:

```bash
mise exec -- cargo fmt --all --check
mise exec -- taplo fmt --check
mise exec -- cargo groups test bin
mise exec -- cargo build --release -p litehouse
```

Contract/provider tranches additionally build their real `wasm32-wasip2`
components and run `wasm-tools validate --features cm-async`. Registry uses its
native/Wasm parity and object/crash suite. Greenfield web uses:

```bash
pnpm --dir web check
```

Cloudflare evidence remains local workerd/Worker integration and Wrangler dry
run through T14. T15 provider-backed staging/signing/observability qualification
is later work requiring a separate decision and explicit external approval.
Container/Butane commands are defined by T9 and run only in a disposable approved
environment. No canonical local completion check requires a production API,
secret, server, device, R2/D1 mutation, email, TURN, DNS, or deployment.

If a canonical command fails from pre-existing unrelated work, report the exact
failure and run the approved clean-worktree equivalent; never hide, suppress,
reset, or absorb the unrelated change.

## Release prerequisites and residual risks

The implementation route is decision-complete, but release remains blocked until:

- issue 35's separate credential/history remediation is explicitly authorized
  and evidenced without exposing values;
- the Open Design source package receives the approved release identity/digest
  used by T4/T12;
- every selected T0–T14 implementation/gate passes from a reviewed clean
  baseline; later T15 publisher/IAM/provider-backed staging evidence is not a
  first-candidate prerequisite;
- production R2/image publication, Cloudflare/D1/email/TURN, DNS/cutover, server,
  and any hardware proof receive their own later approvals (TURN may remain off);
  and
- no legacy `site/` state/source, deferred Matter/Samsung provider, deleted plugin
  shell, generic network/webhook/API/MCP escape hatch, or performance benchmark
  enters the release candidate.

Current incomplete registry/cache/runtime/site source is assigned work, not an
untracked exception. Prototypes and the current web gate reduce uncertainty but
do not satisfy production acceptance.

## Implementation boundary

Resolving this handoff authorizes **no implementation**. Begin only the one
tranche the user explicitly selects, restate its scope and validation plan, and
wait for approval before editing. Tranche approval never authorizes commit,
publish, deploy, credentials, external mutation, server/hardware access, or the
next tranche.
