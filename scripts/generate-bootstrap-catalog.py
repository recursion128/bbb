#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 bootstrap resource-order catalogs."""

from __future__ import annotations

import argparse
import difflib
import os
import re
import struct
import subprocess
import sys
from pathlib import Path
from typing import Any

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
from bootstrap_catalog_extractor import (
    BannerPatternEntry,
    BootstrapEntry,
    CatVariantEntry,
    ConfiguredFeatureCodecEntry,
    DecoratedPotPatternEntry,
    EnchantmentEffectComponentEntry,
    EnchantmentEntry,
    FarmAnimalVariantEntry,
    FrogVariantEntry,
    LootContextEntityTargetEntry,
    LootContextParamEntry,
    LootContextParamSetEntry,
    MapDecorationEntry,
    ModelAndTextureVariantEntry,
    OverworldDayTimelineEntry,
    PaintingVariantEntry,
    PotionEntry,
    RegistryCodecEntry,
    TrimMaterialEntry,
    WolfVariantEntry,
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
    map_color_constants,
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
from bootstrap_catalog_extractor import (
    TimelineKeyframeEntry as TimelineKeyframeEntry,
)
from bootstrap_catalog_extractor import (
    TimelineTrackEntry as TimelineTrackEntry,
)
from gameplay_id_catalog_extractor import mob_effect_entries
from registry_data_catalog_extractor import (
    BuiltInRegistryCatalogEntry,
    RegistryDataCatalogEntry,
    built_in_registry_catalog,
    registry_data_catalog,
)
from sound_event_catalog_extractor import (
    SoundVariantSoundEntry,
    animal_sound_variant_sound_entries,
)
from vanilla_java_source import parse_source
from vanilla_tag_source import (
    VanillaTagSourceError,
    load_tag_resources,
    resolve_all_tag_values,
)

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/enchantment_types.rs"
)
DEFAULT_ENCHANTMENT_CODEC_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-registry-codec/src/enchantment_codec_types.rs"
)
DEFAULT_CODEC_DISPATCH_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-registry-codec/src/codec_dispatch_types.rs"
)
DEFAULT_LOOT_CONTEXT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-registry-codec/src/loot_context_types.rs"
)
DEFAULT_ENCHANTMENT_BRIDGE_OUTPUT = (
    REPO_ROOT / "crates/bbb-client-world/src/registries/enchantments/generated_26_1.rs"
)
DEFAULT_PERSISTENT_REGISTRY_FACTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/persistent_registry_facts_26_1.rs"
)
DEFAULT_PERSISTENT_REGISTRY_TAGS_BIN_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/data/persistent_registry_tags_26_1.bin"
)
DEFAULT_POTION_CATALOG_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/potion_types.rs"
)
DEFAULT_PAINTING_VARIANT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/painting_variant_types.rs"
)
DEFAULT_PAINTING_VARIANTS_OUTPUT = REPO_ROOT / "crates/bbb-client-world/src/painting_variants.rs"
DEFAULT_DECORATED_POT_PATTERNS_OUTPUT = (
    REPO_ROOT / "crates/bbb-client-render/src/render_core/decorated_pot_patterns.rs"
)
DEFAULT_BANNER_PATTERNS_OUTPUT = (
    REPO_ROOT / "crates/bbb-client-render/src/entity_models/banner_patterns.rs"
)
DEFAULT_BANNER_PATTERN_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/banner_pattern_types.rs"
)
DEFAULT_VILLAGER_PROFESSIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/villager_model_professions.rs"
)
DEFAULT_VILLAGER_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/villager_model_types.rs"
)
DEFAULT_CAT_VARIANTS_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/cat_variants.rs"
DEFAULT_WOLF_VARIANTS_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/wolf_variants.rs"
DEFAULT_TEMPERATURE_ANIMAL_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/temperature_animal_variants.rs"
)
DEFAULT_ZOMBIE_NAUTILUS_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/zombie_nautilus_variants.rs"
)
DEFAULT_ANIMAL_SOUND_VARIANT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/animal_sound_variant_types.rs"
)
DEFAULT_ANIMAL_SOUND_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-client-world/src/entities/sound_variants.rs"
)
DEFAULT_INSTRUMENTS_OUTPUT = REPO_ROOT / "crates/bbb-client-world/src/registries/instruments.rs"
DEFAULT_TRIM_PATTERNS_OUTPUT = REPO_ROOT / "crates/bbb-client-world/src/registries/trim_patterns.rs"
DEFAULT_TRIM_MATERIALS_OUTPUT = REPO_ROOT / "crates/bbb-client-world/src/registries/trim_materials.rs"
DEFAULT_MAP_DECORATIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/map_decorations/generated_26_1.rs"
)
DEFAULT_REGISTRY_DATA_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/registry_data_types.rs"
)
DEFAULT_SIGN_WOOD_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-client-world/src/sign_blocks/generated_26_1.rs"
)
DEFAULT_OVERWORLD_DAY_TIMELINE_OUTPUT = (
    REPO_ROOT / "crates/bbb-client/src/runtime/overworld_day_timeline.rs"
)


class SynchronizedRegistryDataCatalogEntry:
    def __init__(
        self,
        constant_name: str,
        resource_id: str,
        persistent_codec: str,
        network_codec: str,
        validator: str | None,
    ) -> None:
        self.constant_name = constant_name
        self.resource_id = resource_id
        self.persistent_codec = persistent_codec
        self.network_codec = network_codec
        self.validator = validator


VANILLA_SYNCHRONIZED_REGISTRY_ORDER_26_1 = (
    "minecraft:worldgen/biome",
    "minecraft:chat_type",
    "minecraft:trim_pattern",
    "minecraft:trim_material",
    "minecraft:wolf_variant",
    "minecraft:wolf_sound_variant",
    "minecraft:pig_variant",
    "minecraft:pig_sound_variant",
    "minecraft:frog_variant",
    "minecraft:cat_variant",
    "minecraft:cat_sound_variant",
    "minecraft:cow_sound_variant",
    "minecraft:cow_variant",
    "minecraft:chicken_sound_variant",
    "minecraft:chicken_variant",
    "minecraft:zombie_nautilus_variant",
    "minecraft:painting_variant",
    "minecraft:dimension_type",
    "minecraft:damage_type",
    "minecraft:banner_pattern",
    "minecraft:enchantment",
    "minecraft:jukebox_song",
    "minecraft:instrument",
    "minecraft:test_environment",
    "minecraft:test_instance",
    "minecraft:dialog",
    "minecraft:world_clock",
    "minecraft:timeline",
)

TRIM_MATERIALS_SOURCE = Path(
    "net/minecraft/world/item/equipment/trim/TrimMaterials.java"
)
MATERIAL_ASSET_GROUP_SOURCE = Path(
    "net/minecraft/world/item/equipment/trim/MaterialAssetGroup.java"
)
TRIM_PATTERNS_SOURCE = Path(
    "net/minecraft/world/item/equipment/trim/TrimPatterns.java"
)
POTIONS_SOURCE = Path("net/minecraft/world/item/alchemy/Potions.java")
MOB_EFFECTS_SOURCE = Path("net/minecraft/world/effect/MobEffects.java")
VILLAGER_TYPES_SOURCE = Path(
    "net/minecraft/world/entity/npc/villager/VillagerType.java"
)
VILLAGER_PROFESSIONS_SOURCE = Path(
    "net/minecraft/world/entity/npc/villager/VillagerProfession.java"
)
WOLF_SOUND_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/wolf/WolfSoundVariants.java"
)
SOUND_EVENTS_SOURCE = Path("net/minecraft/sounds/SoundEvents.java")
WOLF_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/wolf/WolfVariants.java"
)
INSTRUMENTS_SOURCE = Path("net/minecraft/world/item/Instruments.java")
BANNER_PATTERNS_SOURCE = Path(
    "net/minecraft/world/level/block/entity/BannerPatterns.java"
)
PAINTING_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/decoration/painting/PaintingVariants.java"
)
DECORATED_POT_PATTERNS_SOURCE = Path(
    "net/minecraft/world/level/block/entity/DecoratedPotPatterns.java"
)
ITEMS_SOURCE = Path("net/minecraft/world/item/Items.java")
ENCHANTMENTS_SOURCE = Path(
    "net/minecraft/world/item/enchantment/Enchantments.java"
)
LEVEL_BASED_VALUE_SOURCE = Path(
    "net/minecraft/world/item/enchantment/LevelBasedValue.java"
)
ENCHANTMENT_VALUE_EFFECT_SOURCE = Path(
    "net/minecraft/world/item/enchantment/effects/EnchantmentValueEffect.java"
)
ENCHANTMENT_EFFECT_COMPONENTS_SOURCE = Path(
    "net/minecraft/world/item/enchantment/EnchantmentEffectComponents.java"
)
ENCHANTMENT_ENTITY_EFFECT_SOURCE = Path(
    "net/minecraft/world/item/enchantment/effects/EnchantmentEntityEffect.java"
)
ENCHANTMENT_LOCATION_BASED_EFFECT_SOURCE = Path(
    "net/minecraft/world/item/enchantment/effects/EnchantmentLocationBasedEffect.java"
)
LOOT_ITEM_CONDITIONS_SOURCE = Path(
    "net/minecraft/world/level/storage/loot/predicates/LootItemConditions.java"
)
LOOT_CONTEXT_PARAMS_SOURCE = Path(
    "net/minecraft/world/level/storage/loot/parameters/LootContextParams.java"
)
LOOT_CONTEXT_PARAM_SETS_SOURCE = Path(
    "net/minecraft/world/level/storage/loot/parameters/LootContextParamSets.java"
)
LOOT_CONTEXT_SOURCE = Path("net/minecraft/world/level/storage/loot/LootContext.java")
NUMBER_PROVIDERS_SOURCE = Path(
    "net/minecraft/world/level/storage/loot/providers/number/NumberProviders.java"
)
FLOAT_PROVIDERS_SOURCE = Path("net/minecraft/util/valueproviders/FloatProviders.java")
INT_PROVIDERS_SOURCE = Path("net/minecraft/util/valueproviders/IntProviders.java")
SCOREBOARD_NAME_PROVIDERS_SOURCE = Path(
    "net/minecraft/world/level/storage/loot/providers/score/ScoreboardNameProviders.java"
)
BLOCK_PREDICATE_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/blockpredicates/BlockPredicateType.java"
)
BLOCK_STATE_PROVIDER_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/feature/stateproviders/BlockStateProviderType.java"
)
ENTITY_SUB_PREDICATES_SOURCE = Path(
    "net/minecraft/advancements/criterion/EntitySubPredicates.java"
)
WORLD_CARVER_SOURCE = Path(
    "net/minecraft/world/level/levelgen/carver/WorldCarver.java"
)
HEIGHT_PROVIDER_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/heightproviders/HeightProviderType.java"
)
PLACEMENT_MODIFIER_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/placement/PlacementModifierType.java"
)
RULE_TEST_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/structure/templatesystem/RuleTestType.java"
)
TRUNK_PLACER_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/feature/trunkplacers/TrunkPlacerType.java"
)
FOLIAGE_PLACER_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/feature/foliageplacers/FoliagePlacerType.java"
)
TREE_DECORATOR_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/feature/treedecorators/TreeDecoratorType.java"
)
ROOT_PLACER_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/feature/rootplacers/RootPlacerType.java"
)
FEATURE_SIZE_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/feature/featuresize/FeatureSizeType.java"
)
STRUCTURE_PROCESSOR_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/structure/templatesystem/StructureProcessorType.java"
)
POS_RULE_TEST_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/structure/templatesystem/PosRuleTestType.java"
)
RULE_BLOCK_ENTITY_MODIFIER_TYPE_SOURCE = Path(
    "net/minecraft/world/level/levelgen/structure/templatesystem/rule/blockentity/RuleBlockEntityModifierType.java"
)
FEATURE_SOURCE = Path("net/minecraft/world/level/levelgen/feature/Feature.java")
ENCHANTMENT_TAG_SOURCE_DIR = Path("data/minecraft/tags/enchantment")
PERSISTENT_REGISTRY_TAG_SOURCE_DIRS = {
    "minecraft:block": Path("data/minecraft/tags/block"),
    "minecraft:damage_type": Path("data/minecraft/tags/damage_type"),
    "minecraft:enchantment": ENCHANTMENT_TAG_SOURCE_DIR,
    "minecraft:entity_type": Path("data/minecraft/tags/entity_type"),
    "minecraft:fluid": Path("data/minecraft/tags/fluid"),
    "minecraft:painting_variant": Path("data/minecraft/tags/painting_variant"),
    "minecraft:banner_pattern": Path("data/minecraft/tags/banner_pattern"),
    "minecraft:item": Path("data/minecraft/tags/item"),
    "minecraft:instrument": Path("data/minecraft/tags/instrument"),
}
PERSISTENT_REGISTRY_ENTRY_SOURCE_DIRS = {
    "minecraft:dimension_type": Path("data/minecraft/dimension_type"),
    "minecraft:trim_material": Path("data/minecraft/trim_material"),
    "minecraft:trim_pattern": Path("data/minecraft/trim_pattern"),
    "minecraft:instrument": Path("data/minecraft/instrument"),
}
CHUNK_GENERATORS_SOURCE = Path(
    "net/minecraft/world/level/chunk/ChunkGenerators.java"
)
TEMPERATURE_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/TemperatureVariants.java"
)
CAT_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/feline/CatVariants.java"
)
CAT_SOUND_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/feline/CatSoundVariants.java"
)
CHICKEN_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/chicken/ChickenVariants.java"
)
CHICKEN_SOUND_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/chicken/ChickenSoundVariants.java"
)
COW_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/cow/CowVariants.java"
)
COW_SOUND_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/cow/CowSoundVariants.java"
)
PIG_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/pig/PigVariants.java"
)
PIG_SOUND_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/pig/PigSoundVariants.java"
)
FROG_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/frog/FrogVariants.java"
)
ZOMBIE_NAUTILUS_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/nautilus/ZombieNautilusVariants.java"
)
MAP_DECORATIONS_SOURCE = Path(
    "net/minecraft/world/level/saveddata/maps/MapDecorationTypes.java"
)
WOOD_TYPES_SOURCE = Path(
    "net/minecraft/world/level/block/state/properties/WoodType.java"
)
REGISTRY_DATA_LOADER_SOURCE = Path("net/minecraft/resources/RegistryDataLoader.java")
REGISTRIES_SOURCE = Path("net/minecraft/core/registries/Registries.java")
BUILT_IN_REGISTRIES_SOURCE = Path(
    "net/minecraft/core/registries/BuiltInRegistries.java"
)
FLUIDS_SOURCE = Path("net/minecraft/world/level/material/Fluids.java")
CHAT_TYPES_SOURCE = Path("net/minecraft/network/chat/ChatType.java")
TIMELINES_SOURCE = Path("net/minecraft/world/timeline/Timelines.java")

ANIMAL_SOUND_VARIANT_CONFIGS = (
    (
        "cat",
        "VANILLA_CAT_SOUND_VARIANT_KEYS",
        "CatSoundVariant",
        "CatSoundVariantIdentity",
        "CatSoundVariants",
        "CAT_SOUNDS",
        CAT_SOUND_VARIANTS_SOURCE,
        2,
    ),
    (
        "chicken",
        "VANILLA_CHICKEN_SOUND_VARIANT_KEYS",
        "ChickenSoundVariant",
        "ChickenSoundVariantIdentity",
        "ChickenSoundVariants",
        "CHICKEN_SOUNDS",
        CHICKEN_SOUND_VARIANTS_SOURCE,
        2,
    ),
    (
        "cow",
        "VANILLA_COW_SOUND_VARIANT_KEYS",
        "CowSoundVariant",
        "CowSoundVariantIdentity",
        "CowSoundVariants",
        "COW_SOUNDS",
        COW_SOUND_VARIANTS_SOURCE,
        2,
    ),
    (
        "pig",
        "VANILLA_PIG_SOUND_VARIANT_KEYS",
        "PigSoundVariant",
        "PigSoundVariantIdentity",
        "PigSoundVariants",
        "PIG_SOUNDS",
        PIG_SOUND_VARIANTS_SOURCE,
        3,
    ),
    (
        "wolf",
        "VANILLA_WOLF_SOUND_VARIANT_KEYS",
        "WolfSoundVariant",
        "WolfSoundVariantIdentity",
        "WolfSoundVariants",
        "WOLF_SOUNDS",
        WOLF_SOUND_VARIANTS_SOURCE,
        7,
    ),
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--enchantment-codec-types-output",
        type=Path,
        default=DEFAULT_ENCHANTMENT_CODEC_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--codec-dispatch-types-output",
        type=Path,
        default=DEFAULT_CODEC_DISPATCH_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--loot-context-types-output",
        type=Path,
        default=DEFAULT_LOOT_CONTEXT_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--enchantment-bridge-output",
        type=Path,
        default=DEFAULT_ENCHANTMENT_BRIDGE_OUTPUT,
    )
    parser.add_argument(
        "--persistent-registry-facts-output",
        type=Path,
        default=DEFAULT_PERSISTENT_REGISTRY_FACTS_OUTPUT,
    )
    parser.add_argument(
        "--persistent-registry-tags-bin-output",
        type=Path,
    )
    parser.add_argument(
        "--potion-catalog-output",
        type=Path,
        default=DEFAULT_POTION_CATALOG_OUTPUT,
    )
    parser.add_argument("--potions-only", action="store_true")
    parser.add_argument(
        "--painting-variant-types-output",
        type=Path,
        default=DEFAULT_PAINTING_VARIANT_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--painting-variants-output",
        type=Path,
        default=DEFAULT_PAINTING_VARIANTS_OUTPUT,
    )
    parser.add_argument(
        "--decorated-pot-patterns-output",
        type=Path,
        default=DEFAULT_DECORATED_POT_PATTERNS_OUTPUT,
    )
    parser.add_argument(
        "--banner-patterns-output",
        type=Path,
        default=DEFAULT_BANNER_PATTERNS_OUTPUT,
    )
    parser.add_argument(
        "--banner-pattern-types-output",
        type=Path,
        default=DEFAULT_BANNER_PATTERN_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--villager-professions-output",
        type=Path,
        default=DEFAULT_VILLAGER_PROFESSIONS_OUTPUT,
    )
    parser.add_argument(
        "--villager-types-output",
        type=Path,
        default=DEFAULT_VILLAGER_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--cat-variants-output",
        type=Path,
        default=DEFAULT_CAT_VARIANTS_OUTPUT,
    )
    parser.add_argument(
        "--wolf-variants-output",
        type=Path,
        default=DEFAULT_WOLF_VARIANTS_OUTPUT,
    )
    parser.add_argument(
        "--temperature-animal-variants-output",
        type=Path,
        default=DEFAULT_TEMPERATURE_ANIMAL_VARIANTS_OUTPUT,
    )
    parser.add_argument(
        "--zombie-nautilus-variants-output",
        type=Path,
        default=DEFAULT_ZOMBIE_NAUTILUS_VARIANTS_OUTPUT,
    )
    parser.add_argument(
        "--animal-sound-variant-types-output",
        type=Path,
        default=DEFAULT_ANIMAL_SOUND_VARIANT_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--animal-sound-variants-output",
        type=Path,
        default=DEFAULT_ANIMAL_SOUND_VARIANTS_OUTPUT,
    )
    parser.add_argument(
        "--instruments-output",
        type=Path,
        default=DEFAULT_INSTRUMENTS_OUTPUT,
    )
    parser.add_argument(
        "--trim-patterns-output",
        type=Path,
        default=DEFAULT_TRIM_PATTERNS_OUTPUT,
    )
    parser.add_argument(
        "--trim-materials-output",
        type=Path,
        default=DEFAULT_TRIM_MATERIALS_OUTPUT,
    )
    parser.add_argument(
        "--map-decorations-output",
        type=Path,
        default=DEFAULT_MAP_DECORATIONS_OUTPUT,
    )
    parser.add_argument("--map-decorations-only", action="store_true")
    parser.add_argument(
        "--registry-data-types-output",
        type=Path,
        default=DEFAULT_REGISTRY_DATA_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--sign-wood-types-output",
        type=Path,
        default=DEFAULT_SIGN_WOOD_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--overworld-day-timeline-output",
        type=Path,
        default=DEFAULT_OVERWORLD_DAY_TIMELINE_OUTPUT,
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


def parse_java(source_root: Path, relative_path: Path):
    return parse_source(relative_path.as_posix(), (source_root / relative_path).read_bytes())


def vanilla_texture_dimensions(
    source_root: Path, asset_id: str
) -> tuple[int, int]:
    namespace, separator, path = asset_id.partition(":")
    if separator != ":" or not namespace or not path:
        raise ValueError(f"invalid vanilla texture asset id: {asset_id}")
    texture_path = source_root / "assets" / namespace / "textures" / f"{path}.png"
    header = texture_path.read_bytes()[:24]
    if (
        len(header) != 24
        or header[:8] != b"\x89PNG\r\n\x1a\n"
        or header[12:16] != b"IHDR"
    ):
        raise ValueError(f"invalid PNG texture header: {texture_path}")
    dimensions = (
        int.from_bytes(header[16:20], "big"),
        int.from_bytes(header[20:24], "big"),
    )
    if dimensions[0] == 0 or dimensions[1] == 0:
        raise ValueError(f"invalid zero-sized PNG texture: {texture_path}")
    return dimensions


def entity_texture_asset_id(asset_id: str, directory: str) -> str:
    namespace, separator, path = asset_id.partition(":")
    if separator != ":" or namespace != "minecraft" or not path or "/" in path:
        raise ValueError(
            f"entity texture asset must be one minecraft path segment: {asset_id}"
        )
    return f"{namespace}:entity/{directory}/{path}"


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def rust_variant(constant_name: str) -> str:
    return "".join(part.title() for part in constant_name.split("_"))


def render_sign_wood_types(entries: list[WoodTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 WoodType.java registered field order and names.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --map-decorations-only",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --map-decorations-only --check",
        "",
        "/// Vanilla `WoodType.values()` families with sign blocks in registration order.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "pub enum SignWoodKind {",
    ]
    for entry in entries:
        lines.append(f"    {rust_variant(entry.constant_name)},")
    lines.extend((
        "}",
        "",
        "const VANILLA_SIGN_WOOD_TYPES_26_1: &[(SignWoodKind, &str)] = &[",
    ))
    for entry in entries:
        lines.append(
            f"    (SignWoodKind::{rust_variant(entry.constant_name)}, {rust_string(entry.name)}),"
        )
    lines.extend((
        "];",
        "",
        "impl SignWoodKind {",
        "    pub fn name(self) -> &'static str {",
        "        VANILLA_SIGN_WOOD_TYPES_26_1",
        "            .iter()",
        "            .find_map(|&(wood, name)| (wood == self).then_some(name))",
        "            .expect(\"generated sign wood kind must have a name\")",
        "    }",
        "",
        "    fn parse(name: &str) -> Option<Self> {",
        "        VANILLA_SIGN_WOOD_TYPES_26_1",
        "            .iter()",
        "            .find_map(|&(wood, candidate)| (candidate == name).then_some(wood))",
        "    }",
        "}",
        "",
        "pub fn vanilla_sign_wood_kinds() -> impl Iterator<Item = SignWoodKind> {",
        "    VANILLA_SIGN_WOOD_TYPES_26_1.iter().map(|&(wood, _)| wood)",
        "}",
        "",
        "#[cfg(test)]",
        "mod generated_26_1_tests {",
        "    use std::collections::HashSet;",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn generated_sign_wood_types_are_complete_unique_and_round_trip() {",
        f"        assert_eq!(VANILLA_SIGN_WOOD_TYPES_26_1.len(), {len(entries)});",
        "        let mut names = HashSet::new();",
        "        for &(wood, name) in VANILLA_SIGN_WOOD_TYPES_26_1 {",
        "            assert!(names.insert(name));",
        "            assert_eq!(wood.name(), name);",
        "            assert_eq!(SignWoodKind::parse(name), Some(wood));",
        "        }",
        "        assert_eq!(SignWoodKind::parse(\"stone\"), None);",
        "        assert_eq!(vanilla_sign_wood_kinds().count(), VANILLA_SIGN_WOOD_TYPES_26_1.len());",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_map_decorations(entries: list[MapDecorationEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 MapDecorationTypes.java static register calls.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "const VANILLA_MAP_DECORATION_TYPES: &[VanillaMapDecorationType] = &[",
    ]
    for type_id, entry in enumerate(entries):
        lines.extend((
            "    VanillaMapDecorationType {",
            f"        type_id: {type_id},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        sprite_id: {rust_string(entry.asset_id)},",
            f"        render_on_frame: {str(entry.show_on_item_frame).lower()},",
            f"        track_count: {str(entry.track_count).lower()},",
            f"        exploration_map_element: {str(entry.exploration_map_element).lower()},",
            f"        map_color: {'None' if entry.map_color is None else f'Some({entry.map_color})'},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "#[cfg(test)]",
        "mod generated_26_1_tests {",
        "    use std::collections::HashSet;",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn generated_map_decorations_are_contiguous_and_unique() {",
        f"        assert_eq!(VANILLA_MAP_DECORATION_TYPES.len(), {len(entries)});",
        "        let mut resources = HashSet::new();",
        "        for (type_id, entry) in VANILLA_MAP_DECORATION_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.type_id, type_id as i32);",
        "            assert!(resources.insert(entry.resource_id));",
        "        }",
        "        assert!(VANILLA_MAP_DECORATION_TYPES[1].track_count);",
        "        assert!(!VANILLA_MAP_DECORATION_TYPES[4].track_count);",
        "        assert!(!VANILLA_MAP_DECORATION_TYPES[8].track_count);",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def synchronized_registry_data_catalog(
    worldgen: list[RegistryDataCatalogEntry],
    synchronized: list[RegistryDataCatalogEntry],
) -> list[SynchronizedRegistryDataCatalogEntry]:
    def by_resource_id(
        entries: list[RegistryDataCatalogEntry], catalog_name: str
    ) -> dict[str, RegistryDataCatalogEntry]:
        indexed = {}
        for entry in entries:
            if entry.resource_id in indexed:
                raise ValueError(
                    f"{catalog_name} contains duplicate resource id {entry.resource_id}"
                )
            indexed[entry.resource_id] = entry
        return indexed

    worldgen_by_resource_id = by_resource_id(worldgen, "WORLDGEN_REGISTRIES")
    by_resource_id(synchronized, "SYNCHRONIZED_REGISTRIES")
    result = []
    for entry in synchronized:
        persistent = worldgen_by_resource_id.get(entry.resource_id)
        if persistent is None:
            raise ValueError(
                "SYNCHRONIZED_REGISTRIES resource id "
                f"{entry.resource_id} is missing from WORLDGEN_REGISTRIES"
            )
        result.append(
            SynchronizedRegistryDataCatalogEntry(
                constant_name=entry.constant_name,
                resource_id=entry.resource_id,
                persistent_codec=persistent.codec,
                network_codec=entry.codec,
                validator=entry.validator,
            )
        )
    return result


def render_registry_data_types(
    worldgen: list[RegistryDataCatalogEntry],
    dimension: list[RegistryDataCatalogEntry],
    synchronized: list[RegistryDataCatalogEntry],
    built_in: list[BuiltInRegistryCatalogEntry],
    fluid_identities: list[BootstrapEntry],
    chat_type_identities: list[BootstrapEntry],
) -> str:
    synchronized_with_codecs = synchronized_registry_data_catalog(
        worldgen, synchronized
    )
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 RegistryDataLoader.java, BuiltInRegistries.java,",
        "// Registries.java, Fluids.java, and ChatType.java.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
    ]

    def append_fixed_registry_identity(
        type_name: str,
        catalog_name: str,
        entries: list[BootstrapEntry],
    ) -> None:
        lines.extend((
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "#[repr(i32)]",
            f"pub enum {type_name} {{",
            *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
            "}",
            "",
            f"pub const {catalog_name}: [({type_name}, &str); {len(entries)}] = [",
            *(
                f"    ({type_name}::{rust_variant(entry.constant_name)}, {rust_string(entry.resource_id)}),"
                for entry in entries
            ),
            "];",
            "",
            f"impl {type_name} {{",
            "    pub fn for_vanilla_registry_id(holder_id: i32) -> Option<Self> {",
            f"        {catalog_name}",
            "            .get(usize::try_from(holder_id).ok()?)",
            "            .map(|(identity, _)| *identity)",
            "    }",
            "",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            f"        {catalog_name}",
            "            .iter()",
            "            .find(|(_, candidate)| *candidate == resource_id)",
            "            .map(|(identity, _)| *identity)",
            "    }",
            "",
            "    pub const fn vanilla_registry_id(self) -> i32 {",
            "        self as i32",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            f"        {catalog_name}[self as usize].1",
            "    }",
            "}",
            "",
        ))

    append_fixed_registry_identity(
        "VanillaFluidIdentity",
        "VANILLA_FLUID_IDENTITIES",
        fluid_identities,
    )
    append_fixed_registry_identity(
        "VanillaChatTypeIdentity",
        "VANILLA_CHAT_TYPE_IDENTITIES",
        chat_type_identities,
    )
    lines.extend([
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct RegistryDataType {",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub codec: &'static str,",
        "    pub validator: Option<&'static str>,",
        "}",
        "",
    ])

    def append_catalog(name: str, entries: list[RegistryDataCatalogEntry]) -> None:
        lines.append(f"pub const {name}: [RegistryDataType; {len(entries)}] = [")
        for entry in entries:
            lines.extend((
                "    RegistryDataType {",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        codec: {rust_string(entry.codec)},",
                f"        validator: {'Some(' + rust_string(entry.validator) + ')' if entry.validator else 'None'},",
                "    },",
            ))
        lines.extend(("];", ""))

    append_catalog("VANILLA_WORLDGEN_REGISTRY_DATA_TYPES", worldgen)
    append_catalog("VANILLA_DIMENSION_REGISTRY_DATA_TYPES", dimension)
    lines.extend((
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct SynchronizedRegistryDataType {",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub persistent_codec: &'static str,",
        "    pub network_codec: &'static str,",
        "    pub validator: Option<&'static str>,",
        "}",
        "",
        f"pub const VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES: [SynchronizedRegistryDataType; {len(synchronized_with_codecs)}] = [",
    ))
    for entry in synchronized_with_codecs:
        lines.extend((
            "    SynchronizedRegistryDataType {",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        persistent_codec: {rust_string(entry.persistent_codec)},",
            f"        network_codec: {rust_string(entry.network_codec)},",
            f"        validator: {'Some(' + rust_string(entry.validator) + ')' if entry.validator else 'None'},",
            "    },",
        ))
    lines.extend(("];", ""))
    lines.extend((
        "pub fn synchronized_registry_data_type(resource_id: &str) -> Option<&'static SynchronizedRegistryDataType> {",
        "    VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES",
        "        .iter()",
        "        .find(|entry| entry.resource_id == resource_id)",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct BuiltInRegistryType {",
        "    pub constant_name: &'static str,",
        "    pub registry_constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub registration: &'static str,",
        "    pub default_id: Option<&'static str>,",
        "}",
        "",
        f"pub const VANILLA_BUILT_IN_REGISTRY_TYPES: [BuiltInRegistryType; {len(built_in)}] = [",
    ))
    for entry in built_in:
        lines.extend((
            "    BuiltInRegistryType {",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        registry_constant_name: {rust_string(entry.registry_constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        registration: {rust_string(entry.registration)},",
            f"        default_id: {'Some(' + rust_string(entry.default_id) + ')' if entry.default_id else 'None'},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "#[inline]",
        "pub fn built_in_registry_type(resource_id: &str) -> Option<&'static BuiltInRegistryType> {",
        "    crate::lookup_tables::built_in_registry_type(resource_id)",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::collections::HashSet;",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn fixed_registry_identity_catalogs_follow_vanilla_order() {",
        "        for (holder_id, (identity, resource_id)) in VANILLA_FLUID_IDENTITIES.iter().enumerate() {",
        "            assert_eq!(identity.vanilla_registry_id(), holder_id as i32);",
        "            assert_eq!(identity.resource_id(), *resource_id);",
        "            assert_eq!(VanillaFluidIdentity::for_vanilla_registry_id(holder_id as i32), Some(*identity));",
        "            assert_eq!(VanillaFluidIdentity::for_resource_id(resource_id), Some(*identity));",
        "        }",
        "        for (holder_id, (identity, resource_id)) in VANILLA_CHAT_TYPE_IDENTITIES.iter().enumerate() {",
        "            assert_eq!(identity.vanilla_registry_id(), holder_id as i32);",
        "            assert_eq!(identity.resource_id(), *resource_id);",
        "            assert_eq!(VanillaChatTypeIdentity::for_vanilla_registry_id(holder_id as i32), Some(*identity));",
        "            assert_eq!(VanillaChatTypeIdentity::for_resource_id(resource_id), Some(*identity));",
        "        }",
        "        assert_eq!(VanillaFluidIdentity::for_vanilla_registry_id(-1), None);",
        f"        assert_eq!(VanillaFluidIdentity::for_vanilla_registry_id({len(fluid_identities)}), None);",
        "        assert_eq!(VanillaChatTypeIdentity::for_resource_id(\"example:chat\"), None);",
        "    }",
        "",
        "    #[test]",
        "    fn registry_data_catalogs_preserve_vanilla_lists() {",
        f"        assert_eq!(VANILLA_WORLDGEN_REGISTRY_DATA_TYPES.len(), {len(worldgen)});",
        f"        assert_eq!(VANILLA_DIMENSION_REGISTRY_DATA_TYPES.len(), {len(dimension)});",
        "        assert_eq!(VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES.len(), 28);",
        f"        assert_eq!(VANILLA_BUILT_IN_REGISTRY_TYPES.len(), {len(built_in)});",
        "        for catalog in [",
        "            VANILLA_WORLDGEN_REGISTRY_DATA_TYPES.as_slice(),",
        "            VANILLA_DIMENSION_REGISTRY_DATA_TYPES.as_slice(),",
        "        ] {",
        "            let mut constants = HashSet::new();",
        "            let mut resources = HashSet::new();",
        "            for entry in catalog {",
        "                assert!(constants.insert(entry.constant_name));",
        "                assert!(resources.insert(entry.resource_id));",
        "                assert!(entry.resource_id.starts_with(\"minecraft:\"));",
        "                assert!(!entry.codec.is_empty());",
        "            }",
        "        }",
        "        let mut constants = HashSet::new();",
        "        let mut resources = HashSet::new();",
        "        for entry in &VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {",
        "            assert!(constants.insert(entry.constant_name));",
        "            assert!(resources.insert(entry.resource_id));",
        "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
        "            assert!(!entry.persistent_codec.is_empty());",
        "            assert!(!entry.network_codec.is_empty());",
        "        }",
        "        assert_eq!(",
        "            VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES",
        "                .iter()",
        "                .map(|entry| entry.resource_id)",
        "                .collect::<Vec<_>>(),",
        "            vec![",
        *(
            f"                {rust_string(resource_id)},"
            for resource_id in VANILLA_SYNCHRONIZED_REGISTRY_ORDER_26_1
        ),
        "            ]",
        "        );",
        "        assert_eq!(",
        "            VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES",
        "                .iter()",
        "                .filter(|entry| entry.validator == Some(\"RegistryValidator.nonEmpty ()\"))",
        "                .count(),",
        "            13",
        "        );",
        "    }",
        "",
        "    #[test]",
        "    fn built_in_registry_catalog_preserves_static_layer_identity() {",
        "        let mut resources = HashSet::new();",
        "        for entry in &VANILLA_BUILT_IN_REGISTRY_TYPES {",
        "            assert!(resources.insert(entry.resource_id));",
        "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
        "            assert!(synchronized_registry_data_type(entry.resource_id).is_none());",
        "            assert_eq!(built_in_registry_type(entry.resource_id), Some(entry));",
        "        }",
        "        let item = built_in_registry_type(\"minecraft:item\").unwrap();",
        "        assert_eq!(item.registration, \"registerDefaultedWithIntrusiveHolders\");",
        "        assert_eq!(item.default_id, Some(\"minecraft:air\"));",
        "        let sound = built_in_registry_type(\"minecraft:sound_event\").unwrap();",
        "        assert_eq!(sound.registration, \"registerSimple\");",
        "        assert_eq!(sound.default_id, None);",
        "        assert_eq!(built_in_registry_type(\"example:custom\"), None);",
        "    }",
        "",
        "    #[test]",
        "    fn synchronized_registry_lookup_uses_the_generated_resource_ids() {",
        "        for entry in &VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {",
        "            assert_eq!(synchronized_registry_data_type(entry.resource_id), Some(entry));",
        "        }",
        "        assert_eq!(synchronized_registry_data_type(\"minecraft:item\"), None);",
        "        assert_eq!(synchronized_registry_data_type(\"example:custom\"), None);",
        "    }",
        "",
        "    #[test]",
        "    fn registry_data_codecs_and_validators_match_distinct_loader_lists() {",
        "        assert_eq!(VANILLA_WORLDGEN_REGISTRY_DATA_TYPES[0].resource_id, \"minecraft:dimension_type\");",
        "        assert_eq!(VANILLA_DIMENSION_REGISTRY_DATA_TYPES[0].resource_id, \"minecraft:dimension\");",
        "        assert_eq!(VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES[0].resource_id, \"minecraft:worldgen/biome\");",
        "        assert_eq!(VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES[0].persistent_codec, \"Biome.DIRECT_CODEC\");",
        "        assert_eq!(VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES[0].network_codec, \"Biome.NETWORK_CODEC\");",
        "        let dimension_type = synchronized_registry_data_type(\"minecraft:dimension_type\").unwrap();",
        "        assert_eq!(dimension_type.persistent_codec, \"DimensionType.DIRECT_CODEC\");",
        "        assert_eq!(dimension_type.network_codec, \"DimensionType.NETWORK_CODEC\");",
        "        let timeline = VANILLA_WORLDGEN_REGISTRY_DATA_TYPES.iter().find(|entry| entry.constant_name == \"TIMELINE\").unwrap();",
        "        assert_eq!(timeline.validator, Some(\"Timeline::validateRegistry\"));",
        "        let wolf = synchronized_registry_data_type(\"minecraft:wolf_variant\").unwrap();",
        "        assert_eq!(wolf.persistent_codec, \"WolfVariant.DIRECT_CODEC\");",
        "        assert_eq!(wolf.network_codec, \"WolfVariant.NETWORK_CODEC\");",
        "        assert_eq!(wolf.validator, Some(\"RegistryValidator.nonEmpty ()\"));",
        "        let chat = synchronized_registry_data_type(\"minecraft:chat_type\").unwrap();",
        "        assert_eq!(chat.persistent_codec, \"ChatType.DIRECT_CODEC\");",
        "        assert_eq!(chat.network_codec, chat.persistent_codec);",
        "        let cow_sound = synchronized_registry_data_type(\"minecraft:cow_sound_variant\").unwrap();",
        "        assert_eq!(cow_sound.persistent_codec, \"CowSoundVariant.DIRECT_CODEC\");",
        "        assert_eq!(cow_sound.network_codec, cow_sound.persistent_codec);",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_overworld_day_timeline(entry: OverworldDayTimelineEntry) -> str:
    constant_names = {
        "FOG_COLOR": "VANILLA_OVERWORLD_FOG_COLOR_MULTIPLIER_KEYFRAMES",
        "SKY_COLOR": "VANILLA_OVERWORLD_SKY_COLOR_MULTIPLIER_KEYFRAMES",
        "SKY_LIGHT_COLOR": "VANILLA_OVERWORLD_SKY_LIGHT_COLOR_KEYFRAMES",
        "SKY_LIGHT_FACTOR": "VANILLA_OVERWORLD_SKY_LIGHT_FACTOR_KEYFRAMES",
        "SKY_LIGHT_LEVEL": "VANILLA_OVERWORLD_SKY_LIGHT_LEVEL_KEYFRAMES",
        "SUNRISE_SUNSET_COLOR": "VANILLA_OVERWORLD_SUNRISE_SUNSET_COLOR_KEYFRAMES",
        "STAR_BRIGHTNESS": "VANILLA_OVERWORLD_STAR_BRIGHTNESS_KEYFRAMES",
        "CLOUD_COLOR": "VANILLA_OVERWORLD_CLOUD_COLOR_MULTIPLIER_KEYFRAMES",
    }
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Timelines.java OVERWORLD_DAY bootstrap registration.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        f"pub(super) const VANILLA_OVERWORLD_DAY_PERIOD_TICKS: i64 = {entry.period_ticks};",
        (
            "pub(super) const VANILLA_LIGHTMAP_DAY_PERIOD_TICKS: i64 = "
            "VANILLA_OVERWORLD_DAY_PERIOD_TICKS;"
        ),
        "",
    ]
    for track in entry.tracks:
        constant_name = constant_names[track.attribute_name]
        value_type = "i32" if track.value_kind == "i32" else "f32"
        lines.append(
            f"pub(super) const {constant_name}: [(i64, {value_type}); "
            f"{len(track.keyframes)}] = ["
        )
        for keyframe in track.keyframes:
            value = (
                str(keyframe.encoded_value)
                if track.value_kind == "i32"
                else f"f32::from_bits(0x{keyframe.encoded_value:08x})"
            )
            lines.append(f"    ({keyframe.ticks}, {value}),")
        lines.extend(("];", ""))

    total_keyframes = sum(len(track.keyframes) for track in entry.tracks)
    lines.extend((
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "",
        "    fn ticks_are_ordered<T>(keyframes: &[(i64, T)]) -> bool {",
        "        keyframes.windows(2).all(|pair| pair[0].0 <= pair[1].0)",
        "    }",
        "",
        "    #[test]",
        "    fn generated_overworld_day_visual_tracks_are_complete_and_ordered() {",
        f"        assert_eq!(VANILLA_OVERWORLD_DAY_PERIOD_TICKS, {entry.period_ticks});",
        "        let mut total_keyframes = 0usize;",
    ))
    for track in entry.tracks:
        constant_name = constant_names[track.attribute_name]
        lines.extend((
            f"        assert_eq!({constant_name}.len(), {len(track.keyframes)});",
            f"        assert!(ticks_are_ordered(&{constant_name}));",
            f"        total_keyframes += {constant_name}.len();",
        ))
    lines.extend((
        f"        assert_eq!(total_keyframes, {total_keyframes});",
        "        assert_eq!(",
        "            VANILLA_OVERWORLD_SKY_LIGHT_COLOR_KEYFRAMES.last().unwrap().1,",
        f"            {entry.night_sky_light_color},",
        "        );",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_potion_catalog(entries: list[PotionEntry]) -> str:
    effect_constants = sorted(
        {effect.constant_name for entry in entries for effect in entry.effects}
    )
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Potions.java definitions and MobEffects.java registrations.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "use crate::mob_effect_types::{",
        *(
            f"    VANILLA_MOB_EFFECT_{constant_name}_ID,"
            for constant_name in effect_constants
        ),
        "};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct VanillaPotionEffect {",
        "    pub effect_id: i32,",
        "    pub duration: i32,",
        "    pub amplifier: i32,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct VanillaPotion {",
        "    pub id: i32,",
        "    pub resource_id: &'static str,",
        "    pub name: &'static str,",
        "    pub effects: &'static [VanillaPotionEffect],",
        "}",
        "",
        "pub const VANILLA_POTIONS: &[VanillaPotion] = &[",
    ]
    for potion_id, entry in enumerate(entries):
        lines.extend(
            (
                "    VanillaPotion {",
                f"        id: {potion_id},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        name: {rust_string(entry.name)},",
                "        effects: &[",
            )
        )
        for effect in entry.effects:
            lines.extend(
                (
                    "            VanillaPotionEffect {",
                    f"                effect_id: VANILLA_MOB_EFFECT_{effect.constant_name}_ID,",
                    f"                duration: {effect.duration},",
                    f"                amplifier: {effect.amplifier},",
                    "            },",
                )
            )
        lines.extend(("        ],", "    },"))
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_potion(id: i32) -> Option<&'static VanillaPotion> {",
            "    VANILLA_POTIONS.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_potion_for_resource_id(resource_id: &str) -> Option<&'static VanillaPotion> {",
            "    VANILLA_POTIONS.iter().find(|potion| potion.resource_id == resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn potion_catalog_is_complete_contiguous_and_unique() {",
            f"        assert_eq!(VANILLA_POTIONS.len(), {len(entries)});",
            "        let mut resources = HashSet::new();",
            "        for (id, entry) in VANILLA_POTIONS.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            f"        assert_eq!(VANILLA_POTIONS.iter().map(|entry| entry.effects.len()).sum::<usize>(), {sum(len(entry.effects) for entry in entries)});",
            "    }",
            "",
            "    #[test]",
            "    fn potion_catalog_preserves_vanilla_order_and_effects() {",
            '        assert_eq!(vanilla_potion(0).unwrap().resource_id, "minecraft:water");',
            '        assert_eq!(vanilla_potion(3).unwrap().resource_id, "minecraft:awkward");',
            '        assert_eq!(vanilla_potion_for_resource_id("minecraft:awkward").unwrap().id, 3);',
            '        assert_eq!(vanilla_potion_for_resource_id("minecraft:long_swiftness").unwrap().name, "swiftness");',
            '        assert_eq!(vanilla_potion_for_resource_id("minecraft:strong_swiftness").unwrap().name, "swiftness");',
            '        assert_eq!(vanilla_potion(42).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_WIND_CHARGED_ID);',
            '        assert_eq!(vanilla_potion(43).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_WEAVING_ID);',
            '        assert_eq!(vanilla_potion(44).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_OOZING_ID);',
            '        assert_eq!(vanilla_potion(45).unwrap().effects, &[VanillaPotionEffect { effect_id: VANILLA_MOB_EFFECT_INFESTED_ID, duration: 3600, amplifier: 0 }]);',
            "        assert_eq!(vanilla_potion(-1), None);",
            "        assert_eq!(vanilla_potion(46), None);",
            '        assert_eq!(vanilla_potion_for_resource_id("minecraft:missing"), None);',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def codec_family_variants(
    entries: list[RegistryCodecEntry],
    *,
    common_owner: str | None = None,
) -> list[tuple[RegistryCodecEntry, str]]:
    variants: list[tuple[RegistryCodecEntry, str]] = []
    for entry in entries:
        owner_parts = entry.codec_owner.split(".")
        if common_owner is not None:
            if len(owner_parts) < 2 or owner_parts[0] != common_owner:
                raise ValueError(
                    f"codec owner {entry.codec_owner} is outside {common_owner}"
                )
            owner_parts = owner_parts[1:]
        variant = "".join(owner_parts)
        if not variant:
            raise ValueError(f"codec owner {entry.codec_owner} has no family name")
        variants.append((entry, variant))
    if len({variant for _, variant in variants}) != len(variants):
        raise ValueError("codec owners collapse to duplicate Rust enum variants")
    return variants


def append_registry_codec_catalog(
    lines: list[str],
    entries: list[RegistryCodecEntry],
    *,
    family_type: str,
    entry_type: str,
    catalog_name: str,
    lookup_name: str,
    common_owner: str | None = None,
    visibility: str = "pub(crate)",
    expect_dead_code: bool = False,
) -> None:
    variants = codec_family_variants(entries, common_owner=common_owner)
    lint_expectation = (
        (
            "#[cfg_attr(not(test), expect(dead_code, reason = "
            '"wired by the Enchantment root in this engineering slice"))]'
        ),
    ) if expect_dead_code else ()
    lines.extend(
        (
            *lint_expectation,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
            f"{visibility} enum {family_type} {{",
            *(f"    {variant}," for _, variant in variants),
            "}",
            "",
            "#[cfg(test)]",
            f"impl {family_type} {{",
            "    pub(crate) const fn codec_owner(self) -> &'static str {",
            "        match self {",
            *(
                f"            Self::{variant} => {rust_string(entry.codec_owner)},"
                for entry, variant in variants
            ),
            "        }",
            "    }",
            "}",
            "",
            *lint_expectation,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            f"{visibility} struct {entry_type} {{",
            f"    {visibility} resource_id: &'static str,",
            f"    {visibility} codec_family: {family_type},",
            "}",
            "",
            *lint_expectation,
            f"{visibility} const {catalog_name}: &[{entry_type}] = &[",
        )
    )
    for entry, variant in variants:
        lines.extend(
            (
                f"    {entry_type} {{",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        codec_family: {family_type}::{variant},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            *lint_expectation,
            f"{visibility} fn {lookup_name}(resource_id: &str) -> Option<&'static {entry_type}> {{",
            f"    {catalog_name}",
            "        .iter()",
            "        .find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
        )
    )


def append_registry_codec_catalog_test(
    lines: list[str],
    entries: list[RegistryCodecEntry],
    *,
    test_name: str,
    family_type: str,
    catalog_name: str,
    lookup_name: str,
    common_owner: str | None = None,
) -> None:
    variants = codec_family_variants(entries, common_owner=common_owner)
    first_entry, first_variant = variants[0]
    lines.extend(
        (
            "    #[test]",
            f"    fn {test_name}_preserves_order_uniqueness_and_lookup() {{",
            "        assert_eq!(",
            f"            {catalog_name}",
            "                .iter()",
            "                .map(|entry| entry.resource_id)",
            "                .collect::<Vec<_>>(),",
            "            [",
            *(f"                {rust_string(entry.resource_id)}," for entry in entries),
            "            ]",
            "        );",
            "        let mut resource_ids = HashSet::new();",
            "        let mut codec_families = HashSet::new();",
            f"        for entry in {catalog_name} {{",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(codec_families.insert(entry.codec_family));",
            f"            assert_eq!({lookup_name}(entry.resource_id), Some(entry));",
            "        }",
            (
                f"        let first = {lookup_name}("
                f"{rust_string(first_entry.resource_id)}).unwrap();"
            ),
            f"        assert_eq!(first.codec_family, {family_type}::{first_variant});",
            (
                "        assert_eq!(first.codec_family.codec_owner(), "
                f"{rust_string(first_entry.codec_owner)});"
            ),
            f"        assert!({lookup_name}(\"minecraft:missing\").is_none());",
            "    }",
            "",
        )
    )


def append_effect_component_catalog(
    lines: list[str],
    entries: list[EnchantmentEffectComponentEntry],
    param_sets: list[LootContextParamSetEntry],
) -> None:
    families = list(dict.fromkeys(entry.codec_family for entry in entries))
    contexts = list(
        dict.fromkeys(
            entry.validation_context
            for entry in entries
            if entry.validation_context is not None
        )
    )
    param_set_ids = {entry.constant_name: entry.resource_id for entry in param_sets}
    missing_contexts = sorted(set(contexts) - set(param_set_ids))
    if missing_contexts:
        raise ValueError(
            "enchantment effect validation contexts lack loot context param sets: "
            f"{missing_contexts}"
        )
    lines.extend(
        (
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
            "pub(crate) enum EnchantmentEffectComponentCodecFamily {",
            *(f"    {family}," for family in families),
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
            "pub(crate) enum EnchantmentEffectValidationContext {",
            *(f"    {rust_variant(context)}," for context in contexts),
            "}",
            "",
            "impl EnchantmentEffectValidationContext {",
            "    pub(crate) const fn loot_context_param_set_resource_id(self) -> &'static str {",
            "        match self {",
            *(
                f"            Self::{rust_variant(context)} => {rust_string(param_set_ids[context])},"
                for context in contexts
            ),
            "        }",
            "    }",
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub(crate) struct EnchantmentEffectComponentCodecType {",
            "    pub(crate) resource_id: &'static str,",
            "    pub(crate) codec_family: EnchantmentEffectComponentCodecFamily,",
            "    pub(crate) validation_context: Option<EnchantmentEffectValidationContext>,",
            "}",
            "",
            "pub(crate) const VANILLA_ENCHANTMENT_EFFECT_COMPONENT_CODEC_TYPES:",
            "    &[EnchantmentEffectComponentCodecType] = &[",
        )
    )
    for entry in entries:
        context = (
            "Some(EnchantmentEffectValidationContext::"
            f"{rust_variant(entry.validation_context)})"
            if entry.validation_context is not None
            else "None"
        )
        lines.extend(
            (
                "    EnchantmentEffectComponentCodecType {",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "        codec_family: EnchantmentEffectComponentCodecFamily::"
                f"{entry.codec_family},",
                f"        validation_context: {context},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub(crate) fn enchantment_effect_component_codec_type(",
            "    resource_id: &str,",
            ") -> Option<&'static EnchantmentEffectComponentCodecType> {",
            "    VANILLA_ENCHANTMENT_EFFECT_COMPONENT_CODEC_TYPES",
            "        .iter()",
            "        .find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
        )
    )


def render_loot_context_types(
    params: list[LootContextParamEntry],
    targets: list[LootContextEntityTargetEntry],
    param_sets: list[LootContextParamSetEntry],
) -> str:
    variants = {
        entry.constant_name: rust_variant(entry.constant_name) for entry in params
    }
    if len(set(variants.values())) != len(variants):
        raise ValueError("duplicate generated VanillaLootContextParam Rust variant")
    unknown_params = sorted(
        {entry.context_param for entry in targets} - set(variants)
    )
    if unknown_params:
        raise ValueError(f"loot entity targets reference unknown params: {unknown_params}")
    unknown_set_params = sorted(
        {
            param
            for entry in param_sets
            for param in (*entry.required, *entry.optional)
        }
        - set(variants)
    )
    if unknown_set_params:
        raise ValueError(
            f"loot context param sets reference unknown params: {unknown_set_params}"
        )
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1",
        "// - net/minecraft/world/level/storage/loot/parameters/LootContextParams.java",
        "// - net/minecraft/world/level/storage/loot/parameters/LootContextParamSets.java",
        "// - net/minecraft/world/level/storage/loot/LootContext.java",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]",
        "pub enum VanillaLootContextParam {",
        *(f"    {variants[entry.constant_name]}," for entry in params),
        "}",
        "",
        "impl VanillaLootContextParam {",
        "    pub const fn resource_id(self) -> &'static str {",
        "        match self {",
        *(
            f"            Self::{variants[entry.constant_name]} => {rust_string(entry.resource_id)},"
            for entry in params
        ),
        "        }",
        "    }",
        "}",
        "",
        "pub const VANILLA_LOOT_CONTEXT_PARAMS: &[VanillaLootContextParam] = &[",
        *(
            f"    VanillaLootContextParam::{variants[entry.constant_name]},"
            for entry in params
        ),
        "];",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct VanillaLootEntityTarget {",
        "    pub serialized_name: &'static str,",
        "    pub context_param: VanillaLootContextParam,",
        "}",
        "",
        "pub const VANILLA_LOOT_ENTITY_TARGETS: &[VanillaLootEntityTarget] = &[",
    ]
    for entry in targets:
        lines.extend(
            (
                "    VanillaLootEntityTarget {",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                "        context_param: "
                f"VanillaLootContextParam::{variants[entry.context_param]},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_loot_entity_target(",
            "    name: &str,",
            ") -> Option<&'static VanillaLootEntityTarget> {",
            "    VANILLA_LOOT_ENTITY_TARGETS",
            "        .iter()",
            "        .find(|target| target.serialized_name == name)",
            "}",
            "",
        )
    )
    lines.extend(
        (
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct VanillaLootContextParamSet {",
            "    pub constant_name: &'static str,",
            "    pub resource_id: &'static str,",
            "    pub required: &'static [VanillaLootContextParam],",
            "    pub optional: &'static [VanillaLootContextParam],",
            "}",
            "",
            "impl VanillaLootContextParamSet {",
            "    pub fn allows(&self, param: VanillaLootContextParam) -> bool {",
            "        self.required.contains(&param) || self.optional.contains(&param)",
            "    }",
            "}",
            "",
            "pub const VANILLA_LOOT_CONTEXT_PARAM_SETS: &[VanillaLootContextParamSet] = &[",
        )
    )
    for entry in param_sets:
        lines.extend(
            (
                "    VanillaLootContextParamSet {",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "        required: &[",
                *(
                    f"            VanillaLootContextParam::{variants[param]},"
                    for param in entry.required
                ),
                "        ],",
                "        optional: &[",
                *(
                    f"            VanillaLootContextParam::{variants[param]},"
                    for param in entry.optional
                ),
                "        ],",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_loot_context_param_set(",
            "    resource_id: &str,",
            ") -> Option<&'static VanillaLootContextParamSet> {",
            "    VANILLA_LOOT_CONTEXT_PARAM_SETS",
            "        .iter()",
            "        .find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn generated_loot_context_params_are_complete_unique_and_ordered() {",
            "        assert_eq!(VANILLA_LOOT_CONTEXT_PARAMS.len(), 15);",
            "        assert_eq!(",
            "            VANILLA_LOOT_CONTEXT_PARAMS",
            "                .iter()",
            "                .map(|param| param.resource_id())",
            "                .collect::<Vec<_>>(),",
            "            [",
            *(f"                {rust_string(entry.resource_id)}," for entry in params),
            "            ]",
            "        );",
            "        assert_eq!(",
            "            VANILLA_LOOT_CONTEXT_PARAMS",
            "                .iter()",
            "                .map(|param| param.resource_id())",
            "                .collect::<HashSet<_>>()",
            "                .len(),",
            "            VANILLA_LOOT_CONTEXT_PARAMS.len()",
            "        );",
            "    }",
            "",
            "    #[test]",
            "    fn generated_loot_entity_targets_preserve_order_mapping_and_lookup() {",
            "        assert_eq!(VANILLA_LOOT_ENTITY_TARGETS.len(), 6);",
            "        assert_eq!(",
            "            VANILLA_LOOT_ENTITY_TARGETS",
            "                .iter()",
            "                .map(|target| (target.serialized_name, target.context_param))",
            "                .collect::<Vec<_>>(),",
            "            [",
            *(
                "                ("
                f"{rust_string(entry.serialized_name)}, "
                f"VanillaLootContextParam::{variants[entry.context_param]}),"
                for entry in targets
            ),
            "            ]",
            "        );",
            "        for target in VANILLA_LOOT_ENTITY_TARGETS {",
            "            assert_eq!(vanilla_loot_entity_target(target.serialized_name), Some(target));",
            "        }",
            '        assert!(vanilla_loot_entity_target("missing").is_none());',
            "    }",
            "",
            "    #[test]",
            "    fn generated_loot_context_param_sets_preserve_order_membership_and_lookup() {",
            f"        assert_eq!(VANILLA_LOOT_CONTEXT_PARAM_SETS.len(), {len(param_sets)});",
            "        assert_eq!(",
            "            VANILLA_LOOT_CONTEXT_PARAM_SETS",
            "                .iter()",
            "                .map(|entry| (entry.constant_name, entry.resource_id))",
            "                .collect::<Vec<_>>(),",
            "            [",
            *(
                f"                ({rust_string(entry.constant_name)}, {rust_string(entry.resource_id)}),"
                for entry in param_sets
            ),
            "            ]",
            "        );",
            "        for entry in VANILLA_LOOT_CONTEXT_PARAM_SETS {",
            "            assert_eq!(vanilla_loot_context_param_set(entry.resource_id), Some(entry));",
            "            let required = entry.required.iter().copied().collect::<HashSet<_>>();",
            "            let optional = entry.optional.iter().copied().collect::<HashSet<_>>();",
            "            assert_eq!(required.len(), entry.required.len());",
            "            assert_eq!(optional.len(), entry.optional.len());",
            "            assert!(required.is_disjoint(&optional));",
            "            for param in VANILLA_LOOT_CONTEXT_PARAMS {",
            "                assert_eq!(entry.allows(*param), required.contains(param) || optional.contains(param));",
            "            }",
            "        }",
            '        assert!(vanilla_loot_context_param_set("minecraft:missing").is_none());',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def configured_feature_codec_family_variants(
    entries: list[ConfiguredFeatureCodecEntry],
) -> list[tuple[str, str]]:
    owners = list(dict.fromkeys(entry.config_codec_owner for entry in entries))
    variants = [(owner, "".join(owner.split("."))) for owner in owners]
    if any(not variant for _, variant in variants):
        raise ValueError("configured Feature codec owner has no family name")
    if len({variant for _, variant in variants}) != len(variants):
        raise ValueError(
            "configured Feature codec owners collapse to duplicate Rust enum variants"
        )
    return variants


def append_configured_feature_codec_catalog(
    lines: list[str], entries: list[ConfiguredFeatureCodecEntry]
) -> None:
    variants = configured_feature_codec_family_variants(entries)
    variant_by_owner = dict(variants)
    lines.extend(
        (
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
            "pub enum ConfiguredFeatureCodecFamily {",
            *(f"    {variant}," for _, variant in variants),
            "}",
            "",
            "#[cfg(test)]",
            "impl ConfiguredFeatureCodecFamily {",
            "    pub(crate) const fn codec_owner(self) -> &'static str {",
            "        match self {",
            *(
                f"            Self::{variant} => {rust_string(owner)},"
                for owner, variant in variants
            ),
            "        }",
            "    }",
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct ConfiguredFeatureCodecType {",
            "    pub(crate) resource_id: &'static str,",
            "    pub(crate) feature_owner: &'static str,",
            "    pub(crate) codec_family: ConfiguredFeatureCodecFamily,",
            "}",
            "",
            "pub const VANILLA_CONFIGURED_FEATURE_CODEC_TYPES: &[ConfiguredFeatureCodecType] = &[",
        )
    )
    for entry in entries:
        lines.extend(
            (
                "    ConfiguredFeatureCodecType {",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        feature_owner: {rust_string(entry.feature_owner)},",
                "        codec_family: "
                f"ConfiguredFeatureCodecFamily::{variant_by_owner[entry.config_codec_owner]},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn configured_feature_codec_type(",
            "    resource_id: &str,",
            ") -> Option<&'static ConfiguredFeatureCodecType> {",
            "    VANILLA_CONFIGURED_FEATURE_CODEC_TYPES",
            "        .iter()",
            "        .find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
        )
    )


def append_configured_feature_codec_catalog_test(
    lines: list[str], entries: list[ConfiguredFeatureCodecEntry]
) -> None:
    lines.extend(
        (
            "    #[test]",
            "    fn configured_feature_codec_type_preserves_registration_order_and_families() {",
            "        assert_eq!(VANILLA_CONFIGURED_FEATURE_CODEC_TYPES.len(), 60);",
            "        assert_eq!(",
            "            VANILLA_CONFIGURED_FEATURE_CODEC_TYPES",
            "                .iter()",
            "                .map(|entry| (",
            "                    entry.resource_id,",
            "                    entry.feature_owner,",
            "                    entry.codec_family.codec_owner(),",
            "                ))",
            "                .collect::<Vec<_>>(),",
            "            [",
            *(
                "                ("
                f"{rust_string(entry.resource_id)}, "
                f"{rust_string(entry.feature_owner)}, "
                f"{rust_string(entry.config_codec_owner)}),"
                for entry in entries
            ),
            "            ]",
            "        );",
            "        let mut resource_ids = HashSet::new();",
            "        let mut feature_owners = HashSet::new();",
            "        let mut family_counts = HashMap::new();",
            "        for entry in VANILLA_CONFIGURED_FEATURE_CODEC_TYPES {",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(feature_owners.insert(entry.feature_owner));",
            "            *family_counts",
            "                .entry(entry.codec_family.codec_owner())",
            "                .or_insert(0usize) += 1;",
            "            assert_eq!(",
            "                configured_feature_codec_type(entry.resource_id),",
            "                Some(entry)",
            "            );",
            "        }",
            "        assert_eq!(family_counts.len(), 39);",
            "        assert_eq!(family_counts[\"NoneFeatureConfiguration\"], 18);",
            "        assert_eq!(family_counts[\"HugeMushroomFeatureConfiguration\"], 2);",
            "        assert_eq!(family_counts[\"OreConfiguration\"], 2);",
            "        assert_eq!(family_counts[\"ProbabilityFeatureConfiguration\"], 2);",
            "        assert_eq!(family_counts[\"VegetationPatchConfiguration\"], 2);",
            "        assert_eq!(family_counts[\"LakeFeature.Configuration\"], 1);",
            "        assert_eq!(",
            "            configured_feature_codec_type(\"minecraft:lake\").unwrap().codec_family,",
            "            ConfiguredFeatureCodecFamily::LakeFeatureConfiguration",
            "        );",
            "        assert!(configured_feature_codec_type(\"minecraft:missing\").is_none());",
            "    }",
            "",
        )
    )


def render_codec_dispatch_types(
    catalogs: list[tuple[str, str, str, str, list[RegistryCodecEntry]]],
    configured_features: list[ConfiguredFeatureCodecEntry] | None = None,
) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 loot/provider bootstrap registries and static",
        "// codec registries. This catalog owns registration membership and",
        "// codec-family identity, not payload schemas. Feature.java additionally",
        "// owns configured-feature registration order and concrete feature owners.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
    ]
    for family_type, entry_type, catalog_name, lookup_name, entries in catalogs:
        append_registry_codec_catalog(
            lines,
            entries,
            family_type=family_type,
            entry_type=entry_type,
            catalog_name=catalog_name,
            lookup_name=lookup_name,
            visibility="pub",
        )
    if configured_features is not None:
        append_configured_feature_codec_catalog(lines, configured_features)
    lines.extend(
        (
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::{HashMap, HashSet};",
            "",
            "    use super::*;",
            "",
        )
    )
    for family_type, _, catalog_name, lookup_name, entries in catalogs:
        append_registry_codec_catalog_test(
            lines,
            entries,
            test_name=lookup_name,
            family_type=family_type,
            catalog_name=catalog_name,
            lookup_name=lookup_name,
        )
    if configured_features is not None:
        append_configured_feature_codec_catalog_test(lines, configured_features)
    lines.extend(("}", ""))
    return "\n".join(lines)


def render_enchantment_codec_types(
    level_based_values: list[RegistryCodecEntry],
    value_effects: list[RegistryCodecEntry],
    effect_components: list[EnchantmentEffectComponentEntry],
    entity_effects: list[RegistryCodecEntry],
    location_effects: list[RegistryCodecEntry],
    loot_context_param_sets: list[LootContextParamSetEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 LevelBasedValue.java, EnchantmentValueEffect.java,",
        "// EnchantmentEffectComponents.java, EnchantmentEntityEffect.java, and",
        "// EnchantmentLocationBasedEffect.java registration order and codec identity;",
        "// LootContextParamSets.java validation-context identity.",
        "// This catalog owns registration membership and codec-family identity,",
        "// not payload schemas.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
    ]
    catalogs = (
        (
            level_based_values,
            "LevelBasedValueCodecFamily",
            "LevelBasedValueCodecType",
            "VANILLA_LEVEL_BASED_VALUE_CODEC_TYPES",
            "level_based_value_codec_type",
            "LevelBasedValue",
            False,
        ),
        (
            value_effects,
            "EnchantmentValueEffectCodecFamily",
            "EnchantmentValueEffectCodecType",
            "VANILLA_ENCHANTMENT_VALUE_EFFECT_CODEC_TYPES",
            "enchantment_value_effect_codec_type",
            None,
            False,
        ),
        (
            entity_effects,
            "EnchantmentEntityEffectCodecFamily",
            "EnchantmentEntityEffectCodecType",
            "VANILLA_ENCHANTMENT_ENTITY_EFFECT_CODEC_TYPES",
            "enchantment_entity_effect_codec_type",
            None,
            False,
        ),
        (
            location_effects,
            "EnchantmentLocationBasedEffectCodecFamily",
            "EnchantmentLocationBasedEffectCodecType",
            "VANILLA_ENCHANTMENT_LOCATION_BASED_EFFECT_CODEC_TYPES",
            "enchantment_location_based_effect_codec_type",
            None,
            False,
        ),
    )
    for (
        entries,
        family_type,
        entry_type,
        catalog_name,
        lookup_name,
        owner,
        expect,
    ) in catalogs:
        append_registry_codec_catalog(
            lines,
            entries,
            family_type=family_type,
            entry_type=entry_type,
            catalog_name=catalog_name,
            lookup_name=lookup_name,
            common_owner=owner,
            expect_dead_code=expect,
        )
    append_effect_component_catalog(lines, effect_components, loot_context_param_sets)
    lines.extend(
        (
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use crate::loot_context_types::vanilla_loot_context_param_set;",
            "",
            "    use super::*;",
            "",
        )
    )
    for entries, family_type, _, catalog_name, lookup_name, owner, _ in catalogs:
        append_registry_codec_catalog_test(
            lines,
            entries,
            test_name=lookup_name,
            family_type=family_type,
            catalog_name=catalog_name,
            lookup_name=lookup_name,
            common_owner=owner,
        )
    lines.extend(
        (
            "    #[test]",
            "    fn enchantment_effect_component_codec_type_preserves_order_and_lookup() {",
            "        assert_eq!(",
            "            VANILLA_ENCHANTMENT_EFFECT_COMPONENT_CODEC_TYPES",
            "                .iter()",
            "                .map(|entry| entry.resource_id)",
            "                .collect::<Vec<_>>(),",
            "            [",
            *(
                f"                {rust_string(entry.resource_id)},"
                for entry in effect_components
            ),
            "            ]",
            "        );",
            "        assert_eq!(",
            "            EnchantmentEffectValidationContext::EnchantedDamage",
            "                .loot_context_param_set_resource_id(),",
            '            "minecraft:enchanted_damage"',
            "        );",
            "        let mut resource_ids = HashSet::new();",
            "        let mut validation_contexts = HashSet::new();",
            "        let mut validated_components = 0;",
            "        for entry in VANILLA_ENCHANTMENT_EFFECT_COMPONENT_CODEC_TYPES {",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert_eq!(",
            "                enchantment_effect_component_codec_type(entry.resource_id),",
            "                Some(entry)",
            "            );",
            "            if let Some(validation_context) = entry.validation_context {",
            "                validated_components += 1;",
            "                validation_contexts.insert(validation_context);",
            "                assert!(vanilla_loot_context_param_set(",
            "                    validation_context.loot_context_param_set_resource_id()",
            "                ).is_some());",
            "            }",
            "        }",
            "        assert_eq!(validated_components, 24);",
            "        assert_eq!(validation_contexts.len(), 5);",
            "        let damage =",
            '            enchantment_effect_component_codec_type("minecraft:damage").unwrap();',
            "        assert_eq!(",
            "            damage.codec_family,",
            "            EnchantmentEffectComponentCodecFamily::ConditionalValueEffect",
            "        );",
            "        assert_eq!(",
            "            damage.validation_context,",
            "            Some(EnchantmentEffectValidationContext::EnchantedDamage)",
            "        );",
            "        assert!(",
            '            enchantment_effect_component_codec_type("minecraft:missing").is_none()',
            "        );",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_enchantment_catalog(
    enchantments: list[EnchantmentEntry],
    tooltip_order: tuple[str, ...],
) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Enchantments.java bootstrap definitions and tooltip_order tag.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
    ]
    lines.extend(
        (
            "/// Vanilla 26.1 enchantment tooltip order used when no resource-pack tag overrides it.",
            "pub const VANILLA_ENCHANTMENT_TOOLTIP_ORDER: &[&str] = &[",
            *(f"    {rust_string(resource_id)}," for resource_id in tooltip_order),
            "];",
            "",
            "/// One known vanilla 26.1 enchantment identity.",
            "///",
            "/// Session holder ids must be resolved through the synchronized enchantment registry",
            "/// before this fixed-order fallback; registry elements separately own dynamic definitions.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "#[repr(u8)]",
            "pub enum VanillaEnchantmentIdentity {",
            *(f"    {rust_variant(entry.constant_name)}," for entry in enchantments),
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "struct VanillaEnchantmentDescriptor {",
            "    enchantment: VanillaEnchantmentIdentity,",
            "    resource_id: &'static str,",
            "    max_level: i32,",
            "}",
            "",
            "const VANILLA_ENCHANTMENT_DESCRIPTORS: &[VanillaEnchantmentDescriptor] = &[",
        )
    )
    for entry in enchantments:
        lines.extend(
            (
                "    VanillaEnchantmentDescriptor {",
                f"        enchantment: VanillaEnchantmentIdentity::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        max_level: {entry.max_level},",
                "    },",
            )
        )
    lines.extend(("];", ""))
    lines.extend(
        (
            "impl VanillaEnchantmentIdentity {",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_ENCHANTMENT_DESCRIPTORS",
            "            .iter()",
            "            .find(|entry| entry.resource_id == resource_id)",
            "            .map(|entry| entry.enchantment)",
            "    }",
            "",
            "    /// Resolves the vanilla 26.1 bootstrap ordinal used when no session registry is available.",
            "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
            "        VANILLA_ENCHANTMENT_DESCRIPTORS",
            "            .get(usize::try_from(registry_id).ok()?)",
            "            .map(|entry| entry.enchantment)",
            "    }",
            "",
            "    pub const fn vanilla_registry_index(self) -> usize {",
            "        self as usize",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_ENCHANTMENT_DESCRIPTORS[self.vanilla_registry_index()].resource_id",
            "    }",
            "",
            "    pub fn max_level(self) -> i32 {",
            "        VANILLA_ENCHANTMENT_DESCRIPTORS[self.vanilla_registry_index()].max_level",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn enchantment_catalog_is_complete_ordered_and_unique() {",
        )
    )
    lines.extend(
        (
            f"        assert_eq!(VANILLA_ENCHANTMENT_DESCRIPTORS.len(), {len(enchantments)});",
            f"        assert_eq!(VANILLA_ENCHANTMENT_TOOLTIP_ORDER.len(), {len(tooltip_order)});",
            "        let mut enchantment_resources = HashSet::new();",
            "        for (index, entry) in VANILLA_ENCHANTMENT_DESCRIPTORS.iter().enumerate() {",
            "            assert_eq!(entry.enchantment.vanilla_registry_index(), index);",
            "            assert!(enchantment_resources.insert(entry.resource_id));",
            '            assert!(entry.resource_id.starts_with("minecraft:"));',
            "            assert!((1..=255).contains(&entry.max_level));",
            "        }",
            "        let tooltip_resources: HashSet<_> = VANILLA_ENCHANTMENT_TOOLTIP_ORDER",
            "            .iter()",
            "            .copied()",
            "            .collect();",
            "        assert_eq!(tooltip_resources, enchantment_resources);",
            "    }",
            "",
            "    #[test]",
            "    fn enchantment_catalog_preserves_vanilla_mappings() {",
            '        assert_eq!(VanillaEnchantmentIdentity::for_vanilla_registry_id(13), Some(VanillaEnchantmentIdentity::Sharpness));',
            '        assert_eq!(VanillaEnchantmentIdentity::Sharpness.resource_id(), "minecraft:sharpness");',
            "        assert_eq!(VanillaEnchantmentIdentity::Sharpness.max_level(), 5);",
            '        assert_eq!(VanillaEnchantmentIdentity::for_resource_id("minecraft:mending"), Some(VanillaEnchantmentIdentity::Mending));',
            '        assert_eq!(VanillaEnchantmentIdentity::for_resource_id("example:custom_enchantment"), None);',
            "        assert_eq!(VanillaEnchantmentIdentity::for_vanilla_registry_id(-1), None);",
            f"        assert_eq!(VanillaEnchantmentIdentity::for_vanilla_registry_id({len(enchantments)}), None);",
            '        assert_eq!(VANILLA_ENCHANTMENT_TOOLTIP_ORDER.first(), Some(&"minecraft:binding_curse"));',
            '        assert_eq!(VANILLA_ENCHANTMENT_TOOLTIP_ORDER.last(), Some(&"minecraft:mending"));',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_enchantment_catalog_bridge() -> str:
    return "\n".join(
        (
            "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
            "// Owner: bbb-facts enchantment_types; this bridge preserves the bbb-client-world API.",
            "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
            "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
            "",
            "pub use bbb_protocol::enchantment_types::{",
            "    VanillaEnchantmentIdentity, VANILLA_ENCHANTMENT_TOOLTIP_ORDER,",
            "};",
            "",
        )
    )


def persistent_registry_entry_ids(
    source_root: Path,
) -> dict[str, tuple[str, ...]]:
    entries: dict[str, tuple[str, ...]] = {}
    for registry_id, relative_directory in PERSISTENT_REGISTRY_ENTRY_SOURCE_DIRS.items():
        directory = source_root / relative_directory
        if not directory.is_dir():
            raise VanillaTagSourceError(
                f"{relative_directory.as_posix()}: vanilla registry entry directory "
                "does not exist"
            )
        entries[registry_id] = tuple(
            f"minecraft:{path.stem}" for path in sorted(directory.glob("*.json"))
        )
    return entries


def chunk_generator_entry_ids(
    source_root: Path,
) -> tuple[str, ...]:
    """Extracts `ChunkGenerators.bootstrap` registration order (`ChunkGenerators.java`)."""
    source_path = source_root / CHUNK_GENERATORS_SOURCE
    if not source_path.is_file():
        raise VanillaTagSourceError(
            f"{CHUNK_GENERATORS_SOURCE.as_posix()}: vanilla chunk generator "
            "bootstrap source does not exist"
        )
    return tuple(
        f"minecraft:{name}"
        for name in re.findall(
            r'Registry\.register\(registry, "([^"]+)"', source_path.read_text()
        )
    )


def _pack_persistent_registry_tags_tables(
    registry_tags: dict[str, dict[str, tuple[str, ...]]],
) -> tuple[bytes, int, int, int, int]:
    str_bytes = bytearray()
    str_spans: list[tuple[int, int]] = []
    str_index: dict[str, int] = {}

    def intern(text: str) -> int:
        idx = str_index.get(text)
        if idx is None:
            encoded = text.encode("utf-8")
            if len(encoded) > 0xFFFF or len(str_spans) > 0xFFFF:
                raise ValueError("persistent registry tag string pool exceeds u16")
            idx = len(str_spans)
            str_spans.append((len(str_bytes), len(encoded)))
            str_bytes.extend(encoded)
            str_index[text] = idx
        return idx

    member_indices: list[int] = []
    tag_records: list[tuple[int, int, int, int]] = []
    for registry_id, tags in sorted(registry_tags.items()):
        reg_idx = intern(registry_id)
        for tag_id, members in sorted(tags.items()):
            tag_idx = intern(tag_id)
            start = len(member_indices)
            for member in members:
                member_indices.append(intern(member))
            tag_records.append((reg_idx, tag_idx, start, len(members)))

    out = bytearray(str_bytes)
    spans_offset = len(out)
    for off, length in str_spans:
        out.extend(struct.pack("<IH", off, length))
    members_offset = len(out)
    for member_idx in member_indices:
        out.extend(struct.pack("<H", member_idx))
    tags_offset = len(out)
    for reg_idx, tag_idx, start, count in tag_records:
        out.extend(struct.pack("<HHII", reg_idx, tag_idx, start, count))

    return bytes(out), len(tag_records), spans_offset, members_offset, tags_offset


def pack_persistent_registry_tags(
    registry_tags: dict[str, dict[str, tuple[str, ...]]],
) -> bytes:
    return _pack_persistent_registry_tags_tables(registry_tags)[0]


def render_persistent_registry_facts(
    registry_tags: dict[str, dict[str, tuple[str, ...]]],
    registry_entry_ids: dict[str, tuple[str, ...]],
    banner_patterns: list[BannerPatternEntry],
    bin_rel_path: str = "../data/persistent_registry_tags_26_1.bin",
) -> str:
    (
        _blob,
        tag_count,
        spans_offset,
        members_offset,
        tags_offset,
    ) = _pack_persistent_registry_tags_tables(registry_tags)
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 data/minecraft/tags/"
        "{banner_pattern,block,damage_type,enchantment,entity_type,fluid,"
        "instrument,item,painting_variant};",
        "// vanilla 26.1 data/minecraft/"
        "{dimension_type,instrument,trim_material,trim_pattern} entry ids;",
        "// vanilla 26.1 ChunkGenerators.java bootstrap registration order;",
        "// fixed catalog sources: vanilla 26.1 Attributes.java, Potions.java"
        " with MobEffects.java, MapDecorationTypes.java, and Stats.java with"
        " BuiltInRegistries.java;",
        "// built-in attribute, block, block entity type, banner pattern,"
        " custom stat, damage type, enchantment, entity type, game event,"
        " item, map decoration type, mob effect, potion, jukebox song, sound"
        " event, stat type, villager profession, and fixed variant catalogs.",
        "// This snapshot intentionally excludes world and data-pack overlays.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "use crate::{",
        "    animal_sound_variant_types::{",
        "        CatSoundVariantIdentity, ChickenSoundVariantIdentity, CowSoundVariantIdentity,",
        "        PigSoundVariantIdentity, WolfSoundVariantIdentity,",
        "    },",
        "    attribute_types::vanilla_attribute_type_for_resource_id,",
        "    block_entity_types::vanilla_block_entity_type_for_resource_id,",
        "    block_types::vanilla_block_type_for_resource_id,",
        "    damage_types::vanilla_damage_type_for_resource_id,",
        "    debug_subscription_types::VANILLA_DEBUG_GAME_EVENT_TYPES,",
        "    entity_types::vanilla_entity_type_for_resource_id,",
        "    item_types::vanilla_item_type_for_resource_id,",
        "    enchantment_types::VanillaEnchantmentIdentity,",
        "    jukebox_song_types::vanilla_26_1_jukebox_song_by_song_id,",
        "    mob_effect_types::vanilla_mob_effect_type_for_resource_id,",
        "    painting_variant_types::vanilla_painting_variant_for_resource_id,",
        "    potion_types::vanilla_potion_for_resource_id,",
        "    registry_data_types::VanillaFluidIdentity,",
        "    sound_event_types::vanilla_sound_event_registry_id,",
        "    stat_types::{vanilla_custom_stat_by_resource_id, vanilla_stat_type_by_resource_id},",
        "    temperature_animal_variants::{",
        "        ChickenModelVariant, CowModelVariant, FrogModelVariant, PigModelVariant,",
        "    },",
        "    vanilla_map_decoration_type_for_resource_id,",
        "    wolf_variants::WolfModelVariant,",
        "    zombie_nautilus_variants::ZombieNautilusModelVariant,",
        "    BannerPatternKind, CatModelVariant, PersistentRegistryMembership,"
        " PersistentRegistryTagMembers,",
        "    VillagerModelProfession, VillagerModelType,",
        "};",
        "",
        f'const TAG_BYTES: &[u8] = include_bytes!("{bin_rel_path}");',
        f"const TAG_COUNT: usize = {tag_count};",
        f"const STRING_SPANS_OFFSET: usize = {spans_offset};",
        f"const MEMBER_INDICES_OFFSET: usize = {members_offset};",
        f"const TAG_RECORDS_OFFSET: usize = {tags_offset};",
        "",
        "#[inline]",
        "fn u16_at(offset: usize) -> u16 {",
        "    u16::from_le_bytes([TAG_BYTES[offset], TAG_BYTES[offset + 1]])",
        "}",
        "",
        "#[inline]",
        "fn u32_at(offset: usize) -> u32 {",
        "    u32::from_le_bytes([",
        "        TAG_BYTES[offset],",
        "        TAG_BYTES[offset + 1],",
        "        TAG_BYTES[offset + 2],",
        "        TAG_BYTES[offset + 3],",
        "    ])",
        "}",
        "",
        "#[inline]",
        "fn tag_str_at(index: u16) -> &'static str {",
        "    let span = STRING_SPANS_OFFSET + usize::from(index) * 6;",
        "    let start = u32_at(span) as usize;",
        "    let len = usize::from(u16_at(span + 4));",
        "    core::str::from_utf8(&TAG_BYTES[start..start + len])",
        '        .expect("valid UTF-8 in persistent_registry_tags_26_1.bin")',
        "}",
        "",
        "#[inline]",
        "fn tag_record_at(index: usize) -> (&'static str, &'static str, usize, usize) {",
        "    let base = TAG_RECORDS_OFFSET + index * 12;",
        "    (",
        "        tag_str_at(u16_at(base)),",
        "        tag_str_at(u16_at(base + 2)),",
        "        u32_at(base + 4) as usize,",
        "        u32_at(base + 8) as usize,",
        "    )",
        "}",
        "",
        "#[inline]",
        "fn tag_member_at(index: usize) -> &'static str {",
        "    tag_str_at(u16_at(MEMBER_INDICES_OFFSET + index * 2))",
        "}",
        "",
    ]
    for registry_id, entry_ids in sorted(registry_entry_ids.items()):
        const_name = (
            "VANILLA_"
            + registry_id.removeprefix("minecraft:").upper()
            + "_REGISTRY_IDS_26_1"
        )
        lines.append(f"const {const_name}: &[&str] = &[")
        lines.extend(f"    {rust_string(entry_id)}," for entry_id in entry_ids)
        lines.extend(("];", ""))
    lines.extend(
        (
            "/// Checks one resource against the immutable Vanilla 26.1 built-in registry view.",
            "pub fn vanilla_persistent_registry_membership_26_1(",
            "    registry_id: &str,",
            "    resource_id: &str,",
            ") -> PersistentRegistryMembership {",
            "    let Some(contains) = vanilla_registry_member_lookup(registry_id) else {",
            "        return PersistentRegistryMembership::Unavailable;",
            "    };",
            "    if contains(resource_id) {",
            "        PersistentRegistryMembership::Present",
            "    } else {",
            "        PersistentRegistryMembership::Absent",
            "    }",
            "}",
            "",
            "pub fn vanilla_persistent_registry_available_26_1(registry_id: &str) -> bool {",
            "    vanilla_registry_member_lookup(registry_id).is_some()",
            "}",
            "",
            "fn vanilla_registry_member_lookup(registry_id: &str) -> Option<fn(&str) -> bool> {",
            "    let contains: fn(&str) -> bool = match registry_id {",
            '        "minecraft:attribute" =>'
            " |resource_id| vanilla_attribute_type_for_resource_id(resource_id).is_some(),",
            '        "minecraft:block" =>'
            " |resource_id| vanilla_block_type_for_resource_id(resource_id).is_some(),",
            '        "minecraft:block_entity_type" =>'
            " |resource_id| vanilla_block_entity_type_for_resource_id(resource_id).is_some(),",
            '        "minecraft:damage_type" =>'
            " |resource_id| vanilla_damage_type_for_resource_id(resource_id).is_some(),",
            '        "minecraft:enchantment" =>'
            " |resource_id| VanillaEnchantmentIdentity::for_resource_id(resource_id).is_some(),",
            '        "minecraft:entity_type" =>'
            " |resource_id| vanilla_entity_type_for_resource_id(resource_id).is_some(),",
            '        "minecraft:fluid" =>'
            " |resource_id| VanillaFluidIdentity::for_resource_id(resource_id).is_some(),",
            '        "minecraft:game_event" => |resource_id|'
            " VANILLA_DEBUG_GAME_EVENT_TYPES.iter().any(|entry| entry.resource_id == resource_id),",
            '        "minecraft:item" =>'
            " |resource_id| vanilla_item_type_for_resource_id(resource_id).is_some(),",
            '        "minecraft:map_decoration_type" =>'
            " |resource_id| vanilla_map_decoration_type_for_resource_id(resource_id).is_some(),",
            '        "minecraft:banner_pattern" =>'
            " |resource_id| BannerPatternKind::for_resource_id(resource_id).is_some(),",
            '        "minecraft:mob_effect" =>'
            " |resource_id| vanilla_mob_effect_type_for_resource_id(resource_id).is_some(),",
            '        "minecraft:jukebox_song" =>'
            " |resource_id| vanilla_26_1_jukebox_song_by_song_id(resource_id).is_some(),",
            '        "minecraft:sound_event" =>'
            " |resource_id| vanilla_sound_event_registry_id(resource_id).is_some(),",
            '        "minecraft:stat_type" =>'
            " |resource_id| vanilla_stat_type_by_resource_id(resource_id).is_some(),",
            '        "minecraft:villager_profession" =>'
            " |resource_id| VillagerModelProfession::for_resource_id(resource_id).is_some(),",
            '        "minecraft:villager_type" =>'
            " |resource_id| VillagerModelType::for_resource_id(resource_id).is_some(),",
            '        "minecraft:wolf_variant" =>'
            " |resource_id| WolfModelVariant::for_resource_id(resource_id).is_some(),",
            '        "minecraft:wolf_sound_variant" =>'
            " |resource_id| WolfSoundVariantIdentity::for_resource_id(resource_id).is_some(),",
            '        "minecraft:pig_variant" =>'
            " |resource_id| PigModelVariant::for_resource_id(resource_id).is_some(),",
            '        "minecraft:pig_sound_variant" =>'
            " |resource_id| PigSoundVariantIdentity::for_resource_id(resource_id).is_some(),",
            '        "minecraft:cow_variant" =>'
            " |resource_id| CowModelVariant::for_resource_id(resource_id).is_some(),",
            '        "minecraft:cow_sound_variant" =>'
            " |resource_id| CowSoundVariantIdentity::for_resource_id(resource_id).is_some(),",
            '        "minecraft:chicken_variant" =>'
            " |resource_id| ChickenModelVariant::for_resource_id(resource_id).is_some(),",
            '        "minecraft:chicken_sound_variant" =>'
            " |resource_id| ChickenSoundVariantIdentity::for_resource_id(resource_id).is_some(),",
            '        "minecraft:zombie_nautilus_variant" =>'
            " |resource_id| ZombieNautilusModelVariant::for_resource_id(resource_id).is_some(),",
            '        "minecraft:frog_variant" =>'
            " |resource_id| FrogModelVariant::for_resource_id(resource_id).is_some(),",
            '        "minecraft:painting_variant" =>'
            " |resource_id| vanilla_painting_variant_for_resource_id(resource_id).is_some(),",
            '        "minecraft:potion" =>'
            " |resource_id| vanilla_potion_for_resource_id(resource_id).is_some(),",
            '        "minecraft:cat_variant" =>'
            " |resource_id| CatModelVariant::for_resource_id(resource_id).is_some(),",
            '        "minecraft:cat_sound_variant" =>'
            " |resource_id| CatSoundVariantIdentity::for_resource_id(resource_id).is_some(),",
            '        "minecraft:custom_stat" =>'
            " |resource_id| vanilla_custom_stat_by_resource_id(resource_id).is_some(),",
            '        "minecraft:trim_material" =>'
            " |resource_id| VANILLA_TRIM_MATERIAL_REGISTRY_IDS_26_1.contains(&resource_id),",
            '        "minecraft:trim_pattern" =>'
            " |resource_id| VANILLA_TRIM_PATTERN_REGISTRY_IDS_26_1.contains(&resource_id),",
            '        "minecraft:instrument" =>'
            " |resource_id| VANILLA_INSTRUMENT_REGISTRY_IDS_26_1.contains(&resource_id),",
            '        "minecraft:dimension_type" =>'
            " |resource_id| VANILLA_DIMENSION_TYPE_REGISTRY_IDS_26_1.contains(&resource_id),",
            '        "minecraft:chunk_generator" =>'
            " |resource_id| VANILLA_CHUNK_GENERATOR_REGISTRY_IDS_26_1.contains(&resource_id),",
            "        _ => return None,",
            "    };",
            "    Some(contains)",
            "}",
            "",
            "/// Resolves a fully expanded base Vanilla 26.1 tag without pack overlays.",
            "pub fn vanilla_persistent_registry_tag_members_26_1(",
            "    registry_id: &str,",
            "    tag_id: &str,",
            ") -> PersistentRegistryTagMembers {",
            "    match registry_id {",
            '        "minecraft:mob_effect"',
            '        | "minecraft:jukebox_song"',
            '        | "minecraft:sound_event"',
            '        | "minecraft:villager_profession"',
            '        | "minecraft:villager_type"',
            '        | "minecraft:wolf_variant"',
            '        | "minecraft:wolf_sound_variant"',
            '        | "minecraft:pig_variant"',
            '        | "minecraft:pig_sound_variant"',
            '        | "minecraft:cow_variant"',
            '        | "minecraft:cow_sound_variant"',
            '        | "minecraft:chicken_variant"',
            '        | "minecraft:chicken_sound_variant"',
            '        | "minecraft:zombie_nautilus_variant"',
            '        | "minecraft:frog_variant"',
            '        | "minecraft:cat_variant"',
            '        | "minecraft:cat_sound_variant"',
            '        | "minecraft:trim_material"',
            '        | "minecraft:trim_pattern"',
            '        | "minecraft:dimension_type"',
            '        | "minecraft:chunk_generator" => {',
            "            return PersistentRegistryTagMembers::Absent;",
            "        }",
            '        "minecraft:block" | "minecraft:damage_type" | "minecraft:enchantment"'
            ' | "minecraft:entity_type" | "minecraft:fluid" | "minecraft:item"'
            ' | "minecraft:painting_variant" | "minecraft:banner_pattern"'
            ' | "minecraft:instrument" => {}',
            "        _ => return PersistentRegistryTagMembers::Unavailable,",
            "    }",
            "    let mut lo = 0usize;",
            "    let mut hi = TAG_COUNT;",
            "    while lo < hi {",
            "        let mid = lo + (hi - lo) / 2;",
            "        let (entry_registry, entry_tag, start, count) = tag_record_at(mid);",
            "        match (entry_registry, entry_tag).cmp(&(registry_id, tag_id)) {",
            "            core::cmp::Ordering::Less => lo = mid + 1,",
            "            core::cmp::Ordering::Greater => hi = mid,",
            "            core::cmp::Ordering::Equal => {",
            "                return PersistentRegistryTagMembers::Present(",
            "                    (start..start + count)",
            "                        .map(|pos| tag_member_at(pos).to_owned())",
            "                        .collect(),",
            "                );",
            "            }",
            "        }",
            "    }",
            "    PersistentRegistryTagMembers::Absent",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use super::*;",
            "",
            "    fn count_tags_for_registry(registry_id: &str) -> usize {",
            "        (0..TAG_COUNT)",
            "            .filter(|&idx| tag_record_at(idx).0 == registry_id)",
            "            .count()",
            "    }",
            "",
            "    #[test]",
            "    fn membership_distinguishes_known_unknown_custom_and_unavailable_resources() {",
            "        for (registry_id, known) in [",
            '            ("minecraft:attribute", "minecraft:max_health"),',
            '            ("minecraft:block", "minecraft:stone"),',
            '            ("minecraft:damage_type", "minecraft:generic"),',
            '            ("minecraft:enchantment", "minecraft:mending"),',
            '            ("minecraft:entity_type", "minecraft:pig"),',
            '            ("minecraft:game_event", "minecraft:step"),',
            '            ("minecraft:item", "minecraft:stone"),',
            '            ("minecraft:map_decoration_type", "minecraft:mansion"),',
            '            ("minecraft:banner_pattern", "minecraft:base"),',
            '            ("minecraft:mob_effect", "minecraft:speed"),',
            '            ("minecraft:jukebox_song", "minecraft:cat"),',
            '            ("minecraft:sound_event", "minecraft:ambient.cave"),',
            '            ("minecraft:stat_type", "minecraft:custom"),',
            '            ("minecraft:villager_profession", "minecraft:farmer"),',
            '            ("minecraft:villager_type", "minecraft:plains"),',
            '            ("minecraft:wolf_variant", "minecraft:pale"),',
            '            ("minecraft:wolf_sound_variant", "minecraft:cute"),',
            '            ("minecraft:pig_variant", "minecraft:warm"),',
            '            ("minecraft:pig_sound_variant", "minecraft:mini"),',
            '            ("minecraft:cow_variant", "minecraft:cold"),',
            '            ("minecraft:cow_sound_variant", "minecraft:moody"),',
            '            ("minecraft:chicken_variant", "minecraft:temperate"),',
            '            ("minecraft:chicken_sound_variant", "minecraft:picky"),',
            '            ("minecraft:zombie_nautilus_variant", "minecraft:temperate"),',
            '            ("minecraft:frog_variant", "minecraft:cold"),',
            '            ("minecraft:painting_variant", "minecraft:dennis"),',
            '            ("minecraft:potion", "minecraft:awkward"),',
            '            ("minecraft:cat_variant", "minecraft:jellie"),',
            '            ("minecraft:cat_sound_variant", "minecraft:royal"),',
            '            ("minecraft:custom_stat", "minecraft:jump"),',
            '            ("minecraft:trim_material", "minecraft:resin"),',
            '            ("minecraft:trim_pattern", "minecraft:rib"),',
            '            ("minecraft:instrument", "minecraft:ponder_goat_horn"),',
            '            ("minecraft:dimension_type", "minecraft:overworld"),',
            '            ("minecraft:chunk_generator", "minecraft:noise"),',
            "        ] {",
            "            assert!(vanilla_persistent_registry_available_26_1(registry_id));",
            "            assert_eq!(vanilla_persistent_registry_membership_26_1("
            "registry_id, known), PersistentRegistryMembership::Present);",
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            'registry_id, "minecraft:not_registered"), PersistentRegistryMembership::Absent);',
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            'registry_id, "example:custom"), PersistentRegistryMembership::Absent);',
            "        }",
            '        assert!(!vanilla_persistent_registry_available_26_1("example:registry"));',
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"example:registry", "minecraft:stone"), PersistentRegistryMembership::Unavailable);',
            "    }",
            "",
            "    #[test]",
            "    fn stat_type_and_custom_stat_catalog_membership_is_complete() {",
            "        assert_eq!(crate::stat_types::VANILLA_STAT_TYPES.len(), 9);",
            "        for entry in crate::stat_types::VANILLA_STAT_TYPES {",
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:stat_type", entry.resource_id), PersistentRegistryMembership::Present);',
            "        }",
            "        assert_eq!(crate::stat_types::VANILLA_CUSTOM_STATS.len(), 77);",
            "        for entry in crate::stat_types::VANILLA_CUSTOM_STATS {",
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:custom_stat", entry.resource_id), PersistentRegistryMembership::Present);',
            "        }",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:stat_type", "minecraft:not_registered"),'
            " PersistentRegistryMembership::Absent);",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:custom_stat", "minecraft:not_registered"),'
            " PersistentRegistryMembership::Absent);",
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1("minecraft:stat_type", "minecraft:any"), PersistentRegistryTagMembers::Unavailable);',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1("minecraft:custom_stat", "minecraft:any"), PersistentRegistryTagMembers::Unavailable);',
            "    }",
            "",
            "    #[test]",
            "    fn attribute_potion_and_map_decoration_catalog_membership_is_complete() {",
            "        assert_eq!(crate::attribute_types::VANILLA_ATTRIBUTE_TYPES.len(), 35);",
            "        for entry in crate::attribute_types::VANILLA_ATTRIBUTE_TYPES {",
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:attribute", entry.resource_id), PersistentRegistryMembership::Present);',
            "        }",
            "        assert_eq!(crate::potion_types::VANILLA_POTIONS.len(), 46);",
            "        for entry in crate::potion_types::VANILLA_POTIONS {",
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:potion", entry.resource_id), PersistentRegistryMembership::Present);',
            "        }",
            "        let map_decorations ="
            " crate::vanilla_map_decoration_types().collect::<Vec<_>>();",
            "        assert_eq!(map_decorations.len(), 35);",
            "        for entry in map_decorations {",
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:map_decoration_type", entry.resource_id),'
            " PersistentRegistryMembership::Present);",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn game_event_named_membership_reuses_catalog_without_claiming_tag_facts() {",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:game_event", "minecraft:step"), PersistentRegistryMembership::Present);',
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:game_event", "minecraft:resonate_15"),'
            " PersistentRegistryMembership::Present);",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:game_event", "minecraft:not_registered"),'
            " PersistentRegistryMembership::Absent);",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:game_event", "example:custom"), PersistentRegistryMembership::Absent);',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:game_event", "minecraft:any"),'
            " PersistentRegistryTagMembers::Unavailable);",
            "    }",
            "",
            "    #[test]",
            "    fn entity_type_catalog_and_tags_are_complete() {",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:entity_type", "minecraft:pig"), PersistentRegistryMembership::Present);',
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:entity_type", "minecraft:not_registered"),'
            " PersistentRegistryMembership::Absent);",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:entity_type", "example:custom"), PersistentRegistryMembership::Absent);',
            '        assert_eq!(count_tags_for_registry("minecraft:entity_type"), 47);',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:entity_type", "minecraft:can_equip_saddle"),'
            ' PersistentRegistryTagMembers::Present(vec!["minecraft:horse".to_owned(),'
            ' "minecraft:skeleton_horse".to_owned(), "minecraft:zombie_horse".to_owned(),'
            ' "minecraft:donkey".to_owned(), "minecraft:mule".to_owned(),'
            ' "minecraft:pig".to_owned(), "minecraft:strider".to_owned(),'
            ' "minecraft:camel".to_owned(), "minecraft:camel_husk".to_owned(),'
            ' "minecraft:nautilus".to_owned(), "minecraft:zombie_nautilus".to_owned()]));',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:entity_type", "minecraft:can_equip_harness"),'
            ' PersistentRegistryTagMembers::Present(vec!["minecraft:happy_ghast".to_owned()]));',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:entity_type", "minecraft:not_a_tag"),'
            " PersistentRegistryTagMembers::Absent);",
            "    }",
            "",
            "    #[test]",
            "    fn block_entity_membership_reuses_the_fixed_catalog_without_tag_facts() {",
            "        for entry in crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES {",
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:block_entity_type", entry.resource_id),'
            " PersistentRegistryMembership::Present);",
            "        }",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:block_entity_type", "minecraft:not_registered"),'
            " PersistentRegistryMembership::Absent);",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:block_entity_type", "example:custom"),'
            " PersistentRegistryMembership::Absent);",
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:block_entity_type", "minecraft:any"),'
            " PersistentRegistryTagMembers::Unavailable);",
            "    }",
            "",
            "    #[test]",
            "    fn banner_pattern_catalog_and_tags_are_complete() {",
            "        let known = [",
            *(f"            {rust_string(entry.resource_id)}," for entry in banner_patterns),
            "        ];",
            f"        assert_eq!(known.len(), {len(banner_patterns)});",
            "        for resource_id in known {",
            '            assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:banner_pattern", resource_id), PersistentRegistryMembership::Present);',
            "        }",
            '        assert_eq!(count_tags_for_registry("minecraft:banner_pattern"), 11);',
            '        let PersistentRegistryTagMembers::Present(no_item_required) ='
            ' vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:banner_pattern", "minecraft:no_item_required")'
            ' else { panic!("missing banner no_item_required tag") };',
            "        assert_eq!(no_item_required.len(), 32);",
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:banner_pattern", "minecraft:pattern_item/creeper"),'
            ' PersistentRegistryTagMembers::Present(vec!["minecraft:creeper".to_owned()]));',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:banner_pattern", "minecraft:not_a_tag"),'
            " PersistentRegistryTagMembers::Absent);",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:banner_pattern", "example:custom"),'
            " PersistentRegistryMembership::Absent);",
            "    }",
            "",
            "    #[test]",
            "    fn item_catalog_and_tags_are_complete() {",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:item", "minecraft:stone"), PersistentRegistryMembership::Present);',
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:item", "minecraft:leather"), PersistentRegistryMembership::Present);',
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:item", "minecraft:not_registered"),'
            " PersistentRegistryMembership::Absent);",
            '        assert_eq!(vanilla_persistent_registry_membership_26_1('
            '"minecraft:item", "example:custom"), PersistentRegistryMembership::Absent);',
            '        assert_eq!(count_tags_for_registry("minecraft:item"), 207);',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:item", "minecraft:repairs_leather_armor"),'
            ' PersistentRegistryTagMembers::Present(vec!["minecraft:leather".to_owned()]));',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:item", "minecraft:not_a_tag"), PersistentRegistryTagMembers::Absent);',
            "    }",
            "",
            "    #[test]",
            "    fn tags_distinguish_nested_empty_missing_and_unavailable_facts() {",
            '        let PersistentRegistryTagMembers::Present(signs) ='
            ' vanilla_persistent_registry_tag_members_26_1("minecraft:block", "minecraft:all_signs")'
            ' else { panic!("missing nested block tag") };',
            "        assert!(!signs.is_empty());",
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:block", "minecraft:incorrect_for_diamond_tool"),'
            " PersistentRegistryTagMembers::Present(Vec::new()));",
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:block", "minecraft:not_a_tag"), PersistentRegistryTagMembers::Absent);',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:mob_effect", "minecraft:any"), PersistentRegistryTagMembers::Absent);',
            '        let PersistentRegistryTagMembers::Present(placeable) ='
            ' vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:painting_variant", "minecraft:placeable")'
            ' else { panic!("missing painting placeable tag") };',
            "        assert_eq!(placeable.len(), 47);",
            '        assert_eq!(placeable.first().map(String::as_str), Some("minecraft:kebab"));',
            '        assert_eq!(placeable.last().map(String::as_str), Some("minecraft:dennis"));',
            '        for registry_id in ["minecraft:jukebox_song",'
            ' "minecraft:villager_profession", "minecraft:villager_type",'
            ' "minecraft:wolf_variant", "minecraft:wolf_sound_variant",'
            ' "minecraft:pig_variant", "minecraft:pig_sound_variant",'
            ' "minecraft:cow_variant", "minecraft:cow_sound_variant",'
            ' "minecraft:chicken_variant", "minecraft:chicken_sound_variant",'
            ' "minecraft:zombie_nautilus_variant", "minecraft:frog_variant",'
            ' "minecraft:cat_variant", "minecraft:cat_sound_variant",'
            ' "minecraft:trim_material", "minecraft:trim_pattern",'
            ' "minecraft:dimension_type", "minecraft:chunk_generator"] {',
            '            assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            'registry_id, "minecraft:any"), PersistentRegistryTagMembers::Absent,'
            ' "{registry_id}");',
            "        }",
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"example:registry", "minecraft:any"), PersistentRegistryTagMembers::Unavailable);',
            "    }",
            "",
            "    #[test]",
            "    fn instrument_catalog_and_tags_are_complete() {",
            "        assert_eq!(VANILLA_INSTRUMENT_REGISTRY_IDS_26_1.len(), 8);",
            "        assert_eq!(VANILLA_TRIM_MATERIAL_REGISTRY_IDS_26_1.len(), 11);",
            "        assert_eq!(VANILLA_TRIM_PATTERN_REGISTRY_IDS_26_1.len(), 18);",
            '        assert_eq!(count_tags_for_registry("minecraft:instrument"), 3);',
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:instrument", "minecraft:regular_goat_horns"),'
            ' PersistentRegistryTagMembers::Present(vec!['
            '"minecraft:ponder_goat_horn".to_owned(), "minecraft:sing_goat_horn".to_owned(),'
            ' "minecraft:seek_goat_horn".to_owned(), "minecraft:feel_goat_horn".to_owned()]));',
            '        let PersistentRegistryTagMembers::Present(goat_horns) ='
            ' vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:instrument", "minecraft:goat_horns")'
            ' else { panic!("missing nested instrument tag") };',
            "        assert_eq!(goat_horns.len(), 8);",
            '        assert_eq!(vanilla_persistent_registry_tag_members_26_1('
            '"minecraft:instrument", "minecraft:not_a_tag"),'
            " PersistentRegistryTagMembers::Absent);",
            "    }",
            "",
            "    #[test]",
            "    fn dimension_type_and_chunk_generator_catalogs_are_complete() {",
            "        assert_eq!(VANILLA_DIMENSION_TYPE_REGISTRY_IDS_26_1.len(), 4);",
            '        assert_eq!(VANILLA_DIMENSION_TYPE_REGISTRY_IDS_26_1.first(),'
            ' Some(&"minecraft:overworld"));',
            '        assert_eq!(VANILLA_DIMENSION_TYPE_REGISTRY_IDS_26_1.last(),'
            ' Some(&"minecraft:the_nether"));',
            '        assert_eq!(VANILLA_CHUNK_GENERATOR_REGISTRY_IDS_26_1,'
            ' ["minecraft:noise", "minecraft:flat", "minecraft:debug"].as_slice());',
            "    }",
            "",
            "    #[test]",
            "    fn every_generated_tag_member_exists_in_its_registry() {",
            "        for idx in 0..TAG_COUNT {",
            "            let (registry_id, tag_id, start, count) = tag_record_at(idx);",
            "            for pos in start..start + count {",
            "                let member = tag_member_at(pos);",
            "                assert_eq!(",
            "                    vanilla_persistent_registry_membership_26_1(registry_id, member),",
            "                    PersistentRegistryMembership::Present,",
            '                    "{} {} contains missing member {}",',
            "                    registry_id,",
            "                    tag_id,",
            "                    member",
            "                );",
            "            }",
            "        }",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_painting_variants(entries: list[PaintingVariantEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 PaintingVariants.java bootstrap order and properties.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct VanillaPaintingVariant {",
        "    pub id: i32,",
        "    pub resource_id: &'static str,",
        "    pub width: i32,",
        "    pub height: i32,",
        "    pub has_author: bool,",
        "}",
        "",
        "pub const VANILLA_PAINTING_VARIANTS: &[VanillaPaintingVariant] = &[",
    ]
    for variant_id, entry in enumerate(entries):
        lines.extend(
            (
                "    VanillaPaintingVariant {",
                f"        id: {variant_id},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        width: {entry.width},",
                f"        height: {entry.height},",
                f"        has_author: {str(entry.has_author).lower()},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_painting_variant(id: i32) -> Option<&'static VanillaPaintingVariant> {",
            "    VANILLA_PAINTING_VARIANTS.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_painting_variant_for_resource_id(resource_id: &str) -> Option<&'static VanillaPaintingVariant> {",
            "    VANILLA_PAINTING_VARIANTS.iter().find(|variant| variant.resource_id == resource_id)",
            "}",
            "",
            "pub fn vanilla_painting_variant_resource_id(id: i32) -> Option<&'static str> {",
            "    vanilla_painting_variant(id).map(|variant| variant.resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn painting_variant_catalog_is_complete_contiguous_and_unique() {",
            f"        assert_eq!(VANILLA_PAINTING_VARIANTS.len(), {len(entries)});",
            "        let mut resources = HashSet::new();",
            "        for (id, entry) in VANILLA_PAINTING_VARIANTS.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(entry.width > 0);",
            "            assert!(entry.height > 0);",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn painting_variant_catalog_preserves_vanilla_order_and_properties() {",
            "        assert_eq!(vanilla_painting_variant_resource_id(0), Some(\"minecraft:kebab\"));",
            "        assert_eq!(vanilla_painting_variant_resource_id(50), Some(\"minecraft:dennis\"));",
            "        assert_eq!(vanilla_painting_variant_resource_id(-1), None);",
            "        assert_eq!(vanilla_painting_variant_resource_id(51), None);",
            "        assert!(!vanilla_painting_variant(19).unwrap().has_author);",
            "        let dennis = vanilla_painting_variant(50).unwrap();",
            "        assert_eq!((dennis.width, dennis.height), (3, 3));",
            "        for entry in VANILLA_PAINTING_VARIANTS {",
            "            assert_eq!(vanilla_painting_variant_for_resource_id(entry.resource_id), Some(entry));",
            "        }",
            "        assert_eq!(vanilla_painting_variant_for_resource_id(\"minecraft:missing\"), None);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_painting_variants_bridge() -> str:
    return "\n".join(
        (
            "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
            "// Owner: bbb-facts painting_variant_types; this bridge preserves the bbb-client-world API.",
            "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
            "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
            "",
            "pub use bbb_protocol::painting_variant_types::{",
            "    vanilla_painting_variant, vanilla_painting_variant_for_resource_id,",
            "    vanilla_painting_variant_resource_id, VanillaPaintingVariant,",
            "    VANILLA_PAINTING_VARIANTS,",
            "};",
            "",
        )
    )


def render_decorated_pot_patterns(
    entries: list[DecoratedPotPatternEntry],
    texture_dimensions: dict[str, tuple[int, int]],
) -> str:
    patterns = [entry for entry in entries if entry.constant_name != "BLANK"]
    blank_entries = [entry for entry in entries if entry.constant_name == "BLANK"]
    if len(blank_entries) != 1:
        raise ValueError(
            f"decorated-pot catalog must contain one BLANK entry: {blank_entries!r}"
        )

    def texture_constant(entry: DecoratedPotPatternEntry) -> str:
        if entry.constant_name == "BLANK":
            return "DECORATED_POT_SIDE_TEXTURE_REF"
        return f"DECORATED_POT_{entry.constant_name}_TEXTURE_REF"

    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 DecoratedPotPatterns.java keys, item map, bootstrap assets,",
        "// and the referenced official PNG dimensions.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "use bbb_render_types::EntityModelTextureRef;",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum DecoratedPotPattern {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in patterns),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaDecoratedPotPattern {",
        "    pattern: DecoratedPotPattern,",
        "    resource_id: &'static str,",
        "    item_resource_id: &'static str,",
        "    texture_ref: EntityModelTextureRef,",
        "}",
        "",
    ]
    for entry in entries:
        asset_path = entry.asset_id.removeprefix("minecraft:")
        size = texture_dimensions[entry.asset_id]
        lines.extend(
            (
                f"pub const {texture_constant(entry)}: EntityModelTextureRef =",
                "    EntityModelTextureRef {",
                f"        path: {rust_string(f'textures/entity/decorated_pot/{asset_path}.png')},",
                f"        size: [{size[0]}, {size[1]}],",
                "    };",
            )
        )
    lines.extend(("", "const VANILLA_DECORATED_POT_PATTERNS: &[VanillaDecoratedPotPattern] = &["))
    for entry in patterns:
        lines.extend(
            (
                "    VanillaDecoratedPotPattern {",
                f"        pattern: DecoratedPotPattern::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        item_resource_id: {rust_string(entry.item_resource_id)},",
                f"        texture_ref: {texture_constant(entry)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl DecoratedPotPattern {",
            "    pub fn for_sherd_item_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_DECORATED_POT_PATTERNS",
            "            .iter()",
            "            .find(|entry| entry.item_resource_id == resource_id)",
            "            .map(|entry| entry.pattern)",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_DECORATED_POT_PATTERNS[self as usize].resource_id",
            "    }",
            "",
            "    pub fn texture_ref(self) -> EntityModelTextureRef {",
            "        VANILLA_DECORATED_POT_PATTERNS[self as usize].texture_ref",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn decorated_pot_pattern_catalog_is_complete_and_unique() {",
            f"        assert_eq!(VANILLA_DECORATED_POT_PATTERNS.len(), {len(patterns)});",
            "        let mut resources = HashSet::new();",
            "        let mut items = HashSet::new();",
            "        let mut textures = HashSet::from([DECORATED_POT_SIDE_TEXTURE_REF.path]);",
            "        assert!(DECORATED_POT_SIDE_TEXTURE_REF.size.into_iter().all(|dimension| dimension > 0));",
            "        for (index, entry) in VANILLA_DECORATED_POT_PATTERNS.iter().enumerate() {",
            "            assert_eq!(entry.pattern as usize, index);",
            "            assert!(resources.insert(entry.resource_id));",
            "            assert!(items.insert(entry.item_resource_id));",
            "            assert!(textures.insert(entry.texture_ref.path));",
            "            assert!(entry.texture_ref.size.into_iter().all(|dimension| dimension > 0));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn decorated_pot_pattern_catalog_preserves_item_and_asset_mappings() {",
            '        assert_eq!(DecoratedPotPattern::for_sherd_item_resource_id("minecraft:angler_pottery_sherd"), Some(DecoratedPotPattern::Angler));',
            '        assert_eq!(DecoratedPotPattern::for_sherd_item_resource_id("minecraft:snort_pottery_sherd"), Some(DecoratedPotPattern::Snort));',
            '        assert_eq!(DecoratedPotPattern::for_sherd_item_resource_id("minecraft:brick"), None);',
            '        assert_eq!(DecoratedPotPattern::for_sherd_item_resource_id("minecraft:diamond"), None);',
            '        assert_eq!(DecoratedPotPattern::Angler.resource_id(), "minecraft:angler");',
            '        assert_eq!(DecoratedPotPattern::Snort.texture_ref().path, "textures/entity/decorated_pot/snort_pottery_pattern.png");',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def banner_pattern_variant(entry: BannerPatternEntry) -> str:
    resource_path = entry.resource_id.removeprefix("minecraft:")
    return rust_variant(resource_path.upper())


def banner_texture_constant(entry: BannerPatternEntry) -> str:
    asset_name = entry.asset_id.removeprefix("minecraft:").upper()
    if asset_name == "BASE":
        return "BANNER_PATTERN_BASE_TEXTURE_REF"
    return f"BANNER_{asset_name}_TEXTURE_REF"


def render_banner_pattern_types(entries: list[BannerPatternEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 BannerPatterns.java keys and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// Fixed vanilla 26.1 banner-pattern identity catalog.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum BannerPatternKind {",
        *(f"    {banner_pattern_variant(entry)}," for entry in entries),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaBannerPatternIdentity {",
        "    pattern: BannerPatternKind,",
        "    resource_id: &'static str,",
        "    translation_key: &'static str,",
        "}",
        "",
        "const VANILLA_BANNER_PATTERN_IDENTITIES: &[VanillaBannerPatternIdentity] = &[",
    ]
    for entry in entries:
        lines.extend(
            (
                "    VanillaBannerPatternIdentity {",
                f"        pattern: BannerPatternKind::{banner_pattern_variant(entry)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        translation_key: {rust_string(entry.translation_key)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl BannerPatternKind {",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_BANNER_PATTERN_IDENTITIES",
            "            .iter()",
            "            .find(|entry| entry.resource_id == resource_id)",
            "            .map(|entry| entry.pattern)",
            "    }",
            "",
            "    /// Resolves the vanilla 26.1 bootstrap ordinal used when no session registry is available.",
            "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
            "        let index = usize::try_from(registry_id).ok()?;",
            "        VANILLA_BANNER_PATTERN_IDENTITIES.get(index).map(|entry| entry.pattern)",
            "    }",
            "",
            "    pub const fn vanilla_registry_index(self) -> usize {",
            "        self as usize",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_BANNER_PATTERN_IDENTITIES[self.vanilla_registry_index()].resource_id",
            "    }",
            "",
            "    pub fn translation_key(self) -> &'static str {",
            "        VANILLA_BANNER_PATTERN_IDENTITIES[self.vanilla_registry_index()].translation_key",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn banner_pattern_identity_catalog_is_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_BANNER_PATTERN_IDENTITIES.len(), {len(entries)});",
            "        let mut resources = HashSet::new();",
            "        let mut translations = HashSet::new();",
            "        for (index, entry) in VANILLA_BANNER_PATTERN_IDENTITIES.iter().enumerate() {",
            "            assert_eq!(entry.pattern.vanilla_registry_index(), index);",
            "            assert!(resources.insert(entry.resource_id));",
            "            assert!(translations.insert(entry.translation_key));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "            assert!(entry.translation_key.starts_with(\"block.minecraft.banner.\"));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn banner_pattern_identity_catalog_preserves_vanilla_mappings() {",
            '        assert_eq!(BannerPatternKind::for_resource_id("minecraft:base"), Some(BannerPatternKind::Base));',
            '        assert_eq!(BannerPatternKind::for_resource_id("minecraft:diagonal_up_right"), Some(BannerPatternKind::DiagonalUpRight));',
            '        assert_eq!(BannerPatternKind::for_resource_id("minecraft:ominous"), None);',
            '        assert_eq!(BannerPatternKind::for_vanilla_registry_id(5), Some(BannerPatternKind::StripeBottom));',
            '        assert_eq!(BannerPatternKind::for_vanilla_registry_id(42), Some(BannerPatternKind::Guster));',
            '        assert_eq!(BannerPatternKind::for_vanilla_registry_id(-1), None);',
            '        assert_eq!(BannerPatternKind::for_vanilla_registry_id(43), None);',
            '        assert_eq!(BannerPatternKind::Gradient.resource_id(), "minecraft:gradient");',
            '        assert_eq!(BannerPatternKind::CurlyBorder.resource_id(), "minecraft:curly_border");',
            '        assert_eq!(BannerPatternKind::Guster.translation_key(), "block.minecraft.banner.guster");',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_banner_patterns(
    entries: list[BannerPatternEntry],
    texture_dimensions: dict[str, tuple[int, int]],
) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 BannerPatterns.java keys, bootstrap order, assets,",
        "// and the referenced official PNG dimensions.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "use super::catalog::EntityModelTextureRef;",
        "pub use bbb_render_types::BannerPatternKind;",
        "",
        "/// Fixed vanilla 26.1 banner-pattern presentation catalog.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaBannerPatternPresentation {",
        "    texture_ref: EntityModelTextureRef,",
        "}",
        "",
    ]
    for entry in entries:
        asset_path = entry.asset_id.removeprefix("minecraft:")
        size = texture_dimensions[entry.asset_id]
        constant = banner_texture_constant(entry)
        lines.extend(
            (
                f"pub(in crate::entity_models) const {constant}: EntityModelTextureRef =",
                "    EntityModelTextureRef {",
                f"        path: {rust_string(f'textures/entity/banner/{asset_path}.png')},",
                f"        size: [{size[0]}, {size[1]}],",
                "    };",
            )
        )
    lines.extend(
        (
            "",
            "const VANILLA_BANNER_PATTERN_PRESENTATIONS: &[VanillaBannerPatternPresentation] = &[",
        )
    )
    for entry in entries:
        lines.extend(
            (
                "    VanillaBannerPatternPresentation {",
                f"        texture_ref: {banner_texture_constant(entry)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub(crate) fn banner_pattern_texture_ref(",
            "    pattern: BannerPatternKind,",
            ") -> EntityModelTextureRef {",
            "    VANILLA_BANNER_PATTERN_PRESENTATIONS[pattern.vanilla_registry_index()].texture_ref",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn banner_pattern_catalog_is_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_BANNER_PATTERN_PRESENTATIONS.len(), {len(entries)});",
            "        let mut textures = HashSet::new();",
            "        for entry in VANILLA_BANNER_PATTERN_PRESENTATIONS {",
            "            assert!(textures.insert(entry.texture_ref.path));",
            "            assert!(entry.texture_ref.size.into_iter().all(|dimension| dimension > 0));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn banner_pattern_catalog_preserves_asset_mappings() {",
            '        assert_eq!(banner_pattern_texture_ref(BannerPatternKind::SmallStripes).path, "textures/entity/banner/small_stripes.png");',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_villager_professions(entries: list[BootstrapEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 VillagerProfession.java keys and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// Fixed vanilla 26.1 villager-profession registry catalog.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum VillagerModelProfession {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaVillagerProfession {",
        "    profession: VillagerModelProfession,",
        "    resource_id: &'static str,",
        "}",
        "",
        "const VANILLA_VILLAGER_PROFESSIONS: &[VanillaVillagerProfession] = &[",
    ]
    for entry in entries:
        lines.extend(
            (
                "    VanillaVillagerProfession {",
                f"        profession: VillagerModelProfession::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl VillagerModelProfession {",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_VILLAGER_PROFESSIONS",
            "            .iter()",
            "            .find(|entry| entry.resource_id == resource_id)",
            "            .map(|entry| entry.profession)",
            "    }",
            "",
            "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
            "        VANILLA_VILLAGER_PROFESSIONS",
            "            .get(usize::try_from(registry_id).ok()?)",
            "            .map(|entry| entry.profession)",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_VILLAGER_PROFESSIONS[self as usize].resource_id",
            "    }",
            "",
            "    pub const fn vanilla_registry_index(self) -> usize {",
            "        self as usize",
            "    }",
            "",
            "    pub fn texture_index(self) -> Option<usize> {",
            "        (self as usize).checked_sub(1)",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn villager_profession_catalog_is_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_VILLAGER_PROFESSIONS.len(), {len(entries)});",
            "        let mut resources = HashSet::new();",
            "        for (registry_id, entry) in VANILLA_VILLAGER_PROFESSIONS.iter().enumerate() {",
            "            assert_eq!(entry.profession as usize, registry_id);",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn villager_profession_catalog_preserves_registry_and_texture_ordinals() {",
            '        assert_eq!(VillagerModelProfession::for_vanilla_registry_id(0), Some(VillagerModelProfession::None));',
            '        assert_eq!(VillagerModelProfession::for_vanilla_registry_id(14), Some(VillagerModelProfession::Weaponsmith));',
            '        assert_eq!(VillagerModelProfession::for_vanilla_registry_id(-1), None);',
            '        assert_eq!(VillagerModelProfession::for_vanilla_registry_id(15), None);',
            '        assert_eq!(VillagerModelProfession::for_resource_id("minecraft:mason"), Some(VillagerModelProfession::Mason));',
            '        assert_eq!(VillagerModelProfession::for_resource_id("minecraft:beekeeper"), None);',
            '        assert_eq!(VillagerModelProfession::Mason.vanilla_registry_index(), 10);',
            '        assert_eq!(VillagerModelProfession::None.texture_index(), None);',
            '        assert_eq!(VillagerModelProfession::Armorer.texture_index(), Some(0));',
            '        assert_eq!(VillagerModelProfession::Weaponsmith.texture_index(), Some(13));',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_villager_types(entries: list[BootstrapEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 VillagerType.java keys and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// Fixed vanilla 26.1 villager-type registry catalog.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum VillagerModelType {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaVillagerType {",
        "    villager_type: VillagerModelType,",
        "    resource_id: &'static str,",
        "}",
        "",
        "const VANILLA_VILLAGER_TYPES: &[VanillaVillagerType] = &[",
    ]
    for entry in entries:
        lines.extend(
            (
                "    VanillaVillagerType {",
                f"        villager_type: VillagerModelType::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl VillagerModelType {",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_VILLAGER_TYPES",
            "            .iter()",
            "            .find(|entry| entry.resource_id == resource_id)",
            "            .map(|entry| entry.villager_type)",
            "    }",
            "",
            "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
            "        VANILLA_VILLAGER_TYPES",
            "            .get(usize::try_from(registry_id).ok()?)",
            "            .map(|entry| entry.villager_type)",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_VILLAGER_TYPES[self as usize].resource_id",
            "    }",
            "",
            "    pub const fn vanilla_registry_index(self) -> usize {",
            "        self as usize",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn villager_type_catalog_is_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_VILLAGER_TYPES.len(), {len(entries)});",
            "        let mut resources = HashSet::new();",
            "        for (registry_id, entry) in VANILLA_VILLAGER_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.villager_type as usize, registry_id);",
            "            assert!(resources.insert(entry.resource_id));",
            "            assert_eq!(entry.villager_type.vanilla_registry_index(), registry_id);",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn villager_type_catalog_preserves_registry_and_resource_order() {",
            '        assert_eq!(VillagerModelType::for_vanilla_registry_id(0), Some(VillagerModelType::Desert));',
            '        assert_eq!(VillagerModelType::for_vanilla_registry_id(6), Some(VillagerModelType::Taiga));',
            '        assert_eq!(VillagerModelType::for_vanilla_registry_id(-1), None);',
            '        assert_eq!(VillagerModelType::for_vanilla_registry_id(7), None);',
            '        assert_eq!(VillagerModelType::for_resource_id("minecraft:plains"), Some(VillagerModelType::Plains));',
            '        assert_eq!(VillagerModelType::for_resource_id("minecraft:coast"), None);',
            '        assert_eq!(VillagerModelType::Snow.resource_id(), "minecraft:snow");',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_cat_variants(
    entries: list[CatVariantEntry],
    texture_dimensions: dict[str, tuple[int, int]],
) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 CatVariants.java keys, bootstrap order, asset ids,",
        "// and the referenced official PNG dimensions.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "use super::EntityModelTextureRef;",
        "",
        "/// Known vanilla 26.1 cat-variant identities and built-in presentation assets.",
        "///",
        "/// Session holder ids must be resolved through the synchronized registry before this lookup;",
        "/// that registry can reorder entries and carries the adult/baby client-asset payload.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum CatModelVariant {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaCatVariant {",
        "    variant: CatModelVariant,",
        "    resource_id: &'static str,",
        "    texture_start: usize,",
        "}",
        "",
        "const VANILLA_CAT_VARIANTS: &[VanillaCatVariant] = &[",
    ]
    for registry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    VanillaCatVariant {",
                f"        variant: CatModelVariant::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        texture_start: {registry_id * 2},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            f"pub const VANILLA_CAT_VARIANT_TEXTURE_REFS: [EntityModelTextureRef; {len(entries) * 2}] = [",
        )
    )
    for entry in entries:
        for asset_id in (entry.adult_asset_id, entry.baby_asset_id):
            if not asset_id.startswith("minecraft:"):
                raise ValueError(f"cat asset must use the minecraft namespace: {asset_id}")
            path = asset_id.removeprefix("minecraft:")
            size = texture_dimensions[asset_id]
            texture_path = rust_string(f"textures/{path}.png")
            lines.extend(
                (
                    "    EntityModelTextureRef {",
                    f"        path: {texture_path},",
                    f"        size: [{size[0]}, {size[1]}],",
                    "    },",
                )
            )
    lines.extend(
        (
            "];",
            "",
            "impl CatModelVariant {",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_CAT_VARIANTS",
            "            .iter()",
            "            .find(|entry| entry.resource_id == resource_id)",
            "            .map(|entry| entry.variant)",
            "    }",
            "",
            "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
            "        VANILLA_CAT_VARIANTS",
            "            .get(usize::try_from(registry_id).ok()?)",
            "            .map(|entry| entry.variant)",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_CAT_VARIANTS[self as usize].resource_id",
            "    }",
            "",
            "    pub const fn builtin_texture_ref(",
            "        self,",
            "        baby: bool,",
            "    ) -> EntityModelTextureRef {",
            "        let age_offset = if baby { 1 } else { 0 };",
            "        VANILLA_CAT_VARIANT_TEXTURE_REFS",
            "            [VANILLA_CAT_VARIANTS[self as usize].texture_start + age_offset]",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn cat_variant_catalog_is_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_CAT_VARIANTS.len(), {len(entries)});",
            "        let mut resources = HashSet::new();",
            "        let mut textures = HashSet::new();",
            "        for (registry_id, entry) in VANILLA_CAT_VARIANTS.iter().enumerate() {",
            "            assert_eq!(entry.variant as usize, registry_id);",
            "            assert_eq!(entry.texture_start, registry_id * 2);",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "        for texture in VANILLA_CAT_VARIANT_TEXTURE_REFS {",
            "            assert!(textures.insert(texture.path));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn cat_variant_catalog_preserves_registry_and_resource_order() {",
            '        assert_eq!(CatModelVariant::for_vanilla_registry_id(0), Some(CatModelVariant::Tabby));',
            '        assert_eq!(CatModelVariant::for_vanilla_registry_id(10), Some(CatModelVariant::AllBlack));',
            '        assert_eq!(CatModelVariant::for_vanilla_registry_id(-1), None);',
            '        assert_eq!(CatModelVariant::for_vanilla_registry_id(11), None);',
            '        assert_eq!(CatModelVariant::for_resource_id("minecraft:jellie"), Some(CatModelVariant::Jellie));',
            '        assert_eq!(CatModelVariant::for_resource_id("minecraft:lynx"), None);',
            '        assert_eq!(CatModelVariant::Black.resource_id(), "minecraft:black");',
            '        assert_eq!(CatModelVariant::Tabby.builtin_texture_ref(false).path, "textures/entity/cat/cat_tabby.png");',
            '        assert_eq!(CatModelVariant::Jellie.builtin_texture_ref(true).path, "textures/entity/cat/cat_jellie_baby.png");',
            f"        assert_eq!(CatModelVariant::AllBlack.builtin_texture_ref(true).size, {list(texture_dimensions[entries[-1].baby_asset_id])!r});",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_wolf_variants(
    entries: list[WolfVariantEntry],
    texture_dimensions: dict[str, tuple[int, int]],
) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 WolfVariants.java keys, bootstrap order, asset file names,",
        "// and the referenced official PNG dimensions.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "use super::EntityModelTextureRef;",
        "",
        "/// Known vanilla 26.1 wolf-variant identities and built-in presentation assets.",
        "///",
        "/// Session holder ids must be resolved through the synchronized registry before this lookup;",
        "/// that registry can reorder entries and carries adult/baby wild/tame/angry assets.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum WolfModelVariant {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaWolfVariant {",
        "    variant: WolfModelVariant,",
        "    resource_id: &'static str,",
        "    texture_start: usize,",
        "}",
        "",
        "const VANILLA_WOLF_VARIANTS: &[VanillaWolfVariant] = &[",
    ]
    for registry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    VanillaWolfVariant {",
                f"        variant: WolfModelVariant::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        texture_start: {registry_id * 6},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            f"pub const VANILLA_WOLF_VARIANT_TEXTURE_REFS: [EntityModelTextureRef; {len(entries) * 6}] = [",
        )
    )
    for entry in entries:
        for asset_id in entry.asset_ids:
            if not asset_id.startswith("minecraft:"):
                raise ValueError(f"wolf asset must use the minecraft namespace: {asset_id}")
            path = asset_id.removeprefix("minecraft:")
            size = texture_dimensions[asset_id]
            texture_path = rust_string(f"textures/{path}.png")
            lines.extend(
                (
                    "    EntityModelTextureRef {",
                    f"        path: {texture_path},",
                    f"        size: [{size[0]}, {size[1]}],",
                    "    },",
                )
            )
    lines.extend(
        (
            "];",
            "",
            "impl WolfModelVariant {",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_WOLF_VARIANTS",
            "            .iter()",
            "            .find(|entry| entry.resource_id == resource_id)",
            "            .map(|entry| entry.variant)",
            "    }",
            "",
            "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
            "        VANILLA_WOLF_VARIANTS",
            "            .get(usize::try_from(registry_id).ok()?)",
            "            .map(|entry| entry.variant)",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_WOLF_VARIANTS[self as usize].resource_id",
            "    }",
            "",
            "    pub const fn builtin_texture_ref(",
            "        self,",
            "        baby: bool,",
            "        tame: bool,",
            "        angry: bool,",
            "    ) -> EntityModelTextureRef {",
            "        let age_offset = if baby { 3 } else { 0 };",
            "        let state_offset = if tame { 1 } else if angry { 2 } else { 0 };",
            "        VANILLA_WOLF_VARIANT_TEXTURE_REFS",
            "            [VANILLA_WOLF_VARIANTS[self as usize].texture_start + age_offset + state_offset]",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn wolf_variant_catalog_is_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_WOLF_VARIANTS.len(), {len(entries)});",
            "        let mut resources = HashSet::new();",
            "        let mut textures = HashSet::new();",
            "        for (registry_id, entry) in VANILLA_WOLF_VARIANTS.iter().enumerate() {",
            "            assert_eq!(entry.variant as usize, registry_id);",
            "            assert_eq!(entry.texture_start, registry_id * 6);",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "        for texture in VANILLA_WOLF_VARIANT_TEXTURE_REFS {",
            "            assert!(textures.insert(texture.path));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn wolf_variant_catalog_preserves_registry_and_resource_order() {",
            '        assert_eq!(WolfModelVariant::for_vanilla_registry_id(0), Some(WolfModelVariant::Pale));',
            '        assert_eq!(WolfModelVariant::for_vanilla_registry_id(8), Some(WolfModelVariant::Striped));',
            '        assert_eq!(WolfModelVariant::for_vanilla_registry_id(-1), None);',
            '        assert_eq!(WolfModelVariant::for_vanilla_registry_id(9), None);',
            '        assert_eq!(WolfModelVariant::for_resource_id("minecraft:chestnut"), Some(WolfModelVariant::Chestnut));',
            '        assert_eq!(WolfModelVariant::for_resource_id("minecraft:blue"), None);',
            '        assert_eq!(WolfModelVariant::Pale.resource_id(), "minecraft:pale");',
            '        assert_eq!(WolfModelVariant::Pale.builtin_texture_ref(false, false, false).path, "textures/entity/wolf/wolf.png");',
            '        assert_eq!(WolfModelVariant::Spotted.builtin_texture_ref(false, true, true).path, "textures/entity/wolf/wolf_spotted_tame.png");',
            '        assert_eq!(WolfModelVariant::Striped.builtin_texture_ref(true, false, true).path, "textures/entity/wolf/wolf_striped_angry_baby.png");',
            f"        assert_eq!(WolfModelVariant::Striped.builtin_texture_ref(true, false, true).size, {list(texture_dimensions[entries[-1].asset_ids[-1]])!r});",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_temperature_animal_variants(
    catalogs: list[tuple[str, list[BootstrapEntry]]],
    farm_catalogs: list[tuple[str, list[FarmAnimalVariantEntry]]],
    frog_entries: list[FrogVariantEntry],
    texture_dimensions: dict[str, tuple[int, int]],
) -> str:
    expected_catalogs = (
        ("VANILLA_CHICKEN_VARIANT_KEYS", "ChickenModelVariant"),
        ("VANILLA_COW_VARIANT_KEYS", "CowModelVariant"),
        ("VANILLA_PIG_VARIANT_KEYS", "PigModelVariant"),
        ("VANILLA_FROG_VARIANT_KEYS", "FrogModelVariant"),
    )
    catalog_map = dict(catalogs)
    reference = catalog_map[expected_catalogs[0][0]]
    reference_facts = [
        (entry.constant_name, entry.resource_id) for entry in reference
    ]
    for catalog_name, _ in expected_catalogs[1:]:
        facts = [
            (entry.constant_name, entry.resource_id)
            for entry in catalog_map[catalog_name]
        ]
        if facts != reference_facts:
            raise ValueError(
                f"{catalog_name} no longer matches the shared temperature variant order: "
                f"{facts!r} != {reference_facts!r}"
            )
    frog_facts = [
        (entry.constant_name, entry.resource_id) for entry in frog_entries
    ]
    if frog_facts != reference_facts:
        raise ValueError(
            "frog presentation entries no longer match the shared identity order: "
            f"{frog_facts!r} != {reference_facts!r}"
        )
    expected_farm_catalogs = (
        ("Chicken", "ChickenModelVariant", "ChickenVariantModelType"),
        ("Cow", "CowModelVariant", "CowVariantModelType"),
        ("Pig", "PigModelVariant", "PigVariantModelType"),
    )
    farm_catalog_map = dict(farm_catalogs)
    for family, _, _ in expected_farm_catalogs:
        facts = [
            (entry.constant_name, entry.resource_id)
            for entry in farm_catalog_map[family]
        ]
        if facts != reference_facts:
            raise ValueError(
                f"{family} presentation entries no longer match the shared identity order: "
                f"{facts!r} != {reference_facts!r}"
            )

    def texture_path(asset_id: str) -> str:
        namespace, separator, path = asset_id.partition(":")
        if separator != ":" or namespace != "minecraft" or not path:
            raise ValueError(
                f"farm-animal asset must use the minecraft namespace: {asset_id}"
            )
        return f"textures/{path}.png"

    def model_key(family: str, asset_id: str) -> str:
        prefix = f"minecraft:entity/{family.lower()}/"
        if not asset_id.startswith(prefix):
            raise ValueError(
                f"{family} asset does not use the expected directory: {asset_id}"
            )
        key = asset_id.removeprefix(prefix)
        if not key or "/" in key:
            raise ValueError(f"{family} asset does not yield one model key: {asset_id}")
        return key

    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Chicken/Cow/Pig/FrogVariants.java bootstrap presentation",
        "// payloads and the referenced official PNG dimensions.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "use super::EntityModelTextureRef;",
        "",
        "const VANILLA_TEMPERATURE_VARIANT_RESOURCES: &[&str] = &[",
        *(f"    {rust_string(entry.resource_id)}," for entry in reference),
        "];",
        "",
        "macro_rules! define_temperature_model_variant {",
        "    ($name:ident) => {",
        "        #[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "        #[repr(u8)]",
        "        pub enum $name {",
        *(f"            {rust_variant(entry.constant_name)}," for entry in reference),
        "        }",
        "",
        "        impl $name {",
        "            pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
        "                match resource_id {",
        *(
            f"                    {rust_string(entry.resource_id)} => Some(Self::{rust_variant(entry.constant_name)}),"
            for entry in reference
        ),
        "                    _ => None,",
        "                }",
        "            }",
        "",
        "            pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
        "                match registry_id {",
        *(
            f"                    {registry_id} => Some(Self::{rust_variant(entry.constant_name)}),"
            for registry_id, entry in enumerate(reference)
        ),
        "                    _ => None,",
        "                }",
        "            }",
        "",
        "            pub fn resource_id(self) -> &'static str {",
        "                VANILLA_TEMPERATURE_VARIANT_RESOURCES[self as usize]",
        "            }",
        "        }",
        "    };",
        "}",
        "",
    ]
    for _, type_name in expected_catalogs:
        lines.append(f"define_temperature_model_variant!({type_name});")

    for family, variant_type, model_type in expected_farm_catalogs:
        entries = farm_catalog_map[family]
        model_types = list(dict.fromkeys(entry.model_type for entry in entries))
        lines.extend(
            (
                "",
                "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
                f"pub enum {model_type} {{",
                *(f"    {rust_variant(value)}," for value in model_types),
                "}",
                "",
                "#[derive(Debug, Clone, Copy)]",
                f"struct Builtin{family}VariantPresentation {{",
                f"    model_type: {model_type},",
                "    adult_model_key: &'static str,",
                "    adult_texture: EntityModelTextureRef,",
                "    baby_model_key: &'static str,",
                "    baby_texture: EntityModelTextureRef,",
                "}",
                "",
                f"const VANILLA_{family.upper()}_VARIANT_PRESENTATIONS: [Builtin{family}VariantPresentation; {len(entries)}] = [",
            )
        )
        for entry in entries:
            adult_size = texture_dimensions[entry.adult_asset_id]
            baby_size = texture_dimensions[entry.baby_asset_id]
            lines.extend(
                (
                    f"    Builtin{family}VariantPresentation {{",
                    f"        model_type: {model_type}::{rust_variant(entry.model_type)},",
                    f"        adult_model_key: {rust_string(model_key(family, entry.adult_asset_id))},",
                    "        adult_texture: EntityModelTextureRef {",
                    f"            path: {rust_string(texture_path(entry.adult_asset_id))},",
                    f"            size: [{adult_size[0]}, {adult_size[1]}],",
                    "        },",
                    f"        baby_model_key: {rust_string(model_key(family, entry.baby_asset_id))},",
                    "        baby_texture: EntityModelTextureRef {",
                    f"            path: {rust_string(texture_path(entry.baby_asset_id))},",
                    f"            size: [{baby_size[0]}, {baby_size[1]}],",
                    "        },",
                    "    },",
                )
            )
        lines.extend(
            (
                "];",
                "",
                f"pub const VANILLA_{family.upper()}_VARIANT_TEXTURE_REFS: [EntityModelTextureRef; {len(entries) * 2}] = [",
            )
        )
        for index in range(len(entries)):
            lines.extend(
                (
                    f"    VANILLA_{family.upper()}_VARIANT_PRESENTATIONS[{index}].adult_texture,",
                    f"    VANILLA_{family.upper()}_VARIANT_PRESENTATIONS[{index}].baby_texture,",
                )
            )
        lines.extend(
            (
                "];",
                "",
                f"impl {variant_type} {{",
                f"    const fn builtin_presentation(self) -> Builtin{family}VariantPresentation {{",
                f"        VANILLA_{family.upper()}_VARIANT_PRESENTATIONS[self as usize]",
                "    }",
                "",
                f"    pub const fn builtin_model_type(self) -> {model_type} {{",
                "        self.builtin_presentation().model_type",
                "    }",
                "",
                "    pub const fn builtin_model_key(self, baby: bool) -> &'static str {",
                "        let presentation = self.builtin_presentation();",
                "        if baby {",
                "            presentation.baby_model_key",
                "        } else {",
                "            presentation.adult_model_key",
                "        }",
                "    }",
                "",
                "    pub const fn builtin_texture_ref(",
                "        self,",
                "        baby: bool,",
                "    ) -> EntityModelTextureRef {",
                "        let presentation = self.builtin_presentation();",
                "        if baby {",
                "            presentation.baby_texture",
                "        } else {",
                "            presentation.adult_texture",
                "        }",
                "    }",
                "}",
            )
        )

    lines.extend(
        (
            "",
            f"pub const VANILLA_FROG_VARIANT_TEXTURE_REFS: [EntityModelTextureRef; {len(frog_entries)}] = [",
        )
    )
    for entry in frog_entries:
        if not entry.asset_id.startswith("minecraft:"):
            raise ValueError(
                f"frog asset must use the minecraft namespace: {entry.asset_id}"
            )
        path = entry.asset_id.removeprefix("minecraft:")
        size = texture_dimensions[entry.asset_id]
        frog_texture_path = rust_string(f"textures/{path}.png")
        lines.extend(
            (
                "    EntityModelTextureRef {",
                f"        path: {frog_texture_path},",
                f"        size: [{size[0]}, {size[1]}],",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl FrogModelVariant {",
            "    pub const fn builtin_texture_ref(self) -> EntityModelTextureRef {",
            "        VANILLA_FROG_VARIANT_TEXTURE_REFS[self as usize]",
            "    }",
            "}",
        )
    )
    lines.extend(
        (
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn temperature_animal_variant_catalogs_are_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_TEMPERATURE_VARIANT_RESOURCES.len(), {len(reference)});",
            "        let resources: HashSet<_> = VANILLA_TEMPERATURE_VARIANT_RESOURCES.iter().copied().collect();",
            "        assert_eq!(resources.len(), VANILLA_TEMPERATURE_VARIANT_RESOURCES.len());",
            "        let mut presentation_textures: HashSet<_> = VANILLA_FROG_VARIANT_TEXTURE_REFS.iter().map(|texture| texture.path).collect();",
            "        assert_eq!(presentation_textures.len(), VANILLA_FROG_VARIANT_TEXTURE_REFS.len());",
        )
    )
    for _, type_name in expected_catalogs:
        lines.extend(
            (
                f"        assert_eq!({type_name}::for_vanilla_registry_id(0), Some({type_name}::Temperate));",
                f"        assert_eq!({type_name}::for_vanilla_registry_id(2), Some({type_name}::Cold));",
                f"        assert_eq!({type_name}::for_vanilla_registry_id(-1), None);",
                f"        assert_eq!({type_name}::for_vanilla_registry_id(3), None);",
                f"        assert_eq!({type_name}::for_resource_id(\"minecraft:warm\"), Some({type_name}::Warm));",
                f"        assert_eq!({type_name}::for_resource_id(\"minecraft:humid\"), None);",
                f"        assert_eq!({type_name}::Cold.resource_id(), \"minecraft:cold\");",
            )
        )
    for family, variant_type, model_type in expected_farm_catalogs:
        entries = farm_catalog_map[family]
        lines.extend(
            (
                f"        assert_eq!(VANILLA_{family.upper()}_VARIANT_PRESENTATIONS.len(), VANILLA_TEMPERATURE_VARIANT_RESOURCES.len());",
                f"        assert_eq!(VANILLA_{family.upper()}_VARIANT_TEXTURE_REFS.len(), VANILLA_TEMPERATURE_VARIANT_RESOURCES.len() * 2);",
                f"        for presentation in VANILLA_{family.upper()}_VARIANT_PRESENTATIONS {{",
                "            assert!(presentation_textures.insert(presentation.adult_texture.path));",
                "            assert!(presentation_textures.insert(presentation.baby_texture.path));",
                "        }",
            )
        )
        for entry in entries:
            variant = rust_variant(entry.constant_name)
            adult_size = texture_dimensions[entry.adult_asset_id]
            baby_size = texture_dimensions[entry.baby_asset_id]
            lines.extend(
                (
                    f"        assert_eq!({variant_type}::{variant}.builtin_model_type(), {model_type}::{rust_variant(entry.model_type)});",
                    f"        assert_eq!({variant_type}::{variant}.builtin_model_key(false), {rust_string(model_key(family, entry.adult_asset_id))});",
                    f"        assert_eq!({variant_type}::{variant}.builtin_model_key(true), {rust_string(model_key(family, entry.baby_asset_id))});",
                    f"        assert_eq!({variant_type}::{variant}.builtin_texture_ref(false), EntityModelTextureRef {{ path: {rust_string(texture_path(entry.adult_asset_id))}, size: [{adult_size[0]}, {adult_size[1]}] }});",
                    f"        assert_eq!({variant_type}::{variant}.builtin_texture_ref(true), EntityModelTextureRef {{ path: {rust_string(texture_path(entry.baby_asset_id))}, size: [{baby_size[0]}, {baby_size[1]}] }});",
                )
            )
    lines.extend(
        (
            '        assert_eq!(FrogModelVariant::Temperate.builtin_texture_ref().path, "textures/entity/frog/frog_temperate.png");',
            '        assert_eq!(FrogModelVariant::Cold.builtin_texture_ref().path, "textures/entity/frog/frog_cold.png");',
            f"        assert_eq!(FrogModelVariant::Warm.builtin_texture_ref().size, {list(texture_dimensions[frog_entries[1].asset_id])!r});",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_zombie_nautilus_variants(
    entries: list[ModelAndTextureVariantEntry],
    texture_dimensions: dict[str, tuple[int, int]],
) -> str:
    asset_prefix = "minecraft:entity/nautilus/"

    def model_key(asset_id: str) -> str:
        if not asset_id.startswith(asset_prefix):
            raise ValueError(
                f"zombie-nautilus asset does not use the expected directory: {asset_id}"
            )
        key = asset_id.removeprefix(asset_prefix)
        if not key or "/" in key:
            raise ValueError(
                f"zombie-nautilus asset does not yield one model key: {asset_id}"
            )
        return key

    def texture_path(asset_id: str) -> str:
        return f"textures/{asset_id.removeprefix('minecraft:')}.png"

    model_types = list(dict.fromkeys(entry.model_type for entry in entries))
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 ZombieNautilusVariants.java bootstrap presentation payloads",
        "// and the referenced official PNG dimensions.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "use super::EntityModelTextureRef;",
        "",
        "/// Known vanilla 26.1 zombie-nautilus variant identities and built-in presentation.",
        "///",
        "/// Session holder ids must be resolved through the synchronized registry before this lookup;",
        "/// synchronized replacement model/asset payloads belong to session RegistryAccess.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum ZombieNautilusModelVariant {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ZombieNautilusVariantModelType {",
        *(f"    {rust_variant(value)}," for value in model_types),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaZombieNautilusVariant {",
        "    variant: ZombieNautilusModelVariant,",
        "    resource_id: &'static str,",
        "    model_type: ZombieNautilusVariantModelType,",
        "    model_key: &'static str,",
        "    texture: EntityModelTextureRef,",
        "}",
        "",
        f"const VANILLA_ZOMBIE_NAUTILUS_VARIANTS: [VanillaZombieNautilusVariant; {len(entries)}] = [",
    ]
    for entry in entries:
        size = texture_dimensions[entry.asset_id]
        lines.extend(
            (
                "    VanillaZombieNautilusVariant {",
                f"        variant: ZombieNautilusModelVariant::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        model_type: ZombieNautilusVariantModelType::{rust_variant(entry.model_type)},",
                f"        model_key: {rust_string(model_key(entry.asset_id))},",
                "        texture: EntityModelTextureRef {",
                f"            path: {rust_string(texture_path(entry.asset_id))},",
                f"            size: [{size[0]}, {size[1]}],",
                "        },",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            f"pub const VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS: [EntityModelTextureRef; {len(entries)}] = [",
            *(
                f"    VANILLA_ZOMBIE_NAUTILUS_VARIANTS[{index}].texture,"
                for index in range(len(entries))
            ),
            "];",
            "",
            "impl ZombieNautilusModelVariant {",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_ZOMBIE_NAUTILUS_VARIANTS",
            "            .iter()",
            "            .find(|entry| entry.resource_id == resource_id)",
            "            .map(|entry| entry.variant)",
            "    }",
            "",
            "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
            "        VANILLA_ZOMBIE_NAUTILUS_VARIANTS",
            "            .get(usize::try_from(registry_id).ok()?)",
            "            .map(|entry| entry.variant)",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_ZOMBIE_NAUTILUS_VARIANTS[self as usize].resource_id",
            "    }",
            "",
            "    pub const fn builtin_model_type(",
            "        self,",
            "    ) -> ZombieNautilusVariantModelType {",
            "        VANILLA_ZOMBIE_NAUTILUS_VARIANTS[self as usize].model_type",
            "    }",
            "",
            "    pub const fn builtin_model_key(self) -> &'static str {",
            "        VANILLA_ZOMBIE_NAUTILUS_VARIANTS[self as usize].model_key",
            "    }",
            "",
            "    pub const fn builtin_texture_ref(",
            "        self,",
            "    ) -> EntityModelTextureRef {",
            "        VANILLA_ZOMBIE_NAUTILUS_VARIANTS[self as usize].texture",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn zombie_nautilus_variant_catalog_is_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_ZOMBIE_NAUTILUS_VARIANTS.len(), {len(entries)});",
            "        let resources: HashSet<_> = VANILLA_ZOMBIE_NAUTILUS_VARIANTS",
            "            .iter()",
            "            .map(|entry| entry.resource_id)",
            "            .collect();",
            "        assert_eq!(resources.len(), VANILLA_ZOMBIE_NAUTILUS_VARIANTS.len());",
            "        let textures: HashSet<_> = VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS.iter().map(|texture| texture.path).collect();",
            "        assert_eq!(textures.len(), VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS.len());",
            "        assert_eq!(ZombieNautilusModelVariant::for_vanilla_registry_id(0), Some(ZombieNautilusModelVariant::Temperate));",
            "        assert_eq!(ZombieNautilusModelVariant::for_vanilla_registry_id(1), Some(ZombieNautilusModelVariant::Warm));",
            "        assert_eq!(ZombieNautilusModelVariant::for_vanilla_registry_id(-1), None);",
            "        assert_eq!(ZombieNautilusModelVariant::for_vanilla_registry_id(2), None);",
            "        assert_eq!(ZombieNautilusModelVariant::for_resource_id(\"minecraft:warm\"), Some(ZombieNautilusModelVariant::Warm));",
            "        assert_eq!(ZombieNautilusModelVariant::for_resource_id(\"minecraft:cold\"), None);",
            "        assert_eq!(ZombieNautilusModelVariant::Temperate.resource_id(), \"minecraft:temperate\");",
        )
    )
    for entry in entries:
        variant = rust_variant(entry.constant_name)
        size = texture_dimensions[entry.asset_id]
        lines.extend(
            (
                f"        assert_eq!(ZombieNautilusModelVariant::{variant}.builtin_model_type(), ZombieNautilusVariantModelType::{rust_variant(entry.model_type)});",
                f"        assert_eq!(ZombieNautilusModelVariant::{variant}.builtin_model_key(), {rust_string(model_key(entry.asset_id))});",
                f"        assert_eq!(ZombieNautilusModelVariant::{variant}.builtin_texture_ref(), EntityModelTextureRef {{ path: {rust_string(texture_path(entry.asset_id))}, size: [{size[0]}, {size[1]}] }});",
            )
        )
    lines.extend(
        (
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_animal_sound_variants(
    catalogs: list[tuple[str, list[BootstrapEntry]]],
    sound_catalogs: dict[str, list[SoundVariantSoundEntry]],
) -> str:
    expected_catalogs = tuple(
        (family, catalog_name, identity_type)
        for (
            family,
            catalog_name,
            _,
            identity_type,
            _,
            _,
            _,
            _,
        ) in ANIMAL_SOUND_VARIANT_CONFIGS
    )
    catalog_map = dict(catalogs)
    ordered_sound_catalogs = {}
    for family, catalog_name, _ in expected_catalogs:
        sounds_by_constant = {
            entry.constant_name: entry for entry in sound_catalogs[family]
        }
        if len(sounds_by_constant) != len(sound_catalogs[family]):
            raise ValueError(
                f"{family} sound-set presentation contains duplicate constants"
            )
        ordered_sounds = []
        for identity in catalog_map[catalog_name]:
            sound = sounds_by_constant.pop(identity.constant_name, None)
            if sound is None or sound.resource_id != identity.resource_id:
                raise ValueError(
                    f"{family} sound-set presentation does not match bootstrap identity "
                    f"{identity.constant_name}/{identity.resource_id}"
                )
            ordered_sounds.append(sound)
        if sounds_by_constant:
            raise ValueError(
                f"{family} sound-set presentation has unregistered constants: "
                f"{sorted(sounds_by_constant)}"
            )
        ordered_sound_catalogs[family] = ordered_sounds
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Cat/Chicken/Cow/Pig/WolfSoundVariants.java bootstrap order",
        "// and SoundEvents.java built-in adult/baby hurt/death sound payloads.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// Defines one known vanilla sound-variant identity and built-in consumed-sound catalog.",
        "///",
        "/// Session holder ids must be resolved through the corresponding synchronized registry",
        "/// before this fixed-order fallback; synchronized SoundEvent payloads belong to RegistryAccess.",
        "macro_rules! define_sound_variant_catalog {",
        "    ($name:ident, $entries:ident, [$(($variant:ident, $resource:literal, $adult_hurt:literal, $baby_hurt:literal, $adult_death:literal, $baby_death:literal)),+ $(,)?]) => {",
        "        #[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "        #[repr(u8)]",
        "        pub enum $name {",
        "            $($variant),+",
        "        }",
        "",
        "        const $entries: &[(&str, $name, &str, &str, &str, &str)] = &[",
        "            $(($resource, $name::$variant, $adult_hurt, $baby_hurt, $adult_death, $baby_death)),+",
        "        ];",
        "",
        "        impl $name {",
        "            pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
        "                $entries",
        "                    .iter()",
        "                    .find(|(resource, _, _, _, _, _)| *resource == resource_id)",
        "                    .map(|(_, variant, _, _, _, _)| *variant)",
        "            }",
        "",
        "            pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
        "                $entries",
        "                    .get(usize::try_from(registry_id).ok()?)",
        "                    .map(|(_, variant, _, _, _, _)| *variant)",
        "            }",
        "",
        "            pub fn resource_id(self) -> &'static str {",
        "                $entries[self as usize].0",
        "            }",
        "",
        "            pub fn builtin_variants() -> impl ExactSizeIterator<Item = (i32, Self)> {",
        "                $entries.iter().enumerate().map(|(registry_id, (_, variant, _, _, _, _))| (registry_id as i32, *variant))",
        "            }",
        "",
        "            pub const fn builtin_hurt_sound_event(self, baby: bool) -> &'static str {",
        "                let entry = $entries[self as usize];",
        "                if baby { entry.3 } else { entry.2 }",
        "            }",
        "",
        "            pub const fn builtin_death_sound_event(self, baby: bool) -> &'static str {",
        "                let entry = $entries[self as usize];",
        "                if baby { entry.5 } else { entry.4 }",
        "            }",
        "        }",
        "    };",
        "}",
        "",
    ]
    for family, catalog_name, type_name in expected_catalogs:
        entries = catalog_map[catalog_name]
        sounds = ordered_sound_catalogs[family]
        const_name = catalog_name.removesuffix("_KEYS") + "_IDENTITIES"
        lines.extend(
            (
                "define_sound_variant_catalog!(",
                f"    {type_name},",
                f"    {const_name},",
                "    [",
                *(
                    f"        ({rust_variant(identity.constant_name)}, {rust_string(identity.resource_id)}, {rust_string(sound.adult_hurt_sound_event_id)}, {rust_string(sound.baby_hurt_sound_event_id)}, {rust_string(sound.adult_death_sound_event_id)}, {rust_string(sound.baby_death_sound_event_id)}),"
                    for identity, sound in zip(entries, sounds, strict=True)
                ),
                "    ]",
                ");",
                "",
            )
        )
    lines.extend(
        (
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn animal_sound_variant_catalogs_are_complete_ordered_and_unique() {",
        )
    )
    for family, catalog_name, type_name in expected_catalogs:
        entries = catalog_map[catalog_name]
        sounds = ordered_sound_catalogs[family]
        const_name = catalog_name.removesuffix("_KEYS") + "_IDENTITIES"
        last = entries[-1]
        lines.extend(
            (
                f"        assert_eq!({const_name}.len(), {len(entries)});",
                f"        let resources: HashSet<_> = {const_name}.iter().map(|(resource, _, _, _, _, _)| *resource).collect();",
                f"        assert_eq!(resources.len(), {const_name}.len());",
                f"        assert_eq!({type_name}::for_vanilla_registry_id(0), Some({type_name}::Classic));",
                f"        assert_eq!({type_name}::for_vanilla_registry_id({len(entries) - 1}), Some({type_name}::{rust_variant(last.constant_name)}));",
                f"        assert_eq!({type_name}::for_vanilla_registry_id(-1), None);",
                f"        assert_eq!({type_name}::for_vanilla_registry_id({len(entries)}), None);",
                f"        assert_eq!({type_name}::for_resource_id({rust_string(last.resource_id)}), Some({type_name}::{rust_variant(last.constant_name)}));",
                f"        assert_eq!({type_name}::for_resource_id(\"minecraft:unknown\"), None);",
                f"        assert_eq!({type_name}::Classic.resource_id(), \"minecraft:classic\");",
            )
        )
        for identity, sound in zip(entries, sounds, strict=True):
            variant = rust_variant(identity.constant_name)
            lines.extend(
                (
                    f"        assert_eq!({type_name}::{variant}.builtin_hurt_sound_event(false), {rust_string(sound.adult_hurt_sound_event_id)});",
                    f"        assert_eq!({type_name}::{variant}.builtin_hurt_sound_event(true), {rust_string(sound.baby_hurt_sound_event_id)});",
                    f"        assert_eq!({type_name}::{variant}.builtin_death_sound_event(false), {rust_string(sound.adult_death_sound_event_id)});",
                    f"        assert_eq!({type_name}::{variant}.builtin_death_sound_event(true), {rust_string(sound.baby_death_sound_event_id)});",
                )
            )
    lines.extend(
        (
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_animal_sound_variants_bridge() -> str:
    return "\n".join(
        (
            "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
            "// Owner: bbb-facts animal_sound_variant_types; this bridge preserves the bbb-client-world API.",
            "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
            "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
            "",
            "pub use bbb_protocol::animal_sound_variant_types::{",
            "    CatSoundVariantIdentity, ChickenSoundVariantIdentity, CowSoundVariantIdentity,",
            "    PigSoundVariantIdentity, WolfSoundVariantIdentity,",
            "};",
            "",
        )
    )


def render_instruments(entries: list[BootstrapEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Instruments.java keys and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// One known vanilla 26.1 instrument identity.",
        "///",
        "/// Session holder ids must be resolved through the synchronized instrument registry before",
        "/// this fixed-order fallback; registry elements separately own the direct instrument payload.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum VanillaInstrumentIdentity {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "const VANILLA_INSTRUMENT_IDENTITIES: &[(&str, VanillaInstrumentIdentity)] = &[",
        *(
            f"    ({rust_string(entry.resource_id)}, VanillaInstrumentIdentity::{rust_variant(entry.constant_name)}),"
            for entry in entries
        ),
        "];",
        "",
        "impl VanillaInstrumentIdentity {",
        "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
        "        VANILLA_INSTRUMENT_IDENTITIES",
        "            .iter()",
        "            .find(|(resource, _)| *resource == resource_id)",
        "            .map(|(_, instrument)| *instrument)",
        "    }",
        "",
        "    /// Resolves the vanilla 26.1 bootstrap ordinal used when no session registry is available.",
        "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
        "        VANILLA_INSTRUMENT_IDENTITIES",
        "            .get(usize::try_from(registry_id).ok()?)",
        "            .map(|(_, instrument)| *instrument)",
        "    }",
        "",
        "    pub const fn vanilla_registry_index(self) -> usize {",
        "        self as usize",
        "    }",
        "",
        "    pub fn resource_id(self) -> &'static str {",
        "        VANILLA_INSTRUMENT_IDENTITIES[self.vanilla_registry_index()].0",
        "    }",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::collections::HashSet;",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn instrument_identity_catalog_is_complete_ordered_and_unique() {",
        f"        assert_eq!(VANILLA_INSTRUMENT_IDENTITIES.len(), {len(entries)});",
        "        let mut resources = HashSet::new();",
        "        for (index, (resource_id, instrument)) in",
        "            VANILLA_INSTRUMENT_IDENTITIES.iter().enumerate()",
        "        {",
        "            assert_eq!(instrument.vanilla_registry_index(), index);",
        "            assert!(resources.insert(*resource_id));",
        "            assert!(resource_id.starts_with(\"minecraft:\"));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn instrument_identity_catalog_preserves_vanilla_mappings() {",
        '        assert_eq!(VanillaInstrumentIdentity::for_vanilla_registry_id(0), Some(VanillaInstrumentIdentity::PonderGoatHorn));',
        '        assert_eq!(VanillaInstrumentIdentity::for_vanilla_registry_id(7), Some(VanillaInstrumentIdentity::DreamGoatHorn));',
        '        assert_eq!(VanillaInstrumentIdentity::for_vanilla_registry_id(-1), None);',
        '        assert_eq!(VanillaInstrumentIdentity::for_vanilla_registry_id(8), None);',
        '        assert_eq!(VanillaInstrumentIdentity::for_resource_id("minecraft:call_goat_horn"), Some(VanillaInstrumentIdentity::CallGoatHorn));',
        '        assert_eq!(VanillaInstrumentIdentity::for_resource_id("example:custom_horn"), None);',
        '        assert_eq!(VanillaInstrumentIdentity::DreamGoatHorn.resource_id(), "minecraft:dream_goat_horn");',
        "    }",
        "}",
        "",
    ]
    return "\n".join(lines)


def render_trim_patterns(entries: list[BootstrapEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 TrimPatterns.java keys and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// One known vanilla 26.1 armor-trim pattern identity.",
        "///",
        "/// Session holder ids must be resolved through the synchronized trim-pattern registry before",
        "/// this fixed-order fallback; registry elements separately own asset, description, and decal.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum VanillaTrimPatternIdentity {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "const VANILLA_TRIM_PATTERN_IDENTITIES: &[(&str, VanillaTrimPatternIdentity)] = &[",
        *(
            f"    ({rust_string(entry.resource_id)}, VanillaTrimPatternIdentity::{rust_variant(entry.constant_name)}),"
            for entry in entries
        ),
        "];",
        "",
        "impl VanillaTrimPatternIdentity {",
        "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
        "        VANILLA_TRIM_PATTERN_IDENTITIES",
        "            .iter()",
        "            .find(|(resource, _)| *resource == resource_id)",
        "            .map(|(_, pattern)| *pattern)",
        "    }",
        "",
        "    /// Resolves the vanilla 26.1 bootstrap ordinal used when no session registry is available.",
        "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
        "        VANILLA_TRIM_PATTERN_IDENTITIES",
        "            .get(usize::try_from(registry_id).ok()?)",
        "            .map(|(_, pattern)| *pattern)",
        "    }",
        "",
        "    pub const fn vanilla_registry_index(self) -> usize {",
        "        self as usize",
        "    }",
        "",
        "    pub fn resource_id(self) -> &'static str {",
        "        VANILLA_TRIM_PATTERN_IDENTITIES[self.vanilla_registry_index()].0",
        "    }",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::collections::HashSet;",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn trim_pattern_identity_catalog_is_complete_ordered_and_unique() {",
        f"        assert_eq!(VANILLA_TRIM_PATTERN_IDENTITIES.len(), {len(entries)});",
        "        let mut resources = HashSet::new();",
        "        for (index, (resource_id, pattern)) in",
        "            VANILLA_TRIM_PATTERN_IDENTITIES.iter().enumerate()",
        "        {",
        "            assert_eq!(pattern.vanilla_registry_index(), index);",
        "            assert!(resources.insert(*resource_id));",
        "            assert!(resource_id.starts_with(\"minecraft:\"));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn trim_pattern_identity_catalog_preserves_vanilla_mappings() {",
        '        assert_eq!(VanillaTrimPatternIdentity::for_vanilla_registry_id(0), Some(VanillaTrimPatternIdentity::Sentry));',
        '        assert_eq!(VanillaTrimPatternIdentity::for_vanilla_registry_id(17), Some(VanillaTrimPatternIdentity::Bolt));',
        '        assert_eq!(VanillaTrimPatternIdentity::for_vanilla_registry_id(-1), None);',
        '        assert_eq!(VanillaTrimPatternIdentity::for_vanilla_registry_id(18), None);',
        '        assert_eq!(VanillaTrimPatternIdentity::for_resource_id("minecraft:silence"), Some(VanillaTrimPatternIdentity::Silence));',
        '        assert_eq!(VanillaTrimPatternIdentity::for_resource_id("example:custom_trim"), None);',
        '        assert_eq!(VanillaTrimPatternIdentity::Bolt.resource_id(), "minecraft:bolt");',
        "    }",
        "}",
        "",
    ]
    return "\n".join(lines)


def render_trim_materials(entries: list[TrimMaterialEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 TrimMaterials.java bootstrap order and description colors,",
        "// plus MaterialAssetGroup.java base and equipment-asset override suffixes.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// One known vanilla 26.1 armor-trim material identity.",
        "///",
        "/// Session holder ids must be resolved through the synchronized trim-material registry",
        "/// before this fixed-order fallback; registry elements separately own assets and description.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum VanillaTrimMaterialIdentity {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaTrimMaterialDescriptor {",
        "    material: VanillaTrimMaterialIdentity,",
        "    resource_id: &'static str,",
        "    description_color: u32,",
        "    asset_name: &'static str,",
        "    override_armor_assets: &'static [(&'static str, &'static str)],",
        "}",
        "",
        "const VANILLA_TRIM_MATERIAL_DESCRIPTORS: &[VanillaTrimMaterialDescriptor] = &[",
    ]
    for entry in entries:
        lines.extend(
            (
                "    VanillaTrimMaterialDescriptor {",
                f"        material: VanillaTrimMaterialIdentity::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        description_color: 0x{entry.description_color:06X},",
                f"        asset_name: {rust_string(entry.asset_name)},",
                "        override_armor_assets: &[",
                *(
                    f"            ({rust_string(asset)}, {rust_string(suffix)}),"
                    for asset, suffix in entry.override_armor_assets
                ),
                "        ],",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl VanillaTrimMaterialIdentity {",
            "    pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
            "        VANILLA_TRIM_MATERIAL_DESCRIPTORS",
            "            .iter()",
            "            .find(|entry| entry.resource_id == resource_id)",
            "            .map(|entry| entry.material)",
            "    }",
            "",
            "    /// Resolves the vanilla 26.1 bootstrap ordinal used when no session registry is available.",
            "    pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
            "        VANILLA_TRIM_MATERIAL_DESCRIPTORS",
            "            .get(usize::try_from(registry_id).ok()?)",
            "            .map(|entry| entry.material)",
            "    }",
            "",
            "    pub const fn vanilla_registry_index(self) -> usize {",
            "        self as usize",
            "    }",
            "",
            "    pub fn resource_id(self) -> &'static str {",
            "        VANILLA_TRIM_MATERIAL_DESCRIPTORS[self.vanilla_registry_index()].resource_id",
            "    }",
            "",
            "    pub fn description_color(self) -> u32 {",
            "        VANILLA_TRIM_MATERIAL_DESCRIPTORS[self.vanilla_registry_index()]",
            "            .description_color",
            "    }",
            "",
            "    pub fn asset_name(self) -> &'static str {",
            "        VANILLA_TRIM_MATERIAL_DESCRIPTORS[self.vanilla_registry_index()].asset_name",
            "    }",
            "",
            "    pub fn asset_suffix_for_equipment(self, equipment_asset: &str) -> &'static str {",
            "        let descriptor = &VANILLA_TRIM_MATERIAL_DESCRIPTORS[self.vanilla_registry_index()];",
            "        descriptor",
            "            .override_armor_assets",
            "            .iter()",
            "            .find(|(asset, _)| *asset == equipment_asset)",
            "            .map_or(descriptor.asset_name, |(_, suffix)| *suffix)",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn trim_material_catalog_is_complete_ordered_and_unique() {",
            f"        assert_eq!(VANILLA_TRIM_MATERIAL_DESCRIPTORS.len(), {len(entries)});",
            "        let mut resources = HashSet::new();",
            "        for (index, entry) in VANILLA_TRIM_MATERIAL_DESCRIPTORS.iter().enumerate() {",
            "            assert_eq!(entry.material.vanilla_registry_index(), index);",
            "            assert!(resources.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "            assert!(entry.description_color <= 0xFF_FF_FF);",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn trim_material_catalog_preserves_vanilla_mappings() {",
            '        assert_eq!(VanillaTrimMaterialIdentity::for_vanilla_registry_id(0), Some(VanillaTrimMaterialIdentity::Quartz));',
            '        assert_eq!(VanillaTrimMaterialIdentity::for_vanilla_registry_id(10), Some(VanillaTrimMaterialIdentity::Resin));',
            '        assert_eq!(VanillaTrimMaterialIdentity::for_vanilla_registry_id(-1), None);',
            '        assert_eq!(VanillaTrimMaterialIdentity::for_vanilla_registry_id(11), None);',
            '        assert_eq!(VanillaTrimMaterialIdentity::for_resource_id("minecraft:iron"), Some(VanillaTrimMaterialIdentity::Iron));',
            '        assert_eq!(VanillaTrimMaterialIdentity::for_resource_id("example:custom_trim"), None);',
            '        assert_eq!(VanillaTrimMaterialIdentity::Iron.resource_id(), "minecraft:iron");',
            '        assert_eq!(VanillaTrimMaterialIdentity::Iron.description_color(), 0xEC_EC_EC);',
            '        assert_eq!(VanillaTrimMaterialIdentity::Iron.asset_name(), "iron");',
            '        assert_eq!(VanillaTrimMaterialIdentity::Iron.asset_suffix_for_equipment("minecraft:iron"), "iron_darker");',
            '        assert_eq!(VanillaTrimMaterialIdentity::Iron.asset_suffix_for_equipment("minecraft:diamond"), "iron");',
            '        assert_eq!(VanillaTrimMaterialIdentity::Resin.description_color(), 0xFC_78_12);',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def extract_catalogs(source_root: Path) -> list[tuple[str, list[BootstrapEntry]]]:
    temperature_paths = {
        f"TemperatureVariants.{name}": path
        for name, path in identifier_constant_paths(
            parse_java(source_root, TEMPERATURE_VARIANTS_SOURCE)
        ).items()
    }
    return [
        (
            "VANILLA_TRIM_PATTERN_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, TRIM_PATTERNS_SOURCE),
                "TrimPattern",
                {"registryKey"},
                18,
            ),
        ),
        (
            "VANILLA_VILLAGER_TYPE_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, VILLAGER_TYPES_SOURCE),
                "VillagerType",
                {"createKey"},
                7,
            ),
        ),
        (
            "VANILLA_WOLF_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, WOLF_VARIANTS_SOURCE),
                "WolfVariant",
                {"createKey"},
                9,
            ),
        ),
        (
            "VANILLA_INSTRUMENT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, INSTRUMENTS_SOURCE),
                "Instrument",
                {"create"},
                8,
            ),
        ),
        (
            "VANILLA_CAT_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, CAT_VARIANTS_SOURCE),
                "CatVariant",
                {"createKey"},
                11,
                registration_names={"register", "registerForAnyConditions"},
            ),
        ),
        (
            "VANILLA_CHICKEN_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, CHICKEN_VARIANTS_SOURCE),
                "ChickenVariant",
                {"createKey"},
                3,
                referenced_paths=temperature_paths,
            ),
        ),
        (
            "VANILLA_COW_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, COW_VARIANTS_SOURCE),
                "CowVariant",
                {"createKey"},
                3,
                referenced_paths=temperature_paths,
            ),
        ),
        (
            "VANILLA_PIG_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, PIG_VARIANTS_SOURCE),
                "PigVariant",
                {"createKey"},
                3,
                referenced_paths=temperature_paths,
            ),
        ),
        (
            "VANILLA_FROG_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, FROG_VARIANTS_SOURCE),
                "FrogVariant",
                {"createKey"},
                3,
                referenced_paths=temperature_paths,
            ),
        ),
    ] + [
        (
            catalog_name,
            sound_variant_entries(
                parse_java(source_root, source_path),
                value_type,
                sound_set_owner,
                sound_map_field,
                expected_count,
            ),
        )
        for (
            _,
            catalog_name,
            value_type,
            _,
            sound_set_owner,
            sound_map_field,
            source_path,
            expected_count,
        ) in ANIMAL_SOUND_VARIANT_CONFIGS
    ]


def format_rust(rendered: str) -> str:
    formatted = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input=rendered,
        text=True,
        capture_output=True,
        check=True,
    )
    return formatted.stdout


def generate(
    source_root: Path,
    persistent_tags_bin_rel: str = "../data/persistent_registry_tags_26_1.bin",
) -> tuple[Any, ...]:
    sign_wood_types = wood_type_entries(parse_java(source_root, WOOD_TYPES_SOURCE))
    trim_materials = trim_material_entries(
        parse_java(source_root, TRIM_MATERIALS_SOURCE),
        parse_java(source_root, MATERIAL_ASSET_GROUP_SOURCE),
    )
    banner_patterns = banner_pattern_entries(
        parse_java(source_root, BANNER_PATTERNS_SOURCE)
    )
    villager_professions = bootstrap_resource_key_entries(
        parse_java(source_root, VILLAGER_PROFESSIONS_SOURCE),
        "VillagerProfession",
        {"createKey"},
        15,
    )
    villager_types = bootstrap_resource_key_entries(
        parse_java(source_root, VILLAGER_TYPES_SOURCE),
        "VillagerType",
        {"createKey"},
        7,
    )
    paintings = painting_variant_entries(
        parse_java(source_root, PAINTING_VARIANTS_SOURCE)
    )
    decorated_pot_patterns = decorated_pot_pattern_entries(
        parse_java(source_root, DECORATED_POT_PATTERNS_SOURCE),
        parse_java(source_root, ITEMS_SOURCE),
    )
    decorated_pot_texture_dimensions = {
        entry.asset_id: vanilla_texture_dimensions(
            source_root,
            entity_texture_asset_id(entry.asset_id, "decorated_pot"),
        )
        for entry in decorated_pot_patterns
    }
    enchantments = enchantment_entries(parse_java(source_root, ENCHANTMENTS_SOURCE))
    level_based_value_codecs = registry_codec_entries(
        parse_java(source_root, LEVEL_BASED_VALUE_SOURCE), 6
    )
    enchantment_value_effect_codecs = registry_codec_entries(
        parse_java(source_root, ENCHANTMENT_VALUE_EFFECT_SOURCE), 6
    )
    enchantment_effect_components = enchantment_effect_component_entries(
        parse_java(source_root, ENCHANTMENT_EFFECT_COMPONENTS_SOURCE), 31
    )
    enchantment_entity_effect_codecs = registry_codec_entries(
        parse_java(source_root, ENCHANTMENT_ENTITY_EFFECT_SOURCE), 15
    )
    enchantment_location_effect_codecs = registry_codec_entries(
        parse_java(source_root, ENCHANTMENT_LOCATION_BASED_EFFECT_SOURCE),
        16,
        codec_terminals=frozenset({"CODEC", "MAP_CODEC"}),
    )
    loot_context_params = loot_context_param_entries(
        parse_java(source_root, LOOT_CONTEXT_PARAMS_SOURCE)
    )
    loot_context_entity_targets = loot_context_entity_target_entries(
        parse_java(source_root, LOOT_CONTEXT_SOURCE), loot_context_params
    )
    loot_context_param_sets = loot_context_param_set_entries(
        parse_java(source_root, LOOT_CONTEXT_PARAM_SETS_SOURCE), loot_context_params
    )
    configured_feature_codecs = configured_feature_codec_entries(
        parse_java(source_root, FEATURE_SOURCE)
    )
    codec_dispatch_catalogs = [
        (
            "LootConditionCodecFamily",
            "LootConditionCodecType",
            "VANILLA_LOOT_CONDITION_CODEC_TYPES",
            "loot_condition_codec_type",
            registry_codec_entries(
                parse_java(source_root, LOOT_ITEM_CONDITIONS_SOURCE),
                20,
                codec_terminals=frozenset({"MAP_CODEC"}),
            ),
        ),
        (
            "NumberProviderCodecFamily",
            "NumberProviderCodecType",
            "VANILLA_NUMBER_PROVIDER_CODEC_TYPES",
            "number_provider_codec_type",
            registry_codec_entries(
                parse_java(source_root, NUMBER_PROVIDERS_SOURCE),
                8,
                codec_terminals=frozenset({"MAP_CODEC"}),
            ),
        ),
        (
            "FloatProviderCodecFamily",
            "FloatProviderCodecType",
            "VANILLA_FLOAT_PROVIDER_CODEC_TYPES",
            "float_provider_codec_type",
            registry_codec_entries(
                parse_java(source_root, FLOAT_PROVIDERS_SOURCE),
                4,
                codec_terminals=frozenset({"MAP_CODEC"}),
            ),
        ),
        (
            "IntProviderCodecFamily",
            "IntProviderCodecType",
            "VANILLA_INT_PROVIDER_CODEC_TYPES",
            "int_provider_codec_type",
            registry_codec_entries(
                parse_java(source_root, INT_PROVIDERS_SOURCE),
                7,
                codec_terminals=frozenset({"MAP_CODEC"}),
            ),
        ),
        (
            "ScoreboardNameProviderCodecFamily",
            "ScoreboardNameProviderCodecType",
            "VANILLA_SCOREBOARD_NAME_PROVIDER_CODEC_TYPES",
            "scoreboard_name_provider_codec_type",
            registry_codec_entries(
                parse_java(source_root, SCOREBOARD_NAME_PROVIDERS_SOURCE),
                2,
                codec_terminals=frozenset({"MAP_CODEC"}),
            ),
        ),
        (
            "BlockPredicateCodecFamily",
            "BlockPredicateCodecType",
            "VANILLA_BLOCK_PREDICATE_CODEC_TYPES",
            "block_predicate_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, BLOCK_PREDICATE_TYPE_SOURCE),
                "BlockPredicateType<",
                "BuiltInRegistries.BLOCK_PREDICATE_TYPE",
                "()->codec",
                13,
            ),
        ),
        (
            "BlockStateProviderCodecFamily",
            "BlockStateProviderCodecType",
            "VANILLA_BLOCK_STATE_PROVIDER_CODEC_TYPES",
            "block_state_provider_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, BLOCK_STATE_PROVIDER_TYPE_SOURCE),
                "BlockStateProviderType<",
                "BuiltInRegistries.BLOCKSTATE_PROVIDER_TYPE",
                "newBlockStateProviderType<>(codec)",
                8,
            ),
        ),
        (
            "EntitySubPredicateCodecFamily",
            "EntitySubPredicateCodecType",
            "VANILLA_ENTITY_SUB_PREDICATE_CODEC_TYPES",
            "entity_sub_predicate_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, ENTITY_SUB_PREDICATES_SOURCE),
                "MapCodec<",
                "BuiltInRegistries.ENTITY_SUB_PREDICATE_TYPE",
                "value",
                6,
                bootstrap_sentinel="LIGHTNING",
            ),
        ),
        (
            "ConfiguredCarverCodecFamily",
            "ConfiguredCarverCodecType",
            "VANILLA_CONFIGURED_CARVER_CODEC_TYPES",
            "configured_carver_codec_type",
            world_carver_codec_entries(parse_java(source_root, WORLD_CARVER_SOURCE)),
        ),
        (
            "HeightProviderCodecFamily",
            "HeightProviderCodecType",
            "VANILLA_HEIGHT_PROVIDER_CODEC_TYPES",
            "height_provider_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, HEIGHT_PROVIDER_TYPE_SOURCE),
                "HeightProviderType<",
                "BuiltInRegistries.HEIGHT_PROVIDER_TYPE",
                "()->codec",
                6,
                codec_terminals=frozenset({"CODEC"}),
            ),
        ),
        (
            "PlacementModifierCodecFamily",
            "PlacementModifierCodecType",
            "VANILLA_PLACEMENT_MODIFIER_CODEC_TYPES",
            "placement_modifier_codec_type",
            placement_modifier_codec_entries(
                parse_java(source_root, PLACEMENT_MODIFIER_TYPE_SOURCE)
            ),
        ),
        (
            "RuleTestCodecFamily",
            "RuleTestCodecType",
            "VANILLA_RULE_TEST_CODEC_TYPES",
            "rule_test_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, RULE_TEST_TYPE_SOURCE),
                "RuleTestType<",
                "BuiltInRegistries.RULE_TEST",
                "()->codec",
                6,
            ),
        ),
        (
            "TrunkPlacerCodecFamily",
            "TrunkPlacerCodecType",
            "VANILLA_TRUNK_PLACER_CODEC_TYPES",
            "trunk_placer_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, TRUNK_PLACER_TYPE_SOURCE),
                "TrunkPlacerType<",
                "BuiltInRegistries.TRUNK_PLACER_TYPE",
                "newTrunkPlacerType<>(codec)",
                9,
            ),
        ),
        (
            "FoliagePlacerCodecFamily",
            "FoliagePlacerCodecType",
            "VANILLA_FOLIAGE_PLACER_CODEC_TYPES",
            "foliage_placer_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, FOLIAGE_PLACER_TYPE_SOURCE),
                "FoliagePlacerType<",
                "BuiltInRegistries.FOLIAGE_PLACER_TYPE",
                "newFoliagePlacerType<>(codec)",
                11,
            ),
        ),
        (
            "TreeDecoratorCodecFamily",
            "TreeDecoratorCodecType",
            "VANILLA_TREE_DECORATOR_CODEC_TYPES",
            "tree_decorator_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, TREE_DECORATOR_TYPE_SOURCE),
                "TreeDecoratorType<",
                "BuiltInRegistries.TREE_DECORATOR_TYPE",
                "newTreeDecoratorType<>(codec)",
                10,
            ),
        ),
        (
            "RootPlacerCodecFamily",
            "RootPlacerCodecType",
            "VANILLA_ROOT_PLACER_CODEC_TYPES",
            "root_placer_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, ROOT_PLACER_TYPE_SOURCE),
                "RootPlacerType<",
                "BuiltInRegistries.ROOT_PLACER_TYPE",
                "newRootPlacerType<>(codec)",
                1,
            ),
        ),
        (
            "FeatureSizeCodecFamily",
            "FeatureSizeCodecType",
            "VANILLA_FEATURE_SIZE_CODEC_TYPES",
            "feature_size_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, FEATURE_SIZE_TYPE_SOURCE),
                "FeatureSizeType<",
                "BuiltInRegistries.FEATURE_SIZE_TYPE",
                "newFeatureSizeType<>(codec)",
                2,
            ),
        ),
        (
            "StructureProcessorCodecFamily",
            "StructureProcessorCodecType",
            "VANILLA_STRUCTURE_PROCESSOR_CODEC_TYPES",
            "structure_processor_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, STRUCTURE_PROCESSOR_TYPE_SOURCE),
                "StructureProcessorType<",
                "BuiltInRegistries.STRUCTURE_PROCESSOR",
                "()->codec",
                11,
            ),
        ),
        (
            "PosRuleTestCodecFamily",
            "PosRuleTestCodecType",
            "VANILLA_POS_RULE_TEST_CODEC_TYPES",
            "pos_rule_test_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, POS_RULE_TEST_TYPE_SOURCE),
                "PosRuleTestType<",
                "BuiltInRegistries.POS_RULE_TEST",
                "()->codec",
                3,
            ),
        ),
        (
            "RuleBlockEntityModifierCodecFamily",
            "RuleBlockEntityModifierCodecType",
            "VANILLA_RULE_BLOCK_ENTITY_MODIFIER_CODEC_TYPES",
            "rule_block_entity_modifier_codec_type",
            static_registry_codec_entries(
                parse_java(source_root, RULE_BLOCK_ENTITY_MODIFIER_TYPE_SOURCE),
                "RuleBlockEntityModifierType<",
                "BuiltInRegistries.RULE_BLOCK_ENTITY_MODIFIER",
                "()->codec",
                4,
            ),
        ),
    ]
    mob_effects = mob_effect_entries(parse_java(source_root, MOB_EFFECTS_SOURCE))
    potions = potion_entries(
        parse_java(source_root, POTIONS_SOURCE),
        {entry.constant_name for entry in mob_effects},
    )
    tooltip_order = enchantment_tooltip_order(
        load_tag_resources(source_root, ENCHANTMENT_TAG_SOURCE_DIR), enchantments
    )
    persistent_registry_tags = {
        registry_id: resolve_all_tag_values(
            load_tag_resources(source_root, relative_directory),
            registry_name=registry_id.removeprefix("minecraft:"),
        )
        for registry_id, relative_directory in PERSISTENT_REGISTRY_TAG_SOURCE_DIRS.items()
    }
    persistent_entry_ids = persistent_registry_entry_ids(source_root)
    persistent_entry_ids["minecraft:chunk_generator"] = chunk_generator_entry_ids(source_root)
    catalogs = extract_catalogs(source_root)
    animal_sound_variant_sounds = animal_sound_variant_sound_entries(
        parse_java(source_root, SOUND_EVENTS_SOURCE),
        {
            family: parse_java(source_root, source_path)
            for (
                family,
                _,
                _,
                _,
                _,
                _,
                source_path,
                _,
            ) in ANIMAL_SOUND_VARIANT_CONFIGS
        },
    )
    temperature_paths = {
        f"TemperatureVariants.{name}": path
        for name, path in identifier_constant_paths(
            parse_java(source_root, TEMPERATURE_VARIANTS_SOURCE)
        ).items()
    }
    frog_variants = frog_variant_entries(
        parse_java(source_root, FROG_VARIANTS_SOURCE),
        temperature_paths,
    )
    farm_variants = [
        (
            "Chicken",
            farm_animal_variant_entries(
                parse_java(source_root, CHICKEN_VARIANTS_SOURCE),
                temperature_paths,
                "ChickenVariant",
                "chicken",
            ),
        ),
        (
            "Cow",
            farm_animal_variant_entries(
                parse_java(source_root, COW_VARIANTS_SOURCE),
                temperature_paths,
                "CowVariant",
                "cow",
            ),
        ),
        (
            "Pig",
            farm_animal_variant_entries(
                parse_java(source_root, PIG_VARIANTS_SOURCE),
                temperature_paths,
                "PigVariant",
                "pig",
            ),
        ),
    ]
    temperature_texture_dimensions = {
        asset_id: vanilla_texture_dimensions(source_root, asset_id)
        for _, entries in farm_variants
        for entry in entries
        for asset_id in (entry.adult_asset_id, entry.baby_asset_id)
    }
    temperature_texture_dimensions.update(
        {
            entry.asset_id: vanilla_texture_dimensions(source_root, entry.asset_id)
            for entry in frog_variants
        }
    )
    zombie_nautilus_variants = model_and_texture_variant_entries(
        parse_java(source_root, ZOMBIE_NAUTILUS_VARIANTS_SOURCE),
        temperature_paths,
        "ZombieNautilusVariant",
        "nautilus",
        expected_count=2,
    )
    zombie_nautilus_texture_dimensions = {
        entry.asset_id: vanilla_texture_dimensions(source_root, entry.asset_id)
        for entry in zombie_nautilus_variants
    }
    trim_patterns = dict(catalogs)["VANILLA_TRIM_PATTERN_KEYS"]
    instruments = dict(catalogs)["VANILLA_INSTRUMENT_KEYS"]
    cat_variants = cat_variant_entries(
        parse_java(source_root, CAT_VARIANTS_SOURCE)
    )
    cat_texture_dimensions = {
        asset_id: vanilla_texture_dimensions(source_root, asset_id)
        for entry in cat_variants
        for asset_id in (entry.adult_asset_id, entry.baby_asset_id)
    }
    wolf_variants = wolf_variant_entries(
        parse_java(source_root, WOLF_VARIANTS_SOURCE)
    )
    wolf_texture_dimensions = {
        asset_id: vanilla_texture_dimensions(source_root, asset_id)
        for entry in wolf_variants
        for asset_id in entry.asset_ids
    }
    map_decorations = map_decoration_entries(
        parse_java(source_root, MAP_DECORATIONS_SOURCE),
        map_colors=map_color_constants(parse_java(source_root, Path("net/minecraft/world/level/material/MapColor.java"))),
    )
    registry_data_loader = parse_java(source_root, REGISTRY_DATA_LOADER_SOURCE)
    registries = parse_java(source_root, REGISTRIES_SOURCE)
    worldgen_registry_data = registry_data_catalog(
        registry_data_loader, registries, "WORLDGEN_REGISTRIES"
    )
    dimension_registry_data = registry_data_catalog(
        registry_data_loader, registries, "DIMENSION_REGISTRIES"
    )
    synchronized_registry_data = registry_data_catalog(
        registry_data_loader, registries, "SYNCHRONIZED_REGISTRIES"
    )
    built_in_registry_data = built_in_registry_catalog(
        parse_java(source_root, BUILT_IN_REGISTRIES_SOURCE), registries
    )
    fluid_identities = static_register_entries(
        parse_java(source_root, FLUIDS_SOURCE),
        ("Fluid", "FlowingFluid"),
        5,
    )
    chat_type_identities = bootstrap_resource_key_entries(
        parse_java(source_root, CHAT_TYPES_SOURCE),
        "ChatType",
        {"create"},
        7,
    )
    overworld_day_timeline = overworld_day_timeline_entry(
        parse_java(source_root, TIMELINES_SOURCE)
    )
    banner_texture_dimensions = {
        entry.asset_id: vanilla_texture_dimensions(
            source_root,
            entity_texture_asset_id(entry.asset_id, "banner"),
        )
        for entry in banner_patterns
    }
    rendered = render_enchantment_catalog(enchantments, tooltip_order)
    return (
        format_rust(rendered),
        format_rust(
            render_enchantment_codec_types(
                level_based_value_codecs,
                enchantment_value_effect_codecs,
                enchantment_effect_components,
                enchantment_entity_effect_codecs,
                enchantment_location_effect_codecs,
                loot_context_param_sets,
            )
        ),
        format_rust(
            render_codec_dispatch_types(
                codec_dispatch_catalogs, configured_feature_codecs
            )
        ),
        format_rust(
            render_loot_context_types(
                loot_context_params,
                loot_context_entity_targets,
                loot_context_param_sets,
            )
        ),
        format_rust(render_enchantment_catalog_bridge()),
        format_rust(
            render_persistent_registry_facts(
                persistent_registry_tags,
                persistent_entry_ids,
                banner_patterns,
                persistent_tags_bin_rel,
            )
        ),
        format_rust(render_potion_catalog(potions)),
        format_rust(render_painting_variants(paintings)),
        format_rust(render_painting_variants_bridge()),
        format_rust(
            render_decorated_pot_patterns(
                decorated_pot_patterns,
                decorated_pot_texture_dimensions,
            )
        ),
        format_rust(render_banner_pattern_types(banner_patterns)),
        format_rust(render_banner_patterns(banner_patterns, banner_texture_dimensions)),
        format_rust(render_villager_professions(villager_professions)),
        format_rust(render_villager_types(villager_types)),
        format_rust(render_cat_variants(cat_variants, cat_texture_dimensions)),
        format_rust(render_wolf_variants(wolf_variants, wolf_texture_dimensions)),
        format_rust(
            render_temperature_animal_variants(
                catalogs,
                farm_variants,
                frog_variants,
                temperature_texture_dimensions,
            )
        ),
        format_rust(
            render_zombie_nautilus_variants(
                zombie_nautilus_variants,
                zombie_nautilus_texture_dimensions,
            )
        ),
        format_rust(
            render_animal_sound_variants(
                catalogs,
                animal_sound_variant_sounds,
            )
        ),
        format_rust(render_animal_sound_variants_bridge()),
        format_rust(render_instruments(instruments)),
        format_rust(render_trim_patterns(trim_patterns)),
        format_rust(render_trim_materials(trim_materials)),
        format_rust(render_map_decorations(map_decorations)),
        format_rust(
            render_registry_data_types(
                worldgen_registry_data,
                dimension_registry_data,
                synchronized_registry_data,
                built_in_registry_data,
                fluid_identities,
                chat_type_identities,
            )
        ),
        format_rust(render_sign_wood_types(sign_wood_types)),
        format_rust(render_overworld_day_timeline(overworld_day_timeline)),
        pack_persistent_registry_tags(persistent_registry_tags),
    )


def check_generated(path: Path, generated: str) -> bool:
    current = path.read_text() if path.exists() else ""
    if current == generated:
        return True
    sys.stderr.writelines(
        difflib.unified_diff(
            current.splitlines(keepends=True),
            generated.splitlines(keepends=True),
            fromfile=str(path),
            tofile="generated",
        )
    )
    return False


def check_generated_bytes(path: Path, generated: bytes) -> bool:
    current = path.read_bytes() if path.exists() else b""
    if current == generated:
        return True
    sys.stderr.write(
        f"binary output differs at {path} "
        f"(actual {len(current)} B vs expected {len(generated)} B)\n"
    )
    return False


def main() -> int:
    args = parse_args()
    if args.potions_only:
        mob_effects = mob_effect_entries(parse_java(args.source_root, MOB_EFFECTS_SOURCE))
        generated_potions = format_rust(
            render_potion_catalog(
                potion_entries(
                    parse_java(args.source_root, POTIONS_SOURCE),
                    {entry.constant_name for entry in mob_effects},
                )
            )
        )
        if args.check:
            return 0 if check_generated(args.potion_catalog_output, generated_potions) else 1
        args.potion_catalog_output.parent.mkdir(parents=True, exist_ok=True)
        args.potion_catalog_output.write_text(generated_potions)
        return 0
    if args.map_decorations_only:
        generated_map_decorations = format_rust(
            render_map_decorations(
                map_decoration_entries(
                    parse_java(args.source_root, MAP_DECORATIONS_SOURCE),
                    map_colors=map_color_constants(parse_java(args.source_root, Path("net/minecraft/world/level/material/MapColor.java"))),
                )
            )
        )
        if args.check:
            return 0 if check_generated(args.map_decorations_output, generated_map_decorations) else 1
        args.map_decorations_output.parent.mkdir(parents=True, exist_ok=True)
        args.map_decorations_output.write_text(generated_map_decorations)
        return 0
    persistent_tags_bin_output = args.persistent_registry_tags_bin_output
    if persistent_tags_bin_output is None:
        persistent_tags_bin_output = (
            DEFAULT_PERSISTENT_REGISTRY_TAGS_BIN_OUTPUT
            if args.persistent_registry_facts_output
            == DEFAULT_PERSISTENT_REGISTRY_FACTS_OUTPUT
            else args.persistent_registry_facts_output.with_suffix(".bin")
        )
    persistent_tags_bin_rel = os.path.relpath(
        persistent_tags_bin_output, args.persistent_registry_facts_output.parent
    ).replace("\\", "/")
    (
        generated,
        generated_enchantment_codec_types,
        generated_codec_dispatch_types,
        generated_loot_context_types,
        generated_enchantment_bridge,
        generated_persistent_registry_facts,
        generated_potions,
        generated_painting_variant_types,
        generated_painting_variants,
        generated_decorated_pot_patterns,
        generated_banner_pattern_types,
        generated_banner_patterns,
        generated_villager_professions,
        generated_villager_types,
        generated_cat_variants,
        generated_wolf_variants,
        generated_temperature_animal_variants,
        generated_zombie_nautilus_variants,
        generated_animal_sound_variant_types,
        generated_animal_sound_variants,
        generated_instruments,
        generated_trim_patterns,
        generated_trim_materials,
        generated_map_decorations,
        generated_registry_data_types,
        generated_sign_wood_types,
        generated_overworld_day_timeline,
        generated_persistent_tags_bin,
    ) = generate(args.source_root, persistent_tags_bin_rel)
    if args.check:
        matches = check_generated(args.output, generated)
        matches &= check_generated(
            args.enchantment_codec_types_output,
            generated_enchantment_codec_types,
        )
        matches &= check_generated(
            args.codec_dispatch_types_output,
            generated_codec_dispatch_types,
        )
        matches &= check_generated(
            args.loot_context_types_output,
            generated_loot_context_types,
        )
        matches &= check_generated(
            args.enchantment_bridge_output, generated_enchantment_bridge
        )
        matches &= check_generated(
            args.persistent_registry_facts_output,
            generated_persistent_registry_facts,
        )
        matches &= check_generated_bytes(
            persistent_tags_bin_output,
            generated_persistent_tags_bin,
        )
        matches &= check_generated(args.potion_catalog_output, generated_potions)
        matches &= check_generated(
            args.painting_variant_types_output, generated_painting_variant_types
        )
        matches &= check_generated(
            args.painting_variants_output, generated_painting_variants
        )
        matches &= check_generated(
            args.decorated_pot_patterns_output,
            generated_decorated_pot_patterns,
        )
        matches &= check_generated(
            args.banner_pattern_types_output,
            generated_banner_pattern_types,
        )
        matches &= check_generated(
            args.banner_patterns_output,
            generated_banner_patterns,
        )
        matches &= check_generated(
            args.villager_professions_output,
            generated_villager_professions,
        )
        matches &= check_generated(
            args.villager_types_output,
            generated_villager_types,
        )
        matches &= check_generated(
            args.cat_variants_output,
            generated_cat_variants,
        )
        matches &= check_generated(
            args.wolf_variants_output,
            generated_wolf_variants,
        )
        matches &= check_generated(
            args.temperature_animal_variants_output,
            generated_temperature_animal_variants,
        )
        matches &= check_generated(
            args.zombie_nautilus_variants_output,
            generated_zombie_nautilus_variants,
        )
        matches &= check_generated(
            args.animal_sound_variant_types_output,
            generated_animal_sound_variant_types,
        )
        matches &= check_generated(
            args.animal_sound_variants_output,
            generated_animal_sound_variants,
        )
        matches &= check_generated(
            args.instruments_output,
            generated_instruments,
        )
        matches &= check_generated(
            args.trim_patterns_output,
            generated_trim_patterns,
        )
        matches &= check_generated(
            args.trim_materials_output,
            generated_trim_materials,
        )
        matches &= check_generated(
            args.map_decorations_output,
            generated_map_decorations,
        )
        matches &= check_generated(
            args.registry_data_types_output,
            generated_registry_data_types,
        )
        matches &= check_generated(
            args.sign_wood_types_output,
            generated_sign_wood_types,
        )
        matches &= check_generated(
            args.overworld_day_timeline_output,
            generated_overworld_day_timeline,
        )
        return 0 if matches else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    args.enchantment_codec_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.enchantment_codec_types_output.write_text(generated_enchantment_codec_types)
    args.codec_dispatch_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.codec_dispatch_types_output.write_text(generated_codec_dispatch_types)
    args.loot_context_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.loot_context_types_output.write_text(generated_loot_context_types)
    args.enchantment_bridge_output.parent.mkdir(parents=True, exist_ok=True)
    args.enchantment_bridge_output.write_text(generated_enchantment_bridge)
    args.persistent_registry_facts_output.parent.mkdir(parents=True, exist_ok=True)
    args.persistent_registry_facts_output.write_text(
        generated_persistent_registry_facts
    )
    persistent_tags_bin_output.parent.mkdir(parents=True, exist_ok=True)
    persistent_tags_bin_output.write_bytes(generated_persistent_tags_bin)
    args.potion_catalog_output.parent.mkdir(parents=True, exist_ok=True)
    args.potion_catalog_output.write_text(generated_potions)
    args.painting_variant_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.painting_variant_types_output.write_text(generated_painting_variant_types)
    args.painting_variants_output.parent.mkdir(parents=True, exist_ok=True)
    args.painting_variants_output.write_text(generated_painting_variants)
    args.decorated_pot_patterns_output.parent.mkdir(parents=True, exist_ok=True)
    args.decorated_pot_patterns_output.write_text(generated_decorated_pot_patterns)
    args.banner_pattern_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.banner_pattern_types_output.write_text(generated_banner_pattern_types)
    args.banner_patterns_output.parent.mkdir(parents=True, exist_ok=True)
    args.banner_patterns_output.write_text(generated_banner_patterns)
    args.villager_professions_output.parent.mkdir(parents=True, exist_ok=True)
    args.villager_professions_output.write_text(generated_villager_professions)
    args.villager_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.villager_types_output.write_text(generated_villager_types)
    args.cat_variants_output.parent.mkdir(parents=True, exist_ok=True)
    args.cat_variants_output.write_text(generated_cat_variants)
    args.wolf_variants_output.parent.mkdir(parents=True, exist_ok=True)
    args.wolf_variants_output.write_text(generated_wolf_variants)
    args.temperature_animal_variants_output.parent.mkdir(parents=True, exist_ok=True)
    args.temperature_animal_variants_output.write_text(
        generated_temperature_animal_variants
    )
    args.zombie_nautilus_variants_output.parent.mkdir(parents=True, exist_ok=True)
    args.zombie_nautilus_variants_output.write_text(
        generated_zombie_nautilus_variants
    )
    args.animal_sound_variant_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.animal_sound_variant_types_output.write_text(
        generated_animal_sound_variant_types
    )
    args.animal_sound_variants_output.parent.mkdir(parents=True, exist_ok=True)
    args.animal_sound_variants_output.write_text(generated_animal_sound_variants)
    args.instruments_output.parent.mkdir(parents=True, exist_ok=True)
    args.instruments_output.write_text(generated_instruments)
    args.trim_patterns_output.parent.mkdir(parents=True, exist_ok=True)
    args.trim_patterns_output.write_text(generated_trim_patterns)
    args.trim_materials_output.parent.mkdir(parents=True, exist_ok=True)
    args.trim_materials_output.write_text(generated_trim_materials)
    args.map_decorations_output.parent.mkdir(parents=True, exist_ok=True)
    args.map_decorations_output.write_text(generated_map_decorations)
    args.registry_data_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.registry_data_types_output.write_text(generated_registry_data_types)
    args.sign_wood_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.sign_wood_types_output.write_text(generated_sign_wood_types)
    args.overworld_day_timeline_output.parent.mkdir(parents=True, exist_ok=True)
    args.overworld_day_timeline_output.write_text(
        generated_overworld_day_timeline
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
