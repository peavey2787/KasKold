import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[4] / "qa/checks"))
from pathlib import Path
from typing import Any
import importlib.util
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[4]
RUST_SYNTAX_PATH = ROOT / "qa/checks/architecture/core/rust_syntax.py"
RUST_SYNTAX_SPEC = importlib.util.spec_from_file_location("rust_syntax_contract", RUST_SYNTAX_PATH)
assert RUST_SYNTAX_SPEC is not None and RUST_SYNTAX_SPEC.loader is not None
rust_syntax: Any = importlib.util.module_from_spec(RUST_SYNTAX_SPEC)
RUST_SYNTAX_SPEC.loader.exec_module(rust_syntax)


class OnlineWatcherTestCompileContractTests(unittest.TestCase):
    def test_rust_syntax_gate_rejects_stray_prose_in_function_declarations(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "broken.rs"
            source = "#[test]\nfn valid_prefix stray_token() {}\n"
            path.write_text(source)
            errors = rust_syntax._function_declaration_errors(path, source, root)
        self.assertEqual(len(errors), 1)
        self.assertIn("unexpected token 'stray_token'", errors[0])


if __name__ == "__main__":
    unittest.main()
