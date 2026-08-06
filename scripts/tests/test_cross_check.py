import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
CARGO_DEV = REPO_ROOT / "scripts" / "cargo-dev.sh"


def load_cross_check():
    spec = importlib.util.spec_from_file_location(
        "cross_check", REPO_ROOT / "scripts" / "cross_check.py"
    )
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


cross_check = load_cross_check()


class WorkspaceMemberTests(unittest.TestCase):
    def test_members_are_read_from_the_real_manifest(self) -> None:
        members = cross_check.workspace_members(REPO_ROOT)
        self.assertIn("bbb-native", members)
        self.assertIn(cross_check.REQUIRED_CROSS_CHECKED_CRATE, members)
        for member in members:
            with self.subTest(member=member):
                self.assertTrue((REPO_ROOT / "crates" / member / "Cargo.toml").is_file())

    def test_member_list_matches_the_crate_directories(self) -> None:
        declared = set(cross_check.workspace_members(REPO_ROOT))
        on_disk = {
            path.parent.name
            for path in (REPO_ROOT / "crates").glob("*/Cargo.toml")
        }
        self.assertEqual(declared, on_disk)


class FailureClassificationTests(unittest.TestCase):
    def test_known_apple_toolchain_build_scripts_are_attributed(self) -> None:
        for package, reason in cross_check.APPLE_TOOLCHAIN_BUILD_SCRIPTS.items():
            with self.subTest(package=package):
                output = (
                    "error: failed to run custom build command for "
                    f"`{package} v1.2.3`\n\nCaused by: ...\n"
                )
                self.assertEqual(
                    cross_check.classify_failure(output), (package, reason)
                )

    def test_a_type_error_is_not_mistaken_for_a_toolchain_limit(self) -> None:
        output = (
            "error[E0425]: cannot find value `marker` in this scope\n"
            " --> crates/bbb-platform-macos/src/lib.rs:33:9\n"
        )
        self.assertIsNone(cross_check.classify_failure(output))

    def test_an_unknown_build_script_failure_is_not_attributed(self) -> None:
        output = "error: failed to run custom build command for `some-other-sys v0.1.0`"
        self.assertIsNone(cross_check.classify_failure(output))

    def test_the_first_known_blocker_in_a_multi_failure_run_is_reported(self) -> None:
        output = (
            "error: failed to run custom build command for `ring v0.17.14`\n"
            "error: failed to run custom build command for `objc_exception v0.1.2`\n"
        )
        self.assertEqual(cross_check.classify_failure(output)[0], "ring")


class CrossCheckCliTests(unittest.TestCase):
    def run_cross_check(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["python3", str(REPO_ROOT / "scripts" / "cross_check.py"), *args],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def test_an_uninstalled_target_skips_attributably(self) -> None:
        result = self.run_cross_check("--target", "not-a-real-triple")
        self.assertEqual(result.returncode, 0)
        self.assertIn("rustup target add not-a-real-triple", result.stdout)
        self.assertIn("skipped", result.stdout)


class GateCrossCheckStatusTests(unittest.TestCase):
    def run_git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return result.stdout.strip()

    def make_repo(self, root: Path) -> Path:
        repo = root / "repo"
        (repo / "crates" / "bbb-example" / "src").mkdir(parents=True)
        (repo / "docs").mkdir()
        self.run_git(repo, "init", "-q")
        self.run_git(repo, "config", "user.name", "Cross Check Test")
        self.run_git(repo, "config", "user.email", "cross-check-test@example.invalid")
        (repo / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
        (repo / "crates" / "bbb-example" / "src" / "lib.rs").write_text(
            "// lib\n", encoding="utf-8"
        )
        (repo / "docs" / "note.md").write_text("note\n", encoding="utf-8")
        self.run_git(repo, "add", ".")
        self.run_git(repo, "commit", "-q", "-m", "root commit")
        return repo

    def status(self, repo: Path, *, full: bool = False) -> str:
        env = os.environ.copy()
        env.pop("BBB_GATE_FULL", None)
        if full:
            env["BBB_GATE_FULL"] = "1"
        result = subprocess.run(
            [str(CARGO_DEV), "gate-cross-check-status"],
            cwd=repo,
            env=env,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        return result.stdout.strip()

    def test_a_docs_only_change_skips(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "docs" / "note.md").write_text("changed\n", encoding="utf-8")
            self.assertEqual(
                self.status(repo), "skip: no crate source change (working tree vs HEAD)"
            )

    def test_a_crate_source_change_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "crates" / "bbb-example" / "src" / "lib.rs").write_text(
                "// changed\n", encoding="utf-8"
            )
            self.assertEqual(
                self.status(repo), "run: crate source changed (working tree vs HEAD)"
            )

    def test_a_workspace_manifest_change_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "Cargo.toml").write_text(
                "[workspace]\nmembers = []\n", encoding="utf-8"
            )
            self.assertEqual(
                self.status(repo), "run: crate source changed (working tree vs HEAD)"
            )

    def test_an_untracked_crate_file_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "crates" / "bbb-example" / "src" / "new.rs").write_text(
                "// new\n", encoding="utf-8"
            )
            self.assertEqual(
                self.status(repo), "run: crate source changed (working tree vs HEAD)"
            )

    def test_a_committed_docs_only_slice_skips(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "docs" / "note.md").write_text("changed\n", encoding="utf-8")
            self.run_git(repo, "commit", "-q", "-am", "docs-only slice")
            self.assertEqual(
                self.status(repo), "skip: no crate source change (HEAD vs HEAD^)"
            )

    def test_full_gate_forces_the_step(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "docs" / "note.md").write_text("changed\n", encoding="utf-8")
            self.assertEqual(self.status(repo, full=True), "run: BBB_GATE_FULL=1")


class MacOsSourceOwnershipTests(unittest.TestCase):
    """The extraction only pays off while `bbb-native` keeps no macOS body.

    `bbb-native` cannot be cross-checked, so any AppKit call left inside it is
    code nothing on this host compiles. The cross-checkable crate is the only
    legal home for one.
    """

    def test_the_cross_checkable_crate_owns_the_appkit_calls(self) -> None:
        native = REPO_ROOT / "crates" / "bbb-native" / "src"
        offenders = []
        for path in native.rglob("*.rs"):
            text = path.read_text(encoding="utf-8")
            if "objc2" in text:
                offenders.append(str(path.relative_to(REPO_ROOT)))
        self.assertEqual(offenders, [])

    def test_the_cross_checkable_crate_declares_no_blocking_dependency(self) -> None:
        manifest = (
            REPO_ROOT
            / "crates"
            / cross_check.REQUIRED_CROSS_CHECKED_CRATE
            / "Cargo.toml"
        ).read_text(encoding="utf-8")
        for package in cross_check.APPLE_TOOLCHAIN_BUILD_SCRIPTS:
            with self.subTest(package=package):
                self.assertNotIn(f"\n{package} ", manifest)


if __name__ == "__main__":
    unittest.main()
