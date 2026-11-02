
import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[4] / "qa/checks"))
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]


class CredentialKdfHardwareMemoryTests(unittest.TestCase):


    def test_current_firmware_kdf_calls_cannot_bypass_psram_adapter(self) -> None:
        memory_adapter = ROOT / "apps/kaskold-hardware/src/services/memory/password_kdf.rs"
        offenders = []
        for path in (ROOT / "apps/kaskold-hardware/src").rglob("*.rs"):
            if path == memory_adapter:
                continue
            for line in path.read_text(errors="ignore").splitlines():
                if "password_kdf::derive_key_32" not in line:
                    continue
                if "crate::services::memory::password_kdf::derive_key_32" in line:
                    continue
                offenders.append(path.relative_to(ROOT).as_posix())
                break
        self.assertEqual(offenders, [])


if __name__ == "__main__":
    unittest.main()
