from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[4]
FIRST_PARTY = (
    "apps/kaskold-companion-web/Cargo.toml",
    "apps/kaskold-hardware/Cargo.toml",
    "crates/kaskold-protocol/Cargo.toml",
    "crates/kaskold-sdk/Cargo.toml",
    "crates/offline-signer/Cargo.toml",
    "crates/online-watcher/Cargo.toml",
    "crates/shared-signer/Cargo.toml",
    "crates/kaskold-hardware-core/Cargo.toml",
    "qa/Cargo.toml",
    "qa/fuzz/Cargo.toml",
    "tools/Cargo.toml",
)


class ReleaseVersionTests(unittest.TestCase):


    def test_every_first_party_package_is_pinned_to_2_0_0(self):
        for relative in FIRST_PARTY:
            manifest = tomllib.loads((ROOT / relative).read_text())
            self.assertEqual(manifest["package"]["version"], "2.0.0", relative)


if __name__ == "__main__":
    unittest.main()
