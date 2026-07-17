# Plan runtime resilience closure

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-16
Blocked by: 05, 16, 17, 24, 28, 30, 33, 37
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which exact policy, tests, diagnostics, rollout gates, and explicit deferrals
close fail-closed HTTP authority matching and DNS policy, Samsung phase
deadlines/cancellation, bounded restart backoff, event payload forwarding,
output-discovery rejection, stale generations, queue bounds, traps,
cancellation, compatibility-cache invalidation, and rollback before v1?

## Answer

Close V1 around one supervised async actor and fresh Wasmtime store per configured
plugin instance/generation. Normal guest calls retain Component Model async
concurrency and overlapping I/O; the host does not serialize all provider work.
It bounds admission, starts calls in FIFO admission order, drives them
cooperatively through `run_concurrent`, propagates deadlines/cancellation, and
retires the whole generation when a call cannot be cancelled safely.

Replace the default WASI HTTP sender with one host-owned exact-origin connector.
Make traps, hard cancellation, unrecoverable resource destruction, output-
discovery rejection, and activation incompatibility explicit typed lifecycle
failures. Use fresh-generation restart with bounded automatic probes. Treat the
compiled module cache as disposable and compatibility-namespaced. Preserve the
already-fixed event payload forwarding and output-discovery rejection with
regression tests rather than redesigning them.

Samsung is not a V1 runtime gate after issue 37: its package is deferred and is
never published, installed by default, or executed against a user device. Its
phase-deadline work remains a Samsung re-entry condition, not a reason to weaken
or delay the supported provider runtime.

All limits below are safety/correctness bounds, not performance SLOs. They can be
revised only by a later runtime-profile release with conformance evidence; they
are not operator tuning knobs in V1.

## Async instance execution

### Actor and store ownership

- One configured instance owns one actor, fresh Wasmtime store, provider
  generation, endpoint set, mailbox, in-flight table, cancellation table, and
  lifecycle state.
- The V1 runtime profile accepts only the current per-instance sandbox strategy.
  Global/per-plugin shared-store modes fail configuration with a typed migration
  message; they cannot silently preserve serialized calls or cross-instance trap
  impact. Their source removal is implementation work, not a compatibility gate.
- Stores, guest resources, accessors, and resource-table handles never cross
  actors or generations. The Engine and verified compiled component bytes may be
  shared.
- Each instance has a durable `u64 next-runtime-generation`. Allocation increments
  and journals it before store construction; values are never reused and overflow
  disables the instance with `execution.protocol-failed`. A separate atomic
  current-runtime table holds instance, allocated generation, committed
  ActivationSet ID, config/grant revisions, and lifecycle state. It—not issue
  24's immutable activation-time generation—is endpoint/proxy/event routing
  authority.
- Construction, initialization, subscription declaration, output discovery,
  activation publication, quiesce, and resource destruction are exclusive
  lifecycle phases. Normal capability calls/events start only after publication.
- Normal guest calls may overlap. The host starts admitted work in FIFO order and
  permits at most **32 in-flight guest calls per instance**. Wasmtime cooperatively
  interleaves guest tasks and overlaps host I/O; this does not claim simultaneous
  mutable CPU execution across threads inside one store.
- The plugin owns coordination of its private connection/cache/state after calls
  start. Completion may be out of order. Contracts and provider code remain
  responsible for operation semantics; host admission order is not a physical
  transaction guarantee.
- Synchronous and asynchronous same-instance service-call cycles are rejected.
  Cross-instance cycles are bounded by the original request deadline and cycle
  detection; no lock is held across guest entry or guest-controlled await.

### Admission and queues

Each instance has:

- one ordinary FIFO mailbox with **64 waiting items**, excluding already
  dispatched calls;
- one cancellation token per admitted request, looked up by request ID;
- one capacity-one watch carrying the latest monotonic lifecycle/grant epoch for
  quiesce, shutdown, revocation, and hard teardown; and
- one in-flight table capped at 32.

There is no general-purpose unbounded control mailbox. Marking a known request's
cancellation token is O(1); duplicate cancellation is idempotent and an unknown
ID returns `binding.not-found`. Tokens are removed on completion. A bounded
terminal lookup cache retains at most 256 entries/128 KiB per instance and 4,096
entries/2 MiB process-wide for 60 seconds; each encoded entry is at most 512 bytes
and eviction is earliest terminal monotonic time then lexical request ID. An
evicted read/cancellation lookup is `binding.not-found`; durable mutation receipts
use issue 28's separate retention contract and are never evicted by this cache.
Lifecycle state coalesces by monotonic epoch, and the hard-teardown abort handle
does not wait behind guest work.

A direct request arriving when the mailbox is full receives typed `overloaded`
with bounded retry advice and is never enqueued. Existing work is never silently
dropped or displaced.

Typed subscription/event delivery uses the same admission accounting, a
monotonic `u64` delivery epoch, and a closed state machine:
`live -> gapped -> rehydrating -> live`. Every queued/in-flight event and every
host child call captures subscription ID plus delivery epoch.

On first rejected/lost event the host atomically stops dispatch for that
subscription, increments its epoch (overflow is generation-fatal), enters
`gapped`, removes and accounts for all queued old-epoch events, cancels all
in-flight old-epoch deliveries, and stores one capacity-one gap latch outside the
ordinary mailbox. The latch records source/entity/generation, first/last lost
sequence, and a saturating `u64` missed count plus `count-overflow`; later loss
updates that latch. Old handlers receive the ordinary two-second cancellation
grace. If any survives, the whole provider generation is retired before gap
recovery; no gap/resume proceeds in that store.

Before each guest dispatch, child service/effect call, post-`await` continuation,
and host-visible commit, the host checks captured subscription epoch as well as
provider generation. The scheduler delivers the latched gap only after every
old-epoch delivery is terminal, then enters `rehydrating`. The consumer obtains
the contract-defined current snapshot/cursor and calls `resume-subscription` with
subscription ID, delivery epoch, source generation, and cursor. The host resumes
`live` only if all still match and starts after that baseline; otherwise it
returns/stays gapped with an updated latch. A source with no replay cursor may
explicitly resume `at-now`, but the receipt says continuity was not recovered.
There is no implicit acknowledgement, and no queued, in-flight, or delayed
pre-gap event can dispatch or commit after gap delivery/resume.

A periodic timer has at most one pending tick. Missed ticks update the same
saturating count/overflow evidence rather than building backlog. One instance may
declare at most **64 subscriptions**, including at most **16 timers**; timer
periods are integral 1 second through 365 days. Zero, overflowing, duplicate,
malformed, or over-limit declarations reject activation.

Every admitted item carries request/event identity, authorization/grant revision,
provider/facet generation, enqueue time, absolute monotonic deadline, and reply or
gap context. Authorization and generation are checked at admission and again
immediately before dispatch. Stale or revoked work is removed with a definite
typed result.

The one authorization/admission lock from issue 28 orders cancellation, deadline,
grant revocation, generation withdrawal, and dispatch. For a mutation it durably
records `queued -> dispatching` with request, attempt, and generation **before**
guest entry or any child effect. Cancellation before that transition is definite;
at or after it is conservatively `outcome-unknown` unless the provider/host can
prove no effect was sent. Reads use the same ordering without a durable effect
record. Every continuation captures request/attempt/generation and fences again
after each `await` and immediately before publishing an endpoint/event/result,
writing state/cache, or completing a durable mutation. A late old-generation
completion is discarded as stale; it cannot commit through the current instance.

V1 also enforces process-wide safety limits: **64 configured/active instances**,
**1,024 waiting ordinary items**, **256 in-flight guest calls**, **128 concurrent
outbound host-I/O operations**, **four concurrent starts/probes/preflight stores**,
**one compilation**, and **eight concurrent teardowns**. Each store has at most
**64 MiB total guest linear memory** and **4,096 guest resource-table entries**;
a shared limiter caps all live guest linear memory at **256 MiB**.
Exceeding a config/memory/resource limit fails closed; transient global admission
returns `overloaded`. Start/probe work uses a process token bucket of four tokens,
refilling one token per second, with FIFO instance fairness; automatic and manual
probes consume the same bucket. These fixed V1 limits prevent per-instance limits from multiplying without a
process bound.

A component blob is rejected from its signed descriptor before download/read if
it exceeds **16 MiB**, and streaming verification never materializes more than
that. The host reserves 192 MiB of issue 30's 512 MiB envelope for its executable,
ordinary Rust heaps/stacks, and libraries; a shared 288 MiB dynamic-memory ledger
covers guest linear memory, resident compiled code (hard cap 128 MiB), mapped/
materialized component bytes, and cache codec buffers (hard cap 32 MiB resident).
An allocation that cannot reserve the ledger first fails with typed capacity.

Compilation runs only in one child invocation of the same verified binary. It
inherits only input/output/status file descriptors and an empty environment, sets
`no_new_privs`, applies a seccomp allowlist with no socket, path-open, exec, or
ptrace calls, and has no writable path except its already-open output descriptor.
Linux limits are `AS=192 MiB`, `FSIZE=32 MiB`, `CPU=60s`, `NOFILE=4`, `NPROC=1`,
and `CORE=0`; wall time is 60 seconds. Before launch the parent atomically
reserves 192 MiB of the 288 MiB dynamic ledger for the compiler's full lifetime
and proceeds only if existing charges fit in the remaining 96 MiB. Guest growth,
cache work, component materialization, and resident-code loading reserve that same
ledger, so they cannot cross 96 MiB until child exit. Parent reserve + allowed
parent dynamic + child address space is therefore at most 480 MiB with 32 MiB
cgroup headroom. The reservation is released only after child reap and bounded-
output cleanup. The parent checks output fingerprint/digest before loading and
separately charges resident code to the restored ledger. Waiting 30 seconds for compilation capacity then
fails activation without affecting running instances. The release fixture proves
these limits with the largest accepted blob and generated code; V1 has no
unbounded in-process compilation fallback.

### Deadlines

- Each generated operation declares a default and maximum deadline. Callers may
  shorten but never lengthen it.
- Ordinary provider operations have a host hard ceiling of **30 seconds** from
  admission. Queue wait and all nested service/host I/O consume the same
  remaining deadline.
- Construction/initialization has a 30-second bound; subscription declaration and
  output discovery each have a 10-second bound; cooperative quiesce plus explicit
  guest resource destruction has a 5-second bound.
- A provider-specific contract may set a lower deadline or stage bound. Bulk
  registry/image work is not an ordinary provider call and keeps its separately
  defined transfer contract.
- CPU epoch/fuel interruption and every host I/O deadline are both required: an
  epoch cannot wake blocked I/O, and an I/O timeout cannot preempt CPU-bound Wasm.

These ceilings define failure behavior, not responsiveness promises.

## Cancellation and hard teardown

Queued cancellation removes the item before dispatch and returns definite
`cancelled` with origin. Once dispatch starts, cancellation is advisory:

1. under the admission lock, mark the request token cancelled and classify its
   durable dispatch state;
2. cancel every generation-qualified host child operation and prevent new
   child/service calls for that request;
3. continue driving `run_concurrent` for a **2-second cancellation grace** so the
   guest can unwind and explicitly drop resources; and
4. if the top-level call remains alive, stop admission, retire the entire store
   generation, and hard-drop it within the lifecycle teardown bound.

Dropping a `call_concurrent` future alone is never treated as cancellation. The
classification is exact:

| State at the ordered boundary | Read result | Mutation result |
|---|---|---|
| queued; caller cancellation wins | `timing.cancelled(caller)` | `timing.cancelled(caller)`, definite |
| queued; deadline wins | `timing.deadline-exceeded` | `timing.deadline-exceeded`, definite |
| queued; grant/generation withdrawal wins | corresponding authorization/binding error | same, definite |
| `dispatching` recorded; guest/child still running | timing/availability cause if confirmation remains possible | `outcome-unknown` unless no-send proof exists |
| typed provider rejection with verified no-send proof | forwarded contract/execution error | forwarded definite contract/execution error |
| trap/hard teardown after dispatch | `execution.provider-trapped` or `execution.protocol-failed` | `outcome-unknown` |
| new call while failed/restarting/shutting down | matching `availability` variant | matching `availability` variant |

Generation failure applies the cause-specific typed envelope to queued and read
work: `execution.provider-trapped` for a trap, `execution.protocol-failed` for a
runtime/protocol invariant, or `availability.unavailable` for lost infrastructure.
It never reports the slash-separated alternatives as an implementation choice.
No hard teardown claims external rollback. A durable mutation receipt preserves
the old request/attempt/generation and its ambiguity across restart, update, and
host-image rollback.

Shutdown follows the same path but first marks the process not ready, stops new
admission, and preserves durable queued rule/effect state under issues 29 and 30.
The process-level 30-second graceful window remains authoritative; instance
teardowns run concurrently within it and are force-dropped before process exit.

## Generation-fatal failure and restart

The following are generation-fatal:

- guest trap or canonical-ABI/runtime invariant failure;
- cancellation surviving the 2-second grace;
- unrecoverable resource destructor failure or teardown timeout;
- malformed provider lifecycle/output descriptors that invalidate the published
  endpoint set;
- store/event-loop loss; and
- host-detected instance invariant violation.

A normal typed provider/domain error is not generation-fatal unless its contract
classifies the connector/session as unrecoverable.

On fatal failure the host atomically:

1. stops admission and marks the instance failed/restarting;
2. withdraws every endpoint in that generation and makes old proxies/cursors
   stale;
3. classifies queued work, reads, and mutations by the ordered table above;
4. bounds resource destruction, then drops the store regardless of destructor
   success;
5. records one safe generation-failure event and restart state; and
6. allocates only a fresh durable runtime generation on the next probe—never
   reuses guest handles, endpoints, subscriptions, or in-memory connector state.

A probe's allocated generation remains journaled/unpublished until successful
lifecycle completion; publication atomically replaces that instance's current-
runtime row and endpoints. Every call/continuation checks that row. Failed probes
leave the prior failed row for health evidence and never make their generation
current.

Restart state keeps a consecutive failure ordinal and a deque of failure
instants. A generation-fatal death, failed initial start, or failed automatic or
manual restart probe increments the ordinal once and appends one instant; a
no-grant update preflight failure does not affect the running generation's
restart state. Before appending, instants older than 120 seconds are removed.
Ordinals 1–7 choose base delays `1s, 2s, 4s, 8s, 16s, 32s, 60s`; later ordinals
stay at 60 seconds. If appending produces five failures in the rolling window,
the instance enters sparse state and uses a 300-second base until a probe starts
successfully.

Jitter is reproducible. Hash the UTF-8 domain separator
`litehouse-restart-jitter-v1`, the canonical 16 UUID bytes of the instance, and
the big-endian `u64` failure ordinal with SHA-256. Interpret the first two digest
bytes as big-endian `u16`; `offset_bp = (value % 4001) - 2000`. The delay in
milliseconds is
`max(1, (base_ms * (10000 + offset_bp) + 5000) / 10000)` using checked integer
arithmetic. Schedule it from the recorded failure instant.

Only one start may be active for an instance. Every schedule change increments a
`u64` schedule epoch; a queued probe captures epoch and expected failure ordinal,
then rechecks both plus failed/no-active-start state after acquiring the global
token and instance lock. Any mismatch releases the token and exits without a
start.

An owner-authorized manual probe is rate-limited to once per 30 seconds, consumes
the process-wide start token, and cannot bypass artifact/config/grant checks. Its
request leaves the automatic schedule valid only while waiting. If it acquires
the instance start lease first, it increments schedule epoch before starting, so
an already queued automatic probe becomes stale. Failure increments the ordinal
and installs a new epoch/schedule from that failure; success increments the epoch
and cancels retries. If the automatic probe wins first, the manual request sees
an active/changed epoch and becomes a no-op rather than a second attempt.
Successful start exits sparse state and marks a healthy-reset deadline; **600
uninterrupted healthy seconds** clears the ordinal/deque. Failure before then
continues the old ordinal. There is no permanent circuit.

Provider failure never makes the Litehouse process unready when auth, listeners,
volume, and supervisors remain healthy. Health exposes failed/restarting/sparse-
probe/manual-probe state and next safe retry time. A successful probe publishes
the fresh generation atomically; failed probes remain unpublished and never
resurrect old handles.

## HTTP authority and DNS policy

The existing authority substring/port-stripping hook is rejected. V1 uses one
host connector that parses and authorizes exactly:

```text
(scheme: http | https,
 host: canonical DNS name | IPv4 bits | IPv6 bits,
 effective-port: u16)
```

A DNS name is 1–253 ASCII bytes and at least two dot-separated labels of 1–63
bytes after ASCII lowercasing. Each label's first and last byte is an ASCII
lowercase letter or digit; interior bytes are ASCII lowercase letters, digits, or
hyphens. V1 rejects trailing dots, empty labels, underscores, labels beginning
`xn--`, and all Unicode/percent-encoded host forms. This deliberately supports no
IDNA conversion.

The parser applies only default ports 80/443 and compares the complete tuple. It
rejects absent/empty authority, userinfo, URI fragments, wildcards/suffixes,
non-canonical IPv4, malformed/unbracketed IPv6, zone IDs, port zero/out-of-range,
unsupported schemes, proxy environment, and every redirect. Each request is
rechecked against the instance's exact active grant; config validation alone is
insufficient. Connection pools are keyed by tuple, grant revision, and provider
generation and are closed on revocation or generation change.

### Literal addresses

- Cleartext `http` is supported only for an exact IPv4 literal in RFC 1918
  `10/8`, `172.16/12`, or `192.168/16` and an explicit/default port declared by
  the provider profile. It never uses DNS, redirects, proxies, IPv6, CGNAT,
  public/link-local/multicast/broadcast/unspecified/loopback destinations, or
  authority changes. Tasmota's separately documented plaintext LAN risk remains
  visible.
- `https` to a literal address requires certificate validation for that exact IP
  SAN and the granted public/private class. Private IPv4 means only the RFC 1918
  prefixes above; private IPv6 means only RFC 4193 `fc00::/7`. Unsupported
  bracketed-IPv6 paths fail closed until the custom connector's IPv6 fixture
  passes.

### DNS names

DNS authorities are allowed only for `https`. The host—not the guest—resolves the
exact configured name for each new connection attempt within three seconds, with
at most eight CNAME links, 64 KiB DNS response bytes, and eight final addresses.
Empty, oversized, malformed, mixed-policy, multicast, link-local, unspecified,
loopback, or otherwise disallowed answer sets fail before connect.

Private addresses are exactly the RFC 1918/RFC 4193 sets above. A public address
is unicast outside those private sets and either is outside every prefix in the
IANA IPv4/IPv6 Special-Purpose Address Registries or is covered only by a
most-specific entry whose `Globally Reachable` property is true. A covering entry
with false/blank/ambiguous status denies it. The runtime profile vendors the
source XML snapshots and their SHA-256 digests and generates the classifier and
boundary vectors; release validation fails if generated bytes drift from those
locked snapshots. Every answer must fit the grant's one declared class; one bad
answer rejects the whole set.

The connector chooses only from that validated set and connects directly to the
selected address bytes—never a second name lookup—while preserving the original
DNS name for HTTP `Host` and WebPKI SNI/certificate verification. TLS handshake
has at most five seconds, eight certificates, 64 KiB per certificate, and 256 KiB
for the chain, all inside the request deadline. Re-resolution may produce a
changed address set only through a new connection; successful WebPKI proof for
the same exact DNS name plus the same address-class policy permits the new set.
Cleartext DNS connectors and arbitrary resolver authority are unsupported.

An HTTP-only guest receives no inherited environment, DNS/name-lookup API,
resolved addresses, raw TCP/UDP grant, alternate authority, redirect decision,
or unrestricted WASI socket path. A separate exact `tcp-client` grant remains
possible only for contracts that declare it; it is never implied by an HTTP
grant.

Tests cover default/explicit ports, every DNS grammar edge, canonical IPv4,
bracketed IPv6, malformed/userinfo/percent forms, absent authority, every vendored
IANA prefix boundary, public/private class changes, mixed answers, CNAME/byte/
answer bounds, rebinding between resolution/connect, TLS name/IP and certificate
bounds, pool invalidation, redirects, and exact request destination recording.
Malicious guest fixtures prove an HTTP-only grant cannot inherit environment,
resolve names itself, open raw sockets, or bypass the connector.

The public `crates/plugin/src/http.rs` helper whose request path is `todo!()` is
not a supported V1 path. The native-async SDK tranche removes that helper and its
legacy client dependency; generated WASI HTTP bindings are the only guest HTTP
surface. A compiled guest fixture must complete a request through the connector
without panic before any first-party HTTP provider activates.

## Events, outputs, generations, and traps

The native-async migration already forwards the real broadcast payload and
propagates output-discovery rejection. Closure requires regression tests proving:

- non-default payload variants/fields survive host-to-guest delivery unchanged;
- host-generated event ID, received time, source identity, and generation are
  non-placeholder and remain stable for retransmission;
- lag/overflow produces an explicit scoped gap rather than silent continuation;
- a guest output-discovery error aborts preflight/activation and publishes no
  endpoint;
- output discovery permits at most **256 descriptors**, **64 KiB canonical encoded
  bytes total**, and **256 UTF-8 bytes per identifier/label**; malformed,
  duplicate, or over-limit descriptors reject the whole generation before
  publication; and
- no startup/output-discovery path performs a device control action.

The current transitional `id=0`/`timestamp=0`, immediate in-place trap restart,
`expect`/`unwrap` lifecycle assumptions, and endpoint-less logging are not V1
behavior. Runtime/component errors become typed envelopes; no guest-controlled
failure panics the process. Traps withdraw the complete old generation before a
candidate restart is visible.

## Compilation and compatibility cache

Compiled Wasm cache bytes are non-authoritative and never required for startup,
offline operation, activation, update, or rollback.

Replace the current unversioned panic-on-corruption file with a checksummed
envelope containing cache-format version, target, Wasmtime/Cranelift compatibility
fingerprint, Component Model async/runtime-profile ID, complete host-WIT/adapter
digest, and entries keyed by exact verified component bytes plus engine key.

The V1 cache profile is exact: **64 MiB maximum compressed file**, **256 MiB
maximum decoded envelope**, **4,096 entries**, **256 bytes per key**, **32 MiB per
compiled value**, and 256 MiB total decoded entry bytes. Size fields use checked
`u64` arithmetic and are verified before allocation; decoding is streaming and
aborts at the decoded-byte cap. At most four namespaces and 320 MiB total cache
disk usage—including one temporary file—are allowed. Crossing the 256 MiB high
watermark evicts least-recently-used non-current entries/namespaces to 192 MiB;
ties use lexical namespace/key order. The current namespace may lose LRU entries
but not its file while open. Compilation uses the process-wide single helper and
memory ledger.

Unknown version, checksum/decompression/decode failure, impossible size/count,
fingerprint mismatch, unreadable path, or other cache I/O failure records one
coalesced warning, disables/ignores the affected cache, and compiles from verified
component bytes; it never fails readiness. Corrupt bytes are not quarantined or
preserved. The next successful write replaces them.

Writes hold the cache lock and use one same-directory mode-0600 temporary file,
complete bounded encoding, checksum, file flush/fsync, atomic rename, then parent-
directory fsync. After taking the exclusive startup lock, the host removes any
orphan temporary file before loading. Interrupted writes leave the previous valid
cache or no cache; write/delete/fsync failure disables further writes for that
process without affecting execution.

A host WIT, WASI runtime profile, Preview adapter, Wasmtime/Cranelift,
architecture, compilation setting, or component-byte change creates a new cache
namespace. Native-async host and rebuilt components roll out together as one
runtime profile. An old image/plugin rollback may ignore and recompile newer
cache data; cache state never forces a compatibility bridge or blocks the one
retained predecessor.

## Plugin update, activation, and rollback

The pointer is issue 24's one global `ActivationSet`, not a per-instance pointer.
One activation transaction covers the complete affected-instance diff, sorted by
canonical instance UUID; a one-instance update is the degenerate case. The
ordering below supersedes conflicting candidate-before-predecessor wording in
issues 24 and 44 and permits no old/candidate device-session overlap for the same
instance:

1. verify every exact signed artifact, runtime profile, contract, config, grant,
   and candidate `ActivationSet` entry;
2. for every affected candidate under the global start cap, use a disposable
   preflight store with **no environment, network, service, timer, or event
   grants** to link/instantiate and validate static lifecycle/output descriptors,
   then drop it; any attempted I/O fails the entire preflight before disruption;
3. acquire the global activation lock and durably journal prior/candidate
   `ActivationSet` IDs plus every affected instance's prior/desired digest,
   allocated runtime generation, and phase while the old global pointer remains
   authoritative;
4. stop admission for all affected predecessors, preserve durable mutation
   receipts, quiesce/withdraw/drop each under teardown bounds, and record each
   phase; unaffected actors continue;
5. start one granted candidate per affected instance under global limits, run
   bounded lifecycle/connector hydration, and keep endpoints hidden; new installs
   have no predecessor, while replacement candidates start only after their own
   predecessor is gone;
6. after **all** candidates pass identity/duplicate/facet/binding compatibility,
   revalidate unchanged entries' immutable artifact/config/grant identity, then
   atomically switch the single global pointer, install only affected instances'
   current-runtime rows, and publish all candidate endpoints; mark the batch
   journal complete.

There is no partial pointer commit or mixed activation set. Unaffected actors may
restart during the batch: they allocate/update only their separate current-runtime
row, and pointer commit neither copies nor overwrites that row. Their immutable
ActivationSet entry and historical activation-time generation remain unchanged.
Affected actors cannot restart outside the batch while its lock is held. Preflight may coexist
with old actors because it has no real grants or generation. Before pointer
commit, any candidate failure drops every started candidate and restarts every
journaled predecessor; additions return to inactive. A crash at any pre-commit
phase uses the still-old pointer and journal to do the same deterministic restore.
A crash after commit uses only the complete candidate set. Failed predecessor
restart is visible and follows normal backoff but never causes a candidate pointer
commit. Fixtures cover two- and 64-instance failure/crash at every per-instance
and pointer boundary.

A later runtime failure follows normal restart and does not silently roll back
versions. Explicit **plugin-set** rollback runs the same all-or-nothing transaction
with retained verified predecessors and works without compilation cache. **Host
image/data rollback is not this pointer transaction**: it follows issue 30's
operator-selected signed image, stopped volume copy, N-1 compatibility, readiness
check, and pre-restore receipt guard. A matching-data restore is prohibited while
any post-copy `outcome-unknown` receipt exists or cannot be checked. Neither
rollback claims to undo external effects; ambiguity is preserved rather than
silently rewound.

Old proxies/cursors always fail stale after pointer/generation change. Same-
identity rebinding creates new proxies only under the existing binding policy;
labels, list position, or matching contract alone never retargets one.

## Samsung disposition

Samsung remains source-only deferred transport evidence under issue 37. It is
excluded from signed V1 registry publication, default builds/images, release
claims, real-device checks, and runtime readiness. Its current scheduled fixed
app-launch behavior must never run in V1.

Re-entry requires its own approved typed TV/media contract, removal of all
startup/scheduled control side effects, certificate enrollment/pinning decision,
and phase-specific connect, TCP, TLS, WebSocket handshake, first-frame, command,
response, shutdown, cancellation, and ambiguous-outcome tests. This explicit
defer closes rather than implements the Samsung portion of this ticket.

## Diagnostics and observability

Emit structured events for admission/overload, queue wait/deadline, soft/hard
cancellation, trap/destructor failure, generation withdrawal, restart/backoff/
sparse/manual probe, preflight/activation, endpoint publication, gap, authority/
DNS denial category, cache hit/miss/invalidation/corruption, pointer commit, and
rollback.

The process diagnostic limiter permits 100 events/second with burst 200. A noisy
category+instance emits at most one detailed event per 10 seconds; suppressed
counts are coalesced into one summary per minute. Lifecycle state transitions and
durable mutation receipts are not discarded, but repeated render/log events for
them are coalesced. Request/error/correlation IDs are fields, never metric-label
keys. Exported sinks receive a host-keyed pseudonym for server/instance/facet IDs;
raw stable IDs remain only in owner-protected local diagnostics.

Safe fields are pseudonymous server/plugin/instance/facet/generation/request/
error/correlation IDs, operation/phase, queue and in-flight counts, deadline
class, retry state, origin category (never private full authority), cache namespace
digest, typed outcome, and suppression count. Never log credentials, environment,
raw config, private addresses, resolved DNS answers, request/response/event
payloads, guest error text, component bytes, or unrestricted backtraces. Existing
tracing remains the only operational record; no audit database is introduced.

## Required validation matrix

| Area | Stable seam | Required evidence |
|---|---|---|
| Async execution | deterministic concurrent guest fixture | per-instance mode required; shared modes rejected; FIFO start, overlapping I/O, out-of-order completion, 32/64 local and 256/1,024 global limits, exclusive lifecycle, cycle rejection |
| Aggregate overload | fake 64-instance common outage/flood | dynamic ledger, held 192-MiB compiler reservation during concurrent guest growth, 16-MiB component, isolated compiler, resident-code/resource/I/O/start/probe/teardown caps; terminal-cache eviction; FIFO tokens; bounded shutdown |
| Events/outputs | mailbox, subscription, and generated ABI fixtures | 64/16 subscription/timer and 256/64-KiB descriptor bounds; exact live/gapped/rehydrating order, reserved latch, delivery-epoch cancellation/fatal escalation, queued/in-flight old completion after snapshot/resume, `u64` saturation, invalid/stale/at-now resume; real payload/ID/time/generation; no startup control |
| Deadlines/cancel | controllable CPU/I/O plus durable mutation fixture | queue-inclusive expiry, every cancellation/deadline/revocation/dispatch race, pre-effect `dispatching`, child cancellation, epoch interruption, two-second grace, exact typed result/ambiguity |
| Generation fence | delayed host-I/O/service/callback and crash fixture | durable non-reused generation allocation; old completion cannot commit; current-runtime row is routing authority; failed allocation stays hidden; counter overflow disables safely |
| Traps/restart | trap/destructor/startup fixtures with fake time | endpoint withdrawal, exact SHA-256 jitter vectors/rounding, failure ordinal/window/epoch transitions, queued-auto versus manual failure/success races after token acquisition, 600-second reset, isolation |
| HTTP authority | pure parser plus fake resolver/TLS and malicious guest | every grammar/IANA boundary, DNS/CNAME/byte/TLS bound, direct-address connect, SNI/IP-SAN, pool invalidation, rebinding, redirect, absent authority, no env/DNS/socket bypass, end-to-end generated guest request |
| Cache | byte/corruption/crash/compile-storm fixtures | every numeric boundary, decompression bomb, version/fingerprint invalidation, LRU tie, namespace churn, unreadable path, orphan temp, atomic fsync/rename interruption, empty-cache startup/rollback |
| Plugin-set update | 2- and 64-instance preflight/quiesce/batch-journal crash matrix | no-grant preflight, no same-instance dual session, each crash, all-candidate abort/all-predecessor restore, one global pointer plus affected runtime rows, concurrent unaffected restart not overwritten, set rollback without cache |
| Host rollback | issue-30 stopped-volume fixture | matching signed image/data, N-1 behavior, receipt watermark, unreadable/unknown/post-copy-ambiguous restore denial, reconciled restore, cache ignored, global pointer revalidated |
| Diagnostics | fake event storm/export sink | 100/s burst-200 bound, per-category coalescing/suppression count, no high-cardinality labels, pseudonyms, forbidden-field scan |
| Repository completion | current Rust workspace/CI seams | targeted tests and diagnostics; generated WIT/artifact checks; `mise exec -- cargo fmt --all --check`; `mise exec -- taplo fmt --check`; `mise exec -- cargo groups test bin`; `mise exec -- cargo build --release -p litehouse` |

Use deterministic fake time, fake DNS/TLS, fixture components, dummy credentials,
and isolated data directories. Canonical checks require no production API,
server mutation, household device, or external credential. Supervised ESPHome
hardware proof remains separately authorized and does not replace fixtures.

## Rollout gates

1. Land pure authority parser/connector fixtures and cache envelope tests without
   changing active provider behavior.
2. Land actor/mailbox/deadline/cancellation/restart fixtures behind a development
   runtime profile.
3. Ship the host plus generated contracts/adapters in the issue-30 image and
   publish first-party components separately under their signed registry locks;
   the release manifest pins one mutually compatible runtime profile, and any
   separately installed component with another profile fails before activation.
4. Run preflight/quiesce/activation-journal/plugin-rollback and issue-30 host-
   rollback/container-shutdown matrices in an isolated environment.
5. Repeat the approved supervised ESPHome proof only for behaviors that fixtures
   cannot establish, without broadening hardware access.
6. Make the profile mandatory only after supported providers, API/UI/MCP error
   serialization, and rollback evidence agree.

Plugin-set rollback reactivates the retained affected artifacts through the batch
transaction above. Host/runtime-profile rollback uses issue 30's stopped-volume
procedure and its matching prior image/components, blocks destructive restore on
unreadable or unresolved post-copy ambiguity, ignores the newer compilation
cache, and then validates the global activation pointer. Neither path can undo
external effects; ambiguity is retained or rollback stops.

## Explicit deferrals

- Samsung implementation and TV/media contracts.
- CPU-parallel mutable execution inside one Wasmtime store; V1 concurrency is
  cooperative async with overlapping host I/O.
- Operator-configurable mailbox/concurrency/deadline/backoff tuning.
- Process-per-plugin isolation and automatic container restart by Litehouse.
- Durable event replay/exactly-once side effects.
- Generic DNS, cleartext hostname, redirect, proxy, wildcard/suffix, CIDR/range,
  multicast/broadcast, or arbitrary resolver grants.
- Runtime benchmarking, latency/memory SLOs, and optimization work without a
  demonstrated regression.

## Implementation boundary

This decision authorizes no code change, package deletion, server mutation,
hardware access, publication, deployment, or commit. Implement it only through
separately approved cohesive tranches with targeted validation and final
scope/secret review.
