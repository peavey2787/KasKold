#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
COMPANION_PROJECT="$ROOT/apps/kaskold-companion-ios/KasKold.xcodeproj"
VAULT_PROJECT="$ROOT/apps/kaskold-vault-ios/KasKoldVault.xcodeproj"
TOOLCHAINS_ENV="$ROOT/qa/config/toolchains.env"
MODE="${1:-build}"
PRODUCTS="${KASKOLD_IOS_PRODUCTS:-all}"
COMPANION_DERIVED_DATA="$ROOT/target/ios/CompanionDerivedData"
VAULT_DERIVED_DATA="$ROOT/target/ios/VaultDerivedData"

fail() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

case "$PRODUCTS" in
  all) BUILD_COMPANION=1; BUILD_VAULT=1 ;;
  companion) BUILD_COMPANION=1; BUILD_VAULT=0 ;;
  vault) BUILD_COMPANION=0; BUILD_VAULT=1 ;;
  *) fail "unknown KASKOLD_IOS_PRODUCTS=$PRODUCTS (expected all, companion, or vault)" ;;
esac

if [[ "$(uname -s)" != "Darwin" ]]; then
  fail "KasKold iOS applications require macOS with Xcode. Run this target on a macOS/Xcode host."
fi
command -v xcodebuild >/dev/null 2>&1 || fail "xcodebuild is required for KasKold iOS."

if [[ "$BUILD_VAULT" == "1" ]]; then
  command -v rustup >/dev/null 2>&1 || fail "rustup is required to build KasKold Vault iOS."
  command -v cargo >/dev/null 2>&1 || fail "cargo is required to build KasKold Vault iOS."
  [[ -f "$TOOLCHAINS_ENV" ]] || fail "Missing central toolchain policy: $TOOLCHAINS_ENV"
  # shellcheck disable=SC1090
  source "$TOOLCHAINS_ENV"
  [[ -n "${KASKOLD_STABLE_RUST:-}" ]] || fail "KASKOLD_STABLE_RUST is missing from $TOOLCHAINS_ENV."

  # The Vault Xcode build phase selects one of these targets from PLATFORM_NAME
  # and the active architecture. Require the complete supported target set so
  # Intel/Apple-Silicon simulators and device archives use the same pinned Rust.
  installed_targets="$(rustup target list --toolchain "$KASKOLD_STABLE_RUST" --installed 2>/dev/null || true)"
  for rust_target in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
    grep -qx "$rust_target" <<<"$installed_targets" \
      || fail "Rust target $rust_target is required. Run: rustup target add $rust_target --toolchain $KASKOLD_STABLE_RUST"
  done
fi

if [[ "$BUILD_COMPANION" == "1" ]]; then
  "$ROOT/scripts/mac/build/ios-runtime-sync.sh"
fi

build_vault_simulator() {
  local destination="$1"
  mkdir -p "$VAULT_DERIVED_DATA"
  xcodebuild -project "$VAULT_PROJECT" -scheme KasKoldVault -configuration Debug \
    -sdk iphonesimulator -destination "$destination" \
    -derivedDataPath "$VAULT_DERIVED_DATA" CODE_SIGNING_ALLOWED=NO build
  local artifact="$VAULT_DERIVED_DATA/Build/Products/Debug-iphonesimulator/KasKoldVault.app"
  [[ -d "$artifact" ]] || { echo "ERROR: Vault iOS Debug build completed but $artifact was not produced." >&2; exit 1; }
  printf 'Built Vault artifact:\n  %s\n' "$artifact"
}

case "$MODE" in
  build)
    if [[ "$BUILD_COMPANION" == "1" ]]; then
      mkdir -p "$COMPANION_DERIVED_DATA"
      xcodebuild -project "$COMPANION_PROJECT" -scheme KasKold -configuration Debug \
        -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
        -derivedDataPath "$COMPANION_DERIVED_DATA" KASKOLD_IOS_RUNTIME_SYNCED=1 CODE_SIGNING_ALLOWED=NO build
      companion_artifact="$COMPANION_DERIVED_DATA/Build/Products/Debug-iphonesimulator/KasKold.app"
      [[ -d "$companion_artifact" ]] || { echo "ERROR: Companion iOS Debug build completed but $companion_artifact was not produced." >&2; exit 1; }
      printf 'Built Companion artifact:\n  %s\n' "$companion_artifact"
    fi
    if [[ "$BUILD_VAULT" == "1" ]]; then
      build_vault_simulator 'generic/platform=iOS Simulator'
    fi
    echo "KasKold iOS build complete (products=$PRODUCTS)."
    ;;
  release)
    team="${KASKOLD_IOS_DEVELOPMENT_TEAM:-}"
    [[ -n "$team" ]] || {
      echo "ERROR: KASKOLD_IOS_DEVELOPMENT_TEAM is required for signed iOS Release archives." >&2
      echo "Set it to the publishing Apple Developer Team ID; do not commit account-specific team IDs." >&2
      exit 2
    }
    mkdir -p "$ROOT/target/ios"
    if [[ "$BUILD_COMPANION" == "1" ]]; then
      companion_archive="$ROOT/target/ios/KasKold.xcarchive"
      rm -rf "$companion_archive"
      xcodebuild archive -project "$COMPANION_PROJECT" -scheme KasKold -configuration Release \
        -destination 'generic/platform=iOS' -archivePath "$companion_archive" \
        KASKOLD_IOS_RUNTIME_SYNCED=1 "DEVELOPMENT_TEAM=$team"
      [[ -d "$companion_archive" ]] || { echo "ERROR: Companion iOS Release archive was not produced." >&2; exit 1; }
      printf 'Built Companion archive:\n  %s\n' "$companion_archive"
    fi
    if [[ "$BUILD_VAULT" == "1" ]]; then
      vault_archive="$ROOT/target/ios/KasKoldVault.xcarchive"
      rm -rf "$vault_archive"
      xcodebuild archive -project "$VAULT_PROJECT" -scheme KasKoldVault -configuration Release \
        -destination 'generic/platform=iOS' -archivePath "$vault_archive" \
        "DEVELOPMENT_TEAM=$team"
      [[ -d "$vault_archive" ]] || { echo "ERROR: Vault iOS Release archive was not produced." >&2; exit 1; }
      printf 'Built Vault archive:\n  %s\n' "$vault_archive"
    fi
    echo "KasKold iOS release archive complete (products=$PRODUCTS)."
    ;;
  test)
    destination="${KASKOLD_IOS_TEST_DESTINATION:-}"
    if [[ -z "$destination" ]]; then
      selector="$ROOT/tools/build/ios/select_simulator.py"
      [[ -f "$selector" ]] || fail "missing iOS simulator selector: $selector"
      command -v xcrun >/dev/null 2>&1 || fail "xcrun is required to select an iOS simulator."
      destination="$(python3 "$selector")" || exit $?
      echo "KasKold iOS - selected test destination: $destination"
    fi
    if [[ "$BUILD_COMPANION" == "1" ]]; then
      result_bundle="$ROOT/target/ios/KasKoldTests.xcresult"
      mkdir -p "$COMPANION_DERIVED_DATA" "$(dirname "$result_bundle")"
      rm -rf "$result_bundle"
      xcodebuild -project "$COMPANION_PROJECT" -scheme KasKold -configuration Debug -destination "$destination" \
        -derivedDataPath "$COMPANION_DERIVED_DATA" -resultBundlePath "$result_bundle" \
        KASKOLD_IOS_RUNTIME_SYNCED=1 test
      [[ -d "$result_bundle" ]] || { echo "ERROR: Companion iOS tests completed but $result_bundle was not produced." >&2; exit 1; }
      printf 'Companion test result bundle:\n  %s\n' "$result_bundle"
    fi
    # Vault intentionally has no network or XCTest target. Building it against
    # the selected simulator validates the Rust C ABI, Swift bridge, QR/camera
    # code, and linker path.
    if [[ "$BUILD_VAULT" == "1" ]]; then
      build_vault_simulator "$destination"
    fi
    echo "KasKold iOS test/build validation complete (products=$PRODUCTS)."
    ;;
  *)
    fail "unknown iOS build mode: $MODE (expected build, release, or test)"
    ;;
esac
