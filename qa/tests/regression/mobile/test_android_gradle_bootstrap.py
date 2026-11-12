from __future__ import annotations

import json
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]


class AndroidGradleBootstrapTests(unittest.TestCase):

    def test_signing_authorization_quality_evidence_follows_firmware_core_owner(self) -> None:
        policy = json.loads((ROOT / "qa/checks/security/policy.json").read_text())
        self.assertIn(
            "crates/kaskold-hardware-core/src/unit_tests",
            policy["test_quality"]["critical_test_roots"],
        )
        requirement = next(
            item for item in policy["test_quality"]["required_evidence"]
            if item["id"] == "signing-state-authorization"
        )
        self.assertEqual(
            requirement["files"],
            [
                "crates/kaskold-hardware-core/src/unit_tests/security_tests.rs",
                "apps/kaskold-hardware/src/runtime/unit_tests/input_tests.rs",
            ],
        )
        self.assertIn("ReviewIncomplete", requirement["terms"])
        self.assertIn("review_approved", requirement["terms"])

    def test_mixed_qr_quality_evidence_includes_canonical_protocol_decoder(self) -> None:
        policy = json.loads((ROOT / "qa/checks/security/policy.json").read_text())
        requirement = next(
            item for item in policy["test_quality"]["required_evidence"]
            if item["id"] == "mixed-qr-sessions"
        )
        self.assertIn("crates/kaskold-protocol/src/unit_tests/mod.rs", requirement["files"])
        self.assertIn("QrDecoder", requirement["terms"])


    @unittest.skipUnless(os.name == "posix", "Android CLI bootstrap fixture uses POSIX shell stubs")
    def test_linux_android_sdk_helper_prefers_android_cli_preview_channel(self) -> None:
        helper = ROOT / "scripts/linux/lib/android-sdk.sh"
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sdk = root / "sdk"
            tools = sdk / "cmdline-tools/latest/bin"
            tools.mkdir(parents=True)
            log = root / "android-cli.log"
            android_cli = tools / "android"
            android_cli.write_text(
                "#!/usr/bin/env bash\n"
                "set -eu\n"
                "printf '%s\\n' \"$*\" > \"$KASKOLD_ANDROID_CLI_LOG\"\n"
                "sdk=''\n"
                "for arg in \"$@\"; do case \"$arg\" in --sdk=*) sdk=${arg#--sdk=} ;; esac; done\n"
                "mkdir -p \"$sdk/platforms/android-37\" \"$sdk/build-tools/37.0.0\"\n"
                ": > \"$sdk/platforms/android-37/android.jar\"\n"
                "printf 'AndroidVersion.ApiLevel = 37\\n' > \"$sdk/platforms/android-37/source.properties\"\n"
                "printf '#!/usr/bin/env bash\\nexit 0\\n' > \"$sdk/build-tools/37.0.0/aapt2\"\n"
                "chmod +x \"$sdk/build-tools/37.0.0/aapt2\"\n",
                encoding="utf-8",
            )
            android_cli.chmod(0o755)
            sdkmanager = tools / "sdkmanager"
            sdkmanager.write_text("#!/usr/bin/env bash\nexit 91\n", encoding="utf-8")
            sdkmanager.chmod(0o755)
            env = os.environ.copy()
            env.update({
                "KASKOLD_ANDROID_API": "37",
                "KASKOLD_ANDROID_BUILD_TOOLS": "37.0.0",
                "KASKOLD_ANDROID_CLI_LOG": str(log),
            })
            command = (
                "set -Eeuo pipefail; "
                "fail(){ printf 'ERROR: %s\\n' \"$*\" >&2; exit 1; }; "
                "info(){ printf '==> %s\\n' \"$*\"; }; "
                f"source '{helper}'; "
                f"ensure_android_sdk_build_packages '{sdk}' '{root / 'toolchains.env'}'"
            )
            result = subprocess.run(
                ["bash", "-lc", command], env=env, text=True, capture_output=True, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            invocation = log.read_text(encoding="utf-8")
            self.assertIn("sdk install --canary", invocation)
            self.assertIn("platforms/android-37", invocation)
            self.assertIn("build-tools/37.0.0", invocation)


    def test_windows_powershell_build_scripts_are_ascii_safe_for_windows_powershell_51(self) -> None:
        # Windows PowerShell 5.1 interprets UTF-8-without-BOM source as the active
        # ANSI code page. Non-ASCII punctuation can therefore become mojibake
        # containing typographic quote characters and change PowerShell parsing.
        for base in (ROOT / "scripts/windows", ROOT / "qa/windows"):
            for path in sorted(base.rglob("*")):
                if path.suffix.lower() not in {".ps1", ".psm1", ".psd1"}:
                    continue
                with self.subTest(path=path.relative_to(ROOT).as_posix()):
                    data = path.read_bytes()
                    try:
                        data.decode("ascii")
                    except UnicodeDecodeError as exc:
                        self.fail(
                            f"{path.relative_to(ROOT).as_posix()} contains non-ASCII source bytes; "
                            "native Windows PowerShell 5.1 runners must remain ASCII-safe: "
                            f"{exc}"
                        )


if __name__ == "__main__":
    unittest.main()
