#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
# shellcheck source=qa/linux/lib/terminal_pause.sh
source "$ROOT/qa/linux/lib/terminal_pause.sh"
kaskold_qa_install_exit_handler "Software assurance generation"
OUT="${1:-${KASKOLD_RELEASE_EVIDENCE_DIR:-$ROOT/target/qa/release/evidence}}"
SOURCE_SHA256="${KASKOLD_SOURCE_SHA256:-}"
RELEASE_SHA256="${KASKOLD_RELEASE_ARTIFACT_SHA256:-}"
SIGNER_KEY_ID="${KASKOLD_RELEASE_EVIDENCE_SIGNER_KEY_ID:-}"
SIGNING_KEY="${KASKOLD_RELEASE_EVIDENCE_SIGNING_KEY:-}"
[[ -n "$SOURCE_SHA256" && -n "$RELEASE_SHA256" && -n "$SIGNER_KEY_ID" && -n "$SIGNING_KEY" ]] || {
  echo 'ERROR: set KASKOLD_SOURCE_SHA256, KASKOLD_RELEASE_ARTIFACT_SHA256, KASKOLD_RELEASE_EVIDENCE_SIGNER_KEY_ID, and KASKOLD_RELEASE_EVIDENCE_SIGNING_KEY.' >&2
  exit 2
}
mkdir -p "$OUT/software"
for tool in cargo-deny syft osv-scanner openssl; do
  command -v "$tool" >/dev/null 2>&1 || { echo "ERROR: required release-assurance tool not found: $tool" >&2; exit 2; }
done
(
  cd "$ROOT"
  cargo deny check advisories licenses 2>&1 | tee "$OUT/software/cargo-deny.txt"
  syft dir:. -o cyclonedx-json="$OUT/software/sbom.cdx.json"
  osv-scanner scan source -r . --format json > "$OUT/software/osv.json"
)
python3 "$ROOT/qa/checks/release/generate_software_assurance.py" \
  --evidence-dir "$OUT" \
  --source-sha256 "$SOURCE_SHA256" \
  --release-artifact-sha256 "$RELEASE_SHA256" \
  --signer-key-id "$SIGNER_KEY_ID" \
  --signing-key "$SIGNING_KEY"
