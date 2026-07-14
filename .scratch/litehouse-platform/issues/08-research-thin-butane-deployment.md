# Research thin Butane/CoreOS deployment

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which Fedora CoreOS primitives are useful if “Litehouse OS” is only a thin
configurator around a portable container?

## Answer

Partial research confirms the thin boundary:

- Butane/Ignition can declaratively provision storage, files, users, and systemd
  units at first boot. Ignition is provisioning, not ongoing configuration or a
  secret store.
- Podman Quadlet/systemd can run the portable Litehouse container with explicit
  volumes, devices, network, resource limits, health/readiness, restart, and
  sandboxing.
- rpm-ostree/Zincati provide signed transactional host-OS updates and a previous
  bootable deployment, but do not roll back application data or define
  Litehouse health.
- CoreOS does not supply application-consistent encrypted backup/restore,
  Litehouse schema rollback, secret custody, or non-expert recovery UX.

Primary sources: [Fedora CoreOS provisioning](https://docs.fedoraproject.org/en-US/fedora-coreos/producing-ign/),
[Ignition](https://coreos.github.io/ignition/getting-started/),
[Zincati](https://coreos.github.io/zincati/usage/auto-updates/), and
[Podman Quadlet](https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html).

Disposition: v1 defines a portable container contract and optionally emits a
small validated Butane/Quadlet configuration. It does not bake an OS, coordinate
CoreOS updates, or require a VM lifecycle prototype.
