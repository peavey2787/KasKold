#!/usr/bin/env python3
"""Forbid HTML-parsing DOM sinks in the Companion and Vault browser code.

Dynamic UI is built with `core/ui/elements.js` (Companion) and DOM APIs, and
generated QR codes are shown as images (`core/qr/svg_image.js`). No authored
module may hand a string to the HTML parser, so values from a node, an invite
or a scanned QR can never become markup.
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
JS_ROOTS = (
    ROOT / 'apps/kaskold-companion-web/web/js',
    ROOT / 'apps/kaskold-vault-web/web/js',
)
HTML_SINKS = re.compile(
    r'\.(?:innerHTML|outerHTML)\s*[+]?=|\.insertAdjacentHTML\s*\(|document\.write(?:ln)?\s*\('
    r'|createContextualFragment\s*\(|new\s+DOMParser\s*\(|\.setHTMLUnsafe\s*\('
)
INLINE_HANDLER = re.compile(r'''\son[a-z]+\s*=\s*['"]''')


def portable_relative(path: Path) -> str:
    """Repository-relative POSIX path, independent of the host's separator."""
    return str(path.relative_to(ROOT)).replace('\\', '/')


def authored_modules() -> list[Path]:
    return sorted(path for root in JS_ROOTS for path in root.rglob('*.js'))


def sink_errors(path: Path) -> list[str]:
    text = path.read_text(encoding='utf-8', errors='replace')
    errors = []
    for match in HTML_SINKS.finditer(text):
        line = text.count('\n', 0, match.start()) + 1
        errors.append(f'{portable_relative(path)}:{line}: HTML-parsing sink {match.group(0).strip()!r}; build nodes instead')
    for match in INLINE_HANDLER.finditer(text):
        line = text.count('\n', 0, match.start()) + 1
        errors.append(f'{portable_relative(path)}:{line}: inline event-handler markup')
    return errors


def main() -> int:
    modules = authored_modules()
    errors = [error for path in modules for error in sink_errors(path)]
    if not modules:
        errors.append('no browser modules found; the sink scan would pass vacuously')
    if errors:
        for error in errors:
            print('ERROR:', error)
        return 1
    print(f'PASS: {len(modules)} browser modules build DOM nodes without HTML-parsing sinks')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
