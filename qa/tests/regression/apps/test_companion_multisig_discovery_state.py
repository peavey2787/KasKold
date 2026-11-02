#!/usr/bin/env python3
"""Companion sealed-state contracts, including multisig branch discovery."""
import sys as _portal_sys
from pathlib import Path
import re
import unittest

_portal_sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "qa/checks"))

ROOT = Path(__file__).resolve().parents[4]
WEB_JS = ROOT / "apps/kaskold-companion-web/web/js"
STATE_ROOT = WEB_JS / "app/state"


class CompanionMultisigDiscoveryStateTests(unittest.TestCase):


    def test_every_direct_assignment_to_a_sealed_state_has_a_declared_field(self) -> None:
        sealed: dict[str, tuple[Path, set[str]]] = {}
        pattern = re.compile(r"export const (\w+) = Object\.seal\(\{([\s\S]*?)\}\);")
        key_pattern = re.compile(r"['\"]([A-Za-z_$][\w$]*)['\"]\s*:")
        for path in STATE_ROOT.rglob("*.js"):
            match = pattern.search(path.read_text())
            if match:
                name, body = match.groups()
                sealed[name] = (path, set(key_pattern.findall(body)))

        errors: list[str] = []
        sources = [(path, path.read_text(errors="ignore")) for path in WEB_JS.rglob("*.js")]
        for name, (state_path, declared) in sealed.items():
            assignment = re.compile(rf"\b{re.escape(name)}\.([A-Za-z_$][\w$]*)\s*=")
            for source_path, source in sources:
                for field in assignment.findall(source):
                    if field not in declared:
                        errors.append(
                            f"{source_path.relative_to(ROOT)} assigns undeclared sealed field "
                            f"{name}.{field}; declare it in {state_path.relative_to(ROOT)}"
                        )
        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
