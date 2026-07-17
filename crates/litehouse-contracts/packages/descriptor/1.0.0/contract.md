# `litehouse:descriptor@1.0.0`

## Facets

A facet descriptor's binding identity is provider instance, stable facet key, incarnation/generation, and exact package identity. Labels, tags, source routing keys, group keys, and list position MUST NOT be used as identity. Every facet has exactly one primary composition group; groups are presentation metadata, not routing or transaction boundaries.

Features, operations, state fields, and events are explicit names. Capability packages additionally expose typed closed sets. Semantic roles/profiles and namespaced tags are inert hints: they contain no renderer, HTML, component, or executable UI. Identifiers and labels are UTF-8 and at most 256 bytes; package namespace/name are bounded kebab identifiers of at most 64 bytes.

## Runtime metadata

`runtime-metadata-v1` is the typed metadata imported by the plugin host. It declares package identity, same-major runtime-profile requirement, provided contracts and events, required contracts/operations with selector cardinality, maximum requested host facilities, optional config schema, and closed config annotations.

Only `environment-secret` and `private-local` config annotations exist. They describe handling, never carry a secret value, variable value, key material, or signature. Environment field tokens match `[A-Z][A-Z0-9_]{0,63}`. Private projection is only configured-status or omitted. Metadata is deterministic, at most 64 KiB, with description at most 1,024 bytes, at most 64 provided and 64 required contracts, 16 exact HTTP and 16 exact TCP origins, 128 annotations, and a 48 KiB config schema.

`unsigned-release-input-v1` is deterministic release input only. It contains digests and provenance but no signature, key, publication target, verification claim, or secret. Signatures and immutable release locks belong to later registry records, not this contract.
