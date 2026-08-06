from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402
VANILLA_ROOT = VanillaWorkspace.resolve("26.1").root


class EntityCatalogTests(unittest.TestCase):
    def test_generated_entity_catalog_matches_vanilla_sources(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(REPO_ROOT / "scripts/generate-entity-metadata-schema.py"),
                "check",
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


if __name__ == "__main__":
    unittest.main()
