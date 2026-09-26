# This file is sourced by qa/linux/run-all.sh. If a file manager executes it
# directly, explain that it is a support library and keep the terminal visible.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
    # shellcheck source=qa/linux/lib/terminal_pause.sh
    source "${ROOT_DIR}/qa/linux/lib/terminal_pause.sh"
    kaskold_qa_install_exit_handler "QA runner support library"
    printf 'ERROR: %s is sourced by qa/linux/run-all.sh and is not a standalone QA entrypoint.\n' \
        "${BASH_SOURCE[0]}" >&2
    exit 2
fi

# Stable test catalog and step dispatch for run-all.sh.

declare -a STEPS=()
declare -A STEP_SCOPES=()
CATALOG_PATH="${ROOT_DIR}/qa/config/run_all_steps.tsv"
while IFS=$'\t' read -r scope category workspace id description; do
    [[ -n "$scope" && "$scope" != \#* ]] || continue
    STEPS+=("${category}|${workspace}|${id}|${description}")
    STEP_SCOPES["$id"]="$scope"
done < "$CATALOG_PATH"


run_step() {
    local id="$1"
    KASKOLD_STEP_SKIPPED=false
    case "$id" in
        preflight.companion-build)
            require_command rustup || return
            run_in_directory "$ROOT_DIR" bash scripts/linux/build/kaskold-companion-web-build.sh
            ;;
        preflight.vault-web-build)
            require_command python3 || return
            require_command rustup || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/web/check_vault_web_architecture.py || return $?
            run_in_directory "$ROOT_DIR" bash scripts/linux/build/kaskold-vault-web-build.sh
            ;;
        preflight.firmware-source-contracts)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/firmware/check_firmware_source_contracts.py
            ;;
        preflight.repository-lockfiles)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/workspace/check_lockfile.py
            ;;
        preflight.crap-check)
            require_command bash || return
            run_in_directory "$ROOT_DIR" bash qa/linux/run-pinned-branch-coverage.sh
            ;;
        preflight.core-ci)
            run_core_ci_gate || return $?
            CORE_CI_TESTS_COMPLETE=true
            ;;
        preflight.security-assurance)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/security_invariants.py || return $?
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/watcher_only_apps.py || return $?
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/irreversible_action_policy.py || return $?
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/test_quality.py || return $?
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/repository_test_quality.py || return $?
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/security_control_evidence.py
            ;;
        preflight.cargo-resolution) run_all_cargo_resolutions ;;
        unit.shared-signer)
            run_cargo_test --manifest-path Cargo.toml -p shared-signer --all-features --locked ;;
        unit.kaskold-hardware-core)
            run_cargo_test --manifest-path Cargo.toml -p kaskold-hardware-core --all-features --locked ;;
        unit.offline-signer)
            run_cargo_test --manifest-path Cargo.toml -p offline-signer --all-features --locked ;;
        unit.online-watcher)
            run_cargo_test --manifest-path Cargo.toml -p online-watcher --all-features --locked ;;
        unit.kaskold-companion-web)
            run_cargo_test --manifest-path apps/kaskold-companion-web/Cargo.toml --lib --locked ;;
        unit.kaskold-vault-web)
            run_cargo_test --manifest-path apps/kaskold-vault-web/Cargo.toml --lib --locked ;;
        unit.signer-firmware) run_firmware_unit_compilation ;;
        unit.external-rqrr)
            run_cargo_test --manifest-path external/rqrr-nostd/Cargo.toml --all-features --locked ;;
        unit.tools)
            # A resumed run can skip preflight.cargo-resolution, so revalidate
            # the independent tools lock before invoking cargo test --locked.
            run_cargo_metadata_check "$ROOT_DIR/tools" "Cargo.toml" || return $?
            run_cargo_test --manifest-path tools/Cargo.toml --lib --bins --locked ;;
        static.qa-orchestration-catalog)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/workspace/check_qa_orchestration.py
            ;;
        unit.repository-python-qa)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/workspace/run_repository_python_qa.py --scope shared
            ;;
        integration.shared-signer-conformance)
            run_cargo_test --manifest-path qa/Cargo.toml --test conformance --locked ;;
        integration.repository-layout)
            run_cargo_test --manifest-path qa/Cargo.toml --test integration --locked ;;
        integration.offline-signer-firmware-signing)
            run_cargo_test --manifest-path qa/Cargo.toml --test tooling_firmware_signing --locked ;;
        integration.online-watcher-source)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/web/check_web_javascript.py
            ;;
        integration.kaskold-companion-web-generated)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 tools/build/web/build_web_index.py --check || return $?
            run_in_directory "$ROOT_DIR" python3 tools/build/web/build_app_css.py --check || return $?
            run_in_directory "$ROOT_DIR" python3 qa/checks/web/check_web_dom_contract.py || return $?
            run_in_directory "$ROOT_DIR" python3 qa/checks/web/check_safe_html.py || return $?
            run_in_directory "$ROOT_DIR" node --test qa/checks/web/safe_html_hostile.test.mjs || return $?
            run_in_directory "$ROOT_DIR" node qa/checks/web/network_routing.test.mjs || return $?
            ;;
        integration.kaskold-companion-web-browser)
            require_command node || return
            run_in_directory "$ROOT_DIR" node qa/checks/web/check_web_runtime.mjs || return $?
            run_in_directory "$ROOT_DIR" node qa/checks/web/check_web_covenant_interactions.mjs || return $?
            run_in_directory "$ROOT_DIR" node qa/checks/web/covenant_sign_protocol.test.mjs || return $?
            run_in_directory "$ROOT_DIR" node qa/checks/web/check_web_critical_paths.mjs || return $?
            ;;
        static.firmware-assurance-contracts)
            require_command python3 || return
            local contract
            for contract in \
                qa/checks/firmware/board_partition_contract.py \
                qa/checks/firmware/m5stack_production_security.py \
                qa/checks/firmware/production_e2e_coverage.py \
                qa/checks/firmware/production_runtime_qualification.py \
                qa/checks/firmware/production_ui_graph.py \
                qa/checks/firmware/wallet_recovery_contract.py; do
                run_in_directory "$ROOT_DIR" python3 "$contract" || return $?
            done
            ;;
        coverage.critical-branch-targets)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/branch_ratchets.py || return $?
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/branch_ratchets.py --require-target
            ;;
        integration.real-node)
            run_in_directory "$ROOT_DIR" bash qa/linux/run-real-node-integration.sh
            ;;
        integration.funded-testnet-e2e)
            run_in_directory "$ROOT_DIR" bash qa/linux/run-funded-testnet-e2e.sh || {
                local funded_status=$?
                if ((funded_status != 77)); then return "$funded_status"; fi
                KASKOLD_STEP_SKIPPED=true
                printf '  ~ SKIP: funded testnet E2E requires an interactive maintainer terminal.\n'
            }
            ;;
        mutation.repository-security-fresh)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/mutation.py run --fresh
            ;;
        mutation.repository-crypto-certification)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/security/mutation.py crypto-check
            ;;
        integration.signer-firmware-builds)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/firmware/check_firmware_builds.py
            ;;
        integration.signer-firmware-lints)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/firmware/check_firmware_lints.py
            ;;
        integration.repository-architecture)
            require_command python3 || return
            run_in_directory "$ROOT_DIR" python3 qa/checks/check_architecture.py || return $?
            ;;
        emulation.signer-firmware-qemu)
            run_in_directory "$ROOT_DIR" scripts/linux/qemu/test.sh ;;
        hardware.signer-firmware-device) run_firmware_hardware_tests ;;
        bench.shared-signer-protocol-throughput)
            require_command cargo || return
            # A resumed benchmark can bypass the global Cargo-resolution preflight.
            # Re-verify/repair the QA lock graph here so --locked remains fail-closed.
            run_cargo_metadata_check "$ROOT_DIR/qa" "Cargo.toml" || return $?
            run_in_directory "$ROOT_DIR" env CARGO_TARGET_DIR="$ROOT_DIR/target/qa" \
                cargo bench --manifest-path qa/Cargo.toml --bench protocol_throughput --locked
            ;;
        fuzz.repository-security-targets) run_fuzz_targets ;;
        *) printf 'ERROR: unknown catalog step: %s\n' "$id" >&2; return 2 ;;
    esac
}

step_supports_test_filter() {
    case "$1" in
        unit.shared-signer|unit.kaskold-hardware-core|unit.offline-signer|unit.online-watcher|unit.kaskold-companion-web|unit.kaskold-vault-web|\
        unit.external-rqrr|unit.tools|integration.shared-signer-conformance|\
        integration.repository-layout|integration.offline-signer-firmware-signing)
            return 0 ;;
        *) return 1 ;;
    esac
}
