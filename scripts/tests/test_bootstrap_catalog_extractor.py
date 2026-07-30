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
    EnchantmentEntry,
    FarmAnimalVariantEntry,
    FrogVariantEntry,
    ModelAndTextureVariantEntry,
    WoodTypeEntry,
    banner_pattern_entries,
    bootstrap_resource_key_entries,
    decorated_pot_pattern_entries,
    enchantment_entries,
    enchantment_tooltip_order,
    farm_animal_variant_entries,
    frog_variant_entries,
    identifier_constant_paths,
    map_decoration_entries,
    model_and_texture_variant_entries,
    painting_variant_entries,
    potion_entries,
    sound_variant_entries,
    static_register_entries,
    trim_material_entries,
    wood_type_entries,
    cat_variant_entries,
    wolf_variant_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


class BootstrapCatalogExtractorTests(unittest.TestCase):
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

        self.assertEqual(
            [
                (
                    entry.constant_name,
                    entry.resource_id,
                    entry.asset_id,
                    entry.show_on_item_frame,
                )
                for entry in entries
            ],
            [
                ("PLAYER", "minecraft:player", "minecraft:player", False),
                (
                    "MANSION",
                    "minecraft:mansion",
                    "minecraft:woodland_mansion",
                    True,
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
