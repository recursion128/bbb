#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 core registry catalogs from Tree-sitter CST facts.

The current first-stage catalog covers damage types. Resource ids come from the
DamageTypes key declarations, while numeric ids follow the bootstrap registration
order. Normal builds consume only the generated Rust file; this maintenance tool
is the only path that needs the local decompiled vanilla sources.
"""

from __future__ import annotations

import argparse
import difflib
import struct
import subprocess
import sys
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
from core_registry_catalog_extractor import (
    AnimalVariantComponentRegistryEntry,
    BlockEntityTypeEntry,
    DataComponentTypeEntry,
    EntityTypeEntry,
    NoteBlockInstrumentEntry,
    ParticleTypeEntry,
    PoiTypeEntry,
    RegistryEntry,
    VanillaParticleCodecFamily,
    activity_entries,
    animal_variant_component_registry_entries,
    animal_variant_texture_registry_entries,
    block_entity_menu_provider_classes,
    block_entity_type_entries,
    common_item_component_constant_names,
    damage_type_entries,
    data_component_predicate_type_entries,
    data_component_type_entries,
    entity_type_entries,
    entity_type_track_delta_exclusions,
    mob_category_entries,
    note_block_instrument_entries,
    note_block_instrument_type_names,
    number_format_type_entries,
    particle_type_entries,
    poi_type_entries,
    vec3_constant,
)
from vanilla_java_source import parse_source
from vanilla_tag_source import load_tag_resources, resolve_tag_values

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DAMAGE_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/damage_types.rs"
DEFAULT_ENTITY_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/entity_types.rs"
DEFAULT_ENTITY_TYPES_BIN_OUTPUT = REPO_ROOT / "crates/bbb-facts/data/entity_types_26_1.bin"
DEFAULT_BLOCK_ENTITY_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/block_entity_types.rs"
)
DEFAULT_DATA_COMPONENT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/data_component_types.rs"
)
DEFAULT_PARTICLE_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/particle_types.rs"
DEFAULT_ACTIVITY_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/activity_types.rs"
DEFAULT_NUMBER_FORMAT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/number_format_types.rs"
)
DEFAULT_POI_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/poi_types.rs"
DEFAULT_NOTE_BLOCK_INSTRUMENTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-facts/src/note_block_instruments.rs"
)
DAMAGE_TYPES_SOURCE = Path("net/minecraft/world/damagesource/DamageTypes.java")
ENTITY_TYPES_SOURCE = Path("net/minecraft/world/entity/EntityType.java")
MOB_CATEGORIES_SOURCE = Path("net/minecraft/world/entity/MobCategory.java")
AVATAR_SOURCE = Path("net/minecraft/world/entity/Avatar.java")
ENTITY_TYPE_TAGS_DIRECTORY = Path("data/minecraft/tags/entity_type")
BLOCK_ENTITY_TYPES_SOURCE = Path(
    "net/minecraft/world/level/block/entity/BlockEntityType.java"
)
BLOCK_ENTITY_CLASS_DIRECTORIES = (
    Path("net/minecraft/world/level/block/entity"),
    Path("net/minecraft/world/level/block/piston"),
)
DATA_COMPONENT_PREDICATE_TYPES_SOURCE = Path(
    "net/minecraft/core/component/predicates/DataComponentPredicates.java"
)
DATA_COMPONENT_TYPES_SOURCE = Path("net/minecraft/core/component/DataComponents.java")
REGISTRIES_SOURCE = Path("net/minecraft/core/registries/Registries.java")
PARTICLE_TYPES_SOURCE = Path("net/minecraft/core/particles/ParticleTypes.java")
ACTIVITY_TYPES_SOURCE = Path("net/minecraft/world/entity/schedule/Activity.java")
NUMBER_FORMAT_TYPES_SOURCE = Path(
    "net/minecraft/network/chat/numbers/NumberFormatTypes.java"
)
POI_TYPES_SOURCE = Path("net/minecraft/world/entity/ai/village/poi/PoiTypes.java")
NOTE_BLOCK_INSTRUMENTS_SOURCE = Path(
    "net/minecraft/world/level/block/state/properties/NoteBlockInstrument.java"
)
SOUND_EVENTS_SOURCE = Path("net/minecraft/sounds/SoundEvents.java")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_root(parser)
    parser.add_argument(
        "--damage-types-output", type=Path, default=DEFAULT_DAMAGE_TYPES_OUTPUT
    )
    parser.add_argument(
        "--entity-types-output", type=Path, default=DEFAULT_ENTITY_TYPES_OUTPUT
    )
    parser.add_argument(
        "--entity-types-bin-output",
        type=Path,
        default=DEFAULT_ENTITY_TYPES_BIN_OUTPUT,
    )
    parser.add_argument(
        "--block-entity-types-output",
        type=Path,
        default=DEFAULT_BLOCK_ENTITY_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--data-component-types-output",
        type=Path,
        default=DEFAULT_DATA_COMPONENT_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--particle-types-output", type=Path, default=DEFAULT_PARTICLE_TYPES_OUTPUT
    )
    parser.add_argument(
        "--activity-types-output", type=Path, default=DEFAULT_ACTIVITY_TYPES_OUTPUT
    )
    parser.add_argument(
        "--number-format-types-output",
        type=Path,
        default=DEFAULT_NUMBER_FORMAT_TYPES_OUTPUT,
    )
    parser.add_argument("--poi-types-output", type=Path, default=DEFAULT_POI_TYPES_OUTPUT)
    parser.add_argument(
        "--note-block-instruments-output",
        type=Path,
        default=DEFAULT_NOTE_BLOCK_INSTRUMENTS_OUTPUT,
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def render_damage_types(entries: list[RegistryEntry]) -> str:
    missing_effects = [entry.constant_name for entry in entries if entry.damage_effect is None]
    if missing_effects:
        raise ValueError(f"damage types lack constructor effects: {missing_effects}")
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 DamageTypes.java key declarations and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct DamageTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub effect: &'static str,",
        "}",
        "",
        "pub const VANILLA_DAMAGE_TYPES: &[DamageTypeCatalogEntry] = &[",
    ]
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    DamageTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        effect: {rust_string(entry.damage_effect)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_damage_type(id: i32) -> Option<&'static DamageTypeCatalogEntry> {",
            "    VANILLA_DAMAGE_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_damage_type_resource_id(id: i32) -> Option<&'static str> {",
            "    Some(vanilla_damage_type(id)?.resource_id)",
            "}",
            "",
            "pub fn vanilla_damage_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static DamageTypeCatalogEntry> {",
            "    VANILLA_DAMAGE_TYPES",
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
            "    fn damage_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_DAMAGE_TYPES.len(), 50);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (id, entry) in VANILLA_DAMAGE_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn damage_type_catalog_follows_bootstrap_not_declaration_order() {",
            "        assert_eq!(vanilla_damage_type(0).unwrap().constant_name, \"IN_FIRE\");",
            "        assert_eq!(vanilla_damage_type(35).unwrap().constant_name, \"FIREWORKS\");",
            "        assert_eq!(vanilla_damage_type(48).unwrap().constant_name, \"WIND_CHARGE\");",
            "        assert_eq!(vanilla_damage_type_resource_id(49), Some(\"minecraft:mace_smash\"));",
            "        assert_eq!(vanilla_damage_type(-1), None);",
            "        assert_eq!(vanilla_damage_type(50), None);",
            "    }",
            "",
            "    #[test]",
            "    fn damage_type_catalog_tracks_vanilla_effects() {",
            "        assert_eq!(vanilla_damage_type(0).unwrap().effect, \"burning\");",
            "        assert_eq!(vanilla_damage_type(2).unwrap().effect, \"hurt\");",
            "        assert_eq!(vanilla_damage_type(8).unwrap().effect, \"drowning\");",
            "        assert_eq!(vanilla_damage_type(20).unwrap().effect, \"poking\");",
            "        assert_eq!(vanilla_damage_type(21).unwrap().effect, \"freezing\");",
            "        assert_eq!(vanilla_damage_type(41).unwrap().effect, \"thorns\");",
            "        assert_eq!(",
            "            vanilla_damage_type_for_resource_id(\"minecraft:drown\")",
            "                .unwrap()",
            "                .id,",
            "            8",
            "        );",
            "        assert_eq!(vanilla_damage_type_for_resource_id(\"example:drown\"), None);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_number_format_types(entries: list[RegistryEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 NumberFormatTypes.bootstrap registration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct NumberFormatTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_NUMBER_FORMAT_TYPE_{entry.constant_name}_ID: i32 = {entry_id};"
        )
    lines.extend(("", "pub const VANILLA_NUMBER_FORMAT_TYPES: &[NumberFormatTypeCatalogEntry] = &["))
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    NumberFormatTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_number_format_type(id: i32) -> Option<&'static NumberFormatTypeCatalogEntry> {",
            "    VANILLA_NUMBER_FORMAT_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn number_format_type_catalog_follows_bootstrap_order() {",
            "        assert_eq!(VANILLA_NUMBER_FORMAT_TYPES.len(), 3);",
            "        assert_eq!(vanilla_number_format_type(VANILLA_NUMBER_FORMAT_TYPE_BLANK_ID).unwrap().resource_id, \"minecraft:blank\");",
            "        assert_eq!(vanilla_number_format_type(VANILLA_NUMBER_FORMAT_TYPE_STYLED_ID).unwrap().resource_id, \"minecraft:styled\");",
            "        assert_eq!(vanilla_number_format_type(VANILLA_NUMBER_FORMAT_TYPE_FIXED_ID).unwrap().resource_id, \"minecraft:fixed\");",
            "        assert_eq!(vanilla_number_format_type(-1), None);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_poi_types(entries: list[PoiTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 PoiTypes key declarations and bootstrap order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct PoiTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub max_tickets: i32,",
        "    pub valid_range: i32,",
        "    pub matching_blocks: &'static [&'static str],",
        "    pub head_only: bool,",
        "}",
        "",
        "pub const VANILLA_POI_TYPES: &[PoiTypeCatalogEntry] = &[",
    ]
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    PoiTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        max_tickets: {entry.max_tickets},",
                f"        valid_range: {entry.valid_range},",
                "        matching_blocks: &[" + ", ".join(rust_string(block) for block in entry.matching_blocks) + "],",
                f"        head_only: {str(entry.head_only).lower()},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_poi_type(id: i32) -> Option<&'static PoiTypeCatalogEntry> {",
            "    VANILLA_POI_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_poi_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static PoiTypeCatalogEntry> {",
            "    VANILLA_POI_TYPES",
            "        .iter()",
            "        .find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn poi_type_catalog_preserves_registry_limits() {",
            "        assert_eq!(VANILLA_POI_TYPES.len(), 21);",
            "        let armorer = vanilla_poi_type_for_resource_id(\"minecraft:armorer\").unwrap();",
            "        assert_eq!((armorer.max_tickets, armorer.valid_range), (1, 1));",
            "        let meeting = vanilla_poi_type_for_resource_id(\"minecraft:meeting\").unwrap();",
            "        assert_eq!((meeting.max_tickets, meeting.valid_range), (32, 6));",
            "        let portal = vanilla_poi_type_for_resource_id(\"minecraft:nether_portal\").unwrap();",
            "        assert_eq!((portal.max_tickets, portal.valid_range), (0, 1));",
            "        assert_eq!(vanilla_poi_type(-1), None);",
            "        assert_eq!(vanilla_poi_type_for_resource_id(\"example:unknown\"), None);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def rust_enum_variant(constant_name: str) -> str:
    return "".join(part.capitalize() for part in constant_name.split("_"))


def render_note_block_instruments(
    entries: list[NoteBlockInstrumentEntry], type_names: list[str]
) -> str:
    variants = {name: rust_enum_variant(name) for name in type_names}
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 NoteBlockInstrument.java enum constants, joined to the",
        "// SoundEvents.java registration each one names.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "",
        "/// `NoteBlockInstrument.Type` (`NoteBlockInstrument.java:68-72`), the third",
        "/// constructor argument every instrument carries and the one",
        "/// `NoteBlockInstrument.isTunable` (`:56-58`), `hasCustomSound` (`:60-62`) and",
        "/// `worksAboveNoteBlock` (`:64-66`) are all defined in terms of.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum NoteBlockInstrumentType {",
    ]
    for name in type_names:
        lines.append(f"    /// `NoteBlockInstrument.Type.{name}`.")
        lines.append(f"    {variants[name]},")
    lines.extend(
        (
            "}",
            "",
            "/// One `NoteBlockInstrument` constant (`NoteBlockInstrument.java:9-35`).",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct NoteBlockInstrumentCatalogEntry {",
            "    /// The constant's ordinal, i.e. its declaration position.",
            "    pub id: i32,",
            "    pub constant_name: &'static str,",
            "    /// `getSerializedName` (`NoteBlockInstrument.java:48-50`), which is the",
            "    /// value the `instrument` block-state property carries.",
            "    pub serialized_name: &'static str,",
            "    /// The `SoundEvents` constant the instrument names.",
            "    pub sound_constant_name: &'static str,",
            "    /// That constant's registered resource id (`SoundEvents.java:1852-1866`).",
            "    pub sound_resource_id: &'static str,",
            "    pub instrument_type: NoteBlockInstrumentType,",
            "}",
            "",
            "pub const VANILLA_NOTE_BLOCK_INSTRUMENTS: &[NoteBlockInstrumentCatalogEntry] = &[",
        )
    )
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    NoteBlockInstrumentCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                f"        sound_constant_name: {rust_string(entry.sound_constant_name)},",
                f"        sound_resource_id: {rust_string(entry.sound_resource_id)},",
                "        instrument_type: NoteBlockInstrumentType::"
                f"{variants[entry.instrument_type]},",
                "    },",
            )
        )
    base_block = variants["BASE_BLOCK"]
    mob_head = variants["MOB_HEAD"]
    custom = variants["CUSTOM"]
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_note_block_instrument(",
            "    id: i32,",
            ") -> Option<&'static NoteBlockInstrumentCatalogEntry> {",
            "    VANILLA_NOTE_BLOCK_INSTRUMENTS.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "/// The instrument one `instrument` block-state property value names.",
            "pub fn vanilla_note_block_instrument_for_name(",
            "    serialized_name: &str,",
            ") -> Option<&'static NoteBlockInstrumentCatalogEntry> {",
            "    VANILLA_NOTE_BLOCK_INSTRUMENTS",
            "        .iter()",
            "        .find(|entry| entry.serialized_name == serialized_name)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "    use crate::sound_event_types::vanilla_sound_event_registry_id;",
            "",
            "    #[test]",
            "    fn note_block_instrument_catalog_is_complete_contiguous_and_unique() {",
            f"        assert_eq!(VANILLA_NOTE_BLOCK_INSTRUMENTS.len(), {len(entries)});",
            "        let mut constant_names = HashSet::new();",
            "        let mut serialized_names = HashSet::new();",
            "        for (id, entry) in VANILLA_NOTE_BLOCK_INSTRUMENTS.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(serialized_names.insert(entry.serialized_name));",
            "            assert!(entry.sound_resource_id.starts_with(\"minecraft:\"));",
            "        }",
            f"        assert_eq!(vanilla_note_block_instrument({len(entries)}), None);",
            "        assert_eq!(vanilla_note_block_instrument(-1), None);",
            "        assert_eq!(",
            "            vanilla_note_block_instrument_for_name(\"not_an_instrument\"),",
            "            None",
            "        );",
            "    }",
            "",
            "    #[test]",
            "    fn note_block_instrument_catalog_carries_the_declaration_facts() {",
            "        let harp = vanilla_note_block_instrument_for_name(\"harp\").unwrap();",
            "        assert_eq!(harp.id, 0);",
            "        assert_eq!(harp.constant_name, \"HARP\");",
            "        assert_eq!(harp.sound_resource_id, \"minecraft:block.note_block.harp\");",
            "        assert_eq!(",
            "            harp.instrument_type,",
            f"            NoteBlockInstrumentType::{base_block}",
            "        );",
            "        let dragon = vanilla_note_block_instrument_for_name(\"dragon\").unwrap();",
            "        assert_eq!(",
            "            dragon.sound_resource_id,",
            "            \"minecraft:block.note_block.imitate.ender_dragon\"",
            "        );",
            "        assert_eq!(",
            "            dragon.instrument_type,",
            f"            NoteBlockInstrumentType::{mob_head}",
            "        );",
            "        // The one instrument whose sound is not a note-block sound at all;",
            "        // `NoteBlock.triggerEvent` (`NoteBlock.java:160-166`) replaces it",
            "        // with the skull's own before it is ever played.",
            "        let custom_head =",
            "            vanilla_note_block_instrument_for_name(\"custom_head\").unwrap();",
            "        assert_eq!(custom_head.sound_resource_id, \"minecraft:ui.button.click\");",
            "        assert_eq!(",
            "            custom_head.instrument_type,",
            f"            NoteBlockInstrumentType::{custom}",
            "        );",
            "    }",
            "",
            "    /// The join this catalog is built on has to survive in the checked-in",
            "    /// snapshots too: every sound an instrument names must be one the",
            "    /// generated sound-event registry can hand a wire id for.",
            "    #[test]",
            "    fn every_instrument_sound_is_in_the_generated_sound_event_catalog() {",
            "        for entry in VANILLA_NOTE_BLOCK_INSTRUMENTS {",
            "            assert!(",
            "                vanilla_sound_event_registry_id(entry.sound_resource_id).is_some(),",
            "                \"{} names the unregistered sound {}\",",
            "                entry.constant_name,",
            "                entry.sound_resource_id",
            "            );",
            "        }",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_entity_types(
    entries: list[EntityTypeEntry],
    track_delta_exclusions: frozenset[str],
    mob_categories: list[tuple[str, str]],
    arrows: frozenset[str],
    redirectable_projectiles: frozenset[str],
    deflects_projectiles: frozenset[str],
    powder_snow_walkable_mobs: frozenset[str],
    freeze_immune_entity_types: frozenset[str],
    non_controlling_riders: frozenset[str],
    can_float_while_ridden: frozenset[str],
    can_equip_saddle: frozenset[str],
    can_wear_horse_armor: frozenset[str],
) -> tuple[str, bytes]:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 EntityType.java and selected entity_type data tags.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaEntityMobCategory {",
        "    Ambient,",
        "    Axolotls,",
        "    Creature,",
        "    Misc,",
        "    Monster,",
        "    UndergroundWaterCreature,",
        "    WaterAmbient,",
        "    WaterCreature,",
        "}",
        "",
        "pub const VANILLA_ENTITY_MOB_CATEGORIES: &[(VanillaEntityMobCategory, &str)] = &[",
        *(
            f"    (VanillaEntityMobCategory::{''.join(part.title() for part in constant.lower().split('_'))}, {rust_string(serialized)}),"
            for constant, serialized in mob_categories
        ),
        "];",
        "",
        "pub fn vanilla_entity_mob_category_for_serialized_name(",
        "    serialized_name: &str,",
        ") -> Option<VanillaEntityMobCategory> {",
        "    VANILLA_ENTITY_MOB_CATEGORIES",
        "        .iter()",
        "        .find_map(|(category, name)| (*name == serialized_name).then_some(*category))",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq)]",
        "pub struct EntityTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub default_loot_table: Option<&'static str>,",
        "    pub mob_category: VanillaEntityMobCategory,",
        "    pub allowed_in_peaceful: bool,",
        "    pub only_op_can_set_nbt: bool,",
        "    pub serializable: bool,",
        "    pub can_summon: bool,",
        "    pub fire_immune: bool,",
        "    pub track_deltas: bool,",
        "    pub width: f32,",
        "    pub height: f32,",
        "    pub client_tracking_range: i32,",
        "    pub update_interval: i32,",
        "    pub can_spawn_far_from_player: bool,",
        "    pub eye_height: Option<f32>,",
        "    pub name_tag_offset: Option<f32>,",
        "    pub passenger_attachments: &'static [[f32; 3]],",
        "    pub vehicle_attachment: [f32; 3],",
        "    pub in_arrows_tag: bool,",
        "    pub redirectable_projectile: bool,",
        "    pub deflects_projectiles: bool,",
        "    pub powder_snow_walkable: bool,",
        "    pub freeze_immune: bool,",
        "    pub non_controlling_rider: bool,",
        "    pub can_float_while_ridden: bool,",
        "    pub can_equip_saddle: bool,",
        "    pub can_wear_horse_armor: bool,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_ENTITY_TYPE_{entry.constant_name}_ID: i32 = {entry_id};"
        )

    mob_category_variants = [
        "Ambient",
        "Axolotls",
        "Creature",
        "Misc",
        "Monster",
        "UndergroundWaterCreature",
        "WaterAmbient",
        "WaterCreature",
    ]
    mob_category_tags = {name: idx for idx, name in enumerate(mob_category_variants)}

    string_pool = bytearray()
    string_offsets: dict[str, tuple[int, int]] = {}

    def intern_str(text: str) -> tuple[int, int]:
        cached = string_offsets.get(text)
        if cached is not None:
            return cached
        raw = text.encode("utf-8")
        if not raw.isascii():
            raise ValueError(f"non-ASCII entity catalog string: {text!r}")
        if len(raw) >= 0xFFFF:
            raise ValueError(f"string exceeds u16 length: {len(raw)}")
        entry_span = (len(string_pool), len(raw))
        string_pool.extend(raw)
        string_offsets[text] = entry_span
        return entry_span

    attachments_pool: list[tuple[float, float, float]] = []
    attachment_struct = struct.Struct("<fff")
    record_struct = struct.Struct("<IHIHIHBIffii2fHBfff")
    packed_records = bytearray()

    for entry in entries:
        const_off, const_len = intern_str(entry.constant_name)
        res_off, res_len = intern_str(entry.resource_id)
        if entry.default_loot_table is not None:
            loot_off, loot_len = intern_str(entry.default_loot_table)
        else:
            loot_off, loot_len = (0, 0xFFFF)
        cat_variant = "".join(
            part.title() for part in entry.mob_category.lower().split("_")
        )
        cat_tag = mob_category_tags[cat_variant]
        track_deltas = entry.constant_name not in track_delta_exclusions
        flags = (
            (1 if entry.allowed_in_peaceful else 0)
            | (2 if entry.only_op_can_set_nbt else 0)
            | (4 if entry.serializable else 0)
            | (8 if entry.can_summon else 0)
            | (16 if entry.fire_immune else 0)
            | (32 if track_deltas else 0)
            | (64 if entry.can_spawn_far_from_player else 0)
            | (128 if entry.eye_height is not None else 0)
            | (256 if entry.name_tag_offset is not None else 0)
            | (512 if entry.resource_id in arrows else 0)
            | (1024 if entry.resource_id in redirectable_projectiles else 0)
            | (2048 if entry.resource_id in deflects_projectiles else 0)
            | (4096 if entry.resource_id in powder_snow_walkable_mobs else 0)
            | (8192 if entry.resource_id in freeze_immune_entity_types else 0)
            | (16384 if entry.resource_id in non_controlling_riders else 0)
            | (32768 if entry.resource_id in can_float_while_ridden else 0)
            | (65536 if entry.resource_id in can_equip_saddle else 0)
            | (131072 if entry.resource_id in can_wear_horse_armor else 0)
        )
        if entry.passenger_attachments:
            pass_start = len(attachments_pool)
            pass_len = len(entry.passenger_attachments)
            if pass_start > 0xFFFF or pass_len > 0xFF:
                raise ValueError("passenger attachments exceed packed bounds")
            for pt in entry.passenger_attachments:
                attachments_pool.append((pt[0], pt[1], pt[2]))
        else:
            pass_start = 0
            pass_len = 0
        veh_x, veh_y, veh_z = entry.vehicle_attachment
        packed_records.extend(
            record_struct.pack(
                const_off,
                const_len,
                res_off,
                res_len,
                loot_off,
                loot_len,
                cat_tag,
                flags,
                entry.width,
                entry.height,
                entry.client_tracking_range,
                entry.update_interval,
                0.0 if entry.eye_height is None else entry.eye_height,
                0.0 if entry.name_tag_offset is None else entry.name_tag_offset,
                pass_start,
                pass_len,
                veh_x,
                veh_y,
                veh_z,
            )
        )

    attachments_blob = bytearray()
    for pt in attachments_pool:
        attachments_blob.extend(attachment_struct.pack(*pt))

    attachments_offset = len(string_pool)
    records_offset = attachments_offset + len(attachments_blob)
    blob = bytes(string_pool + attachments_blob + packed_records)

    lines.extend(
        (
            "",
            'const BYTES: &[u8] = include_bytes!("../data/entity_types_26_1.bin");',
            f"const ENTITY_COUNT: usize = {len(entries)};",
            f"const ATTACHMENT_COUNT: usize = {len(attachments_pool)};",
            f"const ATTACHMENTS_OFFSET: usize = {attachments_offset};",
            f"const RECORDS_OFFSET: usize = {records_offset};",
            f"const RECORD_STRIDE: usize = {record_struct.size};",
            "const STRINGS_END: usize = ATTACHMENTS_OFFSET;",
            "",
            "const _: () = {",
            "    let mut i = 0;",
            "    while i + 8 <= STRINGS_END {",
            "        let word = u64::from_ne_bytes([",
            "            BYTES[i],",
            "            BYTES[i + 1],",
            "            BYTES[i + 2],",
            "            BYTES[i + 3],",
            "            BYTES[i + 4],",
            "            BYTES[i + 5],",
            "            BYTES[i + 6],",
            "            BYTES[i + 7],",
            "        ]);",
            "        assert!(word & 0x8080_8080_8080_8080 == 0);",
            "        i += 8;",
            "    }",
            "    while i < STRINGS_END {",
            "        assert!(BYTES[i] < 0x80);",
            "        i += 1;",
            "    }",
            "};",
            "",
            "macro_rules! u16_at {",
            "    ($offset:expr) => {{",
            "        let off = $offset;",
            "        u16::from_le_bytes([BYTES[off], BYTES[off + 1]])",
            "    }};",
            "}",
            "",
            "macro_rules! u32_at {",
            "    ($offset:expr) => {{",
            "        let off = $offset;",
            "        u32::from_le_bytes([",
            "            BYTES[off],",
            "            BYTES[off + 1],",
            "            BYTES[off + 2],",
            "            BYTES[off + 3],",
            "        ])",
            "    }};",
            "}",
            "",
            "macro_rules! i32_at {",
            "    ($offset:expr) => {{",
            "        let off = $offset;",
            "        i32::from_le_bytes([",
            "            BYTES[off],",
            "            BYTES[off + 1],",
            "            BYTES[off + 2],",
            "            BYTES[off + 3],",
            "        ])",
            "    }};",
            "}",
            "",
            "macro_rules! f32_at {",
            "    ($offset:expr) => {{",
            "        f32::from_bits(u32_at!($offset))",
            "    }};",
            "}",
            "",
            "macro_rules! str_at {",
            "    ($offset:expr, $len:expr) => {{",
            "        let off: usize = $offset;",
            "        let len: usize = $len;",
            "        assert!(off + len <= STRINGS_END);",
            "        // SAFETY: `BYTES[..STRINGS_END]` is verified ASCII above and `off + len`",
            "        // is bounded within `BYTES`.",
            "        unsafe {",
            "            core::str::from_utf8_unchecked(core::slice::from_raw_parts(",
            "                BYTES.as_ptr().add(off),",
            "                len,",
            "            ))",
            "        }",
            "    }};",
            "}",
            "",
            "macro_rules! opt_str_at {",
            "    ($offset:expr, $len:expr) => {{",
            "        let len_u16: u16 = $len;",
            "        if len_u16 == u16::MAX {",
            "            None",
            "        } else {",
            "            Some(str_at!($offset, len_u16 as usize))",
            "        }",
            "    }};",
            "}",
            "",
            "const MOB_CATEGORY_TABLE: [VanillaEntityMobCategory; 8] = [",
            "    VanillaEntityMobCategory::Ambient,",
            "    VanillaEntityMobCategory::Axolotls,",
            "    VanillaEntityMobCategory::Creature,",
            "    VanillaEntityMobCategory::Misc,",
            "    VanillaEntityMobCategory::Monster,",
            "    VanillaEntityMobCategory::UndergroundWaterCreature,",
            "    VanillaEntityMobCategory::WaterAmbient,",
            "    VanillaEntityMobCategory::WaterCreature,",
            "];",
            "",
            "const fn decode_passenger_attachments() -> [[f32; 3]; ATTACHMENT_COUNT] {",
            "    let mut out = [[0.0_f32; 3]; ATTACHMENT_COUNT];",
            "    let mut idx = 0;",
            "    while idx < ATTACHMENT_COUNT {",
            "        let base = ATTACHMENTS_OFFSET + idx * 12;",
            "        out[idx] = [f32_at!(base), f32_at!(base + 4), f32_at!(base + 8)];",
            "        idx += 1;",
            "    }",
            "    out",
            "}",
            "",
            "static DECODED_PASSENGER_ATTACHMENTS: [[f32; 3]; ATTACHMENT_COUNT] =",
            "    decode_passenger_attachments();",
            "",
            "macro_rules! slice_attachments {",
            "    ($start:expr, $len:expr) => {{",
            "        let start: usize = $start;",
            "        let len: usize = $len;",
            "        if len == 0 {",
            "            &[]",
            "        } else {",
            "            assert!(start + len <= ATTACHMENT_COUNT);",
            "            // SAFETY: `start + len <= ATTACHMENT_COUNT` bounds the slice within",
            "            // `DECODED_PASSENGER_ATTACHMENTS`.",
            "            unsafe {",
            "                core::slice::from_raw_parts(",
            "                    DECODED_PASSENGER_ATTACHMENTS.as_ptr().add(start),",
            "                    len,",
            "                )",
            "            }",
            "        }",
            "    }};",
            "}",
            "",
            "const EMPTY_ENTITY_ENTRY: EntityTypeCatalogEntry = EntityTypeCatalogEntry {",
            "    id: 0,",
            '    constant_name: "",',
            '    resource_id: "",',
            "    default_loot_table: None,",
            "    mob_category: VanillaEntityMobCategory::Misc,",
            "    allowed_in_peaceful: false,",
            "    only_op_can_set_nbt: false,",
            "    serializable: false,",
            "    can_summon: false,",
            "    fire_immune: false,",
            "    track_deltas: false,",
            "    width: 0.0,",
            "    height: 0.0,",
            "    client_tracking_range: 0,",
            "    update_interval: 0,",
            "    can_spawn_far_from_player: false,",
            "    eye_height: None,",
            "    name_tag_offset: None,",
            "    passenger_attachments: &[],",
            "    vehicle_attachment: [0.0; 3],",
            "    in_arrows_tag: false,",
            "    redirectable_projectile: false,",
            "    deflects_projectiles: false,",
            "    powder_snow_walkable: false,",
            "    freeze_immune: false,",
            "    non_controlling_rider: false,",
            "    can_float_while_ridden: false,",
            "    can_equip_saddle: false,",
            "    can_wear_horse_armor: false,",
            "};",
            "",
            "const fn decode_entity_types() -> [EntityTypeCatalogEntry; ENTITY_COUNT] {",
            "    let mut out = [EMPTY_ENTITY_ENTRY; ENTITY_COUNT];",
            "    let mut idx = 0;",
            "    while idx < ENTITY_COUNT {",
            "        let base = RECORDS_OFFSET + idx * RECORD_STRIDE;",
            "        let flags = u32_at!(base + 19);",
            "        out[idx] = EntityTypeCatalogEntry {",
            "            id: idx as i32,",
            "            constant_name: str_at!(u32_at!(base) as usize, u16_at!(base + 4) as usize),",
            "            resource_id: str_at!(u32_at!(base + 6) as usize, u16_at!(base + 10) as usize),",
            "            default_loot_table: opt_str_at!(u32_at!(base + 12) as usize, u16_at!(base + 16)),",
            "            mob_category: MOB_CATEGORY_TABLE[BYTES[base + 18] as usize],",
            "            allowed_in_peaceful: (flags & (1 << 0)) != 0,",
            "            only_op_can_set_nbt: (flags & (1 << 1)) != 0,",
            "            serializable: (flags & (1 << 2)) != 0,",
            "            can_summon: (flags & (1 << 3)) != 0,",
            "            fire_immune: (flags & (1 << 4)) != 0,",
            "            track_deltas: (flags & (1 << 5)) != 0,",
            "            width: f32_at!(base + 23),",
            "            height: f32_at!(base + 27),",
            "            client_tracking_range: i32_at!(base + 31),",
            "            update_interval: i32_at!(base + 35),",
            "            can_spawn_far_from_player: (flags & (1 << 6)) != 0,",
            "            eye_height: if (flags & (1 << 7)) != 0 { Some(f32_at!(base + 39)) } else { None },",
            "            name_tag_offset: if (flags & (1 << 8)) != 0 {",
            "                Some(f32_at!(base + 43))",
            "            } else {",
            "                None",
            "            },",
            "            passenger_attachments: slice_attachments!(",
            "                u16_at!(base + 47) as usize,",
            "                BYTES[base + 49] as usize",
            "            ),",
            "            vehicle_attachment: [f32_at!(base + 50), f32_at!(base + 54), f32_at!(base + 58)],",
            "            in_arrows_tag: (flags & (1 << 9)) != 0,",
            "            redirectable_projectile: (flags & (1 << 10)) != 0,",
            "            deflects_projectiles: (flags & (1 << 11)) != 0,",
            "            powder_snow_walkable: (flags & (1 << 12)) != 0,",
            "            freeze_immune: (flags & (1 << 13)) != 0,",
            "            non_controlling_rider: (flags & (1 << 14)) != 0,",
            "            can_float_while_ridden: (flags & (1 << 15)) != 0,",
            "            can_equip_saddle: (flags & (1 << 16)) != 0,",
            "            can_wear_horse_armor: (flags & (1 << 17)) != 0,",
            "        };",
            "        idx += 1;",
            "    }",
            "    out",
            "}",
            "",
            "static DECODED_ENTITY_TYPES: [EntityTypeCatalogEntry; ENTITY_COUNT] =",
            "    decode_entity_types();",
            "pub const VANILLA_ENTITY_TYPES: &[EntityTypeCatalogEntry] = &DECODED_ENTITY_TYPES;",
            "",
            "pub fn vanilla_entity_type(id: i32) -> Option<&'static EntityTypeCatalogEntry> {",
            "    VANILLA_ENTITY_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "#[inline]",
            "pub fn vanilla_entity_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static EntityTypeCatalogEntry> {",
            "    crate::lookup_tables::entity_type_for_resource_id(resource_id)",
            "}",
            "",
            "#[inline]",
            "pub fn vanilla_entity_type_id_for_resource_id(resource_id: &str) -> Option<i32> {",
            "    crate::lookup_tables::entity_type_id_for_resource_id(resource_id)",
            "}",
            "",
            "pub fn vanilla_entity_resource_id_for_type_id(entity_type_id: i32) -> Option<String> {",
            "    Some(vanilla_entity_type(entity_type_id)?.resource_id.to_string())",
            "}",
            "",
            "pub fn vanilla_entity_type_allowed_in_peaceful(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.allowed_in_peaceful)",
            "        .unwrap_or(true)",
            "}",
            "",
            "pub fn vanilla_entity_type_only_op_can_set_nbt(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.only_op_can_set_nbt)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_is_serializable(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.serializable)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_can_summon(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.can_summon)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_eye_height(entity_type_id: i32) -> Option<f32> {",
            "    vanilla_entity_type(entity_type_id)?.eye_height",
            "}",
            "",
            "pub fn vanilla_entity_type_name_tag_offset(entity_type_id: i32) -> Option<f32> {",
            "    vanilla_entity_type(entity_type_id)?.name_tag_offset",
            "}",
            "",
            "pub fn vanilla_entity_type_dimensions(entity_type_id: i32) -> Option<[f32; 2]> {",
            "    let entry = vanilla_entity_type(entity_type_id)?;",
            "    Some([entry.width, entry.height])",
            "}",
            "",
            "pub fn vanilla_entity_type_passenger_attachments(",
            "    entity_type_id: i32,",
            ") -> Option<&'static [[f32; 3]]> {",
            "    let points = vanilla_entity_type(entity_type_id)?.passenger_attachments;",
            "    (!points.is_empty()).then_some(points)",
            "}",
            "",
            "pub fn vanilla_entity_type_vehicle_attachment(",
            "    entity_type_id: i32,",
            ") -> Option<[f32; 3]> {",
            "    Some(vanilla_entity_type(entity_type_id)?.vehicle_attachment)",
            "}",
            "",
            "pub fn vanilla_entity_type_can_equip_saddle(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.can_equip_saddle)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_is_arrow_tagged(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.in_arrows_tag)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_is_redirectable_projectile(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.redirectable_projectile)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_deflects_projectiles(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.deflects_projectiles)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_is_powder_snow_walkable(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.powder_snow_walkable)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_is_fire_immune(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.fire_immune)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_is_freeze_immune(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.freeze_immune)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_is_non_controlling_rider(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.non_controlling_rider)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_can_float_while_ridden(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.can_float_while_ridden)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_entity_type_can_wear_horse_armor(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.can_wear_horse_armor)",
            "        .unwrap_or(false)",
            "}",
            "",
            "/// Chunk radius vanilla tracks this entity type over (`EntityType.clientTrackingRange`).",
            "pub fn vanilla_entity_type_client_tracking_range(entity_type_id: i32) -> Option<i32> {",
            "    Some(vanilla_entity_type(entity_type_id)?.client_tracking_range)",
            "}",
            "",
            "/// Tick period between tracked-entity updates (`EntityType.updateInterval`).",
            "pub fn vanilla_entity_type_update_interval(entity_type_id: i32) -> Option<i32> {",
            "    Some(vanilla_entity_type(entity_type_id)?.update_interval)",
            "}",
            "",
            "pub fn vanilla_entity_type_can_spawn_far_from_player(entity_type_id: i32) -> bool {",
            "    vanilla_entity_type(entity_type_id)",
            "        .map(|entry| entry.can_spawn_far_from_player)",
            "        .unwrap_or(false)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn entity_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_ENTITY_TYPES.len(), 157);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (id, entry) in VANILLA_ENTITY_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| !entry.allowed_in_peaceful).count(), 37);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.mob_category == VanillaEntityMobCategory::Monster).count(), 44);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.only_op_can_set_nbt).count(), 3);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| !entry.serializable).count(), 4);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| !entry.can_summon).count(), 2);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.fire_immune).count(), 15);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| !entry.track_deltas).count(), 10);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.eye_height.is_some()).count(), 101);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.name_tag_offset.is_some()).count(), 1);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.width >= 0.0 && entry.height >= 0.0).count(), 157);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| !entry.passenger_attachments.is_empty()).count(), 57);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.vehicle_attachment != [0.0; 3]).count(), 24);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.in_arrows_tag).count(), 2);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.redirectable_projectile).count(), 3);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.deflects_projectiles).count(), 1);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.powder_snow_walkable).count(), 4);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.freeze_immune).count(), 4);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.non_controlling_rider).count(), 2);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.can_float_while_ridden).count(), 6);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.can_equip_saddle).count(), 11);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.can_wear_horse_armor).count(), 2);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.client_tracking_range == 10).count(), 74);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.client_tracking_range == 8).count(), 49);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.client_tracking_range == 4).count(), 23);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.client_tracking_range > 10).count(), 5);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.update_interval == 3).count(), 120);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.update_interval < 3).count(), 7);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.update_interval == i32::MAX).count(), 7);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.can_spawn_far_from_player).count(), 105);",
            "        assert_eq!(VANILLA_ENTITY_TYPES.iter().filter(|entry| entry.client_tracking_range >= 0 && entry.update_interval >= 1).count(), 157);",
            "    }",
            "",
            "    #[test]",
            "    fn entity_delta_tracking_follows_vanilla_exclusions() {",
            "        // EntityType.java:1556-1566",
            "        assert!(!vanilla_entity_type(VANILLA_ENTITY_TYPE_PLAYER_ID).unwrap().track_deltas);",
            "        assert!(vanilla_entity_type(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap().track_deltas);",
            "    }",
            "",
            "    #[test]",
            "    fn mob_category_serialized_names_preserve_vanilla_enum_order() {",
            "        assert_eq!(VANILLA_ENTITY_MOB_CATEGORIES.len(), 8);",
            "        assert_eq!(VANILLA_ENTITY_MOB_CATEGORIES[0], (VanillaEntityMobCategory::Monster, \"monster\"));",
            "        assert_eq!(VANILLA_ENTITY_MOB_CATEGORIES[7], (VanillaEntityMobCategory::Misc, \"misc\"));",
            "        assert_eq!(vanilla_entity_mob_category_for_serialized_name(\"axolotls\"), Some(VanillaEntityMobCategory::Axolotls));",
            "        assert_eq!(vanilla_entity_mob_category_for_serialized_name(\"missing\"), None);",
            "    }",
            "",
            "    #[test]",
            "    fn entity_tracking_facts_follow_the_vanilla_builder_chain() {",
            "        // EntityType.java:374-376 omits both setters, so DOLPHIN keeps the",
            "        // EntityType.java:1615-1616 builder defaults of 5 chunks and 3 ticks.",
            "        assert_eq!(",
            "            vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_DOLPHIN_ID),",
            "            Some(5)",
            "        );",
            "        assert_eq!(",
            "            vanilla_entity_type_update_interval(VANILLA_ENTITY_TYPE_DOLPHIN_ID),",
            "            Some(3)",
            "        );",
            "        // EntityType.java:1163-1173 tracks PLAYER over 32 chunks every 2 ticks.",
            "        assert_eq!(",
            "            vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_PLAYER_ID),",
            "            Some(32)",
            "        );",
            "        assert_eq!(",
            "            vanilla_entity_type_update_interval(VANILLA_ENTITY_TYPE_PLAYER_ID),",
            "            Some(2)",
            "        );",
            "        // EntityType.java:701 gives MARKER a zero chunk range and",
            "        // EntityType.java:223 pins AREA_EFFECT_CLOUD to Integer.MAX_VALUE ticks.",
            "        assert_eq!(",
            "            vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_MARKER_ID),",
            "            Some(0)",
            "        );",
            "        assert_eq!(",
            "            vanilla_entity_type_update_interval(VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID),",
            "            Some(i32::MAX)",
            "        );",
            "        assert_eq!(vanilla_entity_type_client_tracking_range(-1), None);",
            "        assert_eq!(vanilla_entity_type_update_interval(-1), None);",
            "        // EntityType.java:1630 seeds canSpawnFarFromPlayer for CREATURE and MISC,",
            "        // while EntityType.java:814 and EntityType.java:863 force it on for the",
            "        // only two MobCategory.MONSTER types that carry it.",
            "        assert!(vanilla_entity_type_can_spawn_far_from_player(",
            "            VANILLA_ENTITY_TYPE_PILLAGER_ID",
            "        ));",
            "        assert!(vanilla_entity_type_can_spawn_far_from_player(",
            "            VANILLA_ENTITY_TYPE_SHULKER_ID",
            "        ));",
            "        assert!(vanilla_entity_type_can_spawn_far_from_player(",
            "            VANILLA_ENTITY_TYPE_COW_ID",
            "        ));",
            "        assert!(!vanilla_entity_type_can_spawn_far_from_player(",
            "            VANILLA_ENTITY_TYPE_ZOMBIE_ID",
            "        ));",
            "        assert!(!vanilla_entity_type_can_spawn_far_from_player(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn entity_type_equipment_memberships_come_from_vanilla_tags() {",
            "        assert!(vanilla_entity_type_can_equip_saddle(VANILLA_ENTITY_TYPE_HORSE_ID));",
            "        assert!(vanilla_entity_type_can_equip_saddle(VANILLA_ENTITY_TYPE_PIG_ID));",
            "        assert!(vanilla_entity_type_can_equip_saddle(VANILLA_ENTITY_TYPE_STRIDER_ID));",
            "        assert!(!vanilla_entity_type_can_equip_saddle(VANILLA_ENTITY_TYPE_LLAMA_ID));",
            "        assert!(!vanilla_entity_type_can_equip_saddle(-1));",
            "        assert!(vanilla_entity_type_can_wear_horse_armor(VANILLA_ENTITY_TYPE_HORSE_ID));",
            "        assert!(vanilla_entity_type_can_wear_horse_armor(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID));",
            "        assert!(!vanilla_entity_type_can_wear_horse_armor(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID));",
            "        assert!(!vanilla_entity_type_can_wear_horse_armor(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn arrow_membership_comes_from_the_vanilla_entity_type_tag() {",
            "        assert!(vanilla_entity_type_is_arrow_tagged(VANILLA_ENTITY_TYPE_ARROW_ID));",
            "        assert!(vanilla_entity_type_is_arrow_tagged(VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID));",
            "        assert!(!vanilla_entity_type_is_arrow_tagged(VANILLA_ENTITY_TYPE_TRIDENT_ID));",
            "        assert!(!vanilla_entity_type_is_arrow_tagged(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn redirectable_projectile_membership_comes_from_the_vanilla_tag() {",
            "        assert!(vanilla_entity_type_is_redirectable_projectile(VANILLA_ENTITY_TYPE_FIREBALL_ID));",
            "        assert!(vanilla_entity_type_is_redirectable_projectile(VANILLA_ENTITY_TYPE_WIND_CHARGE_ID));",
            "        assert!(vanilla_entity_type_is_redirectable_projectile(VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID));",
            "        assert!(!vanilla_entity_type_is_redirectable_projectile(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));",
            "        assert!(!vanilla_entity_type_is_redirectable_projectile(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn deflects_projectiles_membership_comes_from_the_vanilla_tag() {",
            "        assert!(vanilla_entity_type_deflects_projectiles(VANILLA_ENTITY_TYPE_BREEZE_ID));",
            "        assert!(!vanilla_entity_type_deflects_projectiles(VANILLA_ENTITY_TYPE_WIND_CHARGE_ID));",
            "        assert!(!vanilla_entity_type_deflects_projectiles(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn non_controlling_rider_membership_comes_from_the_vanilla_tag() {",
            "        assert!(vanilla_entity_type_is_non_controlling_rider(VANILLA_ENTITY_TYPE_SLIME_ID));",
            "        assert!(vanilla_entity_type_is_non_controlling_rider(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID));",
            "        assert!(!vanilla_entity_type_is_non_controlling_rider(VANILLA_ENTITY_TYPE_ZOMBIE_ID));",
            "        assert!(!vanilla_entity_type_is_non_controlling_rider(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn can_float_while_ridden_membership_comes_from_the_vanilla_tag() {",
            "        assert!(vanilla_entity_type_can_float_while_ridden(VANILLA_ENTITY_TYPE_HORSE_ID));",
            "        assert!(vanilla_entity_type_can_float_while_ridden(VANILLA_ENTITY_TYPE_CAMEL_ID));",
            "        assert!(!vanilla_entity_type_can_float_while_ridden(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID));",
            "        assert!(!vanilla_entity_type_can_float_while_ridden(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn powder_snow_walkable_membership_comes_from_the_vanilla_tag() {",
            "        assert!(vanilla_entity_type_is_powder_snow_walkable(VANILLA_ENTITY_TYPE_RABBIT_ID));",
            "        assert!(vanilla_entity_type_is_powder_snow_walkable(VANILLA_ENTITY_TYPE_ENDERMITE_ID));",
            "        assert!(vanilla_entity_type_is_powder_snow_walkable(VANILLA_ENTITY_TYPE_SILVERFISH_ID));",
            "        assert!(vanilla_entity_type_is_powder_snow_walkable(VANILLA_ENTITY_TYPE_FOX_ID));",
            "        assert!(!vanilla_entity_type_is_powder_snow_walkable(VANILLA_ENTITY_TYPE_COW_ID));",
            "        assert!(!vanilla_entity_type_is_powder_snow_walkable(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn block_effect_immunity_facts_come_from_builder_and_vanilla_tag() {",
            "        assert!(vanilla_entity_type_is_fire_immune(VANILLA_ENTITY_TYPE_BLAZE_ID));",
            "        assert!(vanilla_entity_type_is_fire_immune(VANILLA_ENTITY_TYPE_STRIDER_ID));",
            "        assert!(!vanilla_entity_type_is_fire_immune(VANILLA_ENTITY_TYPE_COW_ID));",
            "        assert!(vanilla_entity_type_is_freeze_immune(VANILLA_ENTITY_TYPE_STRAY_ID));",
            "        assert!(vanilla_entity_type_is_freeze_immune(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID));",
            "        assert!(vanilla_entity_type_is_freeze_immune(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID));",
            "        assert!(vanilla_entity_type_is_freeze_immune(VANILLA_ENTITY_TYPE_WITHER_ID));",
            "        assert!(!vanilla_entity_type_is_freeze_immune(VANILLA_ENTITY_TYPE_COW_ID));",
            "        assert!(!vanilla_entity_type_is_fire_immune(-1));",
            "        assert!(!vanilla_entity_type_is_freeze_immune(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn operator_only_custom_data_comes_from_generated_entity_metadata() {",
            "        assert!(vanilla_entity_type_only_op_can_set_nbt(VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID));",
            "        assert!(vanilla_entity_type_only_op_can_set_nbt(VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID));",
            "        assert!(vanilla_entity_type_only_op_can_set_nbt(VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID));",
            "        assert!(!vanilla_entity_type_only_op_can_set_nbt(VANILLA_ENTITY_TYPE_PIG_ID));",
            "        assert!(!vanilla_entity_type_only_op_can_set_nbt(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn serializable_property_comes_from_generated_builder_metadata() {",
            "        assert!(vanilla_entity_type_is_serializable(VANILLA_ENTITY_TYPE_ARROW_ID));",
            "        assert!(!vanilla_entity_type_is_serializable(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));",
            "        assert!(!vanilla_entity_type_is_serializable(VANILLA_ENTITY_TYPE_PLAYER_ID));",
            "        assert!(!vanilla_entity_type_is_serializable(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn summonable_property_comes_from_generated_builder_metadata() {",
            "        assert!(vanilla_entity_type_can_summon(VANILLA_ENTITY_TYPE_ARROW_ID));",
            "        assert!(vanilla_entity_type_can_summon(VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID));",
            "        assert!(!vanilla_entity_type_can_summon(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID));",
            "        assert!(!vanilla_entity_type_can_summon(VANILLA_ENTITY_TYPE_PLAYER_ID));",
            "        assert!(!vanilla_entity_type_can_summon(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn eye_height_comes_from_generated_builder_metadata() {",
            "        assert_eq!(vanilla_entity_type_eye_height(VANILLA_ENTITY_TYPE_ALLAY_ID), Some(0.36));",
            "        assert_eq!(vanilla_entity_type_eye_height(VANILLA_ENTITY_TYPE_TADPOLE_ID), Some(0.19500001));",
            "        assert_eq!(vanilla_entity_type_eye_height(VANILLA_ENTITY_TYPE_CREEPER_ID), None);",
            "        assert_eq!(vanilla_entity_type_eye_height(-1), None);",
            "    }",
            "",
            "    #[test]",
            "    fn name_tag_offset_comes_from_generated_builder_metadata() {",
            "        assert_eq!(vanilla_entity_type_name_tag_offset(VANILLA_ENTITY_TYPE_SNIFFER_ID), Some(2.05));",
            "        assert_eq!(vanilla_entity_type_name_tag_offset(VANILLA_ENTITY_TYPE_PLAYER_ID), None);",
            "        assert_eq!(vanilla_entity_type_name_tag_offset(-1), None);",
            "    }",
            "",
            "    #[test]",
            "    fn dimensions_come_from_generated_builder_metadata() {",
            "        assert_eq!(vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID), Some([1.375, 0.5625]));",
            "        assert_eq!(vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_PLAYER_ID), Some([0.6, 1.8]));",
            "        assert_eq!(vanilla_entity_type_dimensions(-1), None);",
            "    }",
            "",
            "    #[test]",
            "    fn passenger_attachments_come_from_generated_builder_metadata() {",
            "        assert_eq!(vanilla_entity_type_passenger_attachments(VANILLA_ENTITY_TYPE_CHICKEN_ID), Some(&[[0.0, 0.7, -0.1]][..]));",
            "        assert_eq!(vanilla_entity_type_passenger_attachments(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID).unwrap().len(), 4);",
            "        assert_eq!(vanilla_entity_type_passenger_attachments(VANILLA_ENTITY_TYPE_ARROW_ID), None);",
            "        assert_eq!(vanilla_entity_type_passenger_attachments(-1), None);",
            "        assert_eq!(vanilla_entity_type_vehicle_attachment(VANILLA_ENTITY_TYPE_PLAYER_ID), Some([0.0, 0.6, 0.0]));",
            "        assert_eq!(vanilla_entity_type_vehicle_attachment(VANILLA_ENTITY_TYPE_ZOMBIE_ID), Some([0.0, 0.7, 0.0]));",
            "        assert_eq!(vanilla_entity_type_vehicle_attachment(VANILLA_ENTITY_TYPE_TURTLE_ID), Some([0.0; 3]));",
            "        assert_eq!(vanilla_entity_type_vehicle_attachment(-1), None);",
            "    }",
            "",
            "    #[test]",
            "    fn entity_type_catalog_preserves_vanilla_order_and_compatibility_api() {",
            "        assert_eq!(VANILLA_ENTITY_TYPES.first().unwrap().resource_id, \"minecraft:acacia_boat\");",
            "        assert_eq!(VANILLA_ENTITY_TYPE_PLAYER_ID, 155);",
            "        assert_eq!(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, 156);",
            "        assert_eq!(vanilla_entity_type_id_for_resource_id(\"minecraft:zombie\"), Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID));",
            "        assert_eq!(vanilla_entity_type_id_for_resource_id(\"creeper\"), Some(VANILLA_ENTITY_TYPE_CREEPER_ID));",
            "        assert_eq!(vanilla_entity_resource_id_for_type_id(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID).as_deref(), Some(\"minecraft:fishing_bobber\"));",
            "        assert_eq!(vanilla_entity_type_id_for_resource_id(\"example:zombie\"), None);",
            "        assert_eq!(vanilla_entity_resource_id_for_type_id(-1), None);",
            "    }",
            "",
            "    #[test]",
            "    fn peaceful_allowance_comes_from_generated_builder_metadata() {",
            "        assert!(!vanilla_entity_type_allowed_in_peaceful(VANILLA_ENTITY_TYPE_ZOMBIE_ID));",
            "        assert!(!vanilla_entity_type_allowed_in_peaceful(VANILLA_ENTITY_TYPE_WITHER_ID));",
            "        assert!(vanilla_entity_type_allowed_in_peaceful(VANILLA_ENTITY_TYPE_PIG_ID));",
            "        assert!(vanilla_entity_type_allowed_in_peaceful(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID));",
            "        assert!(vanilla_entity_type_allowed_in_peaceful(-1));",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines), blob


def render_block_entity_types(entries: list[BlockEntityTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 BlockEntityType.java static register calls and OP_ONLY_CUSTOM_DATA.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct BlockEntityTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub only_op_can_set_nbt: bool,",
        "    pub provides_menu: bool,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_BLOCK_ENTITY_TYPE_{entry.constant_name}_ID: i32 = {entry_id};"
        )
    lines.extend(("", "pub const VANILLA_BLOCK_ENTITY_TYPES: &[BlockEntityTypeCatalogEntry] = &["))
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    BlockEntityTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        only_op_can_set_nbt: {str(entry.only_op_can_set_nbt).lower()},",
                f"        provides_menu: {str(entry.provides_menu).lower()},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_block_entity_type(",
            "    type_id: i32,",
            ") -> Option<&'static BlockEntityTypeCatalogEntry> {",
            "    VANILLA_BLOCK_ENTITY_TYPES.get(usize::try_from(type_id).ok()?)",
            "}",
            "",
            "#[inline]",
            "pub fn vanilla_block_entity_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static BlockEntityTypeCatalogEntry> {",
            "    crate::lookup_tables::block_entity_type_for_resource_id(resource_id)",
            "}",
            "",
            "#[inline]",
            "pub fn vanilla_block_entity_type_id_for_resource_id(resource_id: &str) -> Option<i32> {",
            "    crate::lookup_tables::block_entity_type_id_for_resource_id(resource_id)",
            "}",
            "",
            "pub fn vanilla_block_entity_resource_id_for_type_id(type_id: i32) -> Option<&'static str> {",
            "    Some(vanilla_block_entity_type(type_id)?.resource_id)",
            "}",
            "",
            "pub fn vanilla_block_entity_type_only_op_can_set_nbt(type_id: i32) -> bool {",
            "    vanilla_block_entity_type(type_id)",
            "        .map(|entry| entry.only_op_can_set_nbt)",
            "        .unwrap_or(false)",
            "}",
            "",
            "pub fn vanilla_block_entity_type_provides_menu(type_id: i32) -> bool {",
            "    vanilla_block_entity_type(type_id)",
            "        .map(|entry| entry.provides_menu)",
            "        .unwrap_or(false)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn block_entity_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_BLOCK_ENTITY_TYPES.len(), 49);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (id, entry) in VANILLA_BLOCK_ENTITY_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "        assert_eq!(VANILLA_BLOCK_ENTITY_TYPES.iter().filter(|entry| entry.only_op_can_set_nbt).count(), 6);",
            "    }",
            "",
            "    #[test]",
            "    fn menu_provider_block_entity_types_come_from_generated_metadata() {",
            "        assert_eq!(VANILLA_BLOCK_ENTITY_TYPES.iter().filter(|entry| entry.provides_menu).count(), 14);",
            "        for type_id in [",
            "            VANILLA_BLOCK_ENTITY_TYPE_FURNACE_ID,",
            "            VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,",
            "            VANILLA_BLOCK_ENTITY_TYPE_TRAPPED_CHEST_ID,",
            "            VANILLA_BLOCK_ENTITY_TYPE_BARREL_ID,",
            "            VANILLA_BLOCK_ENTITY_TYPE_HOPPER_ID,",
            "            VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID,",
            "            VANILLA_BLOCK_ENTITY_TYPE_CRAFTER_ID,",
            "        ] {",
            "            assert!(vanilla_block_entity_type_provides_menu(type_id));",
            "        }",
            "        assert!(!vanilla_block_entity_type_provides_menu(VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID));",
            "        assert!(!vanilla_block_entity_type_provides_menu(VANILLA_BLOCK_ENTITY_TYPE_BELL_ID));",
            "        assert!(!vanilla_block_entity_type_provides_menu(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn operator_only_custom_data_comes_from_generated_block_entity_metadata() {",
            "        assert!(vanilla_block_entity_type_only_op_can_set_nbt(VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID));",
            "        assert!(vanilla_block_entity_type_only_op_can_set_nbt(VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID));",
            "        assert!(vanilla_block_entity_type_only_op_can_set_nbt(VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID));",
            "        assert!(vanilla_block_entity_type_only_op_can_set_nbt(VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID));",
            "        assert!(vanilla_block_entity_type_only_op_can_set_nbt(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID));",
            "        assert!(vanilla_block_entity_type_only_op_can_set_nbt(VANILLA_BLOCK_ENTITY_TYPE_TRIAL_SPAWNER_ID));",
            "        assert!(!vanilla_block_entity_type_only_op_can_set_nbt(VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID));",
            "        assert!(!vanilla_block_entity_type_only_op_can_set_nbt(-1));",
            "    }",
            "",
            "    #[test]",
            "    fn block_entity_type_catalog_preserves_vanilla_order_and_compatibility_api() {",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(0), Some(\"minecraft:furnace\"));",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(9), Some(\"minecraft:mob_spawner\"));",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(45), Some(\"minecraft:vault\"));",
            "        assert_eq!(VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID, 45);",
            "        assert_eq!(vanilla_block_entity_type_id_for_resource_id(\"minecraft:mob_spawner\"), Some(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID));",
            "        assert_eq!(vanilla_block_entity_type_id_for_resource_id(\"vault\"), Some(VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID));",
            "        assert_eq!(vanilla_block_entity_type_id_for_resource_id(\"example:vault\"), None);",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(48), Some(\"minecraft:copper_golem_statue\"));",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(-1), None);",
            "        assert_eq!(vanilla_block_entity_resource_id_for_type_id(49), None);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_data_component_types(
    component_entries: list[DataComponentTypeEntry],
    predicate_entries: list[RegistryEntry],
    common_item_components: list[str],
    animal_variant_registries: list[AnimalVariantComponentRegistryEntry],
    animal_texture_variant_registries: list[AnimalVariantComponentRegistryEntry],
) -> str:
    common_item_component_ids = [
        f"VANILLA_DATA_COMPONENT_{constant_name}_TYPE_ID"
        for constant_name in common_item_components
    ]
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 DataComponents.java and DataComponentPredicates.java.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "// Wire codecs remain hand-bound in packets/data_components/catalog.rs.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct DataComponentTypeCatalogEntry {",
        "    pub type_id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub persistent: bool,",
        "}",
        "",
    ]
    for type_id, entry in enumerate(component_entries):
        lines.append(
            f"pub const VANILLA_DATA_COMPONENT_{entry.constant_name}_TYPE_ID: i32 = {type_id};"
        )
    lines.extend((
        "",
        "pub const VANILLA_COMMON_ITEM_COMPONENT_TYPE_IDS: &[i32] = &[",
    ))
    for constant_name in common_item_component_ids:
        lines.append(f"    {constant_name},")
    lines.extend((
        "];",
        "",
        "pub const VANILLA_DATA_COMPONENT_TYPES: &[DataComponentTypeCatalogEntry] = &[",
    ))
    for type_id, entry in enumerate(component_entries):
        lines.extend(
            (
                "    DataComponentTypeCatalogEntry {",
                f"        type_id: {type_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        persistent: {str(entry.persistent).lower()},",
                "    },",
            )
        )
    known_component_constants = {entry.constant_name for entry in component_entries}
    unknown_variant_components = {
        entry.component_constant_name for entry in animal_variant_registries
    } - known_component_constants
    if unknown_variant_components:
        raise ValueError(
            "animal variant registries reference unknown components: "
            f"{sorted(unknown_variant_components)}"
        )
    animal_texture_variant_components = {
        entry.component_constant_name for entry in animal_texture_variant_registries
    }
    unknown_texture_variant_components = animal_texture_variant_components - {
        entry.component_constant_name for entry in animal_variant_registries
    }
    if unknown_texture_variant_components:
        raise ValueError(
            "animal texture variant registries reference unknown animal components: "
            f"{sorted(unknown_texture_variant_components)}"
        )
    lines.extend((
        "];",
        "",
        "const VANILLA_ANIMAL_VARIANT_COMPONENT_REGISTRIES: &[(i32, &str, bool)] = &[",
    ))
    for entry in animal_variant_registries:
        lines.append(
            "    (VANILLA_DATA_COMPONENT_"
            + entry.component_constant_name
            + "_TYPE_ID, "
            + rust_string(entry.registry_resource_id)
            + ", "
            + str(entry.component_constant_name in animal_texture_variant_components).lower()
            + "),"
        )
    lines.extend((
        "];",
        "",
        "pub fn vanilla_data_component_animal_variant_registry(type_id: i32) -> Option<&'static str> {",
        "    VANILLA_ANIMAL_VARIANT_COMPONENT_REGISTRIES",
        "        .iter()",
        "        .find_map(|&(candidate, registry, _)| (candidate == type_id).then_some(registry))",
        "}",
        "",
        "pub fn vanilla_data_component_animal_variant_registries() -> impl Iterator<Item = (i32, &'static str)> {",
        "    VANILLA_ANIMAL_VARIANT_COMPONENT_REGISTRIES",
        "        .iter()",
        "        .map(|&(type_id, registry, _)| (type_id, registry))",
        "}",
        "",
        "pub fn vanilla_data_component_animal_texture_variant_registries() -> impl Iterator<Item = (i32, &'static str)> {",
        "    VANILLA_ANIMAL_VARIANT_COMPONENT_REGISTRIES",
        "        .iter()",
        "        .filter_map(|&(type_id, registry, has_texture_payload)| {",
        "            has_texture_payload.then_some((type_id, registry))",
        "        })",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct DataComponentPredicateTypeCatalogEntry {",
        "    pub type_id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
    ))
    for type_id, entry in enumerate(predicate_entries):
        lines.append(
            f"pub const VANILLA_DATA_COMPONENT_PREDICATE_{entry.constant_name}_TYPE_ID: i32 = {type_id};"
        )
    lines.extend((
        "",
        "pub const VANILLA_DATA_COMPONENT_PREDICATE_TYPES: &[DataComponentPredicateTypeCatalogEntry] = &[",
    ))
    for type_id, entry in enumerate(predicate_entries):
        lines.extend(
            (
                "    DataComponentPredicateTypeCatalogEntry {",
                f"        type_id: {type_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_data_component_type(",
            "    type_id: i32,",
            ") -> Option<&'static DataComponentTypeCatalogEntry> {",
            "    VANILLA_DATA_COMPONENT_TYPES.get(usize::try_from(type_id).ok()?)",
            "}",
            "",
            "#[inline]",
            "pub fn vanilla_data_component_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static DataComponentTypeCatalogEntry> {",
            "    crate::lookup_tables::data_component_type_for_resource_id(resource_id)",
            "}",
            "",
            "pub fn vanilla_data_component_resource_id_for_type_id(type_id: i32) -> Option<&'static str> {",
            "    Some(vanilla_data_component_type(type_id)?.resource_id)",
            "}",
            "",
            "pub fn vanilla_data_component_type_is_persistent(type_id: i32) -> bool {",
            "    vanilla_data_component_type(type_id).is_some_and(|entry| entry.persistent)",
            "}",
            "",
            "pub fn vanilla_data_component_predicate_type(",
            "    type_id: i32,",
            ") -> Option<&'static DataComponentPredicateTypeCatalogEntry> {",
            "    VANILLA_DATA_COMPONENT_PREDICATE_TYPES.get(usize::try_from(type_id).ok()?)",
            "}",
            "",
            "pub fn vanilla_data_component_predicate_resource_id(type_id: i32) -> Option<&'static str> {",
            "    Some(vanilla_data_component_predicate_type(type_id)?.resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn component_type_catalog_is_complete_contiguous_unique_and_persistent() {",
            "        assert_eq!(VANILLA_DATA_COMPONENT_TYPES.len(), 110);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (type_id, entry) in VANILLA_DATA_COMPONENT_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.type_id, type_id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "        assert_eq!(VANILLA_DATA_COMPONENT_TYPES.iter().filter(|entry| !entry.persistent).map(|entry| entry.type_id).collect::<Vec<_>>(), vec![20, 41, 48]);",
            "        assert_eq!(vanilla_data_component_type_for_resource_id(\"minecraft:custom_name\").map(|entry| entry.type_id), Some(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID));",
            "        assert_eq!(vanilla_data_component_type_for_resource_id(\"minecraft:missing\"), None);",
            "        assert_eq!(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, 37);",
            "        assert_eq!(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, 46);",
            "    }",
            "",
            "    #[test]",
            "    fn common_item_components_follow_vanilla_builder_order() {",
            f"        assert_eq!(VANILLA_COMMON_ITEM_COMPONENT_TYPE_IDS.len(), {len(common_item_component_ids)});",
            f"        assert_eq!(VANILLA_COMMON_ITEM_COMPONENT_TYPE_IDS, &[{', '.join(common_item_component_ids)}]);",
            "    }",
            "",
            "    #[test]",
            "    fn animal_variant_component_registries_follow_holder_value_types() {",
            f"        assert_eq!(VANILLA_ANIMAL_VARIANT_COMPONENT_REGISTRIES.len(), {len(animal_variant_registries)});",
            "        let actual = vanilla_data_component_animal_variant_registries().collect::<Vec<_>>();",
            "        let expected = VANILLA_ANIMAL_VARIANT_COMPONENT_REGISTRIES.iter().map(|&(type_id, registry, _)| (type_id, registry)).collect::<Vec<_>>();",
            "        assert_eq!(actual, expected);",
            "        for &(type_id, registry, _) in VANILLA_ANIMAL_VARIANT_COMPONENT_REGISTRIES {",
            "            assert_eq!(vanilla_data_component_animal_variant_registry(type_id), Some(registry));",
            "        }",
            "        assert_eq!(vanilla_data_component_animal_variant_registry(VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID), None);",
            "        assert_eq!(vanilla_data_component_animal_variant_registry(VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID), None);",
            "        assert_eq!(vanilla_data_component_animal_variant_registry(-1), None);",
            "    }",
            "",
            "    #[test]",
            "    fn animal_texture_variant_registries_follow_network_codec_dependencies() {",
            f"        assert_eq!(vanilla_data_component_animal_texture_variant_registries().count(), {len(animal_texture_variant_registries)});",
            "        for (type_id, registry) in vanilla_data_component_animal_texture_variant_registries() {",
            "            assert_eq!(vanilla_data_component_animal_variant_registry(type_id), Some(registry));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn compatibility_queries_delegate_to_component_descriptor_catalog() {",
            "        assert_eq!(vanilla_data_component_resource_id_for_type_id(0), Some(\"minecraft:custom_data\"));",
            "        assert_eq!(vanilla_data_component_resource_id_for_type_id(109), Some(\"minecraft:shulker/color\"));",
            "        assert_eq!(vanilla_data_component_resource_id_for_type_id(-1), None);",
            "        assert_eq!(vanilla_data_component_resource_id_for_type_id(110), None);",
            "        assert!(vanilla_data_component_type_is_persistent(0));",
            "        assert!(!vanilla_data_component_type_is_persistent(20));",
            "        assert!(!vanilla_data_component_type_is_persistent(41));",
            "        assert!(!vanilla_data_component_type_is_persistent(48));",
            "    }",
            "",
            "    #[test]",
            "    fn predicate_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_DATA_COMPONENT_PREDICATE_TYPES.len(), 15);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (type_id, entry) in VANILLA_DATA_COMPONENT_PREDICATE_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.type_id, type_id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "        assert_eq!(vanilla_data_component_predicate_resource_id(0), Some(\"minecraft:damage\"));",
            "        assert_eq!(vanilla_data_component_predicate_resource_id(14), Some(\"minecraft:villager/variant\"));",
            "        assert_eq!(vanilla_data_component_predicate_resource_id(-1), None);",
            "        assert_eq!(vanilla_data_component_predicate_resource_id(15), None);",
            "        assert_eq!(VANILLA_DATA_COMPONENT_PREDICATE_DAMAGE_TYPE_ID, 0);",
            "        assert_eq!(VANILLA_DATA_COMPONENT_PREDICATE_CONTAINER_TYPE_ID, 5);",
            "        assert_eq!(VANILLA_DATA_COMPONENT_PREDICATE_VILLAGER_VARIANT_TYPE_ID, 14);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_particle_types(entries: list[ParticleTypeEntry]) -> str:
    rust_family = {
        VanillaParticleCodecFamily.SIMPLE: "Simple",
        VanillaParticleCodecFamily.BLOCK: "Block",
        VanillaParticleCodecFamily.POWER: "Power",
        VanillaParticleCodecFamily.DUST: "Dust",
        VanillaParticleCodecFamily.DUST_COLOR_TRANSITION: "DustColorTransition",
        VanillaParticleCodecFamily.SPELL: "Spell",
        VanillaParticleCodecFamily.COLOR: "Color",
        VanillaParticleCodecFamily.SCULK_CHARGE: "SculkCharge",
        VanillaParticleCodecFamily.ITEM: "Item",
        VanillaParticleCodecFamily.VIBRATION: "Vibration",
        VanillaParticleCodecFamily.TRAIL: "Trail",
        VanillaParticleCodecFamily.SHRIEK: "Shriek",
    }
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ParticleTypes.java static register calls.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum VanillaParticleCodecFamily {",
        "    Simple,",
        "    Block,",
        "    Power,",
        "    Dust,",
        "    DustColorTransition,",
        "    Spell,",
        "    Color,",
        "    SculkCharge,",
        "    Item,",
        "    Vibration,",
        "    Trail,",
        "    Shriek,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ParticleTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub override_limiter: bool,",
        "    pub codec_family: VanillaParticleCodecFamily,",
        "}",
        "",
    ]
    for type_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_PARTICLE_TYPE_{entry.constant_name}_ID: i32 = {type_id};"
        )
    lines.extend(("", "pub const VANILLA_PARTICLE_TYPES: &[ParticleTypeCatalogEntry] = &["))
    for type_id, entry in enumerate(entries):
        lines.extend(
            (
                "    ParticleTypeCatalogEntry {",
                f"        id: {type_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        override_limiter: {str(entry.override_limiter).lower()},",
                f"        codec_family: VanillaParticleCodecFamily::{rust_family[entry.codec_family]},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_particle_type(id: i32) -> Option<&'static ParticleTypeCatalogEntry> {",
            "    VANILLA_PARTICLE_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "#[inline]",
            "pub fn vanilla_particle_type_for_resource_id(resource_id: &str) -> Option<&'static ParticleTypeCatalogEntry> {",
            "    crate::lookup_tables::particle_type_for_resource_id(resource_id)",
            "}",
            "",
            "const fn particle_resource_id_bytes_eq(left: &[u8], right: &[u8]) -> bool {",
            "    if left.len() != right.len() {",
            "        return false;",
            "    }",
            "    let mut index = 0;",
            "    while index < left.len() {",
            "        if left[index] != right[index] {",
            "            return false;",
            "        }",
            "        index += 1;",
            "    }",
            "    true",
            "}",
            "",
            "pub const fn vanilla_particle_type_id_for_resource_id(resource_id: &str) -> Option<i32> {",
            "    let mut index = 0;",
            "    while index < VANILLA_PARTICLE_TYPES.len() {",
            "        if particle_resource_id_bytes_eq(",
            "            VANILLA_PARTICLE_TYPES[index].resource_id.as_bytes(),",
            "            resource_id.as_bytes(),",
            "        ) {",
            "            return Some(index as i32);",
            "        }",
            "        index += 1;",
            "    }",
            "    None",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn particle_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_PARTICLE_TYPES.len(), 117);",
            "        let mut constant_names = HashSet::new();",
            "        let mut resource_ids = HashSet::new();",
            "        for (id, entry) in VANILLA_PARTICLE_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constant_names.insert(entry.constant_name));",
            "            assert!(resource_ids.insert(entry.resource_id));",
            "            assert!(entry.resource_id.starts_with(\"minecraft:\"));",
            "        }",
            "        assert_eq!(VANILLA_PARTICLE_TYPES.iter().filter(|entry| entry.override_limiter).count(), 28);",
            "        let counts = |family| VANILLA_PARTICLE_TYPES.iter().filter(|entry| entry.codec_family == family).count();",
            "        assert_eq!(counts(VanillaParticleCodecFamily::Simple), 99);",
            "        assert_eq!(counts(VanillaParticleCodecFamily::Block), 5);",
            "        assert_eq!(counts(VanillaParticleCodecFamily::Power), 1);",
            "        assert_eq!(counts(VanillaParticleCodecFamily::Dust), 1);",
            "        assert_eq!(counts(VanillaParticleCodecFamily::DustColorTransition), 1);",
            "        assert_eq!(counts(VanillaParticleCodecFamily::Spell), 2);",
            "        assert_eq!(counts(VanillaParticleCodecFamily::Color), 3);",
            "        for family in [VanillaParticleCodecFamily::SculkCharge, VanillaParticleCodecFamily::Item, VanillaParticleCodecFamily::Vibration, VanillaParticleCodecFamily::Trail, VanillaParticleCodecFamily::Shriek] { assert_eq!(counts(family), 1); }",
            "    }",
            "",
            "    #[test]",
            "    fn particle_type_catalog_preserves_vanilla_order_metadata_and_const_lookup() {",
            "        assert_eq!(vanilla_particle_type(0).unwrap().resource_id, \"minecraft:angry_villager\");",
            "        assert!(vanilla_particle_type(2).unwrap().override_limiter);",
            "        assert_eq!(vanilla_particle_type(48).unwrap().resource_id, \"minecraft:vibration\");",
            "        assert_eq!(vanilla_particle_type(116).unwrap().resource_id, \"minecraft:firefly\");",
            "        const FIREWORK_ID: Option<i32> = vanilla_particle_type_id_for_resource_id(\"minecraft:firework\");",
            "        assert_eq!(FIREWORK_ID, Some(30));",
            "        assert_eq!(vanilla_particle_type_id_for_resource_id(\"minecraft:missing\"), None);",
            "        assert_eq!(vanilla_particle_type(-1), None);",
            "        assert_eq!(vanilla_particle_type(117), None);",
            "        assert_eq!(VANILLA_PARTICLE_TYPE_ANGRY_VILLAGER_ID, 0);",
            "        assert_eq!(VANILLA_PARTICLE_TYPE_VIBRATION_ID, 48);",
            "        assert_eq!(VANILLA_PARTICLE_TYPE_FIREFLY_ID, 116);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_activity_types(entries: list[RegistryEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-core-registry-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Activity.java static register calls.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-core-registry-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ActivityCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
        "pub const VANILLA_ACTIVITIES: &[ActivityCatalogEntry] = &[",
    ]
    for entry_id, entry in enumerate(entries):
        lines.extend((
            "    ActivityCatalogEntry {",
            f"        id: {entry_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn vanilla_activity_for_resource_id(resource_id: &str) -> Option<&'static ActivityCatalogEntry> {",
        "    VANILLA_ACTIVITIES.iter().find(|entry| entry.resource_id == resource_id)",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::collections::HashSet;",
        "    use super::*;",
        "    #[test]",
        "    fn activity_catalog_is_complete_ordered_and_unique() {",
        "        assert_eq!(VANILLA_ACTIVITIES.len(), 26);",
        "        let mut names = HashSet::new();",
        "        let mut resources = HashSet::new();",
        "        for (id, entry) in VANILLA_ACTIVITIES.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert!(names.insert(entry.constant_name));",
        "            assert!(resources.insert(entry.resource_id));",
        "        }",
        "        assert_eq!(VANILLA_ACTIVITIES[0].resource_id, \"minecraft:core\");",
        "        assert_eq!(VANILLA_ACTIVITIES[25].resource_id, \"minecraft:dig\");",
        "        assert_eq!(vanilla_activity_for_resource_id(\"minecraft:panic\").unwrap().id, 6);",
        "        assert!(vanilla_activity_for_resource_id(\"example:unknown\").is_none());",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def format_rust(source: str) -> str:
    formatted = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input=source,
        text=True,
        capture_output=True,
        check=True,
    )
    return formatted.stdout


def generated_files(
    source_root: Path,
    damage_types_output: Path,
    entity_types_output: Path,
    entity_types_bin_output: Path,
    block_entity_types_output: Path,
    data_component_types_output: Path,
    particle_types_output: Path,
    activity_types_output: Path,
    number_format_types_output: Path,
    poi_types_output: Path,
    note_block_instruments_output: Path,
) -> dict[Path, str | bytes]:
    def parse(relative_path: Path):
        path = source_root / relative_path
        return parse_source(relative_path.as_posix(), path.read_bytes())

    damage_types_source = parse(DAMAGE_TYPES_SOURCE)
    entity_types_source = parse(ENTITY_TYPES_SOURCE)
    mob_categories_source = parse(MOB_CATEGORIES_SOURCE)
    avatar_source = parse(AVATAR_SOURCE)
    entity_types = entity_type_entries(
        entity_types_source,
        named_vec3_constants={
            "Avatar.DEFAULT_VEHICLE_ATTACHMENT": vec3_constant(
                avatar_source, "DEFAULT_VEHICLE_ATTACHMENT"
            )
        },
    )
    track_delta_exclusions = entity_type_track_delta_exclusions(entity_types_source)
    entity_type_resources = {entry.resource_id for entry in entity_types}
    entity_type_tags = load_tag_resources(source_root, ENTITY_TYPE_TAGS_DIRECTORY)
    arrows = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:arrows",
            2,
            registry_name="entity type",
        )
    )
    redirectable_projectiles = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:redirectable_projectile",
            3,
            registry_name="entity type",
        )
    )
    deflects_projectiles = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:deflects_projectiles",
            1,
            registry_name="entity type",
        )
    )
    powder_snow_walkable_mobs = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:powder_snow_walkable_mobs",
            4,
            registry_name="entity type",
        )
    )
    freeze_immune_entity_types = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:freeze_immune_entity_types",
            4,
            registry_name="entity type",
        )
    )
    non_controlling_riders = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:non_controlling_rider",
            2,
            registry_name="entity type",
        )
    )
    can_float_while_ridden = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:can_float_while_ridden",
            6,
            registry_name="entity type",
        )
    )
    can_equip_saddle = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:can_equip_saddle",
            11,
            registry_name="entity type",
        )
    )
    can_wear_horse_armor = frozenset(
        resolve_tag_values(
            entity_type_tags,
            "minecraft:can_wear_horse_armor",
            2,
            registry_name="entity type",
        )
    )
    unknown_tag_values = (
        arrows
        | redirectable_projectiles
        | deflects_projectiles
        | powder_snow_walkable_mobs
        | freeze_immune_entity_types
        | non_controlling_riders
        | can_float_while_ridden
        | can_equip_saddle
        | can_wear_horse_armor
    ) - entity_type_resources
    if unknown_tag_values:
        raise ValueError(
            f"entity type tags reference unknown registry entries: {sorted(unknown_tag_values)}"
        )
    block_entity_types_source = parse(BLOCK_ENTITY_TYPES_SOURCE)
    block_entity_class_sources = [
        parse(directory / path.name)
        for directory in BLOCK_ENTITY_CLASS_DIRECTORIES
        for path in sorted((source_root / directory).glob("*.java"))
    ]
    menu_provider_classes = block_entity_menu_provider_classes(block_entity_class_sources)
    predicate_types_source = parse(DATA_COMPONENT_PREDICATE_TYPES_SOURCE)
    component_types_source = parse(DATA_COMPONENT_TYPES_SOURCE)
    registries_source = parse(REGISTRIES_SOURCE)
    component_types = data_component_type_entries(component_types_source)
    animal_variant_registries = animal_variant_component_registry_entries(
        component_types_source, registries_source
    )
    animal_variant_value_sources = {
        entry.value_type_fqcn: parse(Path(entry.value_type_fqcn.replace(".", "/") + ".java"))
        for entry in animal_variant_registries
    }
    animal_texture_variant_registries = animal_variant_texture_registry_entries(
        animal_variant_registries, animal_variant_value_sources
    )
    particle_types_source = parse(PARTICLE_TYPES_SOURCE)
    activity_types_source = parse(ACTIVITY_TYPES_SOURCE)
    number_format_types_source = parse(NUMBER_FORMAT_TYPES_SOURCE)
    poi_types_source = parse(POI_TYPES_SOURCE)
    note_block_instruments_source = parse(NOTE_BLOCK_INSTRUMENTS_SOURCE)
    sound_events_source = parse(SOUND_EVENTS_SOURCE)
    entity_types_rust, entity_types_blob = render_entity_types(
        entity_types,
        track_delta_exclusions,
        mob_category_entries(mob_categories_source),
        arrows,
        redirectable_projectiles,
        deflects_projectiles,
        powder_snow_walkable_mobs,
        freeze_immune_entity_types,
        non_controlling_riders,
        can_float_while_ridden,
        can_equip_saddle,
        can_wear_horse_armor,
    )
    return {
        damage_types_output: format_rust(
            render_damage_types(damage_type_entries(damage_types_source))
        ),
        entity_types_output: format_rust(entity_types_rust),
        entity_types_bin_output: entity_types_blob,
        block_entity_types_output: format_rust(
            render_block_entity_types(
                block_entity_type_entries(
                    block_entity_types_source, menu_provider_classes
                )
            )
        ),
        data_component_types_output: format_rust(
            render_data_component_types(
                component_types,
                data_component_predicate_type_entries(predicate_types_source),
                common_item_component_constant_names(component_types_source),
                animal_variant_registries,
                animal_texture_variant_registries,
            )
        ),
        particle_types_output: format_rust(
            render_particle_types(particle_type_entries(particle_types_source))
        ),
        activity_types_output: format_rust(
            render_activity_types(activity_entries(activity_types_source))
        ),
        number_format_types_output: format_rust(
            render_number_format_types(
                number_format_type_entries(number_format_types_source)
            )
        ),
        poi_types_output: format_rust(render_poi_types(poi_type_entries(poi_types_source, include_states=True))),
        note_block_instruments_output: format_rust(
            render_note_block_instruments(
                note_block_instrument_entries(
                    note_block_instruments_source, sound_events_source
                ),
                note_block_instrument_type_names(note_block_instruments_source),
            )
        ),
    }


def check_file(path: Path, generated: str | bytes) -> bool:
    if isinstance(generated, bytes):
        current_bytes = path.read_bytes() if path.exists() else b""
        if current_bytes == generated:
            return True
        sys.stderr.write(
            f"binary output differs: {path} "
            f"(expected {len(generated)} bytes, got {len(current_bytes)})\n"
        )
        return False
    current = path.read_text(encoding="utf-8") if path.exists() else ""
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
    outputs = generated_files(
        args.source_root,
        args.damage_types_output,
        args.entity_types_output,
        args.entity_types_bin_output,
        args.block_entity_types_output,
        args.data_component_types_output,
        args.particle_types_output,
        args.activity_types_output,
        args.number_format_types_output,
        args.poi_types_output,
        args.note_block_instruments_output,
    )
    if args.check:
        return 0 if all(check_file(path, generated) for path, generated in outputs.items()) else 1
    for path, generated in outputs.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(generated, bytes):
            path.write_bytes(generated)
        else:
            path.write_text(generated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
