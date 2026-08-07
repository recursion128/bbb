from __future__ import annotations

import importlib.util
import json
import os
import sys
import tempfile
import unittest
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


if __name__ == "__main__":
    unittest.main()
