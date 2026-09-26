from __future__ import annotations

import pathlib
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
CHECKS = ROOT / "qa/checks"
if str(CHECKS) not in sys.path:
    sys.path.insert(0, str(CHECKS))

from architecture.firmware.subsystems.firmware_storage import _check_transport_owner_imports  # noqa: E402


class FirmwareTransportOwnerTests(unittest.TestCase):
    def _root(self) -> tuple[tempfile.TemporaryDirectory[str], pathlib.Path]:
        temporary = tempfile.TemporaryDirectory()
        root = pathlib.Path(temporary.name)
        transport = root / "apps/kaskold-hardware/src/hw/m5stack/storage/transport"
        protocol = transport / "protocol"
        protocol.mkdir(parents=True)
        (protocol / "wire.rs").write_text(
            "crate::hw::m5stack::spi_bus::with_sd_selected();\n", encoding="utf-8"
        )
        (transport / "block.rs").write_text("// CoreS3 transport leaf\n", encoding="utf-8")
        return temporary, root

    def test_accepts_transport_owned_wire_access(self) -> None:
        temporary, root = self._root()
        with temporary:
            self.assertEqual(_check_transport_owner_imports(root), [])

    def test_rejects_wildcard_inheritance(self) -> None:
        temporary, root = self._root()
        with temporary:
            target = root / "apps/kaskold-hardware/src/hw/m5stack/storage/transport/block.rs"
            target.write_text("use super::*;\n", encoding="utf-8")
            errors = _check_transport_owner_imports(root)
            self.assertTrue(any("wildcard inheritance" in error for error in errors))

    def test_rejects_shared_spi_owner_bypass(self) -> None:
        temporary, root = self._root()
        with temporary:
            target = root / "apps/kaskold-hardware/src/hw/m5stack/storage/transport/block.rs"
            target.write_text("const REGISTER: u32 = SPI2_CLOCK_REG;\n", encoding="utf-8")
            errors = _check_transport_owner_imports(root)
            self.assertTrue(any("bypasses the shared SPI2 owner" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
