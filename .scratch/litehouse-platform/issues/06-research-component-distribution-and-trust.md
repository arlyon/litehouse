# Research component distribution and trust

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which current formats and tools can Litehouse reuse for independent contract and
plugin releases, and which semantics remain Litehouse-owned?

## Answer

Partial ecosystem research recommends reusing rather than inventing:

- binary-encoded WIT packages and Component Model binaries;
- OCI 1.1 content-addressed manifests/referrers and the existing Wasm OCI layout;
- `wkg`/`wasm-pkg-tools` where its pre-1.0 conventions fit;
- pinned `wasm-tools component targets` and `semver-check` for structural checks;
- one v1 signature profile, preferably an offline-verifiable Sigstore bundle;
- SLSA/in-toto provenance and one SBOM format attached to exact digests.

Litehouse must still define package/release identity, immutable
`(package, version) -> digest` binding, exact contract digest association,
within-major semantic policy, accepted Component/WASI feature profile, trusted
publisher identities, key succession, offline trust-root updates, cache
freshness, rollback protection, and the distinction between yanked, revoked,
and deleted artifacts. OCI storage and a valid signature do not define those
policies.

Useful primary references: [WIT package format](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#package-format),
[OCI Image Spec 1.1.1](https://github.com/opencontainers/image-spec/releases/tag/v1.1.1),
[`wasm-pkg-tools`](https://github.com/bytecodealliance/wasm-pkg-tools),
[`wasm-tools`](https://github.com/bytecodealliance/wasm-tools),
[Sigstore bundles](https://docs.sigstore.dev/about/bundle/), and
[SLSA 1.2](https://slsa.dev/spec/v1.2/).

Unknowns include registry referrer retention/copy behavior, stable publisher
OIDC identity, and runtime enforcement of withdrawal/revocation for cached
artifacts.
