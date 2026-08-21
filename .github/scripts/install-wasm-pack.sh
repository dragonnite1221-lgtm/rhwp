#!/usr/bin/env bash
set -euo pipefail

readonly WASM_PACK_VERSION="0.13.1"
readonly WASM_PACK_TARGET="x86_64-unknown-linux-musl"
readonly WASM_PACK_SHA256="c539d91ccab2591a7e975bcf82c82e1911b03335c80aa83d67ad25ed2ad06539"
readonly WASM_PACK_ARCHIVE="wasm-pack-v${WASM_PACK_VERSION}-${WASM_PACK_TARGET}.tar.gz"
readonly WASM_PACK_URL="https://github.com/wasm-bindgen/wasm-pack/releases/download/v${WASM_PACK_VERSION}/${WASM_PACK_ARCHIVE}"
readonly ARCHIVE_PATH="${RUNNER_TEMP:?RUNNER_TEMP is required}/${WASM_PACK_ARCHIVE}"
readonly EXTRACT_PATH="${RUNNER_TEMP}/wasm-pack-v${WASM_PACK_VERSION}"
readonly BIN_PATH="${RUNNER_TEMP}/wasm-pack-bin"

curl \
  --proto '=https' \
  --tlsv1.2 \
  --fail \
  --location \
  --silent \
  --show-error \
  --output "${ARCHIVE_PATH}" \
  "${WASM_PACK_URL}"

printf '%s  %s\n' "${WASM_PACK_SHA256}" "${ARCHIVE_PATH}" \
  | sha256sum --check --strict

mkdir -p "${EXTRACT_PATH}" "${BIN_PATH}"
tar -xzf "${ARCHIVE_PATH}" -C "${EXTRACT_PATH}"
install -m 0755 \
  "${EXTRACT_PATH}/wasm-pack-v${WASM_PACK_VERSION}-${WASM_PACK_TARGET}/wasm-pack" \
  "${BIN_PATH}/wasm-pack"

printf '%s\n' "${BIN_PATH}" >> "${GITHUB_PATH:?GITHUB_PATH is required}"
"${BIN_PATH}/wasm-pack" --version
