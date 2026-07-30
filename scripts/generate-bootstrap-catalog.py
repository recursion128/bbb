#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 bootstrap resource-order catalogs."""

from __future__ import annotations

import argparse
import difflib
import json
import subprocess
import sys
from pathlib import Path

from bootstrap_catalog_extractor import (
    BannerPatternEntry,
    BootstrapEntry,
    CatVariantEntry,
    DecoratedPotPatternEntry,
    EnchantmentEntry,
    FarmAnimalVariantEntry,
    FrogVariantEntry,
    MapDecorationEntry,
    ModelAndTextureVariantEntry,
    OverworldDayTimelineEntry,
    PaintingVariantEntry,
    PotionEntry,
    TimelineKeyframeEntry,
    TimelineTrackEntry,
    TrimMaterialEntry,
    WoodTypeEntry,
    WolfVariantEntry,
    banner_pattern_entries,
    bootstrap_resource_key_entries,
    cat_variant_entries,
    decorated_pot_pattern_entries,
    enchantment_entries,
    enchantment_tooltip_order,
    farm_animal_variant_entries,
    frog_variant_entries,
    identifier_constant_paths,
    map_decoration_entries,
    model_and_texture_variant_entries,
    overworld_day_timeline_entry,
    painting_variant_entries,
    potion_entries,
    sound_variant_entries,
    static_register_entries,
    trim_material_entries,
    wolf_variant_entries,
    wood_type_entries,
)
from gameplay_id_catalog_extractor import mob_effect_entries
from registry_data_catalog_extractor import (
    BuiltInRegistryCatalogEntry,
    RegistryDataCatalogEntry,
    RegistryHolderConsumerEntry,
    built_in_registry_catalog,
    registry_data_catalog,
    registry_holder_consumer_catalog,
)
from sound_event_catalog_extractor import (
    SoundVariantSoundEntry,
    animal_sound_variant_sound_entries,
)
from vanilla_java_source import parse_source
from vanilla_tag_source import load_tag_resources


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-world/src/registries/enchantments/generated_26_1.rs"
)
DEFAULT_POTION_CATALOG_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/potion_types.rs"
)
DEFAULT_PAINTING_VARIANTS_OUTPUT = REPO_ROOT / "crates/bbb-world/src/painting_variants.rs"
DEFAULT_DECORATED_POT_PATTERNS_OUTPUT = (
    REPO_ROOT / "crates/bbb-renderer/src/entity_models/decorated_pot_patterns.rs"
)
DEFAULT_BANNER_PATTERNS_OUTPUT = (
    REPO_ROOT / "crates/bbb-renderer/src/entity_models/banner_patterns.rs"
)
DEFAULT_BANNER_PATTERN_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/banner_pattern_types.rs"
)
DEFAULT_VILLAGER_PROFESSIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/villager_model_professions.rs"
)
DEFAULT_VILLAGER_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/villager_model_types.rs"
)
DEFAULT_CAT_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/cat_variants.rs"
)
DEFAULT_WOLF_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/wolf_variants.rs"
)
DEFAULT_TEMPERATURE_ANIMAL_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/temperature_animal_variants.rs"
)
DEFAULT_ZOMBIE_NAUTILUS_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/zombie_nautilus_variants.rs"
)
DEFAULT_ANIMAL_SOUND_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-world/src/entities/sound_variants.rs"
)
DEFAULT_INSTRUMENTS_OUTPUT = REPO_ROOT / "crates/bbb-world/src/registries/instruments.rs"
DEFAULT_TRIM_PATTERNS_OUTPUT = REPO_ROOT / "crates/bbb-world/src/registries/trim_patterns.rs"
DEFAULT_TRIM_MATERIALS_OUTPUT = REPO_ROOT / "crates/bbb-world/src/registries/trim_materials.rs"
DEFAULT_MAP_DECORATIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/map_decorations/generated_26_1.rs"
)
DEFAULT_REGISTRY_DATA_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/registry_data_types.rs"
)
DEFAULT_REGISTRY_HOLDER_CONSUMERS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/registry_holder_consumers.rs"
)
DEFAULT_REGISTRY_HOLDER_COVERAGE_OUTPUT = (
    REPO_ROOT / "docs/registry-holder-consumer-coverage-26.1.json"
)
DEFAULT_REGISTRY_HOLDER_COVERAGE_OVERRIDES = (
    REPO_ROOT / "scripts/data/registry-holder-consumer-coverage-26.1.json"
)
DEFAULT_SIGN_WOOD_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-world/src/sign_blocks/generated_26_1.rs"
)
DEFAULT_OVERWORLD_DAY_TIMELINE_OUTPUT = (
    REPO_ROOT / "crates/bbb-native/src/runtime/overworld_day_timeline.rs"
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
ENCHANTMENT_TAG_SOURCE_DIR = Path("data/minecraft/tags/enchantment")
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
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--potion-catalog-output",
        type=Path,
        default=DEFAULT_POTION_CATALOG_OUTPUT,
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
    parser.add_argument(
        "--registry-data-types-output",
        type=Path,
        default=DEFAULT_REGISTRY_DATA_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--registry-holder-consumers-output",
        type=Path,
        default=DEFAULT_REGISTRY_HOLDER_CONSUMERS_OUTPUT,
    )
    parser.add_argument(
        "--registry-holder-coverage-output",
        type=Path,
        default=DEFAULT_REGISTRY_HOLDER_COVERAGE_OUTPUT,
    )
    parser.add_argument(
        "--registry-holder-coverage-overrides",
        type=Path,
        default=DEFAULT_REGISTRY_HOLDER_COVERAGE_OVERRIDES,
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
    return parser.parse_args()


def parse_java(source_root: Path, relative_path: Path):
    return parse_source(relative_path.as_posix(), (source_root / relative_path).read_bytes())


def registry_holder_consumer_sources(source_root: Path):
    tokens = (
        b"RegistryFixedCodec",
        b"RegistryFileCodec",
        b"RegistryCodecs",
        b"HolderSetCodec",
        b"ByteBufCodecs.holder",
        b"TagKey.",
        b"holderByNameCodec",
    )
    java_root = source_root / "net/minecraft"
    sources = []
    for path in sorted(java_root.rglob("*.java")):
        raw = path.read_bytes()
        if any(token in raw for token in tokens):
            relative_path = path.relative_to(source_root)
            sources.append(parse_source(relative_path.as_posix(), raw))
    return sources


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
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
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
        "",
        "    #[test]",
        "    fn production_consumers_do_not_restore_sign_wood_tables() {",
        "        let world = include_str!(\"../sign_blocks.rs\");",
        "        let parser = world.split(\"pub fn sign_wood_and_form_for_block_name\").nth(1).and_then(|source| source.split(\"/// Vanilla `RotationSegment.convertToDegrees\").next()).expect(\"sign block-name parser\");",
        "        assert_eq!(parser.matches(\"SignWoodKind::parse(wood)\").count(), 1);",
        "        let fixture = world.split(\"fn maps_every_generated_sign_wood_name_to_each_block_form\").nth(1).and_then(|source| source.split(\"\\n    #[test]\").next()).expect(\"sign block-name fixture\");",
        "        assert!(fixture.contains(\"vanilla_sign_wood_kinds()\"));",
        "        assert!(fixture.contains(\"wood.name()\"));",
        "        assert!(!fixture.contains(\"SignWoodKind::\"), \"sign parser fixture restored representative wood variants\");",
        "        let native = include_str!(\"../../../bbb-native/src/hud_assets.rs\");",
        "        let loader = native.split(\"fn load_hanging_sign_backgrounds(\").nth(1).and_then(|source| source.split(\"fn load_gui_sprites(\").next()).expect(\"hanging-sign background loader\");",
        "        assert_eq!(loader.matches(\"vanilla_sign_wood_kinds()\").count(), 1);",
        "        assert_eq!(loader.matches(\"wood.name()\").count(), 1);",
        "        assert!(!native.contains(\"HANGING_SIGN_BACKGROUNDS\"));",
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
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "#[cfg(test)]",
        "mod generated_26_1_tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
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
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_map_decoration_catalog() {",
        "        let renderer = include_str!(\"../../../bbb-renderer/src/item_models/map.rs\");",
        "        let lookup = renderer.split(\"pub fn item_frame_map_decoration_type\").nth(1).and_then(|source| source.split(\"/// Bakes vanilla\").next()).expect(\"map decoration lookup\");",
        "        assert_eq!(lookup.matches(\"vanilla_map_decoration_type(type_id)\").count(), 1);",
        "        let bake = renderer.split(\"fn bake_map_decoration_surface(\").nth(1).and_then(|source| source.split(\"/// Bakes one item-frame-visible vanilla `MapRenderer` decoration name submit.\").next()).expect(\"map decoration surface baker\");",
        "        assert_eq!(bake.matches(\"item_frame_map_decoration_type(type_id)?\").count(), 1);",
        "        assert!(bake.contains(\"decoration_type.render_on_frame\"));",
        "        assert!(bake.contains(\"decoration_type.sprite_id\"));",
        "        let fixture = renderer.split(\"fn generated_map_decoration_catalog_drives_all_surface_metadata\").nth(1).and_then(|source| source.split_once(\"\\n    }\")).map(|(body, _)| body).expect(\"map decoration full-catalog fixture\");",
        "        assert_eq!(fixture.matches(\"vanilla_map_decoration_types\").count(), 2);",
        "        assert!(fixture.contains(\"entry.render_on_frame\"));",
        "        assert_eq!(fixture.matches(\"entry.sprite_id\").count(), 2);",
        "        for entry in VANILLA_MAP_DECORATION_TYPES {",
        "            assert!(!fixture.contains(entry.resource_id), \"fixture restored resource literal {}\", entry.resource_id);",
        "            assert!(!fixture.contains(entry.sprite_id), \"fixture restored sprite literal {}\", entry.sprite_id);",
        "            assert!(!lookup.contains(entry.resource_id), \"lookup restored resource literal {}\", entry.resource_id);",
        "            assert!(!bake.contains(entry.sprite_id), \"baker restored sprite literal {}\", entry.sprite_id);",
        "        }",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/map_decorations/generated_26_1.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"const VANILLA_MAP_DECORATION_TYPES\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel map decoration catalog owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_registry_data_types(
    worldgen: list[RegistryDataCatalogEntry],
    dimension: list[RegistryDataCatalogEntry],
    synchronized: list[RegistryDataCatalogEntry],
    built_in: list[BuiltInRegistryCatalogEntry],
    fluid_identities: list[BootstrapEntry],
    chat_type_identities: list[BootstrapEntry],
) -> str:
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
    append_catalog("VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES", synchronized)
    lines.extend((
        "pub fn synchronized_registry_data_type(resource_id: &str) -> Option<&'static RegistryDataType> {",
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
        "pub fn built_in_registry_type(resource_id: &str) -> Option<&'static BuiltInRegistryType> {",
        "    VANILLA_BUILT_IN_REGISTRY_TYPES",
        "        .iter()",
        "        .find(|entry| entry.resource_id == resource_id)",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
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
        "    fn production_sources_do_not_redeclare_fixed_registry_identities() {",
        "        const FLUID_IDS: &[&str] = &[",
        *(f"            {rust_string(entry.resource_id)}," for entry in fluid_identities),
        "        ];",
        "        const CHAT_TYPE_IDS: &[&str] = &[",
        *(f"            {rust_string(entry.resource_id)}," for entry in chat_type_identities),
        "        ];",
        "        fn visit(path: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(path).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() {",
        "                    if path.file_name().is_some_and(|name| name == \"target\" || name == \".git\") { continue; }",
        "                    visit(&path, violations);",
        "                } else if path.extension().is_some_and(|extension| extension == \"rs\")",
        "                    && !path.ends_with(\"bbb-protocol/src/registry_data_types.rs\")",
        "                {",
        "                    let source = fs::read_to_string(&path).unwrap();",
        "                    let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                    if production.contains(\"const VANILLA_FLUID_RESOURCE_IDS\")",
        "                        || production.contains(\"const VANILLA_CHAT_TYPE_RESOURCE_IDS\")",
        "                        || production.contains(\"fn vanilla_chat_type_resource_id(\")",
        "                        || production.contains(\"fn vanilla_chat_type_holder_id(\")",
        "                        || FLUID_IDS.iter().all(|resource_id| production.contains(resource_id))",
        "                        || CHAT_TYPE_IDS.iter().all(|resource_id| production.contains(resource_id))",
        "                    {",
        "                        violations.push(path.display().to_string());",
        "                    }",
        "                }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel fixed-registry identity owners: {violations:?}\");",
        "    }",
        "",
        "    #[test]",
        "    fn registry_data_catalogs_preserve_vanilla_lists() {",
        f"        assert_eq!(VANILLA_WORLDGEN_REGISTRY_DATA_TYPES.len(), {len(worldgen)});",
        f"        assert_eq!(VANILLA_DIMENSION_REGISTRY_DATA_TYPES.len(), {len(dimension)});",
        f"        assert_eq!(VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES.len(), {len(synchronized)});",
        f"        assert_eq!(VANILLA_BUILT_IN_REGISTRY_TYPES.len(), {len(built_in)});",
        "        for catalog in [",
        "            VANILLA_WORLDGEN_REGISTRY_DATA_TYPES.as_slice(),",
        "            VANILLA_DIMENSION_REGISTRY_DATA_TYPES.as_slice(),",
        "            VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES.as_slice(),",
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
        "        assert_eq!(VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES[0].codec, \"Biome.NETWORK_CODEC\");",
        "        let timeline = VANILLA_WORLDGEN_REGISTRY_DATA_TYPES.iter().find(|entry| entry.constant_name == \"TIMELINE\").unwrap();",
        "        assert_eq!(timeline.validator, Some(\"Timeline::validateRegistry\"));",
        "        let wolf = synchronized_registry_data_type(\"minecraft:wolf_variant\").unwrap();",
        "        assert_eq!(wolf.validator, Some(\"RegistryValidator.nonEmpty ()\"));",
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
        "    use std::{fs, path::Path};",
        "",
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
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_overworld_day_visual_tracks() {",
        "        const OWNED_NAMES: &[&str] = &[",
        '            "VANILLA_OVERWORLD_DAY_PERIOD_TICKS",',
        '            "VANILLA_LIGHTMAP_DAY_PERIOD_TICKS",',
        '            "VANILLA_TIMELINE_NIGHT_SKY_LIGHT_COLOR",',
        *(
            f'            "{constant_names[track.attribute_name]}",'
            for track in entry.tracks
        ),
        "        ];",
        "        fn visit(path: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(path).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() {",
        "                    if path.file_name().is_some_and(|name| name == \"target\") { continue; }",
        "                    visit(&path, violations);",
        "                } else if path.extension().is_some_and(|extension| extension == \"rs\")",
        "                    && !path.ends_with(\"runtime/overworld_day_timeline.rs\")",
        "                    && !path.ends_with(\"tests.rs\")",
        "                    && !path.to_string_lossy().contains(\"/tests/\")",
        "                {",
        "                    let source = fs::read_to_string(&path).unwrap();",
        "                    let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                    if OWNED_NAMES.iter().any(|name| production.contains(&format!(\"const {name}\"))) {",
        "                        violations.push(path.display().to_string());",
        "                    }",
        "                }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(&Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"src\"), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel overworld-day visual track owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_registry_holder_consumers(
    entries: list[RegistryHolderConsumerEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 concrete Holder/HolderSet/TagKey codec factory calls.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct RegistryHolderConsumer {",
        "    pub source_path: &'static str,",
        "    pub line: u32,",
        "    pub column: u32,",
        "    pub owner: &'static str,",
        "    pub factory: &'static str,",
        "    pub kind: &'static str,",
        "    pub registry_expression: &'static str,",
        "    pub resource_id: Option<&'static str>,",
        "}",
        "",
        f"pub const VANILLA_REGISTRY_HOLDER_CONSUMERS: [RegistryHolderConsumer; {len(entries)}] = [",
    ]
    for entry in entries:
        lines.extend((
            "    RegistryHolderConsumer {",
            f"        source_path: {rust_string(entry.source_path)},",
            f"        line: {entry.line},",
            f"        column: {entry.column},",
            f"        owner: {rust_string(entry.owner)},",
            f"        factory: {rust_string(entry.factory)},",
            f"        kind: {rust_string(entry.kind)},",
            f"        registry_expression: {rust_string(entry.registry_expression)},",
            f"        resource_id: {'Some(' + rust_string(entry.resource_id) + ')' if entry.resource_id else 'None'},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn registry_holder_consumer_count(resource_id: &str) -> usize {",
        "    VANILLA_REGISTRY_HOLDER_CONSUMERS",
        "        .iter()",
        "        .filter(|entry| entry.resource_id == Some(resource_id))",
        "        .count()",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::collections::HashSet;",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn holder_consumer_inventory_preserves_unique_cst_call_sites() {",
        f"        assert_eq!(VANILLA_REGISTRY_HOLDER_CONSUMERS.len(), {len(entries)});",
        "        let mut call_sites = HashSet::new();",
        "        for entry in &VANILLA_REGISTRY_HOLDER_CONSUMERS {",
        "            assert!(entry.source_path.starts_with(\"net/minecraft/\"));",
        "            assert!(entry.source_path.ends_with(\".java\"));",
        "            assert!(entry.line > 0);",
        "            assert!(entry.column > 0);",
        "            assert!(!entry.owner.is_empty());",
        "            assert!(!entry.registry_expression.is_empty());",
        "            assert!(call_sites.insert((entry.source_path, entry.line, entry.column, entry.factory)));",
        "            if let Some(resource_id) = entry.resource_id {",
        "                assert!(resource_id.starts_with(\"minecraft:\"));",
        "            }",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn holder_consumer_inventory_keeps_reference_inline_set_tag_and_generic_shapes() {",
        "        assert!(VANILLA_REGISTRY_HOLDER_CONSUMERS.iter().any(|entry|",
        "            entry.source_path.ends_with(\"sounds/SoundEvent.java\")",
        "                && entry.factory == \"ByteBufCodecs.holder\"",
        "                && entry.kind == \"network_holder_or_inline\"",
        "                && entry.resource_id == Some(\"minecraft:sound_event\")));",
        "        assert!(VANILLA_REGISTRY_HOLDER_CONSUMERS.iter().any(|entry|",
        "            entry.factory == \"ByteBufCodecs.holderSet\"",
        "                && entry.kind == \"network_holder_set_or_tag\"));",
        "        assert!(VANILLA_REGISTRY_HOLDER_CONSUMERS.iter().any(|entry|",
        "            entry.factory == \"TagKey.hashedCodec\"",
        "                && entry.kind == \"persistent_hashed_tag\"));",
        "        assert!(VANILLA_REGISTRY_HOLDER_CONSUMERS.iter().any(|entry|",
        "            entry.source_path.ends_with(\"core/RegistryCodecs.java\")",
        "                && entry.resource_id.is_none()));",
        "        assert!(registry_holder_consumer_count(\"minecraft:item\") > 0);",
        "        assert!(registry_holder_consumer_count(\"minecraft:worldgen/biome\") > 0);",
        "        assert_eq!(registry_holder_consumer_count(\"example:custom\"), 0);",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def registry_holder_consumer_call_site(entry: RegistryHolderConsumerEntry) -> str:
    return f"{entry.source_path}:{entry.line}:{entry.column}:{entry.factory}"


def load_registry_holder_coverage_families(path: Path) -> list[dict[str, object]]:
    def strict_object(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError(f"{path}: duplicate JSON key {key!r}")
            value[key] = item
        return value

    value = json.loads(path.read_text(), object_pairs_hook=strict_object)
    if value.get("schema_version") != 1 or not isinstance(value.get("families"), list):
        raise ValueError(f"{path}: invalid registry holder coverage schema")
    return value["families"]


def validate_registry_holder_coverage_locator(
    locator: str,
    context: str,
    source_root: Path,
) -> None:
    if "#" not in locator:
        raise ValueError(f"{context}: malformed locator {locator!r}")
    path_text, symbol = locator.split("#", 1)
    if not symbol:
        raise ValueError(f"{context}: locator lacks symbol {locator!r}")
    if path_text.startswith("vanilla:"):
        root = source_root
        path_text = path_text.removeprefix("vanilla:")
    else:
        root = REPO_ROOT
    path = Path(path_text)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"{context}: locator path must be relative {locator!r}")
    source_path = root / path
    if not source_path.is_file():
        raise ValueError(f"{context}: locator file does not exist: {locator}")
    if symbol not in source_path.read_text():
        raise ValueError(f"{context}: locator symbol not found: {locator}")


def registry_holder_family_matches(
    entry: RegistryHolderConsumerEntry,
    selector: dict[str, object],
) -> bool:
    allowed_keys = {
        "kind",
        "resource_ids",
        "exclude_resource_ids",
        "source_paths",
        "exclude_source_paths",
    }
    unknown_keys = set(selector) - allowed_keys
    if unknown_keys:
        raise ValueError(f"unknown registry holder coverage selector keys: {sorted(unknown_keys)}")
    kind = selector.get("kind")
    if kind is not None and entry.kind != kind:
        return False
    resource_ids = selector.get("resource_ids")
    if resource_ids is not None:
        if not isinstance(resource_ids, list) or not all(
            isinstance(item, str) and item for item in resource_ids
        ):
            raise ValueError("registry holder coverage resource_ids must be strings")
        if entry.resource_id not in resource_ids:
            return False
    exclude_resource_ids = selector.get("exclude_resource_ids", [])
    if not isinstance(exclude_resource_ids, list) or not all(
        isinstance(item, str) and item for item in exclude_resource_ids
    ):
        raise ValueError("registry holder coverage exclude_resource_ids must be strings")
    if entry.resource_id in exclude_resource_ids:
        return False
    source_paths = selector.get("source_paths")
    if source_paths is not None:
        if not isinstance(source_paths, list) or not all(
            isinstance(item, str) and item for item in source_paths
        ):
            raise ValueError("registry holder coverage source_paths must be strings")
        if entry.source_path not in source_paths:
            return False
    exclude_source_paths = selector.get("exclude_source_paths", [])
    if not isinstance(exclude_source_paths, list) or not all(
        isinstance(item, str) and item for item in exclude_source_paths
    ):
        raise ValueError("registry holder coverage exclude_source_paths must be strings")
    return entry.source_path not in exclude_source_paths


def render_registry_holder_coverage_ledger(
    entries: list[RegistryHolderConsumerEntry],
    families: list[dict[str, object]],
    source_root: Path,
) -> str:
    allowed_statuses = {"complete", "partial", "deferred", "not-applicable"}
    normalized_families: list[dict[str, object]] = []
    family_ids: set[str] = set()
    for family in families:
        if not isinstance(family, dict):
            raise ValueError("registry holder coverage family must be an object")
        unknown_keys = set(family) - {
            "id",
            "selector",
            "status",
            "consumer",
            "evidence",
            "decision",
        }
        if unknown_keys:
            raise ValueError(f"registry holder coverage family has unknown keys {sorted(unknown_keys)}")
        family_id = family.get("id")
        selector = family.get("selector")
        status = family.get("status")
        consumer = family.get("consumer")
        evidence = family.get("evidence")
        decision = family.get("decision")
        if not isinstance(family_id, str) or not family_id or family_id in family_ids:
            raise ValueError(f"invalid or duplicate registry holder coverage family id {family_id!r}")
        family_ids.add(family_id)
        if not isinstance(selector, dict):
            raise ValueError(f"{family_id}: selector must be an object")
        if status not in allowed_statuses:
            raise ValueError(f"{family_id}: invalid status {status!r}")
        if consumer is not None and not isinstance(consumer, str):
            raise ValueError(f"{family_id}: consumer must be a locator or null")
        if not isinstance(evidence, list) or not evidence or not all(
            isinstance(item, str) and item for item in evidence
        ):
            raise ValueError(f"{family_id}: evidence must contain non-empty locators")
        if not isinstance(decision, str) or not decision:
            raise ValueError(f"{family_id}: decision must be a non-empty string")
        if status in {"complete", "partial"} and consumer is None:
            raise ValueError(f"{family_id}: {status} family lacks a consumer")
        if status == "partial" and not decision.startswith("docs/unsupported-features.md#"):
            raise ValueError(f"{family_id}: partial decision must point to the gap ledger")
        if status == "deferred" and not decision.startswith("restart: "):
            raise ValueError(f"{family_id}: deferred decision lacks a restart condition")
        if status == "not-applicable" and consumer is not None:
            raise ValueError(f"{family_id}: not-applicable family must not name a consumer")
        if consumer is not None:
            validate_registry_holder_coverage_locator(consumer, family_id, source_root)
        for locator in evidence:
            validate_registry_holder_coverage_locator(locator, family_id, source_root)
        normalized_families.append(family)

    ledger_entries = []
    used_families: set[str] = set()
    for entry in entries:
        matches = [
            family
            for family in normalized_families
            if registry_holder_family_matches(entry, family["selector"])
        ]
        call_site = registry_holder_consumer_call_site(entry)
        if len(matches) != 1:
            raise ValueError(
                f"{call_site}: expected one registry holder coverage family, got "
                f"{[family['id'] for family in matches]}"
            )
        family = matches[0]
        used_families.add(family["id"])
        ledger_entries.append(
            {
                "call_site": call_site,
                "source_path": entry.source_path,
                "line": entry.line,
                "column": entry.column,
                "owner": entry.owner,
                "factory": entry.factory,
                "kind": entry.kind,
                "registry_expression": entry.registry_expression,
                "resource_id": entry.resource_id,
                "family": family["id"],
                "status": family["status"],
                "consumer": family["consumer"],
                "evidence": family["evidence"],
                "decision": family["decision"],
            }
        )
    unused_families = family_ids - used_families
    if unused_families:
        raise ValueError(f"unused registry holder coverage families: {sorted(unused_families)}")
    value = {
        "schema_version": 1,
        "vanilla_version": "26.1",
        "generated_by": "scripts/generate-bootstrap-catalog.py",
        "update_command": "scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "check_command": "scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "coverage_families": "scripts/data/registry-holder-consumer-coverage-26.1.json",
        "allowed_statuses": sorted(allowed_statuses),
        "scope": "All vanilla Holder, HolderSet, and TagKey network stream-codec and persistent Codec factory call sites.",
        "entries": ledger_entries,
    }
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            '        assert_eq!(vanilla_potion(42).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_WIND_CHARGED_ID);',
            '        assert_eq!(vanilla_potion(43).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_WEAVING_ID);',
            '        assert_eq!(vanilla_potion(44).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_OOZING_ID);',
            '        assert_eq!(vanilla_potion(45).unwrap().effects, &[VanillaPotionEffect { effect_id: VANILLA_MOB_EFFECT_INFESTED_ID, duration: 3600, amplifier: 0 }]);',
            "        assert_eq!(vanilla_potion(-1), None);",
            "        assert_eq!(vanilla_potion(46), None);",
            '        assert_eq!(vanilla_potion_for_resource_id("minecraft:missing"), None);',
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_potion_catalog_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/potion_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"const VANILLA_POTION_KEYS\") || source.contains(\"const VANILLA_POTION_EFFECTS_BY_ID\") || source.contains(\"fn vanilla_potion_resource_id(\") || source.contains(\"fn vanilla_potion_effects(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"..\").as_path(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel potion catalog owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_enchantment_catalog_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/enchantments/generated_26_1.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum VanillaEnchantmentIdentity\") || source.contains(\"struct VanillaEnchantment {\") || source.contains(\"const VANILLA_ENCHANTMENTS\") || source.contains(\"const VANILLA_ENCHANTMENT_TOOLTIP_ORDER\") || source.contains(\"fn vanilla_enchantment_resource_id(\") || source.contains(\"fn vanilla_enchantment_max_level(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel enchantment catalog owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_painting_variant_catalog_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/bbb-world/src/painting_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"PAINTING_VARIANT_SIZES\") || source.contains(\"struct VanillaPaintingVariant\") || source.contains(\"const VANILLA_PAINTING_VARIANTS\") || source.contains(\"fn vanilla_painting_variant_resource_id(\") || source.contains(\"fn vanilla_painting_variant_for_resource_id(\") || source.contains(\"fn vanilla_painting_variant_by_resource_id(\") || source.contains(\"fn vanilla_painting_variant(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel painting-variant catalog owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


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
        "use super::catalog::EntityModelTextureRef;",
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
                f"pub(in crate::entity_models) const {texture_constant(entry)}: EntityModelTextureRef =",
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
            "    pub(in crate::entity_models) fn texture_ref(self) -> EntityModelTextureRef {",
            "        VANILLA_DECORATED_POT_PATTERNS[self as usize].texture_ref",
            "    }",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_decorated_pot_pattern_facts() {",
            "        let fixture = include_str!(\"tests/decorated_pot.rs\").split(\"fn decorated_pot_model_keys_and_texture_refs_match_vanilla_selection\").nth(1).and_then(|source| source.split(\"#[test]\").next()).expect(\"decorated-pot texture fixture\");",
            "        assert!(!fixture.contains(\"DECORATED_POT_SIDE_TEXTURE_REF.size\"));",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/decorated_pot_patterns.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum DecoratedPotPattern\") || source.contains(\"fn decorated_pot_pattern_for_sherd_item\") || source.contains(\"const DECORATED_POT_ANGLER_TEXTURE_REF\") || source.contains(\"const DECORATED_POT_SIDE_TEXTURE_REF\") || source.contains(\"fn decorated_pot_side_sheet_texture_ref\") || source.contains(\"_pottery_sherd\\\" => Some(DecoratedPotPattern::\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel decorated-pot pattern owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_banner_pattern_identity_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/banner_pattern_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum BannerPatternKind\") || source.contains(\"struct VanillaBannerPattern {\") || source.contains(\"const VANILLA_BANNER_PATTERNS\") || source.contains(\"fn vanilla_banner_pattern_resource_id(\") || source.contains(\"fn vanilla_banner_pattern_translation_key(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel banner-pattern identity owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_banner_pattern_facts() {",
            "        let fixture = include_str!(\"tests/banner.rs\").split(\"fn banner_model_keys_and_texture_refs_match_vanilla_selection\").nth(1).and_then(|source| source.split(\"#[test]\").next()).expect(\"banner texture fixture\");",
            "        assert!(!fixture.contains(\"BANNER_PATTERN_BASE_TEXTURE_REF.size\"));",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/banner_patterns.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"fn banner_pattern_texture_ref(\") || source.contains(\"const BANNER_PATTERN_BASE_TEXTURE_REF\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel banner-pattern catalog owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_villager_profession_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/villager_model_professions.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum VillagerModelProfession\") || source.contains(\"fn villager_profession_from_entry_id(\") || source.contains(\"fn villager_profession_from_vanilla_registry_id(\") || source.contains(\"fn villager_profession_texture_index(\") || source.contains(\"\\\"minecraft:armorer\\\" => Some(VillagerModelProfession::\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel villager-profession catalog owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_villager_type_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/villager_model_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum VillagerModelType\") || source.contains(\"fn villager_type_from_entry_id(\") || source.contains(\"fn villager_type_from_vanilla_registry_id(\") || source.contains(\"fn villager_type_index(\") || source.contains(\"fn vanilla_villager_type_resource_id(\") || source.contains(\"const VANILLA_VILLAGER_TYPE_KEYS\") || source.contains(\"\\\"minecraft:desert\\\" => Some(VillagerModelType::\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel villager-type catalog owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_cat_variant_catalog_facts() {",
            "        let native = include_str!(\"../../bbb-native/src/entity_assets.rs\");",
            "        let fixture = native.split(\"fn loads_vanilla_entity_textures_from_resource_stack\").nth(1).and_then(|source| source.split(\"    #[test]\").next()).expect(\"native entity texture loader fixture\");",
            "        assert_eq!(fixture.matches(\"entity_model_texture_refs()\").count(), 3);",
            "        for texture in VANILLA_CAT_VARIANT_TEXTURE_REFS {",
            "            assert!(!fixture.contains(texture.path), \"native fixture restored cat asset literal {}\", texture.path);",
            "        }",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/cat_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum CatModelVariant\") || source.contains(\"fn cat_variant_from_entry_id(\") || source.contains(\"fn cat_variant_from_vanilla_registry_id(\") || source.contains(\"fn vanilla_cat_variant_resource_id(\") || source.contains(\"\\\"minecraft:tabby\\\" => Some(CatModelVariant::\") || source.contains(\"const FELINE_CAT_BLACK_TEXTURE_REF\") || source.contains(\"CatModelVariant::Tabby, false) =>\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel cat-variant catalog owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_wolf_variant_catalog_facts() {",
            "        let native = include_str!(\"../../bbb-native/src/entity_assets.rs\");",
            "        let fixture = native.split(\"fn loads_vanilla_entity_textures_from_resource_stack\").nth(1).and_then(|source| source.split(\"    #[test]\").next()).expect(\"native entity texture loader fixture\");",
            "        assert_eq!(fixture.matches(\"entity_model_texture_refs()\").count(), 3);",
            "        for texture in VANILLA_WOLF_VARIANT_TEXTURE_REFS {",
            "            assert!(!fixture.contains(texture.path), \"native fixture restored wolf asset literal {}\", texture.path);",
            "        }",
            "        let renderer_tests = include_str!(\"../../bbb-renderer/src/entity_models/tests/wolf.rs\");",
            "        let texture_fixture = renderer_tests.split(\"fn wolf_texture_refs_match_vanilla_renderer_pale_variant_assets\").nth(1).and_then(|source| source.split(\"\\n#[test]\").next()).expect(\"wolf texture-ref fixture\");",
            "        assert!(texture_fixture.contains(\"wolf_entity_texture_refs()\"));",
            "        assert!(!texture_fixture.contains(\"size: [\"), \"wolf texture fixture restored hand-authored PNG dimensions\");",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/wolf_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum WolfModelVariant\") || source.contains(\"fn wolf_variant_from_entry_id(\") || source.contains(\"fn wolf_variant_from_vanilla_registry_id(\") || source.contains(\"fn vanilla_wolf_variant_resource_id(\") || source.contains(\"\\\"minecraft:pale\\\" => Some(WolfModelVariant::\") || source.contains(\"const WOLF_SPOTTED_TEXTURE_REF\") || source.contains(\"WolfModelVariant::Spotted => [\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel wolf-variant catalog owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_temperature_animal_variant_catalog_facts() {",
            "        let native = include_str!(\"../../bbb-native/src/entity_assets.rs\");",
            "        let loader = native.split(\"fn load_entity_model_texture_images\").nth(1).and_then(|source| source.split(\"fn load_entity_model_texture_image\").next()).expect(\"native entity texture loader\");",
            "        assert_eq!(loader.matches(\"entity_model_texture_refs()\").count(), 1);",
            "        let fixture = native.split(\"fn loads_vanilla_entity_textures_from_resource_stack\").nth(1).and_then(|source| source.split(\"    #[test]\").next()).expect(\"native entity texture loader fixture\");",
            "        assert_eq!(fixture.matches(\"entity_model_texture_refs()\").count(), 3);",
            "        for texture in VANILLA_CHICKEN_VARIANT_TEXTURE_REFS.iter().chain(&VANILLA_COW_VARIANT_TEXTURE_REFS).chain(&VANILLA_PIG_VARIANT_TEXTURE_REFS).chain(&VANILLA_FROG_VARIANT_TEXTURE_REFS) {",
            "            assert!(!fixture.contains(texture.path), \"native fixture restored temperature-animal asset literal {}\", texture.path);",
            "        }",
            "        for (source, texture_test, atlas_test, refs_fn, asset_prefix) in [",
            "            (include_str!(\"../../bbb-renderer/src/entity_models/tests/chicken.rs\"), \"fn chicken_texture_refs_match_vanilla_variant_assets\", \"fn entity_texture_atlas_stitches_official_chicken_png_slots\", \"chicken_entity_texture_refs()\", \"textures/entity/chicken/\"),",
            "            (include_str!(\"../../bbb-renderer/src/entity_models/tests/cow.rs\"), \"fn cow_texture_refs_match_vanilla_renderers\", \"fn entity_texture_atlas_stitches_official_cow_png_slots\", \"cow_entity_texture_refs()\", \"textures/entity/cow/\"),",
            "            (include_str!(\"../../bbb-renderer/src/entity_models/tests/pig.rs\"), \"fn pig_texture_refs_match_vanilla_variant_assets\", \"fn entity_texture_atlas_stitches_official_pig_png_slots\", \"pig_entity_texture_refs()\", \"textures/entity/pig/\"),",
            "        ] {",
            "            let texture_fixture = source.split(texture_test).nth(1).and_then(|source| source.split(\"\\n#[test]\").next()).expect(\"temperature-animal texture-ref fixture\");",
            "            assert!(texture_fixture.contains(refs_fn));",
            "            assert!(!texture_fixture.contains(\"size: [\"), \"{texture_test} restored hand-authored PNG dimensions\");",
            "            let atlas_fixture = source.split(atlas_test).nth(1).and_then(|source| source.split(\"\\n#[test]\").next()).expect(\"temperature-animal atlas fixture\");",
            "            assert!(atlas_fixture.contains(\"assert_texture_atlas_layout_matches_refs\"));",
            "            assert!(atlas_fixture.contains(\"texture_atlas_entry_y\"));",
            "            assert!(!atlas_fixture.contains(asset_prefix), \"{atlas_test} restored hand-authored texture paths\");",
            "        }",
            "        let creeper_tests = include_str!(\"../../bbb-renderer/src/entity_models/tests/creeper.rs\");",
            "        let creeper_fixture = creeper_tests.split(\"fn creeper_texture_ref_matches_vanilla_renderer\").nth(1).and_then(|source| source.split(\"\\n#[test]\").next()).expect(\"creeper texture-ref fixture\");",
            "        assert!(creeper_fixture.contains(\"CHICKEN_TEMPERATE_TEXTURE_REF\"));",
            "        let chicken_assertion = creeper_fixture.split(\"EntityModelKind::Chicken\").nth(1).expect(\"creeper chicken texture assertion\");",
            "        assert!(!chicken_assertion.contains(\"size: [\"), \"creeper fixture restored hand-authored chicken PNG dimensions\");",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/temperature_animal_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"enum ChickenModelVariant\") || production.contains(\"enum CowModelVariant\") || production.contains(\"enum PigModelVariant\") || production.contains(\"enum FrogModelVariant\") || production.contains(\"fn chicken_variant_from_entry_id(\") || production.contains(\"fn cow_variant_from_entry_id(\") || production.contains(\"fn pig_variant_from_entry_id(\") || production.contains(\"fn frog_variant_from_entry_id(\") || production.contains(\"fn chicken_variant_from_vanilla_registry_id(\") || production.contains(\"fn cow_variant_from_vanilla_registry_id(\") || production.contains(\"fn pig_variant_from_vanilla_registry_id(\") || production.contains(\"fn frog_variant_from_vanilla_registry_id(\") || production.contains(\"fn vanilla_chicken_variant_resource_id(\") || production.contains(\"fn vanilla_cow_variant_resource_id(\") || production.contains(\"fn vanilla_pig_variant_resource_id(\") || production.contains(\"fn vanilla_frog_variant_resource_id(\") || production.contains(\"FrogModelVariant::Temperate => FROG_\") || production.contains(\"const CHICKEN_ENTITY_TEXTURE_REFS\") || production.contains(\"const COW_ENTITY_TEXTURE_REFS\") || production.contains(\"const PIG_ENTITY_TEXTURE_REFS\") || production.contains(\"textures/entity/chicken/chicken_temperate.png\") || production.contains(\"textures/entity/cow/cow_temperate.png\") || production.contains(\"textures/entity/pig/pig_temperate.png\") || production.contains(\"(ChickenModelVariant::Cold, false) =>\") || production.contains(\"CowModelVariant::Warm => (\") || production.contains(\"PigModelVariant::Cold => &COLD_PIG_BODY\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel temperature animal-variant catalog owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_zombie_nautilus_variant_catalog_facts() {",
            "        let native = include_str!(\"../../bbb-native/src/entity_assets.rs\");",
            "        let fixture = native.split(\"fn loads_vanilla_entity_textures_from_resource_stack\").nth(1).and_then(|source| source.split(\"    #[test]\").next()).expect(\"native entity texture loader fixture\");",
            "        assert_eq!(fixture.matches(\"entity_model_texture_refs()\").count(), 3);",
            "        for texture in VANILLA_ZOMBIE_NAUTILUS_VARIANT_TEXTURE_REFS {",
            "            assert!(!fixture.contains(texture.path), \"native fixture restored zombie-nautilus asset literal {}\", texture.path);",
            "        }",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/zombie_nautilus_variants.rs\") || display.ends_with(\"/bootstrap_catalog.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"enum ZombieNautilusModelVariant\") || production.contains(\"fn zombie_nautilus_variant_from_entry_id(\") || production.contains(\"fn zombie_nautilus_variant_from_vanilla_registry_id(\") || production.contains(\"fn zombie_nautilus_variant_is_coral(\") || production.contains(\"textures/entity/nautilus/zombie_nautilus.png\") || production.contains(\"matches!(variant, ZombieNautilusModelVariant::Warm)\") || production.contains(\"ZombieNautilusModelVariant::Warm =>\") || production.contains(\"variant: ZombieNautilusModelVariant::Temperate,\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel zombie-nautilus variant catalog owners: {violations:?}\");",
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
        "            #[cfg(test)]",
        "            pub(crate) fn builtin_variants() -> impl ExactSizeIterator<Item = (i32, Self)> {",
        "                $entries.iter().enumerate().map(|(registry_id, (_, variant, _, _, _, _))| (registry_id as i32, *variant))",
        "            }",
        "",
        "            pub(crate) const fn builtin_hurt_sound_event(self, baby: bool) -> &'static str {",
        "                let entry = $entries[self as usize];",
        "                if baby { entry.3 } else { entry.2 }",
        "            }",
        "",
        "            pub(crate) const fn builtin_death_sound_event(self, baby: bool) -> &'static str {",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_animal_sound_variant_catalog_facts() {",
            "        let audio = include_str!(\"../client/audio.rs\");",
            "        let fixture = audio.split(\"fn built_in_animal_sound_variants_drive_adult_and_baby_consumed_sounds\").nth(1).and_then(|source| source.split(\"    #[test]\").next()).expect(\"full animal consumed-sound fixture\");",
            f"        assert_eq!(fixture.matches(\"assert_family!\").count(), {len(expected_catalogs)});",
            "        assert_eq!(fixture.matches(\"builtin_variants()\").count(), 1);",
            "        assert_eq!(fixture.matches(\"builtin_hurt_sound_event(baby)\").count(), 1);",
            "        assert_eq!(fixture.matches(\"builtin_death_sound_event(baby)\").count(), 1);",
        )
    )
    for family, catalog_name, type_name in expected_catalogs:
        const_name = catalog_name.removesuffix("_KEYS") + "_IDENTITIES"
        lines.extend(
            (
                f"        assert_eq!(fixture.matches(\"{type_name}\").count(), 1);",
                f"        for (_, variant, adult_hurt, baby_hurt, adult_death, baby_death) in {const_name} {{",
                "            assert!(!fixture.contains(adult_hurt), \"fixture restored adult hurt-sound literal {adult_hurt}\");",
                "            assert!(!fixture.contains(baby_hurt), \"fixture restored baby hurt-sound literal {baby_hurt}\");",
                "            assert!(!fixture.contains(adult_death), \"fixture restored adult death-sound literal {adult_death}\");",
                "            assert!(!fixture.contains(baby_death), \"fixture restored baby death-sound literal {baby_death}\");",
                f"            assert!(!fixture.contains(&format!(\"{type_name}::{{variant:?}}\")), \"fixture restored representative {type_name} variant {{variant:?}}\");",
                "        }",
            )
        )
    lines.extend(
        (
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/sound_variants.rs\") || display.ends_with(\"/sound_event_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"enum CatSoundVariantIdentity\") || production.contains(\"enum ChickenSoundVariantIdentity\") || production.contains(\"enum CowSoundVariantIdentity\") || production.contains(\"enum PigSoundVariantIdentity\") || production.contains(\"enum WolfSoundVariantIdentity\") || production.contains(\"fn vanilla_cat_sound_variant_resource_id(\") || production.contains(\"fn vanilla_chicken_sound_variant_resource_id(\") || production.contains(\"fn vanilla_cow_sound_variant_resource_id(\") || production.contains(\"fn vanilla_pig_sound_variant_resource_id(\") || production.contains(\"fn vanilla_wolf_sound_variant_resource_id(\") || production.contains(\"minecraft:entity.cat_royal.death\") || production.contains(\"minecraft:entity.chicken_picky.death\") || production.contains(\"minecraft:entity.cow_moody.death\") || production.contains(\"minecraft:entity.pig_big.death\") || production.contains(\"minecraft:entity.wolf_cute.death\") || production.contains(\"minecraft:entity.baby_cat.death\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel animal sound-variant catalog owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


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
        "    use std::{collections::HashSet, fs, path::Path};",
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
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_instrument_identity_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/instruments.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                if source.contains(\"enum VanillaInstrumentIdentity\") || source.contains(\"const VANILLA_INSTRUMENT_KEYS\") || source.contains(\"fn vanilla_instrument_resource_id(\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel instrument identity owners: {violations:?}\");",
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
        "    use std::{collections::HashSet, fs, path::Path};",
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
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_trim_pattern_identity_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/trim_patterns.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                if source.contains(\"enum VanillaTrimPatternIdentity\") || source.contains(\"const VANILLA_TRIM_PATTERN_KEYS\") || source.contains(\"fn vanilla_trim_pattern_resource_id(\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel trim-pattern identity owners: {violations:?}\");",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_trim_material_catalog_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/trim_materials.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum VanillaTrimMaterialIdentity\") || source.contains(\"struct VanillaTrimMaterial {\") || source.contains(\"const VANILLA_TRIM_MATERIALS\") || source.contains(\"fn vanilla_trim_material_resource_id(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel trim-material catalog owners: {violations:?}\");",
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
    registry_holder_coverage_overrides: Path,
) -> tuple[str, ...]:
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
    mob_effects = mob_effect_entries(parse_java(source_root, MOB_EFFECTS_SOURCE))
    potions = potion_entries(
        parse_java(source_root, POTIONS_SOURCE),
        {entry.constant_name for entry in mob_effects},
    )
    tooltip_order = enchantment_tooltip_order(
        load_tag_resources(source_root, ENCHANTMENT_TAG_SOURCE_DIR), enchantments
    )
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
        parse_java(source_root, MAP_DECORATIONS_SOURCE)
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
    registry_holder_consumers = registry_holder_consumer_catalog(
        registry_holder_consumer_sources(source_root),
        registries,
        built_in_registry_data,
    )
    registry_holder_coverage_families = load_registry_holder_coverage_families(
        registry_holder_coverage_overrides
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
        format_rust(render_potion_catalog(potions)),
        format_rust(render_painting_variants(paintings)),
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
        format_rust(render_registry_holder_consumers(registry_holder_consumers)),
        render_registry_holder_coverage_ledger(
            registry_holder_consumers,
            registry_holder_coverage_families,
            source_root,
        ),
        format_rust(render_sign_wood_types(sign_wood_types)),
        format_rust(render_overworld_day_timeline(overworld_day_timeline)),
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


def main() -> int:
    args = parse_args()
    (
        generated,
        generated_potions,
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
        generated_animal_sound_variants,
        generated_instruments,
        generated_trim_patterns,
        generated_trim_materials,
        generated_map_decorations,
        generated_registry_data_types,
        generated_registry_holder_consumers,
        generated_registry_holder_coverage,
        generated_sign_wood_types,
        generated_overworld_day_timeline,
    ) = generate(args.source_root, args.registry_holder_coverage_overrides)
    if args.check:
        matches = check_generated(args.output, generated)
        matches &= check_generated(args.potion_catalog_output, generated_potions)
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
            args.registry_holder_consumers_output,
            generated_registry_holder_consumers,
        )
        matches &= check_generated(
            args.registry_holder_coverage_output,
            generated_registry_holder_coverage,
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
    args.potion_catalog_output.parent.mkdir(parents=True, exist_ok=True)
    args.potion_catalog_output.write_text(generated_potions)
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
    args.registry_holder_consumers_output.parent.mkdir(parents=True, exist_ok=True)
    args.registry_holder_consumers_output.write_text(
        generated_registry_holder_consumers
    )
    args.registry_holder_coverage_output.parent.mkdir(parents=True, exist_ok=True)
    args.registry_holder_coverage_output.write_text(
        generated_registry_holder_coverage
    )
    args.sign_wood_types_output.parent.mkdir(parents=True, exist_ok=True)
    args.sign_wood_types_output.write_text(generated_sign_wood_types)
    args.overworld_day_timeline_output.parent.mkdir(parents=True, exist_ok=True)
    args.overworld_day_timeline_output.write_text(
        generated_overworld_day_timeline
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
