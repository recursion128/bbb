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
    BeaconEffectReference,
    BossBarStyleEntry,
    BossEventOperationTypeEntry,
    BuiltinEnumComponentEntry,
    BuiltinEnumValueTypeEntry,
    ChatFormattingEntry,
    ClientInformationOptionEntry,
    ClientCommandActionEntry,
    ClientGameEventTypeEntry,
    ComponentValueEntry,
    ContainerInputEntry,
    ConsumeEffectTypeEntry,
    CustomChatCompletionsActionEntry,
    DebugHolderTypeEntry,
    DebugSubscriptionEntry,
    DifficultyEntry,
    EntityAnchorEntry,
    EntityDataSerializerEntry,
    EntityPoseEntry,
    EquipmentSlotEntry,
    EquipmentSlotGroupEntry,
    FilterMaskTypeEntry,
    GameplayIdExtractionError,
    GameRuleEntry,
    GameTypeEntry,
    HeightmapDebugColorEntry,
    HeightmapTypeEntry,
    InteractionHandEntry,
    MapBrightnessEntry,
    MapColorEntry,
    MenuTypeEntry,
    MobEffectEntry,
    NamedComponentValueEntry,
    ObjectiveRenderTypeEntry,
    PlayerInfoActionEntry,
    PlayerActionKindEntry,
    PlayerCommandActionEntry,
    ResourcePackResponseActionEntry,
    RemoteDebugSampleTypeEntry,
    RecipeBookCategoryEntry,
    RecipeBookSearchCategoryEntry,
    RecipeBookTypeEntry,
    RecipeDisplayTypeEntry,
    SlotDisplayTypeEntry,
    ScoreboardDisplaySlotEntry,
    ScoreboardMethodEntry,
    ScoreboardTeamOptionEntry,
    SoundSourceEntry,
    WaypointDataTypeEntry,
    WaypointOperationEntry,
    WorldClockEntry,
    StatTypeEntry,
    CustomStatEntry,
    TropicalFishPatternEntry,
    TropicalFishCommonVariantEntry,
    attribute_entries,
    attribute_modifier_type_entries,
    advancement_frame_type_entries,
    boss_bar_style_entries,
    boss_event_operation_type_entries,
    builtin_enum_component_value_entries,
    chat_formatting_entries,
    client_information_option_entries,
    container_input_entries,
    consume_effect_type_entries,
    client_command_action_entries,
    client_game_event_type_entries,
    custom_chat_completions_action_entries,
    debug_game_event_entries,
    debug_poi_type_entries,
    debug_subscription_entries,
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
    game_rule_entries,
    heightmap_debug_color_entries,
    heightmap_type_entries,
    interaction_hand_entries,
    item_component_value_type_entries,
    map_color_entries,
    menu_type_entries,
    mob_effect_entries,
    objective_render_type_entries,
    objective_criteria_names,
    player_info_action_entries,
    player_inventory_equipment_slot_indices,
    player_action_kind_entries,
    player_command_action_entries,
    resource_pack_response_action_entries,
    remote_debug_sample_type_entries,
    recipe_wire_type_entries,
    slot_display_type_entries,
    scoreboard_display_slot_entries,
    scoreboard_method_entries,
    scoreboard_team_option_entries,
    sound_source_entries,
    tropical_fish_pattern_entries,
    waypoint_wire_type_entries,
    world_clock_entries,
    stat_catalog_entries,
    beacon_effect_tiers,
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
DEFAULT_DEBUG_SUBSCRIPTION_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/debug_subscription_types.rs"
)
DEFAULT_WORLD_CLOCK_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/world_clock_types.rs"
)
DEFAULT_GAME_RULES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/game_rules.rs"
DEFAULT_STAT_TYPES_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/stat_types.rs"
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
DEFAULT_RECIPE_WIRE_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/recipe_wire_types.rs"
)
DEFAULT_ITEM_COMPONENT_VALUE_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/item_component_value_types.rs"
)
DEFAULT_CONSUME_EFFECT_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/consume_effect_types.rs"
)
DEFAULT_BUILTIN_ENUM_COMPONENT_VALUES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/builtin_enum_component_values.rs"
)
DEFAULT_ATTRIBUTE_MODIFIER_TYPES_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/attribute_modifier_types.rs"
)
DEFAULT_TROPICAL_FISH_PATTERNS_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/tropical_fish_patterns.rs"
)
DEFAULT_MAP_COLORS_OUTPUT = (
    REPO_ROOT / "crates/bbb-native/src/map_textures/generated_26_1.rs"
)
ATTRIBUTE_TYPES_SOURCE = Path(
    "net/minecraft/world/entity/ai/attributes/Attributes.java"
)
MOB_EFFECT_TYPES_SOURCE = Path("net/minecraft/world/effect/MobEffects.java")
BEACON_BLOCK_ENTITY_SOURCE = Path(
    "net/minecraft/world/level/block/entity/BeaconBlockEntity.java"
)
MENU_TYPES_SOURCE = Path("net/minecraft/world/inventory/MenuType.java")
EQUIPMENT_SLOTS_SOURCE = Path("net/minecraft/world/entity/EquipmentSlot.java")
PLAYER_INVENTORY_SOURCE = Path("net/minecraft/world/entity/player/Inventory.java")
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
HEIGHTMAP_DEBUG_COLORS_SOURCE = Path(
    "net/minecraft/client/renderer/debug/HeightMapRenderer.java"
)
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
DEBUG_SUBSCRIPTION_TYPES_SOURCE = Path(
    "net/minecraft/util/debug/DebugSubscriptions.java"
)
DEBUG_GAME_EVENT_TYPES_SOURCE = Path(
    "net/minecraft/world/level/gameevent/GameEvent.java"
)
DEBUG_POI_TYPES_SOURCE = Path(
    "net/minecraft/world/entity/ai/village/poi/PoiTypes.java"
)
WORLD_CLOCK_TYPES_SOURCE = Path("net/minecraft/world/clock/WorldClocks.java")
GAME_RULES_SOURCE = Path("net/minecraft/world/level/gamerules/GameRules.java")
STAT_TYPES_SOURCE = Path("net/minecraft/stats/Stats.java")
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
RECIPE_BOOK_TYPES_SOURCE = Path("net/minecraft/world/inventory/RecipeBookType.java")
RECIPE_DISPLAY_TYPES_SOURCE = Path(
    "net/minecraft/world/item/crafting/display/RecipeDisplays.java"
)
SLOT_DISPLAY_TYPES_SOURCE = Path(
    "net/minecraft/world/item/crafting/display/SlotDisplays.java"
)
RECIPE_BOOK_CATEGORIES_SOURCE = Path(
    "net/minecraft/world/item/crafting/RecipeBookCategories.java"
)
SEARCH_RECIPE_BOOK_CATEGORIES_SOURCE = Path(
    "net/minecraft/client/gui/screens/recipebook/SearchRecipeBookCategory.java"
)
ITEM_USE_ANIMATIONS_SOURCE = Path("net/minecraft/world/item/ItemUseAnimation.java")
SWING_ANIMATION_TYPES_SOURCE = Path("net/minecraft/world/item/SwingAnimationType.java")
ITEM_RARITIES_SOURCE = Path("net/minecraft/world/item/Rarity.java")
MAP_POST_PROCESSING_SOURCE = Path(
    "net/minecraft/world/item/component/MapPostProcessing.java"
)
FIREWORK_EXPLOSION_SOURCE = Path(
    "net/minecraft/world/item/component/FireworkExplosion.java"
)
CONSUME_EFFECT_TYPES_SOURCE = Path(
    "net/minecraft/world/item/consume_effects/ConsumeEffect.java"
)
DATA_COMPONENTS_SOURCE = Path("net/minecraft/core/component/DataComponents.java")
DYE_COLOR_SOURCE = Path("net/minecraft/world/item/DyeColor.java")
FOX_SOURCE = Path("net/minecraft/world/entity/animal/fox/Fox.java")
SALMON_SOURCE = Path("net/minecraft/world/entity/animal/fish/Salmon.java")
PARROT_SOURCE = Path("net/minecraft/world/entity/animal/parrot/Parrot.java")
MUSHROOM_COW_SOURCE = Path("net/minecraft/world/entity/animal/cow/MushroomCow.java")
RABBIT_SOURCE = Path("net/minecraft/world/entity/animal/rabbit/Rabbit.java")
HORSE_VARIANT_SOURCE = Path("net/minecraft/world/entity/animal/equine/Variant.java")
LLAMA_SOURCE = Path("net/minecraft/world/entity/animal/equine/Llama.java")
AXOLOTL_SOURCE = Path("net/minecraft/world/entity/animal/axolotl/Axolotl.java")
ATTRIBUTE_MODIFIER_SOURCE = Path(
    "net/minecraft/world/entity/ai/attributes/AttributeModifier.java"
)
ITEM_ATTRIBUTE_MODIFIERS_SOURCE = Path(
    "net/minecraft/world/item/component/ItemAttributeModifiers.java"
)
TROPICAL_FISH_SOURCE = Path(
    "net/minecraft/world/entity/animal/fish/TropicalFish.java"
)
MAP_COLORS_SOURCE = Path("net/minecraft/world/level/material/MapColor.java")


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
        "--debug-subscription-types-output",
        type=Path,
        default=DEFAULT_DEBUG_SUBSCRIPTION_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--world-clock-types-output",
        type=Path,
        default=DEFAULT_WORLD_CLOCK_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--game-rules-output", type=Path, default=DEFAULT_GAME_RULES_OUTPUT
    )
    parser.add_argument(
        "--stat-types-output", type=Path, default=DEFAULT_STAT_TYPES_OUTPUT
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
    parser.add_argument(
        "--recipe-wire-types-output", type=Path, default=DEFAULT_RECIPE_WIRE_TYPES_OUTPUT
    )
    parser.add_argument(
        "--item-component-value-types-output",
        type=Path,
        default=DEFAULT_ITEM_COMPONENT_VALUE_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--consume-effect-types-output",
        type=Path,
        default=DEFAULT_CONSUME_EFFECT_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--builtin-enum-component-values-output",
        type=Path,
        default=DEFAULT_BUILTIN_ENUM_COMPONENT_VALUES_OUTPUT,
    )
    parser.add_argument(
        "--attribute-modifier-types-output",
        type=Path,
        default=DEFAULT_ATTRIBUTE_MODIFIER_TYPES_OUTPUT,
    )
    parser.add_argument(
        "--tropical-fish-patterns-output",
        type=Path,
        default=DEFAULT_TROPICAL_FISH_PATTERNS_OUTPUT,
    )
    parser.add_argument(
        "--map-colors-output", type=Path, default=DEFAULT_MAP_COLORS_OUTPUT
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


def render_map_colors(
    colors: list[MapColorEntry],
    brightness: list[MapBrightnessEntry],
    palette_size: int,
) -> str:
    palette = [0] * palette_size
    names = ["NONE"] * palette_size
    for entry in colors:
        palette[entry.color_id] = entry.rgb
        names[entry.color_id] = entry.constant_name
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 MapColor.java material colors and Brightness enum.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        f"const MAP_MATERIAL_COLORS: [u32; {palette_size}] = [",
    ]
    for color_id, (name, rgb) in enumerate(zip(names, palette)):
        lines.append(f"    0x{rgb:06X}, // {color_id}: {name}")
    lines.extend((
        "];",
        "",
        f"const MAP_BRIGHTNESS_MODIFIERS: [u32; {len(brightness)}] = [",
    ))
    for entry in brightness:
        lines.append(
            f"    {entry.modifier}, // {entry.brightness_id}: {entry.constant_name}"
        )
    lines.extend((
        "];",
        "",
        "#[cfg(test)]",
        "mod generated_26_1_tests {",
        "    use std::{fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn generated_map_color_tables_cover_the_packed_id_domains() {",
        f"        assert_eq!(MAP_MATERIAL_COLORS.len(), {palette_size});",
        f"        assert!(MAP_MATERIAL_COLORS[{len(colors)}..].iter().all(|color| *color == 0));",
        f"        assert_eq!(MAP_BRIGHTNESS_MODIFIERS.len(), {len(brightness)});",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_map_color_tables() {",
        '        let native = include_str!("../map_textures.rs");',
        '        let projection = native.split("pub(crate) fn map_color_rgba8").nth(1).and_then(|source| source.split("#[cfg(test)]").next()).expect("map color projection");',
        '        assert_eq!(projection.matches("MAP_MATERIAL_COLORS").count(), 1);',
        '        assert_eq!(projection.matches("MAP_BRIGHTNESS_MODIFIERS").count(), 1);',
        '        assert!(!projection.contains("match material_id"));',
        '        let fixture = native.split("fn map_color_rgba_matches_generated_full_packed_domain").nth(1).and_then(|source| source.split_once("\\n    }")).map(|(body, _)| body).expect("map color full-domain fixture");',
        '        assert!(fixture.contains("u8::MIN..=u8::MAX"));',
        '        assert_eq!(fixture.matches("MAP_MATERIAL_COLORS").count(), 1);',
        '        assert_eq!(fixture.matches("MAP_BRIGHTNESS_MODIFIERS").count(), 1);',
        '        for removed in ["MapColor.GRASS", "0x7f", "109, 153, 48"] {',
        '            assert!(!fixture.contains(removed), "fixture restored {removed}");',
        '        }',
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/map_textures/generated_26_1.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"const MAP_MATERIAL_COLORS\") || production.contains(\"const MAP_BRIGHTNESS_MODIFIERS\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel map color table owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


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
        "    pub default_value_bits: u64,",
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
                f"        default_value_bits: {entry.default_value_bits},",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "        assert_eq!(f64::from_bits(vanilla_attribute_type(VANILLA_ATTRIBUTE_MAX_HEALTH_ID).unwrap().default_value_bits), 20.0);",
            "        assert_eq!(f64::from_bits(vanilla_attribute_type(VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID).unwrap().default_value_bits), f64::from(0.42_f32));",
            "        assert_eq!(vanilla_attribute_type(7).unwrap().sentiment, AttributeSentiment::Negative);",
            "        assert_eq!(vanilla_attribute_type(25).unwrap().sentiment, AttributeSentiment::Neutral);",
            "        assert_eq!(vanilla_attribute_type_for_resource_id(\"minecraft:attack_speed\").unwrap().id, 4);",
            "        assert_eq!(vanilla_attribute_type(-1), None);",
            "        assert_eq!(vanilla_attribute_type(35), None);",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_attribute_catalog_lookups() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/attribute_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"struct VanillaAttribute\") || production.contains(\"fn vanilla_attribute(\") || production.contains(\"fn vanilla_attribute_for_key(\") || production.contains(\"fn vanilla_attribute_id_for_key(\") || production.contains(\"fn vanilla_attribute_resource_id(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel attribute catalog lookup bridges: {violations:?}\");",
            "    }",
            "",
            '    fn contains_numeric_attribute_id_reference(source: &str) -> bool {',
            "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
            '        let numeric_construction = compact.match_indices("attribute_id:").any(|(offset, _)| {',
            '            compact[offset + "attribute_id:".len()..].chars().next().is_some_and(|character| character.is_ascii_digit())',
            "        });",
            '        numeric_construction || compact.match_indices(".attribute_id").any(|(offset, _)| {',
            '            let suffix = &compact[offset + ".attribute_id".len()..];',
            '            [",", "==", "!=", "="].iter().any(|separator| {',
            "                suffix.strip_prefix(separator).is_some_and(|value| value.chars().next().is_some_and(|character| character.is_ascii_digit()))",
            "            })",
            "        })",
            "    }",
            "",
            "    #[test]",
            "    fn numeric_attribute_id_guard_covers_construction_and_comparison() {",
            '        assert!(contains_numeric_attribute_id_reference("attribute_id: 19,"));',
            '        assert!(contains_numeric_attribute_id_reference("assert_eq!(snapshot.attribute_id,\\n 22);"));',
            '        assert!(!contains_numeric_attribute_id_reference("attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,"));',
            '        assert!(!contains_numeric_attribute_id_reference("assert_eq!(snapshot.attribute_id, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);"));',
            "    }",
            "",
            "    #[test]",
            "    fn update_attribute_semantic_sources_use_generated_catalog_ids() {",
            "        fn visit(path: &Path, violations: &mut Vec<String>) {",
            "            if path.is_dir() { for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); } return; }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let source = fs::read_to_string(path).unwrap();",
            '            if contains_numeric_attribute_id_reference(&source) { violations.push(format!("{}: numeric attribute_id reference", path.display())); }',
            "        }",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let mut violations = Vec::new();",
            "        for path in [crates.join(\"bbb-net/src\"), crates.join(\"bbb-world/src/entities\"), crates.join(\"bbb-world/src/level.rs\"), crates.join(\"bbb-native/src/runtime\")] { visit(&path, &mut violations); }",
            '        assert!(violations.is_empty(), "semantic UpdateAttributes consumers restored numeric attribute ids:\\n{}", violations.join("\\n"));',
            "    }",
            "",
            "    #[test]",
            "    fn update_attributes_keeps_session_resolution_at_the_world_boundary() {",
            '        let protocol = include_str!("packets/entities.rs");',
            '        let decode = protocol.split("pub(super) fn decode_update_attributes").nth(1).and_then(|source| source.split("pub(super) fn decode_attribute_holder_id").next()).expect("UpdateAttributes decoder");',
            '        assert!(decode.contains("decoder.read_var_i32()?"));',
            '        assert!(!decode.contains("decode_attribute_holder_id"));',
            '        let world = include_str!("../../bbb-world/src/entities/metadata.rs");',
            '        let apply = world.split("pub fn apply_update_attributes").nth(1).and_then(|source| source.split("\\n    }").next()).expect("UpdateAttributes world apply");',
            '        assert!(apply.contains("frozen_registry_access()"));',
            '        assert!(apply.contains("vanilla_attribute_type_for_resource_id"));',
            '        assert!(apply.contains("vanilla_entity_type_has_attribute"));',
            "    }",
            "",
            "    #[test]",
            "    fn builtin_attribute_component_sources_use_generated_catalog_ids() {",
            "        fn visit(path: &Path, violations: &mut Vec<String>) {",
            "            if path.is_dir() { for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); } return; }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let source = fs::read_to_string(path).unwrap();",
            '            if contains_numeric_attribute_id_reference(&source) { violations.push(format!("{}: numeric attribute_id reference", path.display())); }',
            "        }",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let mut violations = Vec::new();",
            "        for path in [crates.join(\"bbb-native/src/input\"), crates.join(\"bbb-world/src/client/dialog\")] { visit(&path, &mut violations); }",
            '        assert!(violations.is_empty(), "built-in attribute component consumers restored numeric attribute ids:\\n{}", violations.join("\\n"));',
            "    }",
            "",
            "    #[test]",
            "    fn attribute_fixtures_do_not_restore_fixed_scale_or_max_health_ids() {",
            '        let player_pose = include_str!("../../bbb-world/src/entities/tests/player_pose_and_sleeping_projection.rs");',
            '        let creature_bounds = include_str!("../../bbb-world/src/entities/tests/creature_pick_bounds_projection.rs");',
            '        let specialized_bounds = include_str!("../../bbb-world/src/entities/tests/specialized_pick_bounds_and_render_scale.rs");',
            '        let native_scene = include_str!("../../bbb-native/src/entity_scene/tests.rs");',
            "        for source in [player_pose, creature_bounds, specialized_bounds, native_scene] {",
            '            assert!(source.contains("VANILLA_ATTRIBUTE_SCALE_ID"));',
            '            assert!(!source.contains("const VANILLA_ATTRIBUTE_SCALE_ID"));',
            "        }",
            '        let local_player = include_str!("../../bbb-world/src/client/local_player.rs");',
            '        assert!(local_player.contains("VANILLA_ATTRIBUTE_MAX_HEALTH_ID"));',
            '        assert!(!local_player.contains("const VANILLA_ATTRIBUTE_MAX_HEALTH_ID"));',
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_mob_effects(
    entries: list[MobEffectEntry],
    beacon_tiers: list[tuple[BeaconEffectReference, ...]],
) -> str:
    attribute_constants = sorted(
        {
            modifier.attribute_constant_name
            for entry in entries
            for modifier in entry.attribute_modifiers
        }
    )
    operation_constants = sorted(
        {
            modifier.operation_constant_name
            for entry in entries
            for modifier in entry.attribute_modifiers
        }
    )
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 MobEffects.java registrations and BeaconBlockEntity.java BEACON_EFFECTS.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use crate::{",
        "    attribute_modifier_types::{",
    ]
    for constant in operation_constants:
        lines.append(f"        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_{constant}_ID,")
    lines.extend((
        "    },",
        "    attribute_types::{",
    ))
    for constant in attribute_constants:
        lines.append(f"        VANILLA_ATTRIBUTE_{constant}_ID,")
    lines.extend((
        "    },",
        "};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum MobEffectCategory {",
        "    Beneficial,",
        "    Harmful,",
        "    Neutral,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct MobEffectAttributeModifierDescriptor {",
        "    pub attribute_id: i32,",
        "    pub modifier_id: &'static str,",
        "    pub amount_bits: u64,",
        "    pub operation_id: i32,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct MobEffectTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub category: MobEffectCategory,",
        "    pub attribute_modifiers: &'static [MobEffectAttributeModifierDescriptor],",
        "}",
        "",
    ))
    for entry_id, entry in enumerate(entries):
        lines.append(
            f"pub const VANILLA_MOB_EFFECT_{entry.constant_name}_ID: i32 = {entry_id};"
        )
    lines.extend(
        (
            "",
            "pub const VANILLA_BEACON_EFFECT_TIERS: &[&[i32]] = &[",
        )
    )
    for tier in beacon_tiers:
        constants = ", ".join(
            f"VANILLA_MOB_EFFECT_{reference.constant_name}_ID" for reference in tier
        )
        lines.append(f"    &[{constants}],")
    lines.extend(
        (
            "];",
            "",
            "pub const VANILLA_MOB_EFFECT_TYPES: &[MobEffectTypeCatalogEntry] = &[",
        )
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
                "        attribute_modifiers: &[",
            )
        )
        for modifier in entry.attribute_modifiers:
            lines.extend(
                (
                    "            MobEffectAttributeModifierDescriptor {",
                    f"                attribute_id: VANILLA_ATTRIBUTE_{modifier.attribute_constant_name}_ID,",
                    f"                modifier_id: {rust_string(modifier.modifier_id)},",
                    f"                amount_bits: {modifier.amount_bits},",
                    f"                operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_{modifier.operation_constant_name}_ID,",
                    "            },",
                )
            )
        lines.extend(
            (
                "        ],",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "        assert_eq!(VANILLA_MOB_EFFECT_TYPES.iter().map(|entry| entry.attribute_modifiers.len()).sum::<usize>(), 12);",
            "        assert_eq!(vanilla_mob_effect_type(VANILLA_MOB_EFFECT_SPEED_ID).unwrap().attribute_modifiers[0].modifier_id, \"minecraft:effect.speed\");",
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
            "    fn beacon_effect_tiers_preserve_vanilla_membership_and_order() {",
            "        assert_eq!(VANILLA_BEACON_EFFECT_TIERS.len(), 4);",
            "        assert_eq!(VANILLA_BEACON_EFFECT_TIERS[0], &[VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_HASTE_ID]);",
            "        assert_eq!(VANILLA_BEACON_EFFECT_TIERS[1], &[VANILLA_MOB_EFFECT_RESISTANCE_ID, VANILLA_MOB_EFFECT_JUMP_BOOST_ID]);",
            "        assert_eq!(VANILLA_BEACON_EFFECT_TIERS[2], &[VANILLA_MOB_EFFECT_STRENGTH_ID]);",
            "        assert_eq!(VANILLA_BEACON_EFFECT_TIERS[3], &[VANILLA_MOB_EFFECT_REGENERATION_ID]);",
            "    }",
            "",
            "    #[test]",
            "    fn production_sources_do_not_redeclare_mob_effect_catalog_facts() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let display = path.to_string_lossy();",
            "                if display.ends_with(\"/mob_effect_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "                if production.contains(\"SPEED_ATTRIBUTE_MODIFIERS\") || production.contains(\"fn attribute_modifier(\\n\") || production.contains(\"const BEACON_PRIMARY_EFFECT_ROWS\") || production.contains(\"const BEACON_SECONDARY_EFFECTS\") || production.contains(\"struct VanillaMobEffect\") || production.contains(\"fn vanilla_mob_effect(\") || production.contains(\"fn vanilla_mob_effect_key(\") || production.contains(\"fn vanilla_mob_effect_category(\") || production.contains(\"fn vanilla_mob_effect_attribute_modifiers(\") || production.contains(\"fn vanilla_mob_effect_resource_id(\") { violations.push(display.to_string()); }",
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
            "        assert!(violations.is_empty(), \"parallel mob-effect catalog owners or lookup bridges: {violations:?}\");",
            "    }",
            "",
            "    #[test]",
            "    fn swing_duration_fixtures_do_not_restore_fixed_mob_effect_ids() {",
            '        let source = include_str!("../../bbb-world/src/entities/tests/movement_combat_and_swing_projection.rs");',
            '        for required in ["VANILLA_MOB_EFFECT_HASTE_ID", "VANILLA_MOB_EFFECT_MINING_FATIGUE_ID", "VANILLA_MOB_EFFECT_CONDUIT_POWER_ID"] {',
            '            assert!(source.contains(required), "swing-duration fixture missing {required}");',
            '            let removed = format!("const {required}");',
            '            assert!(!source.contains(&removed), "swing-duration fixture restored {removed}");',
            "        }",
            "    }",
            "",
            "    #[test]",
            "    fn local_movement_fixtures_do_not_restore_fixed_mob_effect_ids() {",
            '        let source = include_str!("../../bbb-world/src/client/local_player_movement/tests.rs");',
            '        let required = "VANILLA_MOB_EFFECT_BAD_OMEN_ID";',
            '        assert!(source.contains(required), "local-movement fixture missing {required}");',
            '        let removed = format!("const {required}");',
            '        assert!(!source.contains(&removed), "local-movement fixture restored {removed}");',
            "    }",
            "",
            "    fn starts_with_integer_literal(value: &str) -> bool {",
            "        let mut characters = value.chars();",
            "        let Some(first) = characters.next() else { return false; };",
            "        first.is_ascii_digit() || (first == '-' && characters.next().is_some_and(|character| character.is_ascii_digit()))",
            "    }",
            "",
            "    fn contains_numeric_mob_effect_id_reference(source: &str) -> bool {",
            "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
            '        let numeric_construction = compact.match_indices("effect_id:").any(|(offset, _)| {',
            '            let boundary = offset == 0 || compact[..offset].chars().next_back().is_some_and(|character| !character.is_ascii_alphanumeric() && character != \'_\');',
            '            boundary && starts_with_integer_literal(&compact[offset + "effect_id:".len()..])',
            "        });",
            '        let numeric_field_reference = compact.match_indices(".effect_id").any(|(offset, _)| {',
            '            let suffix = &compact[offset + ".effect_id".len()..];',
            '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
            "        });",
            '        let numeric_holder_reference = compact.match_indices(".effect.holder_id").any(|(offset, _)| {',
            '            let suffix = &compact[offset + ".effect.holder_id".len()..];',
            '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
            "        });",
            '        let numeric_holder_construction = compact.match_indices("effect:holder(").any(|(offset, _)| starts_with_integer_literal(&compact[offset + "effect:holder(".len()..]));',
            '        let numeric_query = compact.match_indices("entity_effect(").any(|(offset, _)| {',
            '            compact[offset + "entity_effect(".len()..].split(\')\').next().and_then(|arguments| arguments.split(\',\').nth(1)).is_some_and(starts_with_integer_literal)',
            "        });",
            '        let numeric_map_lookup = compact.match_indices(".effects.get(&").any(|(offset, _)| {',
            '            let prefix = &compact[..offset];',
            '            let belongs_to_mob_effects = prefix.rfind("mob_effects(").is_some_and(|start| !prefix[start..].contains(\';\'));',
            '            belongs_to_mob_effects && starts_with_integer_literal(&compact[offset + ".effects.get(&".len()..])',
            "        });",
            "        numeric_construction || numeric_field_reference || numeric_holder_reference || numeric_holder_construction || numeric_query || numeric_map_lookup",
            "    }",
            "",
            "    #[test]",
            "    fn numeric_mob_effect_id_guard_covers_construction_queries_and_comparison() {",
            '        assert!(contains_numeric_mob_effect_id_reference("effect_id: 18,"));',
            '        assert!(contains_numeric_mob_effect_id_reference("snapshot.effect_id = 40;"));',
            '        assert!(contains_numeric_mob_effect_id_reference("effect: holder(18, \\"minecraft:poison\\")"));',
            '        assert!(contains_numeric_mob_effect_id_reference("effect.effect.holder_id == 2"));',
            '        assert!(contains_numeric_mob_effect_id_reference("world.entity_effect(entity_id,\\n 3)"));',
            '        assert!(contains_numeric_mob_effect_id_reference("snapshot.mob_effects(entity_id).unwrap().effects.get(&3)"));',
            '        assert!(!contains_numeric_mob_effect_id_reference("other.effects.get(&3)"));',
            '        assert!(!contains_numeric_mob_effect_id_reference("block_effect_id: 3,"));',
            '        assert!(!contains_numeric_mob_effect_id_reference("effect_id: VANILLA_MOB_EFFECT_POISON_ID,"));',
            '        assert!(!contains_numeric_mob_effect_id_reference("world.entity_effect(entity_id, unknown_effect_id)"));',
            "    }",
            "",
            "    #[test]",
            "    fn semantic_sources_do_not_restore_numeric_mob_effect_ids() {",
            "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
            "            for entry in fs::read_dir(directory).unwrap() {",
            "                let path = entry.unwrap().path();",
            "                if path.is_dir() { visit(&path, violations); continue; }",
            "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
            "                let source = fs::read_to_string(&path).unwrap();",
            '                if contains_numeric_mob_effect_id_reference(&source) { violations.push(format!("{}: numeric mob-effect id reference", path.display())); }',
            "            }",
            "        }",
            "        let mut violations = Vec::new();",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        for directory in [\"bbb-native/src\", \"bbb-world/src\", \"bbb-net/src\", \"bbb-item-model/src\", \"bbb-renderer/src\"] { visit(&crates.join(directory), &mut violations); }",
            '        assert!(violations.is_empty(), "semantic consumers restored numeric mob-effect ids:\\n{}", violations.join("\\n"));',
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "    fn starts_with_integer_literal(value: &str) -> bool {",
            "        let mut characters = value.chars();",
            "        let Some(first) = characters.next() else { return false; };",
            "        first.is_ascii_digit() || (first == '-' && characters.next().is_some_and(|character| character.is_ascii_digit()))",
            "    }",
            "",
            "    fn contains_numeric_menu_type_id_reference(source: &str) -> bool {",
            "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
            '        let numeric_construction = compact.match_indices("menu_type_id:").any(|(offset, _)| {',
            '            let boundary = offset == 0 || compact[..offset].chars().next_back().is_some_and(|character| !character.is_ascii_alphanumeric() && character != \'_\');',
            '            boundary && starts_with_integer_literal(&compact[offset + "menu_type_id:".len()..])',
            "        });",
            '        let numeric_field_reference = compact.match_indices(".menu_type_id").any(|(offset, _)| {',
            '            let suffix = &compact[offset + ".menu_type_id".len()..];',
            '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
            '                || [",Some(", "==Some(", "!=Some(", "=Some("].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
            "        });",
            "        numeric_construction || numeric_field_reference",
            "    }",
            "",
            "    #[test]",
            "    fn numeric_menu_type_guard_covers_semantic_consumers() {",
            '        assert!(contains_numeric_menu_type_id_reference("menu_type_id: 2,"));',
            '        assert!(contains_numeric_menu_type_id_reference("assert_eq!(container.menu_type_id, Some(19));"));',
            '        assert!(contains_numeric_menu_type_id_reference("container.menu_type_id = Some(24);"));',
            '        assert!(!contains_numeric_menu_type_id_reference("menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,"));',
            '        assert!(!contains_numeric_menu_type_id_reference("assert_eq!(container.menu_type_id, Some(VANILLA_MENU_TYPE_MERCHANT_ID));"));',
            "    }",
            "",
            "    #[test]",
            "    fn semantic_consumers_use_generated_menu_type_ids() {",
            "        fn visit(path: &Path, violations: &mut Vec<String>) {",
            "            if path.is_dir() {",
            "                for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); }",
            "                return;",
            "            }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let source = fs::read_to_string(path).unwrap();",
            '            if contains_numeric_menu_type_id_reference(&source) { violations.push(format!("{}: numeric menu type id reference", path.display())); }',
            "        }",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let mut violations = Vec::new();",
            "        for directory in [\"bbb-world/src\", \"bbb-net/src\", \"bbb-control/src\", \"bbb-native/src\", \"bbb-item-model/src\", \"bbb-renderer/src\"] { visit(&crates.join(directory), &mut violations); }",
            '        assert!(violations.is_empty(), "semantic consumers restored numeric menu type ids:\\n{}", violations.join("\\n"));',
            "    }",
            "",
            "    #[test]",
            "    fn menu_consumers_do_not_restore_local_id_aliases() {",
            "        fn visit(path: &Path, violations: &mut Vec<String>) {",
            "            if path.is_dir() {",
            "                for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); }",
            "                return;",
            "            }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let source = fs::read_to_string(path).unwrap();",
            "            for identifier in source.split(|character: char| !character.is_ascii_alphanumeric() && character != '_') {",
            "                if (identifier.ends_with(\"_MENU_TYPE_ID\") && !identifier.starts_with(\"VANILLA_MENU_TYPE_\")) || identifier == \"VANILLA_MENU_TYPE_CRAFTER_ID\" {",
            "                    violations.push(format!(\"{}: {identifier}\", path.display()));",
            "                }",
            "            }",
            "        }",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let mut violations = Vec::new();",
            "        for path in [",
            "            crates.join(\"bbb-native/src/input/inventory.rs\"),",
            "            crates.join(\"bbb-native/src/input/inventory\"),",
            "            crates.join(\"bbb-native/src/runtime.rs\"),",
            "            crates.join(\"bbb-world/src/client/inventory.rs\"),",
            "            crates.join(\"bbb-world/src/client/inventory\"),",
            "            crates.join(\"bbb-net/src/probe/play.rs\"),",
            "        ] { visit(&path, &mut violations); }",
            "        assert!(violations.is_empty(), \"local menu type aliases:\\n{}\", violations.join(\"\\n\"));",
            "    }",
            "}",
            "",
        )
    )
    return "\n".join(lines)


def render_equipment_slots(
    slots: list[EquipmentSlotEntry],
    player_inventory_slots: dict[str, int],
    groups: list[EquipmentSlotGroupEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 EquipmentSlot.java, player Inventory.java, and EquipmentSlotGroup.java.",
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
        "    pub player_inventory_slot: Option<i32>,",
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
    lines.append("")
    for entry in slots:
        if entry.constant_name in player_inventory_slots:
            lines.append(
                f"pub const VANILLA_PLAYER_INVENTORY_{entry.constant_name}_SLOT: i32 = "
                f"{player_inventory_slots[entry.constant_name]};"
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
                "        player_inventory_slot: "
                + (
                    f"Some(VANILLA_PLAYER_INVENTORY_{entry.constant_name}_SLOT),"
                    if entry.constant_name in player_inventory_slots
                    else "None,"
                ),
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
            "        assert_eq!(VANILLA_PLAYER_INVENTORY_FEET_SLOT, 36);",
            "        assert_eq!(VANILLA_PLAYER_INVENTORY_CHEST_SLOT, 38);",
            "        assert_eq!(VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT, 40);",
            "        assert_eq!(VANILLA_PLAYER_INVENTORY_SADDLE_SLOT, 42);",
            "        assert_eq!(vanilla_equipment_slot_by_id(VANILLA_EQUIPMENT_SLOT_MAINHAND_ID).unwrap().player_inventory_slot, None);",
            "        assert_eq!(vanilla_equipment_slot_by_id(VANILLA_EQUIPMENT_SLOT_CHEST_ID).unwrap().player_inventory_slot, Some(VANILLA_PLAYER_INVENTORY_CHEST_SLOT));",
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
            "",
            "    #[test]",
            "    fn player_inventory_consumers_use_generated_equipment_slot_mapping() {",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let inventory = fs::read_to_string(crates.join(\"bbb-world/src/client/inventory.rs\")).unwrap();",
            "        for required in [",
            "            \"VANILLA_PLAYER_INVENTORY_FEET_SLOT\", \"VANILLA_PLAYER_INVENTORY_LEGS_SLOT\",",
            "            \"VANILLA_PLAYER_INVENTORY_CHEST_SLOT\", \"VANILLA_PLAYER_INVENTORY_HEAD_SLOT\",",
            "            \"VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT\", \"VANILLA_PLAYER_INVENTORY_BODY_SLOT\",",
            "            \"VANILLA_PLAYER_INVENTORY_SADDLE_SLOT\",",
            "        ] { assert!(inventory.contains(required), \"world inventory missing {required}\"); }",
            "        let slot_ops = fs::read_to_string(crates.join(\"bbb-world/src/client/inventory/slot_ops.rs\")).unwrap();",
            "        let input_tests = fs::read_to_string(crates.join(\"bbb-native/src/input/tests.rs\")).unwrap();",
            "        let mouse = fs::read_to_string(crates.join(\"bbb-native/src/input/mouse.rs\")).unwrap();",
            "        let destroy = fs::read_to_string(crates.join(\"bbb-world/src/client/local_player_destroy.rs\")).unwrap();",
            "        let movement = fs::read_to_string(crates.join(\"bbb-world/src/client/local_player_movement/tests.rs\")).unwrap();",
            "        assert!(slot_ops.contains(\"VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT\"));",
            "        assert!(!slot_ops.lines().any(|line| line.contains(\"40\") && line.contains(\"OFFHAND\")), \"slot operations restored raw offhand inventory button\");",
            "        assert!(input_tests.contains(\"VANILLA_PLAYER_INVENTORY_CHEST_SLOT\"));",
            "        assert!(mouse.contains(\"VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT\"));",
            "        assert!(destroy.contains(\"VANILLA_PLAYER_INVENTORY_HEAD_SLOT\"));",
            "        assert!(movement.contains(\"VANILLA_PLAYER_INVENTORY_FEET_SLOT\"));",
            "        let consumers = [inventory.as_str(), slot_ops.as_str(), input_tests.as_str(), mouse.as_str(), destroy.as_str(), movement.as_str()];",
            "        for removed in [",
            "            \"PLAYER_FEET_EQUIPMENT_SLOT\", \"PLAYER_LEGS_EQUIPMENT_SLOT\",",
            "            \"PLAYER_CHEST_EQUIPMENT_SLOT\", \"PLAYER_HEAD_EQUIPMENT_SLOT\",",
            "            \"PLAYER_OFFHAND_SLOT\", \"PLAYER_BODY_EQUIPMENT_SLOT\",",
            "            \"PLAYER_SADDLE_EQUIPMENT_SLOT\", \"VANILLA_PLAYER_CHEST_EQUIPMENT_SLOT\",",
            "            \"VANILLA_PLAYER_OFFHAND_SLOT\",",
            "        ] {",
            "            assert!(!consumers.iter().any(|source| source.contains(removed)), \"consumer restored removed {removed}\");",
            "        }",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "    fn entity_pose_consumers_do_not_restore_local_id_aliases() {",
            "        fn visit(path: &Path, violations: &mut Vec<String>) {",
            "            if path.is_dir() {",
            "                for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); }",
            "                return;",
            "            }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let source = fs::read_to_string(path).unwrap();",
            "            for identifier in source.split(|character: char| !character.is_ascii_alphanumeric() && character != '_') {",
            "                if identifier.starts_with(\"VANILLA_POSE_\") {",
            "                    violations.push(format!(\"{}: {identifier}\", path.display()));",
            "                }",
            "            }",
            "        }",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let mut violations = Vec::new();",
            "        for path in [",
            "            crates.join(\"bbb-world/src/entities\"),",
            "            crates.join(\"bbb-world/src/entities.rs\"),",
            "            crates.join(\"bbb-world/src/client/local_player_collision.rs\"),",
            "            crates.join(\"bbb-native/src/input/tests.rs\"),",
            "        ] { visit(&path, &mut violations); }",
            "        assert!(violations.is_empty(), \"local entity-pose aliases:\\n{}\", violations.join(\"\\n\"));",
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
            "    use std::{collections::HashSet, fs, path::Path};",
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
            "",
            "    #[test]",
            "    fn generated_serializer_catalog_has_one_decoder_route_each() {",
            "        let decoder = include_str!(\"packets/entities.rs\");",
            "        for serializer in VANILLA_ENTITY_DATA_SERIALIZERS {",
            "            let identifier = format!(\"VANILLA_ENTITY_DATA_SERIALIZER_{}_ID\", serializer.constant_name);",
            "            assert_eq!(decoder.matches(&identifier).count(), 1, \"{identifier} must have exactly one decoder route\");",
            "        }",
            '        assert!(decoder.contains("fn decode_entity_data_value("), "entity-data decoder entry point is missing");',
            '        assert!(decoder.contains("unsupported entity data serializer {other}"), "unknown serializer ids must stay rejected");',
            "    }",
            "",
            "    #[test]",
            "    fn production_consumers_do_not_restore_serializer_id_aliases() {",
            "        fn visit(path: &Path, violations: &mut Vec<String>) {",
            "            if path.is_dir() {",
            "                for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); }",
            "                return;",
            "            }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let display = path.to_string_lossy();",
            "            if display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { return; }",
            "            let source = fs::read_to_string(path).unwrap();",
            "            let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
            "            for identifier in production.split(|character: char| !character.is_ascii_alphanumeric() && character != '_') {",
            "                if identifier.starts_with(\"VANILLA_ENTITY_DATA_\") && identifier.ends_with(\"_SERIALIZER_ID\") {",
            "                    violations.push(format!(\"{}: {identifier}\", path.display()));",
            "                }",
            "            }",
            "        }",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let mut violations = Vec::new();",
            "        for path in [crates.join(\"bbb-world/src\"), crates.join(\"bbb-native/src\"), crates.join(\"bbb-net/src\")] { visit(&path, &mut violations); }",
            "        assert!(violations.is_empty(), \"local entity-data serializer aliases:\\n{}\", violations.join(\"\\n\"));",
            "    }",
            "",
            '    fn contains_numeric_serializer_id_reference(source: &str) -> bool {',
            "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
            '        let numeric_construction = compact.match_indices("serializer_id:").any(|(offset, _)| {',
            '            compact[offset + "serializer_id:".len()..].chars().next().is_some_and(|character| character.is_ascii_digit())',
            "        });",
            '        numeric_construction || compact.match_indices(".serializer_id").any(|(offset, _)| {',
            '            let suffix = &compact[offset + ".serializer_id".len()..];',
            '            [",", "==", "!=", "="].iter().any(|separator| {',
            "                suffix.strip_prefix(separator).is_some_and(|value| value.chars().next().is_some_and(|character| character.is_ascii_digit()))",
            "            })",
            "        })",
            "    }",
            "",
            "    #[test]",
            "    fn numeric_serializer_id_reference_guard_covers_construction_and_comparison() {",
            '        assert!(contains_numeric_serializer_id_reference("serializer_id: 0,"));',
            '        assert!(contains_numeric_serializer_id_reference("assert_eq!(value.serializer_id,\\n 0);"));',
            '        assert!(contains_numeric_serializer_id_reference("value.serializer_id != 42"));',
            '        assert!(!contains_numeric_serializer_id_reference("serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,"));',
            '        assert!(!contains_numeric_serializer_id_reference("assert_eq!(value.serializer_id, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID);"));',
            "    }",
            "",
            "    #[test]",
            "    fn semantic_consumers_use_generated_serializer_ids() {",
            "        fn visit(path: &Path, violations: &mut Vec<String>) {",
            "            if path.is_dir() {",
            "                for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); }",
            "                return;",
            "            }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let source = fs::read_to_string(path).unwrap();",
            '            if contains_numeric_serializer_id_reference(&source) { violations.push(format!("{}: numeric .serializer_id reference", path.display())); }',
            "            for line in source.lines() {",
            "                if line.contains(\"SERIALIZER_ID: i32 =\") { violations.push(format!(\"{}: {line}\", path.display())); }",
            '                let numeric = line.trim().strip_prefix("serializer_id:")',
            '                    .and_then(|value| value.strip_suffix(","))',
            "                    .is_some_and(|value| value.trim().chars().all(|character| character.is_ascii_digit() || character == '_'));",
            "                if numeric { violations.push(format!(\"{}: {line}\", path.display())); }",
            "            }",
            "        }",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let mut violations = Vec::new();",
            "        for path in [crates.join(\"bbb-native/src\"), crates.join(\"bbb-world/src\"), crates.join(\"bbb-net/src\")] { visit(&path, &mut violations); }",
            '        assert!(violations.is_empty(), "semantic consumers restored serializer ids:\\n{}", violations.join("\\n"));',
            "    }",
            "",
            "    #[test]",
            "    fn numeric_serializer_ids_are_confined_to_protocol_wire_samples() {",
            "        fn visit(path: &Path, fields: &mut Vec<String>) {",
            "            if path.is_dir() {",
            "                for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), fields); }",
            "                return;",
            "            }",
            "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
            "            let source = fs::read_to_string(path).unwrap();",
            "            for (line_number, line) in source.lines().enumerate() {",
            '                let numeric = line.trim().strip_prefix("serializer_id:")',
            '                    .and_then(|value| value.strip_suffix(","))',
            "                    .is_some_and(|value| value.trim().chars().all(|character| character.is_ascii_digit() || character == '_'));",
            "                if numeric { fields.push(format!(\"{}:{}: {line}\", path.display(), line_number + 1)); }",
            "            }",
            "        }",
            "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
            "        let wire_sample = crates.join(\"bbb-protocol/src/packets/entities/tests.rs\");",
            "        let wire_source = fs::read_to_string(&wire_sample).unwrap();",
            '        assert!(wire_source.contains("WIRE_SAMPLE_SERIALIZER_IDS:"), "protocol wire serializer sample marker is missing");',
            "        let mut fields = Vec::new();",
            "        visit(crates, &mut fields);",
            '        assert_eq!(fields.len(), 18, "numeric serializer-id wire sample count changed; review and update the audited count:\\n{}", fields.join("\\n"));',
            '        assert!(fields.iter().all(|field| field.contains("/bbb-protocol/src/packets/entities/tests.rs:")), "numeric serializer ids escaped the audited protocol wire sample:\\n{}", fields.join("\\n"));',
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


def render_heightmap_types(
    entries: list[HeightmapTypeEntry], debug_colors: list[HeightmapDebugColorEntry]
) -> str:
    debug_colors_by_name = {
        entry.constant_name: entry.color_rgba for entry in debug_colors
    }
    if set(debug_colors_by_name) != {entry.constant_name for entry in entries}:
        raise GameplayIdExtractionError(
            "HeightMapRenderer colors do not exactly cover Heightmap.Types"
        )
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 Heightmap.Types ids/keys/usage and HeightMapRenderer.getColor.",
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
        "    pub debug_color_rgba: [u8; 4],",
        "}",
        "",
    ]
    for entry in entries:
        lines.append(f"pub const VANILLA_HEIGHTMAP_{entry.constant_name}_ID: i32 = {entry.type_id};")
    lines.extend(("", "pub const VANILLA_HEIGHTMAP_TYPES: &[HeightmapTypeCatalogEntry] = &["))
    for entry in entries:
        debug_color = debug_colors_by_name[entry.constant_name]
        lines.extend((
            "    HeightmapTypeCatalogEntry {",
            f"        id: {entry.type_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialization_key: {rust_string(entry.serialization_key)},",
            f"        usage: HeightmapUsage::{rust_pascal_case(entry.usage)},",
            f"        debug_color_rgba: {list(debug_color)},",
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
        "        assert_eq!(vanilla_heightmap_type(0).unwrap().debug_color_rgba, [255, 255, 0, 255]);",
        "        assert_eq!(vanilla_heightmap_type(1).unwrap().debug_color_rgba, [0, 178, 0, 255]);",
        "        assert_eq!(vanilla_heightmap_type(5).unwrap().debug_color_rgba, [0, 127, 127, 255]);",
        "    }", "",
        "    #[test]", "    fn production_sources_do_not_handwrite_heightmap_catalog_facts() {",
        "        let native = include_str!(\"../../bbb-native/src/runtime.rs\");",
        "        let projection = native.split(\"fn debug_heightmap_boxes(\").nth(1).and_then(|source| source.split(\"#[derive(Debug, Clone, Copy, PartialEq)]\").next()).expect(\"native heightmap debug projection\");",
        "        assert_eq!(projection.matches(\"vanilla_heightmap_type(kind_id)\").count(), 1);",
        "        assert_eq!(projection.matches(\"debug_color_rgba\").count(), 1);",
        "        let native_tests = include_str!(\"../../bbb-native/src/runtime/tests.rs\");",
        "        let fixture = native_tests.split(\"fn debug_heightmap_boxes_follow_custom_entry_and_generated_descriptor_order\").nth(1).and_then(|source| source.split(\"\\n#[test]\").next()).expect(\"native heightmap debug fixture\");",
        "        assert!(fixture.contains(\"VANILLA_HEIGHTMAP_TYPES\"));",
        "        assert!(fixture.contains(\"entry.debug_color_rgba\"));",
        "        assert!(!fixture.contains(\"color: [\"), \"native heightmap fixture restored hand-authored debug colors\");",
        "        fn starts_with_integer_literal(source: &str) -> bool {",
        "            source.strip_prefix('-').unwrap_or(source).chars().next().is_some_and(|character| character.is_ascii_digit())",
        "        }",
        "        fn contains_numeric_heightmap_kind_reference(source: &str) -> bool {",
        "            let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        "            let markers = [\"kind_id:\", \".kind_id,\", \".kind_id==\", \".kind_id!=\", \"sample_heightmap_first_available(\", \".first_available(\", \"heightmap_data(\", \"kind_id==\", \"constVANILLA_HEIGHTMAP_KIND_COUNT:usize=\"];",
        "            let old_kind_sets = [\"vec![1,3,4,5]\", \"vec![0,1,2,3,4,5]\", \"vec![0,1,3,4,5]\", \"forkind_idin[1,3,4,5]\", \"packet.chunk_data.heightmaps=(0..=5)\", \"forkind_idin0..=5\"];",
        "            markers.iter().any(|marker| compact.match_indices(marker).any(|(offset, _)| starts_with_integer_literal(&compact[offset + marker.len()..]))) || compact.contains(\"matches!(kind_id,\") || old_kind_sets.iter().any(|pattern| compact.contains(pattern))",
        "        }",
        "        assert!(contains_numeric_heightmap_kind_reference(\"kind_id: 1,\"));",
        "        assert!(contains_numeric_heightmap_kind_reference(\"assert_eq!(map.kind_id, 4);\"));",
        "        assert!(contains_numeric_heightmap_kind_reference(\"sampler.first_available(0, 2, 3)\"));",
        "        assert!(contains_numeric_heightmap_kind_reference(\"matches!(kind_id, 1 | 3 | 4 | 5)\"));",
        "        assert!(contains_numeric_heightmap_kind_reference(\"vec![0, 1, 2, 3, 4, 5]\"));",
        "        assert!(contains_numeric_heightmap_kind_reference(\"for kind_id in 0..=5\"));",
        "        assert!(!contains_numeric_heightmap_kind_reference(\"kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,\"));",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/heightmap_types.rs\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let semantic_consumer = [\"/bbb-native/src/\", \"/bbb-world/src/\", \"/bbb-net/src/\", \"/bbb-control/src/\"].iter().any(|directory| display.contains(directory));",
        "                if semantic_consumer && contains_numeric_heightmap_kind_reference(&source) { violations.push(format!(\"{display}: numeric heightmap kind reference\")); }",
        "                if display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                if source.contains(\"DEBUG_HEIGHTMAP_COLORS\") { violations.push(format!(\"{display}: handwritten debug colors\")); }",
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
        "        assert!(violations.is_empty(), \"handwritten heightmap catalog facts: {violations:?}\");",
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


def render_debug_subscription_types(
    entries: list[DebugSubscriptionEntry],
    game_events: list[DebugHolderTypeEntry],
    poi_types: list[DebugHolderTypeEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 DebugSubscriptions, GameEvent, and PoiTypes registration order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_DEBUG_SUBSCRIPTION_{entry.constant_name}_ID: i32 = {entry.subscription_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct DebugSubscriptionTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub has_value_codec: bool,",
        "    pub expire_after_ticks: i32,",
        "}",
        "",
        "pub const VANILLA_DEBUG_SUBSCRIPTION_TYPES: &[DebugSubscriptionTypeCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    DebugSubscriptionTypeCatalogEntry {",
            f"        id: {entry.subscription_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        has_value_codec: {str(entry.has_value_codec).lower()},",
            f"        expire_after_ticks: {entry.expire_after_ticks},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn vanilla_debug_subscription_type_by_id(",
        "    id: i32,",
        ") -> Option<&'static DebugSubscriptionTypeCatalogEntry> {",
        "    VANILLA_DEBUG_SUBSCRIPTION_TYPES.get(usize::try_from(id).ok()?)",
        "}",
        "",
        "pub fn vanilla_debug_subscription_type_by_resource_id(",
        "    resource_id: &str,",
        ") -> Option<&'static DebugSubscriptionTypeCatalogEntry> {",
        "    VANILLA_DEBUG_SUBSCRIPTION_TYPES",
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
        "    fn debug_subscription_catalog_preserves_vanilla_registration_order() {",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES.len(), 16);",
        "        assert_eq!(",
        "            VANILLA_DEBUG_SUBSCRIPTION_TYPES",
        "                .iter()",
        "                .map(|entry| (entry.constant_name, entry.resource_id))",
        "                .collect::<Vec<_>>(),",
        "            vec![",
    ))
    for entry in entries:
        lines.append(
            f"                ({rust_string(entry.constant_name)}, {rust_string(entry.resource_id)}),"
        )
    lines.extend((
        "            ]",
        "        );",
        "        let mut resource_ids = HashSet::new();",
        "        for (expected_id, entry) in VANILLA_DEBUG_SUBSCRIPTION_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.id, expected_id as i32);",
        "            assert!(resource_ids.insert(entry.resource_id));",
        "            assert_eq!(vanilla_debug_subscription_type_by_id(entry.id), Some(entry));",
        "            assert_eq!(vanilla_debug_subscription_type_by_resource_id(entry.resource_id), Some(entry));",
        "        }",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[0].constant_name, \"DEDICATED_SERVER_TICK_TIME\");",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[0].resource_id, \"minecraft:dedicated_server_tick_time\");",
        "        assert!(!VANILLA_DEBUG_SUBSCRIPTION_TYPES[0].has_value_codec);",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[0].expire_after_ticks, 0);",
        "        assert!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[6].has_value_codec);",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[6].expire_after_ticks, 100);",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[9].expire_after_ticks, 200);",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[14].expire_after_ticks, 200);",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[15].constant_name, \"GAME_EVENTS\");",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[15].resource_id, \"minecraft:game_events\");",
        "        assert_eq!(VANILLA_DEBUG_SUBSCRIPTION_TYPES[15].expire_after_ticks, 60);",
        "        assert_eq!(vanilla_debug_subscription_type_by_id(-1), None);",
        "        assert_eq!(vanilla_debug_subscription_type_by_id(16), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_debug_subscription_ids() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/debug_subscription_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID: i32 =\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel debug subscription id owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))

    lines.extend((
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct DebugHolderTypeCatalogEntry {",
        "    pub holder_id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
    ))
    for array_name, values in [
        ("VANILLA_DEBUG_GAME_EVENT_TYPES", game_events),
        ("VANILLA_DEBUG_POI_TYPES", poi_types),
    ]:
        lines.append(f"pub const {array_name}: &[DebugHolderTypeCatalogEntry] = &[")
        for entry in values:
            lines.extend((
                "    DebugHolderTypeCatalogEntry {",
                f"        holder_id: {entry.holder_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        resource_id: {rust_string(entry.resource_id)},",
                "    },",
            ))
        lines.append("];")
        lines.append("")
    lines.extend((
        "pub fn vanilla_debug_game_event_type_by_id(id: i32) -> Option<&'static DebugHolderTypeCatalogEntry> {",
        "    VANILLA_DEBUG_GAME_EVENT_TYPES.get(usize::try_from(id).ok()?)",
        "}",
        "",
        "pub fn vanilla_debug_poi_type_by_id(id: i32) -> Option<&'static DebugHolderTypeCatalogEntry> {",
        "    VANILLA_DEBUG_POI_TYPES.get(usize::try_from(id).ok()?)",
        "}",
        "",
        "#[cfg(test)]",
        "mod holder_tests {",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn debug_holder_catalogs_preserve_vanilla_registration_order() {",
        "        assert_eq!(VANILLA_DEBUG_GAME_EVENT_TYPES.len(), 60);",
        "        assert_eq!(VANILLA_DEBUG_POI_TYPES.len(), 21);",
        "        assert_eq!(vanilla_debug_game_event_type_by_id(0).unwrap().resource_id, \"minecraft:block_activate\");",
        "        assert_eq!(vanilla_debug_game_event_type_by_id(59).unwrap().resource_id, \"minecraft:resonate_15\");",
        "        assert_eq!(vanilla_debug_poi_type_by_id(19).unwrap().resource_id, \"minecraft:test_instance\");",
        "        assert_eq!(vanilla_debug_poi_type_by_id(20).unwrap().resource_id, \"minecraft:lightning_rod\");",
        "        assert!(vanilla_debug_game_event_type_by_id(60).is_none());",
        "        assert!(vanilla_debug_poi_type_by_id(21).is_none());",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_world_clock_types(entries: list[WorldClockEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 WorldClocks bootstrap registration order and resource ids.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_WORLD_CLOCK_{entry.constant_name}_ID: i32 = {entry.clock_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct WorldClockTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
        "pub const VANILLA_WORLD_CLOCK_TYPES: &[WorldClockTypeCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    WorldClockTypeCatalogEntry {",
            f"        id: {entry.clock_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn vanilla_world_clock_type_by_id(",
        "    id: i32,",
        ") -> Option<&'static WorldClockTypeCatalogEntry> {",
        "    VANILLA_WORLD_CLOCK_TYPES.get(usize::try_from(id).ok()?)",
        "}",
        "",
        "pub fn vanilla_world_clock_type_by_resource_id(",
        "    resource_id: &str,",
        ") -> Option<&'static WorldClockTypeCatalogEntry> {",
        "    VANILLA_WORLD_CLOCK_TYPES",
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
        "    fn world_clock_catalog_preserves_vanilla_bootstrap_order() {",
        "        assert_eq!(VANILLA_WORLD_CLOCK_TYPES.len(), 2);",
        "        assert_eq!(",
        "            VANILLA_WORLD_CLOCK_TYPES",
        "                .iter()",
        "                .map(|entry| (entry.id, entry.constant_name, entry.resource_id))",
        "                .collect::<Vec<_>>(),",
        "            vec![",
    ))
    for entry in entries:
        lines.append(
            f"                ({entry.clock_id}, {rust_string(entry.constant_name)}, {rust_string(entry.resource_id)}),"
        )
    lines.extend((
        "            ]",
        "        );",
        "        let mut resource_ids = HashSet::new();",
        "        for entry in VANILLA_WORLD_CLOCK_TYPES {",
        "            assert!(resource_ids.insert(entry.resource_id));",
        "            assert_eq!(vanilla_world_clock_type_by_id(entry.id), Some(entry));",
        "            assert_eq!(vanilla_world_clock_type_by_resource_id(entry.resource_id), Some(entry));",
        "        }",
        "        assert_eq!(vanilla_world_clock_type_by_id(-1), None);",
        "        assert_eq!(vanilla_world_clock_type_by_id(2), None);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_world_clock_holder_ids() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/world_clock_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"VANILLA_WORLD_CLOCK_THE_END_ID: i32 =\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel world clock holder id owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_game_rules(entries: list[GameRuleEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 GameRules.java built-in registrations and value codecs.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum GameRuleValueKind {",
        "    Boolean,",
        "    Integer { min: i32, max: i32 },",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct GameRuleCatalogEntry {",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub value_kind: GameRuleValueKind,",
        "}",
        "",
        "pub const VANILLA_GAME_RULES: &[GameRuleCatalogEntry] = &[",
    ]
    for entry in entries:
        value_kind = (
            "GameRuleValueKind::Boolean"
            if entry.value_kind == "boolean"
            else "GameRuleValueKind::Integer { "
            f"min: {entry.min_value}, max: {entry.max_value} }}"
        )
        lines.extend((
            "    GameRuleCatalogEntry {",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        value_kind: {value_kind},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn vanilla_game_rule(resource_id: &str) -> Option<&'static GameRuleCatalogEntry> {",
        "    VANILLA_GAME_RULES",
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
        "    fn game_rule_catalog_is_complete_unique_and_typed() {",
        f"        assert_eq!(VANILLA_GAME_RULES.len(), {len(entries)});",
        "        let mut resource_ids = HashSet::new();",
        "        for entry in VANILLA_GAME_RULES {",
        "            assert!(resource_ids.insert(entry.resource_id));",
        "            assert_eq!(vanilla_game_rule(entry.resource_id), Some(entry));",
        "        }",
        "        assert_eq!(vanilla_game_rule(\"minecraft:unknown\"), None);",
        "        assert_eq!(",
        "            vanilla_game_rule(\"minecraft:keep_inventory\").unwrap().value_kind,",
        "            GameRuleValueKind::Boolean",
        "        );",
        "        assert_eq!(",
        "            vanilla_game_rule(\"minecraft:fire_spread_radius_around_player\").unwrap().value_kind,",
        "            GameRuleValueKind::Integer { min: -1, max: i32::MAX },",
        "        );",
        "        assert_eq!(",
        "            vanilla_game_rule(\"minecraft:max_snow_accumulation_height\").unwrap().value_kind,",
        "            GameRuleValueKind::Integer { min: 0, max: 8 },",
        "        );",
        "        assert_eq!(",
        "            vanilla_game_rule(\"minecraft:max_minecart_speed\").unwrap().value_kind,",
        "            GameRuleValueKind::Integer { min: 1, max: 1000 },",
        "        );",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_the_game_rule_catalog() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/game_rules.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"const VANILLA_GAME_RULES:\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel game-rule catalog owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_stat_types(
    types: list[StatTypeEntry],
    custom: list[CustomStatEntry],
    objective_criteria: list[str],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Sources: vanilla 26.1 Stats.java and ObjectiveCriteria.java.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct StatTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub value_registry: &'static str,",
        "}",
        "",
    ]
    for entry in types:
        lines.append(
            f"pub const VANILLA_STAT_TYPE_{entry.constant_name}_ID: i32 = {entry.type_id};"
        )
    lines.extend(("", "pub const VANILLA_STAT_TYPES: &[StatTypeCatalogEntry] = &["))
    for entry in types:
        lines.extend((
            "    StatTypeCatalogEntry {",
            f"        id: {entry.type_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        value_registry: {rust_string(entry.value_registry)},",
            "    },",
        ))
    lines.extend((
        "];", "",
        "pub const VANILLA_OBJECTIVE_CRITERIA_NAMES: &[&str] = &[",
    ))
    for name in objective_criteria:
        lines.append(f"    {rust_string(name)},")
    lines.extend((
        "];", "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct CustomStatCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub formatter: &'static str,",
        "}", "",
        "pub const VANILLA_CUSTOM_STATS: &[CustomStatCatalogEntry] = &[",
    ))
    for entry in custom:
        lines.extend((
            "    CustomStatCatalogEntry {",
            f"        id: {entry.value_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        formatter: {rust_string(entry.formatter)},",
            "    },",
        ))
    lines.extend((
        "];", "",
        "pub fn vanilla_stat_type_by_id(id: i32) -> Option<&'static StatTypeCatalogEntry> {",
        "    VANILLA_STAT_TYPES.get(usize::try_from(id).ok()?)",
        "}", "",
        "pub fn vanilla_stat_type_by_resource_id(resource_id: &str) -> Option<&'static StatTypeCatalogEntry> {",
        "    VANILLA_STAT_TYPES.iter().find(|entry| entry.resource_id == resource_id)",
        "}", "",
        "pub fn vanilla_custom_stat_by_id(id: i32) -> Option<&'static CustomStatCatalogEntry> {",
        "    VANILLA_CUSTOM_STATS.get(usize::try_from(id).ok()?)",
        "}", "",
        "pub fn vanilla_custom_stat_by_resource_id(resource_id: &str) -> Option<&'static CustomStatCatalogEntry> {",
        "    VANILLA_CUSTOM_STATS.iter().find(|entry| entry.resource_id == resource_id)",
        "}", "",
        "#[cfg(test)]", "mod tests {", "    use std::{collections::HashSet, fs, path::Path};", "", "    use super::*;", "",
        "    #[test]", "    fn stat_catalogs_preserve_vanilla_declaration_order() {",
        "        assert_eq!(VANILLA_STAT_TYPES.len(), 9);",
        "        assert_eq!(VANILLA_CUSTOM_STATS.len(), 77);",
        "        assert_eq!(VANILLA_OBJECTIVE_CRITERIA_NAMES.len(), 43);",
        "        let mut type_resources = HashSet::new();",
        "        for (id, entry) in VANILLA_STAT_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert!(type_resources.insert(entry.resource_id));",
        "            assert_eq!(vanilla_stat_type_by_id(entry.id), Some(entry));",
        "        }",
        "        let mut custom_resources = HashSet::new();",
        "        for (id, entry) in VANILLA_CUSTOM_STATS.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert!(custom_resources.insert(entry.resource_id));",
        "            assert_eq!(vanilla_custom_stat_by_id(entry.id), Some(entry));",
        "        }",
        "        assert_eq!(VANILLA_STAT_TYPES[0].resource_id, \"minecraft:mined\");",
        "        assert_eq!(VANILLA_STAT_TYPES[8].value_registry, \"minecraft:custom_stat\");",
        "        assert_eq!(VANILLA_STAT_TYPE_BLOCK_MINED_ID, 0);",
        "        assert_eq!(VANILLA_STAT_TYPE_ITEM_CRAFTED_ID, 1);",
        "        assert_eq!(VANILLA_STAT_TYPE_ENTITY_KILLED_ID, 6);",
        "        assert_eq!(VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID, 7);",
        "        assert_eq!(VANILLA_STAT_TYPE_CUSTOM_ID, 8);",
        "        assert_eq!(VANILLA_CUSTOM_STATS[0].resource_id, \"minecraft:leave_game\");",
        "        assert_eq!(VANILLA_CUSTOM_STATS[76].resource_id, \"minecraft:interact_with_smithing_table\");",
        "        assert!(VANILLA_OBJECTIVE_CRITERIA_NAMES.contains(&\"dummy\"));",
        "        assert!(VANILLA_OBJECTIVE_CRITERIA_NAMES.contains(&\"teamkill.dark_blue\"));",
        "        assert!(VANILLA_OBJECTIVE_CRITERIA_NAMES.contains(&\"killedByTeam.white\"));",
        "    }", "",
        "    fn starts_with_integer_literal(source: &str) -> bool {",
        "        source.trim_start().strip_prefix('-').unwrap_or(source.trim_start()).chars().next().is_some_and(|character| character.is_ascii_digit())",
        "    }", "",
        "    fn contains_numeric_stat_type_id_reference(source: &str) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        "        let numeric_construction = compact.match_indices(\"stat_type_id:\").any(|(offset, _)| starts_with_integer_literal(&compact[offset + \"stat_type_id:\".len()..]));",
        "        let numeric_field_comparison = compact.match_indices(\".stat_type_id\").any(|(offset, _)| {",
        "            let suffix = &compact[offset + \".stat_type_id\".len()..];",
        "            [\",\", \"==\", \"!=\", \"=\"].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))",
        "        });",
        "        let numeric_json_comparison = compact.match_indices(\"[\\\"stat_type_id\\\"]\").any(|(offset, _)| {",
        "            let suffix = &compact[offset + \"[\\\"stat_type_id\\\"]\".len()..];",
        "            [\",\", \"==\", \"!=\", \"=\"].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))",
        "        });",
        "        let numeric_query = [\"stat_value(\", \"stat_identity(\"].iter().any(|marker| compact.match_indices(marker).any(|(offset, _)| starts_with_integer_literal(&compact[offset + marker.len()..])));",
        "        let numeric_alias = source.lines().any(|line| {",
        "            let compact: String = line.chars().filter(|character| !character.is_whitespace()).collect();",
        "            (compact.starts_with(\"let\") || compact.starts_with(\"const\")) && compact.contains(\"stat_type\") && compact.rsplit_once('=').is_some_and(|(_, value)| starts_with_integer_literal(value))",
        "        });",
        "        numeric_construction || numeric_field_comparison || numeric_json_comparison || numeric_query || numeric_alias",
        "    }", "",
        "    #[test]", "    fn numeric_stat_type_guard_covers_semantic_consumers() {",
        "        assert!(contains_numeric_stat_type_id_reference(\"stat_type_id: 8,\"));",
        "        assert!(contains_numeric_stat_type_id_reference(\"assert_eq!(entry.stat_type_id, 6);\"));",
        "        assert!(contains_numeric_stat_type_id_reference(\"assert_eq!(entry[\\\"stat_type_id\\\"], 0);\"));",
        "        assert!(contains_numeric_stat_type_id_reference(\"world.stat_value(8, 10)\"));",
        "        assert!(contains_numeric_stat_type_id_reference(\"let custom_stat_type_id = 8;\"));",
        "        assert!(!contains_numeric_stat_type_id_reference(\"stat_type_id: VANILLA_STAT_TYPE_CUSTOM_ID,\"));",
        "        assert!(!contains_numeric_stat_type_id_reference(\"world.stat_identity(unknown_stat_type_id, 10)\"));",
        "    }", "",
        "    #[test]", "    fn semantic_sources_do_not_restore_numeric_stat_type_ids() {",
        "        fn visit(path: &Path, violations: &mut Vec<String>) {",
        "            if path.is_dir() { for entry in fs::read_dir(path).unwrap() { visit(&entry.unwrap().path(), violations); } return; }",
        "            if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { return; }",
        "            let source = fs::read_to_string(path).unwrap();",
        "            if contains_numeric_stat_type_id_reference(&source) { violations.push(format!(\"{}: numeric StatType id reference\", path.display())); }",
        "        }",
        "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
        "        let mut violations = Vec::new();",
        "        for path in [crates.join(\"bbb-world/src\"), crates.join(\"bbb-net/src\"), crates.join(\"bbb-control/src\"), crates.join(\"bbb-native/src\")] { visit(&path, &mut violations); }",
        "        assert!(violations.is_empty(), \"semantic consumers restored numeric StatType ids:\\n{}\", violations.join(\"\\n\"));",
        "    }", "}", "",
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


def render_recipe_wire_types(
    books: list[RecipeBookTypeEntry],
    displays: list[RecipeDisplayTypeEntry],
    categories: list[RecipeBookCategoryEntry],
    search_categories: list[RecipeBookSearchCategoryEntry],
    slot_displays: list[SlotDisplayTypeEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 RecipeBookType declaration, RecipeDisplays bootstrap order,",
        "// RecipeBookCategories registration order, and SearchRecipeBookCategory membership.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
    ]
    for entry in books:
        lines.append(
            f"pub const VANILLA_RECIPE_BOOK_TYPE_{entry.constant_name}_ORDINAL: i32 = {entry.ordinal};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "#[serde(rename_all = \"snake_case\")]",
        "pub enum RecipeBookType {",
    ))
    for entry in books:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_RECIPE_BOOK_TYPE_{entry.constant_name}_ORDINAL,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct RecipeBookTypeCatalogEntry {",
        "    pub ordinal: i32,",
        "    pub constant_name: &'static str,",
        "    pub book_type: RecipeBookType,",
        "}",
        "",
        "pub const VANILLA_RECIPE_BOOK_TYPES: &[RecipeBookTypeCatalogEntry] = &[",
    ))
    for entry in books:
        lines.extend((
            "    RecipeBookTypeCatalogEntry {",
            f"        ordinal: {entry.ordinal},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        book_type: RecipeBookType::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl RecipeBookType {",
        "    pub const fn id(self) -> i32 { self as i32 }",
        "",
        "    pub fn from_ordinal(ordinal: i32) -> Option<Self> {",
        "        VANILLA_RECIPE_BOOK_TYPES",
        "            .get(usize::try_from(ordinal).ok()?)",
        "            .map(|entry| entry.book_type)",
        "    }",
        "}",
        "",
    ))
    for entry in categories:
        lines.append(
            f"pub const VANILLA_RECIPE_BOOK_CATEGORY_{entry.constant_name}_ID: i32 = {entry.category_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct RecipeBookCategoryCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
        "pub const VANILLA_RECIPE_BOOK_CATEGORIES: &[RecipeBookCategoryCatalogEntry] = &[",
    ))
    for entry in categories:
        lines.extend((
            "    RecipeBookCategoryCatalogEntry {",
            f"        id: {entry.category_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn vanilla_recipe_book_category(",
        "    category_id: i32,",
        ") -> Option<&'static RecipeBookCategoryCatalogEntry> {",
        "    VANILLA_RECIPE_BOOK_CATEGORIES.get(usize::try_from(category_id).ok()?)",
        "}",
        "",
    ))
    lines.extend((
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct RecipeBookSearchCategoryCatalogEntry {",
        "    pub constant_name: &'static str,",
        "    pub book_type: RecipeBookType,",
        "    pub included_category_ids: &'static [i32],",
        "}",
        "",
        "pub const VANILLA_RECIPE_BOOK_SEARCH_CATEGORIES: &[RecipeBookSearchCategoryCatalogEntry] = &[",
    ))
    for entry in search_categories:
        included = ", ".join(
            f"VANILLA_RECIPE_BOOK_CATEGORY_{constant}_ID"
            for constant in entry.included_category_constants
        )
        lines.extend((
            "    RecipeBookSearchCategoryCatalogEntry {",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        book_type: RecipeBookType::{rust_pascal_case(entry.constant_name)},",
            f"        included_category_ids: &[{included}],",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn vanilla_recipe_book_search_category_ids(",
        "    book_type: RecipeBookType,",
        ") -> &'static [i32] {",
        "    VANILLA_RECIPE_BOOK_SEARCH_CATEGORIES[book_type.id() as usize].included_category_ids",
        "}",
        "",
    ))
    for entry in displays:
        lines.append(
            f"pub const VANILLA_RECIPE_DISPLAY_TYPE_{entry.constant_name}_ID: i32 = {entry.type_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "pub enum RecipeDisplayType {",
    ))
    for entry in displays:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_RECIPE_DISPLAY_TYPE_{entry.constant_name}_ID,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct RecipeDisplayTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub display_type: RecipeDisplayType,",
        "}",
        "",
        "pub const VANILLA_RECIPE_DISPLAY_TYPES: &[RecipeDisplayTypeCatalogEntry] = &[",
    ))
    for entry in displays:
        lines.extend((
            "    RecipeDisplayTypeCatalogEntry {",
            f"        id: {entry.type_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        display_type: RecipeDisplayType::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl RecipeDisplayType {",
        "    pub const fn id(self) -> i32 { self as i32 }",
        "",
        "    pub fn as_str(self) -> &'static str {",
        "        VANILLA_RECIPE_DISPLAY_TYPES[self.id() as usize]",
        "            .resource_id",
        "            .strip_prefix(\"minecraft:\")",
        "            .unwrap()",
        "    }",
        "",
        "    pub fn from_id(id: i32) -> Option<Self> {",
        "        VANILLA_RECIPE_DISPLAY_TYPES",
        "            .get(usize::try_from(id).ok()?)",
        "            .map(|entry| entry.display_type)",
        "    }",
        "}",
        "",
    ))
    for entry in slot_displays:
        lines.append(
            f"pub const VANILLA_SLOT_DISPLAY_TYPE_{entry.constant_name}_ID: i32 = {entry.type_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct SlotDisplayTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "}",
        "",
        "pub const VANILLA_SLOT_DISPLAY_TYPES: &[SlotDisplayTypeCatalogEntry] = &[",
    ))
    for entry in slot_displays:
        lines.extend((
            "    SlotDisplayTypeCatalogEntry {",
            f"        id: {entry.type_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn vanilla_slot_display_type(id: i32) -> Option<&'static SlotDisplayTypeCatalogEntry> {",
        "    VANILLA_SLOT_DISPLAY_TYPES.get(usize::try_from(id).ok()?)",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn recipe_wire_catalogs_are_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_RECIPE_BOOK_TYPES.len(), 4);",
        "        assert_eq!(VANILLA_RECIPE_BOOK_CATEGORIES.len(), 13);",
        "        assert_eq!(VANILLA_RECIPE_BOOK_SEARCH_CATEGORIES.len(), 4);",
        "        assert_eq!(VANILLA_RECIPE_DISPLAY_TYPES.len(), 5);",
        "        assert_eq!(VANILLA_SLOT_DISPLAY_TYPES.len(), 11);",
        "        let mut constants = HashSet::new();",
        "        for (ordinal, entry) in VANILLA_RECIPE_BOOK_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.ordinal, ordinal as i32);",
        "            assert_eq!(entry.book_type.id(), entry.ordinal);",
        "            assert!(constants.insert(entry.constant_name));",
        "        }",
        "        constants.clear();",
        "        let mut resources = HashSet::new();",
        "        for (id, entry) in VANILLA_RECIPE_BOOK_CATEGORIES.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert!(constants.insert(entry.constant_name));",
        "            assert!(resources.insert(entry.resource_id));",
        "        }",
        "        constants.clear();",
        "        resources.clear();",
        "        for (id, entry) in VANILLA_RECIPE_DISPLAY_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert_eq!(entry.display_type.id(), entry.id);",
        "            assert!(constants.insert(entry.constant_name));",
        "            assert!(resources.insert(entry.resource_id));",
        "        }",
        "        constants.clear();",
        "        resources.clear();",
        "        for (id, entry) in VANILLA_SLOT_DISPLAY_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert_eq!(vanilla_slot_display_type(entry.id), Some(entry));",
        "            assert!(constants.insert(entry.constant_name));",
        "            assert!(resources.insert(entry.resource_id));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn recipe_wire_catalogs_preserve_enum_and_bootstrap_order() {",
        "        assert_eq!(RecipeBookType::Crafting.id(), 0);",
        "        assert_eq!(RecipeBookType::Smoker.id(), 3);",
        "        assert_eq!(RecipeBookType::from_ordinal(4), None);",
        "        assert_eq!(VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID, 0);",
        "        assert_eq!(VANILLA_RECIPE_BOOK_CATEGORY_SMOKER_FOOD_ID, 9);",
        "        assert_eq!(vanilla_recipe_book_category(12).unwrap().resource_id, \"minecraft:campfire\");",
        "        assert_eq!(vanilla_recipe_book_category(13), None);",
        "        assert_eq!(RecipeDisplayType::CraftingShapeless.id(), 0);",
        "        assert_eq!(RecipeDisplayType::Smithing.id(), 4);",
        "        assert_eq!(RecipeDisplayType::Stonecutter.as_str(), \"stonecutter\");",
        "        assert_eq!(RecipeDisplayType::from_id(2), Some(RecipeDisplayType::Furnace));",
        "        assert_eq!(RecipeDisplayType::from_id(5), None);",
        "        assert_eq!(VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID, 0);",
        "        assert_eq!(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID, 4);",
        "        assert_eq!(VANILLA_SLOT_DISPLAY_TYPE_WITH_REMAINDER_ID, 9);",
        "        assert_eq!(VANILLA_SLOT_DISPLAY_TYPE_COMPOSITE_ID, 10);",
        "        assert_eq!(vanilla_slot_display_type(11), None);",
        "    }",
        "",
        "    #[test]",
        "    fn recipe_book_search_categories_preserve_vanilla_membership_and_order() {",
        "        for (ordinal, entry) in VANILLA_RECIPE_BOOK_SEARCH_CATEGORIES.iter().enumerate() {",
        "            assert_eq!(entry.book_type.id(), ordinal as i32);",
        "            assert_eq!(entry.constant_name, VANILLA_RECIPE_BOOK_TYPES[ordinal].constant_name);",
        "        }",
        "        assert_eq!(vanilla_recipe_book_search_category_ids(RecipeBookType::Crafting), &[VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID, VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID, VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID, VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_REDSTONE_ID]);",
        "        assert_eq!(vanilla_recipe_book_search_category_ids(RecipeBookType::Furnace), &[VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_FOOD_ID, VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_BLOCKS_ID, VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_MISC_ID]);",
        "        assert_eq!(vanilla_recipe_book_search_category_ids(RecipeBookType::BlastFurnace), &[VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_BLOCKS_ID, VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_MISC_ID]);",
        "        assert_eq!(vanilla_recipe_book_search_category_ids(RecipeBookType::Smoker), &[VANILLA_RECIPE_BOOK_CATEGORY_SMOKER_FOOD_ID]);",
        "    }",
        "",
        "    fn starts_with_integer_literal(source: &str) -> bool {",
        "        source.trim_start().strip_prefix('-').unwrap_or(source.trim_start()).chars().next().is_some_and(|character| character.is_ascii_digit())",
        "    }",
        "",
        "    fn contains_numeric_recipe_book_category_reference(source: &str) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        '        let numeric_construction = compact.match_indices("category_id:").any(|(offset, _)| {',
        '            starts_with_integer_literal(&compact[offset + "category_id:".len()..])',
        "        });",
        '        let numeric_field_comparison = compact.match_indices(".category_id").any(|(offset, _)| {',
        '            let suffix = &compact[offset + ".category_id".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
        "        });",
        '        let numeric_json_comparison = compact.match_indices("[\\\"category_id\\\"]").any(|(offset, _)| {',
        '            let suffix = &compact[offset + "[\\\"category_id\\\"]".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
        "        });",
        "        numeric_construction || numeric_field_comparison || numeric_json_comparison",
        "    }",
        "",
        "    fn contains_numeric_recipe_display_type_reference(source: &str) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        '        let numeric_construction = compact.match_indices("recipe_display_type_id:").any(|(offset, _)| {',
        '            starts_with_integer_literal(&compact[offset + "recipe_display_type_id:".len()..])',
        "        });",
        '        let numeric_field_comparison = compact.match_indices(".recipe_display_type_id").any(|(offset, _)| {',
        '            let suffix = &compact[offset + ".recipe_display_type_id".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
        "        });",
        "        numeric_construction || numeric_field_comparison",
        "    }",
        "",
        "    fn contains_numeric_slot_display_type_reference(source: &str) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        '        let numeric_construction = compact.match_indices("display_type_id:").any(|(offset, _)| {',
        '            let boundary = offset == 0 || !compact.as_bytes()[offset - 1].is_ascii_alphanumeric() && compact.as_bytes()[offset - 1] != b\'_\';',
        '            boundary && starts_with_integer_literal(&compact[offset + "display_type_id:".len()..])',
        "        });",
        '        let numeric_field_comparison = compact.match_indices(".display_type_id").any(|(offset, _)| {',
        '            let suffix = &compact[offset + ".display_type_id".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
        "        });",
        '        let numeric_json_comparison = compact.match_indices("[\\\"display_type_id\\\"]").any(|(offset, _)| {',
        '            let suffix = &compact[offset + "[\\\"display_type_id\\\"]".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_integer_literal))',
        "        });",
        "        let numeric_match_arm = compact.match_indices(\"matchdisplay_type_id{\").any(|(offset, _)| {",
        "            let body = &compact[offset + \"matchdisplay_type_id{\".len()..];",
        "            let mut depth = 0usize;",
        "            for (index, character) in body.char_indices() {",
        "                match character {",
        "                    '{' => depth += 1,",
        "                    '}' if depth == 0 => return false,",
        "                    '}' => depth -= 1,",
        "                    character if depth == 0 && character.is_ascii_digit() => {",
        "                        let suffix = &body[index..];",
        "                        let digit_count = suffix.bytes().take_while(u8::is_ascii_digit).count();",
        "                        let after = &suffix[digit_count..];",
        "                        if after.starts_with(\"=>\") || after.starts_with('|') { return true; }",
        "                    }",
        "                    _ => {}",
        "                }",
        "            }",
        "            false",
        "        });",
        "        numeric_construction || numeric_field_comparison || numeric_json_comparison || numeric_match_arm",
        "    }",
        "",
        "    #[test]",
        "    fn numeric_recipe_book_category_guard_covers_semantic_fixtures() {",
        '        assert!(contains_numeric_recipe_book_category_reference("category_id: 10,"));',
        '        assert!(contains_numeric_recipe_book_category_reference("assert_eq!(entry.category_id, 2);"));',
        '        assert!(contains_numeric_recipe_book_category_reference("assert_eq!(entry[\\\"category_id\\\"], 3);"));',
        '        assert!(!contains_numeric_recipe_book_category_reference("category_id: VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,"));',
        '        assert!(!contains_numeric_recipe_book_category_reference("category_id: id + 10,"));',
        "    }",
        "",
        "    #[test]",
        "    fn numeric_recipe_display_type_guard_covers_semantic_fixtures() {",
        '        assert!(contains_numeric_recipe_display_type_reference("recipe_display_type_id: 3,"));',
        '        assert!(contains_numeric_recipe_display_type_reference("assert_eq!(entry.recipe_display_type_id, 3);"));',
        '        assert!(!contains_numeric_recipe_display_type_reference("recipe_display_type_id: VANILLA_RECIPE_DISPLAY_TYPE_STONECUTTER_ID,"));',
        "    }",
        "",
        "    #[test]",
        "    fn numeric_slot_display_type_guard_covers_semantic_consumers() {",
        '        assert!(contains_numeric_slot_display_type_reference("display_type_id: 4,"));',
        '        assert!(contains_numeric_slot_display_type_reference("assert_eq!(entry.display_type_id, 5);"));',
        '        assert!(contains_numeric_slot_display_type_reference("assert_eq!(entry[\\\"display_type_id\\\"], 6);"));',
        '        assert!(contains_numeric_slot_display_type_reference("match display_type_id { 9 => child(), 10 => list(), _ => none() }"));',
        '        assert!(!contains_numeric_slot_display_type_reference("display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,"));',
        '        assert!(!contains_numeric_slot_display_type_reference("recipe_display_type_id: 3,"));',
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_recipe_wire_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/recipe_wire_types.rs\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        '                if contains_numeric_recipe_book_category_reference(&source) { violations.push(format!("{}: numeric recipe-book category reference", path.display())); }',
        '                if contains_numeric_recipe_display_type_reference(&source) { violations.push(format!("{}: numeric recipe-display type reference", path.display())); }',
        '                if contains_numeric_slot_display_type_reference(&source) { violations.push(format!("{}: numeric slot-display type reference", path.display())); }',
        "                if display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                let old_category_tables = [",
        "                    \"CRAFTING_BUILDING_BLOCKS\", \"CRAFTING_REDSTONE\",",
        "                    \"CRAFTING_EQUIPMENT\", \"CRAFTING_MISC\", \"FURNACE_FOOD\",",
        "                    \"FURNACE_BLOCKS\", \"FURNACE_MISC\", \"BLAST_FURNACE_BLOCKS\",",
        "                    \"BLAST_FURNACE_MISC\", \"SMOKER_FOOD\",",
        "                ];",
        "                let redeclares_category = old_category_tables.iter().any(|name| {",
        "                    production.contains(&format!(\"const {name}_CATEGORY_ID\"))",
        "                });",
        "                if production.contains(\"enum RecipeBookType {\") || production.contains(\"enum RecipeDisplayType {\") || production.contains(\"Self::Crafting => 0\") || production.contains(\"Self::CraftingShapeless => 0\") || production.contains(\"0 => Self::CraftingShapeless\") || production.contains(\"_TAB_CATEGORIES\") || redeclares_category { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel recipe wire fact owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_item_component_value_types(
    item_uses: list[NamedComponentValueEntry],
    swings: list[NamedComponentValueEntry],
    rarities: list[NamedComponentValueEntry],
    map_post_processing: list[ComponentValueEntry],
    firework_shapes: list[NamedComponentValueEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 item component value enum ids, names, and ZERO strategies.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use serde::{Deserialize, Serialize};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ItemComponentValueIdOutOfBoundsStrategy { Zero }",
        "",
        "pub const VANILLA_ITEM_COMPONENT_VALUE_OUT_OF_BOUNDS_STRATEGY: ItemComponentValueIdOutOfBoundsStrategy = ItemComponentValueIdOutOfBoundsStrategy::Zero;",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ItemComponentValueCatalogEntry<T> {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: Option<&'static str>,",
        "    pub value: T,",
        "}",
        "",
    ]

    def add_named(
        entries: list[NamedComponentValueEntry],
        rust_type: str,
        constant_prefix: str,
        catalog_name: str,
    ) -> None:
        for entry in entries:
            lines.append(
                f"pub const {constant_prefix}_{entry.constant_name}_ID: i32 = {entry.value_id};"
            )
        lines.extend((
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(i32)]",
            "#[serde(rename_all = \"snake_case\")]",
            f"pub enum {rust_type} {{",
        ))
        for entry in entries:
            lines.append(
                f"    {rust_pascal_case(entry.constant_name)} = {constant_prefix}_{entry.constant_name}_ID,"
            )
        lines.extend((
            "}",
            "",
            f"pub const {catalog_name}: &[ItemComponentValueCatalogEntry<{rust_type}>] = &[",
        ))
        for entry in entries:
            lines.extend((
                "    ItemComponentValueCatalogEntry {",
                f"        id: {entry.value_id},",
                f"        constant_name: {rust_string(entry.constant_name)},",
                f"        serialized_name: Some({rust_string(entry.serialized_name)}),",
                f"        value: {rust_type}::{rust_pascal_case(entry.constant_name)},",
                "    },",
            ))
        zero_variant = rust_pascal_case(entries[0].constant_name)
        lines.extend((
            "];",
            "",
            f"impl {rust_type} {{",
            "    pub const fn id(self) -> i32 { self as i32 }",
            "",
            "    pub fn from_id(id: i32) -> Self {",
            f"        {catalog_name}",
            "            .get(usize::try_from(id).unwrap_or(usize::MAX))",
            f"            .map_or(Self::{zero_variant}, |entry| entry.value)",
            "    }",
            "",
            "    pub fn as_str(self) -> &'static str {",
            f"        {catalog_name}[self.id() as usize].serialized_name.unwrap()",
            "    }",
            "",
            "    pub fn from_serialized_name(name: &str) -> Option<Self> {",
            f"        {catalog_name}",
            "            .iter()",
            "            .find(|entry| entry.serialized_name == Some(name))",
            "            .map(|entry| entry.value)",
            "    }",
            "}",
            "",
        ))

    add_named(
        item_uses,
        "ItemUseAnimationSummary",
        "VANILLA_ITEM_USE_ANIMATION",
        "VANILLA_ITEM_USE_ANIMATIONS",
    )
    add_named(
        swings,
        "SwingAnimationTypeSummary",
        "VANILLA_SWING_ANIMATION_TYPE",
        "VANILLA_SWING_ANIMATION_TYPES",
    )
    add_named(
        rarities,
        "ItemRaritySummary",
        "VANILLA_ITEM_RARITY",
        "VANILLA_ITEM_RARITIES",
    )

    for entry in map_post_processing:
        lines.append(
            f"pub const VANILLA_MAP_POST_PROCESSING_{entry.constant_name}_ID: i32 = {entry.value_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(i32)]",
        "#[serde(rename_all = \"snake_case\")]",
        "pub enum MapPostProcessingSummary {",
    ))
    for entry in map_post_processing:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_MAP_POST_PROCESSING_{entry.constant_name}_ID,"
        )
    lines.extend((
        "}",
        "",
        "pub const VANILLA_MAP_POST_PROCESSING_VALUES: &[ItemComponentValueCatalogEntry<MapPostProcessingSummary>] = &[",
    ))
    for entry in map_post_processing:
        lines.extend((
            "    ItemComponentValueCatalogEntry {",
            f"        id: {entry.value_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            "        serialized_name: None,",
            f"        value: MapPostProcessingSummary::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl MapPostProcessingSummary {",
        "    pub const fn id(self) -> i32 { self as i32 }",
        "",
        "    pub fn from_id(id: i32) -> Self {",
        "        VANILLA_MAP_POST_PROCESSING_VALUES",
        "            .get(usize::try_from(id).unwrap_or(usize::MAX))",
        "            .map_or(Self::Lock, |entry| entry.value)",
        "    }",
        "}",
        "",
    ))

    add_named(
        firework_shapes,
        "FireworkExplosionShapeSummary",
        "VANILLA_FIREWORK_EXPLOSION_SHAPE",
        "VANILLA_FIREWORK_EXPLOSION_SHAPES",
    )

    lines.extend((
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    fn assert_catalog<T: Copy>(catalog: &[ItemComponentValueCatalogEntry<T>]) {",
        "        let mut constants = HashSet::new();",
        "        let mut names = HashSet::new();",
        "        for (id, entry) in catalog.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert!(constants.insert(entry.constant_name));",
        "            if let Some(name) = entry.serialized_name { assert!(names.insert(name)); }",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn item_component_value_catalogs_are_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_ITEM_USE_ANIMATIONS.len(), 12);",
        "        assert_eq!(VANILLA_SWING_ANIMATION_TYPES.len(), 3);",
        "        assert_eq!(VANILLA_ITEM_RARITIES.len(), 4);",
        "        assert_eq!(VANILLA_MAP_POST_PROCESSING_VALUES.len(), 2);",
        "        assert_eq!(VANILLA_FIREWORK_EXPLOSION_SHAPES.len(), 5);",
        "        assert_catalog(VANILLA_ITEM_USE_ANIMATIONS);",
        "        assert_catalog(VANILLA_SWING_ANIMATION_TYPES);",
        "        assert_catalog(VANILLA_ITEM_RARITIES);",
        "        assert_catalog(VANILLA_MAP_POST_PROCESSING_VALUES);",
        "        assert_catalog(VANILLA_FIREWORK_EXPLOSION_SHAPES);",
        "    }",
        "",
        "    #[test]",
        "    fn item_component_value_catalogs_preserve_ids_names_and_zero_fallback() {",
        "        assert_eq!(ItemUseAnimationSummary::Spear.id(), 11);",
        "        assert_eq!(ItemUseAnimationSummary::from_id(12), ItemUseAnimationSummary::None);",
        "        assert_eq!(ItemUseAnimationSummary::TootHorn.as_str(), \"toot_horn\");",
        "        assert_eq!(SwingAnimationTypeSummary::from_id(2), SwingAnimationTypeSummary::Stab);",
        "        assert_eq!(SwingAnimationTypeSummary::from_id(-1), SwingAnimationTypeSummary::None);",
        "        assert_eq!(ItemRaritySummary::Rare.as_str(), \"rare\");",
        "        assert_eq!(ItemRaritySummary::from_id(4), ItemRaritySummary::Common);",
        "        assert_eq!(MapPostProcessingSummary::from_id(1), MapPostProcessingSummary::Scale);",
        "        assert_eq!(MapPostProcessingSummary::from_id(2), MapPostProcessingSummary::Lock);",
        "        assert_eq!(FireworkExplosionShapeSummary::from_serialized_name(\"large_ball\"), Some(FireworkExplosionShapeSummary::LargeBall));",
        "        assert_eq!(FireworkExplosionShapeSummary::from_id(5), FireworkExplosionShapeSummary::SmallBall);",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_item_component_value_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/item_component_value_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"enum ItemUseAnimationSummary {\") || production.contains(\"enum SwingAnimationTypeSummary {\") || production.contains(\"enum FireworkExplosionShapeSummary {\") || production.contains(\"enum ItemRaritySummary {\") || production.contains(\"enum MapPostProcessingSummary {\") || production.contains(\"fn firework_explosion_shape_name\") || production.contains(\"trait ItemRaritySummaryExt\") || production.contains(\"1 => ItemRaritySummary::Uncommon\") || production.contains(\"1 => SwingAnimationTypeSummary::Whack\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel item component value owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_consume_effect_types(entries: list[ConsumeEffectTypeEntry]) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 ConsumeEffect.Type nested register field order.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_CONSUME_EFFECT_TYPE_{entry.constant_name}_ID: i32 = {entry.type_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(i32)]",
        "pub(crate) enum ConsumeEffectType {",
    ))
    for entry in entries:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_CONSUME_EFFECT_TYPE_{entry.constant_name}_ID,"
        )
    lines.extend((
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ConsumeEffectTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub resource_id: &'static str,",
        "    pub(crate) effect_type: ConsumeEffectType,",
        "}",
        "",
        "pub const VANILLA_CONSUME_EFFECT_TYPES: &[ConsumeEffectTypeCatalogEntry] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    ConsumeEffectTypeCatalogEntry {",
            f"        id: {entry.type_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        resource_id: {rust_string(entry.resource_id)},",
            f"        effect_type: ConsumeEffectType::{rust_pascal_case(entry.constant_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "impl ConsumeEffectType {",
        "    pub(crate) fn from_id(id: i32) -> Option<Self> {",
        "        VANILLA_CONSUME_EFFECT_TYPES",
        "            .get(usize::try_from(id).ok()?)",
        "            .map(|entry| entry.effect_type)",
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
        "    fn consume_effect_type_catalog_is_complete_contiguous_and_unique() {",
        "        assert_eq!(VANILLA_CONSUME_EFFECT_TYPES.len(), 5);",
        "        let mut constants = HashSet::new();",
        "        let mut resources = HashSet::new();",
        "        for (id, entry) in VANILLA_CONSUME_EFFECT_TYPES.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert_eq!(entry.effect_type as i32, entry.id);",
        "            assert!(constants.insert(entry.constant_name));",
        "            assert!(resources.insert(entry.resource_id));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn consume_effect_type_catalog_preserves_vanilla_register_order() {",
        "        assert_eq!(ConsumeEffectType::from_id(0), Some(ConsumeEffectType::ApplyEffects));",
        "        assert_eq!(ConsumeEffectType::from_id(4), Some(ConsumeEffectType::PlaySound));",
        "        assert_eq!(ConsumeEffectType::from_id(-1), None);",
        "        assert_eq!(ConsumeEffectType::from_id(5), None);",
        "        assert_eq!(VANILLA_CONSUME_EFFECT_TYPES[3].resource_id, \"minecraft:teleport_randomly\");",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_consume_effect_type_ids() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/consume_effect_types.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"0 => {\") && production.contains(\"ConsumeEffectSummary::ApplyEffects\") && production.contains(\"4 => Ok(ConsumeEffectSummary::PlaySound\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel consume-effect type ids: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_builtin_enum_component_values(
    value_types: list[BuiltinEnumValueTypeEntry],
    components: list[BuiltinEnumComponentEntry],
) -> str:
    component_constants = sorted({entry.constant_name for entry in components})
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 DataComponents and built-in enum value/ByIdMap declarations.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "use crate::data_component_types::{",
    ]
    for constant in component_constants:
        lines.append(f"    VANILLA_DATA_COMPONENT_{constant}_TYPE_ID,")
    lines.extend((
        "};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum BuiltinEnumValueIdStrategy { Zero, Clamp, Wrap, SparseDefault }",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct BuiltinEnumValueEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct BuiltinEnumValueTypeDescriptor {",
        "    pub constant_name: &'static str,",
        "    pub strategy: BuiltinEnumValueIdStrategy,",
        "    pub default_id: i32,",
        "    pub values: &'static [BuiltinEnumValueEntry],",
        "}",
        "",
        "impl BuiltinEnumValueTypeDescriptor {",
        "    fn normalized_id(self, id: i32) -> i32 {",
        "        let exact = self.values.iter().any(|entry| entry.id == id);",
        "        match self.strategy {",
        "            BuiltinEnumValueIdStrategy::Zero | BuiltinEnumValueIdStrategy::SparseDefault =>",
        "                if exact { id } else { self.default_id },",
        "            BuiltinEnumValueIdStrategy::Clamp => id.clamp(self.values.first().unwrap().id, self.values.last().unwrap().id),",
        "            BuiltinEnumValueIdStrategy::Wrap => id.rem_euclid(i32::try_from(self.values.len()).unwrap()),",
        "        }",
        "    }",
        "",
        "    fn value_name(self, id: i32) -> &'static str {",
        "        let id = self.normalized_id(id);",
        "        self.values.iter().find(|entry| entry.id == id).unwrap().serialized_name",
        "    }",
        "}",
        "",
    ))
    for value_type in value_types:
        catalog = f"VANILLA_{value_type.constant_name}_VALUES"
        descriptor = f"VANILLA_{value_type.constant_name}_VALUE_TYPE"
        for value in value_type.values:
            lines.append(
                f"pub const VANILLA_{value_type.constant_name}_{value.constant_name}_ID: i32 = {value.value_id};"
            )
        lines.append("")
        lines.append(f"pub const {catalog}: &[BuiltinEnumValueEntry] = &[")
        for value in value_type.values:
            lines.extend((
                "    BuiltinEnumValueEntry {",
                f"        id: {value.value_id},",
                f"        constant_name: {rust_string(value.constant_name)},",
                f"        serialized_name: {rust_string(value.serialized_name)},",
                "    },",
            ))
        rust_strategy = {
            "ZERO": "Zero",
            "CLAMP": "Clamp",
            "WRAP": "Wrap",
            "SPARSE": "SparseDefault",
        }[value_type.strategy]
        lines.extend((
            "];",
            "",
            f"pub const {descriptor}: BuiltinEnumValueTypeDescriptor = BuiltinEnumValueTypeDescriptor {{",
            f"    constant_name: {rust_string(value_type.constant_name)},",
            f"    strategy: BuiltinEnumValueIdStrategy::{rust_strategy},",
            f"    default_id: {value_type.default_id},",
            f"    values: {catalog},",
            "};",
            "",
        ))
        if value_type.texture_diffuse_colors is not None:
            colors = ", ".join(
                f"0x{color:06X}" for color in value_type.texture_diffuse_colors
            )
            lines.extend((
                f"pub const VANILLA_{value_type.constant_name}_TEXTURE_DIFFUSE_COLORS: &[u32] = &[",
                f"    {colors},",
                "];",
                "",
            ))
    lines.append(
        "pub const VANILLA_BUILTIN_ENUM_VALUE_TYPES: &[&BuiltinEnumValueTypeDescriptor] = &["
    )
    for value_type in value_types:
        lines.append(f"    &VANILLA_{value_type.constant_name}_VALUE_TYPE,")
    lines.extend((
        "];",
        "",
    ))
    lines.extend((
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct BuiltinEnumComponentDescriptor {",
        "    pub type_id: i32,",
        "    pub constant_name: &'static str,",
        "    pub value_type: &'static BuiltinEnumValueTypeDescriptor,",
        "}",
        "",
        "pub const VANILLA_BUILTIN_ENUM_COMPONENTS: &[BuiltinEnumComponentDescriptor] = &[",
    ))
    for component in components:
        lines.extend((
            "    BuiltinEnumComponentDescriptor {",
            f"        type_id: VANILLA_DATA_COMPONENT_{component.constant_name}_TYPE_ID,",
            f"        constant_name: {rust_string(component.constant_name)},",
            f"        value_type: &VANILLA_{component.value_type_constant_name}_VALUE_TYPE,",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "fn descriptor(type_id: i32) -> Option<&'static BuiltinEnumComponentDescriptor> {",
        "    VANILLA_BUILTIN_ENUM_COMPONENTS.iter().find(|entry| entry.type_id == type_id)",
        "}",
        "",
        "pub fn normalize_builtin_enum_component_value_id(type_id: i32, id: i32) -> Option<i32> {",
        "    Some(descriptor(type_id)?.value_type.normalized_id(id))",
        "}",
        "",
        "pub fn builtin_enum_component_value_name(type_id: i32, id: i32) -> Option<&'static str> {",
        "    Some(descriptor(type_id)?.value_type.value_name(id))",
        "}",
        "",
        "pub fn builtin_enum_component_value_for_name(type_id: i32, name: &str) -> Option<&'static BuiltinEnumValueEntry> {",
        "    descriptor(type_id)?.value_type.values.iter().find(|entry| entry.serialized_name == name)",
        "}",
        "",
        "pub fn vanilla_dye_color_name(id: i32) -> &'static str {",
        "    VANILLA_DYE_COLOR_VALUE_TYPE.value_name(id)",
        "}",
        "",
        "pub fn normalize_vanilla_dye_color_id(id: i32) -> i32 {",
        "    VANILLA_DYE_COLOR_VALUE_TYPE.normalized_id(id)",
        "}",
        "",
        "pub fn vanilla_dye_color_texture_diffuse_color(id: i32) -> u32 {",
        "    let normalized = normalize_vanilla_dye_color_id(id);",
        "    VANILLA_DYE_COLOR_TEXTURE_DIFFUSE_COLORS[usize::try_from(normalized).unwrap()]",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn built_in_enum_component_catalog_is_complete_and_unique() {",
        "        assert_eq!(VANILLA_BUILTIN_ENUM_COMPONENTS.len(), 16);",
        "        let mut ids = HashSet::new();",
        "        let mut names = HashSet::new();",
        "        for descriptor in VANILLA_BUILTIN_ENUM_COMPONENTS {",
        "            assert!(ids.insert(descriptor.type_id));",
        "            assert!(names.insert(descriptor.constant_name));",
        "            assert!(!descriptor.value_type.values.is_empty());",
        "        }",
        "        assert!(VANILLA_BUILTIN_ENUM_COMPONENTS.windows(2).all(|pair| pair[0].type_id < pair[1].type_id));",
        "        assert_eq!(VANILLA_BUILTIN_ENUM_COMPONENTS.first().unwrap().constant_name, \"DYE\");",
        "        assert_eq!(VANILLA_BUILTIN_ENUM_COMPONENTS.last().unwrap().constant_name, \"SHULKER_COLOR\");",
        "        assert_eq!(VANILLA_BUILTIN_ENUM_VALUE_TYPES.len(), 9);",
        "        for value_type in VANILLA_BUILTIN_ENUM_VALUE_TYPES {",
        "            let mut value_ids = HashSet::new();",
        "            let mut value_names = HashSet::new();",
        "            for value in value_type.values {",
        "                assert!(value_ids.insert(value.id));",
        "                assert!(value_names.insert(value.serialized_name));",
        "            }",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn built_in_enum_component_values_preserve_vanilla_fallbacks_and_names() {",
        "        assert_eq!(VANILLA_DYE_COLOR_WHITE_ID, 0);",
        "        assert_eq!(VANILLA_DYE_COLOR_ORANGE_ID, 1);",
        "        assert_eq!(VANILLA_DYE_COLOR_GRAY_ID, 7);",
        "        assert_eq!(VANILLA_DYE_COLOR_RED_ID, 14);",
        "        assert_eq!(vanilla_dye_color_name(14), \"red\");",
        "        assert_eq!(vanilla_dye_color_name(16), \"white\");",
        "        assert_eq!(normalize_vanilla_dye_color_id(-1), 0);",
        "        assert_eq!(vanilla_dye_color_texture_diffuse_color(VANILLA_DYE_COLOR_WHITE_ID), 0xF9FFFE);",
        "        assert_eq!(vanilla_dye_color_texture_diffuse_color(VANILLA_DYE_COLOR_RED_ID), 0xB02E26);",
        "        assert_eq!(vanilla_dye_color_texture_diffuse_color(16), 0xF9FFFE);",
        "        assert_eq!(builtin_enum_component_value_name(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, -1), Some(\"small\"));",
        "        assert_eq!(builtin_enum_component_value_name(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, 3), Some(\"large\"));",
        "        assert_eq!(builtin_enum_component_value_name(VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID, -1), Some(\"dark_brown\"));",
        "        assert_eq!(builtin_enum_component_value_name(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, 98), Some(\"brown\"));",
        "        assert_eq!(normalize_builtin_enum_component_value_id(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, 99), Some(99));",
        "        assert_eq!(builtin_enum_component_value_for_name(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, \"evil\").map(|entry| entry.id), Some(99));",
        "        assert_eq!(builtin_enum_component_value_for_name(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, \"killer\"), None);",
        "        assert_eq!(normalize_builtin_enum_component_value_id(-1, 0), None);",
        "    }",
        "",
        "    fn starts_with_numeric_or_optional_numeric(value: &str) -> bool {",
        '        let value = value.strip_prefix("Some(").unwrap_or(value);',
        "        let mut characters = value.chars();",
        "        let Some(first) = characters.next() else { return false; };",
        "        first.is_ascii_digit() || (first == '-' && characters.next().is_some_and(|character| character.is_ascii_digit()))",
        "    }",
        "",
        "    fn contains_numeric_dye_color_reference(source: &str) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        '        ["dye_color_id", "base_color_id", "color_id"].iter().any(|field| {',
        "            compact.match_indices(field).any(|(offset, _)| {",
        "                let suffix = &compact[offset + field.len()..];",
        '                [":", ",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_numeric_or_optional_numeric))',
        "            })",
        "        })",
        "    }",
        "",
        "    #[test]",
        "    fn numeric_dye_color_guard_covers_semantic_fixtures() {",
        '        assert!(contains_numeric_dye_color_reference("dye_color_id: Some(14),"));',
        '        assert!(contains_numeric_dye_color_reference("base_color_id: Some(3),"));',
        '        assert!(contains_numeric_dye_color_reference("color_id: 11,"));',
        '        assert!(contains_numeric_dye_color_reference("assert_eq!(patch.tropical_fish_pattern_color_id, Some(7));"));',
        '        assert!(!contains_numeric_dye_color_reference("dye_color_id: Some(VANILLA_DYE_COLOR_RED_ID),"));',
        '        assert!(!contains_numeric_dye_color_reference("tropical_fish_pattern_color_id: Some(unknown_dye_color_id),"));',
        "    }",
        "",
        "    fn contains_numeric_builtin_enum_component_map_entry(source: &str) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        '        compact.split("builtin_enum_component_ids:BTreeMap::from([(").skip(1).any(|suffix| {',
        '            let Some((entries, _)) = suffix.split_once("])") else { return false; };',
        '            entries.split("),(").any(|entry| {',
        "                entry.strip_prefix('(').unwrap_or(entry).split(',').take(2).any(starts_with_numeric_or_optional_numeric)",
        "            })",
        "        })",
        "    }",
        "",
        "    #[test]",
        "    fn numeric_builtin_enum_component_map_guard_checks_keys_and_values() {",
        '        assert!(contains_numeric_builtin_enum_component_map_entry("builtin_enum_component_ids: BTreeMap::from([(84, 14), (85, 1)]),"));',
        '        assert!(contains_numeric_builtin_enum_component_map_entry("builtin_enum_component_ids: BTreeMap::from([(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, 14)]),"));',
        '        assert!(!contains_numeric_builtin_enum_component_map_entry("builtin_enum_component_ids: BTreeMap::from([(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, VANILLA_DYE_COLOR_RED_ID)]),"));',
        "    }",
        "",
        "    #[test]",
        "    fn semantic_fixtures_do_not_restore_numeric_builtin_enum_component_map_entries() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        '                if contains_numeric_builtin_enum_component_map_entry(&source) { violations.push(path.display().to_string()); }',
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
        "        for directory in [\"bbb-native/src\", \"bbb-world/src\", \"bbb-net/src\", \"bbb-item-model/src\", \"bbb-renderer/src\"] { visit(&crates.join(directory), &mut violations); }",
        "        assert!(violations.is_empty(), \"numeric built-in enum component map entries: {violations:?}\");",
        "    }",
        "",
        "    #[test]",
        "    fn semantic_fixtures_do_not_restore_numeric_dye_color_ids() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        '                if contains_numeric_dye_color_reference(&source) { violations.push(path.display().to_string()); }',
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
        "        for directory in [\"bbb-native/src\", \"bbb-world/src\", \"bbb-net/src\", \"bbb-item-model/src\", \"bbb-renderer/src\"] { visit(&crates.join(directory), &mut violations); }",
        "        assert!(violations.is_empty(), \"numeric dye color ids: {violations:?}\");",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_built_in_enum_component_values() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/builtin_enum_component_values.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"fn normalize_builtin_enum_component_id\") || production.contains(\"fn normalize_dye_color_id\") || production.contains(\"fn debug_builtin_enum_component_name\") || production.contains(\"fn debug_dye_color_name\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel built-in enum component value owners: {violations:?}\");",
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_attribute_modifier_types(
    operations: list[NamedComponentValueEntry],
    displays: list[NamedComponentValueEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 AttributeModifier.Operation and ItemAttributeModifiers.Display.Type.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct AttributeModifierTypeCatalogEntry {",
        "    pub id: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "}",
        "",
    ]
    for entry in operations:
        lines.append(
            f"pub const VANILLA_ATTRIBUTE_MODIFIER_OPERATION_{entry.constant_name}_ID: i32 = {entry.value_id};"
        )
    lines.extend((
        "",
        "pub const VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS: &[AttributeModifierTypeCatalogEntry] = &[",
    ))
    for entry in operations:
        lines.extend((
            "    AttributeModifierTypeCatalogEntry {",
            f"        id: {entry.value_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
    ))
    for entry in displays:
        lines.append(
            f"pub const VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_{entry.constant_name}_ID: i32 = {entry.value_id};"
        )
    lines.extend((
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "#[repr(i32)]",
        "pub(crate) enum AttributeModifierDisplayType {",
    ))
    for entry in displays:
        lines.append(
            f"    {rust_pascal_case(entry.constant_name)} = VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_{entry.constant_name}_ID,"
        )
    lines.extend((
        "}",
        "",
        "pub const VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES: &[AttributeModifierTypeCatalogEntry] = &[",
    ))
    for entry in displays:
        lines.extend((
            "    AttributeModifierTypeCatalogEntry {",
            f"        id: {entry.value_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "fn zero_entry(catalog: &'static [AttributeModifierTypeCatalogEntry], id: i32) -> &'static AttributeModifierTypeCatalogEntry {",
        "    catalog.get(usize::try_from(id).unwrap_or(usize::MAX)).unwrap_or(&catalog[0])",
        "}",
        "",
        "pub fn normalize_attribute_modifier_operation_id(id: i32) -> i32 {",
        "    zero_entry(VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS, id).id",
        "}",
        "",
        "pub fn attribute_modifier_operation_name(id: i32) -> Option<&'static str> {",
        "    VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS.get(usize::try_from(id).ok()?).map(|entry| entry.serialized_name)",
        "}",
        "",
        "pub fn attribute_modifier_operation_id(name: &str) -> Option<i32> {",
        "    VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS.iter().find(|entry| entry.serialized_name == name).map(|entry| entry.id)",
        "}",
        "",
        "pub fn attribute_modifier_display_name(id: i32) -> Option<&'static str> {",
        "    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES.get(usize::try_from(id).ok()?).map(|entry| entry.serialized_name)",
        "}",
        "",
        "pub fn attribute_modifier_display_id(name: &str) -> Option<i32> {",
        "    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES.iter().find(|entry| entry.serialized_name == name).map(|entry| entry.id)",
        "}",
        "",
        "impl AttributeModifierDisplayType {",
        "    pub(crate) fn from_id(id: i32) -> Self {",
        "        match zero_entry(VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES, id).id {",
    ))
    for entry in displays[1:]:
        lines.append(
            f"            VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_{entry.constant_name}_ID => Self::{rust_pascal_case(entry.constant_name)},"
        )
    lines.extend((
        "            _ => Self::Default,",
        "        }",
        "    }",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    fn assert_catalog(catalog: &[AttributeModifierTypeCatalogEntry]) {",
        "        let mut names = HashSet::new();",
        "        for (id, entry) in catalog.iter().enumerate() {",
        "            assert_eq!(entry.id, id as i32);",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn attribute_modifier_type_catalogs_preserve_ids_names_and_zero_fallback() {",
        "        assert_catalog(VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS);",
        "        assert_catalog(VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES);",
        "        assert_eq!(normalize_attribute_modifier_operation_id(3), 0);",
        "        assert_eq!(attribute_modifier_operation_name(2), Some(\"add_multiplied_total\"));",
        "        assert_eq!(attribute_modifier_operation_id(\"add_multiplied_base\"), Some(1));",
        "        assert_eq!(attribute_modifier_display_name(1), Some(\"hidden\"));",
        "        assert_eq!(attribute_modifier_display_id(\"override\"), Some(2));",
        "        assert_eq!(AttributeModifierDisplayType::from_id(1), AttributeModifierDisplayType::Hidden);",
        "        assert_eq!(AttributeModifierDisplayType::from_id(3), AttributeModifierDisplayType::Default);",
        "    }",
        "",
        "    fn starts_with_numeric_or_optional_numeric(value: &str) -> bool {",
        '        let value = value.strip_prefix("Some(").unwrap_or(value);',
        "        let mut characters = value.chars();",
        "        let Some(first) = characters.next() else { return false; };",
        "        first.is_ascii_digit() || (first == '-' && characters.next().is_some_and(|character| character.is_ascii_digit()))",
        "    }",
        "",
        "    fn contains_numeric_attribute_modifier_operation_reference(source: &str) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        '        let numeric_construction = compact.match_indices("operation_id:").any(|(offset, _)| {',
        '            starts_with_numeric_or_optional_numeric(&compact[offset + "operation_id:".len()..])',
        "        });",
        '        numeric_construction || compact.match_indices(".operation_id").any(|(offset, _)| {',
        '            let suffix = &compact[offset + ".operation_id".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| {',
        "                suffix.strip_prefix(separator).is_some_and(starts_with_numeric_or_optional_numeric)",
        "            })",
        "        })",
        "    }",
        "",
        "    fn contains_numeric_attribute_modifier_slot_or_display_reference(source: &str) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        '        let numeric_slot_construction = compact.match_indices("slot_group_id:").any(|(offset, _)| {',
        '            starts_with_numeric_or_optional_numeric(&compact[offset + "slot_group_id:".len()..])',
        "        });",
        '        let numeric_slot_reference = compact.match_indices(".slot_group_id").any(|(offset, _)| {',
        '            let suffix = &compact[offset + ".slot_group_id".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_numeric_or_optional_numeric))',
        "        });",
        '        let numeric_display_construction = compact.match_indices("DialogAttributeModifierDisplayState{type_id:").any(|(offset, _)| {',
        '            starts_with_numeric_or_optional_numeric(&compact[offset + "DialogAttributeModifierDisplayState{type_id:".len()..])',
        "        });",
        '        let numeric_display_reference = compact.match_indices(".display.type_id").any(|(offset, _)| {',
        '            let suffix = &compact[offset + ".display.type_id".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_numeric_or_optional_numeric))',
        "        });",
        "        numeric_slot_construction || numeric_slot_reference || numeric_display_construction || numeric_display_reference",
        "    }",
        "",
        "    #[test]",
        "    fn numeric_attribute_modifier_operation_guard_covers_construction_and_comparison() {",
        '        assert!(contains_numeric_attribute_modifier_operation_reference("operation_id: 0,"));',
        '        assert!(contains_numeric_attribute_modifier_operation_reference("operation_id: Some(1),"));',
        '        assert!(contains_numeric_attribute_modifier_operation_reference("assert_eq!(modifier.operation_id,\\n 2);"));',
        '        assert!(contains_numeric_attribute_modifier_operation_reference("assert_eq!(modifier.operation_id, Some(2));"));',
        '        assert!(!contains_numeric_attribute_modifier_operation_reference("operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,"));',
        '        assert!(!contains_numeric_attribute_modifier_operation_reference("assert_eq!(modifier.operation_id, VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID);"));',
        "    }",
        "",
        "    #[test]",
        "    fn numeric_attribute_modifier_slot_and_display_guard_covers_semantic_fixtures() {",
        '        assert!(contains_numeric_attribute_modifier_slot_or_display_reference("slot_group_id: 0,"));',
        '        assert!(contains_numeric_attribute_modifier_slot_or_display_reference("slot_group_id: Some(3),"));',
        '        assert!(contains_numeric_attribute_modifier_slot_or_display_reference("DialogAttributeModifierDisplayState { type_id: 2 }"));',
        '        assert!(contains_numeric_attribute_modifier_slot_or_display_reference("assert_eq!(modifier.display.type_id, 1);"));',
        '        assert!(!contains_numeric_attribute_modifier_slot_or_display_reference("slot_group_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,"));',
        '        assert!(!contains_numeric_attribute_modifier_slot_or_display_reference("DialogAttributeModifierDisplayState { type_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID }"));',
        "    }",
        "",
        "    #[test]",
        "    fn semantic_sources_do_not_redeclare_attribute_modifier_types() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/attribute_modifier_types.rs\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        '                if contains_numeric_attribute_modifier_operation_reference(&source) { violations.push(format!("{}: numeric operation_id reference", path.display())); }',
        '                if contains_numeric_attribute_modifier_slot_or_display_reference(&source) { violations.push(format!("{}: numeric attribute-modifier slot/display reference", path.display())); }',
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"fn decode_attribute_modifier_operation_id\") || production.contains(\"fn attribute_modifier_operation_id\") || production.contains(\"[\\\"add_value\\\", \\\"add_multiplied_base\\\", \\\"add_multiplied_total\\\"]\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
        "        for directory in [\"bbb-native/src\", \"bbb-world/src\", \"bbb-net/src\", \"bbb-item-model/src\", \"bbb-renderer/src\"] { visit(&crates.join(directory), &mut violations); }",
        "        assert!(violations.is_empty(), \"parallel attribute modifier type owners: {violations:?}\");",
        "    }",
        "",
        "    #[test]",
        "    fn item_model_custom_registry_fixture_uses_generated_operation_constants() {",
        '        let source = include_str!("../../bbb-item-model/src/item_runtime/tests.rs");',
        '        let fixture = source.split("let custom_attribute_id").nth(1).and_then(|source| source.split("let default_attribute_item").next()).expect("item-model custom attribute fixture");',
        '        assert!(fixture.contains("VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID"));',
        '        assert!(fixture.contains("VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID"));',
        '        assert!(fixture.contains("custom_attribute_id(\\\"minecraft:attack_damage\\\")"));',
        '        assert!(fixture.contains("custom_attribute_id(\\\"minecraft:scale\\\")"));',
        '        assert!(!fixture.contains("operation_id, slot_id|"));',
        '        assert!(!fixture.contains("attribute_id: 7"));',
        '        assert!(!fixture.contains("modifier_with_attribute(0,"));',
        '        assert!(!fixture.contains("modifier_with_attribute(1,"));',
        "    }",
        "}",
        "",
    ))
    return "\n".join(lines)


def render_tropical_fish_patterns(
    entries: list[TropicalFishPatternEntry],
    common_variants: list[TropicalFishCommonVariantEntry],
) -> str:
    lines = [
        "// @generated by scripts/generate-gameplay-id-catalog.py; do not edit manually.",
        "// Source: vanilla 26.1 TropicalFish.Base and TropicalFish.Pattern.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py --check",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct TropicalFishPatternDescriptor {",
        "    pub packed_id: i32,",
        "    pub constant_name: &'static str,",
        "    pub serialized_name: &'static str,",
        "    pub base_id: i32,",
        "    pub pattern_index: u8,",
        "}",
        "",
    ]
    for entry in entries:
        lines.append(
            f"pub const VANILLA_TROPICAL_FISH_PATTERN_{entry.constant_name}_PACKED_ID: i32 = {entry.packed_id};"
        )
    lines.extend((
        "",
        "pub const VANILLA_TROPICAL_FISH_PATTERNS: &[TropicalFishPatternDescriptor] = &[",
    ))
    for entry in entries:
        lines.extend((
            "    TropicalFishPatternDescriptor {",
            f"        packed_id: {entry.packed_id},",
            f"        constant_name: {rust_string(entry.constant_name)},",
            f"        serialized_name: {rust_string(entry.serialized_name)},",
            f"        base_id: {entry.base_id},",
            f"        pattern_index: {entry.pattern_index},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct TropicalFishCommonVariantDescriptor {",
        "    pub pattern_packed_id: i32,",
        "    pub base_color_id: i32,",
        "    pub pattern_color_id: i32,",
        "}",
        "",
        "pub const VANILLA_TROPICAL_FISH_COMMON_VARIANTS: &[TropicalFishCommonVariantDescriptor] = &[",
    ))
    for variant in common_variants:
        lines.extend((
            "    TropicalFishCommonVariantDescriptor {",
            f"        pattern_packed_id: {variant.pattern_packed_id},",
            f"        base_color_id: {variant.base_color_id},",
            f"        pattern_color_id: {variant.pattern_color_id},",
            "    },",
        ))
    lines.extend((
        "];",
        "",
        "pub fn vanilla_tropical_fish_pattern(packed_id: i32) -> &'static TropicalFishPatternDescriptor {",
        "    VANILLA_TROPICAL_FISH_PATTERNS.iter().find(|entry| entry.packed_id == packed_id).unwrap_or(&VANILLA_TROPICAL_FISH_PATTERNS[0])",
        "}",
        "",
        "pub fn vanilla_tropical_fish_pattern_for_name(name: &str) -> Option<&'static TropicalFishPatternDescriptor> {",
        "    VANILLA_TROPICAL_FISH_PATTERNS.iter().find(|entry| entry.serialized_name == name)",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::{collections::HashSet, fs, path::Path};",
        "",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn tropical_fish_pattern_catalog_preserves_sparse_ids_names_and_kob_fallback() {",
        "        assert_eq!(VANILLA_TROPICAL_FISH_PATTERNS.len(), 12);",
        "        let mut ids = HashSet::new();",
        "        let mut names = HashSet::new();",
        "        for entry in VANILLA_TROPICAL_FISH_PATTERNS {",
        "            assert!(ids.insert(entry.packed_id));",
        "            assert!(names.insert(entry.serialized_name));",
        "        }",
        "        assert_eq!(vanilla_tropical_fish_pattern(257).serialized_name, \"stripey\");",
        "        assert_eq!(vanilla_tropical_fish_pattern(9999).constant_name, \"KOB\");",
        "        assert_eq!(vanilla_tropical_fish_pattern(1281).base_id, 1);",
        "        assert_eq!(vanilla_tropical_fish_pattern(1281).pattern_index, 5);",
        "        assert_eq!(vanilla_tropical_fish_pattern_for_name(\"spotty\").map(|entry| entry.packed_id), Some(1280));",
        "        assert_eq!(vanilla_tropical_fish_pattern_for_name(\"missing\"), None);",
        "        assert_eq!(VANILLA_TROPICAL_FISH_COMMON_VARIANTS.len(), 22);",
        "        assert_eq!(VANILLA_TROPICAL_FISH_COMMON_VARIANTS[0], TropicalFishCommonVariantDescriptor { pattern_packed_id: 257, base_color_id: 1, pattern_color_id: 7 });",
        "        assert_eq!(VANILLA_TROPICAL_FISH_COMMON_VARIANTS[21], TropicalFishCommonVariantDescriptor { pattern_packed_id: 1, base_color_id: 4, pattern_color_id: 4 });",
        "    }",
        "",
        "    fn starts_with_numeric_or_optional_numeric(value: &str) -> bool {",
        '        let value = value.strip_prefix("Some(").unwrap_or(value);',
        "        let mut characters = value.chars();",
        "        let Some(first) = characters.next() else { return false; };",
        "        first.is_ascii_digit() || (first == '-' && characters.next().is_some_and(|character| character.is_ascii_digit()))",
        "    }",
        "",
        "    fn contains_numeric_tropical_fish_pattern_reference(source: &str, include_dialog_value_id: bool) -> bool {",
        "        let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();",
        '        let numeric_construction = compact.match_indices("tropical_fish_pattern_id:").any(|(offset, _)| {',
        '            starts_with_numeric_or_optional_numeric(&compact[offset + "tropical_fish_pattern_id:".len()..])',
        "        });",
        '        let numeric_reference = compact.match_indices(".tropical_fish_pattern_id").any(|(offset, _)| {',
        '            let suffix = &compact[offset + ".tropical_fish_pattern_id".len()..];',
        '            [",", "==", "!=", "="].iter().any(|separator| suffix.strip_prefix(separator).is_some_and(starts_with_numeric_or_optional_numeric))',
        "        });",
        '        let numeric_dialog_value_id = include_dialog_value_id && compact.match_indices("value_id:").any(|(offset, _)| {',
        '            starts_with_numeric_or_optional_numeric(&compact[offset + "value_id:".len()..])',
        "        });",
        "        numeric_construction || numeric_reference || numeric_dialog_value_id",
        "    }",
        "",
        "    #[test]",
        "    fn numeric_tropical_fish_pattern_guard_covers_semantic_fixtures() {",
        '        assert!(contains_numeric_tropical_fish_pattern_reference("tropical_fish_pattern_id: Some(257),", false));',
        '        assert!(contains_numeric_tropical_fish_pattern_reference("assert_eq!(patch.tropical_fish_pattern_id, Some(1));", false));',
        '        assert!(contains_numeric_tropical_fish_pattern_reference("value_id: 1,", true));',
        '        assert!(!contains_numeric_tropical_fish_pattern_reference("value_id: 1,", false));',
        '        assert!(!contains_numeric_tropical_fish_pattern_reference("tropical_fish_pattern_id: Some(VANILLA_TROPICAL_FISH_PATTERN_STRIPEY_PACKED_ID),", false));',
        '        assert!(!contains_numeric_tropical_fish_pattern_reference("assert_eq!(patch.tropical_fish_pattern_id, Some(VANILLA_TROPICAL_FISH_PATTERN_FLOPPER_PACKED_ID));", false));',
        "    }",
        "",
        "    #[test]",
        "    fn semantic_fixtures_do_not_restore_numeric_tropical_fish_pattern_ids() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        '                let include_dialog_value_id = path.to_string_lossy().ends_with("/bbb-world/src/client/dialog/body.rs");',
        '                if contains_numeric_tropical_fish_pattern_reference(&source, include_dialog_value_id) { violations.push(path.display().to_string()); }',
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        let crates = Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap();",
        "        for directory in [\"bbb-native/src\", \"bbb-world/src\", \"bbb-net/src\", \"bbb-item-model/src\", \"bbb-renderer/src\"] { visit(&crates.join(directory), &mut violations); }",
        "        assert!(violations.is_empty(), \"numeric tropical fish pattern ids: {violations:?}\");",
        "    }",
        "",
        "    #[test]",
        "    fn production_sources_do_not_redeclare_tropical_fish_pattern_facts() {",
        "        fn visit(directory: &Path, violations: &mut Vec<String>) {",
        "            for entry in fs::read_dir(directory).unwrap() {",
        "                let path = entry.unwrap().path();",
        "                if path.is_dir() { visit(&path, violations); continue; }",
        "                if path.extension().and_then(|value| value.to_str()) != Some(\"rs\") { continue; }",
        "                let display = path.to_string_lossy();",
        "                if display.ends_with(\"/tropical_fish_patterns.rs\") || display.ends_with(\"/tests.rs\") || display.contains(\"/tests/\") { continue; }",
        "                let source = fs::read_to_string(&path).unwrap();",
        "                let production = source.split(\"#[cfg(test)]\").next().unwrap_or(&source);",
        "                if production.contains(\"fn debug_tropical_fish_pattern_name\") || production.contains(\"const TROPICAL_FISH_PATTERNS:\") { violations.push(display.to_string()); }",
        "            }",
        "        }",
        "        let mut violations = Vec::new();",
        "        visit(Path::new(env!(\"CARGO_MANIFEST_DIR\")).parent().unwrap(), &mut violations);",
        "        assert!(violations.is_empty(), \"parallel tropical fish pattern owners: {violations:?}\");",
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
    beacon_source = parse_java(source_root, BEACON_BLOCK_ENTITY_SOURCE)
    entries = mob_effect_entries(source)
    tiers = beacon_effect_tiers(beacon_source)
    known_constants = {entry.constant_name for entry in entries}
    for tier in tiers:
        for reference in tier:
            if reference.constant_name not in known_constants:
                span = reference.expression.span
                raise GameplayIdExtractionError(
                    f"{beacon_source.relative_path}:{span.start_point[0] + 1}:"
                    f"{span.start_point[1] + 1}: unknown beacon mob effect "
                    f"{reference.constant_name}"
                )
    return format_rust(render_mob_effects(entries, tiers))


def generate_menu_types(source_root: Path) -> str:
    source = parse_java(source_root, MENU_TYPES_SOURCE)
    return format_rust(render_menu_types(menu_type_entries(source)))


def generate_equipment_slots(source_root: Path) -> str:
    slots_source = parse_java(source_root, EQUIPMENT_SLOTS_SOURCE)
    inventory_source = parse_java(source_root, PLAYER_INVENTORY_SOURCE)
    groups_source = parse_java(source_root, EQUIPMENT_SLOT_GROUPS_SOURCE)
    slots = equipment_slot_entries(slots_source)
    return format_rust(
        render_equipment_slots(
            slots,
            player_inventory_equipment_slot_indices(inventory_source, slots),
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
    debug_source = parse_java(source_root, HEIGHTMAP_DEBUG_COLORS_SOURCE)
    return format_rust(
        render_heightmap_types(
            heightmap_type_entries(source),
            heightmap_debug_color_entries(debug_source),
        )
    )


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


def generate_debug_subscription_types(source_root: Path) -> str:
    source = parse_java(source_root, DEBUG_SUBSCRIPTION_TYPES_SOURCE)
    game_event_source = parse_java(source_root, DEBUG_GAME_EVENT_TYPES_SOURCE)
    poi_source = parse_java(source_root, DEBUG_POI_TYPES_SOURCE)
    return format_rust(
        render_debug_subscription_types(
            debug_subscription_entries(source),
            debug_game_event_entries(game_event_source),
            debug_poi_type_entries(poi_source),
        )
    )


def generate_world_clock_types(source_root: Path) -> str:
    source = parse_java(source_root, WORLD_CLOCK_TYPES_SOURCE)
    return format_rust(render_world_clock_types(world_clock_entries(source)))


def generate_game_rules(source_root: Path) -> str:
    source = parse_java(source_root, GAME_RULES_SOURCE)
    return format_rust(render_game_rules(game_rule_entries(source)))


def generate_stat_types(source_root: Path) -> str:
    source = parse_java(source_root, STAT_TYPES_SOURCE)
    objective_source = parse_java(source_root, OBJECTIVE_RENDER_TYPES_SOURCE)
    chat_source = parse_java(source_root, CHAT_FORMATTINGS_SOURCE)
    types, custom = stat_catalog_entries(source)
    criteria = objective_criteria_names(
        objective_source, chat_formatting_entries(chat_source)
    )
    return format_rust(render_stat_types(types, custom, criteria))


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


def generate_recipe_wire_types(source_root: Path) -> str:
    books_source = parse_java(source_root, RECIPE_BOOK_TYPES_SOURCE)
    displays_source = parse_java(source_root, RECIPE_DISPLAY_TYPES_SOURCE)
    slot_displays_source = parse_java(source_root, SLOT_DISPLAY_TYPES_SOURCE)
    categories_source = parse_java(source_root, RECIPE_BOOK_CATEGORIES_SOURCE)
    search_categories_source = parse_java(
        source_root, SEARCH_RECIPE_BOOK_CATEGORIES_SOURCE
    )
    books, displays, categories, search_categories = recipe_wire_type_entries(
        books_source, displays_source, categories_source, search_categories_source
    )
    slot_displays = slot_display_type_entries(slot_displays_source)
    return format_rust(
        render_recipe_wire_types(
            books, displays, categories, search_categories, slot_displays
        )
    )


def generate_item_component_value_types(source_root: Path) -> str:
    item_uses = parse_java(source_root, ITEM_USE_ANIMATIONS_SOURCE)
    swings = parse_java(source_root, SWING_ANIMATION_TYPES_SOURCE)
    rarities = parse_java(source_root, ITEM_RARITIES_SOURCE)
    map_post = parse_java(source_root, MAP_POST_PROCESSING_SOURCE)
    firework = parse_java(source_root, FIREWORK_EXPLOSION_SOURCE)
    entries = item_component_value_type_entries(
        item_uses, swings, rarities, map_post, firework
    )
    return format_rust(render_item_component_value_types(*entries))


def generate_consume_effect_types(source_root: Path) -> str:
    source = parse_java(source_root, CONSUME_EFFECT_TYPES_SOURCE)
    return format_rust(render_consume_effect_types(consume_effect_type_entries(source)))


def generate_builtin_enum_component_values(source_root: Path) -> str:
    sources = [
        parse_java(source_root, path)
        for path in (
            DATA_COMPONENTS_SOURCE,
            DYE_COLOR_SOURCE,
            FOX_SOURCE,
            SALMON_SOURCE,
            PARROT_SOURCE,
            MUSHROOM_COW_SOURCE,
            RABBIT_SOURCE,
            HORSE_VARIANT_SOURCE,
            LLAMA_SOURCE,
            AXOLOTL_SOURCE,
        )
    ]
    return format_rust(
        render_builtin_enum_component_values(
            *builtin_enum_component_value_entries(*sources)
        )
    )


def generate_attribute_modifier_types(source_root: Path) -> str:
    modifier = parse_java(source_root, ATTRIBUTE_MODIFIER_SOURCE)
    item_modifiers = parse_java(source_root, ITEM_ATTRIBUTE_MODIFIERS_SOURCE)
    return format_rust(
        render_attribute_modifier_types(
            *attribute_modifier_type_entries(modifier, item_modifiers)
        )
    )


def generate_tropical_fish_patterns(source_root: Path) -> str:
    source = parse_java(source_root, TROPICAL_FISH_SOURCE)
    dye_colors = parse_java(source_root, DYE_COLOR_SOURCE)
    return format_rust(
        render_tropical_fish_patterns(*tropical_fish_pattern_entries(source, dye_colors))
    )


def generate_map_colors(source_root: Path) -> str:
    source = parse_java(source_root, MAP_COLORS_SOURCE)
    return format_rust(render_map_colors(*map_color_entries(source)))


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
    generated_debug_subscription_types = generate_debug_subscription_types(
        args.source_root
    )
    generated_world_clock_types = generate_world_clock_types(args.source_root)
    generated_game_rules = generate_game_rules(args.source_root)
    generated_stat_types = generate_stat_types(args.source_root)
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
    generated_recipe_wire_types = generate_recipe_wire_types(args.source_root)
    generated_item_component_value_types = generate_item_component_value_types(
        args.source_root
    )
    generated_consume_effect_types = generate_consume_effect_types(args.source_root)
    generated_builtin_enum_component_values = generate_builtin_enum_component_values(
        args.source_root
    )
    generated_attribute_modifier_types = generate_attribute_modifier_types(args.source_root)
    generated_tropical_fish_patterns = generate_tropical_fish_patterns(args.source_root)
    generated_map_colors = generate_map_colors(args.source_root)
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
            args.debug_subscription_types_output, generated_debug_subscription_types
        )
        matches &= check_generated(
            args.world_clock_types_output, generated_world_clock_types
        )
        matches &= check_generated(args.game_rules_output, generated_game_rules)
        matches &= check_generated(args.stat_types_output, generated_stat_types)
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
        matches &= check_generated(
            args.recipe_wire_types_output, generated_recipe_wire_types
        )
        matches &= check_generated(
            args.item_component_value_types_output,
            generated_item_component_value_types,
        )
        matches &= check_generated(
            args.consume_effect_types_output, generated_consume_effect_types
        )
        matches &= check_generated(
            args.builtin_enum_component_values_output,
            generated_builtin_enum_component_values,
        )
        matches &= check_generated(
            args.attribute_modifier_types_output, generated_attribute_modifier_types
        )
        matches &= check_generated(
            args.tropical_fish_patterns_output, generated_tropical_fish_patterns
        )
        matches &= check_generated(args.map_colors_output, generated_map_colors)
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
    args.debug_subscription_types_output.write_text(generated_debug_subscription_types)
    args.world_clock_types_output.write_text(generated_world_clock_types)
    args.game_rules_output.write_text(generated_game_rules)
    args.stat_types_output.write_text(generated_stat_types)
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
    args.recipe_wire_types_output.write_text(generated_recipe_wire_types)
    args.item_component_value_types_output.write_text(
        generated_item_component_value_types
    )
    args.consume_effect_types_output.write_text(generated_consume_effect_types)
    args.builtin_enum_component_values_output.write_text(
        generated_builtin_enum_component_values
    )
    args.attribute_modifier_types_output.write_text(generated_attribute_modifier_types)
    args.tropical_fish_patterns_output.write_text(generated_tropical_fish_patterns)
    args.map_colors_output.parent.mkdir(parents=True, exist_ok=True)
    args.map_colors_output.write_text(generated_map_colors)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
