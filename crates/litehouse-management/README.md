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
key is ambiguous across instances. The connected handlers are `inventory_snapshot`, `facet_query`, and `interact` (`on_off_set` remains a compatibility wrapper for the supervised proof).

`McpAdapter` is an in-process `rmcp` projection over those handlers. Phase 1 exposes exactly `litehouse_inventory`, `litehouse_query`, `litehouse_interact`, and `litehouse_workflows`. `litehouse_interact` is a closed `operation`-tagged union generated from exactly the 29 authoritative capability operations; each supported variant invokes a typed provider-host method after facet-kind, advertised-operation, required-feature, generation, deadline, and idempotency checks. It has no operation-ID-plus-JSON dispatcher. On/Off toggle and notification-template expansion remain deterministic `unsupported` results because the provider component ABI does not expose toggle and Phase 1 has no notification-template repository. The workflow tool uses a separate closed tagged action schema and writes authored files without activating them.

The proving principal is limited to `litehouse:read` and `litehouse:operate`. Canonical handlers authorize before resource lookup, preserve ambiguous-facet failure and no-blind-retry behavior, enforce manifest deadlines and bounded input, pass validated idempotency keys through the typed host context, emit structured operation logs, and return closed safe error envelopes. The existing bounded provider-generation-local cache continues to deduplicate On/Off set. Receipts are safely projected without exposing provider detail.

`examples/esphome_mcp_proof.rs` is an explicitly guarded supervisor for the approved set/observe/restore evidence path. It requires runtime-only component, address, instance, and Noise-key environment inputs plus `LITEHOUSE_APPROVE_RELAY1_PROOF=yes`; it selects exactly one `Relay 1` On/Off facet and always attempts the absolute restore after mutation is attempted.
