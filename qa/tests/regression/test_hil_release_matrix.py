import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]


class HILReleaseMatrix(unittest.TestCase):
    def test_cores3_family_profiles_are_hil_selectable(self):
        runner = (ROOT / "qa/checks/firmware/run_hardware_tests.py").read_text()
        cli = (ROOT / "qa/linux/run-all.sh").read_text()
        self.assertIn('choices=("m5stack", "m5stack-lite")', runner)
        self.assertIn('m5stack|m5stack-lite', cli)
        self.assertNotIn('ov5640-af', runner)


if __name__ == "__main__":
    unittest.main()
