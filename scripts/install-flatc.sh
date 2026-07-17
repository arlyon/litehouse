#!/usr/bin/env bash
set -euo pipefail

readonly version='25.12.19'
readonly checksum='9f87066dc5dfa7fe02090b55bab5f3e55df03e32c9b0cdf229004ade7d091039'
readonly url='https://github.com/google/flatbuffers/releases/download/v25.12.19/Linux.flatc.binary.g%2B%2B-13.zip'
readonly root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/litehouse-flatc-${version}"
readonly archive="${root}/flatc.zip"
readonly bin_dir="${root}/bin"

rm -rf "${root}"
mkdir -p "${bin_dir}"
curl \
	--proto '=https' \
	--tlsv1.2 \
	--fail \
	--location \
	--silent \
	--show-error \
	"${url}" \
	--output "${archive}"
printf '%s  %s\n' "${checksum}" "${archive}" | sha256sum --check --status
unzip -q -j "${archive}" flatc -d "${bin_dir}"
chmod 0755 "${bin_dir}/flatc"
[[ "$("${bin_dir}/flatc" --version)" == "flatc version ${version}" ]]

if [[ -n "${GITHUB_PATH:-}" ]]; then
	printf '%s\n' "${bin_dir}" >>"${GITHUB_PATH}"
else
	printf '%s\n' "${bin_dir}"
fi
