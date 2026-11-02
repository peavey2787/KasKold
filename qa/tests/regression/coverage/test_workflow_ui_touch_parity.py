from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[4]
FW = ROOT / "apps/kaskold-hardware"


class WorkflowUiTouchParityTests(unittest.TestCase):


    def test_workflow_ui_cannot_override_production_layout(self):
        allowed = {
            "runtime_evidence.rs",
            "redraw/navigation/developer.rs",
            "layout.rs",  # workflow menu visibility only; production geometry is unconditional
        }
        forbidden_features = ("workflow-runtime-auto", "workflow-test-auto", "workflow-hil-auto")
        offenders = []
        for path in (FW / "src/ui").rglob("*.rs"):
            rel = path.relative_to(FW / "src/ui").as_posix()
            text = path.read_text(encoding="utf-8")
            if any(feature in text for feature in forbidden_features) and rel not in allowed:
                offenders.append(rel)
        self.assertEqual(offenders, [], f"workflow-only UI layout branches: {offenders}")

    def test_literal_workflow_back_flags_match_production_back_zone(self):
        pattern = re.compile(r"TouchInput::new\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(true|false)\s*\)")
        mismatches = []
        for path in (FW / "src/runtime/workflow_tests/connected").rglob("*.rs"):
            text = path.read_text(encoding="utf-8")
            for match in pattern.finditer(text):
                x, y = int(match.group(1)), int(match.group(2))
                claimed = match.group(3) == "true"
                actual = x < 49 and y < 49
                if claimed != actual:
                    line = text[:match.start()].count("\n") + 1
                    mismatches.append(f"{path.relative_to(FW)}:{line}: ({x},{y}) claimed_back={claimed}")
        self.assertEqual(mismatches, [], "workflow forged physical Back events:\n" + "\n".join(mismatches))


if __name__ == "__main__":
    unittest.main()
