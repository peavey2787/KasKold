#!/usr/bin/env python3
"""Regression coverage for the self-provisioning QEMU workflow."""

from pathlib import Path
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]

import sys
sys.path.insert(0, str(ROOT / "qa/checks"))
from toolchains import load_toolchains, qa_bash_executable  # noqa: E402

PINS = load_toolchains()


def bash_path(path: Path) -> str:
    """Return a native path translated for the Bash implementation under test."""
    resolved = path.resolve()
    text = resolved.as_posix()
    if os.name != "nt" or len(text) < 3 or text[1:3] != ":/":
        return text

    # Windows CI/developer hosts may resolve ``bash`` to Git Bash/MSYS or to
    # WSL. Their mount syntaxes differ (/c/... vs /mnt/c/...), so do not
    # hard-code either convention. Ask the selected Bash to translate the
    # exact native path using its own path utility.
    environment = os.environ.copy()
    environment["KASKOLD_QA_NATIVE_PATH"] = str(resolved)
    translated = subprocess.run(
        [
            qa_bash_executable(),
            "-lc",
            'if command -v wslpath >/dev/null 2>&1; then '
            'wslpath -u "$KASKOLD_QA_NATIVE_PATH"; '
            'elif command -v cygpath >/dev/null 2>&1; then '
            'cygpath -u "$KASKOLD_QA_NATIVE_PATH"; '
            'else exit 127; fi',
        ],
        env=environment,
        check=False,
        capture_output=True,
        text=True,
    )
    converted = translated.stdout.strip()
    if translated.returncode == 0 and converted:
        return converted

    # Conservative MSYS fallback for minimal Bash installations without
    # cygpath. WSL normally always provides wslpath, so reaching this branch
    # there indicates a broken test host and will fail transparently later.
    return f"/{text[0].lower()}{text[2:]}"


class QemuScriptTests(unittest.TestCase):


    @unittest.skipUnless(os.name == "posix", "Espressif QEMU setup execution is POSIX-specific")
    def test_espressif_setup_resolves_qemu_without_idf_activation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            idf_path = root / "esp-idf"
            idf_tools = idf_path / "tools/idf_tools.py"
            tools_root = root / "idf-tools"
            idf_tools.parent.mkdir(parents=True)
            idf_tools.write_text(
                "from pathlib import Path\n"
                "import os\n"
                "import sys\n"
                "assert sys.argv[1:] == ['install', 'qemu-xtensa']\n"
                "binary = (Path(os.environ['IDF_TOOLS_PATH']) / "
                "'tools/qemu-xtensa/v1/qemu/bin/qemu-system-xtensa')\n"
                "binary.parent.mkdir(parents=True, exist_ok=True)\n"
                "binary.write_text('#!/bin/sh\\necho qemu-stub\\n')\n"
                "binary.chmod(0o755)\n"
            )

            command = (
                'source "scripts/linux/lib/qemu-common.sh"\n'
                'source "scripts/linux/lib/qemu-espressif.sh"\n'
                "install_espressif_qemu\n"
                "printf 'SELECTED=%s\\n' \"$QEMU_SYSTEM_XTENSA\"\n"
            )
            environment = os.environ.copy()
            environment.update(
                {
                    "HOME": bash_path(root / "home"),
                    "PATH": "/usr/bin:/bin",
                    "KASKOLD_IDF_PATH": bash_path(idf_path),
                    "IDF_TOOLS_PATH": bash_path(tools_root),
                }
            )
            completed = subprocess.run(
                [qa_bash_executable(), "-c", command],
                cwd=ROOT,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                completed.returncode,
                0,
                completed.stdout + completed.stderr,
            )
            expected = (
                tools_root
                / "tools/qemu-xtensa/v1/qemu/bin/qemu-system-xtensa"
            )
            self.assertIn(f"SELECTED={expected}", completed.stdout)
            self.assertNotIn("Activating ESP-IDF", completed.stdout)


    def test_sudo_is_centralized_behind_terminal_helper(self) -> None:
        offenders = []
        for path in (ROOT / "scripts").rglob("*.sh"):
            if path == ROOT / "scripts/linux/lib/admin.sh":
                continue
            if "sudo " in path.read_text():
                offenders.append(str(path.relative_to(ROOT)))
        self.assertEqual(offenders, [])


    def test_existing_esp_directory_passes_runtime_probe(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary)
            bin_dir = home / "bin"
            toolchain_bin = home / ".rustup/toolchains/esp/bin"
            bin_dir.mkdir()
            toolchain_bin.mkdir(parents=True)

            (bin_dir / "rustup").write_text(
                "#!/bin/sh\n"
                "if [ \"$1\" = run ]; then exit 1; fi\n"
                "exit 0\n"
            )
            (bin_dir / "cargo").write_text(
                "#!/bin/sh\n"
                "if [ \"$1\" = install ]; then exit 91; fi\n"
                f"echo 'cargo {PINS['KASKOLD_STABLE_RUST']}'\n"
            )
            (toolchain_bin / "rustc").write_text(
                f"#!/bin/sh\necho 'rustc {PINS['KASKOLD_ESP_RUST']} (esp)'\n"
            )
            for executable in (
                bin_dir / "rustup",
                bin_dir / "cargo",
                toolchain_bin / "rustc",
            ):
                executable.chmod(0o755)

            command = (
                'source "scripts/linux/lib/qemu-common.sh"\n'
                'source "scripts/linux/lib/qemu-rust.sh"\n'
                "install_esp_rust_toolchain\n"
            )
            environment = os.environ.copy()
            environment.update(
                {
                    "HOME": bash_path(home),
                    "PATH": f"{bash_path(bin_dir)}:/usr/bin:/bin",
                    "RUSTUP_HOME": bash_path(home / ".rustup"),
                    "CARGO_HOME": bash_path(home / ".cargo"),
                }
            )
            completed = subprocess.run(
                [qa_bash_executable(), "-c", command],
                cwd=ROOT,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                completed.returncode,
                0,
                completed.stdout + completed.stderr,
            )
            self.assertIn("ESP Rust toolchain ready", completed.stdout)

    def test_admin_explanation_precedes_password_validation(self) -> None:
        # Define sudo in the Bash process itself instead of placing a fake
        # executable in a Windows temporary directory. That keeps this ordering
        # regression independent of WSL/MSYS path translation, mount options,
        # and Windows executable-bit semantics. `command -v sudo` recognizes
        # shell functions, so request_admin_access exercises its real branches.
        command = (
            'source "scripts/linux/lib/admin.sh"; '
            'sudo() { '
            'if [[ "$1" == "-n" ]]; then return 1; fi; '
            'if [[ "$1" == "-v" ]]; then printf "SUDO_VALIDATE\n" >&2; return 0; fi; '
            'return 0; '
            '}; '
            'request_admin_access "Install QEMU host libraries."'
        )
        environment = os.environ.copy()
        environment.update(
            {
                "DISPLAY": "",
                "WAYLAND_DISPLAY": "",
                "DBUS_SESSION_BUS_ADDRESS": "",
            }
        )
        completed = subprocess.run(
            [qa_bash_executable(), "-c", command],
            cwd=ROOT,
            env=environment,
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("Reason: Install QEMU host libraries.", completed.stderr)
        self.assertIn("SUDO_VALIDATE", completed.stderr)
        self.assertLess(
            completed.stderr.index("Reason: Install QEMU host libraries."),
            completed.stderr.index("SUDO_VALIDATE"),
        )


if __name__ == "__main__":
    unittest.main()
