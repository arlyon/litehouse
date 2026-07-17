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

Later resolved decisions supersede stale architecture-PRD language. The PRD's
FR1/FR5/FR10/FR11/FR14/FR15 and representative acceptance/check text have been
aligned to direct `wasm32-wasip2`, one physical device per instance, local
`ActivationSet` locks, R2 plus one Rust/Wasm query core, durable receipts/tracing,
and operator-managed receipt-guarded restore. Its old delivery phases are
historical only; this issue is the implementation ordering authority.

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

Performance numbers in runtime/queue/cache/container contracts are correctness
bounds, not SLOs. No benchmark or optimization tranche exists without a
reproducible production-path regression.

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

### T3 — Async runtime resilience, config, and activation

**Prerequisites:** T2.

**Scope:** require per-instance stores; implement concurrent cooperative guest
calls, local/global admission, queue-inclusive deadlines, durable dispatch,
soft/hard cancellation, delivery-epoch event gaps, durable runtime generations,
trap isolation, deterministic restart/sparse probes, exact-origin HTTP/DNS/TLS,
environment-only protected values, bounded isolated compilation, compatibility
cache, pure validation, and all-or-nothing global `ActivationSet` transactions.
Delete the legacy guest HTTP helper. Implement issue 38's receipt-guarded host
rollback integration.

**Not scope:** registry publication, real devices/endpoints, operator secret
management, process-per-plugin isolation, or performance optimization.

**Acceptance:** the complete issue-38 matrix: 32/64 per-instance and aggregate
bounds; concurrent/out-of-order completion; cancellation/revocation/dispatch
races; CPU/I/O interruption; queued/in-flight pre-gap fencing; durable generation
and stale completion; exact jitter/probe epochs; malicious authority/DNS/socket/
environment fixtures; corrupt/oversized cache and compiler bounds; two- and
64-instance activation crash points; predecessor restore; stopped-copy receipt
guard; and no startup control effect.

**Rollback:** desired and active state remain distinct; one predecessor is
retained; plugin-set rollback uses the same batch journal; host/data rollback uses
issue 30's N-1/stopped-copy procedure and blocks if ambiguity would be erased.

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
adoption waits for T12's integrated auth/registry/cache gate and T15's separately
approved provider-backed staging/rollback gate; any issue-50 failure switches to
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

**Scope:** canonical versioned Rule JSON Schema and port catalog, strict bounded
CEL condition profile, UCUM quantity operations, selectors, semantic JCS digest,
native/browser-Wasm validator/compiler, and deterministic pre-state-to-plan
simulation.

**Not scope:** effect dispatch, durable runs, React Flow persistence, loops/sagas,
or optimistic physical state.

**Acceptance:** valid/invalid schema, graph/type/cardinality/CEL/UCUM/bound
fixtures; native/Wasm byte-identical plans; layout-independent semantic digest;
unknown/stale input; and proof simulation cannot call a provider or mutate state.

**Rollback:** schema/evaluator versions are immutable/pinned; migration creates a
new draft and never rewrites an active rule.

### T8 — Durable rule executor and canonical management core

**Prerequisites:** T3 and T7.

**Scope:** run records/microsteps, timers, outbox, durable pre-effect dispatch,
receipts/reconciliation, bounded retry, drafts/validate/plan/apply/activate/
rollback, canonical operation handlers, revisions/idempotency/result lookup,
state/event/config/registry/rule/diagnostic projections, and fixed OAuth scope
requirements.

**Not scope:** transport-specific behavior, UI, MCP, external audit DB, or host
escape.

**Acceptance:** crash at every checkpoint/dispatch boundary; timer/restart;
disable/drain; predecessor rule rollback; gap/hydration; ambiguous-effect no-
replay; authorization/revision/idempotency/result-retention; redaction; and one
handler producing semantically identical projected results for fake adapters.
Observed physical state never changes merely because delivery was accepted.

**Rollback:** active rule plus predecessor and evaluator remain pinned; rollback
revalidates and never rewinds external effects.

### T9 — Portable local server and container

**Prerequisites:** T3, T5, T8.

**Scope:** one-volume layout/lock/migrations, central-token and standalone-
external-OIDC modes, local HTTPS plus loopback-only unsafe mode, canonical HTTP
API, health/readiness/shutdown, `data check`/`rollback-check`, signed digest-pinned
amd64/arm64 non-root read-only OCI image, and optional thin Butane/Quadlet output.

**Not scope:** Litehouse OS, VM appliance, auto-update, PKI, backup service,
secret store, arbitrary host path/device, or host mutation.

**Acceptance:** volume/path/symlink/lock/migration crash fixtures; issuer-mode
exclusivity; local TLS/cache isolation; shutdown ambiguity; image metadata/SBOM/
provenance and architecture smoke; 512 MiB/2-vCPU/256-PID envelope; stopped-copy
watermark and denial/reconciled restore; and Butane/Ignition/Quadlet golden/
syntax/sandbox tests in a disposable matching environment when separately
approved.

**Rollback:** issue 30's signed N-1 image/data procedure. No restore force flag
and no claim to undo device effects.

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

### T11 — Signaling-only Cockpit and WebRTC management

**Prerequisites:** T8–T10.

**Scope:** separate Cockpit Worker, per-server Durable Object, one authenticated
reliable data channel, HELLO/version negotiation, strict control JSON/binary
chunks, credits/backpressure, cancellation, streams, reauthorization, resume,
idempotency/result lookup, direct HTTP/WebRTC canonical-handler parity, and
scoped TURN adapter disabled by default.

**Not scope:** UI, Better Auth cookies, D1 writes, registry writes, management
relay, arbitrary data channels, or TURN enablement.

**Acceptance:** codec golden/fuzz limits; grant/nonce/epoch/resource replay;
transcript negotiation; SCTP/byte/queue credits; disconnect/reconnect hydration;
stream generation/gaps; cancellation and ambiguous mutation; HTTP/WebRTC typed
parity; DO replacement/hibernation; and proof routine home payloads never enter
web/Cockpit/D1/log/registry storage.

**Rollback:** schema/message-compatible prior Worker/DO version; TURN remains off
until separately approved abuse/quota/expiry staging evidence.

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
pointer evidence at 320–1440 pixels. The integrated issue-50 gate also proves
Better Auth/D1 journeys, lazy R2/Rust-Wasm rendering, public/private cache
segregation, purge failure bounds, server-function compatibility, production
build stability, and local rollback. Failure switches to React Router 8 before
T15.

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

### T13 — Thin MCP adapter

**Prerequisites:** T8–T9.

**Scope:** official `rmcp` Streamable HTTP over local HTTPS, one generated tool
per canonical operation, safe projections/redaction, bounded sessions/admission,
pull-batch stream resources, shared scopes/revisions/idempotency/errors/receipts.

**Not scope:** generic API caller, host/file/shell/container/network tools,
secret values, ownership/trust migration, central gateway, prompts/chat UI, or
MCP in loopback-unsafe mode.

**Acceptance:** MCP conformance/Inspector fixture; OAuth resource/server binding;
tool/resource/operation manifest parity; schema/projection/redaction; cancellation/
ambiguity/result lookup; session/queue/stream/gap bounds; and malicious host-
escape/self-grant/generic-call denial.

**Rollback:** disable/remove only the thin adapter; canonical HTTP/WebRTC/web
behavior remains unchanged.

### T14 — Connected-release integration and release-candidate proof

**Prerequisites:** every prior release tranche. Matter work is excluded.

**Scope:** assemble the compatible host/runtime profile, four separately
**test-signed** provider artifacts, R2 fixtures, clean D1/Workers, container,
web/Cockpit, rules,
and MCP; execute the complete clean login → pair → review/install → configure →
control → rule simulate/activate → inspect/MCP journey in an isolated environment.
Create the exact artifact/digest/SBOM/provenance/rollback inventory.

**Not scope:** live publication, production/staging cutover without approval,
real credentials/email/TURN, server changes, DNS, or household hardware.

**Acceptance:** all targeted matrices; canonical checks below; generated digest
inventory; plugin-set, rule, Worker, image/data, and route rollback drills;
WCAG 2.2 AA evidence; credential/pattern scan; final diff/status/scope review;
and independent adversarial correctness/risk PASS with no waived blocker.

**Rollback:** demonstrate each bounded rollback independently. External staging
and publisher qualification remain T15.

### T15 — Provider-backed staging, signing, and release-readiness gate

**Prerequisites:** T14 PASS, issue-35 remediation evidence, approved Open Design
release, and separate explicit authorization for each named staging/signing/
cloud/DNS action plus its cleanup. This is a mandatory release gate, not implicit
T14 authority.

**Scope:** qualify T5b's protected plugin/OCI publisher identities and actual
provider IAM without exposing key material; sign the exact candidate and publish
only to isolated staging R2 and OCI registry namespaces under separate roles;
prove object-first CAS/purge and OCI signature/SBOM/provenance binding; deploy
isolated web/Cockpit/DO bindings with clean staging D1 and stable staging auth
origin; run provider-enforced cache, gradual-deploy, recovery, observability,
permission-denial, and route/DNS rollback evidence.

**Not scope:** production registry/image publication, production D1/email/TURN,
production DNS/cutover, household/server access, or real home management payloads.
Staging TURN is also out unless explicitly selected and authorized in the T15
approval; production TURN remains a later launch decision.

**Acceptance:** safe key-ID/digest evidence proves plugin/OCI domain separation,
rotation overlap, revocation, and publisher-versus-deployer denial; the exact
staging OCI digest is bound to its signature, SBOM, provenance, and source.
Actual CI/IAM negative tests deny cross-role R2, cache, OCI publication, D1,
Cockpit, signing, deployment, Logpush-configuration, and retention-sink read/
administration access. Provider delivery may write the sink, but signer,
publisher, deployer, web, and Cockpit identities cannot read, alter, disable, or
delete retained evidence. Staging proves R2 root CAS/range/cache purge failure
bounds; cross-tenant
Workers Cache isolation; clean D1 migrations plus point-in-time restore rehearsal
and plan qualification; passkey/OAuth/pairing on the stable staging origin;
preview/binding/secret/email isolation; DO hibernation/replacement and compatible
gradual versions; Logpush delivery, redaction, retention, lag/failure alerting;
custom staging-domain rollback; and the complete T12 browser/accessibility
journey. TanStack is final only if every objective issue-50 gate passes here;
otherwise switch to React Router 8 and repeat T12–T15. TURN is optional: with it
disabled, prove no provider credential request and explicit relay-unavailable UI/
protocol behavior; if selected, its own separately approved provider abuse/
destination/quota/expiry/telemetry/circuit-breaker/cleanup evidence becomes
mandatory before T15 can pass.

**Rollback/cleanup:** use only the pre-approved staging rollback/cleanup plan:
revoke candidate roles/keys as applicable, restore prior Worker/DO-compatible
versions, rebind/restore isolated D1, roll back staging routes, remove staging
R2/OCI objects, and disable staging Logpush configuration without deleting the
retained evidence before its policy expires. Cleanup is an external mutation
covered only when explicitly listed in the T15 approval. External effects are never claimed undone.

A production launch decision may be considered only after T15 PASS; it is not a
tranche hidden inside this handoff.

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
  -> T12 connected web -> T12b cleanup ---+-> T14 candidate -> T15 staging gate
  -> T13 MCP -----------------------------┘
```

T4 may proceed after T1/Open-Design approval in parallel with T2–T3 but remains
provisional through T12/T15. T6a–T6d are independent after T1/T3 and finish
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
| FR6 queue/deadline/cancel/stale/isolation/auth | T3 |
| FR7 manifest contracts/permissions | T1, T3, T5 |
| FR8 deterministic typed binding | T2, T7 |
| FR9 rules use the same contracts | T7, T8 |
| FR10 immutable contract/release/local lock | T5, T5b, T15 |
| FR11 native/Wasm JS registry queries/R2 cache | T5, T12, T15 |
| FR12 durable authorized inventory | T10, T12, T15 |
| FR13 negotiated/cancellable/resumable/backpressured WebRTC | T11, T15 |
| FR14 scoped/revisioned/idempotent durable mutations | T8, T10, T11, T13, T15 |
| FR15 install/pair/update/rollback/local diagnostics | T5, T9–T12, T15 |
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
| Management operations | one versioned closed manifest | HTTP/WebRTC/web/MCP bindings generated from it; no transport-only operation |
| Rule JSON/port catalog | versioned schema/catalog source | Rust/browser-Wasm validator/compiler vectors and semantic digests match |
| Registry | FlatBuffer schemas, partition/signature versions, release locks | bindings/root/shard/signature vectors generated from pinned tools; old schema never edited in place |
| Activation/config | source schemas/annotations plus signed artifact IDs | deterministic environment requirement names and `ActivationSet` bytes; values never generated/persisted |
| D1 | reviewed expand/contract migrations | generated types may be refreshed; clean bootstrap and migration hashes are tested; no legacy import |
| Web/Workers | source routes/bindings/cache classes and exact package lock | TanStack route/build output is project-generated; workerd/Wrangler dry run must consume the built Worker |
| Container/Butane | Docker/build/generator source and compatibility metadata | OCI manifests/SBOM/provenance and Ignition/Quadlet goldens are reproducible and contain no secret |

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
run through T14. After separate explicit external approval, T15's provider-backed
staging/signing/recovery/observability checks are mandatory before release.
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
- every T0–T15 implementation/gate passes from a reviewed clean baseline,
  including protected publisher/IAM denial and provider-backed staging evidence;
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
