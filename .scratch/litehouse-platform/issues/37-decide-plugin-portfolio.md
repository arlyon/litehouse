# Decide the plugin portfolio

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-16
Blocked by: 18, 24, 52, 53
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which roles should the existing Samsung, MQTT, Presence, Sonoff, Tasmota,
Weather, persistent-task, no-op, and untracked Twilio packages serve in v1—real
integration, conformance fixture, example, explicit defer, or removal—and which
first-party provider supplies `notification-sink@1` without making unrelated
skeletons prerequisites for the ESPHome path?

## Answer

V1 has four real, independently testable first-party providers: **ESPHome,
Tasmota HTTP, Open-Meteo current conditions, and ntfy**. Only these four are
signed registry releases and release gates. The no-op and persistent-task
packages become non-published development assets. Samsung is explicitly
deferred. Empty MQTT, Presence, Sonoff, and Twilio shells are removed in a
separately approved implementation tranche; no replacement, compatibility shim,
or silent config remapping is added.

This is intentionally broader than the minimum ESPHome slice, but it is not a
license to create a general integration framework. Every real provider owns one
closed protocol mapping into the already-decided exact-major contracts. There is
no arbitrary URL, webhook, MQTT-topic mapping, command template, user-defined
JSON path, plugin-supplied UI, LAN discovery, or generic network escape hatch.

### Portfolio disposition

| Package/provider | V1 role | Publication and release consequence |
|---|---|---|
| ESPHome | Real supported integration | Signed first-party registry release and primary connected-device proof |
| Tasmota HTTP | Real supported integration | Signed first-party registry release after the narrow contract in issue 53 passes |
| Open-Meteo | Real, explicitly non-commercial integration | Signed first-party registry release only with fixed free-endpoint acknowledgement and attribution gates |
| ntfy | Real supported notification integration | Signed first-party registry release supplying `notification-sink@1` |
| persistent-task | Conformance fixture | Test-only scheduler/actor-lifecycle component; never listed or installable from the registry |
| no-op | Authoring example and minimal ABI smoke fixture | Built in development checks, documented as non-functional, never listed as a user integration |
| Samsung | Explicit defer | Preserve transport evidence in source, but do not publish, install by default, market, or make it a V1 gate |
| MQTT | Removal | Delete the empty skeleton; a future typed MQTT design starts from a new decision rather than reviving a generic topic escape hatch |
| Presence | Removal | Delete the empty skeleton; Cockpit connection presence is not a device plugin, and household presence needs a future typed contract/privacy decision |
| Sonoff | Removal | Delete the empty skeleton; supported Sonoff hardware may use ESPHome or the narrow Tasmota provider when it actually runs those firmwares |
| Twilio | Removal | Delete the untracked ABI shell; a future SMS-specific provider requires its own approved contract and does not block ntfy |

Removal and relocation are future code changes, not authorized by this planning
decision. Git history remains the record for tracked skeletons. Existing local
configuration referencing a removed shell is rejected explicitly; Litehouse does
not reinterpret it as another provider.

## Real-provider boundaries

### ESPHome

ESPHome remains the primary physical-device proof. One explicitly configured
instance represents one node and owns one Noise session, cache, reconnect
lifecycle, topology, and deterministic identity. It publishes every supported
advertised facet among:

- `on-off@1`;
- `level-control@1`;
- `color-control@1`;
- `boolean-state@1`;
- `measurement@1`;
- `thermostat@1`;
- `window-covering@1`; and
- `momentary-action@1`.

ESPHome does not fabricate `notification-sink@1`; ntfy supplies that contract.
The provider follows issues 17, 18, 22, 43, and 44: no mDNS adoption, no nested
instances, no command acknowledgement claim, and no fabricated state, range, or
unit. Discovery-advertised metadata is authoritative except where issue 43
establishes a fixed protocol interpretation; specifically, ESPHome thermostat
transport temperatures remain Celsius when its unreliable unit field is absent.

### Tasmota HTTP

Issue 53 is normative for the V1 profile. One instance represents one explicitly
configured node at a literal private IPv4 address on port 80. It uses only HTTP
Basic authentication with fixed user `admin` and operator-injected
`WEB_PASSWORD`; it never puts credentials in a URL. This remains plaintext LAN
HTTP and must be described honestly rather than as a secure session.

The provider exposes:

- one independently identified `on-off@1` facet per reported power channel,
  supporting only absolute `set(false)` and `set(true)`; and
- only the closed `measurement@1` whitelist for temperature, relative humidity,
  power, voltage, current, frequency, and total energy with validated native
  units.

It uses fixed `Status`, `Status 2`, `Status 10`, `Template`, and indexed absolute
power commands only. There is no MQTT, discovery, hostname/custom port, redirect,
query credential, arbitrary command, Backlog, rule, OTA, light/color, cover,
climate, custom JSON mapping, or generic HTTP operation. A valid command response
is target/protocol acknowledgement, not observed physical completion. A possibly
dispatched ambiguous command is never replayed. Polling independently produces
source-reported observations and reconciles topology.

### Open-Meteo current conditions

Issue 53 is normative. One instance represents one configured WGS84 coordinate
and exposes four read-only `measurement@1` facets: temperature at 2 m, relative
humidity at 2 m, surface pressure, and wind speed at 10 m. The provider uses one
fixed WebPKI-validated authority and query shape, polls no more often than every
15 minutes, validates returned units, and labels values as best-match weather-
model conditions rather than local sensor readings.

V1 deliberately supports only Open-Meteo's free **non-commercial** endpoint.
Instance creation requires an explicit `free-noncommercial` operator
acknowledgement. Every UI, rule export, API response, MCP presentation, or other
surface that displays or exports these facets must render adjacent fixed
Open-Meteo and CC BY 4.0 attribution and indicate Litehouse normalization. If a
surface cannot guarantee that attribution, it cannot expose the facet and the
provider is not releasable.

Paid/commercial access is deferred because the documented API key is placed in a
URL query parameter, conflicting with Litehouse's secret boundary. There is no
custom endpoint, key field, forecast/history, arbitrary variable list,
geocoding, model selector, or generic HTTP operation.

### ntfy notification sink

Issue 52 is normative. One instance is one fixed ntfy origin and topic with an
operator-injected `ACCESS_TOKEN` and exact outbound authority. It implements only
bounded plain-text `notification-sink@1`: body, optional bounded title, and exact
priority. It rejects redirects and does not expose tags, links, actions,
attachments, templates, email, calls, schedules, custom headers, URLs, or topic
selection at invocation time.

Only the exact bounded status-200 response defined in issue 52 means
`accepted-for-delivery`. It never means subscriber receipt, handset delivery,
read, or presentation. ntfy provides no request idempotency, so issue 52's
ambiguous status/transport/parse outcomes become `outcome-unknown` and are never
retried automatically. A self-hosted server is supported through the same fixed
contract; Litehouse does not provision or manage ntfy users, ACLs, tokens, or
servers.

## Non-published development assets

### Persistent-task

Persistent-task becomes a deterministic conformance fixture for periodic
subscription admission, actor/store lifetime, cancellation, restart, and bounded
logging. It exposes no user capability, appears in no registry index, and is not
an integration prerequisite. The implementation may relocate it under a test
fixture directory if that simplifies release packaging.

### No-op

No-op remains the smallest Rust authoring example and an ABI/package smoke
fixture. Its documentation must say that it is intentionally non-functional. It
may prove component packaging, activation, and clean shutdown, but cannot count
as capability, provider, registry, or connected-device evidence.

## Samsung defer boundary

Samsung's existing exact-address TLS/WebSocket transport and tests remain useful
historical evidence, but the current plugin is not a product integration: it uses
the transitional ABI, exposes no typed facets, accepts any server certificate,
lacks complete deadlines/cancellation, and sends a fixed app-launch command from
a periodic update. It must never run against a user device as part of V1.

Reconsider Samsung only after all of the following have separate approval:

1. a typed TV/media contract outside the canonical nine-facet catalog;
2. zero startup, activation, scheduled, or reconnect control side effects;
3. bounded connect/TLS/WebSocket/first-frame/call/shutdown deadlines and
   cancellation tests;
4. explicit certificate enrollment/pinning or a newly approved residual-risk
   policy; and
5. deterministic fixtures plus supervised real-device proof.

## Release independence and ordering

The selected integrations share generated contracts, host execution,
authorization, registry, and conformance infrastructure, but not provider code or
activation. ESPHome must remain buildable, testable, installable, and provable
without compiling, configuring, or connecting Tasmota, Open-Meteo, ntfy,
Samsung, or any removed skeleton. The same independence applies to each other
real provider.

Implementation order is:

1. generated contracts, provider host, registry packaging, and shared
   conformance harness;
2. ESPHome primary vertical slice;
3. Tasmota typed HTTP provider;
4. ntfy notification provider; and
5. Open-Meteo provider plus attribution propagation.

All four selected providers must pass their gates before calling the broader V1
portfolio complete. A failure in one provider disables or withholds that
artifact; it must not crash the host or silently downgrade another provider.

## Validation gates

### Common gates

Every signed provider must prove the behavior it actually advertises, without
inventing inapplicable command, receipt, observation, topology, or reconnect
requirements:

- one explicit configured instance with no LAN discovery or child instances;
- closed config schema and annotations, deterministic environment-secret names
  where secrets exist, exact WASI/service grants, and redacted diagnostics;
- exhaustive exact-major feature advertisement and unsupported-feature
  rejection;
- safety-bounded request/response sizes, queues, deadlines, cancellation, and
  generation teardown for every operation or poll it performs;
- no startup/activation **control** action; bounded read-only activation probes
  are allowed only where the provider profile declares them;
- stable facet identity; only providers with dynamic topology must prove explicit
  incarnation changes on unprovable remaps;
- signed artifact publication, offline install, explicit update, failure
  isolation, and predecessor rollback; and
- deterministic canonical CI with dummy credentials and fixtures, never live
  production APIs or household hardware.

The applicability matrix is normative:

| Provider | Commands and receipts | Observations/freshness | Dynamic topology/reconnect | Ambiguity/replay gate |
|---|---|---|---|---|
| ESPHome | Dispatch-only desired operations; no device acknowledgement | Subscribed source observations with unknown/stale/unavailable quality | Entity relist and generation reconciliation | Possibly dispatched commands become ambiguous and are never replayed |
| Tasmota | Absolute power set with target/protocol acknowledgement only | Independent polls with stale/unavailable recovery | Power/sensor path reconciliation and routing-digest remap | Retry only when no request byte was sent; otherwise no replay |
| ntfy | Notification send with acceptance-only receipt | None | Static one-sink facet; transport reconnect is per request | Every issue-52 ambiguous outcome is terminal and never replayed |
| Open-Meteo | None | Four polled model observations with sibling isolation and stale/unavailable recovery | Static four-facet topology; no connector session | Not applicable because it has no mutation |

Command providers test rejection, timeout before dispatch, possible-dispatch
ambiguity, and receipt truth. Observation providers test missing/invalid values,
freshness transitions, and recovery. Dynamic-topology providers test reconnect,
addition/removal, stable identity, and unprovable remap. All providers test
success, malformed/oversized upstream data where they parse it, authorization or
upstream denial, cancellation at applicable boundaries, and redaction.

### Provider-specific gates

- ESPHome must retain deterministic fixtures for every advertised family. The
  supervised Node S proof from issue 22 covers four switches, three binary
  sensors, initial hydration, topology, stable identity, and reconnect only.
  Measurement, Level, and Color remain fixture-proven; no real relay command or
  air-quality-node proof is claimed or required without separate approval.
- Tasmota must pass the exact authority/auth/request, multi-channel identity,
  closed measurement, custom state-text, topology-remap, command ambiguity, and
  two-cycle stale/unavailable fixtures in issue 53.
- ntfy must pass fixed origin/topic, bearer redaction, every issue-52 numeric
  body/title/header/token/origin/response/ID/deadline boundary, exact valid 200,
  other 2xx, 3xx, documented/undocumented 4xx, 429, 5xx, disconnect before/after
  first request byte, malformed/duplicate-key response, and zero-ambiguous-retry
  fixtures.
- Open-Meteo must pass fixed authority/query, four-field/unit/source-time,
  missing-sibling isolation, model provenance, rate-budget, non-commercial
  acknowledgement, and adjacent attribution tests across UI/API/MCP/export seams
  in issue 53.
- Persistent-task and no-op pass only fixture/example checks and cannot satisfy a
  real-provider gate.

## Deferred triggers

- A future MQTT integration requires a new typed mapping and authority decision;
  arbitrary topic/payload templates remain forbidden.
- Household/device presence requires a privacy model and typed contract distinct
  from Cockpit signaling presence.
- Native Sonoff support requires a specific protocol/product decision; firmware
  brand alone is not a contract.
- Twilio requires a dedicated SMS contract, fixed sender/destination semantics,
  restricted credentials, cost/segmentation/rate behavior, and optional status
  callback design.
- Commercial weather requires a provider/authentication design compatible with
  the no-secrets-in-URLs boundary or an explicit revision of that boundary.

## Evidence

- [Vendor-neutral contract catalog](18-define-vendor-neutral-contract-catalog.md)
- [Provider execution and ESPHome lifecycle](17-define-provider-execution-and-esphome-lifecycle.md)
- [ESPHome capability semantics](43-research-esphome-capability-semantics.md)
- [ESPHome prototype](22-prototype-real-esphome-capabilities.md)
- [Explicit instance configuration; no discovery](44-define-plugin-driven-instance-discovery.md)
- [Notification-provider research](52-research-first-party-notification-provider.md)
- [Tasmota/Open-Meteo research](53-research-tasmota-and-weather-provider-contracts.md)
