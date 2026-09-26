#!/usr/bin/env bash
# Shared terminal-exit behavior for user-facing Linux QA launchers.
#
# Direct interactive launches print an unambiguous PASS/FAIL result and pause
# before the terminal closes. Nested QA launchers, GNU Make recipes, CI jobs,
# and non-interactive invocations never pause.

if [[ "${KASKOLD_QA_LAUNCHER_ACTIVE:-0}" == "1" ]]; then
    KASKOLD_QA_LAUNCHER_TOP_LEVEL=false
else
    KASKOLD_QA_LAUNCHER_TOP_LEVEL=true
    export KASKOLD_QA_LAUNCHER_ACTIVE=1
fi

kaskold_qa_should_pause() {
    # run-all.sh keeps its documented --pause switch. An explicit request wins
    # even when stdin/stdout are not terminals, matching the old behavior.
    if [[ "${PAUSE_ON_EXIT:-false}" == "true" ]]; then
        return 0
    fi

    $KASKOLD_QA_LAUNCHER_TOP_LEVEL || return 1
    [[ "${KASKOLD_QA_NO_PAUSE:-0}" != "1" ]] || return 1
    [[ "${CI:-}" != "1" && "${CI:-}" != "true" ]] || return 1
    [[ "${MAKELEVEL:-0}" =~ ^0*$ ]] || return 1
    [[ -t 0 && -t 1 ]] || return 1
}

kaskold_qa_exit_handler() {
    local status=$?
    trap - EXIT

    if $KASKOLD_QA_LAUNCHER_TOP_LEVEL; then
        {
            printf '\n================================================================================\n'
            if ((status == 0)); then
                printf 'PASS: %s completed successfully.\n' "$KASKOLD_QA_LAUNCHER_LABEL"
            elif ((status == 77)); then
                printf 'SKIP: %s is not eligible in this environment (exit 77).\n' "$KASKOLD_QA_LAUNCHER_LABEL"
            else
                printf 'FAIL: %s exited with code %s.\n' "$KASKOLD_QA_LAUNCHER_LABEL" "$status"
            fi
            printf '================================================================================\n'
        } >&2
    fi

    if kaskold_qa_should_pause; then
        if [[ -t 0 ]]; then
            read -r -p 'Press Enter to close this terminal...' _ || true
        else
            printf 'Press Enter to close this terminal...'
            read -r _ || true
        fi
    fi

    exit "$status"
}

kaskold_qa_install_exit_handler() {
    KASKOLD_QA_LAUNCHER_LABEL="$1"
    trap kaskold_qa_exit_handler EXIT
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    kaskold_qa_install_exit_handler "QA terminal helper"
    printf 'ERROR: %s is a support library and is not a standalone QA entrypoint.\n' \
        "${BASH_SOURCE[0]}" >&2
    exit 2
fi
