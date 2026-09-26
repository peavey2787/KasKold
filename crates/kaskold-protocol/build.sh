#!/usr/bin/env bash
set -Eeuo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${DIR}/../.." && pwd)"
PKG_DIR="${KASKOLD_SDK_OUTPUT_ROOT:-${ROOT}/target/sdk}/kaskold-protocol/pkg"
exec "${DIR}/../../scripts/linux/lib/rust-wasm-sdk.sh" kaskold-protocol kaskold_protocol "${PKG_DIR}" "KasKold protocol Rust/WASM" "@kaskold/protocol"
