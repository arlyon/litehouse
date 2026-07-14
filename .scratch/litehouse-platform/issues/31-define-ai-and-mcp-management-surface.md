# Define the AI and MCP management surface

Type: grilling
Mode: HITL
Status: open
Blocked by: 02, 09, 13, 19, 27, 28, 29, 30, 44
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 12

## Question

Which canonical Litehouse resources/operations, scopes, schemas, revisions,
idempotency, validation/plan/apply/verify phases, bounded diagnostics, redaction,
audit, cancellation, and errors are exposed with web/API parity, and how does a
lightweight Rust MCP adapter map them without adding authority or exposing host
shell, arbitrary I/O, secrets, ownership, or safeguard controls?
