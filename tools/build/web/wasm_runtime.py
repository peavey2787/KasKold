"""Shared Web/WASM runtime builder behind the Companion and Vault Web entry points.

Cargo lock reconciliation, Rust/wasm-bindgen pins, authored asset generation,
and target/ output ownership stay identical across products and hosts.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib

WEB_BUILD_DIR = Path(__file__).resolve().parent
if str(WEB_BUILD_DIR) not in sys.path:
    sys.path.insert(0, str(WEB_BUILD_DIR))
from static_asset_revision import authored_asset_revision, stamp_site_asset_revision

ROOT = Path(__file__).resolve().parents[3]
TOOLCHAINS = ROOT / "qa/config/toolchains.env"
WASM_TARGET = "wasm32-unknown-unknown"
VENDORED_JSQR = ROOT / "apps/web-common/lib/jsQR.js"


@dataclass(frozen=True)
class WebRuntime:
    """What differs between one Web/WASM product and another."""

    label: str
    app: Path
    bindgen_stem: str
    site_env: str
    site_default: Path
    target_dir: Path
    jsqr_url: str = "lib/jsQR.js"
    authored_generators: tuple[Path, ...] = ()

    def log(self, message: str, *, file=None) -> None:
        print(f"{self.label} — {message}", file=file or sys.stdout)


def cargo_wasm_artifact_stem(runtime: WebRuntime) -> str:
    """Return Cargo's cdylib artifact stem for the product crate.

    Cargo normalizes hyphens in package/lib target names to underscores for
    emitted library artifacts. Keep that internal Cargo artifact name separate
    from the stable browser package name used by wasm-bindgen.
    """
    manifest = tomllib.loads((runtime.app / "Cargo.toml").read_text(encoding="utf-8"))
    lib_name = manifest.get("lib", {}).get("name")
    target_name = lib_name or manifest["package"]["name"]
    return str(target_name).replace("-", "_")


def load_env(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


def host_env(toolchain: str) -> dict[str, str]:
    env = os.environ.copy()
    for key in ("RUSTC", "RUSTDOC", "CARGO_BUILD_TARGET", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"):
        env.pop(key, None)
    env["RUSTUP_TOOLCHAIN"] = toolchain
    env["CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS"] = "fallback"
    cargo_bin = str(Path.home() / ".cargo" / "bin")
    env["PATH"] = cargo_bin + os.pathsep + env.get("PATH", "")
    return env


def run(args: list[str], *, env: dict[str, str] | None = None, capture: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=ROOT,
        env=env,
        text=True,
        check=True,
        capture_output=capture,
    )


def cargo_args(toolchain: str, *args: str) -> list[str]:
    return ["rustup", "run", toolchain, "cargo", *args]


def subprocess_error_detail(exc: subprocess.CalledProcessError) -> str:
    """Render captured Cargo/tool stderr instead of hiding it behind exit status 2."""
    parts = [f"command failed with exit code {exc.returncode}: {subprocess.list2cmdline(exc.cmd)}"]
    for label, value in (("stdout", exc.stdout), ("stderr", exc.stderr)):
        if value:
            text = str(value).strip()
            if text:
                parts.append(f"{label}:\n{text}")
    return "\n".join(parts)


def ensure_cargo_toolchain(toolchain: str, env: dict[str, str]) -> None:
    try:
        run(cargo_args(toolchain, "--version"), env=env, capture=True)
    except subprocess.CalledProcessError as exc:
        detail = (exc.stderr or exc.stdout or "").strip()
        suffix = f": {detail}" if detail else ""
        raise RuntimeError(f"pinned Rust toolchain {toolchain} is not installed or usable{suffix}") from exc


def metadata(runtime: WebRuntime, toolchain: str, env: dict[str, str], *extra: str) -> subprocess.CompletedProcess[str]:
    return run(
        cargo_args(
            toolchain,
            "metadata",
            "--manifest-path",
            str(runtime.app / "Cargo.toml"),
            "--format-version",
            "1",
            "--filter-platform",
            WASM_TARGET,
            *extra,
        ),
        env=env,
        capture=True,
    )


def ensure_lock_current(runtime: WebRuntime, toolchain: str, env: dict[str, str]) -> None:
    lock = runtime.app / "Cargo.lock"
    try:
        metadata(runtime, toolchain, env, "--locked")
        return
    except subprocess.CalledProcessError:
        pass

    original = lock.read_bytes()
    runtime.log(f"Cargo.lock is stale under pinned Cargo {toolchain}; reconciling transactionally...", file=sys.stderr)
    try:
        try:
            metadata(runtime, toolchain, env, "--offline")
        except subprocess.CalledProcessError:
            runtime.log("Offline reconciliation was insufficient; retrying with registry access...", file=sys.stderr)
            metadata(runtime, toolchain, env)
        metadata(runtime, toolchain, env, "--locked")
    except Exception:
        lock.write_bytes(original)
        raise
    runtime.log("reconciled Cargo.lock verified with --locked.", file=sys.stderr)


def locked_wasm_bindgen_version(runtime: WebRuntime, toolchain: str, env: dict[str, str]) -> str:
    data = json.loads(metadata(runtime, toolchain, env, "--locked").stdout)
    versions = sorted({p["version"] for p in data.get("packages", []) if p.get("name") == "wasm-bindgen"})
    if len(versions) != 1:
        raise RuntimeError(f"expected exactly one locked wasm-bindgen version in {runtime.label} metadata")
    return versions[0]


def ensure_target(runtime: WebRuntime, toolchain: str, env: dict[str, str]) -> None:
    installed = run(["rustup", "target", "list", "--toolchain", toolchain, "--installed"], env=env, capture=True).stdout
    if WASM_TARGET not in installed:
        runtime.log(f"Installing {WASM_TARGET} for Rust {toolchain}...")
        run(["rustup", "target", "add", WASM_TARGET, "--toolchain", toolchain], env=env)


def wasm_bindgen_binary(pin: str) -> Path:
    cache_base = Path(
        os.environ.get("KASKOLD_TOOL_CACHE_DIR")
        or (Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "kaskold" / "tools")
    )
    suffix = ".exe" if os.name == "nt" else ""
    return cache_base / f"wasm-bindgen-cli-{pin}" / "bin" / f"wasm-bindgen{suffix}"


def ensure_wasm_bindgen(runtime: WebRuntime, toolchain: str, pin: str, env: dict[str, str]) -> Path:
    binary = wasm_bindgen_binary(pin)
    expected = f"wasm-bindgen {pin}"
    actual = ""
    if binary.is_file():
        try:
            actual = run([str(binary), "--version"], env=env, capture=True).stdout.strip()
        except subprocess.CalledProcessError:
            actual = ""
    if actual != expected:
        root = binary.parents[1]
        shutil.rmtree(root, ignore_errors=True)
        runtime.log(f"Installing pinned {expected} into isolated KasKold tool cache...")
        run(
            cargo_args(
                toolchain,
                "install",
                "wasm-bindgen-cli",
                "--version",
                pin,
                "--locked",
                "--root",
                str(root),
            ),
            env=env,
        )
    actual = run([str(binary), "--version"], env=env, capture=True).stdout.strip()
    if actual != expected:
        raise RuntimeError(f"expected {expected}, received {actual or 'missing'}")
    return binary


def sync_vendored_assets(runtime: WebRuntime) -> None:
    """Mirror the library both web apps ship into this app's git-ignored serving tree."""
    destination = runtime.app / "web" / runtime.jsqr_url
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(VENDORED_JSQR, destination)


def copy_site(runtime: WebRuntime, site: Path) -> str:
    authored = runtime.app / "web"
    sync_vendored_assets(runtime)
    # Never copy a stale local runtime into the canonical deployable site.
    shutil.rmtree(authored / "pkg", ignore_errors=True)
    revision = authored_asset_revision(authored)
    shutil.rmtree(site, ignore_errors=True)
    shutil.copytree(authored, site)
    (site / "pkg").mkdir(parents=True, exist_ok=True)
    stamp_site_asset_revision(site, revision)
    return revision


def sync_local_web_package(runtime: WebRuntime, site: Path) -> None:
    """Mirror the freshly generated runtime for direct local web serving.

    The site under target/ stays canonical for deployment and mobile consumers.
    The app's web/pkg is generated convenience output only and is excluded from
    repository inventory and source archives.
    """
    source = site / "pkg"
    local = runtime.app / "web" / "pkg"
    shutil.rmtree(local, ignore_errors=True)
    shutil.copytree(source, local)


def _prepare_toolchain(runtime: WebRuntime, pins: dict[str, str], env: dict[str, str]) -> Path:
    toolchain = pins["KASKOLD_STABLE_RUST"]
    bindgen_pin = pins["KASKOLD_WASM_BINDGEN_CLI_VERSION"]
    ensure_lock_current(runtime, toolchain, env)
    locked = locked_wasm_bindgen_version(runtime, toolchain, env)
    if locked != bindgen_pin:
        raise RuntimeError(f"wasm-bindgen crate/CLI mismatch: lock={locked} pin={bindgen_pin}")
    ensure_target(runtime, toolchain, env)
    return ensure_wasm_bindgen(runtime, toolchain, bindgen_pin, env)


def _compile_wasm(runtime: WebRuntime, toolchain: str, mode: str, env: dict[str, str]) -> Path:
    args = cargo_args(
        toolchain,
        "build",
        "--manifest-path",
        str(runtime.app / "Cargo.toml"),
        "--locked",
        "--target",
        WASM_TARGET,
    )
    profile = "debug"
    if mode == "release":
        args.append("--release")
        profile = "release"
    build_env = env.copy()
    build_env["CARGO_TARGET_DIR"] = str(runtime.target_dir)
    runtime.log(f"Building WASM ({mode})...")
    run(args, env=build_env)

    wasm = runtime.target_dir / WASM_TARGET / profile / f"{cargo_wasm_artifact_stem(runtime)}.wasm"
    if not wasm.is_file() or wasm.stat().st_size == 0:
        raise RuntimeError(f"missing compiled {runtime.label} WASM: {wasm}")
    return wasm


def _generate_bindings(runtime: WebRuntime, bindgen: Path, wasm: Path, site: Path, env: dict[str, str]) -> None:
    runtime.log("Generating browser bindings with wasm-bindgen...")
    run([
        str(bindgen),
        "--target", "web",
        "--out-dir", str(site / "pkg"),
        "--out-name", runtime.bindgen_stem,
        str(wasm),
    ], env=env)
    for name in (f"{runtime.bindgen_stem}.js", f"{runtime.bindgen_stem}_bg.wasm"):
        path = site / "pkg" / name
        if not path.is_file() or path.stat().st_size == 0:
            raise RuntimeError(f"wasm-bindgen did not generate {path}")


def build(runtime: WebRuntime, mode: str) -> Path:
    pins = load_env(TOOLCHAINS)
    toolchain = pins["KASKOLD_STABLE_RUST"]
    env = host_env(toolchain)

    if shutil.which("rustup", path=env.get("PATH")) is None:
        raise RuntimeError(f"rustup is required to build the {runtime.label} runtime")
    ensure_cargo_toolchain(toolchain, env)

    runtime.log("Regenerating authored assets...")
    for generator in runtime.authored_generators:
        run([sys.executable, str(generator)])
    bindgen = _prepare_toolchain(runtime, pins, env)

    site = Path(os.environ.get(runtime.site_env, runtime.site_default))
    runtime.log(f"Preparing deployable site under target/ ({mode})...")
    revision = copy_site(runtime, site)
    runtime.log(f"Static asset revision {revision}.")

    wasm = _compile_wasm(runtime, toolchain, mode, env)
    _generate_bindings(runtime, bindgen, wasm, site, env)
    sync_local_web_package(runtime, site)
    runtime.log(f"runtime ready: {site}")
    runtime.log(f"local web runtime ready: {runtime.app / 'web'}")
    return site


def main(runtime: WebRuntime) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("release", "dev"), default="release")
    args = parser.parse_args()
    try:
        build(runtime, args.mode)
    except subprocess.CalledProcessError as exc:
        print(f"ERROR: {subprocess_error_detail(exc)}", file=sys.stderr)
        return 2
    except (KeyError, RuntimeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    return 0
