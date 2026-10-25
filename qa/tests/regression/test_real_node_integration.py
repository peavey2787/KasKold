import importlib.util
from pathlib import Path
import urllib.request
import unittest

ROOT = Path(__file__).resolve().parents[3]


def load_real_node_runner():
    path = ROOT / "qa/checks/integration/real_node_browser.py"
    spec = importlib.util.spec_from_file_location("kaskold_real_node_browser", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RealNodeIntegrationContractTests(unittest.TestCase):
    def test_gate_uses_real_wasm_and_public_resolver_only(self):
        shell = (ROOT / "qa/linux/run-real-node-integration.sh").read_text()
        runner = (ROOT / "qa/checks/integration/real_node_browser.py").read_text()
        case = (ROOT / "qa/checks/integration/browser_real_node_case.mjs").read_text()
        resolver = (ROOT / "apps/kaskold-companion-web/web/js/core/node/resolver.js").read_text()
        makefile = (ROOT / "Makefile").read_text()
        catalog = (ROOT / "qa/config/run_all_steps.tsv").read_text()

        self.assertIn('pkg/companion_web_bg.wasm', runner)
        self.assertIn('implausibly small; fixture/stub builds are forbidden', runner)
        self.assertIn("withNodeRetry", case)
        self.assertIn("networkState.customNodeUrl = null", case)
        self.assertIn("public-node gate requires mainnet", case)
        self.assertIn("resolver returned a local endpoint", case)
        self.assertIn('wasm.get_virtual_daa_score', case)
        self.assertIn('wasm.get_fee_estimate', case)
        self.assertIn('wasm.fetch_utxos_for_address_js', case)
        self.assertIn('/__qa_real_node_result__', runner)
        self.assertIn('/__qa_real_node_result__', case)
        self.assertIn('time.monotonic()', runner)
        self.assertNotIn('--virtual-time-budget', runner)
        self.assertNotIn('--dump-dom', runner)
        self.assertNotIn('wasm.broadcast_signed', case)

        combined = "\n".join((shell, runner, case, makefile))
        self.assertNotIn('KASPAD_BIN', combined)
        self.assertNotIn('KASPAD_EXTRA_ARGS', combined)
        self.assertNotIn('local_kaspad', combined)
        self.assertNotIn('"--simnet"', combined)
        self.assertNotIn('rpclisten-borsh', combined)
        self.assertNotIn('run-real-node-integration.sh both', combined)
        self.assertIn("globalThis.location?.protocol === 'https:' ? 'tls' : 'any'", resolver)
        self.assertIn("if (security === 'tls') return url.startsWith('wss://')", resolver)
        self.assertIn("/${security}/wrpc/borsh", resolver)
        self.assertIn('official resolver pool', shell)
        self.assertIn('kaskold_reconcile_host_locks', shell)
        self.assertIn('--evidence', shell)
        self.assertIn('complete_hardening.py', shell)
        self.assertIn('parser.add_argument("--evidence"', runner)
        self.assertIn('write_evidence(args.evidence, detail)', runner)
        self.assertIn('integration.real-node', catalog)
        self.assertIn('tempfile.mkdtemp(prefix="kaskold-real-node-chrome-")', runner)
        self.assertIn('stop_chromium(process)', runner)
        self.assertIn('cleanup_chromium_profile(profile_dir)', runner)
        self.assertNotIn('TemporaryDirectory(prefix="kaskold-real-node-chrome-")', runner)


    def test_http_server_uses_browser_safe_module_mime_and_ignores_favicon(self):
        runner = load_real_node_runner()
        with runner.serve(runner.RealNodeHandler) as port:
            with urllib.request.urlopen(
                f"http://127.0.0.1:{port}/qa/checks/integration/browser_real_node_case.mjs"
            ) as response:
                self.assertEqual(response.status, 200)
                self.assertEqual(response.headers.get_content_type(), "text/javascript")
                self.assertIn(b"withNodeRetry", response.read())
            with urllib.request.urlopen(
                f"http://127.0.0.1:{port}/apps/kaskold-companion-web/web/js/core/node/resolver.js"
            ) as response:
                self.assertEqual(response.headers.get_content_type(), "text/javascript")
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/favicon.ico") as response:
                self.assertEqual(response.status, 204)


if __name__ == "__main__":
    unittest.main()
