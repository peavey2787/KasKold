from pathlib import Path
import hashlib
import json
import io
import shlex
import shutil
import tomllib
import tarfile
import subprocess
import sys
import tempfile
import os
import unittest

if os.name == "posix":
    import pwd
else:
    pwd = None  # type: ignore[assignment]
from unittest import mock

if os.name != "posix":
    raise unittest.SkipTest("Linux reproducible-build runner tests are POSIX-specific")

ROOT = Path(__file__).resolve().parents[3]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.linux.build.reproducible.common import deterministic_file_manifest, verify_file_manifest  # noqa: E402
from scripts.linux.build.reproducible import common, prefetch, toolchains, ubuntu  # noqa: E402

BASH_EXECUTABLE = shutil.which("bash")
DOCKER_ACCESS_LIBRARY = ROOT / "scripts/linux/build/reproducible/docker_access.sh"
# name -> (body, whether the invocation is appended to the command log)
FAKE_DOCKER_COMMANDS = {
    "docker": (
        'if [ -e "$REPRO_TEST_STATE/docker-up" ]; then exit 0; fi\n'
        'case "$REPRO_DOCKER_FAILURE" in\n'
        '  daemon) echo "Cannot connect to the Docker daemon. Is the docker daemon running?" >&2 ;;\n'
        '  permission) echo "permission denied while trying to connect to the Docker daemon socket" >&2 ;;\n'
        "esac\n"
        "exit 1\n",
        True,
    ),
    "service": (
        'if [ "$1 $2" = "docker start" ]; then : > "$REPRO_TEST_STATE/docker-up"; fi\n',
        True,
    ),
    "systemctl": (
        'if [ "$1 $2" = "start docker" ]; then : > "$REPRO_TEST_STATE/docker-up"; fi\n',
        True,
    ),
    "usermod": ("", True),
    "id": (
        'if [ "$#" -eq 1 ]; then printf "%s\\n" "$REPRO_TEST_ACTIVE_GROUPS"\n'
        'elif [ "$#" -eq 2 ]; then printf "%s\\n" "$REPRO_TEST_CONFIGURED_GROUPS"\n'
        'else printf "uid=1000(alice)\\n"; fi\n',
        False,
    ),
    "sg": ('printf \'%s\\0\' "$@" > "$REPRO_TEST_STATE/sg-argv"\n', True),
}


class _DockerAccessSandbox:
    """Runs docker_access.sh functions against fake docker tooling on a private PATH."""

    def __init__(
        self,
        case: unittest.TestCase,
        *,
        omit: tuple[str, ...] = (),
        docker_failure: str = "",
        active_groups: str = "",
        configured_groups: str = "",
    ) -> None:
        directory = tempfile.TemporaryDirectory()
        case.addCleanup(directory.cleanup)
        root = Path(directory.name)
        self.bin = root / "bin"
        self.state = root / "state"
        self.log = root / "commands.log"
        self.bin.mkdir()
        self.state.mkdir()
        for name, (body, logged) in FAKE_DOCKER_COMMANDS.items():
            if name in omit:
                continue
            prelude = (
                'printf "%s" "${0##*/}" >> "$REPRO_TEST_LOG"\n'
                'for arg in "$@"; do printf " %s" "$arg" >> "$REPRO_TEST_LOG"; done\n'
                'printf "\\n" >> "$REPRO_TEST_LOG"\n'
            ) if logged else ""
            path = self.bin / name
            path.write_text(f"#!{BASH_EXECUTABLE}\n{prelude}{body}")
            path.chmod(0o755)
        self._env = {
            "PATH": str(self.bin),
            "USER": "alice",
            "REPRO_TEST_LOG": str(self.log),
            "REPRO_TEST_STATE": str(self.state),
            "REPRO_DOCKER_FAILURE": docker_failure,
            "REPRO_TEST_ACTIVE_GROUPS": active_groups,
            "REPRO_TEST_CONFIGURED_GROUPS": configured_groups,
        }

    def run(self, body: str, *, extra_env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        script = (
            f"source {shlex.quote(str(DOCKER_ACCESS_LIBRARY))}\n"
            'run_as_root() { shift; "$@"; }\n'
            f"{body}"
        )
        return subprocess.run(
            [BASH_EXECUTABLE, "-c", script],
            env={**self._env, **(extra_env or {})},
            capture_output=True,
            text=True,
            stdin=subprocess.DEVNULL,
            check=False,
        )

    def command_log(self) -> list[str]:
        return self.log.read_text().splitlines() if self.log.exists() else []

    def sg_argv(self) -> list[str] | None:
        path = self.state / "sg-argv"
        if not path.exists():
            return None
        return path.read_bytes().decode().split("\0")[:-1]


@unittest.skipUnless(os.name == "posix", "Linux reproducible-build runner tests are POSIX-specific")
class ReproducibleBuildRunnerTests(unittest.TestCase):
    def test_network_json_retries_transient_dns_failures_then_succeeds(self) -> None:
        class FakeResponse:
            def __enter__(self):
                return self
            def __exit__(self, *_args):
                return False
            def read(self):
                return b'{"ok": true}'

        attempts = [
            common.urllib.error.URLError(OSError(-3, "Temporary failure in name resolution")),
            common.urllib.error.URLError(OSError(-3, "Temporary failure in name resolution")),
            FakeResponse(),
        ]
        with mock.patch.object(common.urllib.request, "urlopen", side_effect=attempts) as opened, \
             mock.patch.object(common.time, "sleep") as slept:
            value = common.download_json("https://example.invalid/release")
        self.assertEqual(value, {"ok": True})
        self.assertEqual(opened.call_count, 3)
        self.assertEqual(slept.call_count, 2)

    def test_network_download_retries_and_does_not_leave_partial_file(self) -> None:
        class FakeResponse:
            def __init__(self):
                self.sent = False
            def __enter__(self):
                return self
            def __exit__(self, *_args):
                return False
            def read(self, _size=-1):
                if self.sent:
                    return b""
                self.sent = True
                return b"payload"

        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "asset"
            attempts = [common.urllib.error.URLError("dns"), FakeResponse()]
            with mock.patch.object(common.urllib.request, "urlopen", side_effect=attempts), \
                 mock.patch.object(common.time, "sleep"):
                common.atomic_download("https://example.invalid/asset", destination)
            self.assertEqual(destination.read_bytes(), b"payload")
            self.assertFalse(destination.with_suffix(".part").exists())


    def test_verified_espup_cache_avoids_github_api_lookup(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            downloads = output / "downloads"
            downloads.mkdir()
            binary = downloads / "espup"
            binary.write_bytes(b"verified espup")
            digest = "sha256:" + hashlib.sha256(binary.read_bytes()).hexdigest()
            (downloads / "espup.sha256").write_text(digest + "\n")
            with mock.patch.object(toolchains, "download_json", side_effect=AssertionError("network should not be used")):
                resolved, resolved_digest = toolchains._download_espup(output, "0.16.0")
            self.assertEqual(resolved, binary)
            self.assertEqual(resolved_digest, digest)


    def test_reproducible_prefetch_is_immune_to_qa_toolchains_module_collision(self) -> None:
        probe = f"""
import sys
from pathlib import Path
root = Path({str(ROOT)!r})
sys.path.insert(0, str(root / 'qa/checks'))
import toolchains as qa_toolchains
assert qa_toolchains.__file__.endswith('/qa/checks/toolchains.py')
sys.path.insert(0, str(root))
from scripts.linux.build.reproducible import prefetch
assert prefetch.prefetch_toolchains.__module__ == 'scripts.linux.build.reproducible.toolchains'
"""
        subprocess.run([sys.executable, "-c", probe], cwd=ROOT, check=True)


    def _assert_reproducible_workspace_manifest(self, relative_path: str) -> None:
        with (ROOT / relative_path).open("rb") as handle:
            manifest = tomllib.load(handle)
        self.assertEqual(manifest["package"]["rust-version"], "1.95")
        self.assertEqual(manifest["workspace"]["resolver"], "2")

    def test_companion_reproducible_workspace_declares_rust_195_and_keeps_independent_resolver2(self) -> None:
        self._assert_reproducible_workspace_manifest("apps/kaskold-companion-web/Cargo.toml")

    def test_vault_web_reproducible_workspace_declares_rust_195_and_keeps_independent_resolver2(self) -> None:
        self._assert_reproducible_workspace_manifest("apps/kaskold-vault-web/Cargo.toml")


    def test_msrv_metadata_checker_reports_only_packages_above_limit(self) -> None:
        library = ROOT / "scripts/linux/lib/cargo_locks.sh"
        metadata = json.dumps({
            "packages": [
                {"name": "old", "version": "1.0.0", "rust_version": "1.70"},
                {"name": "edge", "version": "1.0.0", "rust_version": "1.85"},
                {"name": "new", "version": "2.0.0", "rust_version": "1.88"},
                {"name": "unspecified", "version": "3.0.0", "rust_version": None},
            ]
        })
        shell = f"source {library}; printf '%s' \"$META\" | kaskold_metadata_incompatible_with_rust 1.85"
        result = subprocess.run(
            ["bash", "-c", shell],
            cwd=ROOT,
            env={**os.environ, "META": metadata},
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout.strip(), "new 2.0.0 requires Rust 1.88")

    def test_docker_access_messages_distinguish_daemon_and_permissions(self) -> None:
        library = ROOT / "scripts/linux/build/reproducible/docker_access.sh"
        command = [
            "bash",
            "-c",
            f'''source {library!s};
repro_docker_error_kind "Cannot connect to the Docker daemon. Is the docker daemon running?";
repro_docker_error_kind "permission denied while trying to connect to the Docker daemon socket"''',
        ]
        result = subprocess.run(command, capture_output=True, text=True, check=True)
        self.assertEqual(result.stdout.splitlines(), ["daemon", "permission"])

    def test_docker_start_prefers_service_then_systemctl_and_fails_without_a_manager(self) -> None:
        service = _DockerAccessSandbox(self, omit=("systemctl",))
        result = service.run("repro_start_docker")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(service.command_log(), ["service docker start"])

        systemd = _DockerAccessSandbox(self, omit=("service",))
        result = systemd.run("repro_start_docker")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(systemd.command_log(), ["systemctl start docker"])

        none = _DockerAccessSandbox(self, omit=("service", "systemctl"))
        result = none.run("repro_start_docker")
        self.assertEqual(result.returncode, 2)
        self.assertIn("no supported Docker service manager was found", result.stderr)
        self.assertEqual(none.command_log(), [])

    def test_docker_group_enablement_runs_usermod_and_names_root_equivalence(self) -> None:
        sandbox = _DockerAccessSandbox(self)
        result = sandbox.run("repro_enable_docker_group", extra_env={"USER": "alice"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sandbox.command_log(), ["usermod -aG docker alice"])
        self.assertIn("root-equivalent", result.stdout)
        self.assertIn("continue this same build automatically", result.stdout)

    def test_docker_group_reexec_passes_original_arguments_through_sg(self) -> None:
        sandbox = _DockerAccessSandbox(self)
        script = "/opt/kaskold/reproducible-build.sh"
        result = sandbox.run(f"repro_reexec_with_docker_group {shlex.quote(script)} one 'two words'", extra_env={"USER": "alice"})
        self.assertEqual(result.returncode, 0, result.stderr)
        argv = sandbox.sg_argv()
        self.assertIsNotNone(argv)
        self.assertEqual(argv[:2], ["docker", "-c"])
        self.assertEqual(
            shlex.split(argv[2]),
            ["env", "KASKOLD_DOCKER_GROUP_REEXEC=1", script, "one", "two words"],
        )
        self.assertIn("no newgrp/login step is required", result.stdout)
        self.assertNotIn("Run `newgrp docker`", result.stdout + result.stderr)

    def test_docker_group_reexec_refuses_a_second_hop(self) -> None:
        sandbox = _DockerAccessSandbox(self)
        result = sandbox.run(
            "repro_reexec_with_docker_group /opt/kaskold/reproducible-build.sh",
            extra_env={"USER": "alice", "KASKOLD_DOCKER_GROUP_REEXEC": "1"},
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("still unavailable after activating the docker group", result.stderr)
        self.assertIsNone(sandbox.sg_argv())

    def test_docker_group_reexec_requires_sg(self) -> None:
        sandbox = _DockerAccessSandbox(self, omit=("sg",))
        result = sandbox.run("repro_reexec_with_docker_group /opt/kaskold/reproducible-build.sh")
        self.assertEqual(result.returncode, 2)
        self.assertIn("the `sg` command is unavailable", result.stderr)

    def test_docker_bootstrap_starts_daemon_then_retries_the_same_check(self) -> None:
        sandbox = _DockerAccessSandbox(self, docker_failure="daemon")
        result = sandbox.run("repro_ensure_docker_access", extra_env={"USER": "alice"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(sandbox.command_log(), ["docker info", "service docker start", "docker info"])
        self.assertIn("Starting it automatically", result.stdout)

    def test_docker_bootstrap_adds_absent_group_and_requests_reexec(self) -> None:
        sandbox = _DockerAccessSandbox(self, docker_failure="permission")
        result = sandbox.run("repro_ensure_docker_access", extra_env={"USER": "alice"})
        self.assertEqual(result.returncode, 11)
        self.assertEqual(sandbox.command_log(), ["docker info", "usermod -aG docker alice"])
        self.assertIn("continue this same build automatically", result.stdout)
        self.assertNotIn("read -r -p", result.stdout + result.stderr)

    def test_docker_bootstrap_reexecs_when_group_is_configured_but_inactive(self) -> None:
        sandbox = _DockerAccessSandbox(self, docker_failure="permission", configured_groups="alice docker")
        result = sandbox.run("repro_ensure_docker_access", extra_env={"USER": "alice"})
        self.assertEqual(result.returncode, 11)
        self.assertEqual(sandbox.command_log(), ["docker info"])


    def test_ubuntu_prefetch_accepts_indexes_and_resolves_exact_linux_amd64_child(self) -> None:
        child = {
            "schemaVersion": 2,
            "mediaType": ubuntu.DOCKER_MANIFEST,
            "layers": [{"digest": "sha256:" + "c" * 64}],
        }
        child_bytes = json.dumps(child, separators=(",", ":")).encode()
        child_digest = "sha256:" + hashlib.sha256(child_bytes).hexdigest()
        index = {
            "schemaVersion": 2,
            "mediaType": ubuntu.DOCKER_INDEX,
            "manifests": [
                {
                    "mediaType": ubuntu.DOCKER_MANIFEST,
                    "digest": "sha256:" + "a" * 64,
                    "size": 1,
                    "platform": {"os": "linux", "architecture": "arm64"},
                },
                {
                    "mediaType": "application/vnd.in-toto+json",
                    "digest": "sha256:" + "b" * 64,
                    "size": 1,
                    "platform": {"os": "linux", "architecture": "amd64"},
                },
                {
                    "mediaType": ubuntu.DOCKER_MANIFEST,
                    "digest": child_digest,
                    "size": len(child_bytes),
                    "platform": {"os": "linux", "architecture": "amd64"},
                },
            ],
        }
        index_bytes = json.dumps(index, separators=(",", ":")).encode()
        index_digest = "sha256:" + hashlib.sha256(index_bytes).hexdigest()

        def fake_fetch(_token: str, digest: str) -> bytes:
            if digest == index_digest:
                return index_bytes
            if digest == child_digest:
                return child_bytes
            raise AssertionError(f"unexpected manifest fetch: {digest}")

        with mock.patch.object(ubuntu, "_fetch_manifest", side_effect=fake_fetch):
            resolved_bytes, resolved, retained_index, resolved_digest = ubuntu._resolve_image_manifest("token", index_digest)

        self.assertEqual(resolved_bytes, child_bytes)
        self.assertEqual(resolved, child)
        self.assertEqual(retained_index, index_bytes)
        self.assertEqual(resolved_digest, child_digest)
        self.assertIn(ubuntu.OCI_INDEX, ubuntu.ACCEPT_MANIFESTS)
        self.assertIn(ubuntu.DOCKER_INDEX, ubuntu.ACCEPT_MANIFESTS)

    def test_ubuntu_prefetch_records_pinned_index_and_verified_platform_manifest(self) -> None:
        layer_bytes = b"fake-rootfs-layer"
        layer_digest = "sha256:" + hashlib.sha256(layer_bytes).hexdigest()
        child = {
            "schemaVersion": 2,
            "mediaType": ubuntu.DOCKER_MANIFEST,
            "layers": [{"digest": layer_digest}],
        }
        child_bytes = json.dumps(child, separators=(",", ":")).encode()
        child_digest = "sha256:" + hashlib.sha256(child_bytes).hexdigest()
        index_bytes = b'{"schemaVersion":2,"mediaType":"application/vnd.docker.distribution.manifest.list.v2+json","manifests":[]}'
        index_digest = "sha256:" + hashlib.sha256(index_bytes).hexdigest()

        def fake_download(_url: str, destination: Path, headers=None) -> None:
            destination.write_bytes(layer_bytes)

        with tempfile.TemporaryDirectory() as temporary, \
             mock.patch.object(ubuntu, "_registry_token", return_value="token"), \
             mock.patch.object(ubuntu, "_resolve_image_manifest", return_value=(child_bytes, child, index_bytes, child_digest)), \
             mock.patch.object(ubuntu, "atomic_download", side_effect=fake_download):
            output = Path(temporary)
            layer, metadata = ubuntu.prefetch_base(output, index_digest)
            self.assertEqual(layer.read_bytes(), layer_bytes)
            self.assertEqual((output / "ubuntu-index.json").read_bytes(), index_bytes)
            self.assertEqual((output / "ubuntu-manifest.json").read_bytes(), child_bytes)
            self.assertEqual(metadata["source_digest"], index_digest)
            self.assertEqual(metadata["index_digest"], index_digest)
            self.assertEqual(metadata["manifest_digest"], child_digest)
            self.assertEqual(metadata["layer_digest"], layer_digest)
            self.assertEqual(metadata["platform"], "linux/amd64")

    def test_espup_clang_symlink_is_repaired_from_broken_absolute_link_and_relocated(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home = root / "root-home"
            actual = home / ".rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-20.1.1_20250829/esp-clang/lib"
            actual.mkdir(parents=True)
            library = actual / "libclang.so.20.1.1"
            library.write_bytes(b"clang")
            link = home / ".espup/esp-clang"
            link.parent.mkdir(parents=True)
            link.symlink_to("/old/host/path/.rustup/toolchains/esp/missing/esp-clang/lib", target_is_directory=True)

            resolved = toolchains._normalize_esp_clang_symlink(home)

            self.assertEqual(resolved, actual.resolve())
            self.assertTrue(link.is_symlink())
            self.assertFalse(os.path.isabs(os.readlink(link)))
            self.assertEqual(link.resolve(strict=True), actual.resolve())

            destination = root / "context-root-home"
            prefetch._materialize_context_tree(home, destination)
            copied = destination / ".espup/esp-clang/libclang.so.20.1.1"
            self.assertTrue(copied.is_file())
            self.assertFalse((destination / ".espup/esp-clang").is_symlink())
            self.assertEqual(os.stat(library).st_ino, os.stat(copied).st_ino)

    def test_espup_clang_symlink_normalizer_requires_one_unambiguous_library_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary) / "root-home"
            esp = home / ".rustup/toolchains/esp"
            for index in (1, 2):
                directory = esp / f"clang-{index}"
                directory.mkdir(parents=True)
                (directory / f"libclang.so.{index}").write_bytes(b"clang")
            with self.assertRaisesRegex(RuntimeError, "exactly one libclang directory"):
                toolchains._normalize_esp_clang_symlink(home)

    def test_prefetch_context_hardlinks_large_immutable_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            destination = root / "context"
            (source / "nested").mkdir(parents=True)
            payload = source / "nested/toolchain.bin"
            payload.write_bytes(b"large immutable toolchain bytes")
            prefetch._materialize_context_tree(source, destination)
            linked = destination / "nested/toolchain.bin"
            self.assertEqual(linked.read_bytes(), payload.read_bytes())
            self.assertEqual(os.stat(payload).st_ino, os.stat(linked).st_ino)

    def test_prefetch_context_dereferences_symlinked_toolchain_tree_without_copying_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            real = source / "real"
            real.mkdir(parents=True)
            payload = real / "libLLVM.so"
            payload.write_bytes(b"toolchain library")
            (source / "esp-clang").symlink_to(real, target_is_directory=True)
            destination = root / "context"
            prefetch._materialize_context_tree(source, destination)
            copied = destination / "esp-clang/libLLVM.so"
            self.assertTrue(copied.is_file())
            self.assertFalse((destination / "esp-clang").is_symlink())
            self.assertEqual(os.stat(payload).st_ino, os.stat(copied).st_ino)

    def test_prefetch_refuses_disk_hungry_copy_fallback_when_hardlinks_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "input.bin"
            destination = Path(temporary) / "output.bin"
            source.write_bytes(b"immutable")
            with mock.patch.object(prefetch.os, "link", side_effect=OSError("no hardlinks")):
                with self.assertRaisesRegex(RuntimeError, "must support same-filesystem hard links"):
                    prefetch._hardlink_context_file(str(source), str(destination))
            self.assertFalse(destination.exists())


    def test_prefetch_is_noninteractive_and_uses_private_apt_cache_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "prefetch"
            calls: list[list[str]] = []

            def fake_run(argv: list[str], **_: object) -> None:
                calls.append(list(argv))
                if "--download-only" in argv:
                    (output / "debs" / "fixture.deb").write_bytes(b"fixture-deb")

            with mock.patch.object(ubuntu, "run", side_effect=fake_run), \
                mock.patch.object(ubuntu, "require_command"), \
                mock.patch.object(ubuntu, "_extract_member"), \
                mock.patch.object(
                    ubuntu,
                    "_write_local_apt_packages",
                    side_effect=lambda _debs, destination: destination.write_bytes(b"Package: curl\n"),
                ):
                ubuntu.prefetch_debs(output, Path(temporary) / "layer.tar", "20260101T000000Z", ["curl"])

            download = next(argv for argv in calls if "--download-only" in argv)
            install_at = download.index("install")
            self.assertEqual(download[install_at - 1], "-y")
            self.assertEqual(download[install_at + 1:], ["curl"])
            for option in ("Dir::Cache::pkgcache=", "Dir::Cache::srcpkgcache="):
                matches = [argv for argv in download if argv.startswith(option)]
                self.assertEqual(len(matches), 1, option)
                self.assertTrue(matches[0].removeprefix(option).startswith(str(output)), matches[0])
            self.assertTrue(any("--simulate" in argv for argv in calls))

    @unittest.skipUnless(os.name == "posix", "passwd-home semantics are POSIX-specific")
    def test_rustup_init_uses_passwd_home_but_keeps_isolated_rust_homes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            home = Path(temporary) / "root-home"
            env = toolchains._rustup_init_env(home)
            self.assertEqual(env["HOME"], pwd.getpwuid(os.geteuid()).pw_dir)
            self.assertEqual(env["CARGO_HOME"], str(home / ".cargo"))
            self.assertEqual(env["RUSTUP_HOME"], str(home / ".rustup"))
            self.assertEqual(env["RUSTUP_INIT_SKIP_PATH_CHECK"], "yes")


    def _write_esp_rom_sys_fixture(self, root: Path, home: Path, *, source_line: str | None = None) -> tuple[Path, str]:
        version = toolchains.ESP_ROM_SYS_PATCH_VERSION
        crate_root = f"esp-rom-sys-{version}"
        archive_dir = home / ".cargo/registry/cache/index.crates.io-fixture"
        archive_dir.mkdir(parents=True)
        archive = archive_dir / f"{crate_root}.crate"
        source_line = source_line or toolchains.ESP_ROM_SYS_BAD_STRCASECMP
        files = {
            f"{crate_root}/Cargo.toml": (
                '[package]\nname = "esp-rom-sys"\nversion = "0.1.3"\n'
                '[dependencies]\ncfg-if = "1"\n'
            ).encode(),
            f"{crate_root}/src/lib.rs": (
                'unsafe fn fixture(s1_i: *const i8, s2_i: *const i8) {\n'
                f'    {source_line}\n'
                '    let _ = val;\n}\n'
            ).encode(),
            f"{crate_root}/src/syscall/mod.rs": (
                "\n".join(
                    f"let _slot_{index} = core::mem::transmute({toolchains.ESP_ROM_SYS_BAD_FUNCTION_CAST});"
                    for index in range(toolchains.ESP_ROM_SYS_FUNCTION_CAST_OCCURRENCES)
                ) + "\n"
            ).encode(),
        }
        with tarfile.open(archive, "w:gz") as bundle:
            for name, payload in files.items():
                info = tarfile.TarInfo(name)
                info.size = len(payload)
                info.mode = 0o644
                bundle.addfile(info, io.BytesIO(payload))
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        firmware = root / "apps/kaskold-hardware"
        firmware.mkdir(parents=True)
        (firmware / "Cargo.lock").write_text(
            'version = 4\n\n'
            '[[package]]\n'
            'name = "esp-rom-sys"\n'
            'version = "0.1.3"\n'
            'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            f'checksum = "{digest}"\n'
        )
        return archive, digest

    def test_esp_rom_sys_xtensa_patch_is_lock_bound_source_only_and_relocatable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            home = Path(temporary) / "root-home"
            archive, digest = self._write_esp_rom_sys_fixture(root, home)

            provenance = toolchains._install_esp_rom_sys_xtensa_patch(root, home)

            patched = home / ".kaskold-patches/esp-rom-sys-0.1.3/src/lib.rs"
            self.assertTrue(patched.is_file())
            text = patched.read_text()
            self.assertNotIn(toolchains.ESP_ROM_SYS_BAD_STRCASECMP, text)
            self.assertIn(toolchains.ESP_ROM_SYS_FIXED_STRCASECMP, text)
            self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), digest)
            self.assertEqual(provenance["registry_sha256"], digest)
            self.assertEqual(provenance["package"], "esp-rom-sys")
            self.assertEqual(provenance["version"], "0.1.3")
            self.assertEqual(provenance["patch"], "xtensa-c-char-and-function-pointer-compatibility-v2")
            syscall = home / ".kaskold-patches/esp-rom-sys-0.1.3/src/syscall/mod.rs"
            syscall_text = syscall.read_text(encoding="utf-8")
            self.assertNotIn(toolchains.ESP_ROM_SYS_BAD_FUNCTION_CAST, syscall_text)
            self.assertEqual(
                syscall_text.count(toolchains.ESP_ROM_SYS_FIXED_FUNCTION_CAST),
                toolchains.ESP_ROM_SYS_FUNCTION_CAST_OCCURRENCES,
            )
            config = (home / ".cargo/config.toml").read_text()
            self.assertIn('paths = [".kaskold-patches/esp-rom-sys-0.1.3"]', config)
            self.assertNotIn(str(home), config)
            recorded = json.loads((home / ".kaskold-patches/esp-rom-sys-0.1.3/KASKOLD-PATCH.json").read_text())
            self.assertEqual(recorded, provenance)

    def test_esp_rom_sys_xtensa_patch_fails_closed_on_source_drift(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            home = Path(temporary) / "root-home"
            self._write_esp_rom_sys_fixture(root, home, source_line="let val = 0;")
            with self.assertRaisesRegex(RuntimeError, "expected exactly one canonical"):
                toolchains._install_esp_rom_sys_xtensa_patch(root, home)


    def test_esp_build_std_prefetch_executes_online_then_offline_with_same_pinned_build(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            output = Path(directory) / "prefetch"
            home = output / "root-home"
            export_file = home / "esp-env.sh"
            (root / "apps/kaskold-hardware").mkdir(parents=True)
            (root / "apps/kaskold-hardware/Cargo.toml").write_text("[package]\nname='fixture'\nversion='0.0.0'\n")
            (home / ".cargo/bin").mkdir(parents=True)
            export_file.write_text("export FIXTURE_ESP=1\n")
            calls = []

            def capture(command, **kwargs):
                calls.append((command, kwargs))

            with mock.patch.object(toolchains, "run", side_effect=capture):
                toolchains._prefetch_esp_firmware_build_std(root, output, home, export_file)

            self.assertEqual(len(calls), 2)
            online, offline = calls[0][0], calls[1][0]
            self.assertNotIn("--offline", online)
            self.assertIn("--offline", offline)
            for command, kwargs in calls:
                self.assertIn("+esp", command)
                self.assertIn("--locked", command)
                self.assertIn("--release", command)
                self.assertIn("m5stack,production", command)
                self.assertNotIn("--manifest-path", command)
                self.assertEqual(kwargs.get("cwd"), root / "apps/kaskold-hardware")
            self.assertFalse((output / "firmware-build-std-prefetch-target").exists())


    def test_local_apt_packages_index_binds_each_prefetched_deb(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            deb = root / "example_1.2.3_amd64.deb"
            deb.write_bytes(b"deb bytes")
            destination = root / "Packages"
            completed = subprocess.CompletedProcess(
                ["dpkg-deb", "--field", str(deb)],
                0,
                stdout="Package: example\nVersion: 1.2.3\nArchitecture: amd64\n",
                stderr="",
            )
            with mock.patch.object(ubuntu, "require_command"), \
                 mock.patch.object(ubuntu.subprocess, "run", return_value=completed):
                ubuntu._write_local_apt_packages([deb], destination)
            index = destination.read_text()
            self.assertIn("Package: example", index)
            self.assertIn("Filename: debs/example_1.2.3_amd64.deb", index)
            self.assertIn(f"Size: {deb.stat().st_size}", index)
            self.assertIn(f"SHA256: {hashlib.sha256(deb.read_bytes()).hexdigest()}", index)


    def test_post_preflight_context_finalization_repairs_hardlink_manifest_drift(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_home = root / "root-home"
            context = root / "context"
            (source_home / ".cargo").mkdir(parents=True)
            cache = source_home / ".cargo/cache-state"
            cache.write_text("before\n")
            prefetch._materialize_context_tree(source_home, context / "root-home")
            (context / "BUILD-INPUT-MANIFEST.json").write_text("{}\n")
            prefetch.finalize_context_manifests(context)

            # Host Cargo cache bookkeeping mutates the source inode after the
            # hardlinked Docker context was inventoried. This reproduces the
            # Dockerfile.base sha256sum failure from the release runner.
            cache.write_text("after host preflight\n")
            with self.assertRaisesRegex(RuntimeError, "SHA-256 mismatch"):
                verify_file_manifest(context, context / "BUILD-INPUT-SHA256SUMS")

            prefetch.finalize_context_manifests(context)
            verify_file_manifest(context, context / "BUILD-INPUT-SHA256SUMS")
            verify_file_manifest(context / "root-home", context / "TOOLCHAIN-SHA256SUMS")
            self.assertEqual((context / "root-home/.cargo/cache-state").read_text(), "after host preflight\n")

    def test_prefetch_checksum_manifest_detects_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "a").write_text("one\n")
            (root / "sub").mkdir()
            (root / "sub/b").write_text("two\n")
            manifest = root / "SHA256SUMS"
            deterministic_file_manifest(root, manifest, exclude={Path("SHA256SUMS")})
            verify_file_manifest(root, manifest)
            (root / "sub/b").write_text("changed\n")
            with self.assertRaisesRegex(RuntimeError, "SHA-256 mismatch"):
                verify_file_manifest(root, manifest)


@unittest.skipUnless(os.name == "posix", "Firmware hash shell-reader tests are POSIX-specific")
class FirmwareHashConvergenceReaderTests(unittest.TestCase):
    def _run_reader(self, source: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                str(ROOT / "tools/build/firmware/build_with_hash.sh"),
                "--read-generated-hash",
                str(source),
            ],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def _fixture(self, text: str) -> Path:
        handle = tempfile.NamedTemporaryFile("w", suffix=".rs", delete=False)
        self.addCleanup(lambda: Path(handle.name).unlink(missing_ok=True))
        with handle:
            handle.write(text)
        return Path(handle.name)

    @staticmethod
    def _canonical(count: int = 32) -> str:
        values = ", ".join(f"0x{index:02x}" for index in range(count))
        return (
            "pub static EXPECTED_FIRMWARE_HASH: [u8; 32] = [\n"
            f"    {values}\n"
            "];\n"
        )

    def test_repository_generated_hash_decodes_exactly(self) -> None:
        result = self._run_reader(ROOT / "apps/kaskold-hardware/src/firmware_hash.rs")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertRegex(result.stdout.strip(), r"^[0-9a-f]{64}$")

    def test_hash_reader_accepts_only_exact_canonical_array(self) -> None:
        result = self._run_reader(self._fixture(self._canonical()))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout.strip(),
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        )
        short = self._run_reader(self._fixture(self._canonical(31)))
        self.assertNotEqual(short.returncode, 0)
        duplicate = self._run_reader(self._fixture(self._canonical() + self._canonical()))
        self.assertNotEqual(duplicate.returncode, 0)
        drift = self._run_reader(self._fixture(self._canonical().replace("0x1f\n", "0x1f /* drift */\n")))
        self.assertNotEqual(drift.returncode, 0)


@unittest.skipUnless(os.name == "posix", "Firmware tools lock fixture uses POSIX executable shims")
class FirmwareToolsLockReconciliationTests(unittest.TestCase):
    REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"

    def _lock(self, packages: list[tuple[str, str, str]]) -> str:
        blocks = ["version = 4", ""]
        for name, version, checksum in packages:
            blocks.extend([
                "[[package]]",
                f'name = "{name}"',
                f'version = "{version}"',
                f'source = "{self.REGISTRY}"',
                f'checksum = "{checksum}"',
                "",
            ])
        return "\n".join(blocks)

    def _fixture(self, repaired: str) -> tuple[Path, dict[str, str]]:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        workspace = root / "tools"
        bin_dir = root / "bin"
        workspace.mkdir()
        bin_dir.mkdir()
        (workspace / "Cargo.toml").write_text('[package]\nname="fixture"\nversion="1.0.0"\n')
        original = "# stale\n" + self._lock([
            ("kept", "1.0.0", "a" * 64),
            ("unused", "2.0.0", "b" * 64),
        ])
        (workspace / "Cargo.lock").write_text(original)
        candidate = root / "candidate.lock"
        candidate.write_text(repaired)
        cargo = bin_dir / "cargo"
        cargo.write_text(
            "#!/usr/bin/env python3\n"
            "import os, pathlib, shutil, sys\n"
            "lock = pathlib.Path(os.environ['FAKE_LOCK'])\n"
            "if '--locked' in sys.argv:\n"
            "    sys.exit(101 if '# stale' in lock.read_text() else 0)\n"
            "if '--offline' in sys.argv:\n"
            "    shutil.copyfile(os.environ['FAKE_CANDIDATE'], lock)\n"
            "    sys.exit(0)\n"
            "sys.exit(97)\n"
        )
        cargo.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = str(bin_dir) + os.pathsep + env.get("PATH", "")
        env["FAKE_LOCK"] = str(workspace / "Cargo.lock")
        env["FAKE_CANDIDATE"] = str(candidate)
        return workspace, env

    def _run(self, workspace: Path, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(ROOT / "tools/build/firmware/reconcile_tools_lock.py"),
                "--workspace",
                str(workspace),
            ],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def test_offline_reconciliation_may_only_prune_pinned_external_identities(self) -> None:
        repaired = self._lock([("kept", "1.0.0", "a" * 64)])
        workspace, env = self._fixture(repaired)
        result = self._run(workspace, env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((workspace / "Cargo.lock").read_text(), repaired)
        self.assertIn("no new external identities", result.stdout)

    def test_offline_reconciliation_rejects_new_external_identity_and_restores_lock(self) -> None:
        repaired = self._lock([
            ("kept", "1.0.0", "a" * 64),
            ("evil", "9.9.9", "e" * 64),
        ])
        workspace, env = self._fixture(repaired)
        original = (workspace / "Cargo.lock").read_bytes()
        result = self._run(workspace, env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("introduced external package identities", result.stderr)
        self.assertEqual((workspace / "Cargo.lock").read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
