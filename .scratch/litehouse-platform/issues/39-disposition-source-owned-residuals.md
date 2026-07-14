# Disposition source-owned residuals

Type: grilling
Mode: HITL
Status: open
Blocked by: 24, 27, 33, 38
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which verified source-owned residuals are implemented, deleted, explicitly
deferred with an owner/trigger, or ruled out—and what evidence closes each
choice? The inventory must include:

- JSON-Schema keyword paths in `crates/litehouse/src/util.rs`;
- registry insertion and temp-file safety in
  `crates/registry/src/partitioned_store.rs` and
  `crates/registry/src/partition.rs`;
- blocking reads in `crates/opendal-fs-cache/src/lib.rs`;
- legacy HTTP middleware in `crates/plugin/src/http.rs`;
- metadata injection in `crates/plugin-macro/src/lib.rs`;
- pairing validation in `crates/cockpit/src/client.rs`;
- the admin check in `site/components/toolbar.tsx`;
- memory markers in `crates/litehouse/src/runtime.rs`,
  `crates/litehouse/src/server.rs`, and `crates/litehouse/src/cache.rs`;
- the WASI compatibility bridge, plugin skeletons, and migration residuals.
