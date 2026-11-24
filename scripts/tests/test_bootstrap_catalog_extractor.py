from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bootstrap_catalog_extractor import (  # noqa: E402
    BootstrapCatalogExtractionError,
    BootstrapEntry,
    CatVariantEntry,
    ConfiguredFeatureCodecEntry,
    EnchantmentEffectComponentEntry,
    EnchantmentEntry,
    FarmAnimalVariantEntry,
    FrogVariantEntry,
    LootContextEntityTargetEntry,
    LootContextParamEntry,
    LootContextParamSetEntry,
    ModelAndTextureVariantEntry,
    RegistryCodecEntry,
    WoodTypeEntry,
    banner_pattern_entries,
    bootstrap_resource_key_entries,
    cat_variant_entries,
    configured_feature_codec_entries,
    decorated_pot_pattern_entries,
    enchantment_effect_component_entries,
    enchantment_entries,
    enchantment_tooltip_order,
    farm_animal_variant_entries,
    frog_variant_entries,
    identifier_constant_paths,
    loot_context_entity_target_entries,
    loot_context_param_entries,
    loot_context_param_set_entries,
    map_decoration_entries,
    model_and_texture_variant_entries,
    overworld_day_timeline_entry,
    painting_variant_entries,
    placement_modifier_codec_entries,
    potion_entries,
    registry_codec_entries,
    sound_variant_entries,
    static_register_entries,
    static_registry_codec_entries,
    trim_material_entries,
    wolf_variant_entries,
    wood_type_entries,
    world_carver_codec_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


class BootstrapCatalogExtractorTests(unittest.TestCase):
    @staticmethod
    def configured_feature_fixture() -> bytes:
        return b"""
package fixture;
abstract class Feature<FC extends FeatureConfiguration> {
  public static final Feature<NoneFeatureConfiguration> NO_OP =
      register("no_op", new NoOpFeature(NoneFeatureConfiguration.CODEC));
  public static final Feature<TreeConfiguration> TREE =
      register("tree", new TreeFeature(TreeConfiguration.CODEC));
  public static final Feature<LakeFeature.Configuration> LAKE =
      register("lake", new LakeFeature(LakeFeature.Configuration.CODEC));
  public static final SeagrassFeature SEAGRASS =
      register("seagrass", new SeagrassFeature(ProbabilityFeatureConfiguration.CODEC));
  public static final BonusChestFeature BONUS_CHEST =
      register("bonus_chest", new BonusChestFeature(NoneFeatureConfiguration.CODEC));
  private final MapCodec<ConfiguredFeature<FC, Feature<FC>>> configuredCodec;
  private static <C extends FeatureConfiguration, F extends Feature<C>> F register(
      final String name, final F feature) {
    return Registry.register(BuiltInRegistries.FEATURE, name, feature);
  }
  public Feature(final Codec<FC> codec) {
    this.configuredCodec = codec.fieldOf("config").xmap(
        c -> new ConfiguredFeature<>(this, (FC)c), ConfiguredFeature::config);
  }
  public MapCodec<ConfiguredFeature<FC, Feature<FC>>> configuredCodec() {
    return this.configuredCodec;
  }
}
"""

    def test_configured_feature_catalog_pins_concrete_owner_and_codec_contract(
        self,
    ) -> None:
        source = parse_source(
            "fixture/Feature.java", self.configured_feature_fixture()
        )
        self.assertEqual(
            configured_feature_codec_entries(source, expected_count=5),
            [
                ConfiguredFeatureCodecEntry(
                    "minecraft:no_op", "NoOpFeature", "NoneFeatureConfiguration"
                ),
                ConfiguredFeatureCodecEntry(
                    "minecraft:tree", "TreeFeature", "TreeConfiguration"
                ),
                ConfiguredFeatureCodecEntry(
                    "minecraft:lake", "LakeFeature", "LakeFeature.Configuration"
                ),
                ConfiguredFeatureCodecEntry(
                    "minecraft:seagrass",
                    "SeagrassFeature",
                    "ProbabilityFeatureConfiguration",
                ),
                ConfiguredFeatureCodecEntry(
                    "minecraft:bonus_chest",
                    "BonusChestFeature",
                    "NoneFeatureConfiguration",
                ),
            ],
        )

        invalid_constructor = parse_source(
            "fixture/Feature.java",
            source.source.replace(b'fieldOf("config")', b'fieldOf("payload")'),
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "configured codec construction changed",
        ):
            configured_feature_codec_entries(invalid_constructor, expected_count=5)

        duplicate_owner = parse_source(
            "fixture/Feature.java",
            source.source.replace(
                b"new BonusChestFeature(NoneFeatureConfiguration.CODEC)",
                b"new SeagrassFeature(NoneFeatureConfiguration.CODEC)",
            ),
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "duplicate concrete Feature owner"
        ):
            configured_feature_codec_entries(duplicate_owner, expected_count=5)

        invalid_registry = parse_source(
            "fixture/Feature.java",
            source.source.replace(
                b"BuiltInRegistries.FEATURE, name, feature",
                b"BuiltInRegistries.OTHER, name, feature",
            ),
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "register helper target changed"
        ):
            configured_feature_codec_entries(invalid_registry, expected_count=5)

    def test_world_carver_catalog_pins_registry_and_configuration_codec_shape(self) -> None:
        source = parse_source(
            "fixture/WorldCarver.java",
            b"""
package fixture;
class WorldCarver<C> {
  static final WorldCarver<CaveConfig> CAVE = register("cave", new CaveCarver(CaveConfig.CODEC));
  static final WorldCarver<CanyonConfig> CANYON = register("canyon", new CanyonCarver(CanyonConfig.CODEC));
  private static <C, F extends WorldCarver<C>> F register(String name, F carver) {
    return Registry.register(BuiltInRegistries.CARVER, name, carver);
  }
}
""",
        )
        self.assertEqual(
            world_carver_codec_entries(source, expected_count=2),
            [
                RegistryCodecEntry("minecraft:cave", "Cave"),
                RegistryCodecEntry("minecraft:canyon", "Canyon"),
            ],
        )

    @staticmethod
    def loot_context_params_fixture() -> bytes:
        return b"""
package fixture;
class LootContextParams {
  public static final ContextKey<Entity> THIS_ENTITY = ContextKey.vanilla("this_entity");
  public static final ContextKey<Entity> INTERACTING_ENTITY = ContextKey.vanilla("interacting_entity");
  public static final ContextKey<Entity> TARGET_ENTITY = ContextKey.vanilla("target_entity");
  public static final ContextKey<Player> LAST_DAMAGE_PLAYER = ContextKey.vanilla("last_damage_player");
  public static final ContextKey<DamageSource> DAMAGE_SOURCE = ContextKey.vanilla("damage_source");
  public static final ContextKey<Entity> ATTACKING_ENTITY = ContextKey.vanilla("attacking_entity");
  public static final ContextKey<Entity> DIRECT_ATTACKING_ENTITY = ContextKey.vanilla("direct_attacking_entity");
  public static final ContextKey<Vec3> ORIGIN = ContextKey.vanilla("origin");
  public static final ContextKey<BlockState> BLOCK_STATE = ContextKey.vanilla("block_state");
  public static final ContextKey<BlockEntity> BLOCK_ENTITY = ContextKey.vanilla("block_entity");
  public static final ContextKey<ItemInstance> TOOL = ContextKey.vanilla("tool");
  public static final ContextKey<Float> EXPLOSION_RADIUS = ContextKey.vanilla("explosion_radius");
  public static final ContextKey<Integer> ENCHANTMENT_LEVEL = ContextKey.vanilla("enchantment_level");
  public static final ContextKey<Boolean> ENCHANTMENT_ACTIVE = ContextKey.vanilla("enchantment_active");
  public static final ContextKey<Unit> ADDITIONAL_COST_COMPONENT_ALLOWED = ContextKey.vanilla("additional_cost_component_allowed");
}
"""

    @staticmethod
    def loot_context_fixture() -> bytes:
        return b"""
package fixture;
class LootContext {
  enum EntityTarget {
    THIS("this", LootContextParams.THIS_ENTITY),
    ATTACKER("attacker", LootContextParams.ATTACKING_ENTITY),
    DIRECT_ATTACKER("direct_attacker", LootContextParams.DIRECT_ATTACKING_ENTITY),
    ATTACKING_PLAYER("attacking_player", LootContextParams.LAST_DAMAGE_PLAYER),
    TARGET_ENTITY("target_entity", LootContextParams.TARGET_ENTITY),
    INTERACTING_ENTITY("interacting_entity", LootContextParams.INTERACTING_ENTITY);
    EntityTarget(String name, ContextKey<? extends Entity> param) {}
  }
}
"""

    @staticmethod
    def loot_context_param_sets_fixture() -> bytes:
        return b"""
package fixture;
class LootContextParamSets {
  private static final BiMap<Identifier, ContextKeySet> REGISTRY = HashBiMap.create();
  public static final ContextKeySet EMPTY = register("empty", builder -> {});
  public static final ContextKeySet CHEST = register("chest", builder -> builder.required(LootContextParams.ORIGIN).optional(LootContextParams.THIS_ENTITY));
  public static final ContextKeySet ENTITY = register("entity", builder -> builder.required(LootContextParams.THIS_ENTITY).required(LootContextParams.ORIGIN).optional(LootContextParams.ATTACKING_ENTITY));

  private static ContextKeySet register(final String name, final Consumer<ContextKeySet.Builder> consumer) {
    ContextKeySet.Builder builder = new ContextKeySet.Builder();
    consumer.accept(builder);
    ContextKeySet result = builder.build();
    Identifier id = Identifier.withDefaultNamespace(name);
    ContextKeySet prev = REGISTRY.put(id, result);
    if (prev != null) {
      throw new IllegalStateException("Loot table parameter set " + id + " is already registered");
    } else {
      return result;
    }
  }
}
"""

    def loot_context_params(self) -> list[LootContextParamEntry]:
        return loot_context_param_entries(
            parse_source(
                "fixture/LootContextParams.java",
                self.loot_context_params_fixture(),
            )
        )

    def test_loot_context_facts_preserve_param_and_entity_target_order(self) -> None:
        params = self.loot_context_params()
        targets = loot_context_entity_target_entries(
            parse_source("fixture/LootContext.java", self.loot_context_fixture()),
            params,
        )

        self.assertEqual(
            params,
            [
                LootContextParamEntry("THIS_ENTITY", "minecraft:this_entity"),
                LootContextParamEntry("INTERACTING_ENTITY", "minecraft:interacting_entity"),
                LootContextParamEntry("TARGET_ENTITY", "minecraft:target_entity"),
                LootContextParamEntry("LAST_DAMAGE_PLAYER", "minecraft:last_damage_player"),
                LootContextParamEntry("DAMAGE_SOURCE", "minecraft:damage_source"),
                LootContextParamEntry("ATTACKING_ENTITY", "minecraft:attacking_entity"),
                LootContextParamEntry(
                    "DIRECT_ATTACKING_ENTITY", "minecraft:direct_attacking_entity"
                ),
                LootContextParamEntry("ORIGIN", "minecraft:origin"),
                LootContextParamEntry("BLOCK_STATE", "minecraft:block_state"),
                LootContextParamEntry("BLOCK_ENTITY", "minecraft:block_entity"),
                LootContextParamEntry("TOOL", "minecraft:tool"),
                LootContextParamEntry("EXPLOSION_RADIUS", "minecraft:explosion_radius"),
                LootContextParamEntry("ENCHANTMENT_LEVEL", "minecraft:enchantment_level"),
                LootContextParamEntry("ENCHANTMENT_ACTIVE", "minecraft:enchantment_active"),
                LootContextParamEntry(
                    "ADDITIONAL_COST_COMPONENT_ALLOWED",
                    "minecraft:additional_cost_component_allowed",
                ),
            ],
        )
        self.assertEqual(
            targets,
            [
                LootContextEntityTargetEntry("THIS", "this", "THIS_ENTITY"),
                LootContextEntityTargetEntry("ATTACKER", "attacker", "ATTACKING_ENTITY"),
                LootContextEntityTargetEntry(
                    "DIRECT_ATTACKER", "direct_attacker", "DIRECT_ATTACKING_ENTITY"
                ),
                LootContextEntityTargetEntry(
                    "ATTACKING_PLAYER", "attacking_player", "LAST_DAMAGE_PLAYER"
                ),
                LootContextEntityTargetEntry(
                    "TARGET_ENTITY", "target_entity", "TARGET_ENTITY"
                ),
                LootContextEntityTargetEntry(
                    "INTERACTING_ENTITY", "interacting_entity", "INTERACTING_ENTITY"
                ),
            ],
        )

    def test_loot_context_param_extraction_rejects_duplicate_and_count_drift(self) -> None:
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "duplicate loot context param resource id",
        ):
            loot_context_param_entries(
                parse_source(
                    "fixture/LootContextParams.java",
                    self.loot_context_params_fixture().replace(
                        b'ContextKey.vanilla("interacting_entity")',
                        b'ContextKey.vanilla("this_entity")',
                    ),
                )
            )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "expected 15 loot context params, extracted 14",
        ):
            loot_context_param_entries(
                parse_source(
                    "fixture/LootContextParams.java",
                    self.loot_context_params_fixture().replace(
                        b'  public static final ContextKey<Unit> ADDITIONAL_COST_COMPONENT_ALLOWED = ContextKey.vanilla("additional_cost_component_allowed");\n',
                        b"",
                    ),
                )
            )

    def test_loot_entity_targets_reject_duplicate_unknown_and_count_drift(self) -> None:
        params = self.loot_context_params()
        for changed, expected in [
            (
                self.loot_context_fixture().replace(
                    b'ATTACKER("attacker",', b'ATTACKER("this",'
                ),
                "duplicate loot entity target serialized name",
            ),
            (
                self.loot_context_fixture().replace(
                    b"LootContextParams.ATTACKING_ENTITY",
                    b"LootContextParams.UNKNOWN",
                ),
                "references unknown loot context param UNKNOWN",
            ),
            (
                self.loot_context_fixture().replace(
                    b'    INTERACTING_ENTITY("interacting_entity", LootContextParams.INTERACTING_ENTITY);\n',
                    b"    ;\n",
                ),
                "expected 6 loot entity targets, extracted 5",
            ),
        ]:
            with self.subTest(expected=expected), self.assertRaisesRegex(
                BootstrapCatalogExtractionError, expected
            ):
                loot_context_entity_target_entries(
                    parse_source("fixture/LootContext.java", changed), params
                )

    def test_loot_context_param_sets_preserve_order_required_optional_and_empty(self) -> None:
        entries = loot_context_param_set_entries(
            parse_source(
                "fixture/LootContextParamSets.java",
                self.loot_context_param_sets_fixture(),
            ),
            self.loot_context_params(),
            expected_count=3,
        )

        self.assertEqual(
            entries,
            [
                LootContextParamSetEntry("EMPTY", "minecraft:empty", (), ()),
                LootContextParamSetEntry(
                    "CHEST",
                    "minecraft:chest",
                    ("ORIGIN",),
                    ("THIS_ENTITY",),
                ),
                LootContextParamSetEntry(
                    "ENTITY",
                    "minecraft:entity",
                    ("THIS_ENTITY", "ORIGIN"),
                    ("ATTACKING_ENTITY",),
                ),
            ],
        )

    def test_loot_context_param_sets_reject_membership_and_count_drift(self) -> None:
        fixture = self.loot_context_param_sets_fixture()
        changes = [
            (
                fixture.replace(
                    b"LootContextParams.ATTACKING_ENTITY",
                    b"LootContextParams.UNKNOWN",
                ),
                "references unknown loot context param UNKNOWN",
            ),
            (
                fixture.replace(
                    b".optional(LootContextParams.ATTACKING_ENTITY)",
                    b".optional(LootContextParams.THIS_ENTITY)",
                ),
                "loot context param set ENTITY repeats a param",
            ),
            (
                fixture.replace(
                    b".optional(LootContextParams.ATTACKING_ENTITY)",
                    b".forbidden(LootContextParams.ATTACKING_ENTITY)",
                ),
                "unsupported loot context param-set builder call forbidden",
            ),
            (
                fixture.replace(
                    b'  public static final ContextKeySet EMPTY = register("empty", builder -> {});\n',
                    b"",
                ),
                "expected 3 loot context param sets, extracted 2",
            ),
        ]
        for changed, expected in changes:
            with self.subTest(expected=expected), self.assertRaisesRegex(
                BootstrapCatalogExtractionError, expected
            ):
                loot_context_param_set_entries(
                    parse_source("fixture/LootContextParamSets.java", changed),
                    self.loot_context_params(),
                    expected_count=3,
                )

    def test_loot_context_param_sets_reject_duplicate_ids_and_helper_drift(self) -> None:
        fixture = self.loot_context_param_sets_fixture()
        changes = [
            (
                fixture.replace(b"ContextKeySet CHEST", b"ContextKeySet EMPTY"),
                "duplicate loot context param-set constant",
            ),
            (
                fixture.replace(b'register("chest"', b'register("empty"'),
                "duplicate loot context param-set resource id",
            ),
            (
                fixture.replace(b"REGISTRY.put(id, result)", b"REGISTRY.forcePut(id, result)"),
                "register helper/REGISTRY sentinel shape changed",
            ),
        ]
        for changed, expected in changes:
            with self.subTest(expected=expected), self.assertRaisesRegex(
                BootstrapCatalogExtractionError, expected
            ):
                loot_context_param_set_entries(
                    parse_source("fixture/LootContextParamSets.java", changed),
                    self.loot_context_params(),
                    expected_count=3,
                )

    @staticmethod
    def overworld_day_timeline_fixture() -> bytes:
        return b"""
package fixture;
interface Timelines {
  ResourceKey<Timeline> OVERWORLD_DAY = key("day");
  int NIGHT_SKY_LIGHT_COLOR = ARGB.colorFromFloat(1.0F, 0.48F, 0.48F, 1.0F);
  static void bootstrap(BootstrapContext<Timeline> context) {
    context.register(
        OVERWORLD_DAY,
        Timeline.builder(clock)
            .setPeriodTicks(24000)
            .addModifierTrack(
                EnvironmentAttributes.FOG_COLOR,
                ColorModifier.MULTIPLY_RGB,
                track -> track.addKeyframe(0, -1))
            .addModifierTrack(
                EnvironmentAttributes.SKY_COLOR,
                ColorModifier.MULTIPLY_RGB,
                track -> track.addKeyframe(1, -16777216))
            .addModifierTrack(
                EnvironmentAttributes.SKY_LIGHT_COLOR,
                ColorModifier.MULTIPLY_RGB,
                track -> track.addKeyframe(2, NIGHT_SKY_LIGHT_COLOR))
            .addModifierTrack(
                EnvironmentAttributes.SKY_LIGHT_FACTOR,
                FloatModifier.MULTIPLY,
                track -> track.addKeyframe(3, 1.0F).addKeyframe(4, 0.25F))
            .addModifierTrack(
                EnvironmentAttributes.SKY_LIGHT_LEVEL,
                FloatModifier.MULTIPLY,
                track -> track.addKeyframe(5, 0.5F))
            .addTrack(
                EnvironmentAttributes.SUNRISE_SUNSET_COLOR,
                track -> track.addKeyframe(6, 123))
            .addModifierTrack(
                EnvironmentAttributes.STAR_BRIGHTNESS,
                FloatModifier.MAXIMUM,
                track -> track.addKeyframe(7, 0.125F))
            .addModifierTrack(
                EnvironmentAttributes.CLOUD_COLOR,
                ColorModifier.MULTIPLY_ARGB,
                track -> track.addKeyframe(8, -456))
            .build());
  }
}
"""

    def test_overworld_day_timeline_extracts_visual_tracks_and_float_bits(self) -> None:
        source = parse_source(
            "fixture/Timelines.java",
            self.overworld_day_timeline_fixture(),
        )

        entry = overworld_day_timeline_entry(source)

        self.assertEqual(entry.period_ticks, 24_000)
        self.assertEqual(entry.night_sky_light_color, -8_750_337)
        self.assertEqual(
            [track.attribute_name for track in entry.tracks],
            [
                "FOG_COLOR",
                "SKY_COLOR",
                "SKY_LIGHT_COLOR",
                "SKY_LIGHT_FACTOR",
                "SKY_LIGHT_LEVEL",
                "SUNRISE_SUNSET_COLOR",
                "STAR_BRIGHTNESS",
                "CLOUD_COLOR",
            ],
        )
        self.assertEqual(
            [
                keyframe.encoded_value
                for keyframe in entry.tracks[3].keyframes
            ],
            [0x3F80_0000, 0x3E80_0000],
        )
        self.assertEqual(entry.tracks[2].keyframes[0].encoded_value, -8_750_337)

    def test_overworld_day_timeline_rejects_changed_modifier_contract(self) -> None:
        source = parse_source(
            "fixture/Timelines.java",
            self.overworld_day_timeline_fixture().replace(
                b"FloatModifier.MAXIMUM",
                b"FloatModifier.MULTIPLY",
            ),
        )

        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "STAR_BRIGHTNESS modifier changed",
        ):
            overworld_day_timeline_entry(source)

    def test_overworld_day_timeline_rejects_non_default_visual_easing(self) -> None:
        source = parse_source(
            "fixture/Timelines.java",
            self.overworld_day_timeline_fixture().replace(
                b"track -> track.addKeyframe(7, 0.125F)",
                (
                    b"track -> track.setEasing(EasingType.CONSTANT)"
                    b".addKeyframe(7, 0.125F)"
                ),
            ),
        )

        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "visual track no longer uses default easing",
        ):
            overworld_day_timeline_entry(source)

    def test_wood_type_entries_follow_registered_constructor_names(self) -> None:
        source = parse_source(
            "fixture/WoodType.java",
            b'''
package fixture;
class WoodType {
  private static final Map<String, WoodType> TYPES = new Object2ObjectArrayMap<>();
  public static final WoodType OAK = register(new WoodType("oak", BlockSetType.OAK));
  public static final WoodType DARK_OAK = register(new WoodType(
      "dark_oak", BlockSetType.DARK_OAK, SoundType.WOOD, SoundType.HANGING_SIGN,
      SoundEvents.CLOSE, SoundEvents.OPEN));
  private static WoodType register(final WoodType type) {
    TYPES.put(type.name(), type);
    return type;
  }
  public static Stream<WoodType> values() { return TYPES.values().stream(); }
}
''',
        )

        self.assertEqual(
            wood_type_entries(source, expected_count=2),
            [WoodTypeEntry("OAK", "oak"), WoodTypeEntry("DARK_OAK", "dark_oak")],
        )

    def test_wood_type_entries_reject_changed_constructor_shape(self) -> None:
        source = parse_source(
            "fixture/WoodType.java",
            b'''
package fixture;
class WoodType {
  private static final Map<String, WoodType> TYPES = new Object2ObjectArrayMap<>();
  public static final WoodType OAK = register(new WoodType("oak"));
  private static WoodType register(final WoodType type) {
    TYPES.put(type.name(), type);
    return type;
  }
  public static Stream<WoodType> values() { return TYPES.values().stream(); }
}
''',
        )

        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "unsupported constructor arity"
        ):
            wood_type_entries(source, expected_count=1)

    def test_sound_variant_entries_follow_bootstrap_sound_map_payload(self) -> None:
        source = parse_source(
            "fixture/CatSoundVariants.java",
            b'''
package fixture;
class CatSoundVariants {
  static final ResourceKey<CatSoundVariant> CLASSIC = createKey(SoundSet.CLASSIC);
  static final ResourceKey<CatSoundVariant> ROYAL = createKey(SoundSet.ROYAL);
  static ResourceKey<CatSoundVariant> createKey(SoundSet soundSet) {
    return ResourceKey.create(Registries.CAT_SOUND_VARIANT,
        Identifier.withDefaultNamespace(soundSet.getIdentifier()));
  }
  static void bootstrap(BootstrapContext<CatSoundVariant> context) {
    register(context, CLASSIC, CatSoundVariants.SoundSet.CLASSIC);
    register(context, ROYAL, CatSoundVariants.SoundSet.ROYAL);
  }
  static void register(BootstrapContext<CatSoundVariant> context,
      ResourceKey<CatSoundVariant> key, CatSoundVariants.SoundSet soundSet) {
    context.register(key, SoundEvents.CAT_SOUNDS.get(soundSet));
  }
  enum SoundSet {
    CLASSIC("classic", "cat"), ROYAL("royal", "cat_royal");
    final String identifier;
    final String event;
    SoundSet(String identifier, String event) {
      this.identifier = identifier;
      this.event = event;
    }
    String getIdentifier() { return this.identifier; }
  }
}
''',
        )
        self.assertEqual(
            sound_variant_entries(
                source,
                "CatSoundVariant",
                "CatSoundVariants",
                "CAT_SOUNDS",
                2,
            ),
            [
                BootstrapEntry("CLASSIC", "minecraft:classic"),
                BootstrapEntry("ROYAL", "minecraft:royal"),
            ],
        )

    def test_sound_variant_entries_reject_changed_sound_map_payload(self) -> None:
        source = parse_source(
            "fixture/CatSoundVariants.java",
            b'''
package fixture;
class CatSoundVariants {
  static final ResourceKey<CatSoundVariant> CLASSIC = createKey(SoundSet.CLASSIC);
  static ResourceKey<CatSoundVariant> createKey(SoundSet soundSet) {
    return ResourceKey.create(Registries.CAT_SOUND_VARIANT,
        Identifier.withDefaultNamespace(soundSet.getIdentifier()));
  }
  static void bootstrap(BootstrapContext<CatSoundVariant> context) {
    register(context, CLASSIC, CatSoundVariants.SoundSet.CLASSIC);
  }
  static void register(BootstrapContext<CatSoundVariant> context,
      ResourceKey<CatSoundVariant> key, CatSoundVariants.SoundSet soundSet) {
    context.register(key, fallback(soundSet));
  }
  enum SoundSet {
    CLASSIC("classic", "cat");
    final String identifier;
    final String event;
    SoundSet(String identifier, String event) {
      this.identifier = identifier;
      this.event = event;
    }
    String getIdentifier() { return this.identifier; }
  }
}
''',
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "sound-variant register helper payload changed",
        ):
            sound_variant_entries(
                source,
                "CatSoundVariant",
                "CatSoundVariants",
                "CAT_SOUNDS",
                1,
            )

    def test_model_and_texture_variant_assets_follow_bootstrap_payload(self) -> None:
        source = parse_source(
            "fixture/ZombieNautilusVariants.java",
            b'''
package fixture;
class ZombieNautilusVariants {
  static final ResourceKey<ZombieNautilusVariant> TEMPERATE = createKey(Temperature.TEMPERATE);
  static final ResourceKey<ZombieNautilusVariant> WARM = createKey(Temperature.WARM);
  static void register(BootstrapContext<ZombieNautilusVariant> context,
      ResourceKey<ZombieNautilusVariant> name, ZombieNautilusVariant.ModelType modelType,
      String textureName, TagKey<Biome> spawnBiome) {
    HolderSet<Biome> biomes = context.lookup(Registries.BIOME).getOrThrow(spawnBiome);
    register(context, name, modelType, textureName,
        SpawnPrioritySelectors.single(new BiomeCheck(biomes), 1));
  }
  static void register(BootstrapContext<ZombieNautilusVariant> context,
      ResourceKey<ZombieNautilusVariant> name, ZombieNautilusVariant.ModelType modelType,
      String textureName, SpawnPrioritySelectors selectors) {
    Identifier textureId = Identifier.withDefaultNamespace("entity/nautilus/" + textureName);
    context.register(name, new ZombieNautilusVariant(
        new ModelAndTexture<>(modelType, textureId), selectors));
  }
  static void bootstrap(BootstrapContext<ZombieNautilusVariant> context) {
    register(context, TEMPERATE, ZombieNautilusVariant.ModelType.NORMAL,
        "zombie_nautilus", selectors());
    register(context, WARM, ZombieNautilusVariant.ModelType.WARM,
        "zombie_nautilus_coral", tag());
  }
}
''',
        )

        self.assertEqual(
            model_and_texture_variant_entries(
                source,
                {
                    "Temperature.TEMPERATE": "temperate",
                    "Temperature.WARM": "warm",
                },
                "ZombieNautilusVariant",
                "nautilus",
                expected_count=2,
            ),
            [
                ModelAndTextureVariantEntry(
                    "TEMPERATE",
                    "minecraft:temperate",
                    "NORMAL",
                    "minecraft:entity/nautilus/zombie_nautilus",
                ),
                ModelAndTextureVariantEntry(
                    "WARM",
                    "minecraft:warm",
                    "WARM",
                    "minecraft:entity/nautilus/zombie_nautilus_coral",
                ),
            ],
        )

    def test_model_and_texture_variant_assets_reject_changed_payload(self) -> None:
        source = parse_source(
            "fixture/ZombieNautilusVariants.java",
            b'''
package fixture;
class ZombieNautilusVariants {
  static final ResourceKey<ZombieNautilusVariant> TEMPERATE = createKey(Temperature.TEMPERATE);
  static void register(BootstrapContext<ZombieNautilusVariant> context,
      ResourceKey<ZombieNautilusVariant> name, ZombieNautilusVariant.ModelType modelType,
      String textureName, TagKey<Biome> spawnBiome) {
    HolderSet<Biome> biomes = context.lookup(Registries.BIOME).getOrThrow(spawnBiome);
    register(context, name, modelType, textureName,
        SpawnPrioritySelectors.single(new BiomeCheck(biomes), 1));
  }
  static void register(BootstrapContext<ZombieNautilusVariant> context,
      ResourceKey<ZombieNautilusVariant> name, ZombieNautilusVariant.ModelType modelType,
      String textureName, SpawnPrioritySelectors selectors) {
    Identifier textureId = Identifier.withDefaultNamespace("entity/nautilus/" + textureName);
    context.register(name, new ZombieNautilusVariant(modelType, textureId, selectors));
  }
  static void bootstrap(BootstrapContext<ZombieNautilusVariant> context) {
    register(context, TEMPERATE, ZombieNautilusVariant.ModelType.NORMAL,
        "zombie_nautilus", selectors());
  }
}
''',
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "model-and-texture registration payload changed",
        ):
            model_and_texture_variant_entries(
                source,
                {"Temperature.TEMPERATE": "temperate"},
                "ZombieNautilusVariant",
                "nautilus",
                expected_count=1,
            )

    def test_farm_animal_variant_assets_follow_model_and_texture_payload(self) -> None:
        source = parse_source(
            "fixture/ChickenVariants.java",
            b'''
package fixture;
class ChickenVariants {
  static final ResourceKey<ChickenVariant> TEMPERATE = createKey(Temperature.TEMPERATE);
  static final ResourceKey<ChickenVariant> COLD = createKey(Temperature.COLD);
  static void register(BootstrapContext<ChickenVariant> context,
      ResourceKey<ChickenVariant> name, ChickenVariant.ModelType modelType,
      String textureName, String babyTextureName, TagKey<Biome> spawnBiome) {
    HolderSet<Biome> biomes = context.lookup(Registries.BIOME).getOrThrow(spawnBiome);
    register(context, name, modelType, textureName, babyTextureName,
        SpawnPrioritySelectors.single(new BiomeCheck(biomes), 1));
  }
  static void register(BootstrapContext<ChickenVariant> context,
      ResourceKey<ChickenVariant> name, ChickenVariant.ModelType modelType,
      String textureName, String babyTextureName, SpawnPrioritySelectors selectors) {
    Identifier textureId = Identifier.withDefaultNamespace("entity/chicken/" + textureName);
    Identifier babyTextureId = Identifier.withDefaultNamespace(
        "entity/chicken/" + babyTextureName);
    context.register(name, new ChickenVariant(
        new ModelAndTexture<>(modelType, textureId),
        new ClientAsset.ResourceTexture(babyTextureId), selectors));
  }
  static void bootstrap(BootstrapContext<ChickenVariant> context) {
    register(context, TEMPERATE, ChickenVariant.ModelType.NORMAL,
        "chicken_temperate", "chicken_temperate_baby", selectors());
    register(context, COLD, ChickenVariant.ModelType.COLD,
        "chicken_cold", "chicken_cold_baby", tag());
  }
}
''',
        )

        self.assertEqual(
            farm_animal_variant_entries(
                source,
                {
                    "Temperature.TEMPERATE": "temperate",
                    "Temperature.COLD": "cold",
                },
                "ChickenVariant",
                "chicken",
                expected_count=2,
            ),
            [
                FarmAnimalVariantEntry(
                    "TEMPERATE",
                    "minecraft:temperate",
                    "NORMAL",
                    "minecraft:entity/chicken/chicken_temperate",
                    "minecraft:entity/chicken/chicken_temperate_baby",
                ),
                FarmAnimalVariantEntry(
                    "COLD",
                    "minecraft:cold",
                    "COLD",
                    "minecraft:entity/chicken/chicken_cold",
                    "minecraft:entity/chicken/chicken_cold_baby",
                ),
            ],
        )

    def test_farm_animal_variant_assets_reject_changed_path_formula(self) -> None:
        source = parse_source(
            "fixture/PigVariants.java",
            b'''
package fixture;
class PigVariants {
  static final ResourceKey<PigVariant> TEMPERATE = createKey(Temperature.TEMPERATE);
  static void register(BootstrapContext<PigVariant> context,
      ResourceKey<PigVariant> name, PigVariant.ModelType modelType,
      String textureName, String babyTextureName, TagKey<Biome> spawnBiome) {
    HolderSet<Biome> biomes = context.lookup(Registries.BIOME).getOrThrow(spawnBiome);
    register(context, name, modelType, textureName, babyTextureName,
        SpawnPrioritySelectors.single(new BiomeCheck(biomes), 1));
  }
  static void register(BootstrapContext<PigVariant> context,
      ResourceKey<PigVariant> name, PigVariant.ModelType modelType,
      String textureName, String babyTextureName, SpawnPrioritySelectors selectors) {
    Identifier textureId = Identifier.withDefaultNamespace("entity/wrong/" + textureName);
    Identifier babyTextureId = Identifier.withDefaultNamespace("entity/pig/" + babyTextureName);
    context.register(name, new PigVariant(
        new ModelAndTexture<>(modelType, textureId),
        new ClientAsset.ResourceTexture(babyTextureId), selectors));
  }
  static void bootstrap(BootstrapContext<PigVariant> context) {
    register(context, TEMPERATE, PigVariant.ModelType.NORMAL,
        "pig_temperate", "pig_temperate_baby", selectors());
  }
}
''',
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "farm-animal asset formula for textureId changed",
        ):
            farm_animal_variant_entries(
                source,
                {"Temperature.TEMPERATE": "temperate"},
                "PigVariant",
                "pig",
                expected_count=1,
            )

    def test_frog_variant_assets_follow_bootstrap_literals_and_helper_payload(self) -> None:
        source = parse_source(
            "fixture/FrogVariants.java",
            b'''
package fixture;
class FrogVariants {
  static final ResourceKey<FrogVariant> TEMPERATE = createKey(Temperature.TEMPERATE);
  static final ResourceKey<FrogVariant> WARM = createKey(Temperature.WARM);
  static void register(BootstrapContext<FrogVariant> context,
      ResourceKey<FrogVariant> name, String assetId, TagKey<Biome> limitToBiome) {
    HolderSet<Biome> biomes = context.lookup(Registries.BIOME).getOrThrow(limitToBiome);
    register(context, name, assetId,
        SpawnPrioritySelectors.single(new BiomeCheck(biomes), 1));
  }
  static void register(BootstrapContext<FrogVariant> context,
      ResourceKey<FrogVariant> name, String assetId, SpawnPrioritySelectors selectors) {
    context.register(name, new FrogVariant(
        new ClientAsset.ResourceTexture(Identifier.withDefaultNamespace(assetId)), selectors));
  }
  static void bootstrap(BootstrapContext<FrogVariant> registry) {
    register(registry, TEMPERATE, "entity/frog/frog_temperate", selectors());
    register(registry, WARM, "entity/frog/frog_warm", tag());
  }
}
''',
        )

        self.assertEqual(
            frog_variant_entries(
                source,
                {
                    "Temperature.TEMPERATE": "temperate",
                    "Temperature.WARM": "warm",
                },
                expected_count=2,
            ),
            [
                FrogVariantEntry(
                    "TEMPERATE",
                    "minecraft:temperate",
                    "minecraft:entity/frog/frog_temperate",
                ),
                FrogVariantEntry(
                    "WARM",
                    "minecraft:warm",
                    "minecraft:entity/frog/frog_warm",
                ),
            ],
        )

    def test_frog_variant_assets_reject_changed_helper_payload(self) -> None:
        source = parse_source(
            "fixture/FrogVariants.java",
            b'''
package fixture;
class FrogVariants {
  static final ResourceKey<FrogVariant> TEMPERATE = createKey(Temperature.TEMPERATE);
  static void register(BootstrapContext<FrogVariant> context,
      ResourceKey<FrogVariant> name, String assetId, TagKey<Biome> limitToBiome) {
    HolderSet<Biome> biomes = context.lookup(Registries.BIOME).getOrThrow(limitToBiome);
    register(context, name, assetId,
        SpawnPrioritySelectors.single(new BiomeCheck(biomes), 1));
  }
  static void register(BootstrapContext<FrogVariant> context,
      ResourceKey<FrogVariant> name, String assetId, SpawnPrioritySelectors selectors) {
    context.register(name, new FrogVariant(
        new ClientAsset.ResourceTexture(Identifier.withDefaultNamespace("wrong")), selectors));
  }
  static void bootstrap(BootstrapContext<FrogVariant> registry) {
    register(registry, TEMPERATE, "entity/frog/frog_temperate", selectors());
  }
}
''',
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "frog asset registration payload changed",
        ):
            frog_variant_entries(
                source,
                {"Temperature.TEMPERATE": "temperate"},
                expected_count=1,
            )

    def test_cat_variant_assets_follow_bootstrap_literals_and_helper_payload(self) -> None:
        source = parse_source(
            "fixture/CatVariants.java",
            b'''
package fixture;
class CatVariants {
  static final ResourceKey<CatVariant> TABBY = createKey("tabby");
  static final ResourceKey<CatVariant> ALL_BLACK = createKey("all_black");
  static void registerForAnyConditions(BootstrapContext<CatVariant> context,
      ResourceKey<CatVariant> name, String adultTexture, String babyTexture) {
    register(context, name, adultTexture, babyTexture, SpawnPrioritySelectors.fallback(0));
  }
  static void register(BootstrapContext<CatVariant> context,
      ResourceKey<CatVariant> name, String adultTexture, String babyTexture,
      SpawnPrioritySelectors spawnConditions) {
    context.register(name, new CatVariant(
        new ClientAsset.ResourceTexture(Identifier.withDefaultNamespace(adultTexture)),
        new ClientAsset.ResourceTexture(Identifier.withDefaultNamespace(babyTexture)),
        spawnConditions));
  }
  static void bootstrap(BootstrapContext<CatVariant> context) {
    registerForAnyConditions(
        context, TABBY, "entity/cat/cat_tabby", "entity/cat/cat_tabby_baby");
    register(
        context, ALL_BLACK, "entity/cat/cat_all_black",
        "entity/cat/cat_all_black_baby", selectors());
  }
}
''',
        )

        self.assertEqual(
            cat_variant_entries(source, expected_count=2),
            [
                CatVariantEntry(
                    "TABBY",
                    "minecraft:tabby",
                    "minecraft:entity/cat/cat_tabby",
                    "minecraft:entity/cat/cat_tabby_baby",
                ),
                CatVariantEntry(
                    "ALL_BLACK",
                    "minecraft:all_black",
                    "minecraft:entity/cat/cat_all_black",
                    "minecraft:entity/cat/cat_all_black_baby",
                ),
            ],
        )

    def test_cat_variant_assets_reject_changed_helper_payload(self) -> None:
        source = parse_source(
            "fixture/CatVariants.java",
            b'''
package fixture;
class CatVariants {
  static final ResourceKey<CatVariant> TABBY = createKey("tabby");
  static void registerForAnyConditions(BootstrapContext<CatVariant> context,
      ResourceKey<CatVariant> name, String adultTexture, String babyTexture) {
    register(context, name, adultTexture, babyTexture, SpawnPrioritySelectors.fallback(0));
  }
  static void register(BootstrapContext<CatVariant> context,
      ResourceKey<CatVariant> name, String adultTexture, String babyTexture,
      SpawnPrioritySelectors spawnConditions) {
    context.register(name, new CatVariant(
        new ClientAsset.ResourceTexture(Identifier.withDefaultNamespace(adultTexture)),
        new ClientAsset.ResourceTexture(Identifier.withDefaultNamespace(adultTexture)),
        spawnConditions));
  }
  static void bootstrap(BootstrapContext<CatVariant> context) {
    registerForAnyConditions(
        context, TABBY, "entity/cat/cat_tabby", "entity/cat/cat_tabby_baby");
  }
}
''',
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "cat asset registration payload changed",
        ):
            cat_variant_entries(source, expected_count=1)

    def test_wolf_variant_assets_follow_bootstrap_file_names_and_helper_formula(self) -> None:
        source = parse_source(
            "fixture/WolfVariants.java",
            b'''
package fixture;
class WolfVariants {
  static final ResourceKey<WolfVariant> PALE = createKey("pale");
  static final ResourceKey<WolfVariant> SPOTTED = createKey("spotted");
  static void register(BootstrapContext<WolfVariant> context,
                       ResourceKey<WolfVariant> name, String fileName,
                       SpawnPrioritySelectors selectors) {
    Identifier wildTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName);
    Identifier tameTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_tame");
    Identifier angryTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_angry");
    Identifier babyTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_baby");
    Identifier tameBabyTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_tame_baby");
    Identifier angryBabyTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_angry_baby");
    context.register(name, new WolfVariant(
        new WolfVariant.AssetInfo(
            new ClientAsset.ResourceTexture(wildTexture),
            new ClientAsset.ResourceTexture(tameTexture),
            new ClientAsset.ResourceTexture(angryTexture)),
        new WolfVariant.AssetInfo(
            new ClientAsset.ResourceTexture(babyTexture),
            new ClientAsset.ResourceTexture(tameBabyTexture),
            new ClientAsset.ResourceTexture(angryBabyTexture)),
        selectors));
  }
  static void bootstrap(BootstrapContext<WolfVariant> context) {
    register(context, SPOTTED, "wolf_spotted", selectors());
    register(context, PALE, "wolf", selectors());
  }
}
''',
        )

        entries = wolf_variant_entries(source, expected_count=2)

        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id, entry.file_name, entry.asset_ids)
                for entry in entries
            ],
            [
                (
                    "SPOTTED",
                    "minecraft:spotted",
                    "wolf_spotted",
                    (
                        "minecraft:entity/wolf/wolf_spotted",
                        "minecraft:entity/wolf/wolf_spotted_tame",
                        "minecraft:entity/wolf/wolf_spotted_angry",
                        "minecraft:entity/wolf/wolf_spotted_baby",
                        "minecraft:entity/wolf/wolf_spotted_tame_baby",
                        "minecraft:entity/wolf/wolf_spotted_angry_baby",
                    ),
                ),
                (
                    "PALE",
                    "minecraft:pale",
                    "wolf",
                    (
                        "minecraft:entity/wolf/wolf",
                        "minecraft:entity/wolf/wolf_tame",
                        "minecraft:entity/wolf/wolf_angry",
                        "minecraft:entity/wolf/wolf_baby",
                        "minecraft:entity/wolf/wolf_tame_baby",
                        "minecraft:entity/wolf/wolf_angry_baby",
                    ),
                ),
            ],
        )

    def test_wolf_variant_assets_reject_changed_helper_formula(self) -> None:
        source = parse_source(
            "fixture/WolfVariants.java",
            b'''
package fixture;
class WolfVariants {
  static final ResourceKey<WolfVariant> PALE = createKey("pale");
  static void register(BootstrapContext<WolfVariant> context,
                       ResourceKey<WolfVariant> name, String fileName,
                       SpawnPrioritySelectors selectors) {
    Identifier wildTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName);
    Identifier tameTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_friendly");
    Identifier angryTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_angry");
    Identifier babyTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_baby");
    Identifier tameBabyTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_tame_baby");
    Identifier angryBabyTexture = Identifier.withDefaultNamespace("entity/wolf/" + fileName + "_angry_baby");
    context.register(name, new WolfVariant(
        new WolfVariant.AssetInfo(
            new ClientAsset.ResourceTexture(wildTexture),
            new ClientAsset.ResourceTexture(tameTexture),
            new ClientAsset.ResourceTexture(angryTexture)),
        new WolfVariant.AssetInfo(
            new ClientAsset.ResourceTexture(babyTexture),
            new ClientAsset.ResourceTexture(tameBabyTexture),
            new ClientAsset.ResourceTexture(angryBabyTexture)),
        selectors));
  }
  static void bootstrap(BootstrapContext<WolfVariant> context) {
    register(context, PALE, "wolf", selectors());
  }
}
''',
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "wolf asset formula for tameTexture changed",
        ):
            wolf_variant_entries(source, expected_count=1)

    def test_map_decorations_preserve_registration_projection_order(self) -> None:
        source = parse_source(
            "fixture/MapDecorationTypes.java",
            b"""package fixture;
public class MapDecorationTypes {
  static final Holder<MapDecorationType> PLAYER = register("player", "player", false, true);
  static final Holder<MapDecorationType> MANSION = register(
      "mansion", "woodland_mansion", true, 5393476, false, true);
}
""",
        )

        entries = map_decoration_entries(source, 2)
        self.assertEqual([entry.map_color for entry in entries], [None, 5393476])
        self.assertEqual([entry.exploration_map_element for entry in entries], [False, True])

        self.assertEqual(
            [
                (
                    entry.constant_name,
                    entry.resource_id,
                    entry.asset_id,
                    entry.show_on_item_frame,
                    entry.track_count,
                )
                for entry in entries
            ],
            [
                ("PLAYER", "minecraft:player", "minecraft:player", False, True),
                (
                    "MANSION",
                    "minecraft:mansion",
                    "minecraft:woodland_mansion",
                    True,
                    False,
                ),
            ],
        )

    def test_bootstrap_order_is_cross_checked_against_resource_key_fields(self) -> None:
        source = parse_source(
            "fixture/Variants.java",
            b"""
package fixture;
public class Variants {
  public static final ResourceKey<Variant> FIRST = key("first");
  public static final ResourceKey<Variant> SECOND = key("second");
  public static final ResourceKey<Variant> DEFAULT = FIRST;
  static void bootstrap(BootstrapContext<Variant> context) {
    register(context, SECOND); register(context, FIRST);
  }
}
""",
        )
        entries = bootstrap_resource_key_entries(source, "Variant", {"key"}, 2)
        self.assertEqual(
            [(entry.constant_name, entry.resource_id) for entry in entries],
            [("SECOND", "minecraft:second"), ("FIRST", "minecraft:first")],
        )

    def test_registry_codec_entries_include_return_registration_and_owner_identity(
        self,
    ) -> None:
        source = parse_source(
            "fixture/CodecTypes.java",
            b"""
package fixture;
interface CodecTypes {
  static MapCodec<? extends Value> bootstrap(Registry<MapCodec<? extends Value>> registry) {
    Registry.register(registry, "first", Value.First.CODEC);
    return Registry.register(registry, "all_of", AllOf.ValueEffects.CODEC);
  }
}
""",
        )

        self.assertEqual(
            registry_codec_entries(source, 2),
            [
                RegistryCodecEntry("minecraft:first", "Value.First"),
                RegistryCodecEntry("minecraft:all_of", "AllOf.ValueEffects"),
            ],
        )

    def test_registry_codec_entries_fail_closed_on_unknown_registration_shape(
        self,
    ) -> None:
        fixtures = [
            (
                b'Registry.register(other, "first", Value.First.CODEC);',
                "does not target the bootstrap registry",
            ),
            (
                b'registry.register("first", Value.First.CODEC);',
                "unsupported bootstrap register owner",
            ),
            (
                b'Registry.register(registry, "first", codec());',
                "codec is not a qualified field reference",
            ),
            (
                b'Registry.register(registry, "first", Value.First.MAP_CODEC);',
                "registered codec terminal must be one of",
            ),
        ]
        for body, message in fixtures:
            with self.subTest(message=message):
                source = parse_source(
                    "fixture/CodecTypes.java",
                    b"package fixture; interface CodecTypes { "
                    b"static C bootstrap(R registry) { "
                    + body
                    + b" return null; } }",
                )
                with self.assertRaisesRegex(
                    BootstrapCatalogExtractionError, message
                ):
                    registry_codec_entries(source, 1)

    def test_registry_codec_entries_accept_only_explicit_frozen_terminals(self) -> None:
        source = parse_source(
            "fixture/MapCodecTypes.java",
            b"""
package fixture;
interface MapCodecTypes {
  static C bootstrap(R registry) {
    return Registry.register(registry, "first", Value.First.MAP_CODEC);
  }
}
""",
        )
        self.assertEqual(
            registry_codec_entries(
                source, 1, codec_terminals=frozenset({"MAP_CODEC"})
            ),
            [RegistryCodecEntry("minecraft:first", "Value.First")],
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "codec terminals must be"
        ):
            registry_codec_entries(
                source, 1, codec_terminals=frozenset({"STREAM_CODEC"})
            )

    def test_registry_codec_entries_reject_duplicates_and_count_drift(self) -> None:
        duplicate_id = parse_source(
            "fixture/DuplicateIds.java",
            b"""
package fixture;
interface DuplicateIds {
  static C bootstrap(R registry) {
    Registry.register(registry, "same", Value.First.CODEC);
    return Registry.register(registry, "same", Value.Second.CODEC);
  }
}
""",
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "duplicate registry codec resource id"
        ):
            registry_codec_entries(duplicate_id, 2)

        duplicate_owner = parse_source(
            "fixture/DuplicateOwners.java",
            b"""
package fixture;
interface DuplicateOwners {
  static C bootstrap(R registry) {
    Registry.register(registry, "first", Value.First.CODEC);
    return Registry.register(registry, "second", Value.First.CODEC);
  }
}
""",
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "duplicate registry codec owner"
        ):
            registry_codec_entries(duplicate_owner, 2)

        single = parse_source(
            "fixture/SingleCodec.java",
            b"""
package fixture;
interface SingleCodec {
  static C bootstrap(R registry) {
    return Registry.register(registry, "first", Value.First.CODEC);
  }
}
""",
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "expected 2 registry codec entries"
        ):
            registry_codec_entries(single, 2)

    def test_static_registry_codec_entries_validate_fields_helper_and_sentinel(
        self,
    ) -> None:
        source = parse_source(
            "fixture/StaticCodecTypes.java",
            b"""
package fixture;
interface StaticCodecTypes {
  MapCodec<First> FIRST = register("first", First.CODEC);
  MapCodec<Second> SECOND = register("second", Second.CODEC);
  private static <T> MapCodec<T> register(String id, MapCodec<T> value) {
    return Registry.register(BuiltInRegistries.EXAMPLE, id, value);
  }
  static MapCodec<?> bootstrap(Registry<MapCodec<?>> registry) { return FIRST; }
}
""",
        )
        self.assertEqual(
            static_registry_codec_entries(
                source,
                "MapCodec<",
                "BuiltInRegistries.EXAMPLE",
                "value",
                2,
                bootstrap_sentinel="FIRST",
            ),
            [
                RegistryCodecEntry("minecraft:first", "First"),
                RegistryCodecEntry("minecraft:second", "Second"),
            ],
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "payload construction changed"
        ):
            static_registry_codec_entries(
                source,
                "MapCodec<",
                "BuiltInRegistries.EXAMPLE",
                "new Wrapper<>(value)",
                2,
                bootstrap_sentinel="FIRST",
            )

    def test_static_registry_codec_entries_extract_placement_modifier_contract(
        self,
    ) -> None:
        source = parse_source(
            "fixture/PlacementModifierType.java",
            b"""
package fixture;
interface PlacementModifierType<P extends PlacementModifier> {
  PlacementModifierType<BlockPredicateFilter> BLOCK_PREDICATE_FILTER =
      register("block_predicate_filter", BlockPredicateFilter.CODEC);
  PlacementModifierType<BiomeFilter> BIOME_FILTER =
      register("biome", BiomeFilter.CODEC);
  MapCodec<P> codec();
  private static <P extends PlacementModifier> PlacementModifierType<P> register(
      final String id, final MapCodec<P> codec) {
    return Registry.register(
        BuiltInRegistries.PLACEMENT_MODIFIER_TYPE, id, () -> codec);
  }
}
""",
        )
        self.assertEqual(
            placement_modifier_codec_entries(source, 2),
            [
                RegistryCodecEntry(
                    "minecraft:block_predicate_filter", "BlockPredicateFilter"
                ),
                RegistryCodecEntry("minecraft:biome", "BiomeFilter"),
            ],
        )

        invalid_map_codec = parse_source(
            "fixture/InvalidPlacementModifierType.java",
            source.source.replace(b"MapCodec<P> codec();", b"Codec<P> codec();"),
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "MapCodec owner method changed"
        ):
            placement_modifier_codec_entries(invalid_map_codec, 2)

        invalid_payload = parse_source(
            "fixture/InvalidPlacementModifierType.java",
            source.source.replace(b"() -> codec", b"codec"),
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "payload construction changed"
        ):
            placement_modifier_codec_entries(invalid_payload, 2)

    def test_enchantment_effect_components_extract_typed_family_and_context(
        self,
    ) -> None:
        source = parse_source(
            "fixture/EnchantmentEffectComponents.java",
            b"""
package fixture;
interface EnchantmentEffectComponents {
  DataComponentType<List<ConditionalEffect<EnchantmentValueEffect>>> DAMAGE =
      register("damage", b -> b.persistent(validatedListCodec(
          ConditionalEffect.codec(EnchantmentValueEffect.CODEC),
          LootContextParamSets.ENCHANTED_DAMAGE)));
  DataComponentType<Unit> PREVENT =
      register("prevent", b -> b.persistent(Unit.CODEC));
  static DataComponentType<?> bootstrap(Registry<DataComponentType<?>> registry) {
    return DAMAGE_PROTECTION;
  }
  private static <T> DataComponentType<T> register(
      String id, UnaryOperator<DataComponentType.Builder<T>> builder) {
    return Registry.register(
        BuiltInRegistries.ENCHANTMENT_EFFECT_COMPONENT_TYPE,
        id,
        builder.apply(DataComponentType.builder()).build());
  }
}
""",
        )
        self.assertEqual(
            enchantment_effect_component_entries(source, 2),
            [
                EnchantmentEffectComponentEntry(
                    "minecraft:damage",
                    "ConditionalValueEffect",
                    "ENCHANTED_DAMAGE",
                ),
                EnchantmentEffectComponentEntry("minecraft:prevent", "Unit", None),
            ],
        )

    def test_enchantment_effect_components_fail_closed_on_builder_drift(self) -> None:
        source = parse_source(
            "fixture/EnchantmentEffectComponents.java",
            b"""
package fixture;
interface EnchantmentEffectComponents {
  DataComponentType<Unit> DAMAGE = register("damage", b -> b.network(Unit.CODEC));
  static DataComponentType<?> bootstrap(Registry<DataComponentType<?>> registry) {
    return DAMAGE_PROTECTION;
  }
  private static <T> DataComponentType<T> register(
      String id, UnaryOperator<DataComponentType.Builder<T>> builder) {
    return Registry.register(
        BuiltInRegistries.ENCHANTMENT_EFFECT_COMPONENT_TYPE,
        id,
        builder.apply(DataComponentType.builder()).build());
  }
}
""",
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "unsupported initializer call network"
        ):
            enchantment_effect_component_entries(source, 1)

    def test_direct_context_bootstrap_order_is_cross_checked_against_keys(self) -> None:
        source = parse_source(
            "fixture/ChatTypes.java",
            b"""
package fixture;
public class ChatTypes {
  public static final ResourceKey<ChatType> CHAT = create("chat");
  public static final ResourceKey<ChatType> EMOTE = create("emote");
  static void bootstrap(BootstrapContext<ChatType> context) {
    context.register(EMOTE, new ChatType());
    context.register(CHAT, new ChatType());
  }
}
""",
        )
        entries = bootstrap_resource_key_entries(
            source, "ChatType", {"create"}, 2
        )
        self.assertEqual(
            [(entry.constant_name, entry.resource_id) for entry in entries],
            [("EMOTE", "minecraft:emote"), ("CHAT", "minecraft:chat")],
        )

    def test_nested_enum_constructor_string_resolves_without_text_parser(self) -> None:
        source = parse_source(
            "fixture/Sounds.java",
            b"""
package fixture;
public class Sounds {
  public static final ResourceKey<Sound> CLASSIC = key(Sounds.Set.CLASSIC);
  static void bootstrap(BootstrapContext<Sound> context) { register(context, CLASSIC); }
  enum Set { CLASSIC("classic", "sound"); Set(String id, String sound) {} }
}
""",
        )
        entries = bootstrap_resource_key_entries(source, "Sound", {"key"}, 1)
        self.assertEqual(entries[0].resource_id, "minecraft:classic")

    def test_static_register_fields_preserve_cst_declaration_order(self) -> None:
        source = parse_source(
            "fixture/Potions.java",
            b"""
package fixture;
public class Potions {
  static final Holder.Reference<Potion> WATER = register("water", new Potion());
  static final Holder.Reference<Potion> LONG = register(
      "long", new Potion("fake register(\\\"ignored\\\")"));
}
""",
        )
        entries = static_register_entries(source, "Holder.Reference<Potion>", 2)
        self.assertEqual(
            [entry.resource_id for entry in entries],
            ["minecraft:water", "minecraft:long"],
        )

    def test_static_register_fields_accept_multiple_exact_value_types(self) -> None:
        source = parse_source(
            "fixture/Fluids.java",
            b"""
package fixture;
public class Fluids {
  static final Fluid EMPTY = register("empty", new EmptyFluid());
  static final FlowingFluid FLOWING_WATER =
      register("flowing_water", new WaterFluid.Flowing());
  static final FluidState IGNORED = state("ignored");
  static final FlowingFluid WATER = register("water", new WaterFluid.Source());
}
""",
        )
        entries = static_register_entries(
            source, ("Fluid", "FlowingFluid"), 3
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id)
                for entry in entries
            ],
            [
                ("EMPTY", "minecraft:empty"),
                ("FLOWING_WATER", "minecraft:flowing_water"),
                ("WATER", "minecraft:water"),
            ],
        )

    def test_potion_effects_come_from_typed_constructor_arguments(self) -> None:
        source = parse_source(
            "fixture/Potions.java",
            b"""
package fixture;
public class Potions {
  static final Holder.Reference<Potion> WATER = register("water", new Potion("water"));
  static final Holder.Reference<Potion> SPEED = register(
      "speed", new Potion("speed", new MobEffectInstance(MobEffects.SPEED, 3_600)));
  static final Holder.Reference<Potion> MIXED = register(
      "mixed", new Potion("shared_suffix",
          new MobEffectInstance(MobEffects.SLOWNESS, 400, 3),
          new MobEffectInstance(MobEffects.RESISTANCE, 400, 2)));
}
""",
        )
        entries = potion_entries(
            source,
            {"SPEED", "SLOWNESS", "RESISTANCE"},
            expected_count=3,
            expected_effect_count=3,
        )
        self.assertEqual(
            [
                (
                    entry.constant_name,
                    entry.resource_id,
                    entry.name,
                    [
                        (effect.constant_name, effect.duration, effect.amplifier)
                        for effect in entry.effects
                    ],
                )
                for entry in entries
            ],
            [
                ("WATER", "minecraft:water", "water", []),
                ("SPEED", "minecraft:speed", "speed", [("SPEED", 3600, 0)]),
                (
                    "MIXED",
                    "minecraft:mixed",
                    "shared_suffix",
                    [("SLOWNESS", 400, 3), ("RESISTANCE", 400, 2)],
                ),
            ],
        )

    def test_potion_effect_rejects_unknown_mob_effect_constant(self) -> None:
        source = parse_source(
            "fixture/Potions.java",
            b"""
package fixture;
public class Potions {
  static final Holder.Reference<Potion> BAD = register(
      "bad", new Potion("bad", new MobEffectInstance(MobEffects.UNKNOWN, 20)));
}
""",
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "unknown MobEffects constant UNKNOWN",
        ):
            potion_entries(
                source,
                {"SPEED"},
                expected_count=1,
                expected_effect_count=1,
            )

    def test_trim_material_colors_follow_bootstrap_order(self) -> None:
        source = parse_source(
            "fixture/TrimMaterials.java",
            b"""
package fixture;
class TrimMaterials {
  static final ResourceKey<TrimMaterial> FIRST = registryKey("first");
  static final ResourceKey<TrimMaterial> SECOND = registryKey("second");
  static void bootstrap(BootstrapContext<TrimMaterial> context) {
    register(context, SECOND, Style.EMPTY.withColor(0x12), MaterialAssetGroup.SECOND);
    register(context, FIRST, Style.EMPTY.withColor(34), MaterialAssetGroup.FIRST);
  }
}
""",
        )
        material_asset_groups = parse_source(
            "fixture/MaterialAssetGroup.java",
            b"""
package fixture;
class MaterialAssetGroup {
  static final MaterialAssetGroup FIRST = create("first");
  static final MaterialAssetGroup SECOND = create(
      "second", Map.of(EquipmentAssets.IRON, "second_darker"));
}
""",
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            "expected a decimal integer literal",
        ):
            trim_material_entries(source, material_asset_groups, expected_count=2)

        source = parse_source(
            "fixture/TrimMaterials.java",
            source.source.replace(b"0x12", b"18"),
        )
        entries = trim_material_entries(
            source, material_asset_groups, expected_count=2
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id, entry.description_color)
                for entry in entries
            ],
            [
                ("SECOND", "minecraft:second", 18),
                ("FIRST", "minecraft:first", 34),
            ],
        )
        self.assertEqual(entries[0].asset_name, "second")
        self.assertEqual(
            entries[0].override_armor_assets,
            (("minecraft:iron", "second_darker"),),
        )
        self.assertEqual(entries[1].asset_name, "first")
        self.assertEqual(entries[1].override_armor_assets, ())

    def test_decorated_pot_patterns_join_items_map_and_bootstrap(self) -> None:
        patterns = parse_source(
            "fixture/DecoratedPotPatterns.java",
            b"""
package fixture;
class DecoratedPotPatterns {
  static final ResourceKey<DecoratedPotPattern> BLANK = create("blank");
  static final ResourceKey<DecoratedPotPattern> ANGLER = create("angler");
  static final Map<Item, ResourceKey<DecoratedPotPattern>> ITEM_TO_POT_TEXTURE = Map.ofEntries(
      Map.entry(Items.BRICK, BLANK),
      Map.entry(Items.ANGLER_POTTERY_SHERD, ANGLER));
  static DecoratedPotPattern bootstrap(Registry<DecoratedPotPattern> registry) {
    register(registry, ANGLER, "angler_pottery_pattern");
    return register(registry, BLANK, "decorated_pot_side");
  }
}
""",
        )
        items = parse_source(
            "fixture/Items.java",
            b"""
package fixture;
class Items {
  static final Item BRICK = registerItem("brick");
  static final Item ANGLER_POTTERY_SHERD = registerItem(
      "angler_pottery_sherd", new Item.Properties());
}
""",
        )
        self.assertEqual(
            [
                (
                    entry.constant_name,
                    entry.resource_id,
                    entry.item_constant_name,
                    entry.item_resource_id,
                    entry.asset_id,
                )
                for entry in decorated_pot_pattern_entries(
                    patterns,
                    items,
                    expected_count=2,
                )
            ],
            [
                (
                    "ANGLER",
                    "minecraft:angler",
                    "ANGLER_POTTERY_SHERD",
                    "minecraft:angler_pottery_sherd",
                    "minecraft:angler_pottery_pattern",
                ),
                (
                    "BLANK",
                    "minecraft:blank",
                    "BRICK",
                    "minecraft:brick",
                    "minecraft:decorated_pot_side",
                ),
            ],
        )

    def test_missing_bootstrap_registration_fails_explicitly(self) -> None:
        source = parse_source(
            "fixture/Broken.java",
            b"""
package fixture;
class Broken {
  static final ResourceKey<Variant> ONE = key("one");
  static void bootstrap(BootstrapContext<Variant> context) {}
}
""",
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            r"fixture/Broken\.java: declaration/bootstrap mismatch: .*missing=\['ONE'\]",
        ):
            bootstrap_resource_key_entries(source, "Variant", {"key"}, 1)

    def test_painting_dimensions_and_optional_author_flag_come_from_calls(self) -> None:
        source = parse_source(
            "fixture/Paintings.java",
            b"""
package fixture;
class Paintings {
  static final ResourceKey<PaintingVariant> ONE = create("one");
  static final ResourceKey<PaintingVariant> TWO = create("two");
  static void bootstrap(BootstrapContext<PaintingVariant> context) {
    register(context, TWO, 4, 3, false); register(context, ONE, 1, 2);
  }
}
""",
        )
        entries = painting_variant_entries(source, 2)
        self.assertEqual(
            [
                (entry.resource_id, entry.width, entry.height, entry.has_author)
                for entry in entries
            ],
            [
                ("minecraft:two", 4, 3, False),
                ("minecraft:one", 1, 2, True),
            ],
        )

    def test_banner_translation_key_formula_is_validated_before_derivation(self) -> None:
        source = parse_source(
            "fixture/Banners.java",
            b"""
package fixture;
class Banners {
  static final ResourceKey<BannerPattern> BASE = create("base");
  static void bootstrap(BootstrapContext<BannerPattern> context) {
    register(context, BASE);
  }
  static void register(BootstrapContext<BannerPattern> context,
                       ResourceKey<BannerPattern> key) {
    context.register(key, new BannerPattern(
        key.identifier(),
        "block.minecraft.banner." + key.identifier().toShortLanguageKey()));
  }
}
""",
        )
        entry = banner_pattern_entries(source, 1)[0]
        self.assertEqual(entry.asset_id, "minecraft:base")
        self.assertEqual(entry.translation_key, "block.minecraft.banner.base")

    def test_banner_asset_formula_is_validated_before_derivation(self) -> None:
        source = parse_source(
            "fixture/Banners.java",
            b"""
package fixture;
class Banners {
  static final ResourceKey<BannerPattern> BASE = create("base");
  static void bootstrap(BootstrapContext<BannerPattern> context) {
    register(context, BASE);
  }
  static void register(BootstrapContext<BannerPattern> context,
                       ResourceKey<BannerPattern> key) {
    context.register(key, new BannerPattern(
        Identifier.withDefaultNamespace("wrong"),
        "block.minecraft.banner." + key.identifier().toShortLanguageKey()));
  }
}
""",
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError, "unsupported banner asset expression"
        ):
            banner_pattern_entries(source, 1)

    def test_enchantment_order_and_max_level_come_from_definition_calls(self) -> None:
        source = parse_source(
            "fixture/Enchantments.java",
            b"""
package fixture;
class Enchantments {
  static final ResourceKey<Enchantment> ONE = key("one");
  static final ResourceKey<Enchantment> TWO = key("two");
  static void bootstrap(BootstrapContext<Enchantment> context) {
    register(context, TWO, Enchantment.enchantment(
        Enchantment.definition(items, primary, 4, 5, min(), max(), 1, SLOT)));
    register(context, ONE, Enchantment.enchantment(
        Enchantment.definition(items, 2, 3, min(), max(), 1, SLOT))
        .withEffect(EFFECT, "fake definition(items, 0, 99)"));
  }
}
""",
        )
        entries = enchantment_entries(source, 2)
        self.assertEqual(
            [(entry.resource_id, entry.max_level) for entry in entries],
            [("minecraft:two", 5), ("minecraft:one", 3)],
        )

    def test_enchantment_tooltip_tag_must_close_over_generated_registry(self) -> None:
        enchantments = [
            EnchantmentEntry("ONE", "minecraft:one", 1),
            EnchantmentEntry("TWO", "minecraft:two", 1),
        ]
        resources = {
            "minecraft:tooltip_order": (
                "data/minecraft/tags/enchantment/tooltip_order.json",
                b'{"values":["minecraft:two","minecraft:one"]}',
            )
        }
        self.assertEqual(
            enchantment_tooltip_order(resources, enchantments, 2),
            ("minecraft:two", "minecraft:one"),
        )
        with self.assertRaisesRegex(
            BootstrapCatalogExtractionError,
            r"tooltip order/enchantment registry mismatch: unknown=\['minecraft:other'\], missing=\['minecraft:one'\]",
        ):
            enchantment_tooltip_order(
                {
                    "minecraft:tooltip_order": (
                        "data/minecraft/tags/enchantment/tooltip_order.json",
                        b'{"values":["minecraft:two","minecraft:other"]}',
                    )
                },
                enchantments,
                2,
            )

    def test_external_identifier_constants_and_alternate_registration_calls(self) -> None:
        identifiers = parse_source(
            "fixture/Temperature.java",
            b"""
package fixture;
interface Temperature {
  Identifier TEMPERATE = Identifier.withDefaultNamespace("temperate");
  Identifier COLD = Identifier.withDefaultNamespace("cold");
}
""",
        )
        variants = parse_source(
            "fixture/Variants.java",
            b"""
package fixture;
interface Variants {
  ResourceKey<Variant> TEMPERATE = key(Temperature.TEMPERATE);
  ResourceKey<Variant> COLD = key(Temperature.COLD);
  static void bootstrap(BootstrapContext<Variant> context) {
    registerSimple(context, TEMPERATE); register(context, COLD);
  }
}
""",
        )
        paths = {
            f"Temperature.{name}": path
            for name, path in identifier_constant_paths(identifiers).items()
        }
        entries = bootstrap_resource_key_entries(
            variants,
            "Variant",
            {"key"},
            2,
            referenced_paths=paths,
            registration_names={"register", "registerSimple"},
        )
        self.assertEqual(
            [entry.resource_id for entry in entries],
            ["minecraft:temperate", "minecraft:cold"],
        )


if __name__ == "__main__":
    unittest.main()
