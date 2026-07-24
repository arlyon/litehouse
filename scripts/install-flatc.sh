#!/usr/bin/env bash
set -euo pipefail

readonly version='25.12.19'
readonly root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/litehouse-flatc-${version}"
readonly bin_dir="${root}/bin"
architecture="$(uname -m)"
readonly architecture

download() {
	local download_url="$1"
	local download_destination="$2"
	curl \
		--proto '=https' \
		--tlsv1.2 \
		--fail \
		--location \
		--retry 3 \
		--retry-delay 2 \
		--retry-all-errors \
		--silent \
		--show-error \
		"${download_url}" \
		--output "${download_destination}"
}

rm -rf "${root}"
mkdir -p "${bin_dir}"

case "${architecture}" in
	x86_64)
		readonly checksum='9f87066dc5dfa7fe02090b55bab5f3e55df03e32c9b0cdf229004ade7d091039'
		readonly url='https://github.com/google/flatbuffers/releases/download/v25.12.19/Linux.flatc.binary.g%2B%2B-13.zip'
		readonly archive="${root}/flatc.zip"
		download "${url}" "${archive}"
		printf '%s  %s\n' "${checksum}" "${archive}" | sha256sum --check --status
		unzip -q -j "${archive}" flatc -d "${bin_dir}"
		;;
	aarch64 | arm64)
		readonly source_checksum='f81c3162b1046fe8b84b9a0dbdd383e24fdbcf88583b9cb6028f90d04d90696a'
		readonly source_url='https://github.com/google/flatbuffers/archive/refs/tags/v25.12.19.tar.gz'
		readonly source_archive="${root}/flatbuffers.tar.gz"
		readonly source_dir="${root}/source"
		readonly build_dir="${root}/build"
		download "${source_url}" "${source_archive}"
		printf '%s  %s\n' "${source_checksum}" "${source_archive}" \
			| sha256sum --check --status
		mkdir -p "${source_dir}"
		tar -xzf "${source_archive}" --strip-components=1 -C "${source_dir}"
		CC=clang CXX=clang++ cmake \
			-S "${source_dir}" \
			-B "${build_dir}" \
			-G Ninja \
			-DCMAKE_BUILD_TYPE=Release \
			-DFLATBUFFERS_BUILD_TESTS=OFF \
			-DFLATBUFFERS_BUILD_GRPCTEST=OFF >&2
		cmake --build "${build_dir}" --target flatc --parallel 2 >&2
		cp "${build_dir}/flatc" "${bin_dir}/flatc"
		;;
	*)
		echo "unsupported flatc build architecture: ${architecture}" >&2
		exit 1
		;;
esac

chmod 0755 "${bin_dir}/flatc"
[[ "$("${bin_dir}/flatc" --version)" == "flatc version ${version}" ]]

if [[ -n "${GITHUB_PATH:-}" ]]; then
	printf '%s\n' "${bin_dir}" >>"${GITHUB_PATH}"
else
	printf '%s\n' "${bin_dir}"
fi
