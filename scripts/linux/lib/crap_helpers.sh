#!/usr/bin/env bash
# Shared presentation helpers for the Linux CRAP pipeline.

file_size() {
    wc -c < "$1" | tr -d '[:space:]'
}

print_artifacts() {
    local summary_line=""
    if [[ -f "$OUTPUT_DIR/crap_summary.json" ]]; then
        summary_line="$(python3 - "$OUTPUT_DIR/crap_summary.json" <<'PY'
import json
from pathlib import Path
import sys
summary = json.loads(Path(sys.argv[1]).read_text())
production = summary["scopes"]["production"]
status = production["status"]
print(
    f"{production['functions']} production functions; "
    f"{status['fail']} failures; {status['warning']} warnings"
)
PY
)"
    fi
    cat <<EOF_ARTIFACTS
[CRAP 4/4] PASS: reports classified and checked.
Fresh CRAP artifacts are ready while the remaining QA tests run:
  Full report:       $OUTPUT_DIR/crap_report_full.txt
  Production report: $OUTPUT_DIR/crap_report_prod.txt
  Tests report:      $OUTPUT_DIR/crap_report_tests.txt
  External report:   $OUTPUT_DIR/crap_report_external.txt
  Tools report:      $OUTPUT_DIR/crap_report_tools.txt
  Summary:           $OUTPUT_DIR/crap_summary.json
  Health audit:      $OUTPUT_DIR/health_summary.json
  LCOV data:         $OUTPUT_DIR/lcov.info
  Coverage log:      $OUTPUT_DIR/coverage_run.txt
  Run manifest:      $OUTPUT_DIR/run.json
  Browser recovery:  $OUTPUT_DIR/browser_recovery/summary.json
  Web runtime map:    $OUTPUT_DIR/web_runtime/summary.json
Committed quality ratchet:
  Contract:          $RATCHET_PATH
EOF_ARTIFACTS
    [[ -z "$summary_line" ]] || printf '  Production result: %s\n' "$summary_line"
    printf '\nThe CRAP analysis is complete. The remaining QA catalog starts now.\n\n'
}
