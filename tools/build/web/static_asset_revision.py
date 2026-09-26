#!/usr/bin/env python3
"""Shared deterministic cache revisioning for KasKold Web deployables."""
from __future__ import annotations

import hashlib
from pathlib import Path
import re


def authored_asset_revision(authored: Path) -> str:
    """Return a deterministic revision for authored static assets."""
    digest = hashlib.sha256()
    for path in sorted(authored.rglob("*")):
        if not path.is_file() or "pkg" in path.relative_to(authored).parts:
            continue
        relative = path.relative_to(authored).as_posix().encode("utf-8")
        digest.update(len(relative).to_bytes(4, "big"))
        digest.update(relative)
        data = path.read_bytes()
        digest.update(len(data).to_bytes(8, "big"))
        digest.update(data)
    return digest.hexdigest()[:16]


def _with_revision(url: str, revision: str) -> str:
    base = url.split("?", 1)[0]
    return f"{base}?rev={revision}"


def stamp_site_asset_revision(site: Path, revision: str) -> None:
    """Cache-bust local static assets and every relative ES-module edge."""
    index = site / "index.html"
    html = index.read_text(encoding="utf-8")
    html = re.sub(
        r'(?P<prefix>\b(?:href|src)=["\'])(?P<url>(?:css|js|lib|img)/[^"\']+)(?P<suffix>["\'])',
        lambda match: (
            f'{match.group("prefix")}{_with_revision(match.group("url"), revision)}'
            f'{match.group("suffix")}'
        ),
        html,
    )
    index.write_text(html, encoding="utf-8")

    module_pattern = re.compile(
        r'(?P<prefix>\bfrom\s*["\']|\bimport\s*\(\s*["\']|\bimport\s*["\'])'
        r'(?P<url>\.{1,2}/[^"\']+\.js(?:\?[^"\']*)?)'
        r'(?P<suffix>["\'])'
    )
    for path in sorted((site / "js").rglob("*.js")):
        source = path.read_text(encoding="utf-8")
        source = module_pattern.sub(
            lambda match: (
                f'{match.group("prefix")}{_with_revision(match.group("url"), revision)}'
                f'{match.group("suffix")}'
            ),
            source,
        )
        path.write_text(source, encoding="utf-8")
