from __future__ import annotations

from hashlib import sha256
from pathlib import Path
import os
import shutil
import stat
import subprocess
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[4]


class AndroidGradleExecutableRecoveryTests(unittest.TestCase):

    @unittest.skipUnless(os.name == "posix", "Gradle executable-mode recovery is POSIX-specific")
    def test_android_build_repairs_fresh_and_cached_non_executable_gradle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            script = root / "scripts/linux/build/android-build.sh"
            script.parent.mkdir(parents=True)
            shutil.copy2(ROOT / "scripts/linux/build/android-build.sh", script)
            script.chmod(0o755)
            sdk_helper = root / "scripts/linux/lib/android-sdk.sh"
            sdk_helper.parent.mkdir(parents=True)
            shutil.copy2(ROOT / "scripts/linux/lib/android-sdk.sh", sdk_helper)

            android = root / "apps/kaskold-companion-android"
            wrapper = android / "gradle/wrapper/gradle-wrapper.properties"
            wrapper.parent.mkdir(parents=True)
            (android / "gradle/gradle-daemon-jvm.properties").write_text("toolchainVersion=25\n", encoding="utf-8")
            toolchains = root / "qa/config/toolchains.env"
            toolchains.parent.mkdir(parents=True)
            toolchains.write_text(
                "KASKOLD_ANDROID_JDK=25\n"
                "KASKOLD_ANDROID_API=37\n"
                "KASKOLD_ANDROID_BUILD_TOOLS=37.0.0\n",
                encoding="utf-8",
            )
            sdk = root / "sdk"
            platform = sdk / "platforms/android-37"
            platform.mkdir(parents=True)
            (platform / "android.jar").write_bytes(b"")
            (platform / "source.properties").write_text("AndroidVersion.ApiLevel = 37\n")
            aapt2 = sdk / "build-tools/37.0.0/aapt2"
            aapt2.parent.mkdir(parents=True)
            aapt2.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
            aapt2.chmod(0o755)

            archive = root / "fixtures/gradle-9.5.0-bin.zip"
            archive.parent.mkdir(parents=True)
            launcher_text = """#!/usr/bin/env bash
set -eu
if [[ "${1:-}" == "--version" ]]; then
  printf 'Gradle 9.5.0\\n'
  exit 0
fi
exit 0
"""
            info = zipfile.ZipInfo("gradle-9.5.0/bin/gradle")
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | 0o755) << 16
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as package:
                package.writestr(info, launcher_text)
            digest = sha256(archive.read_bytes()).hexdigest()
            wrapper.write_text(
                "distributionBase=GRADLE_USER_HOME\n"
                "distributionPath=wrapper/dists\n"
                f"distributionUrl={archive.as_uri()}\n"
                f"distributionSha256Sum={digest}\n"
                "zipStoreBase=GRADLE_USER_HOME\n"
                "zipStorePath=wrapper/dists\n"
            )

            fake_jdk = root / "jdk-25"
            java = fake_jdk / "bin/java"
            java.parent.mkdir(parents=True)
            java.write_text(
                "#!/usr/bin/env bash\n"
                "echo 'openjdk version \"25.0.0\"' >&2\n",
                encoding="utf-8",
            )
            java.chmod(0o755)

            gradle_home = root / "gradle-home"
            env = os.environ.copy()
            env["KASKOLD_ANDROID_SDK_ROOT"] = str(sdk)
            env["JAVA_HOME"] = str(fake_jdk)
            env["GRADLE_USER_HOME"] = str(gradle_home)
            env["KASKOLD_ANDROID_PRODUCTS"] = "companion"
            env.pop("GRADLE_BIN", None)
            command = ["bash", str(script), "test"]

            first = subprocess.run(command, cwd=root, env=env, text=True, capture_output=True)
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            installed = gradle_home / "kaskold/distributions/gradle-9.5.0/bin/gradle"
            self.assertTrue(os.access(installed, os.X_OK))

            installed.chmod(0o644)
            self.assertFalse(os.access(installed, os.X_OK))
            second = subprocess.run(command, cwd=root, env=env, text=True, capture_output=True)
            self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
            self.assertTrue(os.access(installed, os.X_OK))


if __name__ == "__main__":
    unittest.main()
