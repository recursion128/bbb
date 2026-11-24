from __future__ import annotations

import contextlib
import importlib.util
import io
import re
import sys
import tempfile
import unittest
from pathlib import Path

import pytest

pytestmark = pytest.mark.vanilla

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location(
    "generate_server_block_sound_facts",
    SCRIPTS / "generate-server-block-sound-facts.py",
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)

from vanilla_java_source import parse_source  # noqa: E402

BLOCKS_OWNER = "net.minecraft.world.level.block.Blocks"
BLOCK_SET_TYPE_OWNER = (
    "net.minecraft.world.level.block.state.properties.BlockSetType"
)
WOOD_TYPE_OWNER = "net.minecraft.world.level.block.state.properties.WoodType"

SYNTHETIC_SOUND_EVENTS = """
package net.minecraft.sounds;
public class SoundEvents {
   public static final SoundEvent WOOD_PLACE = register("block.wood.place");
   public static final SoundEvent WOOD_BREAK = register("block.wood.break");
   public static final SoundEvent EMPTY_PLACE = register("intentionally_empty");
   private static SoundEvent register(final String id) {
      return null;
   }
}
"""

SYNTHETIC_SOUND_TYPES = """
package net.minecraft.world.level.block;
import net.minecraft.sounds.SoundEvents;
public class SoundType {
   public static final SoundType EMPTY = new SoundType(
      1.0F,
      1.0F,
      SoundEvents.EMPTY_PLACE,
      SoundEvents.EMPTY_PLACE,
      SoundEvents.EMPTY_PLACE,
      SoundEvents.EMPTY_PLACE,
      SoundEvents.EMPTY_PLACE
   );
   public static final SoundType WOOD = new SoundType(
      1.0F, 0.5F, SoundEvents.WOOD_BREAK, SoundEvents.EMPTY_PLACE, SoundEvents.WOOD_PLACE, SoundEvents.EMPTY_PLACE, SoundEvents.EMPTY_PLACE
   );
   public SoundType(
      final float volume,
      final float pitch,
      final SoundEvent breakSound,
      final SoundEvent stepSound,
      final SoundEvent placeSound,
      final SoundEvent hitSound,
      final SoundEvent fallSound
   ) {
   }
}
"""

SYNTHETIC_BLOCK_SET_TYPES = """
package net.minecraft.world.level.block.state.properties;
import net.minecraft.world.level.block.SoundType;
public record BlockSetType(
   String name, boolean canOpenByHand, boolean canOpenByWindCharge, boolean canButtonBeActivatedByArrows, String sensitivity, SoundType soundType
) {
   public static final BlockSetType OAK = register(new BlockSetType("oak"));
   public static final BlockSetType IRON = register(
      new BlockSetType("iron", false, false, false, "everything", SoundType.EMPTY)
   );
   private static BlockSetType register(final BlockSetType type) {
      return type;
   }
}
"""

SYNTHETIC_WOOD_TYPES = """
package net.minecraft.world.level.block.state.properties;
import net.minecraft.world.level.block.SoundType;
public record WoodType(String name, BlockSetType setType, SoundType soundType, SoundType hangingSignSoundType) {
   public static final WoodType OAK = register(new WoodType("oak", BlockSetType.OAK));
   public static final WoodType IRON = register(
      new WoodType("iron", BlockSetType.IRON, SoundType.EMPTY, SoundType.EMPTY)
   );
   public WoodType(final String name, final BlockSetType setType) {
      this(name, setType, SoundType.WOOD, SoundType.EMPTY);
   }
   private static WoodType register(final WoodType type) {
      return type;
   }
}
"""

WOOD_FAMILY_BLOCK = (
    "package net.minecraft.world.level.block;\n"
    "import net.minecraft.world.level.block.state.BlockBehaviour;\n"
    "import net.minecraft.world.level.block.state.properties.BlockSetType;\n"
    "public class DoorBlock {\n"
    "   public DoorBlock(final BlockSetType type, final BlockBehaviour.Properties properties) {\n"
    "      super(properties.sound(type.soundType()));\n"
    "   }\n"
    "}\n"
)


class SyntheticEnvironment:
    """A resolver over synthetic `Blocks.java` members and block classes."""

    def __init__(
        self,
        blocks_members: str,
        class_sources: dict[str, str] | None = None,
        factories: dict[str, str] | None = None,
    ) -> None:
        self._directory = tempfile.TemporaryDirectory(prefix="bbb-block-sound-")
        root = Path(self._directory.name)
        block_root = root / "net/minecraft/world/level/block"
        block_root.mkdir(parents=True)
        (block_root / "Blocks.java").write_text(
            "package net.minecraft.world.level.block;\n"
            "import net.minecraft.world.level.block.state.BlockBehaviour;\n"
            "import net.minecraft.world.level.block.state.properties.BlockSetType;\n"
            "import net.minecraft.world.level.block.state.properties.WoodType;\n"
            "public class Blocks {\n"
            "   static Block register(final String id, final BlockBehaviour.Properties properties) {\n"
            "      return null;\n"
            "   }\n"
            "   static Block register(final String id, final java.util.function.Function<BlockBehaviour.Properties, Block> factory, final BlockBehaviour.Properties properties) {\n"
            "      return null;\n"
            "   }\n"
            f"{blocks_members}\n"
            "}\n",
            encoding="utf-8",
        )
        for name, source in (class_sources or {}).items():
            (block_root / f"{name}.java").write_text(source, encoding="utf-8")
        blocks = parse_source(
            GENERATOR.BLOCKS_PATH, (block_root / "Blocks.java").read_bytes()
        )
        sound_types = GENERATOR.extract_sound_types(
            parse_source(GENERATOR.SOUND_TYPE_PATH, SYNTHETIC_SOUND_TYPES.encode()),
            parse_source(GENERATOR.SOUND_EVENTS_PATH, SYNTHETIC_SOUND_EVENTS.encode()),
        )
        block_sets = GENERATOR._record_constant_sound_types(
            parse_source(
                GENERATOR.BLOCK_SET_TYPE_PATH, SYNTHETIC_BLOCK_SET_TYPES.encode()
            ),
            BLOCK_SET_TYPE_OWNER,
            {GENERATOR.BLOCK_SET_TYPE_SOUND_INDEX: "soundType"},
        )
        wood_types = GENERATOR._record_constant_sound_types(
            parse_source(GENERATOR.WOOD_TYPE_PATH, SYNTHETIC_WOOD_TYPES.encode()),
            WOOD_TYPE_OWNER,
            {
                GENERATOR.WOOD_TYPE_SOUND_INDEX: "soundType",
                GENERATOR.WOOD_TYPE_HANGING_SIGN_SOUND_INDEX: "hangingSignSoundType",
            },
        )
        fields = {
            field.name: field
            for field in blocks.fields_for(BLOCKS_OWNER)
            if field.initializer is not None
        }
        self.resolver = GENERATOR.BlockSoundResolver(
            root,
            blocks,
            sound_types,
            block_sets,
            wood_types,
            fields,
            {name: factories.get(name, "Block") for name in fields}
            if factories
            else {name: "Block" for name in fields},
            {},
        )
        self.fields = fields


class GenerateServerBlockSoundFactsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.args = GENERATOR.parse_args([])
        cls.catalog = GENERATOR.extract(cls.args.source_root, cls.args.block_states)
        cls.generated = GENERATOR.render_catalog(cls.catalog)
        cls.sound_types = {row.name: row for row in cls.catalog.sound_types}
        cls.by_resource = {row.resource_id: row for row in cls.catalog.blocks}

    def test_sound_types_carry_vanillas_own_factors_and_events(self) -> None:
        self.assertEqual(len(self.catalog.sound_types), 123)
        empty = self.sound_types["EMPTY"]
        self.assertEqual(
            (empty.volume, empty.pitch, empty.events),
            ("1.0", "1.0", ("minecraft:intentionally_empty",) * 5),
        )
        anvil = self.sound_types["ANVIL"]
        self.assertEqual((anvil.volume, anvil.pitch), ("0.3", "1.0"))
        self.assertEqual(anvil.events[2], "minecraft:block.anvil.place")
        metal = self.sound_types["METAL"]
        self.assertEqual((metal.volume, metal.pitch), ("1.0", "1.5"))
        # A group whose five events come from more than one family.
        self.assertEqual(
            self.sound_types["LILY_PAD"].events,
            (
                "minecraft:block.big_dripleaf.break",
                "minecraft:block.big_dripleaf.step",
                "minecraft:block.lily_pad.place",
                "minecraft:block.big_dripleaf.hit",
                "minecraft:block.big_dripleaf.fall",
            ),
        )
        crop = self.sound_types["CROP"]
        self.assertEqual(
            crop.events,
            (
                "minecraft:block.crop.break",
                "minecraft:block.grass.step",
                "minecraft:item.crop.plant",
                "minecraft:block.grass.hit",
                "minecraft:block.grass.fall",
            ),
        )

    def test_hand_verified_blocks_answer_vanillas_own_group(self) -> None:
        expected = {
            # The `SoundType.STONE` default (`BlockBehaviour.java:973`).
            "minecraft:air": "STONE",
            "minecraft:stone": "STONE",
            # Groups that differ from the default, straight off `.sound(...)`.
            "minecraft:oak_planks": "WOOD",
            "minecraft:anvil": "ANVIL",
            "minecraft:deepslate": "DEEPSLATE",
            "minecraft:netherite_block": "NETHERITE_BLOCK",
            "minecraft:water": "EMPTY",
            # The sculk families the growth sound can land on
            # (`SculkBlock.java:68-74`).
            "minecraft:sculk": "SCULK",
            "minecraft:sculk_vein": "SCULK_VEIN",
            "minecraft:sculk_sensor": "SCULK_SENSOR",
            "minecraft:sculk_shrieker": "SCULK_SHRIEKER",
            "minecraft:sculk_catalyst": "SCULK_CATALYST",
            # `Blocks.java`'s own property helpers.
            "minecraft:oak_log": "WOOD",
            "minecraft:oak_leaves": "GRASS",
            "minecraft:crimson_stem": "STEM",
            "minecraft:candle": "CANDLE",
            "minecraft:wheat": "CROP",
            "minecraft:acacia_shelf": "SHELF",
            "minecraft:twisting_vines": "WEEPING_VINES",
            # `ofFullCopy`/`ofLegacyCopy` and the wall variant's copy.
            "minecraft:oak_stairs": "WOOD",
            "minecraft:oak_slab": "WOOD",
            "minecraft:cobblestone_wall": "STONE",
            # `properties.sound(type.soundType())` in the block constructor.
            "minecraft:oak_door": "WOOD",
            "minecraft:iron_door": "IRON",
            "minecraft:bamboo_door": "BAMBOO_WOOD",
            "minecraft:oak_pressure_plate": "WOOD",
            "minecraft:oak_button": "WOOD",
            "minecraft:oak_fence_gate": "WOOD",
            "minecraft:oak_sign": "WOOD",
            # `properties.sound(type.hangingSignSoundType())`.
            "minecraft:oak_hanging_sign": "HANGING_SIGN",
            "minecraft:cherry_hanging_sign": "CHERRY_WOOD_HANGING_SIGN",
        }
        self.assertEqual(
            {
                resource_id: self.by_resource[resource_id].sound_type
                for resource_id in expected
            },
            expected,
        )

    def test_every_declared_sound_assignment_agrees_with_the_source_text(self) -> None:
        """The AST walk against the plain text of every `.sound(...)` chain."""

        declarations = GENERATOR.extract_block_registry_catalog(
            parse_source(
                GENERATOR.BLOCKS_PATH,
                (self.args.source_root / GENERATOR.BLOCKS_PATH).read_bytes(),
            ),
            parse_source(
                GENERATOR.BLOCK_IDS_PATH,
                (self.args.source_root / GENERATOR.BLOCK_IDS_PATH).read_bytes(),
            ),
            parse_source(
                GENERATOR.BLOCK_ENTITY_TYPES_PATH,
                (
                    self.args.source_root / GENERATOR.BLOCK_ENTITY_TYPES_PATH
                ).read_bytes(),
            ),
        ).declarations
        source = (self.args.source_root / GENERATOR.BLOCKS_PATH).read_text()
        checked = 0
        for declaration in declarations:
            text = source[declaration["start_byte"] : declaration["end_byte"]]
            named = set(re.findall(r"\.sound\(SoundType\.([A-Z_0-9]+)\)", text))
            if len(named) != 1:
                continue
            checked += 1
            self.assertEqual(
                self.by_resource[declaration["resource_id"]].sound_type,
                named.pop(),
                declaration["resource_id"],
            )
        self.assertEqual(checked, 498)

    def test_every_block_resolves_and_the_ids_follow_the_registry(self) -> None:
        rows = self.catalog.blocks
        self.assertEqual(len(rows), 1168)
        self.assertEqual([row.block_type_id for row in rows], list(range(1168)))
        self.assertEqual(len({row.resource_id for row in rows}), 1168)
        for row in rows:
            self.assertIn(row.sound_type, self.sound_types, row.resource_id)
        # The declaration order `Blocks.java` writes is the registry order the
        # embedded state catalog lists.
        GENERATOR.assert_catalog_matches_runtime_order(
            self.catalog, self.args.block_states
        )
        self.assertEqual(
            sorted(
                {row.sound_type for row in rows}
                | {row.sound_type for row in self.catalog.overrides}
            ),
            sorted(set(self.sound_types) - {"TWISTING_VINES"}),
            "TWISTING_VINES is declared but no 26.1 block uses it",
        )

    def test_the_state_dependent_override_answers_the_cracked_states(self) -> None:
        self.assertEqual(
            self.by_resource["minecraft:decorated_pot"].sound_type, "DECORATED_POT"
        )
        self.assertEqual(len(self.catalog.overrides), 8)
        self.assertTrue(
            all(
                row.sound_type == "DECORATED_POT_CRACKED"
                for row in self.catalog.overrides
            )
        )
        self.assertEqual(
            [row.state_id for row in self.catalog.overrides],
            sorted(row.state_id for row in self.catalog.overrides),
        )

    def test_synthetic_registrations_resolve_each_way_vanilla_assigns_a_group(self) -> None:
        environment = SyntheticEnvironment(
            """
   public static final Block FIELD_DIRECT = register("direct", BlockBehaviour.Properties.of().sound(SoundType.WOOD));
   public static final Block FIELD_DEFAULT = register("default", BlockBehaviour.Properties.of());
   public static final Block FIELD_COPY = register("copy", BlockBehaviour.Properties.ofFullCopy(FIELD_DIRECT));
   public static final Block FIELD_LEGACY = register("legacy", BlockBehaviour.Properties.ofLegacyCopy(FIELD_DIRECT));
   public static final Block FIELD_COPIED_STAIR = registerStair("copied_stair", FIELD_COPY);
   static Block registerStair(final String id, final Block base) {
      return register(id, p -> new net.minecraft.world.level.block.StairBlock(p), BlockBehaviour.Properties.ofFullCopy(base));
   }
""",
            {"DoorBlock": WOOD_FAMILY_BLOCK},
        )
        resolver = environment.resolver
        self.assertIn("FIELD_DIRECT", environment.fields)
        self.assertEqual(resolver.sound_type_for_field("FIELD_DIRECT"), "WOOD")
        self.assertEqual(resolver.sound_type_for_field("FIELD_DEFAULT"), "STONE")
        self.assertEqual(resolver.sound_type_for_field("FIELD_COPY"), "WOOD")
        self.assertEqual(resolver.sound_type_for_field("FIELD_LEGACY"), "WOOD")
        # A helper whose copy source is its own parameter.
        self.assertEqual(resolver.sound_type_for_field("FIELD_COPIED_STAIR"), "WOOD")

    def test_synthetic_wood_family_registration_resolves_its_family_group(self) -> None:
        environment = SyntheticEnvironment(
            """
   public static final Block FIELD_OAK_DOOR = register("oak_door", p -> new DoorBlock(BlockSetType.OAK, p), BlockBehaviour.Properties.of());
   public static final Block FIELD_IRON_DOOR = register("iron_door", p -> new DoorBlock(BlockSetType.IRON, p), BlockBehaviour.Properties.of());
   public static final Block FIELD_NO_FAMILY = register("no_family", p -> new DoorBlock(null, p), BlockBehaviour.Properties.of());
""",
            {"DoorBlock": WOOD_FAMILY_BLOCK},
            {
                "FIELD_OAK_DOOR": "DoorBlock",
                "FIELD_IRON_DOOR": "DoorBlock",
                "FIELD_NO_FAMILY": "DoorBlock",
            },
        )
        self.assertEqual(
            environment.resolver.sound_type_for_field("FIELD_OAK_DOOR"), "WOOD"
        )
        self.assertEqual(
            environment.resolver.sound_type_for_field("FIELD_IRON_DOOR"), "EMPTY"
        )
        with self.assertRaisesRegex(
            GENERATOR.BlockSoundExtractionError, "hands it 0"
        ):
            environment.resolver.sound_type_for_field("FIELD_NO_FAMILY")

    def test_synthetic_unknown_shapes_are_refused(self) -> None:
        with self.assertRaisesRegex(
            GENERATOR.BlockSoundExtractionError, "unknown SoundEvents.MISSING"
        ):
            GENERATOR.extract_sound_types(
                parse_source(
                    GENERATOR.SOUND_TYPE_PATH,
                    SYNTHETIC_SOUND_TYPES.replace(
                        "SoundEvents.WOOD_PLACE", "SoundEvents.MISSING"
                    ).encode(),
                ),
                parse_source(
                    GENERATOR.SOUND_EVENTS_PATH, SYNTHETIC_SOUND_EVENTS.encode()
                ),
            )
        with self.assertRaisesRegex(GENERATOR.BlockSoundExtractionError, "not SoundEvents"):
            GENERATOR.extract_sound_types(
                parse_source(
                    GENERATOR.SOUND_TYPE_PATH,
                    SYNTHETIC_SOUND_TYPES.replace(
                        "SoundEvents.EMPTY_PLACE", "SoundType.EMPTY"
                    ).encode(),
                ),
                parse_source(
                    GENERATOR.SOUND_EVENTS_PATH, SYNTHETIC_SOUND_EVENTS.encode()
                ),
            )
        with self.assertRaisesRegex(
            GENERATOR.BlockSoundExtractionError, "is not register"
        ):
            GENERATOR._record_constant_sound_types(
                parse_source(
                    GENERATOR.BLOCK_SET_TYPE_PATH,
                    SYNTHETIC_BLOCK_SET_TYPES.replace(
                        'register(new BlockSetType("oak"))', "null"
                    ).encode(),
                ),
                BLOCK_SET_TYPE_OWNER,
                {GENERATOR.BLOCK_SET_TYPE_SOUND_INDEX: "soundType"},
            )

        environment = SyntheticEnvironment(
            """
   public static final Block FIELD_UNKNOWN_GROUP = register("unknown_group", BlockBehaviour.Properties.ofFullCopy(NOT_A_BLOCK));
   public static final Block FIELD_UNKNOWN_SOUND = register("unknown_sound", BlockBehaviour.Properties.of().sound(SoundType.MISSING));
"""
        )
        with self.assertRaisesRegex(
            GENERATOR.BlockSoundExtractionError, "unsupported properties copy source"
        ):
            environment.resolver.sound_type_for_field("FIELD_UNKNOWN_GROUP")
        with self.assertRaisesRegex(
            GENERATOR.BlockSoundExtractionError, "unknown SoundType.MISSING"
        ):
            environment.resolver.sound_type_for_field("FIELD_UNKNOWN_SOUND")

        catalog = GENERATOR.BlockSoundCatalog(
            sound_types=self.catalog.sound_types,
            blocks=tuple(
                GENERATOR.BlockSoundRow(
                    block_type_id=row.block_type_id,
                    resource_id=row.resource_id,
                    sound_type="NOT_A_SOUND_TYPE",
                )
                if row.resource_id == "minecraft:air"
                else row
                for row in self.catalog.blocks
            ),
            overrides=self.catalog.overrides,
        )
        with self.assertRaisesRegex(
            GENERATOR.BlockSoundExtractionError, "names NOT_A_SOUND_TYPE"
        ):
            GENERATOR.render_catalog(catalog)

    def test_a_state_override_that_is_not_one_boolean_is_refused(self) -> None:
        environment = SyntheticEnvironment(
            '   public static final Block FIELD_POT = register("pot", BlockBehaviour.Properties.of());\n',
            {
                "PotBlock": (
                    "package net.minecraft.world.level.block;\n"
                    "import net.minecraft.world.level.block.state.BlockState;\n"
                    "public class PotBlock {\n"
                    "   protected SoundType getSoundType(final BlockState state) {\n"
                    "      return SoundType.WOOD;\n"
                    "   }\n"
                    "}\n"
                ),
            },
        )
        self.assertTrue(environment.resolver.has_state_dependent_sound("PotBlock"))
        with self.assertRaisesRegex(
            GENERATOR.BlockSoundExtractionError, "not a conditional on one property"
        ):
            GENERATOR._state_split(environment.resolver, "PotBlock")

    def test_checked_in_output_is_exact_and_check_fails_closed(self) -> None:
        self.assertEqual(
            GENERATOR.DEFAULT_OUTPUT.read_text(encoding="utf-8"), self.generated
        )
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
                self.generated.replace(
                    "minecraft:oak_planks", "minecraft:not_oak_planks", 1
                ),
                encoding="utf-8",
            )
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(GENERATOR.main(arguments), 1)


if __name__ == "__main__":
    unittest.main()
