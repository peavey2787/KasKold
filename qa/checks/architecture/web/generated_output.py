"""Policy helpers enforcing generated Companion runtime ownership."""

from __future__ import annotations

from pathlib import Path

from architecture.core.common import is_generated_tree
from architecture.core.inventory.repository_inventory import _excluded as excluded_from_inventory


def check_web_pkg_policy(root: Path) -> list[str]:
    """Keep target/ canonical while permitting an ignored local web/pkg mirror."""
    errors: list[str] = []
    builder = (root / "tools/build/web/build_companion_runtime.py").read_text(errors="ignore")
    if "target/kaskold-companion-web/site" not in builder:
        errors.append("Canonical Companion runtime builder must stage the deployable site under target/kaskold-companion-web/site")
    if 'shutil.rmtree(authored / "pkg"' not in builder:
        errors.append("Canonical Companion runtime builder must remove stale local web/pkg before staging")
    if "sync_local_web_package(site)" not in builder:
        errors.append("Canonical Companion runtime builder must mirror fresh bindings into local web/pkg")

    for product, directory in (("Companion", "kaskold-companion-web"), ("Vault", "kaskold-vault-web")):
        binding = Path(f"apps/{directory}/web/pkg/runtime.js")
        if not excluded_from_inventory(binding):
            errors.append(
                f"Generated local {product} web/pkg must be excluded from repository inventory/source archives"
            )
        if not is_generated_tree(root / binding, root):
            errors.append(
                f"Generated local {product} web/pkg must be excluded from authored source-quality scans"
            )

    for facade in (root / "apps/kaskold-companion-web/build.sh", root / "apps/kaskold-companion-web/build.ps1"):
        if "build_companion_runtime.py" not in facade.read_text(errors="ignore"):
            errors.append(f"Companion build facade must delegate to canonical builder: {facade.relative_to(root)}")
    return errors
