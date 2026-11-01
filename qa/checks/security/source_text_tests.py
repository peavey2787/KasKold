#!/usr/bin/env python3
"""Refuse Python tests that assert over production source text.

A test that greps Rust, JavaScript, Kotlin or Swift proves a token is present,
not that the code behaves. Behaviour is proven by the firmware simulator, the
real-DOM suite and each language's own tests. A rule that truly is about source
text belongs in qa/contracts/security/invariants.json, where it can state why.
"""

from __future__ import annotations

import ast
from collections.abc import Iterable
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
TESTS_ROOT = ROOT / "qa/tests"
PRODUCTION_SOURCE = re.compile(r"^(?:apps|crates)/|(?:^|/)src/[\w./-]+\.(?:rs|js|mjs|kt|swift)$")
FILE_READS = {"read_text", "read_bytes"}
PARSED = {"loads", "load", "parse", "safe_load", "sha256"}
OWN_FILES = {"write_text", "write_bytes", "TemporaryDirectory", "mkdtemp"}


def _names_production_source(node: ast.AST) -> bool:
    return any(
        isinstance(child, ast.Constant)
        and isinstance(child.value, str)
        and PRODUCTION_SOURCE.search(child.value) is not None
        for child in ast.walk(node)
    )


def _production_paths(scopes: list[ast.AST]) -> set[str]:
    """Shared constants that point into production source."""
    names: set[str] = set()
    grew = True
    while grew:
        before = len(names)
        for scope in scopes:
            for target, value in _bindings(scope):
                if _names_production_source(value) or _referenced(value) & names:
                    names |= _referenced(target)
        grew = len(names) != before
    return names


def _called(node: ast.AST) -> set[str]:
    names: set[str] = set()
    for child in ast.walk(node):
        if isinstance(child, ast.Call):
            target = child.func
            names.add(getattr(target, "attr", getattr(target, "id", "")))
    return names


def _referenced(node: ast.AST) -> set[str]:
    names: set[str] = set()
    for child in ast.walk(node):
        if isinstance(child, ast.Name):
            names.add(child.id)
        elif isinstance(child, ast.Attribute) and getattr(child.value, "id", "") in ("self", "cls"):
            names.add(f"self.{child.attr}")
    return names


def _text_readers(tree: ast.AST) -> set[str]:
    """Helpers that hand back a file's text, directly or through another helper."""
    helpers = [
        node
        for node in ast.walk(tree)
        if isinstance(node, ast.FunctionDef) and not node.name.startswith("test")
    ]
    readers: set[str] = set()
    grew = True
    while grew:
        found = {
            helper.name
            for helper in helpers
            if _called(helper) & (FILE_READS | readers) and not _called(helper) & PARSED
        }
        grew = not found <= readers
        readers |= found
    return readers


def _bindings(scope: ast.AST) -> Iterable[tuple[ast.AST, ast.AST]]:
    for node in ast.walk(scope):
        if isinstance(node, ast.Assign):
            yield from ((target, node.value) for target in node.targets)
        elif isinstance(node, ast.AugAssign):
            yield node.target, node.value
        elif isinstance(node, ast.For):
            yield node.target, node.iter


def _text_names(scopes: list[ast.AST], readers: set[str], known: set[str]) -> set[str]:
    """Names bound to file text; parsing it into data ends the trail."""
    names = set(known)
    grew = True
    while grew:
        before = len(names)
        for scope in scopes:
            for target, value in _bindings(scope):
                carries_text = _called(value) & (FILE_READS | readers) or _referenced(value) & names
                if carries_text and not _called(value) & PARSED:
                    names |= _referenced(target)
        grew = len(names) != before
    return names


def _is_assertion(node: ast.AST) -> bool:
    return (
        isinstance(node, ast.Call)
        and isinstance(node.func, ast.Attribute)
        and node.func.attr.startswith("assert")
        and node.func.attr != "assertRaises"
    )


def _shared_scopes(tree: ast.Module) -> list[ast.AST]:
    scopes: list[ast.AST] = [
        node for node in tree.body if isinstance(node, (ast.Assign, ast.AugAssign, ast.For))
    ]
    for test_case in (node for node in ast.walk(tree) if isinstance(node, ast.ClassDef)):
        scopes += [
            member
            for member in test_case.body
            if isinstance(member, ast.Assign)
            or (isinstance(member, ast.FunctionDef) and member.name in ("setUp", "setUpClass"))
        ]
    return scopes


def file_count(source: str) -> int:
    """Assertions over file text in a test module that reads production source."""
    tree = ast.parse(source)
    readers = _text_readers(tree)
    scopes = _shared_scopes(tree)
    shared = _text_names(scopes, readers, set())
    production = _production_paths(scopes)
    count = 0
    for test in (node for node in ast.walk(tree) if isinstance(node, ast.FunctionDef)):
        names_source = _names_production_source(test) or _referenced(test) & (production | shared)
        if not names_source or _called(test) & OWN_FILES:
            continue
        text = _text_names([test], readers, shared)
        count += sum(
            1
            for node in ast.walk(test)
            if _is_assertion(node)
            and not _called(node) & PARSED
            and (_called(node) & (FILE_READS | readers) or _referenced(node) & text)
        )
    return count


def scan(root: Path = TESTS_ROOT) -> dict[str, int]:
    counts: dict[str, int] = {}
    for path in sorted(root.rglob("test_*.py")):
        count = file_count(path.read_text(encoding="utf-8"))
        if count:
            counts[path.relative_to(ROOT).as_posix()] = count
    return counts


def check() -> tuple[list[str], int]:
    current = scan()
    errors = [
        f"{path}: {count} assertions over source text; test behaviour, "
        "or state the rule in qa/contracts/security/invariants.json"
        for path, count in current.items()
    ]
    return errors, sum(current.values())


def main() -> int:
    errors, _ = check()
    for error in errors:
        print(f"ERROR: {error}")
    if errors:
        return 1
    print("PASS: no Python test asserts over production source text")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
