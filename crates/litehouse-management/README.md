# Litehouse management

This crate owns deterministic authored-workflow loading and the canonical handlers
used by the first connected-release proof. `WorkflowDirectory` recursively loads
one strict Rule JSON document per JSON/YAML file through bounded capability-scoped
handles, rejects symlinks and duplicate rule IDs, and returns one immutable
compiled snapshot plus an authored-directory SHA-256 digest. `WorkflowRepository`
retains the immutable process-loaded snapshot and supports create/update/delete
mutations guarded by expected source SHA-256 digests. Mutations serialize through
the authored-directory advisory lock, validate the complete candidate set, replace
through a synced sibling temporary file, and distinguish pre-commit conflicts from
indeterminate or externally changed post-commit state. They never change the loaded
snapshot: Phase 1 activates authored settings and workflows only by starting a new
process epoch.

`ManagementService::from_bundled` aggregates a deterministic set of exactly the
four Phase 1 provider kinds. Inventory is ordered by provider instance, bounded
across the complete set, and facet lookup fails closed when an unqualified facet
key is ambiguous across instances. The existing proof handlers are:

- `inventory_snapshot`
- `facet_query`
- `on_off_set`

`McpAdapter` is an in-process `rmcp` projection over those handlers. It exposes exactly `litehouse_inventory`, `litehouse_query`, and `litehouse_interact`; tool schemas come from the checked management manifest. The adapter does not dispatch arbitrary operations, call Wasmtime directly, or define separate authorization policy.

The proving principal is limited to `litehouse:read` and `litehouse:operate`. Canonical handlers authorize before resource lookup, enforce manifest deadlines and bounded input, emit structured operation logs, and return closed safe error envelopes. A bounded provider-generation-local host cache supplies the contract-required idempotency for On/Off set. Writes remain absolute and accepted-for-delivery until an independent query observes state.

`examples/esphome_mcp_proof.rs` is an explicitly guarded supervisor for the approved set/observe/restore evidence path. It requires runtime-only component, address, instance, and Noise-key environment inputs plus `LITEHOUSE_APPROVE_RELAY1_PROOF=yes`; it selects exactly one `Relay 1` On/Off facet and always attempts the absolute restore after mutation is attempted.
