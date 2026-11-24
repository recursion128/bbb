import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location(
    "generate_bootstrap_catalog", SCRIPTS / "generate-bootstrap-catalog.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class BootstrapCatalogGeneratorTests(unittest.TestCase):
    def test_map_decoration_renderer_emits_track_count(self) -> None:
        rendered = GENERATOR.render_map_decorations(
            [
                GENERATOR.MapDecorationEntry(
                    "FRAME", "minecraft:frame", "minecraft:frame", True, True
                ),
                GENERATOR.MapDecorationEntry(
                    "TARGET_X",
                    "minecraft:target_x",
                    "minecraft:target_x",
                    True,
                    False,
                ),
            ]
        )

        self.assertIn("track_count: true", rendered)
        self.assertIn("track_count: false", rendered)
        self.assertIn("map_color: None", rendered)

    def test_persistent_registry_facts_reuse_the_generated_stat_catalogs(self) -> None:
        rendered = GENERATOR.render_persistent_registry_facts({}, {}, [])

        self.assertIn(
            "stat_types::{vanilla_custom_stat_by_resource_id, vanilla_stat_type_by_resource_id}",
            rendered,
        )
        self.assertIn(
            '"minecraft:custom_stat" => |resource_id| vanilla_custom_stat_by_resource_id(resource_id).is_some(),',
            rendered,
        )
        self.assertIn(
            '"minecraft:stat_type" => |resource_id| vanilla_stat_type_by_resource_id(resource_id).is_some(),',
            rendered,
        )
        self.assertIn(
            "pub fn vanilla_persistent_registry_available_26_1(registry_id: &str) -> bool",
            rendered,
        )
        self.assertIn(
            "stat_type_and_custom_stat_catalog_membership_is_complete", rendered
        )
        self.assertIn(
            'vanilla_persistent_registry_tag_members_26_1("minecraft:custom_stat", "minecraft:any"), PersistentRegistryTagMembers::Unavailable',
            rendered,
        )

    def test_loot_context_renderer_emits_typed_catalogs_and_mapping(self) -> None:
        rendered = GENERATOR.render_loot_context_types(
            [
                GENERATOR.LootContextParamEntry(
                    "THIS_ENTITY", "minecraft:this_entity"
                ),
                GENERATOR.LootContextParamEntry(
                    "ATTACKING_ENTITY", "minecraft:attacking_entity"
                ),
            ],
            [
                GENERATOR.LootContextEntityTargetEntry(
                    "THIS", "this", "THIS_ENTITY"
                ),
                GENERATOR.LootContextEntityTargetEntry(
                    "ATTACKER", "attacker", "ATTACKING_ENTITY"
                ),
            ],
            [
                GENERATOR.LootContextParamSetEntry(
                    "EMPTY", "minecraft:empty", (), ()
                ),
                GENERATOR.LootContextParamSetEntry(
                    "ENTITY",
                    "minecraft:entity",
                    ("THIS_ENTITY",),
                    ("ATTACKING_ENTITY",),
                ),
            ],
        )

        # The catalog lives in bbb-registry-codec and is read from
        # bbb-server-core, so the enum is crate-public rather than private.
        self.assertIn("pub enum VanillaLootContextParam", rendered)
        self.assertIn(
            "Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord", rendered
        )
        self.assertIn(
            'Self::ThisEntity => "minecraft:this_entity"', rendered
        )
        self.assertIn(
            "pub const VANILLA_LOOT_CONTEXT_PARAMS", rendered
        )
        self.assertIn(
            "pub struct VanillaLootEntityTarget", rendered
        )
        self.assertIn(
            'serialized_name: "attacker"', rendered
        )
        self.assertIn(
            "context_param: VanillaLootContextParam::AttackingEntity", rendered
        )
        self.assertIn(
            "pub fn vanilla_loot_entity_target", rendered
        )
        self.assertIn("pub struct VanillaLootContextParamSet", rendered)
        self.assertIn("pub const VANILLA_LOOT_CONTEXT_PARAM_SETS", rendered)
        self.assertIn('resource_id: "minecraft:empty"', rendered)
        self.assertIn("pub fn vanilla_loot_context_param_set", rendered)
        self.assertIn("fn allows(&self, param: VanillaLootContextParam)", rendered)
        self.assertIn("LootContextParams.java", rendered)
        self.assertIn("LootContextParamSets.java", rendered)
        self.assertIn("LootContext.java", rendered)

    def test_loot_context_renderer_rejects_unknown_target_param(self) -> None:
        with self.assertRaisesRegex(ValueError, "unknown params"):
            GENERATOR.render_loot_context_types(
                [
                    GENERATOR.LootContextParamEntry(
                        "THIS_ENTITY", "minecraft:this_entity"
                    )
                ],
                [
                    GENERATOR.LootContextEntityTargetEntry(
                        "ATTACKER", "attacker", "ATTACKING_ENTITY"
                    )
                ],
                [],
            )

    def test_loot_context_renderer_rejects_unknown_param_set_param(self) -> None:
        with self.assertRaisesRegex(ValueError, "param sets reference unknown params"):
            GENERATOR.render_loot_context_types(
                [
                    GENERATOR.LootContextParamEntry(
                        "THIS_ENTITY", "minecraft:this_entity"
                    )
                ],
                [],
                [
                    GENERATOR.LootContextParamSetEntry(
                        "ENTITY",
                        "minecraft:entity",
                        ("UNKNOWN",),
                        (),
                    )
                ],
            )

    def test_enchantment_codec_renderer_emits_typed_families_and_lookups(self) -> None:
        rendered = GENERATOR.render_enchantment_codec_types(
            [
                GENERATOR.RegistryCodecEntry(
                    "minecraft:clamped", "LevelBasedValue.Clamped"
                ),
                GENERATOR.RegistryCodecEntry(
                    "minecraft:lookup", "LevelBasedValue.Lookup"
                ),
            ],
            [
                GENERATOR.RegistryCodecEntry("minecraft:add", "AddValue"),
                GENERATOR.RegistryCodecEntry(
                    "minecraft:all_of", "AllOf.ValueEffects"
                ),
            ],
            [
                GENERATOR.EnchantmentEffectComponentEntry(
                    "minecraft:damage",
                    "ConditionalValueEffect",
                    "ENCHANTED_DAMAGE",
                ),
                GENERATOR.EnchantmentEffectComponentEntry(
                    "minecraft:prevent_armor_change", "Unit", None
                ),
            ],
            [GENERATOR.RegistryCodecEntry("minecraft:damage_entity", "DamageEntity")],
            [
                GENERATOR.RegistryCodecEntry(
                    "minecraft:attribute", "EnchantmentAttributeEffect"
                )
            ],
            [
                GENERATOR.LootContextParamSetEntry(
                    "ENCHANTED_DAMAGE",
                    "minecraft:enchanted_damage",
                    (),
                    (),
                )
            ],
        )

        self.assertIn("pub(crate) enum LevelBasedValueCodecFamily", rendered)
        self.assertIn("LevelBasedValueCodecFamily::Clamped", rendered)
        self.assertIn("Self::Lookup => \"LevelBasedValue.Lookup\"", rendered)
        self.assertIn(
            "pub(crate) enum EnchantmentValueEffectCodecFamily", rendered
        )
        self.assertIn("EnchantmentValueEffectCodecFamily::AllOfValueEffects", rendered)
        self.assertIn(
            "pub(crate) fn enchantment_value_effect_codec_type", rendered
        )
        self.assertIn(
            "enchantment_entity_effect_codec_type_preserves_order", rendered
        )
        self.assertIn("pub(crate) enum EnchantmentEffectValidationContext", rendered)
        self.assertIn("EnchantmentEffectValidationContext::EnchantedDamage", rendered)
        self.assertIn("loot_context_param_set_resource_id", rendered)
        self.assertIn('Self::EnchantedDamage => "minecraft:enchanted_damage"', rendered)
        self.assertIn("pub(crate) enum EnchantmentLocationBasedEffectCodecFamily", rendered)

    def test_enchantment_codec_renderer_requires_source_param_set_binding(self) -> None:
        with self.assertRaisesRegex(
            ValueError, "validation contexts lack loot context param sets"
        ):
            GENERATOR.render_enchantment_codec_types(
                [],
                [],
                [
                    GENERATOR.EnchantmentEffectComponentEntry(
                        "minecraft:damage",
                        "ConditionalValueEffect",
                        "ENCHANTED_DAMAGE",
                    )
                ],
                [],
                [],
                [],
            )

    def test_dispatch_renderer_emits_public_typed_catalogs(self) -> None:
        rendered = GENERATOR.render_codec_dispatch_types(
            [
                (
                    "IntProviderCodecFamily",
                    "IntProviderCodecType",
                    "VANILLA_INT_PROVIDER_CODEC_TYPES",
                    "int_provider_codec_type",
                    [
                        GENERATOR.RegistryCodecEntry(
                            "minecraft:constant", "ConstantInt"
                        ),
                        GENERATOR.RegistryCodecEntry(
                            "minecraft:uniform", "UniformInt"
                        ),
                    ],
                )
            ]
        )
        self.assertIn("pub enum IntProviderCodecFamily", rendered)
        self.assertIn("pub struct IntProviderCodecType", rendered)
        self.assertIn("pub resource_id: &'static str", rendered)
        self.assertIn("pub const VANILLA_INT_PROVIDER_CODEC_TYPES", rendered)
        self.assertIn("pub fn int_provider_codec_type", rendered)
        self.assertIn("int_provider_codec_type_preserves_order", rendered)

    def test_dispatch_renderer_emits_placement_modifier_catalog(self) -> None:
        rendered = GENERATOR.render_codec_dispatch_types(
            [
                (
                    "PlacementModifierCodecFamily",
                    "PlacementModifierCodecType",
                    "VANILLA_PLACEMENT_MODIFIER_CODEC_TYPES",
                    "placement_modifier_codec_type",
                    [
                        GENERATOR.RegistryCodecEntry(
                            "minecraft:block_predicate_filter", "BlockPredicateFilter"
                        ),
                        GENERATOR.RegistryCodecEntry(
                            "minecraft:fixed_placement", "FixedPlacement"
                        ),
                    ],
                )
            ]
        )
        self.assertIn("pub enum PlacementModifierCodecFamily", rendered)
        self.assertIn("PlacementModifierCodecFamily::BlockPredicateFilter", rendered)
        self.assertIn("pub struct PlacementModifierCodecType", rendered)
        self.assertIn("pub const VANILLA_PLACEMENT_MODIFIER_CODEC_TYPES", rendered)
        self.assertIn("pub fn placement_modifier_codec_type", rendered)
        self.assertIn("placement_modifier_codec_type_preserves_order", rendered)

    def test_dispatch_renderer_emits_configured_feature_catalog_with_repeated_families(
        self,
    ) -> None:
        rendered = GENERATOR.render_codec_dispatch_types(
            [],
            [
                GENERATOR.ConfiguredFeatureCodecEntry(
                    "minecraft:no_op",
                    "NoOpFeature",
                    "NoneFeatureConfiguration",
                ),
                GENERATOR.ConfiguredFeatureCodecEntry(
                    "minecraft:bonus_chest",
                    "BonusChestFeature",
                    "NoneFeatureConfiguration",
                ),
                GENERATOR.ConfiguredFeatureCodecEntry(
                    "minecraft:lake",
                    "LakeFeature",
                    "LakeFeature.Configuration",
                ),
            ],
        )
        self.assertIn("pub enum ConfiguredFeatureCodecFamily", rendered)
        self.assertEqual(rendered.count("    NoneFeatureConfiguration,"), 1)
        self.assertIn("    LakeFeatureConfiguration,", rendered)
        self.assertIn(
            'Self::LakeFeatureConfiguration => "LakeFeature.Configuration"', rendered
        )
        self.assertIn("pub struct ConfiguredFeatureCodecType", rendered)
        self.assertIn("pub(crate) feature_owner: &'static str", rendered)
        self.assertIn("pub const VANILLA_CONFIGURED_FEATURE_CODEC_TYPES", rendered)
        self.assertIn("pub fn configured_feature_codec_type", rendered)
        self.assertIn(
            "configured_feature_codec_type_preserves_registration_order_and_families",
            rendered,
        )

    def test_configured_feature_renderer_rejects_variant_collisions(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate Rust enum variants"):
            GENERATOR.configured_feature_codec_family_variants(
                [
                    GENERATOR.ConfiguredFeatureCodecEntry(
                        "minecraft:a", "FeatureA", "Outer.Same"
                    ),
                    GENERATOR.ConfiguredFeatureCodecEntry(
                        "minecraft:b", "FeatureB", "OuterSame"
                    ),
                ]
            )

    def test_codec_family_renderer_rejects_owner_or_variant_drift(self) -> None:
        with self.assertRaisesRegex(ValueError, "outside LevelBasedValue"):
            GENERATOR.codec_family_variants(
                [GENERATOR.RegistryCodecEntry("minecraft:add", "AddValue")],
                common_owner="LevelBasedValue",
            )
        with self.assertRaisesRegex(ValueError, "duplicate Rust enum variants"):
            GENERATOR.codec_family_variants(
                [
                    GENERATOR.RegistryCodecEntry("minecraft:a", "Outer.Same"),
                    GENERATOR.RegistryCodecEntry("minecraft:b", "OuterSame"),
                ],
                common_owner=None,
            )

    def test_overworld_day_renderer_emits_typed_tracks(self) -> None:
        configurations = [
            ("FOG_COLOR", "ColorModifier.MULTIPLY_RGB", "i32", -1),
            ("SKY_COLOR", "ColorModifier.MULTIPLY_RGB", "i32", -2),
            ("SKY_LIGHT_COLOR", "ColorModifier.MULTIPLY_RGB", "i32", -3),
            ("SKY_LIGHT_FACTOR", "FloatModifier.MULTIPLY", "f32", 0x3F80_0000),
            ("SKY_LIGHT_LEVEL", "FloatModifier.MULTIPLY", "f32", 0x3E80_0000),
            ("SUNRISE_SUNSET_COLOR", "override", "i32", 4),
            ("STAR_BRIGHTNESS", "FloatModifier.MAXIMUM", "f32", 0x3F00_0000),
            ("CLOUD_COLOR", "ColorModifier.MULTIPLY_ARGB", "i32", -5),
        ]
        rendered = GENERATOR.render_overworld_day_timeline(
            GENERATOR.OverworldDayTimelineEntry(
                24_000,
                tuple(
                    GENERATOR.TimelineTrackEntry(
                        attribute,
                        modifier,
                        value_kind,
                        (GENERATOR.TimelineKeyframeEntry(index, value),),
                    )
                    for index, (attribute, modifier, value_kind, value) in enumerate(
                        configurations
                    )
                ),
                -8_750_337,
            )
        )

        self.assertIn(
            "pub(super) const VANILLA_OVERWORLD_DAY_PERIOD_TICKS: i64 = 24000",
            rendered,
        )
        self.assertIn(
            "pub(super) const VANILLA_OVERWORLD_SKY_LIGHT_FACTOR_KEYFRAMES",
            rendered,
        )
        self.assertIn("f32::from_bits(0x3f800000)", rendered)

    def test_registry_data_renderer_emits_typed_fixed_registry_identities(self) -> None:
        rendered = GENERATOR.render_registry_data_types(
            [],
            [],
            [],
            [],
            [
                GENERATOR.BootstrapEntry("EMPTY", "minecraft:empty"),
                GENERATOR.BootstrapEntry("WATER", "minecraft:water"),
            ],
            [
                GENERATOR.BootstrapEntry("CHAT", "minecraft:chat"),
                GENERATOR.BootstrapEntry("EMOTE_COMMAND", "minecraft:emote_command"),
            ],
        )

        self.assertIn("pub enum VanillaFluidIdentity", rendered)
        self.assertIn("VanillaFluidIdentity::Water, \"minecraft:water\"", rendered)
        self.assertIn("pub enum VanillaChatTypeIdentity", rendered)
        self.assertIn(
            "VanillaChatTypeIdentity::EmoteCommand, \"minecraft:emote_command\"",
            rendered,
        )
        self.assertIn("pub struct SynchronizedRegistryDataType", rendered)
        self.assertIn(
            "Option<&'static SynchronizedRegistryDataType>", rendered
        )

    def test_synchronized_registry_data_catalog_joins_codecs_by_resource_id(
        self,
    ) -> None:
        worldgen = [
            GENERATOR.RegistryDataCatalogEntry(
                "BIOME", "minecraft:worldgen/biome", "Biome.DIRECT_CODEC", None
            ),
            GENERATOR.RegistryDataCatalogEntry(
                "WOLF_VARIANT",
                "minecraft:wolf_variant",
                "WolfVariant.DIRECT_CODEC",
                "RegistryValidator.nonEmpty ()",
            ),
        ]
        synchronized = [
            GENERATOR.RegistryDataCatalogEntry(
                "WOLF_VARIANT",
                "minecraft:wolf_variant",
                "WolfVariant.NETWORK_CODEC",
                "RegistryValidator.nonEmpty ()",
            ),
            GENERATOR.RegistryDataCatalogEntry(
                "BIOME", "minecraft:worldgen/biome", "Biome.NETWORK_CODEC", None
            ),
        ]

        paired = GENERATOR.synchronized_registry_data_catalog(
            worldgen, synchronized
        )

        self.assertEqual(
            [entry.resource_id for entry in paired],
            ["minecraft:wolf_variant", "minecraft:worldgen/biome"],
        )
        self.assertEqual(paired[0].persistent_codec, "WolfVariant.DIRECT_CODEC")
        self.assertEqual(paired[0].network_codec, "WolfVariant.NETWORK_CODEC")
        self.assertEqual(
            paired[0].validator, "RegistryValidator.nonEmpty ()"
        )

    def test_synchronized_registry_data_catalog_rejects_missing_or_duplicate_ids(
        self,
    ) -> None:
        biome = GENERATOR.RegistryDataCatalogEntry(
            "BIOME", "minecraft:worldgen/biome", "Biome.DIRECT_CODEC", None
        )
        network_biome = GENERATOR.RegistryDataCatalogEntry(
            "BIOME", "minecraft:worldgen/biome", "Biome.NETWORK_CODEC", None
        )
        missing = GENERATOR.RegistryDataCatalogEntry(
            "CHAT_TYPE", "minecraft:chat_type", "ChatType.DIRECT_CODEC", None
        )

        with self.assertRaisesRegex(ValueError, "missing from WORLDGEN_REGISTRIES"):
            GENERATOR.synchronized_registry_data_catalog([biome], [missing])
        with self.assertRaisesRegex(
            ValueError, "WORLDGEN_REGISTRIES contains duplicate resource id"
        ):
            GENERATOR.synchronized_registry_data_catalog(
                [biome, biome], [network_biome]
            )
        with self.assertRaisesRegex(
            ValueError, "SYNCHRONIZED_REGISTRIES contains duplicate resource id"
        ):
            GENERATOR.synchronized_registry_data_catalog(
                [biome], [network_biome, network_biome]
            )

    def test_sign_wood_renderer_emits_catalog_queries(self) -> None:
        rendered = GENERATOR.render_sign_wood_types(
            [
                GENERATOR.WoodTypeEntry("OAK", "oak"),
                GENERATOR.WoodTypeEntry("DARK_OAK", "dark_oak"),
            ]
        )

        self.assertIn("pub enum SignWoodKind", rendered)
        self.assertIn("SignWoodKind::DarkOak, \"dark_oak\"", rendered)
        self.assertIn("pub fn vanilla_sign_wood_kinds", rendered)

    def test_vanilla_texture_dimensions_reads_png_ihdr(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            texture = root / "assets/minecraft/textures/entity/test.png"
            texture.parent.mkdir(parents=True)
            texture.write_bytes(
                b"\x89PNG\r\n\x1a\n"
                + b"\x00\x00\x00\x0dIHDR"
                + (37).to_bytes(4, "big")
                + (19).to_bytes(4, "big")
            )

            self.assertEqual(
                GENERATOR.vanilla_texture_dimensions(
                    root, "minecraft:entity/test"
                ),
                (37, 19),
            )

            texture.write_bytes(
                b"\x89PNG\r\n\x1a\n"
                + b"\x00\x00\x00\x0dIHDR"
                + (0).to_bytes(4, "big")
                + (19).to_bytes(4, "big")
            )
            with self.assertRaisesRegex(ValueError, "zero-sized PNG texture"):
                GENERATOR.vanilla_texture_dimensions(root, "minecraft:entity/test")

    def test_entity_texture_asset_id_qualifies_bootstrap_asset(self) -> None:
        self.assertEqual(
            GENERATOR.entity_texture_asset_id("minecraft:angler", "decorated_pot"),
            "minecraft:entity/decorated_pot/angler",
        )
        with self.assertRaisesRegex(ValueError, "one minecraft path segment"):
            GENERATOR.entity_texture_asset_id("example:angler", "decorated_pot")

    def test_variant_renderers_use_supplied_texture_dimensions(self) -> None:
        pot = GENERATOR.DecoratedPotPatternEntry(
            "ANGLER",
            "minecraft:angler",
            "ANGLER_POTTERY_SHERD",
            "minecraft:angler_pottery_sherd",
            "minecraft:angler_pottery_pattern",
        )
        blank_pot = GENERATOR.DecoratedPotPatternEntry(
            "BLANK",
            "minecraft:blank",
            "BRICK",
            "minecraft:brick",
            "minecraft:decorated_pot_side",
        )
        rendered_pot = GENERATOR.render_decorated_pot_patterns(
            [pot, blank_pot],
            {pot.asset_id: (17, 19), blank_pot.asset_id: (23, 29)},
        )
        self.assertIn("size: [17, 19]", rendered_pot)
        self.assertIn("size: [23, 29]", rendered_pot)

        banner = GENERATOR.BannerPatternEntry(
            "BASE",
            "minecraft:base",
            "minecraft:base",
            "block.minecraft.banner.base",
        )
        rendered_banner = GENERATOR.render_banner_patterns(
            [banner], {banner.asset_id: (67, 69)}
        )
        self.assertIn("size: [67, 69]", rendered_banner)
        self.assertIn("pub(crate) fn banner_pattern_texture_ref", rendered_banner)

        cat = GENERATOR.CatVariantEntry(
            "TABBY",
            "minecraft:tabby",
            "minecraft:entity/cat/adult",
            "minecraft:entity/cat/baby",
        )
        rendered_cat = GENERATOR.render_cat_variants(
            [cat],
            {
                cat.adult_asset_id: (71, 41),
                cat.baby_asset_id: (43, 23),
            },
        )
        self.assertIn("size: [71, 41]", rendered_cat)
        self.assertIn("size: [43, 23]", rendered_cat)

        wolf = GENERATOR.WolfVariantEntry(
            "PALE",
            "minecraft:pale",
            "wolf",
            tuple(f"minecraft:entity/wolf/asset_{index}" for index in range(6)),
        )
        wolf_dimensions = {
            asset_id: (80 + index, 40 + index)
            for index, asset_id in enumerate(wolf.asset_ids)
        }
        rendered_wolf = GENERATOR.render_wolf_variants([wolf], wolf_dimensions)
        for size in wolf_dimensions.values():
            self.assertIn(f"size: [{size[0]}, {size[1]}]", rendered_wolf)

        resources = [
            GENERATOR.BootstrapEntry("TEMPERATE", "minecraft:temperate"),
            GENERATOR.BootstrapEntry("WARM", "minecraft:warm"),
            GENERATOR.BootstrapEntry("COLD", "minecraft:cold"),
        ]
        catalogs = [
            (name, resources)
            for name in (
                "VANILLA_CHICKEN_VARIANT_KEYS",
                "VANILLA_COW_VARIANT_KEYS",
                "VANILLA_PIG_VARIANT_KEYS",
                "VANILLA_FROG_VARIANT_KEYS",
            )
        ]
        farm_catalogs = []
        dimensions = {}
        for family in ("Chicken", "Cow", "Pig"):
            entries = []
            for index, resource in enumerate(resources):
                adult = f"minecraft:entity/{family.lower()}/{resource.constant_name.lower()}"
                baby = f"{adult}_baby"
                dimensions[adult] = (50 + index, 30 + index)
                dimensions[baby] = (25 + index, 15 + index)
                entries.append(
                    GENERATOR.FarmAnimalVariantEntry(
                        resource.constant_name,
                        resource.resource_id,
                        "NORMAL",
                        adult,
                        baby,
                    )
                )
            farm_catalogs.append((family, entries))
        frogs = [
            GENERATOR.FrogVariantEntry(
                resource.constant_name,
                resource.resource_id,
                f"minecraft:entity/frog/{resource.constant_name.lower()}",
            )
            for resource in resources
        ]
        for index, frog in enumerate(frogs):
            dimensions[frog.asset_id] = (91 + index, 61 + index)

        rendered_temperature = GENERATOR.render_temperature_animal_variants(
            catalogs, farm_catalogs, frogs, dimensions
        )
        for frog in frogs:
            size = dimensions[frog.asset_id]
            self.assertIn(f"size: [{size[0]}, {size[1]}]", rendered_temperature)


if __name__ == "__main__":
    unittest.main()
