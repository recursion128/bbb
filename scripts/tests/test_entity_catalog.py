from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
VANILLA_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"


class EntityCatalogTests(unittest.TestCase):
    def test_generated_entity_catalog_matches_vanilla_sources(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(REPO_ROOT / "scripts/generate-entity-metadata-schema.py"),
                "check",
                "--source-root",
                str(VANILLA_SOURCE_ROOT),
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


if __name__ == "__main__":
    unittest.main()
