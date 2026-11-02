"""A feature domain reaches WASM and application state through its own boundary module."""
from __future__ import annotations

import re
from pathlib import Path

MODULE_NAME = re.compile(r"^[a-z0-9_]+\.js$")
MODULE_REFERENCE = re.compile(r"""(?:from\s*|import\s*\(\s*|import\s+)['"](?P<source>\.[^'"]+)['"]""")
RE_EXPORT = re.compile(r"""export\s*\{(?P<names>[^}]*)\}\s*from\s*['"](?P<source>[^'"]+)['"]\s*;""")
BOUNDARY = "boundary.js"
BOUNDARY_HEADER = re.compile(r"\A(?:\s*//[^\n]*\n)*")


def check_feature_boundaries(web_js_root: Path) -> list[str]:
    features = web_js_root / "features"
    if not features.is_dir():
        return []
    errors = _check_module_names(web_js_root)
    for domain in sorted(path for path in features.iterdir() if path.is_dir()):
        errors.extend(_check_domain(web_js_root, domain))
    return errors


def _check_module_names(web_js_root: Path) -> list[str]:
    return [
        f"web module name must be lower_snake_case: {path.relative_to(web_js_root).as_posix()}"
        for path in sorted(web_js_root.rglob("*.js"))
        if not MODULE_NAME.match(path.name)
    ]


def _check_domain(web_js_root: Path, domain: Path) -> list[str]:
    boundary = domain / BOUNDARY
    modules = [path for path in sorted(domain.rglob("*.js")) if path != boundary]
    errors = [
        error for module in modules for error in _check_references(web_js_root, domain, module)
    ]
    if boundary.exists():
        errors.extend(_check_boundary(web_js_root, boundary, modules))
    return errors


def _check_references(web_js_root: Path, domain: Path, module: Path) -> list[str]:
    errors: list[str] = []
    relative = module.relative_to(web_js_root).as_posix()
    for match in MODULE_REFERENCE.finditer(module.read_text(errors="ignore")):
        target = (module.parent / match.group("source")).resolve()
        if _is_shared_runtime(web_js_root, target):
            errors.append(f"{relative} must reach {match.group('source')} through its {BOUNDARY}")
        elif target.name == BOUNDARY and target != (domain / BOUNDARY).resolve():
            errors.append(f"{relative} must not use another domain's {BOUNDARY}")
    return errors


def _is_shared_runtime(web_js_root: Path, target: Path) -> bool:
    root = web_js_root.resolve()
    return target == root / "wasm/api.js" or target.is_relative_to(root / "app/state")


def _check_boundary(web_js_root: Path, boundary: Path, modules: list[Path]) -> list[str]:
    relative = boundary.relative_to(web_js_root).as_posix()
    source = boundary.read_text(errors="ignore")
    if RE_EXPORT.sub("", BOUNDARY_HEADER.sub("", source, count=1)).strip():
        return [f"{relative} may only re-export WASM and application state"]
    errors: list[str] = []
    exported: set[str] = set()
    for match in RE_EXPORT.finditer(source):
        if not _is_shared_runtime(web_js_root, (boundary.parent / match.group("source")).resolve()):
            errors.append(f"{relative} re-exports something other than WASM and state: {match.group('source')}")
        exported.update(name.strip() for name in match.group("names").split(",") if name.strip())
    domain_source = "\n".join(module.read_text(errors="ignore") for module in modules)
    errors.extend(
        f"{relative} re-exports a name its domain never uses: {name}"
        for name in sorted(exported)
        if not re.search(rf"\b{re.escape(name)}\b", domain_source)
    )
    return errors
