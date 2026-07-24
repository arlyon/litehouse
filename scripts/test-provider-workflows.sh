#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-nightly-2026-07-15}"

cargo build -q -p litehouse-provider-host-fixture --target wasm32-wasip2
target_directory="$(cargo metadata --format-version 1 --no-deps \
  | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
fixture="$target_directory/wasm32-wasip2/debug/litehouse_provider_host_fixture.wasm"
test -f "$fixture"
export LITEHOUSE_PROVIDER_FIXTURE_WASM="$fixture"

cargo test -q -p litehouse-provider-host --test wasmtime \
  typed_handles_stay_in_actor_while_host_routes_query_set_send_and_withdrawal \
  -- --ignored --exact
cargo test -q -p litehouse-management --test mcp \
  four_rmcp_tools_use_closed_handlers_and_no_generic_dispatch \
  -- --ignored --exact
cargo test -q -p litehouse-management --test workflow_executor \
  provider_component_drives_triggers_receipts_timers_outcomes_and_failure_isolation \
  -- --ignored --exact
cargo test -q -p litehouse-management \
  workflow_runtime::tests::outcomes_are_newest_first_and_bounded_per_workflow \
  -- --exact
cargo test -q -p litehouse-management \
  workflow_runtime::tests::outcomes_are_bounded_server_wide \
  -- --exact
