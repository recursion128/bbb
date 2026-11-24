from __future__ import annotations

import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path

import pytest

pytestmark = pytest.mark.vanilla

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

SPEC = importlib.util.spec_from_file_location(
    "generate_entity_default_attributes",
    SCRIPTS / "generate-entity-default-attributes.py",
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)


class GenerateEntityDefaultAttributesTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.args = GENERATOR.parse_args([])
        cls.rows = GENERATOR.extract(cls.args.source_root).entity_types
        cls.ids = GENERATOR.attribute_ids()
        cls.generated = GENERATOR.render_rows(cls.rows, cls.ids)
        cls.by_resource = {row.resource_id: row for row in cls.rows}

    def facts(self, resource_id: str) -> dict[int, int | None]:
        return {
            self.ids[constant]: override
            for constant, override in self.by_resource[
                resource_id
            ].attribute_base_overrides
        }

    def test_extraction_has_all_157_entity_rows_including_non_living(self) -> None:
        self.assertEqual(len(self.rows), 157)
        self.assertEqual(
            self.by_resource["minecraft:arrow"].attribute_base_overrides, ()
        )
        self.assertIn('("minecraft:arrow", &[\n    ])', self.generated)

    def test_known_blaze_breeze_and_skeleton_facts(self) -> None:
        for resource_id in (
            "minecraft:blaze",
            "minecraft:bogged",
            "minecraft:breeze",
            "minecraft:cave_spider",
            "minecraft:elder_guardian",
            "minecraft:giant",
            "minecraft:guardian",
            "minecraft:parched",
            "minecraft:silverfish",
            "minecraft:skeleton",
            "minecraft:spider",
            "minecraft:stray",
            "minecraft:wither_skeleton",
        ):
            self.assertTrue(self.by_resource[resource_id].attribute_base_overrides)

        blaze = self.facts("minecraft:blaze")
        self.assertEqual(blaze[self.ids["MOVEMENT_SPEED"]], 4597454643755220992)
        self.assertEqual(blaze[self.ids["FOLLOW_RANGE"]], 4631952216750555136)
        self.assertEqual(blaze[self.ids["ATTACK_DAMAGE"]], 4618441417868443648)

        breeze = self.facts("minecraft:breeze")
        self.assertEqual(breeze[self.ids["MAX_HEALTH"]], 4629137466983448576)
        self.assertEqual(breeze[self.ids["MOVEMENT_SPEED"]], 4603849755032813568)
        self.assertEqual(breeze[self.ids["ATTACK_DAMAGE"]], 4613937818241073152)

        skeleton = self.facts("minecraft:skeleton")
        self.assertEqual(
            skeleton[self.ids["MOVEMENT_SPEED"]], 4598175219545276416
        )
        self.assertEqual(skeleton[self.ids["FOLLOW_RANGE"]], 4625196817309499392)
        self.assertIsNone(skeleton[self.ids["ATTACK_DAMAGE"]])

    def test_checked_in_output_is_current_and_check_rejects_mutation(self) -> None:
        self.assertEqual(
            GENERATOR.DEFAULT_OUTPUT.read_text(encoding="utf-8"), self.generated
        )
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "generated.rs"
            output.write_text(self.generated, encoding="utf-8")
            self.assertEqual(
                GENERATOR.main(
                    [
                        "--vanilla-root",
                        str(self.args.source_root.parents[1]),
                        "--output",
                        str(output),
                        "--check",
                    ]
                ),
                0,
            )
            output.write_text(
                self.generated.replace("minecraft:blaze", "minecraft:mutated", 1),
                encoding="utf-8",
            )
            with contextlib.redirect_stderr(io.StringIO()):
                result = GENERATOR.main(
                    [
                        "--vanilla-root",
                        str(self.args.source_root.parents[1]),
                        "--output",
                        str(output),
                        "--check",
                    ]
                )
            self.assertEqual(result, 1)


if __name__ == "__main__":
    unittest.main()
