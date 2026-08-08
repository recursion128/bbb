import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from parity_acceptance import (
    AcceptanceError,
    artifact_binary_provenance,
    artifact_scenario_ids,
    build_report,
    external_artifact,
    parse_matrix,
    validate_report,
)
from bbb_tools.executable_identity import elf_load_image_identity


class ParityAcceptanceTests(unittest.TestCase):
    commit = "1" * 40

    def make_fixture(self, root: Path) -> tuple[Path, Path]:
        repo = root / "repo"
        source_root = root / "sources"
        matrix = repo / "docs/wiki/parity/vanilla-26.1-matrix.md"
        inventory = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        ledger = repo / "docs/unsupported-features.md"
        rust = repo / "crates/example/src/lib.rs"
        tool = repo / "scripts/parity_acceptance.py"
        executable_identity = repo / "scripts/bbb_tools/executable_identity.py"
        gate = repo / ".git/bbb/gate-log.tsv"
        vanilla = source_root / "net/minecraft/client/Example.java"
        for path in (
            matrix,
            inventory,
            ledger,
            rust,
            tool,
            executable_identity,
            gate,
            vanilla,
        ):
            path.parent.mkdir(parents=True, exist_ok=True)
        matrix.write_text(
            """\
# Fixture matrix

| Mechanism ID | 领域 | Vanilla owner/class | BBB owner/module | Required state/projection | 状态 | Focused test/evidence | Route / reopen condition |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `fixture-equivalent` | test | `Example.run` | example | state | `equivalent` | `crates/example/src/lib.rs#decision_matches_vanilla` | Example owner changes时重开 |
| `fixture-not-needed` | test | `Example.unreachable` | none | no effect | `not-needed` | [ledger](../../unsupported-features.md) | [ledger](../../unsupported-features.md) |
""",
            encoding="utf-8",
        )
        ledger.write_text(
            """\
# Feature Gap TODO List

# Todo

# Not needed

## Fixture ruling

Matrix route: `fixture-not-needed`

- Vanilla `Example.unreachable` is unreachable. Reopen when the caller exists.
""",
            encoding="utf-8",
        )
        rust.write_text(
            """\
#[cfg(test)]
mod tests {
    #[test]
    fn decision_matches_vanilla() {
        assert_eq!(2 + 2, 4);
    }
}
""",
            encoding="utf-8",
        )
        tool.write_text("# fixture generator\n", encoding="utf-8")
        executable_identity.write_text("# fixture executable identity\n", encoding="utf-8")
        vanilla.write_text("package net.minecraft.client; class Example {}\n", encoding="utf-8")
        vanilla_hash = hashlib.sha256(vanilla.read_bytes()).hexdigest()
        inventory.write_text(
            json.dumps(
                {
                    "generated_by": "fixture",
                    "mechanisms": {
                        "fixture-equivalent": {
                            "bbb_evidence": [
                                "crates/example/src/lib.rs#decision_matches_vanilla"
                            ],
                            "owner_count": 1,
                            "owner_evidence_count": 1,
                        }
                    },
                    "owners": [
                        {
                            "bbb_evidence": {
                                "implementation": ["crates/example/src/lib.rs"],
                                "tests": [
                                    "crates/example/src/lib.rs#decision_matches_vanilla"
                                ],
                            },
                            "category": "fixture",
                            "domain": "fixture",
                            "kind": "method",
                            "mechanism_id": "fixture-equivalent",
                            "owner_id": "method:Example#run()",
                            "role": "decision",
                            "scope": "external-client",
                            "source": {
                                "line": 1,
                                "path": "net/minecraft/client/Example.java",
                            },
                            "vanilla_owner": "net.minecraft.client.Example#run()",
                        }
                    ],
                    "schema_version": 1,
                    "source_files": [
                        {
                            "path": "net/minecraft/client/Example.java",
                            "sha256": vanilla_hash,
                        }
                    ],
                    "vanilla_version": "26.1",
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
        gate.write_text(
            f"2026-08-07T00:00:00Z\t{self.commit}\tclean\ttotal:1s,test:1s\n",
            encoding="utf-8",
        )
        return repo, source_root

    def build_fixture_report(self, repo: Path, source_root: Path):
        return build_report(
            repo=repo,
            source_root=source_root,
            evidence_commit=self.commit,
            gate_log=repo / ".git/bbb/gate-log.tsv",
        )

    def test_report_derives_matrix_rows_and_reverse_evidence_mappings(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            self.assertEqual(
                [row["mechanism_id"] for row in report["mechanisms"]],
                ["fixture-equivalent", "fixture-not-needed"],
            )
            equivalent = report["mechanisms"][0]
            self.assertTrue(equivalent["valid"])
            self.assertEqual(equivalent["final_conclusion"], "equivalent")
            self.assertEqual(
                equivalent["focused_tests"][0]["reference"],
                "crates/example/src/lib.rs#decision_matches_vanilla",
            )
            self.assertEqual(report["summary"]["false_equivalent_ids"], [])
            self.assertEqual(
                validate_report(
                    report,
                    repo=repo,
                    source_root=source_root,
                    enforce_current=True,
                ),
                [],
            )
            artifact = next(
                item
                for item in report["artifacts"]
                if item["artifact_id"] == "repo:crates/example/src/lib.rs"
            )
            self.assertEqual(artifact["mechanism_ids"], ["fixture-equivalent"])
            scenario = next(
                item
                for item in report["scenarios"]
                if item["scenario_id"] == "focused-fixture-equivalent"
            )
            self.assertEqual(scenario["mechanism_ids"], ["fixture-equivalent"])

    def test_false_equivalent_is_machine_detectable(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            (repo / ".git/bbb/gate-log.tsv").write_text("", encoding="utf-8")
            report = self.build_fixture_report(repo, source_root)
            self.assertEqual(
                report["summary"]["false_equivalent_ids"], ["fixture-equivalent"]
            )
            issues = validate_report(
                report, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn("false equivalent: fixture-equivalent", issues)

    def test_false_equivalent_cannot_be_hidden_by_editing_valid_flag(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            (repo / ".git/bbb/gate-log.tsv").write_text("", encoding="utf-8")
            report = self.build_fixture_report(repo, source_root)
            equivalent = report["mechanisms"][0]
            equivalent["valid"] = True
            equivalent["validity_reasons"] = []
            equivalent["final_conclusion"] = "equivalent"
            issues = validate_report(
                report, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn("false equivalent: fixture-equivalent", issues)

    def test_gate_receipt_hash_survives_append_only_log_growth(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            artifact = next(
                item
                for item in report["artifacts"]
                if item["artifact_id"] == "full-gate"
            )
            receipt = artifact["receipt"]
            self.assertEqual(artifact["digest_kind"], "gate-receipt-line-v1")
            self.assertEqual(
                artifact["sha256"],
                hashlib.sha256(f"{receipt}\n".encode("utf-8")).hexdigest(),
            )

            gate_log = repo / ".git/bbb/gate-log.tsv"
            with gate_log.open("a", encoding="utf-8") as handle:
                handle.write(
                    f"2026-08-07T00:00:01Z\t{'2' * 40}\tclean\ttotal:2s,test:2s\n"
                )
                handle.write(
                    f"2026-08-07T00:00:02Z\t{self.commit}\tclean\ttotal:3s,test:3s\n"
                )

            issues = validate_report(
                report,
                repo=repo,
                source_root=source_root,
                enforce_current=True,
                require_artifacts=True,
            )
            self.assertFalse(any("full-gate" in issue for issue in issues), issues)

    def test_gate_receipt_hash_rejects_removed_or_tampered_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            gate_log = repo / ".git/bbb/gate-log.tsv"
            gate_log.write_text(
                f"2026-08-07T00:00:01Z\t{'2' * 40}\tclean\ttotal:2s,test:2s\n",
                encoding="utf-8",
            )
            issues = validate_report(
                report,
                repo=repo,
                source_root=source_root,
                enforce_current=True,
                require_artifacts=True,
            )
            self.assertIn("external gate receipt is absent: full-gate", issues)

            gate_log.write_text(
                f"2026-08-07T00:00:00Z\t{self.commit}\tclean\ttotal:1s,test:1s\n",
                encoding="utf-8",
            )
            tampered = copy.deepcopy(report)
            artifact = next(
                item
                for item in tampered["artifacts"]
                if item["artifact_id"] == "full-gate"
            )
            artifact["sha256"] = "0" * 64
            issues = validate_report(
                tampered,
                repo=repo,
                source_root=source_root,
                enforce_current=True,
                require_artifacts=True,
            )
            self.assertIn("external gate receipt hash is stale: full-gate", issues)

    def test_owner_mapping_and_unrouted_status_are_rederived(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            report["mechanisms"][0]["vanilla"]["inventory_owner_ids"] = []
            issues = validate_report(
                report, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn(
                "stale Vanilla owner mapping for fixture-equivalent", issues
            )

            ledger = repo / "docs/unsupported-features.md"
            ledger.write_text("# Feature Gap TODO List\n", encoding="utf-8")
            report = self.build_fixture_report(repo, source_root)
            issues = validate_report(
                report, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn("unrouted matrix status for fixture-not-needed", issues)

    def test_visual_artifacts_expose_named_replay_scenarios(self):
        value = {
            "screenshot_pairs": [
                {"capture_segment": "static_orbit"},
                {"capture_segment": "preloaded_flight"},
                {"capture_segment": "chunk_boundary_crossing"},
                {"capture_segment": "static_orbit"},
            ]
        }
        self.assertEqual(
            artifact_scenario_ids("workload", value),
            [
                "chunk_boundary_crossing",
                "preloaded_flight",
                "static_orbit",
            ],
        )

    def test_external_visual_receipt_requires_ancestor_byte_identical_binary(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repo = root / "repo"
            repo.mkdir()
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
            tracked.write_text("capture\n", encoding="utf-8")
            subprocess.run(["git", "add", "tracked"], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "capture"], cwd=repo, check=True)
            capture_commit = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            tracked.write_text("evidence\n", encoding="utf-8")
            subprocess.run(["git", "commit", "-qam", "evidence"], cwd=repo, check=True)
            evidence_commit = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()

            capture_binary = root / "bbb-capture"
            evidence_binary = root / "bbb-evidence"
            capture_binary.write_bytes(b"same-binary")
            evidence_binary.write_bytes(b"same-binary")
            digest = hashlib.sha256(b"same-binary").hexdigest()
            summary = root / "workload.json"
            summary.write_text(
                json.dumps(
                    {
                        "status": "matched",
                        "bbb_commit": evidence_commit,
                        "bbb_provenance": {
                            "capture_commit": capture_commit,
                            "evidence_commit": evidence_commit,
                            "binary_sha256": digest,
                            "binary_identical": True,
                            "capture_binary": {
                                "path": str(capture_binary),
                                "sha256": digest,
                            },
                            "evidence_binary": {
                                "path": str(evidence_binary),
                                "sha256": digest,
                            },
                        },
                    }
                ),
                encoding="utf-8",
            )
            artifact = external_artifact(
                repo=repo,
                kind="workload",
                artifact_id="visual-workload",
                path=summary,
                evidence_commit=evidence_commit,
            )
            self.assertTrue(artifact["valid"])
            self.assertEqual(
                artifact["bbb_provenance"]["capture_commit"], capture_commit
            )

            evidence_binary.write_bytes(b"changed")
            stale = external_artifact(
                repo=repo,
                kind="workload",
                artifact_id="visual-workload",
                path=summary,
                evidence_commit=evidence_commit,
            )
            self.assertFalse(stale["valid"])
            self.assertIn(
                "BBB evidence binary SHA-256 is stale", stale["invalid_reasons"]
            )

    def test_external_visual_receipt_accepts_ancestor_elf_load_image(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repo = root / "repo"
            repo.mkdir()
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
            tracked.write_text("capture\n", encoding="utf-8")
            subprocess.run(["git", "add", "tracked"], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "capture"], cwd=repo, check=True)
            capture_commit = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            tracked.write_text("evidence\n", encoding="utf-8")
            subprocess.run(["git", "commit", "-qam", "evidence"], cwd=repo, check=True)
            evidence_commit = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            capture_binary = root / "bbb-capture"
            evidence_binary = root / "bbb-evidence"
            capture_binary.write_bytes(Path(sys.executable).read_bytes())
            evidence_binary.write_bytes(
                capture_binary.read_bytes() + b"non-loaded-linker-metadata"
            )
            capture_sha256 = hashlib.sha256(capture_binary.read_bytes()).hexdigest()
            evidence_sha256 = hashlib.sha256(evidence_binary.read_bytes()).hexdigest()
            runtime_sha256 = elf_load_image_identity(capture_binary)["sha256"]
            provenance = {
                "capture_commit": capture_commit,
                "evidence_commit": evidence_commit,
                "binary_sha256": capture_sha256,
                "binary_identical": False,
                "runtime_image_identical": True,
                "identity_kind": "elf-load-image-v1",
                "runtime_image_sha256": runtime_sha256,
                "capture_binary": {
                    "path": str(capture_binary),
                    "sha256": capture_sha256,
                },
                "evidence_binary": {
                    "path": str(evidence_binary),
                    "sha256": evidence_sha256,
                },
            }

            normalized, reasons = artifact_binary_provenance(
                repo, {"bbb_provenance": provenance}
            )

            self.assertEqual(reasons, [])
            self.assertIsNotNone(normalized)
            self.assertTrue(normalized["runtime_image_identical"])
            self.assertEqual(normalized["runtime_image_sha256"], runtime_sha256)

    def test_unknown_or_one_way_artifact_mapping_fails_lint(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            broken = copy.deepcopy(report)
            broken["mechanisms"][0]["artifact_ids"].append("unknown")
            issues = validate_report(
                broken, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn(
                "unknown artifact 'unknown' for fixture-equivalent", issues
            )
            broken = copy.deepcopy(report)
            artifact = next(
                item
                for item in broken["artifacts"]
                if item["artifact_id"] == "repo:crates/example/src/lib.rs"
            )
            artifact["mechanism_ids"] = []
            issues = validate_report(
                broken, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn(
                "stale reverse Mechanism-ID mapping for artifact repo:crates/example/src/lib.rs",
                issues,
            )

    def test_stale_vanilla_owner_hash_and_generator_input_fail_lint(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            (source_root / "net/minecraft/client/Example.java").write_text(
                "class Changed {}\n", encoding="utf-8"
            )
            issues = validate_report(
                report, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn(
                "stale Vanilla source hash: net/minecraft/client/Example.java",
                issues,
            )
            (repo / "scripts/parity_acceptance.py").write_text(
                "# changed generator\n", encoding="utf-8"
            )
            issues = validate_report(
                report, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn(
                "stale generator input: scripts/parity_acceptance.py", issues
            )

    def test_strict_lint_requires_external_artifacts_to_remain_available(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            report["summary"]["declared_scope_complete"] = False
            issues = validate_report(
                report,
                repo=repo,
                source_root=source_root,
                enforce_current=True,
                require_artifacts=True,
                require_complete=True,
            )
            self.assertIn("external artifact path is absent: performance-abba", issues)
            self.assertIn("declared-scope parity acceptance is incomplete", issues)

    def test_open_matrix_status_cannot_publish_closed_conclusion(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            matrix = repo / "docs/wiki/parity/vanilla-26.1-matrix.md"
            matrix.write_text(
                matrix.read_text(encoding="utf-8").replace(
                    "`fixture-equivalent` | test | `Example.run` | example | state | `equivalent`",
                    "`fixture-equivalent` | test | `Example.run` | example | state | `partial`",
                ),
                encoding="utf-8",
            )
            ledger = repo / "docs/unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8")
                + "\n## Open fixture\n\nMatrix route: `fixture-equivalent`\n\n- Close the fixture.\n",
                encoding="utf-8",
            )
            report = self.build_fixture_report(repo, source_root)
            self.assertEqual(report["mechanisms"][0]["final_conclusion"], "open")
            broken = copy.deepcopy(report)
            broken["mechanisms"][0]["final_conclusion"] = "equivalent"
            issues = validate_report(
                broken, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn(
                "open mechanism has a closed conclusion: fixture-equivalent", issues
            )

    def test_duplicate_matrix_id_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "matrix.md"
            row = (
                "| `duplicate` | test | owner | bbb | state | `equivalent` | "
                "evidence | reopen |\n"
            )
            path.write_text(row + row, encoding="utf-8")
            with self.assertRaisesRegex(AcceptanceError, "duplicate matrix mechanism"):
                parse_matrix(path)


if __name__ == "__main__":
    unittest.main()
