from __future__ import annotations

import re
from pathlib import Path

WASM_EXPORT_ATTR = r'(?:#\[wasm_bindgen(?:\([^\]]*\))?\]|#\[cfg_attr\(target_arch\s*=\s*"wasm32",\s*wasm_bindgen::prelude::wasm_bindgen(?:\([^\]]*\))?\)\])'
WASM_API_RELATIVE = "wasm/api.js"
GENERATED_EXPORT_PATTERN = r"const\s+GENERATED_WASM_EXPORTS\s*=\s*Object\.freeze\(\[(.*?)\]\);"
NAMED_IMPORT_PATTERN = r"""import\s*\{(?P<names>.*?)\}\s*from\s*['"](?P<source>[^'"]+)['"]"""


def check_wasm_api(root: Path, web_js_root: Path) -> tuple[list[str], list[str]]:
    wasm_api_path = web_js_root / WASM_API_RELATIVE
    if not wasm_api_path.exists():
        return [], []
    wasm_api_source = wasm_api_path.read_text(errors="ignore")
    export_match = re.search(GENERATED_EXPORT_PATTERN, wasm_api_source, re.S)
    if not export_match:
        return ["web WASM API module must declare generated export inventory"], []
    wasm_import_names = [
        "init",
        *re.findall(r"['\"]([A-Za-z_][A-Za-z0-9_]*)['\"]", export_match.group(1)),
    ]
    errors = [
        *_check_wasm_loading(wasm_api_source, wasm_import_names),
        *_check_wasm_declarations(wasm_api_source, wasm_import_names),
        *_check_rust_exports(root, wasm_import_names),
        *_check_wasm_consumption(web_js_root, wasm_api_path, wasm_import_names),
    ]
    return errors, wasm_import_names


def _check_wasm_loading(wasm_api_source: str, wasm_import_names: list[str]) -> list[str]:
    errors: list[str] = []
    if len(wasm_import_names) != len(set(wasm_import_names)):
        errors.append("browser WASM API contains duplicate export names")
    if "await import('../../pkg/companion_web.js')" not in wasm_api_source:
        errors.append("web WASM API must load the generated package lazily")
    if re.search(r"\}\s*from\s*['\"]\.\./\.\./pkg/companion_web\.js['\"]", wasm_api_source):
        errors.append("web WASM API must not statically re-export the generated package")
    return errors


def _check_wasm_declarations(wasm_api_source: str, wasm_import_names: list[str]) -> list[str]:
    return [
        f"browser WASM API wrapper missing: {name}"
        for name in wasm_import_names
        if not re.search(
            rf"export\s+(?:async\s+)?function\s+{re.escape(name)}\s*\(", wasm_api_source
        )
    ]


def _check_rust_exports(root: Path, wasm_import_names: list[str]) -> list[str]:
    rust_exports = set(re.findall(
        rf"{WASM_EXPORT_ATTR}(?:\s*#\[[^]]+\])*\s*pub(?:\s+async)?\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)",
        _rust_wasm_export_source(root),
    ))
    imported = {name for name in wasm_import_names if name != "init"}
    missing = sorted(imported - rust_exports)
    if missing:
        return [f"browser imports missing Rust WASM exports: {missing}"]
    return []


def _check_wasm_consumption(
    web_js_root: Path,
    wasm_api_path: Path,
    wasm_import_names: list[str],
) -> list[str]:
    consumed_names: set[str] = set()
    named_import = re.compile(NAMED_IMPORT_PATTERN, re.S)
    for path in sorted(web_js_root.rglob("*.js")):
        if path == wasm_api_path or "pkg" in path.parts or "lib" in path.parts:
            continue
        source = path.read_text(errors="ignore")
        for match in named_import.finditer(source):
            import_path = (path.parent / match.group("source")).resolve()
            if import_path != wasm_api_path.resolve():
                continue
            consumed_names.update(_imported_binding_names(match.group("names")))
    return [
        f"browser WASM import has no authored import consumer: {name}"
        for name in wasm_import_names
        if name not in consumed_names
    ]


def _imported_binding_names(import_list: str) -> set[str]:
    names: set[str] = set()
    for raw_name in import_list.split(","):
        name = raw_name.split("//", 1)[0].strip()
        if name:
            names.add(name.split(" as ", 1)[0].strip())
    return names


def _rust_wasm_export_source(root: Path) -> str:
    roots = (root / "crates/online-watcher/src/wasm_api", root / "apps/kaskold-companion-web/src")
    return "\n".join(path.read_text(errors="ignore") for base in roots for path in base.rglob("*.rs"))
