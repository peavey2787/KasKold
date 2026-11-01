#!/usr/bin/env python3
"""Regression coverage for the public documentation and platform QA layout."""
from __future__ import annotations

from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from architecture.core.common import is_generated_tree  # noqa: E402

DOCS = ROOT / "docs"
QA = ROOT / "qa"


class RepositoryOrganizationTests(unittest.TestCase):


    def test_native_qa_entrypoints_are_mirrored(self) -> None:
        linux = {
            path.relative_to(QA / "linux").with_suffix("").as_posix()
            for path in (QA / "linux").rglob("*.sh")
            if not {"runner", "lib"} & set(path.relative_to(QA / "linux").parts)
        }
        windows = {
            path.relative_to(QA / "windows").with_suffix("").as_posix()
            for path in (QA / "windows").rglob("*.ps1")
        }
        self.assertEqual(linux, windows)


    def test_public_docs_only_advertise_real_make_targets(self) -> None:
        import re

        makefile = (ROOT / "Makefile").read_text(encoding="utf-8")
        targets = set(re.findall(r"^([A-Za-z0-9_.-]+):(?:\s|$)", makefile, re.MULTILINE))
        public = [
            ROOT / "README.md",
            ROOT / "CONTRIBUTING.md",
            ROOT / "SECURITY.md",
            *DOCS.rglob("*.md"),
            *[path for path in (ROOT / "apps").rglob("README.md") if not is_generated_tree(path, ROOT)],
            *[path for path in (ROOT / "crates").rglob("README.md") if not is_generated_tree(path, ROOT)],
            ROOT / "qa/release/README.md",
        ]
        stale: list[str] = []
        for path in public:
            if not path.is_file():
                continue
            in_fence = False
            for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                stripped = line.strip()
                if stripped.startswith("```"):
                    in_fence = not in_fence
                    continue
                candidates: list[str] = [
                    span for span in re.findall(r"`([^`]*)`", line)
                    if re.search(r"\bmake\s+[A-Za-z0-9_.-]+", span)
                ]
                if in_fence and re.match(r"^(?:[A-Z_][A-Z0-9_]*=[^ ]+\s+)*make\s+", stripped):
                    candidates.append(stripped)
                for candidate in candidates:
                    for match in re.finditer(r"\bmake\s+([A-Za-z0-9_.-]+)", candidate):
                        target = match.group(1)
                        if target not in targets:
                            stale.append(f"{path.relative_to(ROOT)}:{line_number}: make {target}")
        self.assertEqual(stale, [])


    def test_public_markdown_local_links_resolve_after_reorganization(self) -> None:
        import re
        paths = [
            ROOT / "README.md", ROOT / "CONTRIBUTING.md", ROOT / "SECURITY.md", ROOT / "CHANGELOG.md",
            *DOCS.rglob("*.md"), * (ROOT / ".github").rglob("*.md"),
        ]
        missing: list[str] = []
        for path in paths:
            if not path.is_file():
                continue
            for match in re.finditer(r"\[[^\]]*\]\(([^)]+)\)", path.read_text(encoding="utf-8")):
                target = match.group(1).split("#", 1)[0].strip()
                if not target or "://" in target or target.startswith(("mailto:", "#")):
                    continue
                if not (path.parent / target).resolve().exists():
                    missing.append(f"{path.relative_to(ROOT)} -> {target}")
        self.assertEqual(missing, [])


if __name__ == "__main__":
    unittest.main()
