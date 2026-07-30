import importlib.util
import json
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
    def test_overworld_day_renderer_emits_typed_tracks_and_reverse_guard(self) -> None:
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
        self.assertIn(
            "production_sources_do_not_redeclare_overworld_day_visual_tracks",
            rendered,
        )
        self.assertIn('"VANILLA_LIGHTMAP_DAY_PERIOD_TICKS"', rendered)

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
        self.assertIn(
            "production_sources_do_not_redeclare_fixed_registry_identities",
            rendered,
        )

    def test_sign_wood_renderer_emits_catalog_queries_and_native_guard(self) -> None:
        rendered = GENERATOR.render_sign_wood_types(
            [
                GENERATOR.WoodTypeEntry("OAK", "oak"),
                GENERATOR.WoodTypeEntry("DARK_OAK", "dark_oak"),
            ]
        )

        self.assertIn("pub enum SignWoodKind", rendered)
        self.assertIn("SignWoodKind::DarkOak, \"dark_oak\"", rendered)
        self.assertIn("pub fn vanilla_sign_wood_kinds", rendered)
        self.assertIn("HANGING_SIGN_BACKGROUNDS", rendered)

    def test_registry_holder_coverage_requires_one_family_per_network_row(self) -> None:
        entry = GENERATOR.RegistryHolderConsumerEntry(
            "net/minecraft/example/Test.java",
            7,
            11,
            "net.minecraft.example.Test",
            "ByteBufCodecs.holderRegistry",
            "network_holder_reference",
            "Registries.ITEM",
            "minecraft:item",
        )
        locator = (
            "scripts/generate-bootstrap-catalog.py#"
            "def render_registry_holder_coverage_ledger"
        )
        family = {
            "id": "reference",
            "selector": {"kind": "network_holder_reference"},
            "status": "complete",
            "consumer": locator,
            "evidence": [locator],
            "decision": "covered by the focused generator fixture",
        }
        rendered = json.loads(
            GENERATOR.render_registry_holder_coverage_ledger(
                [entry], [family], Path("/unused")
            )
        )
        self.assertEqual(rendered["entries"][0]["family"], "reference")

        with self.assertRaisesRegex(ValueError, "expected one.*got"):
            GENERATOR.render_registry_holder_coverage_ledger(
                [entry], [family, {**family, "id": "duplicate"}], Path("/unused")
            )

    def test_registry_holder_coverage_selector_can_isolate_shared_source_by_resource(
        self,
    ) -> None:
        entries = [
            GENERATOR.RegistryHolderConsumerEntry(
                "net/minecraft/example/Shared.java",
                line,
                11,
                "net.minecraft.example.Shared",
                "ByteBufCodecs.holderRegistry",
                "network_holder_reference",
                registry,
                resource_id,
            )
            for line, registry, resource_id in [
                (7, "Registries.BANNER_PATTERN", "minecraft:banner_pattern"),
                (8, "Registries.ITEM", "minecraft:item"),
            ]
        ]
        selector = {
            "kind": "network_holder_reference",
            "resource_ids": ["minecraft:banner_pattern"],
        }
        self.assertTrue(
            GENERATOR.registry_holder_family_matches(entries[0], selector)
        )
        self.assertFalse(
            GENERATOR.registry_holder_family_matches(entries[1], selector)
        )
        catch_all = {
            "kind": "network_holder_reference",
            "exclude_resource_ids": ["minecraft:banner_pattern"],
        }
        self.assertFalse(
            GENERATOR.registry_holder_family_matches(entries[0], catch_all)
        )
        self.assertTrue(
            GENERATOR.registry_holder_family_matches(entries[1], catch_all)
        )

    def test_registry_holder_coverage_deferred_family_requires_restart_condition(
        self,
    ) -> None:
        entry = GENERATOR.RegistryHolderConsumerEntry(
            "net/minecraft/example/Test.java",
            7,
            11,
            "net.minecraft.example.Test",
            "TagKey.streamCodec",
            "network_tag",
            "Registries.ITEM",
            "minecraft:item",
        )
        locator = (
            "scripts/generate-bootstrap-catalog.py#"
            "def render_registry_holder_coverage_ledger"
        )
        family = {
            "id": "deferred-tag",
            "selector": {"kind": "network_tag"},
            "status": "deferred",
            "consumer": None,
            "evidence": [locator],
            "decision": "not scheduled",
        }
        with self.assertRaisesRegex(ValueError, "lacks a restart condition"):
            GENERATOR.render_registry_holder_coverage_ledger(
                [entry], [family], Path("/unused")
            )

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
