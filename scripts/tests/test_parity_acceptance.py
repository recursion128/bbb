import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from parity_acceptance import (
    AcceptanceError,
    artifact_binary_provenance,
    artifact_scenario_ids,
    build_report,
    capture_contract_identity,
    change_impact_plan,
    external_artifact,
    external_artifact_ids,
    focused_test_references,
    main,
    parse_args,
    parse_matrix,
    performance_prerequisite_reasons,
    validation_classes,
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

    def test_change_plan_command_does_not_require_vanilla_workspace(self):
        with mock.patch.object(sys, "argv", ["parity_acceptance.py", "plan"]):
            with mock.patch("parity_acceptance.resolve_source_root") as resolve:
                args = parse_args()
        resolve.assert_not_called()
        self.assertIsNone(args.source_root)

    def test_final_head_check_cannot_run_before_report_is_sealed(self):
        args = SimpleNamespace(command="generate", require_final_head=True)
        with mock.patch("parity_acceptance.parse_args", return_value=args):
            with mock.patch("sys.stderr"):
                self.assertEqual(main(), 2)

    def test_matrix_validation_classes_and_external_artifacts_cannot_diverge(self):
        rows = parse_matrix(
            REPO_ROOT / "docs/wiki/parity/vanilla-26.1-matrix.md"
        )
        class_to_artifact = {
            "audio-host-device": "audio-host-device",
            "draw-evidence": "draw-evidence",
            "performance-abba": "performance-abba",
            "protocol-state-receipt": "linux-platform",
            "resource-platform-receipt": "linux-platform",
            "visual-workload": "visual-workload",
        }
        for row in rows:
            classes = set(validation_classes(row))
            artifact_ids = set(external_artifact_ids(row))
            for validation_class, artifact_id in class_to_artifact.items():
                if validation_class in classes:
                    self.assertIn(
                        artifact_id,
                        artifact_ids,
                        f"{row['mechanism_id']} lacks {validation_class} artifact",
                    )
            if (
                "rendering" in row["domain"].split("/")
                and row["status"] not in {"not-needed", "deferred-external"}
            ):
                self.assertTrue(
                    {"visual-workload", "draw-evidence"}.issubset(classes),
                    row["mechanism_id"],
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

    def test_partial_reason_is_derived_from_stage_and_cannot_be_erased(self):
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
                + "\n## Open fixture\n\nMatrix route: `fixture-equivalent`\n\n- Await the formal receipt.\n",
                encoding="utf-8",
            )
            report = self.build_fixture_report(repo, source_root)
            mechanism = report["mechanisms"][0]
            self.assertEqual(
                mechanism["open_reasons"], ["formal-receipt-pending"]
            )
            self.assertTrue(report["workflow"]["implementation_ready"])
            self.assertEqual(
                report["workflow"]["implementation_blocking_ids"], []
            )
            broken = copy.deepcopy(report)
            broken["mechanisms"][0]["open_reasons"] = []
            issues = validate_report(
                broken, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn(
                "open mechanism lacks machine-readable reason: fixture-equivalent",
                issues,
            )
            self.assertIn("stale open reasons for fixture-equivalent", issues)

    def test_ignored_test_keeps_focused_evidence_pending(self):
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
                + "\n## Open fixture\n\nMatrix route: `fixture-equivalent`\n\n- Await focused evidence.\n",
                encoding="utf-8",
            )
            test_source = repo / "crates/example/src/lib.rs"
            test_source.write_text(
                test_source.read_text(encoding="utf-8").replace(
                    "    #[test]\n", "    #[test]\n    #[ignore]\n"
                ),
                encoding="utf-8",
            )

            report = self.build_fixture_report(repo, source_root)
            mechanism = report["mechanisms"][0]
            self.assertIn("focused-evidence-pending", mechanism["open_reasons"])
            self.assertFalse(
                any(
                    scenario["scenario_id"] == "focused-fixture-equivalent"
                    for scenario in mechanism["differential_scenarios"]
                )
            )
            self.assertEqual(mechanism["focused_tests"][0]["result"], "ignored")
            self.assertFalse(report["workflow"]["implementation_ready"])
            self.assertFalse(report["workflow"]["code_freeze_ready"])
            self.assertEqual(
                report["workflow"]["implementation_blocking_ids"],
                ["fixture-equivalent"],
            )
            premature = build_report(
                repo=repo,
                source_root=source_root,
                evidence_commit=self.commit,
                gate_log=repo / ".git/bbb/gate-log.tsv",
                workflow_stage="code-freeze",
            )
            self.assertIn(
                "workflow stage advanced before a clean gated code freeze",
                validate_report(
                    premature,
                    repo=repo,
                    source_root=source_root,
                    enforce_current=True,
                ),
            )

    def test_tokio_async_test_is_executable_focused_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            test_source = repo / "crates/example/src/lib.rs"
            test_source.write_text(
                """\
#[cfg(test)]
mod tests {
    #[allow(dead_code)]
    async fn helper_is_not_a_test() {}

    #[tokio::test]
    async fn decision_matches_vanilla() {
        assert_eq!(2 + 2, 4);
    }
}
""",
                encoding="utf-8",
            )

            report = self.build_fixture_report(repo, source_root)
            focused = report["mechanisms"][0]["focused_tests"]
            self.assertEqual(
                [test["reference"] for test in focused],
                ["crates/example/src/lib.rs#decision_matches_vanilla"],
            )
            self.assertEqual(focused[0]["result"], "passed-via-full-gate")

    def test_nested_mechanism_page_tests_are_scoped_to_its_declared_id(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, _source_root = self.make_fixture(Path(temporary))
            mechanism_page = repo / "docs/wiki/mechanisms/fixture.md"
            mechanism_page.parent.mkdir(parents=True, exist_ok=True)
            mechanism_page.write_text(
                """\
# Fixture mechanism

机制 ID：`fixture-equivalent`

`crates/example/src/lib.rs#decision_matches_vanilla`
""",
                encoding="utf-8",
            )
            reference = mechanism_page.relative_to(repo).as_posix()

            matching = focused_test_references(
                repo, [reference], "fixture-equivalent"
            )
            unrelated = focused_test_references(
                repo, [reference], "fixture-unrelated"
            )

            self.assertEqual(
                [test["reference"] for test in matching],
                ["crates/example/src/lib.rs#decision_matches_vanilla"],
            )
            self.assertEqual(unrelated, [])

    def test_change_plan_scopes_direct_central_and_unrelated_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            plan = change_impact_plan(
                report["mechanisms"],
                [
                    "README.md",
                    "crates/example/src/lib.rs",
                    "scripts/parity_acceptance.py",
                ],
                self.commit,
            )
            by_path = {item["path"]: item for item in plan["path_impacts"]}
            self.assertEqual(by_path["README.md"]["mechanism_ids"], [])
            self.assertEqual(
                by_path["crates/example/src/lib.rs"]["mechanism_ids"],
                ["fixture-equivalent"],
            )
            self.assertEqual(
                by_path["scripts/parity_acceptance.py"]["mechanism_ids"],
                ["fixture-equivalent", "fixture-not-needed"],
            )
            self.assertEqual(
                by_path["scripts/parity_acceptance.py"]["reasons"],
                ["central-owner"],
            )

            visual_records = copy.deepcopy(report["mechanisms"])
            visual_records[0]["required_validation_classes"].append(
                "visual-workload"
            )
            fixture_plan = change_impact_plan(
                visual_records,
                ["scripts/fixtures/official-client-benchmark/replay-26.1.json"],
                self.commit,
            )
            self.assertEqual(
                fixture_plan["affected_mechanism_ids"], ["fixture-equivalent"]
            )
            central_owner_records = copy.deepcopy(report["mechanisms"])
            central_owner_records[0]["required_validation_classes"].extend(
                ["protocol-state-receipt", "visual-workload", "release-identity"]
            )
            central_owner_plan = change_impact_plan(
                central_owner_records,
                ["crates/bbb-world/src/client.rs"],
                self.commit,
            )
            self.assertEqual(
                central_owner_plan["affected_mechanism_ids"],
                ["fixture-equivalent"],
            )
            self.assertEqual(
                central_owner_plan["required_validation_classes"],
                ["protocol-state-receipt", "release-identity", "visual-workload"],
            )

            test_only_records = copy.deepcopy(report["mechanisms"])
            test_only_records[0]["required_validation_classes"] = [
                "focused-evidence",
                "full-gate",
                "visual-workload",
                "performance-abba",
            ]
            test_only_records[0]["bbb"]["dependency_closure"] = [
                {
                    "path": "scripts/tests/test_fixture.py",
                    "roles": ["focused-test"],
                    "sha256": "0" * 64,
                }
            ]
            test_only_plan = change_impact_plan(
                test_only_records,
                ["scripts/tests/test_fixture.py"],
                self.commit,
            )
            self.assertEqual(
                test_only_plan["required_validation_classes"],
                ["focused-evidence", "full-gate"],
            )

            metadata_plan = change_impact_plan(
                report["mechanisms"],
                ["docs/wiki/parity/vanilla-26.1-matrix.md"],
                self.commit,
            )
            self.assertEqual(
                metadata_plan["affected_mechanism_ids"],
                ["fixture-equivalent", "fixture-not-needed"],
            )
            self.assertEqual(metadata_plan["functional_changed_paths"], [])
            self.assertEqual(
                metadata_plan["functional_affected_mechanism_ids"], []
            )

    def test_changed_owner_dependency_cannot_reuse_old_focused_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            (repo / "crates/example/src/lib.rs").write_text(
                "#[test]\nfn decision_matches_vanilla() { assert!(false); }\n",
                encoding="utf-8",
            )
            issues = validate_report(
                report, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn(
                "stale BBB dependency crates/example/src/lib.rs for fixture-equivalent",
                issues,
            )

    def test_ledger_source_hashes_are_scoped_to_the_mechanism_section(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            matrix = repo / "docs/wiki/parity/vanilla-26.1-matrix.md"
            matrix.write_text(
                matrix.read_text(encoding="utf-8").replace(
                    "`Example.unreachable` | none | no effect",
                    "external owner | none | no effect",
                ),
                encoding="utf-8",
            )
            ledger = repo / "docs/unsupported-features.md"
            ledger.write_text(
                """\
# Feature Gap TODO List

# Not needed

## Unrelated ruling

Matrix route: `unrelated-mechanism`

- Vanilla `UnrelatedOwner` is unreachable.

## Fixture ruling

Matrix route: `fixture-not-needed`

- Vanilla `ScopedOwner` is unreachable. Reopen when the caller exists.
""",
                encoding="utf-8",
            )
            unrelated = source_root / "example/UnrelatedOwner.java"
            scoped = source_root / "example/ScopedOwner.java"
            unrelated.parent.mkdir(parents=True, exist_ok=True)
            unrelated.write_text("class UnrelatedOwner {}\n", encoding="utf-8")
            scoped.write_text("class ScopedOwner {}\n", encoding="utf-8")

            report = self.build_fixture_report(repo, source_root)
            mechanism = next(
                item
                for item in report["mechanisms"]
                if item["mechanism_id"] == "fixture-not-needed"
            )
            self.assertEqual(
                mechanism["vanilla"]["source_hashes"],
                [
                    {
                        "path": "example/ScopedOwner.java",
                        "sha256": hashlib.sha256(scoped.read_bytes()).hexdigest(),
                    }
                ],
            )
            mechanism["vanilla"]["source_hashes"] = [
                {
                    "path": "example/UnrelatedOwner.java",
                    "sha256": hashlib.sha256(unrelated.read_bytes()).hexdigest(),
                }
            ]
            self.assertIn(
                "stale Vanilla source mapping for fixture-not-needed",
                validate_report(
                    report,
                    repo=repo,
                    source_root=source_root,
                    enforce_current=True,
                ),
            )

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

    def test_diagnostic_smoke_cannot_satisfy_a_formal_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifact_path = root / "smoke.json"
            artifact_path.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "mode": "smoke",
                        "formal_comparison_complete": False,
                    }
                ),
                encoding="utf-8",
            )
            artifact = external_artifact(
                repo=root,
                kind="performance",
                artifact_id="performance-abba",
                path=artifact_path,
                evidence_commit=self.commit,
            )
            self.assertEqual(artifact["evidence_class"], "diagnostic")
            self.assertFalse(artifact["valid"])
            self.assertIn(
                "diagnostic smoke cannot satisfy a formal receipt",
                artifact["invalid_reasons"],
            )

    def test_formal_lifecycle_receipt_requires_binary_identity(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifact_path = root / "lifecycle.json"
            artifact_path.write_text(
                json.dumps({"schema": 1, "head": self.commit, "status": "passed"}),
                encoding="utf-8",
            )
            artifact = external_artifact(
                repo=root,
                kind="lifecycle",
                artifact_id="lifecycle-soak",
                path=artifact_path,
                evidence_commit=self.commit,
            )
            self.assertFalse(artifact["valid"])
            self.assertIn(
                "formal receipt lacks BBB binary/load-image identity",
                artifact["invalid_reasons"],
            )

            artifact_path.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "head": self.commit,
                        "status": "passed",
                        "client_binary": "/tmp/bbb-native",
                        "client_binary_sha256": "2" * 64,
                    }
                ),
                encoding="utf-8",
            )
            artifact = external_artifact(
                repo=root,
                kind="lifecycle",
                artifact_id="lifecycle-soak",
                path=artifact_path,
                evidence_commit=self.commit,
            )
            self.assertTrue(artifact["valid"])

    def test_capture_contract_detects_tool_changes_after_capture(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary) / "repo"
            tool = repo / "scripts/official_client_benchmark.py"
            capture_input = (
                repo
                / "scripts/fixtures/official-client-benchmark/replay-26.1.json"
            )
            tool.parent.mkdir(parents=True)
            capture_input.parent.mkdir(parents=True)
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(
                ["git", "config", "user.email", "fixture@example.invalid"],
                cwd=repo,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Fixture"], cwd=repo, check=True
            )
            tool.write_text("capture contract\n", encoding="utf-8")
            capture_input.write_text("{}\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "capture"], cwd=repo, check=True)
            capture_commit = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            tool.write_text("changed contract\n", encoding="utf-8")
            capture_input.write_text('{"changed": true}\n', encoding="utf-8")
            contract = capture_contract_identity(
                repo, "workload", {"schema": 1}, "formal", capture_commit
            )
            self.assertNotEqual(
                contract["tools"][0]["capture_sha256"],
                contract["tools"][0]["current_sha256"],
            )
            self.assertNotEqual(
                contract["inputs"][0]["capture_sha256"],
                contract["inputs"][0]["current_sha256"],
            )

    def test_formal_abba_is_invalidated_before_its_stage(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            report = self.build_fixture_report(repo, source_root)
            artifact = next(
                item
                for item in report["artifacts"]
                if item["artifact_id"] == "performance-abba"
            )
            artifact["evidence_class"] = "formal"
            artifact["invalid_reasons"] = []
            artifact["valid"] = True
            issues = validate_report(
                report, repo=repo, source_root=source_root, enforce_current=True
            )
            self.assertIn("formal ABBA lacks a stage-order invalidation", issues)

    def test_formal_abba_prerequisite_is_commit_sealed_and_code_free(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary) / "repo"
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
            functional = repo / "functional"
            functional.write_text("frozen\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "freeze"], cwd=repo, check=True)
            evidence_commit = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            receipt_path = (
                repo / "docs/evidence/vanilla-26.1/parity-acceptance.json"
            )
            receipt_path.parent.mkdir(parents=True)
            receipt_path.write_text(
                json.dumps(
                    {
                        "bbb_evidence_commit": evidence_commit,
                        "workflow": {
                            "consistency_gate_passed": True,
                            "semantic_contract_sha256": "3" * 64,
                            "stage": "evidence-sweep",
                        },
                    }
                ),
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "seal"], cwd=repo, check=True)
            report_commit = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            gate_dir = repo / ".git/bbb"
            gate_dir.mkdir(parents=True)
            gate_line = (
                f"2026-08-07T00:00:00Z\t{report_commit}\tclean\ttotal:1s,test:1s"
            )
            (gate_dir / "gate-log.tsv").write_text(
                f"{gate_line}\n", encoding="utf-8"
            )
            identity = {
                "bbb_evidence_commit": evidence_commit,
                "full_gate_receipt_sha256": hashlib.sha256(
                    f"{gate_line}\n".encode()
                ).hexdigest(),
                "path": "docs/evidence/vanilla-26.1/parity-acceptance.json",
                "report_commit": report_commit,
                "semantic_contract_sha256": "3" * 64,
                "sha256": hashlib.sha256(receipt_path.read_bytes()).hexdigest(),
                "stage": "evidence-sweep",
            }
            value = {"prerequisites": {"consistency_receipt": identity}}
            self.assertEqual(
                performance_prerequisite_reasons(repo, value, evidence_commit), []
            )
            broken = copy.deepcopy(value)
            broken["prerequisites"]["consistency_receipt"]["sha256"] = "0" * 64
            self.assertIn(
                "formal ABBA consistency receipt is not sealed by its commit",
                performance_prerequisite_reasons(repo, broken, evidence_commit),
            )

    def test_final_exact_head_requires_clean_current_full_gate(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo, source_root = self.make_fixture(Path(temporary))
            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            subprocess.run(
                ["git", "config", "user.email", "fixture@example.invalid"],
                cwd=repo,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Fixture"], cwd=repo, check=True
            )
            subprocess.run(["git", "add", "."], cwd=repo, check=True)
            subprocess.run(["git", "commit", "-qm", "fixture"], cwd=repo, check=True)
            head = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            gate = repo / ".git/bbb/gate-log.tsv"
            gate.parent.mkdir(parents=True, exist_ok=True)
            gate.write_text(
                f"2026-08-07T00:00:00Z\t{head}\tclean\ttotal:1s,test:1s\n",
                encoding="utf-8",
            )
            (repo / ".git/bbb/last-gated").write_text(
                f"{head}\n", encoding="utf-8"
            )
            report = build_report(
                repo=repo,
                source_root=source_root,
                evidence_commit=head,
                gate_log=gate,
                workflow_stage="final",
            )
            self.assertEqual(
                validate_report(
                    report,
                    repo=repo,
                    source_root=source_root,
                    enforce_current=True,
                    require_final_head=True,
                ),
                [],
            )
            acceptance_path = (
                repo / "docs/evidence/vanilla-26.1/parity-acceptance.json"
            )
            acceptance_path.parent.mkdir(parents=True, exist_ok=True)
            acceptance_path.write_text(
                json.dumps(report, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            subprocess.run(
                ["git", "add", acceptance_path.relative_to(repo).as_posix()],
                cwd=repo,
                check=True,
            )
            subprocess.run(
                ["git", "commit", "-qm", "seal acceptance"], cwd=repo, check=True
            )
            sealed_head = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            with gate.open("a", encoding="utf-8") as handle:
                handle.write(
                    f"2026-08-07T00:00:01Z\t{sealed_head}\tclean\ttotal:1s,test:1s\n"
                )
            (repo / ".git/bbb/last-gated").write_text(
                f"{sealed_head}\n", encoding="utf-8"
            )
            self.assertEqual(
                validate_report(
                    report,
                    repo=repo,
                    source_root=source_root,
                    enforce_current=True,
                    require_final_head=True,
                ),
                [],
            )
            (repo / "crates/example/src/lib.rs").write_text(
                "dirty\n", encoding="utf-8"
            )
            issues = validate_report(
                report,
                repo=repo,
                source_root=source_root,
                enforce_current=True,
                require_final_head=True,
            )
            self.assertIn(
                "final exact-HEAD check requires a clean worktree", issues
            )
            subprocess.run(
                ["git", "commit", "-qam", "functional change"],
                cwd=repo,
                check=True,
            )
            functional_head = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=repo,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
            with gate.open("a", encoding="utf-8") as handle:
                handle.write(
                    f"2026-08-07T00:00:02Z\t{functional_head}\tclean\ttotal:1s,test:1s\n"
                )
            (repo / ".git/bbb/last-gated").write_text(
                f"{functional_head}\n", encoding="utf-8"
            )
            issues = validate_report(
                report,
                repo=repo,
                source_root=source_root,
                enforce_current=True,
                require_final_head=True,
            )
            self.assertIn(
                "final exact-HEAD check permits only closure-metadata changes after evidence commit",
                issues,
            )

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
