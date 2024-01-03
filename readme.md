# litehouse

An experimental replacement for home-assistant using webassembly
and wit components. Using wit bindgen, plugins can be written in
any language that can compile to webassembly.

The result is a lightweight core for home automation that can be
extended with plugins written in any language. In addition, plugins
may depend on other plugins, allowing for a modular architecture,
though there is not a reference implementation for that _yet_.

## Protocol

The basic protocol is defined in `crates/plugin-macro/wit/plugin.wit`,
the core of which being the 'runner' interface. This is the interface
the server uses to run your plugin. For now, it will simply call the
update function at an interval specified by the plugin in the
subscribe function. In the future, you will be able to subscibe to
events from other plugins, and the server will call your update
function when those events occur.

The module also provides an `update` global, which your plugin can use
to send events to the server at any time.

## Capabilities

The server provides native implementations for a number of capabilities,
the primary one (used in both reference plugins) being `wasi:http/outgoing-handler`. Plugins may use this to make http requests, which
are served using the server's native http client, meaning plugins do not
need to bundle heavy dependencies. Other capabilities, such as filesystem,
clocks, random numbers, and more are also available but not exposed to plugins
yet.

Once a scheme for handling configuration is decided on, plugins will be able
declare network ports to listen on, and the server will forward requests to
their incoming handlers. This will be the foundation of the next plugins,
namely `mqtt` and `web`.

## Building

Plugins can be written in any language and compiled to webassembly. Currently,
wit bindgen supports a few guest languages namely rust, c/c++, java, and tinygo.
The reference plugins and server are rust-based. Building the server is as
simple as running cargo install in the server directory.

```bash
cargo install --path crates/litehouse
```

Plugins are a little more complicated. You must build against the
`wasm32-wasi` target, and then process the resulting binary with
`wasm-tools` and the `./wasi_snapshot_preview1.reactor.wasm` adaptor
bundled in the repo. It comes from the specific git commit of
wasmtime that the server uses. In the future, it is expected that
this step will no longer be necessary.

```bash
cargo build --target wasm32-wasi --release
wasm-tools component new ../../target/wasm32-wasi/release/plugin.wasm --adapt ./wasi_snapshot_preview1.reactor.wasm -o plugin.wasm
# optionally, strip
wasm-tools strip plugin.wasm -o plugin.wasm
```

The weather plugin weighs in at about 200KB, and the tasmota at 300KB.
