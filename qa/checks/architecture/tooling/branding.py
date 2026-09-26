"""Register the KasKold product-brand audit with architecture QA."""
from pathlib import Path

from branding import check_kaskold_brand


def check(root: Path) -> list[str]:
    return check_kaskold_brand.check(root)
