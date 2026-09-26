#!/usr/bin/env bash
set -Eeuo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${DIR}/../.." && pwd)"
PKG_DIR="${KASKOLD_SDK_OUTPUT_ROOT:-${ROOT}/target/sdk}/kaskold-sdk/pkg"
exec "${DIR}/../../scripts/linux/lib/rust-wasm-sdk.sh" kaskold-sdk kaskold_sdk "${PKG_DIR}" "KasKold SDK Rust/WASM" "@kaskold/sdk"
