import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))

from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[3]
CRAP_CHECK_DIR = ROOT / "qa/checks/quality/crap"
sys.path.insert(0, str(CRAP_CHECK_DIR))
from report import CrapEntry  # noqa: E402


class OnlineWatcherHostCoveragePolicyTests(unittest.TestCase):


    def test_crap_classifier_retains_effective_and_raw_assessment_fields(self) -> None:
        fields = set(CrapEntry.__dataclass_fields__)
        self.assertTrue({"assessment_basis", "raw_status", "coverage_state"} <= fields)


if __name__ == "__main__":
    unittest.main()
