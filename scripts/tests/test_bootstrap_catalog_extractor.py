from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bootstrap_catalog_extractor import (  # noqa: E402
    BootstrapCatalogExtractionError,
    banner_pattern_entries,
    bootstrap_resource_key_entries,
    enchantment_entries,
    painting_variant_entries,
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


if __name__ == "__main__":
    unittest.main()
