# Research safe AI operation patterns

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which current authorization and operation patterns should constrain AI clients
with full access to Litehouse APIs?

## Answer

Partial standards/security research converges on one closed, typed management
surface with server-enforced controls—not general-purpose host tools made safe
by confirmation:

- Use `inspect -> validate -> plan -> approve/apply -> verify`, with schema-
  validated targets, audience-bound scopes, deadlines, revisions, reason, and
  correlation/idempotency IDs.
- Bind approvals and applies to an immutable plan digest, exact targets,
  revisions, policy version, expiry, and redacted machine-readable diff. Recheck
  authorization and preconditions at apply time.
- Mutations require idempotency keys and optimistic revision/ETag preconditions;
  duplicate retries return the original outcome and stale state fails typed.
- Audit user, agent, operation, policy decision, targets, revisions, outcome,
  partial effects, rollback, and validation. Redact before plans, logs, traces,
  support bundles, or model context.
- Diagnostics are typed, allowlisted, paginated, time/byte/item bounded, and
  source-redacted—not arbitrary shell/filesystem/database/container/network
  access.
- Never expose raw secrets, token passthrough, trust/ownership/safeguard changes,
  self-granted permissions, unsigned code installation, or deletion of the last
  recovery path through AI tools.

Primary references: [OWASP Excessive Agency](https://genai.owasp.org/llmrisk/llm062025-excessive-agency/),
[MCP authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization),
[NIST SP 800-53](https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final), and
[Kubernetes API concepts](https://kubernetes.io/docs/reference/using-api/api-concepts/).

The user chose full access to authenticated Litehouse APIs, not unrestricted
host access. Exact operation inventory, scopes, atomicity, quotas, audit
retention, and MCP transport remain decisions.
