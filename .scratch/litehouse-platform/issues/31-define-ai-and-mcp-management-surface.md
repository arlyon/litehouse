# Define the AI and MCP management surface

Type: grilling
Mode: HITL
Status: resolved; exact initial subset deferred to T13 selection
Resolved: 2026-07-17
Blocked by: 02, 09, 13, 19, 27, 29, 30, 44, 47
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 12

## Question

How should Litehouse expose a small useful subset of its management API to MCP
clients without creating host, shell, filesystem, secret, or generic-network
authority?

## Answer

Add a thin local MCP adapter after the canonical T8 management handlers exist.
Use the official Rust MCP SDK (`rmcp`) over Streamable HTTP on the configured
local HTTPS server. Generate adapters only for a small, explicitly approved
subset of canonical operations.

Do not expose every operation merely because it exists in the management
manifest. Do not add a generic method/path/body caller or a separate AI policy
engine.

This decision supersedes the former one-tool-per-operation launch requirement,
74-tool parity gate, custom MCP resource catalog, and bespoke session/CPU quota
system in this issue.

## Initial tool selection

The exact first tool list is selected at T13 after T8 and the connected web
journey demonstrate which operations are genuinely useful. The default selection
principle is:

- safe inspection needed to understand the server and configured instances;
- current facet/state reads needed to answer ordinary home questions; and
- a small number of essential, reversible device controls already exposed in the
  web UI.

Registry publication, ownership, account recovery, trust changes, secret values,
host/container administration, arbitrary configuration, destructive cascades,
and unrestricted diagnostics are excluded.

Adding a tool later is a reviewed adapter addition, not a protocol redesign.

## Adapter boundary

Each MCP tool:

- has a stable name and closed input/output schema generated from one approved
  canonical operation;
- invokes the same T8 handler used by HTTP, WebRTC, and the web UI;
- uses the same principal, scopes, authorization, validation, revision,
  idempotency, redaction, and typed errors as that handler; and
- adds no authority, fallback implementation, generic JSON escape hatch, or
  plugin-supplied instructions.

MCP is an adapter, not a second management API.

## Explicit exclusions

No MCP tool may provide:

- shell commands or process execution;
- filesystem, database, container-engine, or host access;
- arbitrary URLs, HTTP requests, sockets, or guest RPC;
- secret values, environment enumeration, tokens, or credentials;
- registry signing/publication or trust-root mutation;
- account ownership, pairing reset, passkey/recovery administration, or OAuth
  client administration;
- generic log queries or unrestricted diagnostics;
- prompts, sampling/model calls, or plugin-authored tool descriptions; or
- a generic canonical-operation caller that bypasses the approved subset.

## Authentication and transport

Serve `/mcp` only on the configured authenticated HTTPS origin. Reuse normal
Litehouse OAuth/resource authorization from T10/T8. Do not add MCP over WebRTC,
a central MCP gateway, a production stdio child process, or a separate token
system.

Use the SDK's normal bounded request/session behavior and the server's ordinary
admission/deadline controls. Add special MCP limits only if conformance or
measurement demonstrates a concrete need.

## Validation

T13 passes when:

1. the explicitly selected tool list is small and reviewable;
2. every tool maps to exactly one canonical handler;
3. valid and invalid fixtures match the corresponding HTTP handler semantics;
4. scopes and redaction prevent cross-owner, secret, and excluded-field access;
5. attempts to invoke generic host/network/filesystem or unselected operations
   fail because no such tool exists;
6. the adapter passes MCP SDK/conformance smoke tests; and
7. removing or disabling the adapter leaves HTTP, WebRTC, and web behavior
   unchanged.

The initial subset must be presented for explicit approval before T13 code is
written.
