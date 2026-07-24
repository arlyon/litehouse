# Decide the workspace and map baseline

Type: grilling
Mode: HITL
Status: resolved
Resolved: 2026-07-13
Blocked by: None
Map: [Find the route to Litehouse's first connected release](../map.md)

## Question

Which Rust edition is canonical, how are the dirty registry edition and
generated FlatBuffers output owned/regenerated, which exact-commit versus dirty
workspace checks are authoritative, and should `.scratch/litehouse-platform/`
remain local, become ignored, or be reviewed and tracked—without overwriting or
silently absorbing user work?

## Answer

Rust 2024 is the canonical workspace edition. Every workspace package inherits
it unless a concrete external compatibility constraint is separately reviewed
and documented. The dirty root downgrade to 2021 and registry-local 2024
override are unexplained local experiments, not the baseline. Wayfinder does not
modify or absorb them; a separately approved baseline implementation tranche
restores `edition.workspace = true` everywhere and validates the complete
workspace under 2024.

`registry.fbs` is the source of the current FlatBuffers model. Generated Rust is
derived build output, never hand-edited source. If FlatBuffers survives the
registry-format decision, pin a compiler compatible with the Rust runtime,
generate deterministically into Cargo `OUT_DIR`, and make CI regenerate/check
fixtures without tracking output under `target/`. If the Cargo-like static-index
research selects another canonical format, delete the obsolete schema/generator
and tracked output together in the approved registry tranche. The current dirty
`registry_generated.rs`, produced by local `flatc 25.12.19` against an older
runtime dependency, remains preserved and excluded from the planning commit.

Canonical implementation validation runs in a clean worktree at the reviewed
planning baseline plus only the approved patch. Targeted checks in the dirty
working directory remain useful compatibility feedback, and both result sets
are reported, but unrelated local experiments cannot silently change the
release verdict. Any deliberate update to the baseline is explicit in the
relevant implementation proposal.

After a path-scoped secret and scope review, track
`.scratch/litehouse-platform/` at its current location. It is the durable local-
Markdown issue tracker and implementation handoff record. Add only that exact
directory; never bulk-add `.scratch/`, `.mise.toml`, unrelated untracked files,
credential-bearing infrastructure, signing material, generated binaries, or
crash reports.

The user explicitly authorized one commit containing only the reviewed
Litehouse Wayfinder map and tickets. Its parent is code baseline `d1549ee`; the
resulting planning-only commit becomes the canonical map baseline without
changing application code, manifests, generated registry output, credentials,
or external systems.
