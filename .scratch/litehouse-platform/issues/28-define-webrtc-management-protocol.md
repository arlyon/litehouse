# Define the WebRTC management protocol

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-14
Blocked by: 07, 16, 27
Map: [Find the route to Litehouse's first connected release](../map.md)
PRD item: 9

## Question

Which protocol-v1 codec, `HELLO` negotiation, envelope, grant/fingerprint
binding, request/stream IDs, sequence/correlation/deadline fields, domains,
16-KiB chunking, bounded queues/credits, cancellation, heartbeat, disconnect,
outcome lookup, resume, ICE restart, idempotency, revision, audit, and major-
version failure rules govern direct management?

## Answer

Use one in-band DCEP data channel named `litehouse-management`, configured
reliable and ordered, with application protocol `litehouse-management/1`. Carry
strict typed JSON control envelopes and bounded raw binary chunk frames. HTTP,
WebRTC, and later MCP adapters dispatch the same generated management operation
IDs into the same typed handlers and authorization seam. Do not tunnel arbitrary
HTTP, expose guest RPC, create channels per stream, or define a second management
API.

### Authentication and negotiation

The first application frame is a `client-hello` JSON envelope no larger than
16 KiB containing protocol major/minor ranges, client ID, supported features and
limits, signaling-transcript digest, management JWT, and fresh client nonce. No
other frame is accepted first. The server verifies the Cockpit signaling
transcript/DTLS commitments and the complete token profile from **Define
authentication and pairing** before returning `server-hello` with selected exact
major/minor, connection ID, server nonce, token expiry, accepted limits/features,
and initial receive credits.

Both sides independently compute `SHA-256("litehouse-management-transcript-v1\0"
|| length-prefixed fields)` over signaling-grant ID, owner, client, server, pair
epoch, both signaling nonces, exact offer bytes and browser DTLS fingerprint,
exact server-key-signed answer bytes and server DTLS fingerprint, and the actual
peer certificate fingerprints observed after DTLS. `client-hello` supplies that
digest and `server-hello` proves the same value with both fresh nonces. Any
missing/mismatched field closes the channel; SDP reserialization is forbidden.

Major 1 has no implicit adaptation. No common major returns one bounded protocol
error and closes. Unknown required features, invalid transcript, expired/wrong-
server token, data before `HELLO`, or conflicting negotiation close the channel.
Unknown additive optional fields are ignored only where the generated schema
marks them optional. Minor versions are additive and the selected minor is the
lower mutually supported value.

The channel authorization lease ends at JWT expiry. A `reauth` control exchange
must finish before expiry using a token for the same issuer, owner, client,
server, pair epoch, and connection transcript; it may narrow but never broaden
scope without opening a new connection. Failure closes the channel. Owner, pair-
epoch, or trust-generation change closes it immediately. Every operation runs ordinary token/scope/resource/revision authorization at
admission and again atomically immediately before dispatch. Queue entries capture
the authorization-lease revision. Expiry, reauthentication, scope narrowing,
owner/epoch/trust change increments it and cancels queued entries that no longer
pass; no queued work dispatches under stale authority.

### Codec, framing, and bounds

Control frames are UTF-8 JSON objects with one generated schema per envelope.
Every control object has exactly `v` (integer `1`), `type` (closed string enum),
`seq` (decimal-string `u64`), `connection_id` (128-bit lowercase hex after
`HELLO`), and a type-specific object. Sequence starts at zero per sender and
increments by one; duplicate, skipped, or overflowing control sequence closes
the channel. Reject duplicate keys, invalid UTF-8, unknown required enum/union
cases, nesting beyond 16, more than 128 object members or 1,024 array elements,
strings beyond 64 KiB, trailing bytes, and non-schema values. JSON is not a
signature format; equality/idempotency digesting alone uses RFC 8785 after schema
validation. Never put arbitrary headers or unrestricted logs in envelopes.

SCTP text messages are control frames; binary messages are chunk frames. Every
physical message is at most 16,384 bytes. Control metadata and inline body
references must fit one text frame. Generated operations classify complete
inputs/outputs as standard (at most 65,536 logical bytes) or bulk (at most
4,194,304 logical bytes); only bulk-marked operations may exceed the standard
limit. After `HELLO`, any body whose complete encoded control would exceed 4,096
bytes is replaced by one transfer reference. V1 adds no transport compression.
`HELLO` may select
lower limits but never higher ones.

A binary chunk begins with this exact 40-byte network-byte-order header:
`"LHCH" [4]`, major `u8=1`, flags `u8`, reserved `u16=0`, transfer ID `[u8;16]`,
sequence `u32`, offset `u32`, total length `u32`, payload length `u16`, and final
reserved `u16=0`; the remaining bytes are exactly the payload. Only flag bit 0
(`final`) is defined; other bits fail. Payload length is at most 16,344 and must
equal frame length minus 40. A preceding `transfer-open` commits transfer ID,
owner request/stream ID, direction, content type, complete length, chunk count,
and SHA-256. Offsets are contiguous and sequences exact; final flag, total,
count, and digest must agree. Overflow, overlap, drift, unknown transfer, or
mismatch aborts and releases bytes. A transfer has a 30-second absolute and
10-second idle timeout beginning at `transfer-open`, independent of the later
request deadline, and never outlives its owner or connection.

### Generated operation envelopes

The canonical management API build generates stable operation IDs, input/output
schemas, required OAuth scope, mutation/read classification, idempotency rule,
default/maximum deadline, revision requirements, and streaming shape. IDs are
exact-major names such as `<domain>.<operation>@1`; aliases and arbitrary paths
are forbidden. HTTP and WebRTC adapters validate the same generated records and
call the same handler.

Each operation definition also generates one exact HTTP method/path and maps
request ID, relative deadline, idempotency key, and expected revision to bounded
`Litehouse-Request-Id`, `Litehouse-Timeout-Ms`, `Idempotency-Key`, and
`If-Match` headers; the typed input/output/error JSON is otherwise identical.
HTTP status is secondary to the complete error envelope. Server streams use SSE
with the same hydration/caught-up/live/gap/stale/end records and scoped cursor in
`Last-Event-ID`. Generated request cancellation uses
`DELETE /api/v1/requests/<request-id>` and result lookup uses
`GET /api/v1/results/<operation-id>/<idempotency-key>`, under the same owner,
client, server, epoch/generation, and authorization checks. Unsupported HTTP
stream resume returns the same typed gap/stale behavior, never silent restart.

Control unions are limited to `request`, `response`, `stream-open`,
`stream-item`, `stream-end`, `cancel`, `transfer-open`, `transfer-abort`,
`window-update`, `ping`, `pong`, `reauth`, and `protocol-error`. Their generated
closed schemas require:

- request: request/operation IDs, inline input or transfer ID, relative timeout,
  optional expected revision, and mutation idempotency key;
- response: request ID, terminal success or complete shared typed error,
  resulting revision, and safe correlation/error IDs;
- stream open/item/end: request/stream IDs, entity/generation, item sequence,
  scoped cursor, and one of hydration, caught-up, live, gap, stale, or terminal;
- cancel: request or stream ID and caller reason;
- transfer open/abort: the committed transfer fields above or typed abort reason;
- window update: scope (`connection` or transfer ID), cumulative consumed bytes,
  and monotonically increasing update sequence;
- ping/pong: nonce and last inbound control sequence; reauth: replacement token
  and both nonces; protocol error: stable safe code and correlation ID.

IDs are random 128-bit lowercase hex. Transport status never replaces domain
errors. Optional fields and nullability come only from the generated schema.

Relative deadlines start when a fully validated request reaches admission, not
when its first chunk arrives. Default maximum is 30 seconds unless generated
operation metadata explicitly allows a longer bounded job. Expired work is not
admitted. Queue time consumes the deadline. Clocks need not be synchronized.

### Admission, flow control, and fairness

Bulk credit counts complete physical binary-frame bytes. Each receiver advertises
a fixed 262,144-byte connection window and 65,536-byte window per active
transfer. Sender and receiver maintain checked `u64` sent/consumed totals.
`window-update` reports cumulative bytes durably released by the application,
not a delta; stale/duplicate totals grant nothing, a decrease or value beyond
sent bytes closes, and available credit is `consumed + window - sent` checked for
overflow. A chunk charges both its transfer and connection windows before send.

Text controls do not consume bulk credit. Instead each receiver reserves a
separate non-negotiable queue of at most 32,768 bytes and 128 frames; it debits
on receipt and replenishes only after processing. Controls are at most 4,096
bytes after `HELLO` and pass token buckets of 32 frames and 128 KiB per second,
with bursts of 64 frames but only 32 KiB. Exceeding either rate or the queue
closes the channel, so continuously processed ping/update/cancel floods remain
bounded. Terminal response, stream-end,
transfer-abort, protocol-error, reauth, ping/pong, and window-update are always
served before bulk chunks. No control can itself create bulk credit except a
validated cumulative window update.

A sender also pauses when SCTP `bufferedAmount` reaches 524,288 bytes and resumes
below 131,072. That never substitutes for credits. Round-robin at most one chunk
per transfer before the next request/stream. Per connection allow at most 32
admitted requests, eight streams, four transfers, and 4 MiB total reassembly.
Excess admission returns typed overload before execution; invalid accounting or
repeated protocol abuse closes the channel.

### Streams, cursors, and gaps

Streams implement **Define the state and event model**: current-state hydration,
an explicit caught-up barrier, then ordered items only within that entity/
generation scope. Each item has stream ID, sequence, scoped cursor, entity and
generation identity, and typed payload/error. Sequence is transport-local and
cannot imply cross-entity order.

A reconnect creates a new connection and subscription. The client may present
the last scoped cursor. A matching entity and generation replays when the
bounded buffer covers it; uncovered history emits typed gap, hydration, caught-
up, then live items. An entity/generation mismatch returns typed stale cursor/
handle and ends that stream; the client must resolve a current handle and open a
new subscription. Events remain at least once with stable event IDs. No WebRTC
cursor creates durable replay or exactly-once effects.

### Cancellation, disconnect, and result lookup

`cancel` is advisory and idempotent. One admission lock orders cancel, deadline,
authorization lease, and dispatch. Before the durable dispatch transition,
cancellation/deadline removes work with a definite typed result. At or after
that transition, cancellation propagates cooperatively, but any deadline,
cancel, disconnect, or lost confirmation returns `outcome-unknown` unless the
handler proves no effect. Hard teardown follows provider-generation rules and
never claims rollback.

Every mutation requires an idempotency key. Its canonical request digest is
SHA-256 over `"litehouse-management-request-v1\0"`, owner/client/server, pair
epoch or trust generation, operation ID, expected revision, and RFC 8785
canonical validated input. The durable key scope contains those authority fields
plus the key. In one storage transaction before queueing, admission reserves the
operation's generated worst-case result bytes and writes `reserved`, then
`queued`. Immediately before invoking any effect it durably writes
`dispatching`; acceptance by the handler writes `running`; completion writes
`terminal-known` with the full typed result/revision or `terminal-unknown`.

A conflicting digest is rejected. A matching duplicate never dispatches: if the
record is terminal it returns the same result; otherwise it joins the existing
record and waits only to its own deadline for the terminal result. If still in
progress, it returns terminal `outcome-unknown` with the lookup state/context.
On restart or connection loss, reserved/queued records become terminal definite
cancellation because no effect was dispatched; they are never requeued without a
new authenticated request. Dispatching/running records reconcile with a durable
handler result where available, otherwise become terminal unknown and are never
replayed. Result lookup exposes exactly these states.

Records remain queryable for 24 hours, bounded globally to 10,000 records and
32 MiB per server across all trust generations; generation remains an isolation
key, not a new quota. GC removes expired records and read caches first.
Admission atomically reserves capacity; if unavailable, it rejects before the
mutation record or effect rather than evicting live recoverability. Pair epoch/
trust change makes old records non-authorizing historical results and prevents
key reuse across trust.

Disconnect cancels undispatched work. Dispatched reads fail unavailable.
Dispatched mutations follow the durable states above; clients reconnect and use
result lookup or current revision, never automatic replay. A running record that
cannot establish a terminal result after bounded reconciliation becomes
terminal unknown.

### Liveness, reconnect, and ICE

After 15 seconds without valid inbound application traffic, a peer sends `ping`
with nonce and last inbound control sequence. Only validated inbound traffic or
a matching inbound `pong` resets that peer's receive-liveness timer; sending a
ping never does. Forty-five seconds without valid inbound traffic closes the
channel and triggers disconnect semantics. Ping/pong does not return bulk credit
or prove operation consumption.

ICE failure, DTLS/SCTP restart, browser reload, or Cockpit reconnection creates
one new peer connection, signaling grant, data channel, and authenticated
`HELLO`, with fresh IDs, credits, and streams. V1 does not migrate in-flight
requests or transfers across connections. Stable idempotency records and state
cursors provide bounded recovery. ICE restart never extends the old JWT lease
or reuses signaling fingerprint authorization.

### Failure handling and observability

Malformed JSON, impossible binary framing, credit underflow, ID collision,
sequence/offset violation, spoofed response, auth failure, or repeated unknown
frame closes the connection after at most one safe protocol error. A single
valid request/domain failure does not close it. Safe protocol errors expose a
stable code and correlation ID, never parser internals, tokens, payload snippets,
or unrestricted peer text.

Emit bounded redacted `litehouse.authn`, `authz`, and `management` tracing for
negotiation, lease, every authorization/effect boundary, admission, overload,
stream gap, transfer abort, cancellation, disconnect, result lookup, close, and
ICE lifecycle. Include safe connection/request/stream/transfer/operation,
principal/server/resource/action, pair epoch/trust generation, grant revision,
decision/reason, correlation/idempotency, outcome confidence, and byte/count/
latency fields. Never log JWTs, signaling descriptions, authorization headers,
raw request/config/state bodies, chunk bytes, or secrets. This is ordinary
tracing, not a second audit store.

Required validation includes native/browser golden JSON and binary vectors,
malformed/fuzz/property tests, no-common-major/feature tests, token expiry and
reauth, cross-server/transcript replay, every hard bound, credit starvation and
underflow, queue overload, chunk interleaving/digest/abort, cancellation at each
admission/dispatch point, disconnect before/after effects, idempotency conflict
and durable lookup across restart, retention exhaustion, stream replay/gap/
rehydration, heartbeat half-open closure, ICE reconnect, authorization on every
operation, and proof that HTTP and WebRTC adapters return semantically
equivalent typed results for the same generated operation fixture.
