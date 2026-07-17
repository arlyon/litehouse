#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
manifest="$root/Cargo.toml"
cd "$root"

cargo fmt --all --check
cargo test --manifest-path "$manifest" -p registry-flow-core
cargo build --manifest-path "$manifest" --target wasm32-wasip2 -p registry-flow-parity-guest
cargo test --manifest-path "$manifest" -p registry-flow-parity-host
cargo clippy --manifest-path "$manifest" --all-targets -- -D warnings

if [[ ${1:-} == --r2 ]]; then
	cargo run --manifest-path "$manifest" -p registry-flow-r2-smoke
fi
