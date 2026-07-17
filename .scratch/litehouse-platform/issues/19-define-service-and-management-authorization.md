# Define service and management authorization

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 02, 09, 14, 16
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which plugin declarations, host grants, selector scopes, contract operations,
OAuth audiences/scopes, AI/MCP permissions, and checks at install, bind, rebind,
invocation, remote management, and audit form one default-deny authorization
model while keeping WASI capabilities, service permissions, and host-level
access distinct?

## Answer

Use one small default-deny authorization seam with typed principals, actions,
resources, and grants. Do not add a policy DSL, approval-token protocol, separate
audit database, plugin OAuth, or AI-specific authorization system.

### Principals and permission planes

External web, CLI, mobile, and MCP clients authenticate through the same OAuth
protected management API. Fixed v1 wire scopes are `litehouse:read`,
`litehouse:operate`, `litehouse:configure`, `litehouse:install`,
`litehouse:diagnostics`, and bounded `litehouse:admin`. Tokens remain bound to
issuer/audience, client, user, server, and pair epoch. `litehouse:admin` is not
host root. It never implies raw-secret access, ownership/trust bypass, host shell,
or arbitrary filesystem/network/container access.

Each MCP/AI integration is a separate OAuth client with the subset of scopes the
owner chooses. It cannot reuse browser cookies, grant itself scopes, or bypass
server checks. Given the same scopes/resource authority, clients use the same
API and receive the same behavior. There is no generic plan/approve/second-factor
operation layer: an authorized human or client performs the operation directly.
Validate/dry-run/plan endpoints may exist where useful, not as mandatory gates.
Ordinary destructive-confirmation UI remains UX, not a second authorization
protocol.

Plugins and rules do not receive OAuth tokens. Their unforgeable principals are
host-owned configured-instance/generation and active-rule/revision identities.
All principals reach one conceptual `authorize(principal, action, resource,
context)` decision seam, but three grant planes stay non-convertible:

- OAuth scopes authorize external management API groups;
- service grants authorize exact contract operations/selectors; and
- WASI grants authorize host facilities such as exact network destinations.

Operator-injected environment values are typed instance configuration, not a
fourth permission plane or host secret service. For each schema-marked
environment-secret field, the host derives one variable name from the host-owned
instance ID and reviewed field token; no caller can select/rebind it. The host
resolves only that value and never gives a guest ambient environment access.
Ordinary plugins use only declared service contracts, explicit WASI grants, and
their typed config. They never receive the host management API or use one plane
to obtain another.

### Plugin and service grants

A signed artifact declares its maximum requested WASI facilities, provided and
required exact-major contracts, and operations. Authority is granted per
configured plugin instance, never once for every current/future instance of an
artifact. The stored grant subset contains only concrete network authority,
contract operations, and selector scope. Environment-secret field tokens are
signed schema requirements, not grants; config/activation metadata stores only
their deterministic requirement digest/status. Names are derived and values are
never stored.

Service grants are operation-level. Read/observe, absolute set, toggle,
continuous/relative operations, invoke, and configure remain distinguishable.
Providing a capability grants no right to consume it.

A wildcard grant intentionally authorizes future matching facets within that
same configured instance. Active bindings remain deterministic snapshots: a new
facet does not appear in an existing `many` binding until explicit re-resolution,
but it needs no new permission review when the durable grant already covers it.
Same-identity generation rebind still rechecks the current grant.

Enforce at simple fixed points:

1. **Load/install:** validate declarations and reject undeclared/unsupported
   requests; store the instance grant subset.
2. **Bind/re-resolve:** intersect contract, operation, selector, provider,
   feature support, and current grant; issue only host-owned proxies.
3. **Invoke:** check proxy generation, exact operation, grant revision, caller
   deadline, and resource before queue admission.
4. **Revoke/change:** increment grant revision and deny new calls immediately;
   cancel queued work, cooperatively cancel dispatched work where possible, and
   preserve ambiguous side-effect outcomes.
5. **Remote management:** validate OAuth token/scope/server binding, then the
   exact API operation/resource/revision. Scope alone does not bypass ordinary
   input, ownership, concurrency, or safety validation.

Host-synthesized denial uses the shared typed error envelope. Guests cannot
supply or forge provider IDs, authorization context, grant revisions, or host
resource handles.

### Observability

Do not build a second durable audit layer. Authorization and effect boundaries
emit bounded, redacted structured `tracing` events under stable groups such as
`litehouse.authn`, `authz`, `grant`, `service_call`, and `management`. Operators
may tee/filter those groups into any configured retention/observability service.
Events contain safe principal/resource/action IDs, grant revision, decision and
reason, correlation/idempotency IDs, and outcome confidence; never tokens,
secrets, or unrestricted state payloads. Routine state reads/stream items are not
duplicated as audit records.

This model deliberately chooses clarity over speculative flexibility: fixed
scopes, typed records, one decision function, explicit checks, and ordinary logs.
Any future policy engine, finer scope vocabulary, per-user sharing, third-party
publisher roles, or stronger audit storage requires demonstrated pressure and a
separate decision.
