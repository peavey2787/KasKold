import sys
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks"))


class NavigationStateMachineTests(unittest.TestCase):


    def test_renderers_cannot_mutate_navigation_state(self) -> None:
        redraw_root = ROOT / "apps/kaskold-hardware/src/ui/redraw"
        screen_root = ROOT / "apps/kaskold-hardware/src/ui/screens"
        paths = [ROOT / "apps/kaskold-hardware/src/ui/redraw.rs"]
        paths.extend(redraw_root.rglob("*.rs"))
        paths.extend(screen_root.rglob("*.rs"))
        forbidden = ("navigation.app.state =", ".navigate(", ".go_home(", "go_main_menu(")
        offenders = []
        for path in paths:
            source = path.read_text()
            if any(token in source for token in forbidden):
                offenders.append(path.relative_to(ROOT).as_posix())
        self.assertEqual(offenders, [], "renderers/screens must not own AppState transitions")


if __name__ == "__main__":
    unittest.main()
