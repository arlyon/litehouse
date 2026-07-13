<!-- cargo-rdme start -->

Samsung plugin for the Litehouse home automation system.

This crate provides functionality for controlling Samsung TVs via the network,
allowing for integration into the Litehouse system for tasks such as launching
apps or controlling volume.

<!-- cargo-rdme end -->

## Required capability

Grant the TV's exact TCP endpoint in `settings.json`:

```json
{
  "capabilities": ["tcp-client:192.168.1.50:8002"]
}
```

The plugin intentionally accepts the Samsung TV's self-signed TLS certificate.
This makes setup possible without certificate enrollment, but allows a device
on the same network to impersonate the TV and observe the token-bearing
WebSocket request. Restrict the TCP capability to the TV's address.
