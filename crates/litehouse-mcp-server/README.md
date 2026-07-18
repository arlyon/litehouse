# Litehouse MCP Streamable HTTP

This crate serves the existing `litehouse-management` MCP adapter at `/mcp` over stateful Streamable HTTP.

## Security model

T13a is intentionally unauthenticated: **if a client can reach the listener, it can use the configured adapter principal**. Startup emits a prominent structured warning. Operators must provide an explicit bind address, exact allowed hosts, and exact HTTP(S) origins; wildcard origins and hosts are rejected.

The adapter factory is the single boundary reserved for later authentication and principal injection. This crate does not add tools or a generic operation dispatcher: the adapter still exposes only `litehouse_inventory`, `litehouse_query`, and `litehouse_interact`.

Sessions have configurable count and idle bounds. `McpServer::shutdown` cancels the listener, closes active sessions, and applies a bounded graceful-shutdown deadline.
