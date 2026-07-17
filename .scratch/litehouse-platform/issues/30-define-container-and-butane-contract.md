# Define the container and thin Butane contract

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-15
Blocked by: 03, 24, 25, 27, 28, 29, 44
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 11 (narrowed)

## Question

Which image platforms, ports, health/readiness signals, capabilities/devices,
one-volume layout, local secret ownership, config schema, resource limits,
graceful shutdown, digest-pinned update/rollback hooks, stopped-directory backup/
restore boundaries, and minimal validated Butane/Quadlet output make Litehouse portable without
owning an OS, orchestrator, host update, or automatic PKI lifecycle?

## Answer

Ship one multi-platform OCI image for `linux/amd64` and `linux/arm64`, always run
by immutable digest. Use a normal bridged container network with one management
TCP port and a fixed UDP ICE range. The host/operator changes image digests and
owns pull, restart, rollback, filesystem backup, encryption, retention, TLS
certificate lifecycle, and host-OS updates. Litehouse owns only application data
compatibility, health/readiness, graceful shutdown, and crash-consistent use of
one persistent directory.

The optional Fedora CoreOS path emits only a small validated Butane document and
system Quadlet for that same image contract. It does not bake a Litehouse OS,
manage Zincati/rpm-ostree, operate Podman, provision application secrets, create
PKI, or coordinate host/application rollback.

### OCI image contract

Publish one manifest list with exactly:

- `linux/amd64` using `x86_64-unknown-linux-musl`; and
- `linux/arm64` using `aarch64-unknown-linux-musl`.

Do not claim armv7, 32-bit, Windows, macOS, or non-Linux support in v1. Build both
architectures from the same Rust 2024 workspace lock, pinned compiler/tooling,
FlatBuffers/WIT generation, and source revision. Release metadata records image
index digest, per-platform manifest/config/layer digests, Litehouse version,
source revision, build provenance, SBOM digest, config/data schema ranges, and
rollback compatibility. Tags are human discovery aliases only.

The runtime image contains the static `/litehouse` binary, bundled trust roots,
pinned timezone database, licence notices, and minimum passwd/group metadata.
It contains no shell, package manager, compiler, source, private key, credential,
default password, or mutable data. Exact OCI process metadata is:

```text
Entrypoint ["/litehouse"]
Cmd        ["run", "--data-dir", "/var/lib/litehouse"]
User       65532:65532
WorkingDir /
StopSignal SIGTERM
Exposed    8443/tcp, 50000-50031/udp
HealthCmd  /litehouse health --socket /run/litehouse/health.sock --check live
```

The root filesystem is read-only.

Replace current prototype hazards: unpinned nightly/base/cargo-chef/flatc
fetches, architecture-specific output, `:latest`, and auto-update labels are not
release behavior. Build dependencies are digest/version/checksum pinned and kept
out of the final image. The pipeline validates both images, generated files,
SBOM/provenance, and native smoke tests before publishing the index digest.

Digest identity alone is not authenticity. A distinct protected first-party OCI
image key signs the domain-separated exact index digest, source revision,
platform manifest digests, provenance digest, and compatibility metadata.
Install/update tooling pins that public key through the product trust bundle and
verifies the detached release manifest before pull/run. Image and plugin signing
contexts/keys are non-convertible; normal rotation ships overlapping image keys
before signer change. An unverified digest is never executed against the data
volume.

### Network and ports

Use bridge networking by default. The application has two fixed container
surfaces:

- TCP `8443` for configured built-in HTTPS, or plaintext application-authenticated
  backend when the host maps it only to loopback for an operator TLS proxy; and
- UDP `50000-50031` as the bounded ICE host-candidate allocation range.

The host may map TCP 8443 to another host port, but canonical HTTPS origin stays
exact. UDP 50000-50031 uses mandatory one-to-one same-port host mappings. Config
also supplies an allowlisted host/LAN address set to advertise as host candidates;
the ICE agent pairs each address only with those mapped ports and validates
server-reflexive candidates through configured STUN. It never advertises the
container bridge address as externally reachable or falls back to arbitrary
UDP. One allocation is held per active negotiation/connection; exhaustion at 32
returns typed capacity failure and waits for release rather than widening range.

`HELLO`, signaling grants, DTLS fingerprints, and token leases remain governed by
the WebRTC/auth decisions. STUN/TURN selection and credentials belong to central-
control-plane configuration, not discovery or ambient network access.

Cloud-connected operation requires outbound HTTPS/DNS and configured connector
destinations. Standalone exposes configured HTTPS. `unsecure-local` stays inside
the container and is never published. Direct TLS publishes selected interfaces.
Proxy mode publishes `127.0.0.1:<host-port>:8443` and places Litehouse on a
dedicated bridge with no unrelated peer containers; a containerized proxy must
be the only explicitly admitted peer or use a shared ephemeral Unix socket.
Static origin/application authentication remains mandatory behind the proxy.
Generated validation proves an unrelated container cannot reach the plaintext
backend.

No mDNS, DNS-SD, SSDP, multicast, broadcast, subnet scanning, raw socket, host
network, privileged mode, or device pass-through is part of the v1 container.
Per-plugin networking remains Wasmtime-mediated exact unicast authority inside
the host process and does not create extra container ports.

### Process sandbox

The reference invocation requires:

- non-root `65532:65532`;
- read-only root filesystem;
- all Linux capabilities dropped and `no-new-privileges`;
- the runtime's default seccomp and SELinux/AppArmor confinement;
- no host PID/IPC/user namespace sharing, Docker/Podman socket, `/dev` device,
  host filesystem bind, or supplementary group;
- writable bind/volume only at `/var/lib/litehouse`;
- tmpfs `/run/litehouse` owned `65532:65532` mode `0700`, and `/tmp` owned
  `65532:65532` mode `1770`, both bounded and noexec/nosuid/nodev where
  supported; and
- one process plus its ordinary async/compiler threads, with a bounded PID limit.

The container host process necessarily owns ordinary outbound sockets and the
published listeners. Guest plugin authority is still enforced by Wasmtime host
interfaces and exact grants; container isolation is not substituted for plugin
capability checks. Hardware plugins and explicit device mappings require a future
container-contract extension and per-device review.

Initial reference limits are 512 MiB memory, two CPUs, 256 PIDs, 64 MiB `/tmp`,
and 16 MiB `/run/litehouse`. They are conservative deployment defaults, not
product performance promises. **Establish representative performance baselines**
may raise the documented minimum/default before release; operators may lower
limits only with visible readiness/overload risk. OOM/kill never authorizes
partial state writes because application records use atomic/journaled protocols.

### One persistent directory

Mount exactly one persistent volume or host directory at `/var/lib/litehouse`.
It is mode `0700`, owned by runtime UID/GID, and self-describing through a
checksummed top-level version/instance manifest. A process holds a nonblocking
exclusive advisory lock on the stable shared inode
`/var/lib/litehouse/.instance.lock` for its entire lifetime; startup verifies
local-filesystem lock semantics and fails on unsupported/unreliable network
filesystems or contention. Its fixed v1 layout is:

```text
/var/lib/litehouse/
  MANIFEST.json          data/config schema versions and instance identity
  config/                desired config and immutable config revisions
  state/                 activation sets, rules, auth, runs, public identities
  registry/              verified root/shard/blob cache and activation pins
  local-artifacts/       explicitly digest-pinned developer artifacts
  backups/               absent/reserved; Litehouse never writes backup copies
```

Transient sockets, PIDs, and scratch files live under `/run/litehouse` or
`/tmp`; only the cross-container lock lives on the volume. Structured logs go to
stdout/stderr and configured external tracing sinks, not an unbounded volume
log. Cache files remain verified and GC-able; config, state, active/predecessor
pins, journals, and local artifact bytes are not cache.

`MANIFEST.json` contains no secret but commits installation ID, format version,
minimum/maximum readable application versions, config/rule/activation/auth schema
versions, migration journal identity, and checksums for critical index records.
Subdirectories have narrow ownership/modes; APIs and diagnostics never serve
arbitrary paths. Symlinks, escaping hard links, special files, unexpected
ownership, traversal, unreliable locking, or contention fail startup.

Litehouse has no local secret store. Long-lived server, connector, OAuth-client,
and direct-TLS secret values enter only through the process environment and are
never copied into the data directory. Podman/Quadlet/systemd, another container
orchestrator, or the operator owns injection, access control, replacement, and
revocation. For Podman, use externally created secrets bound with
`type=env,target=<VARIABLE>`; a plain operator-owned environment file is allowed
but not generated or parsed specially by Litehouse. Values are read at process
startup only, so changing one requires container recreation. Protocol-generated
short-lived nonce/session state remains ordinary bounded auth state, not a
reusable secret-management facility.

### Configuration contract

Publish a closed JSON Schema Draft 2020-12 for the top-level Litehouse desired
configuration, with immutable exact-release URL/digest and offline shipped copy.
It replaces the current Draft-07, permissive, generated prototype schema. The
file has explicit config schema/revision, auth mode, listener/ICE settings,
registry origin, exact plugin instances, rule references, grants, private local
address fields, schema-marked environment-secret fields with deterministic
runtime names, and resource bounds. Unknown fields and unsupported versions fail
closed; migrations produce new atomic revisions and retain the previous valid
revision.

Container CLI flags cover only bootstrap mechanics: fixed data path, log filter/
format, health socket, and an optional config path for development. CLI arguments
never carry secret values. Environment variables may carry explicitly bound
secret values but cannot override persisted grants, selectors, package pins,
auth mode, or trust records. Litehouse does not load `.env` files, enumerate or
forward the ambient environment, persist values, or implement secret CRUD/
rotation. Production uses `/var/lib/litehouse` and desired configuration therein.

Startup acquires the exclusive volume lock, verifies ownership/manifest and
schema range, resumes or rejects incomplete journals safely, reconciles the last
activation pointer, validates auth/TLS mode, binds required listeners, restores
bounded runtime state, and only then becomes ready. Device availability, R2,
central issuer, Cockpit, and internet reachability do not gate readiness; their
status is reported separately.

### Health, readiness, and shutdown

The binary creates process-owned mode-`0600`
`/run/litehouse/health.sock`. Exact exec checks are `litehouse health --socket
/run/litehouse/health.sock --check live` and `--check ready`; they return only a
small versioned status code—no config, state, identity, or secrets. Health mode
marks itself non-dumpable and clears its inherited environment before tracing,
config, or diagnostics initialization; it then reads only this socket.

- **Liveness** succeeds when the main supervisor/event loop responds and shutdown
  has not crossed its terminal phase. It does not depend on devices, registry,
  cloud, DNS, TLS certificates, migrations, or readiness.
- **Readiness** succeeds after the volume lock/manifest/migrations, auth mode,
  activation reconciliation, runtime supervisors, and configured listeners are
  ready to accept authorized work. It becomes false before shutdown, migration,
  unrecoverable state error, or listener/auth failure. Offline devices merely
  degrade instance health.
- **Startup** uses readiness with a longer bounded grace; liveness is not used to
  kill legitimate migration/reconciliation work before that grace expires.

OCI health runs the exact live command every 30 seconds with a 5-second timeout,
120-second start period, and three failures. Litehouse sends `READY=1` only when
the ready check would succeed and `STOPPING=1` before shutdown; Quadlet uses the
supported container `sd_notify` wiring. Podman health-on-failure kills the
container after the threshold, making systemd `Restart=on-failure` effective.
Litehouse never restarts its own container.

SIGTERM/SIGINT immediately mark not-ready, stop new trigger/request/instance
admission, and begin bounded actor shutdown. Undispatched management requests
become definite cancellation under the WebRTC contract. Queued rule triggers,
checkpoints, timers, and prepared effect outboxes remain durably pending under
the rule contract; they are not expired merely for an image restart. Dispatched
effects preserve ambiguous outcomes. Flush journals/activation pointers and
close the health socket before exit.
Reference graceful timeout is 30 seconds; Quadlet uses 45 seconds before SIGKILL.
A second signal requests immediate termination without claiming a clean state.
Startup reconciliation handles any interrupted phase.

### Image update and rollback

Every deployment pins `Image=<registry>@sha256:<index-digest>`. Generated files
contain no floating tag and no Podman auto-update label/config. Litehouse never
pulls an image, edits Quadlet, invokes Podman/systemd, or restarts itself.

The operator update sequence is:

1. select and verify the signed release manifest/index digest with a previously
   trusted verifier/key, then pull through host tooling;
2. stop the old container and wait for lock release;
3. run the verified new image's `litehouse data check` with network disabled,
   read-only root and data mount, dropped capabilities, no secrets in output,
   and the same UID/resource sandbox;
4. if compatibility requires data rollback, create and verify a complete stopped-
   directory copy with OS tools and record its durable mutation-receipt high-
   watermark; abort unless both copy and watermark complete;
5. update the host/Quadlet digest and start the new image;
6. wait for readiness and inspect migration/activation status; and
7. on failure, stop the new image, then either restart the old digest directly
   when image-only compatible or pass the receipt guard below before restoring
   the mandatory matching data copy and restarting the old digest.

V1 data migrations are journaled expand/contract changes. Every release declares
whether the immediately preceding supported image can still read post-migration
data. The generated update plan permits image-only N-1 rollback only when that
flag is true. Otherwise it requires a stopped-volume copy before migration and
rollback restores both old digest and matching data. No software can make an old
binary understand arbitrary newer state, so readiness never falsely advertises
safe image-only rollback.

Plugin updates remain independent affected-store activations. Image-only
rollback does not rewind desired plugins, observations, or rule receipts.

Before any stopped-copy restore, the verified current image runs
`litehouse data rollback-check` with current and copy mounts read-only. It compares
the recorded receipt high-watermark and fails closed if either volume/watermark
cannot be read or any post-copy mutation receipt remains `outcome-unknown`. There
is no V1 force flag. The operator must reconcile those receipts under the current
data/image or use a declared image-only-compatible rollback; unsupported manual
volume replacement is outside Litehouse's safe rollback claim.

After the guard proves no unresolved post-copy ambiguity, restoring the directory
copy explicitly rewinds every later local change, including config/state/definite
receipts, and still cannot undo external device effects. The update UI/plan states
that data-loss boundary before migration. This guard preserves ambiguous evidence
by blocking a restore that would erase it rather than inventing a cross-version
receipt merge/archive format.

### Backup and restore boundary

Litehouse implements no archive/export/import format, encryption, cloud backup,
retention, scheduler, or filesystem snapshot API in v1. The supported complete
backup is a byte-for-byte copy or snapshot of `/var/lib/litehouse` while the
container is stopped and the exclusive lock released. The operator/OS chooses
copy tool, snapshot mechanism, destination, encryption, access control,
retention, and verification.

A restore copies one internally consistent directory into an empty destination
with preserved ownership/modes while Litehouse is stopped. Startup verifies the
manifest, critical indexes, schema/application compatibility, journals, and auth
restore rules before listening. Mixing snapshots, selected-file restore, or live
file copying without an atomic filesystem snapshot is unsupported; Litehouse
does not claim it can detect every arbitrary operator-created mixture. The full
folder excludes injected private keys and device credentials, but configuration,
home state, rules, identity metadata, and auth protocol state still make it
sensitive; encryption remains explicitly an OS concern.

A redacted diagnostics endpoint may expose bounded safe status through the
management API, but it is not a backup and never packages the data directory.

### Thin Butane and Quadlet output

The optional generator accepts typed Fedora CoreOS target, a strictly parsed SSH
public key or existing access policy, verified signed image index digest, and
management exposure/host port. Advertised ICE addresses remain exact persisted
Litehouse desired configuration set later through the local authenticated config
flow; Butane/Quadlet neither injects nor overrides application configuration. The
data path is fixed to `/var/lib/litehouse`; the generator never accepts, formats,
mounts, relabels, or chowns an arbitrary path/block device. An operator may mount
a separate filesystem there beforehand under ordinary OS administration. Inputs
reject control characters, systemd specifiers, relative/symlink paths, and
unparsed directive fragments. It emits through typed Butane/Quadlet serializers:

- one data directory or filesystem mount at `/var/lib/litehouse` with exact
  ownership/mode;
- one system Quadlet under `/etc/containers/systemd/` using the digest-pinned
  image, non-root user, one SELinux-labelled volume, read-only root, tmpfs,
  dropped capabilities, no-new-privileges, a dedicated bridge, TCP mapping, 32
  one-to-one UDP mappings, exact live/ready commands, process-owned tmpfs,
  `sd_notify`, health-on-failure kill, `LimitCORE=0`, resource/PID limits,
  startup ordering, and 45-second stop timeout; and
- no application config, secret, TLS private key, registry credential, automatic
  update unit, backup timer, Caddy, Cockpit, Kubernetes YAML, Zincati policy, or
  OS/package customization.

Proxy TLS stays outside Litehouse. Direct-TLS certificate/key material is
operator-injected at runtime through declared environment bindings and is never
written to the data volume or Butane user data. The generator may omit TCP
publication for cloud-only operation but always preserves
the configured ICE mapping. Host firewall/NAT configuration outside the
published ports remains operator-owned.

Each generator release pins one official Butane image digest/version and an
explicit supported FCOS release/Podman feature matrix; other targets fail before
output. Tests transpile every fixture to Ignition, validate Ignition and Quadlet
syntax, run `systemd-analyze verify`/Podman dry-run in a matching environment,
compare semantic golden output, and reject floating images, secrets, extra mounts,
capabilities, devices, host networking, auto-update, and unsupported Butane/FCOS
versions. The existing `examples/litehouse.bu` rootless kube prototype and
unrelated `infra/coreos` services are evidence only, not migration inputs.

### Validation and observability

Required implementation checks cover reproducible multi-arch builds, per-
platform smoke/health, non-root/read-only/capability/seccomp enforcement, volume
ownership/locking/path attacks, config/manifest schema and migration crash
points, liveness/readiness/startup distinctions, offline operation, TCP/ICE
mappings and exhaustion, proxy/direct TLS, graceful/forced shutdown during
rules/effects/plugin activation, digest-only update/N-1 rollback matrices,
stopped-copy receipt-watermark creation plus unreadable/unknown/post-copy-
ambiguous denial and reconciled restore, rejection of detectable manifest/journal
mixtures, explicit
documentation/tests that arbitrary operator-created mixtures may be undetectable,
and Butane/Ignition/Quadlet validation in a disposable matching environment.

Emit bounded redacted startup, migration, readiness, shutdown, update-
compatibility, and restore-validation events with safe image/data/config/
activation/correlation IDs. Never log environment, secret paths/values, private
keys, raw config, volume contents, ICE credentials, or unrestricted host details.
No additional audit database is introduced.
