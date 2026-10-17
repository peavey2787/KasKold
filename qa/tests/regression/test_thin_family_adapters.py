from __future__ import annotations

import pathlib
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
CHECKS = ROOT / "qa/checks"
if str(CHECKS) not in sys.path:
    sys.path.insert(0, str(CHECKS))

from architecture.core.duplication import _thin_shared_boundary_pair  # noqa: E402

VAULT_ADAPTER = """
pub async fn {kind}_vault_genesis_pskb() {{ build_vault_genesis_pskb(VaultGenesisKind::{kind}).await }}
pub async fn {kind}_vault_spend_pskb() {{ build_vault_spend_pskb(VaultSpendKind::{kind}).await }}
"""


class ThinFamilyAdapterTests(unittest.TestCase):
    def _pair(self, left: str, right: str) -> bool:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            paths = (root / "left.rs", root / "right.rs")
            for path, text in zip(paths, (left, right)):
                path.write_text(text)
            return _thin_shared_boundary_pair(*paths)

    def test_vault_kind_adapters_that_only_delegate_are_thin(self) -> None:
        self.assertTrue(
            self._pair(VAULT_ADAPTER.format(kind="Split"), VAULT_ADAPTER.format(kind="Tagged"))
        )

    def test_vault_adapter_that_plans_itself_is_not_thin(self) -> None:
        planning = VAULT_ADAPTER.format(kind="Split") + "fn f() { prepare_vault_spend_material(); }"
        self.assertFalse(self._pair(planning, VAULT_ADAPTER.format(kind="Tagged")))

    def test_adapter_missing_a_shared_planner_is_not_thin(self) -> None:
        genesis_only = "pub async fn g() { build_vault_genesis_pskb().await }"
        self.assertFalse(self._pair(genesis_only, VAULT_ADAPTER.format(kind="Tagged")))

    def test_pairs_from_different_families_are_not_thin(self) -> None:
        withdrawal = "fn w() { create_withdrawal(); create_topup(); }"
        self.assertFalse(self._pair(withdrawal, VAULT_ADAPTER.format(kind="Tagged")))


if __name__ == "__main__":
    unittest.main()
