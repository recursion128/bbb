#!/usr/bin/env python3
"""Generate vanilla 26.1 gameplay numeric-id catalogs from Tree-sitter CST facts.

The current catalog covers attributes, mob effects, menu types, equipment slot
ordinals, entity poses, entity-data serializers, client game-event types, and
sound sources, heightmap types, game types, difficulties, player-info action
ordinals, and resource-pack response action ordinals. Normal builds consume only
the generated Rust files; this maintenance tool reads local vanilla sources.
"""

from __future__ import annotations

import argparse
import difflib
import subprocess
import sys
from pathlib import Path

from gameplay_id_catalog_extractor import (
    AttributeEntry,
    ClientGameEventTypeEntry,
    DifficultyEntry,
    EntityDataSerializerEntry,
    EntityPoseEntry,
    EquipmentSlotEntry,
    EquipmentSlotGroupEntry,
    GameTypeEntry,
    HeightmapTypeEntry,
    MenuTypeEntry,
    MobEffectEntry,
    PlayerInfoActionEntry,
    ResourcePackResponseActionEntry,
    SoundSourceEntry,
    attribute_entries,
    client_game_event_type_entries,
    difficulty_entries,
    difficulty_out_of_bounds_strategy,
    entity_data_serializer_entries,
    entity_pose_entries,
    equipment_slot_entries,
    equipment_slot_group_entries,
    game_type_entries,
    game_type_not_set_id,
    heightmap_type_entries,
    menu_type_entries,
    mob_effect_entries,
    player_info_action_entries,
    resource_pack_response_action_entries,
    sound_source_entries,
)
from vanilla_java_source import parse_source


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
DEFAULT_ENTITY_DATA_SERIALIZERS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/entity_data_serializers.rs"
)
DEFAULT_GAME_EVENT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/game_event_types.rs"
)
DEFAULT_SOUND_SOURCES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/sound_sources.rs"
DEFAULT_HEIGHTMAP_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/heightmap_types.rs"
DEFAULT_GAME_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/game_types.rs"
DEFAULT_PLAYER_INFO_ACTIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/player_info_actions.rs"
)
DEFAULT_DIFFICULTIES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/difficulties.rs"
DEFAULT_RESOURCE_PACK_RESPONSE_ACTIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/resource_pack_response_actions.rs"
)
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
ENTITY_DATA_SERIALIZERS_SOURCE = Path(
    "net/minecraft/network/syncher/EntityDataSerializers.java"
)
GAME_EVENT_TYPES_SOURCE = Path(
    "net/minecraft/network/protocol/game/ClientboundGameEventPacket.java"
)
SOUND_SOURCES_SOURCE = Path("net/minecraft/sounds/SoundSource.java")
HEIGHTMAP_TYPES_SOURCE = Path("net/minecraft/world/level/levelgen/Heightmap.java")
GAME_TYPES_SOURCE = Path("net/minecraft/world/level/GameType.java")
PLAYER_INFO_ACTIONS_SOURCE = Path(
    "net/minecraft/network/protocol/game/ClientboundPlayerInfoUpdatePacket.java"
)
DIFFICULTIES_SOURCE = Path("net/minecraft/world/Difficulty.java")
RESOURCE_PACK_RESPONSE_ACTIONS_SOURCE = Path(
    "net/minecraft/network/protocol/common/ServerboundResourcePackPacket.java"
)


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
    parser.add_argument(
        "--entity-data-serializers-output",
        type=Path,
        default=DEFAULT_ENTITY_DATA_SERIALIZERS_OUTPUT,
    )
    parser.add_argument(
        "--game-event-types-output",
        type=Path,
        default=DEFAULT_GAME_EVENT_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--sound-sources-output",
        type=Path,
        default=DEFAULT_SOUND_SOURCES_OUTPUT,
    )
    parser.add_argument(
        "--heightmap-types-output", type=Path, default=DEFAULT_HEIGHTMAP_TYPES_OUTPUT
    )
    parser.add_argument("--game-types-output", type=Path, default=DEFAULT_GAME_TYPES_OUTPUT)
    parser.add_argument(
        "--player-info-actions-output",
        type=Path,
        default=DEFAULT_PLAYER_INFO_ACTIONS_OUTPUT,
    )
    parser.add_argument(
        "--difficulties-output", type=Path, default=DEFAULT_DIFFICULTIES_OUTPUT
    )
    parser.add_argument(
        "--resource-pack-response-actions-output",
        type=Path,
        default=DEFAULT_RESOURCE_PACK_RESPONSE_ACTIONS_OUTPUT,
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def rust_pascal_case(value: str) -> str:
    return "".join(part.capitalize() for part in value.split("_"))


def render_attributes(entries: list[AttributeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Attributes.java static register calls.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
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
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_mob_effects(entries: list[MobEffectEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 MobEffects.java static register calls.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
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
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_menu_types(entries: list[MenuTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 MenuType.java static register calls.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
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
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
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
        "    pub slot: EquipmentSlot,",
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
    lines.extend(
        (
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]",
            "#[repr(u8)]",
            "pub enum EquipmentSlot {",
        )
    )
    for ordinal, entry in enumerate(slots):
        variant = {
            "MAINHAND": "MainHand",
            "OFFHAND": "OffHand",
        }.get(entry.constant_name, rust_pascal_case(entry.constant_name))
        lines.append(
            f"    {variant} = VANILLA_EQUIPMENT_SLOT_{entry.constant_name}_ORDINAL,"
        )
    lines.extend(
        (
            "}",
            "",
            "pub const VANILLA_EQUIPMENT_SLOTS: &[EquipmentSlotCatalogEntry] = &[",
        )
    )
    for ordinal, entry in enumerate(slots):
        variant = {
            "MAINHAND": "MainHand",
            "OFFHAND": "OffHand",
        }.get(entry.constant_name, rust_pascal_case(entry.constant_name))
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
                f"        slot: EquipmentSlot::{variant},",
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
            "impl EquipmentSlot {",
            "    pub const fn ordinal(self) -> u8 {",
            "        self as u8",
            "    }",
            "",
            "    pub(crate) fn from_ordinal(ordinal: u8) -> Option<Self> {",
            "        vanilla_equipment_slot_by_ordinal(ordinal).map(|entry| entry.slot)",
            "    }",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "            assert_eq!(entry.slot.ordinal(), entry.ordinal);",
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
            "        assert_eq!(EquipmentSlot::from_ordinal(1), Some(EquipmentSlot::OffHand));",
            "        assert_eq!(EquipmentSlot::from_ordinal(8), None);",
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
            "    fn production_sources_do_not_redeclare_equipment_slot_ordinals() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/equipment_slots.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"enum EquipmentSlot\") || production.contains(\"impl EquipmentSlot\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel equipment-slot ordinal owners: {violations:?}\");",
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
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
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
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_entity_data_serializers(entries: list[EntityDataSerializerEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 EntityDataSerializers.java static registration block.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct EntityDataSerializerCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "}",
        "",
    ]
    for serializer_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_ENTITY_DATA_SERIALIZER_{entry.constant_name}_ID: i32 = {serializer_id};"
        )
    lines.extend(
        (
            "",
            "pub const VANILLA_ENTITY_DATA_SERIALIZERS: &[EntityDataSerializerCatalogEntry] = &[",
        )
    )
    for serializer_id, entry in enumerate(entries):
        lines.extend(
            (
                "    EntityDataSerializerCatalogEntry {",
                f"        id: {serializer_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_entity_data_serializer(",
            "    id: i32,",
            ") -> Option<&'static EntityDataSerializerCatalogEntry> {",
            "    VANILLA_ENTITY_DATA_SERIALIZERS.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_entity_data_serializer_for_constant_name(",
            "    constant_name: &str,",
            ") -> Option<&'static EntityDataSerializerCatalogEntry> {",
            "    VANILLA_ENTITY_DATA_SERIALIZERS.iter().find(|entry| entry.constant_name == constant_name)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn serializer_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_ENTITY_DATA_SERIALIZERS.len(), 43);",
            "        let mut constants = HashSet::new();",
            "        for (id, entry) in VANILLA_ENTITY_DATA_SERIALIZERS.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert!(constants.insert(entry.constant_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn serializer_catalog_preserves_vanilla_registration_order() {",
            "        assert_eq!(VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, 0);",
            "        assert_eq!(VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID, 20);",
            "        assert_eq!(VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_GLOBAL_POS_ID, 33);",
            "        assert_eq!(VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID, 42);",
            "        assert_eq!(vanilla_entity_data_serializer_for_constant_name(\"PAINTING_VARIANT\").unwrap().id, 34);",
            "        assert_eq!(vanilla_entity_data_serializer(-1), None);",
            "        assert_eq!(vanilla_entity_data_serializer(43), None);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_client_game_event_types(entries: list[ClientGameEventTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ClientboundGameEventPacket.java Type declarations.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ClientGameEventTypeCatalogEntry {",
        "    pub id: u8,",
        "    pub constant_name: &'static str,",
        "}",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_CLIENT_GAME_EVENT_{entry.constant_name}_ID: u8 = {entry.event_id};"
        )
    lines.extend(
        (
            "",
            "pub const VANILLA_CLIENT_GAME_EVENT_TYPES: &[ClientGameEventTypeCatalogEntry] = &[",
        )
    )
    for entry in entries:
        lines.extend(
            (
                "    ClientGameEventTypeCatalogEntry {",
                f"        id: {entry.event_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_client_game_event_type(",
            "    id: u8,",
            ") -> Option<&'static ClientGameEventTypeCatalogEntry> {",
            "    VANILLA_CLIENT_GAME_EVENT_TYPES.get(usize::from(id))",
            "}",
            "",
            "pub fn vanilla_client_game_event_type_for_constant_name(",
            "    constant_name: &str,",
            ") -> Option<&'static ClientGameEventTypeCatalogEntry> {",
            "    VANILLA_CLIENT_GAME_EVENT_TYPES.iter().find(|entry| entry.constant_name == constant_name)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn client_game_event_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_CLIENT_GAME_EVENT_TYPES.len(), 14);",
            "        let mut constants = HashSet::new();",
            "        for (id, entry) in VANILLA_CLIENT_GAME_EVENT_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as u8);",
            "            assert!(constants.insert(entry.constant_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn client_game_event_type_catalog_preserves_vanilla_explicit_ids() {",
            "        assert_eq!(VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID, 0);",
            "        assert_eq!(VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID, 3);",
            "        assert_eq!(VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID, 10);",
            "        assert_eq!(VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID, 13);",
            "        assert_eq!(vanilla_client_game_event_type_for_constant_name(\"LIMITED_CRAFTING\").unwrap().id, 12);",
            "        assert_eq!(vanilla_client_game_event_type(14), None);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_sound_sources(entries: list[SoundSourceEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 SoundSource.java enum declaration.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for ordinal, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_SOUND_SOURCE_{entry.constant_name}_ORDINAL: i32 = {ordinal};"
        )
    lines.extend(
        (
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            "pub enum SoundSource {",
        )
    )
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_SOUND_SOURCE_{entry.constant_name}_ORDINAL,"
        )
    lines.extend(
        (
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct SoundSourceCatalogEntry {",
            "    pub ordinal: i32,",
            "    pub constant_name: &'static str,",
            "    pub serialized_name: &'static str,",
            "    pub source: SoundSource,",
            "}",
            "",
            "pub const VANILLA_SOUND_SOURCES: &[SoundSourceCatalogEntry] = &[",
        )
    )
    for ordinal, entry in enumerate(entries):
        lines.extend(
            (
                "    SoundSourceCatalogEntry {",
                f"        ordinal: {ordinal},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                f"        source: SoundSource::{rust_pascal_case(entry.constant_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_sound_source(ordinal: i32) -> Option<&'static SoundSourceCatalogEntry> {",
            "    VANILLA_SOUND_SOURCES.get(usize::try_from(ordinal).ok()?)",
            "}",
            "",
            "pub fn vanilla_sound_source_for_serialized_name(",
            "    serialized_name: &str,",
            ") -> Option<&'static SoundSourceCatalogEntry> {",
            "    VANILLA_SOUND_SOURCES.iter().find(|entry| entry.serialized_name == serialized_name)",
            "}",
            "",
            "impl SoundSource {",
            "    pub fn ordinal(self) -> i32 {",
            "        self as i32",
            "    }",
            "",
            "    pub fn as_str(self) -> &'static str {",
            "        vanilla_sound_source(self.ordinal()).unwrap().serialized_name",
            "    }",
            "",
            "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
            "        Some(vanilla_sound_source(ordinal)?.source)",
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
            "    fn sound_source_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_SOUND_SOURCES.len(), 11);",
            "        let mut constants = HashSet::new();",
            "        let mut names = HashSet::new();",
            "        for (ordinal, entry) in VANILLA_SOUND_SOURCES.iter().enumerate() {",
            "            assert_eq!(entry.ordinal, ordinal as i32);",
            "            assert_eq!(entry.source.ordinal(), entry.ordinal);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(names.insert(entry.serialized_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn sound_source_catalog_preserves_vanilla_order_and_names() {",
            "        assert_eq!(VANILLA_SOUND_SOURCE_MASTER_ORDINAL, 0);",
            "        assert_eq!(VANILLA_SOUND_SOURCE_PLAYERS_ORDINAL, 7);",
            "        assert_eq!(VANILLA_SOUND_SOURCE_UI_ORDINAL, 10);",
            "        assert_eq!(SoundSource::Records.as_str(), \"record\");",
            "        assert_eq!(SoundSource::Players.as_str(), \"player\");",
            "        assert_eq!(SoundSource::from_ordinal(10), Some(SoundSource::Ui));",
            "        assert_eq!(SoundSource::from_ordinal(-1), None);",
            "        assert_eq!(SoundSource::from_ordinal(11), None);",
            "        assert_eq!(vanilla_sound_source_for_serialized_name(\"weather\").unwrap().source, SoundSource::Weather);",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_heightmap_types(entries: list[HeightmapTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Heightmap.Types enum explicit ids, keys, and usage.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum HeightmapUsage { Worldgen, LiveWorld, Client }",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct HeightmapTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialization_key: &'static str,",
        "    pub usage: HeightmapUsage,",
        "}",
        "",
    ]
    for entry in entries:
        lines.append(f"pub const VANILLA_HEIGHTMAP_{entry.constant_name}_ID: i32 = {entry.type_id};")
    lines.extend(("", "pub const VANILLA_HEIGHTMAP_TYPES: &[HeightmapTypeCatalogEntry] = &["))
    for entry in entries:
        lines.extend((
            "    HeightmapTypeCatalogEntry {",
            f"        id: {entry.type_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialization_key: {rust_string(entry.serialization_key)},",
            f"        usage: HeightmapUsage::{rust_pascal_case(entry.usage)},",
            "    },",
        ))
    lines.extend((
        "];", "",
        "pub fn vanilla_heightmap_type(id: i32) -> Option<&'static HeightmapTypeCatalogEntry> {",
        "    VANILLA_HEIGHTMAP_TYPES.get(usize::try_from(id).ok()?)", "}", "",
        "#[cfg(test)]", "mod tests {",
        "    use std::{fs, path::Path};", "", "    use super::*;", "",
        "    #[test]", "    fn heightmap_types_match_vanilla_explicit_ids_and_usage() {",
        "        assert_eq!(VANILLA_HEIGHTMAP_TYPES.len(), 6);",
        "        for (id, entry) in VANILLA_HEIGHTMAP_TYPES.iter().enumerate() { assert_eq!(entry.id, id as i32); }",
        "        assert_eq!(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 1);",
        "        assert_eq!(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 4);",
        "        assert_eq!(vanilla_heightmap_type(3).unwrap().usage, HeightmapUsage::LiveWorld);",
        "        assert_eq!(vanilla_heightmap_type(5).unwrap().serialization_key, \"MOTION_BLOCKING_NO_LEAVES\");",
        "    }", "",
        "    #[test]", "    fn production_sources_do_not_handwrite_heightmap_ids() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/heightmap_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                for (offset, _) in source.match_indices(\": i32 =\") {",
        "                    let start = source[..offset].rfind('\\n').map_or(0, |index| index + 1);",
        "                    let declaration = &source[start..offset];",
        "                    let value = source[offset + \": i32 =\".len()..].trim_start();",
        "                    if declaration.contains(\"VANILLA_HEIGHTMAP_\") && value.as_bytes().first().is_some_and(u8::is_ascii_digit) { violations.push(display.to_string()); }",
        "                }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"handwritten heightmap ids: {violations:?}\");",
        "    }", "}", "",
    ))
    return "\n".join(lines)


def render_game_types(entries: list[GameTypeEntry], not_set_id: int) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 GameType enum explicit ids, names, and nullable sentinel.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        f"pub const VANILLA_GAME_TYPE_NOT_SET_ID: i32 = {not_set_id};",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_GAME_TYPE_{entry.constant_name}_ID: i32 = {entry.game_type_id};"
        )
    lines.extend(
        (
            "",
            "#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            "pub enum GameType {",
        )
    )
    for entry in entries:
        default = "#[default] " if entry.constant_name == "SURVIVAL" else ""
        lines.append(
            f"    {default}{rust_pascal_case(entry.constant_name)} = VANILLA_GAME_TYPE_{entry.constant_name}_ID,"
        )
    lines.extend(
        (
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct GameTypeCatalogEntry {",
            "    pub id: i32,",
            "    pub constant_name: &'static str,",
            "    pub serialized_name: &'static str,",
            "    pub game_type: GameType,",
            "}",
            "",
            "pub const VANILLA_GAME_TYPES: &[GameTypeCatalogEntry] = &[",
        )
    )
    for entry in entries:
        lines.extend(
            (
                "    GameTypeCatalogEntry {",
                f"        id: {entry.game_type_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                f"        game_type: GameType::{rust_pascal_case(entry.constant_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_game_type(id: i32) -> Option<&'static GameTypeCatalogEntry> {",
            "    VANILLA_GAME_TYPES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_game_type_for_serialized_name(",
            "    serialized_name: &str,",
            ") -> Option<&'static GameTypeCatalogEntry> {",
            "    VANILLA_GAME_TYPES",
            "        .iter()",
            "        .find(|entry| entry.serialized_name == serialized_name)",
            "}",
            "",
            "impl GameType {",
            "    pub const fn id(self) -> i32 {",
            "        self as i32",
            "    }",
            "",
            "    pub fn from_id(id: i32) -> Self {",
            "        vanilla_game_type(id).map_or(Self::Survival, |entry| entry.game_type)",
            "    }",
            "",
            "    pub fn serialized_name(self) -> &'static str {",
            "        vanilla_game_type(self.id()).unwrap().serialized_name",
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
            "    fn game_type_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_GAME_TYPES.len(), 4);",
            "        let mut constants = HashSet::new();",
            "        let mut names = HashSet::new();",
            "        for (id, entry) in VANILLA_GAME_TYPES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert_eq!(entry.game_type.id(), entry.id);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(names.insert(entry.serialized_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn game_type_catalog_preserves_vanilla_ids_names_and_fallback() {",
            "        assert_eq!(VANILLA_GAME_TYPE_NOT_SET_ID, -1);",
            "        assert_eq!(VANILLA_GAME_TYPE_SURVIVAL_ID, 0);",
            "        assert_eq!(VANILLA_GAME_TYPE_SPECTATOR_ID, 3);",
            "        assert_eq!(GameType::Creative.serialized_name(), \"creative\");",
            "        assert_eq!(GameType::from_id(3), GameType::Spectator);",
            "        assert_eq!(GameType::from_id(99), GameType::Survival);",
            "        assert_eq!(vanilla_game_type_for_serialized_name(\"adventure\").unwrap().id, 2);",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_handwrite_game_type_ids() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/game_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                for line in production.lines() {",
            "                    let compact = line.replace([' ', '\\t'], \"\");",
            "                    let named_numeric = compact.contains(\"VANILLA_\") && compact.contains(\"GAME_TYPE\") && compact.contains(\":i32=\") && compact.split(\":i32=\").nth(1).is_some_and(|value| value.as_bytes().first().is_some_and(|byte| byte.is_ascii_digit() || *byte == b'-'));",
            "                    let raw_comparison = [\"gameplay().game_type==0\", \"gameplay().game_type==1\", \"gameplay().game_type==2\", \"gameplay().game_type==3\", \"gameplay().game_type!=0\", \"gameplay().game_type!=1\", \"gameplay().game_type!=2\", \"gameplay().game_type!=3\", \"previous_game_type==-1\"].iter().any(|pattern| compact.contains(pattern));",
            "                    let raw_field = [\"game_type:0,\", \"game_type:1,\", \"game_type:2,\", \"game_type:3,\"].iter().any(|pattern| compact.contains(pattern));",
            "                    if named_numeric || raw_comparison || raw_field { violations.push(format!(\"{}: {}\", display, line.trim())); }",
            "                }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"handwritten game type ids: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_player_info_actions(entries: list[PlayerInfoActionEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ClientboundPlayerInfoUpdatePacket.Action declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_PLAYER_INFO_ACTION_{entry.constant_name}_ORDINAL: u8 = {entry.ordinal};"
        )
    lines.extend(
        (
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(u8)]",
            "pub enum PlayerInfoAction {",
        )
    )
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_PLAYER_INFO_ACTION_{entry.constant_name}_ORDINAL,"
        )
    lines.extend(
        (
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct PlayerInfoActionCatalogEntry {",
            "    pub ordinal: u8,",
            "    pub constant_name: &'static str,",
            "    pub action: PlayerInfoAction,",
            "}",
            "",
            "pub const VANILLA_PLAYER_INFO_ACTIONS: &[PlayerInfoActionCatalogEntry] = &[",
        )
    )
    for entry in entries:
        lines.extend(
            (
                "    PlayerInfoActionCatalogEntry {",
                f"        ordinal: {entry.ordinal},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        action: PlayerInfoAction::{rust_pascal_case(entry.constant_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl PlayerInfoAction {",
            "    pub const fn ordinal(self) -> u8 {",
            "        self as u8",
            "    }",
            "",
            "    pub fn from_ordinal(ordinal: u8) -> Option<Self> {",
            "        VANILLA_PLAYER_INFO_ACTIONS",
            "            .get(usize::from(ordinal))",
            "            .map(|entry| entry.action)",
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
            "    fn player_info_action_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_PLAYER_INFO_ACTIONS.len(), 8);",
            "        let mut constants = HashSet::new();",
            "        for (ordinal, entry) in VANILLA_PLAYER_INFO_ACTIONS.iter().enumerate() {",
            "            assert_eq!(entry.ordinal, ordinal as u8);",
            "            assert_eq!(entry.action.ordinal(), entry.ordinal);",
            "            assert!(constants.insert(entry.constant_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn player_info_action_catalog_preserves_vanilla_wire_bit_order() {",
            "        assert_eq!(VANILLA_PLAYER_INFO_ACTION_ADD_PLAYER_ORDINAL, 0);",
            "        assert_eq!(VANILLA_PLAYER_INFO_ACTION_UPDATE_LIST_ORDER_ORDINAL, 6);",
            "        assert_eq!(VANILLA_PLAYER_INFO_ACTION_UPDATE_HAT_ORDINAL, 7);",
            "        assert_eq!(PlayerInfoAction::from_ordinal(6), Some(PlayerInfoAction::UpdateListOrder));",
            "        assert_eq!(PlayerInfoAction::from_ordinal(8), None);",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_player_info_action_ordinals() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/player_info_actions.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"enum PlayerInfoAction\") || production.contains(\"impl PlayerInfoAction\") { violations.push(display.to_string()); continue; }",
            "                for line in production.lines() {",
            "                    let compact = line.replace([' ', '\\t'], \"\");",
            "                    if compact.contains(\"VANILLA_PLAYER_INFO_ACTION_\") && compact.contains(\":u8=\") && compact.split(\":u8=\").nth(1).is_some_and(|value| value.as_bytes().first().is_some_and(u8::is_ascii_digit)) { violations.push(format!(\"{}: {}\", display, line.trim())); }",
            "                }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"handwritten player-info action ordinals: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_resource_pack_response_actions(
    entries: list[ResourcePackResponseActionEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ServerboundResourcePackPacket.Action declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_RESOURCE_PACK_RESPONSE_ACTION_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend(
        (
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            "pub enum ResourcePackResponseAction {",
        )
    )
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_RESOURCE_PACK_RESPONSE_ACTION_{entry.constant_name}_ORDINAL,"
        )
    lines.extend(
        (
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct ResourcePackResponseActionCatalogEntry {",
            "    pub ordinal: i32,",
            "    pub constant_name: &'static str,",
            "    pub action: ResourcePackResponseAction,",
            "}",
            "",
            "pub const VANILLA_RESOURCE_PACK_RESPONSE_ACTIONS: &[ResourcePackResponseActionCatalogEntry] = &[",
        )
    )
    for entry in entries:
        lines.extend(
            (
                "    ResourcePackResponseActionCatalogEntry {",
                f"        ordinal: {entry.ordinal},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        action: ResourcePackResponseAction::{rust_pascal_case(entry.constant_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl ResourcePackResponseAction {",
            "    pub const fn ordinal(self) -> i32 {",
            "        self as i32",
            "    }",
            "",
            "    pub fn from_ordinal(ordinal: i32) -> Option<Self> {",
            "        VANILLA_RESOURCE_PACK_RESPONSE_ACTIONS",
            "            .get(usize::try_from(ordinal).ok()?)",
            "            .map(|entry| entry.action)",
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
            "    fn resource_pack_response_action_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_RESOURCE_PACK_RESPONSE_ACTIONS.len(), 8);",
            "        let mut constants = HashSet::new();",
            "        for (ordinal, entry) in VANILLA_RESOURCE_PACK_RESPONSE_ACTIONS.iter().enumerate() {",
            "            assert_eq!(entry.ordinal, ordinal as i32);",
            "            assert_eq!(entry.action.ordinal(), entry.ordinal);",
            "            assert!(constants.insert(entry.constant_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn resource_pack_response_action_catalog_preserves_vanilla_wire_order() {",
            "        assert_eq!(VANILLA_RESOURCE_PACK_RESPONSE_ACTION_SUCCESSFULLY_LOADED_ORDINAL, 0);",
            "        assert_eq!(VANILLA_RESOURCE_PACK_RESPONSE_ACTION_DISCARDED_ORDINAL, 7);",
            "        assert_eq!(ResourcePackResponseAction::from_ordinal(5), Some(ResourcePackResponseAction::InvalidUrl));",
            "        assert_eq!(ResourcePackResponseAction::from_ordinal(-1), None);",
            "        assert_eq!(ResourcePackResponseAction::from_ordinal(8), None);",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_resource_pack_response_action_ordinals() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/resource_pack_response_actions.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"enum ResourcePackResponseAction {\") || production.contains(\"impl ResourcePackResponseAction {\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel resource-pack response action ordinal owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_difficulties(entries: list[DifficultyEntry], strategy: str) -> str:
    if strategy != "WRAP":
        raise ValueError(f"unsupported difficulty id strategy {strategy}")
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Difficulty enum explicit ids, names, and BY_ID strategy.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum DifficultyIdOutOfBoundsStrategy {",
        "    Wrap,",
        "}",
        "",
        "pub const VANILLA_DIFFICULTY_ID_OUT_OF_BOUNDS_STRATEGY: DifficultyIdOutOfBoundsStrategy =",
        "    DifficultyIdOutOfBoundsStrategy::Wrap;",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_DIFFICULTY_{entry.constant_name}_ID: i32 = {entry.difficulty_id};"
        )
    lines.extend(
        (
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            "pub enum Difficulty {",
        )
    )
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_DIFFICULTY_{entry.constant_name}_ID,"
        )
    lines.extend(
        (
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct DifficultyCatalogEntry {",
            "    pub id: i32,",
            "    pub constant_name: &'static str,",
            "    pub serialized_name: &'static str,",
            "    pub difficulty: Difficulty,",
            "}",
            "",
            "pub const VANILLA_DIFFICULTIES: &[DifficultyCatalogEntry] = &[",
        )
    )
    for entry in entries:
        lines.extend(
            (
                "    DifficultyCatalogEntry {",
                f"        id: {entry.difficulty_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                f"        difficulty: Difficulty::{rust_pascal_case(entry.constant_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "pub fn vanilla_difficulty(id: i32) -> Option<&'static DifficultyCatalogEntry> {",
            "    VANILLA_DIFFICULTIES.get(usize::try_from(id).ok()?)",
            "}",
            "",
            "pub fn vanilla_difficulty_for_serialized_name(",
            "    serialized_name: &str,",
            ") -> Option<&'static DifficultyCatalogEntry> {",
            "    VANILLA_DIFFICULTIES",
            "        .iter()",
            "        .find(|entry| entry.serialized_name == serialized_name)",
            "}",
            "",
            "impl Difficulty {",
            "    pub const fn id(self) -> i32 {",
            "        self as i32",
            "    }",
            "",
            "    pub fn from_id(id: i32) -> Self {",
            "        let id = match VANILLA_DIFFICULTY_ID_OUT_OF_BOUNDS_STRATEGY {",
            "            DifficultyIdOutOfBoundsStrategy::Wrap =>",
            "                id.rem_euclid(VANILLA_DIFFICULTIES.len() as i32),",
            "        };",
            "        vanilla_difficulty(id).unwrap().difficulty",
            "    }",
            "",
            "    pub fn serialized_name(self) -> &'static str {",
            "        vanilla_difficulty(self.id()).unwrap().serialized_name",
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
            "    fn difficulty_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_DIFFICULTIES.len(), 4);",
            "        let mut constants = HashSet::new();",
            "        let mut names = HashSet::new();",
            "        for (id, entry) in VANILLA_DIFFICULTIES.iter().enumerate() {",
            "            assert_eq!(entry.id, id as i32);",
            "            assert_eq!(entry.difficulty.id(), entry.id);",
            "            assert!(constants.insert(entry.constant_name));",
            "            assert!(names.insert(entry.serialized_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn difficulty_catalog_preserves_vanilla_names_and_wrap_fallback() {",
            "        assert_eq!(VANILLA_DIFFICULTY_PEACEFUL_ID, 0);",
            "        assert_eq!(VANILLA_DIFFICULTY_HARD_ID, 3);",
            "        assert_eq!(Difficulty::Normal.serialized_name(), \"normal\");",
            "        assert_eq!(Difficulty::from_id(-1), Difficulty::Hard);",
            "        assert_eq!(Difficulty::from_id(4), Difficulty::Peaceful);",
            "        assert_eq!(vanilla_difficulty_for_serialized_name(\"easy\").unwrap().id, 1);",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_difficulty_ids() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/difficulties.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"enum Difficulty {\") || production.contains(\"impl Difficulty {\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel difficulty id owners: {violations:?}\");",
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


def parse_java(source_root: Path, relative_path: Path):
    return parse_source(relative_path.as_posix(), (source_root / relative_path).read_bytes())


def generate_attributes(source_root: Path) -> str:
    source = parse_java(source_root, ATTRIBUTE_TYPES_SOURCE)
    return format_rust(render_attributes(attribute_entries(source)))


def generate_mob_effects(source_root: Path) -> str:
    source = parse_java(source_root, MOB_EFFECT_TYPES_SOURCE)
    return format_rust(render_mob_effects(mob_effect_entries(source)))


def generate_menu_types(source_root: Path) -> str:
    source = parse_java(source_root, MENU_TYPES_SOURCE)
    return format_rust(render_menu_types(menu_type_entries(source)))


def generate_equipment_slots(source_root: Path) -> str:
    slots_source = parse_java(source_root, EQUIPMENT_SLOTS_SOURCE)
    groups_source = parse_java(source_root, EQUIPMENT_SLOT_GROUPS_SOURCE)
    return format_rust(
        render_equipment_slots(
            equipment_slot_entries(slots_source),
            equipment_slot_group_entries(groups_source),
        )
    )


def generate_entity_poses(source_root: Path) -> str:
    source = parse_java(source_root, ENTITY_POSES_SOURCE)
    return format_rust(render_entity_poses(entity_pose_entries(source)))


def generate_entity_data_serializers(source_root: Path) -> str:
    source = parse_java(source_root, ENTITY_DATA_SERIALIZERS_SOURCE)
    return format_rust(
        render_entity_data_serializers(entity_data_serializer_entries(source))
    )


def generate_client_game_event_types(source_root: Path) -> str:
    source = parse_java(source_root, GAME_EVENT_TYPES_SOURCE)
    return format_rust(
        render_client_game_event_types(client_game_event_type_entries(source))
    )


def generate_sound_sources(source_root: Path) -> str:
    source = parse_java(source_root, SOUND_SOURCES_SOURCE)
    return format_rust(render_sound_sources(sound_source_entries(source)))


def generate_heightmap_types(source_root: Path) -> str:
    source = parse_java(source_root, HEIGHTMAP_TYPES_SOURCE)
    return format_rust(render_heightmap_types(heightmap_type_entries(source)))


def generate_game_types(source_root: Path) -> str:
    source = parse_java(source_root, GAME_TYPES_SOURCE)
    return format_rust(
        render_game_types(game_type_entries(source), game_type_not_set_id(source))
    )


def generate_player_info_actions(source_root: Path) -> str:
    source = parse_java(source_root, PLAYER_INFO_ACTIONS_SOURCE)
    return format_rust(render_player_info_actions(player_info_action_entries(source)))


def generate_difficulties(source_root: Path) -> str:
    source = parse_java(source_root, DIFFICULTIES_SOURCE)
    return format_rust(
        render_difficulties(
            difficulty_entries(source), difficulty_out_of_bounds_strategy(source)
        )
    )


def generate_resource_pack_response_actions(source_root: Path) -> str:
    source = parse_java(source_root, RESOURCE_PACK_RESPONSE_ACTIONS_SOURCE)
    return format_rust(
        render_resource_pack_response_actions(
            resource_pack_response_action_entries(source)
        )
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
    generated_attributes = generate_attributes(args.source_root)
    generated_mob_effects = generate_mob_effects(args.source_root)
    generated_menu_types = generate_menu_types(args.source_root)
    generated_equipment_slots = generate_equipment_slots(args.source_root)
    generated_entity_poses = generate_entity_poses(args.source_root)
    generated_entity_data_serializers = generate_entity_data_serializers(
        args.source_root
    )
    generated_game_event_types = generate_client_game_event_types(args.source_root)
    generated_sound_sources = generate_sound_sources(args.source_root)
    generated_heightmap_types = generate_heightmap_types(args.source_root)
    generated_game_types = generate_game_types(args.source_root)
    generated_player_info_actions = generate_player_info_actions(args.source_root)
    generated_difficulties = generate_difficulties(args.source_root)
    generated_resource_pack_response_actions = generate_resource_pack_response_actions(
        args.source_root
    )
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
        matches &= check_generated(
            args.entity_data_serializers_output,
            generated_entity_data_serializers,
        )
        matches &= check_generated(
            args.game_event_types_output,
            generated_game_event_types,
        )
        matches &= check_generated(
            args.sound_sources_output,
            generated_sound_sources,
        )
        matches &= check_generated(args.heightmap_types_output, generated_heightmap_types)
        matches &= check_generated(args.game_types_output, generated_game_types)
        matches &= check_generated(
            args.player_info_actions_output, generated_player_info_actions
        )
        matches &= check_generated(args.difficulties_output, generated_difficulties)
        matches &= check_generated(
            args.resource_pack_response_actions_output,
            generated_resource_pack_response_actions,
        )
        return 0 if matches else 1
    args.attribute_types_output.write_text(generated_attributes)
    args.mob_effect_types_output.write_text(generated_mob_effects)
    args.menu_types_output.write_text(generated_menu_types)
    args.equipment_slots_output.write_text(generated_equipment_slots)
    args.entity_poses_output.write_text(generated_entity_poses)
    args.entity_data_serializers_output.write_text(
        generated_entity_data_serializers
    )
    args.game_event_types_output.write_text(generated_game_event_types)
    args.sound_sources_output.write_text(generated_sound_sources)
    args.heightmap_types_output.write_text(generated_heightmap_types)
    args.game_types_output.write_text(generated_game_types)
    args.player_info_actions_output.write_text(generated_player_info_actions)
    args.difficulties_output.write_text(generated_difficulties)
    args.resource_pack_response_actions_output.write_text(
        generated_resource_pack_response_actions
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
