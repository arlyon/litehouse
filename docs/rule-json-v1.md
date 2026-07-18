# Litehouse Rule JSON V1 server subset

This document describes the first server-side tranche of the Rule JSON decision in
`.scratch/litehouse-platform/issues/29-define-rule-ir-and-editor-contract.md`.
The normative Rust model is `litehouse-rule-json`; the checked release artifacts are:

- `crates/litehouse-rule-json/schema/rule-ir-v1.0.0.schema.json`
- `crates/litehouse-rule-json/schema/node-port-catalog-v1.0.0.json`

The schema uses JSON Schema Draft 2020-12 and the immutable exact ID
`https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json`. The generator binary
reproduces both artifacts from the closed Rust model and port inventory. Native
compilation requires the rule's schema lock to contain that exact URL, version
`1.0.0`, and the SHA-256 of the shipped schema bytes. There is no migration or
fallback for an unknown version.

## Supported semantic path

This tranche deliberately supports one useful acyclic path:

1. one `state_observation` trigger with an exact provider/facet/contract selector;
2. `snapshot` nodes that read a named immutable input field as `bool`, `string`,
   `i64`, or a built-in Litehouse quantity profile;
3. `compare` nodes with a typed constant (`equal`/`not_equal` for every type and
   ordered comparisons for `i64` and quantity);
4. a `branch` with mandatory `true`, `false`, `unknown`, and `error` outputs;
5. zero or more exact `litehouse:on-off/capability@1` absolute `on_off_set`
   effect intents; and
6. typed `end` outcomes: `success`, `noop`, `unknown`, or `error`.

Edges use stable node/edge IDs, named typed ports, and explicit zero-based order.
The compiler rejects duplicate IDs and port orders, dangling nodes, unknown or
backwards ports, incompatible value ports, missing required branches, bad
cardinality, unreachable nodes, cycles, and excessive graph depth. Node and
expression variants are closed tagged unions; all records deny unknown fields.
There is no metadata escape hatch.

Quantity literals must already use the existing quantity core's canonical
profile, decimal magnitude, and canonical unit. Conversion, profile validation,
comparison, affine restrictions, and checked arithmetic invariants therefore
come from `litehouse-contracts`; binary floating point never enters the rule
model. This tranche exposes quantity comparison only, not quantity arithmetic or
quantization nodes.

## Pure planning boundary

`EvaluationInput` is constructed through checked APIs and has private fields. It
contains the exact rule revision and profile digest, explicit evaluation time,
state-observation generation/revision/time, generation/revision-qualified
snapshots with explicit freshness and known/unknown/error values, and resolved
effect targets with grant revision and absolute-set feature support.

Planning only reads that input. A `TransitionPlan` contains a typed end outcome,
ordered On/Off effect intents, every snapshot/target assumption, and the visited
node path. It contains no observed post-state and performs no provider I/O.
`check_assumptions` can reject a plan when a generation, facet revision, grant
revision, or required target feature has changed before a future executor uses
it. Provider observations remain the only authority for observed state.

Unknown, stale, gap, unavailable, unsupported, source error, quantity error, and
type error do not become false. Snapshot unknown/stale values route through the
branch's `unknown` port; evaluator/source errors route through `error`. Every
branch output must be connected at compile time.

## Enforced bounds

The native parser/compiler enforces:

- 128 KiB raw JSON;
- 128 total semantic nodes including the trigger;
- 256 edges;
- graph depth 32;
- JSON nesting depth 32;
- 16 KiB per string; and
- 1,024 items per JSON collection (with narrower field bounds in the schema).

Any JSON floating-point number is rejected before shape deserialization. Rule
limits may lower, but cannot raise, the hard node/edge/depth maxima.

## Deferred semantics

This is not the whole Rule JSON decision. The following remain explicitly
outside this tranche:

- React Flow, browser/Wasm compilation, editor/layout metadata, and UI;
- draft/active persistence, activation, migration, durable runs, and executor or
  provider dispatch;
- selector resolution against live descriptors, grants, or generated field
  catalogs (the planner receives already-resolved immutable inputs);
- CEL, arbitrary code, desired-state patching, quantity arithmetic, timers,
  debounce/throttle, fork/join, retries, receipts, or ambiguous-outcome runtime;
- state/discrete/time trigger variants beyond state observation; and
- semantic JCS serialization/digests and plan/provenance digests. No audited RFC
  8785 implementation is already available in the workspace, so this tranche
  does not substitute sorted JSON or another fake canonicalization.

The schema/profile lock is present now, but hosting aliases, release publication,
profile artifact resolution, and byte-identical browser delivery remain later
release work.
