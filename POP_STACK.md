# POP Stack

> Compact execution map for Litehouse's first connected release. Updated 2026-07-18.

## Current position

The Wayfinder effort has reached a decision-complete implementation handoff. Its destination remains a first connected release with typed provider facets, explicit authority, a portable local runtime, a greenfield web surface, rules, and a small MCP adapter. The map has no unresolved first-release architecture fog; implementation is proceeding through bounded commits on `agent/connected-release`.

Current committed foundation:

| Layer | Commit | Result |
|---|---|---|
| T0 execution safety | `697b766` | Reproducible FlatBuffers generation, non-publishing CI baseline, pinned validation |
| Implementation handoff | `293803d` | Initial connected-release execution handoff |
| T1 contracts | `cecdfc3` | Nine exact-major capability contracts, quantity/runtime profile, canonical management manifest |
| T2 service fabric | `44faf5a` | Typed endpoint registry, provider-local resources, consumer proxies, binding and authorization |
| Planning corrections | `5fea7bb` | Simplified T9, T11, T13, T14, and deferred T15 boundaries |
| T3 runtime resilience | `e3c7240` | Focused cache, deadline, trap, event-continuity, capability, and protected-value hardening |
| T4 greenfield web gate | `7fcd1fc` | TanStack/Worker capability renderer gate with digest-locked Open Design input |
| Typed ESPHome resource spike | `b6dc3e8` | One guest/store returned repeated typed On/Off, Level, and Color resources safely |
| Combined route handoff | `1f66492` | Frozen T6a plus three-tool MCP execution route |
| Provider and MCP contracts | `7a5c8da` | Production provider ABI, provider admission, and three closed operation projections |
| Production provider host | `4254b15` | Store-local typed-resource actor with exact TCP authority and generation fencing |
| ESPHome component | `e24f212` | Encrypted single-session ESPHome switch and binary-sensor provider |
| Canonical MCP slice | `62fdbc8` | Three `rmcp` tools over authorized typed handlers with deterministic end-to-end proof |
| Supervised ESPHome proof | `c81ee1c` | Relay 1 absolute set, observe, restore, and observe through the three MCP tools |
| Provider HTTP foundation | `428e4d5` | Typed Measurement/Notification resources and exact-origin WASI HTTP authority |
| Tasmota HTTP | `31f402a` | Bounded private-LAN absolute relay and measurement provider |
| Complete provider ABI | `263bea6` | All nine typed provider resource families and host projections |
| Open-Meteo | `e8331a4` | Four attributed current-condition measurements with bounded polling |
| MCP Streamable HTTP | `20f4685` | Explicit-bind, bounded, intentionally unauthenticated `/mcp` transport |
| Rule JSON planner | `f8c0bb0` | Closed schema, typed DAG compiler, immutable inputs, and pure effect planning |
| ntfy | `0030efe` | Fixed-origin/topic notification sink with acceptance-only receipts |
| T8.0 contract freeze | `267917e` | Restart-clean management V1, Fjall 3.1.7, JCS/CEL/time locks, and unchanged provider ABI |
| T8.1a Rule JSON V1 contract | this tranche | Complete closed model, generated schema/fixtures/catalog, and private T7 planner lowering |

The production worktree should be clean before each phase. Build output belongs under `/home/arlyon/.cache/litehouse-connected-release-target` and project tools run through `mise --no-config`.

## User-selected route changes

These choices supersede the older tranche order where they conflict:

1. **Keep the existing registry for now.** T5 replacement and T5b publisher custody are deferred. The current registry remains available as its existing, partially working implementation; this vertical slice uses a local compiled component and does not depend on registry replacement or publication.
2. **Combine the ESPHome and first MCP proving slices.** The first production host adapter is exercised through exactly three MCP tools rather than waiting for the complete T8/T9/T10 stack.
3. **Keep final MCP transport separate.** The proving slice uses `rmcp` in-process transport and a fixed local proof principal. T13 later mounts the same adapter through authenticated stateful Streamable HTTP after local HTTPS and identity exist.
4. **Use absolute device control.** The proof reads current Relay 1 state, sends one absolute opposite state, observes it, restores the original absolute state, and verifies restoration. It never sends ESPHome Toggle and never touches Smart Bulb Mode or the other relays.

## Current architecture stack

```text
Three approved MCP tools
  litehouse_inventory
  litehouse_query
  litehouse_interact (On/Off set only)
          │
          ▼
Canonical management handlers
  inventory.snapshot@1
  facets.query@1
  capability.on-off.set@1
          │
          ▼
Production provider host adapter
  actor + Wasmtime store + generation/incarnation fences
  typed handles retained in provider store
  handle-free EndpointDescriptors published to service fabric
          │
          ▼
Four first-party wasm32-wasip2 providers
  ESPHome Native API (one encrypted session/cache)
  Tasmota exact private-LAN HTTP
  ntfy exact origin/topic HTTP
  Open-Meteo fixed non-commercial endpoint
          │
          ▼
Nine typed provider families
  On/Off, Boolean, Level, Color, Measurement,
  Thermostat, Window Covering, Momentary Action,
  Notification Sink
```

The MCP adapter invokes canonical handlers; it never calls Wasmtime directly. The host adapter invokes generated methods on typed `Resource<T>` handles. Those opaque handles never enter another store or the service-fabric registry.

## Completed combined T6a + MCP vertical slice

### Freeze provider and MCP contracts

- Promote the proven resource pattern into production WIT.
- Add provider control plus dynamic On/Off and Boolean endpoint resources.
- Preserve legacy plugin compatibility.
- Add `inventory.snapshot@1` and `facets.query@1`.
- Reuse `capability.on-off.set@1`.
- Freeze exactly three MCP tools and closed schemas.

### Build production provider host adapter

- Instantiate one provider component/store per configured node.
- Keep guest resource handles only in the provider actor.
- Publish handle-free endpoint metadata.
- Route query and absolute set calls through generated bindings.
- Enforce queue-inclusive deadlines, exact grants, generation/incarnation fences, and endpoint withdrawal.

### Implement ESPHome Wasm component

- Pin `esphome-client` 0.2.0 with API 1.14 and mDNS disabled.
- Use one Noise session/cache per configured node.
- Publish four On/Off facets and three Boolean facets for the representative node.
- Preserve stable facet identities across reconnect.
- Return accepted-for-delivery only after a successful write.
- Report post-write uncertainty as outcome-unknown without blind replay.
- Receive the Noise key only through the existing protected configuration mechanism.

### Implement three canonical MCP tools

| Tool | Scope | Initial behavior |
|---|---|---|
| `litehouse_inventory` | `litehouse:read` | Nested instance and facet inventory |
| `litehouse_query` | `litehouse:read` | Typed observed state, availability, quality, revision, generation |
| `litehouse_interact` | `litehouse:operate` | Absolute On/Off set only |

No shell, filesystem, arbitrary network, secret, registry publication, ownership, trust, generic operation, or unselected management tool is exposed.

### Prove deterministic end-to-end behavior

- Build and validate the real component.
- Use secret-free Native API/Noise fixtures.
- Prove discovery, state hydration, reconnect identity, absolute command encoding, and truthful receipts.
- Prove host routing, stale-generation rejection, endpoint withdrawal, and store isolation.
- Prove exact three-tool MCP inventory, schema rejection, scope denial, redaction, and handler semantics.

### Run supervised Relay 1 MCP proof

1. Load the existing test credential through runtime-only protected configuration.
2. Start Litehouse with the compiled ESPHome component and exact TCP grant.
3. Call MCP inventory and identify Relay 1.
4. Query and record its original observed state.
5. Interact with the opposite absolute value.
6. Query until the new observation appears.
7. Interact with the original absolute value.
8. Query until restoration appears.
9. Stop Litehouse and remove runtime credential material.

A restoration failure is a blocker and requires manual verification. No other entity may be mutated.

The supervised proof completed successfully. Relay 1 was observed Off, set
absolutely On, observed at revision 2, restored absolutely Off, and observed at
revision 3. Both writes returned accepted-for-delivery; restoration was
independently observed. No other entity was selected or mutated, and the
runtime-only credential material left with the proof process.

### Validate and commit

- LSP diagnostics on changed Rust files.
- Focused contract, provider, host-adapter, Wasmtime, and rmcp tests.
- Direct `wasm32-wasip2` component build.
- `wasm-tools validate --features cm-async`.
- Release Litehouse build.
- Rustfmt and Taplo checks.
- Full T0 baseline.
- Secret, generated-file, scope, diff, and status review.
- Bounded commits staged only from tranche-owned paths.

## Remaining tranches

| Tranche | State | Purpose |
|---|---|---|
| T5 / T5b | Deferred | Registry replacement, protected publisher, and signing custody; existing registry retained |
| T6a + MCP proving slice | Completed | ESPHome component, production host adapter, three MCP tools, supervised Relay 1 proof |
| T6b | Completed | Tasmota HTTP provider |
| T6c | Completed | ntfy notification provider |
| T6d | Completed | attributed Open-Meteo provider |
| T6e | Completed | Four-provider release group; empty MQTT, Presence, Sonoff, and Weather shells removed |
| T6f | Completed | ESPHome switches, binary sensors, numeric sensors, lights, fans, sirens, climates, covers, and buttons mapped conservatively |
| T7 | Completed | Rule JSON schema, typed compiler, immutable input, and pure planner subset |
| T13a | Completed | Network-reachable bounded Streamable HTTP with explicit no-auth warning |
| T8.0 | Completed | Restart-clean management/dependency/observation contracts and Fjall 3.1.7 feasibility |
| T8.1a | Completed | Complete Rule JSON V1 model/schema/catalog/fixtures with planner compatibility lowering |
| T8.1b | Completed | RFC 8785 semantic identity, bounded CEL, exact quantity compilation, and Jiff/tzdb schedules |
| T8.1c | In progress | Complete compiled forms landed; closed microstep planner parity is proceeding in bounded phases |
| T8.2–T8.6 | Pending | Persistent canonical management core and restart-clean execution; adopt the proving handlers |
| T9 | Pending | Portable local server/container and thin Butane/Quadlet output |
| T10 | Pending | Clean D1 identity, owner account, inventory, and pairing |
| T11 | Pending | Signaling-only broker and direct WebRTC JSON-RPC |
| T12 / T12b | Pending | Connected greenfield web journeys, then legacy site retirement |
| T13 | Pending | Promote the same three-tool adapter to authenticated stateful Streamable HTTP |
| T14 | Pending | Local connected-release candidate proof; scope must acknowledge deferred T5 unless restored |
| T15 | Deferred | Separately selected external signing, publication, cloud, DNS, staging, and deployment qualification |

## Standing boundaries

- Preserve existing runtime compiler/cache, event loop, store strategies, and legacy plugin behavior.
- One configured provider instance represents one physical device.
- Rule JSON remains authoritative; UI projections are not alternate state.
- Open Design remains digest-locked static source.
- Long-lived protected values are injected and never persisted by Litehouse.
- The existing registry is not rewritten in the active slice.
- No live registry/cloud/DNS/signing/deployment mutation is authorized.
- Current device authority is limited to the approved representative ESPHome node and supervised Relay 1 set-and-restore proof.
- Never print or commit the encryption key, raw protected config, or unrestricted device identifiers.

## Known constraints

- Workspace-wide `cargo check --workspace --all-targets` remains blocked by the pre-existing registry test reference to unresolved `crate::registry` in `partitioned_store.rs`.
- `cargo groups test bin` may remain unavailable because `cargo-groups`/`cargo-binstall` is not installed; run the documented clean-worktree package equivalents and report the limitation.
- `/mcp` is network-reachable but intentionally unauthenticated in T13a: reachability grants use. Authenticated `/mcp` remains T13 work.
- T8 deliberately does not resume or replay in-flight automation after restart; authored state persists, then rules restart only after their own dependencies rehydrate.
- ESPHome's mapped API 1.14 entities are covered by deterministic typed feature-gate and command fixtures; unsupported entities remain bounded diagnostics.
- Shared-store T3 failures still require process restart for reactivation; this provider uses the default per-instance strategy.

## Compaction handoff

After context compaction, reorient in this order:

1. `POP_STACK.md`
2. `.scratch/litehouse-platform/implementation-decision-log.md`
3. `.scratch/litehouse-platform/issues/40-prepare-implementation-ready-handoff.md`
4. `git status --short` and `git log --oneline -8`
5. The active combined T6a + MCP todo subtree

The next implementation action is **complete full deterministic planner parity (T8.1c), then persist authored management state in Fjall 3.1.7 (T8.2)**. Do not reopen T0–T7, T8.1a–b, T13a, the provider-WIT observation decision, or the proven three-tool adapter unless a demonstrated regression requires it.
