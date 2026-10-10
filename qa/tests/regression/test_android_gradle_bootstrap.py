from __future__ import annotations

import json
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


class AndroidGradleBootstrapTests(unittest.TestCase):
    def test_protocol_does_not_export_extended_public_keys(self) -> None:
        # BIP32 extended keys are Kaspa Portal's; kaskold-protocol re-exports
        # only the watch-only import surface.
        source = (ROOT / "crates/kaskold-protocol/src/account/mod.rs").read_text()
        self.assertNotRegex(source, r"pub use [^;]*ExtPubKey")

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
        self.assertIn("review_authorized", requirement["terms"])

    def test_mixed_qr_quality_evidence_includes_canonical_protocol_decoder(self) -> None:
        policy = json.loads((ROOT / "qa/checks/security/policy.json").read_text())
        requirement = next(
            item for item in policy["test_quality"]["required_evidence"]
            if item["id"] == "mixed-qr-sessions"
        )
        self.assertIn("crates/kaskold-protocol/src/unit_tests/mod.rs", requirement["files"])
        self.assertIn("QrDecoder", requirement["terms"])

    def test_android_gradle_daemon_jvm_matches_central_jdk_pin(self) -> None:
        toolchains = {}
        for raw in (ROOT / "qa/config/toolchains.env").read_text(encoding="utf-8").splitlines():
            key, sep, value = raw.partition("=")
            if sep:
                toolchains[key.strip()] = value.strip()
        daemon = {}
        for raw in (ROOT / "apps/kaskold-companion-android/gradle/gradle-daemon-jvm.properties").read_text(encoding="utf-8").splitlines():
            key, sep, value = raw.partition("=")
            if sep:
                daemon[key.strip()] = value.strip()
        self.assertEqual(toolchains.get("KASKOLD_ANDROID_JDK"), "25")
        self.assertEqual(daemon.get("toolchainVersion"), toolchains["KASKOLD_ANDROID_JDK"])

    def test_linux_android_build_bootstraps_wrapper_distribution_with_sha256(self) -> None:
        source = (ROOT / "scripts/linux/build/android-build.sh").read_text()
        wrapper = (ROOT / "apps/kaskold-companion-android/gradle/wrapper/gradle-wrapper.properties").read_text()
        self.assertIn("distributionSha256Sum=", wrapper)
        self.assertIn("distributionUrl=https\\://services.gradle.org/distributions/gradle-9.5.0-bin.zip", wrapper)
        self.assertIn('Pinned Gradle $GRADLE_VERSION is not installed; downloading and verifying it', source)
        self.assertIn("urllib.request.urlopen(url)", source)
        self.assertIn("Gradle SHA-256 mismatch", source)
        self.assertIn('GRADLE_USER_HOME="${GRADLE_USER_HOME:-$HOME/.gradle}"', source)
        self.assertIn('DAEMON_JVM_PROPERTIES="$ANDROID_APP/gradle/gradle-daemon-jvm.properties"', source)
        self.assertIn('required_java="${KASKOLD_ANDROID_JDK:-}"', source)
        self.assertIn('[[ "$daemon_java" == "$required_java" ]]', source)
        self.assertIn('$HOME/.local/share/kaskold/jdk-$required_java/bin/java', source)
        self.assertIn('install_managed_java()', source)
        self.assertIn('api.adoptium.net/v3/assets/latest/{major}/hotspot', source)
        self.assertIn('JDK SHA-256 mismatch', source)
        self.assertIn('java_bin="$(install_managed_java)"', source)
        self.assertNotIn('required_java=21', source)
        self.assertNotIn('required_java=17', source)
        self.assertNotIn('Install the verified Gradle 9.5.0 distribution before building.', source)

    def test_linux_android_build_discovers_project_and_common_sdk_locations(self) -> None:
        source = (ROOT / "scripts/linux/build/android-build.sh").read_text()
        sdk_helper = (ROOT / "scripts/linux/lib/android-sdk.sh").read_text()
        self.assertIn('read_local_sdk_dir', source)
        self.assertIn('"/mnt/Extra/android-dev/sdk"', source)
        self.assertIn('source "$ROOT/scripts/linux/lib/android-sdk.sh"', source)
        self.assertIn('ensure_android_sdk_build_packages', source)
        self.assertIn('AndroidVersion\\.ApiLevel', sdk_helper)
        self.assertIn('KASKOLD_ANDROID_API', sdk_helper)
        self.assertIn('platforms/android-$api', sdk_helper)
        self.assertIn('build-tools/$build_tools', sdk_helper)
        self.assertIn('sdk install --canary', sdk_helper)
        self.assertIn('--channel=3', sdk_helper)
        self.assertIn('KASKOLD_ANDROID_CMDLINE_TOOLS_LINUX_SHA256', sdk_helper)

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

    def test_windows_android_build_has_equivalent_verified_bootstrap(self) -> None:
        source = (ROOT / "scripts/windows/build/android-build.ps1").read_text()
        self.assertIn("distributionSha256Sum", source)
        self.assertIn("Invoke-WebRequest", source)
        self.assertIn("Get-FileHash", source)
        self.assertIn("Gradle SHA-256 mismatch", source)
        self.assertIn("Read-LocalSdk", source)
        self.assertIn("Find-AndroidPlatformJar", source)
        self.assertIn("Ensure-AndroidSdkBuildPackages", source)
        self.assertIn('"platforms/android-$Api"', source)
        self.assertIn('"build-tools/$BuildToolsVersion"', source)
        self.assertIn("'sdk', 'install', '--canary'", source)
        self.assertIn("'--channel=3'", source)


    def test_windows_android_build_bootstraps_pinned_vault_native_toolchain(self) -> None:
        source = (ROOT / "scripts/windows/build/android-build.ps1").read_text(encoding="utf-8")
        self.assertIn("function Ensure-RustAndroidTargets([string]$Toolchain)", source)
        self.assertIn("'target','add',$rustTarget,'--toolchain',$Toolchain", source)
        self.assertIn("Rust target $rustTarget could not be prepared", source)
        self.assertNotIn("Run: rustup target add $rustTarget", source)
        self.assertIn("function Ensure-CargoNdk([string]$Toolchain, [string]$Version)", source)
        self.assertIn("'install','cargo-ndk','--version',$Version,'--locked','--force'", source)
        self.assertIn("Pinned cargo-ndk $Version could not be prepared", source)
        self.assertIn("function Ensure-AndroidCommandLineTools([string]$Sdk)", source)
        self.assertIn("KASKOLD_ANDROID_CMDLINE_TOOLS_WINDOWS_SHA256", source)
        self.assertIn("Android command-line tools SHA-256 mismatch", source)
        self.assertIn("function Ensure-AndroidNdk([string]$Sdk, [string]$Version)", source)
        self.assertIn('Install-AndroidSdkPackages $Sdk @("ndk/$Version")', source)
        self.assertIn("Ensure-RustAndroidTargets $stableRust", source)
        self.assertIn("Ensure-CargoNdk $stableRust $cargoNdkVersion", source)
        self.assertIn("$ndkPath = Ensure-AndroidNdk $sdk $androidNdkVersion", source)

    def test_windows_android_build_selects_managed_java_without_native_stderr_failure(self) -> None:
        source = (ROOT / "scripts/windows/build/android-build.ps1").read_text(encoding="utf-8")
        self.assertIn("Import-KasKoldToolchains $root", source)
        self.assertIn("$daemonJvmProperties = Join-Path $android 'gradle/gradle-daemon-jvm.properties'", source)
        self.assertIn("$requiredJava = [int]$centralJavaText", source)
        self.assertIn("if ([int]$daemonJavaText -ne $requiredJava)", source)
        self.assertIn(".kaskold/tools/jdk-$($env:KASKOLD_ANDROID_JDK)/bin/java.exe", source)
        self.assertIn("Invoke-KasKoldCapture -Command $Java -Arguments @('-version')", source)
        self.assertIn("if (-not (Test-Path -LiteralPath $Java -PathType Leaf)) { return 0 }", source)
        self.assertIn("} catch {", source)
        self.assertIn("A stale or partially prepared managed JDK must be treated as absent", source)
        self.assertIn("$env:JAVA_HOME = Split-Path -Parent $javaBin", source)
        self.assertIn("function Install-ManagedJava([int]$RequiredMajor)", source)
        self.assertIn("api.adoptium.net/v3/assets/latest/$RequiredMajor/hotspot", source)
        self.assertIn("JDK SHA-256 mismatch", source)
        self.assertIn("$managed = Install-ManagedJava $RequiredMajor", source)
        self.assertIn("if ($major -eq $RequiredMajor)", source)
        self.assertNotIn("if ($major -ge $RequiredMajor)", source)
        self.assertNotIn("& java -version 2>&1", source)

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

    def test_windows_android_bootstrap_probes_native_versions_through_safe_capture(self) -> None:
        installer = (ROOT / "scripts/windows/install/install.ps1").read_text(encoding="utf-8")
        studio = (ROOT / "scripts/windows/build/android-studio.ps1").read_text(encoding="utf-8")
        self.assertIn("Invoke-KasKoldCapture -Command $targetJava -Arguments @('-version')", installer)
        self.assertIn("Invoke-KasKoldCapture -Command $managedJava -Arguments @('-version')", installer)
        self.assertIn("Invoke-KasKoldCapture -Command 'gradle' -Arguments @('--version')", installer)
        self.assertIn("Invoke-KasKoldCapture -Command 'kotlinc' -Arguments @('-version')", installer)
        self.assertNotIn("& java -version 2>&1", installer)
        self.assertIn("Invoke-KasKoldCapture -Command $java -Arguments @('-version')", studio)


if __name__ == "__main__":
    unittest.main()
