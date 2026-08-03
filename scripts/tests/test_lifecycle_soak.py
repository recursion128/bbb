import copy
import importlib.util
import json
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPO_ROOT / "scripts" / "lifecycle_soak.py"
MANIFEST_PATH = (
    REPO_ROOT
    / "scripts"
    / "fixtures"
    / "lifecycle-soak"
    / "manifest-26.1.json"
)
SPEC = importlib.util.spec_from_file_location("lifecycle_soak", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
lifecycle_soak = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(lifecycle_soak)


class LifecycleSoakManifestTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))

    def test_checked_in_manifest_is_complete(self):
        lifecycle_soak.validate_manifest(self.manifest)

    def test_total_budget_must_cover_every_iteration(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["total_timeout_seconds"] = (
            manifest["per_iteration_timeout_seconds"] * manifest["iterations"] - 1
        )
        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "must cover every per-iteration budget"
        ):
            lifecycle_soak.validate_manifest(manifest)

    def test_terminal_quiescence_budget_must_be_positive_and_bounded(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["terminal_quiescence_timeout_seconds"] = 0
        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure,
            "terminal_quiescence_timeout_seconds must be a positive integer",
        ):
            lifecycle_soak.validate_manifest(manifest)

        manifest["terminal_quiescence_timeout_seconds"] = (
            manifest["per_iteration_timeout_seconds"] + 1
        )
        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "must not exceed the per-iteration budget"
        ):
            lifecycle_soak.validate_manifest(manifest)

    def test_both_terminal_modes_are_required(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["termination_modes"] = ["quit"] * manifest["iterations"]
        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "exercise both quit and kick"
        ):
            lifecycle_soak.validate_manifest(manifest)

    def test_fault_schedule_is_strictly_ordered(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["fault_plan"]["actions"][1]["after_millis"] = manifest[
            "fault_plan"
        ]["actions"][0]["after_millis"]
        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "strictly increasing"
        ):
            lifecycle_soak.validate_manifest(manifest)

    def test_every_required_fault_is_present(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["fault_plan"]["actions"] = [
            action
            for action in manifest["fault_plan"]["actions"]
            if action["action"] != "surface_timeout"
        ]
        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "missing fault actions.*surface_timeout"
        ):
            lifecycle_soak.validate_manifest(manifest)

    def test_terminal_gauge_owner_set_is_exact(self):
        manifest = copy.deepcopy(self.manifest)
        del manifest["terminal_gauges"]["renderer.dynamic_texture_cache_pending"]
        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "keys differ from the fixed owner set"
        ):
            lifecycle_soak.validate_manifest(manifest)


class LifecycleSoakEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))

    def evidence(self):
        return {
            "mode": "live",
            "vanilla_version": "26.1",
            "lifecycle": {
                "stage": "complete",
                "resource_pack": {
                    "active_resource_generation": 2,
                    "after_pop_resource_generation": 3,
                },
                "transfer": {"registry_revision_after": {"session": 2}},
            },
        }

    def termination(self, mode="quit"):
        renderer = {
            name: minimum
            for name, minimum in lifecycle_soak.EXPECTED_FAULT_RECEIPTS.items()
        }
        snapshot = {
            "world": {"resource_packs_tracked": 0},
            "renderer": {
                **renderer,
                "terrain_mesh_queue_depth": 0,
                "dynamic_texture_cache_pending": 0,
                "dynamic_texture_worker_result_bytes": 0,
                "pending_particle_spawns": 0,
            },
            "audio": {
                "enabled": False,
                "disabled_reason": "lifecycle-soak injected no audio device",
                "sound_static_channels_used": 0,
                "sound_streaming_channels_used": 0,
            },
            "net": {"simulation_output_backlog_current": 1},
        }
        return {
            "mode": mode,
            "kick_disconnected": mode == "kick",
            "quiescent_snapshot": snapshot,
            "process_metrics": {
                "rss_bytes": 100_000_000,
                "file_descriptors": 30,
                "threads": 10,
            },
            "exit_status": 0,
            "process_absent": True,
        }

    def test_passing_iteration_retains_receipts_gauges_and_generations(self):
        result = lifecycle_soak.validate_iteration(
            self.manifest, 1, self.evidence(), self.termination()
        )

        self.assertEqual(result["termination_mode"], "quit")
        self.assertEqual(result["resource_generations"], {"active": 2, "after_pop": 3})
        self.assertEqual(result["transfer_session"], 2)
        self.assertEqual(result["terminal_gauges"]["world.resource_packs_tracked"], 0)

    def test_missing_fault_receipt_fails(self):
        termination = self.termination()
        termination["quiescent_snapshot"]["renderer"]["surface_timeouts"] = 0

        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "surface_timeouts must be >= 1"
        ):
            lifecycle_soak.validate_iteration(
                self.manifest, 1, self.evidence(), termination
            )

    def test_non_quiescent_gauge_fails(self):
        termination = self.termination()
        termination["quiescent_snapshot"]["renderer"][
            "terrain_mesh_queue_depth"
        ] = 1

        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "terrain_mesh_queue_depth=1 exceeds 0"
        ):
            lifecycle_soak.validate_iteration(
                self.manifest, 1, self.evidence(), termination
            )

    def test_nonzero_or_surviving_process_fails(self):
        termination = self.termination()
        termination["exit_status"] = 9
        with self.assertRaisesRegex(lifecycle_soak.SoakFailure, "exit status is 9"):
            lifecycle_soak.validate_iteration(
                self.manifest, 1, self.evidence(), termination
            )

        termination["exit_status"] = 0
        termination["process_absent"] = False
        with self.assertRaisesRegex(lifecycle_soak.SoakFailure, "survived shutdown"):
            lifecycle_soak.validate_iteration(
                self.manifest, 1, self.evidence(), termination
            )

    def test_kick_iteration_requires_disconnect_receipt(self):
        termination = self.termination("kick")
        termination["kick_disconnected"] = False

        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "kick did not reach disconnected"
        ):
            lifecycle_soak.validate_iteration(
                self.manifest, 2, self.evidence(), termination
            )

    def test_process_trend_fails_only_beyond_checked_in_spread(self):
        iterations = []
        for rss in (1, 10, 10 + 134_217_729):
            iterations.append(
                {
                    "process_metrics": {
                        "rss_bytes": rss,
                        "file_descriptors": 30,
                        "threads": 10,
                    }
                }
            )

        with self.assertRaisesRegex(
            lifecycle_soak.SoakFailure, "rss_bytes grows monotonically"
        ):
            lifecycle_soak.validate_process_trends(self.manifest, iterations)

        iterations[-1]["process_metrics"]["rss_bytes"] = 9
        result = lifecycle_soak.validate_process_trends(self.manifest, iterations)
        self.assertFalse(result["rss_bytes"]["strictly_rising"])


class LifecycleSoakScriptContractTests(unittest.TestCase):
    def test_runner_projects_the_manifest_terminal_budget_without_a_shadow_literal(self):
        source = MODULE_PATH.read_text(encoding="utf-8")

        self.assertIn(
            'manifest["terminal_quiescence_timeout_seconds"]', source
        )
        self.assertNotIn('"BBB_PLAYABLE_TERMINATION_TIMEOUT_SECONDS": "60"', source)

    def test_wrapper_builds_feature_binary_and_records_only_passing_report(self):
        source = (
            REPO_ROOT / "scripts" / "lifecycle-soak-gate.sh"
        ).read_text(encoding="utf-8")

        self.assertIn("--features lifecycle-soak", source)
        self.assertIn("scripts/playable-gate.sh", source)
        self.assertIn("BBB_MC_CODE_ROOT", source)
        self.assertIn("sources/26.1/assets/minecraft", source)
        self.assertIn("last-lifecycle-soak-report.json", source)
        self.assertLess(source.index('"$python_runner" python "$helper" run'), source.index("record_gate_stamp"))


if __name__ == "__main__":
    unittest.main()
