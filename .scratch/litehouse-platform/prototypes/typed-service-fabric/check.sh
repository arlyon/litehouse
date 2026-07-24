#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
manifest="$root/Cargo.toml"
target="$root/target/wasm32-wasip2/debug"

cargo build --manifest-path "$manifest" --target wasm32-wasip2 \
	-p service-spike-provider \
	-p service-spike-consumer \
	-p service-spike-p3-smoke

for component in \
	"$target/service_spike_provider.wasm" \
	"$target/service_spike_consumer.wasm" \
	"$target/service_spike_p3_smoke.wasm"; do
	wasm-tools validate --features cm-async "$component"
done

wasm-tools component wit "$target/service_spike_provider.wasm" |
	grep -q 'invoke: async func'
wasm-tools component wit "$target/service_spike_p3_smoke.wasm" |
	grep -q 'wasi:cli/run@0.3.0'

if grep -R --include='*.rs' --exclude-dir=target -n 'ResourceAny' "$root"; then
	echo 'cross-store ResourceAny is forbidden' >&2
	exit 1
fi

cargo test --manifest-path "$manifest" -p service-spike-host
