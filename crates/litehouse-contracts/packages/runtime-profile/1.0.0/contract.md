# `litehouse:runtime-profile@1.0.0`

This package describes compatibility; it does not redefine or grant external host interfaces. A component still self-describes exact imports, and installation rejects imports outside its declared profile.

Profile 1 accepts Component Model async/concurrent calls on Wasmtime 46.0.1, Rust `wasm32-wasip2`/`std` compatibility imports from official WASI 0.2.12 packages, official native-async WASI 0.3.0 and `wasi:http@0.3.0`, and versioned Litehouse service-fabric and tracing host interfaces. The official package definitions remain authoritative and are not copied here. Domain capability contracts import no WASI.

The only v1 sandbox is one supervised actor and fresh store per configured plugin instance/generation. There are at most 32 in-flight and 64 waiting guest calls per instance, 64 MiB guest linear memory, 4,096 guest resources, and a 30-second hard ceiling for ordinary calls including queue time. Unsupported shared-store strategies fail closed.

Linkability is not authority. Filesystem has no preopens by default; HTTP/sockets receive exact grants; protected values arrive only through typed instance config; guests inherit no ambient environment; stdio is bounded and is never a shell. Runtime-profile minors may only add accepted imports/features while preserving prior compatibility. Removal or changed meaning requires a new major.
