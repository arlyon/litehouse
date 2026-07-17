# Registry flow prototype

Planning evidence for Litehouse's R2-only registry route. This is isolated
prototype code, not application implementation.

## Proven behavior

- FlatBuffer root, seven immutable alphabetical shards, signed release records,
  and release payloads.
- SHA-256 content-addressed component blobs and shard objects.
- Ed25519 release verification with digest, length, package, version, and semver
  validation.
- Create-only immutable writes and ETag compare-and-swap for the one mutable
  root.
- Bounded CAS retry that preserves concurrent publications.
- Exact-version lookup, fail-closed yank handling, verified offline cache
  lookup, byte-range resume, and final digest validation.
- Activation rejects permission growth and failed startup, changes only after
  bounded startup succeeds, retains one predecessor, and rolls back offline.
- The same FlatBuffer bytes and verification code produce an identical result
  natively and in a `wasm32-wasip2` component hosted by Wasmtime.
- A live temporary Cloudflare R2 bucket accepted create-only
  `If-None-Match: *`, root `If-Match`, conditional reads, concurrent
  publication, byte ranges, resume, and prefix cleanup. The bucket was deleted
  after the proof.

## Checks

```sh
./check.sh
```

The optional live smoke requires a newly created disposable R2 bucket and a
regular, non-symlink `0600` file at
`${XDG_RUNTIME_DIR}/litehouse-registry-proof/targets.json`:

```sh
./check.sh --r2
```

The smoke uses a random object prefix and attempts to remove that prefix on both
success and failure. It never prints credentials or endpoint details. Bucket
and token deletion remain operator-controlled cleanup.

## Deliberate boundaries

- R2 is the sole backend; there is no S3 fallback.
- CDN/public-domain configuration and browser CORS are deployment concerns and
  were not needed to prove the registry protocol.
- CI signing custody, product key rotation, revocation publication, CLI UX, and
  actual Wasmtime store replacement remain implementation work governed by the
  resolved planning tickets.
- The R2 adapter's status mapping is intentionally small and should become typed
  SDK error handling in production.
