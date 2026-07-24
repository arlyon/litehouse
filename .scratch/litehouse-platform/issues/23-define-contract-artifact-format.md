# Define the contract artifact format

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 06, 18, 21
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 5

## Question

Which canonical binary/source WIT, identity/version metadata, normalized digest,
documentation, generated Rust provider/consumer helpers, examples, state/error/
receipt and feature conformance assets, built-in quantity-profile and pinned
UCUM/QUDT source manifests where applicable, Matter-mapping notes, semantic
change declarations, accepted Component/WASI profile, and automated within-major
shape/meaning compatibility checks make a contract release reproducible and
independently publishable?

## Answer

Publish contracts through the same package, dependency, release, lock, signing,
cache, yank, and revocation machinery as executable plugins. Distinguish only
the primary artifact kind:

- plugin: executable Component Model `.wasm`;
- contract: standard type-only binary WIT package `.wasm`; and
- runtime profile: machine-readable host-interface support package.

Do not create a second interface registry or duplicate WIT in FlatBuffers. The
registry's FlatBuffers describe package releases and dependency edges, not
contract types.

### Contract release

A contract release contains two content-addressed payloads:

```text
<package>-<version>.wasm       canonical binary WIT package
<package>-<version>.tar.gz     deterministic developer/source bundle
```

`wasm-tools` parses the package's WIT graph and emits the first file. It is a
Component Model binary containing types/worlds/interfaces, not executable plugin
code or concatenated text. Its SHA-256 is the ABI digest used by generation and
structural compatibility checks.

The immutable signed release record is the package lock authority. It commits to
package identity/version/kind, ABI digest and size, source-bundle digest and
size, dependency requirements, exact resolved dependency lock, license, build
provenance, and publication identity. A plugin lock records both the release and
ABI digests. This also prevents normative documentation or fixtures from
changing beneath an existing version.

A documentation-only patch may reuse the same ABI digest but always has a new
version, source digest, and release digest. `(package, version)` is immutable;
multiple versions may intentionally point to one ABI blob.

The deterministic source bundle contains only necessary review/build assets:

```text
wkg.toml         # optional standard metadata/overrides
wkg.lock         # standard exact WIT package graph
wit/
contract.md
CHANGELOG.md
LICENSES/
examples/
fixtures/
quantity/        # only where applicable
matter.md        # only where useful; explicitly non-normative
```

WIT package identity/version and dependency requirements stay in standard WIT
and wasm-pkg metadata. `wkg.lock` records exact resolved versions, registries,
and SHA-256 digests. The release record/provenance pins Litehouse-specific
payload and build facts. Do not add a competing `contract.toml` or custom lock
format. Transitive dependencies are not vendored into the source archive; the
standard lock graph resolves them through the same package system.

`contract.md` is the concise normative behavioral specification. The changelog
classifies a release as patch, additive minor, or breaking major in ordinary
prose. Do not add a semantic-policy DSL or machine-readable duplicate of
operation meaning.

Fixtures are bounded canonical JSON examples for descriptors, features,
current/target state, unknown/unavailable quality, errors, receipts, and each
operation's meaningful edge cases. They support generated provider conformance
tests and cross-language/UI validation but are not a second wire format.
Examples remain tiny provider/consumer usage examples.

The shared quantity/profile packages own immutable profile manifests. The
measurement contract depends on their exact-major release rather than copying
profiles into every contract. Those releases record the UCUM 2.2 source,
license, normalization vectors, and digest; any QUDT material is a pinned
version/digest/reference and Matter mapping aid, not a second runtime unit
authority. `matter.md` records useful mapping/range/nullability notes but cannot
change Litehouse semantics.

### Dependency and generation flow

Requirements use Cargo-style same-major lower bounds. For example:

```text
on-off = ^1.2.3
```

accepts `>=1.2.3,<2.0.0`. The resolver chooses the highest eligible non-yanked
release and records its exact version/release/ABI digests. First-party contracts
start at `1.0.0`; do not introduce special `0.x` compatibility policy.

Runtime routing uses package namespace/name/major. Exact build locks remain
available for validation and diagnostics. Same-major provider/consumer packages
interoperate when the resolved release satisfies both minimums and named
feature/operation negotiation permits the call.

The existing `litehouse build` remains the one developer command. Its thin
pre-build step integrates standard `wasm-pkg` resolver/lock/materialization
libraries with a Litehouse static-registry adapter:

```text
litehouse build
  -> resolve/fetch missing WIT packages through registry-core
  -> update standard wkg.lock and materialize wit/deps
  -> generate!(...) reads the locked local WIT graph
  -> Cargo builds directly for wasm32-wasip2
  -> wasm-tools validates exact contract/WASI imports and runtime profile
```

Cargo and `Cargo.lock` continue to own Rust dependencies. `wkg.toml`/`wkg.lock`
and wasm-pkg own language-neutral WIT dependencies. Litehouse owns only its
registry adapter, component validation, and publish/install boundary; it does
not implement custom add/resolve/lock/build semantics. There is no registry
access during code generation once dependencies are materialized, no checked-in
generated Rust source, and no separately published helper-crate matrix. Small
handwritten validators/conformance harnesses live in the shared Litehouse SDK;
generated API docs may be attachments but never authorities.

### Compatibility

SemVer policy is deliberately strict:

- patch: docs, fixtures, examples, provenance, or tooling corrections only; the
  binary WIT ABI digest is unchanged;
- minor: additive compatible WIT only—optional features, operations, events, or
  fields with safe absence/default behavior; and
- major: any removal, rename, required addition, narrowing, type change, unit or
  profile meaning change, operation/receipt/error behavior change, or other
  semantic break.

Within one major, existing names, meanings, requirements, units, quality,
receipt confidence, retry/idempotency claims, and error behavior never change.
A meaning change is a major bump even if Component Model shape is unchanged.
Reviewers enforce prose semantics; Litehouse does not invent a semantic diff
language.

Release CI performs only useful deterministic checks:

1. parse/format source WIT and reject undeclared dependencies;
2. build the binary WIT package twice with exact-pinned tools and compare bytes;
3. decode the binary and prove it corresponds to the release source graph;
4. validate it with pinned `wasm-tools`;
5. for patch, require the prior ABI digest unchanged;
6. for minor, run `wasm-tools component semver-check` against the previous
   release and major baseline;
7. validate dependency ranges, exact lock digests, fixture schemas, examples,
   licenses, archive normalization, and release-record payload digests; and
8. run the generated provider/consumer conformance harness.

A linear immutable release history plus previous-release and major-baseline
checks makes compatibility transitive without comparing every pair. Yanked
releases remain in compatibility history; yanking does not rewrite ancestry.

### Runtime/WASI profile

Domain contracts import only shared Litehouse domain packages. They do not
import WASI runtime interfaces. Plugin components combine contract dependencies
with separately granted WASI/host interfaces and generated typed configuration;
consuming On/Off must not implicitly authorize sockets or clocks.

Litehouse publishes a separately versioned package such as
`litehouse:runtime-profile@1.7.0`. Its immutable release pins the accepted
Component Model feature set, exact official WASI interface/package versions,
Litehouse host interfaces, Wasmtime compatibility evidence, and import
validation fixtures. A plugin declares a Cargo-style minimum profile requirement
and its component still self-describes exact imports.

V1 uses official Wasmtime implementations rather than Litehouse copies:

- Wasmtime 46.0.1 Component Model async/concurrency;
- WASI 0.2.12 compatibility imports produced by Rust `wasm32-wasip2`/`std`;
- official WASI 0.3.0 and `wasi:http` 0.3.0 native-async interfaces; and
- Litehouse service-fabric and tracing host interfaces.

The host may link official interfaces while granting no resources. Filesystem
has no preopens by default; sockets/HTTP receive exact grants; protected runtime
values arrive only inside generated typed instance config and guests inherit no
ambient environment; CLI context/stdio is bounded and never a shell.
Publication rejects component imports outside the declared runtime profile before
installation.

Runtime-profile minors are additive and preserve every prior import. Removing or
breaking a host/WASI interface requires a new profile major. Litehouse product
releases advertise the profile releases they implement, but product and runtime
profile versions remain independent.
