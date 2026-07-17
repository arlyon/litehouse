# Research standardized rule DSL options

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-14
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

How do CEL, JSON Logic, DMN/FEEL, CNCF Serverless Workflow, and other maintained
standards compare for a small typed Litehouse rule graph edited with React Flow,
including Rust/TypeScript implementations, static typing, determinism,
sandboxing, time/state inputs, serialization stability, versioning, simulation,
traces, and interoperability?

## Answer

Use a mandatory, bounded **`litehouse-cel@1` profile of CEL 0.25.2** for Boolean
condition expressions inside a Litehouse-owned typed rule IR. React Flow remains
an editor detail. CEL does not become the workflow engine.

Keep triggers, exact-major capability selection, UCUM quantity operations,
delay/debounce/retry/timeout/parallelism, action ordering, receipt/ambiguous-
outcome handling, state freshness/availability/generation, simulation, traces,
migrations, and activation state in the graph model.

CEL is the best semantic base because it is side-effect-free and terminating for
a fixed environment, supports static checked expressions and source spans, and
has maintained Rust and TypeScript ecosystems. JSON Logic lacks normative static
typing/versioning and relies on coercion; DMN/FEEL is powerful but enterprise-
heavy; Serverless Workflow would import a second orchestration runtime; Rego,
Starlark, jq/JSONata/JMESPath are larger or more dynamic than one condition node
requires.

The v1 profile includes Boolean/string/signed-integer/timestamp/duration values,
field selection over generated immutable input objects, Boolean operators,
equality, ordered primitive/temporal comparisons, and a bounded same-type
conditional. Static checking is mandatory and the final type is always Boolean.

Exclude `dyn`, Protobuf dynamic values, float/uint and implicit numeric coercion,
list/map/message literals, comprehensions/macros, regex, arbitrary extensions,
ambient clock/randomness, I/O, mutation, logging, device selection, service
calls, retry, and graph control. `has()` is allowed only where generated presence
semantics are unambiguous.

The graph compiler owns domain typing:

- environments expose only fields from exact locked contract majors;
- unsupported versus supported-but-unknown fields remain distinct;
- availability, freshness, quality, generation, revision, gap, source time, and
  receive time remain explicit inputs;
- every evaluation receives one immutable snapshot and explicit
  `evaluation-time`; and
- UCUM canonical decimal conversion/arithmetic/comparison remains in typed graph
  nodes using the Rust/Wasm quantity core. CEL receives their typed Boolean
  results, never quantity strings or binary floats.

Persist canonical versioned Litehouse rule JSON with CEL profile/source and
input wiring, exact contract and quantity-catalog locks, compiler/profile
version, normalized source digest, enabled revision, and migration history. A
checked CEL AST may be a cache keyed by compiler/runtime digest but is never the
persistence or migration authority.

Litehouse owns bounded simulation/trace output: rule/run IDs and digests,
evaluation time, input endpoint/generation/revision/availability/freshness/gap,
node status/results, source spans/expression IDs, quantity decisions, planned
action intent, dispatch/receipt/retry/ambiguous outcomes, and truncation markers.
Simulation stops before provider dispatch. Missing/stale/unavailable/error states
never silently coerce to false, zero, or null.

Use one authoritative Rust checker/evaluator compiled to Wasm for browser
validation/simulation. Rust `cel` 0.14.0 is a parser/evaluator candidate but
lacks the complete checker, standard checked-AST, and metering Litehouse needs;
`@bufbuild/cel` 0.6.0 is a checker/conformance oracle, not a second production
evaluator. If implementing the bounded checker over Rust CEL approaches most of
full CEL, reduce the accepted grammar further rather than adopting JSON Logic or
shipping divergent Rust/TypeScript engines.

Implementation validation must prove accepted-expression differential
conformance, stable rejection diagnostics, exact-major field rejection,
quantity/affine/overflow safety, native/Wasm parity, parser/evaluator memory/
stack/fuel/deadline bounds, explicit-time replay, unknown-state behavior,
canonical serialization/migration, and Wasm size/license feasibility.

Primary sources:

- [CEL 0.25.2](https://github.com/cel-expr/cel-spec/releases/tag/v0.25.2)
- [Rust `cel` 0.14.0](https://crates.io/crates/cel/0.14.0)
- [`@bufbuild/cel` 0.6.0](https://github.com/bufbuild/cel-es/releases/tag/v0.6.0)
- [JSON Logic](https://jsonlogic.com/)
- [OMG DMN 1.5](https://www.omg.org/spec/DMN/1.5/)
- [CNCF Serverless Workflow 1.0](https://github.com/serverlessworkflow/specification/releases/tag/v1.0.0)
- [OPA/Rego](https://www.openpolicyagent.org/docs/latest/policy-language/)
- [Starlark specification](https://github.com/bazelbuild/starlark/blob/master/spec.md)
