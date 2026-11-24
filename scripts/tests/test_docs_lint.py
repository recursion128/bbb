import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
DOCS_LINT = REPO_ROOT / "scripts" / "docs_lint.py"


class DocsLintTests(unittest.TestCase):
    def sync_inventory(self, repo: Path) -> None:
        paths = sorted(
            path.relative_to(repo).as_posix() for path in repo.rglob("*.md")
        )
        inventory = repo / "docs" / "wiki" / "development" / "document-inventory.md"
        inventory_relative = inventory.relative_to(repo).as_posix()
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
        (repo / "docs" / "wiki" / "development").mkdir(parents=True)
        (repo / "docs" / "unsupported-client-features.md").write_text(
            """\
# Feature Gap TODO List

# Todo

## Deferred owner

- A bounded gap is `deferred`. Reopen when its exact owner exists.

# Not needed

## Vanilla exclusions

- Vanilla `ExampleOwner.method` has no client behavior.
""",
            encoding="utf-8",
        )
        (repo / "docs" / "unsupported-server-features.md").write_text(
            """\
# Dedicated Server Feature Gap List

# Todo

## Deferred server owner

- A server gap is `deferred`. Reopen when the server fixture exists.

# Not needed

## Vanilla server exclusions

- Vanilla `DedicatedServer` fixture has no declared server behavior.
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
- [文档清单](development/document-inventory.md)

页面只描述当前机制，完成史只查 Git。
""",
            encoding="utf-8",
        )
        (repo / "docs" / "wiki" / "glossary.md").write_text(
            "# 术语表\n\n## 核心术语\n\n这里用中文解释语义所有者、状态投影、回退行为、聚焦测试和完整门禁，供测试页面共享。\n\n## 维护规则\n\n内容不是占位符，并由入口页面导航。\n",
            encoding="utf-8",
        )
        self.sync_inventory(repo)
        return repo

    def test_historical_receipt_snapshot_in_wiki_fails(self) -> None:
        cases = (
            "旧验收 capture `abcdef1` → gated evidence `1234567`。",
            "主树基线是 `d57abb7866f3eed174fe1c9a0636dc08e1acfb56`。",
            "本地快照记录于 2026-11-06 17:53:51（Asia/Hong_Kong）。",
            "见本机状态文件 `handoff-state.json`。",
        )
        for snippet in cases:
            with self.subTest(snippet=snippet), tempfile.TemporaryDirectory() as root:
                repo = self.make_repo(Path(root))
                page = repo / "docs" / "wiki" / "glossary.md"
                page.write_text(
                    page.read_text(encoding="utf-8") + f"\n{snippet}\n",
                    encoding="utf-8",
                )
                self.sync_inventory(repo)

                result = self.run_lint(repo)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn("historical receipt snapshots", result.stdout)

    def test_mechanism_page_without_id_or_required_sections_fails(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            repo = self.make_repo(Path(root))
            mech_dir = repo / "docs" / "wiki" / "mechanisms" / "world"
            mech_dir.mkdir(parents=True)
            mech_page = mech_dir / "server-sample.md"
            mech_page.write_text(
                "# 示例服务端机制\n\n## 模块职责\n\n这段中文机制说明故意缺少机制 ID 与统一章节结构，用于验证机制目录结构检查。\n",
                encoding="utf-8",
            )
            index = repo / "docs" / "wiki" / "README.md"
            index.write_text(
                index.read_text(encoding="utf-8")
                + "\n- [示例服务端机制](mechanisms/world/server-sample.md)\n",
                encoding="utf-8",
            )
            self.sync_inventory(repo)

            result = self.run_lint(repo)

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn("missing 机制 ID or 扩展 ID", result.stdout)

    def run_lint(
        self,
        repo: Path,
        *extra_args: str,
    ) -> subprocess.CompletedProcess[str]:
        workspace_root = repo / "vanilla-workspace"
        sources = workspace_root / "sources" / "26.1"
        for relative in (
            "net/minecraft/client/main/Main.java",
            "net/minecraft/client/Minecraft.java",
            "net/minecraft/client/multiplayer/ClientPacketListener.java",
            "net/minecraft/client/renderer/block/BlockModelLighter.java",
            "net/minecraft/client/renderer/block/FluidRenderer.java",
            "net/minecraft/client/gui/font/FontSet.java",
            "net/minecraft/client/gui/font/providers/UnihexProvider.java",
            "net/minecraft/server/Main.java",
            "net/minecraft/server/MinecraftServer.java",
            "net/minecraft/server/dedicated/DedicatedServer.java",
            "net/minecraft/server/level/ServerLevel.java",
            "net/minecraft/server/network/ServerConnectionListener.java",
            "net/minecraft/server/network/ServerLoginPacketListenerImpl.java",
            "net/minecraft/server/network/ServerConfigurationPacketListenerImpl.java",
            "net/minecraft/server/network/ServerGamePacketListenerImpl.java",
            "net/minecraft/server/jsonrpc/IncomingRpcMethods.java",
            "net/minecraft/server/jsonrpc/internalapi/MinecraftApi.java",
            "net/minecraft/server/jsonrpc/internalapi/MinecraftExecutorServiceImpl.java",
        ):
            source = sources / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.touch()
        (sources / "assets" / "minecraft").mkdir(parents=True, exist_ok=True)
        (workspace_root / "downloads" / "26.1").mkdir(parents=True, exist_ok=True)
        state = workspace_root / "state" / "26.1.json"
        state.parent.mkdir(parents=True, exist_ok=True)
        state.write_text('{"status":"complete"}\n', encoding="utf-8")
        env = os.environ.copy()
        env["BBB_DOCS_LINT_REPO_ROOT"] = str(repo)
        env["BBB_VANILLA_ROOT"] = str(workspace_root)
        env["PYTHONUTF8"] = "1"
        env["PYTHONIOENCODING"] = "utf-8"
        command = [sys.executable, str(DOCS_LINT), *extra_args]
        return subprocess.run(
            command,
            cwd=repo,
            env=env,
            text=True,
            encoding="utf-8",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def test_valid_ledger_and_wiki_state_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            result = self.run_lint(self.make_repo(Path(directory)))

        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("ledgers consistent", result.stdout)

    def test_deferred_item_under_not_needed_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-client-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8")
                + "\n- Vanilla `ExampleOwner` gap — deferred. "
                + "Reopen when its owner exists.\n",
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("deferred items must stay under their owning # Todo", result.stdout)

    def test_todo_item_outside_a_domain_heading_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-server-features.md"
            text = ledger.read_text(encoding="utf-8")
            self.assertIn("# Todo\n", text)
            ledger.write_text(
                text.replace(
                    "# Todo\n",
                    "# Todo\n- **`orphan` is `deferred`.** Reopen when a domain claims it.\n",
                    1,
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("must sit under a ## domain heading", result.stdout)
        self.assertIn("orphan", result.stdout)

    def test_deferred_item_without_reopen_condition_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-client-features.md"
            ledger.write_text(
                ledger.read_text(encoding="utf-8").replace(
                    " Reopen when its exact owner exists.", ""
                ),
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("deferred items need an explicit 'Reopen when ...'", result.stdout)

    def test_not_needed_item_without_evidence_anchor_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            ledger = repo / "docs" / "unsupported-client-features.md"
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

    def test_noncanonical_markdown_outside_wiki_fails(self) -> None:
        for relative in ("docs/archive/old-design.md", "docs/unowned-note.md"):
            with self.subTest(relative=relative), tempfile.TemporaryDirectory() as directory:
                repo = self.make_repo(Path(directory))
                note = repo / relative
                note.parent.mkdir(parents=True, exist_ok=True)
                note.write_text("# Unowned note\n\nNoncanonical prose.\n", encoding="utf-8")
                self.sync_inventory(repo)
                result = self.run_lint(repo)

            self.assertNotEqual(result.returncode, 0, result.stdout)
            expected = (
                "completed-history archive files are not allowed"
                if relative.startswith("docs/archive/")
                else "historical Markdown outside the Wiki/flow allowlist"
            )
            self.assertIn(expected, result.stdout)

    def test_completed_history_archive_is_rejected_before_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            archive = repo / "docs" / "goal-archive.md"
            archive.write_text("# Goal archive\n", encoding="utf-8")
            result = self.run_lint(repo)

        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("completed-history archive files are not allowed", result.stdout)

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

    def test_skip_vanilla_passes_without_vanilla_workspace(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            env = os.environ.copy()
            env["BBB_DOCS_LINT_REPO_ROOT"] = str(repo)
            env["BBB_VANILLA_ROOT"] = str(repo / "absent-vanilla")
            result = subprocess.run(
                [sys.executable, str(DOCS_LINT), "--skip-vanilla"],
                cwd=repo,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
            )

        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("local Vanilla source check skipped (--skip-vanilla)", result.stdout)

    def test_broken_markdown_heading_anchor_fails(self) -> None:
        for link in ("[坏锚点](glossary.md#不存在的章节)", "[本页坏锚点](#missing-anchor)"):
            with self.subTest(link=link), tempfile.TemporaryDirectory() as directory:
                repo = self.make_repo(Path(directory))
                index = repo / "docs" / "wiki" / "README.md"
                index.write_text(
                    index.read_text(encoding="utf-8") + f"\n{link}\n",
                    encoding="utf-8",
                )
                result = self.run_lint(repo)

                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn("broken Markdown heading anchor", result.stdout)

    def test_valid_markdown_heading_and_html_anchors_pass(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            glossary = repo / "docs" / "wiki" / "glossary.md"
            glossary.write_text(
                glossary.read_text(encoding="utf-8")
                + '\n<a id="custom-anchor"></a>\n\n## Live 与发布级验收（Windows）\n\n说明文本。\n',
                encoding="utf-8",
            )
            index = repo / "docs" / "wiki" / "README.md"
            index.write_text(
                index.read_text(encoding="utf-8")
                + "\n- [核心术语](glossary.md#核心术语)\n"
                + "- [自定义锚点](glossary.md#custom-anchor)\n"
                + "- [中英混合锚点](glossary.md#live-与发布级验收windows)\n",
                encoding="utf-8",
            )
            result = self.run_lint(repo)

        self.assertEqual(result.returncode, 0, result.stdout)

    def test_invalid_mechanism_table_status_fails_and_valid_passes(self) -> None:
        template = """\
# 示例客户端机制

机制 ID：`client-sample`

## 机制目的和边界
说明客户端示例机制的目的和边界，确保中文正文足够完整。

## Vanilla 26.1 权威实现
`SampleOwner.tick`

## 决策输入与状态所有权
状态归属清晰。

## 判断与处理顺序
按步骤执行。

## 生命周期和 fallback
失败时回退到安全默认值。

## BBB 实现映射
`bbb-client` 负责该示例机制。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 示例行为 | `SampleOwner.tick` | `bbb-client` | {status} | 单元测试 |

## 结构性不变量
单一所有者。

## 聚焦测试（focused tests）和真实证据
聚焦测试通过。

## 当前差异
无额外差异。

## 账本归属
见客户端账本。

## 精确重开条件
协议变更时重开。

## 相关页面和源码索引
见术语表。
"""
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            mech_dir = repo / "docs" / "wiki" / "mechanisms" / "ui"
            mech_dir.mkdir(parents=True)
            mech_page = mech_dir / "client-sample.md"
            mech_page.write_text(template.format(status="见 ledger"), encoding="utf-8")
            index = repo / "docs" / "wiki" / "README.md"
            index.write_text(
                index.read_text(encoding="utf-8")
                + "\n- [示例机制](mechanisms/ui/client-sample.md)\n",
                encoding="utf-8",
            )
            self.sync_inventory(repo)

            bad = self.run_lint(repo)
            self.assertNotEqual(bad.returncode, 0, bad.stdout)
            self.assertIn("invalid Vanilla/BBB table status", bad.stdout)

            mech_page.write_text(template.format(status="`aligned`"), encoding="utf-8")
            good = self.run_lint(repo)
            self.assertEqual(good.returncode, 0, good.stdout)

    def test_all_repository_generators_are_registered_without_overlap(self) -> None:
        sys.path.insert(0, str(REPO_ROOT / "scripts"))
        import docs_lint

        groups = (
            docs_lint.OFFLINE_NO_VANILLA_GENERATORS,
            docs_lint.FAST_VANILLA_GENERATORS,
            docs_lint.SLOW_VANILLA_GENERATORS,
            docs_lint.JVM_RUNTIME_GENERATORS,
        )
        registered: list[str] = [name for group in groups for name in group]
        self.assertEqual(len(registered), len(set(registered)))
        actual = sorted(p.name for p in (REPO_ROOT / "scripts").glob("generate-*.py"))
        self.assertEqual(sorted(registered), actual)

    def test_all_generators_flag_runs_jvm_runtime_generators_only_when_enabled(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            scripts_dir = repo / "scripts"
            scripts_dir.mkdir(parents=True)
            marker = repo / "jvm-generator-args.txt"
            (scripts_dir / "generate-block-placement-defaults.py").write_text(
                "import sys\n"
                "from pathlib import Path\n"
                f"Path({str(marker)!r}).write_text('\\n'.join(sys.argv[1:]), encoding='utf-8')\n",
                encoding="utf-8",
            )

            default_run = self.run_lint(repo)
            self.assertEqual(default_run.returncode, 0, default_run.stdout)
            self.assertFalse(marker.exists())

            full_run = self.run_lint(repo, "--all-generators")
            self.assertEqual(full_run.returncode, 0, full_run.stdout)
            self.assertIn("1 JVM runtime generator checks passed", full_run.stdout)
            recorded_args = marker.read_text(encoding="utf-8").splitlines()
            self.assertEqual(
                recorded_args,
                ["--vanilla-root", str(repo / "vanilla-workspace"), "--check"],
            )

    def test_gate_docs_sync_matches_repository_and_detects_drift(self) -> None:
        sys.path.insert(0, str(REPO_ROOT / "scripts"))
        import docs_lint

        self.assertEqual(docs_lint.check_gate_docs_sync(REPO_ROOT), [])

        gate_rel = Path("crates/bbb-xtask/src/gate.rs")
        doc_rel = Path("docs/wiki/development/testing-and-gates.md")
        gate_src = (REPO_ROOT / gate_rel).read_text(encoding="utf-8")
        doc_src = (REPO_ROOT / doc_rel).read_text(encoding="utf-8")

        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / gate_rel.parent).mkdir(parents=True, exist_ok=True)
            (repo / doc_rel.parent).mkdir(parents=True, exist_ok=True)
            (repo / gate_rel).write_text(gate_src, encoding="utf-8")
            (repo / doc_rel).write_text(
                doc_src.replace("cargo clippy --workspace --all-targets", "workspace clippy", 1),
                encoding="utf-8",
            )
            label_errors = docs_lint.check_gate_docs_sync(repo)
            self.assertTrue(
                any("step 10 (clippy)" in err for err in label_errors),
                label_errors,
            )

            (repo / doc_rel).write_text(doc_src, encoding="utf-8")
            (repo / gate_rel).write_text(
                gate_src.replace('    ("doctest", "cargo test --doc --workspace"),\n', "", 1),
                encoding="utf-8",
            )
            count_errors = docs_lint.check_gate_docs_sync(repo)
            self.assertTrue(
                any("GATE_STEPS has 13" in err for err in count_errors),
                count_errors,
            )

    def test_markdown_discovery_ignores_venv_local_and_target_dirs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            for rel in (
                ".venv/lib/pkg/README.md",
                ".local/xtask-target/notes.md",
                "target/doc/README.md",
            ):
                ignored = repo / rel
                ignored.parent.mkdir(parents=True, exist_ok=True)
                ignored.write_text("# Ignored [broken](does-not-exist.md)\n", encoding="utf-8")

            result = self.run_lint(repo)

        self.assertEqual(result.returncode, 0, result.stdout)
