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


if __name__ == "__main__":
    unittest.main()
