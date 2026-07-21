# Ship Litehouse through local MCP and remote UI

Label: wayfinder:map
Status: active
Tracker: local-markdown
Supersedes downstream execution planning in: [Find the route to Litehouse's first connected release](../litehouse-platform/map.md)

## Destination

Produce a decision-complete, evidence-backed route through two independently
releasable milestones:

1. **Phase 1 — local MCP:** one headless Litehouse daemon with CLI/schema setup,
   exactly four bundled providers, automatic restart-clean Rule JSON execution,
   four small unauthenticated MCP tools, bounded process-local run outcomes,
   readiness, and a multi-architecture OCI/Compose release. Network reachability
   is the entire authority boundary. Every authored settings or workflow change
   requires a process restart.
2. **Phase 2 — remote UI:** one central Litehouse identity and pairing path,
   minimal owner/server cloud data, transient signaling, direct authenticated
   WebRTC JSON-RPC, and a remote owner UI with parity to Phase 1 inventory,
   queries, typed interactions, workflows, and recent outcomes. The UI cannot
   edit settings or restart the server.

The map is complete when both phases have implementation-ready contracts,
dependency order, rollback boundaries, and deterministic validation gates.
Implementation and external deployment remain separately controlled.

## Notes

### Working mode

- This map plans the remaining route from committed T8.2 file authority. It does
  not reopen the completed provider ABI, four-provider implementations, MCP HTTP
  transport, Rule JSON compiler/planner, or authored-file foundation without a
  demonstrated defect.
- Prefer the thinnest path that reuses working seams. Delete stale promises rather
  than implementing placeholders for an oversized platform.
- Phase 1 is intentionally unauthenticated: if a client can reach MCP, it may
  inventory, query, interact, and manage workflow source. Exposure must be
  explicit and never described as secure.
- MCP is Phase 1's only product interaction surface. CLI, generated schemas,
  health/readiness, bundled providers, and container metadata are operational
  substrate rather than additional management products.
- All authored changes require restart. There is no live settings reload, live
  workflow reload, remote restart, durable run state, replay, catch-up, or
  compensation.
- Phase 2 cloud storage is limited to owner/authentication data, server identity,
  pair epoch, display metadata, last-seen state, and short-lived signaling state.
  It never stores local settings, workflows, observations, device state,
  provider credentials, management requests, or run traces.
- The broad 75-operation management catalog is historical input, not the Phase
  1/2 destination. Keep the 29 typed capability operations; select only the
  canonical operations consumed by the four MCP domains and remote UI.
- Commit bounded planning and implementation tranches regularly. Preserve
  unrelated work and do not perform external writes, publication, deployment,
  account mutation, or hardware interaction without separate authority.

### Fixed Phase 1 product boundary

- CLI: `init`, `validate`, and `run`.
- Deterministic settings and Rule JSON schemas.
- Exactly ESPHome, Tasmota HTTP, ntfy, and Open-Meteo bundled with the release.
- Four MCP tools: `litehouse_inventory`, `litehouse_query`,
  `litehouse_interact`, and `litehouse_workflows`.
- `litehouse_interact` is a closed tagged union over all 29 typed capability
  operations; there is no arbitrary operation ID or JSON dispatch.
- `litehouse_workflows` owns bounded list/get/validate/simulate/write/delete/
  enable/disable and process-local outcome inspection. Writes report restart
  required and cannot activate or reload the executor.
- Live Rule JSON V1 execution uses startup-only immutable settings/workflow
  snapshots. Restart discards unfinished work and all retained outcomes.
- Retain the current active run plus at most 16 terminal outcomes per workflow
  and 256 terminal outcomes server-wide, newest first, with no pagination or
  cancellation API.
- Minimal unauthenticated liveness/readiness, graceful shutdown, one writable
  `/config` mount, non-root `linux/amd64` and `linux/arm64` images, and one Docker
  Compose example.
- No UI, account, pairing, WebRTC, registry/install/update, live reload, remote
  settings, Kubernetes, Butane/CoreOS, or appliance work.

### Fixed Phase 2 product boundary

- One central Litehouse identity path; no standalone OIDC mode.
- One owner, server pairing, pair epochs, bounded short-lived signaling, and
  direct WebRTC management.
- One authenticated ordered JSON-RPC channel projecting the exact Phase 1
  canonical domains; no generic HTTP tunnel, batches, resume, or mutation replay.
- Remote UI journeys: sign in, list/pair servers, connect, inspect/query/control
  typed facets, manage workflow source, and inspect recent outcomes.
- No settings editor, provider configuration, remote restart, registry/install,
  durable diagnostics, or cloud copy of local operational data.

## Decisions so far

<!-- Resolved ticket links are appended here; destination decisions live above. -->

## Not yet specified

- Exact release publication/version/tag channel after local image validation.
- Whether Phase 2's demonstrated connectivity floor requires TURN or may ship
  with an explicit direct-connect failure boundary.
- Exact central authentication recovery methods beyond the minimum owner login.
- Which legacy packages are deleted versus retained as unreachable historical
  evidence after each phase.

## Out of scope

- **Phase 3:** the broader canonical management catalog, provider
  registry/install/update/rollback, remote settings and restart, standalone
  identity, sharing, durable diagnostics/history, Kubernetes/CoreOS, and other
  platform operations that are useful eventually but do not block Phases 1–2.
- Matter/shared-dispatcher expansion and external staging/signing/cloud/DNS
  operations remain separate efforts with their own approval boundaries.
