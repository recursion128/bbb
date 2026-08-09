from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPO_ROOT / "scripts" / "capture-vanilla-audio-receipt.py"
SPEC = importlib.util.spec_from_file_location("vanilla_audio_receipt", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
receipt = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = receipt
SPEC.loader.exec_module(receipt)


def lifecycle() -> dict[str, object]:
    return {
        "minimum_source_lifetime_ticks": 20,
        "trace": [
            {
                "tick": tick,
                "is_active": tick < 20,
                "channel_retained": tick < 20,
                "delete_time_retained": tick < 20,
            }
            for tick in range(21)
        ],
    }


def observation(requested: bool, *, supported: bool = True) -> dict[str, object]:
    return {
        "schema": 1,
        "minecraft_version": "26.1",
        "requested_hrtf": requested,
        "status": "passed",
        "current_device": "OpenAL Soft on Fixture",
        "mono_sources": 256,
        "channel_debug": "Sounds: 0/248 + 0/8",
        "hrtf_extension": supported,
        "hrtf_specifiers": 1 if supported else 0,
        "hrtf_status": 1 if requested and supported else 0,
        "lifecycle": lifecycle(),
    }


class VanillaAudioReceiptTests(unittest.TestCase):
    def test_formal_receipt_commit_identity_requires_clean_tree(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(
                ["git", "config", "user.email", "fixture@example.invalid"],
                cwd=repo,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Fixture"], cwd=repo, check=True
            )
            tracked = repo / "tracked"
            tracked.write_text("clean\n", encoding="utf-8")
            subprocess.run(["git", "add", "tracked"], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "fixture"], cwd=repo, check=True)
            head = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            self.assertEqual(
                receipt.repo_commit_identity(repo, require_clean=True), head
            )
            tracked.write_text("dirty\n", encoding="utf-8")
            with self.assertRaisesRegex(
                receipt.AudioReceiptFailure, "requires a clean worktree"
            ):
                receipt.repo_commit_identity(repo, require_clean=True)
            self.assertEqual(
                receipt.repo_commit_identity(repo, require_clean=False), head
            )

    def test_validates_owner_pool_hrtf_and_lifecycle_contract(self) -> None:
        contract = receipt.validate_probe_pair(observation(False), observation(True))
        self.assertEqual(
            contract,
            {
                "mono_sources": 256,
                "static_capacity": 248,
                "streaming_capacity": 8,
                "hrtf_supported": True,
                "hrtf_disabled_status": 0,
                "hrtf_enabled_status": 1,
                "minimum_source_lifetime_ticks": 20,
            },
        )

    def test_accepts_vanilla_unsupported_hrtf_fallback(self) -> None:
        contract = receipt.validate_probe_pair(
            observation(False, supported=False), observation(True, supported=False)
        )
        self.assertFalse(contract["hrtf_supported"])
        self.assertEqual(contract["hrtf_enabled_status"], 0)

    def test_rejects_shared_or_wrong_pool_projection(self) -> None:
        enabled = observation(True)
        enabled["channel_debug"] = "Sounds: 0/256 + 0/256"
        with self.assertRaisesRegex(
            receipt.AudioReceiptFailure, "device-derived formula"
        ):
            receipt.validate_probe_pair(observation(False), enabled)

    def test_rejects_early_stopped_handle_removal(self) -> None:
        enabled = observation(True)
        enabled_lifecycle = enabled["lifecycle"]
        assert isinstance(enabled_lifecycle, dict)
        trace = enabled_lifecycle["trace"]
        assert isinstance(trace, list)
        trace[1]["is_active"] = False
        with self.assertRaisesRegex(receipt.AudioReceiptFailure, "tick 1"):
            receipt.validate_probe_pair(observation(False), enabled)

    def test_parses_only_one_json_report_amid_logs(self) -> None:
        value = observation(False)
        output = "[Render thread/INFO] initialized\n" + json.dumps(value) + "\n"
        self.assertEqual(receipt.parse_probe_output(output), value)
        with self.assertRaisesRegex(receipt.AudioReceiptFailure, "got 0"):
            receipt.parse_probe_output("OpenAL failed\n")

    def test_channel_formula_clamps_like_vanilla(self) -> None:
        self.assertEqual(receipt.vanilla_channel_limits(256), (248, 8))
        self.assertEqual(receipt.vanilla_channel_limits(30), (25, 5))
        self.assertEqual(receipt.vanilla_channel_limits(2), (8, 2))
        self.assertEqual(receipt.vanilla_channel_limits(1024), (255, 8))

    def test_host_audio_access_records_exact_node_permissions(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            node = root / "regular-file-is-not-a-device"
            node.write_text("fixture", encoding="utf-8")
            observation = receipt.host_audio_access(root)
            self.assertEqual(observation["uid"], os.getuid())
            self.assertEqual(observation["gid"], os.getgid())
            self.assertTrue(observation["device_root_present"])
            self.assertEqual(observation["device_nodes"], [])
            self.assertEqual(observation["accessible_device_nodes"], 0)

    def test_cargo_output_selects_the_exact_device_feature_test_binary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "bbb_audio-fixture"
            binary.write_text("fixture\n", encoding="utf-8")
            binary.chmod(0o755)
            output = json.dumps(
                {
                    "reason": "compiler-artifact",
                    "manifest_path": str(receipt.BBB_AUDIO_MANIFEST),
                    "target": {"name": "bbb_audio"},
                    "profile": {"test": True},
                    "executable": str(binary),
                }
            )
            self.assertEqual(receipt.parse_bbb_test_binary(output), binary.resolve())
            with self.assertRaisesRegex(receipt.AudioReceiptFailure, "got 0"):
                receipt.parse_bbb_test_binary(
                    json.dumps(
                        {
                            "reason": "compiler-artifact",
                            "manifest_path": str(receipt.BBB_AUDIO_MANIFEST),
                            "target": {"name": "bbb_audio"},
                            "profile": {"test": False},
                            "executable": str(binary),
                        }
                    )
                )

    def test_bbb_build_uses_repo_root_and_device_feature(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "bbb_audio-fixture"
            binary.write_text("fixture\n", encoding="utf-8")
            binary.chmod(0o755)
            output = json.dumps(
                {
                    "reason": "compiler-artifact",
                    "manifest_path": str(receipt.BBB_AUDIO_MANIFEST),
                    "target": {"name": "bbb_audio"},
                    "profile": {"test": True},
                    "executable": str(binary),
                }
            )
            completed = subprocess.CompletedProcess([], 0, output, "")
            with mock.patch.object(
                receipt.subprocess, "run", return_value=completed
            ) as run:
                identity = receipt.build_bbb_test_binary(root)
            argv = run.call_args.args[0]
            self.assertEqual(run.call_args.kwargs["cwd"], receipt.REPO_ROOT)
            self.assertIn("bbb-audio", argv)
            self.assertIn("device", argv)
            self.assertIn("--message-format=json", argv)
            self.assertEqual(identity["path"], str(binary.resolve()))

    def test_bbb_runner_rejects_zero_test_success_and_records_every_owner(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "bbb-audio"
            openal = root / "libopenal.so"
            sound = root / "sound.ogg"
            for path in (binary, openal, sound):
                path.write_bytes(b"fixture")
            successes = []
            for test_id in receipt.BBB_AUDIO_TEST_IDS:
                successes.append(
                    subprocess.CompletedProcess(
                        [],
                        0,
                        stdout=(
                            "running 1 test\n"
                            f"test {test_id} ... ok\n"
                            "test result: ok. 1 passed; 0 failed; 0 ignored\n"
                        ),
                        stderr="",
                    )
                )
            with mock.patch.object(
                receipt.subprocess, "run", side_effect=successes
            ) as run:
                records = receipt.run_bbb_audio_tests(
                    binary=binary,
                    openal_library=openal,
                    sound_asset=sound,
                    driver="alsa",
                    artifact_dir=root,
                )
            self.assertEqual(run.call_count, len(receipt.BBB_AUDIO_TEST_IDS))
            self.assertEqual(
                [record["test_id"] for record in records],
                list(receipt.BBB_AUDIO_TEST_IDS),
            )
            self.assertTrue(all(record["status"] == "passed" for record in records))

            zero = subprocess.CompletedProcess(
                [],
                0,
                stdout="running 0 tests\ntest result: ok. 0 passed; 0 failed\n",
                stderr="",
            )
            zero_dir = root / "zero"
            zero_dir.mkdir()
            with mock.patch.object(receipt.subprocess, "run", return_value=zero):
                failed = receipt.run_bbb_audio_tests(
                    binary=binary,
                    openal_library=openal,
                    sound_asset=sound,
                    driver="default",
                    artifact_dir=zero_dir,
                )
            self.assertTrue(all(record["status"] == "failed" for record in failed))


if __name__ == "__main__":
    unittest.main()
