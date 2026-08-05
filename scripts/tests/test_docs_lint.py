import os
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
DOCS_LINT = REPO_ROOT / "scripts" / "docs-lint.sh"
sys.path.insert(0, str(REPO_ROOT / "scripts"))
from docs_wiki_lint import WikiLint


ROADMAP_STATUSES = """\
<!-- roadmap-status:start -->
- p1-playable: maintenance
- p1-generated-facts: maintenance
- p2-non-entity: maintenance
- p2-hud: maintenance
- p3-resources: maintenance
- p4-delivery: maintenance
- architecture-convergence: maintenance
<!-- roadmap-status:end -->
"""


class DocsLintTests(unittest.TestCase):
    def reload_projection_fixture(
        self, repo: Path, owner_ids: list[str]
    ) -> tuple[WikiLint, list[dict[str, object]], dict[str, object]]:
        page = repo / "docs/wiki/mechanisms/resources/offline-asset-trust.md"
        page.parent.mkdir(parents=True, exist_ok=True)
        table_rows = "\n".join(
            f"| {index} | `{owner_id}` | listener | bbb | owner | `existing` | evidence |"
            for index, owner_id in enumerate(owner_ids, start=1)
        )
        page.write_text(
            "<!-- reload-listener-projections:start -->\n"
            "| 序 | Inventory owner | Vanilla | BBB | apply | 状态 | Evidence |\n"
            "| ---: | --- | --- | --- | --- | --- | --- |\n"
            f"{table_rows}\n"
            "<!-- reload-listener-projections:end -->\n",
            encoding="utf-8",
        )
        owners = [
            {
                "mechanism_id": "resources-reload-listeners",
                "owner_id": owner_id,
                "source": {"path": "fixture/Minecraft.java", "line": 10 + index},
            }
            for index, owner_id in enumerate(owner_ids)
        ]
        mechanisms: dict[str, object] = {"resources-reload-listeners": {}}
        return WikiLint(repo), owners, mechanisms

    def sync_inventory(self, repo: Path) -> None:
        paths = sorted(
            str(path.relative_to(repo)) for path in repo.rglob("*.md")
        )
        inventory = repo / "docs" / "wiki" / "development" / "document-inventory.md"
        inventory_relative = str(inventory.relative_to(repo))
        if inventory_relative not in paths:
            paths.append(inventory_relative)
            paths.sort()
        rows = "\n".join(
            f"| `{path}` | fixture | 是 | 同路径 | 保留 | fixture | 测试 | 无冲突 |"
            for path in paths
        )
        inventory.write_text(
            """\
# 测试文档清单

这是中文 Wiki lint 的最小测试清单，用于验证所有 Markdown 都被唯一记录。

| 当前路径 | 当前职责 | 是否仍准确 | Canonical destination | 处理方式 | 引用者/脚本 | 内容类别 | 重复或冲突 |
| --- | --- | --- | --- | --- | --- | --- | --- |
"""
            + rows
            + "\n",
            encoding="utf-8",
        )

    def make_repo(self, root: Path) -> Path:
        repo = root / "repo"
        (repo / "docs" / "wiki" / "parity").mkdir(parents=True)
        (repo / "docs" / "wiki" / "development").mkdir(parents=True)
        evidence = repo / "docs" / "evidence" / "vanilla-26.1"
        evidence.mkdir(parents=True)
        (repo / "goal.md").write_text(
            "# Goal\n\n" + ROADMAP_STATUSES,
            encoding="utf-8",
        )
        (repo / "docs" / "goal-archive.md").write_text(
            "# Goal archive\n", encoding="utf-8"
        )
        (repo / "docs" / "unsupported-features.md").write_text(
            """\
# Feature Gap TODO List

# Todo

## Deferred owner

Matrix routes: `fixture-deferred`

- A bounded gap is `deferred`. Reopen when its exact owner exists.

# Not needed

## Vanilla exclusions

Matrix routes: `fixture-not-needed`

- Vanilla `ExampleOwner.method` has no client behavior.
""",
            encoding="utf-8",
        )
        (repo / "docs" / "README.md").write_text(
            "# 文档入口\n\n这是测试仓库的中文文档入口，链接到[中文机制 Wiki](wiki/README.md)，用于验证导航和链接。\n",
            encoding="utf-8",
        )
        (repo / "docs" / "wiki" / "README.md").write_text(
            """\
# 中文机制 Wiki

这是 fixture 的中文 Wiki 入口，正文足够完整且不是英文占位页面。

- [术语表](glossary.md)
- [机制矩阵](parity/vanilla-26.1-matrix.md)
- [文档清单](development/document-inventory.md)
""",
            encoding="utf-8",
        )
        (repo / "docs" / "wiki" / "glossary.md").write_text(
            "# 术语表\n\n## 核心术语\n\n这里用中文解释语义所有者、状态投影、回退行为、聚焦测试和完整门禁，供测试页面共享。\n\n## 维护规则\n\n内容不是占位符，并由入口页面导航。\n",
            encoding="utf-8",
        )
        (repo / "docs" / "wiki" / "parity" / "vanilla-26.1-matrix.md").write_text(
            """\
# Vanilla 机制矩阵

这是中文机制覆盖测试矩阵，状态只用于审计，不维护第二份工程任务。

| Mechanism ID | 领域 | Vanilla owner/class | BBB owner/module | Required state/projection | 状态 | Focused test/evidence | Route / reopen condition |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `fixture-valid` | 测试 | `Owner.method` | fixture | value | `equivalent` | fixture | 版本变化时重开 |
| `fixture-deferred` | 测试 | `Owner.deferred` | fixture | value | `deferred-external` | fixture | [ledger](../../unsupported-features.md) |
| `fixture-not-needed` | 测试 | `Owner.excluded` | fixture | value | `not-needed` | fixture | [ledger](../../unsupported-features.md) |
""",
            encoding="utf-8",
        )
        glossary = repo / "docs" / "wiki" / "glossary.md"
        (evidence / "client-mechanism-inventory.json").write_text(
            json.dumps(
                {
                    "generated_by": "fixture",
                    "generator_inputs": [
                        {
                            "path": "docs/wiki/glossary.md",
                            "sha256": hashlib.sha256(glossary.read_bytes()).hexdigest(),
                        }
                    ],
                    "mechanisms": {
                        "fixture-valid": {
                            "bbb_evidence": ["docs/wiki/glossary.md"],
                            "owner_count": 1,
                        }
                    },
                    "owners": [
                        {
                            "category": "fixture",
                            "domain": "fixture",
                            "kind": "method",
                            "mechanism_id": "fixture-valid",
                            "owner_id": "method:fixture.Owner#method()",
                            "scope": "external-client",
                            "source": {"line": 1, "path": "fixture/Owner.java"},
                            "vanilla_owner": "fixture.Owner#method()",
                        }
                    ],
                    "schema_version": 1,
                    "selection_contract": {"policy": "fixture"},
                    "source_files": [
                        {"path": "fixture/Owner.java", "sha256": "0" * 64}
                    ],
                    "summary": {
                        "by_category": {"fixture": 1},
                        "by_domain": {"fixture": 1},
                        "by_role": {},
                        "mapped_owner_count": 1,
                        "owner_count": 1,
                        "unmapped_owner_count": 0,
                    },
                    "vanilla_version": "26.1",
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
        for name in (
            "client-options-catalog.json",
            "entity-coverage.json",
            "item-component-coverage.json",
            "key-mapping-catalog.json",
            "packet-coverage.json",
            "registry-holder-consumer-coverage.json",
        ):
            (evidence / name).write_text("{}\n", encoding="utf-8")
        self.sync_inventory(repo)
        return repo

    def add_architecture_candidate(self, repo: Path) -> None:
        owner = repo / "crates" / "fixture" / "src" / "owner.rs"
        caller = repo / "crates" / "fixture" / "src" / "caller.rs"
        evidence = repo / "scripts" / "fixture-architecture-guard.sh"
        owner.parent.mkdir(parents=True, exist_ok=True)
        evidence.parent.mkdir(parents=True, exist_ok=True)
        owner.write_text("pub fn owner() {}\n", encoding="utf-8")
        caller.write_text("pub fn caller() {}\n", encoding="utf-8")
        evidence.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")

        ledger = repo / "docs" / "unsupported-features.md"
        candidate = """\

## Architecture drift candidates

Owner: fixture workspace; census evidence owned by `scripts/docs_wiki_lint.py`

Matrix route: `fixture-valid`

<!-- architecture-candidates:start -->
- **`arch-fixture-single-owner` is `active`.**
  - Semantic mechanism: one bounded fixture decision.
  - Canonical input: one immutable fixture value.
  - Owners: `crates/fixture/src/owner.rs` and one duplicate fixture owner.
  - Production callers: `crates/fixture/src/caller.rs` submits the value.
  - Direction: source to owner to immutable consumer projection.
  - Lifetime: one atomic fixture generation; stale generations are rejected.
  - Ordering/fallback/rollback: preserve FIFO, default fallback and old generation on failure.
  - Public API: one narrow fixture projection.
  - Delete: duplicate fixture resolver and compatibility cache.
  - Evidence: `scripts/fixture-architecture-guard.sh` plus focused fixture tests.
  - Risk/benefit/rank: drift high; behavior medium; deletion medium; rank 1/1.
  - Goal route: `architecture-convergence`; `fixture-valid`.
  - Reopen condition: reopen only if a second production owner returns.
<!-- architecture-candidates:end -->
"""
        ledger.write_text(
            ledger.read_text(encoding="utf-8").replace("\n# Not needed\n", candidate + "\n# Not needed\n"),
            encoding="utf-8",
        )
        goal = repo / "goal.md"
        goal.write_text(
            goal.read_text(encoding="utf-8").replace(
                "- architecture-convergence: maintenance",
                "- architecture-convergence: active",
            ),
            encoding="utf-8",
        )

    def run_lint(
        self, repo: Path, vanilla_source_root: Path | None = None
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["BBB_DOCS_LINT_REPO_ROOT"] = str(repo)
        env["BBB_DOCS_LINT_VANILLA_SOURCE_ROOT"] = str(
            vanilla_source_root or repo / "missing-vanilla"
        )
        return subprocess.run(
            ["sh", str(DOCS_LINT)],
            cwd=repo,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def test_valid_zero_ready_maintenance_state_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = self.run_lint(self.make_repo(Path(directory)))

        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("route/ledger states consistent", result.stdout)

    def test_valid_architecture_candidate_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            self.add_architecture_candidate(repo)
            result = self.run_lint(repo)

        self.assertEqual(result.returncode, 0, result.stdout)

    def test_duplicate_architecture_candidate_id_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            self.add_architecture_candidate(repo)
            ledger = repo / "docs" / "unsupported-features.md"
            text = ledger.read_text(encoding="utf-8")
            block = text.split("<!-- architecture-candidates:start -->", 1)[1].split(
                "<!-- architecture-candidates:end -->", 1
            )[0]
            ledger.write_text(
                text.replace(
                    "<!-- architecture-candidates:end -->",
                    block + "<!-- architecture-candidates:end -->",
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate architecture candidate ids", result.stdout)

    def test_architecture_candidate_broken_goal_route_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            self.add_architecture_candidate(repo)
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    "Goal route: `architecture-convergence`",
                    "Goal route: `p2-hud`",
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("has invalid goal route", result.stdout)

    def test_architecture_candidate_stale_owner_path_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            self.add_architecture_candidate(repo)
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    "crates/fixture/src/owner.rs",
                    "crates/fixture/src/missing-owner.rs",
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stale architecture candidate owners path", result.stdout)

    def test_architecture_candidate_stale_evidence_path_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            self.add_architecture_candidate(repo)
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    "scripts/fixture-architecture-guard.sh",
                    "scripts/missing-architecture-guard.sh",
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stale architecture candidate evidence path", result.stdout)

    def test_architecture_candidate_unknown_matrix_route_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            self.add_architecture_candidate(repo)
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    "Goal route: `architecture-convergence`; `fixture-valid`.",
                    "Goal route: `architecture-convergence`; `fixture-stale`.",
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("references unknown matrix mechanism", result.stdout)

    def test_deferred_item_under_not_needed_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8")
                + "\n- Vanilla `ExampleOwner` gap — deferred. "
                + "Reopen when its owner exists.\n",
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("deferred items must stay under their owning # Todo", result.stdout)

    def test_deferred_item_without_reopen_condition_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    " Reopen when its exact owner exists.", ""
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("deferred items need an explicit 'Reopen when ...'", result.stdout)

    def test_synchronized_custom_identity_open_task_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            goal = repo / "goal.md"
            goal.write_text(
                goal.read_text(encoding="utf-8")
                + "\n1. **同步 registry 到 custom presentation**："
                + "custom block/entity identity 接现有 loader。\n",
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(
            "synchronized custom block/entity identity cannot be an open roadmap task",
            result.stdout,
        )

    def test_zero_active_todo_cannot_keep_active_roadmap_queue(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            goal = repo / "goal.md"
            goal.write_text(
                goal.read_text(encoding="utf-8").replace(
                    "- p2-hud: maintenance", "- p2-hud: active"
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(
            "active Todo is zero, so roadmap statuses cannot retain an active queue",
            result.stdout,
        )

    def test_active_todo_requires_an_active_roadmap_owner(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    "- A bounded gap is `deferred`. Reopen when its exact owner exists.",
                    "- Implement one bounded behavior.",
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(
            "active Todo items exist but no roadmap route owns them", result.stdout
        )

    def test_not_needed_item_without_evidence_anchor_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    "- Vanilla `ExampleOwner.method` has no client behavior.",
                    "- This behavior is unnecessary.",
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(
            "# Not needed items need a Vanilla statement or source-code anchor",
            result.stdout,
        )

    def test_broken_wiki_link_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            index = repo / "docs" / "wiki" / "README.md"
            index.write_text(
                index.read_text(encoding="utf-8") + "\n[断链](missing.md)\n",
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("broken relative Markdown link", result.stdout)

    def test_historical_markdown_outside_wiki_fails(self) -> None:
        for relative in ("docs/archive/old-design.md", "docs/legacy-note.md"):
            with self.subTest(relative=relative), tempfile.TemporaryDirectory() as directory:
                repo = self.make_repo(Path(directory))
                legacy = repo / relative
                legacy.parent.mkdir(parents=True, exist_ok=True)
                legacy.write_text("# Old design\n\nHistorical prose.\n", encoding="utf-8")
                self.sync_inventory(repo)
                result = self.run_lint(repo)

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn(
                "historical Markdown outside the Wiki/flow allowlist", result.stdout
            )

    def test_vanilla_evidence_missing_member_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "docs/evidence/vanilla-26.1/entity-coverage.json").unlink()
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("Vanilla evidence directory missing members", result.stdout)

    def test_vanilla_evidence_unexpected_member_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "docs/evidence/vanilla-26.1/extra.json").write_text(
                "{}\n", encoding="utf-8"
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("Vanilla evidence directory has unexpected members", result.stdout)

    def test_root_level_docs_json_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "docs/root.json").write_text("{}\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("root-level docs JSON is not allowed", result.stdout)

    def test_legacy_vanilla_evidence_reference_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            legacy = "docs/" + "packet-coverage-26.1.json"
            goal = repo / "goal.md"
            goal.write_text(
                goal.read_text(encoding="utf-8") + f"\nLegacy: {legacy}\n",
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(
            "legacy Vanilla evidence references are not allowed", result.stdout
        )

    def test_duplicate_mechanism_id_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            index = repo / "docs" / "wiki" / "README.md"
            for name in ("one.md", "two.md"):
                (repo / "docs" / "wiki" / name).write_text(
                    "# 中文机制\n\n机制 ID：`fixture-duplicate`\n\n这是完整中文测试机制页面，包含所有者、状态和验证说明。\n",
                    encoding="utf-8",
                )
                index.write_text(
                    index.read_text(encoding="utf-8") + f"\n[{name}]({name})\n",
                    encoding="utf-8",
                )
            self.sync_inventory(repo)
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate mechanism id fixture-duplicate", result.stdout)

    def test_illegal_parity_status_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            matrix = repo / "docs" / "wiki" / "parity" / "vanilla-26.1-matrix.md"
            matrix.write_text(
                matrix.read_text(encoding="utf-8").replace(
                    "`equivalent`", "`almost-done`"
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("illegal parity status", result.stdout)

    def test_orphan_wiki_page_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "docs" / "wiki" / "orphan.md").write_text(
                "# 孤儿页面\n\n这是没有从 Wiki 入口可达的中文机制页面，应被文档 lint 明确拒绝。\n\n它不是占位内容。\n",
                encoding="utf-8",
            )
            self.sync_inventory(repo)
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("orphan canonical Wiki page", result.stdout)

    def test_open_parity_state_without_ledger_or_goal_route_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            matrix = repo / "docs" / "wiki" / "parity" / "vanilla-26.1-matrix.md"
            matrix.write_text(
                matrix.read_text(encoding="utf-8").replace(
                    "`equivalent`", "`active`"
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("invalid open-state route", result.stdout)

    def test_inventory_owner_without_mapping_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            path = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["owners"][0]["mechanism_id"] = "fixture-unmapped"
            path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("unmapped Vanilla owner", result.stdout)

    def test_inventory_mechanism_without_matrix_row_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            path = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["mechanisms"]["fixture-unmapped"] = {
                "bbb_evidence": ["docs/wiki/glossary.md"],
                "owner_count": 0,
            }
            path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("unmapped inventory mechanism id fixture-unmapped", result.stdout)

    def test_duplicate_inventory_owner_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            path = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["owners"].append(dict(value["owners"][0]))
            path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("duplicate Vanilla owner ids", result.stdout)

    def test_invalid_inventory_owner_role_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            path = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["owners"][0]["role"] = "looks-covered"
            path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("invalid role for Vanilla owner", result.stdout)

    def test_stale_inventory_generator_input_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            glossary = repo / "docs" / "wiki" / "glossary.md"
            glossary.write_text(glossary.read_text(encoding="utf-8") + "\n发生变化。\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("generator input is stale", result.stdout)

    def test_stale_inventory_source_hash_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            source_root = repo / "vanilla"
            source = source_root / "fixture" / "Owner.java"
            source.parent.mkdir(parents=True)
            source.write_text("class Owner { void changed() {} }\n", encoding="utf-8")
            result = self.run_lint(repo, source_root)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("inventory source is stale", result.stdout)

    def test_stale_bbb_evidence_path_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            path = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["mechanisms"]["fixture-valid"]["bbb_evidence"] = ["crates/missing.rs"]
            path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stale BBB evidence path", result.stdout)

    def test_stale_bbb_evidence_symbol_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            path = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["mechanisms"]["fixture-valid"]["bbb_evidence"] = [
                "docs/wiki/glossary.md#missing_fixture_symbol"
            ]
            path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stale BBB evidence symbol", result.stdout)

    def test_declared_owner_evidence_mechanism_without_owner_cells_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            path = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["selection_contract"]["owner_evidence_mechanisms"] = [
                "fixture-valid"
            ]
            value["mechanisms"]["fixture-valid"]["owner_evidence_count"] = 1
            path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("lacks implementation/test BBB evidence", result.stdout)

    def test_stale_per_owner_evidence_symbol_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            path = repo / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            value["selection_contract"]["owner_evidence_mechanisms"] = [
                "fixture-valid"
            ]
            value["mechanisms"]["fixture-valid"]["owner_evidence_count"] = 1
            value["owners"][0]["bbb_evidence"] = {
                "implementation": ["docs/wiki/glossary.md#missing_owner_symbol"],
                "tests": ["docs/wiki/glossary.md#核心术语"],
            }
            path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("stale BBB evidence symbol for inventory owner", result.stdout)

    def test_ledger_route_to_unknown_matrix_id_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    "Matrix routes: `fixture-deferred`",
                    "Matrix routes: `fixture-unknown`",
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("ledger route references unknown mechanism id", result.stdout)

    def test_equivalent_row_without_locatable_evidence_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            matrix = repo / "docs" / "wiki" / "parity" / "vanilla-26.1-matrix.md"
            matrix.write_text(
                matrix.read_text(encoding="utf-8")
                + "| `fixture-no-evidence` | 测试 | owner | bbb | state | `equivalent` | prose only | version |\n",
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("lacks locatable evidence", result.stdout)

    def test_reload_listener_projection_matches_inventory_source_order(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            lint, owners, mechanisms = self.reload_projection_fixture(
                Path(directory), ["registration:reload:0", "registration:reload:1"]
            )
            lint.check_reload_listener_projections(owners, mechanisms)

        self.assertEqual(lint.issues, [])

    def test_reload_listener_projection_missing_owner_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            lint, owners, mechanisms = self.reload_projection_fixture(
                Path(directory), ["registration:reload:0"]
            )
            owners.append(
                {
                    "mechanism_id": "resources-reload-listeners",
                    "owner_id": "registration:reload:1",
                    "source": {"path": "fixture/Minecraft.java", "line": 12},
                }
            )
            lint.check_reload_listener_projections(owners, mechanisms)

        self.assertIn("reload-listener projection table is stale", lint.issues[0])

    def test_reload_listener_projection_reordered_owner_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            lint, owners, mechanisms = self.reload_projection_fixture(
                Path(directory), ["registration:reload:1", "registration:reload:0"]
            )
            owners[0]["source"] = {"path": "fixture/Minecraft.java", "line": 12}
            owners[1]["source"] = {"path": "fixture/Minecraft.java", "line": 11}
            lint.check_reload_listener_projections(owners, mechanisms)

        self.assertIn("reload-listener projection table is stale", lint.issues[0])


if __name__ == "__main__":
    unittest.main()
