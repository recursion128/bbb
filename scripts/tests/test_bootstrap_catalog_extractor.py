from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bootstrap_catalog_extractor import (  # noqa: E402
    BootstrapCatalogExtractionError,
    EnchantmentEntry,
    banner_pattern_entries,
    bootstrap_resource_key_entries,
    enchantment_entries,
    enchantment_tooltip_order,
    identifier_constant_paths,
    painting_variant_entries,
    potion_entries,
    static_register_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


class BootstrapCatalogExtractorTests(unittest.TestCase):
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
      "mixed", new Potion("mixed",
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
                    [
                        (effect.constant_name, effect.duration, effect.amplifier)
                        for effect in entry.effects
                    ],
                )
                for entry in entries
            ],
            [
                ("WATER", "minecraft:water", []),
                ("SPEED", "minecraft:speed", [("SPEED", 3600, 0)]),
                (
                    "MIXED",
                    "minecraft:mixed",
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
        self.assertEqual(entry.translation_key, "block.minecraft.banner.base")

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
