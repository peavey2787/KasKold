#!/usr/bin/env python3
"""Audit security-critical Rust tests for assertions and required evidence."""

from __future__ import annotations

import argparse
import sys
import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
POLICY = ROOT / "qa/checks/security/policy.json"
sys.path.insert(0, str(ROOT / "qa/checks"))
from portal_source import display_path, kaskold_glob  # noqa: E402
DEFAULT_OUTPUT = ROOT / "target/qa/security/test-quality.json"
TEST_HEADER = re.compile(r"#\s*\[\s*(?:[A-Za-z_][\w:]*::)?test(?:\([^]]*\))?\s*\]")
FUNCTION = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\([^)]*\)[^{]*\{")
ASSERTION = re.compile(r"\b(assert|assert_eq|assert_ne|debug_assert|panic|unreachable)!\s*\(")
SIMPLE_LITERAL = re.compile(
    r"^(?:true|false|None|-?[0-9][0-9_]*(?:[iu](?:8|16|32|64|128|size))?"
    r"|0x[0-9a-fA-F_]+|0b[01_]+|\"(?:[^\"\\]|\\.)*\"|'(?:[^'\\]|\\.)')$"
)
_OPENERS = {"(": "paren", "[": "bracket", "{": "brace"}
_CLOSERS = {")": "paren", "]": "bracket", "}": "brace"}


class _SourceScanner:
    """Track string, char and bracket state while walking Rust source text."""

    def __init__(self) -> None:
        self.depth = {"paren": 0, "bracket": 0, "brace": 0}
        self._quote: str | None = None
        self._escaped = False

    def consume(self, char: str) -> bool:
        """Advance over one character; return True when it is code outside any literal."""
        if self._quote is not None:
            if self._escaped:
                self._escaped = False
            elif char == "\\":
                self._escaped = True
            elif char == self._quote:
                self._quote = None
            return False
        if char in {'"', "'"}:
            self._quote = char
            return False
        if char in _OPENERS:
            self.depth[_OPENERS[char]] += 1
        elif char in _CLOSERS:
            self.depth[_CLOSERS[char]] -= 1
        return True

    def at_top_level(self) -> bool:
        return all(value == 0 for value in self.depth.values())


def _balanced_macro_contents(text: str, open_paren: int) -> tuple[str, int] | None:
    scanner = _SourceScanner()
    for index in range(open_paren, len(text)):
        char = text[index]
        if scanner.consume(char) and char == ")" and scanner.depth["paren"] == 0:
            return text[open_paren + 1:index], index + 1
    return None


def _split_top_level_args(contents: str) -> list[str]:
    args: list[str] = []
    start = 0
    scanner = _SourceScanner()
    for index, char in enumerate(contents):
        if scanner.consume(char) and char == "," and scanner.at_top_level():
            args.append(contents[start:index].strip())
            start = index + 1
    args.append(contents[start:].strip())
    return args


def _normalized_expression(expression: str) -> str:
    value = re.sub(r"\s+", "", expression)
    while value.startswith("(") and value.endswith(")"):
        parsed = _balanced_macro_contents(value, 0)
        if parsed is None or parsed[1] != len(value):
            break
        value = parsed[0]
    return value


def _top_level_operator(expression: str, operator: str) -> tuple[str, str] | None:
    scanner = _SourceScanner()
    index = 0
    while index <= len(expression) - len(operator):
        char = expression[index]
        if scanner.consume(char) and scanner.at_top_level() and expression.startswith(operator, index):
            return expression[:index], expression[index + len(operator):]
        index += 1
    return None


def _simple_literal(expression: str) -> str | None:
    value = _normalized_expression(expression)
    return value if SIMPLE_LITERAL.fullmatch(value) else None


def _boolean_assert_findings(macro: str, args: list[str]) -> list[str]:
    expression = _normalized_expression(args[0])
    if expression in {"true", "!false"}:
        return [f"{macro}! has constant true condition"]
    equality = _top_level_operator(args[0], "==")
    if equality and _normalized_expression(equality[0]) == _normalized_expression(equality[1]):
        return [f"{macro}! compares an expression with itself"]
    inequality = _top_level_operator(args[0], "!=")
    if inequality:
        left = _simple_literal(inequality[0])
        right = _simple_literal(inequality[1])
        if left is not None and right is not None and left != right:
            return [f"{macro}! has a constant-only true inequality"]
    return []


def _equality_macro_findings(macro: str, args: list[str]) -> list[str]:
    if macro == "assert_eq" and _normalized_expression(args[0]) == _normalized_expression(args[1]):
        return ["assert_eq! compares an expression with itself"]
    left_literal = _simple_literal(args[0])
    right_literal = _simple_literal(args[1])
    if left_literal is not None and right_literal is not None:
        if macro == "assert_eq" and left_literal == right_literal:
            return ["assert_eq! is a constant-only equality"]
        if macro == "assert_ne" and left_literal != right_literal:
            return ["assert_ne! is a constant-only inequality"]
    return []


def _assertion_findings(macro: str, args: list[str]) -> list[str]:
    if macro in {"assert", "debug_assert"} and args:
        return _boolean_assert_findings(macro, args)
    if macro in {"assert_eq", "assert_ne"} and len(args) >= 2:
        return _equality_macro_findings(macro, args)
    return []


def trivial_assertions(body: str) -> list[str]:
    """Return obvious tautological or constant-only passing assertions."""
    findings: list[str] = []
    for match in ASSERTION.finditer(body):
        parsed = _balanced_macro_contents(body, match.end() - 1)
        if parsed is None:
            continue
        findings.extend(_assertion_findings(match.group(1), _split_top_level_args(parsed[0])))
    return findings


def load_policy(path: Path) -> dict[str, Any]:
    document = json.loads(path.read_text())
    if document.get("schema_version") != 1:
        raise ValueError("unsupported security policy schema")
    return document


def _brace_block_end(text: str, start: int) -> int | None:
    depth = 0
    in_string = False
    escaped = False
    for index in range(start, len(text)):
        char = text[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index + 1
    return None


def extract_tests(text: str) -> list[tuple[str, str]]:
    tests: list[tuple[str, str]] = []
    for header in TEST_HEADER.finditer(text):
        function = FUNCTION.search(text, header.end())
        if function is None:
            continue
        end = _brace_block_end(text, function.end() - 1)
        if end is not None:
            tests.append((function.group(1), text[function.start():end]))
    return tests


def _critical_test_files(policy: dict[str, Any]) -> list[Path]:
    files: list[Path] = []
    for root in policy.get("critical_test_roots", []):
        path = ROOT / root
        if path.is_file():
            files.append(path)
        elif path.is_dir():
            files.extend(path.rglob("*.rs"))
    return sorted(set(files))


def _test_record(path: Path, name: str, body: str) -> dict[str, Any]:
    return {
        "path": str(path.relative_to(ROOT)),
        "name": name,
        "assertions": len(ASSERTION.findall(body)),
        "negative_path": bool(re.search(r"\b(?:Err\s*\(|is_err\s*\(|reject|invalid|malformed|corrupt|truncat|fail)", body, re.I)),
        "round_trip": bool(re.search(r"round.?trip|reparse|re-?encode|restor", body, re.I)),
        "state_transition": bool(re.search(r"state|transition|advance|reset|authorize", body, re.I)),
        "exact_error": bool(re.search(r"(?:assert_eq!|matches!)\s*\([\s\S]*?Err\s*\(", body)),
        "has_assertion": bool(ASSERTION.search(body)) or "#[should_panic" in body,
        "trivial_assertions": trivial_assertions(body),
    }


def _scan_tests(files: list[Path]) -> tuple[list[dict[str, Any]], list[str], list[dict[str, Any]]]:
    test_records: list[dict[str, Any]] = []
    assertion_free: list[str] = []
    trivial_assertion_tests: list[dict[str, Any]] = []
    for path in files:
        text = path.read_text(errors="replace")
        for name, body in extract_tests(text):
            record = _test_record(path, name, body)
            test_records.append(record)
            test_id = f"{record['path']}::{name}"
            if not record["has_assertion"]:
                assertion_free.append(test_id)
            if record["trivial_assertions"]:
                trivial_assertion_tests.append({
                    "test": test_id,
                    "findings": record["trivial_assertions"],
                })
    return test_records, assertion_free, trivial_assertion_tests


def _evidence_contents(requirement: dict[str, Any]) -> tuple[list[str], list[str]]:
    resolved: list[str] = []
    contents: list[str] = []
    for pattern in requirement.get("files", []):
        for path in kaskold_glob(pattern):
            if path.is_file():
                resolved.append(display_path(path))
                contents.append(path.read_text(errors="replace"))
    return resolved, contents


def _evidence_requirement_result(requirement: dict[str, Any]) -> tuple[dict[str, Any], list[str]]:
    resolved, contents = _evidence_contents(requirement)
    combined = "\n".join(contents).lower()
    missing = [term for term in requirement.get("terms", []) if term.lower() not in combined]
    if not resolved:
        missing.append("<no files resolved>")
    result = {
        "id": requirement.get("id"),
        "files": sorted(set(resolved)),
        "required_terms": requirement.get("terms", []),
        "missing_terms": missing,
        "met": not missing,
    }
    return result, missing


def _check_required_evidence(policy: dict[str, Any]) -> tuple[list[dict[str, Any]], list[str]]:
    evidence_results: list[dict[str, Any]] = []
    missing_evidence: list[str] = []
    for requirement in policy.get("required_evidence", []):
        result, missing = _evidence_requirement_result(requirement)
        evidence_results.append(result)
        missing_evidence.extend(f"{requirement.get('id')}: missing {term!r}" for term in missing)
    return evidence_results, missing_evidence


def audit(policy_path: Path = POLICY) -> tuple[list[str], dict[str, Any]]:
    policy = load_policy(policy_path)["test_quality"]
    files = _critical_test_files(policy)
    test_records, assertion_free, trivial_assertion_tests = _scan_tests(files)
    evidence_results, missing_evidence = _check_required_evidence(policy)

    errors = [f"assertion-free critical test: {value}" for value in assertion_free]
    errors.extend(
        f"trivial critical assertion: {item['test']}: {finding}"
        for item in trivial_assertion_tests
        for finding in item["findings"]
    )
    errors.extend(missing_evidence)
    report = {
        "schema_version": 1,
        "healthy": not errors,
        "files_scanned": len(files),
        "tests_scanned": len(test_records),
        "assertion_free_tests": assertion_free,
        "trivial_assertion_tests": trivial_assertion_tests,
        "capabilities": {
            "negative_path_tests": sum(item["negative_path"] for item in test_records),
            "exact_error_tests": sum(item["exact_error"] for item in test_records),
            "round_trip_tests": sum(item["round_trip"] for item in test_records),
            "state_transition_tests": sum(item["state_transition"] for item in test_records),
        },
        "required_evidence": evidence_results,
        "tests": test_records,
        "errors": errors,
    }
    return errors, report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--policy", type=Path, default=POLICY)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    arguments = parser.parse_args()
    try:
        errors, report = audit(arguments.policy)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"ERROR: test-quality audit failed: {error}")
        return 1
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    for error in errors:
        print(f"ERROR: {error}")
    if errors:
        return 1
    capabilities = report["capabilities"]
    print(
        f"PASS: {report['tests_scanned']} critical tests contain assertions and no obvious trivial passes; "
        f"negative={capabilities['negative_path_tests']} exact-errors={capabilities['exact_error_tests']} "
        f"round-trips={capabilities['round_trip_tests']} state-transitions={capabilities['state_transition_tests']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
