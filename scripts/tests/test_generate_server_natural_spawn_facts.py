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
    "generate_server_natural_spawn_facts",
    SCRIPTS / "generate-server-natural-spawn-facts.py",
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)

from entity_metadata_extractor import ExtractionError  # noqa: E402
from vanilla_java_source import parse_source  # noqa: E402


def placement_source(registrations: str):
    return parse_source(
        "net/minecraft/world/entity/SpawnPlacements.java",
        f"""
package net.minecraft.world.entity;
import net.minecraft.world.entity.animal.fish.WaterAnimal;
public class SpawnPlacements {{
    static {{
        {registrations}
    }}
}}
""".encode(),
    )


def category_source(first_constant: str = "MONSTER"):
    return parse_source(
        "net/minecraft/world/entity/MobCategory.java",
        f"""
package net.minecraft.world.entity;
public enum MobCategory {{
    {first_constant}("monster", 70, false, false, 128),
    CREATURE("creature", 10, true, true, 128),
    AMBIENT("ambient", 15, true, false, 128),
    AXOLOTLS("axolotls", 5, true, false, 128),
    UNDERGROUND_WATER_CREATURE("underground_water_creature", 5, true, false, 128),
    WATER_CREATURE("water_creature", 5, true, false, 128),
    WATER_AMBIENT("water_ambient", 20, true, false, 64),
    MISC("misc", -1, true, true, 128);
    private final int noDespawnDistance = 32;
    MobCategory(String name, int max, boolean friendly, boolean persistent, int distance) {{}}
}}
""".encode(),
    )


class GenerateServerNaturalSpawnFactsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.args = GENERATOR.parse_args([])
        cls.extraction = GENERATOR.extract(cls.args.source_root)
        cls.catalog = GENERATOR.build_catalog(cls.extraction)
        cls.generated = GENERATOR.render_catalog(cls.catalog)
        cls.by_resource = {row.resource_id: row for row in cls.catalog.entity_types}

    def test_mob_categories_have_exact_vanilla_values(self) -> None:
        self.assertEqual(
            [
                (
                    row.constant_name,
                    row.serialized_name,
                    row.max_instances_per_chunk,
                    row.friendly,
                    row.persistent,
                    row.no_despawn_distance,
                    row.despawn_distance,
                )
                for row in self.catalog.categories
            ],
            [
                ("MONSTER", "monster", 70, False, False, 32, 128),
                ("CREATURE", "creature", 10, True, True, 32, 128),
                ("AMBIENT", "ambient", 15, True, False, 32, 128),
                ("AXOLOTLS", "axolotls", 5, True, False, 32, 128),
                (
                    "UNDERGROUND_WATER_CREATURE",
                    "underground_water_creature",
                    5,
                    True,
                    False,
                    32,
                    128,
                ),
                ("WATER_CREATURE", "water_creature", 5, True, False, 32, 128),
                ("WATER_AMBIENT", "water_ambient", 20, True, False, 32, 64),
                ("MISC", "misc", -1, True, True, 32, 128),
            ],
        )

    def test_entity_rows_join_id_resource_category_and_runtime_class_exactly(self) -> None:
        rows = self.catalog.entity_types
        self.assertEqual(len(rows), 157)
        self.assertEqual([row.entity_type_id for row in rows], list(range(157)))
        self.assertEqual(len({row.constant_name for row in rows}), 157)
        self.assertEqual(len({row.resource_id for row in rows}), 157)
        self.assertEqual(
            self.by_resource["minecraft:player"].runtime_class,
            "net.minecraft.world.entity.player.Player",
        )
        self.assertEqual(self.by_resource["minecraft:zombie"].category_variant, "Monster")

    def test_registered_and_default_placements_keep_predicate_provenance(self) -> None:
        cod = self.by_resource["minecraft:cod"]
        self.assertEqual(
            (cod.placement, cod.heightmap, cod.predicate_kind),
            ("InWater", "MotionBlockingNoLeaves", "SurfaceWaterAnimal"),
        )
        self.assertEqual(
            (cod.predicate_owner, cod.predicate_method),
            (
                "net.minecraft.world.entity.animal.fish.WaterAnimal",
                "checkSurfaceWaterAnimalSpawnRules",
            ),
        )
        self.assertEqual(self.by_resource["minecraft:ocelot"].heightmap, "MotionBlocking")
        self.assertEqual(self.by_resource["minecraft:strider"].placement, "InLava")
        allay = self.by_resource["minecraft:allay"]
        self.assertEqual(
            (allay.placement, allay.heightmap, allay.predicate_kind),
            ("NoRestrictions", "MotionBlockingNoLeaves", "Unrestricted"),
        )
        self.assertEqual((allay.predicate_owner, allay.predicate_method), (None, None))

    def test_mob_obstruction_owners_have_base_and_overrides(self) -> None:
        self.assertEqual(sum(row.is_mob for row in self.catalog.entity_types), 89)
        owners = {
            row.check_spawn_obstruction_owner
            for row in self.catalog.entity_types
            if row.check_spawn_obstruction_owner is not None
        }
        self.assertIn(GENERATOR.MOB, owners)
        self.assertGreater(len(owners), 1)
        for resource_id in ("minecraft:player", "minecraft:arrow"):
            self.assertFalse(self.by_resource[resource_id].is_mob)
            self.assertIsNone(self.by_resource[resource_id].check_spawn_obstruction_owner)

    def test_delegated_virtual_owners_follow_the_runtime_class(self) -> None:
        self.assertEqual(
            self.by_resource["minecraft:zombie"].walk_target_value_owner,
            "net.minecraft.world.entity.monster.Monster",
        )
        self.assertEqual(
            self.by_resource["minecraft:bee"].walk_target_value_owner,
            "net.minecraft.world.entity.animal.bee.Bee",
        )
        self.assertIsNone(self.by_resource["minecraft:bat"].walk_target_value_owner)

    def test_synthetic_duplicate_unknown_and_count_drift_are_refused(self) -> None:
        valid = (
            "register(EntityType.COD, SpawnPlacementTypes.IN_WATER, "
            "Heightmap.Types.MOTION_BLOCKING_NO_LEAVES, "
            "WaterAnimal::checkSurfaceWaterAnimalSpawnRules);"
        )
        with self.assertRaisesRegex(GENERATOR.NaturalSpawnExtractionError, "duplicate placement"):
            GENERATOR.spawn_placement_facts(
                placement_source(valid + valid), {"COD"}, expected_count=1
            )
        unknown_predicate = valid.replace("checkSurfaceWaterAnimalSpawnRules", "unknownSpawnRule")
        with self.assertRaisesRegex(
            GENERATOR.NaturalSpawnExtractionError, "unknown spawn predicate"
        ):
            GENERATOR.spawn_placement_facts(
                placement_source(unknown_predicate), {"COD"}, expected_count=1
            )
        with self.assertRaisesRegex(GENERATOR.NaturalSpawnExtractionError, "unknown entity type"):
            GENERATOR.spawn_placement_facts(placement_source(valid), {"AXOLOTL"}, expected_count=1)
        with self.assertRaisesRegex(
            GENERATOR.NaturalSpawnExtractionError, "unknown spawn placement"
        ):
            GENERATOR.spawn_placement_facts(
                placement_source(valid.replace("IN_WATER", "IN_AIR")),
                {"COD"},
                expected_count=1,
            )
        with self.assertRaisesRegex(
            GENERATOR.NaturalSpawnExtractionError, "unknown spawn heightmap"
        ):
            GENERATOR.spawn_placement_facts(
                placement_source(valid.replace("MOTION_BLOCKING_NO_LEAVES", "WORLD_SURFACE")),
                {"COD"},
                expected_count=1,
            )
        with self.assertRaisesRegex(GENERATOR.NaturalSpawnExtractionError, "expected 2 placements"):
            GENERATOR.spawn_placement_facts(placement_source(valid), {"COD"}, expected_count=2)
        with self.assertRaisesRegex(GENERATOR.NaturalSpawnExtractionError, "unknown mob category"):
            GENERATOR.mob_category_facts(category_source("FANTASY"))

    def test_missing_mob_virtual_declaration_is_refused(self) -> None:
        class_map = {item.fqcn: item for item in self.extraction.classes}
        with self.assertRaisesRegex(ExtractionError, "no definitelyMissing/0 declaration"):
            GENERATOR.class_nearest_method_owner(
                self.extraction,
                class_map,
                GENERATOR.MOB,
                "definitelyMissing",
                0,
                {},
            )

    def test_checked_in_output_is_exact_and_check_fails_closed(self) -> None:
        self.assertEqual(GENERATOR.DEFAULT_OUTPUT.read_text(encoding="utf-8"), self.generated)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "facts.rs"
            output.write_text(self.generated, encoding="utf-8")
            arguments = [
                "--vanilla-root",
                str(self.args.source_root.parents[1]),
                "--output",
                str(output),
                "--check",
            ]
            self.assertEqual(GENERATOR.main(arguments), 0)
            output.write_text(
                self.generated.replace("minecraft:allay", "minecraft:not_allay", 1),
                encoding="utf-8",
            )
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(GENERATOR.main(arguments), 1)


if __name__ == "__main__":
    unittest.main()
