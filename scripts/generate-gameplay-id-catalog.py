#!/usr/bin/env python3
"""Generate vanilla 26.1 gameplay numeric-id catalogs from Tree-sitter CST facts.

This maintenance tool reads local vanilla sources; normal builds consume only
the checked-in Rust catalogs it generates.
"""

from __future__ import annotations

import argparse
import difflib
import subprocess
import sys
from pathlib import Path

from gameplay_id_catalog_extractor import (
    AttributeEntry,
    AdvancementFrameTypeEntry,
    BossBarStyleEntry,
    BossEventOperationTypeEntry,
    ChatFormattingEntry,
    ClientInformationOptionEntry,
    ContainerInputEntry,
    ClientCommandActionEntry,
    ClientGameEventTypeEntry,
    CustomChatCompletionsActionEntry,
    DifficultyEntry,
    EntityAnchorEntry,
    EntityDataSerializerEntry,
    EntityPoseEntry,
    EquipmentSlotEntry,
    EquipmentSlotGroupEntry,
    FilterMaskTypeEntry,
    GameTypeEntry,
    HeightmapTypeEntry,
    InteractionHandEntry,
    MenuTypeEntry,
    MobEffectEntry,
    ObjectiveRenderTypeEntry,
    PlayerInfoActionEntry,
    PlayerActionKindEntry,
    PlayerCommandActionEntry,
    ResourcePackResponseActionEntry,
    RemoteDebugSampleTypeEntry,
    ScoreboardDisplaySlotEntry,
    ScoreboardMethodEntry,
    ScoreboardTeamOptionEntry,
    SoundSourceEntry,
    WaypointDataTypeEntry,
    WaypointOperationEntry,
    attribute_entries,
    advancement_frame_type_entries,
    boss_bar_style_entries,
    boss_event_operation_type_entries,
    chat_formatting_entries,
    client_information_option_entries,
    container_input_entries,
    client_command_action_entries,
    client_game_event_type_entries,
    custom_chat_completions_action_entries,
    difficulty_entries,
    difficulty_out_of_bounds_strategy,
    entity_anchor_entries,
    entity_data_serializer_entries,
    entity_pose_entries,
    equipment_slot_entries,
    equipment_slot_group_entries,
    filter_mask_type_entries,
    game_type_entries,
    game_type_not_set_id,
    heightmap_type_entries,
    interaction_hand_entries,
    menu_type_entries,
    mob_effect_entries,
    objective_render_type_entries,
    player_info_action_entries,
    player_action_kind_entries,
    player_command_action_entries,
    resource_pack_response_action_entries,
    remote_debug_sample_type_entries,
    scoreboard_display_slot_entries,
    scoreboard_method_entries,
    scoreboard_team_option_entries,
    sound_source_entries,
    waypoint_wire_type_entries,
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
DEFAULT_CUSTOM_CHAT_COMPLETIONS_ACTIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/custom_chat_completions_actions.rs"
)
DEFAULT_CLIENT_COMMAND_ACTIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/client_command_actions.rs"
)
DEFAULT_PLAYER_ACTION_KINDS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/player_action_kinds.rs"
)
DEFAULT_PLAYER_COMMAND_ACTIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/player_command_actions.rs"
)
DEFAULT_ENTITY_ANCHORS_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/entity_anchors.rs"
DEFAULT_REMOTE_DEBUG_SAMPLE_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/remote_debug_sample_types.rs"
)
DEFAULT_OBJECTIVE_RENDER_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/objective_render_types.rs"
)
DEFAULT_ADVANCEMENT_FRAME_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/advancement_frame_types.rs"
)
DEFAULT_BOSS_BAR_STYLES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/boss_bar_styles.rs"
DEFAULT_SCOREBOARD_DISPLAY_SLOTS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/scoreboard_display_slots.rs"
)
DEFAULT_SCOREBOARD_TEAM_OPTIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/scoreboard_team_options.rs"
)
DEFAULT_CHAT_FORMATTINGS_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/chat_formattings.rs"
DEFAULT_SCOREBOARD_METHODS_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/scoreboard_methods.rs"
DEFAULT_BOSS_EVENT_OPERATION_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/boss_event_operation_types.rs"
)
DEFAULT_FILTER_MASK_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/filter_mask_types.rs"
DEFAULT_WAYPOINT_WIRE_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/waypoint_wire_types.rs"
)
DEFAULT_INTERACTION_HANDS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/interaction_hands.rs"
)
DEFAULT_CLIENT_INFORMATION_OPTIONS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/client_information_options.rs"
)
DEFAULT_CONTAINER_INPUTS_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/container_inputs.rs"
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
CUSTOM_CHAT_COMPLETIONS_ACTIONS_SOURCE = Path(
    "net/minecraft/network/protocol/game/ClientboundCustomChatCompletionsPacket.java"
)
CLIENT_COMMAND_ACTIONS_SOURCE = Path(
    "net/minecraft/network/protocol/game/ServerboundClientCommandPacket.java"
)
PLAYER_ACTION_KINDS_SOURCE = Path(
    "net/minecraft/network/protocol/game/ServerboundPlayerActionPacket.java"
)
PLAYER_COMMAND_ACTIONS_SOURCE = Path(
    "net/minecraft/network/protocol/game/ServerboundPlayerCommandPacket.java"
)
ENTITY_ANCHORS_SOURCE = Path(
    "net/minecraft/commands/arguments/EntityAnchorArgument.java"
)
REMOTE_DEBUG_SAMPLE_TYPES_SOURCE = Path(
    "net/minecraft/util/debugchart/RemoteDebugSampleType.java"
)
OBJECTIVE_RENDER_TYPES_SOURCE = Path(
    "net/minecraft/world/scores/criteria/ObjectiveCriteria.java"
)
ADVANCEMENT_FRAME_TYPES_SOURCE = Path("net/minecraft/advancements/AdvancementType.java")
BOSS_BAR_STYLES_SOURCE = Path("net/minecraft/world/BossEvent.java")
SCOREBOARD_DISPLAY_SLOTS_SOURCE = Path("net/minecraft/world/scores/DisplaySlot.java")
SCOREBOARD_TEAM_OPTIONS_SOURCE = Path("net/minecraft/world/scores/Team.java")
CHAT_FORMATTINGS_SOURCE = Path("net/minecraft/ChatFormatting.java")
SCOREBOARD_OBJECTIVE_METHODS_SOURCE = Path(
    "net/minecraft/network/protocol/game/ClientboundSetObjectivePacket.java"
)
SCOREBOARD_TEAM_METHODS_SOURCE = Path(
    "net/minecraft/network/protocol/game/ClientboundSetPlayerTeamPacket.java"
)
BOSS_EVENT_OPERATION_TYPES_SOURCE = Path(
    "net/minecraft/network/protocol/game/ClientboundBossEventPacket.java"
)
FILTER_MASK_TYPES_SOURCE = Path("net/minecraft/network/chat/FilterMask.java")
WAYPOINT_OPERATION_SOURCE = Path(
    "net/minecraft/network/protocol/game/ClientboundTrackedWaypointPacket.java"
)
WAYPOINT_DATA_TYPE_SOURCE = Path("net/minecraft/world/waypoints/TrackedWaypoint.java")
INTERACTION_HANDS_SOURCE = Path("net/minecraft/world/InteractionHand.java")
CLIENT_CHAT_VISIBILITY_SOURCE = Path(
    "net/minecraft/world/entity/player/ChatVisiblity.java"
)
CLIENT_MAIN_HAND_SOURCE = Path("net/minecraft/world/entity/HumanoidArm.java")
CLIENT_PARTICLE_STATUS_SOURCE = Path("net/minecraft/server/level/ParticleStatus.java")
CONTAINER_INPUTS_SOURCE = Path("net/minecraft/world/inventory/ContainerInput.java")


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
    parser.add_argument(
        "--custom-chat-completions-actions-output",
        type=Path,
        default=DEFAULT_CUSTOM_CHAT_COMPLETIONS_ACTIONS_OUTPUT,
    )
    parser.add_argument(
        "--client-command-actions-output",
        type=Path,
        default=DEFAULT_CLIENT_COMMAND_ACTIONS_OUTPUT,
    )
    parser.add_argument(
        "--player-action-kinds-output",
        type=Path,
        default=DEFAULT_PLAYER_ACTION_KINDS_OUTPUT,
    )
    parser.add_argument(
        "--player-command-actions-output",
        type=Path,
        default=DEFAULT_PLAYER_COMMAND_ACTIONS_OUTPUT,
    )
    parser.add_argument(
        "--entity-anchors-output", type=Path, default=DEFAULT_ENTITY_ANCHORS_OUTPUT
    )
    parser.add_argument(
        "--remote-debug-sample-types-output",
        type=Path,
        default=DEFAULT_REMOTE_DEBUG_SAMPLE_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--objective-render-types-output",
        type=Path,
        default=DEFAULT_OBJECTIVE_RENDER_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--advancement-frame-types-output",
        type=Path,
        default=DEFAULT_ADVANCEMENT_FRAME_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--boss-bar-styles-output", type=Path, default=DEFAULT_BOSS_BAR_STYLES_OUTPUT
    )
    parser.add_argument(
        "--scoreboard-display-slots-output",
        type=Path,
        default=DEFAULT_SCOREBOARD_DISPLAY_SLOTS_OUTPUT,
    )
    parser.add_argument(
        "--scoreboard-team-options-output",
        type=Path,
        default=DEFAULT_SCOREBOARD_TEAM_OPTIONS_OUTPUT,
    )
    parser.add_argument(
        "--chat-formattings-output", type=Path, default=DEFAULT_CHAT_FORMATTINGS_OUTPUT
    )
    parser.add_argument(
        "--scoreboard-methods-output", type=Path, default=DEFAULT_SCOREBOARD_METHODS_OUTPUT
    )
    parser.add_argument(
        "--boss-event-operation-types-output",
        type=Path,
        default=DEFAULT_BOSS_EVENT_OPERATION_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--filter-mask-types-output", type=Path, default=DEFAULT_FILTER_MASK_TYPES_OUTPUT
    )
    parser.add_argument(
        "--waypoint-wire-types-output", type=Path, default=DEFAULT_WAYPOINT_WIRE_TYPES_OUTPUT
    )
    parser.add_argument(
        "--interaction-hands-output", type=Path, default=DEFAULT_INTERACTION_HANDS_OUTPUT
    )
    parser.add_argument(
        "--client-information-options-output",
        type=Path,
        default=DEFAULT_CLIENT_INFORMATION_OPTIONS_OUTPUT,
    )
    parser.add_argument(
        "--container-inputs-output", type=Path, default=DEFAULT_CONTAINER_INPUTS_OUTPUT
    )
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def rust_char(value: str) -> str:
    escaped = {
        "\\": "\\\\",
        "'": "\\'",
        "\n": "\\n",
        "\r": "\\r",
        "\t": "\\t",
    }.get(value, value)
    return f"'{escaped}'"


def rust_pascal_case(value: str) -> str:
    return "".join(part.capitalize() for part in value.split("_"))


def client_command_action_variant_name(value: str) -> str:
    return rust_pascal_case(value).replace("Gamerule", "GameRule")


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


def render_custom_chat_completions_actions(
    entries: list[CustomChatCompletionsActionEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ClientboundCustomChatCompletionsPacket.Action declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTION_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend(
        (
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            "pub enum CustomChatCompletionsAction {",
        )
    )
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTION_{entry.constant_name}_ORDINAL,"
        )
    lines.extend(
        (
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub struct CustomChatCompletionsActionCatalogEntry {",
            "    pub ordinal: i32,",
            "    pub constant_name: &'static str,",
            "    pub action: CustomChatCompletionsAction,",
            "}",
            "",
            "pub const VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTIONS: &[CustomChatCompletionsActionCatalogEntry] = &[",
        )
    )
    for entry in entries:
        lines.extend(
            (
                "    CustomChatCompletionsActionCatalogEntry {",
                f"        ordinal: {entry.ordinal},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        action: CustomChatCompletionsAction::{rust_pascal_case(entry.constant_name)},",
                "    },",
            )
        )
    lines.extend(
        (
            "];",
            "",
            "impl CustomChatCompletionsAction {",
            "    pub const fn ordinal(self) -> i32 {",
            "        self as i32",
            "    }",
            "",
            "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
            "        VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTIONS",
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
            "    fn custom_chat_completions_action_catalog_is_complete_contiguous_and_unique() {",
            "        assert_eq!(VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTIONS.len(), 3);",
            "        let mut constants = HashSet::new();",
            "        for (ordinal, entry) in VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTIONS.iter().enumerate() {",
            "            assert_eq!(entry.ordinal, ordinal as i32);",
            "            assert_eq!(entry.action.ordinal(), entry.ordinal);",
            "            assert!(constants.insert(entry.constant_name));",
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn custom_chat_completions_action_catalog_preserves_vanilla_wire_order() {",
            "        assert_eq!(VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTION_ADD_ORDINAL, 0);",
            "        assert_eq!(VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTION_REMOVE_ORDINAL, 1);",
            "        assert_eq!(VANILLA_CUSTOM_CHAT_COMPLETIONS_ACTION_SET_ORDINAL, 2);",
            "        assert_eq!(CustomChatCompletionsAction::from_ordinal(2), Some(CustomChatCompletionsAction::Set));",
            "        assert_eq!(CustomChatCompletionsAction::from_ordinal(-1), None);",
            "        assert_eq!(CustomChatCompletionsAction::from_ordinal(3), None);",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_custom_chat_completions_action_ordinals() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/custom_chat_completions_actions.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"enum CustomChatCompletionsAction {\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel custom-chat completions action ordinal owners: {violations:?}\");",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_client_command_actions(entries: list[ClientCommandActionEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ServerboundClientCommandPacket.Action declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_CLIENT_COMMAND_ACTION_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum ClientCommandAction {",
    ))
    for entry in entries:
        lines.append(
            f"    {client_command_action_variant_name(entry.constant_name)} = VANILLA_CLIENT_COMMAND_ACTION_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ClientCommandActionCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub action: ClientCommandAction,",
        "}",
        "",
        "pub const VANILLA_CLIENT_COMMAND_ACTIONS: &[ClientCommandActionCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    ClientCommandActionCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        action: ClientCommandAction::{client_command_action_variant_name(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl ClientCommandAction {",
        "    pub(crate) const fn ordinal(self) -> i32 {",
        "        self as i32",
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
        "    fn client_command_action_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_CLIENT_COMMAND_ACTIONS.len(), 3);",
        "        let mut constants = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_CLIENT_COMMAND_ACTIONS.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.action.ordinal(), entry.ordinal);",
        "            assert!(constants.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn client_command_action_catalog_preserves_vanilla_wire_order() {",
        "        assert_eq!(VANILLA_CLIENT_COMMAND_ACTION_PERFORM_RESPAWN_ORDINAL, 0);",
        "        assert_eq!(VANILLA_CLIENT_COMMAND_ACTION_REQUEST_STATS_ORDINAL, 1);",
        "        assert_eq!(VANILLA_CLIENT_COMMAND_ACTION_REQUEST_GAMERULE_VALUES_ORDINAL, 2);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_client_command_action_ordinals() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/client_command_actions.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum ClientCommandAction {\") || production.contains(\"impl ClientCommandAction {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel client command action ordinal owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_player_action_kinds(entries: list[PlayerActionKindEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ServerboundPlayerActionPacket.Action declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_PLAYER_ACTION_KIND_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum PlayerActionKind {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_PLAYER_ACTION_KIND_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct PlayerActionKindCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub kind: PlayerActionKind,",
        "}",
        "",
        "pub const VANILLA_PLAYER_ACTION_KINDS: &[PlayerActionKindCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    PlayerActionKindCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        kind: PlayerActionKind::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl PlayerActionKind {",
        "    pub(crate) const fn ordinal(self) -> i32 {",
        "        self as i32",
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
        "    fn player_action_kind_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_PLAYER_ACTION_KINDS.len(), 8);",
        "        let mut constants = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_PLAYER_ACTION_KINDS.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.kind.ordinal(), entry.ordinal);",
        "            assert!(constants.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn player_action_kind_catalog_preserves_vanilla_wire_order() {",
        "        assert_eq!(VANILLA_PLAYER_ACTION_KIND_START_DESTROY_BLOCK_ORDINAL, 0);",
        "        assert_eq!(VANILLA_PLAYER_ACTION_KIND_SWAP_ITEM_WITH_OFFHAND_ORDINAL, 6);",
        "        assert_eq!(VANILLA_PLAYER_ACTION_KIND_STAB_ORDINAL, 7);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_player_action_kind_ordinals() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/player_action_kinds.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum PlayerActionKind {\") || production.contains(\"impl PlayerActionKind {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel player action kind ordinal owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_player_command_actions(entries: list[PlayerCommandActionEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ServerboundPlayerCommandPacket.Action declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_PLAYER_COMMAND_ACTION_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum PlayerCommandAction {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_PLAYER_COMMAND_ACTION_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct PlayerCommandActionCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub action: PlayerCommandAction,",
        "}",
        "",
        "pub const VANILLA_PLAYER_COMMAND_ACTIONS: &[PlayerCommandActionCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    PlayerCommandActionCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        action: PlayerCommandAction::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl PlayerCommandAction {",
        "    pub(crate) const fn ordinal(self) -> i32 {",
        "        self as i32",
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
        "    fn player_command_action_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_PLAYER_COMMAND_ACTIONS.len(), 7);",
        "        let mut constants = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_PLAYER_COMMAND_ACTIONS.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.action.ordinal(), entry.ordinal);",
        "            assert!(constants.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn player_command_action_catalog_preserves_vanilla_wire_order() {",
        "        assert_eq!(VANILLA_PLAYER_COMMAND_ACTION_STOP_SLEEPING_ORDINAL, 0);",
        "        assert_eq!(VANILLA_PLAYER_COMMAND_ACTION_OPEN_INVENTORY_ORDINAL, 5);",
        "        assert_eq!(VANILLA_PLAYER_COMMAND_ACTION_START_FALL_FLYING_ORDINAL, 6);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_player_command_action_ordinals() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/player_command_actions.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum PlayerCommandAction {\") || production.contains(\"impl PlayerCommandAction {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel player command action ordinal owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_entity_anchors(entries: list[EntityAnchorEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 EntityAnchorArgument.Anchor declaration order and names.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_ENTITY_ANCHOR_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum EntityAnchor {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_ENTITY_ANCHOR_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct EntityAnchorCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub anchor: EntityAnchor,",
        "}",
        "",
        "pub const VANILLA_ENTITY_ANCHORS: &[EntityAnchorCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    EntityAnchorCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            f"        anchor: EntityAnchor::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl EntityAnchor {",
        "    pub const fn ordinal(self) -> i32 {",
        "        self as i32",
        "    }",
        "",
        "    pub fn as_str(self) -> &'static str {",
        "        VANILLA_ENTITY_ANCHORS[self.ordinal() as usize].serialized_name",
        "    }",
        "",
        "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_ENTITY_ANCHORS",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.anchor)",
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
        "    fn entity_anchor_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_ENTITY_ANCHORS.len(), 2);",
        "        let mut constants = HashSet::new();",
        "        let mut names = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_ENTITY_ANCHORS.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.anchor.ordinal(), entry.ordinal);",
        "            assert!(constants.insert(entry.constant_name));",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn entity_anchor_catalog_preserves_vanilla_order_and_names() {",
        "        assert_eq!(VANILLA_ENTITY_ANCHOR_FEET_ORDINAL, 0);",
        "        assert_eq!(VANILLA_ENTITY_ANCHOR_EYES_ORDINAL, 1);",
        "        assert_eq!(EntityAnchor::Feet.as_str(), \"feet\");",
        "        assert_eq!(EntityAnchor::Eyes.as_str(), \"eyes\");",
        "        assert_eq!(EntityAnchor::from_ordinal(-1), None);",
        "        assert_eq!(EntityAnchor::from_ordinal(2), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_entity_anchor_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/entity_anchors.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum EntityAnchor {\") || production.contains(\"impl EntityAnchor {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel entity anchor fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_remote_debug_sample_types(entries: list[RemoteDebugSampleTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 RemoteDebugSampleType declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_REMOTE_DEBUG_SAMPLE_TYPE_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum RemoteDebugSampleType {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_REMOTE_DEBUG_SAMPLE_TYPE_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct RemoteDebugSampleTypeCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub sample_type: RemoteDebugSampleType,",
        "}",
        "",
        "pub const VANILLA_REMOTE_DEBUG_SAMPLE_TYPES: &[RemoteDebugSampleTypeCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    RemoteDebugSampleTypeCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        sample_type: RemoteDebugSampleType::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl RemoteDebugSampleType {",
        "    pub const fn ordinal(self) -> i32 {",
        "        self as i32",
        "    }",
        "",
        "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_REMOTE_DEBUG_SAMPLE_TYPES",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.sample_type)",
        "    }",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn remote_debug_sample_type_catalog_preserves_vanilla_order() {",
        "        assert_eq!(VANILLA_REMOTE_DEBUG_SAMPLE_TYPES.len(), 1);",
        "        let entry = VANILLA_REMOTE_DEBUG_SAMPLE_TYPES[0];",
        "        assert_eq!(entry.ordinal, 0);",
        "        assert_eq!(entry.constant_name, \"TICK_TIME\");",
        "        assert_eq!(entry.sample_type, RemoteDebugSampleType::TickTime);",
        "        assert_eq!(RemoteDebugSampleType::from_ordinal(-1), None);",
        "        assert_eq!(RemoteDebugSampleType::from_ordinal(1), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_remote_debug_sample_type_ordinals() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/remote_debug_sample_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum RemoteDebugSampleType {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel remote debug sample type ordinal owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_objective_render_types(entries: list[ObjectiveRenderTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ObjectiveCriteria.RenderType declaration order and names.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_OBJECTIVE_RENDER_TYPE_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum ObjectiveRenderType {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_OBJECTIVE_RENDER_TYPE_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ObjectiveRenderTypeCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub render_type: ObjectiveRenderType,",
        "}",
        "",
        "pub const VANILLA_OBJECTIVE_RENDER_TYPES: &[ObjectiveRenderTypeCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    ObjectiveRenderTypeCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            f"        render_type: ObjectiveRenderType::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl ObjectiveRenderType {",
        "    pub const fn ordinal(self) -> i32 {",
        "        self as i32",
        "    }",
        "",
        "    pub fn as_str(self) -> &'static str {",
        "        VANILLA_OBJECTIVE_RENDER_TYPES[self.ordinal() as usize].serialized_name",
        "    }",
        "",
        "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_OBJECTIVE_RENDER_TYPES",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.render_type)",
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
        "    fn objective_render_type_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_OBJECTIVE_RENDER_TYPES.len(), 2);",
        "        let mut constants = HashSet::new();",
        "        let mut names = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_OBJECTIVE_RENDER_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.render_type.ordinal(), entry.ordinal);",
        "            assert!(constants.insert(entry.constant_name));",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn objective_render_type_catalog_preserves_vanilla_order_and_names() {",
        "        assert_eq!(VANILLA_OBJECTIVE_RENDER_TYPE_INTEGER_ORDINAL, 0);",
        "        assert_eq!(VANILLA_OBJECTIVE_RENDER_TYPE_HEARTS_ORDINAL, 1);",
        "        assert_eq!(ObjectiveRenderType::Integer.as_str(), \"integer\");",
        "        assert_eq!(ObjectiveRenderType::Hearts.as_str(), \"hearts\");",
        "        assert_eq!(ObjectiveRenderType::from_ordinal(-1), None);",
        "        assert_eq!(ObjectiveRenderType::from_ordinal(2), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_objective_render_type_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/objective_render_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum ObjectiveRenderType {\") || production.contains(\"impl ObjectiveRenderType {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel objective render type fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_advancement_frame_types(entries: list[AdvancementFrameTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 AdvancementType declaration order and names.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_ADVANCEMENT_FRAME_TYPE_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum AdvancementFrameType {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_ADVANCEMENT_FRAME_TYPE_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct AdvancementFrameTypeCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub frame_type: AdvancementFrameType,",
        "}",
        "",
        "pub const VANILLA_ADVANCEMENT_FRAME_TYPES: &[AdvancementFrameTypeCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    AdvancementFrameTypeCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            f"        frame_type: AdvancementFrameType::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl AdvancementFrameType {",
        "    pub const fn ordinal(self) -> i32 {",
        "        self as i32",
        "    }",
        "",
        "    pub fn as_str(self) -> &'static str {",
        "        VANILLA_ADVANCEMENT_FRAME_TYPES[self.ordinal() as usize].serialized_name",
        "    }",
        "",
        "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_ADVANCEMENT_FRAME_TYPES",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.frame_type)",
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
        "    fn advancement_frame_type_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_ADVANCEMENT_FRAME_TYPES.len(), 3);",
        "        let mut constants = HashSet::new();",
        "        let mut names = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_ADVANCEMENT_FRAME_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.frame_type.ordinal(), entry.ordinal);",
        "            assert!(constants.insert(entry.constant_name));",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn advancement_frame_type_catalog_preserves_vanilla_order_and_names() {",
        "        assert_eq!(VANILLA_ADVANCEMENT_FRAME_TYPE_TASK_ORDINAL, 0);",
        "        assert_eq!(VANILLA_ADVANCEMENT_FRAME_TYPE_CHALLENGE_ORDINAL, 1);",
        "        assert_eq!(VANILLA_ADVANCEMENT_FRAME_TYPE_GOAL_ORDINAL, 2);",
        "        assert_eq!(AdvancementFrameType::Task.as_str(), \"task\");",
        "        assert_eq!(AdvancementFrameType::Challenge.as_str(), \"challenge\");",
        "        assert_eq!(AdvancementFrameType::Goal.as_str(), \"goal\");",
        "        assert_eq!(AdvancementFrameType::from_ordinal(-1), None);",
        "        assert_eq!(AdvancementFrameType::from_ordinal(3), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_advancement_frame_type_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/advancement_frame_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum AdvancementFrameType {\") || production.contains(\"impl AdvancementFrameType {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel advancement frame type fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_boss_bar_styles(
    colors: list[BossBarStyleEntry], overlays: list[BossBarStyleEntry]
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 BossEvent boss-bar enum declaration order and names.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]

    def append_catalog(
        rust_type: str,
        constant_prefix: str,
        catalog_name: str,
        entry_type: str,
        value_field: str,
        entries: list[BossBarStyleEntry],
    ) -> None:
        for entry in entries:
            lines.append(
                f"pub const {constant_prefix}_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
            )
        lines.extend((
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            f"pub enum {rust_type} {{",
        ))
        for entry in entries:
            lines.append(
                f"    {rust_pascal_case(entry.constant_name)} = {constant_prefix}_{entry.constant_name}_ORDINAL,"
            )
        lines.extend((
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            f"pub struct {entry_type} {{",
            "    pub ordinal: i32,",
            "    pub constant_name: &'static str,",
            "    pub serialized_name: &'static str,",
            f"    pub {value_field}: {rust_type},",
            "}",
            "",
            f"pub const {catalog_name}: &[{entry_type}] = &[",
        ))
        for entry in entries:
            lines.extend((
                f"    {entry_type} {{",
                f"        ordinal: {entry.ordinal},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                f"        {value_field}: {rust_type}::{rust_pascal_case(entry.constant_name)},",
                "    },",
            ))
        lines.extend((
            "];",
            "",
            f"impl {rust_type} {{",
            "    pub const fn ordinal(self) -> i32 { self as i32 }",
            "",
            "    pub fn as_str(self) -> &'static str {",
            f"        {catalog_name}[self.ordinal() as usize].serialized_name",
            "    }",
            "",
            "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
            f"        {catalog_name}",
            "            .get(usize::try_from(ordinal).ok()?)",
            f"            .map(|entry| entry.{value_field})",
            "    }",
            "}",
            "",
        ))

    append_catalog(
        "BossBarColor", "VANILLA_BOSS_BAR_COLOR", "VANILLA_BOSS_BAR_COLORS",
        "BossBarColorCatalogEntry", "color", colors,
    )
    append_catalog(
        "BossBarOverlay", "VANILLA_BOSS_BAR_OVERLAY", "VANILLA_BOSS_BAR_OVERLAYS",
        "BossBarOverlayCatalogEntry", "overlay", overlays,
    )
    lines.extend((
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn boss_bar_style_catalogs_are_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_BOSS_BAR_COLORS.len(), 7);",
        "        assert_eq!(VANILLA_BOSS_BAR_OVERLAYS.len(), 5);",
        "        let mut names = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_BOSS_BAR_COLORS.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.color.ordinal(), entry.ordinal);",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "        names.clear();",
        "        for (ordinal, entry) in VANILLA_BOSS_BAR_OVERLAYS.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.overlay.ordinal(), entry.ordinal);",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn boss_bar_style_catalogs_preserve_vanilla_order_and_names() {",
        "        assert_eq!(VANILLA_BOSS_BAR_COLOR_PINK_ORDINAL, 0);",
        "        assert_eq!(VANILLA_BOSS_BAR_COLOR_WHITE_ORDINAL, 6);",
        "        assert_eq!(BossBarColor::Purple.as_str(), \"purple\");",
        "        assert_eq!(VANILLA_BOSS_BAR_OVERLAY_PROGRESS_ORDINAL, 0);",
        "        assert_eq!(VANILLA_BOSS_BAR_OVERLAY_NOTCHED_20_ORDINAL, 4);",
        "        assert_eq!(BossBarOverlay::Notched10.as_str(), \"notched_10\");",
        "        assert_eq!(BossBarColor::from_ordinal(7), None);",
        "        assert_eq!(BossBarOverlay::from_ordinal(5), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_boss_bar_style_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/boss_bar_styles.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum BossBarColor {\") || production.contains(\"impl BossBarColor {\") || production.contains(\"enum BossBarOverlay {\") || production.contains(\"impl BossBarOverlay {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel boss-bar style fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_scoreboard_display_slots(
    entries: list[ScoreboardDisplaySlotEntry], strategy: str
) -> str:
    if strategy != "ZERO":
        raise ValueError(f"unsupported display slot strategy {strategy}")
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 DisplaySlot explicit ids, names, and BY_ID strategy.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ScoreboardDisplaySlotOutOfBoundsStrategy { Zero }",
        "",
        "pub const VANILLA_SCOREBOARD_DISPLAY_SLOT_OUT_OF_BOUNDS_STRATEGY: ScoreboardDisplaySlotOutOfBoundsStrategy = ScoreboardDisplaySlotOutOfBoundsStrategy::Zero;",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_SCOREBOARD_DISPLAY_SLOT_{entry.constant_name}_ID: i32 = {entry.slot_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum ScoreboardDisplaySlot {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_SCOREBOARD_DISPLAY_SLOT_{entry.constant_name}_ID,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ScoreboardDisplaySlotCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub slot: ScoreboardDisplaySlot,",
        "}",
        "",
        "pub const VANILLA_SCOREBOARD_DISPLAY_SLOTS: &[ScoreboardDisplaySlotCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    ScoreboardDisplaySlotCatalogEntry {",
            f"        id: {entry.slot_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            f"        slot: ScoreboardDisplaySlot::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl ScoreboardDisplaySlot {",
        "    pub const fn id(self) -> i32 { self as i32 }",
        "",
        "    pub fn as_str(self) -> &'static str {",
        "        VANILLA_SCOREBOARD_DISPLAY_SLOTS[self.id() as usize].serialized_name",
        "    }",
        "",
        "    pub(crate) fn from_id(id: i32) -> Self {",
        "        VANILLA_SCOREBOARD_DISPLAY_SLOTS",
        "            .get(usize::try_from(id).unwrap_or(usize::MAX))",
        "            .map(|entry| entry.slot)",
        "            .unwrap_or(VANILLA_SCOREBOARD_DISPLAY_SLOTS[0].slot)",
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
        "    fn scoreboard_display_slot_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_SCOREBOARD_DISPLAY_SLOTS.len(), 19);",
        "        let mut names = HashSet::new();",
        "        for (id, entry) in VANILLA_SCOREBOARD_DISPLAY_SLOTS.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert_eq!(entry.slot.id(), entry.id);",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn scoreboard_display_slot_catalog_preserves_names_and_zero_fallback() {",
        "        assert_eq!(VANILLA_SCOREBOARD_DISPLAY_SLOT_LIST_ID, 0);",
        "        assert_eq!(VANILLA_SCOREBOARD_DISPLAY_SLOT_TEAM_WHITE_ID, 18);",
        "        assert_eq!(ScoreboardDisplaySlot::BelowName.as_str(), \"below_name\");",
        "        assert_eq!(ScoreboardDisplaySlot::TeamDarkAqua.as_str(), \"sidebar.team.dark_aqua\");",
        "        assert_eq!(ScoreboardDisplaySlot::from_id(-1), ScoreboardDisplaySlot::List);",
        "        assert_eq!(ScoreboardDisplaySlot::from_id(19), ScoreboardDisplaySlot::List);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_scoreboard_display_slot_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/scoreboard_display_slots.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum ScoreboardDisplaySlot {\") || production.contains(\"impl ScoreboardDisplaySlot {\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel scoreboard display slot fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_scoreboard_team_options(
    visibility: list[ScoreboardTeamOptionEntry],
    visibility_strategy: str,
    collision: list[ScoreboardTeamOptionEntry],
    collision_strategy: str,
) -> str:
    if visibility_strategy != "ZERO" or collision_strategy != "ZERO":
        raise ValueError(
            "unsupported scoreboard team option strategies "
            f"{visibility_strategy}/{collision_strategy}"
        )
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Team nested enum explicit ids, names, and BY_ID strategies.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ScoreboardTeamOptionOutOfBoundsStrategy { Zero }",
        "",
        "pub const VANILLA_TEAM_VISIBILITY_OUT_OF_BOUNDS_STRATEGY: ScoreboardTeamOptionOutOfBoundsStrategy = ScoreboardTeamOptionOutOfBoundsStrategy::Zero;",
        "pub const VANILLA_TEAM_COLLISION_RULE_OUT_OF_BOUNDS_STRATEGY: ScoreboardTeamOptionOutOfBoundsStrategy = ScoreboardTeamOptionOutOfBoundsStrategy::Zero;",
        "",
    ]

    def append_catalog(
        rust_type: str,
        constant_prefix: str,
        catalog_name: str,
        entry_type: str,
        value_field: str,
        entries: list[ScoreboardTeamOptionEntry],
    ) -> None:
        for entry in entries:
            lines.append(
                f"pub const {constant_prefix}_{entry.constant_name}_ID: i32 = {entry.option_id};"
            )
        lines.extend((
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            f"pub enum {rust_type} {{",
        ))
        for entry in entries:
            lines.append(
                f"    {rust_pascal_case(entry.constant_name)} = {constant_prefix}_{entry.constant_name}_ID,"
            )
        lines.extend((
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            f"pub struct {entry_type} {{",
            "    pub id: i32,",
            "    pub constant_name: &'static str,",
            "    pub serialized_name: &'static str,",
            f"    pub {value_field}: {rust_type},",
            "}",
            "",
            f"pub const {catalog_name}: &[{entry_type}] = &[",
        ))
        for entry in entries:
            lines.extend((
                f"    {entry_type} {{",
                f"        id: {entry.option_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: {rust_string(entry.serialized_name)},",
                f"        {value_field}: {rust_type}::{rust_pascal_case(entry.constant_name)},",
                "    },",
            ))
        lines.extend((
            "];",
            "",
            f"impl {rust_type} {{",
            "    pub const fn id(self) -> i32 { self as i32 }",
            "",
            "    pub fn as_str(self) -> &'static str {",
            f"        {catalog_name}[self.id() as usize].serialized_name",
            "    }",
            "",
            "    pub(crate) fn from_id(id: i32) -> Self {",
            f"        {catalog_name}",
            "            .get(usize::try_from(id).unwrap_or(usize::MAX))",
            f"            .map(|entry| entry.{value_field})",
            f"            .unwrap_or({catalog_name}[0].{value_field})",
            "    }",
            "}",
            "",
        ))

    append_catalog(
        "TeamVisibility", "VANILLA_TEAM_VISIBILITY", "VANILLA_TEAM_VISIBILITIES",
        "TeamVisibilityCatalogEntry", "visibility", visibility,
    )
    append_catalog(
        "TeamCollisionRule", "VANILLA_TEAM_COLLISION_RULE", "VANILLA_TEAM_COLLISION_RULES",
        "TeamCollisionRuleCatalogEntry", "collision_rule", collision,
    )
    lines.extend((
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn scoreboard_team_option_catalogs_are_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_TEAM_VISIBILITIES.len(), 4);",
        "        assert_eq!(VANILLA_TEAM_COLLISION_RULES.len(), 4);",
        "        let mut names = HashSet::new();",
        "        for (id, entry) in VANILLA_TEAM_VISIBILITIES.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert_eq!(entry.visibility.id(), entry.id);",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "        names.clear();",
        "        for (id, entry) in VANILLA_TEAM_COLLISION_RULES.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert_eq!(entry.collision_rule.id(), entry.id);",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn scoreboard_team_option_catalogs_preserve_names_and_zero_fallback() {",
        "        assert_eq!(TeamVisibility::HideForOtherTeams.as_str(), \"hideForOtherTeams\");",
        "        assert_eq!(TeamCollisionRule::PushOwnTeam.as_str(), \"pushOwnTeam\");",
        "        assert_eq!(TeamVisibility::from_id(-1), TeamVisibility::Always);",
        "        assert_eq!(TeamVisibility::from_id(4), TeamVisibility::Always);",
        "        assert_eq!(TeamCollisionRule::from_id(-1), TeamCollisionRule::Always);",
        "        assert_eq!(TeamCollisionRule::from_id(4), TeamCollisionRule::Always);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_scoreboard_team_option_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/scoreboard_team_options.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum TeamVisibility {\") || production.contains(\"impl TeamVisibility {\") || production.contains(\"enum TeamCollisionRule {\") || production.contains(\"impl TeamCollisionRule {\") || production.contains(\"ProtocolTeamVisibility::Always =>\") || production.contains(\"ProtocolTeamCollisionRule::Always =>\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel scoreboard team option fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_chat_formattings(entries: list[ChatFormattingEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ChatFormatting enum declaration order and constructor facts.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_CHAT_FORMATTING_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum ChatFormatting {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_CHAT_FORMATTING_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ChatFormattingCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub legacy_code: char,",
        "    pub id: i32,",
        "    pub is_format: bool,",
        "    pub color: Option<u32>,",
        "    pub formatting: ChatFormatting,",
        "}",
        "",
        "pub const VANILLA_CHAT_FORMATTINGS: &[ChatFormattingCatalogEntry] = &[",
    ))
    for entry in entries:
        color = "None" if entry.color is None else f"Some(0x{entry.color:06X})"
        lines.extend((
            "    ChatFormattingCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            f"        legacy_code: {rust_char(entry.legacy_code)},",
            f"        id: {entry.formatting_id},",
            f"        is_format: {str(entry.is_format).lower()},",
            f"        color: {color},",
            f"        formatting: ChatFormatting::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl ChatFormatting {",
        "    pub const fn ordinal(self) -> i32 { self as i32 }",
        "",
        "    fn descriptor(self) -> &'static ChatFormattingCatalogEntry {",
        "        &VANILLA_CHAT_FORMATTINGS[self.ordinal() as usize]",
        "    }",
        "",
        "    pub fn as_str(self) -> &'static str { self.descriptor().serialized_name }",
        "",
        "    pub fn legacy_code(self) -> char { self.descriptor().legacy_code }",
        "",
        "    pub fn id(self) -> i32 { self.descriptor().id }",
        "",
        "    pub fn is_format(self) -> bool { self.descriptor().is_format }",
        "",
        "    pub fn color(self) -> Option<u32> { self.descriptor().color }",
        "",
        "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_CHAT_FORMATTINGS",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.formatting)",
        "    }",
        "}",
        "",
        "pub fn vanilla_chat_formatting_color_by_name(name: &str) -> Option<u32> {",
        "    VANILLA_CHAT_FORMATTINGS",
        "        .iter()",
        "        .find(|entry| entry.serialized_name == name)",
        "        .and_then(|entry| entry.color)",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn chat_formatting_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_CHAT_FORMATTINGS.len(), 22);",
        "        let mut names = HashSet::new();",
        "        let mut codes = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_CHAT_FORMATTINGS.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.formatting.ordinal(), entry.ordinal);",
        "            assert!(names.insert(entry.serialized_name));",
        "            assert!(codes.insert(entry.legacy_code));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn chat_formatting_catalog_preserves_colors_formats_and_names() {",
        "        assert_eq!(ChatFormatting::Black.id(), 0);",
        "        assert_eq!(ChatFormatting::Red.color(), Some(0xFF5555));",
        "        assert_eq!(ChatFormatting::White.id(), 15);",
        "        assert_eq!(ChatFormatting::DarkPurple.as_str(), \"dark_purple\");",
        "        assert_eq!(ChatFormatting::Bold.legacy_code(), 'l');",
        "        assert!(ChatFormatting::Bold.is_format());",
        "        assert!(!ChatFormatting::Reset.is_format());",
        "        assert_eq!(ChatFormatting::Reset.color(), None);",
        "        assert_eq!(vanilla_chat_formatting_color_by_name(\"red\"), Some(0xFF5555));",
        "        assert_eq!(vanilla_chat_formatting_color_by_name(\"bold\"), None);",
        "        assert_eq!(ChatFormatting::from_ordinal(-1), None);",
        "        assert_eq!(ChatFormatting::from_ordinal(22), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_chat_formatting_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/chat_formattings.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum ChatFormatting {\") || production.contains(\"impl ChatFormatting {\") || production.contains(\"ProtocolChatFormatting::Black =>\") || production.contains(\"\\\"dark_blue\\\" =>\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel chat-formatting fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_scoreboard_methods(
    objective: list[ScoreboardMethodEntry], team: list[ScoreboardMethodEntry]
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 scoreboard packet METHOD_* constants.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]

    def append_catalog(
        rust_type: str,
        constant_prefix: str,
        catalog_name: str,
        entry_type: str,
        value_field: str,
        entries: list[ScoreboardMethodEntry],
    ) -> None:
        for entry in entries:
            lines.append(
                f"pub const {constant_prefix}_{entry.constant_name}_ID: i8 = {entry.method_id};"
            )
        lines.extend((
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i8)]",
            f"pub enum {rust_type} {{",
        ))
        for entry in entries:
            lines.append(
                f"    {rust_pascal_case(entry.constant_name)} = {constant_prefix}_{entry.constant_name}_ID,"
            )
        lines.extend((
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            f"pub struct {entry_type} {{",
            "    pub id: i8,",
            "    pub constant_name: &'static str,",
            f"    pub {value_field}: {rust_type},",
            "}",
            "",
            f"pub const {catalog_name}: &[{entry_type}] = &[",
        ))
        for entry in entries:
            lines.extend((
                f"    {entry_type} {{",
                f"        id: {entry.method_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        {value_field}: {rust_type}::{rust_pascal_case(entry.constant_name)},",
                "    },",
            ))
        lines.extend((
            "];",
            "",
            f"impl {rust_type} {{",
            "    pub const fn id(self) -> i8 { self as i8 }",
            "",
            "    pub(crate) fn from_id(id: i8) -> Option<Self> {",
            f"        {catalog_name}",
            "            .get(usize::try_from(id).ok()?)",
            f"            .map(|entry| entry.{value_field})",
            "    }",
            "}",
            "",
        ))

    append_catalog(
        "SetObjectiveMethod", "VANILLA_SET_OBJECTIVE_METHOD", "VANILLA_SET_OBJECTIVE_METHODS",
        "SetObjectiveMethodCatalogEntry", "method", objective,
    )
    append_catalog(
        "PlayerTeamMethod", "VANILLA_PLAYER_TEAM_METHOD", "VANILLA_PLAYER_TEAM_METHODS",
        "PlayerTeamMethodCatalogEntry", "method", team,
    )
    lines.extend((
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn scoreboard_method_catalogs_are_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_SET_OBJECTIVE_METHODS.len(), 3);",
        "        assert_eq!(VANILLA_PLAYER_TEAM_METHODS.len(), 5);",
        "        let mut names = HashSet::new();",
        "        for (id, entry) in VANILLA_SET_OBJECTIVE_METHODS.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i8);",
        "            assert_eq!(entry.method.id(), entry.id);",
        "            assert!(names.insert(entry.constant_name));",
        "        }",
        "        names.clear();",
        "        for (id, entry) in VANILLA_PLAYER_TEAM_METHODS.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i8);",
        "            assert_eq!(entry.method.id(), entry.id);",
        "            assert!(names.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn scoreboard_method_catalogs_preserve_packet_ids() {",
        "        assert_eq!(SetObjectiveMethod::Add.id(), 0);",
        "        assert_eq!(SetObjectiveMethod::Change.id(), 2);",
        "        assert_eq!(PlayerTeamMethod::Add.id(), 0);",
        "        assert_eq!(PlayerTeamMethod::Leave.id(), 4);",
        "        assert_eq!(SetObjectiveMethod::from_id(-1), None);",
        "        assert_eq!(SetObjectiveMethod::from_id(3), None);",
        "        assert_eq!(PlayerTeamMethod::from_id(-1), None);",
        "        assert_eq!(PlayerTeamMethod::from_id(5), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_scoreboard_method_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/scoreboard_methods.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum SetObjectiveMethod {\") || production.contains(\"enum PlayerTeamMethod {\") || production.contains(\"0 => Self::Add\") || production.contains(\"1 => Self::Remove\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel scoreboard method fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_boss_event_operation_types(
    entries: list[BossEventOperationTypeEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ClientboundBossEventPacket.OperationType declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_BOSS_EVENT_OPERATION_TYPE_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(i32)]",
        "pub(crate) enum BossEventOperationType {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_BOSS_EVENT_OPERATION_TYPE_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct BossEventOperationTypeCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub(crate) operation_type: BossEventOperationType,",
        "}",
        "",
        "pub const VANILLA_BOSS_EVENT_OPERATION_TYPES: &[BossEventOperationTypeCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    BossEventOperationTypeCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        operation_type: BossEventOperationType::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl BossEventOperationType {",
        "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_BOSS_EVENT_OPERATION_TYPES",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.operation_type)",
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
        "    fn boss_event_operation_type_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_BOSS_EVENT_OPERATION_TYPES.len(), 6);",
        "        let mut names = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_BOSS_EVENT_OPERATION_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.operation_type as i32, entry.ordinal);",
        "            assert!(names.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn boss_event_operation_type_catalog_preserves_wire_order() {",
        "        assert_eq!(BossEventOperationType::Add as i32, 0);",
        "        assert_eq!(BossEventOperationType::UpdateProperties as i32, 5);",
        "        assert_eq!(BossEventOperationType::from_ordinal(-1), None);",
        "        assert_eq!(BossEventOperationType::from_ordinal(6), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_boss_event_operation_type_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/boss_event_operation_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum BossEventOperationType {\") || production.contains(\"0 => BossEventOperation::Add\") || production.contains(\"5 => BossEventOperation::UpdateProperties\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel boss-event operation-type fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_filter_mask_types(entries: list[FilterMaskTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 FilterMask.Type declaration order and serialized names.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_FILTER_MASK_TYPE_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum FilterMaskKind {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_FILTER_MASK_TYPE_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct FilterMaskTypeCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub kind: FilterMaskKind,",
        "}",
        "",
        "pub const VANILLA_FILTER_MASK_TYPES: &[FilterMaskTypeCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    FilterMaskTypeCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            f"        kind: FilterMaskKind::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl FilterMaskKind {",
        "    pub const fn ordinal(self) -> i32 { self as i32 }",
        "",
        "    pub fn as_str(self) -> &'static str {",
        "        VANILLA_FILTER_MASK_TYPES[self.ordinal() as usize].serialized_name",
        "    }",
        "",
        "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_FILTER_MASK_TYPES",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.kind)",
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
        "    fn filter_mask_type_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_FILTER_MASK_TYPES.len(), 3);",
        "        let mut names = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_FILTER_MASK_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.kind.ordinal(), entry.ordinal);",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn filter_mask_type_catalog_preserves_wire_order_and_names() {",
        "        assert_eq!(FilterMaskKind::PassThrough.as_str(), \"pass_through\");",
        "        assert_eq!(FilterMaskKind::FullyFiltered.ordinal(), 1);",
        "        assert_eq!(FilterMaskKind::PartiallyFiltered.as_str(), \"partially_filtered\");",
        "        assert_eq!(FilterMaskKind::from_ordinal(-1), None);",
        "        assert_eq!(FilterMaskKind::from_ordinal(3), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_filter_mask_type_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/filter_mask_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum FilterMaskKind {\") || production.contains(\"impl FilterMaskKind {\") || production.contains(\"0 => FilterMaskKind::PassThrough\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel filter-mask type fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_waypoint_wire_types(
    operations: list[WaypointOperationEntry],
    operation_strategy: str,
    data_types: list[WaypointDataTypeEntry],
) -> str:
    if operation_strategy != "WRAP":
        raise ValueError(f"unsupported waypoint operation strategy {operation_strategy}")
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 waypoint operation/data-type enum declaration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum WaypointOperationOutOfBoundsStrategy { Wrap }",
        "",
        "pub const VANILLA_WAYPOINT_OPERATION_OUT_OF_BOUNDS_STRATEGY: WaypointOperationOutOfBoundsStrategy = WaypointOperationOutOfBoundsStrategy::Wrap;",
        "",
    ]
    for entry in operations:
        lines.append(
            f"pub const VANILLA_WAYPOINT_OPERATION_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum WaypointOperation {",
    ))
    for entry in operations:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_WAYPOINT_OPERATION_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct WaypointOperationCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub operation: WaypointOperation,",
        "}",
        "",
        "pub const VANILLA_WAYPOINT_OPERATIONS: &[WaypointOperationCatalogEntry] = &[",
    ))
    for entry in operations:
        lines.extend((
            "    WaypointOperationCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            f"        operation: WaypointOperation::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl WaypointOperation {",
        "    pub const fn ordinal(self) -> i32 { self as i32 }",
        "",
        "    pub fn as_str(self) -> &'static str {",
        "        VANILLA_WAYPOINT_OPERATIONS[self.ordinal() as usize].serialized_name",
        "    }",
        "",
        "    pub(crate) fn from_wire_id(id: i32) -> Self {",
        "        let ordinal = id.rem_euclid(VANILLA_WAYPOINT_OPERATIONS.len() as i32);",
        "        VANILLA_WAYPOINT_OPERATIONS[ordinal as usize].operation",
        "    }",
        "}",
        "",
    ))
    for entry in data_types:
        lines.append(
            f"pub const VANILLA_WAYPOINT_DATA_TYPE_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(i32)]",
        "pub(crate) enum WaypointDataType {",
    ))
    for entry in data_types:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_WAYPOINT_DATA_TYPE_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct WaypointDataTypeCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub(crate) data_type: WaypointDataType,",
        "}",
        "",
        "pub const VANILLA_WAYPOINT_DATA_TYPES: &[WaypointDataTypeCatalogEntry] = &[",
    ))
    for entry in data_types:
        lines.extend((
            "    WaypointDataTypeCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        data_type: WaypointDataType::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl WaypointDataType {",
        "    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_WAYPOINT_DATA_TYPES",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.data_type)",
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
        "    fn waypoint_wire_catalogs_are_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_WAYPOINT_OPERATIONS.len(), 3);",
        "        assert_eq!(VANILLA_WAYPOINT_DATA_TYPES.len(), 4);",
        "        let mut names = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_WAYPOINT_OPERATIONS.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.operation.ordinal(), entry.ordinal);",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "        names.clear();",
        "        for (ordinal, entry) in VANILLA_WAYPOINT_DATA_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.data_type as i32, entry.ordinal);",
        "            assert!(names.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn waypoint_wire_catalogs_preserve_wrap_and_type_order() {",
        "        assert_eq!(WaypointOperation::Track.as_str(), \"track\");",
        "        assert_eq!(WaypointOperation::from_wire_id(-1), WaypointOperation::Update);",
        "        assert_eq!(WaypointOperation::from_wire_id(3), WaypointOperation::Track);",
        "        assert_eq!(WaypointDataType::from_ordinal(0), Some(WaypointDataType::Empty));",
        "        assert_eq!(WaypointDataType::from_ordinal(3), Some(WaypointDataType::Azimuth));",
        "        assert_eq!(WaypointDataType::from_ordinal(4), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_waypoint_wire_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/waypoint_wire_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum WaypointOperation {\") || production.contains(\"impl WaypointOperation {\") || production.contains(\"0 => WaypointData::Empty\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel waypoint wire fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_interaction_hands(
    entries: list[InteractionHandEntry], strategy: str
) -> str:
    if strategy != "ZERO":
        raise ValueError(f"unsupported interaction hand id strategy {strategy}")
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 InteractionHand explicit ids and BY_ID strategy.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum InteractionHandIdOutOfBoundsStrategy { Zero }",
        "",
        "pub const VANILLA_INTERACTION_HAND_OUT_OF_BOUNDS_STRATEGY: InteractionHandIdOutOfBoundsStrategy = InteractionHandIdOutOfBoundsStrategy::Zero;",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_INTERACTION_HAND_{entry.constant_name}_ID: i32 = {entry.hand_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum InteractionHand {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_INTERACTION_HAND_{entry.constant_name}_ID,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct InteractionHandCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub hand: InteractionHand,",
        "}",
        "",
        "pub const VANILLA_INTERACTION_HANDS: &[InteractionHandCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    InteractionHandCatalogEntry {",
            f"        id: {entry.hand_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        hand: InteractionHand::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl InteractionHand {",
        "    pub const fn id(self) -> i32 { self as i32 }",
        "",
        "    pub fn from_id(id: i32) -> Self {",
        "        VANILLA_INTERACTION_HANDS",
        "            .get(usize::try_from(id).unwrap_or(usize::MAX))",
        "            .map_or(Self::MainHand, |entry| entry.hand)",
        "    }",
        "",
        "    pub fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_INTERACTION_HANDS",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.hand)",
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
        "    fn interaction_hand_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_INTERACTION_HANDS.len(), 2);",
        "        let mut constants = HashSet::new();",
        "        for (id, entry) in VANILLA_INTERACTION_HANDS.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert_eq!(entry.hand.id(), entry.id);",
        "            assert!(constants.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn interaction_hand_catalog_preserves_ids_and_zero_fallback() {",
        "        assert_eq!(InteractionHand::MainHand.id(), 0);",
        "        assert_eq!(InteractionHand::OffHand.id(), 1);",
        "        assert_eq!(InteractionHand::from_id(1), InteractionHand::OffHand);",
        "        assert_eq!(InteractionHand::from_id(-1), InteractionHand::MainHand);",
        "        assert_eq!(InteractionHand::from_id(2), InteractionHand::MainHand);",
        "        assert_eq!(InteractionHand::from_ordinal(1), Some(InteractionHand::OffHand));",
        "        assert_eq!(InteractionHand::from_ordinal(2), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_interaction_hand_wire_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/interaction_hands.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum InteractionHand {\") || production.contains(\"fn decode_interaction_hand\") || production.contains(\"Self::MainHand => 0\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel interaction-hand wire owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_client_information_options(
    chat_visibilities: list[ClientInformationOptionEntry],
    main_hands: list[ClientInformationOptionEntry],
    particle_statuses: list[ClientInformationOptionEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ChatVisiblity, HumanoidArm, and ParticleStatus explicit ids.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ClientInformationOptionCatalogEntry<T> {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub value: T,",
        "}",
        "",
    ]

    def add_option(
        entries: list[ClientInformationOptionEntry],
        rust_type: str,
        constant_prefix: str,
        catalog_name: str,
    ) -> None:
        for entry in entries:
            lines.append(
                f"pub const {constant_prefix}_{entry.constant_name}_ID: i32 = {entry.option_id};"
            )
        lines.extend((
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            f"pub enum {rust_type} {{",
        ))
        for entry in entries:
            lines.append(
                f"    {rust_pascal_case(entry.constant_name)} = {constant_prefix}_{entry.constant_name}_ID,"
            )
        lines.extend((
            "}",
            "",
            f"pub const {catalog_name}: &[ClientInformationOptionCatalogEntry<{rust_type}>] = &[",
        ))
        for entry in entries:
            lines.extend((
                "    ClientInformationOptionCatalogEntry {",
                f"        id: {entry.option_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        value: {rust_type}::{rust_pascal_case(entry.constant_name)},",
                "    },",
            ))
        lines.extend((
            "];",
            "",
            f"impl {rust_type} {{",
            "    pub const fn id(self) -> i32 { self as i32 }",
            "",
            "    pub fn from_ordinal(ordinal: i32) -> Option<Self> {",
            f"        {catalog_name}",
            "            .get(usize::try_from(ordinal).ok()?)",
            "            .map(|entry| entry.value)",
            "    }",
            "}",
            "",
        ))

    add_option(
        chat_visibilities,
        "ClientChatVisibility",
        "VANILLA_CLIENT_CHAT_VISIBILITY",
        "VANILLA_CLIENT_CHAT_VISIBILITIES",
    )
    add_option(
        main_hands,
        "ClientMainHand",
        "VANILLA_CLIENT_MAIN_HAND",
        "VANILLA_CLIENT_MAIN_HANDS",
    )
    add_option(
        particle_statuses,
        "ClientParticleStatus",
        "VANILLA_CLIENT_PARTICLE_STATUS",
        "VANILLA_CLIENT_PARTICLE_STATUSES",
    )
    lines.extend((
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    fn assert_catalog<T: Copy>(catalog: &[ClientInformationOptionCatalogEntry<T>]) {",
        "        let mut constants = HashSet::new();",
        "        for (id, entry) in catalog.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert!(constants.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn client_information_option_catalogs_are_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_CLIENT_CHAT_VISIBILITIES.len(), 3);",
        "        assert_eq!(VANILLA_CLIENT_MAIN_HANDS.len(), 2);",
        "        assert_eq!(VANILLA_CLIENT_PARTICLE_STATUSES.len(), 3);",
        "        assert_catalog(VANILLA_CLIENT_CHAT_VISIBILITIES);",
        "        assert_catalog(VANILLA_CLIENT_MAIN_HANDS);",
        "        assert_catalog(VANILLA_CLIENT_PARTICLE_STATUSES);",
        "    }",
        "",
        "    #[test]",
        "    fn client_information_options_preserve_vanilla_wire_ordinals() {",
        "        assert_eq!(ClientChatVisibility::Full.id(), 0);",
        "        assert_eq!(ClientChatVisibility::Hidden.id(), 2);",
        "        assert_eq!(ClientMainHand::Left.id(), 0);",
        "        assert_eq!(ClientMainHand::Right.id(), 1);",
        "        assert_eq!(ClientParticleStatus::All.id(), 0);",
        "        assert_eq!(ClientParticleStatus::Minimal.id(), 2);",
        "        assert_eq!(ClientChatVisibility::from_ordinal(3), None);",
        "        assert_eq!(ClientMainHand::from_ordinal(-1), None);",
        "        assert_eq!(ClientParticleStatus::from_ordinal(1), Some(ClientParticleStatus::Decreased));",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_client_information_option_ordinals() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/client_information_options.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum ClientChatVisibility {\") || production.contains(\"enum ClientMainHand {\") || production.contains(\"enum ClientParticleStatus {\") || production.contains(\"Self::Full => 0\") || production.contains(\"Self::Left => 0\") || production.contains(\"Self::All => 0\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel client-information option owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_container_inputs(entries: list[ContainerInputEntry], strategy: str) -> str:
    if strategy != "ZERO":
        raise ValueError(f"unsupported container input id strategy {strategy}")
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ContainerInput explicit ids and BY_ID strategy.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ContainerInputIdOutOfBoundsStrategy { Zero }",
        "",
        "pub const VANILLA_CONTAINER_INPUT_OUT_OF_BOUNDS_STRATEGY: ContainerInputIdOutOfBoundsStrategy = ContainerInputIdOutOfBoundsStrategy::Zero;",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_CONTAINER_INPUT_{entry.constant_name}_ID: i32 = {entry.input_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum ContainerInput {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_CONTAINER_INPUT_{entry.constant_name}_ID,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ContainerInputCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub input: ContainerInput,",
        "}",
        "",
        "pub const VANILLA_CONTAINER_INPUTS: &[ContainerInputCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    ContainerInputCatalogEntry {",
            f"        id: {entry.input_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        input: ContainerInput::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl ContainerInput {",
        "    pub const fn id(self) -> i32 { self as i32 }",
        "",
        "    pub fn from_id(id: i32) -> Self {",
        "        VANILLA_CONTAINER_INPUTS",
        "            .get(usize::try_from(id).unwrap_or(usize::MAX))",
        "            .map_or(Self::Pickup, |entry| entry.input)",
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
        "    fn container_input_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_CONTAINER_INPUTS.len(), 7);",
        "        let mut constants = HashSet::new();",
        "        for (id, entry) in VANILLA_CONTAINER_INPUTS.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert_eq!(entry.input.id(), entry.id);",
        "            assert!(constants.insert(entry.constant_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn container_input_catalog_preserves_ids_and_zero_fallback() {",
        "        assert_eq!(ContainerInput::Pickup.id(), 0);",
        "        assert_eq!(ContainerInput::QuickCraft.id(), 5);",
        "        assert_eq!(ContainerInput::PickupAll.id(), 6);",
        "        assert_eq!(ContainerInput::from_id(4), ContainerInput::Throw);",
        "        assert_eq!(ContainerInput::from_id(-1), ContainerInput::Pickup);",
        "        assert_eq!(ContainerInput::from_id(7), ContainerInput::Pickup);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_container_input_wire_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/container_inputs.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum ContainerInput {\") || production.contains(\"Self::Pickup => 0\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel container-input wire owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
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


def generate_custom_chat_completions_actions(source_root: Path) -> str:
    source = parse_java(source_root, CUSTOM_CHAT_COMPLETIONS_ACTIONS_SOURCE)
    return format_rust(
        render_custom_chat_completions_actions(
            custom_chat_completions_action_entries(source)
        )
    )


def generate_client_command_actions(source_root: Path) -> str:
    source = parse_java(source_root, CLIENT_COMMAND_ACTIONS_SOURCE)
    return format_rust(render_client_command_actions(client_command_action_entries(source)))


def generate_player_action_kinds(source_root: Path) -> str:
    source = parse_java(source_root, PLAYER_ACTION_KINDS_SOURCE)
    return format_rust(render_player_action_kinds(player_action_kind_entries(source)))


def generate_player_command_actions(source_root: Path) -> str:
    source = parse_java(source_root, PLAYER_COMMAND_ACTIONS_SOURCE)
    return format_rust(render_player_command_actions(player_command_action_entries(source)))


def generate_entity_anchors(source_root: Path) -> str:
    source = parse_java(source_root, ENTITY_ANCHORS_SOURCE)
    return format_rust(render_entity_anchors(entity_anchor_entries(source)))


def generate_remote_debug_sample_types(source_root: Path) -> str:
    source = parse_java(source_root, REMOTE_DEBUG_SAMPLE_TYPES_SOURCE)
    return format_rust(
        render_remote_debug_sample_types(remote_debug_sample_type_entries(source))
    )


def generate_objective_render_types(source_root: Path) -> str:
    source = parse_java(source_root, OBJECTIVE_RENDER_TYPES_SOURCE)
    return format_rust(render_objective_render_types(objective_render_type_entries(source)))


def generate_advancement_frame_types(source_root: Path) -> str:
    source = parse_java(source_root, ADVANCEMENT_FRAME_TYPES_SOURCE)
    return format_rust(render_advancement_frame_types(advancement_frame_type_entries(source)))


def generate_boss_bar_styles(source_root: Path) -> str:
    source = parse_java(source_root, BOSS_BAR_STYLES_SOURCE)
    colors, overlays = boss_bar_style_entries(source)
    return format_rust(render_boss_bar_styles(colors, overlays))


def generate_scoreboard_display_slots(source_root: Path) -> str:
    source = parse_java(source_root, SCOREBOARD_DISPLAY_SLOTS_SOURCE)
    entries, strategy = scoreboard_display_slot_entries(source)
    return format_rust(render_scoreboard_display_slots(entries, strategy))


def generate_scoreboard_team_options(source_root: Path) -> str:
    source = parse_java(source_root, SCOREBOARD_TEAM_OPTIONS_SOURCE)
    visibility, visibility_strategy, collision, collision_strategy = (
        scoreboard_team_option_entries(source)
    )
    return format_rust(
        render_scoreboard_team_options(
            visibility, visibility_strategy, collision, collision_strategy
        )
    )


def generate_chat_formattings(source_root: Path) -> str:
    source = parse_java(source_root, CHAT_FORMATTINGS_SOURCE)
    return format_rust(render_chat_formattings(chat_formatting_entries(source)))


def generate_scoreboard_methods(source_root: Path) -> str:
    objective_source = parse_java(source_root, SCOREBOARD_OBJECTIVE_METHODS_SOURCE)
    team_source = parse_java(source_root, SCOREBOARD_TEAM_METHODS_SOURCE)
    objective, team = scoreboard_method_entries(objective_source, team_source)
    return format_rust(render_scoreboard_methods(objective, team))


def generate_boss_event_operation_types(source_root: Path) -> str:
    source = parse_java(source_root, BOSS_EVENT_OPERATION_TYPES_SOURCE)
    return format_rust(
        render_boss_event_operation_types(boss_event_operation_type_entries(source))
    )


def generate_filter_mask_types(source_root: Path) -> str:
    source = parse_java(source_root, FILTER_MASK_TYPES_SOURCE)
    return format_rust(render_filter_mask_types(filter_mask_type_entries(source)))


def generate_waypoint_wire_types(source_root: Path) -> str:
    packet_source = parse_java(source_root, WAYPOINT_OPERATION_SOURCE)
    waypoint_source = parse_java(source_root, WAYPOINT_DATA_TYPE_SOURCE)
    operations, operation_strategy, data_types = waypoint_wire_type_entries(
        packet_source, waypoint_source
    )
    return format_rust(
        render_waypoint_wire_types(operations, operation_strategy, data_types)
    )


def generate_interaction_hands(source_root: Path) -> str:
    source = parse_java(source_root, INTERACTION_HANDS_SOURCE)
    entries, strategy = interaction_hand_entries(source)
    return format_rust(render_interaction_hands(entries, strategy))


def generate_client_information_options(source_root: Path) -> str:
    chat_source = parse_java(source_root, CLIENT_CHAT_VISIBILITY_SOURCE)
    hand_source = parse_java(source_root, CLIENT_MAIN_HAND_SOURCE)
    particle_source = parse_java(source_root, CLIENT_PARTICLE_STATUS_SOURCE)
    chat, hands, particles = client_information_option_entries(
        chat_source, hand_source, particle_source
    )
    return format_rust(render_client_information_options(chat, hands, particles))


def generate_container_inputs(source_root: Path) -> str:
    source = parse_java(source_root, CONTAINER_INPUTS_SOURCE)
    entries, strategy = container_input_entries(source)
    return format_rust(render_container_inputs(entries, strategy))


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
    generated_custom_chat_completions_actions = generate_custom_chat_completions_actions(
        args.source_root
    )
    generated_client_command_actions = generate_client_command_actions(args.source_root)
    generated_player_action_kinds = generate_player_action_kinds(args.source_root)
    generated_player_command_actions = generate_player_command_actions(args.source_root)
    generated_entity_anchors = generate_entity_anchors(args.source_root)
    generated_remote_debug_sample_types = generate_remote_debug_sample_types(
        args.source_root
    )
    generated_objective_render_types = generate_objective_render_types(args.source_root)
    generated_advancement_frame_types = generate_advancement_frame_types(args.source_root)
    generated_boss_bar_styles = generate_boss_bar_styles(args.source_root)
    generated_scoreboard_display_slots = generate_scoreboard_display_slots(args.source_root)
    generated_scoreboard_team_options = generate_scoreboard_team_options(args.source_root)
    generated_chat_formattings = generate_chat_formattings(args.source_root)
    generated_scoreboard_methods = generate_scoreboard_methods(args.source_root)
    generated_boss_event_operation_types = generate_boss_event_operation_types(
        args.source_root
    )
    generated_filter_mask_types = generate_filter_mask_types(args.source_root)
    generated_waypoint_wire_types = generate_waypoint_wire_types(args.source_root)
    generated_interaction_hands = generate_interaction_hands(args.source_root)
    generated_client_information_options = generate_client_information_options(
        args.source_root
    )
    generated_container_inputs = generate_container_inputs(args.source_root)
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
        matches &= check_generated(
            args.custom_chat_completions_actions_output,
            generated_custom_chat_completions_actions,
        )
        matches &= check_generated(
            args.client_command_actions_output, generated_client_command_actions
        )
        matches &= check_generated(
            args.player_action_kinds_output, generated_player_action_kinds
        )
        matches &= check_generated(
            args.player_command_actions_output, generated_player_command_actions
        )
        matches &= check_generated(args.entity_anchors_output, generated_entity_anchors)
        matches &= check_generated(
            args.remote_debug_sample_types_output, generated_remote_debug_sample_types
        )
        matches &= check_generated(
            args.objective_render_types_output, generated_objective_render_types
        )
        matches &= check_generated(
            args.advancement_frame_types_output, generated_advancement_frame_types
        )
        matches &= check_generated(args.boss_bar_styles_output, generated_boss_bar_styles)
        matches &= check_generated(
            args.scoreboard_display_slots_output, generated_scoreboard_display_slots
        )
        matches &= check_generated(
            args.scoreboard_team_options_output, generated_scoreboard_team_options
        )
        matches &= check_generated(args.chat_formattings_output, generated_chat_formattings)
        matches &= check_generated(args.scoreboard_methods_output, generated_scoreboard_methods)
        matches &= check_generated(
            args.boss_event_operation_types_output, generated_boss_event_operation_types
        )
        matches &= check_generated(args.filter_mask_types_output, generated_filter_mask_types)
        matches &= check_generated(
            args.waypoint_wire_types_output, generated_waypoint_wire_types
        )
        matches &= check_generated(
            args.interaction_hands_output, generated_interaction_hands
        )
        matches &= check_generated(
            args.client_information_options_output,
            generated_client_information_options,
        )
        matches &= check_generated(
            args.container_inputs_output, generated_container_inputs
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
    args.custom_chat_completions_actions_output.write_text(
        generated_custom_chat_completions_actions
    )
    args.client_command_actions_output.write_text(generated_client_command_actions)
    args.player_action_kinds_output.write_text(generated_player_action_kinds)
    args.player_command_actions_output.write_text(generated_player_command_actions)
    args.entity_anchors_output.write_text(generated_entity_anchors)
    args.remote_debug_sample_types_output.write_text(generated_remote_debug_sample_types)
    args.objective_render_types_output.write_text(generated_objective_render_types)
    args.advancement_frame_types_output.write_text(generated_advancement_frame_types)
    args.boss_bar_styles_output.write_text(generated_boss_bar_styles)
    args.scoreboard_display_slots_output.write_text(generated_scoreboard_display_slots)
    args.scoreboard_team_options_output.write_text(generated_scoreboard_team_options)
    args.chat_formattings_output.write_text(generated_chat_formattings)
    args.scoreboard_methods_output.write_text(generated_scoreboard_methods)
    args.boss_event_operation_types_output.write_text(
        generated_boss_event_operation_types
    )
    args.filter_mask_types_output.write_text(generated_filter_mask_types)
    args.waypoint_wire_types_output.write_text(generated_waypoint_wire_types)
    args.interaction_hands_output.write_text(generated_interaction_hands)
    args.client_information_options_output.write_text(
        generated_client_information_options
    )
    args.container_inputs_output.write_text(generated_container_inputs)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
