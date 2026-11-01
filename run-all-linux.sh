#!/usr/bin/env bash
# Complete KasKold validation suite on Linux.
exec "$(dirname "${BASH_SOURCE[0]}")/qa/linux/run-all.sh" "$@"
