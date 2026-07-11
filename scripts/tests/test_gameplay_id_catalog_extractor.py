from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from gameplay_id_catalog_extractor import (  # noqa: E402
    GameplayIdExtractionError,
    attribute_entries,
    advancement_frame_type_entries,
    client_game_event_type_entries,
    client_command_action_entries,
    boss_bar_style_entries,
    chat_formatting_entries,
    custom_chat_completions_action_entries,
    difficulty_entries,
    difficulty_out_of_bounds_strategy,
    entity_data_serializer_entries,
    entity_anchor_entries,
    entity_pose_entries,
    equipment_slot_entries,
    game_type_entries,
    game_type_not_set_id,
    heightmap_type_entries,
    mob_effect_entries,
    objective_render_type_entries,
    player_info_action_entries,
    player_action_kind_entries,
    player_command_action_entries,
    resource_pack_response_action_entries,
    remote_debug_sample_type_entries,
    sound_source_entries,
    scoreboard_display_slot_entries,
    scoreboard_team_option_entries,
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
