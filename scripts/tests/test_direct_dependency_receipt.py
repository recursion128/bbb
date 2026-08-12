from __future__ import annotations

import json
from pathlib import Path
import sys
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"
sys.path.insert(0, str(SCRIPTS))

from direct_dependency_receipt import (  # noqa: E402
    ADVISORIES,
    GROUPS,
    build_receipt,
    render_receipt,
)


RECEIPT = (
    REPO_ROOT / "docs/evidence/dependency-receipts/direct-workspace-dependencies.json"
)


class DirectDependencyReceiptTests(unittest.TestCase):
    def test_committed_receipt_matches_manifests_lock_metadata_and_history(self) -> None:
        self.assertEqual(RECEIPT.read_text(encoding="utf-8"), render_receipt(REPO_ROOT))

    def test_every_direct_registry_dependency_has_the_required_audit_fields(self) -> None:
        receipt = build_receipt(REPO_ROOT)
        dependencies = receipt["dependencies"]
        self.assertEqual(receipt["direct_registry_dependency_count"], len(dependencies))
        self.assertEqual(len({entry["name"] for entry in dependencies}), len(dependencies))
        for entry in dependencies:
            self.assertTrue(entry["manifest_constraint"])
            self.assertTrue(entry["locked"]["checksum"])
            self.assertTrue(entry["latest_stable"]["version"])
            self.assertEqual(len(entry["first_introduced_commit"]), 40)
            self.assertEqual(len(entry["latest_constraint_commit"]), 40)
            self.assertTrue(entry["dependency_classes"])
            self.assertTrue(entry["workspace_members"])
            self.assertTrue(entry["platform_impact"])
            self.assertTrue(entry["domain_impact"])
            self.assertTrue(entry["maintenance"]["state"])
            self.assertTrue(entry["migration_risk"]["workspace_msrv"])
            self.assertTrue(entry["modernization"]["decision"])

    def test_risk_routes_cover_the_high_priority_groups_and_known_advisories(self) -> None:
        self.assertEqual(
            set(GROUPS),
            {
                "criterion",
                "glam",
                "hecs",
                "md5",
                "num-bigint",
                "pollster",
                "rand",
                "reqwest",
                "rsa",
                "sha1",
                "sha2",
                "symphonia",
                "winit",
                "zip",
            },
        )
        advisory_ids = {
            advisory["id"]
            for advisories in ADVISORIES.values()
            for advisory in advisories
        }
        self.assertTrue(
            {
                "RUSTSEC-2021-0134",
                "RUSTSEC-2023-0071",
                "RUSTSEC-2026-0186",
                "RUSTSEC-2026-0190",
                "RUSTSEC-2026-0192",
                "RUSTSEC-2026-0194",
                "RUSTSEC-2026-0195",
                "RUSTSEC-2026-0204",
                "crates.io-yanked",
            }.issubset(advisory_ids)
        )


if __name__ == "__main__":
    unittest.main()
