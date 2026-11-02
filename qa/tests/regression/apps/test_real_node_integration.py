import importlib.util
from pathlib import Path
import urllib.request
import unittest

ROOT = Path(__file__).resolve().parents[4]


def load_real_node_runner():
    path = ROOT / "qa/checks/integration/real_node_browser.py"
    spec = importlib.util.spec_from_file_location("kaskold_real_node_browser", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RealNodeIntegrationContractTests(unittest.TestCase):


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
