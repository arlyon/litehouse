# Define secret lifecycle and credential remediation

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-15
Blocked by: 02, 27, 30, 47
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

What policy governs local secret injection/storage, scope, rotation, revocation,
backup, redaction, diagnostics, and AI exclusion; and what separately approved,
history-aware sequence removes tracked plaintext AWS/mail credential material
and audits quarantined infrastructure/signing artifacts without printing values
or combining remediation with unrelated tooling changes?

## Answer

Do not build a Litehouse secret manager. Long-lived runtime secret values are an
operator/orchestrator concern and enter Litehouse only through environment
variables. Litehouse does not create a secret database, application encryption,
master key, secret CRUD API, vault abstraction, export/import path, rotation
scheduler, escrow, or backup format.

Podman is the reference deployment mechanism, not a dependency in Litehouse.
Create secrets outside the application and bind each into the container with
Podman/Quadlet `Secret=<name>,type=env,target=<VARIABLE>` (equivalent to
`podman --secret <name>,type=env,target=<VARIABLE>`). Docker, Compose, Kubernetes,
systemd, a credential runner, or an operator-owned environment file may supply
the same variable contract. Litehouse never invokes or configures those systems.

### Exact local environment contract

The closed config/plugin schema marks environment-secret fields; it never stores
a value or caller-selected secret reference. Each signed plugin schema declares
an immutable reviewed uppercase field token. For an instance, the host derives:

```text
LITEHOUSE_INSTANCE_<UUID32_UPPER>_<FIELD_TOKEN>
```

`UUID32_UPPER` is the host-owned UUID's 16 bytes encoded as exactly 32 uppercase
hexadecimal digits with no hyphens. The field token must match
`[A-Z][A-Z0-9_]{0,63}`;
the complete name must be at most 127 characters and unique in the accepted schema.
Private non-secret connector addresses remain ordinary local config with API/MCP
redaction; they are not forced through this environment mechanism. Fixed host-
wide variables are exactly `LITEHOUSE_SERVER_IDENTITY_SEED` (base64 of a
32-byte seed), `LITEHOUSE_TLS_CERTIFICATE_PEM`,
`LITEHOUSE_TLS_PRIVATE_KEY_PEM`, and, only for the configured confidential
standalone BFF method, `LITEHOUSE_STANDALONE_OAUTH_CLIENT_SECRET`. Public-PKCE
standalone clients have no client secret. Additional fixed names require a new
schema release. There is no generic prefix scan or dynamic interpolation.

Litehouse snapshots only the exact required names at process startup. Values are
UTF-8 or schema-declared base64, at most 64 KiB each, and are passed in memory
only to the generated typed host config for that exact instance/field. A Wasm
guest receives no ambient environment and no host environment lookup service.
Missing, empty, malformed, oversized, duplicate, or unexpected protected input
keeps the affected instance—or the authenticated listener for a host identity/
TLS input—inactive with a safe field code. Other independent instances may run.

Creating a draft with environment-secret fields returns the deterministic names
and configured/unconfigured status. The operator creates/binds those secrets and
recreates the container before activation. Management APIs, web, MCP, diffs, and
diagnostics may show the required variable name/status because it is not the
secret; they never return the value. A caller cannot choose, move, share, or
rebind a protected value between instance fields.

Environment values do not override desired config, OAuth scopes, service/WASI
grants, selectors, plugin/version pins, auth mode, trust state, or safeguards.
The main server spawns no child process and exposes no shell. The orchestrator's
exact `litehouse health` exec is the sole exception: it is a privileged same-
container probe that enters a minimal branch, marks itself non-dumpable, clears
its inherited environment before tracing/config initialization, and reads only
the health socket. The system service sets `LimitCORE=0` for main and exec
processes. Treat same-UID/root access,
container inspection, and the orchestrator's backing store as privileged: using
environment variables does not claim protection from the host administrator.

### Rotation, revocation, and backup

Litehouse has no rotation state machine. To rotate a connector credential,
identity seed, TLS key, or OAuth client secret, the operator/provider:

1. creates or activates the replacement at the authoritative provider;
2. replaces the Podman/orchestrator secret binding;
3. recreates the Litehouse container so the process receives a new environment;
4. verifies readiness, pairing/identity expectations, and affected instances;
5. revokes the predecessor at the authoritative provider.

A compromised credential is revoked first where possible; availability is
secondary. Changing the server identity seed additionally uses the explicit
trust-reset/unpair flow and refuses an unexplained key change. Litehouse never
silently retries with an old value, keeps a rollback copy, or claims that a
container restart reloads process environment. Rotation restarts the server,
which is the deliberate KISS trade-off.

The stopped-directory backup of `/var/lib/litehouse` contains no injected
long-lived connector, identity, TLS, mail, TURN, registry, or OAuth-client secret
values. It remains sensitive because it contains home configuration/state,
rules, identity metadata, and bounded auth protocol/session records. Backup
access, encryption, retention, and restoring the matching orchestrator secrets
remain operator concerns. A restore starts not-ready until required environment
inputs exist and authentication reconciliation passes.

Short-lived protocol-generated nonces, pairing transaction material, cookies,
and sessions are bounded protocol state, not reusable deploy-time secrets or a
secret-management API. Persist them only when the authentication state machine
requires crash recovery, with process-only permissions and normal expiry/
consumption. They are never addressable by plugins or general config.

### CLI, central control plane, CI, and release credentials

The v1 first-party CLI requests no offline access/refresh token. It keeps a
short-lived RFC 8628 access token only in memory for the current command/session;
automation may inject `LITEHOUSE_ACCESS_TOKEN` through its own environment or
credential runner. The CLI has no plaintext token file, application credential
vault, fallback token, token-print command, or debug serialization. This closes
the authentication ticket's deferred storage decision without adding a keyring
dependency.

Cloudflare owns central deploy-time secret storage. Better Auth, mail, TURN,
signer, and other Worker credentials use Worker secret bindings—not source,
public vars, D1, R2, build output, preview configuration, or application-managed
records. Production and staging use unrelated values. The two-slot Ed25519 JWT
signer contract remains: publish both public keys, switch the secret binding used
to sign, wait beyond all token/skew/cache leases, then remove the predecessor.
The separately encrypted offline recovery copy remains an operator custody
artifact, not a Litehouse/Worker secret store.

CI jobs receive only their exact environment-scoped platform secret and retain
the already-decided separation among web deploy, Cockpit deploy, D1 migration,
secret/key rotation, route/DNS, TURN, and registry publication. Prefer short-
lived provider identity where a provider supports it; otherwise use a narrowly
scoped revocable CI secret. R2 publication credentials never enter web/Cockpit
Workers. Plugin release signing and OCI signing use separate release identities;
runtime Litehouse receives neither.

### Redaction and AI boundary

Secret values are forbidden from source, desired config, plugin manifests,
instance/rule state, URLs, command-line arguments, HTTP/MCP/WebRTC responses,
errors, diffs, plans, progress, traces, metrics, support output, crash reports,
CI logs, generated Butane/Quadlet, image layers, registry objects, D1, and R2.
Unknown output fields default to hidden. Never dump or enumerate the process
environment. Safe errors contain only stable field/instance/error IDs and
missing/invalid/configured status.

AI/MCP clients may inspect the deterministic environment-secret variable name and
status so they can explain why activation is blocked.

They cannot read values, query the environment, inject values through an API,
create a secret, change host bindings, or obtain host/container/orchestrator
access. A protected value reaches only its
exact generated config field; it is not a service/WASI grant and cannot be used
to acquire another permission.

### Existing credential incident and repository history

The already-documented tracked AWS assignments and hard-coded mail fallback are
assumed compromised regardless of age or apparent validity. Do not test them,
print them, copy them into an issue, or rely on history rewriting as revocation.
The root `.mise.toml` remains a never-inspect file for this agent; its unrelated
workspace/tooling hunk must be preserved and never combined with remediation.
Local ignored/untracked environment files are operator-owned and are not read,
deleted, or migrated automatically.

No remediation action is authorized by this planning decision. Execute only
under a separate scoped incident approval, with provider-account owners present:

1. freeze credential-dependent deploy/publication jobs and record safe incident
   IDs/times/owners without values;
2. revoke/rotate the exposed AWS and mail credentials at their providers first,
   inspect access/audit metadata for abuse, and separately rotate any downstream
   material whose exposure cannot be excluded;
3. in an isolated clean clone, remove the source fallback so missing mail config
   fails closed; use a human-reviewed key-name-only transformation to remove the
   credential assignments from `.mise.toml` while preserving its unrelated dirty
   tooling change in the working repository;
4. add a separately approved exact-pinned secret scanner/pre-commit/CI rule and
   run it with redacted output over the worktree, all branches/tags, relevant
   reflogs/stashes, release artifacts, CI logs/caches, and repository hosting
   views; do not install or run new executable tooling without that approval;
5. after revocation and a coordinated push freeze, make a fresh isolated mirror,
   use `git filter-repo --sensitive-data-removal` with a mode-`0600` tmpfs
   replacement rules file, and rewrite every affected branch/tag; rules match
   known assignment/fallback shapes or compromised values without placing values
   in command arguments, shell history, terminal output, reports, or this repo;
6. validate the rewritten mirror with redacted scanning and exact tree/tag/release
   comparisons, then force-push only with repository-owner approval; request host
   cache/PR-view cleanup, invalidate affected artifacts/caches, and require every
   collaborator/automation clone to be discarded and freshly cloned; and
7. repeat provider audit and redacted scans after propagation. Forks and prior
   copies cannot be recalled, so revoked credentials remain permanently dead.

Do not perform the history rewrite in this dirty workspace, do not create a
backup containing live credentials, do not bulk-stage, and do not mix framework,
registry, auth, signing, `.mise.toml` tooling, or deployment changes into the
remediation commit/maintenance window.

### Quarantined infrastructure and signing artifacts

Under separate approval, inventory quarantined infrastructure/signing artifacts
without rendering contents: path/category, tracked/ref/artifact presence, owner/
mode, size, safe digest, public-key ID when available without private material,
intended system, and last-known use. Keep suspicious material offline in an
operator-encrypted access-controlled quarantine outside the repository. Never
upload it to an AI, issue, paste, CI artifact, or general log.

Any private signing key or infrastructure credential found in source/history/
artifacts is presumed compromised and revoked/replaced in its own trust domain.
Registry package signing, OCI image signing, Cloudflare/R2 deployment, mail, TURN,
and server identity keys remain non-interchangeable. Public trust/key rotation
follows the signing decision; deletion waits until provider/repository evidence
and required incident records are safely retained. This audit does not authorize
infrastructure apply/destroy, key use, deployment, or provider mutation.

### Validation contract

Implementation gates cover deterministic environment-name derivation/collision,
no caller-selected binding, startup snapshot, UTF-8/base64/size/missing-input
failure, per-instance isolation, no ambient Wasm environment, no value persistence
or backup, no secret CRUD/store/encryption/vault dependency, private-address
redaction, Podman/Quadlet `type=env` fixtures with dummy values, container
recreation rotation, identity-
seed change fail-closed/reset, disabled core dumps, CLI no-refresh/no-persistence,
Cloudflare secret-versus-var/preview isolation, CI role separation, schema-
directed redaction across every output/trace/error, MCP status-only exposure, and
redacted scanner fixtures that prove representative current/history leaks fail
without ever exposing fixture values in test output.
