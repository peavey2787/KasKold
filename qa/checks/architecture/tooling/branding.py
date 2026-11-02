"""Register the KasKold product-brand audit with architecture QA."""
import importlib
from pathlib import Path
from typing import Any

# A bare ``from branding import ...`` is resolved statically to this package's
# own namespace, so the sibling check module is loaded dynamically instead.
check_kaskold_brand: Any = importlib.import_module("branding.check_kaskold_brand")


def check(root: Path) -> list[str]:
    return check_kaskold_brand.check(root)
