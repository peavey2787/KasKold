from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[4]
FIRMWARE = ROOT / "apps/kaskold-hardware/src"


class FirmwareErrorUtf8SafetyTests(unittest.TestCase):


    def test_rejected_renderer_is_reserved_for_rejected_state(self):
        callers = []
        for path in FIRMWARE.rglob("*.rs"):
            text = path.read_text(encoding="utf-8", errors="ignore")
            if "draw_rejected_screen(" not in text:
                continue
            relative = path.relative_to(ROOT).as_posix()
            if relative.endswith("ui/screens/signing/errors.rs"):
                continue
            callers.append(relative)
        self.assertEqual(callers, ["apps/kaskold-hardware/src/ui/redraw/navigation.rs"])


    def test_no_arbitrary_str_byte_prefix_truncation_remains(self):
        unsafe = re.compile(r"&(?:message|reason|commit)\[\.\.[0-9]+\]")
        hits = []
        for path in FIRMWARE.rglob("*.rs"):
            source = path.read_text(encoding="utf-8", errors="ignore")
            if unsafe.search(source):
                hits.append(path.relative_to(ROOT).as_posix())
        self.assertEqual(hits, [])


if __name__ == "__main__":
    unittest.main()
