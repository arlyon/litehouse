# Research unit mathematics standards and libraries

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which maintained standard and Rust/TypeScript implementation can give
Litehouse deterministic dynamic unit mathematics over canonical decimal values,
including parsing and canonicalizing expressions such as `J/s`, dimensional
compatibility, conversion, multiplication/division/powers, affine temperatures,
percentages and dimensionless values, bounded resource use, stable
serialization/versioning, and useful diagnostics—without inventing a unit
ontology or making UI display strings authoritative?

## Answer

Research against UCUM 2.2, QUDT 3.4.0, and current Rust/JavaScript unit
libraries found no maintained library that satisfies Litehouse's full dynamic,
exact-decimal, bounded, cross-runtime contract.

Use a constrained case-sensitive UCUM 2.2 expression syntax at ingestion, with
Litehouse's versioned built-in quantity profiles and canonical decimal rules as
the authority. Use QUDT 3.4.0 only while designing/generating built-in quantity
profiles; do not put its RDF ontology or URIs on the runtime wire.

A canonical quantity is conceptually:

```json
{
  "profile": "litehouse:quantity/power@1",
  "magnitude": "1",
  "unit": "W"
}
```

- Magnitudes are canonical decimal strings backed by a bounded signed
  coefficient and scale, never JSON numbers or binary floats.
- Profiles are immutable exact-major IDs from a versioned built-in catalog.
  Providers cannot invent profiles in v1.
- The wire unit is the profile's canonical UCUM code and must match its catalog
  entry. UI labels are presentation metadata, never identifiers.
- Input may use bounded compound UCUM expressions. `J/s` and `kg.m2/s3`
  canonicalize to the power profile in `W`; `%` canonicalizes to a ratio in
  unit `1`.
- Addition/subtraction require the same semantic profile, not merely equal
  dimensions. Multiplication, division, and bounded integer powers may produce
  a value only when the result maps to a built-in profile.
- Absolute temperature and temperature difference are separate profiles.
  Absolute values canonicalize to `K`; affine conversions apply offsets only to
  absolute temperatures. Absolute-minus-absolute yields a difference.
- Reject annotations, arbitrary/custom/logarithmic units, unbounded expressions,
  unknown result profiles, NaN/infinity, implicit rounding, and exponent or
  decimal overflow. Recurring results require explicit scale and round-half-even
  quantization with audit metadata.

The parser must impose independent limits, including bounded input length,
token count, nesting, exponent magnitude, decimal precision, and diagnostic
payloads. Errors use stable codes and bounded source spans; fuzzy catalog-wide
suggestions are not part of the authoritative path.

Library findings:

- Rust `ucum-units` 0.1.0 is the closest prototype/oracle, but is new, uses
  `f64` in quantity APIs, saturates some dimension arithmetic, and does not
  provide Litehouse canonicalization.
- Rust `octofhir-ucum` 0.5.1 reports incomplete UCUM conformance and still
  exposes `f64` paths. Other maintained dynamic Rust crates use non-UCUM
  grammars or binary floats.
- NLM `@lhncbc/ucum-lhc` 7.1.8 is a useful JavaScript differential oracle but
  uses JavaScript numbers and is not a general exact quantity engine.
- `mathjs` 15.2.0 is mature but has a broader, non-UCUM grammar and attack
  surface. Other JavaScript libraries are non-UCUM or insufficiently current.

Therefore implement one bounded Rust unit core and compile that same core to
Wasm for TypeScript/UI use; do not maintain independent arithmetic engines.
Native and Wasm run identical conformance vectors and require byte-identical
canonical outputs and diagnostic codes. Differential tests may use the two
oracles above, but neither controls wire semantics.

Pin the verbatim UCUM 2.2 essence and checksum plus QUDT 3.4.0 reference data in
profile-generation tooling. Record source and generator versions in each
catalog release. UCUM's custom license permits interoperating applications and
verbatim redistribution but requires preserved notices and legal review before
shipping vendored data; QUDT data is CC BY 4.0 and requires attribution.

Primary sources:

- [UCUM 2.2 specification](https://ucum.org/ucum) and
  [release](https://github.com/ucum-org/ucum/releases/tag/v2.2)
- [UCUM license](https://ucum.org/license)
- [QUDT 3.4.0](https://github.com/qudt/qudt-public-repo/releases/tag/v3.4.0)
- [`ucum-units` 0.1.0](https://crates.io/crates/ucum-units/0.1.0)
- [`octofhir-ucum` 0.5.1](https://crates.io/crates/octofhir-ucum/0.5.1)
- [`@lhncbc/ucum-lhc` 7.1.8](https://www.npmjs.com/package/@lhncbc/ucum-lhc/v/7.1.8)
- [`mathjs` 15.2.0](https://www.npmjs.com/package/mathjs/v/15.2.0)
