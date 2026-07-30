import os
from pathlib import Path
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
DOCS_LINT = REPO_ROOT / "scripts" / "docs-lint.sh"


ROADMAP_STATUSES = """\
<!-- roadmap-status:start -->
- p1-playable: maintenance
- p1-generated-facts: maintenance
- p2-non-entity: maintenance
- p2-hud: maintenance
- p3-resources: maintenance
- p4-delivery: maintenance
<!-- roadmap-status:end -->
"""


class DocsLintTests(unittest.TestCase):
    def make_repo(self, root: Path) -> Path:
        repo = root / "repo"
        (repo / "docs").mkdir(parents=True)
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

- A bounded gap is `deferred`. Reopen when its exact owner exists.

# Not needed

## Vanilla exclusions

- Vanilla `ExampleOwner.method` has no client behavior.
""",
            encoding="utf-8",
        )
        return repo

    def run_lint(self, repo: Path) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["BBB_DOCS_LINT_REPO_ROOT"] = str(repo)
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


if __name__ == "__main__":
    unittest.main()
