# litehouse

An experimental replacement for home-assistant using webassembly
and wit components. Using wit bindgen, plugins can be written in
any language that can compile to webassembly.

The result is a lightweight core for home automation that can be
extended with plugins written in any language. In addition, plugins
may depend on other plugins, allowing for a modular architecture,
though there is not a reference implementation for that _yet_.

## Building

Plugins can be written in any language and compiled to webassembly. Currently,
wit bindgen supports a few guest languages namely rust, c/c++, java, and tinygo.
The reference plugins and server are rust-based. Building the server is as
simple as running cargo install in the server directory.

```bash
cargo install --git https://github.com/arlyon/litehouse litehouse
```

You can use litehouse to search for, build, fetch, and publish plugins.
The `fetch` command reads the imports from your settings and fetches
matches plugins from the registry. The `build` command compiles a local
plugin to a given path.

```bash
litehouse build weather
litehouse search weather
litehouse fetch
```

The weather plugin weighs in at about 200KB, and the tasmota at 300KB.

## Running

The server can be run with the `litehouse run` command. It will read
the plugin instances from your local `settings.json` file, and instantiate
one for each of them. The server will then call the update function on
each of them at the interval specified in the plugin's subscribe function.

First, let's set up a settings file, and a schema file.

```bash
litehouse init && litehouse generate
```

Next, we should add some plugins to the import field in `settings.json`.

```diff
{
  "$schema": "schema.json",
- "plugins": {}
+ "plugins": {},
+ "imports": ["weather", "tasmota", "samsung"]
}
```

From there, we can fetch the plugins from the registry, and re-run the
generate command to update the schema file.

```bash
litehouse fetch
litehouse generate
```

Finally, we can validate the settings file and run the server. Editors
with a jsonschema language server will provide validation and
autocompletion for the settings file as you type.

```bash
litehouse validate
litehouse run
```

## Protocol

The basic protocol is defined in `crates/plugin-macro/wit/plugin.wit`,
the core of which being the 'runner' interface. This is the interface
the server uses to run your plugin. For now, it will simply call the
update function at an interval specified by the plugin in the
subscribe function. In the future, you will be able to subscribe to
events from other plugins, and the server will call your update
function when those events occur.

The module also provides an `update` global, which your plugin can use
to send events to the server at any time.

## Capabilities

The server provides native implementations for a number of capabilities,
the primary one (used in both reference plugins) being `wasi:http/outgoing-handler`.
Plugins may use this to make http requests, which are served using the server's
native http client, meaning plugins do not need to bundle heavy dependencies.
Other capabilities, such as filesystem, clocks, random numbers, and more are
also available but not exposed to plugins yet.

Plugins are completely sandboxed by default, so to allow them to access
outside the sandbox, you must award them capabilities by adding them to
the `capabilities` field in your settings file.

```jsonc
{
  "capabilities": [
    "http-client:api.open-meteo.com",
    "http-server:0.0.0.0:8000",
    // (not implemented yet)
    "folder:example",
  ],
}
```
