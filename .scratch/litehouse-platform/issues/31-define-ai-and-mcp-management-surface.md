# Define the AI and MCP management surface

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-15
Blocked by: 02, 09, 13, 19, 27, 28, 29, 30, 44, 47
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 12

## Question

Which canonical Litehouse resources/operations, scopes, schemas, revisions,
idempotency, validation/plan/apply/verify phases, bounded diagnostics, redaction,
audit, cancellation, and errors are exposed with web/API parity, and how does a
lightweight Rust MCP adapter map them without adding authority or exposing host
shell, arbitrary I/O, secrets, ownership, or safeguard controls?

## Answer

Expose one canonical typed management operation registry through HTTP, WebRTC,
web UI, and a thin local MCP adapter. Generate one precise MCP tool per canonical
operation and resource templates for stable reads. Use official `rmcp` 2.2.0 at
`/mcp` with bounded stateful Streamable HTTP. Do not expose a generic method/
path/body caller, shell, filesystem, database, container, network client, guest
RPC, arbitrary log query, or separate AI policy/approval system.

MCP is served directly by each Litehouse server over its configured local HTTPS
origin in cloud-connected or standalone mode. There is no central MCP gateway,
Cloudflare management proxy, MCP-over-WebRTC dialect, production stdio child
process, legacy SSE endpoint, or WebSocket transport. A cloud-connected remote
agent must reach the server's operator-configured HTTPS endpoint; remote MCP
through Cockpit is deferred. `/mcp` is disabled in `unsecure-local` v1; this
narrow point explicitly supersedes the earlier optional exposure in the
authentication decision.

### One canonical operation registry

The same build-time operation definitions used by HTTP/WebRTC generate:

- stable exact-major operation ID and MCP tool name;
- canonical input/output/error Rust types and closed JSON Schemas plus any
  generated MCP field projection schema;
- exact OAuth scope and resource/action authorization metadata;
- read/mutation, destructive, idempotency, revision, deadline, and streaming/
  progress annotations;
- HTTP binding, WebRTC binding, MCP tool adapter, and web client; and
- schema-directed redaction and safe trace fields.

HTTP and WebRTC validate the canonical type. MCP first validates its generated
projection and deterministically maps allowed fields into that same canonical
request (for example, private address fields become `unchanged` and cannot be
set on create). Every adapter then calls the same handler, principal,
authorization seam, revision/CAS, idempotency/result store, deadline/cancellation,
and complete shared error envelope. MCP never reimplements domain logic or
widens a type to generic JSON. Tool names include the operation major; breaking
changes publish a new tool rather than changing old meaning.

The fixed operation domains are:

1. **System/read** (`litehouse:read`) — version/runtime-profile, bounded health,
   readiness, safe server summary, current auth mode label, and public contract/
   schema catalog; never host/process/environment enumeration.
2. **Instances** (`litehouse:read`/`litehouse:configure`) — list/get safe
   metadata and bounded
   non-secret config, validate config, create/update draft, activate, disable,
   delete with explicit dependency cascade, current facets/health, and config/
   grant semantic diff. Private connector addresses persist locally but are
   omitted from MCP input/output; credentials are deterministic runtime
   environment fields returning only required variable name plus configured
   status. MCP may create/update a draft only when omitted private address fields
   already exist or are unnecessary; web/CLI sets addresses. MCP cannot supply/
   reveal a credential value or select/rebind its variable name, and activation
   waits for operator injection plus container recreation.
3. **Capabilities** (`litehouse:read`/`litehouse:operate`) — list/describe exact
   facets/features,
   query generation-qualified state, and one generated tool per exact-major
   contract operation such as On/Off set. Every mutation takes stable selector/
   facet identity, typed arguments, deadline, expected generation/revision where
   applicable, and idempotency key. Unsupported operations are absent/rejected,
   never emulated.
4. **Rules** (`litehouse:read`/`litehouse:configure`/`litehouse:operate`) —
   list/get immutable revision and safe
   status, validate, compile, simulate, save draft, activate/rollback revision,
   enable/disable, delete, trigger where explicitly supported, inspect bounded
   run/result/trace metadata, and cancel undispatched work under rule semantics.
5. **Registry/plugins** (`litehouse:read`/`litehouse:install`) — browse exact
   releases and trust/
   compatibility metadata, refresh, produce update/install/rollback diff, stage
   verified bytes, and explicitly apply exact-version instance/plugin changes.
   No publisher, signing, bucket, arbitrary URL, or unsigned-remote operation.
6. **Operation results** (scope of original operation) — query durable mutation
   state/result by canonical operation and idempotency key, preserving running,
   terminal-known, and terminal-unknown semantics.
7. **Diagnostics** (`litehouse:diagnostics`) — bounded typed health/error
   summaries and
   allowlisted trace queries by safe time range, target group, correlation/error
   ID, severity, and pagination. No raw logs, SQL, files, environment, process,
   network, container, stack dump, or unrestricted tracing expression.
The checked-in/generated `management-operations-v1.json` is the closed machine
inventory and source for HTTP/WebRTC/MCP/web generation. Each row carries exact
operation ID, HTTP method/path, MCP tool, scope/action/resource, canonical input/
output/error and MCP projection schema digests, read/mutation/destructive flags,
idempotency requirement,
optional revision/generation rule, deadline, progress, and MCP resource mapping.
V1 contains only these rows; every scope below is the exact wire string.

- `system.get@1`, `system.health.get@1`, and `schemas.get@1`:
  `litehouse:read`; read tools plus server/schema resources.
- `instances.list@1`, `instances.get@1`, and `instances.facets.list@1`:
  `litehouse:read`; read/page tools plus the instance resource.
- `instances.validate@1`: `litehouse:configure`; pure validation tool.
- `instances.draft.create@1`: `litehouse:configure`; mutation with idempotency
  and no prior revision.
- `instances.draft.update@1`: `litehouse:configure`; mutation with idempotency
  and config revision.
- `instances.activate@1` and `instances.disable@1`: `litehouse:configure`;
  mutation with idempotency and desired/active revision.
- `instances.delete@1`: `litehouse:configure`; destructive mutation with
  idempotency and exact revision/cascade set.
- `facets.get@1` and `facets.state.snapshot@1`: `litehouse:read`; generation-
  scoped tools plus facet/state resources.
- `facets.state.stream@1` and `facets.events.stream@1`: `litehouse:read`; one
  pull/batch tool per bounded stream with cursor gap/stale semantics.
- `rules.list@1`, `rules.get@1`, `rules.revision.get@1`, `rules.runs.page@1`, and
  `rules.run.get@1`: `litehouse:read`; read/page tools plus rule/run resources.
- `rules.validate@1`, `rules.compile@1`, and `rules.simulate@1`:
  `litehouse:configure`; pure tools with optional draft revision.
- `rules.draft.create@1`: `litehouse:configure`; mutation with idempotency and
  no prior revision.
- `rules.draft.update@1`: `litehouse:configure`; mutation with idempotency and
  base revision.
- `rules.activate@1`, `rules.rollback@1`, `rules.enable@1`, `rules.disable@1`,
  and `rules.delete@1`: `litehouse:configure`; mutation with idempotency and
  declared rule/active revision.
- `rules.trigger@1` and `rules.run.cancel@1`: `litehouse:operate`; mutation with
  idempotency and an expected revision only where metadata declares.
- `registry.list@1` and `registry.release.get@1`: `litehouse:read`; read/page
  tools plus the registry resource.
- `registry.refresh@1`: `litehouse:read`; idempotent local-cache mutation with
  idempotency.
- `plugins.install.plan@1`, `plugins.update.plan@1`, and
  `plugins.rollback.plan@1`: `litehouse:install`; pure plan/diff tools.
- `plugins.install.apply@1`, `plugins.update.apply@1`, and
  `plugins.rollback.apply@1`: `litehouse:install`; mutation with idempotency and
  desired/active revision.
- `operations.result.get@1`: the exact scope of its referenced operation; read by
  operation/idempotency key plus the result resource.
- `diagnostics.summary@1` and `diagnostics.events.page@1`:
  `litehouse:diagnostics`; bounded read/page tools.

The exact-major capability catalog adds this closed operation-ID list, each with
`litehouse:operate`, required facet generation, deadline, host idempotency, and
the contract's typed arguments/receipt/error:

```text
capability.on-off.set@1              capability.on-off.toggle@1
capability.level.set@1               capability.level.step@1
capability.level.move@1              capability.level.stop@1
capability.level.transition@1        capability.level.set-coupled@1
capability.level.step-coupled@1      capability.level.move-coupled@1
capability.level.transition-coupled@1
capability.color.set@1               capability.color.step@1
capability.color.move@1              capability.color.stop@1
capability.color.transition@1
capability.thermostat.set-mode@1     capability.thermostat.set-single-target@1
capability.thermostat.set-dual-target@1
capability.thermostat.set-fan@1      capability.thermostat.set-swing@1
capability.thermostat.set-preset@1
capability.covering.open@1           capability.covering.close@1
capability.covering.stop@1           capability.covering.set-lift@1
capability.covering.set-tilt@1
capability.momentary.invoke@1        capability.notification.send@1
```

Color uses one closed discriminated model input rather than model-named tools.
Feature-gated tools remain in the manifest but are hidden/rejected when no
authorized facet advertises support. Boolean State and Measurement are read
through snapshot/event operations. No plugin may add an operation outside an
accepted exact-major catalog release.

For every manifest ID `<domain>.<verb>@<major>`, HTTP is exactly `POST /api/
operations/<domain>.<verb>/v<major>`, WebRTC carries the exact manifest ID, and
MCP is exactly `litehouse_<domain>_<verb>_v<major>` after replacing dots/hyphens
with underscores. Capability IDs include their contract namespace under the same
rule. There are no transport aliases. HTTP/WebRTC canonical bodies are
identical; MCP's manifest-pinned projection maps to/from that body and preserves
operation, authorization, result, and safe error semantics while deliberately
omitting private address/value fields. Read operations also use POST and
`no-store` because private management responses are never shared-cache inputs.

No v1 MCP tool requires `litehouse:admin`; presenting that scope does not reveal
excluded operations. Producing the closed manifest, exact bindings, schemas, and
valid/invalid parity fixtures is the first implementation artifact and launch
gate; domain prose does not permit additional tools.

The central account, passkey, recovery, pairing/unpairing, ownership transfer,
OAuth client registration, issuer/trust mode, server identity-key reset, schema/
trust-policy weakening, container/host update, raw environment-secret reveal/
export, protected endpoint reveal, local-developer artifact selection, registry
publication/signing, and last-recovery-path deletion are not local MCP tools.
They remain central web, local host command, or separately reauthenticated web/
CLI operations as already decided. This is a protocol safety boundary, not an
AI-specific permission engine.

### MCP resources and tools

The closed v1 read resource/template list is:

```text
litehouse://server
litehouse://instances/{instance-id}
litehouse://facets/{facet-id}
litehouse://facets/{facet-id}/state
litehouse://rules/{rule-id}/revisions/{revision}
litehouse://rules/{rule-id}/runs/{run-id}
litehouse://operations/{operation-id}/{idempotency-key}
litehouse://registry/{package}/{version}
litehouse://schemas/{schema-id}/{version}
```

Resource reads use the same handlers/scopes/redaction and bounded pagination as
HTTP. Resource URIs contain opaque IDs, never addresses, secrets, tokens, or raw
selectors. Each lookup re-resolves server/owner membership and action through
the authorization seam; possession, guessing, or cross-session reuse of a URI is
never authority. V1 does not promise MCP resource-subscription notifications. Each
canonical state/event streaming operation has one same-ID MCP tool that returns
one bounded typed snapshot/event batch plus generation/cursor/gap metadata; the
client invokes it again with the cursor. HTTP/WebRTC may keep the same operation
open and push those identical batches. Progress notifications carry only job
progress, never domain events. This is the declared transport projection—not a
second operation or a loss of snapshot/resume/gap semantics.

Generate one MCP tool for each operation visible to the authenticated client's
scopes. Read-only, destructive, idempotent, and open-world annotations reflect
canonical metadata but never replace server authorization. Tool listing may hide
operations lacking scope to reduce context; direct invocation still rechecks.
Dynamic installed contract operations are generated from accepted exact-major
catalog metadata through a fixed host template, not plugin-supplied arbitrary
schemas or tool descriptions.

Tool descriptions and schemas are bounded, factual, and generated from reviewed
contract/API documentation. Plugins cannot inject prompts, Markdown/HTML,
instructions, examples containing device data, or model-targeted text. V1
publishes no server prompts and performs no sampling/model calls.

### Authentication and sessions

`/mcp` publishes RFC 9728 protected-resource metadata and exactly one configured
authorization server. The local HTTPS origin is transport, not necessarily the
OAuth resource: cloud mode publishes and requests the logical per-server
`https://api.litehouse.dev/servers/{server-id}` resource from the authentication
decision; standalone publishes its configured canonical HTTPS origin. JWT
`aud` must equal that applicable identifier, never an alternate local origin.
Use MCP authorization 2025-11-25, Authorization-header bearer tokens, RFC 8707
resource binding, and S256 PKCE. Do not accept ID tokens, cookies, tokens in URLs,
unauthenticated dynamic registration, password/implicit grants, or central
browser sessions.

Each MCP integration is an externally preregistered OAuth client and receives
only owner-approved fixed scopes. Cloud mode uses the issuer's registered client;
standalone configuration contains a bounded allowlist of at most eight client
IDs with exact redirect URIs, replacing the earlier single-client shorthand.
Token issuer/audience, owner subject, client, server ID/pair epoch or standalone
not-before, exact scope set, times, `kid`, and access-token type are validated by
ordinary middleware before `rmcp`. The resulting principal is bound to the MCP
session and passed unchanged to handlers. Every HTTP request, including session
continuation/DELETE, revalidates bearer token and must match session issuer,
subject, client, resource/audience, server/epoch, and identical scope set. Any
scope-set change starts a new session; expiry, unpairing, or trust generation
closes the old session and cancels eligible work.

Hard defaults are eight concurrent MCP sessions per server and two per principal/
client, 15-minute idle and eight-hour absolute session lifetime capped by token
leases, 16 pending requests, four progress streams, 1-MiB inbound HTTP body,
256-KiB structured tool/resource result, 64 queued notifications, and 4 MiB total
session buffers. The host generates 256-bit CSPRNG session IDs after successful
initialization, transports them only in the MCP session header, rejects caller-
chosen/pre-initialization IDs, and never puts them in URLs. Require exact allowed
Host and Origin when present, reject cross-origin browser requests, disable CORS,
require protocol Content-Types, and mark authenticated responses `no-store`.

Cap the visible catalog at 128 tools/resources and 1 MiB of immutable cached
schema/description bytes, with `tools/list` and `resources/list` pages of 50.
Inputs allow depth 32, 16,384 JSON nodes, 256 object members, 4,096 array items,
64-KiB strings, and 1 MiB after decompression; compression is disabled at `/mcp`
in v1. Admission is fixed at 120 requests/minute with burst 32 server-wide and
30/minute with burst eight per principal/client, 16 concurrent requests server-
wide and four per principal/client. A 32-unit global and eight-unit per-principal
semaphore charges read/resource/list as one, mutation as two, validate/compile as
four, and simulation/install apply as eight; at most two eight-unit operations
run globally.

Reserve by exact operation path before reading the body, then charge parsing,
schema validation, compilation/simulation, serialization, and handler CPU
through completion. MCP has rolling measured CPU budgets of 30 CPU-seconds/minute
with burst eight server-wide and eight CPU-seconds/minute with burst two per
principal/client. Per-request CPU ceilings are 100 ms for reads, 500 ms for
ordinary mutation/validation, two seconds for compile/install, and five seconds
for simulation; canonical deadlines may be lower and I/O wait still obeys its
own deadline. Exceeding a rate, weight, concurrency, or CPU budget cancels before
dispatch where possible and returns typed overload; after possible dispatch it
uses the ordinary ambiguity contract. Per-tool canonical limits are usually
lower. Capacity is reserved before accepting work; excess never evicts another
session/result. Shutdown stops new sessions and follows ordinary
cancellation/ambiguity rules.

### Mutations, validation, and authority

The earlier safe-AI research's mandatory approval-token proposal is superseded
by **Define service and management authorization**. An authorized MCP client may
execute its scoped operation directly. There is no universal inspect/approve
credential, policy DSL, second-factor ceremony, or special AI allowlist.

Canonical validate/simulate/diff/plan endpoints remain available where they are
useful—rules, instance config, install/update/rollback, and destructive cascade.
They return redacted machine-readable findings and a semantic digest. Apply
always repeats validation, authorization, grants/features/trust, expected
revisions, and safety checks. Supplying a prior plan digest is an optional stale-
plan precondition, never authority; direct apply performs the same checks.

Every mutation requires a random 128-bit idempotency key. Revision/generation
preconditions are required only where the closed operation metadata declares
them; create, refresh, trigger, or another operation with no prior entity may
explicitly declare none. Reuse with different canonical input conflicts; matching
reuse joins/returns the existing durable result and never dispatches a second
effect. Declared stale revisions fail typed. Destructive operations require an
explicit typed cascade/target list and exact dependency revisions—ordinary UI
confirmation does not become an MCP approval protocol.

An environment-secret instance field has no API value/reference. Its variable
name derives from host-owned instance ID plus reviewed schema field token;
MCP may see the name/status but cannot choose it or read the process environment.
MCP cannot grant scopes, broaden plugin/rule/service/WASI authority, reveal an
injected value, weaken a safeguard, choose unsigned code, alter auth/trust mode,
or use one permission plane to obtain another. Non-secret instance
fields and authorized Rule JSON/CEL source remain available as bounded untrusted
canonical data. Host-
owned IDs, principal, auth context, grant revision, resource handles, and effect
confidence are never caller-supplied authority.

### Cancellation, progress, and verification

`rmcp` request-correlated cancellation maps to the canonical cancellation token.
Before dispatch it yields definite cancellation. After possible side effect it
is cooperative and may yield `outcome-unknown`; MCP never translates cancellation
into rollback or automatic retry. Session loss follows the same durable
idempotency/result-lookup contract. Clients reconnect and query result/revision;
the adapter never replays a mutation.

Progress is bounded structured metadata from canonical jobs: safe phase, current/
total when meaningful, correlation ID, and truncation/terminal status. It is not
raw logs or arbitrary guest text. Progress notification backpressure counts
against session buffers; lag terminates progress delivery without cancelling the
underlying durable mutation unless the client explicitly cancels.

Verification is an ordinary read/result operation: query terminal result,
activation/rule revision, or later observed state. Receipt confidence remains
accepted-for-delivery, acknowledged, completed, or observed-completed exactly as
the capability contract reports. MCP/UI never infer physical completion from a
successful tool call or matching later state without causal evidence.

### Diagnostics, pagination, and redaction

All collection reads use stable opaque cursors scoped to principal, operation,
filter, and snapshot/generation where applicable. Defaults are 50 items, maximum
200, 64-KiB page, and five-minute cursor lifetime. Diagnostic queries cover at
most one hour by default and 24 hours maximum, return at most 100 events/64 KiB,
and use allowlisted groups/fields. Expired/gapped cursors return typed gap and
safe restart guidance.

Redact before serialization—not in UI/model instructions. Normal API/MCP
responses never contain injected environment values, device addresses, Noise
keys, OAuth/TURN material, private identity evidence, protected connector fields,
notification message bodies, unrestricted state payloads, guest diagnostic
text, host paths, environment, or stack traces. Bounded non-secret instance
config and authorized Rule JSON/CEL source are explicitly allowed and marked
untrusted data, never instructions. MCP receives only the deterministic required
environment name and configured/unconfigured status for a protected field. Safe
labels remain bounded untrusted display text.

Schema-directed output allowlists safe fields; unknown fields default to hidden.
Errors preserve the complete typed classification/retry/ambiguity/revision/error-
ID envelope but replace unsafe messages/details with a stable safe summary.
Truncation is explicit. The adapter never places hidden values in MCP metadata,
progress, resource names, annotations, protocol errors, or tracing.

### Error mapping and parity

MCP framing/lifecycle/schema/session failures use standard bounded JSON-RPC/MCP
errors. Once a canonical operation is admitted, domain/authorization/capacity/
timing/execution/ambiguity failures return one structured tool result containing
the exact safe shared error envelope and `isError`, not a lossy string or invented
HTTP status. HTTP/WebRTC/MCP fixtures must produce semantically identical typed
success/error results for the same principal and projected request; parity is
measured after the explicit MCP private-field projection.

Malformed or oversized input fails before handler admission. Guest/plugin errors
cannot choose protocol error codes or inject tool metadata. Unknown tool/resource
IDs, unsupported exact major, and stale generated schemas fail closed with safe
refresh guidance.

### Observability and lifecycle

Use existing bounded `litehouse.authn`, `authz`, and `management` tracing. Record
safe MCP session/client/principal/server, operation/resource/action, scope/grant
revision, request/correlation/idempotency, authorization decision/reason,
deadline/cancel, outcome confidence, duration/count/bytes, and truncation. Never
record tokens, session IDs usable as credentials, raw input/output, secret/
address references, model/provider prompts, notification text, or unrestricted
errors. There is no AI transcript store or second audit database.

The MCP adapter is native host infrastructure, never a plugin/Wasm guest. It has
no authority beyond the authenticated canonical call and no direct access to
Wasmtime stores, registry/R2, D1/Cloudflare, filesystem, sockets, process control,
or secret storage. Removing/disabling `/mcp` does not change HTTP/WebRTC/web API
semantics or active local automations.

### Validation contract

Required implementation gates include exact-pinned minimal `rmcp` feature/binary
cost; MCP 2025-11-25 Inspector/conformance; protected-resource/auth-server
metadata and challenges; cloud logical-resource versus transport-origin and
wrong-audience rejection; PKCE/resource/scope flows with preregistered clients;
standalone client/redirect allowlist; no cloud gateway/legacy transport or
`unsecure-local` endpoint; CSPRNG/header-only session IDs, Host/Origin/Content-
Type/CORS/no-store rules, principal isolation, and identical session scopes;
token expiry/epoch revocation; catalog pagination/aggregate schema, parser,
request/CPU, session, body, result, progress, and buffer bounds; cancellation
before/after dispatch and disconnect; idempotency/result lookup and operation-
specific revision rules; and closed manifest/generated tool/resource/HTTP/
WebRTC canonical plus MCP-projection schema parity.

Also require private-address omitted/unchanged/create-rejection fixtures;
pull-batch stream snapshot/resume/gap parity; dynamic contract-tool injection
rejection; every scope/operation/redaction boundary; non-secret config and Rule
JSON/CEL availability while environment-value/address/notification leakage stays
impossible; and deterministic environment-secret names with caller-rebinding
rejection. bounded diagnostics/cursors/gaps;
safe error parity; and proof that MCP cannot access host I/O, ownership/trust,
raw protected values, local unsigned artifacts, publisher credentials, or
safeguard controls.
