# Define the container and thin Butane contract

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-17
Blocked by: 03, 24, 25, 27, 28, 29, 44
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 11 (narrowed)

## Question

What is the smallest portable container and optional Fedora CoreOS configuration
needed to run the first connected release?

## Answer

Ship a conventional non-root OCI image for `linux/amd64` and `linux/arm64`, one
persistent data directory, health/readiness checks, and graceful shutdown. Offer
a small optional Butane/Quadlet generator for Fedora CoreOS.

This decision supersedes the earlier cross-version restore protocol, receipt
watermarks, prescribed CPU/memory/PID envelope, and extensive OCI evidence in
this issue. There is no prior major release or incompatible migration to restore
from, so V1 does not design that future system now.

## Container image

The image:

- runs the Litehouse binary as a non-root user;
- supports `linux/amd64` and `linux/arm64`;
- has a read-only root filesystem under the reference invocation;
- writes application state only beneath `/var/lib/litehouse`;
- exposes the configured management TCP port and only the WebRTC UDP ports the
  implementation actually uses;
- handles `SIGTERM`/`SIGINT` with graceful shutdown; and
- contains no credentials, private keys, default password, compiler, source tree,
  package manager, or automatic update mechanism.

Tags may aid discovery, but deployment records the exact built image digest.
Litehouse does not pull, restart, update, or roll back its own container.

## OCI validation

T9 requires ordinary release engineering evidence only:

- build both target architectures from the pinned workspace;
- inspect the resulting entrypoint, user, architecture, and declared ports;
- start each image in a disposable local environment;
- verify liveness/readiness and graceful shutdown; and
- record the image digest used by the release candidate.

Protected signing, publication provenance, provider IAM, and staging registry
qualification belong to later, separately approved T15 work. T9 does not create a
custom OCI evidence system.

## Persistent data

Mount one persistent volume or directory at `/var/lib/litehouse`. Runtime sockets
and scratch files remain outside that directory. Litehouse holds an exclusive
instance lock so two processes cannot use the same data directory concurrently.

V1 may initialize its first data schema and reject unreadable or unsupported
data. It does not implement cross-major backup/restore, N-1 migration rollback,
receipt-watermark guards, backup scheduling, snapshots, encryption, retention,
or cloud backup. Operators may stop Litehouse and copy the directory with normal
OS tools; that is operational practice, not a Litehouse restore protocol.

A future release that introduces an incompatible migration must design and test
its upgrade/rollback behavior against the actual old and new formats.

## Configuration and secrets

Use the existing reviewed Litehouse configuration path and one explicit data-dir
argument. Long-lived secret values remain operator-injected and are not written
to the data directory or generated Butane.

Do not add a Litehouse secret manager, arbitrary host mounts, device mappings,
host networking, container-engine access, or OS management.

## Health and shutdown

Provide small liveness and readiness checks suitable for the container runtime:

- liveness means the Litehouse process/event loop responds;
- readiness means configuration, data directory, listeners, and runtime startup
  are ready to accept authorized work;
- unavailable devices or cloud services degrade their own status rather than
  killing process readiness; and
- shutdown marks not-ready, stops accepting new work, finishes or cancels work
  using the ordinary runtime semantics, flushes owned state, and exits.

Exact timeouts are implementation defaults informed by tests, not immutable
product policy.

## Resource policy

T9 declares no mandatory CPU, memory, PID, or tmpfs envelope. Tests may measure
resource use and documentation may report observed requirements. Operators and
container runtimes remain free to set deployment limits.

## Thin Butane and Quadlet generator

The optional generator emits only what is needed to run the same image contract
on Fedora CoreOS:

- a digest-pinned Litehouse image;
- one `/var/lib/litehouse` data directory/volume;
- non-root and read-only-root container settings;
- required TCP/UDP mappings;
- restart, health, and startup ordering; and
- operator-selected SSH access needed to administer the host, when generating a
  complete Butane document.

It emits no application secrets, TLS private keys, automatic update unit,
backup timer, Caddy/Cockpit service, Kubernetes YAML, arbitrary mount/device,
host-network setting, or OS/package customization.

Keep the generator small. Validate its inputs, transpile generated Butane to
Ignition with the pinned supported tool, and syntax-check the generated Quadlet.
A few representative golden fixtures are sufficient; T9 does not build a custom
FCOS qualification laboratory.

## Acceptance

T9 passes when:

1. amd64 and arm64 images build;
2. image metadata and digest are recorded;
3. the image runs non-root with one writable data directory;
4. liveness, readiness, and graceful shutdown work;
5. a second process cannot use the same data directory concurrently;
6. no secret is embedded in the image or generated configuration; and
7. representative Butane output transpiles and its Quadlet syntax validates.

Cross-version restore, fixed resource envelopes, protected image signing, staging
publication, and provider-backed evidence are explicitly deferred.
