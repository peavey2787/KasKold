import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
from portal_source import kaskold_source  # noqa: E402

from pathlib import Path
import importlib.util
import os
import unittest
import urllib.request
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
RUNNER_PATH = ROOT / "qa/checks/integration/funded_testnet_e2e.py"


def load_runner():
    spec = importlib.util.spec_from_file_location("funded_testnet_e2e_under_test", RUNNER_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class FakeProcess:
    pid = 4242
    returncode = None

    def poll(self):
        return self.returncode


class FundedTestnetE2EContractTests(unittest.TestCase):
    def test_network_wallet_and_local_address_precede_public_node_status(self):
        runner = RUNNER_PATH.read_text(encoding="utf-8")
        main = runner[runner.index("def main()") : runner.index("def entrypoint()")]
        self.assertLess(main.index("select_network()"), main.index("build_real_wasm()"))
        self.assertLess(main.index("select_network()"), main.index("ensure_wallet(network)"))
        self.assertLess(main.index('"phase": "address"'), main.index("ensure_funding_ready"))
        self.assertLess(main.index("print_wallet_address"), main.index("ensure_funding_ready"))
        self.assertIn('("1", "testnet-10", "Testnet-10")', runner)
        self.assertIn('("2", "testnet-12", "Testnet-12")', runner)
        self.assertIn('"testnet-10": "https://faucet-testnet.kaspanet.io/"', runner)
        self.assertIn('"testnet-12": "https://faucet-tn12.kaspanet.io/"', runner)
        self.assertNotIn("no Enter key is required", runner)

    def test_funded_flow_uses_local_address_then_real_wasm_and_public_resolver(self):
        browser = (ROOT / "qa/checks/integration/funded_testnet_e2e_case.mjs").read_text()
        runner = RUNNER_PATH.read_text(encoding="utf-8")
        signer = (ROOT / "tools/dev/funded_e2e_signer.rs").read_text()
        gitignore = (ROOT / ".gitignore").read_text()
        resolver = (ROOT / "apps/kaskold-companion-web/web/js/core/node/resolver.js").read_text()
        balance = (ROOT / "apps/kaskold-companion-web/web/js/features/wallet/core/balance.js").read_text()

        self.assertIn("if (input.phase === 'address') return addressCase(input, wallet)", browser)
        self.assertLess(browser.index("if (input.phase === 'address')"), browser.index("configureNetwork(input.network)"))
        self.assertIn("withNodeRetry", browser)
        self.assertIn("withFundedNodeRetry", browser)
        self.assertNotIn("await resolvePublicNode()", browser)
        self.assertIn("networkState.customNodeUrl = null", browser)
        self.assertIn("wasm.create_send_pskb", browser)
        self.assertIn("wasm.pskt_relay_to_kspt", browser)
        self.assertIn("wasm.kaskold_sdk_complete", browser)
        self.assertIn("const merged = signed.psktHex", browser)
        self.assertNotIn("wasm.pskt_merge_signed_kspt", browser)
        self.assertIn("wasm.pskt_finalize_and_broadcast", browser)
        self.assertIn("wasm.fetch_utxos_for_address_js", browser)
        self.assertIn("BigInt(value)", browser)
        self.assertNotIn("Number(utxo.amount)", browser)
        self.assertIn("sign_transaction_multi_addr_with_entropy", signer)
        self.assertIn("review_transaction", signer)
        self.assertIn("OsRng.fill_bytes", signer)
        self.assertIn("/target/", gitignore)
        self.assertIn('Path.home() / ".local/state"', runner)
        self.assertIn('"kaskold/funded-e2e"', runner)
        self.assertIn("KASKOLD_FUNDED_E2E_STATE_DIR", runner)
        self.assertIn("globalThis.location?.protocol === 'https:' ? 'tls' : 'any'", resolver)
        self.assertIn("if (security === 'tls') return url.startsWith('wss://')", resolver)
        self.assertIn("activeWsUrl = wsUrl", browser)
        self.assertIn("attemptedWsUrls.push(wsUrl)", browser)
        self.assertIn("ws_urls_attempted", browser)
        self.assertIn("ws_urls_failed", browser)
        self.assertIn("resolver repeated previously failed WebSocket endpoint", browser)
        self.assertIn("return withNodeRetry", browser)
        self.assertIn("return resolvePublicNode()", balance)

    def test_funded_http_server_uses_browser_safe_module_mime_and_ignores_favicon(self):
        runner = load_runner()
        with runner.serve(runner.FundedHandler) as port:
            with urllib.request.urlopen(
                f"http://127.0.0.1:{port}/qa/checks/integration/funded_testnet_e2e_case.mjs"
            ) as response:
                self.assertEqual(response.status, 200)
                self.assertEqual(response.headers.get_content_type(), "text/javascript")
                self.assertIn(b"withFundedNodeRetry", response.read())
            with urllib.request.urlopen(
                f"http://127.0.0.1:{port}/apps/kaskold-companion-web/web/js/core/node/resolver.js"
            ) as response:
                self.assertEqual(response.headers.get_content_type(), "text/javascript")
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/favicon.ico") as response:
                self.assertEqual(response.status, 204)

    def test_funded_browser_page_embeds_harness_instead_of_dynamic_importing_it(self):
        runner = load_runner()
        document = runner.funded_browser_document().decode("utf-8")
        self.assertIn('id="funded-e2e-module" type="module"', document)
        self.assertIn("withFundedNodeRetry", document)
        self.assertIn("import * as wasm from '/target/kaskold-companion-web/site/js/wasm/api.js';", document)
        self.assertNotIn("import('/qa/checks/integration/funded_testnet_e2e_case.mjs')", document)

    def test_initial_funding_status_skips_prompt_when_wallet_is_already_funded(self):
        runner = load_runner()
        funded = {
            "funding_address": "kaspatest:qexample",
            "balance_sompi": "100000000",
            "funded": True,
        }
        with mock.patch.object(runner, "query_funding_status", return_value=funded) as status, mock.patch.object(
            runner, "print_funding_prompt"
        ) as funding_prompt, mock.patch.object(runner, "wait_for_funding_confirmation") as confirmation:
            result = runner.ensure_funding_ready(
                1234, "testnet-10", "Testnet-10", "kpub1:test", "kaspatest:qexample", existing=True
            )
        self.assertTrue(result["funded"])
        status.assert_called_once_with(1234, "testnet-10", "kpub1:test", "kaspatest:qexample")
        funding_prompt.assert_not_called()
        confirmation.assert_not_called()

    def test_unfunded_wallet_prompts_only_after_automatic_status_check(self):
        runner = load_runner()
        unfunded = {"funding_address": "kaspatest:qexample", "balance_sompi": "0", "funded": False}
        funded = {"funding_address": "kaspatest:qexample", "balance_sompi": "100000000", "funded": True}
        with mock.patch.object(runner, "query_funding_status", side_effect=[unfunded, funded]) as status, mock.patch.object(
            runner, "print_funding_prompt"
        ) as funding_prompt, mock.patch.object(runner, "wait_for_funding_confirmation") as confirmation:
            result = runner.ensure_funding_ready(
                1234, "testnet-10", "Testnet-10", "kpub1:test", "kaspatest:qexample", existing=True
            )
        self.assertTrue(result["funded"])
        self.assertEqual(status.call_count, 2)
        funding_prompt.assert_called_once_with(
            "testnet-10", "Testnet-10", "kaspatest:qexample", existing=True
        )
        confirmation.assert_called_once_with()

    def test_funding_status_rejects_public_node_address_mismatch(self):
        runner = load_runner()
        with mock.patch.object(
            runner,
            "status_with_retries",
            return_value={"funding_address": "kaspatest:qother", "balance_sompi": "0", "funded": False},
        ):
            with self.assertRaises(runner.BrowserPhaseError):
                runner.query_funding_status(1234, "testnet-10", "kpub1:test", "kaspatest:qexample")

    def test_browser_websocket_reports_close_and_timeout_stage(self):
        transport = kaskold_source("crates/online-watcher/src/infrastructure/browser_websocket.rs").read_text()
        errors = kaskold_source("crates/online-watcher/src/network/error.rs").read_text()
        cargo = (ROOT / "crates/online-watcher/Cargo.toml").read_text()

        self.assertIn('"CloseEvent"', cargo)
        self.assertIn("set_onclose", transport)
        self.assertIn("CloseEvent", transport)
        self.assertIn("request_sent.set(true)", transport)
        self.assertIn("NetworkError::ConnectTimeout", transport)
        self.assertIn("NetworkError::ResponseTimeout", transport)
        self.assertIn('"WebSocket connect timeout (15s)"', errors)
        self.assertIn('"WebSocket RPC response timeout (15s)"', errors)
        self.assertNotIn("NetworkError::Timeout", transport)


    def test_funded_signer_helper_uses_pinned_stable_rust_and_verified_tools_graph(self):
        wrapper = (ROOT / "qa/linux/run-funded-testnet-e2e.sh").read_text()
        runner = RUNNER_PATH.read_text(encoding="utf-8")
        self.assertIn('source "$ROOT/qa/config/toolchains.env"', wrapper)
        self.assertIn('export KASKOLD_STABLE_RUST', wrapper)
        self.assertIn('source "$ROOT/scripts/linux/lib/rustup_bootstrap.sh"', wrapper)
        self.assertIn('kaskold_ensure_rust_toolchain "$KASKOLD_STABLE_RUST"', wrapper)
        self.assertIn('"rustup"', runner)
        self.assertIn('"run"', runner)
        self.assertIn('toolchain', runner)
        self.assertIn('"--locked"', runner)
        self.assertIn("def ensure_tools_lock_current", runner)
        lock_guard = runner[runner.index("def ensure_tools_lock_current") : runner.index("def run_signer_helper")]
        self.assertIn('"--offline"', lock_guard)
        self.assertIn('original = lockfile.read_bytes()', lock_guard)
        self.assertIn('refreshed tools/Cargo.lock still fails --locked', lock_guard)
        main = runner[runner.index("def main()") : runner.index("def entrypoint()")]
        self.assertLess(main.index("ensure_tools_lock_current()"), main.index("ensure_wallet(network)"))

    def test_browser_close_reason_keeps_browser_only_adapter_complexity_bounded(self):
        transport = kaskold_source("crates/online-watcher/src/infrastructure/browser_websocket.rs").read_text()
        helper = transport[transport.index("fn close_reason") : transport.index("fn complete")]
        self.assertNotIn("if ", helper)
        self.assertNotIn("match ", helper)
        self.assertIn(".take(160)", helper)
        self.assertIn("!character.is_control()", helper)
        self.assertIn("code {}", helper)
        self.assertIn("clean={}", helper)

    def test_funded_wallet_state_defaults_outside_the_repository_tree(self):
        runner = load_runner()
        with mock.patch.dict(runner.os.environ, {}, clear=True), mock.patch.object(
            runner.Path, "home", return_value=Path("/home/tester")
        ):
            self.assertEqual(
                runner.funded_state_root(),
                Path("/home/tester/.local/state/kaskold/funded-e2e"),
            )
        with mock.patch.dict(
            runner.os.environ,
            {"KASKOLD_FUNDED_E2E_STATE_DIR": "/tmp/kaskold-funded"},
            clear=True,
        ):
            self.assertEqual(runner.funded_state_root(), Path("/tmp/kaskold-funded"))

    def test_funding_confirmation_requires_explicit_yes_and_can_quit(self):
        runner = load_runner()
        with mock.patch("builtins.input", side_effect=["", "no", "y"]):
            runner.wait_for_funding_confirmation()
        with mock.patch("builtins.input", return_value="q"):
            with self.assertRaises(SystemExit) as raised:
                runner.wait_for_funding_confirmation()
        self.assertIn("waiting for testnet funding", str(raised.exception))

    def test_funding_prompt_uses_network_specific_faucet(self):
        runner = load_runner()
        with mock.patch("builtins.print") as printed:
            runner.print_funding_prompt(
                "testnet-12", "Testnet-12", "kaspatest:qexample", existing=False
            )
        text = "\n".join(" ".join(map(str, call.args)) for call in printed.call_args_list)
        self.assertIn("https://faucet-tn12.kaspanet.io/", text)
        self.assertNotIn("https://faucet-testnet.kaspanet.io/", text)
        self.assertIn("automatic public-node check did not find enough funding", text)

    def test_interactive_input_does_not_require_tty_stdout(self):
        runner = load_runner()

        class InteractiveInput:
            def isatty(self):
                return True

        self.assertTrue(runner.interactive_stdin_available(InteractiveInput()))

    def test_interactive_prompts_repair_windows_console_line_and_echo_mode(self):
        runner = load_runner()
        source = RUNNER_PATH.read_text(encoding="utf-8")
        self.assertIn("restore_windows_console_line_input()", source)
        self.assertIn("enable_line_input = 0x0002", source)
        self.assertIn("enable_echo_input = 0x0004", source)
        self.assertIn("enable_virtual_terminal_input = 0x0200", source)
        self.assertIn("& ~enable_virtual_terminal_input", source)
        self.assertIn('timed_interactive_input("Network [1]: ", timeout_seconds)', source)
        self.assertIn('NETWORK_SELECTION_TIMEOUT_SECONDS = 120', source)
        self.assertIn('interactive_input("Funding complete and ready to re-check the public node? [y/q]: ")', source)

        with mock.patch.object(runner, "restore_windows_console_line_input") as repair, mock.patch(
            "builtins.input", return_value="1"
        ) as raw_input:
            self.assertEqual(runner.interactive_input("Network [1]: "), "1")
        repair.assert_called_once_with()
        raw_input.assert_called_once_with("Network [1]: ")

        import ctypes
        import types

        class FakeStream:
            def fileno(self):
                return 0

        class FakeKernel32:
            def __init__(self):
                self.updated = None

            def GetConsoleMode(self, _handle, pointer):
                pointer._obj.value = 0x0200
                return 1

            def SetConsoleMode(self, _handle, mode):
                self.updated = mode
                return 1

        kernel32 = FakeKernel32()
        fake_msvcrt = types.SimpleNamespace(get_osfhandle=lambda _fd: 99)
        with mock.patch.object(runner.os, "name", "nt"), mock.patch.dict(
            "sys.modules", {"msvcrt": fake_msvcrt}
        ), mock.patch.object(ctypes, "windll", types.SimpleNamespace(kernel32=kernel32), create=True):
            self.assertTrue(runner.restore_windows_console_line_input(FakeStream()))
        self.assertEqual(kernel32.updated, 0x0007)

    def test_network_selection_times_out_to_testnet_10_after_120_seconds(self):
        runner = load_runner()
        with mock.patch.object(runner, "timed_interactive_input", return_value=None) as timed, mock.patch(
            "builtins.print"
        ) as printed:
            self.assertEqual(runner.select_network(), ("testnet-10", "Testnet-10"))
        timed.assert_called_once_with("Network [1]: ", 120)
        text = "\n".join(" ".join(map(str, call.args)) for call in printed.call_args_list)
        self.assertIn("automatically selecting Testnet-10", text)

    def test_network_selection_still_honors_explicit_testnet_12(self):
        runner = load_runner()
        with mock.patch.object(runner, "timed_interactive_input", return_value="2"):
            self.assertEqual(runner.select_network(), ("testnet-12", "Testnet-12"))

    def test_chromium_timeout_is_controlled_and_phase_specific(self):
        runner = load_runner()
        with mock.patch.object(runner, "browser_executable", return_value="/usr/bin/chromium"), mock.patch.object(
            runner, "launch_chromium", return_value=FakeProcess()
        ), mock.patch.object(runner, "open_chromium_target"), mock.patch.object(
            runner, "stop_chromium"
        ) as stop, mock.patch.object(runner.time, "monotonic", side_effect=[0.0, 61.0]):
            with self.assertRaises(runner.BrowserPhaseError) as raised:
                runner.chromium_result(12345, {"phase": "status", "network": "testnet-12"}, timeout=60)
        message = str(raised.exception)
        self.assertIn("phase 'status' timed out after 60 wall-clock seconds", message)
        self.assertIn("/usr/bin/chromium", message)
        self.assertTrue(raised.exception.retryable)
        stop.assert_called_once()
        self.assertGreaterEqual(runner.STATUS_TIMEOUT_SECONDS, 90)
        self.assertGreaterEqual(runner.RPC_PHASE_TIMEOUT_SECONDS, 180)

    def test_browser_start_failure_is_a_retryable_status_phase_error(self):
        runner = load_runner()
        failure = runner.ChromiumError("Chromium exited before DevTools became ready (exit 21)")
        with mock.patch.object(runner, "browser_executable", return_value="/usr/bin/chromium"), mock.patch.object(
            runner, "launch_chromium", side_effect=failure
        ), mock.patch.object(runner, "stop_chromium") as stop:
            with self.assertRaises(runner.BrowserPhaseError) as raised:
                runner.chromium_result(12345, {"phase": "status", "network": "testnet-10"}, timeout=10)
        self.assertIn("phase 'status' could not start its Chromium test tab", str(raised.exception))
        self.assertIn("exit 21", str(raised.exception))
        self.assertTrue(raised.exception.retryable)
        stop.assert_not_called()

    def test_tab_open_failure_stops_the_launched_browser(self):
        runner = load_runner()
        process = FakeProcess()
        with mock.patch.object(runner, "browser_executable", return_value="/usr/bin/chromium"), mock.patch.object(
            runner, "launch_chromium", return_value=process
        ), mock.patch.object(
            runner, "open_chromium_target", side_effect=OSError("connection refused")
        ), mock.patch.object(runner, "stop_chromium") as stop:
            with self.assertRaises(runner.BrowserPhaseError) as raised:
                runner.chromium_result(12345, {"phase": "send", "network": "testnet-10"}, timeout=10)
        self.assertFalse(raised.exception.retryable)
        stop.assert_called_once_with(process)

    def test_browser_result_callback_drives_completion_without_dom_capture(self):
        runner = load_runner()

        def launch(*_args, **_kwargs):
            runner.FundedHandler.result_payload = {
                "status": "pass",
                "detail": {"phase": "address", "funding_address": "kaspatest:qexample"},
            }
            runner.FundedHandler.result_event.set()
            return FakeProcess()

        with mock.patch.object(runner, "browser_executable", return_value="/usr/bin/chromium"), mock.patch.object(
            runner, "launch_chromium", side_effect=launch
        ), mock.patch.object(runner, "open_chromium_target") as open_target, mock.patch.object(
            runner, "stop_chromium"
        ):
            status, detail = runner.chromium_result(12345, {"phase": "address", "network": "testnet-10"}, timeout=10)
        self.assertEqual(status, "pass")
        self.assertEqual(detail["funding_address"], "kaspatest:qexample")
        open_target.assert_called_once()
        self.assertEqual(open_target.call_args.args[1], "http://127.0.0.1:12345/__qa_funded_e2e__")

    def test_real_network_browser_reports_through_the_result_callback(self):
        browser = (ROOT / "qa/checks/integration/funded_testnet_e2e_case.mjs").read_text()
        self.assertIn("fetch('/__qa_funded_result__'", browser)
        self.assertIn("await run();", browser)
        self.assertIn("Python", browser)


    def test_funded_gate_is_catalogued_after_real_node_and_before_long_campaigns(self):
        catalog = (ROOT / "qa/config/run_all_steps.tsv").read_text(encoding="utf-8").splitlines()
        ids = [line.split("\t")[3] for line in catalog if line and not line.startswith("#")]
        dispatch = (ROOT / "scripts/common/lib/make_tasks.py").read_text(encoding="utf-8")
        entry = ROOT / "scripts/linux/quality/funded-testnet-e2e.sh"
        runner = RUNNER_PATH.read_text(encoding="utf-8")
        self.assertIn('"funded-testnet-e2e": "quality/funded-testnet-e2e"', dispatch)
        self.assertTrue(entry.is_file())
        if os.name == "posix":
            self.assertTrue(entry.stat().st_mode & 0o111)
        self.assertLess(ids.index("integration.real-node"), ids.index("integration.funded-testnet-e2e"))
        self.assertLess(ids.index("integration.funded-testnet-e2e"), ids.index("mutation.repository-security-fresh"))
        self.assertIn("NONINTERACTIVE_SKIP = 77", runner)
        self.assertIn("interactive_stdin_available()", runner)
        self.assertIn("GetConsoleMode", runner)
        self.assertNotIn("sys.stdout.isatty()", runner)


if __name__ == "__main__":
    unittest.main()
