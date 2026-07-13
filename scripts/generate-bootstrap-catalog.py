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
    REPO_ROOT / "crates/bbb-world/src/registries/enchantments/generated_26_1.rs"
)
DEFAULT_POTION_CATALOG_OUTPUT = (
    REPO_ROOT / "crates/bbb-item-model/src/item_runtime/potion_catalog.rs"
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
    REPO_ROOT / "crates/bbb-renderer/src/entity_models/villager_professions.rs"
)
DEFAULT_VILLAGER_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-render-types/src/villager_model_types.rs"
)
DEFAULT_CAT_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-renderer/src/entity_models/cat_variants.rs"
)
DEFAULT_WOLF_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-renderer/src/entity_models/wolf_variants.rs"
)
DEFAULT_TEMPERATURE_ANIMAL_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-renderer/src/entity_models/temperature_animal_variants.rs"
)
DEFAULT_ZOMBIE_NAUTILUS_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-renderer/src/entity_models/zombie_nautilus_variants.rs"
)
DEFAULT_ANIMAL_SOUND_VARIANTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-world/src/entities/sound_variants.rs"
)
DEFAULT_INSTRUMENTS_OUTPUT = REPO_ROOT / "crates/bbb-world/src/registries/instruments.rs"
DEFAULT_TRIM_PATTERNS_OUTPUT = REPO_ROOT / "crates/bbb-world/src/registries/trim_patterns.rs"
DEFAULT_TRIM_MATERIALS_OUTPUT = REPO_ROOT / "crates/bbb-world/src/registries/trim_materials.rs"

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
VILLAGER_PROFESSIONS_SOURCE = Path(
    "net/minecraft/world/entity/npc/villager/VillagerProfession.java"
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
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def parse_java(source_root: Path, relative_path: Path):
    return parse_source(relative_path.as_posix(), (source_root / relative_path).read_bytes())


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def rust_variant(constant_name: str) -> str:
    return "".join(part.title() for part in constant_name.split("_"))


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
            "                if source.contains(\"enum VanillaEnchantmentIdentity\") || source.contains(\"struct VanillaEnchantment {\") || source.contains(\"const VANILLA_ENCHANTMENTS\") || source.contains(\"const VANILLA_ENCHANTMENT_TOOLTIP_ORDER\") || source.contains(\"fn vanilla_enchantment_resource_id(\") { violations.push(display.to_string()); }",
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
            "                if source.contains(\"PAINTING_VARIANT_SIZES\") || source.contains(\"struct VanillaPaintingVariant\") || source.contains(\"const VANILLA_PAINTING_VARIANTS\") || source.contains(\"fn vanilla_painting_variant_resource_id(\") { violations.push(display.to_string()); }",
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
            "                if source.contains(\"enum BannerPatternKind\") || source.contains(\"struct VanillaBannerPattern {\") || source.contains(\"const VANILLA_BANNER_PATTERNS\") || source.contains(\"fn vanilla_banner_pattern_resource_id(\") { violations.push(display.to_string()); }",
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


def render_banner_patterns(entries: list[BannerPatternEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 BannerPatterns.java keys, bootstrap order, and assets.",
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
        constant = banner_texture_constant(entry)
        lines.extend(
            (
                f"pub(in crate::entity_models) const {constant}: EntityModelTextureRef =",
                "    EntityModelTextureRef {",
                f"        path: {rust_string(f'textures/entity/banner/{asset_path}.png')},",
                "        size: [64, 64],",
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
            "pub(in crate::entity_models) fn banner_pattern_texture_ref(",
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
            "            assert_eq!(entry.texture_ref.size, [64, 64]);",
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
            "    pub(in crate::entity_models) fn texture_index(self) -> Option<usize> {",
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
            "                if display.ends_with(\"/villager_professions.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
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


def render_cat_variants(entries: list[BootstrapEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 CatVariants.java keys and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// Known vanilla 26.1 cat-variant resource identities.",
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
        "}",
        "",
        "const VANILLA_CAT_VARIANTS: &[VanillaCatVariant] = &[",
    ]
    for entry in entries:
        lines.extend(
            (
                "    VanillaCatVariant {",
                f"        variant: CatModelVariant::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
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
            "        for (registry_id, entry) in VANILLA_CAT_VARIANTS.iter().enumerate() {",
            "            assert_eq!(entry.variant as usize, registry_id);",
            "            assert!(resources.insert(entry.resource_id));",
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
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_cat_variant_identity_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/cat_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum CatModelVariant\") || source.contains(\"fn cat_variant_from_entry_id(\") || source.contains(\"fn cat_variant_from_vanilla_registry_id(\") || source.contains(\"fn vanilla_cat_variant_resource_id(\") || source.contains(\"\\\"minecraft:tabby\\\" => Some(CatModelVariant::\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel cat-variant identity owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_wolf_variants(entries: list[BootstrapEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 WolfVariants.java keys and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// Known vanilla 26.1 wolf-variant resource identities.",
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
        "}",
        "",
        "const VANILLA_WOLF_VARIANTS: &[VanillaWolfVariant] = &[",
    ]
    for entry in entries:
        lines.extend(
            (
                "    VanillaWolfVariant {",
                f"        variant: WolfModelVariant::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
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
            "        for (registry_id, entry) in VANILLA_WOLF_VARIANTS.iter().enumerate() {",
            "            assert_eq!(entry.variant as usize, registry_id);",
            "            assert!(resources.insert(entry.resource_id));",
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
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_wolf_variant_identity_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/wolf_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum WolfModelVariant\") || source.contains(\"fn wolf_variant_from_entry_id(\") || source.contains(\"fn wolf_variant_from_vanilla_registry_id(\") || source.contains(\"fn vanilla_wolf_variant_resource_id(\") || source.contains(\"\\\"minecraft:pale\\\" => Some(WolfModelVariant::\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel wolf-variant identity owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_temperature_animal_variants(
    catalogs: list[tuple[str, list[BootstrapEntry]]],
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

    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Chicken/Cow/Pig/FrogVariants.java bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
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
    lines.extend(
        (
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_temperature_animal_variant_identity_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/temperature_animal_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum ChickenModelVariant\") || source.contains(\"enum CowModelVariant\") || source.contains(\"enum PigModelVariant\") || source.contains(\"enum FrogModelVariant\") || source.contains(\"fn chicken_variant_from_entry_id(\") || source.contains(\"fn cow_variant_from_entry_id(\") || source.contains(\"fn pig_variant_from_entry_id(\") || source.contains(\"fn frog_variant_from_entry_id(\") || source.contains(\"fn chicken_variant_from_vanilla_registry_id(\") || source.contains(\"fn cow_variant_from_vanilla_registry_id(\") || source.contains(\"fn pig_variant_from_vanilla_registry_id(\") || source.contains(\"fn frog_variant_from_vanilla_registry_id(\") || source.contains(\"fn vanilla_chicken_variant_resource_id(\") || source.contains(\"fn vanilla_cow_variant_resource_id(\") || source.contains(\"fn vanilla_pig_variant_resource_id(\") || source.contains(\"fn vanilla_frog_variant_resource_id(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel temperature animal-variant identity owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_zombie_nautilus_variants(
    catalogs: list[tuple[str, list[BootstrapEntry]]],
) -> str:
    entries = dict(catalogs)["VANILLA_ZOMBIE_NAUTILUS_VARIANT_KEYS"]
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ZombieNautilusVariants.java keys and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// Known vanilla 26.1 zombie-nautilus variant resource identities.",
        "///",
        "/// Session holder ids must be resolved through the synchronized registry before this lookup;",
        "/// the registry element separately owns the model type and texture asset payload.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(u8)]",
        "pub enum ZombieNautilusModelVariant {",
        *(f"    {rust_variant(entry.constant_name)}," for entry in entries),
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "struct VanillaZombieNautilusVariant {",
        "    variant: ZombieNautilusModelVariant,",
        "    resource_id: &'static str,",
        "}",
        "",
        "const VANILLA_ZOMBIE_NAUTILUS_VARIANTS: &[VanillaZombieNautilusVariant] = &[",
    ]
    for entry in entries:
        lines.extend(
            (
                "    VanillaZombieNautilusVariant {",
                f"        variant: ZombieNautilusModelVariant::{rust_variant(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            )
        )
    lines.extend(
        (
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
            "        assert_eq!(ZombieNautilusModelVariant::for_vanilla_registry_id(0), Some(ZombieNautilusModelVariant::Temperate));",
            "        assert_eq!(ZombieNautilusModelVariant::for_vanilla_registry_id(1), Some(ZombieNautilusModelVariant::Warm));",
            "        assert_eq!(ZombieNautilusModelVariant::for_vanilla_registry_id(-1), None);",
            "        assert_eq!(ZombieNautilusModelVariant::for_vanilla_registry_id(2), None);",
            "        assert_eq!(ZombieNautilusModelVariant::for_resource_id(\"minecraft:warm\"), Some(ZombieNautilusModelVariant::Warm));",
            "        assert_eq!(ZombieNautilusModelVariant::for_resource_id(\"minecraft:cold\"), None);",
            "        assert_eq!(ZombieNautilusModelVariant::Temperate.resource_id(), \"minecraft:temperate\");",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_zombie_nautilus_variant_identity_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/zombie_nautilus_variants.rs\") || display.ends_with(\"/bootstrap_catalog.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum ZombieNautilusModelVariant\") || source.contains(\"fn zombie_nautilus_variant_from_entry_id(\") || source.contains(\"fn zombie_nautilus_variant_from_vanilla_registry_id(\") || source.contains(\"fn zombie_nautilus_variant_is_coral(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel zombie-nautilus variant identity owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_animal_sound_variants(
    catalogs: list[tuple[str, list[BootstrapEntry]]],
) -> str:
    expected_catalogs = (
        ("VANILLA_CAT_SOUND_VARIANT_KEYS", "CatSoundVariantIdentity"),
        ("VANILLA_CHICKEN_SOUND_VARIANT_KEYS", "ChickenSoundVariantIdentity"),
        ("VANILLA_COW_SOUND_VARIANT_KEYS", "CowSoundVariantIdentity"),
        ("VANILLA_PIG_SOUND_VARIANT_KEYS", "PigSoundVariantIdentity"),
        ("VANILLA_WOLF_SOUND_VARIANT_KEYS", "WolfSoundVariantIdentity"),
    )
    catalog_map = dict(catalogs)
    lines = [
        "// @generated by scripts/generate-bootstrap-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Cat/Chicken/Cow/Pig/WolfSoundVariants.java bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-bootstrap-catalog.py --check",
        "",
        "/// Defines one known vanilla sound-variant identity catalog.",
        "///",
        "/// Session holder ids must be resolved through the corresponding synchronized registry",
        "/// before this fixed-order fallback; registry elements separately own SoundEvent holders.",
        "macro_rules! define_sound_variant_identity {",
        "    ($name:ident, $entries:ident, [$(($variant:ident, $resource:literal)),+ $(,)?]) => {",
        "        #[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "        #[repr(u8)]",
        "        pub enum $name {",
        "            $($variant),+",
        "        }",
        "",
        "        const $entries: &[(&str, $name)] = &[",
        "            $(($resource, $name::$variant)),+",
        "        ];",
        "",
        "        impl $name {",
        "            pub fn for_resource_id(resource_id: &str) -> Option<Self> {",
        "                $entries",
        "                    .iter()",
        "                    .find(|(resource, _)| *resource == resource_id)",
        "                    .map(|(_, variant)| *variant)",
        "            }",
        "",
        "            pub fn for_vanilla_registry_id(registry_id: i32) -> Option<Self> {",
        "                $entries",
        "                    .get(usize::try_from(registry_id).ok()?)",
        "                    .map(|(_, variant)| *variant)",
        "            }",
        "",
        "            pub fn resource_id(self) -> &'static str {",
        "                $entries[self as usize].0",
        "            }",
        "        }",
        "    };",
        "}",
        "",
    ]
    for catalog_name, type_name in expected_catalogs:
        entries = catalog_map[catalog_name]
        const_name = catalog_name.removesuffix("_KEYS") + "_IDENTITIES"
        lines.extend(
            (
                "define_sound_variant_identity!(",
                f"    {type_name},",
                f"    {const_name},",
                "    [",
                *(
                    f"        ({rust_variant(entry.constant_name)}, {rust_string(entry.resource_id)}),"
                    for entry in entries
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
    for catalog_name, type_name in expected_catalogs:
        entries = catalog_map[catalog_name]
        const_name = catalog_name.removesuffix("_KEYS") + "_IDENTITIES"
        last = entries[-1]
        lines.extend(
            (
                f"        assert_eq!({const_name}.len(), {len(entries)});",
                f"        let resources: HashSet<_> = {const_name}.iter().map(|(resource, _)| *resource).collect();",
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
    lines.extend(
        (
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_animal_sound_variant_identity_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/sound_variants.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                if source.contains(\"enum CatSoundVariantIdentity\") || source.contains(\"enum ChickenSoundVariantIdentity\") || source.contains(\"enum CowSoundVariantIdentity\") || source.contains(\"enum PigSoundVariantIdentity\") || source.contains(\"enum WolfSoundVariantIdentity\") || source.contains(\"fn vanilla_cat_sound_variant_resource_id(\") || source.contains(\"fn vanilla_chicken_sound_variant_resource_id(\") || source.contains(\"fn vanilla_cow_sound_variant_resource_id(\") || source.contains(\"fn vanilla_pig_sound_variant_resource_id(\") || source.contains(\"fn vanilla_wolf_sound_variant_resource_id(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel animal sound-variant identity owners: {violations:?}\");",
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
        "// Source: vanilla 26.1 TrimMaterials.java bootstrap order and description colors.",
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


def generate(
    source_root: Path,
) -> tuple[str, str, str, str, str, str, str, str, str, str, str, str, str, str, str, str]:
    trim_materials = trim_material_entries(
        parse_java(source_root, TRIM_MATERIALS_SOURCE)
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
    trim_patterns = dict(catalogs)["VANILLA_TRIM_PATTERN_KEYS"]
    instruments = dict(catalogs)["VANILLA_INSTRUMENT_KEYS"]
    cat_variants = dict(catalogs)["VANILLA_CAT_VARIANT_KEYS"]
    wolf_variants = dict(catalogs)["VANILLA_WOLF_VARIANT_KEYS"]
    rendered = render_enchantment_catalog(enchantments, tooltip_order)
    return (
        format_rust(rendered),
        format_rust(render_potion_catalog(potions)),
        format_rust(render_painting_variants(paintings)),
        format_rust(render_decorated_pot_patterns(decorated_pot_patterns)),
        format_rust(render_banner_pattern_types(banner_patterns)),
        format_rust(render_banner_patterns(banner_patterns)),
        format_rust(render_villager_professions(villager_professions)),
        format_rust(render_villager_types(villager_types)),
        format_rust(render_cat_variants(cat_variants)),
        format_rust(render_wolf_variants(wolf_variants)),
        format_rust(render_temperature_animal_variants(catalogs)),
        format_rust(render_zombie_nautilus_variants(catalogs)),
        format_rust(render_animal_sound_variants(catalogs)),
        format_rust(render_instruments(instruments)),
        format_rust(render_trim_patterns(trim_patterns)),
        format_rust(render_trim_materials(trim_materials)),
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
