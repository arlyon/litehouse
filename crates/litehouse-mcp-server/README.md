# Litehouse MCP Streamable HTTP

This crate serves the existing `litehouse-management` MCP adapter at `/mcp` over stateful Streamable HTTP.

## Security model

T13a is intentionally unauthenticated: **if a client can reach the listener, it can use the configured adapter principal**. Startup emits a prominent structured warning. Operators must provide an explicit bind address, exact allowed hosts, and exact HTTP(S) origins; wildcard origins and hosts are rejected.

The adapter factory is the single boundary reserved for later authentication and principal injection. This crate does not add tools or a generic operation dispatcher: the Phase 1 adapter exposes only `litehouse_inventory`, `litehouse_query`, `litehouse_interact`, and `litehouse_workflows`.

Requests are collected by bounded middleware before RMCP receives them; the default maximum is 512 KiB and applies to fixed-length and chunked bodies. Minimal `/healthz` and `/readyz` routes expose no configuration: health is live while the listener runs, while readiness remains false until the daemon explicitly marks startup complete and is cleared before shutdown. Sessions have configurable count and idle bounds. `McpServer::shutdown` cancels the listener, closes active sessions, and applies a bounded graceful-shutdown deadline.
