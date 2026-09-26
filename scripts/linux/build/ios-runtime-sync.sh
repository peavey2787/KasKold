#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
python3 "$ROOT/tools/build/web/build_companion_runtime.py" --mode release
exec python3 "$ROOT/tools/build/ios/sync_runtime.py"
