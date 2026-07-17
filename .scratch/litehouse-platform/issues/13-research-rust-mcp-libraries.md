# Research lightweight Rust MCP libraries

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-14
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which maintained lightweight Rust MCP SDK/library and transport profile can
expose Litehouse's canonical typed management API as a thin adapter, with OAuth
protected-resource discovery, scopes, cancellation, streaming, schema
generation, bounded diagnostics, testability, and minimal dependency/runtime
cost—without becoming a second domain or authorization layer?

## Answer

Use official **`rmcp` 2.2.0** as a thin protocol adapter. Mount its
`StreamableHttpService` Tower service at `/mcp` using bounded, stateful
Streamable HTTP. Do not ship legacy standalone SSE, WebSocket, child-process,
or production stdio transports. Disable default features and enable only the
server, macro/schema, and Streamable HTTP server features actually required.

Stateful sessions are necessary because the stateless path ignores
notifications; cancellation across requests therefore needs a bounded session.
Litehouse owns session count, lifetime, pending work, buffered events, stream
limits, and shutdown.

`rmcp` is the simplest credible choice because it is the official maintained
Rust SDK, targets MCP 2025-11-25, implements Tower service integration,
request-correlated cancellation tokens, progress notifications, structured tool
schemas through `schemars`, in-memory test transports, and Streamable HTTP
session/framing behavior. It is Apache-2.0 and feature-gated. Its Wasm support is
irrelevant: MCP belongs in the native Litehouse daemon, not a guest plugin.

Litehouse continues to own the canonical management types and operations, OAuth
token/resource/server validation, the fixed management scopes, authorization,
revisions/idempotency where needed, domain errors, redaction and diagnostic
limits, TLS/Origin/Host policy, tracing, and all resource/session bounds. The
authenticated principal enters through ordinary HTTP middleware and passes
unchanged to canonical API calls. Do not enable `rmcp` OAuth client machinery or
create MCP-specific policy.

The adapter does only:

```text
MCP input → canonical typed request → canonical API call
          → bounded structured result/error
```

Delegate JSON-RPC/MCP framing, lifecycle/capability negotiation, tool routing,
Streamable HTTP protocol behavior, request IDs, progress/cancellation
correlation, MCP schema serialization, and protocol errors to `rmcp`.

`rust-mcp-sdk` 0.10.0 with `rust-mcp-axum` 0.2.3 is the runner-up, but its HTTP
integration forces an auth feature and associated JWT/HTTP dependencies that
would duplicate Litehouse's boundary, while handler cancellation correlation is
more application-owned. `pmcp` 2.15.0 and `turul-mcp-server` 0.3.47 are broader
frameworks than this thin adapter needs. Older `mcp-protocol-sdk` and `mcp-sdk`
releases predate the required protocol and are not candidates.

Implementation validation should stay small: prove `schemars` 0.8-to-1.x schema
compatibility, authenticated-principal isolation across sessions, compliant
scope-denial challenges, cancellation reaching the canonical future, bounded
progress/session behavior, canonical error redaction, feature-resolved binary
cost, and Inspector/conformance interoperability. These are implementation
checks, not reasons to introduce a second MCP domain model.

Primary sources:

- [`rmcp` 2.2.0](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v2.2.0)
- [`rmcp` features](https://github.com/modelcontextprotocol/rust-sdk/blob/rmcp-v2.2.0/crates/rmcp/Cargo.toml)
- [`rmcp` Streamable HTTP Tower service](https://github.com/modelcontextprotocol/rust-sdk/blob/rmcp-v2.2.0/crates/rmcp/src/transport/streamable_http_server/tower.rs)
- [MCP authorization 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
- [MCP transports 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)
- [`rust-mcp-sdk` 0.10.0](https://github.com/rust-mcp-stack/rust-mcp-sdk/releases/tag/rust-mcp-sdk-v0.10.0)
