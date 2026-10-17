"""Locate the MSVC AddressSanitizer runtime that cargo-fuzz binaries load.

`cargo fuzz` on MSVC links libFuzzer's `main` and the dynamic ASan runtime;
the fuzz binary only starts when `clang_rt.asan_dynamic-x86_64.dll` is on PATH.
"""

from __future__ import annotations

import os
from pathlib import Path
import subprocess

ASAN_DLL = "clang_rt.asan_dynamic-x86_64.dll"
VSWHERE = Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")) / "Microsoft Visual Studio/Installer/vswhere.exe"


def asan_runtime_dir() -> Path:
    """Directory of the newest MSVC toolset's x64 ASan runtime DLL."""
    install = subprocess.run(
        [str(VSWHERE), "-latest", "-products", "*", "-requires",
         "Microsoft.VisualStudio.Component.VC.ASAN", "-property", "installationPath"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    runtimes = sorted(Path(install).glob(f"VC/Tools/MSVC/*/bin/HostX64/x64/{ASAN_DLL}"))
    if not install or not runtimes:
        raise RuntimeError(f"MSVC AddressSanitizer runtime {ASAN_DLL} is not installed")
    return runtimes[-1].parent


def with_asan_path(env: dict[str, str]) -> dict[str, str]:
    """Copy of `env` whose PATH starts with the ASan runtime directory."""
    return {**env, "PATH": f"{asan_runtime_dir()}{os.pathsep}{env.get('PATH', '')}"}


if __name__ == "__main__":
    print(asan_runtime_dir())
