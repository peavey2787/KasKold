#!/usr/bin/env bash
# Shared release policy loader for firmware tooling. Source this file.
set -euo pipefail

KASKOLD_FIRMWARE_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)
KASKOLD_RELEASE_POLICY="$KASKOLD_FIRMWARE_ROOT/apps/kaskold-hardware/release-policy.env"
[[ -f "$KASKOLD_RELEASE_POLICY" ]] || { echo "ERROR: missing release policy: $KASKOLD_RELEASE_POLICY" >&2; return 2; }
# shellcheck disable=SC1090
source "$KASKOLD_RELEASE_POLICY"

for value in KASKOLD_UPDATE_SEQUENCE KASKOLD_SECURITY_VERSION; do
  [[ "${!value:-}" =~ ^[0-9]+$ ]] || { echo "ERROR: invalid $value in release-policy.env" >&2; return 2; }
done
(( KASKOLD_UPDATE_SEQUENCE >= 1 )) || {
  echo "ERROR: KASKOLD_UPDATE_SEQUENCE must be positive" >&2; return 2;
}
(( KASKOLD_SECURITY_VERSION >= 1 && KASKOLD_SECURITY_VERSION <= 16 )) || {
  echo "ERROR: KASKOLD_SECURITY_VERSION must be 1..16 for ESP32-S3" >&2; return 2;
}
[[ "${KASKOLD_ESPTOOL_VERSION:-}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "ERROR: invalid KASKOLD_ESPTOOL_VERSION in release-policy.env" >&2; return 2;
}

kaskold_package_version() {
  python3 - "$KASKOLD_FIRMWARE_ROOT/apps/kaskold-hardware/Cargo.toml" <<'PY'
import pathlib, re, sys
text = pathlib.Path(sys.argv[1]).read_text()
package = text.split("[package]", 1)[1]
match = re.search(r'^version\s*=\s*"([^"]+)"', package, re.M)
if not match:
    raise SystemExit("ERROR: signer-firmware package version not found")
print(match.group(1))
PY
}
