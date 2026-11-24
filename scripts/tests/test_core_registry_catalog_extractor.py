from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from core_registry_catalog_extractor import (  # noqa: E402
    AnimalVariantComponentRegistryEntry,
    CoreRegistryExtractionError,
    VanillaParticleCodecFamily,
    activity_entries,
    animal_variant_component_registry_entries,
    animal_variant_texture_registry_entries,
    block_entity_menu_provider_classes,
    block_entity_type_entries,
    common_item_component_constant_names,
    damage_type_entries,
    data_component_predicate_type_entries,
    data_component_type_entries,
    entity_type_entries,
    entity_type_track_delta_exclusions,
    mob_category_entries,
    note_block_instrument_entries,
    note_block_instrument_type_names,
    number_format_type_entries,
    particle_type_entries,
    poi_type_entries,
    vec3_constant,
)
from vanilla_java_source import parse_source  # noqa: E402


class BlockEntityMenuProviderTests(unittest.TestCase):
    def test_menu_provider_membership_closes_over_the_superclass_chain(self) -> None:
        source = parse_source(
            "fixture/BlockEntities.java",
            b"""
package fixture;
public abstract class BaseContainerBlockEntity extends BlockEntity implements Container, MenuProvider {
}
public class FurnaceBlockEntity extends BaseContainerBlockEntity {
}
public class SignBlockEntity extends BlockEntity {
}
public class LecternBlockEntity extends BlockEntity implements Clearable, net.minecraft.world.MenuProvider {
}
""",
        )
        classes = block_entity_menu_provider_classes([source])
        self.assertIn("BaseContainerBlockEntity", classes)
        self.assertIn("FurnaceBlockEntity", classes)
        self.assertIn("LecternBlockEntity", classes)
        self.assertNotIn("SignBlockEntity", classes)


class CoreRegistryCatalogExtractorTests(unittest.TestCase):
    TRACK_DELTA_EXCLUSIONS = (
        "PLAYER",
        "LLAMA_SPIT",
        "WITHER",
        "BAT",
        "ITEM_FRAME",
        "GLOW_ITEM_FRAME",
        "LEASH_KNOT",
        "PAINTING",
        "END_CRYSTAL",
        "EVOKER_FANGS",
    )

    def _track_delta_source(self, predicate: str, *, body_suffix: str = ""):
        fields = "\n".join(
            f"  public static final EntityType<?> {constant} = null;"
            for constant in self.TRACK_DELTA_EXCLUSIONS
        )
        return parse_source(
            "fixture/EntityType.java",
            f"""
package fixture;
public class EntityType {{
{fields}
  public boolean trackDeltas() {{
    return {predicate};
    {body_suffix}
  }}
}}
""".encode(),
        )

    def test_track_delta_exclusions_come_from_the_exact_predicate(self) -> None:
        predicate = " && ".join(
            f"this != {constant}" for constant in self.TRACK_DELTA_EXCLUSIONS
        )
        self.assertEqual(
            entity_type_track_delta_exclusions(self._track_delta_source(predicate)),
            frozenset(self.TRACK_DELTA_EXCLUSIONS),
        )

    def test_track_delta_exclusions_reject_a_dynamic_predicate(self) -> None:
        predicate = " && ".join(
            [
                *(f"this != {constant}" for constant in self.TRACK_DELTA_EXCLUSIONS[:-1]),
                "shouldTrackDeltas()",
            ]
        )
        with self.assertRaises(CoreRegistryExtractionError) as caught:
            entity_type_track_delta_exclusions(self._track_delta_source(predicate))
        self.assertIn("fixture/EntityType.java:", str(caught.exception))
        self.assertIn("this != CONSTANT", str(caught.exception))

    def test_track_delta_exclusions_reject_an_unknown_constant(self) -> None:
        constants = (*self.TRACK_DELTA_EXCLUSIONS[:-1], "UNKNOWN")
        predicate = " && ".join(f"this != {constant}" for constant in constants)
        with self.assertRaises(CoreRegistryExtractionError) as caught:
            entity_type_track_delta_exclusions(self._track_delta_source(predicate))
        self.assertIn("fixture/EntityType.java:", str(caught.exception))
        self.assertIn("unknown entity type UNKNOWN", str(caught.exception))

    def test_track_delta_exclusions_reject_duplicates(self) -> None:
        constants = (*self.TRACK_DELTA_EXCLUSIONS[:-1], "PLAYER")
        predicate = " && ".join(f"this != {constant}" for constant in constants)
        with self.assertRaises(CoreRegistryExtractionError) as caught:
            entity_type_track_delta_exclusions(self._track_delta_source(predicate))
        self.assertIn("repeats entity type constant PLAYER", str(caught.exception))

    def test_track_delta_exclusions_require_ten_constants(self) -> None:
        predicate = " && ".join(
            f"this != {constant}" for constant in self.TRACK_DELTA_EXCLUSIONS[:-1]
        )
        with self.assertRaises(CoreRegistryExtractionError) as caught:
            entity_type_track_delta_exclusions(self._track_delta_source(predicate))
        self.assertIn("expected 10", str(caught.exception))

    def test_track_delta_exclusions_require_a_single_return(self) -> None:
        predicate = " && ".join(
            f"this != {constant}" for constant in self.TRACK_DELTA_EXCLUSIONS
        )
        with self.assertRaises(CoreRegistryExtractionError) as caught:
            entity_type_track_delta_exclusions(
                self._track_delta_source(predicate, body_suffix="afterReturn();")
            )
        self.assertIn("body is not a single return", str(caught.exception))

    def test_mob_categories_preserve_enum_order_and_serialized_names(self) -> None:
        source = parse_source(
            "fixture/MobCategory.java",
            b"""
package fixture;
public enum MobCategory implements StringRepresentable {
  MONSTER("monster", 70), WATER_CREATURE("water_creature", 5), MISC("misc", -1);
  MobCategory(String name, int max) {}
}
""",
        )
        self.assertEqual(
            mob_category_entries(source, expected_count=3),
            [
                ("MONSTER", "monster"),
                ("WATER_CREATURE", "water_creature"),
                ("MISC", "misc"),
            ],
        )

    def test_poi_matching_states_include_sets_and_only_bed_heads(self) -> None:
        body = b"""
package fixture;
public class PoiTypes {
  public static final ResourceKey<PoiType> HOME = createKey("home");
  public static final ResourceKey<PoiType> MEETING = createKey("meeting");
  private static final Set<BlockState> BEDS = ImmutableList.of(Blocks.RED_BED, Blocks.BLUE_BED)
    .stream().flatMap(block -> block.getStateDefinition().getPossibleStates().stream())
    .filter(state -> state.getValue(BedBlock.PART) == BedPart.HEAD).collect(ImmutableSet.toImmutableSet());
  private static Set<BlockState> getBlockStates(Block block) {
    return ImmutableSet.copyOf(block.getStateDefinition().getPossibleStates());
  }
  public static PoiType bootstrap(Registry<PoiType> registry) {
    register(registry, HOME, BEDS, 1, 1);
    return register(registry, MEETING, getBlockStates(Blocks.BELL), 32, 6);
  }
}
"""
        def extract(value: bytes):
            return poi_type_entries(parse_source("fixture/PoiTypes.java", value), expected_count=2, include_states=True)
        entries = extract(body)
        self.assertEqual(entries[0].matching_blocks, ("RED_BED", "BLUE_BED"))
        self.assertTrue(entries[0].head_only)
        self.assertEqual(entries[1].matching_blocks, ("BELL",))
        self.assertFalse(entries[1].head_only)
        without_filter = body.replace(b".filter(state -> state.getValue(BedBlock.PART) == BedPart.HEAD)", b"")
        self.assertFalse(extract(without_filter)[0].head_only)
        for bad in [
            body.replace(b"BedPart.HEAD", b"BedPart.FOOT"),
            body.replace(b".collect(ImmutableSet.toImmutableSet())", b".limit(1).collect(ImmutableSet.toImmutableSet())"),
            body.replace(b"Blocks.BLUE_BED", b"Blocks.RED_BED"),
            body.replace(b"return ImmutableSet.copyOf(block.getStateDefinition().getPossibleStates());", b"return Set.of(block.defaultBlockState());"),
        ]:
            with self.subTest(source=bad):
                with self.assertRaises(CoreRegistryExtractionError):
                    extract(bad)

    def test_poi_types_preserve_bootstrap_order_and_limits(self) -> None:
        source = parse_source(
            "fixture/PoiTypes.java",
            b"""
package fixture;
public class PoiTypes {
  public static final ResourceKey<PoiType> HOME = createKey("home");
  public static final ResourceKey<PoiType> MEETING = createKey("meeting");
  public static PoiType bootstrap(Registry<PoiType> registry) {
    register(registry, HOME, states(), 1, 1);
    return register(registry, MEETING, states(), 32, 6);
  }
}
""",
        )
        entries = poi_type_entries(source, expected_count=2)
        self.assertEqual(
            [
                (
                    entry.constant_name,
                    entry.resource_id,
                    entry.max_tickets,
                    entry.valid_range,
                )
                for entry in entries
            ],
            [
                ("HOME", "minecraft:home", 1, 1),
                ("MEETING", "minecraft:meeting", 32, 6),
            ],
        )

    def test_number_format_types_follow_bootstrap_registration_order(self) -> None:
        source = parse_source(
            "fixture/NumberFormatTypes.java",
            b"""
package fixture;
public class NumberFormatTypes {
  public static NumberFormatType<?> bootstrap(Registry<NumberFormatType<?>> registry) {
    Registry.register(registry, "blank", BlankFormat.TYPE);
    Registry.register(registry, "styled", StyledFormat.TYPE);
    return Registry.register(registry, "fixed", FixedFormat.TYPE);
  }
}
""",
        )
        entries = number_format_type_entries(source)
        self.assertEqual(
            [(entry.constant_name, entry.resource_id) for entry in entries],
            [
                ("BLANK", "minecraft:blank"),
                ("STYLED", "minecraft:styled"),
                ("FIXED", "minecraft:fixed"),
            ],
        )

    def test_animal_variant_components_join_holder_types_to_registry_keys(self) -> None:
        component_source = parse_source(
            "fixture/DataComponents.java",
            b"""
package fixture;
import net.minecraft.world.entity.animal.wolf.WolfSoundVariant;
import net.minecraft.world.entity.animal.wolf.WolfVariant;
import net.minecraft.world.entity.decoration.painting.PaintingVariant;
public class DataComponents {
  public static final DataComponentType<Holder<WolfVariant>> WOLF_VARIANT = register(
      "wolf/variant", b -> b.persistent(WolfVariant.CODEC));
  public static final DataComponentType<Holder<WolfSoundVariant>> WOLF_SOUND_VARIANT = register(
      "wolf/sound_variant", b -> b.persistent(WolfSoundVariant.CODEC));
  public static final DataComponentType<Holder<PaintingVariant>> PAINTING_VARIANT = register(
      "painting/variant", b -> b.persistent(PaintingVariant.CODEC));
}
""",
        )
        registries_source = parse_source(
            "fixture/Registries.java",
            b"""
package fixture;
import net.minecraft.world.entity.animal.wolf.WolfSoundVariant;
import net.minecraft.world.entity.animal.wolf.WolfVariant;
import net.minecraft.world.entity.decoration.painting.PaintingVariant;
public class Registries {
  public static final ResourceKey<Registry<WolfVariant>> WOLF_VARIANT =
      createRegistryKey("wolf_variant");
  public static final ResourceKey<Registry<WolfSoundVariant>> WOLF_SOUND_VARIANT =
      createRegistryKey("wolf_sound_variant");
  public static final ResourceKey<Registry<PaintingVariant>> PAINTING_VARIANT =
      createRegistryKey("painting_variant");
}
""",
        )

        entries = animal_variant_component_registry_entries(
            component_source, registries_source, expected_count=2
        )
        self.assertEqual(
            entries,
            [
                AnimalVariantComponentRegistryEntry(
                    "WOLF_VARIANT",
                    "WOLF_VARIANT",
                    "minecraft:wolf_variant",
                    "net.minecraft.world.entity.animal.wolf.WolfVariant",
                ),
                AnimalVariantComponentRegistryEntry(
                    "WOLF_SOUND_VARIANT",
                    "WOLF_SOUND_VARIANT",
                    "minecraft:wolf_sound_variant",
                    "net.minecraft.world.entity.animal.wolf.WolfSoundVariant",
                ),
            ],
        )

        wolf_variant_source = parse_source(
            "net/minecraft/world/entity/animal/wolf/WolfVariant.java",
            b"""
package net.minecraft.world.entity.animal.wolf;
import net.minecraft.core.ClientAsset;
public record WolfVariant() {
  public static final Codec<WolfVariant> NETWORK_CODEC = RecordCodecBuilder.create(
      i -> i.group(WolfVariant.AssetInfo.CODEC.fieldOf("assets")));
  public record AssetInfo() {
    public static final Codec<WolfVariant.AssetInfo> CODEC = RecordCodecBuilder.create(
        i -> i.group(ClientAsset.ResourceTexture.CODEC.fieldOf("wild")));
  }
}
""",
        )
        wolf_sound_variant_source = parse_source(
            "net/minecraft/world/entity/animal/wolf/WolfSoundVariant.java",
            b"""
package net.minecraft.world.entity.animal.wolf;
import net.minecraft.sounds.SoundEvent;
public record WolfSoundVariant() {
  public static final Codec<WolfSoundVariant> NETWORK_CODEC = codec();
  private static Codec<WolfSoundVariant> codec() {
    return SoundEvent.CODEC.fieldOf("death_sound").codec();
  }
}
""",
        )
        self.assertEqual(
            animal_variant_texture_registry_entries(
                entries,
                {
                    "net.minecraft.world.entity.animal.wolf.WolfVariant": wolf_variant_source,
                    "net.minecraft.world.entity.animal.wolf.WolfSoundVariant": (
                        wolf_sound_variant_source
                    ),
                },
                expected_count=1,
            ),
            [entries[0]],
        )

    def test_damage_keys_follow_bootstrap_call_order(self) -> None:
        source = parse_source(
            "fixture/DamageTypes.java",
            b"""
package fixture;
public interface DamageTypes {
  ResourceKey<DamageType> FIRST = ResourceKey.create(
      Registries.DAMAGE_TYPE, Identifier.withDefaultNamespace("first"));
  ResourceKey<DamageType> SECOND = ResourceKey.create(
      Registries.DAMAGE_TYPE, Identifier.withDefaultNamespace("second"));
  static void bootstrap(Context context) {
    // context.register(FAKE, fake());
    context.register(SECOND, value()); context.register(FIRST, value());
  }
}
""",
        )
        self.assertEqual(
            [(entry.constant_name, entry.resource_id) for entry in damage_type_entries(source)],
            [("SECOND", "minecraft:second"), ("FIRST", "minecraft:first")],
        )

    def test_damage_effects_come_from_constructor_arguments(self) -> None:
        source = parse_source(
            "fixture/DamageTypes.java",
            b"""
package fixture;
public interface DamageTypes {
  ResourceKey<DamageType> FIRE = ResourceKey.create(
      Registries.DAMAGE_TYPE, Identifier.withDefaultNamespace("fire"));
  ResourceKey<DamageType> GENERIC = ResourceKey.create(
      Registries.DAMAGE_TYPE, Identifier.withDefaultNamespace("generic"));
  static void bootstrap(Context context) {
    context.register(FIRE, new DamageType("fire", 0.1F, DamageEffects.BURNING));
    context.register(GENERIC, new DamageType("generic", 0.0F));
  }
}
""",
        )
        self.assertEqual(
            [entry.damage_effect for entry in damage_type_entries(source)],
            ["burning", "hurt"],
        )

    def test_entity_builder_property_comes_from_initializer_calls(self) -> None:
        source = parse_source(
            "fixture/EntityType.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new, MobCategory.CREATURE).sized(1, 1).eyeHeight(1.25F).nameTagOffset(1.5F).passengerAttachments(0.5F).ridingOffset(-0.25F));
  public static final EntityType<Two> TWO = register(
      "two", Builder.of(Two::new, MobCategory.MONSTER)
          .sized(0.6F, 1.8F).passengerAttachments(new Vec3(0.25, 1.0, -0.5))
          .vehicleAttachment(Avatar.DEFAULT_VEHICLE_ATTACHMENT).notInPeaceful().noSave().noLootTable());
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of(TWO);
  private static String fake = ".notInPeaceful() register(\\\"fake\\\")";
}
""",
        )
        entries = entity_type_entries(
            source,
            expected_count=2,
            named_vec3_constants={"Avatar.DEFAULT_VEHICLE_ATTACHMENT": (0.0, 0.6, 0.0)},
        )
        self.assertEqual([entry.resource_id for entry in entries], ["minecraft:one", "minecraft:two"])
        self.assertEqual(
            [entry.default_loot_table for entry in entries],
            ["minecraft:entities/one", None],
        )
        self.assertEqual([entry.mob_category for entry in entries], ["CREATURE", "MONSTER"])
        self.assertEqual([entry.allowed_in_peaceful for entry in entries], [True, False])
        self.assertEqual([entry.only_op_can_set_nbt for entry in entries], [False, True])
        self.assertEqual([entry.serializable for entry in entries], [True, False])
        self.assertEqual([(entry.width, entry.height) for entry in entries], [(1.0, 1.0), (0.6, 1.8)])
        self.assertEqual([entry.eye_height for entry in entries], [1.25, None])
        self.assertEqual([entry.name_tag_offset for entry in entries], [1.5, None])
        self.assertEqual(
            [entry.passenger_attachments for entry in entries],
            [((0.0, 0.5, 0.0),), ((0.25, 1.0, -0.5),)],
        )
        self.assertEqual(
            [entry.vehicle_attachment for entry in entries],
            [(0.0, 0.25, 0.0), (0.0, 0.6, 0.0)],
        )

    def test_entity_tracking_facts_fall_back_to_builder_defaults(self) -> None:
        source = parse_source(
            "fixture/EntityType.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new, MobCategory.WATER_CREATURE).sized(1, 1));
  public static final EntityType<Two> TWO = register(
      "two", Builder.of(Two::new, MobCategory.MONSTER).sized(1, 1)
          .clientTrackingRange(16).updateInterval(Integer.MAX_VALUE).canSpawnFarFromPlayer());
  public static final EntityType<Three> THREE = register(
      "three", Builder.of(Three::new, MobCategory.MISC).sized(1, 1).clientTrackingRange(0));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        entries = entity_type_entries(source, expected_count=3)
        # EntityType.java:1615 and EntityType.java:1616 default the pair to 5 chunks
        # and 3 ticks; EntityType.java:1630 seeds canSpawnFarFromPlayer per category.
        self.assertEqual(
            [entry.client_tracking_range for entry in entries], [5, 16, 0]
        )
        self.assertEqual(
            [entry.update_interval for entry in entries], [3, 2**31 - 1, 3]
        )
        self.assertEqual(
            [entry.can_spawn_far_from_player for entry in entries],
            [False, True, True],
        )

    def test_entity_tracking_range_rejects_non_literal_builder_values(self) -> None:
        source = parse_source(
            "fixture/DynamicEntityTrackingRange.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new, MobCategory.MISC).sized(1, 1)
          .clientTrackingRange(DISPLAY_TRACKING_RANGE));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"DynamicEntityTrackingRange\.java:\d+:\d+: clientTrackingRange is not an integer literal",
        ):
            entity_type_entries(source, expected_count=1)

    def test_entity_update_interval_rejects_non_positive_values(self) -> None:
        source = parse_source(
            "fixture/ZeroEntityUpdateInterval.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new, MobCategory.MISC).sized(1, 1).updateInterval(0));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"ZeroEntityUpdateInterval\.java:\d+:\d+: updateInterval is not positive",
        ):
            entity_type_entries(source, expected_count=1)

    def test_entity_builder_category_supports_create_nothing(self) -> None:
        source = parse_source(
            "fixture/EntityType.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<Player> PLAYER = register(
      "player", Builder.<Player>createNothing(MobCategory.MISC).sized(0.6F, 1.8F));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        entries = entity_type_entries(source, expected_count=1)
        self.assertEqual(entries[0].mob_category, "MISC")

    def test_vec3_constant_comes_from_literal_constructor(self) -> None:
        source = parse_source(
            "fixture/Avatar.java",
            b"""
package fixture;
public abstract class Avatar {
  public static final Vec3 DEFAULT_VEHICLE_ATTACHMENT = new Vec3(0.0, 0.6, -0.125F);
}
""",
        )
        self.assertEqual(
            vec3_constant(source, "DEFAULT_VEHICLE_ATTACHMENT"),
            (0.0, 0.6, -0.125),
        )

    def test_entity_eye_height_rejects_non_literal_builder_values(self) -> None:
        source = parse_source(
            "fixture/DynamicEntityEyeHeight.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new, MobCategory.MISC).sized(1, 1).eyeHeight(DEFAULT_EYE_HEIGHT));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"DynamicEntityEyeHeight\.java:\d+:\d+: eyeHeight is not a numeric literal",
        ):
            entity_type_entries(source, expected_count=1)

    def test_entity_name_tag_offset_rejects_non_literal_builder_values(self) -> None:
        source = parse_source(
            "fixture/DynamicEntityNameTagOffset.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new, MobCategory.MISC).sized(1, 1).nameTagOffset(DEFAULT_NAME_TAG_OFFSET));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"DynamicEntityNameTagOffset\.java:\d+:\d+: nameTagOffset is not a numeric literal",
        ):
            entity_type_entries(source, expected_count=1)

    def test_entity_dimensions_reject_non_literal_builder_values(self) -> None:
        source = parse_source(
            "fixture/DynamicEntityDimensions.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new, MobCategory.MISC).sized(DEFAULT_WIDTH, 1));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"DynamicEntityDimensions\.java:\d+:\d+: entity width is not a numeric literal",
        ):
            entity_type_entries(source, expected_count=1)

    def test_entity_passenger_attachment_rejects_non_literal_coordinates(self) -> None:
        source = parse_source(
            "fixture/DynamicPassengerAttachment.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new, MobCategory.MISC).sized(1, 1)
          .passengerAttachments(new Vec3(0, DEFAULT_Y, 0)));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"DynamicPassengerAttachment\.java:\d+:\d+: passenger attachment coordinate is not a numeric literal",
        ):
            entity_type_entries(source, expected_count=1)

    def test_other_registry_field_shapes_use_cst_arguments_and_builder_calls(self) -> None:
        block_source = parse_source(
            "fixture/BlockEntityType.java",
            b"""
package fixture;
public class BlockEntityType {
  public static final BlockEntityType<One> ONE = register(
      "one", One::new, Blocks.ONE, Blocks.OTHER);
  private static final Set<BlockEntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of(ONE);
}
""",
        )
        predicate_source = parse_source(
            "fixture/DataComponentPredicates.java",
            b"""
package fixture;
public class DataComponentPredicates {
  public static final DataComponentPredicate.Type<One> ONE = register(
      "one", One.CODEC);
}
""",
        )
        component_source = parse_source(
            "fixture/DataComponents.java",
            b"""
package fixture;
public class DataComponents {
  public static final DataComponentType<One> ONE = register(
      "one", builder -> builder.persistent(One.CODEC).cacheEncoding());
  public static final DataComponentType<Two> TWO = register(
      "two", builder -> builder.networkSynchronized(Two.STREAM_CODEC).ignoreSwapAnimation());
  public static final DataComponentMap COMMON_ITEM_COMPONENTS = DataComponentMap.builder()
      .set(TWO, Two.DEFAULT)
      .set(ONE, One.DEFAULT)
      .build();
}
""",
        )
        particle_source = parse_source(
            "fixture/ParticleTypes.java",
            b"""
package fixture;
public class ParticleTypes {
  public static final SimpleParticleType ONE = register("one", true);
  public static final ParticleType<BlockParticleOption> TWO = register(
      "two", false, BlockParticleOption::codec, BlockParticleOption::streamCodec);
}
""",
        )
        block_entities = block_entity_type_entries(
            block_source,
            frozenset({"One"}),
            expected_count=1,
            expected_menu_provider_count=1,
        )
        self.assertEqual(block_entities[0].resource_id, "minecraft:one")
        self.assertTrue(block_entities[0].only_op_can_set_nbt)
        self.assertTrue(block_entities[0].provides_menu)
        self.assertFalse(
            block_entity_type_entries(
                block_source,
                frozenset(),
                expected_count=1,
                expected_menu_provider_count=0,
            )[0].provides_menu
        )
        self.assertEqual(
            data_component_predicate_type_entries(predicate_source, expected_count=1)[
                0
            ].constant_name,
            "ONE",
        )
        components = data_component_type_entries(component_source, 2)
        self.assertEqual([entry.value_type for entry in components], ["One", "Two"])
        self.assertEqual([entry.persistent for entry in components], [True, False])
        self.assertEqual(
            [entry.persistent_codec for entry in components], ["One.CODEC", None]
        )
        self.assertEqual(
            [entry.network_codec for entry in components],
            ["One.CODEC", "Two.STREAM_CODEC"],
        )
        self.assertEqual(
            [entry.network_codec_source for entry in components],
            ["persistent-fallback", "explicit"],
        )
        self.assertEqual([entry.cache_encoding for entry in components], [True, False])
        self.assertEqual(
            [entry.ignore_swap_animation for entry in components], [False, True]
        )
        self.assertEqual(
            common_item_component_constant_names(component_source, 2),
            ["TWO", "ONE"],
        )
        self.assertEqual(
            [entry.override_limiter for entry in particle_type_entries(particle_source, 2)],
            [True, False],
        )
        self.assertEqual(
            [entry.codec_family for entry in particle_type_entries(particle_source, 2)],
            [VanillaParticleCodecFamily.SIMPLE, VanillaParticleCodecFamily.BLOCK],
        )

    def test_activity_register_calls_preserve_order_and_membership(self) -> None:
        source = parse_source(
            "fixture/Activity.java",
            b"""
package fixture;
public class Activity {
  public static final Activity CORE = register("core");
  public static final Activity IDLE = register("idle");
}
""",
        )
        entries = activity_entries(source, 2)
        self.assertEqual([entry.constant_name for entry in entries], ["CORE", "IDLE"])
        self.assertEqual(
            [entry.resource_id for entry in entries],
            ["minecraft:core", "minecraft:idle"],
        )

    def test_note_block_instruments_join_enum_constants_to_sound_registrations(
        self,
    ) -> None:
        instruments = parse_source(
            "fixture/NoteBlockInstrument.java",
            b"""
package fixture;
public enum NoteBlockInstrument implements StringRepresentable {
   HARP("harp", SoundEvents.NOTE_BLOCK_HARP, NoteBlockInstrument.Type.BASE_BLOCK),
   ZOMBIE("zombie", SoundEvents.NOTE_BLOCK_IMITATE_ZOMBIE, NoteBlockInstrument.Type.MOB_HEAD),
   CUSTOM_HEAD("custom_head", SoundEvents.UI_BUTTON_CLICK, NoteBlockInstrument.Type.CUSTOM);

   private enum Type {
      BASE_BLOCK,
      MOB_HEAD,
      CUSTOM;
   }
}
""",
        )
        sounds = parse_source(
            "fixture/SoundEvents.java",
            b"""
package net.minecraft.sounds;
public class SoundEvents {
   public static final Holder.Reference<SoundEvent> NOTE_BLOCK_HARP = registerForHolder("block.note_block.harp");
   public static final Holder.Reference<SoundEvent> NOTE_BLOCK_IMITATE_ZOMBIE = registerForHolder("block.note_block.imitate.zombie");
   public static final Holder.Reference<SoundEvent> UI_BUTTON_CLICK = registerForHolder("ui.button.click");
}
""",
        )
        self.assertEqual(
            note_block_instrument_type_names(instruments),
            ["BASE_BLOCK", "MOB_HEAD", "CUSTOM"],
        )
        entries = note_block_instrument_entries(
            instruments, sounds, expected_count=3, expected_direct_sound_count=3
        )
        self.assertEqual(
            [
                (
                    entry.constant_name,
                    entry.serialized_name,
                    entry.sound_constant_name,
                    entry.sound_resource_id,
                    entry.instrument_type,
                )
                for entry in entries
            ],
            [
                (
                    "HARP",
                    "harp",
                    "NOTE_BLOCK_HARP",
                    "minecraft:block.note_block.harp",
                    "BASE_BLOCK",
                ),
                (
                    "ZOMBIE",
                    "zombie",
                    "NOTE_BLOCK_IMITATE_ZOMBIE",
                    "minecraft:block.note_block.imitate.zombie",
                    "MOB_HEAD",
                ),
                (
                    "CUSTOM_HEAD",
                    "custom_head",
                    "UI_BUTTON_CLICK",
                    "minecraft:ui.button.click",
                    "CUSTOM",
                ),
            ],
        )

    def test_note_block_instrument_rejects_an_unregistered_sound_constant(self) -> None:
        instruments = parse_source(
            "fixture/UnregisteredSound.java",
            b"""
package fixture;
public enum NoteBlockInstrument implements StringRepresentable {
   HARP("harp", SoundEvents.NOTE_BLOCK_MISSING, NoteBlockInstrument.Type.BASE_BLOCK);

   private enum Type {
      BASE_BLOCK;
   }
}
""",
        )
        sounds = parse_source(
            "fixture/SoundEvents.java",
            b"""
package net.minecraft.sounds;
public class SoundEvents {
   public static final Holder.Reference<SoundEvent> NOTE_BLOCK_HARP = registerForHolder("block.note_block.harp");
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"UnregisteredSound\.java:\d+:\d+: SoundEvents\.NOTE_BLOCK_MISSING is not a "
            r"registered sound event",
        ):
            note_block_instrument_entries(
                instruments, sounds, expected_count=1, expected_direct_sound_count=1
            )

    def test_note_block_instrument_rejects_a_type_the_nested_enum_does_not_declare(
        self,
    ) -> None:
        instruments = parse_source(
            "fixture/UnknownInstrumentType.java",
            b"""
package fixture;
public enum NoteBlockInstrument implements StringRepresentable {
   HARP("harp", SoundEvents.NOTE_BLOCK_HARP, NoteBlockInstrument.Type.PERCUSSION);

   private enum Type {
      BASE_BLOCK;
   }
}
""",
        )
        sounds = parse_source(
            "fixture/SoundEvents.java",
            b"""
package net.minecraft.sounds;
public class SoundEvents {
   public static final Holder.Reference<SoundEvent> NOTE_BLOCK_HARP = registerForHolder("block.note_block.harp");
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"UnknownInstrumentType\.java:\d+:\d+: HARP names an unknown Type",
        ):
            note_block_instrument_entries(
                instruments, sounds, expected_count=1, expected_direct_sound_count=1
            )

    def test_unsupported_registration_fails_with_source_span(self) -> None:
        source = parse_source(
            "fixture/BadEntityType.java",
            b"""
package fixture;
public class BadEntityType {
  public static final EntityType<One> ONE = guess("one", Builder.of(One::new));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"fixture/BadEntityType\.java:\d+:\d+: .*not initialized by register",
        ):
            entity_type_entries(source, expected_count=1)

    def test_operator_only_set_rejects_unknown_registry_constants(self) -> None:
        source = parse_source(
            "fixture/UnknownOperatorEntityType.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register("one", Builder.of(One::new, MobCategory.MISC).sized(1, 1));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of(MISSING);
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"UnknownOperatorEntityType\.java: OP_ONLY_CUSTOM_DATA references unknown entity types",
        ):
            entity_type_entries(source, expected_count=1)


if __name__ == "__main__":
    unittest.main()
