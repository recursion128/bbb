from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from core_registry_catalog_extractor import (  # noqa: E402
    AnimalVariantComponentRegistryEntry,
    CoreRegistryExtractionError,
    animal_variant_component_registry_entries,
    animal_variant_texture_registry_entries,
    block_entity_type_entries,
    common_item_component_constant_names,
    damage_type_entries,
    data_component_predicate_type_entries,
    data_component_type_entries,
    entity_type_entries,
    number_format_type_entries,
    particle_type_entries,
    vec3_constant,
)
from vanilla_java_source import parse_source  # noqa: E402


class CoreRegistryCatalogExtractorTests(unittest.TestCase):
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
          .vehicleAttachment(Avatar.DEFAULT_VEHICLE_ATTACHMENT).notInPeaceful().noSave());
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
  public static final ParticleType<Two> TWO = register(
      "two", false, Two.CODEC, Two.STREAM_CODEC);
}
""",
        )
        self.assertEqual(
            block_entity_type_entries(block_source, expected_count=1)[0].resource_id,
            "minecraft:one",
        )
        self.assertTrue(
            block_entity_type_entries(block_source, expected_count=1)[0].only_op_can_set_nbt
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
