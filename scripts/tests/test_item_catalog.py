from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

import pytest

pytestmark = pytest.mark.vanilla


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

VANILLA_ROOT = VanillaWorkspace.resolve("26.1").root


class ItemCatalogTests(unittest.TestCase):
    def test_generated_item_catalog_matches_vanilla_sources(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(REPO_ROOT / "scripts/generate-item-registry-catalog.py"),
                "--check",
                "--vanilla-root",
                str(VANILLA_ROOT),
            ],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(
            completed.returncode,
            0,
            completed.stderr or completed.stdout,
        )

    def test_generated_item_prototype_components_match_vanilla_runtime(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(REPO_ROOT / "scripts/generate-item-prototype-components.py"),
                "--bootstrap-tags-output",
                str(REPO_ROOT / "crates/bbb-facts/data/builtin_registration_tags_26_1.json"),
                "--check",
            ],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(
            completed.returncode,
            0,
            completed.stderr or completed.stdout,
        )

    def test_protocol_owns_item_prototype_component_facts(self) -> None:
        artifact_name = "item_prototype_components_26_1.json"
        protocol_artifact = REPO_ROOT / "crates/bbb-facts/data" / artifact_name
        pack_artifact = REPO_ROOT / "crates/bbb-client-assets/data" / artifact_name

        self.assertFalse(pack_artifact.exists())
        self.assertEqual(
            sorted(REPO_ROOT.rglob(artifact_name)),
            [protocol_artifact],
        )

        catalog = json.loads(protocol_artifact.read_text(encoding="utf-8"))
        component_values = catalog["component_values"]
        membership_count = sum(
            end - start + 1
            for component in component_values
            for value in component["values"]
            for start, end in value["item_id_ranges"]
        )
        self.assertEqual(len(catalog["item_resource_ids"]), 1506)
        self.assertEqual(len(component_values), 59)
        self.assertEqual(membership_count, 18_981)


if __name__ == "__main__":
    unittest.main()
