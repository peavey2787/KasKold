#!/usr/bin/env bash
# Read-only post-provision CoreS3 production-security evidence collector.
# This script NEVER burns eFuses and NEVER writes/erases flash.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
# shellcheck source=qa/linux/lib/terminal_pause.sh
source "$ROOT/qa/linux/lib/terminal_pause.sh"
kaskold_qa_install_exit_handler "M5Stack security HIL"
PORT=${1:-}
OUT=${2:-"$ROOT/target/qa/state/m5stack-security-hil"}
[[ -n "$PORT" ]] || { echo "usage: $(basename "$0") <serial-port> [output-dir]" >&2; exit 2; }
# shellcheck source=../../tools/build/firmware/lib/release_policy.sh
source "$ROOT/tools/build/firmware/lib/release_policy.sh"
mkdir -p "$OUT"

install_venv_support() {
  if command -v apt-get >/dev/null 2>&1 && command -v sudo >/dev/null 2>&1; then
    echo "==> Python venv support is missing; installing python3-venv"
    sudo apt-get update
    sudo apt-get install -y python3-venv
    return
  fi
  echo "ERROR: Python venv support is unavailable and automatic apt installation is not possible." >&2
  exit 2
}

ensure_esptool() {
  python3 -m ensurepip --version >/dev/null 2>&1 || install_venv_support
  local pinned="$ROOT/tools/build/firmware/pinned_esptool.py"
  ESPTOOL_PYTHON=$(python3 "$pinned" python)
  ESPTOOL=$(python3 "$pinned" esptool)
  ESPTOOL_VERSION_TEXT=$("$ESPTOOL" version 2>&1)
}

ensure_esptool

# Production CoreS3 uses Secure Download mode. Espressif intentionally blocks
# arbitrary memory/eFuse access there, so post-lock evidence relies on the ROM's
# restricted get-security-info command. Keep a human-readable CLI transcript,
# but make the pass/fail decision from esptool's structured API so output-label
# wording cannot turn a disabled security state into a false PASS.
"$ESPTOOL" --chip esp32s3 --port "$PORT" get-security-info \
    | tee "$OUT/get-security-info.txt"

"$ESPTOOL_PYTHON" \
  "$ROOT/qa/linux/lib/collect_esptool_security.py" \
  "$PORT" "$OUT/security-state.json" "$OUT/security-info-raw.json"

python3 - "$OUT" "$KASKOLD_ESPTOOL_VERSION" "$ESPTOOL_VERSION_TEXT" \
  "$KASKOLD_UPDATE_SEQUENCE" "$KASKOLD_SECURITY_VERSION" <<'PY'
import hashlib, json, pathlib, sys
out = pathlib.Path(sys.argv[1])
state = json.loads((out / "security-state.json").read_text())
report = out / "get-security-info.txt"
raw = out / "security-info-raw.json"
payload = {
    "collector": "read-only-post-provision-v4",
    "esptool_pin": sys.argv[2],
    "esptool_version_output": sys.argv[3],
    "update_sequence": int(sys.argv[4]),
    "security_version_policy": int(sys.argv[5]),
    "security_state": state,
    "report": report.name,
    "report_sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
    "structured_report": raw.name,
    "structured_report_sha256": hashlib.sha256(raw.read_bytes()).hexdigest(),
    "notes": "SECURE_VERSION/eFuse detail is bound from provisioning-prelock evidence plus the dedicated anti-rollback HIL fixture; Secure Download mode intentionally blocks espefuse summary after lockdown.",
}
(out / "collection.json").write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
PY

echo "Read-only M5Stack security evidence collected in: $OUT"
