#!/usr/bin/env python3
"""Run the pinned Python and JavaScript linters and type checkers as one gate.

Pinned tools install below ``target/qa/tooling`` rather than on the host, so
every machine and CI job judge the repository with the same rules. npm scripts
stay disabled during installation: the tools run only after the lockfile
integrity hashes have been verified.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from collections.abc import Iterable, Mapping, Sequence

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from toolchains import load_toolchains  # noqa: E402

TOOLING = Path("target/qa/tooling")
LINT_CONFIG = Path("qa/lint")
LINT_FILES = ("package.json", "package-lock.json", "eslint.config.mjs")
PYTHON_ROOTS = ("qa", "scripts", "apps", "tools")
TYPED_PYTHON_ROOTS = ("qa", "scripts", "tools")
JAVASCRIPT_ROOTS = ("apps", "qa")
# Every module below these roots must opt in to the type checker; the list only grows.
TYPED_JAVASCRIPT_ROOTS = ("apps/kaskold-companion-web/web/js/core",)
TS_CHECK_DIRECTIVE = "// @ts-check"
COMPLEXITY_BRANCHES = range(13)
PYTHON_TOO_COMPLEX = "def canary(value):\n" + "".join(
    f"    if value == {branch}:\n        return {branch}\n" for branch in COMPLEXITY_BRANCHES
)
JAVASCRIPT_TOO_COMPLEX = "export function canary(value) {\n" + "".join(
    f"    if (value === {branch}) return {branch};\n" for branch in COMPLEXITY_BRANCHES
) + "}\n"


def run(command: Sequence[str], *, env: Mapping[str, str] | None = None) -> None:
    print(f"$ {' '.join(command)}", flush=True)
    subprocess.run(command, cwd=ROOT, env=env, check=True, stdin=subprocess.DEVNULL)


def python_tool_environment(tool_root: Path) -> dict[str, str]:
    return {**os.environ, "PYTHONPATH": str(tool_root), "PYTHONUTF8": "1"}


def python_requirements(pins: Mapping[str, str]) -> list[str]:
    return [f"ruff=={pins['KASKOLD_RUFF_VERSION']}", f"mypy=={pins['KASKOLD_MYPY_VERSION']}"]


def install_python_tools(root: Path, pins: Mapping[str, str]) -> Path:
    """Install the pinned tools once per pin set and return their directory."""
    tool_root = root / TOOLING / "python"
    requirements = python_requirements(pins)
    stamp = tool_root / "requirements.json"
    if stamp.is_file() and json.loads(stamp.read_text(encoding="utf-8")) == requirements:
        return tool_root
    shutil.rmtree(tool_root, ignore_errors=True)
    run([
        sys.executable, "-m", "pip", "install", "--quiet", "--disable-pip-version-check", "--no-user",
        "--target", str(tool_root), *requirements,
    ])
    stamp.write_text(json.dumps(requirements), encoding="utf-8")
    return tool_root


def install_javascript_tools(root: Path, npm: str) -> Path:
    """Copy the lint manifest beside its packages, because ESM resolves from there."""
    tool_root = root / TOOLING / "lint"
    tool_root.mkdir(parents=True, exist_ok=True)
    for name in LINT_FILES:
        shutil.copyfile(root / LINT_CONFIG / name, tool_root / name)
    prefix = ["--prefix", str(tool_root)]
    run([npm, "ci", "--ignore-scripts", "--no-fund", "--no-audit", *prefix])
    run([npm, "audit", "--audit-level=low", *prefix])
    return tool_root


def require_rejection(
    command: Sequence[str], source: str, rule: str, *, env: Mapping[str, str] | None = None,
) -> None:
    """Prove the committed rule still fires, so a loosened config cannot pass as clean."""
    result = subprocess.run(
        command, cwd=ROOT, env=env, input=source, capture_output=True, text=True, encoding="utf-8", check=False,
    )
    if result.returncode != 1 or rule not in result.stdout:
        print(f"ERROR: {command[0]} no longer rejects {rule}:\n{result.stdout}{result.stderr}", file=sys.stderr)
        raise subprocess.CalledProcessError(result.returncode or 1, command)


def run_python_lints(tool_root: Path) -> None:
    env = python_tool_environment(tool_root)
    cache = str(tool_root.parent / "mypy-cache")
    require_rejection(
        [sys.executable, "-m", "ruff", "check", "--no-cache", "--stdin-filename", "canary.py", "-"],
        PYTHON_TOO_COMPLEX, "C901", env=env,
    )
    run([sys.executable, "-m", "ruff", "check", "--no-cache", *PYTHON_ROOTS], env=env)
    run([sys.executable, "-m", "mypy", f"--cache-dir={cache}", *TYPED_PYTHON_ROOTS], env=env)


def run_javascript_lints(tool_root: Path, node: str) -> None:
    eslint = tool_root / "node_modules/eslint/bin/eslint.js"
    config = tool_root / "eslint.config.mjs"
    require_rejection(
        [node, str(eslint), "--config", str(config), "--stdin", "--stdin-filename", "qa/canary.js"],
        JAVASCRIPT_TOO_COMPLEX, "complexity",
    )
    run([node, str(eslint), "--config", str(config), "--no-warn-ignored", "--max-warnings", "0", *JAVASCRIPT_ROOTS])
    tsc = tool_root / "node_modules/typescript/bin/tsc"
    run([node, str(tsc), "--project", str(LINT_CONFIG / "tsconfig.json")])


def javascript_modules(directory: Path) -> Iterable[Path]:
    return sorted(path for path in directory.rglob("*.js") if "node_modules" not in path.parts)


def untyped_modules(root: Path, typed_roots: Iterable[str] = TYPED_JAVASCRIPT_ROOTS) -> list[str]:
    """Return modules below a typed root that have not opted in to the type checker."""
    missing: list[str] = []
    for typed_root in typed_roots:
        for path in javascript_modules(root / typed_root):
            first_line = path.read_text(encoding="utf-8").split("\n", 1)[0].strip()
            if first_line != TS_CHECK_DIRECTIVE:
                missing.append(path.relative_to(root).as_posix())
    return missing


def node_major_mismatch(node: str, pins: Mapping[str, str]) -> str | None:
    """ESLint and the node test runner need the pinned major release."""
    expected = pins["KASKOLD_NODE_VERSION"].split(".")[0]
    actual = subprocess.run([node, "--version"], capture_output=True, text=True, check=True).stdout.strip()
    return None if actual.lstrip("v").split(".")[0] == expected else f"found {actual}, expected Node.js {expected}"


def main() -> int:
    node = shutil.which("node")
    npm = shutil.which("npm")
    if node is None or npm is None:
        print("ERROR: language lints require node and npm on PATH", file=sys.stderr)
        return 1
    missing = untyped_modules(ROOT)
    for path in missing:
        print(f"ERROR: {path} must start with {TS_CHECK_DIRECTIVE}", file=sys.stderr)
    if missing:
        return 1
    pins = load_toolchains()
    mismatch = node_major_mismatch(node, pins)
    if mismatch:
        print(f"ERROR: language lints need the pinned Node.js: {mismatch}", file=sys.stderr)
        return 1
    try:
        run_python_lints(install_python_tools(ROOT, pins))
        run_javascript_lints(install_javascript_tools(ROOT, npm), node)
    except subprocess.CalledProcessError as error:
        print(f"ERROR: language lint failed with exit {error.returncode}", file=sys.stderr)
        return 1
    print("PASS: ruff, mypy, ESLint, tsc and npm audit report no findings")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
