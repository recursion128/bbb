#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 bootstrap resource-order catalogs."""

from __future__ import annotations

import argparse
import difflib
import subprocess
import sys
from pathlib import Path

from bootstrap_catalog_extractor import (
    BannerPatternEntry,
    BootstrapEntry,
    DecoratedPotPatternEntry,
    EnchantmentEntry,
    PaintingVariantEntry,
    PotionEntry,
    TrimMaterialEntry,
    banner_pattern_entries,
    bootstrap_resource_key_entries,
    decorated_pot_pattern_entries,
    enchantment_entries,
    enchantment_tooltip_order,
    identifier_constant_paths,
    painting_variant_entries,
    potion_entries,
    trim_material_entries,
)
from gameplay_id_catalog_extractor import mob_effect_entries
from vanilla_java_source import parse_source
from vanilla_tag_source import load_tag_resources


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-item-model/src/item_runtime/bootstrap_catalog.rs"
)
DEFAULT_POTION_CATALOG_OUTPUT = (
    REPO_ROOT / "crates/bbb-item-model/src/item_runtime/potion_catalog.rs"
)
DEFAULT_PAINTING_VARIANTS_OUTPUT = REPO_ROOT / "crates/bbb-world/src/painting_variants.rs"
DEFAULT_DECORATED_POT_PATTERNS_OUTPUT = (
    REPO_ROOT / "crates/bbb-renderer/src/entity_models/decorated_pot_patterns.rs"
)

TRIM_MATERIALS_SOURCE = Path(
    "net/minecraft/world/item/equipment/trim/TrimMaterials.java"
)
TRIM_PATTERNS_SOURCE = Path(
    "net/minecraft/world/item/equipment/trim/TrimPatterns.java"
)
POTIONS_SOURCE = Path("net/minecraft/world/item/alchemy/Potions.java")
MOB_EFFECTS_SOURCE = Path("net/minecraft/world/effect/MobEffects.java")
VILLAGER_TYPES_SOURCE = Path(
    "net/minecraft/world/entity/npc/villager/VillagerType.java"
)
WOLF_SOUND_VARIANTS_SOURCE = Path(
    "net/minecraft/world/entity/animal/wolf/WolfSoundVariants.java"
)
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

RESOURCE_LOOKUPS = (
    ("vanilla_cat_variant_resource_id", "VANILLA_CAT_VARIANT_KEYS"),
    ("vanilla_cat_sound_variant_resource_id", "VANILLA_CAT_SOUND_VARIANT_KEYS"),
    ("vanilla_chicken_variant_resource_id", "VANILLA_CHICKEN_VARIANT_KEYS"),
    ("vanilla_chicken_sound_variant_resource_id", "VANILLA_CHICKEN_SOUND_VARIANT_KEYS"),
    ("vanilla_cow_variant_resource_id", "VANILLA_COW_VARIANT_KEYS"),
    ("vanilla_cow_sound_variant_resource_id", "VANILLA_COW_SOUND_VARIANT_KEYS"),
    ("vanilla_pig_variant_resource_id", "VANILLA_PIG_VARIANT_KEYS"),
    ("vanilla_pig_sound_variant_resource_id", "VANILLA_PIG_SOUND_VARIANT_KEYS"),
    ("vanilla_frog_variant_resource_id", "VANILLA_FROG_VARIANT_KEYS"),
    (
        "vanilla_zombie_nautilus_variant_resource_id",
        "VANILLA_ZOMBIE_NAUTILUS_VARIANT_KEYS",
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
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def parse_java(source_root: Path, relative_path: Path):
    return parse_source(relative_path.as_posix(), (source_root / relative_path).read_bytes())


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def rust_variant(constant_name: str) -> str:
    return "".join(part.title() for part in constant_name.split("_"))


def render_array(name: str, entries: list[BootstrapEntry]) -> list[str]:
    lines = [f"pub(super) const {name}: &[&str] = &["]
    lines.extend(f"    {rust_string(entry.resource_id)}," for entry in entries)
    lines.extend(("];", ""))
    return lines


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
        "use bbb_protocol::mob_effect_types::{",
        *(
            f"    VANILLA_MOB_EFFECT_{constant_name}_ID,"
            for constant_name in effect_constants
        ),
        "};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub(super) struct VanillaPotionEffect {",
        "    pub(super) effect_id: i32,",
        "    pub(super) duration: i32,",
        "    pub(super) amplifier: i32,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub(super) struct VanillaPotion {",
        "    pub(super) resource_id: &'static str,",
        "    pub(super) effects: &'static [VanillaPotionEffect],",
        "}",
        "",
        "pub(super) const VANILLA_POTIONS: &[VanillaPotion] = &[",
    ]
    for entry in entries:
        lines.extend(
            (
                "    VanillaPotion {",
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
            "pub(super) fn vanilla_potion(id: i32) -> Option<&'static VanillaPotion> {",
            "    VANILLA_POTIONS.get(usize::try_from(id).ok()?)",
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
            "        for entry in VANILLA_POTIONS {",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            f"        assert_eq!(VANILLA_POTIONS.iter().map(|entry| entry.effects.len()).sum::<usize>(), {sum(len(entry.effects) for entry in entries)});",
            "    }",
            "",
            "    #[test]",
            "    fn potion_catalog_preserves_vanilla_order_and_effects() {",
            '        assert_eq!(vanilla_potion(0).unwrap().resource_id, "minecraft:water");',
            '        assert_eq!(vanilla_potion(3).unwrap().resource_id, "minecraft:awkward");',
            '        assert_eq!(vanilla_potion(42).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_WIND_CHARGED_ID);',
            '        assert_eq!(vanilla_potion(43).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_WEAVING_ID);',
            '        assert_eq!(vanilla_potion(44).unwrap().effects[0].effect_id, VANILLA_MOB_EFFECT_OOZING_ID);',
            '        assert_eq!(vanilla_potion(45).unwrap().effects, &[VanillaPotionEffect { effect_id: VANILLA_MOB_EFFECT_INFESTED_ID, duration: 3600, amplifier: 0 }]);',
            "        assert_eq!(vanilla_potion(-1), None);",
            "        assert_eq!(vanilla_potion(46), None);",
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
            "                if display.ends_with(\"/potion_catalog.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"const VANILLA_POTION_KEYS\") || source.contains(\"const VANILLA_POTION_EFFECTS_BY_ID\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"src\").as_path(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel potion catalog owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_catalog(
    catalogs: list[tuple[str, list[BootstrapEntry]]],
    trim_materials: list[TrimMaterialEntry],
    banner_patterns: list[BannerPatternEntry],
    enchantments: list[EnchantmentEntry],
    tooltip_order: tuple[str, ...],
) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 registry/bootstrap order and data-pack tags.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
    ]
    for name, entries in catalogs:
        lines.extend(render_array(name, entries))
    lines.extend(
        (
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub(super) struct VanillaTrimMaterial {",
            "    pub(super) resource_id: &'static str,",
            "    pub(super) description_color: u32,",
            "}",
            "",
            "pub(super) const VANILLA_TRIM_MATERIALS: &[VanillaTrimMaterial] = &[",
        )
    )
    for entry in trim_materials:
        lines.extend(
            (
                "    VanillaTrimMaterial {",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        description_color: 0x{entry.description_color:06X},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub(super) fn vanilla_trim_material(id: i32) -> Option<&'static VanillaTrimMaterial> {",
            "    VANILLA_TRIM_MATERIALS.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub(super) const VANILLA_ENCHANTMENT_TOOLTIP_ORDER: &[&str] = &[",
            *(f"    {rust_string(resource_id)}," for resource_id in tooltip_order),
            "];",
            "",
        )
    )
    for function_name, array_name in RESOURCE_LOOKUPS:
        lines.extend(
            (
                f"pub fn {function_name}(id: i32) -> Option<&'static str> {{",
                "    let index = usize::try_from(id).ok()?;",
                f"    {array_name}.get(index).copied()",
                "}",
                "",
            )
        )
    lines.extend(
        (
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub(super) struct VanillaBannerPattern {",
            "    pub(super) resource_id: &'static str,",
            "    pub(super) translation_key: &'static str,",
            "}",
            "",
            "pub(super) const VANILLA_BANNER_PATTERNS: &[VanillaBannerPattern] = &[",
        )
    )
    for entry in banner_patterns:
        lines.extend(
            (
                "    VanillaBannerPattern {",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        translation_key: {rust_string(entry.translation_key)},",
                "    },",
            )
        )
    lines.extend(("];", ""))
    lines.extend(
        (
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub(super) struct VanillaEnchantment {",
            "    pub(super) resource_id: &'static str,",
            "    pub(super) max_level: i32,",
            "}",
            "",
            "pub(super) const VANILLA_ENCHANTMENTS: &[VanillaEnchantment] = &[",
        )
    )
    for entry in enchantments:
        lines.extend(
            (
                "    VanillaEnchantment {",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        max_level: {entry.max_level},",
                "    },",
            )
        )
    lines.extend(("];", ""))
    lines.extend(
        (
            "#[cfg(test)]",
            "mod tests {",
            "    use std::{collections::HashSet, fs, path::Path};",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn bootstrap_catalog_lengths_and_resource_ids_are_stable() {",
            "        let catalogs: &[(&[&str], usize)] = &[",
        )
    )
    for name, entries in catalogs:
        lines.append(f"            ({name}, {len(entries)}),")
    lines.extend(
        (
            "        ];",
            "        for (entries, expected_len) in catalogs {",
            "            assert_eq!(entries.len(), *expected_len);",
            "            let resources: HashSet<_> = entries.iter().copied().collect();",
            "            assert_eq!(resources.len(), entries.len());",
            '            assert!(entries.iter().all(|entry| entry.starts_with("minecraft:")));',
            "        }",
            f"        assert_eq!(VANILLA_BANNER_PATTERNS.len(), {len(banner_patterns)});",
            f"        assert_eq!(VANILLA_ENCHANTMENTS.len(), {len(enchantments)});",
            f"        assert_eq!(VANILLA_ENCHANTMENT_TOOLTIP_ORDER.len(), {len(tooltip_order)});",
            f"        assert_eq!(VANILLA_TRIM_MATERIALS.len(), {len(trim_materials)});",
            "        let trim_material_resources: HashSet<_> = VANILLA_TRIM_MATERIALS",
            "            .iter()",
            "            .map(|entry| entry.resource_id)",
            "            .collect();",
            "        assert_eq!(trim_material_resources.len(), VANILLA_TRIM_MATERIALS.len());",
            "        let banner_resources: HashSet<_> = VANILLA_BANNER_PATTERNS",
            "            .iter()",
            "            .map(|entry| entry.resource_id)",
            "            .collect();",
            "        assert_eq!(banner_resources.len(), VANILLA_BANNER_PATTERNS.len());",
            "        let enchantment_resources: HashSet<_> = VANILLA_ENCHANTMENTS",
            "            .iter()",
            "            .map(|entry| entry.resource_id)",
            "            .collect();",
            "        assert_eq!(enchantment_resources.len(), VANILLA_ENCHANTMENTS.len());",
            "        let tooltip_resources: HashSet<_> = VANILLA_ENCHANTMENT_TOOLTIP_ORDER",
            "            .iter()",
            "            .copied()",
            "            .collect();",
            "        assert_eq!(tooltip_resources, enchantment_resources);",
            "    }",
            "",
            "    #[test]",
            "    fn bootstrap_catalog_preserves_vanilla_registration_order() {",
            '        assert_eq!(vanilla_trim_material(0).unwrap().resource_id, "minecraft:quartz");',
            "        assert_eq!(vanilla_trim_material(1).unwrap().description_color, 0xECECEC);",
            '        assert_eq!(vanilla_trim_material(10).unwrap().resource_id, "minecraft:resin");',
            "        assert_eq!(vanilla_trim_material(-1), None);",
            "        assert_eq!(vanilla_trim_material(11), None);",
            '        assert_eq!(VANILLA_TRIM_PATTERN_KEYS.last(), Some(&"minecraft:bolt"));',
            '        assert_eq!(VANILLA_VILLAGER_TYPE_KEYS.last(), Some(&"minecraft:taiga"));',
            '        assert_eq!(VANILLA_WOLF_SOUND_VARIANT_KEYS.get(1), Some(&"minecraft:puglin"));',
            '        assert_eq!(VANILLA_WOLF_VARIANT_KEYS.last(), Some(&"minecraft:striped"));',
            '        assert_eq!(VANILLA_INSTRUMENT_KEYS.last(), Some(&"minecraft:dream_goat_horn"));',
            '        assert_eq!(VANILLA_BANNER_PATTERNS.get(31).unwrap().resource_id, "minecraft:gradient");',
            '        assert_eq!(VANILLA_BANNER_PATTERNS.get(34).unwrap().resource_id, "minecraft:curly_border");',
            '        assert_eq!(VANILLA_ENCHANTMENTS.get(13).unwrap().resource_id, "minecraft:sharpness");',
            '        assert_eq!(VANILLA_ENCHANTMENTS.get(13).unwrap().max_level, 5);',
            '        assert_eq!(VANILLA_ENCHANTMENT_TOOLTIP_ORDER.first(), Some(&"minecraft:binding_curse"));',
            '        assert_eq!(VANILLA_ENCHANTMENT_TOOLTIP_ORDER.last(), Some(&"minecraft:mending"));',
            '        assert_eq!(vanilla_cat_variant_resource_id(10), Some("minecraft:all_black"));',
            '        assert_eq!(vanilla_pig_sound_variant_resource_id(1), Some("minecraft:big"));',
            '        assert_eq!(vanilla_zombie_nautilus_variant_resource_id(1), Some("minecraft:warm"));',
            "        assert_eq!(vanilla_chicken_variant_resource_id(3), None);",
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
            "                if display.ends_with(\"/bootstrap_catalog.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"const VANILLA_TRIM_MATERIAL_KEYS\") || source.contains(\"const VANILLA_TRIM_MATERIAL_COLORS\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"src\").as_path(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel trim-material catalog owners: {violations:?}\");",
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
            "                if display.ends_with(\"/painting_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"PAINTING_VARIANT_SIZES\") || source.contains(\"struct VanillaPaintingVariant\") || source.contains(\"const VANILLA_PAINTING_VARIANTS\") { violations.push(display.to_string()); }",
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


def render_decorated_pot_patterns(entries: list[DecoratedPotPatternEntry]) -> str:
    patterns = [entry for entry in entries if entry.constant_name != "BLANK"]
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 DecoratedPotPatterns.java keys, item map, and bootstrap assets.",
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
    for entry in patterns:
        asset_path = entry.asset_id.removeprefix("minecraft:")
        lines.extend(
            (
                f"pub(in crate::entity_models) const DECORATED_POT_{entry.constant_name}_TEXTURE_REF: EntityModelTextureRef =",
                "    EntityModelTextureRef {",
                f"        path: {rust_string(f'textures/entity/decorated_pot/{asset_path}.png')},",
                "        size: [16, 16],",
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
                f"        texture_ref: DECORATED_POT_{entry.constant_name}_TEXTURE_REF,",
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
            "        let mut textures = HashSet::new();",
            "        for (index, entry) in VANILLA_DECORATED_POT_PATTERNS.iter().enumerate() {",
            "            assert_eq!(entry.pattern as usize, index);",
            "            assert!(resources.insert(entry.resource_id));",
            "            assert!(items.insert(entry.item_resource_id));",
            "            assert!(textures.insert(entry.texture_ref.path));",
            "            assert_eq!(entry.texture_ref.size, [16, 16]);",
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
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/decorated_pot_patterns.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum DecoratedPotPattern\") || source.contains(\"fn decorated_pot_pattern_for_sherd_item\") || source.contains(\"const DECORATED_POT_ANGLER_TEXTURE_REF\") || source.contains(\"_pottery_sherd\\\" => Some(DecoratedPotPattern::\") { violations.push(display.to_string()); }",
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
            "VANILLA_WOLF_SOUND_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, WOLF_SOUND_VARIANTS_SOURCE),
                "WolfSoundVariant",
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
            "VANILLA_CAT_SOUND_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, CAT_SOUND_VARIANTS_SOURCE),
                "CatSoundVariant",
                {"createKey"},
                2,
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
            "VANILLA_CHICKEN_SOUND_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, CHICKEN_SOUND_VARIANTS_SOURCE),
                "ChickenSoundVariant",
                {"createKey"},
                2,
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
            "VANILLA_COW_SOUND_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, COW_SOUND_VARIANTS_SOURCE),
                "CowSoundVariant",
                {"createKey"},
                2,
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
            "VANILLA_PIG_SOUND_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, PIG_SOUND_VARIANTS_SOURCE),
                "PigSoundVariant",
                {"createKey"},
                3,
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
        (
            "VANILLA_ZOMBIE_NAUTILUS_VARIANT_KEYS",
            bootstrap_resource_key_entries(
                parse_java(source_root, ZOMBIE_NAUTILUS_VARIANTS_SOURCE),
                "ZombieNautilusVariant",
                {"createKey"},
                2,
                referenced_paths=temperature_paths,
            ),
        ),
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


def generate(source_root: Path) -> tuple[str, str, str, str]:
    trim_materials = trim_material_entries(
        parse_java(source_root, TRIM_MATERIALS_SOURCE)
    )
    banner_patterns = banner_pattern_entries(
        parse_java(source_root, BANNER_PATTERNS_SOURCE)
    )
    paintings = painting_variant_entries(
        parse_java(source_root, PAINTING_VARIANTS_SOURCE)
    )
    decorated_pot_patterns = decorated_pot_pattern_entries(
        parse_java(source_root, DECORATED_POT_PATTERNS_SOURCE),
        parse_java(source_root, ITEMS_SOURCE),
    )
    enchantments = enchantment_entries(parse_java(source_root, ENCHANTMENTS_SOURCE))
    mob_effects = mob_effect_entries(parse_java(source_root, MOB_EFFECTS_SOURCE))
    potions = potion_entries(
        parse_java(source_root, POTIONS_SOURCE),
        {entry.constant_name for entry in mob_effects},
    )
    tooltip_order = enchantment_tooltip_order(
        load_tag_resources(source_root, ENCHANTMENT_TAG_SOURCE_DIR), enchantments
    )
    rendered = render_catalog(
        extract_catalogs(source_root),
        trim_materials,
        banner_patterns,
        enchantments,
        tooltip_order,
    )
    return (
        format_rust(rendered),
        format_rust(render_potion_catalog(potions)),
        format_rust(render_painting_variants(paintings)),
        format_rust(render_decorated_pot_patterns(decorated_pot_patterns)),
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
    ) = generate(args.source_root)
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
        return 0 if matches else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    args.potion_catalog_output.parent.mkdir(parents=True, exist_ok=True)
    args.potion_catalog_output.write_text(generated_potions)
    args.painting_variants_output.parent.mkdir(parents=True, exist_ok=True)
    args.painting_variants_output.write_text(generated_painting_variants)
    args.decorated_pot_patterns_output.parent.mkdir(parents=True, exist_ok=True)
    args.decorated_pot_patterns_output.write_text(generated_decorated_pot_patterns)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
