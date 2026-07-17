#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
target="${CARGO_TARGET_DIR:-/home/arlyon/.cache/litehouse-connected-release-target/typed-esphome-provider-spike}"
component="$target/wasm32-wasip2/debug/typed_esphome_provider_component.wasm"

cargo build --manifest-path "$root/Cargo.toml" \
  --target wasm32-wasip2 \
  -p typed-esphome-provider-component
wasm-tools validate --features cm-async "$component"
cargo run --manifest-path "$root/Cargo.toml" \
  -p typed-esphome-provider-harness -- "$component"
cargo clippy --manifest-path "$root/Cargo.toml" --all-targets -- -D warnings
