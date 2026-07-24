# Research HTTP authority normalization

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

How should HTTP destinations be normalized and compared fail-closed under
standards and Wasmtime 46/WASI HTTP 0.3 behavior?

## Answer

Partial standards/runtime research supports canonical exact comparison of:

```text
(scheme: http|https, host: validated DNS | IPv4 bits | IPv6 bits,
 effective-port: u16)
```

- Scheme and DNS names are ASCII case-insensitive; omitted ports become 80/443.
  Different scheme, host, or effective port is a different origin.
- Reject userinfo, empty/absent hosts, Unicode or percent-encoded host tricks,
  wildcards/suffix matching, non-standard IPv4 forms, zone identifiers,
  malformed brackets/colons, zero/out-of-range ports, and unsupported schemes.
- Parse IPv4 as exactly four decimal octets and IPv6 numerically from bracketed
  syntax. Adopt an explicit trailing-dot policy; rejecting it is simplest.
- Wasmtime 46.0.1 has bracketed-IPv6 limitations, including defective HTTPS
  hostname extraction; deny unsupported IPv6 combinations until a tested
  connector path exists.
- Re-authorize every redirect if redirect following is ever added.
- Hostname equality alone cannot prevent DNS rebinding. True address policy
  requires resolving once, rejecting disallowed answers, connecting to the
  validated address without re-resolution, and retaining the name for Host/TLS.
  Wasmtime's default sender does not expose that control.

Primary sources: [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html),
[RFC 3986](https://www.rfc-editor.org/rfc/rfc3986.html),
[RFC 5952](https://www.rfc-editor.org/rfc/rfc5952.html), and Wasmtime
[`v46.0.1` WASI HTTP](https://github.com/bytecodealliance/wasmtime/tree/v46.0.1/crates/wasi-http).

The remaining decision is whether DNS grants trust all resolver answers or also
constrain IP ranges; the latter requires connector replacement/customization.
