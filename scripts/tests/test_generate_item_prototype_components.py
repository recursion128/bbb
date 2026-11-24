from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location(
    "generate_item_prototype_components", SCRIPTS / "generate-item-prototype-components.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)
TABLE_SPEC = importlib.util.spec_from_file_location(
    "generate_item_prototype_components_table",
    SCRIPTS / "generate-item-prototype-components-table.py",
)
assert TABLE_SPEC is not None and TABLE_SPEC.loader is not None
TABLE_GENERATOR = importlib.util.module_from_spec(TABLE_SPEC)
TABLE_SPEC.loader.exec_module(TABLE_GENERATOR)
ROOT = SCRIPTS.parent


class GenerateItemPrototypeComponentsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.prototype_text = (
            ROOT / "crates/bbb-facts/data/item_prototype_components_26_1.json"
        ).read_text()
        prototype = json.loads(cls.prototype_text)
        lines = ["version=26.1\titems=1506\tcomponents=59\tmemberships=18981"]
        lines.extend(f"item\t{i}\t{name}" for i, name in enumerate(prototype["item_resource_ids"]))
        for component in prototype["component_values"]:
            for value in component["values"]:
                ids = [
                    str(i)
                    for first, last in value["item_id_ranges"]
                    for i in range(first, last + 1)
                ]
                lines.append(
                    "\t".join(
                        (
                            "component",
                            str(component["type_id"]),
                            component["resource_id"],
                            json.dumps(value["value"], ensure_ascii=False),
                            ",".join(ids),
                        )
                    )
                )
        cls.prototype_report = "\n".join(lines)
        cls.tag_text = (
            ROOT / "crates/bbb-facts/data/builtin_registration_tags_26_1.json"
        ).read_text()
        tags = json.loads(cls.tag_text)
        cls.tag_lines = [
            f"bootstrap_tags\tregistries={tags['registry_count']}\ttags={tags['tag_count']}"
        ]
        cls.tag_lines.extend(
            "\t".join(("bootstrap_registry", row["resource_id"], ",".join(row["tags"])))
            for row in tags["registries"]
        )
        cls.tag_report = "\n".join(cls.tag_lines)

    def test_old_prototype_json_remains_byte_identical_with_and_without_tag_report(self) -> None:
        for source in (self.prototype_report, self.prototype_report + "\n" + self.tag_report):
            prototype, tags = GENERATOR.split_reports(source)
            self.assertEqual(
                GENERATOR.render_catalog(GENERATOR.parse_facts(prototype)), self.prototype_text
            )
            if tags is not None:
                self.assertEqual(
                    GENERATOR.render_catalog(GENERATOR.parse_bootstrap_tags(tags)), self.tag_text
                )

    def test_complete_inventory_includes_empty_registries(self) -> None:
        artifact = GENERATOR.parse_bootstrap_tags(self.tag_report)
        self.assertEqual(artifact["registry_count"], 95)
        self.assertEqual(
            {row["resource_id"] for row in artifact["registries"]}, GENERATOR.builtin_registry_ids()
        )
        self.assertTrue(any(not row["tags"] for row in artifact["registries"]))
        self.assertEqual(
            artifact["tag_count"], sum(len(row["tags"]) for row in artifact["registries"])
        )

    def test_bad_headers_truncation_registry_duplicates_and_order_rejected(self) -> None:
        bad = ["", "version=26.1", "\n".join(self.tag_lines[:-1])]
        for replacement in (
            "bootstrap_tags\tregistries=94\ttags=0",
            "bootstrap_tags\tregistries=95\ttags=0",
        ):
            bad.append("\n".join([replacement, *self.tag_lines[1:]]))
        duplicate = self.tag_lines.copy()
        duplicate[2] = duplicate[1]
        bad.append("\n".join(duplicate))
        unordered = self.tag_lines.copy()
        unordered[1], unordered[2] = unordered[2], unordered[1]
        bad.append("\n".join(unordered))
        unknown = self.tag_lines.copy()
        unknown[1] = "bootstrap_registry\ttest:registry\t"
        bad.append("\n".join(unknown))
        for source in bad:
            with self.subTest(source=source[:100]), self.assertRaises(ValueError):
                GENERATOR.parse_bootstrap_tags(source)

    def test_tag_invalid_duplicate_order_and_duplicate_sections_rejected(self) -> None:
        for tags in ("Bad:Tag", "test:a,test:a", "test:z,test:a", "test:a,"):
            lines = self.tag_lines.copy()
            registry = lines[1].split("\t")[1]
            lines[1] = f"bootstrap_registry\t{registry}\t{tags}"
            with self.subTest(tags=tags), self.assertRaises(ValueError):
                GENERATOR.parse_bootstrap_tags("\n".join(lines))
        with self.assertRaisesRegex(ValueError, "duplicate bootstrap"):
            GENERATOR.split_reports(self.tag_report + "\n" + self.tag_report)

    def test_offline_write_check_and_missing_report_use_no_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report, prototype, tags = (
                root / name for name in ("report", "prototype.json", "tags.json")
            )
            report.write_text(self.prototype_report + "\n" + self.tag_report)
            args = [
                "generator",
                "--facts-input",
                str(report),
                "--output",
                str(prototype),
                "--bootstrap-tags-output",
                str(tags),
            ]
            with (
                patch.object(
                    GENERATOR, "resolve_runtime_inputs", side_effect=AssertionError("runtime")
                ),
                patch.object(GENERATOR, "runtime_facts", side_effect=AssertionError("runtime")),
            ):
                with patch.object(sys, "argv", args):
                    self.assertEqual(GENERATOR.main(), 0)
                with patch.object(sys, "argv", args + ["--check"]):
                    self.assertEqual(GENERATOR.main(), 0)
                self.assertEqual(prototype.read_text(), self.prototype_text)
                self.assertEqual(tags.read_text(), self.tag_text)
                tags.write_text("wrong")
                with (
                    patch.object(sys, "argv", args + ["--check"]),
                    contextlib.redirect_stdout(io.StringIO()) as output,
                ):
                    self.assertEqual(GENERATOR.main(), 1)
                self.assertIn("---", output.getvalue())
                self.assertEqual(tags.read_text(), "wrong")
                tags.write_text(self.tag_text)
                report.write_text(self.prototype_report)
                with (
                    patch.object(sys, "argv", args),
                    self.assertRaisesRegex(ValueError, "missing bootstrap"),
                ):
                    GENERATOR.main()
                self.assertEqual(tags.read_text(), self.tag_text)

    def test_static_rust_table_matches_checked_in_output(self) -> None:
        with patch.object(sys, "argv", ["table_generator", "--check"]):
            self.assertEqual(TABLE_GENERATOR.main(), 0)
        rendered = (
            ROOT / "crates/bbb-facts/src/item_prototype_components/generated_26_1.rs"
        ).read_text(encoding="utf-8")
        self.assertIn("@generated", rendered)


if __name__ == "__main__":
    unittest.main()
