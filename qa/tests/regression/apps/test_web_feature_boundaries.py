"""A feature domain that reaches around its boundary module is refused."""
from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks"))

from architecture.web.web_boundaries import check_feature_boundaries  # noqa: E402


def tree(files: dict[str, str]) -> tuple[tempfile.TemporaryDirectory[str], Path]:
    holder = tempfile.TemporaryDirectory()
    root = Path(holder.name)
    for relative, source in {"wasm/api.js": "", "app/state/index.js": "", **files}.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source)
    return holder, root


class FeatureBoundaries(unittest.TestCase):
    def errors(self, files: dict[str, str]) -> list[str]:
        holder, root = tree(files)
        with holder:
            return check_feature_boundaries(root)

    def test_a_domain_using_its_boundary_passes(self) -> None:
        self.assertEqual(self.errors({
            "features/wallet/boundary.js": "// note\nexport { sign } from '../../wasm/api.js';\n",
            "features/wallet/send.js": "import { sign } from './boundary.js';\nsign();\n",
        }), [])

    def test_a_static_import_of_wasm_is_refused(self) -> None:
        errors = self.errors({"features/wallet/send.js": "import { sign } from '../../wasm/api.js';\n"})
        self.assertEqual(len(errors), 1)
        self.assertIn("through its boundary.js", errors[0])

    def test_a_dynamic_import_of_state_is_refused(self) -> None:
        errors = self.errors({"features/wallet/send.js": "await import('../../app/state/index.js');\n"})
        self.assertEqual(len(errors), 1)

    def test_a_state_submodule_is_refused(self) -> None:
        errors = self.errors({"features/wallet/send.js": "import { x } from '../../app/state/core/wallet.js';\n"})
        self.assertEqual(len(errors), 1)

    def test_another_domains_boundary_is_refused(self) -> None:
        errors = self.errors({
            "features/stealth/boundary.js": "export { sign } from '../../wasm/api.js';\n",
            "features/stealth/use.js": "import { sign } from './boundary.js';\nsign();\n",
            "features/wallet/send.js": "import { sign } from '../stealth/boundary.js';\n",
        })
        self.assertEqual(len(errors), 1)
        self.assertIn("another domain", errors[0])

    def test_a_boundary_with_logic_is_refused(self) -> None:
        errors = self.errors({"features/wallet/boundary.js": "export const sign = () => 1;\n"})
        self.assertEqual(len(errors), 1)
        self.assertIn("may only re-export", errors[0])

    def test_a_boundary_exporting_a_feature_module_is_refused(self) -> None:
        errors = self.errors({
            "features/wallet/boundary.js": "export { send } from './send.js';\n",
            "features/wallet/send.js": "export const send = 1;\n",
        })
        self.assertEqual(len(errors), 1)
        self.assertIn("other than WASM and state", errors[0])

    def test_an_unused_re_export_is_refused(self) -> None:
        errors = self.errors({
            "features/wallet/boundary.js": "export { sign } from '../../wasm/api.js';\n",
            "features/wallet/send.js": "export const send = 1;\n",
        })
        self.assertEqual(len(errors), 1)
        self.assertIn("never uses: sign", errors[0])

    def test_a_badly_named_module_is_refused(self) -> None:
        errors = self.errors({"features/wallet/sendForm.js": ""})
        self.assertEqual(len(errors), 1)
        self.assertIn("lower_snake_case", errors[0])


if __name__ == "__main__":
    unittest.main()
