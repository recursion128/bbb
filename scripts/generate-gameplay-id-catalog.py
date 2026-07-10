#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 gameplay numeric-id catalogs.

The current catalog covers attributes, mob effects, menu types, equipment slot
ordinals, and entity poses. Normal builds consume only the generated Rust
files; this maintenance tool reads local vanilla sources.
"""

from __future__ import annotations

import argparse
import difflib
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_ATTRIBUTE_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/attribute_types.rs"
DEFAULT_MOB_EFFECT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/mob_effect_types.rs"
)
DEFAULT_MENU_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/menu_types.rs"
DEFAULT_EQUIPMENT_SLOTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/equipment_slots.rs"
)
DEFAULT_ENTITY_POSES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/entity_poses.rs"
ATTRIBUTE_TYPES_SOURCE = Path(
    "net/minecraft/world/entity/ai/attributes/Attributes.java"
)
MOB_EFFECT_TYPES_SOURCE = Path("net/minecraft/world/effect/MobEffects.java")
MENU_TYPES_SOURCE = Path("net/minecraft/world/inventory/MenuType.java")
EQUIPMENT_SLOTS_SOURCE = Path("net/minecraft/world/entity/EquipmentSlot.java")
EQUIPMENT_SLOT_GROUPS_SOURCE = Path(
    "net/minecraft/world/entity/EquipmentSlotGroup.java"
)
ENTITY_POSES_SOURCE = Path("net/minecraft/world/entity/Pose.java")


@dataclass(frozen=True)
class AttributeEntry:
    constant_name: str
    resource_id: str
    description_key: str
    sentiment: str
    syncable: bool


@dataclass(frozen=True)
class MobEffectEntry:
    constant_name: str
    resource_id: str
    category: str


@dataclass(frozen=True)
class MenuTypeEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class EquipmentSlotEntry:
    constant_name: str
    slot_type: str
    index: int
    count_limit: int
    slot_id: int
    serialized_name: str


@dataclass(frozen=True)
class EquipmentSlotGroupEntry:
    constant_name: str
    group_id: int
    serialized_name: str


@dataclass(frozen=True)
class EntityPoseEntry:
    constant_name: str
    pose_id: int
    serialized_name: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument(
        "--attribute-types-output", type=Path, default=DEFAULT_ATTRIBUTE_TYPES_OUTPUT
    )
    parser.add_argument(
        "--mob-effect-types-output", type=Path, default=DEFAULT_MOB_EFFECT_TYPES_OUTPUT
    )
    parser.add_argument("--menu-types-output", type=Path, default=DEFAULT_MENU_TYPES_OUTPUT)
    parser.add_argument(
        "--equipment-slots-output", type=Path, default=DEFAULT_EQUIPMENT_SLOTS_OUTPUT
    )
    parser.add_argument(
        "--entity-poses-output", type=Path, default=DEFAULT_ENTITY_POSES_OUTPUT
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def parenthesized_body(source: str, opening_parenthesis: int) -> str:
    depth = 0
    in_string = False
    escaped = False
    for index in range(opening_parenthesis, len(source)):
        character = source[index]
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
            continue
        if character == '"':
            in_string = True
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0:
                return source[opening_parenthesis + 1 : index]
    raise ValueError("unterminated register call")


def attribute_entries(source: str) -> list[AttributeEntry]:
    declaration = re.compile(
        r"public static final Holder<Attribute>\s+(\w+)\s*=\s*register\("
    )
    entries: list[AttributeEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        metadata = re.match(
            r'\s*"([^"]+)"\s*,\s*new RangedAttribute\(\s*"([^"]+)"',
            arguments,
        )
        if metadata is None:
            raise ValueError(f"missing attribute metadata for {constant_name}")
        path, description_key = metadata.groups()
        sentiment_match = re.search(
            r"\.setSentiment\(Attribute\.Sentiment\.(POSITIVE|NEUTRAL|NEGATIVE)\)",
            arguments,
        )
        entries.append(
            AttributeEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{path}",
                description_key=description_key,
                sentiment=(sentiment_match.group(1) if sentiment_match else "POSITIVE"),
                syncable=".setSyncable(true)" in arguments,
            )
        )
    if len(entries) != 35:
        raise ValueError(f"expected 35 attributes, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate attribute constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise ValueError("duplicate attribute resource id")
    return entries


def mob_effect_entries(source: str) -> list[MobEffectEntry]:
    declaration = re.compile(
        r"public static final Holder<MobEffect>\s+(\w+)\s*=\s*register\("
    )
    entries: list[MobEffectEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        resource = re.match(r'\s*"([^"]+)"\s*,', arguments)
        category = re.search(
            r"MobEffectCategory\.(BENEFICIAL|HARMFUL|NEUTRAL)", arguments
        )
        if resource is None or category is None:
            raise ValueError(f"missing mob effect metadata for {constant_name}")
        entries.append(
            MobEffectEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{resource.group(1)}",
                category=category.group(1),
            )
        )
    if len(entries) != 40:
        raise ValueError(f"expected 40 mob effects, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate mob effect constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise ValueError("duplicate mob effect resource id")
    return entries


def menu_type_entries(source: str) -> list[MenuTypeEntry]:
    declaration = re.compile(
        r"public static final MenuType<[^>]+>\s+(\w+)\s*=\s*register\("
    )
    entries: list[MenuTypeEntry] = []
    for match in declaration.finditer(source):
        constant_name = match.group(1)
        arguments = parenthesized_body(source, match.end() - 1)
        resource = re.match(r'\s*"([^"]+)"\s*,', arguments)
        if resource is None:
            raise ValueError(f"missing menu type metadata for {constant_name}")
        entries.append(
            MenuTypeEntry(
                constant_name=constant_name,
                resource_id=f"minecraft:{resource.group(1)}",
            )
        )
    if len(entries) != 25:
        raise ValueError(f"expected 25 menu types, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate menu type constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise ValueError("duplicate menu type resource id")
    return entries


def equipment_slot_entries(source: str) -> list[EquipmentSlotEntry]:
    declaration = re.compile(
        r'^\s+([A-Z_]+)\(EquipmentSlot\.Type\.([A-Z_]+),\s*([0-9]+),\s*([0-9]+)(?:,\s*([0-9]+))?,\s*"([^"]+)"\)[,;]$',
        re.MULTILINE,
    )
    entries: list[EquipmentSlotEntry] = []
    for match in declaration.finditer(source):
        constant_name, slot_type, index, third, explicit_id, serialized_name = (
            match.groups()
        )
        entries.append(
            EquipmentSlotEntry(
                constant_name=constant_name,
                slot_type=slot_type,
                index=int(index),
                count_limit=int(third) if explicit_id is not None else 0,
                slot_id=int(explicit_id) if explicit_id is not None else int(third),
                serialized_name=serialized_name,
            )
        )
    if len(entries) != 8:
        raise ValueError(f"expected 8 equipment slots, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate equipment slot constant")
    if len({entry.slot_id for entry in entries}) != len(entries):
        raise ValueError("duplicate equipment slot id")
    if len({entry.serialized_name for entry in entries}) != len(entries):
        raise ValueError("duplicate equipment slot serialized name")
    return entries


def equipment_slot_group_entries(source: str) -> list[EquipmentSlotGroupEntry]:
    declaration = re.compile(
        r'^\s+([A-Z_]+)\(([0-9]+),\s*"([^"]+)",', re.MULTILINE
    )
    entries = [
        EquipmentSlotGroupEntry(
            constant_name=match.group(1),
            group_id=int(match.group(2)),
            serialized_name=match.group(3),
        )
        for match in declaration.finditer(source)
    ]
    if len(entries) != 11:
        raise ValueError(f"expected 11 equipment slot groups, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate equipment slot group constant")
    if len({entry.group_id for entry in entries}) != len(entries):
        raise ValueError("duplicate equipment slot group id")
    if len({entry.serialized_name for entry in entries}) != len(entries):
        raise ValueError("duplicate equipment slot group serialized name")
    return entries


def entity_pose_entries(source: str) -> list[EntityPoseEntry]:
    declaration = re.compile(
        r'^\s+([A-Z_]+)\(([0-9]+),\s*"([^"]+)"\)[,;]$', re.MULTILINE
    )
    entries = [
        EntityPoseEntry(
            constant_name=match.group(1),
            pose_id=int(match.group(2)),
            serialized_name=match.group(3),
        )
        for match in declaration.finditer(source)
    ]
    if len(entries) != 18:
        raise ValueError(f"expected 18 entity poses, parsed {len(entries)}")
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise ValueError("duplicate entity pose constant")
    if len({entry.pose_id for entry in entries}) != len(entries):
        raise ValueError("duplicate entity pose id")
    if len({entry.serialized_name for entry in entries}) != len(entries):
        raise ValueError("duplicate entity pose serialized name")
    return entries


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def rust_pascal_case(value: str) -> str:
    return "".join(part.capitalize() for part in value.split("_"))


def render_attributes(entries: list[AttributeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Attributes.java static register calls.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum AttributeSentiment {",
        "    Positive,",
        "    Neutral,",
        "    Negative,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct AttributeTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub description_key: &'static str,",
        "    pub sentiment: AttributeSentiment,",
        "    pub syncable: bool,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_ATTRIBUTE_{entry.constant_name}_ID: i32 = {entry_id};"
        )
    lines.extend(("", "pub const VANILLA_ATTRIBUTE_TYPES: &[AttributeTypeCatalogEntry] = &["))
    for entry_id, entry in enumerate(entries):
        sentiment = entry.sentiment.capitalize()
        lines.extend(
            (
                "    AttributeTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        description_key: {rust_string(entry.description_key)},",
                f"        sentiment: AttributeSentiment::{sentiment},",
                f"        syncable: {str(entry.syncable).lower()},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_attribute_type(id: i32) -> Option<&'static AttributeTypeCatalogEntry> {",
            "    VANILLA_ATTRIBUTE_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_attribute_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static AttributeTypeCatalogEntry> {",
            "    VANILLA_ATTRIBUTE_TYPES.iter().find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn attribute_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_ATTRIBUTE_TYPES.len(), 35);",
            "        let mut constants = HashSet::new();",
            "        let mut resources = HashSet::new();",
            "        for (id, entry) in VANILLA_ATTRIBUTE_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "        assert_eq!(VANILLA_ATTRIBUTE_TYPES.iter().filter(|entry| entry.sentiment == AttributeSentiment::Negative).count(), 2);",
            "        assert_eq!(VANILLA_ATTRIBUTE_TYPES.iter().filter(|entry| entry.sentiment == AttributeSentiment::Neutral).count(), 4);",
            "        assert_eq!(VANILLA_ATTRIBUTE_TYPES.iter().filter(|entry| entry.syncable).count(), 27);",
            "    }",
            "",
            "    #[test]",
            "    fn attribute_catalog_preserves_vanilla_order_and_metadata() {",
            "        assert_eq!(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, 2);",
            "        assert_eq!(VANILLA_ATTRIBUTE_SCALE_ID, 25);",
            "        assert_eq!(VANILLA_ATTRIBUTE_WAYPOINT_RECEIVE_RANGE_ID, 34);",
            "        assert_eq!(vanilla_attribute_type(7).unwrap().sentiment, AttributeSentiment::Negative);",
            "        assert_eq!(vanilla_attribute_type(25).unwrap().sentiment, AttributeSentiment::Neutral);",
            "        assert_eq!(vanilla_attribute_type_for_resource_id(\"minecraft:attack_speed\").unwrap().id, 4);",
            "        assert_eq!(vanilla_attribute_type(-1), None);",
            "        assert_eq!(vanilla_attribute_type(35), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else { return; };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/entity/ai/attributes/Attributes.java\").is_file() { return; }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-gameplay-id-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--attribute-types-output\")",
            "            .arg(manifest_dir.join(\"src/attribute_types.rs\"))",
            "            .arg(\"--mob-effect-types-output\")",
            "            .arg(manifest_dir.join(\"src/mob_effect_types.rs\"))",
            "            .arg(\"--menu-types-output\")",
            "            .arg(manifest_dir.join(\"src/menu_types.rs\"))",
            "            .arg(\"--equipment-slots-output\")",
            "            .arg(manifest_dir.join(\"src/equipment_slots.rs\"))",
            "            .arg(\"--entity-poses-output\")",
            "            .arg(manifest_dir.join(\"src/entity_poses.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_mob_effects(entries: list[MobEffectEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 MobEffects.java static register calls.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum MobEffectCategory {",
        "    Beneficial,",
        "    Harmful,",
        "    Neutral,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct MobEffectTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub category: MobEffectCategory,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_MOB_EFFECT_{entry.constant_name}_ID: i32 = {entry_id};"
        )
    lines.extend(
        ("", "pub const VANILLA_MOB_EFFECT_TYPES: &[MobEffectTypeCatalogEntry] = &[")
    )
    for entry_id, entry in enumerate(entries):
        category = entry.category.capitalize()
        lines.extend(
            (
                "    MobEffectTypeCatalogEntry {",
                f"        id: {entry_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                f"        category: MobEffectCategory::{category},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_mob_effect_type(id: i32) -> Option<&'static MobEffectTypeCatalogEntry> {",
            "    VANILLA_MOB_EFFECT_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_mob_effect_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static MobEffectTypeCatalogEntry> {",
            "    VANILLA_MOB_EFFECT_TYPES.iter().find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn mob_effect_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.len(), 40);",
            "        let mut constants = HashSet::new();",
            "        let mut resources = HashSet::new();",
            "        for (id, entry) in VANILLA_MOB_EFFECT_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.iter().filter(|entry| entry.category == MobEffectCategory::Beneficial).count(), 20);",
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.iter().filter(|entry| entry.category == MobEffectCategory::Harmful).count(), 16);",
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.iter().filter(|entry| entry.category == MobEffectCategory::Neutral).count(), 4);",
            "    }",
            "",
            "    #[test]",
            "    fn mob_effect_catalog_preserves_vanilla_order_and_metadata() {",
            "        assert_eq!(VANILLA_MOB_EFFECT_SPEED_ID, 0);",
            "        assert_eq!(VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 15);",
            "        assert_eq!(VANILLA_MOB_EFFECT_BREATH_OF_THE_NAUTILUS_ID, 39);",
            "        assert_eq!(vanilla_mob_effect_type(18).unwrap().category, MobEffectCategory::Harmful);",
            "        assert_eq!(vanilla_mob_effect_type(30).unwrap().category, MobEffectCategory::Neutral);",
            "        assert_eq!(vanilla_mob_effect_type_for_resource_id(\"minecraft:conduit_power\").unwrap().id, 28);",
            "        assert_eq!(vanilla_mob_effect_type(-1), None);",
            "        assert_eq!(vanilla_mob_effect_type(40), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else { return; };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/effect/MobEffects.java\").is_file() { return; }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-gameplay-id-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--attribute-types-output\")",
            "            .arg(manifest_dir.join(\"src/attribute_types.rs\"))",
            "            .arg(\"--mob-effect-types-output\")",
            "            .arg(manifest_dir.join(\"src/mob_effect_types.rs\"))",
            "            .arg(\"--menu-types-output\")",
            "            .arg(manifest_dir.join(\"src/menu_types.rs\"))",
            "            .arg(\"--equipment-slots-output\")",
            "            .arg(manifest_dir.join(\"src/equipment_slots.rs\"))",
            "            .arg(\"--entity-poses-output\")",
            "            .arg(manifest_dir.join(\"src/entity_poses.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_menu_types(entries: list[MenuTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 MenuType.java static register calls.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct MenuTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
    ]
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_MENU_TYPE_{entry.constant_name.upper()}_ID: i32 = {entry_id};"
        )
    lines.extend(("", "pub const VANILLA_MENU_TYPES: &[MenuTypeCatalogEntry] = &["))
    for entry_id, entry in enumerate(entries):
        lines.extend(
            (
                "    MenuTypeCatalogEntry {",
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
            "pub fn vanilla_menu_type(id: i32) -> Option<&'static MenuTypeCatalogEntry> {",
            "    VANILLA_MENU_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_menu_type_for_resource_id(",
            "    resource_id: &str,",
            ") -> Option<&'static MenuTypeCatalogEntry> {",
            "    VANILLA_MENU_TYPES.iter().find(|entry| entry.resource_id == resource_id)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn menu_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_MENU_TYPES.len(), 25);",
            "        let mut constants = HashSet::new();",
            "        let mut resources = HashSet::new();",
            "        for (id, entry) in VANILLA_MENU_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(resources.insert(entry.resource_id));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn menu_type_catalog_preserves_vanilla_order() {",
            "        assert_eq!(VANILLA_MENU_TYPE_GENERIC_9X1_ID, 0);",
            "        assert_eq!(VANILLA_MENU_TYPE_CRAFTER_3X3_ID, 7);",
            "        assert_eq!(VANILLA_MENU_TYPE_STONECUTTER_ID, 24);",
            "        assert_eq!(vanilla_menu_type(12).unwrap().resource_id, \"minecraft:crafting\");",
            "        assert_eq!(vanilla_menu_type_for_resource_id(\"minecraft:merchant\").unwrap().id, 19);",
            "        assert_eq!(vanilla_menu_type(-1), None);",
            "        assert_eq!(vanilla_menu_type(25), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else { return; };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/inventory/MenuType.java\").is_file() { return; }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-gameplay-id-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--attribute-types-output\")",
            "            .arg(manifest_dir.join(\"src/attribute_types.rs\"))",
            "            .arg(\"--mob-effect-types-output\")",
            "            .arg(manifest_dir.join(\"src/mob_effect_types.rs\"))",
            "            .arg(\"--menu-types-output\")",
            "            .arg(manifest_dir.join(\"src/menu_types.rs\"))",
            "            .arg(\"--equipment-slots-output\")",
            "            .arg(manifest_dir.join(\"src/equipment_slots.rs\"))",
            "            .arg(\"--entity-poses-output\")",
            "            .arg(manifest_dir.join(\"src/entity_poses.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_equipment_slots(
    slots: list[EquipmentSlotEntry], groups: list[EquipmentSlotGroupEntry]
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 EquipmentSlot.java and EquipmentSlotGroup.java enum entries.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum EquipmentSlotType {",
        "    Hand,",
        "    HumanoidArmor,",
        "    AnimalArmor,",
        "    Saddle,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct EquipmentSlotCatalogEntry {",
        "    pub ordinal: u8,",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub slot_type: EquipmentSlotType,",
        "    pub index: i32,",
        "    pub count_limit: i32,",
        "}",
        "",
    ]
    for ordinal, entry in enumerate(slots):
        lines.append(
            f"pub const VANILLA_EQUIPMENT_SLOT_{entry.constant_name}_ORDINAL: u8 = {ordinal};"
        )
        lines.append(
            f"pub const VANILLA_EQUIPMENT_SLOT_{entry.constant_name}_ID: i32 = {entry.slot_id};"
        )
    lines.extend(("", "pub const VANILLA_EQUIPMENT_SLOTS: &[EquipmentSlotCatalogEntry] = &["))
    for ordinal, entry in enumerate(slots):
        lines.extend(
            (
                "    EquipmentSlotCatalogEntry {",
                f"        ordinal: {ordinal},",
                f"        id: {entry.slot_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                f"        slot_type: EquipmentSlotType::{rust_pascal_case(entry.slot_type)},",
                f"        index: {entry.index},",
                f"        count_limit: {entry.count_limit},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_equipment_slot_by_ordinal(",
            "    ordinal: u8,",
            ") -> Option<&'static EquipmentSlotCatalogEntry> {",
            "    VANILLA_EQUIPMENT_SLOTS.get(usize::from(ordinal))",
            "}",
            "",
            "pub fn vanilla_equipment_slot_by_id(",
            "    id: i32,",
            ") -> Option<&'static EquipmentSlotCatalogEntry> {",
            "    VANILLA_EQUIPMENT_SLOTS.iter().find(|entry| entry.id == id)",
            "}",
            "",
            "pub fn vanilla_equipment_slot_for_serialized_name(",
            "    serialized_name: &str,",
            ") -> Option<&'static EquipmentSlotCatalogEntry> {",
            "    VANILLA_EQUIPMENT_SLOTS.iter().find(|entry| entry.serialized_name == serialized_name)",
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct EquipmentSlotGroupCatalogEntry {",
            "    pub id: i32,",
            "    pub constant_name: &'static str,",
            "    pub serialized_name: &'static str,",
            "}",
            "",
        )
    )
    for entry in groups:
        lines.append(
            f"pub const VANILLA_EQUIPMENT_SLOT_GROUP_{entry.constant_name}_ID: i32 = {entry.group_id};"
        )
    lines.extend(
        ("", "pub const VANILLA_EQUIPMENT_SLOT_GROUPS: &[EquipmentSlotGroupCatalogEntry] = &[")
    )
    for entry in groups:
        lines.extend(
            (
                "    EquipmentSlotGroupCatalogEntry {",
                f"        id: {entry.group_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_equipment_slot_group(",
            "    id: i32,",
            ") -> Option<&'static EquipmentSlotGroupCatalogEntry> {",
            "    VANILLA_EQUIPMENT_SLOT_GROUPS.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_equipment_slot_group_for_serialized_name(",
            "    serialized_name: &str,",
            ") -> Option<&'static EquipmentSlotGroupCatalogEntry> {",
            "    VANILLA_EQUIPMENT_SLOT_GROUPS.iter().find(|entry| entry.serialized_name == serialized_name)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn equipment_slot_catalog_preserves_distinct_ordinals_and_ids() {",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOTS.len(), 8);",
            "        let mut ids = HashSet::new();",
            "        let mut names = HashSet::new();",
            "        for (ordinal, entry) in VANILLA_EQUIPMENT_SLOTS.iter().enumerate() {",
            "            assert_eq!(entry.ordinal, ordinal as u8);",
            "            assert!(ids.insert(entry.id));",
            "            assert!(names.insert(entry.serialized_name));",
            "        }",
            "        assert_eq!(ids, HashSet::from([0, 1, 2, 3, 4, 5, 6, 7]));",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOT_OFFHAND_ORDINAL, 1);",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOT_OFFHAND_ID, 5);",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOT_FEET_ORDINAL, 2);",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOT_FEET_ID, 1);",
            "        assert_eq!(vanilla_equipment_slot_by_ordinal(1).unwrap().serialized_name, \"offhand\");",
            "        assert_eq!(vanilla_equipment_slot_by_id(1).unwrap().serialized_name, \"feet\");",
            "        assert_eq!(vanilla_equipment_slot_by_ordinal(8), None);",
            "        assert_eq!(vanilla_equipment_slot_by_id(8), None);",
            "    }",
            "",
            "    #[test]",
            "    fn equipment_slot_group_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOT_GROUPS.len(), 11);",
            "        let mut names = HashSet::new();",
            "        for (id, entry) in VANILLA_EQUIPMENT_SLOT_GROUPS.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(names.insert(entry.serialized_name));",
            "        }",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID, 0);",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOT_GROUP_ARMOR_ID, 8);",
            "        assert_eq!(VANILLA_EQUIPMENT_SLOT_GROUP_SADDLE_ID, 10);",
            "        assert_eq!(vanilla_equipment_slot_group_for_serialized_name(\"body\").unwrap().id, 9);",
            "        assert_eq!(vanilla_equipment_slot_group(-1), None);",
            "        assert_eq!(vanilla_equipment_slot_group(11), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else { return; };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/entity/EquipmentSlot.java\").is_file() { return; }",
            "        if !source_root.join(\"net/minecraft/world/entity/EquipmentSlotGroup.java\").is_file() { return; }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-gameplay-id-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--attribute-types-output\")",
            "            .arg(manifest_dir.join(\"src/attribute_types.rs\"))",
            "            .arg(\"--mob-effect-types-output\")",
            "            .arg(manifest_dir.join(\"src/mob_effect_types.rs\"))",
            "            .arg(\"--menu-types-output\")",
            "            .arg(manifest_dir.join(\"src/menu_types.rs\"))",
            "            .arg(\"--equipment-slots-output\")",
            "            .arg(manifest_dir.join(\"src/equipment_slots.rs\"))",
            "            .arg(\"--entity-poses-output\")",
            "            .arg(manifest_dir.join(\"src/entity_poses.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_entity_poses(entries: list[EntityPoseEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Pose.java enum entries.",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct EntityPoseCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "}",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_ENTITY_POSE_{entry.constant_name}_ID: i32 = {entry.pose_id};"
        )
    lines.extend(("", "pub const VANILLA_ENTITY_POSES: &[EntityPoseCatalogEntry] = &["))
    for entry in entries:
        lines.extend(
            (
                "    EntityPoseCatalogEntry {",
                f"        id: {entry.pose_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_entity_pose(id: i32) -> Option<&'static EntityPoseCatalogEntry> {",
            "    VANILLA_ENTITY_POSES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_entity_pose_for_serialized_name(",
            "    serialized_name: &str,",
            ") -> Option<&'static EntityPoseCatalogEntry> {",
            "    VANILLA_ENTITY_POSES.iter().find(|entry| entry.serialized_name == serialized_name)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "    use std::path::PathBuf;",
            "    use std::process::Command;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn entity_pose_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_ENTITY_POSES.len(), 18);",
            "        let mut constants = HashSet::new();",
            "        let mut names = HashSet::new();",
            "        for (id, entry) in VANILLA_ENTITY_POSES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(names.insert(entry.serialized_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn entity_pose_catalog_preserves_vanilla_order() {",
            "        assert_eq!(VANILLA_ENTITY_POSE_STANDING_ID, 0);",
            "        assert_eq!(VANILLA_ENTITY_POSE_CROUCHING_ID, 5);",
            "        assert_eq!(VANILLA_ENTITY_POSE_INHALING_ID, 17);",
            "        assert_eq!(vanilla_entity_pose(13).unwrap().serialized_name, \"emerging\");",
            "        assert_eq!(vanilla_entity_pose_for_serialized_name(\"long_jumping\").unwrap().id, 6);",
            "        assert_eq!(vanilla_entity_pose(-1), None);",
            "        assert_eq!(vanilla_entity_pose(18), None);",
            "    }",
            "",
            "    #[test]",
            "    fn generated_catalog_matches_local_vanilla_sources_when_available() {",
            "        let Some(home) = std::env::var_os(\"HOME\") else { return; };",
            "        let source_root = PathBuf::from(home).join(\"Work/mc-code/sources/26.1\");",
            "        if !source_root.join(\"net/minecraft/world/entity/Pose.java\").is_file() { return; }",
            "        let manifest_dir = PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));",
            "        let repo_root = manifest_dir.ancestors().nth(2).unwrap();",
            "        let status = Command::new(repo_root.join(\"scripts/generate-gameplay-id-catalog.py\"))",
            "            .arg(\"--check\")",
            "            .arg(\"--source-root\")",
            "            .arg(source_root)",
            "            .arg(\"--attribute-types-output\")",
            "            .arg(manifest_dir.join(\"src/attribute_types.rs\"))",
            "            .arg(\"--mob-effect-types-output\")",
            "            .arg(manifest_dir.join(\"src/mob_effect_types.rs\"))",
            "            .arg(\"--menu-types-output\")",
            "            .arg(manifest_dir.join(\"src/menu_types.rs\"))",
            "            .arg(\"--equipment-slots-output\")",
            "            .arg(manifest_dir.join(\"src/equipment_slots.rs\"))",
            "            .arg(\"--entity-poses-output\")",
            "            .arg(manifest_dir.join(\"src/entity_poses.rs\"))",
            "            .status()",
            "            .unwrap();",
            "        assert!(status.success());",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def format_rust(rendered: str) -> str:
    formatted = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input=rendered,
        text=True,
        capture_output=True,
        check=True,
    )
    return formatted.stdout


def generate_attributes(source_root: Path) -> str:
    source = (source_root / ATTRIBUTE_TYPES_SOURCE).read_text()
    return format_rust(render_attributes(attribute_entries(source)))


def generate_mob_effects(source_root: Path) -> str:
    source = (source_root / MOB_EFFECT_TYPES_SOURCE).read_text()
    return format_rust(render_mob_effects(mob_effect_entries(source)))


def generate_menu_types(source_root: Path) -> str:
    source = (source_root / MENU_TYPES_SOURCE).read_text()
    return format_rust(render_menu_types(menu_type_entries(source)))


def generate_equipment_slots(source_root: Path) -> str:
    slots_source = (source_root / EQUIPMENT_SLOTS_SOURCE).read_text()
    groups_source = (source_root / EQUIPMENT_SLOT_GROUPS_SOURCE).read_text()
    return format_rust(
        render_equipment_slots(
            equipment_slot_entries(slots_source),
            equipment_slot_group_entries(groups_source),
        )
    )


def generate_entity_poses(source_root: Path) -> str:
    source = (source_root / ENTITY_POSES_SOURCE).read_text()
    return format_rust(render_entity_poses(entity_pose_entries(source)))


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
    generated_attributes = generate_attributes(args.source_root)
    generated_mob_effects = generate_mob_effects(args.source_root)
    generated_menu_types = generate_menu_types(args.source_root)
    generated_equipment_slots = generate_equipment_slots(args.source_root)
    generated_entity_poses = generate_entity_poses(args.source_root)
    if args.check:
        matches = check_generated(
            args.attribute_types_output, generated_attributes
        )
        matches &= check_generated(
            args.mob_effect_types_output, generated_mob_effects
        )
        matches &= check_generated(args.menu_types_output, generated_menu_types)
        matches &= check_generated(
            args.equipment_slots_output, generated_equipment_slots
        )
        matches &= check_generated(args.entity_poses_output, generated_entity_poses)
        return 0 if matches else 1
    args.attribute_types_output.write_text(generated_attributes)
    args.mob_effect_types_output.write_text(generated_mob_effects)
    args.menu_types_output.write_text(generated_menu_types)
    args.equipment_slots_output.write_text(generated_equipment_slots)
    args.entity_poses_output.write_text(generated_entity_poses)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
