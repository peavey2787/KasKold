from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from architecture.protocols import online_paths  # noqa: E402


class OnlineWatcherPathPolicyTests(unittest.TestCase):
    def _check(self, relative: str, source: str) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "crates/online-watcher/src" / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source)
            return online_paths.check(root)

    def test_rejects_moved_root_module(self) -> None:
        errors = self._check("network/query.rs", "use crate::bip32::WalletData;\n")
        self.assertTrue(any("crate::account::bip32" in error for error in errors))


    def test_rejects_duplicate_protocol_path_in_grouped_import(self) -> None:
        errors = self._check(
            "facade.rs",
            "use crate::protocol::{pskt, protocol::transaction};\n",
        )
        self.assertTrue(any("repeats the protocol module" in error for error in errors))

    def test_accepts_transaction_inside_protocol_group(self) -> None:
        errors = self._check(
            "facade.rs",
            "use crate::protocol::{pskt, transaction};\n",
        )
        self.assertEqual(errors, [])

    def test_ignores_moved_path_text_in_comments(self) -> None:
        errors = self._check("network/query.rs", "// use crate::bip32::WalletData;\n")
        self.assertEqual(errors, [])



    def test_rejects_low_level_locktime_parser_in_wasm_family(self) -> None:
        errors = self._check(
            "wasm_api/contracts/covenant/families/escrow/timelocked.rs",
            "use crate::protocol::script::extract_csv_sequence;\n",
        )
        self.assertTrue(any("normalized locktime façade" in error for error in errors))

    def test_accepts_normalized_locktime_facade_in_wasm_family(self) -> None:
        errors = self._check(
            "wasm_api/contracts/covenant/families/escrow/timelocked.rs",
            "use super::super::extract_csv_sequence;\n",
        )
        self.assertEqual(errors, [])

    def test_rejects_unqualified_nested_opcode_alias(self) -> None:
        errors = self._check(
            "contracts/covenant/script/savings.rs",
            "fn build() {\n    use covenant_ops::*;\n}\n",
        )
        self.assertTrue(any("parent opcode alias" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
