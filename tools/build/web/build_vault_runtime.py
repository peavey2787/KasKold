#!/usr/bin/env python3
"""Canonical cross-platform KasKold Vault Web/WASM runtime builder.

Build the standalone, network-free Vault Web signer from the pinned Rust and wasm-bindgen toolchain.
"""
from __future__ import annotations

import argparse
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
APP = ROOT / "apps" / "kaskold-vault-web"
TOOLCHAINS = ROOT / "qa/config/toolchains.env"
WASM_TARGET = "wasm32-unknown-unknown"
BINDGEN_STEM = "vault_web"


def cargo_wasm_artifact_stem() -> str:
    """Return Cargo's cdylib artifact stem for the Vault Web crate.

    Cargo normalizes hyphens in package/lib target names to underscores for
    emitted library artifacts. Keep that internal Cargo artifact name separate
    from the stable browser package name used by wasm-bindgen.
    """
    manifest = tomllib.loads((APP / "Cargo.toml").read_text(encoding="utf-8"))
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


def metadata(toolchain: str, env: dict[str, str], *extra: str) -> subprocess.CompletedProcess[str]:
    return run(
        cargo_args(
            toolchain,
            "metadata",
            "--manifest-path",
            str(APP / "Cargo.toml"),
            "--format-version",
            "1",
            "--filter-platform",
            WASM_TARGET,
            *extra,
        ),
        env=env,
        capture=True,
    )


def ensure_lock_current(toolchain: str, env: dict[str, str]) -> None:
    lock = APP / "Cargo.lock"
    try:
        metadata(toolchain, env, "--locked")
        return
    except subprocess.CalledProcessError:
        pass

    original = lock.read_bytes()
    print(f"KasKold Vault Web — Cargo.lock is stale under pinned Cargo {toolchain}; reconciling transactionally.", file=sys.stderr)
    try:
        try:
            metadata(toolchain, env, "--offline")
        except subprocess.CalledProcessError:
            print("KasKold Vault Web — Offline reconciliation was insufficient; retrying with registry access.", file=sys.stderr)
            metadata(toolchain, env)
        metadata(toolchain, env, "--locked")
    except Exception:
        lock.write_bytes(original)
        raise
    print("KasKold Vault Web — reconciled Cargo.lock verified with --locked.", file=sys.stderr)


def locked_wasm_bindgen_version(toolchain: str, env: dict[str, str]) -> str:
    data = json.loads(metadata(toolchain, env, "--locked").stdout)
    versions = sorted({p["version"] for p in data.get("packages", []) if p.get("name") == "wasm-bindgen"})
    if len(versions) != 1:
        raise RuntimeError("expected exactly one locked wasm-bindgen version in KasKold Vault Web metadata")
    return versions[0]


def ensure_target(toolchain: str, env: dict[str, str]) -> None:
    installed = run(["rustup", "target", "list", "--toolchain", toolchain, "--installed"], env=env, capture=True).stdout.splitlines()
    if WASM_TARGET not in installed:
        print(f"KasKold Vault Web — Installing {WASM_TARGET} for Rust {toolchain}...")
        run(["rustup", "target", "add", WASM_TARGET, "--toolchain", toolchain], env=env)


def wasm_bindgen_binary(pin: str) -> Path:
    cache_base = Path(
        os.environ.get("KASKOLD_TOOL_CACHE_DIR")
        or (Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "kaskold" / "tools")
    )
    suffix = ".exe" if os.name == "nt" else ""
    return cache_base / f"wasm-bindgen-cli-{pin}" / "bin" / f"wasm-bindgen{suffix}"


def ensure_wasm_bindgen(toolchain: str, pin: str, env: dict[str, str]) -> Path:
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
        print(f"KasKold Vault Web — Installing pinned {expected} into isolated KasKold tool cache...")
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


def copy_site(site: Path) -> str:
    authored = APP / "web"
    # Never copy a stale local runtime into the canonical deployable site.
    shutil.rmtree(authored / "pkg", ignore_errors=True)
    revision = authored_asset_revision(authored)
    shutil.rmtree(site, ignore_errors=True)
    shutil.copytree(authored, site)
    (site / "pkg").mkdir(parents=True, exist_ok=True)
    stamp_site_asset_revision(site, revision)
    return revision


def sync_local_web_package(site: Path) -> None:
    """Mirror the freshly generated runtime for direct local web serving.

    target/kaskold-vault-web/site is canonical for deployment.
    apps/kaskold-vault-web/web/pkg is generated convenience output only and is excluded
    from repository inventory/source archives.
    """
    source = site / "pkg"
    local = APP / "web" / "pkg"
    shutil.rmtree(local, ignore_errors=True)
    shutil.copytree(source, local)


def build(mode: str) -> Path:
    pins = load_env(TOOLCHAINS)
    toolchain = pins["KASKOLD_STABLE_RUST"]
    bindgen_pin = pins["KASKOLD_WASM_BINDGEN_CLI_VERSION"]
    env = host_env(toolchain)
    if shutil.which("rustup", path=env.get("PATH")) is None:
        raise RuntimeError("rustup is required to build the Vault Web runtime")
    ensure_cargo_toolchain(toolchain, env)

    print("KasKold Vault Web — Preparing authored Vault Web assets...")
    ensure_lock_current(toolchain, env)
    locked = locked_wasm_bindgen_version(toolchain, env)
    if locked != bindgen_pin:
        raise RuntimeError(f"wasm-bindgen crate/CLI mismatch: lock={locked} pin={bindgen_pin}")
    ensure_target(toolchain, env)
    bindgen = ensure_wasm_bindgen(toolchain, bindgen_pin, env)

    site = Path(os.environ.get("KASKOLD_VAULT_WEB_SITE", ROOT / "target/kaskold-vault-web/site"))
    print(f"KasKold Vault Web — Preparing deployable site under target/ ({mode})...")
    revision = copy_site(site)
    print(f"KasKold Vault Web — Static asset revision {revision}.")

    target_dir = ROOT / "target/kaskold-vault-web-wasm"
    args = cargo_args(
        toolchain,
        "build",
        "--manifest-path",
        str(APP / "Cargo.toml"),
        "--locked",
        "--target",
        WASM_TARGET,
    )
    profile = "debug"
    if mode == "release":
        args.append("--release")
        profile = "release"
    build_env = env.copy()
    build_env["CARGO_TARGET_DIR"] = str(target_dir)
    print(f"KasKold Vault Web — Building WASM ({mode})...")
    run(args, env=build_env)

    wasm = target_dir / WASM_TARGET / profile / f"{cargo_wasm_artifact_stem()}.wasm"
    if not wasm.is_file() or wasm.stat().st_size == 0:
        raise RuntimeError(f"missing compiled Vault Web WASM: {wasm}")
    print(f"KasKold Vault Web — Generating browser bindings with wasm-bindgen {bindgen_pin}...")
    run([
        str(bindgen),
        "--target", "web",
        "--out-dir", str(site / "pkg"),
        "--out-name", BINDGEN_STEM,
        str(wasm),
    ], env=env)
    for name in (f"{BINDGEN_STEM}.js", f"{BINDGEN_STEM}_bg.wasm"):
        path = site / "pkg" / name
        if not path.is_file() or path.stat().st_size == 0:
            raise RuntimeError(f"wasm-bindgen did not generate {path}")
    sync_local_web_package(site)
    print(f"Vault Web runtime ready: {site}")
    print(f"Vault Web local runtime ready: {APP / 'web'}")
    return site


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("release", "dev"), default="release")
    args = parser.parse_args()
    try:
        build(args.mode)
    except subprocess.CalledProcessError as exc:
        print(f"ERROR: {subprocess_error_detail(exc)}", file=sys.stderr)
        return 2
    except (KeyError, RuntimeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
