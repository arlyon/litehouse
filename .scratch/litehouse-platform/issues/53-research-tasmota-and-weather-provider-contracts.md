# Research V1 Tasmota and Open-Meteo provider contracts

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-16
Blocked by: 17, 18, 19, 35, 43, 44
Map: [Find the route to Litehouse's first connected release](../map.md)

## Decision

Ship two deliberately narrow first-party providers:

- **Tasmota HTTP:** one explicitly configured Tasmota node, exposing independently addressed `on-off@1` power-channel facets plus a closed, conservative subset of `measurement@1` observations. Do not use MQTT, mDNS, device scanning, arbitrary Tasmota commands, or user-defined JSON mappings in V1.
- **Open-Meteo current conditions:** one explicitly configured coordinate, exposing four read-only `measurement@1` facets. V1 supports only the free **non-commercial** endpoint. Paid access is deferred because Open-Meteo documents its API key as a query parameter, while Litehouse forbids secrets in URLs.[OM-P][LH35]

This fits Litehouse's one-instance/one-physical-device rule for Tasmota, explicit instance creation, exact-major facets, per-instance grants, environment-only secrets, bounded work, and honest state/receipt semantics.[LH17][LH18][LH19][LH35][LH43][LH44]

## Common provider boundary

| Concern | Tasmota instance | Open-Meteo instance |
|---|---|---|
| Instance subject | Exactly one physical node | Exactly one WGS84 coordinate |
| Provides | `on-off@1` and `measurement@1` | `measurement@1` only |
| Consumes | No service contracts | No service contracts |
| Network grant | Exact configured literal unicast IPv4 and TCP port 80 | DNS/TLS only for `api.open-meteo.com:443` |
| Discovery | Facets inside the configured node only | None; fields are fixed |
| Secrets | Required schema field token `WEB_PASSWORD`; injected only as `LITEHOUSE_INSTANCE_<UUID32>_WEB_PASSWORD` | None in supported V1 mode |
| Generic HTTP | None: no URL, path, method, header, query, or command passthrough | None: one hard-coded endpoint and query shape |
| Redirects | Disabled | Disabled |

Both providers use one in-flight request at a time, reject duplicate JSON object keys, cap a response at 64 KiB and nesting at 8, require finite numbers, and never publish unknown source fields. Queue wait counts against the caller deadline. Polls are not retried immediately; the next scheduled poll is the retry. A missed interval is skipped rather than accumulated. The WASI grant confines authority, not HTTP paths; therefore the signed first-party code must contain only the fixed request builders below, and conformance must record outbound requests and reject every other method/path/command. No consumer receives an HTTP operation.

## Tasmota V1

### Configuration, activation, and grants

Closed config:

```text
address: literal RFC1918 IPv4 (required; persisted privately and redacted)
poll_seconds: integer 10..300 (default 30)
```

`WEB_PASSWORD` is required, non-empty, and at most 32 bytes; the username is fixed to the official default `admin`. Unauthenticated nodes and custom-build usernames are out of V1. Tasmota's public command docs show legacy plaintext query credentials, but current first-party firmware prefers HTTP Basic authentication and falls back to query parameters. Litehouse must use **only** the Basic `Authorization` header, mark it sensitive, and never construct the documented `user`/`password` query.[T-CMD][T-WEB]

The exact grant is TCP connect to the configured IP on port 80. Reject hostnames, public/loopback/link-local/multicast/broadcast addresses, port changes, redirects, proxy variables, and ambient DNS. This is confidentiality-weak Basic authentication over plaintext LAN HTTP; document that residual risk and do not describe it as a secure session. Supporting HTTPS, reverse proxies, MQTT brokers, or remote Tasmota is deferred.

Activation performs only fixed read commands through `GET /cm?cmnd=<encoded-fixed-command>`:

1. `Status 2` — record firmware diagnostics; conformance is tested against Tasmota 15.5.0.1 at source commit `61fe447`, but response-shape validation, not a broad version claim, is authoritative.[T-VER]
2. `Status` — obtain module, bounded friendly labels, the power bit-string, and power-channel count.
3. `Template` — digest module/template routing evidence.
4. `Status 10` — obtain sensor JSON.

Tasmota's default referer check can block HTTP API calls; current firmware itself tells operators to use `SetOption128 1` for HTTP API commands and recommends a web password. Activation must return a typed remediation code (`http-api-referer-blocked`) rather than changing device configuration.[T-WEB]

There is no trustworthy device identity proof: an HTTP-returned MAC or hostname is spoofable on this unauthenticated transport. Therefore the instance gets no duplicate-device or silent-retarget guarantee. Any address change is a config revision and hidden reactivation. Installation, validation, and startup never scan the LAN.

### Minimum capability surface and identity

**Power channels.** `Status.Power` is a binary string whose width is `devices_present`; channel 1 is the least-significant/rightmost bit. Current firmware emits indexed `POWER<n>` names whenever more than one channel exists, while `SetOption26` can make a single channel appear as `POWER1` instead of `POWER`.[T-STATUS-SRC][T-POWER-SRC][T-CMD]

Publish one independent facet per channel:

```text
facet-key: power/<1-based-index>
contract: on-off@1
operations: set(false), set(true)
quality: source-reported
primary composition group: power/<1-based-index>
label: FriendlyName[index-1] when present, otherwise "Power <index>"
```

Labels, response key spelling, and list position are not identity. The numeric source index is only a locator. Store the `Module` plus `Template` digest as routing evidence. If that evidence or channel count changes, retire and republish all power facets with new incarnations; never silently preserve a binding across an unprovable hardware remap. `toggle`, blink, timed power, backlog, interlock configuration, light/dimmer/color, rules, restart, OTA, and every other Tasmota command are not exposed.

**Measurements.** `Status 10`/`StatusSNS` is intentionally dynamic: official examples show multiple sensor objects and global unit fields, not a typed entity catalog.[T-STATUS][T-CMD] Publish only this closed table:

| Exact source shape | `measurement@1` profile/kind | Native unit |
|---|---|---|
| `<sensor>.Temperature` with top-level `TempUnit` exactly `C` or `F` | temperature / instantaneous | Cel or [degF] |
| `<sensor>.Humidity` | relative humidity / instantaneous | `%` |
| `ENERGY.Power` | power / instantaneous | W |
| `ENERGY.Voltage` | voltage / instantaneous | V |
| `ENERGY.Current` | electric current / instantaneous | A |
| `ENERGY.Frequency` | frequency / instantaneous | Hz |
| `ENERGY.Total` | energy / total | kW.h |

A scalar facet key is `measurement/<percent-encoded-case-sensitive-json-path>`; an array value gets one facet per 1-based element with `/<index>` appended (maximum eight elements). Thus two temperature sensor objects and multiple energy phases remain distinct. Object order is irrelevant. A changed path is removal plus addition, never label-based reconciliation. Missing, `null`, non-numeric, non-finite, unsupported-unit, unknown, array-length-overflow, and unlisted fields do not become measurements; emit only a bounded diagnostic code. Do not publish `Today`/`Yesterday` as monotonic energy, decimal display precision as resolution, or inferred ranges/tolerances.

### Command and observation truth

Poll `Status`, then `Status 10`, sequentially. Each request has a 2 s connect bound and 5 s total bound; a cycle has a 10 s total bound and ±10% startup jitter. No poll overlaps another poll or a command.

`set` emits exactly `Power<n> 0` or `Power<n> 1` (including `n=1`; never `Power0`). Tasmota documents indexed control for multi-relay devices and firmware generates a JSON power response from its internal power state.[T-CMD][T-POWER-SRC] Treat a well-formed HTTP 200 response containing the exact allowable power key (`POWER` or `POWER1` for a singleton; `POWER<n>` for multiple channels) and no command error as **protocol/target-acknowledged**, not completed or observed-completed. Tasmota permits custom `StateText`, so do not interpret response strings as universal `ON`/`OFF` values.[T-CMD] The next `Status.Power` is a separate source-reported observation and is never retroactively made causal.

The power bit represents Tasmota's internal/software state, not verified relay contacts; it is `source-reported`, never physical confirmation. A timeout or cancellation after request bytes may have reached the node returns `outcome-unknown` and is not retried. An absolute `set` may be retried only when the transport proves that no request byte was sent. A response that reports the opposite state (for example a locked output) is a target acknowledgement plus unchanged observation, not fabricated success.

### Tasmota errors

Map without returning raw bodies or credentials:

- 401 → `authentication-failed` (instance inactive);
- referer-block/empty early response → `http-api-referer-blocked`;
- connect refusal/timeout → `provider-unavailable` (or `outcome-unknown` after possible command dispatch);
- HTTP non-200 → `upstream-http-status` with bounded status only;
- `{"Command":"Error"}`, `Unknown`, missing expected envelope/key, malformed/duplicate-key/oversized JSON → `provider-protocol`;
- unsupported unit/path or facet cap → facet-local `unsupported-source-shape` diagnostic.

Keep last confirmed observations but mark them stale after one missed cycle and the instance unavailable after two consecutive failed cycles. A later success creates a new host revision; it does not fill a claimed replay gap.

## Open-Meteo V1

### Configuration, authority, and fixed request

Closed config:

```text
latitude: finite decimal -90..90, serialized to at most 6 fractional digits
longitude: finite decimal -180..180, serialized to at most 6 fractional digits
usage: literal "free-noncommercial" with explicit operator acknowledgement
poll_seconds: integer 900..3600 (default 900)
label: optional UTF-8, 1..80 Unicode scalars and at most 256 bytes
```

The free endpoint is non-commercial only; official terms explicitly include personal home automation but exclude integration into commercial products. It is limited to 600 calls/minute, 5,000/hour, **less than** 10,000/day, and the pricing table also lists 300,000/month.[OM-T][OM-P] A paid subscription uses `customer-api.open-meteo.com` and `&apikey=...`; because that would place a secret in a URL, paid/commercial mode must remain unavailable rather than leaking the key or guessing an undocumented header.[OM-P][LH35]

Issue exactly one HTTPS GET per cycle, with WebPKI validation, SNI `api.open-meteo.com`, no redirect, and this fixed query (canonical encoding/order is an implementation detail, not config):

```text
/v1/forecast
  ?latitude=<canonical-latitude>
  &longitude=<canonical-longitude>
  &current=temperature_2m,relative_humidity_2m,surface_pressure,wind_speed_10m
  &temperature_unit=celsius
  &wind_speed_unit=ms
  &timeformat=unixtime
  &timezone=GMT
```

Open-Meteo documents `/v1/forecast`, required WGS84 coordinates, fixed variable lists and unit controls; current conditions are based on 15-minute model data.[OM-API] There is no endpoint override, API-key field, model selector, multi-location request, forecast window, hourly/daily data, archive call, metadata call, or arbitrary variable selector.

A shared host budget keyed to the free authority should cap all instances at 500/minute, 4,000/hour, 9,000/day, and 270,000/month (deliberate safety margins below provider limits). With a 15-minute minimum interval, cap enabled free instances at 90 unless that shared limiter exists. Count activation calls and do not retry 429/5xx immediately. Use 2 s connect, 8 s total, one in-flight request, and 0–60 s startup jitter.

### Facets and observation truth

Publish four static, independent `measurement@1` facets, all instantaneous and `source-reported`:

| Facet key | API field | Profile | Required unit |
|---|---|---|---|
| `current/temperature-2m` | `temperature_2m` | temperature | °C |
| `current/relative-humidity-2m` | `relative_humidity_2m` | relative humidity | % |
| `current/surface-pressure` | `surface_pressure` | pressure | hPa |
| `current/wind-speed-10m` | `wind_speed_10m` | speed | m/s |

The official definitions identify these fields as instantaneous and define their units.[OM-API] The provider must validate any returned unit metadata when present and reject mismatches; it must not infer a missing field as zero. Preserve `current.time` as source valid-time and host receipt time separately. The returned latitude/longitude are the selected grid-cell centre and may differ by kilometres from the requested coordinate, so preserve them as bounded provenance, never as instance identity or proof of a physical sensor.[OM-API]

These are weather-model current conditions, not measurements from a local station. Add the built-in provenance tag `source/open-meteo-best-match-model`; do not claim sensor accuracy, resolution, update completion, or a stable underlying model. Open-Meteo combines and stitches models, selects the highest-resolution applicable model, updates models every few hours, and is eventually consistent across servers; it recommends waiting ten minutes after model availability for consistency.[OM-API][OM-U]

There are no commands or receipts. After one failed poll, retain values as stale; after two failed polls mark the instance unavailable. Missing/null/non-finite individual fields become unknown for that facet while valid siblings still publish.

### Licensing and attribution

Open-Meteo API data are CC BY 4.0. Every UI/export location that displays these facets must place adjacent links equivalent to **“Weather data by Open-Meteo.com (normalized by Litehouse)”** linking to `https://open-meteo.com/` and `https://creativecommons.org/licenses/by/4.0/`; “normalized by Litehouse” discloses unit/shape adaptation. Open-Meteo explicitly requires appropriate credit, a licence link, indication of changes, and a link next to displayed data.[OM-L]

Encode a non-removable built-in descriptor tag `attribution/open-meteo-cc-by-4.0`; renderer/export conformance must resolve it to the fixed attribution. If Litehouse cannot guarantee adjacent attribution across every rendering/export seam, the weather provider is not releasable. Raw API data need not be retained in V1.

### Open-Meteo errors

Open-Meteo documents HTTP 400 with `{"error":true,"reason":...}` for invalid parameters.[OM-API] Map 400 to `provider-request-rejected` with a closed reason code (never raw upstream text), 429 to `rate-limited` with bounded `Retry-After`, 401/403 to `unsupported-access-tier`, 5xx/TLS/connect/timeout to `provider-unavailable`, and malformed/missing/wrong-unit/duplicate-key/oversized success bodies to `provider-protocol`.

## Required deterministic fixtures

No fixture requires hardware or a live API.

**Tasmota:** Basic challenge/success/failure; referer-blocked response; single `POWER` and `POWER1`; 2/4/32 power bit-strings; custom `StateText`; correct and mismatched `Power<n>` command responses; command `Error`/`Unknown`; locked/interlocked side effects; module/template digest change; repeated temperature objects; C/F; scalar and array energy channels; reordered keys; missing/null/non-finite/unknown unit/path; duplicate keys; 64 KiB boundary; timeout before write versus after possible write; two-cycle stale/unavailable recovery.

**Open-Meteo:** four-field success with source time/grid coordinate; each field independently missing/null/non-finite; wrong units; returned coordinate offset; HTTP 400 error object; 429 with valid/invalid `Retry-After`; 5xx/TLS/timeout; malformed/duplicate-key/oversized JSON; stale/unavailable recovery; global-budget exhaustion; exact URL/authority snapshot; redirect rejection; and renderer/export tests proving adjacent attribution.

## Explicit defer boundary

- Tasmota MQTT/broker provisioning, TLS/reverse proxy, HTTP query credentials, unauthenticated nodes, hostnames/custom ports, mDNS/SSDP/scanning, arbitrary commands/Backlog, lights/color/dimmer, covers, climate, buttons/events, rules, OTA/configuration, user-defined JSON paths, automatic unit guessing, and trustworthy device identity.
- Open-Meteo paid/commercial API keys, custom/self-hosted endpoints, forecasts/hourly/daily/history, weather codes, precipitation windows, wind direction, apparent/dew-point temperature, model selection/update metadata, multiple coordinates, geocoding, and raw response retention.
- Any generic HTTP provider, URL template, header injection, query passthrough, webhook, or “advanced” escape hatch for either integration.

## Primary sources

### Litehouse decisions

- [LH17] [Issue 17: provider execution and lifecycle](17-define-provider-execution-and-esphome-lifecycle.md)
- [LH18] [Issue 18: exact-major contract catalog](18-define-vendor-neutral-contract-catalog.md)
- [LH19] [Issue 19: service and management authorization](19-define-service-and-management-authorization.md)
- [LH35] [Issue 35: environment-only secret lifecycle](35-define-secret-lifecycle-and-remediation.md)
- [LH43] [Issue 43: no fabricated capability semantics](43-research-esphome-capability-semantics.md)
- [LH44] [Issue 44: explicit instances; no LAN discovery](44-define-plugin-driven-instance-discovery.md)

### Tasmota first-party

- [T-CMD] [Official command docs](https://tasmota.github.io/docs/Commands/) and [pinned source (`Commands.md`)](https://github.com/tasmota/docs/blob/d8a3be10b670cedbeec395760b5da69979d990fa/docs/Commands.md)
- [T-STATUS] [Official JSON status-response docs](https://tasmota.github.io/docs/JSON-Status-Responses/) and [pinned source](https://github.com/tasmota/docs/blob/d8a3be10b670cedbeec395760b5da69979d990fa/docs/JSON-Status-Responses.md)
- [T-WEB] [Pinned first-party HTTP authentication/command handler](https://github.com/arendst/Tasmota/blob/61fe447df6b484f1530ea5a5eef437ae8ab63454/tasmota/tasmota_xdrv_driver/xdrv_01_9_webserver.ino#L765-L789) and [handler response logic](https://github.com/arendst/Tasmota/blob/61fe447df6b484f1530ea5a5eef437ae8ab63454/tasmota/tasmota_xdrv_driver/xdrv_01_9_webserver.ino#L3883-L3942)
- [T-STATUS-SRC] [Pinned `Status`/`StatusSNS` implementation](https://github.com/arendst/Tasmota/blob/61fe447df6b484f1530ea5a5eef437ae8ab63454/tasmota/tasmota_support/support_command.ino#L895-L968) and [sensor status](https://github.com/arendst/Tasmota/blob/61fe447df6b484f1530ea5a5eef437ae8ab63454/tasmota/tasmota_support/support_command.ino#L1163-L1176)
- [T-POWER-SRC] [Pinned power command implementation](https://github.com/arendst/Tasmota/blob/61fe447df6b484f1530ea5a5eef437ae8ab63454/tasmota/tasmota_support/support_command.ino#L715-L735) and [state/response implementation](https://github.com/arendst/Tasmota/blob/61fe447df6b484f1530ea5a5eef437ae8ab63454/tasmota/tasmota_support/support_tasmota.ino#L736-L841)
- [T-VER] [Pinned Tasmota version](https://github.com/arendst/Tasmota/blob/61fe447df6b484f1530ea5a5eef437ae8ab63454/tasmota/include/tasmota_version.h#L20-L27)

### Open-Meteo first-party

- [OM-API] [Official Weather Forecast API docs](https://open-meteo.com/en/docs) and [pinned website source](https://github.com/open-meteo/open-meteo-website/blob/1d06b9157387c0b7ac1ee468d6168536f2dda493/src/routes/en/docs/%2Bpage.svelte)
- [OM-P] [Official pricing, access-tier, endpoint, key, and rate-limit docs](https://open-meteo.com/en/pricing) and [pinned source](https://github.com/open-meteo/open-meteo-website/blob/1d06b9157387c0b7ac1ee468d6168536f2dda493/src/routes/en/pricing/%2Bpage.svelte)
- [OM-T] [Official terms](https://open-meteo.com/en/terms) and [pinned source](https://github.com/open-meteo/open-meteo-website/blob/1d06b9157387c0b7ac1ee468d6168536f2dda493/src/routes/en/terms/%2Bpage.svelte)
- [OM-L] [Official licence and attribution requirements](https://open-meteo.com/en/licence) and [pinned source](https://github.com/open-meteo/open-meteo-website/blob/1d06b9157387c0b7ac1ee468d6168536f2dda493/src/routes/en/licence/%2Bpage.svelte#L191-L210)
- [OM-U] [Official model update/consistency docs](https://open-meteo.com/en/docs/model-updates) and [pinned source](https://github.com/open-meteo/open-meteo-website/blob/1d06b9157387c0b7ac1ee468d6168536f2dda493/src/routes/en/docs/model-updates/%2Bpage.svelte)
