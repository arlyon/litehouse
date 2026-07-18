# Litehouse Rule JSON V1

Litehouse Rule JSON is the authoritative authored automation format. React Flow
is an editor projection, not a persistence format. The normative closed Rust
model lives in `litehouse-rule-json`; these release artifacts are generated and
checked in byte-for-byte:

- `crates/litehouse-rule-json/schema/rule-ir-v1.0.0.schema.json`
- `crates/litehouse-rule-json/schema/node-port-catalog-v1.0.0.json`
- `crates/litehouse-rule-json/fixtures/*.json`

The schema is JSON Schema Draft 2020-12 with exact ID
`https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json`. Ordinary objects are
closed. The only maps are bounded semantic annotations and bounded editor
node/group maps.

## Published vocabulary

A document has exactly one trigger of kind `state_observation`, `discrete_event`,
or `time_schedule`. The exact eleven post-trigger node families are:

1. `snapshot`
2. `condition`
3. `quantity`
4. `branch`
5. `desired_patch`
6. `effect_intent`
7. `delay`
8. `debounce_throttle`
9. `fork_join`
10. `receipt_branch`
11. `end`

The retired `compare` and `on_off_set` spellings are invalid. A condition emits a
single `predicate_outcome`; only `branch` routes true, false, unknown, and error.

`CapabilityOperation` is a closed tagged union for the exact 29 capability
operations in the authoritative management manifest. `CapabilityOperationKind`,
contract payload type references, quantity catalog/profile references, and
receipt operation parameters are closed enums. There is no generic executable
operation, event, type-name, or payload string.

Each generated operation template includes:

- the exact closed operation kind and selector contract;
- the exact WIT service package/interface/operation and management projection;
- every argument's `TypeRef`, requiredness, and complete canonical input-property
  schema; and
- receipt and fail-closed retry eligibility.

Tests compare all of those fields, not only operation names, against
`management-operations-v1.json`.

## Events and state fields

The discrete event inventory is generated from the current WIT inventory:

- On/Off: `state-observation`
- Boolean State: `state-observation`, `occurrence`
- Level Control: `state-observation`
- Color Control: `state-observation`
- Measurement: `observation`
- Thermostat: `state-observation`
- Window Covering: `state-observation`

Boolean State does not publish a fabricated `changed` event. Its `occurrence`
event has the exact `litehouse:boolean-state/types@1/occurrence` payload and must
be paired with a Boolean State capability selector.

`StateFieldRef` is an explicit mapping of each WIT `state-field` case to its
actual state-record path and exact `TypeRef`. In particular:

- observation revision is `u64`, never `i64`;
- level, color, and measurement retain their WIT value types;
- thermostat current mode, target mode, action, targets, and choices remain
  distinct, with WIT variants represented as exact contract types; and
- covering current/target/movement projections carry an explicit lift/tilt axis
  rather than a lossy invented flattened field.

The catalog records WIT evidence for every mapping, and tests compare the unique
mapping inventory with the authoritative WIT enums. A snapshot field must belong
to its selector contract before compatibility lowering.

## Ports, effects, and receipts

Port output cardinality is `zero_or_one` when an unconnected outcome means the
run halts or uses the documented default. Required data inputs remain exactly
one; control joins accept one or more.

An effect has no optimistic `next` output. It emits an intent and, only after
provider execution, an exact `receipt<CapabilityOperationKind>`. A receipt branch
names the effect node it continues and accepts that exact same parameterized
receipt type. Wiring a receipt from another operation is a compile error.

The T8.1a compatibility planner can produce a literal absolute On/Off set intent,
but returns `AwaitingReceipt`; it does not visit a receipt branch or success end.
Receipt microsteps and provider dispatch belong to T8.1c. Accepted-for-delivery
is the required non-notification baseline without changing the provider ABI.
Stronger provider receipts remain typed and routable.

Valid non-On/Off operations with matching selectors return a precise
`PlannerSubsetUnsupported` operation diagnostic. An operation/selector mismatch
is rejected first.

## Desired projections

Desired patches are hypothetical review/simulation projections only. Each
`test`, `add`, and `replace` embeds a path-tagged value variant, so an unrelated
value type cannot be paired with `/level`, `/color`, or another path. `remove`
uses a separate enum containing only optional writable fields.

A desired patch must reference an existing snapshot node. Every path must be
writable for that snapshot's selector contract. These checks run before a
planner-subset diagnostic. Desired patches never mutate observed state.

## Retry and deadlines

Default retry is none. `definitely_pre_dispatch` has no Boolean escape hatch: it
always requires matching provider-generation deduplication. Toggle, step, move,
stop, momentary invocation, notification, and the other closed non-idempotent
operations are semantically never-retry.

Attempts are limited to two or three. Backoff is bounded and all attempts plus
backoff share the effect's one absolute deadline; configured backoff must fit
strictly inside that deadline. Anything that may have reached a provider is
outcome-unknown and is not retried.

## Time, retained state, and restart

Schedules are one-shot instants, fixed intervals, or weekly civil schedules with
explicit nonexistent-time (`skip`, `shift_forward`) and duplicate-time (`first`,
`second`, `both`) policy. There is no cron or ambient clock.

Debounce/throttle declares the exact rule trigger context. Its retained winning
input type is generated from that property for all three trigger kinds; it is not
hard-coded to state observations.

All run state is process-epoch-local. Restart discards runs, timers, queues,
delays, admission windows, pending effects, receipt continuations, event dedupe,
and cursors. Dependencies rehydrate before arming, and schedules choose only the
next strictly future occurrence.

## Authored locks versus activation bindings

Authored semantic identity pins exact single-valued IDs and SHA-256 digests for:

- compiler;
- CEL profile;
- quantity catalog;
- runtime profile;
- timezone database;
- contract/WIT catalog;
- management operation catalog; and
- node-port catalog.

The schema emits each fixed ID as a single enum; arbitrary bounded strings are
not accepted.

Resolved provider generations, facet revisions, grant revisions, feature sets,
and concrete binding handles belong to the later activation record. They are not
fabricated in authored `RuleDocument` JSON. Activation and execution must still
recheck them before dispatch.

## T8.1a bounds and deliberate deferrals

`compile_rule_json` enforces raw JSON size/depth/string/collection bounds and all
narrow model bounds needed before compatibility lowering, including rule-local
maxima, deadlines, CEL source and input counts, annotation/editor maps, schedules,
delays, fork branches, effects, timers, desired operation counts/bytes, retry,
node/edge counts, and graph depth.

The 64-KiB RFC 8785 canonical-semantic limit is **not claimed as enforced in
T8.1a**. Correct duplicate-key rejection, JCS projection/digest, and the exact
canonical-size check are T8.1b issue #64. The maximal release fixture deliberately
has more than 64 KiB of non-canonical semantic source while remaining below the
128-KiB raw limit, preventing a false enforcement claim.

Also deferred explicitly:

- full bounded CEL checking/evaluation and normalized AST authority: T8.1b;
- full Jiff schedule compilation and timezone validation: T8.1b;
- full quantity semantic compilation outside the existing comparison seam:
  T8.1b;
- executor receipt/timer behavior: T8.1c; and
- persisted activation binding locks/provider generations/grant revisions: T8.2.

## Deterministic release fixture matrix

Checked-in valid fixtures are coherent connected graphs:

- `valid-minimal.json`: minimal trigger-to-end graph;
- `valid-maximal.json`: maximal name/description/annotation/editor-map/fork and
  weekly schedule fields under the raw bound;
- `valid-planner-compat.json`: supported comparison, branch, effect suspension,
  typed receipt continuation, and explicit terminal routes;
- `valid-all-operations.json`: all 29 argument shapes and receipt parameters in a
  bounded receipt-routing DAG;
- `valid-all-node-families.json`: all eleven node families, dynamic condition and
  fork/join ports, retry, desired projection, delay/admission, and receipt wiring;
- `valid-all-desired-paths.json`: every writable path with its exact tagged value
  type and an existing matching snapshot;
- `valid-time-one-shot.json`;
- `valid-time-fixed-interval.json`; and
- `valid-time-weekly.json`.

The checked fixture inventory includes a named invalid document for every trigger,
node, capability operation, schedule/DST policy, receipt route, desired path,
editor variant, artifact lock, and model/runtime bound. Each fixture is generated
and compared byte-for-byte with its checked file. Draft 2020-12 validation covers
schema-valid and schema-invalid release shapes. Semantic-only failures (including
the recursive JSON-depth case) use strict Rust deserialization followed by the
same semantic compiler as activation; this avoids the exponential recursive-union
expansion in the test-only `jsonschema` 0.17 validator without weakening runtime
validation.

## Regeneration

```bash
cargo run -p litehouse-rule-json --bin generate-rule-schema -- --check
```

Use `--write` only after changing the normative Rust model or catalog generator.
