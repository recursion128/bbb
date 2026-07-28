import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "coverage_status", REPO_ROOT / "scripts" / "coverage-status.py"
)
coverage_status = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(coverage_status)


class CoverageStatusTests(unittest.TestCase):
    def write_ledger(self, root: Path, name: str, entries: list[dict]) -> None:
        (root / "docs" / name).write_text(
            json.dumps({"entries": entries}), encoding="utf-8"
        )

    def make_repo_root(self, root: Path) -> Path:
        (root / "docs").mkdir()
        self.write_ledger(
            root,
            "packet-coverage-26.1.json",
            [
                {
                    "resource_id": "minecraft:closed",
                    "coverage": {
                        "decode": {"status": "complete"},
                        "tests": {"status": "not-applicable"},
                    },
                },
                {
                    "resource_id": "minecraft:open",
                    "coverage": {
                        "decode": {"status": "complete"},
                        "runtime_side_effect": {"status": "partial"},
                    },
                },
                {
                    "resource_id": "minecraft:deferred",
                    "coverage": {"decode": {"status": "deferred"}},
                },
            ],
        )
        self.write_ledger(
            root,
            "registry-holder-consumer-coverage-26.1.json",
            [
                {"constant_name": "FLAT_CLOSED", "status": "complete"},
                {"constant_name": "FLAT_ALSO_CLOSED", "status": "not-applicable"},
            ],
        )
        self.write_ledger(
            root,
            "item-component-coverage-26.1.json",
            [{"resource_id": "minecraft:no_status_fields"}],
        )
        self.write_ledger(
            root,
            "entity-coverage-26.1.json",
            [{"resource_id": "minecraft:pig", "coverage": {"status": "complete"}}],
        )
        (root / "docs" / "unsupported-features.md").write_text(
            "\n".join(
                [
                    "# Feature Gap TODO List",
                    "## Update Rules",
                    "# Todo",
                    "## First open domain",
                    "## Second open domain",
                    "# Deferred",
                    "## One deferred section",
                    "# Not needed",
                    "## One not-needed section",
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        return root

    def run_main(self, root: Path) -> str:
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            exit_code = coverage_status.main(["--repo-root", str(root)])
        self.assertEqual(exit_code, 0)
        return stdout.getvalue()

    def test_summarizes_ledgers_and_feature_domains(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = self.run_main(self.make_repo_root(Path(directory)))

        self.assertIn(
            "packet-coverage-26.1.json: 3 rows | closed 1 | deferred 1 | open 1",
            output,
        )
        self.assertIn(
            "open minecraft:open: coverage.runtime_side_effect:partial",
            output,
        )
        self.assertNotIn("minecraft:closed:", output)
        self.assertNotIn("minecraft:deferred:", output)
        self.assertIn(
            "registry-holder-consumer-coverage-26.1.json: 2 rows"
            " | closed 2 | deferred 0 | open 0 (no open rows)",
            output,
        )
        self.assertIn(
            "open minecraft:no_status_fields: (no status fields):?", output
        )
        self.assertIn(
            "unsupported-features.md: 2 todo domains"
            " | 1 deferred sections | 1 not-needed sections",
            output,
        )
        self.assertIn("  todo First open domain", output)
        self.assertIn("  todo Second open domain", output)
        self.assertNotIn("todo Update Rules", output)

    def test_real_repo_ledgers_parse(self) -> None:
        output = self.run_main(REPO_ROOT)

        for ledger in coverage_status.LEDGERS:
            self.assertIn(Path(ledger).name + ":", output)
        self.assertIn("unsupported-features.md:", output)


if __name__ == "__main__":
    unittest.main()
