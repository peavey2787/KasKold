#!/usr/bin/env bash
set -euo pipefail

PLATFORM_NAME="${1:?platform name required}"
ARCH="${2:?architecture required}"
OUTPUT_DIR="${3:?output directory required}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/../../.." && pwd)"
TOOLCHAINS_ENV="$ROOT/qa/config/toolchains.env"
[[ -f "$TOOLCHAINS_ENV" ]] || { echo "ERROR: missing central toolchain policy: $TOOLCHAINS_ENV" >&2; exit 2; }
# shellcheck disable=SC1090
source "$TOOLCHAINS_ENV"
[[ -n "${KASKOLD_STABLE_RUST:-}" ]] || { echo "ERROR: KASKOLD_STABLE_RUST is missing from $TOOLCHAINS_ENV" >&2; exit 2; }

case "$PLATFORM_NAME" in
  iphoneos) TARGET="aarch64-apple-ios" ;;
  iphonesimulator)
    case "$ARCH" in
      x86_64) TARGET="x86_64-apple-ios" ;;
      *) TARGET="aarch64-apple-ios-sim" ;;
    esac
    ;;
  *) echo "ERROR: unsupported Apple platform: $PLATFORM_NAME" >&2; exit 2 ;;
esac

command -v cargo >/dev/null 2>&1 || { echo "ERROR: Rust cargo is required to build KasKold Vault." >&2; exit 2; }
rustup target list --toolchain "$KASKOLD_STABLE_RUST" --installed | grep -qx "$TARGET" || {
  echo "ERROR: Rust target $TARGET is not installed for $KASKOLD_STABLE_RUST. Run: rustup target add $TARGET --toolchain $KASKOLD_STABLE_RUST" >&2
  exit 2
}

CARGO_TARGET_DIR="$ROOT/target/kaskold-vault-ios" \
  cargo "+$KASKOLD_STABLE_RUST" build --manifest-path "$ROOT/Cargo.toml" -p vault-runtime --release --locked --target "$TARGET"
mkdir -p "$OUTPUT_DIR"
cp "$ROOT/target/kaskold-vault-ios/$TARGET/release/libvault_runtime.a" "$OUTPUT_DIR/libvault_runtime.a"
