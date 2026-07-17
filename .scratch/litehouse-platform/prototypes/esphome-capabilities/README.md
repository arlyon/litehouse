# ESPHome capability spike

Disposable planning evidence for
[Prototype real ESPHome capabilities](../../issues/22-prototype-real-esphome-capabilities.md).
It does not modify production crates or define the final provider component.

## Scope

The fixture model proves:

- one provider instance and session/cache lifecycle per node;
- repeated independently bindable On/Off and Boolean facets without a nested
  device identity;
- stable Litehouse-owned IDs across reconnect;
- separate store generations and facet incarnations;
- metadata-only UI groups;
- feature-gated Level and Color exposure;
- exact built-in temperature, humidity, carbon-dioxide, and particulate
  measurement profiles, with unknown VOC input left diagnostic-only;
- explicit observed/unknown/unavailable quality;
- accepted-for-delivery command receipts that never claim confirmation; and
- isolation between two configured node instances.

A fixed-key local Noise fixture proves that `esphome-client` 0.2.0 with pinned
API 1.14 can decode selected switch/sensor entity and state messages emitted by
an `esphome-native-api` 2.1.0 server using the 2026.6.2 protocol source.

`esphome-native-api` 2.1.0 is useful as a current fixture server and protobuf
source, but its `EspHomeApi::start` implementation behaves as the server-side
responder despite client-oriented documentation. The real-node harness therefore
uses `esphome-client` 0.2.0. The real preflight must still stop on incompatible
major protocol behavior or missing required topology.

The approved physical nodes do not expose a Level/Color light, so those features
remain fixture-proven rather than hardware-proven.

## Real Node S evidence

A supervised read-only proof connected twice over Noise to Node S running
ESPHome 2026.3.2. Both sessions listed four switch entities and three binary
sensors, observed all seven initial states, and produced the same stable
Litehouse identities after reconnect.

The confirmed YAML contains one template switch followed by three GPIO switches.
Native API metadata reports all four as ordinary switch entities and does not
expose the source platform. A generic provider must therefore publish four
On/Off facets unless explicit adoption/configuration excludes one; it must not
guess that only GPIO entities are physical. No real command was needed or sent.

## Validate fixtures

```bash
cargo test --manifest-path Cargo.toml
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
```

## Prepare real read-only preflight

Create this file only on runtime tmpfs:

```text
$XDG_RUNTIME_DIR/litehouse-esphome-proof/targets.json
```

with mode exactly `0600` and this shape:

```json
{
  "node_s": { "address": "<exact-host:port>", "noise_key": "<base64-key>" },
  "node_a": { "address": "<exact-host:port>", "noise_key": "<base64-key>" }
}
```

Never copy the real file into the repository or conversation. The harness uses
only this fixed path, rejects symlinks and non-`0600` files, bounds reads, and
zeroizes parsed credential strings on drop. It does not initialize dependency
logging and maps connection/protocol failures to alias-only errors.

After fresh user approval for an immediate supervised window:

```bash
cargo run --manifest-path Cargo.toml -- preflight node-s
cargo run --manifest-path Cargo.toml -- preflight node-a
```

Preflight connects twice to the selected exact node, reads only
version/entity/state information, and prints alias-only counts plus identity
stability. It issues no commands. The completed proof intentionally stopped
after Node S; Node A measurement behavior remains deterministic fixture
evidence.
