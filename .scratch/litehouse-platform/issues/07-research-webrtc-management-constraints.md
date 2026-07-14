# Research WebRTC management constraints

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which WebRTC data-channel constraints shape the direct management protocol?

## Answer

Partial standards and `webrtc-rs` research found:

- Use an in-band DCEP reliable/ordered channel, then negotiate management
  protocol versions/features inside an application `HELLO`; the channel
  `protocol` string is not capability negotiation.
- Without SCTP message interleaving, large messages can head-of-line block all
  channels. Keep complete frames near or below 16 KiB and chunk larger snapshots
  with bounded reassembly, sequence, end marker, and checksum.
- `bufferedAmount` is sender queue pressure, not receiver consumption. Litehouse
  needs high/low watermarks, bounded queues, and application credits/ACKs.
- WebRTC has no per-operation cancellation, deadline, replay, or resume. Define
  them in the envelope. Disconnect leaves mutation outcomes unknown; recover by
  idempotency/result lookup rather than blind replay.
- Bind each short-lived connection grant to subject, server, scopes, expiry,
  nonce, and authenticated signaling/DTLS fingerprint. A data channel alone is
  not authorization.
- Litehouse currently pins `webrtc` 0.11.0 and has one non-trickle offer/answer
  path without renegotiation. Evaluate current Rust releases before freezing v1.

Primary sources: [RFC 8831](https://www.rfc-editor.org/rfc/rfc8831.html),
[RFC 8832](https://www.rfc-editor.org/rfc/rfc8832.html),
[RFC 8841](https://www.rfc-editor.org/rfc/rfc8841.html),
[WebRTC API](https://www.w3.org/TR/webrtc/), and
[`webrtc-rs` 0.11](https://github.com/webrtc-rs/webrtc/tree/v0.11.0).

Protocol codec, heartbeat/deadline values, retention, queue budgets, browser
matrix, Rust upgrade, ICE restart, and resume semantics remain decisions.
