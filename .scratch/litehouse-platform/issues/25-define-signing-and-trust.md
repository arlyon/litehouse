# Define signing and trust

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 06, 24
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 7

## Question

Which first-party publisher identity, signature/provenance profile, trust roots,
key custody, rotation/recovery, offline verification, local-developer override,
yank/revoke/delete semantics, cached-artifact enforcement, and audit behavior
protect central registry releases while deferring public third-party onboarding?

## Answer

Use one small first-party Ed25519 release-signing profile plus existing HTTPS
registry-origin trust. Do not make Sigstore, TUF, a transparency log, public
publisher PKI, or a second audit store v1 dependencies.

### Release identity and signatures

The deterministic immutable `ReleasePayload` decided in **Define registry
publication and installation locks** is the signing authority. CI signs the
domain-separated message:

```text
"litehouse-release-v1\0" || SHA-256(release-payload-bytes)
```

The signature envelope contains only profile version, `ed25519` algorithm, key
ID, and 64-byte signature. Key ID is SHA-256 of the raw 32-byte public key.
Readers first FlatBuffer-verify and bound the payload, recompute SHA-256 over its
exact bytes, then verify domain-separated Ed25519. They never re-encode semantic
fields or accept a signature over registry shard/root bytes.

The signed payload commits to package/version/kind/publisher, every primary and
attachment descriptor, exact `wkg.lock` dependency graph, runtime-profile
requirement, requested permission maximums, provenance/SBOM/license/source
references, and signing-context version. Package/version and payload digest are
immutable. Registry state—review, yank, withdrawal, revocation, tombstone—stays
outside the publisher signature and is trusted through the configured HTTPS
registry origin.

V1 accepts only the pinned `litehouse:first-party` publisher namespace and
public keys shipped in the host trust bundle. A valid signature proves the
release came from an accepted key; it does not grant permissions, make code
safe, override runtime-profile/import checks, or bypass first-party review.

### Key custody, rotation, and recovery

Keep the Ed25519 private release key only in the protected CI release
environment, unavailable to pull requests, ordinary branch jobs, logs,
artifacts, or bucket credentials. Require protected tags/environment approval
and least-privilege short-lived bucket credentials for the publication job.
Keep one encrypted offline recovery copy under separate custody. Never store the
private key in the repository or Litehouse registry.

Signing and bucket-writing are independent controls: a stolen signing key cannot
publish, and stolen bucket credentials cannot create a client-accepted new
release.

Trusted public keys do not update from the registry they authenticate. Normal
rotation ships a Litehouse trust update that accepts old and new keys, switches
publication to the new key, then removes the old key in a later product release
after an overlap. Product and runtime-profile versions remain independent.

Emergency key compromise uses registry-origin revocation to block affected
releases immediately for clients that refresh, rotates bucket/CI credentials,
and ships a Litehouse trust update containing the replacement public key. Old
clients cannot install replacement-signed releases until updated. Do not invent
self-signed key succession from a compromised key.

Sigstore bundles and SLSA/in-toto provenance may be content-addressed release
attachments later. V1 may record CI/source/SBOM provenance, but acceptance rests
only on the pinned Ed25519 profile, payload/blob digests, review state, and
origin state.

### Acceptance and offline verification

Before any new activation, native and Wasm clients require:

1. validated current-or-cached root and shard descriptors;
2. exact release-payload SHA-256;
3. accepted publisher namespace/key ID and Ed25519 signature;
4. immutable package/version binding;
5. every dependency/release/blob digest and length;
6. accepted first-party review state and non-revoked activation policy;
7. compatible runtime profile and component import set; and
8. configured per-instance grants covering—not inferred from—the signed request
   maximums.

All signature/trust/runtime checks are pure and work from verified local bytes.
Offline operation, restart of the recorded active set, and rollback to an
eligible verified predecessor do not require the registry. Offline new
activation succeeds only when all required signed records, trust keys, and blobs
are cached. Cache age and last-known registry state remain visible.

A trust-bundle change never retroactively deletes content-addressed bytes.
Active and one-predecessor pin counts control cache retention. GC may remove only
unreferenced objects; it cannot remove active code because it was yanked,
revoked, deleted upstream, or signed by a retired key.

### Yank, withdrawal, revocation, and deletion

Keep four explicit states:

- **Yanked:** excluded from fresh range resolution and update suggestions.
  Existing `wkg.lock` and active exact resolutions remain reproducible and may
  restart/rollback with a warning. A newly hand-selected exact yanked release is
  rejected unless already locked.
- **Withdrawn:** publisher/registry declares the release unsupported or defective.
  Block new activation and rollback into it; an already-active set may continue
  and restart with a prominent warning.
- **Security-revoked:** block every new activation/update/rollback into the
  release and emit a critical warning for active instances. Never remotely stop
  a running home plugin. The same recorded active set may restart so refresh or
  reboot does not become a central kill switch; the user explicitly updates,
  disables, or replaces it.
- **Deleted/tombstoned:** exceptional legal, credential-exposure, or abuse
  removal. Retain a registry tombstone/reason where possible. Missing remote
  bytes make uncached installs impossible but never erase verified active cache.

State changes rewrite immutable shards and root through normal CAS. They do not
alter signed payload identity. An origin rollback can hide state changes—the
explicitly accepted Cargo-like v1 risk—and the client does not fabricate a
security generation floor.

### Local developer artifacts

Support unsigned local development only through an explicit per-artifact digest
pin in local desired configuration. The user selects one local component path;
Litehouse computes/persists its SHA-256, marks the instance `local-developer`,
and still enforces Component/runtime-profile validation, config schema, exact
WASI/service grants, environment-secret field isolation, resource bounds, and
normal activation isolation.

There is no global `allow-unsigned`, namespace impersonation, remote unsigned
install, automatic update, or ability for a local artifact to satisfy a signed
registry dependency/publication. Changing bytes changes the digest and requires
a new explicit local selection. MCP/AI cannot create or broaden this override;
it may operate an already configured local plugin only within ordinary scopes.
The UI/API always displays the local/unpublished status.

### Observability

Use the existing structured tracing pipeline, not a second audit database. Emit
redacted events for publication identity/digest, signature/trust decision,
registry-state change, install rejection, local override selection, activation,
and rollback under stable signing/trust targets. Include safe package/version,
release/key IDs, decision/reason, activation/correlation ID, and outcome; never
private keys, tokens, secret values, raw configs, or unrestricted payloads.
Retention and tamper resistance belong to the configured tracing sink.

Required validation includes deterministic signing vectors in native/Wasm,
wrong-domain/payload/key/signature rejection, payload/shard substitution,
rotation overlap/removal, compromised-key recovery behavior, stale/offline
state, yank/withdraw/revoke/delete matrices, local digest replacement, cache pin
and GC safety, and proof that signature success never broadens requested grants.
