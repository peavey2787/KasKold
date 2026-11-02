import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[4] / "qa/checks"))

from pathlib import Path
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[4]


class RetiredRawSignatureProtocolTests(unittest.TestCase):


    def test_live_online_watcher_rng_is_target_scoped_between_wasm_and_native(self) -> None:
        manifest = tomllib.loads((ROOT / "crates/online-watcher/Cargo.toml").read_text())
        self.assertNotIn("getrandom", manifest["dependencies"])

        target = manifest["target"]
        wasm_dependency = target['cfg(target_arch = "wasm32")']["dependencies"]["getrandom"]
        self.assertIsInstance(wasm_dependency, dict)
        self.assertEqual(wasm_dependency.get("version"), "0.2")
        self.assertIn("js", wasm_dependency.get("features", []))

        native_dependency = target['cfg(not(target_arch = "wasm32"))']["dependencies"]["getrandom"]
        if isinstance(native_dependency, dict):
            self.assertEqual(native_dependency.get("version"), "0.2")
            self.assertNotIn("js", native_dependency.get("features", []))
        else:
            self.assertEqual(native_dependency, "0.2")

        for relative in ("Cargo.lock", "apps/kaskold-companion-web/Cargo.lock"):
            lock = tomllib.loads((ROOT / relative).read_text())
            package = next(
                item for item in lock["package"]
                if item["name"] == "online-watcher" and item["version"] == "2.0.0"
            )
            dependency_refs = [
                dependency
                for dependency in package.get("dependencies", [])
                if dependency.split()[0] == "getrandom"
            ]
            self.assertEqual(len(dependency_refs), 1, relative)

            locked_getrandom = [
                item for item in lock["package"]
                if item["name"] == "getrandom"
            ]
            self.assertIn("0.2.17", [item["version"] for item in locked_getrandom], relative)

            dependency_ref = dependency_refs[0].split()
            if len(dependency_ref) == 1:
                self.assertEqual(len(locked_getrandom), 1, relative)
                self.assertEqual(locked_getrandom[0]["version"], "0.2.17", relative)
            else:
                self.assertEqual(dependency_ref[1], "0.2.17", relative)


if __name__ == "__main__":
    unittest.main()
