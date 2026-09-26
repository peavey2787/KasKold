from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from architecture.core.inventory import repository_inventory  # noqa: E402


class RepositoryInventoryTests(unittest.TestCase):
    def _root(self) -> tuple[tempfile.TemporaryDirectory, Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        (root / "qa/baselines").mkdir(parents=True)
        return temporary, root

    def _baseline(self, root: Path) -> None:
        path = root / repository_inventory.INVENTORY_RELATIVE
        path.touch()
        path.write_text("\n".join(repository_inventory.scan(root)) + "\n")

    def test_detects_missing_and_new_paths(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "scripts").mkdir()
        (root / "scripts/run.sh").write_text("#!/bin/sh\n")
        self._baseline(root)

        (root / "scripts/run.sh").unlink()
        (root / "scripts").rmdir()
        (root / ".github").mkdir()
        errors = repository_inventory.check(root)

        self.assertTrue(any("missing tracked directory scripts" in error for error in errors))
        self.assertTrue(any("missing tracked file scripts/run.sh" in error for error in errors))
        self.assertTrue(any("untracked directory .github" in error for error in errors))

    def test_reconcile_can_remove_missing_and_add_new(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "old.txt").write_text("old\n")
        self._baseline(root)
        (root / "old.txt").unlink()
        (root / "new.txt").write_text("new\n")

        decisions = []
        def decide(kind: str, description: str) -> int:
            decisions.append((kind, description))
            return 0

        self.assertEqual(repository_inventory.reconcile(root, decide), [])
        self.assertEqual(repository_inventory.check(root), [])
        self.assertIn(("missing", "file old.txt"), decisions)
        self.assertIn(("new", "file new.txt"), decisions)

    def test_reconcile_ignore_is_only_for_current_run(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "tracked.txt").write_text("tracked\n")
        self._baseline(root)
        (root / "tracked.txt").unlink()

        self.assertEqual(repository_inventory.reconcile(root, lambda _kind, _description: 1), [])
        errors = repository_inventory.check(root)
        self.assertTrue(any("missing tracked file tracked.txt" in error for error in errors))


    def test_reconcile_handles_whole_directory_with_one_decision(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "scripts/sub").mkdir(parents=True)
        (root / "scripts/sub/run.sh").write_text("#!/bin/sh\n")
        self._baseline(root)
        (root / "scripts/sub/run.sh").unlink()
        (root / "scripts/sub").rmdir()
        (root / "scripts").rmdir()

        decisions = []
        def decide(kind: str, description: str) -> int:
            decisions.append((kind, description))
            return 0

        self.assertEqual(repository_inventory.reconcile(root, decide), [])
        self.assertEqual(decisions, [("missing", "directory scripts")])
        self.assertEqual(repository_inventory.check(root), [])

    def test_reconcile_adds_whole_new_directory_with_one_decision(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        self._baseline(root)
        (root / ".github/workflows").mkdir(parents=True)
        (root / ".github/workflows/quality.yml").write_text("name: quality\n")

        decisions = []
        def decide(kind: str, description: str) -> int:
            decisions.append((kind, description))
            return 0

        self.assertEqual(repository_inventory.reconcile(root, decide), [])
        self.assertEqual(decisions, [("new", "directory .github")])
        self.assertEqual(repository_inventory.check(root), [])

    def test_android_studio_ide_state_is_ignored_before_reconciliation(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "apps/kaskold-companion-android").mkdir(parents=True)
        self._baseline(root)
        metadata = root / "apps/kaskold-companion-android/.idea/workspace.xml"
        metadata.parent.mkdir(parents=True)
        metadata.write_text("<project/>\n")
        kotlin_state = root / "apps/kaskold-companion-android/.kotlin/sessions/session.bin"
        kotlin_state.parent.mkdir(parents=True)
        kotlin_state.write_bytes(b"generated\n")

        decisions = []
        errors = repository_inventory.reconcile(
            root,
            lambda kind, description: decisions.append((kind, description)) or 0,
        )

        self.assertEqual(decisions, [])
        self.assertEqual(errors, [])
        self.assertEqual(repository_inventory.check(root), [])

    def test_android_studio_machine_local_state_is_ignored_but_other_ide_state_is_rejected(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "apps/kaskold-companion-android/.idea").mkdir(parents=True)
        (root / "apps/kaskold-companion-android/.kotlin/sessions").mkdir(parents=True)
        (root / "apps/kaskold-companion-android/.kotlin/sessions/session.bin").write_bytes(b"generated\n")
        (root / "apps/kaskold-companion-android/local.properties").write_text(
            "sdk.dir=/home/example/Android/Sdk\n"
        )
        (root / "apps/kaskold-companion-android/kaskold.iml").write_text("<module/>\n")
        (root / "apps/other/.idea").mkdir(parents=True)
        self._baseline(root)

        inventory = "\n".join(repository_inventory.scan(root))
        self.assertNotIn("apps/kaskold-companion-android/local.properties", inventory)
        self.assertNotIn("apps/kaskold-companion-android/.kotlin", inventory)
        errors = repository_inventory.check(root)
        self.assertEqual(
            errors,
            [
                "forbidden local-development state apps/other/.idea; remove it before running QA",
            ],
        )

    def test_local_development_state_is_git_ignored(self) -> None:
        ignore = (ROOT / ".gitignore").read_text().splitlines()
        for entry in (
            "**/.idea/",
            "**/.vscode/",
            "**/*.iml",
            "**/*.ipr",
            "**/*.iws",
            "/apps/kaskold-companion-android/local.properties",
            "/apps/kaskold-companion-android/.kotlin/",
        ):
            self.assertIn(entry, ignore)

    def test_generated_trees_are_not_snapshotted(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        for relative in (
            "target/build.bin",
            "release/firmware.bin",
            "release.tmp.18047/partial.bin",
            "target/qa/security/result.json",
            "target/qa/fuzz/artifacts/std_pskt_parser/crash-deadbeef",
            "target/qa/fuzz/corpus/qr_frame_roundtrip/123cfe70cf3aee89b4c5c608465f2dd3bdb6f314",
            "qa/fuzz/Cargo.lock",
            "target/kaskold-companion-web/site/pkg/generated.js",
            "nested/node_modules/package/file.js",
        ):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("generated\n")
        (root / "scripts").mkdir()

        inventory = repository_inventory.scan(root)
        joined = "\n".join(inventory)
        self.assertIn("D\tscripts", joined)
        self.assertNotIn("target/", joined)
        self.assertNotIn("release/", joined)
        self.assertNotIn("release.tmp.18047", joined)
        self.assertNotIn("target/qa/security", joined)
        self.assertNotIn("target/qa/fuzz", joined)
        self.assertNotIn("qa/fuzz/Cargo.lock", joined)
        self.assertNotIn("target/kaskold-companion-web", joined)
        self.assertNotIn("node_modules", joined)


    def test_checked_in_baseline_does_not_require_empty_directories(self) -> None:
        entries = [
            line.split("\t", 1)
            for line in (ROOT / repository_inventory.INVENTORY_RELATIVE).read_text().splitlines()
            if line.strip()
        ]
        tracked_leaves = tuple(path for kind, path in entries if kind != "D")
        empty_directories = [
            path
            for kind, path in entries
            if kind == "D" and not any(leaf.startswith(path + "/") for leaf in tracked_leaves)
        ]
        self.assertEqual(
            empty_directories,
            [],
            "repository inventory must not require empty directories because Git does not track them",
        )

    def test_compact_crap_ratchet_is_tracked_but_generated_evidence_is_not(self) -> None:
        inventory = "\n".join(repository_inventory.scan(ROOT))
        self.assertIn("F\tqa/contracts/quality/crap_ratchets.json", inventory)
        self.assertNotIn("qa/baselines/crap", inventory)
        self.assertNotIn("target/qa/crap", inventory)

    def test_release_temp_exclusion_is_narrow(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "release.tmp.12345").mkdir()
        (root / "release.tmp.12345/output.bin").write_bytes(b"generated\n")
        (root / "release.tmp.docs").mkdir()
        inventory = "\n".join(repository_inventory.scan(root))
        self.assertNotIn("release.tmp.12345", inventory)
        self.assertIn("D\trelease.tmp.docs", inventory)

    def test_generated_fuzz_state_is_target_only(self) -> None:
        ignore = (ROOT / ".gitignore").read_text().splitlines()
        self.assertIn("/target/", ignore)
        self.assertIn("/qa/fuzz/Cargo.lock", ignore)
        self.assertNotIn("/qa/fuzz/corpus/", ignore)
        self.assertNotIn("/qa/fuzz/artifacts/", ignore)

    def test_fuzz_artifacts_never_trigger_inventory_reconciliation(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "scripts").mkdir()
        (root / "qa/fuzz/seeds/std_pskt_parser").mkdir(parents=True)
        (root / "qa/fuzz/seeds/std_pskt_parser/authored").write_bytes(b"seed\n")
        self._baseline(root)

        artifact = root / "target/qa/fuzz/artifacts/std_pskt_parser/crash-deadbeef"
        artifact.parent.mkdir(parents=True)
        artifact.write_bytes(b"generated fuzz artifact\n")
        learned = root / "target/qa/fuzz/corpus/std_pskt_parser/123cfe70cf3aee89b4c5c608465f2dd3bdb6f314"
        learned.parent.mkdir(parents=True)
        learned.write_bytes(b"learned corpus\n")
        (root / "qa/fuzz/Cargo.lock").write_text("# generated by cargo fuzz\n")

        decisions = []
        self.assertEqual(
            repository_inventory.reconcile(
                root,
                lambda kind, description: decisions.append((kind, description)) or 0,
            ),
            [],
        )
        self.assertEqual(decisions, [])
        self.assertEqual(repository_inventory.check(root), [])
        self.assertIn("F\tqa/fuzz/seeds/std_pskt_parser/authored", repository_inventory.scan(root))

    def test_generated_local_web_pkg_trees_are_excluded(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        generated = (
            root / "apps/kaskold-companion-web/web/pkg/companion_web.js",
            root / "apps/kaskold-vault-web/web/pkg/vault_web.js",
        )
        for path in generated:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("generated runtime\n")
        inventory = "\n".join(repository_inventory.scan(root))
        self.assertNotIn("apps/kaskold-companion-web/web/pkg", inventory)
        self.assertNotIn("apps/kaskold-vault-web/web/pkg", inventory)


    def test_legacy_generated_source_tree_paths_are_not_exempt(self) -> None:
        temporary, root = self._root()
        self.addCleanup(temporary.cleanup)
        (root / "scripts").mkdir()
        legacy = (
            root / "apps/kaskold-companion-ios/KasKold/Resources/CompanionUI/index.html",
            root / "apps/kaskold-companion-android/app/src/main/assets/web/pkg/companion_web.js",
            root / "qa/fuzz/corpus/parser/learned",
            root / "qa/fuzz/artifacts/parser/crash",
        )
        for path in legacy:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("legacy generated output\n")
        inventory = "\n".join(repository_inventory.scan(root))
        for path in legacy:
            self.assertIn(path.relative_to(root).as_posix(), inventory)


if __name__ == "__main__":
    unittest.main()
