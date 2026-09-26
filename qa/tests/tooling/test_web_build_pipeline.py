#!/usr/bin/env python3
"""Ensure every Companion runtime consumer shares one pinned cross-platform builder."""

from pathlib import Path
import importlib.util
import os
import subprocess
import sys
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[3]
BUILDER = ROOT / "tools/build/web/build_companion_runtime.py"
LINUX_FACADE = ROOT / "apps/kaskold-companion-web/build.sh"
WINDOWS_FACADE = ROOT / "apps/kaskold-companion-web/build.ps1"


class WebBuildPipelineTests(unittest.TestCase):
    def test_web_asset_generators_are_utf8_explicit_under_ascii_locale(self) -> None:
        env = os.environ.copy()
        env.update({"PYTHONUTF8": "0", "LC_ALL": "C", "LANG": "C"})
        for builder in (
            "tools/build/web/build_web_index.py",
            "tools/build/web/build_app_css.py",
        ):
            result = subprocess.run(
                [sys.executable, str(ROOT / builder), "--check"],
                cwd=ROOT,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                encoding="utf-8",
                errors="replace",
                check=False,
            )
            self.assertEqual(result.returncode, 0, f"{builder}:\n{result.stdout}")

    def test_web_dom_contract_is_utf8_explicit_under_ascii_locale(self) -> None:
        env = os.environ.copy()
        env.update({
            "PYTHONUTF8": "0",
            "PYTHONCOERCECLOCALE": "0",
            "LC_ALL": "C",
            "LANG": "C",
        })
        result = subprocess.run(
            [sys.executable, str(ROOT / "qa/checks/web/check_web_dom_contract.py")],
            cwd=ROOT,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout)


    def test_safe_html_checker_normalizes_windows_paths_and_utf8(self) -> None:
        checker = ROOT / "qa/checks/web/check_safe_html.py"
        source = checker.read_text(encoding="utf-8")
        self.assertIn("replace('\\\\', '/')", source)
        self.assertIn("read_text(encoding='utf-8', errors='replace')", source)

        # Reproduce the Windows path form that previously bypassed the QR allowlist.
        windows_rel = r"features\transactions\send\review.js"
        normalized = windows_rel.replace("\\", "/")
        self.assertEqual(normalized, "features/transactions/send/review.js")

        env = os.environ.copy()
        env.update({
            "PYTHONUTF8": "0",
            "PYTHONCOERCECLOCALE": "0",
            "LC_ALL": "C",
            "LANG": "C",
        })
        result = subprocess.run(
            [sys.executable, str(checker)],
            cwd=ROOT,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout)

    def test_all_asset_generators_run_before_wasm_build(self) -> None:
        source = BUILDER.read_text(encoding="utf-8")
        wasm = source.index('"build",')
        for builder in (
            "tools/build/web/build_web_index.py",
            "tools/build/web/build_app_css.py",
        ):
            self.assertLess(source.index(builder), wasm)

    def test_full_qa_runs_crap_before_the_normal_companion_build(self) -> None:
        catalog = (ROOT / "qa/config/run_all_steps.tsv").read_text(encoding="utf-8")
        dispatch = (ROOT / "qa/linux/runner/catalog.sh").read_text(encoding="utf-8")
        ids = [line.split("\t")[3] for line in catalog.splitlines() if line and not line.startswith("#")]
        self.assertEqual(ids[0], "preflight.crap-check")
        self.assertLess(ids.index("preflight.crap-check"), ids.index("preflight.companion-build"))
        self.assertIn('run_in_directory "$ROOT_DIR" bash scripts/linux/build/kaskold-companion-web-build.sh', dispatch)
        self.assertNotIn('run_in_directory "$ROOT_DIR" make companion', dispatch)
        self.assertIn('require_command rustup', dispatch)

    def test_browser_stage_persists_recovery_coverage(self) -> None:
        coverage = (ROOT / "scripts/linux/quality/crap.sh").read_text(encoding="utf-8")
        self.assertIn("run_web_recovery_coverage.py", coverage)
        runner = (ROOT / "qa/checks/web/run_web_recovery_coverage.py").read_text(encoding="utf-8")
        self.assertIn("NODE_V8_COVERAGE", runner)
        self.assertIn("thresholds_enforced_by", runner)
        self.assertIn("expected - measured", runner)

    def test_wasm_package_is_rebuilt_from_clean_generated_output(self) -> None:
        source = BUILDER.read_text(encoding="utf-8")
        clean = source.index('shutil.rmtree(authored / "pkg"')
        bindgen = source.index('"--target", "web"')
        mirror = source.index("sync_local_web_package(site)")
        self.assertLess(clean, bindgen)
        self.assertLess(bindgen, mirror)
        self.assertIn('ROOT / "target/kaskold-companion-web/site"', source)
        self.assertIn('local = APP / "web" / "pkg"', source)
        self.assertIn('shutil.copytree(source, local)', source)

    def test_local_web_runtime_sync_copies_generated_bindings(self) -> None:
        spec = importlib.util.spec_from_file_location("build_companion_runtime_test", BUILDER)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            module.APP = root / "apps" / "kaskold-companion-web"
            site = root / "target" / "kaskold-companion-web" / "site"
            (site / "pkg").mkdir(parents=True)
            (site / "pkg" / "companion_web.js").write_text("export default function init() {}\n")
            (site / "pkg" / "companion_web_bg.wasm").write_bytes(b"wasm")
            local = module.APP / "web" / "pkg"
            local.mkdir(parents=True)
            (local / "stale.js").write_text("stale\n")

            module.sync_local_web_package(site)

            self.assertFalse((local / "stale.js").exists())
            self.assertEqual((local / "companion_web.js").read_text(encoding="utf-8"), "export default function init() {}\n")
            self.assertEqual((local / "companion_web_bg.wasm").read_bytes(), b"wasm")


    def test_companion_site_revision_covers_html_assets_and_every_module_edge(self) -> None:
        spec = importlib.util.spec_from_file_location("build_companion_runtime_revision", BUILDER)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            module.APP = root / "apps" / "kaskold-companion-web"
            authored = module.APP / "web"
            (authored / "css").mkdir(parents=True)
            (authored / "js" / "app").mkdir(parents=True)
            (authored / "img").mkdir(parents=True)
            (authored / "lib").mkdir(parents=True)
            (authored / "index.html").write_text(
                '<link rel="stylesheet" href="css/app.css?v=102">\n'
                '<img src="img/logo-sm.png">\n'
                '<script src="lib/jsQR.js"></script>\n'
                '<script type="module" src="js/main.js"></script>\n',
                encoding="utf-8",
            )
            (authored / "css" / "app.css").write_text("body{}\n", encoding="utf-8")
            (authored / "img" / "logo-sm.png").write_bytes(b"logo")
            (authored / "lib" / "jsQR.js").write_text("globalThis.jsQR = {};\n", encoding="utf-8")
            (authored / "js" / "main.js").write_text(
                "import './side_effect.js';\n"
                "import { value } from './app/named.js';\n"
                "export async function boot() { return import('./app/dynamic.js'); }\n"
                "void value;\n",
                encoding="utf-8",
            )
            (authored / "js" / "side_effect.js").write_text("export {};\n", encoding="utf-8")
            (authored / "js" / "app" / "named.js").write_text("export const value = 1;\n", encoding="utf-8")
            (authored / "js" / "app" / "dynamic.js").write_text("export default 1;\n", encoding="utf-8")

            site = root / "target" / "kaskold-companion-web" / "site"
            revision = module.copy_site(site)
            html = (site / "index.html").read_text(encoding="utf-8")
            main = (site / "js" / "main.js").read_text(encoding="utf-8")

            self.assertNotIn("?v=102", html)
            for path in ("css/app.css", "img/logo-sm.png", "lib/jsQR.js", "js/main.js"):
                self.assertIn(f"{path}?rev={revision}", html)
            self.assertIn(f"./side_effect.js?rev={revision}", main)
            self.assertIn(f"./app/named.js?rev={revision}", main)
            self.assertIn(f"./app/dynamic.js?rev={revision}", main)

    def test_build_facades_delegate_without_duplicating_runtime_logic(self) -> None:
        for facade in (LINUX_FACADE, WINDOWS_FACADE):
            source = facade.read_text(encoding="utf-8")
            self.assertIn("build_companion_runtime.py", source)
            self.assertNotIn("cargo build", source)
            self.assertNotIn("wasm-bindgen-cli", source)

    def test_builder_fails_closed_on_unknown_mode(self) -> None:
        source = BUILDER.read_text(encoding="utf-8")
        self.assertIn('choices=("release", "dev")', source)
        self.assertIn("return 2", source)

    def test_companion_cargo_artifact_name_is_distinct_from_stable_browser_binding_name(self) -> None:
        spec = importlib.util.spec_from_file_location("build_companion_runtime_artifact", BUILDER)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        self.assertEqual(module.cargo_wasm_artifact_stem(), "kaskold_companion_web")
        self.assertEqual(module.BINDGEN_STEM, "companion_web")
        source = BUILDER.read_text(encoding="utf-8")
        self.assertIn('f"{cargo_wasm_artifact_stem()}.wasm"', source)
        self.assertNotIn('profile / "companion_web.wasm"', source)

    def test_web_build_uses_direct_wasm_bindgen_not_wasm_pack(self) -> None:
        source = BUILDER.read_text(encoding="utf-8")
        manifest = (ROOT / "apps/kaskold-companion-web/Cargo.toml").read_text(encoding="utf-8")
        self.assertNotIn("wasm-pack", source)
        self.assertNotIn("package.metadata.wasm-pack", manifest)
        self.assertIn('"--target", "web"', source)
        self.assertIn('"--out-name", BINDGEN_STEM', source)
        self.assertIn('BINDGEN_STEM = "companion_web"', source)

    def test_web_build_resolves_lock_for_wasm_target_with_msrv_fallback(self) -> None:
        source = (ROOT / "tools/build/web/build_companion_runtime.py").read_text()
        self.assertIn('"--filter-platform"', source)
        self.assertIn('WASM_TARGET', source)
        self.assertIn('CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS', source)
        self.assertIn('"fallback"', source)
        self.assertIn("ensure_cargo_toolchain(toolchain, env)", source)
        self.assertIn('cargo_args(toolchain, "--version")', source)

    def test_web_build_verifies_companion_lock_with_pinned_cargo(self) -> None:
        source = BUILDER.read_text(encoding="utf-8")
        self.assertIn("ensure_lock_current", source)
        self.assertIn('"metadata"', source)
        self.assertIn('"--locked"', source)
        self.assertIn("reconciling transactionally", source)
        self.assertIn('"--offline"', source)
        self.assertIn("lock.write_bytes(original)", source)

    def test_web_build_pins_wasm_bindgen_cli_and_host_rust(self) -> None:
        source = BUILDER.read_text(encoding="utf-8")
        pins = (ROOT / "qa/config/toolchains.env").read_text(encoding="utf-8")
        self.assertIn("KASKOLD_WASM_BINDGEN_CLI_VERSION=0.2.120", pins)
        self.assertIn("ensure_wasm_bindgen", source)
        self.assertIn('env["RUSTUP_TOOLCHAIN"] = toolchain', source)
        self.assertIn("wasm-bindgen-cli", source)
        self.assertIn('"--root"', source)
        self.assertIn("XDG_CACHE_HOME", source)

    def test_wasm_bindgen_cli_pin_matches_companion_lock(self) -> None:
        lock = tomllib.loads((ROOT / "apps/kaskold-companion-web/Cargo.lock").read_text(encoding="utf-8"))
        versions = {package["version"] for package in lock["package"] if package["name"] == "wasm-bindgen"}
        self.assertEqual(versions, {"0.2.120"})
        source = BUILDER.read_text(encoding="utf-8")
        self.assertIn("locked_wasm_bindgen_version", source)
        self.assertIn("wasm-bindgen crate/CLI mismatch", source)

    def test_rust_195_uses_rustc_194_compatible_wasm_bindgen_family(self) -> None:
        pins = (ROOT / "qa/config/toolchains.env").read_text(encoding="utf-8")
        self.assertIn("KASKOLD_STABLE_RUST=1.95.0", pins)
        self.assertIn("KASKOLD_WASM_BINDGEN_CLI_VERSION=0.2.120", pins)
        # Applications pin the exact family their lockfile and CLI use; the
        # reusable libraries declare the minimum compatible version so hosts
        # with an older wasm-bindgen family can import them.
        manifests = (
            ("apps/kaskold-companion-web/Cargo.toml", "dependencies", "=0.2.120"),
            ("crates/online-watcher/Cargo.toml", "dependencies", "=0.2.120"),
            ("crates/kaskold-protocol/Cargo.toml", "target.cfg(target_arch = \"wasm32\").dependencies", "0.2.108"),
            ("crates/kaskold-sdk/Cargo.toml", "target.cfg(target_arch = \"wasm32\").dependencies", "0.2.108"),
        )
        for relative, table, expected in manifests:
            manifest = tomllib.loads((ROOT / relative).read_text(encoding="utf-8"))
            current = manifest
            for component in table.split("."):
                current = current[component]
            dependency = current["wasm-bindgen"]
            version = dependency if isinstance(dependency, str) else dependency["version"]
            self.assertEqual(version, expected, relative)
        online = tomllib.loads((ROOT / "crates/online-watcher/Cargo.toml").read_text(encoding="utf-8"))
        self.assertEqual(online["dependencies"]["wasm-bindgen-futures"], "=0.4.70")
        self.assertEqual(online["dependencies"]["js-sys"], "=0.3.97")
        self.assertEqual(online["dependencies"]["web-sys"]["version"], "=0.3.97")

    def test_builder_surfaces_captured_subprocess_diagnostics(self) -> None:
        spec = importlib.util.spec_from_file_location("build_companion_runtime_errors", BUILDER)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        error = subprocess.CalledProcessError(101, ["cargo", "metadata"], output="resolver stdout", stderr="resolver stderr")
        rendered = module.subprocess_error_detail(error)
        self.assertIn("exit code 101", rendered)
        self.assertIn("cargo metadata", rendered)
        self.assertIn("resolver stdout", rendered)
        self.assertIn("resolver stderr", rendered)

    def test_browser_build_strips_firmware_rust_overrides(self) -> None:
        source = BUILDER.read_text(encoding="utf-8")
        for name in ("RUSTC", "RUSTDOC", "CARGO_BUILD_TARGET", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"):
            self.assertIn(f'"{name}"', source)
        self.assertIn('build_env["CARGO_TARGET_DIR"]', source)

    def test_gradle_and_xcode_consume_the_same_builder(self) -> None:
        gradle = (ROOT / "apps/kaskold-companion-android/app/build.gradle.kts").read_text(encoding="utf-8")
        project = (ROOT / "apps/kaskold-companion-ios/KasKold.xcodeproj/project.pbxproj").read_text(encoding="utf-8")
        self.assertIn("tools/build/web/build_companion_runtime.py", gradle)
        self.assertIn("tools/build/web/build_companion_runtime.py", project)
        self.assertNotIn('"bash", "-lc"', gradle)


if __name__ == "__main__":
    unittest.main()
