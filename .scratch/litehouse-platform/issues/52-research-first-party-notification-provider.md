# Research first-party notification provider

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-16
Blocked by: 18, 19, 35
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which narrow first-party provider should supply the already-decided
`notification-sink@1` bounded plain-text capability in V1? Compare at least
Twilio Messaging and ntfy for exact outbound authority, operator-injected
secrets, delivery truth, bounds, retries/idempotency, cost/external dependency,
local use, and implementation/test burden. Would an arbitrary webhook provider
violate Litehouse's no-generic-escape-hatch direction?

## Answer

**Ship one product-specific ntfy provider first; defer Twilio Messaging.** ntfy
is the smaller semantic and implementation match: one HTTP publish, native title
and priority, a documented 4,096-byte message limit, bearer tokens, per-topic
ACLs, hosted use, and an open-source self-hosted server. Its response still
proves only server acceptance, never handset receipt or presentation
([publish](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L1-L66),
[limits](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L4920-L4935),
[server path](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/server/server.go#L805-L942)).

Use one configured ntfy sink per instance: one normalized origin, one fixed topic,
one operator-injected `ACCESS_TOKEN`, and one exact host/IP-and-port network
grant. Permit only `POST /<topic>` with a bounded UTF-8 body and optional bounded
title/priority; disable redirects. Advertise only `accepted-for-delivery` and do
not advertise expiry, idempotency, target acknowledgement, delivery, read, or
presentation.

Twilio is a credible later **SMS-specific** provider where reach to an ordinary
phone number justifies a paid account, sender provisioning/regulatory work, SMS
segmentation, and optional delivery-status integration. It is not the narrowest
first proof of `notification-sink@1`.

## Comparison

| Concern | Twilio Programmable Messaging (SMS) | ntfy | Litehouse fit |
|---|---|---|---|
| Exact outbound authority | One fixed Twilio origin and `POST /2010-04-01/Accounts/{AccountSid}/Messages.json`; sender, recipient, and content are required ([Message resource](https://www.twilio.com/docs/messaging/api/message-resource#create-a-message-resource)). | `POST`/`PUT` one topic path on hosted or configured self-hosted origin ([publishing](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L1-L66)). | Both fit only if origin and account/topic/destination are fixed instance config, never invocation input. Disable redirects so responses cannot widen authority. ntfy also permits an exact local origin. |
| Secrets and least privilege | Twilio recommends Basic auth with API-key SID/secret. Main is full access, Standard is broad, and Restricted keys can grant endpoint/action permissions ([auth](https://www.twilio.com/docs/usage/requests-to-twilio#using-api-keys-recommended), [Restricted keys](https://www.twilio.com/docs/iam/api-keys/restricted-api-keys)). | Bearer tokens authenticate a user; ACLs can make a dedicated user write-only on one topic. Private servers should change default access from `read-write` to `deny-all` ([tokens/ACL](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/config.md#L620-L636), [private instance](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/config.md#L892-L915)). | Inject Twilio key SID/secret or one ntfy token only through schema-marked environment fields. Keep non-secret IDs/origin/topic in bounded config; never put credentials in URLs, config, logs, or receipts. Use a Twilio Restricted create-message key or dedicated ntfy write-only user. |
| Acceptance and delivery truth | Create initially reports `queued`/`accepted`; `sent` means carrier acceptance; `delivered` means Twilio received carrier and, only where available, handset confirmation. SMS has no `read`; that is RCS/WhatsApp only ([statuses](https://www.twilio.com/docs/messaging/api/message-resource#message-status-values)). Later states require polling or signed callbacks ([tracking](https://www.twilio.com/docs/messaging/guides/track-outbound-message-status)). | ntfy publishes to connected subscribers but launches FCM/Web Push/upstream delivery asynchronously before returning JSON; downstream bridge failure is absent from the response ([publish handler](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/server/server.go#L805-L942), [async FCM](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/server/server.go#L1035-L1047)). | The synchronous receipt for either is at most `accepted-for-delivery`. Neither success means presented. Twilio can later supply carrier-derived receipts, but `delivered` still is not presentation truth. |
| Bounded plain text/features | `Body` is at most 1,600 characters. Beyond 160 GSM-7 or 70 UCS-2 characters, SMS is segmented, not universally reassembled, and charged per segment. SMS has no native title/priority ([create schema](https://www.twilio.com/docs/messaging/api/message-resource#create-a-message-resource)). | Message is at most 4,096 bytes; longer input becomes an attachment. Native title and five priorities exist ([limits](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L4920-L4935), [priority](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L438-L460)). | Validate before I/O. ntfy V1 caps body at 4,096 UTF-8 bytes and must reject, not convert to attachment. Expose only body/title/priority—not tags, links, actions, attachments, templates, email, calls, scheduling, or arbitrary headers. |
| Expiry | `ValidityPeriod` bounds time in Twilio's outgoing queue from 1 to 36,000 seconds ([pinned OpenAPI](https://github.com/twilio/twilio-oai/blob/1a9189c79a73781ddf45afcd0afd1f210742d68c/spec/yaml/twilio_api_v2010.yaml)). | Response `expires` is server-cache expiry; no per-message expiry input appears in the published parameters ([cache](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L4709-L4724), [parameters](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L4937-L4969)). | ntfy declares expiry unsupported; cache retention is not presentation expiry. |
| Idempotency/ambiguity | The official `CreateMessage` operation has no idempotency field/header. `I-Twilio-Idempotency-Token` instead identifies Twilio-to-customer **webhook** retries ([OpenAPI](https://github.com/twilio/twilio-oai/blob/1a9189c79a73781ddf45afcd0afd1f210742d68c/spec/yaml/twilio_api_v2010.yaml), [webhook overrides](https://www.twilio.com/docs/usage/webhooks/webhooks-connection-overrides)). | Each publish gets a random message ID. Reusing a sequence ID appends a new update event; it does not deduplicate the request/cache entry ([model](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/model/model.go#L22-L41), [sequences](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L3830-L3857)). | Advertise no idempotency. Preserve caller keys for correlation only. After possible dispatch, timeout/cancellation/response loss is `outcome-unknown`; never auto-replay. ntfy sequence IDs are not an idempotency substitute. |
| Rate limits | Sender/country MPS varies and accepted excess queues; separate API concurrency limits return 429, which Twilio says was not processed ([MPS/queues](https://www.twilio.com/docs/messaging/guides/best-practices-at-scale#determine-your-messaging-throughput-needs), [API 429](https://www.twilio.com/docs/usage/rest-api-best-practices#retry-with-exponential-backoff)). | Default token bucket starts at 60 requests and replenishes one per five seconds; ntfy.sh documents 250 messages/day and paid higher tiers ([limits](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md#L4920-L4935), [rate config](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/config.md#L1884-L1905)). | Map explicit 429 to `rate-limit`. Despite known-safe 429, retain the catalog's no-auto-retry rule for notifications. Queue time counts against the caller deadline. |
| Cost/local dependency | Hosted pay-as-you-go; price depends on destination/type/carrier, messages are charged per segment, and carrier/registration fees may apply ([pricing](https://www.twilio.com/en-us/sms/pricing/us)). There is no self-hosted Twilio Messaging server in this API offering. | Hosted free/paid tiers and installable open-source server ([FAQ](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/faq.md#L31-L53), [install](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/install.md#L1-L11)). Hosted Android uses FCM; self-hosted instant iOS requires an APNS/FCM-connected upstream or may be delayed for hours ([FCM/iOS](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/config.md#L1571-L1628)). | ntfy avoids a mandatory paid external account and permits genuine local stream/browser delivery, but “self-hostable” must not be presented as universally cloud-free mobile push. |
| Burden | Medium/high: form encoding, SMS encoding/segments/cost, sender/destination and Restricted-key setup, country rules, two kinds of rate limiting, and callbacks/polling for stronger receipts. | Low: one POST, few bounded headers, bearer auth, one JSON acknowledgement, and a runnable first-party server for integration tests. | ntfy is the better first conformance seam. Twilio callbacks add inbound/public authority, signature validation, dedup/order handling, persistence, and fixtures before adding V1 value. |

## Exact V1 ntfy profile

- Config: normalized `http`/`https` origin containing only scheme, host, and
  explicit/default port, with no userinfo/path beyond `/`, query, or fragment;
  host is at most 253 ASCII bytes. One fixed `[-_A-Za-z0-9]{1,64}` topic has no
  wildcard/range/alternate target.
- Secret: required instance `ACCESS_TOKEN`; send only in `Authorization: Bearer`.
  A self-hosted operator creates the server-side user/token and write-only ACL;
  Litehouse never manages them.
- Grant: exact configured host/IP and port. Reject all 3xx; never reauthorize or
  follow a redirect.
- Request: `POST /<topic>`, `Content-Type: text/plain; charset=utf-8`, body at
  most 4,096 UTF-8 bytes, optional title at most 80 Unicode scalars and 256 UTF-8
  bytes, and priority exactly `min`, `low`, `default`, `high`, or `max`. Total
  generated request headers are at most 2 KiB. No other ntfy feature.
- Bounds and deadlines: the access token is 1..1,024 UTF-8 bytes. The 2 KiB
  generated-request-header bound counts each serialized name, colon, space,
  value, CRLF, and the final CRLF, including `Host`, `Authorization`, title,
  priority, and content headers. Response headers are at most 8 KiB; response
  body is at most 16 KiB with JSON nesting at most 4 and duplicate keys rejected;
  accepted provider ID metadata is ASCII and at most 64 bytes. Use a 2-second
  connect bound and 8-second total bound, including queue wait. These are safety
  bounds, not performance SLOs.
- Receipt: only status 200 plus one well-formed bounded JSON object containing a
  non-empty provider message ID becomes `accepted-for-delivery`. Ignore bounded
  extra response fields. Acceptance is not target acknowledgement, handset
  delivery, read, or presentation.
- Definite failures: validation/config failure before I/O is rejection; DNS,
  TLS, or connect failure before any request byte is written is
  `provider-unavailable`; documented 400/401/403/413 responses are respectively
  request rejection, authentication/authorization failure, or size rejection;
  documented 429 is `rate-limit`. These responses never trigger an internal
  retry.
- Ambiguous failures: any timeout, cancellation, disconnect, or parse/bound
  failure after a request byte may have been written is `outcome-unknown`. So are
  all 2xx responses other than the exact valid 200 above, every 3xx (which is
  never followed), every 5xx, and any undocumented status because acceptance
  cannot be excluded through a proxy or partially failed server. No ambiguous
  result is replayed automatically.
- Privacy: never trace token, Authorization, topic URL, message/title, or
  unrestricted response/error body. Trace only bounded status/category,
  instance/correlation IDs, and accepted provider ID metadata.
- Transport: require HTTPS for public origins. Local cleartext HTTP is acceptable
  only if existing network policy permits the exact private authority; never send
  a credential over an untrusted network.

## Generic webhook disposition

**Yes. A generic arbitrary-webhook provider would violate Litehouse's
no-generic-escape-hatch direction.** Allowing arbitrary URL, method, headers,
body/template, redirect, or secret-bearing header would turn a typed
`notification-sink@1` grant into general network execution; generic HTTP success
also has no truthful notification-delivery meaning.

The acceptable distinction is product-specific: the ntfy adapter owns a closed
request/status mapping and each reviewed instance fixes one origin/topic. A
future notification product gets a separate typed adapter, not configuration of
a universal webhook engine.

## Validation gates

Use a deterministic fake HTTP server, dummy credentials, and optionally a pinned
local ntfy instance—never real phones or production endpoints. Cover exact
origin/path/method, auth redaction, every numeric request/response/deadline
boundary above, feature negotiation, exact valid 200, other 2xx, 3xx,
documented and undocumented 4xx, 429, 5xx, malformed/duplicate-key bodies,
disconnect before and after the first request byte, timeout/cancellation, zero
ambiguous retries, redirect/DNS escape attempts, ACL
denial, and contract conformance that never reports delivered/read/presented.

## Exact primary sources used

### Twilio

- [Message resource/API/statuses](https://www.twilio.com/docs/messaging/api/message-resource)
- [Outbound status tracking](https://www.twilio.com/docs/messaging/guides/track-outbound-message-status)
- [Authentication and API-key types](https://www.twilio.com/docs/usage/requests-to-twilio)
- [Restricted API keys](https://www.twilio.com/docs/iam/api-keys/restricted-api-keys)
- [Messaging throughput and queues](https://www.twilio.com/docs/messaging/guides/best-practices-at-scale)
- [REST API 429 guidance](https://www.twilio.com/docs/usage/rest-api-best-practices)
- [Webhook retries/idempotency-token scope](https://www.twilio.com/docs/usage/webhooks/webhooks-connection-overrides)
- [Official US SMS pricing](https://www.twilio.com/en-us/sms/pricing/us)
- [Twilio OpenAPI `CreateMessage`, commit `1a9189c`](https://github.com/twilio/twilio-oai/blob/1a9189c79a73781ddf45afcd0afd1f210742d68c/spec/yaml/twilio_api_v2010.yaml)

### ntfy

All source links are pinned to first-party commit
[`b55e78a`](https://github.com/binwiederhier/ntfy/commit/b55e78a918580ba5244f8bc2fa43fabead5933a2):

- [Publish/auth/cache/sequences/limits/parameters](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/publish.md)
- [ACL/tokens/rate limits/FCM/iOS](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/config.md)
- [Installation/self-hosting](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/install.md)
- [Self-hosting/Firebase/paid tiers FAQ](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/docs/faq.md)
- [Publish handler/downstream forwarding](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/server/server.go#L805-L942)
- [Message/sequence model](https://github.com/binwiederhier/ntfy/blob/b55e78a918580ba5244f8bc2fa43fabead5933a2/model/model.go#L22-L41)

Litehouse constraints came from [contract catalog](18-define-vendor-neutral-contract-catalog.md),
[authorization](19-define-service-and-management-authorization.md),
[secret lifecycle](35-define-secret-lifecycle-and-remediation.md), and the
[plugin portfolio question](37-decide-plugin-portfolio.md).
