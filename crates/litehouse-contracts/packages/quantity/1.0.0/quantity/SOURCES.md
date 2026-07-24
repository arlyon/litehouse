# Quantity standards sources

The v1 runtime wire is controlled by Litehouse's bounded profile catalog and
exact Rust/Wasm implementation. UCUM 2.2 is the unit-code authority; QUDT 3.4.0
is a design and profile-generation reference only.

`upstream-sources.json` pins the reviewed upstream release commits and content
checksums. The upstream datasets are intentionally **not vendored** in T1:
UCUM redistribution requires preserved notices and legal review, while QUDT
material requires CC BY 4.0 attribution. Vendoring or publishing either dataset
remains blocked until that review is recorded. The T1 engine implements only its
reviewed, allowlisted subset and never fetches these sources at build or runtime.
