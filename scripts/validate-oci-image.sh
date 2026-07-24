#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <image> <amd64|arm64>" >&2
  exit 2
fi

image="$1"
expected_architecture="$2"
runtime="${CONTAINER_RUNTIME:-docker}"
container="litehouse-oci-${expected_architecture}-$$"
volume="litehouse-oci-config-${expected_architecture}-$$"

cleanup() {
  "$runtime" rm --force "$container" >/dev/null 2>&1 || true
  "$runtime" volume rm --force "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

actual_architecture="$($runtime image inspect "$image" --format '{{.Architecture}}')"
if [[ "$actual_architecture" != "$expected_architecture" ]]; then
  echo "expected image architecture $expected_architecture, got $actual_architecture" >&2
  exit 1
fi

configured_user="$($runtime image inspect "$image" --format '{{.Config.User}}')"
if [[ "$configured_user" != "10001:10001" ]]; then
  echo "expected image user 10001:10001, got ${configured_user:-<empty>}" >&2
  exit 1
fi

entrypoint="$($runtime image inspect "$image" --format '{{json .Config.Entrypoint}}')"
exposed_ports="$($runtime image inspect "$image" --format '{{json .Config.ExposedPorts}}')"
stop_signal="$($runtime image inspect "$image" --format '{{.Config.StopSignal}}')"
if [[ "$entrypoint" != '["/usr/local/bin/litehouse"]' ]]; then
  echo "unexpected image entrypoint: $entrypoint" >&2
  exit 1
fi
if [[ "$exposed_ports" != '{"8080/tcp":{}}' ]]; then
  echo "unexpected exposed ports: $exposed_ports" >&2
  exit 1
fi
if [[ "$stop_signal" != "SIGTERM" ]]; then
  echo "expected SIGTERM stop signal, got ${stop_signal:-<empty>}" >&2
  exit 1
fi

runtime_uid="$($runtime run --rm --entrypoint /usr/bin/id "$image" -u)"
runtime_gid="$($runtime run --rm --entrypoint /usr/bin/id "$image" -g)"
runtime_identity="${runtime_uid}:${runtime_gid}"
if [[ "$runtime_identity" != "10001:10001" ]]; then
  echo "expected runtime identity 10001:10001, got $runtime_identity" >&2
  exit 1
fi

providers="$($runtime run --rm --entrypoint /bin/sh "$image" -c \
  'set -eu; test ! -w /opt/litehouse/components; find /opt/litehouse/components -maxdepth 1 -type f -printf "%f\n" | sort')"
expected_providers=$'esphome.wasm\nntfy.wasm\nopen_meteo.wasm\ntasmota.wasm'
if [[ "$providers" != "$expected_providers" ]]; then
  echo "image does not contain exactly the four bundled providers" >&2
  printf 'found:\n%s\n' "$providers" >&2
  exit 1
fi

"$runtime" volume create "$volume" >/dev/null
"$runtime" run --rm --volume "$volume:/config" "$image" init
"$runtime" run --rm --entrypoint /bin/sh --volume "$volume:/config" "$image" -c \
  'set -eu; test -f settings.json; test -f schema.json; test -f rule.schema.json; test -d workflows'

"$runtime" run --detach \
  --name "$container" \
  --read-only \
  --cap-drop ALL \
  --security-opt no-new-privileges:true \
  --tmpfs /tmp:rw,nosuid,nodev,noexec,size=16m \
  --volume "$volume:/config" \
  --publish 127.0.0.1::8080 \
  "$image" >/dev/null

port="$($runtime port "$container" 8080/tcp | awk -F: 'END {print $NF}')"
if [[ -z "$port" ]]; then
  echo "unable to determine published daemon port" >&2
  "$runtime" logs "$container" >&2 || true
  exit 1
fi

health_status="000"
readiness_status="000"
startup_deadline=$((SECONDS + 60))
while ((SECONDS < startup_deadline)); do
  health_status="$(curl --silent --connect-timeout 1 --max-time 2 \
    --output /dev/null --write-out '%{http_code}' \
    "http://127.0.0.1:${port}/healthz" || true)"
  readiness_status="$(curl --silent --connect-timeout 1 --max-time 2 \
    --output /dev/null --write-out '%{http_code}' \
    "http://127.0.0.1:${port}/readyz" || true)"
  if [[ "$health_status" == "200" && "$readiness_status" == "200" ]]; then
    break
  fi
  sleep 1
done

if [[ "$health_status" != "200" || "$readiness_status" != "200" ]]; then
  echo "daemon did not become healthy and ready (health=$health_status readiness=$readiness_status)" >&2
  "$runtime" logs "$container" >&2 || true
  exit 1
fi

stop_started="$SECONDS"
"$runtime" stop --time 20 "$container" >/dev/null
stop_elapsed=$((SECONDS - stop_started))
exit_code="$($runtime inspect "$container" --format '{{.State.ExitCode}}')"
if [[ "$exit_code" != "0" ]]; then
  echo "daemon exited with status $exit_code after SIGTERM" >&2
  "$runtime" logs "$container" >&2 || true
  exit 1
fi
if ((stop_elapsed > 20)); then
  echo "daemon shutdown exceeded 20 seconds (${stop_elapsed}s)" >&2
  exit 1
fi

printf 'validated %s image: non-root, four providers, ready, clean shutdown in %ss\n' \
  "$expected_architecture" "$stop_elapsed"
