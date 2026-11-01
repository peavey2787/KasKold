from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def cargo_metadata(manifest: str) -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--manifest-path", manifest],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return json.loads(result.stdout)


def normal_build_reachable(manifest: str, package: str) -> set[str]:
    """Names of packages reachable from `package` over normal and build edges only."""
    meta = cargo_metadata(manifest)
    nodes = {node["id"]: node for node in meta["resolve"]["nodes"]}
    names = {pkg["id"]: pkg["name"] for pkg in meta["packages"]}
    root = next(pkg_id for pkg_id, name in names.items() if name == package)
    seen: set[str] = set()
    stack = [root]
    while stack:
        for dep in nodes[stack.pop()]["deps"]:
            kinds = {kind["kind"] for kind in dep["dep_kinds"]}
            if kinds & {None, "build"} and dep["pkg"] not in seen:
                seen.add(dep["pkg"])
                stack.append(dep["pkg"])
    return {names[pkg_id] for pkg_id in seen}


class KasKoldCustodyConformanceTests(unittest.TestCase):
    def test_online_watcher_sdk_and_companion_have_no_production_custody_dependency(self) -> None:
        roots = {
            "online-watcher": "crates/online-watcher/Cargo.toml",
            "kaskold-sdk": "crates/kaskold-sdk/Cargo.toml",
            "kaskold-protocol": "crates/kaskold-protocol/Cargo.toml",
            "kaskold-companion-web": "apps/kaskold-companion-web/Cargo.toml",
        }
        for package, manifest in roots.items():
            reachable = normal_build_reachable(manifest, package)
            self.assertFalse(reachable & {"hot-wallet", "offline-signer"}, package)
        companion_reachable = normal_build_reachable(roots["kaskold-companion-web"], "kaskold-companion-web")
        self.assertNotIn("vault-runtime", companion_reachable)
        watcher_meta = cargo_metadata(roots["online-watcher"])
        watcher_pkg = next(pkg for pkg in watcher_meta["packages"] if pkg["name"] == "online-watcher")
        dev_names = {dep["name"] for dep in watcher_pkg["dependencies"] if dep["kind"] == "dev"}
        self.assertIn("offline-signer", dev_names)


if __name__ == "__main__":
    unittest.main()
