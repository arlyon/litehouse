# Litehouse management

This crate owns the canonical handlers used by the first connected-release proof:

- `inventory_snapshot`
- `facet_query`
- `on_off_set`

`McpAdapter` is an in-process `rmcp` projection over those handlers. It exposes exactly `litehouse_inventory`, `litehouse_query`, and `litehouse_interact`; tool schemas come from the checked management manifest. The adapter does not dispatch arbitrary operations, call Wasmtime directly, or define separate authorization policy.

The proving principal is limited to `litehouse:read` and `litehouse:operate`. Canonical handlers authorize before resource lookup, enforce manifest deadlines and bounded input, emit structured operation logs, and return closed safe error envelopes. A bounded provider-generation-local cache supplies the contract-required host idempotency for On/Off set. Writes remain absolute and accepted-for-delivery until an independent query observes state.
