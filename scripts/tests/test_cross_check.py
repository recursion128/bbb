import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FULL_GATE_DETAIL = (
    "mode:full,total:1s,vanilla-verify:1s,host-doctor:1s,docs-lint:1s,fmt:1s,"
    "scripts-lint:1s,scripts-tests:1s,diff-check:1s,check:1s,cross-check:1s,test:1s,doctest:1s"
)


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
        self.assertIn("bbb-client", members)
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
            " --> crates/bbb-client/src/platform_macos.rs:33:9\n"
        )
        self.assertIsNone(cross_check.classify_failure(output))

    def test_an_unknown_build_script_failure_is_not_attributed(self) -> None:
        output = "error: failed to run custom build command for `some-other-sys v0.1.0`"
        self.assertIsNone(cross_check.classify_failure(output))

    def test_obsolete_apple_crates_are_absent_from_blocker_table(self) -> None:
        self.assertEqual(set(cross_check.APPLE_TOOLCHAIN_BUILD_SCRIPTS), {"ring"})

    def test_the_first_known_blocker_in_a_multi_failure_run_is_reported(self) -> None:
        output = (
            "error: failed to run custom build command for `ring v0.17.14`\n"
            "error: failed to run custom build command for `other-sys v0.1.2`\n"
        )
        self.assertEqual(cross_check.classify_failure(output)[0], "ring")


class CrossCheckCliTests(unittest.TestCase):
    def run_cross_check(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(REPO_ROOT / "scripts" / "cross_check.py"), *args],
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

        json_result = self.run_cross_check("--json", "--target", "not-a-real-triple")
        self.assertEqual(json_result.returncode, 0)
        import json

        payload = json.loads(json_result.stdout)
        self.assertTrue(payload["skipped"])
        self.assertIsNone(payload["target"])
        self.assertTrue(payload["host"])

    def test_workspace_fast_path_checks_all_unblocked_crates_in_one_call(self) -> None:
        from unittest.mock import patch

        fake_metadata = {
            "workspace_members": ["id-pure", "id-tls"],
            "packages": [
                {"id": "id-pure", "name": "bbb-pure"},
                {"id": "id-tls", "name": "bbb-tls"},
                {"id": "id-ring", "name": "ring"},
            ],
            "resolve": {
                "nodes": [
                    {
                        "id": "id-pure",
                        "deps": [
                            {"pkg": "id-ring", "dep_kinds": [{"kind": "dev"}]}
                        ],
                    },
                    {
                        "id": "id-tls",
                        "deps": [
                            {"pkg": "id-ring", "dep_kinds": [{"kind": None}]}
                        ],
                    },
                    {"id": "id-ring", "deps": []},
                ]
            },
        }
        blocked = cross_check.apple_blocked_from_metadata(
            fake_metadata, ["bbb-pure", "bbb-tls"]
        )
        self.assertEqual(
            blocked,
            {
                "bbb-tls": (
                    "ring",
                    cross_check.APPLE_TOOLCHAIN_BUILD_SCRIPTS["ring"],
                )
            },
        )

        with (
            patch.object(
                cross_check, "resolve_apple_blocked_crates", return_value=blocked
            ),
            patch.object(
                cross_check, "check_workspace_excluding", return_value=(True, "")
            ) as ws_check,
            patch.object(
                cross_check,
                "check_crate",
                side_effect=AssertionError("per-crate fallback must not run on green workspace"),
            ),
        ):
            checked, blocked_out, broken = cross_check.run_partition(
                REPO_ROOT, ["bbb-pure", "bbb-tls"], "aarch64-apple-darwin"
            )
        ws_check.assert_called_once_with(
            REPO_ROOT, "aarch64-apple-darwin", ["bbb-tls"]
        )
        self.assertEqual(checked, ["bbb-pure"])
        self.assertEqual(len(blocked_out), 1)
        self.assertEqual(blocked_out[0][0], "bbb-tls")
        self.assertEqual(broken, [])

    def test_default_target_selection_avoids_macos_host_and_explains_skips(self) -> None:
        target, msg = cross_check.select_cross_target(
            None,
            host="aarch64-apple-darwin",
            installed=["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"],
        )
        self.assertEqual(target, "x86_64-unknown-linux-gnu")
        self.assertEqual(msg, "")

        target_mac_missing, msg_mac_missing = cross_check.select_cross_target(
            None,
            host="aarch64-apple-darwin",
            installed=["aarch64-apple-darwin", "wasm32-unknown-unknown"],
        )
        self.assertIsNone(target_mac_missing)
        self.assertIn("skipped", msg_mac_missing)
        self.assertIn("rustup target add x86_64-unknown-linux-gnu", msg_mac_missing)

        target_win, msg_win = cross_check.select_cross_target(
            None,
            host="x86_64-pc-windows-msvc",
            installed=["x86_64-pc-windows-msvc", "wasm32-unknown-unknown"],
        )
        self.assertIsNone(target_win)
        self.assertIn("skipped", msg_win)
        self.assertIn("rustup target add aarch64-apple-darwin", msg_win)

        target_win_linux, _ = cross_check.select_cross_target(
            None,
            host="x86_64-pc-windows-msvc",
            installed=["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"],
        )
        self.assertEqual(target_win_linux, "x86_64-unknown-linux-gnu")


if __name__ == "__main__":
    unittest.main()
