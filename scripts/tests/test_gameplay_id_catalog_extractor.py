from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from gameplay_id_catalog_extractor import (  # noqa: E402
    GameplayIdExtractionError,
    attribute_entries,
    attribute_modifier_type_entries,
    advancement_frame_type_entries,
    client_game_event_type_entries,
    client_command_action_entries,
    boss_bar_style_entries,
    boss_event_operation_type_entries,
    builtin_enum_component_value_entries,
    chat_formatting_entries,
    client_information_option_entries,
    container_input_entries,
    consume_effect_type_entries,
    custom_chat_completions_action_entries,
    difficulty_entries,
    difficulty_out_of_bounds_strategy,
    entity_data_serializer_entries,
    entity_anchor_entries,
    entity_pose_entries,
    equipment_slot_entries,
    game_type_entries,
    game_type_not_set_id,
    filter_mask_type_entries,
    heightmap_type_entries,
    interaction_hand_entries,
    item_component_value_type_entries,
    mob_effect_entries,
    objective_render_type_entries,
    player_info_action_entries,
    player_action_kind_entries,
    player_command_action_entries,
    resource_pack_response_action_entries,
    recipe_wire_type_entries,
    remote_debug_sample_type_entries,
    sound_source_entries,
    tropical_fish_pattern_entries,
    scoreboard_display_slot_entries,
    scoreboard_method_entries,
    scoreboard_team_option_entries,
    waypoint_wire_type_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


class GameplayIdCatalogExtractorTests(unittest.TestCase):
    def test_register_fields_extract_builder_metadata_without_text_matching(self) -> None:
        attributes = parse_source(
            "fixture/Attributes.java",
            b"""
package fixture;
public class Attributes {
  public static final Holder<Attribute> ONE = register(
      "one", new RangedAttribute("attribute.one", 0, 0, 1)
          .setSentiment(Attribute.Sentiment.NEUTRAL)
          .setSyncable(true));
  private static String fake = ".setSyncable(true)";
}
""",
        )
        effects = parse_source(
            "fixture/MobEffects.java",
            b"""
package fixture;
public class MobEffects {
  public static final Holder<MobEffect> ONE = register(
      "one", new MobEffect(MobEffectCategory.HARMFUL, 0));
}
""",
        )
        attribute = attribute_entries(attributes, 1)[0]
        self.assertEqual(
            (attribute.resource_id, attribute.description_key, attribute.sentiment, attribute.syncable),
            ("minecraft:one", "attribute.one", "NEUTRAL", True),
        )
        self.assertEqual(mob_effect_entries(effects, 1)[0].category, "HARMFUL")

    def test_enum_constants_preserve_arguments_and_ignore_nested_enum(self) -> None:
        slots = parse_source(
            "fixture/EquipmentSlot.java",
            b"""
package fixture;
public enum EquipmentSlot {
  MAINHAND(EquipmentSlot.Type.HAND, 0, 0, "mainhand"),
  BODY(EquipmentSlot.Type.ANIMAL_ARMOR, 0, 1, 7, "body");
  enum Type { HAND, ANIMAL_ARMOR; }
}
""",
        )
        poses = parse_source(
            "fixture/Pose.java",
            b"""
package fixture;
public enum Pose { STANDING(0, "standing"), CROUCHING(1, "crouching"); }
""",
        )
        sounds = parse_source(
            "fixture/SoundSource.java",
            b"""
package fixture;
public enum SoundSource { MASTER("master"), MUSIC("music"); }
""",
        )
        extracted_slots = equipment_slot_entries(slots, 2)
        self.assertEqual(
            (extracted_slots[1].count_limit, extracted_slots[1].slot_id), (1, 7)
        )
        self.assertEqual([entry.pose_id for entry in entity_pose_entries(poses, 2)], [0, 1])
        self.assertEqual(
            [entry.serialized_name for entry in sound_source_entries(sounds, 2)],
            ["master", "music"],
        )

    def test_serializer_static_calls_and_explicit_game_event_ids_come_from_cst(self) -> None:
        serializers = parse_source(
            "fixture/EntityDataSerializers.java",
            b"""
package fixture;
public class EntityDataSerializers {
  public static final EntityDataSerializer<Byte> BYTE = simple();
  public static final EntityDataSerializer<Int> INT = simple();
  static { registerSerializer(BYTE); registerSerializer(INT); }
}
""",
        )
        events = parse_source(
            "fixture/ClientboundGameEventPacket.java",
            b"""
package fixture;
public class ClientboundGameEventPacket {
  public static final ClientboundGameEventPacket.Type FIRST =
      new ClientboundGameEventPacket.Type(0);
  public static final ClientboundGameEventPacket.Type SECOND =
      new ClientboundGameEventPacket.Type(1);
  private final ClientboundGameEventPacket.Type event;
  static class Type { Type(int id) {} }
}
""",
        )
        self.assertEqual(
            [entry.constant_name for entry in entity_data_serializer_entries(serializers, 2)],
            ["BYTE", "INT"],
        )
        self.assertEqual(
            [entry.event_id for entry in client_game_event_type_entries(events, 2)], [0, 1]
        )

    def test_nested_heightmap_types_preserve_explicit_ids_keys_and_usage(self) -> None:
        source = parse_source(
            "fixture/Heightmap.java",
            b"""
package fixture;
public class Heightmap {
  enum Types {
    WORLD_SURFACE_WG(0, "WORLD_SURFACE_WG", Usage.WORLDGEN, predicate),
    MOTION_BLOCKING(1, "MOTION_BLOCKING", Usage.CLIENT, state -> state.blocks());
  }
  enum Usage { WORLDGEN, LIVE_WORLD, CLIENT; }
}
""",
        )
        entries = heightmap_type_entries(source, 2)
        self.assertEqual(
            [(entry.type_id, entry.serialization_key, entry.usage) for entry in entries],
            [(0, "WORLD_SURFACE_WG", "WORLDGEN"), (1, "MOTION_BLOCKING", "CLIENT")],
        )

    def test_game_types_preserve_explicit_ids_names_and_nullable_sentinel(self) -> None:
        source = parse_source(
            "fixture/GameType.java",
            b"""
package fixture;
public enum GameType {
  SURVIVAL(0, "survival"), CREATIVE(1, "creative"),
  ADVENTURE(2, "adventure"), SPECTATOR(3, "spectator");
  private static final int NOT_SET = -1;
  GameType(int id, String name) {}
}
""",
        )
        entries = game_type_entries(source)
        self.assertEqual(
            [(entry.game_type_id, entry.serialized_name) for entry in entries],
            [(0, "survival"), (1, "creative"), (2, "adventure"), (3, "spectator")],
        )
        self.assertEqual(game_type_not_set_id(source), -1)

    def test_nested_player_info_actions_preserve_wire_bit_order(self) -> None:
        source = parse_source(
            "fixture/ClientboundPlayerInfoUpdatePacket.java",
            b"""
package fixture;
public class ClientboundPlayerInfoUpdatePacket {
  public enum Action {
    ADD_PLAYER((entry, input) -> entry.read(input), (output, entry) -> output.write(entry)),
    UPDATE_LIST_ORDER((entry, input) -> entry.order = input.readVarInt(), writer),
    UPDATE_HAT(reader, (output, entry) -> output.writeBoolean(entry.showHat()));
    Action(Reader reader, Writer writer) {}
  }
}
""",
        )
        self.assertEqual(
            [entry.constant_name for entry in player_info_action_entries(source, 3)],
            ["ADD_PLAYER", "UPDATE_LIST_ORDER", "UPDATE_HAT"],
        )

    def test_difficulties_preserve_ids_names_and_wrap_strategy(self) -> None:
        source = parse_source(
            "fixture/Difficulty.java",
            b"""
package fixture;
public enum Difficulty {
  PEACEFUL(0, "peaceful"), EASY(1, "easy"),
  NORMAL(2, "normal"), HARD(3, "hard");
  private static final IntFunction<Difficulty> BY_ID = ByIdMap.continuous(
      Difficulty::getId, values(), ByIdMap.OutOfBoundsStrategy.WRAP);
  Difficulty(int id, String key) {}
}
""",
        )
        self.assertEqual(
            [(entry.difficulty_id, entry.serialized_name) for entry in difficulty_entries(source)],
            [(0, "peaceful"), (1, "easy"), (2, "normal"), (3, "hard")],
        )
        self.assertEqual(difficulty_out_of_bounds_strategy(source), "WRAP")

    def test_resource_pack_response_actions_preserve_nested_enum_order(self) -> None:
        source = parse_source(
            "fixture/ServerboundResourcePackPacket.java",
            b"""
package fixture;
public record ServerboundResourcePackPacket() {
  public enum Action {
    SUCCESSFULLY_LOADED, DECLINED, FAILED_DOWNLOAD, ACCEPTED,
    DOWNLOADED, INVALID_URL, FAILED_RELOAD, DISCARDED;
  }
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal)
                for entry in resource_pack_response_action_entries(source)
            ],
            [
                ("SUCCESSFULLY_LOADED", 0),
                ("DECLINED", 1),
                ("FAILED_DOWNLOAD", 2),
                ("ACCEPTED", 3),
                ("DOWNLOADED", 4),
                ("INVALID_URL", 5),
                ("FAILED_RELOAD", 6),
                ("DISCARDED", 7),
            ],
        )

    def test_custom_chat_completions_actions_preserve_nested_enum_order(self) -> None:
        source = parse_source(
            "fixture/ClientboundCustomChatCompletionsPacket.java",
            b"""
package fixture;
public record ClientboundCustomChatCompletionsPacket() {
  public enum Action { ADD, REMOVE, SET; }
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal)
                for entry in custom_chat_completions_action_entries(source)
            ],
            [("ADD", 0), ("REMOVE", 1), ("SET", 2)],
        )

    def test_client_command_actions_preserve_nested_enum_order(self) -> None:
        source = parse_source(
            "fixture/ServerboundClientCommandPacket.java",
            b"""
package fixture;
public class ServerboundClientCommandPacket {
  public enum Action { PERFORM_RESPAWN, REQUEST_STATS, REQUEST_GAMERULE_VALUES; }
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal)
                for entry in client_command_action_entries(source)
            ],
            [
                ("PERFORM_RESPAWN", 0),
                ("REQUEST_STATS", 1),
                ("REQUEST_GAMERULE_VALUES", 2),
            ],
        )

    def test_player_action_kinds_preserve_nested_enum_order(self) -> None:
        source = parse_source(
            "fixture/ServerboundPlayerActionPacket.java",
            b"""
package fixture;
public class ServerboundPlayerActionPacket {
  public enum Action {
    START_DESTROY_BLOCK, ABORT_DESTROY_BLOCK, STOP_DESTROY_BLOCK,
    DROP_ALL_ITEMS, DROP_ITEM, RELEASE_USE_ITEM,
    SWAP_ITEM_WITH_OFFHAND, STAB;
  }
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal)
                for entry in player_action_kind_entries(source)
            ],
            [
                ("START_DESTROY_BLOCK", 0),
                ("ABORT_DESTROY_BLOCK", 1),
                ("STOP_DESTROY_BLOCK", 2),
                ("DROP_ALL_ITEMS", 3),
                ("DROP_ITEM", 4),
                ("RELEASE_USE_ITEM", 5),
                ("SWAP_ITEM_WITH_OFFHAND", 6),
                ("STAB", 7),
            ],
        )

    def test_player_command_actions_preserve_nested_enum_order(self) -> None:
        source = parse_source(
            "fixture/ServerboundPlayerCommandPacket.java",
            b"""
package fixture;
public class ServerboundPlayerCommandPacket {
  public enum Action {
    STOP_SLEEPING, START_SPRINTING, STOP_SPRINTING,
    START_RIDING_JUMP, STOP_RIDING_JUMP, OPEN_INVENTORY,
    START_FALL_FLYING;
  }
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal)
                for entry in player_command_action_entries(source)
            ],
            [
                ("STOP_SLEEPING", 0),
                ("START_SPRINTING", 1),
                ("STOP_SPRINTING", 2),
                ("START_RIDING_JUMP", 3),
                ("STOP_RIDING_JUMP", 4),
                ("OPEN_INVENTORY", 5),
                ("START_FALL_FLYING", 6),
            ],
        )

    def test_entity_anchors_preserve_nested_enum_order_and_names(self) -> None:
        source = parse_source(
            "fixture/EntityAnchorArgument.java",
            b"""
package fixture;
public class EntityAnchorArgument {
  public enum Anchor {
    FEET("feet", (p, e) -> p),
    EYES("eyes", (p, e) -> new Vec3(p.x, p.y + e.eyeHeight(), p.z));
  }
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal, entry.serialized_name)
                for entry in entity_anchor_entries(source)
            ],
            [("FEET", 0, "feet"), ("EYES", 1, "eyes")],
        )

    def test_remote_debug_sample_types_preserve_top_level_enum_order(self) -> None:
        source = parse_source(
            "fixture/RemoteDebugSampleType.java",
            b"""
package fixture;
public enum RemoteDebugSampleType {
  TICK_TIME(DebugSubscriptions.DEDICATED_SERVER_TICK_TIME);
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal)
                for entry in remote_debug_sample_type_entries(source)
            ],
            [("TICK_TIME", 0)],
        )

    def test_objective_render_types_preserve_nested_order_and_names(self) -> None:
        source = parse_source(
            "fixture/ObjectiveCriteria.java",
            b"""
package fixture;
public class ObjectiveCriteria {
  public enum RenderType {
    INTEGER("integer"), HEARTS("hearts");
  }
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal, entry.serialized_name)
                for entry in objective_render_type_entries(source)
            ],
            [("INTEGER", 0, "integer"), ("HEARTS", 1, "hearts")],
        )

    def test_advancement_frame_types_preserve_top_level_order_and_names(self) -> None:
        source = parse_source(
            "fixture/AdvancementType.java",
            b"""
package fixture;
public enum AdvancementType {
  TASK("task", ChatFormatting.GREEN),
  CHALLENGE("challenge", ChatFormatting.DARK_PURPLE),
  GOAL("goal", ChatFormatting.GREEN);
}
""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal, entry.serialized_name)
                for entry in advancement_frame_type_entries(source)
            ],
            [
                ("TASK", 0, "task"),
                ("CHALLENGE", 1, "challenge"),
                ("GOAL", 2, "goal"),
            ],
        )

    def test_boss_bar_styles_preserve_nested_order_and_names(self) -> None:
        source = parse_source(
            "fixture/BossEvent.java",
            b"""
package fixture;
public class BossEvent {
  public enum BossBarColor {
    PINK("pink", ChatFormatting.RED), BLUE("blue", ChatFormatting.BLUE),
    RED("red", ChatFormatting.DARK_RED), GREEN("green", ChatFormatting.GREEN),
    YELLOW("yellow", ChatFormatting.YELLOW), PURPLE("purple", ChatFormatting.DARK_BLUE),
    WHITE("white", ChatFormatting.WHITE);
  }
  public enum BossBarOverlay {
    PROGRESS("progress"), NOTCHED_6("notched_6"), NOTCHED_10("notched_10"),
    NOTCHED_12("notched_12"), NOTCHED_20("notched_20");
  }
}
""",
        )
        colors, overlays = boss_bar_style_entries(source)
        self.assertEqual(
            [(entry.constant_name, entry.ordinal, entry.serialized_name) for entry in colors],
            [
                ("PINK", 0, "pink"), ("BLUE", 1, "blue"), ("RED", 2, "red"),
                ("GREEN", 3, "green"), ("YELLOW", 4, "yellow"),
                ("PURPLE", 5, "purple"), ("WHITE", 6, "white"),
            ],
        )
        self.assertEqual(
            [(entry.constant_name, entry.ordinal, entry.serialized_name) for entry in overlays],
            [
                ("PROGRESS", 0, "progress"), ("NOTCHED_6", 1, "notched_6"),
                ("NOTCHED_10", 2, "notched_10"), ("NOTCHED_12", 3, "notched_12"),
                ("NOTCHED_20", 4, "notched_20"),
            ],
        )

    def test_scoreboard_display_slots_preserve_ids_names_and_zero_fallback(self) -> None:
        source = parse_source(
            "fixture/DisplaySlot.java",
            b"""
package fixture;
public enum DisplaySlot {
  LIST(0, "list"), SIDEBAR(1, "sidebar"), BELOW_NAME(2, "below_name"),
  TEAM_BLACK(3, "sidebar.team.black"), TEAM_DARK_BLUE(4, "sidebar.team.dark_blue"),
  TEAM_DARK_GREEN(5, "sidebar.team.dark_green"), TEAM_DARK_AQUA(6, "sidebar.team.dark_aqua"),
  TEAM_DARK_RED(7, "sidebar.team.dark_red"), TEAM_DARK_PURPLE(8, "sidebar.team.dark_purple"),
  TEAM_GOLD(9, "sidebar.team.gold"), TEAM_GRAY(10, "sidebar.team.gray"),
  TEAM_DARK_GRAY(11, "sidebar.team.dark_gray"), TEAM_BLUE(12, "sidebar.team.blue"),
  TEAM_GREEN(13, "sidebar.team.green"), TEAM_AQUA(14, "sidebar.team.aqua"),
  TEAM_RED(15, "sidebar.team.red"), TEAM_LIGHT_PURPLE(16, "sidebar.team.light_purple"),
  TEAM_YELLOW(17, "sidebar.team.yellow"), TEAM_WHITE(18, "sidebar.team.white");
  static final IntFunction<DisplaySlot> BY_ID = ByIdMap.continuous(
      DisplaySlot::id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
}
""",
        )
        entries, strategy = scoreboard_display_slot_entries(source)
        self.assertEqual(strategy, "ZERO")
        self.assertEqual(len(entries), 19)
        self.assertEqual(
            (entries[0].constant_name, entries[0].slot_id, entries[0].serialized_name),
            ("LIST", 0, "list"),
        )
        self.assertEqual(
            (entries[-1].constant_name, entries[-1].slot_id, entries[-1].serialized_name),
            ("TEAM_WHITE", 18, "sidebar.team.white"),
        )

    def test_scoreboard_team_options_preserve_ids_names_and_zero_fallback(self) -> None:
        source = parse_source(
            "fixture/Team.java",
            b"""
package fixture;
public abstract class Team {
  public enum CollisionRule {
    ALWAYS("always", 0), NEVER("never", 1),
    PUSH_OTHER_TEAMS("pushOtherTeams", 2), PUSH_OWN_TEAM("pushOwnTeam", 3);
    private static final IntFunction<CollisionRule> BY_ID = ByIdMap.continuous(
        value -> value.id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
  }
  public enum Visibility {
    ALWAYS("always", 0), NEVER("never", 1),
    HIDE_FOR_OTHER_TEAMS("hideForOtherTeams", 2),
    HIDE_FOR_OWN_TEAM("hideForOwnTeam", 3);
    private static final IntFunction<Visibility> BY_ID = ByIdMap.continuous(
        value -> value.id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
  }
}
""",
        )
        visibility, visibility_strategy, collision, collision_strategy = (
            scoreboard_team_option_entries(source)
        )
        self.assertEqual((visibility_strategy, collision_strategy), ("ZERO", "ZERO"))
        self.assertEqual(
            [
                (entry.constant_name, entry.option_id, entry.serialized_name)
                for entry in visibility
            ],
            [
                ("ALWAYS", 0, "always"),
                ("NEVER", 1, "never"),
                ("HIDE_FOR_OTHER_TEAMS", 2, "hideForOtherTeams"),
                ("HIDE_FOR_OWN_TEAM", 3, "hideForOwnTeam"),
            ],
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.option_id, entry.serialized_name)
                for entry in collision
            ],
            [
                ("ALWAYS", 0, "always"),
                ("NEVER", 1, "never"),
                ("PUSH_OTHER_TEAMS", 2, "pushOtherTeams"),
                ("PUSH_OWN_TEAM", 3, "pushOwnTeam"),
            ],
        )

    def test_chat_formatting_preserves_ordinals_codes_ids_formats_and_colors(self) -> None:
        source = parse_source(
            "fixture/ChatFormatting.java",
            b"""
package fixture;
public enum ChatFormatting {
  BLACK("BLACK", '0', 0, 0), DARK_BLUE("DARK_BLUE", '1', 1, 170),
  DARK_GREEN("DARK_GREEN", '2', 2, 43520), DARK_AQUA("DARK_AQUA", '3', 3, 43690),
  DARK_RED("DARK_RED", '4', 4, 11141120), DARK_PURPLE("DARK_PURPLE", '5', 5, 11141290),
  GOLD("GOLD", '6', 6, 16755200), GRAY("GRAY", '7', 7, 11184810),
  DARK_GRAY("DARK_GRAY", '8', 8, 5592405), BLUE("BLUE", '9', 9, 5592575),
  GREEN("GREEN", 'a', 10, 5635925), AQUA("AQUA", 'b', 11, 5636095),
  RED("RED", 'c', 12, 16733525), LIGHT_PURPLE("LIGHT_PURPLE", 'd', 13, 16733695),
  YELLOW("YELLOW", 'e', 14, 16777045), WHITE("WHITE", 'f', 15, 16777215),
  OBFUSCATED("OBFUSCATED", 'k', true), BOLD("BOLD", 'l', true),
  STRIKETHROUGH("STRIKETHROUGH", 'm', true), UNDERLINE("UNDERLINE", 'n', true),
  ITALIC("ITALIC", 'o', true), RESET("RESET", 'r', -1, null);
}
""",
        )
        entries = chat_formatting_entries(source)
        self.assertEqual(len(entries), 22)
        self.assertEqual(
            entries[0],
            type(entries[0])("BLACK", 0, "black", "0", 0, False, 0),
        )
        self.assertEqual(
            entries[12],
            type(entries[12])("RED", 12, "red", "c", 12, False, 16733525),
        )
        self.assertEqual(
            entries[16],
            type(entries[16])("OBFUSCATED", 16, "obfuscated", "k", -1, True, None),
        )
        self.assertEqual(
            entries[-1],
            type(entries[-1])("RESET", 21, "reset", "r", -1, False, None),
        )

    def test_scoreboard_methods_preserve_explicit_packet_ids(self) -> None:
        objective = parse_source(
            "fixture/ClientboundSetObjectivePacket.java",
            b"""
package fixture;
public class ClientboundSetObjectivePacket {
  public static final int METHOD_ADD = 0;
  public static final int METHOD_REMOVE = 1;
  public static final int METHOD_CHANGE = 2;
}
""",
        )
        team = parse_source(
            "fixture/ClientboundSetPlayerTeamPacket.java",
            b"""
package fixture;
public class ClientboundSetPlayerTeamPacket {
  private static final int METHOD_ADD = 0;
  private static final int METHOD_REMOVE = 1;
  private static final int METHOD_CHANGE = 2;
  private static final int METHOD_JOIN = 3;
  private static final int METHOD_LEAVE = 4;
}
""",
        )
        objective_entries, team_entries = scoreboard_method_entries(objective, team)
        self.assertEqual(
            [(entry.constant_name, entry.method_id) for entry in objective_entries],
            [("ADD", 0), ("REMOVE", 1), ("CHANGE", 2)],
        )
        self.assertEqual(
            [(entry.constant_name, entry.method_id) for entry in team_entries],
            [("ADD", 0), ("REMOVE", 1), ("CHANGE", 2), ("JOIN", 3), ("LEAVE", 4)],
        )

    def test_boss_event_operation_types_preserve_nested_enum_order(self) -> None:
        source = parse_source(
            "fixture/ClientboundBossEventPacket.java",
            b"""
package fixture;
public class ClientboundBossEventPacket {
  private enum OperationType {
    ADD(AddOperation::new),
    REMOVE(input -> REMOVE_OPERATION),
    UPDATE_PROGRESS(UpdateProgressOperation::new),
    UPDATE_NAME(UpdateNameOperation::new),
    UPDATE_STYLE(UpdateStyleOperation::new),
    UPDATE_PROPERTIES(UpdatePropertiesOperation::new);
  }
}
""",
        )
        entries = boss_event_operation_type_entries(source)
        self.assertEqual(
            [(entry.constant_name, entry.ordinal) for entry in entries],
            [
                ("ADD", 0),
                ("REMOVE", 1),
                ("UPDATE_PROGRESS", 2),
                ("UPDATE_NAME", 3),
                ("UPDATE_STYLE", 4),
                ("UPDATE_PROPERTIES", 5),
            ],
        )

    def test_filter_mask_types_preserve_nested_enum_order_and_names(self) -> None:
        source = parse_source(
            "fixture/FilterMask.java",
            b"""
package fixture;
public class FilterMask {
  private enum Type {
    PASS_THROUGH("pass_through", () -> PASS_THROUGH_CODEC),
    FULLY_FILTERED("fully_filtered", () -> FULLY_FILTERED_CODEC),
    PARTIALLY_FILTERED("partially_filtered", () -> PARTIALLY_FILTERED_CODEC);
  }
}
""",
        )
        entries = filter_mask_type_entries(source)
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal, entry.serialized_name)
                for entry in entries
            ],
            [
                ("PASS_THROUGH", 0, "pass_through"),
                ("FULLY_FILTERED", 1, "fully_filtered"),
                ("PARTIALLY_FILTERED", 2, "partially_filtered"),
            ],
        )

    def test_waypoint_wire_types_preserve_order_names_and_wrap_fallback(self) -> None:
        packet = parse_source(
            "fixture/ClientboundTrackedWaypointPacket.java",
            b"""
package fixture;
public record ClientboundTrackedWaypointPacket() {
  private enum Operation {
    TRACK(Manager::track), UNTRACK(Manager::untrack), UPDATE(Manager::update);
    public static final IntFunction<Operation> BY_ID = ByIdMap.continuous(
        Enum::ordinal, values(), ByIdMap.OutOfBoundsStrategy.WRAP);
  }
}
""",
        )
        waypoint = parse_source(
            "fixture/TrackedWaypoint.java",
            b"""
package fixture;
public abstract class TrackedWaypoint {
  private enum Type {
    EMPTY(EmptyWaypoint::new), VEC3I(Vec3iWaypoint::new),
    CHUNK(ChunkWaypoint::new), AZIMUTH(AzimuthWaypoint::new);
  }
}
""",
        )
        operations, strategy, data_types = waypoint_wire_type_entries(packet, waypoint)
        self.assertEqual(strategy, "WRAP")
        self.assertEqual(
            [
                (entry.constant_name, entry.ordinal, entry.serialized_name)
                for entry in operations
            ],
            [("TRACK", 0, "track"), ("UNTRACK", 1, "untrack"), ("UPDATE", 2, "update")],
        )
        self.assertEqual(
            [(entry.constant_name, entry.ordinal) for entry in data_types],
            [("EMPTY", 0), ("VEC3I", 1), ("CHUNK", 2), ("AZIMUTH", 3)],
        )

    def test_interaction_hands_preserve_explicit_ids_and_zero_fallback(self) -> None:
        source = parse_source(
            "fixture/InteractionHand.java",
            b"""
package fixture;
public enum InteractionHand {
  MAIN_HAND(0), OFF_HAND(1);
  private static final IntFunction<InteractionHand> BY_ID = ByIdMap.continuous(
      hand -> hand.id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
  private final int id;
  InteractionHand(final int id) { this.id = id; }
}
""",
        )
        entries, strategy = interaction_hand_entries(source)
        self.assertEqual(strategy, "ZERO")
        self.assertEqual(
            [(entry.constant_name, entry.hand_id) for entry in entries],
            [("MAIN_HAND", 0), ("OFF_HAND", 1)],
        )

    def test_client_information_options_preserve_explicit_ids_and_order(self) -> None:
        chat = parse_source(
            "fixture/ChatVisiblity.java",
            b"""
package fixture;
public enum ChatVisiblity {
  FULL(0, "full"), SYSTEM(1, "system"), HIDDEN(2, "hidden");
}
""",
        )
        hand = parse_source(
            "fixture/HumanoidArm.java",
            b"""
package fixture;
public enum HumanoidArm {
  LEFT(0, "left", "left.key"), RIGHT(1, "right", "right.key");
}
""",
        )
        particles = parse_source(
            "fixture/ParticleStatus.java",
            b"""
package fixture;
public enum ParticleStatus {
  ALL(0, "all"), DECREASED(1, "decreased"), MINIMAL(2, "minimal");
}
""",
        )
        chat_entries, hand_entries, particle_entries = (
            client_information_option_entries(chat, hand, particles)
        )
        self.assertEqual(
            [(entry.constant_name, entry.option_id) for entry in chat_entries],
            [("FULL", 0), ("SYSTEM", 1), ("HIDDEN", 2)],
        )
        self.assertEqual(
            [(entry.constant_name, entry.option_id) for entry in hand_entries],
            [("LEFT", 0), ("RIGHT", 1)],
        )
        self.assertEqual(
            [(entry.constant_name, entry.option_id) for entry in particle_entries],
            [("ALL", 0), ("DECREASED", 1), ("MINIMAL", 2)],
        )

    def test_container_inputs_preserve_explicit_ids_and_zero_fallback(self) -> None:
        source = parse_source(
            "fixture/ContainerInput.java",
            b"""
package fixture;
public enum ContainerInput {
  PICKUP(0), QUICK_MOVE(1), SWAP(2), CLONE(3), THROW(4), QUICK_CRAFT(5), PICKUP_ALL(6);
  private static final IntFunction<ContainerInput> BY_ID = ByIdMap.continuous(
      ContainerInput::id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
}
""",
        )
        entries, strategy = container_input_entries(source)
        self.assertEqual(strategy, "ZERO")
        self.assertEqual(
            [(entry.constant_name, entry.input_id) for entry in entries],
            [
                ("PICKUP", 0),
                ("QUICK_MOVE", 1),
                ("SWAP", 2),
                ("CLONE", 3),
                ("THROW", 4),
                ("QUICK_CRAFT", 5),
                ("PICKUP_ALL", 6),
            ],
        )

    def test_recipe_wire_types_preserve_enum_and_bootstrap_order(self) -> None:
        books = parse_source(
            "fixture/RecipeBookType.java",
            b"""
package fixture;
public enum RecipeBookType { CRAFTING, FURNACE, BLAST_FURNACE, SMOKER; }
""",
        )
        displays = parse_source(
            "fixture/RecipeDisplays.java",
            b"""
package fixture;
public class RecipeDisplays {
  public static Type<?> bootstrap(Registry<Type<?>> registry) {
    Registry.register(registry, "crafting_shapeless", ShapelessCraftingRecipeDisplay.TYPE);
    Registry.register(registry, "crafting_shaped", ShapedCraftingRecipeDisplay.TYPE);
    Registry.register(registry, "furnace", FurnaceRecipeDisplay.TYPE);
    Registry.register(registry, "stonecutter", StonecutterRecipeDisplay.TYPE);
    return Registry.register(registry, "smithing", SmithingRecipeDisplay.TYPE);
  }
}
""",
        )
        book_entries, display_entries = recipe_wire_type_entries(books, displays)
        self.assertEqual(
            [(entry.constant_name, entry.ordinal) for entry in book_entries],
            [("CRAFTING", 0), ("FURNACE", 1), ("BLAST_FURNACE", 2), ("SMOKER", 3)],
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.type_id, entry.resource_id)
                for entry in display_entries
            ],
            [
                ("CRAFTING_SHAPELESS", 0, "minecraft:crafting_shapeless"),
                ("CRAFTING_SHAPED", 1, "minecraft:crafting_shaped"),
                ("FURNACE", 2, "minecraft:furnace"),
                ("STONECUTTER", 3, "minecraft:stonecutter"),
                ("SMITHING", 4, "minecraft:smithing"),
            ],
        )

    def test_item_component_value_types_preserve_ids_names_and_zero_fallback(self) -> None:
        item_uses = parse_source(
            "fixture/ItemUseAnimation.java",
            b"""
package fixture;
public enum ItemUseAnimation {
  NONE(0, "none"), EAT(1, "eat", true), DRINK(2, "drink", true),
  BLOCK(3, "block"), BOW(4, "bow"), TRIDENT(5, "trident"),
  CROSSBOW(6, "crossbow"), SPYGLASS(7, "spyglass"), TOOT_HORN(8, "toot_horn"),
  BRUSH(9, "brush"), BUNDLE(10, "bundle"), SPEAR(11, "spear", true);
  private static final IntFunction<ItemUseAnimation> BY_ID = ByIdMap.continuous(
      ItemUseAnimation::getId, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
}
""",
        )
        swings = parse_source(
            "fixture/SwingAnimationType.java",
            b"""
package fixture;
public enum SwingAnimationType {
  NONE(0, "none"), WHACK(1, "whack"), STAB(2, "stab");
  private static final IntFunction<SwingAnimationType> BY_ID = ByIdMap.continuous(
      SwingAnimationType::getId, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
}
""",
        )
        rarities = parse_source(
            "fixture/Rarity.java",
            b"""
package fixture;
public enum Rarity {
  COMMON(0, "common", ChatFormatting.WHITE),
  UNCOMMON(1, "uncommon", ChatFormatting.YELLOW),
  RARE(2, "rare", ChatFormatting.AQUA),
  EPIC(3, "epic", ChatFormatting.LIGHT_PURPLE);
  public static final IntFunction<Rarity> BY_ID = ByIdMap.continuous(
      rarity -> rarity.id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
}
""",
        )
        map_post = parse_source(
            "fixture/MapPostProcessing.java",
            b"""
package fixture;
public enum MapPostProcessing {
  LOCK(0), SCALE(1);
  public static final IntFunction<MapPostProcessing> ID_MAP = ByIdMap.continuous(
      MapPostProcessing::id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
}
""",
        )
        firework = parse_source(
            "fixture/FireworkExplosion.java",
            b"""
package fixture;
public record FireworkExplosion() {
  public enum Shape {
    SMALL_BALL(0, "small_ball"), LARGE_BALL(1, "large_ball"), STAR(2, "star"),
    CREEPER(3, "creeper"), BURST(4, "burst");
    private static final IntFunction<Shape> BY_ID = ByIdMap.continuous(
        Shape::getId, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
  }
}
""",
        )
        item_entries, swing_entries, rarity_entries, map_entries, shape_entries = (
            item_component_value_type_entries(
                item_uses, swings, rarities, map_post, firework
            )
        )
        self.assertEqual((len(item_entries), len(swing_entries)), (12, 3))
        self.assertEqual(
            (len(rarity_entries), len(map_entries), len(shape_entries)),
            (4, 2, 5),
        )
        self.assertEqual(item_entries[8].serialized_name, "toot_horn")
        self.assertEqual(swing_entries[2].serialized_name, "stab")
        self.assertEqual(rarity_entries[3].serialized_name, "epic")
        self.assertEqual(
            [(entry.constant_name, entry.value_id) for entry in map_entries],
            [("LOCK", 0), ("SCALE", 1)],
        )
        self.assertEqual(shape_entries[1].serialized_name, "large_ball")

    def test_consume_effect_types_preserve_nested_register_order(self) -> None:
        source = parse_source(
            "fixture/ConsumeEffect.java",
            b"""
package fixture;
public interface ConsumeEffect {
  record Type<T>() {
    public static final ConsumeEffect.Type<Apply> APPLY_EFFECTS = register(
        "apply_effects", Apply.CODEC, Apply.STREAM_CODEC);
    public static final ConsumeEffect.Type<Remove> REMOVE_EFFECTS = register(
        "remove_effects", Remove.CODEC, Remove.STREAM_CODEC);
    public static final ConsumeEffect.Type<Clear> CLEAR_ALL_EFFECTS = register(
        "clear_all_effects", Clear.CODEC, Clear.STREAM_CODEC);
    public static final ConsumeEffect.Type<Teleport> TELEPORT_RANDOMLY = register(
        "teleport_randomly", Teleport.CODEC, Teleport.STREAM_CODEC);
    public static final ConsumeEffect.Type<Sound> PLAY_SOUND = register(
        "play_sound", Sound.CODEC, Sound.STREAM_CODEC);
  }
}
""",
        )
        entries = consume_effect_type_entries(source)
        self.assertEqual(
            [
                (entry.constant_name, entry.type_id, entry.resource_id)
                for entry in entries
            ],
            [
                ("APPLY_EFFECTS", 0, "minecraft:apply_effects"),
                ("REMOVE_EFFECTS", 1, "minecraft:remove_effects"),
                ("CLEAR_ALL_EFFECTS", 2, "minecraft:clear_all_effects"),
                ("TELEPORT_RANDOMLY", 3, "minecraft:teleport_randomly"),
                ("PLAY_SOUND", 4, "minecraft:play_sound"),
            ],
        )

    def test_builtin_enum_components_extract_values_strategies_and_codec_owners(self) -> None:
        def source(path: str, declaration: str):
            return parse_source(path, ("package fixture; " + declaration).encode())

        dye = source(
            "fixture/DyeColor.java",
            """enum DyeColor { WHITE(0, \"white\"), ORANGE(1, \"orange\"), MAGENTA(2, \"magenta\"), LIGHT_BLUE(3, \"light_blue\"), YELLOW(4, \"yellow\"), LIME(5, \"lime\"), PINK(6, \"pink\"), GRAY(7, \"gray\"), LIGHT_GRAY(8, \"light_gray\"), CYAN(9, \"cyan\"), PURPLE(10, \"purple\"), BLUE(11, \"blue\"), BROWN(12, \"brown\"), GREEN(13, \"green\"), RED(14, \"red\"), BLACK(15, \"black\"); static final Object BY_ID = ByIdMap.continuous(DyeColor::id, values(), ByIdMap.OutOfBoundsStrategy.ZERO); DyeColor(int id, String name) {} }""",
        )

        def nested(path: str, owner: str, constants: str, strategy: str) -> object:
            return source(
                path,
                f"class {owner} {{ enum Variant {{ {constants}; static final Object BY_ID = ByIdMap.continuous(Variant::id, values(), ByIdMap.OutOfBoundsStrategy.{strategy}); Variant(Object... values) {{}} }} }}",
            )

        fox = nested("fixture/Fox.java", "Fox", 'RED(0, "red"), SNOW(1, "snow")', "ZERO")
        salmon = nested("fixture/Salmon.java", "Salmon", 'SMALL("small", 0), MEDIUM("medium", 1), LARGE("large", 2)', "CLAMP")
        parrot = nested("fixture/Parrot.java", "Parrot", 'RED_BLUE(0, "red_blue"), BLUE(1, "blue"), GREEN(2, "green"), YELLOW_BLUE(3, "yellow_blue"), GRAY(4, "gray")', "CLAMP")
        mushroom = nested("fixture/MushroomCow.java", "MushroomCow", 'RED("red", 0), BROWN("brown", 1)', "CLAMP")
        rabbit = source(
            "fixture/Rabbit.java",
            """class Rabbit { enum Variant { BROWN(0, \"brown\"), WHITE(1, \"white\"), BLACK(2, \"black\"), WHITE_SPLOTCHED(3, \"white_splotched\"), GOLD(4, \"gold\"), SALT(5, \"salt\"), EVIL(99, \"evil\"); static final Variant DEFAULT = BROWN; static final Object BY_ID = ByIdMap.sparse(Variant::id, values(), DEFAULT); Variant(int id, String name) {} } }""",
        )
        horse = source(
            "fixture/Variant.java",
            """enum Variant { WHITE(0, \"white\"), CREAMY(1, \"creamy\"), CHESTNUT(2, \"chestnut\"), BROWN(3, \"brown\"), BLACK(4, \"black\"), GRAY(5, \"gray\"), DARK_BROWN(6, \"dark_brown\"); static final Object BY_ID = ByIdMap.continuous(Variant::id, values(), ByIdMap.OutOfBoundsStrategy.WRAP); Variant(int id, String name) {} }""",
        )
        llama = nested("fixture/Llama.java", "Llama", 'CREAMY(0, "creamy"), WHITE(1, "white"), BROWN(2, "brown"), GRAY(3, "gray")', "CLAMP")
        axolotl = nested("fixture/Axolotl.java", "Axolotl", 'LUCY(0, "lucy"), WILD(1, "wild"), GOLD(2, "gold"), CYAN(3, "cyan"), BLUE(4, "blue")', "ZERO")
        component_types = {
            "DYE": "DyeColor", "BASE_COLOR": "DyeColor", "WOLF_COLLAR": "DyeColor", "FOX_VARIANT": "Fox.Variant", "SALMON_SIZE": "Salmon.Variant", "PARROT_VARIANT": "Parrot.Variant", "TROPICAL_FISH_BASE_COLOR": "DyeColor", "TROPICAL_FISH_PATTERN_COLOR": "DyeColor", "MOOSHROOM_VARIANT": "MushroomCow.Variant", "RABBIT_VARIANT": "Rabbit.Variant", "HORSE_VARIANT": "Variant", "LLAMA_VARIANT": "Llama.Variant", "AXOLOTL_VARIANT": "Axolotl.Variant", "CAT_COLLAR": "DyeColor", "SHEEP_COLOR": "DyeColor", "SHULKER_COLOR": "DyeColor",
        }
        fields = " ".join(
            f'static final DataComponentType<{java_type}> {name} = register("{name.lower()}", b -> b.persistent({java_type}.CODEC).networkSynchronized({java_type}.STREAM_CODEC));'
            for name, java_type in component_types.items()
        )
        components_source = source("fixture/DataComponents.java", f"class DataComponents {{ {fields} }}")
        value_types, components = builtin_enum_component_value_entries(
            components_source, dye, fox, salmon, parrot, mushroom, rabbit, horse, llama, axolotl
        )
        self.assertEqual([entry.strategy for entry in value_types], ["ZERO", "ZERO", "CLAMP", "CLAMP", "CLAMP", "SPARSE", "WRAP", "CLAMP", "ZERO"])
        self.assertEqual(value_types[5].values[-1].value_id, 99)
        self.assertEqual(len(components), 16)
        self.assertEqual(components[0].value_type_constant_name, "DYE_COLOR")
        mismatched = source(
            "fixture/DataComponents.java",
            f"class DataComponents {{ {fields.replace('Fox.Variant.STREAM_CODEC', 'Salmon.Variant.STREAM_CODEC')} }}",
        )
        with self.assertRaisesRegex(
            GameplayIdExtractionError, "FOX_VARIANT codec owner does not match"
        ):
            builtin_enum_component_value_entries(
                mismatched,
                dye,
                fox,
                salmon,
                parrot,
                mushroom,
                rabbit,
                horse,
                llama,
                axolotl,
            )

    def test_attribute_modifier_types_preserve_ids_names_codecs_and_zero_fallback(self) -> None:
        modifier = parse_source(
            "fixture/AttributeModifier.java",
            b"""package fixture;
record AttributeModifier() {
  enum Operation {
    ADD_VALUE("add_value", 0),
    ADD_MULTIPLIED_BASE("add_multiplied_base", 1),
    ADD_MULTIPLIED_TOTAL("add_multiplied_total", 2);
    static final Object BY_ID = ByIdMap.continuous(Operation::id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
    Operation(String name, int id) {}
  }
}
""",
        )
        displays = parse_source(
            "fixture/ItemAttributeModifiers.java",
            b"""package fixture;
record ItemAttributeModifiers() {
  interface Display {
    record Default() {} record Hidden() {} record OverrideText() {}
    enum Type {
      DEFAULT("default", 0, ItemAttributeModifiers.Display.Default.CODEC, ItemAttributeModifiers.Display.Default.STREAM_CODEC),
      HIDDEN("hidden", 1, ItemAttributeModifiers.Display.Hidden.CODEC, ItemAttributeModifiers.Display.Hidden.STREAM_CODEC),
      OVERRIDE("override", 2, ItemAttributeModifiers.Display.OverrideText.CODEC, ItemAttributeModifiers.Display.OverrideText.STREAM_CODEC);
      static final Object BY_ID = ByIdMap.continuous(Type::id, values(), ByIdMap.OutOfBoundsStrategy.ZERO);
      Type(Object... values) {}
    }
  }
}
""",
        )
        operations, display_types = attribute_modifier_type_entries(modifier, displays)
        self.assertEqual(
            [(entry.value_id, entry.serialized_name) for entry in operations],
            [(0, "add_value"), (1, "add_multiplied_base"), (2, "add_multiplied_total")],
        )
        self.assertEqual(
            [(entry.value_id, entry.serialized_name) for entry in display_types],
            [(0, "default"), (1, "hidden"), (2, "override")],
        )

    def test_tropical_fish_patterns_preserve_sparse_packed_ids(self) -> None:
        common_variants = ", ".join(
            [
                "new TropicalFish.Variant(TropicalFish.Pattern.STRIPEY, DyeColor.ORANGE, DyeColor.GRAY)"
            ]
            * 22
        )
        source = parse_source(
            "fixture/TropicalFish.java",
            """package fixture;
class TropicalFish {
 static final Object COMMON_VARIANTS = List.of(__COMMON_VARIANTS__);
 enum Base { SMALL(0), LARGE(1); Base(int id) {} }
 enum Pattern {
  KOB("kob", Base.SMALL, 0), SUNSTREAK("sunstreak", Base.SMALL, 1),
  SNOOPER("snooper", Base.SMALL, 2), DASHER("dasher", Base.SMALL, 3),
  BRINELY("brinely", Base.SMALL, 4), SPOTTY("spotty", Base.SMALL, 5),
  FLOPPER("flopper", Base.LARGE, 0), STRIPEY("stripey", Base.LARGE, 1),
  GLITTER("glitter", Base.LARGE, 2), BLOCKFISH("blockfish", Base.LARGE, 3),
  BETTY("betty", Base.LARGE, 4), CLAYFISH("clayfish", Base.LARGE, 5);
  static final Object BY_ID = ByIdMap.sparse(Pattern::getPackedId, values(), KOB);
  Pattern(String name, Base base, int index) {}
 }
}
""".replace("__COMMON_VARIANTS__", common_variants).encode(),
        )
        dye_colors = parse_source(
            "fixture/DyeColor.java",
            b"""package fixture;
enum DyeColor { WHITE(0), ORANGE(1), MAGENTA(2), LIGHT_BLUE(3), YELLOW(4), LIME(5), PINK(6), GRAY(7), LIGHT_GRAY(8), CYAN(9), PURPLE(10), BLUE(11), BROWN(12), GREEN(13), RED(14), BLACK(15); DyeColor(int id) {} }
""",
        )
        entries, common = tropical_fish_pattern_entries(source, dye_colors)
        self.assertEqual(entries[0].packed_id, 0)
        self.assertEqual(entries[5].packed_id, 1280)
        self.assertEqual(entries[6].packed_id, 1)
        self.assertEqual(entries[-1].packed_id, 1281)
        self.assertEqual(entries[-1].serialized_name, "clayfish")
        self.assertEqual(len(common), 22)
        self.assertEqual(common[0].pattern_packed_id, 257)
        self.assertEqual(common[0].base_color_id, 1)
        self.assertEqual(common[0].pattern_color_id, 7)

    def test_unsupported_register_shape_reports_source_span(self) -> None:
        source = parse_source(
            "fixture/BadAttributes.java",
            b"""
package fixture;
public class BadAttributes {
  public static final Holder<Attribute> ONE = guess(
      "one", new RangedAttribute("attribute.one", 0, 0, 1));
}
""",
        )
        with self.assertRaisesRegex(
            GameplayIdExtractionError,
            r"fixture/BadAttributes\.java:\d+:\d+: .*not initialized by register",
        ):
            attribute_entries(source, 1)


if __name__ == "__main__":
    unittest.main()
