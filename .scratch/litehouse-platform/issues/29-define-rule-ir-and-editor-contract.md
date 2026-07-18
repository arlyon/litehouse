# Define the rule IR and editor contract

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-15
Blocked by: 12, 18, 28, 51
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 10

## Question

Which researched condition DSL and Litehouse-owned versioned graph model define
typed state/time triggers, UCUM/profile-checked quantity arithmetic, conditions,
ordered feature-negotiated contract actions, selectors, explicit receipt and
ambiguous-outcome handling, delay/debounce, bounded parallelism, timeout/retry,
enabled/version state, migration, validation, dry-run/simulation, and readable
traces while keeping React Flow JSON an editor detail and preserving native-
plugin parity?

## Answer

> **T8 restart semantics supersession (2026-07-17):** D-025 keeps the authored
> language, pure transition model, bounds, and explicit receipt outcomes below,
> but supersedes durable run/checkpoint/outbox recovery. Runs, delays, control
> state, timers, queues, pending effects, receipt continuations, and cursors are
> process-epoch-local. A restart discards unfinished work, rehydrates each rule's
> dependencies before arming it, and schedules only the next future time
> occurrence. It never catches up, resumes, compensates, or replays an ambiguous
> effect. Persisted authority is limited to authored rules, immutable revisions,
> active/predecessor pointers, grants, and enabled state.

### T8.1a completion evidence (2026-07-18)

The unreleased V1 model is replaced in place by a closed typed Rust model,
generated Draft 2020-12 schema, 14-entry trigger/node port catalog, exact
29-operation argument inventory, WIT-faithful event/state mappings, closed
desired paths, restart-clean state declarations, and deterministic coherent
fixtures. Published kinds are exactly the three triggers and eleven post-trigger
families decided below; retired T7 node spellings are invalid.

Effect receipt outputs and receipt-branch inputs now resolve to the same exact
closed `receipt<CapabilityOperationKind>` and optional unconnected outcomes halt.
The compatibility planner emits an absolute On/Off intent and returns
`AwaitingReceipt`; it never reaches success before a typed receipt. Operation
selectors are checked before private lowering, so other valid operations return
a precise planner-subset diagnostic. Desired target/path/value compatibility,
fail-closed retry eligibility, one-deadline retry bounds, trigger-parameterized
admission state, and narrow schema/model bounds are mandatory semantic checks.

Authored semantic identity pins compiler, CEL, quantity, runtime, timezone,
contract/WIT, management-operation, and port-catalog artifacts with single-valued IDs. Concrete
provider generations, facet/grant revisions, and resolved handles remain later
activation-record fields rather than fabricated authored JSON.

T8.1a deliberately does not claim duplicate-key rejection, RFC 8785 digest or the
exact 64-KiB canonical-semantic check, full CEL/Jiff/quantity compilation, full
receipt/timer execution, or persisted activation binding revisions. Those remain
T8.1b (#64), T8.1c, and T8.2 as applicable. Generator parity, bounded-memory
native tests, formatting, management-contract tests, diagnostics, and source-only
T0 checks provide the local completion seam; `litehouse:provider@1.0.0` remains
byte-identical. The final `wasm32-wasip2` and Clippy reruns are environment-blocked
because the synchronized local toolchain lacks those optional components; the
immediately preceding repair passed both checks before the final fixture/catalog
hardening.

### T8.0 implementation locks

- Use Fjall `3.1.7` `SingleWriterTxDatabase` in a fresh Fjall V3 directory. Every
  authored mutation is one cross-keyspace transaction with
  `PersistMode::SyncAll`; the database's exclusive path lock rejects a second
  process. Litehouse also stores and validates its own application format
  version and has no implicit V2-to-V3 migration.
- Use `serde_json_canonicalizer` `0.3.2` for RFC 8785 JCS after duplicate-key,
  depth, string, collection, and raw-size validation.
- Use `cel` `0.14.0` with default features disabled only for bounded CEL parsing.
  Litehouse owns the closed type checker, accepted AST subset, cost accounting,
  evaluator, unknown/error model, and cancellation checks; the crate's general
  evaluator is not the `litehouse-cel@1` authority.
- Use `jiff` `0.2.32` with `std` and `tzdb-bundle-always`; pin the bundled
  `jiff-tzdb` `0.1.8` data (`2026c`) in compiler/profile locks. Native,
  `wasm32-wasip2`, and browser-Wasm dependency probes compile successfully.
- Keep `litehouse:provider@1.0.0` byte-identical. The provider actor performs a
  bounded exact-typed query sweep: ESPHome drains and coalesces source-pushed
  Native API frames once per sweep, while Tasmota/Open-Meteo retain their own
  due-time polling. Seven stateful families enter seven typed host ingress
  paths; stateless Momentary and Notification facets do not.
- Revise the unreleased management V1 catalog in place: Rule JSON is the only
  top-level authored language, raw source is at most 128 KiB, and no manual
  `rules.trigger@1` exists. Draft creation accepts the document's client-created
  128-bit rule ID and never rewrites `rule_id`, `revision`, or semantic name;
  updates use the record's optimistic base revision. Provider-safe config is
  bounded JSON validated against exact component metadata, authority grants are
  closed and explicit, and all run/result/stream cursors are process-epoch-
  qualified.

Publish a small, typed, versioned **Litehouse Rule JSON** language and its JSON
Schema. The JSON graph is the authored language and persistence authority; React
Flow is one projection/editor. CEL is only a pure expression leaf. Do not put
edges, state lookup, timers, service calls, retries, or effect construction into
CEL extensions, and do not build a textual lexer/parser for the graph.

The runtime has three explicit stages:

```text
Rule JSON + contract/profile locks
              |
              v
validate, type-check, resolve, topologically compile
              |
              v
immutable pre-state + one event + explicit clock -> transition plan
              |
              v
separate bounded executor -> receipts -> observed-state reconciliation
```

The transition plan may describe desired post-state, rule-local state changes,
timers, and typed effect intents. It never claims that a physical device already
changed. Only provider observations update observed state.

### Published schema and compatibility

Use JSON Schema Draft 2020-12. Every schema document declares the Draft 2020-12
meta-schema in `$schema` and its immutable exact release URL in `$id`, for example:

```text
https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json
```

`/rule-ir/v1/schema.json` is only a human/discovery alias and is never persisted,
hashed, or used at build/runtime. Every exact schema is shipped byte-identically
with the server, browser Wasm validator, source tree, deterministic fixtures, and
documentation. Rules lock exact URL, semantic version, schema SHA-256, and profile
SHA-256. Offline validation uses the digest-matching shipped copy.

Author the normative Rust tagged enums/records once, generate the schema and
fixtures reproducibly, and review the generated schema as a release artifact.
Use closed `oneOf` node variants with a required `kind` discriminator,
`unevaluatedProperties: false`, explicit string/array/object bounds, stable ID
patterns, and no open arbitrary metadata maps except namespaced annotations.
JSON Schema validates document shape; the Rust/Wasm semantic compiler additionally
validates graph topology, ports, types, CEL, selectors, contracts, features,
units, bounds, and authority. A schema-valid rule is not necessarily activatable.

Major versions may break semantics. Minor releases are additive only: existing
valid documents retain identical meaning. Patch releases clarify diagnostics or
fix implementations without changing accepted semantics. Unknown node kinds,
ports, required fields, contract majors, quantity profiles, or CEL profiles fail
closed. Migrations are pure version-to-version functions that emit a new draft,
preserve the old immutable revision, record source/target digests and warnings,
and require explicit activation. Never silently rewrite an active rule.

### Canonical document

The published schema defines this closed top-level shape:

```json
{
  "schema": {
    "url": "https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json",
    "version": "1.0.0",
    "sha256": "<64-lower-hex>",
    "profile_sha256": "<64-lower-hex>"
  },
  "rule_id": "<32-lower-hex>",
  "revision": 1,
  "semantic": {
    "name": "...",
    "description": "...",
    "trigger": { "...": "one TriggerNode" },
    "nodes": [],
    "edges": [],
    "locks": {},
    "limits": {}
  },
  "editor": { "revision": 1, "nodes": {}, "viewport": {} }
}
```

`semantic` contains exactly one trigger, bounded semantic nodes/edges, exact
contract/quantity/runtime/CEL locks, activation/binding policy, resource limits,
and optional namespaced annotations. A separate mutable `RuleRecord` stores
`enabled`, active/draft revision pointers, runtime health, and editor revision;
these are not smuggled into immutable semantic meaning. Editor metadata contains
only positions, viewport, groups, collapsed state, and presentation hints.

The semantic digest is RFC 8785 JCS over `semantic` plus every `schema` lock. It
excludes `editor`, mutable RuleRecord state, transient diagnostics, resolved
handles, compiled CEL ASTs, and runtime state. Layout-only saves advance editor
revision but neither semantic rule revision nor runtime activation. Another
conforming editor can still preserve layout.

Use stable semantic node/edge/port IDs rather than React Flow array indices.
Canonical arrays sort by ID where order is not semantic. Ordered outgoing edges
carry an explicit non-negative `order`; duplicate order at one ordered port is a
compile error. Labels and canvas positions never determine execution.

V1 bounds each rule to one trigger, 128 semantic nodes, 256 edges, graph depth
32, four concurrent branches, 128-KiB raw JSON, and 64-KiB canonical semantics.
JSON nesting is at most 32, strings 16 KiB, and collections 1,024 elements unless
a narrower field bound applies. There are no cycles, recursion, dynamic node
creation/fan-out, arbitrary code, or user-defined node kinds. More complex
state-machine automation uses a native plugin consuming the same contracts.

### Fixed node vocabulary

The exact v1 tagged node families are:

1. **Trigger** — observed-state, discrete-event, or typed time schedule.
2. **Snapshot** — reads one explicitly bound immutable capability/state snapshot
   into the planner input; it performs no provider I/O during evaluation.
3. **Condition** — checked `litehouse-cel@1` returning Boolean, unknown, or error.
4. **Quantity** — profile-locked decimal conversion, comparison, arithmetic, or
   quantization through the shared Rust/Wasm UCUM engine.
5. **Branch** — routes typed true/false/unknown/error outcomes.
6. **Desired patch** — constructs bounded RFC 6902 hypothetical desired-state
   operations, including `test` preconditions, against one generated capability
   snapshot target; it never writes observed or arbitrary rule state.
7. **Effect intent** — one exact-major capability operation, stable selector,
   typed arguments, deadline, required receipt confidence, and retry policy.
8. **Delay** — schedules continuation at an explicit virtual/runtime instant.
9. **Debounce/throttle** — bounded keyed trigger admission with explicit leading/
   trailing semantics.
10. **Fork/join** — two to four statically declared branches, deterministic
    all-branch join, and explicit conflict policy.
11. **Receipt branch** — routes `accepted-for-delivery`, `protocol-or-target-
    acknowledged`, `completed`, `observed-completed`, rejected/error, cancelled,
    or ambiguous outcome without guessing from later state.
12. **End** — returns a typed plan/run outcome.

Every node object has exactly `id`, `kind`, `version`, and its closed kind-
specific configuration; only effect/snapshot nodes carry `selector`. The schema
publishes a byte-identical `node-port-catalog.json` mapping each `(kind, version)`
to named input/output `TypeRef`s and cardinality. `TypeRef` is a closed union of
`bool`, `string`, `i64`, `timestamp`, `duration`, `optional<T>`, bounded
`list<T,N>`, `quantity<catalog,profile>`, generated
`contract<package,interface,major,type>`, `trigger-context<kind>`,
`receipt<contract,major,operation>`, and fixed planner-control types. There is no
`any`, dynamic object, map, or user-defined type.

Each node's normative field list, defaults, bounds, port signatures, output
outcomes, closed runtime-state schema, initial state, and encoded-state maximum
live in generated `$defs` plus the port catalog; stateless nodes declare null
state. Prose examples are not the machine contract. The schema release includes
minimal/maximal valid fixtures and one invalid fixture per variant/bound.
Implementation cannot call v1 published until those files pass native and Wasm
validators byte-identically. Producing those artifacts is the first approved
implementation tranche, not silently hand-authoring a second schema here.

Edges connect compatible named ports. Constants are schema-validated typed
values. Desired-patch targets are statically generated JSON Pointers into one
wired capability snapshot; allowed operations are only `test`, `add`, `replace`,
and `remove` where that generated field is optional. The compiler rejects an
unknown/dynamic pointer, read-only field projection, type mismatch, more than 128
patch operations, or patch output above 32 KiB. No user-authored persistent rule-
state schema or arbitrary rule-state patch exists in v1; fixed timer/debounce/
throttle/fork nodes own their small closed runtime records.

Selectors use the stable exact-field/cardinality/rebind model. Activation resolves
them deterministically and locks exact capability major, operations, features,
facets, grants, and provider generations. A missing feature, permission, profile,
or required receipt level is a compile/activation error, never weak emulation.

### CEL boundary

Retain the mandatory bounded `litehouse-cel@1` profile from **Research
standardized rule DSL options**. A condition node receives only immutable values
wired into its named input ports plus explicit `evaluation_time`. Generated
contract records define the CEL environment. CEL can inspect fields and perform
the accepted Boolean/string/integer/time comparisons; it cannot discover state,
follow graph edges, construct effects, mutate state, wait, retry, log, read
ambient time, or call I/O.

Quantity strings/floats never enter CEL. Quantity nodes produce checked typed
values or Boolean comparisons. Missing, unavailable, stale, gap, unsupported,
unknown, and evaluator error remain distinct. A condition has four explicit
outputs: `true`, `false`, `unknown`, and `error`. Unconnected unknown/error halts
that run with a visible typed outcome; neither silently becomes false.

Persist CEL source and normalized-source digest for review. Checked AST and
compiled evaluator data are caches keyed by rule/compiler/profile/contract
digests, not persistence or migration authority. Hard host maxima are 32
condition nodes, 4-KiB source and 16-KiB checked AST per node, nesting/stack 32,
10,000 accounted evaluation steps, 1 MiB temporary evaluator memory, and a
10-millisecond cancellable evaluation budget. Extensions declare and charge
worst-case cost before evaluation; unknown cost is rejected. Configuration may
lower but never raise these maxima.

One Rust checker/evaluator runs natively and in browser Wasm. If the selected CEL
library cannot meet conformance, bounds, checker, and Wasm-size gates, reduce to
the small owned expression AST from the standards research rather than adding
graph semantics to CEL or switching to jq/Rego/Starlark.

### Pure transition and post-state semantics

A run is a bounded sequence of run-to-completion **microsteps**. Each microstep
is a fresh pure evaluation with one immutable tagged input: `initial-trigger`,
`timer-fired`, or `effect-receipt`. Receipt/timer branches are therefore not
conditional mutations hidden inside the initial plan. They are later explicit
transitions through the same compiled graph and virtual/runtime clock.

Every `EvaluationInput` contains exact rule/revision/compiler/profile digests,
run/microstep, branch, suspension, and resume-node IDs, the complete normalized
trigger/timer/receipt record, all generation/revision-qualified snapshots needed
by that phase, availability/quality/freshness/gap/times, explicit evaluation
time, fixed closed
control-node state, and resolved capability/feature/grant metadata without live
handles. No phase depends on best-effort event replay to reconstruct its input.

Topological evaluation from the resume node is pure and deterministic until it
ends or reaches an effect/timer suspension. Its `TransitionPlan` contains:

- optional bounded desired-state projection patch for review/simulation only;
- ordered typed effect intents/timers, each with stable branch/suspension ID and
  exact continuation node;
- the complete next closed control-node state, saved frontier, and fork/join
  arrival state;
- assumptions: endpoint generation/revision, availability/freshness, features,
  permissions, expected values, rule-runtime state version, and grant revision;
- plan/result/provenance digests and bounded diagnostics.

Committing next control state produces the interpreter's process-epoch-local
post-state. Applying the desired projection produces only a hypothetical device
post-state.
Neither writes observed state. Each effect receipt or timer firing advances its
exact saved frontier entry and produces the next microstep input; independent
branch suspensions may arrive in
any order, while join logic waits for its declared arrivals. Provider
observations alone reconcile physical reality. A timeout/disconnect after
possible dispatch is `outcome-unknown`, never an
optimistic post-state.

Parallel planner branches read the same microstep pre-state. Their desired
patches/intents merge in stable branch/node order. Disjoint patches merge.
Identical intents deduplicate only with matching provider deduplication identity.
Conflicting patches/tests/effects require explicit policy or planning fails. V1
supports `reject-conflict` and `ordered-last-wins` only for hypothetical desired
patches. Device effects never use last-wins to hide two commands.

### Triggers and time

One rule revision has exactly one trigger so its event context remains statically
typed. V1 triggers are:

- state observation: any observation or value change on one resolved facet,
  preserving generation/revision and initial hydration policy;
- discrete event: one exact-major contract event with stable event ID and at-
  least-once deduplication; or
- time schedule: one-shot instant, fixed interval, or typed weekly civil schedule
  with IANA timezone and explicit nonexistent/duplicate DST policy.

Time is always injected. Fixed intervals use monotonic elapsed time anchored by
activation and persist only their activation wall-clock anchor so restart can
select the next strictly future interval. Civil schedules record timezone
database version and occurrence ID. A nonexistent civil time explicitly chooses
`skip` or `shift-forward`; a duplicated civil time chooses `first`, `second`, or
`both`. One-shots in the past and missed interval/weekly occurrences are skipped
on restart. No cron string, ambient `now()`, sunrise/geolocation, or arbitrary
calendar DSL is part of v1. Additive trigger profiles can add those later without
changing existing semantics.

Debounce/throttle keys are fixed at compile time from rule/trigger and optionally
bound endpoint identity. Debounce is trailing-edge last-input-wins; throttle is
leading-edge with an optional one trailing run. Both retain the complete bounded
winning typed input only for the current process epoch and expose dropped/
coalesced counts in traces.

### Restart-clean execution, ordering, and effects

Draft edits never affect execution. Validation plus binding/permission/feature
review atomically activates one immutable revision. Existing process-local runs
retain their revision; disabling prevents new triggers and cooperatively cancels
undispatched work, while dispatched effects keep explicit ambiguous-outcome
semantics. Activation may drain or cancel old undispatched runs but never
retargets them to the new graph.

V1 serializes runs per rule. One run may execute or wait; at most 16 subsequent
trigger inputs queue under the trigger's explicit `drop-newest`, `drop-oldest`,
or `coalesce` policy. Parallelism exists only inside the active run's statically
bounded fork. Run records, queues, frontier/control state, timers, plans, traces,
receipts, and result lookups live only in bounded memory for one process epoch.
No run journal, checkpoint partition, effect outbox, or receipt continuation is
written to Fjall.

Before every dispatch Litehouse rechecks enabled revision, endpoint generation,
grant revision, assumptions, deadline, and authorization. One process-local
idempotency key derives from process epoch, rule, revision, run, microstep, node,
and intent. It may suppress duplicates only while that process remains alive;
it is neither an effect-recovery key nor a promise across restart.

Actions are sequential in edge order unless inside a bounded fork/join. Every
intent declares typed arguments, bounded deadline, required receipt confidence,
and idempotency policy. Default is no retry. A definitely pre-dispatch failure
may follow the declared bounded retry policy only when the exact operation is
safe and the provider advertises matching process-local deduplication. Any call
that may have reached the provider becomes `outcome-unknown` and is never retried
automatically. Toggle, step, move, press, notification, and other non-idempotent
effects never auto-retry.

Timers live at most 24 hours inside the current process and retain the complete
bounded typed input needed for their next phase, excluding protected values and
guest handles. Restart discards them. One-shot schedules already in the past do
not fire; fixed and weekly schedules calculate only their next strictly future
occurrence. There is no late-run, catch-up, replay, compensation, or workflow
resumption path.

Hard aggregate maxima remain 32 timers and 16 queued triggers per rule, 256
active/waiting runs and 1,024 timers per server, 32 effect intents and 128 patch
operations per microstep, 64 KiB plan/input, and 32 KiB trace per run. Admission
reserves bounded in-memory capacity before accepting a trigger; overload returns
a typed capacity outcome before planning or dispatch. Configuration may lower
but never raise these maxima.

Receipt branches distinguish required confidence reached, typed rejection/error,
cancellation before dispatch, and ambiguity after possible dispatch. Default
error/ambiguity halts and traces. Continuing or issuing an explicit compensating
effect requires an edge and cannot upgrade confidence. V1 has no observation-
wait node, rollback claim, arbitrary compensation language, event sequences,
loops, races, or long-lived sagas.

### Drafts, activation, and migration

Store a stable rule record plus immutable revisions. One mutable draft may be
based on an active revision and carries an optimistic base revision. Saving a
draft does not compile into the runtime. Validate/dry-run returns deterministic
shape/type/CEL/binding/permission/feature/unit/resource diagnostics with node,
port, edge, and source spans. Activation requires the base revision still match,
shows a semantic diff excluding layout, resolves bindings, compiles, and atomically
moves the active pointer. A stale draft returns conflict rather than merging
silently.

`enabled`, `disabled`, `invalid`, `binding-stale`, `degraded`, and `migration-
required` are distinct states. Enabled means new triggers may run; it does not
claim bindings/devices are healthy. The last successfully active revision and
one predecessor remain available for explicit rollback. Rollback is ordinary
revalidation/activation and never rewinds observed state or effect receipts.

Every active revision pins its exact schema, compiler, CEL, quantity catalog,
timezone database, and rule-runtime profile artifacts. A server upgrade discards
all old process-local runs before rehydration. If the new process cannot host an
active evaluator, it marks that rule `migration-required` and admits no triggers.
Migration always creates and activates a new revision; no unfinished run delays
artifact removal.

### Simulation and traces

Simulation invokes the same compiled Rust planner under a virtual clock with a
recorded or schema-valid synthetic event/pre-state snapshot. It may advance
bounded timers instantly and explore selected branch outcomes, but it stops before
provider dispatch. A user may inject typed hypothetical receipts to continue
receipt branches; these remain marked simulated. Native and Wasm planners must
produce byte-identical canonical plans and semantic traces.

Every run trace is bounded and records rule/revision/plan/input/event digests,
evaluation/virtual time, node/edge transitions, endpoint/generation/revision,
availability/freshness/gap, CEL digest/source span/result/cost, quantity
decisions, patch preconditions, effect-intent identity, dispatch attempt, receipt
confidence, retry/cancel/ambiguity, reconciliation, truncation, and safe error/
correlation IDs. Never copy CEL source, effect arguments, notification text, or
state values into routine traces. Values and arguments are redacted by default;
only schema fields explicitly marked safe scalar identifiers may be allowlisted.
Authorized management endpoints retrieve source or protected payload detail
separately. Traces never contain OAuth tokens, secrets, unrestricted config/
state, or provider internals and use the existing tracing pipeline rather than a
second audit store.

### React Flow and native parity

The browser loads the exact published schema and Rust/Wasm compiler. It projects
semantic nodes/ports/edges into React Flow and writes layout only under `editor`.
Connections are offered only when port types are compatible; contextual search
may generate nodes from currently bound capability contracts and features. The
UI can hide defaults and provide direct command/search authoring, but every saved
artifact remains valid standalone Rule JSON. Never persist React component names,
callbacks, renderer IDs, or arbitrary React Flow node data as semantics.

The existing `site/components/flow-editor.tsx` is a visual mock with generic
input/output nodes and no persistence/compiler/runtime contract; preserve its
useful interaction experiments but do not treat its current node/edge objects as
a migration format.

Native automation plugins remain the escape hatch for loops, large state
machines, long-lived workflows, custom protocols, or optimization. They consume
the same exact-major capabilities, selectors, grants, state/event semantics,
errors, receipts, idempotency, and tracing. The JSON-rule runtime receives no
extra service authority and never becomes a way to call the management API or
WASI directly.

### Validation contract

Before implementation commitment, prove:

- JSON Schema generation, exact release hosting, offline byte identity, bounds,
  closed variants, invalid fixtures, and schema/runtime acceptance parity;
- canonical semantic projection/JCS digest stability and layout independence;
- DAG/port/type/cardinality/selector/feature/grant validation and cycle/depth/
  size/fan-out rejection;
- strict CEL accepted/rejected corpus, unknown/error behavior, metering, checker
  diagnostics, native/Wasm parity, and release Wasm size;
- UCUM decimal/profile/affine/overflow/quantization behavior;
- deterministic pre-state-to-plan and hypothetical post-state fixtures;
- conflicting patch/effect merges and stable sequential/parallel ordering;
- state/event/time/DST/debounce/throttle triggers, process-local deduplication,
  restart discard, next-future scheduling, and no missed-run catch-up;
- receipt confidence, cancellation, deadline, definitely-pre-dispatch retry,
  crash-after-dispatch ambiguity, process-local result lookup, and no optimistic
  observed-state mutation;
- draft conflict, activation, disable/drain/cancel, migration, predecessor
  rollback, and stale binding/generation handling;
- simulation never dispatches and real/simulated traces remain distinguishable;
  and
- HTTP/WebRTC/MCP/web/native consumers observe the same operation behavior and
  authorization decisions.
