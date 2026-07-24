use super::*;
use crate::{
    builtin_enum_component_values::{VANILLA_DYE_COLOR_BLUE_ID, VANILLA_DYE_COLOR_RED_ID},
    codec::{offline_player_uuid, Decoder, Encoder, ProtocolError},
    data_component_types::{
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID, VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    },
    entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
    ids,
    item_component_value_types::MapPostProcessingSummary,
    item_types::{VANILLA_ITEM_BRICK_ID, VANILLA_ITEM_STONE_ID},
    nbt_summary::decode_nbt_summary_from_decoder,
    NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[test]
fn handshake_and_status_codecs_match_vanilla_sequence_payloads() {
    for (intent, expected_id) in [
        (ClientIntent::Status, 1),
        (ClientIntent::Login, 2),
        (ClientIntent::Transfer, 3),
    ] {
        let (packet_id, payload) = encode_handshake("example.test", 25565, intent).unwrap();
        assert_eq!(packet_id, ids::handshake::SERVERBOUND_CLIENT_INTENTION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), crate::PROTOCOL_VERSION);
        assert_eq!(decoder.read_string(255).unwrap(), "example.test");
        assert_eq!(decoder.read_u16().unwrap(), 25565);
        assert_eq!(decoder.read_var_i32().unwrap(), expected_id);
        assert!(decoder.is_empty());
    }

    encode_handshake(&"a".repeat(255), 25565, ClientIntent::Login).unwrap();
    assert!(encode_handshake(&"a".repeat(256), 25565, ClientIntent::Login).is_err());
    assert!(encode_handshake(&"😀".repeat(128), 25565, ClientIntent::Login).is_err());

    assert_eq!(
        encode_status_request(),
        (ids::status::SERVERBOUND_STATUS_REQUEST, Vec::new())
    );
    let (packet_id, ping) = encode_ping_request(-123_456_789);
    assert_eq!(packet_id, ids::status::SERVERBOUND_PING_REQUEST);
    assert_eq!(decode_pong_response(&ping).unwrap(), -123_456_789);

    let mut response = Encoder::new();
    response.write_string(r#"{"version":{"name":"26.1"}}"#);
    assert_eq!(
        decode_status_response(&response.into_inner()).unwrap(),
        r#"{"version":{"name":"26.1"}}"#
    );
}

#[test]
fn play_clientbound_packet_ids_match_vanilla_26_1_registration_order() {
    let ids = [
        (
            "CLIENTBOUND_BUNDLE_DELIMITER",
            ids::play::CLIENTBOUND_BUNDLE_DELIMITER,
            0,
        ),
        (
            "CLIENTBOUND_ADD_ENTITY",
            ids::play::CLIENTBOUND_ADD_ENTITY,
            1,
        ),
        ("CLIENTBOUND_ANIMATE", ids::play::CLIENTBOUND_ANIMATE, 2),
        (
            "CLIENTBOUND_AWARD_STATS",
            ids::play::CLIENTBOUND_AWARD_STATS,
            3,
        ),
        (
            "CLIENTBOUND_BLOCK_CHANGED_ACK",
            ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK,
            4,
        ),
        (
            "CLIENTBOUND_BLOCK_DESTRUCTION",
            ids::play::CLIENTBOUND_BLOCK_DESTRUCTION,
            5,
        ),
        (
            "CLIENTBOUND_BLOCK_ENTITY_DATA",
            ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
            6,
        ),
        (
            "CLIENTBOUND_BLOCK_EVENT",
            ids::play::CLIENTBOUND_BLOCK_EVENT,
            7,
        ),
        (
            "CLIENTBOUND_BLOCK_UPDATE",
            ids::play::CLIENTBOUND_BLOCK_UPDATE,
            8,
        ),
        (
            "CLIENTBOUND_BOSS_EVENT",
            ids::play::CLIENTBOUND_BOSS_EVENT,
            9,
        ),
        (
            "CLIENTBOUND_CHANGE_DIFFICULTY",
            ids::play::CLIENTBOUND_CHANGE_DIFFICULTY,
            10,
        ),
        (
            "CLIENTBOUND_CHUNK_BATCH_FINISHED",
            ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED,
            11,
        ),
        (
            "CLIENTBOUND_CHUNK_BATCH_START",
            ids::play::CLIENTBOUND_CHUNK_BATCH_START,
            12,
        ),
        (
            "CLIENTBOUND_CHUNKS_BIOMES",
            ids::play::CLIENTBOUND_CHUNKS_BIOMES,
            13,
        ),
        (
            "CLIENTBOUND_CLEAR_TITLES",
            ids::play::CLIENTBOUND_CLEAR_TITLES,
            14,
        ),
        (
            "CLIENTBOUND_COMMAND_SUGGESTIONS",
            ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS,
            15,
        ),
        ("CLIENTBOUND_COMMANDS", ids::play::CLIENTBOUND_COMMANDS, 16),
        (
            "CLIENTBOUND_CONTAINER_CLOSE",
            ids::play::CLIENTBOUND_CONTAINER_CLOSE,
            17,
        ),
        (
            "CLIENTBOUND_CONTAINER_SET_CONTENT",
            ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT,
            18,
        ),
        (
            "CLIENTBOUND_CONTAINER_SET_DATA",
            ids::play::CLIENTBOUND_CONTAINER_SET_DATA,
            19,
        ),
        (
            "CLIENTBOUND_CONTAINER_SET_SLOT",
            ids::play::CLIENTBOUND_CONTAINER_SET_SLOT,
            20,
        ),
        (
            "CLIENTBOUND_COOKIE_REQUEST",
            ids::play::CLIENTBOUND_COOKIE_REQUEST,
            21,
        ),
        ("CLIENTBOUND_COOLDOWN", ids::play::CLIENTBOUND_COOLDOWN, 22),
        (
            "CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS",
            ids::play::CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS,
            23,
        ),
        (
            "CLIENTBOUND_CUSTOM_PAYLOAD",
            ids::play::CLIENTBOUND_CUSTOM_PAYLOAD,
            24,
        ),
        (
            "CLIENTBOUND_DAMAGE_EVENT",
            ids::play::CLIENTBOUND_DAMAGE_EVENT,
            25,
        ),
        (
            "CLIENTBOUND_DEBUG_BLOCK_VALUE",
            ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE,
            26,
        ),
        (
            "CLIENTBOUND_DEBUG_CHUNK_VALUE",
            ids::play::CLIENTBOUND_DEBUG_CHUNK_VALUE,
            27,
        ),
        (
            "CLIENTBOUND_DEBUG_ENTITY_VALUE",
            ids::play::CLIENTBOUND_DEBUG_ENTITY_VALUE,
            28,
        ),
        (
            "CLIENTBOUND_DEBUG_EVENT",
            ids::play::CLIENTBOUND_DEBUG_EVENT,
            29,
        ),
        (
            "CLIENTBOUND_DEBUG_SAMPLE",
            ids::play::CLIENTBOUND_DEBUG_SAMPLE,
            30,
        ),
        (
            "CLIENTBOUND_DELETE_CHAT",
            ids::play::CLIENTBOUND_DELETE_CHAT,
            31,
        ),
        (
            "CLIENTBOUND_DISCONNECT",
            ids::play::CLIENTBOUND_DISCONNECT,
            32,
        ),
        (
            "CLIENTBOUND_DISGUISED_CHAT",
            ids::play::CLIENTBOUND_DISGUISED_CHAT,
            33,
        ),
        (
            "CLIENTBOUND_ENTITY_EVENT",
            ids::play::CLIENTBOUND_ENTITY_EVENT,
            34,
        ),
        (
            "CLIENTBOUND_ENTITY_POSITION_SYNC",
            ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC,
            35,
        ),
        ("CLIENTBOUND_EXPLODE", ids::play::CLIENTBOUND_EXPLODE, 36),
        (
            "CLIENTBOUND_FORGET_LEVEL_CHUNK",
            ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK,
            37,
        ),
        (
            "CLIENTBOUND_GAME_EVENT",
            ids::play::CLIENTBOUND_GAME_EVENT,
            38,
        ),
        (
            "CLIENTBOUND_GAME_RULE_VALUES",
            ids::play::CLIENTBOUND_GAME_RULE_VALUES,
            39,
        ),
        (
            "CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS",
            ids::play::CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS,
            40,
        ),
        (
            "CLIENTBOUND_MOUNT_SCREEN_OPEN",
            ids::play::CLIENTBOUND_MOUNT_SCREEN_OPEN,
            41,
        ),
        (
            "CLIENTBOUND_HURT_ANIMATION",
            ids::play::CLIENTBOUND_HURT_ANIMATION,
            42,
        ),
        (
            "CLIENTBOUND_INITIALIZE_BORDER",
            ids::play::CLIENTBOUND_INITIALIZE_BORDER,
            43,
        ),
        (
            "CLIENTBOUND_KEEP_ALIVE",
            ids::play::CLIENTBOUND_KEEP_ALIVE,
            44,
        ),
        (
            "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT",
            ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
            45,
        ),
        (
            "CLIENTBOUND_LEVEL_EVENT",
            ids::play::CLIENTBOUND_LEVEL_EVENT,
            46,
        ),
        (
            "CLIENTBOUND_LEVEL_PARTICLES",
            ids::play::CLIENTBOUND_LEVEL_PARTICLES,
            47,
        ),
        (
            "CLIENTBOUND_LIGHT_UPDATE",
            ids::play::CLIENTBOUND_LIGHT_UPDATE,
            48,
        ),
        ("CLIENTBOUND_LOGIN", ids::play::CLIENTBOUND_LOGIN, 49),
        (
            "CLIENTBOUND_LOW_DISK_SPACE_WARNING",
            ids::play::CLIENTBOUND_LOW_DISK_SPACE_WARNING,
            50,
        ),
        (
            "CLIENTBOUND_MAP_ITEM_DATA",
            ids::play::CLIENTBOUND_MAP_ITEM_DATA,
            51,
        ),
        (
            "CLIENTBOUND_MERCHANT_OFFERS",
            ids::play::CLIENTBOUND_MERCHANT_OFFERS,
            52,
        ),
        (
            "CLIENTBOUND_MOVE_ENTITY_POS",
            ids::play::CLIENTBOUND_MOVE_ENTITY_POS,
            53,
        ),
        (
            "CLIENTBOUND_MOVE_ENTITY_POS_ROT",
            ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT,
            54,
        ),
        (
            "CLIENTBOUND_MOVE_MINECART_ALONG_TRACK",
            ids::play::CLIENTBOUND_MOVE_MINECART_ALONG_TRACK,
            55,
        ),
        (
            "CLIENTBOUND_MOVE_ENTITY_ROT",
            ids::play::CLIENTBOUND_MOVE_ENTITY_ROT,
            56,
        ),
        (
            "CLIENTBOUND_MOVE_VEHICLE",
            ids::play::CLIENTBOUND_MOVE_VEHICLE,
            57,
        ),
        (
            "CLIENTBOUND_OPEN_BOOK",
            ids::play::CLIENTBOUND_OPEN_BOOK,
            58,
        ),
        (
            "CLIENTBOUND_OPEN_SCREEN",
            ids::play::CLIENTBOUND_OPEN_SCREEN,
            59,
        ),
        (
            "CLIENTBOUND_OPEN_SIGN_EDITOR",
            ids::play::CLIENTBOUND_OPEN_SIGN_EDITOR,
            60,
        ),
        ("CLIENTBOUND_PING", ids::play::CLIENTBOUND_PING, 61),
        (
            "CLIENTBOUND_PONG_RESPONSE",
            ids::play::CLIENTBOUND_PONG_RESPONSE,
            62,
        ),
        (
            "CLIENTBOUND_PLACE_GHOST_RECIPE",
            ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE,
            63,
        ),
        (
            "CLIENTBOUND_PLAYER_ABILITIES",
            ids::play::CLIENTBOUND_PLAYER_ABILITIES,
            64,
        ),
        (
            "CLIENTBOUND_PLAYER_CHAT",
            ids::play::CLIENTBOUND_PLAYER_CHAT,
            65,
        ),
        (
            "CLIENTBOUND_PLAYER_COMBAT_END",
            ids::play::CLIENTBOUND_PLAYER_COMBAT_END,
            66,
        ),
        (
            "CLIENTBOUND_PLAYER_COMBAT_ENTER",
            ids::play::CLIENTBOUND_PLAYER_COMBAT_ENTER,
            67,
        ),
        (
            "CLIENTBOUND_PLAYER_COMBAT_KILL",
            ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL,
            68,
        ),
        (
            "CLIENTBOUND_PLAYER_INFO_REMOVE",
            ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE,
            69,
        ),
        (
            "CLIENTBOUND_PLAYER_INFO_UPDATE",
            ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE,
            70,
        ),
        (
            "CLIENTBOUND_PLAYER_LOOK_AT",
            ids::play::CLIENTBOUND_PLAYER_LOOK_AT,
            71,
        ),
        (
            "CLIENTBOUND_PLAYER_POSITION",
            ids::play::CLIENTBOUND_PLAYER_POSITION,
            72,
        ),
        (
            "CLIENTBOUND_PLAYER_ROTATION",
            ids::play::CLIENTBOUND_PLAYER_ROTATION,
            73,
        ),
        (
            "CLIENTBOUND_RECIPE_BOOK_ADD",
            ids::play::CLIENTBOUND_RECIPE_BOOK_ADD,
            74,
        ),
        (
            "CLIENTBOUND_RECIPE_BOOK_REMOVE",
            ids::play::CLIENTBOUND_RECIPE_BOOK_REMOVE,
            75,
        ),
        (
            "CLIENTBOUND_RECIPE_BOOK_SETTINGS",
            ids::play::CLIENTBOUND_RECIPE_BOOK_SETTINGS,
            76,
        ),
        (
            "CLIENTBOUND_REMOVE_ENTITIES",
            ids::play::CLIENTBOUND_REMOVE_ENTITIES,
            77,
        ),
        (
            "CLIENTBOUND_REMOVE_MOB_EFFECT",
            ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT,
            78,
        ),
        (
            "CLIENTBOUND_RESET_SCORE",
            ids::play::CLIENTBOUND_RESET_SCORE,
            79,
        ),
        (
            "CLIENTBOUND_RESOURCE_PACK_POP",
            ids::play::CLIENTBOUND_RESOURCE_PACK_POP,
            80,
        ),
        (
            "CLIENTBOUND_RESOURCE_PACK_PUSH",
            ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH,
            81,
        ),
        ("CLIENTBOUND_RESPAWN", ids::play::CLIENTBOUND_RESPAWN, 82),
        (
            "CLIENTBOUND_ROTATE_HEAD",
            ids::play::CLIENTBOUND_ROTATE_HEAD,
            83,
        ),
        (
            "CLIENTBOUND_SECTION_BLOCKS_UPDATE",
            ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE,
            84,
        ),
        (
            "CLIENTBOUND_SELECT_ADVANCEMENTS_TAB",
            ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB,
            85,
        ),
        (
            "CLIENTBOUND_SERVER_DATA",
            ids::play::CLIENTBOUND_SERVER_DATA,
            86,
        ),
        (
            "CLIENTBOUND_SET_ACTION_BAR_TEXT",
            ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT,
            87,
        ),
        (
            "CLIENTBOUND_SET_BORDER_CENTER",
            ids::play::CLIENTBOUND_SET_BORDER_CENTER,
            88,
        ),
        (
            "CLIENTBOUND_SET_BORDER_LERP_SIZE",
            ids::play::CLIENTBOUND_SET_BORDER_LERP_SIZE,
            89,
        ),
        (
            "CLIENTBOUND_SET_BORDER_SIZE",
            ids::play::CLIENTBOUND_SET_BORDER_SIZE,
            90,
        ),
        (
            "CLIENTBOUND_SET_BORDER_WARNING_DELAY",
            ids::play::CLIENTBOUND_SET_BORDER_WARNING_DELAY,
            91,
        ),
        (
            "CLIENTBOUND_SET_BORDER_WARNING_DISTANCE",
            ids::play::CLIENTBOUND_SET_BORDER_WARNING_DISTANCE,
            92,
        ),
        (
            "CLIENTBOUND_SET_CAMERA",
            ids::play::CLIENTBOUND_SET_CAMERA,
            93,
        ),
        (
            "CLIENTBOUND_SET_CHUNK_CACHE_CENTER",
            ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER,
            94,
        ),
        (
            "CLIENTBOUND_SET_CHUNK_CACHE_RADIUS",
            ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS,
            95,
        ),
        (
            "CLIENTBOUND_SET_CURSOR_ITEM",
            ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
            96,
        ),
        (
            "CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION",
            ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION,
            97,
        ),
        (
            "CLIENTBOUND_SET_DISPLAY_OBJECTIVE",
            ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE,
            98,
        ),
        (
            "CLIENTBOUND_SET_ENTITY_DATA",
            ids::play::CLIENTBOUND_SET_ENTITY_DATA,
            99,
        ),
        (
            "CLIENTBOUND_SET_ENTITY_LINK",
            ids::play::CLIENTBOUND_SET_ENTITY_LINK,
            100,
        ),
        (
            "CLIENTBOUND_SET_ENTITY_MOTION",
            ids::play::CLIENTBOUND_SET_ENTITY_MOTION,
            101,
        ),
        (
            "CLIENTBOUND_SET_EQUIPMENT",
            ids::play::CLIENTBOUND_SET_EQUIPMENT,
            102,
        ),
        (
            "CLIENTBOUND_SET_EXPERIENCE",
            ids::play::CLIENTBOUND_SET_EXPERIENCE,
            103,
        ),
        (
            "CLIENTBOUND_SET_HEALTH",
            ids::play::CLIENTBOUND_SET_HEALTH,
            104,
        ),
        (
            "CLIENTBOUND_SET_HELD_SLOT",
            ids::play::CLIENTBOUND_SET_HELD_SLOT,
            105,
        ),
        (
            "CLIENTBOUND_SET_OBJECTIVE",
            ids::play::CLIENTBOUND_SET_OBJECTIVE,
            106,
        ),
        (
            "CLIENTBOUND_SET_PASSENGERS",
            ids::play::CLIENTBOUND_SET_PASSENGERS,
            107,
        ),
        (
            "CLIENTBOUND_SET_PLAYER_INVENTORY",
            ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY,
            108,
        ),
        (
            "CLIENTBOUND_SET_PLAYER_TEAM",
            ids::play::CLIENTBOUND_SET_PLAYER_TEAM,
            109,
        ),
        (
            "CLIENTBOUND_SET_SCORE",
            ids::play::CLIENTBOUND_SET_SCORE,
            110,
        ),
        (
            "CLIENTBOUND_SET_SIMULATION_DISTANCE",
            ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE,
            111,
        ),
        (
            "CLIENTBOUND_SET_SUBTITLE_TEXT",
            ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT,
            112,
        ),
        ("CLIENTBOUND_SET_TIME", ids::play::CLIENTBOUND_SET_TIME, 113),
        (
            "CLIENTBOUND_SET_TITLE_TEXT",
            ids::play::CLIENTBOUND_SET_TITLE_TEXT,
            114,
        ),
        (
            "CLIENTBOUND_SET_TITLES_ANIMATION",
            ids::play::CLIENTBOUND_SET_TITLES_ANIMATION,
            115,
        ),
        (
            "CLIENTBOUND_SOUND_ENTITY",
            ids::play::CLIENTBOUND_SOUND_ENTITY,
            116,
        ),
        ("CLIENTBOUND_SOUND", ids::play::CLIENTBOUND_SOUND, 117),
        (
            "CLIENTBOUND_START_CONFIGURATION",
            ids::play::CLIENTBOUND_START_CONFIGURATION,
            118,
        ),
        (
            "CLIENTBOUND_STOP_SOUND",
            ids::play::CLIENTBOUND_STOP_SOUND,
            119,
        ),
        (
            "CLIENTBOUND_STORE_COOKIE",
            ids::play::CLIENTBOUND_STORE_COOKIE,
            120,
        ),
        (
            "CLIENTBOUND_SYSTEM_CHAT",
            ids::play::CLIENTBOUND_SYSTEM_CHAT,
            121,
        ),
        ("CLIENTBOUND_TAB_LIST", ids::play::CLIENTBOUND_TAB_LIST, 122),
        (
            "CLIENTBOUND_TAG_QUERY",
            ids::play::CLIENTBOUND_TAG_QUERY,
            123,
        ),
        (
            "CLIENTBOUND_TAKE_ITEM_ENTITY",
            ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY,
            124,
        ),
        (
            "CLIENTBOUND_TELEPORT_ENTITY",
            ids::play::CLIENTBOUND_TELEPORT_ENTITY,
            125,
        ),
        (
            "CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS",
            ids::play::CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS,
            126,
        ),
        (
            "CLIENTBOUND_TICKING_STATE",
            ids::play::CLIENTBOUND_TICKING_STATE,
            127,
        ),
        (
            "CLIENTBOUND_TICKING_STEP",
            ids::play::CLIENTBOUND_TICKING_STEP,
            128,
        ),
        ("CLIENTBOUND_TRANSFER", ids::play::CLIENTBOUND_TRANSFER, 129),
        (
            "CLIENTBOUND_UPDATE_ADVANCEMENTS",
            ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS,
            130,
        ),
        (
            "CLIENTBOUND_UPDATE_ATTRIBUTES",
            ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES,
            131,
        ),
        (
            "CLIENTBOUND_UPDATE_MOB_EFFECT",
            ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT,
            132,
        ),
        (
            "CLIENTBOUND_UPDATE_RECIPES",
            ids::play::CLIENTBOUND_UPDATE_RECIPES,
            133,
        ),
        (
            "CLIENTBOUND_UPDATE_TAGS",
            ids::play::CLIENTBOUND_UPDATE_TAGS,
            134,
        ),
        (
            "CLIENTBOUND_PROJECTILE_POWER",
            ids::play::CLIENTBOUND_PROJECTILE_POWER,
            135,
        ),
        (
            "CLIENTBOUND_CUSTOM_REPORT_DETAILS",
            ids::play::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
            136,
        ),
        (
            "CLIENTBOUND_SERVER_LINKS",
            ids::play::CLIENTBOUND_SERVER_LINKS,
            137,
        ),
        ("CLIENTBOUND_WAYPOINT", ids::play::CLIENTBOUND_WAYPOINT, 138),
        (
            "CLIENTBOUND_CLEAR_DIALOG",
            ids::play::CLIENTBOUND_CLEAR_DIALOG,
            139,
        ),
        (
            "CLIENTBOUND_SHOW_DIALOG",
            ids::play::CLIENTBOUND_SHOW_DIALOG,
            140,
        ),
    ];

    for (name, actual, expected) in ids {
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn configuration_clientbound_packet_ids_match_vanilla_26_1_registration_order() {
    let ids = [
        (
            "CLIENTBOUND_COOKIE_REQUEST",
            ids::configuration::CLIENTBOUND_COOKIE_REQUEST,
            0,
        ),
        (
            "CLIENTBOUND_CUSTOM_PAYLOAD",
            ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
            1,
        ),
        (
            "CLIENTBOUND_DISCONNECT",
            ids::configuration::CLIENTBOUND_DISCONNECT,
            2,
        ),
        (
            "CLIENTBOUND_FINISH_CONFIGURATION",
            ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION,
            3,
        ),
        (
            "CLIENTBOUND_KEEP_ALIVE",
            ids::configuration::CLIENTBOUND_KEEP_ALIVE,
            4,
        ),
        ("CLIENTBOUND_PING", ids::configuration::CLIENTBOUND_PING, 5),
        (
            "CLIENTBOUND_RESET_CHAT",
            ids::configuration::CLIENTBOUND_RESET_CHAT,
            6,
        ),
        (
            "CLIENTBOUND_REGISTRY_DATA",
            ids::configuration::CLIENTBOUND_REGISTRY_DATA,
            7,
        ),
        (
            "CLIENTBOUND_RESOURCE_PACK_POP",
            ids::configuration::CLIENTBOUND_RESOURCE_PACK_POP,
            8,
        ),
        (
            "CLIENTBOUND_RESOURCE_PACK_PUSH",
            ids::configuration::CLIENTBOUND_RESOURCE_PACK_PUSH,
            9,
        ),
        (
            "CLIENTBOUND_STORE_COOKIE",
            ids::configuration::CLIENTBOUND_STORE_COOKIE,
            10,
        ),
        (
            "CLIENTBOUND_TRANSFER",
            ids::configuration::CLIENTBOUND_TRANSFER,
            11,
        ),
        (
            "CLIENTBOUND_UPDATE_ENABLED_FEATURES",
            ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
            12,
        ),
        (
            "CLIENTBOUND_UPDATE_TAGS",
            ids::configuration::CLIENTBOUND_UPDATE_TAGS,
            13,
        ),
        (
            "CLIENTBOUND_SELECT_KNOWN_PACKS",
            ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS,
            14,
        ),
        (
            "CLIENTBOUND_CUSTOM_REPORT_DETAILS",
            ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
            15,
        ),
        (
            "CLIENTBOUND_SERVER_LINKS",
            ids::configuration::CLIENTBOUND_SERVER_LINKS,
            16,
        ),
        (
            "CLIENTBOUND_CLEAR_DIALOG",
            ids::configuration::CLIENTBOUND_CLEAR_DIALOG,
            17,
        ),
        (
            "CLIENTBOUND_SHOW_DIALOG",
            ids::configuration::CLIENTBOUND_SHOW_DIALOG,
            18,
        ),
        (
            "CLIENTBOUND_CODE_OF_CONDUCT",
            ids::configuration::CLIENTBOUND_CODE_OF_CONDUCT,
            19,
        ),
    ];

    for (name, actual, expected) in ids {
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn decodes_bundle_delimiter_packet() {
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BUNDLE_DELIMITER, &[]).unwrap();
    assert_eq!(packet, PlayClientbound::BundleDelimiter);

    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_BUNDLE_DELIMITER, &[0])
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after bundle delimiter packet")
    );
}

#[test]
fn decodes_start_configuration_unit_packet_strictly() {
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_START_CONFIGURATION, &[]).unwrap(),
        PlayClientbound::StartConfiguration
    );
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_START_CONFIGURATION, &[0])
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after start configuration packet")
    );
}

#[test]
fn encodes_login_hello() {
    let uuid = offline_player_uuid("bbb-client");
    let (id, payload) = encode_login_hello("bbb-client", uuid).unwrap();
    assert_eq!(id, ids::login::SERVERBOUND_HELLO);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(16).unwrap(), "bbb-client");
    assert_eq!(decoder.read_uuid().unwrap(), uuid);
    assert!(decoder.is_empty());

    encode_login_hello("abcdefghijklmnop", uuid).unwrap();
    for invalid in [
        "abcdefghijklmnopq",
        "name with space",
        "line\nbreak",
        "name\u{7f}",
        "玩家",
        "😀😀😀😀😀😀😀😀😀",
    ] {
        assert!(
            encode_login_hello(invalid, uuid).is_err(),
            "{invalid:?} must fail vanilla login-name validation"
        );
    }
}

#[test]
fn login_custom_query_codec_validates_key_discards_payload_and_answers_null() {
    let mut payload = Encoder::new();
    payload.write_var_i32(42);
    payload.write_string("bbb:login_test");
    payload.write_bytes(&[1, 2, 3]);
    assert_eq!(
        decode_login_clientbound(ids::login::CLIENTBOUND_CUSTOM_QUERY, &payload.into_inner())
            .unwrap(),
        LoginClientbound::CustomQuery { transaction_id: 42 }
    );

    let (packet_id, payload) = encode_login_custom_query_answer(42);
    assert_eq!(packet_id, ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn login_custom_query_and_profile_limits_match_vanilla() {
    let mut query = Encoder::new();
    query.write_var_i32(7);
    query.write_string("bbb:oversized");
    query.write_bytes(&vec![0; 1_048_577]);
    let error = decode_login_clientbound(ids::login::CLIENTBOUND_CUSTOM_QUERY, &query.into_inner())
        .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(1_048_577, 1_048_576)
    ));

    let mut profile = Encoder::new();
    profile.write_uuid(Uuid::nil());
    profile.write_string("bbb-client");
    profile.write_var_i32(17);
    let error = decode_login_clientbound(
        ids::login::CLIENTBOUND_LOGIN_FINISHED,
        &profile.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(17, 16)));

    let mut profile = Encoder::new();
    profile.write_uuid(Uuid::nil());
    profile.write_string("bbb-client");
    profile.write_var_i32(1);
    profile.write_string(&"n".repeat(65));
    profile.write_string("value");
    profile.write_bool(false);
    let error = decode_login_clientbound(
        ids::login::CLIENTBOUND_LOGIN_FINISHED,
        &profile.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong { max: 64, .. }
    ));

    let mut profile = Encoder::new();
    profile.write_uuid(Uuid::nil());
    profile.write_string("bbb-client");
    profile.write_var_i32(1);
    profile.write_string("textures");
    profile.write_string("value");
    profile.write_bool(true);
    profile.write_string(&"s".repeat(1025));
    let error = decode_login_clientbound(
        ids::login::CLIENTBOUND_LOGIN_FINISHED,
        &profile.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong { max: 1024, .. }
    ));
}

#[test]
fn offline_login_clientbound_codecs_and_ack_match_vanilla_fields() {
    let mut disconnect = Encoder::new();
    disconnect.write_string(r#"{"text":"bye"}"#);
    assert_eq!(
        decode_login_clientbound(
            ids::login::CLIENTBOUND_LOGIN_DISCONNECT,
            &disconnect.into_inner()
        )
        .unwrap(),
        LoginClientbound::Disconnect {
            reason: "bye".to_string(),
            raw_reason: r#"{"text":"bye"}"#.to_string(),
        }
    );

    let mut styled_disconnect = Encoder::new();
    styled_disconnect.write_string(r#"{"text":"bye","extra":[{"text":" now","bold":true}]}"#);
    assert_eq!(
        decode_login_clientbound(
            ids::login::CLIENTBOUND_LOGIN_DISCONNECT,
            &styled_disconnect.into_inner(),
        )
        .unwrap(),
        LoginClientbound::Disconnect {
            reason: "bye now".to_string(),
            raw_reason: r#"{"text":"bye","extra":[{"text":" now","bold":true}]}"#.to_string(),
        }
    );

    let mut malformed_disconnect = Encoder::new();
    malformed_disconnect.write_string(r#"{"text":}"#);
    assert!(decode_login_clientbound(
        ids::login::CLIENTBOUND_LOGIN_DISCONNECT,
        &malformed_disconnect.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("invalid component json"));

    let mut trailing_disconnect = Encoder::new();
    trailing_disconnect.write_string(r#"{"text":"bye"}"#);
    trailing_disconnect.write_u8(0);
    assert!(decode_login_clientbound(
        ids::login::CLIENTBOUND_LOGIN_DISCONNECT,
        &trailing_disconnect.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after login disconnect packet"));

    let uuid = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
    let mut finished = Encoder::new();
    finished.write_uuid(uuid);
    finished.write_string("bbb-client");
    finished.write_var_i32(1);
    finished.write_string("textures");
    finished.write_string("value");
    finished.write_bool(true);
    finished.write_string("signature");
    assert_eq!(
        decode_login_clientbound(
            ids::login::CLIENTBOUND_LOGIN_FINISHED,
            &finished.into_inner()
        )
        .unwrap(),
        LoginClientbound::LoginFinished {
            profile: GameProfile {
                uuid,
                name: "bbb-client".to_string(),
                properties: vec![GameProfileProperty {
                    name: "textures".to_string(),
                    value: "value".to_string(),
                    signature: Some("signature".to_string()),
                }],
            },
        }
    );

    let mut compression = Encoder::new();
    compression.write_var_i32(256);
    assert_eq!(
        decode_login_clientbound(
            ids::login::CLIENTBOUND_LOGIN_COMPRESSION,
            &compression.into_inner()
        )
        .unwrap(),
        LoginClientbound::SetCompression { threshold: 256 }
    );

    assert_eq!(
        encode_login_acknowledged(),
        (ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED, Vec::new())
    );
}

#[test]
fn decodes_login_encryption_request_strictly() {
    let mut hello = Encoder::new();
    hello.write_string("");
    hello.write_var_i32(4);
    hello.write_bytes(&[0x30, 0x82, 0x01, 0x22]);
    hello.write_var_i32(3);
    hello.write_bytes(&[0xaa, 0xbb, 0xcc]);
    hello.write_bool(true);
    assert_eq!(
        decode_login_clientbound(ids::login::CLIENTBOUND_HELLO, &hello.into_inner()).unwrap(),
        LoginClientbound::EncryptionRequest(EncryptionRequest {
            server_id: String::new(),
            public_key: vec![0x30, 0x82, 0x01, 0x22],
            challenge: vec![0xaa, 0xbb, 0xcc],
            should_authenticate: true,
        })
    );

    let mut trailing = Encoder::new();
    trailing.write_string("server");
    trailing.write_var_i32(0);
    trailing.write_var_i32(0);
    trailing.write_bool(false);
    trailing.write_u8(0);
    assert!(
        decode_login_clientbound(ids::login::CLIENTBOUND_HELLO, &trailing.into_inner())
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after login hello packet")
    );

    let mut truncated = Encoder::new();
    truncated.write_string("server");
    truncated.write_var_i32(4);
    truncated.write_bytes(&[1, 2]);
    assert!(
        decode_login_clientbound(ids::login::CLIENTBOUND_HELLO, &truncated.into_inner()).is_err()
    );
}

#[test]
fn encodes_login_key_encrypted_arrays_in_wire_order() {
    let packet = LoginKey {
        encrypted_secret: vec![0x10, 0x20, 0x30, 0x40],
        encrypted_challenge: vec![0xaa, 0xbb, 0xcc],
    };
    let (id, payload) = encode_login_key(&packet).unwrap();
    assert_eq!(id, ids::login::SERVERBOUND_KEY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_len().unwrap(), 4);
    assert_eq!(
        decoder.read_exact(4, "encrypted secret").unwrap(),
        packet.encrypted_secret
    );
    assert_eq!(decoder.read_len().unwrap(), 3);
    assert_eq!(
        decoder.read_exact(3, "encrypted challenge").unwrap(),
        packet.encrypted_challenge
    );
    assert!(decoder.is_empty());
}

#[test]
fn decodes_disconnect_component_in_configuration_and_play() {
    let mut payload = Vec::new();
    payload.push(8);
    payload.extend_from_slice(&6u16.to_be_bytes());
    payload.extend_from_slice(b"Kicked");

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_DISCONNECT, &payload)
            .unwrap(),
        ConfigurationClientbound::Disconnect {
            reason: "Kicked".to_string(),
            raw_reason: payload.clone(),
        }
    );
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_DISCONNECT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Disconnect(Disconnect {
            reason: "Kicked".to_string(),
            raw_reason: payload.clone(),
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    for error in [
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_DISCONNECT, &trailing)
            .unwrap_err(),
        decode_play_clientbound(ids::play::CLIENTBOUND_DISCONNECT, &trailing).unwrap_err(),
    ] {
        assert!(error
            .to_string()
            .contains("trailing bytes after component nbt"));
    }
}

#[test]
fn decodes_play_login_spawn_info() {
    let mut payload = Encoder::new();
    payload.write_i32(42);
    payload.write_bool(true);
    payload.write_var_i32(3);
    payload.write_string("overworld");
    payload.write_string("the_nether");
    payload.write_string("the_end");
    payload.write_var_i32(20);
    payload.write_var_i32(8);
    payload.write_var_i32(6);
    payload.write_bool(false);
    payload.write_bool(true);
    payload.write_bool(false);
    payload.write_var_i32(1);
    payload.write_string("the_nether");
    payload.write_i64(12345);
    payload.write_i8(1);
    payload.write_i8(-1);
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_bool(true);
    payload.write_string("overworld");
    payload.write_i64(encode_block_pos(1, 64, -2));
    payload.write_var_i32(7);
    payload.write_var_i32(32);
    payload.write_bool(true);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_LOGIN, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Login(PlayLogin {
            player_id: 42,
            hardcore: true,
            levels: vec![
                "minecraft:overworld".to_string(),
                "minecraft:the_nether".to_string(),
                "minecraft:the_end".to_string(),
            ],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: Some(GlobalPos {
                    dimension: "minecraft:overworld".to_string(),
                    pos: BlockPos { x: 1, y: 64, z: -2 },
                }),
                portal_cooldown: 7,
                sea_level: 32,
            },
            enforces_secure_chat: true,
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_LOGIN, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after play login packet")
    );
}

#[test]
fn rejects_invalid_play_login_dimension_key() {
    let mut payload = Encoder::new();
    payload.write_i32(42);
    payload.write_bool(false);
    payload.write_var_i32(1);
    payload.write_string("minecraft:Overworld");

    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_LOGIN, &payload.into_inner()).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn decodes_respawn_spawn_info() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_string("the_end");
    payload.write_i64(98765);
    payload.write_i8(0);
    payload.write_i8(1);
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_var_i32(0);
    payload.write_var_i32(63);
    payload.write_i8(3);

    let payload = payload.into_inner();
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_RESPAWN, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Respawn(Respawn {
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 2,
                dimension: "minecraft:the_end".to_string(),
                seed: 98765,
                game_type: 0,
                previous_game_type: 1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep: 3,
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_RESPAWN, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after respawn packet")
    );
}

#[test]
fn encodes_perform_respawn() {
    let (id, payload) = encode_play_perform_respawn();
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_COMMAND);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_client_command_actions() {
    for (action, expected_ordinal) in [
        (ClientCommandAction::PerformRespawn, 0),
        (ClientCommandAction::RequestStats, 1),
        (ClientCommandAction::RequestGameRuleValues, 2),
    ] {
        let (id, payload) = encode_play_client_command(action);
        assert_eq!(id, ids::play::SERVERBOUND_CLIENT_COMMAND);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), expected_ordinal);
        assert!(decoder.is_empty());
    }
}

#[test]
fn encodes_set_game_rule_entries_in_wire_order() {
    let (id, payload) = encode_play_set_game_rule(&SetGameRule {
        entries: vec![
            SetGameRuleEntry {
                rule: "keep_inventory".to_string(),
                value: "true".to_string(),
            },
            SetGameRuleEntry {
                rule: "minecraft:random_tick_speed".to_string(),
                value: "12".to_string(),
            },
        ],
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_GAME_RULE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(
        decoder.read_string(32767).unwrap(),
        "minecraft:keep_inventory"
    );
    assert_eq!(decoder.read_string(32767).unwrap(), "true");
    assert_eq!(
        decoder.read_string(32767).unwrap(),
        "minecraft:random_tick_speed"
    );
    assert_eq!(decoder.read_string(32767).unwrap(), "12");
    assert!(decoder.is_empty());
}

#[test]
fn set_game_rule_rejects_unknown_duplicate_and_invalid_entries() {
    for packet in [
        SetGameRule {
            entries: vec![SetGameRuleEntry {
                rule: "minecraft:custom_rule".to_string(),
                value: "true".to_string(),
            }],
        },
        SetGameRule {
            entries: vec![
                SetGameRuleEntry {
                    rule: "keep_inventory".to_string(),
                    value: "true".to_string(),
                },
                SetGameRuleEntry {
                    rule: "minecraft:keep_inventory".to_string(),
                    value: "false".to_string(),
                },
            ],
        },
        SetGameRule {
            entries: vec![SetGameRuleEntry {
                rule: "minecraft:keep_inventory".to_string(),
                value: "TRUE".to_string(),
            }],
        },
        SetGameRule {
            entries: vec![SetGameRuleEntry {
                rule: "minecraft:max_snow_accumulation_height".to_string(),
                value: "9".to_string(),
            }],
        },
    ] {
        assert!(encode_play_set_game_rule(&packet).is_err());
    }
}

#[test]
fn encodes_set_carried_item() {
    let (id, payload) = encode_play_set_carried_item(6);
    assert_eq!(id, ids::play::SERVERBOUND_SET_CARRIED_ITEM);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 6);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_componentless_item() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 64,
            component_patch: DataComponentPatchSummary::default(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 64);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_foundational_component_patch() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                ],
                removed_type_ids: vec![
                    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                    VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                ],
                max_stack_size: Some(64),
                max_damage: Some(300),
                damage: Some(7),
                unbreakable: true,
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            4, 2, // positive and negative patch counts
            3, 1, 7, // damage
            4, 0, // zero-byte unbreakable Unit
            1, 1, 64, // max_stack_size
            2, 2, 0xac, 0x02, // max_damage = 300
            0, 109, // removed component types
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_custom_data_component() {
    let custom_data = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "answer".to_string(),
        value: NbtSummaryValue::Int(42),
    }]);
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
                custom_data: Some(custom_data.clone()),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value_decoder =
        Decoder::new(decoder.read_exact(value_len, "custom_data value").unwrap());
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value_decoder).unwrap(),
        Some(custom_data)
    );
    assert!(value_decoder.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_custom_model_data_component() {
    let nan = f32::from_bits(0x7fc0_0001);
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
                ],
                custom_model_data_floats: vec![-0.0, nan].into(),
                custom_model_data_flags: vec![true, false],
                custom_model_data_strings: vec!["variant".to_string(), "😀".to_string()],
                custom_model_data_colors: vec![-1, 0x11_22_33],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "custom_model_data value")
            .unwrap(),
    );
    assert_eq!(value.read_len().unwrap(), 2);
    assert_eq!(value.read_f32().unwrap().to_bits(), (-0.0_f32).to_bits());
    assert_eq!(value.read_f32().unwrap().to_bits(), nan.to_bits());
    assert_eq!(value.read_len().unwrap(), 2);
    assert!(value.read_bool().unwrap());
    assert!(!value.read_bool().unwrap());
    assert_eq!(value.read_len().unwrap(), 2);
    assert_eq!(value.read_string(32_767).unwrap(), "variant");
    assert_eq!(value.read_string(32_767).unwrap(), "😀");
    assert_eq!(value.read_len().unwrap(), 2);
    assert_eq!(value.read_i32().unwrap(), -1);
    assert_eq!(value.read_i32().unwrap(), 0x11_22_33);
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_tooltip_display_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
                ],
                tooltip_hide_tooltip: true,
                tooltip_hidden_component_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
                    crate::data_component_types::VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
                    crate::data_component_types::VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                ],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "tooltip_display value")
            .unwrap(),
    );
    assert!(value.read_bool().unwrap());
    assert_eq!(value.read_len().unwrap(), 3);
    assert_eq!(
        value.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID
    );
    assert_eq!(
        value.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID
    );
    assert_eq!(
        value.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID
    );
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_tool_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
                ],
                tool: Some(ToolSummary {
                    rules: vec![ToolRuleSummary {
                        block_registry_ids: Vec::new(),
                        block_tag: Some("minecraft:mineable/pickaxe".to_string()),
                        speed_bits: Some(8.0_f32.to_bits()),
                        correct_for_drops: Some(true),
                    }],
                    default_mining_speed_bits: 1.0_f32.to_bits(),
                    damage_per_block: 2,
                    can_destroy_blocks_in_creative: false,
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "tool component value")
            .unwrap(),
    );
    assert_eq!(value.read_len().unwrap(), 1);
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(
        value.read_string(32_767).unwrap(),
        "minecraft:mineable/pickaxe"
    );
    assert!(value.read_bool().unwrap());
    assert_eq!(value.read_f32().unwrap(), 8.0);
    assert!(value.read_bool().unwrap());
    assert!(value.read_bool().unwrap());
    assert_eq!(value.read_f32().unwrap(), 1.0);
    assert_eq!(value.read_var_i32().unwrap(), 2);
    assert!(!value.read_bool().unwrap());
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_attack_range_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
                ],
                attack_range: Some(AttackRangeSummary {
                    min_reach: 2.0,
                    max_reach: 4.5,
                    min_creative_reach: 2.0,
                    max_creative_reach: 6.5,
                    hitbox_margin: 0.125,
                    mob_factor: 0.5,
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "attack_range component value")
            .unwrap(),
    );
    for expected in [2.0, 4.5, 2.0, 6.5, 0.125, 0.5] {
        assert_eq!(value.read_f32().unwrap(), expected);
    }
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_equippable_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID],
                equippable: Some(EquippableSummary {
                    slot_id: 4,
                    equip_sound: SoundEventSummary {
                        registry_id: Some(0),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    asset_id: None,
                    camera_overlay: None,
                    allowed_entities: None,
                    dispensable: true,
                    swappable: false,
                    damage_on_hurt: true,
                    equip_on_interact: false,
                    can_be_sheared: true,
                    shearing_sound: SoundEventSummary {
                        registry_id: Some(1),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            32, 11, // equippable type and delimited value length
            4,  // head slot id
            1,  // equip sound registry holder 0
            0, 0, 0, // absent asset, overlay, and allowed entities
            1, 0, 1, 0, 1, // behavior flags
            2, // shearing sound registry holder 1
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_use_cooldown_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID],
                use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
                use_cooldown_ticks: Some(25),
                use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "use_cooldown component value")
            .unwrap(),
    );
    assert_eq!(value.read_f32().unwrap(), 1.25);
    assert!(value.read_bool().unwrap());
    assert_eq!(
        value.read_string(usize::MAX).unwrap(),
        "minecraft:ender_pearl"
    );
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_item_model_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID],
                item_model: Some("example:models/variant".to_string()),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "item_model component value")
            .unwrap(),
    );
    assert_eq!(value.read_string(32_767).unwrap(), "example:models/variant");
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_rarity_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_RARITY_TYPE_ID],
                rarity: Some(ItemRaritySummary::Rare),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "rarity component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), ItemRaritySummary::Rare.id());
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_enchantment_glint_override_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID],
                enchantment_glint_override: Some(false),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "enchantment_glint_override component value")
            .unwrap(),
    );
    assert!(!value.read_bool().unwrap());
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_intangible_projectile_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID],
                intangible_projectile: true,
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    assert_eq!(value_len, 2);
    let value = decoder
        .read_exact(value_len, "intangible_projectile component value")
        .unwrap();
    assert_eq!(value, [10, 0]);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_direct_instrument_component() {
    let styled = vec![crate::StyledTextRun {
        text: "Custom horn".to_string(),
        style: Default::default(),
    }];
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID],
                instrument_description: Some("Custom horn".to_string()),
                instrument_description_styled: Some(styled.clone()),
                instrument_direct: Some(Box::new(InstrumentSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(7),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    use_duration_bits: 3.25_f32.to_bits(),
                    range_bits: 96.0_f32.to_bits(),
                    description: "Custom horn".to_string(),
                    description_styled: Some(styled.into_boxed_slice()),
                    description_nbt: Some(Box::new(NbtSummaryValue::String(
                        "Custom horn".to_string(),
                    ))),
                })),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "instrument component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(value.read_var_i32().unwrap(), 8);
    assert_eq!(value.read_f32().unwrap(), 3.25);
    assert_eq!(value.read_f32().unwrap(), 96.0);
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value).unwrap(),
        Some(NbtSummaryValue::String("Custom horn".to_string()))
    );
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_banner_patterns_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID],
                banner_pattern_layers: vec![
                    BannerPatternLayerSummary {
                        registry_id: Some(4),
                        asset_id: None,
                        translation_key: None,
                        color_id: VANILLA_DYE_COLOR_RED_ID,
                    },
                    BannerPatternLayerSummary {
                        registry_id: None,
                        asset_id: Some("minecraft:stripe_bottom".to_string()),
                        translation_key: Some("block.minecraft.banner.stripe_bottom".to_string()),
                        color_id: VANILLA_DYE_COLOR_BLUE_ID,
                    },
                ],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "banner_patterns component value")
            .unwrap(),
    );
    assert_eq!(value.read_len().unwrap(), 2);
    assert_eq!(value.read_var_i32().unwrap(), 5);
    assert_eq!(value.read_var_i32().unwrap(), VANILLA_DYE_COLOR_RED_ID);
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(
        value.read_string(usize::MAX).unwrap(),
        "minecraft:stripe_bottom"
    );
    assert_eq!(
        value.read_string(usize::MAX).unwrap(),
        "block.minecraft.banner.stripe_bottom"
    );
    assert_eq!(value.read_var_i32().unwrap(), VANILLA_DYE_COLOR_BLUE_ID);
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_bees_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BEES_TYPE_ID],
                bees_present: true,
                bees_count: 1,
                bees: vec![BeeOccupantSummary {
                    entity_type_id: VANILLA_ENTITY_TYPE_BEE_ID,
                    entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
                    ticks_in_hive: -5,
                    min_ticks_in_hive: 2400,
                }],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "bees component value")
            .unwrap(),
    );
    assert_eq!(value.read_len().unwrap(), 1);
    assert_eq!(value.read_var_i32().unwrap(), VANILLA_ENTITY_TYPE_BEE_ID);
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value).unwrap(),
        Some(NbtSummaryValue::Compound(Vec::new()))
    );
    assert_eq!(value.read_var_i32().unwrap(), -5);
    assert_eq!(value.read_var_i32().unwrap(), 2400);
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_lodestone_tracker_component() {
    let target = LodestoneTargetSummary {
        dimension: "minecraft:the_nether".to_string(),
        pos: chunks::BlockPos {
            x: 12,
            y: -64,
            z: 34,
        },
    };
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID],
                lodestone_target: Some(target.clone()),
                lodestone_tracked: Some(false),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "lodestone_tracker component value")
            .unwrap(),
    );
    assert!(value.read_bool().unwrap());
    assert_eq!(value.read_string(usize::MAX).unwrap(), target.dimension);
    assert_eq!(
        chunks::decode_block_pos(value.read_i64().unwrap()),
        target.pos
    );
    assert!(!value.read_bool().unwrap());
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_direct_painting_variant_component() {
    let title_nbt = NbtSummaryValue::String("Gallery".to_string());
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID],
                painting_variant_direct: Some(PaintingVariantSummary {
                    width: 4,
                    height: 2,
                    asset_id: "minecraft:gallery".to_string(),
                    title: Some("Gallery".to_string()),
                    title_styled: Some(vec![StyledTextRun {
                        text: "Gallery".to_string(),
                        style: Default::default(),
                    }]),
                    title_nbt: Some(Box::new(title_nbt.clone())),
                    author: None,
                    author_styled: None,
                    author_nbt: None,
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "painting_variant component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(value.read_var_i32().unwrap(), 4);
    assert_eq!(value.read_var_i32().unwrap(), 2);
    assert_eq!(value.read_string(usize::MAX).unwrap(), "minecraft:gallery");
    assert!(value.read_bool().unwrap());
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value).unwrap(),
        Some(title_nbt)
    );
    assert!(!value.read_bool().unwrap());
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_direct_jukebox_playable_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID],
                jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(7),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    description: "Custom song".to_string(),
                    description_styled: Some(
                        vec![crate::StyledTextRun {
                            text: "Custom song".to_string(),
                            style: Default::default(),
                        }]
                        .into_boxed_slice(),
                    ),
                    description_nbt: Some(Box::new(NbtSummaryValue::String(
                        "Custom song".to_string(),
                    ))),
                    length_in_seconds_bits: 60.0_f32.to_bits(),
                    comparator_output: 7,
                })),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "jukebox_playable component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(value.read_var_i32().unwrap(), 8);
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value).unwrap(),
        Some(NbtSummaryValue::String("Custom song".to_string()))
    );
    assert_eq!(value.read_f32().unwrap(), 60.0);
    assert_eq!(value.read_var_i32().unwrap(), 7);
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_entity_data_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 3,
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                    crate::data_component_types::VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                    crate::data_component_types::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                ],
                entity_data_entity_type_id: Some(
                    crate::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                ),
                entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
                codec_backed_nbt: BTreeMap::from([(
                    crate::data_component_types::VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                    NbtSummaryValue::Compound(Vec::new()),
                )]),
                block_entity_data_type_id: Some(
                    crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
                ),
                block_entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            3, 0, // patch counts
            58, 4, 0x96, 0x01, 10, 0, // entity holder + empty compound
            59, 2, 10, 0, // bucket compound
            60, 3, 9, 10, 0, // block entity holder + empty compound
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_attribute_modifiers_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID],
                attribute_modifiers: vec![AttributeModifierSummary {
                    attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                    modifier_id: "minecraft:test".to_string(),
                    amount_bits: 1.5f64.to_bits(),
                    operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                    slot_id:
                        crate::equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
                    display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    display_text: Some("Override".to_string()),
                    display_text_styled: Some(vec![crate::StyledTextRun {
                        text: "Override".to_string(),
                        style: crate::ComponentStyle::default(),
                    }]),
                    display_text_nbt: Some(NbtSummaryValue::String("Override".to_string())),
                }],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            16, 39, // attribute_modifiers type and delimited value length
            1,  // modifier count
            2,  // attack_damage holder
            14, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b't', b'e', b's',
            b't', // modifier id
            0x3f, 0xf8, 0, 0, 0, 0, 0, 0, // amount = 1.5
            0, // add_value
            0, // any slot group
            2, // override display
            8, 0, 8, b'O', b'v', b'e', b'r', b'r', b'i', b'd', b'e', // component NBT
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_food_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID],
                food_nutrition: Some(6),
                food_saturation_bits: Some(7.2_f32.to_bits()),
                food_can_always_eat: true,
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            23, 6, // food type and delimited value length
            6, // nutrition
            0x40, 0xe6, 0x66, 0x66, // saturation = 7.2
            1,    // can always eat
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_glider_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            34, 0, // glider type and empty delimited Unit payload
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_tooltip_style_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID],
                tooltip_style: Some("example:rare".to_string()),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "tooltip_style component value")
            .unwrap(),
    );
    assert_eq!(value.read_string(32_767).unwrap(), "example:rare");
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_pot_decorations_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID],
                pot_decorations_item_ids: vec![VANILLA_ITEM_STONE_ID],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "pot_decorations component value")
            .unwrap(),
    );
    assert_eq!(value.read_len().unwrap(), 4);
    assert_eq!(value.read_var_i32().unwrap(), VANILLA_ITEM_STONE_ID);
    for _ in 0..3 {
        assert_eq!(value.read_var_i32().unwrap(), VANILLA_ITEM_BRICK_ID);
    }
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_provides_banner_patterns_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID],
                provides_banner_pattern_registry_ids: vec![0, 42],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "provides_banner_patterns component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 3);
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(value.read_var_i32().unwrap(), 42);
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_consume_effect_family_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
                ],
                consumable: Some(ConsumableSummary {
                    consume_seconds: 1.0,
                    animation: ItemUseAnimationSummary::Eat,
                }),
                consumable_sound: Some(SoundEventSummary {
                    registry_id: Some(0),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
                consumable_has_consume_particles: Some(true),
                consumable_effect_count: Some(0),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            2, 0, // patch counts
            24, 8, // consumable type and delimited value length
            0x3f, 0x80, 0, 0, // consume seconds = 1.0
            1, // eat animation
            1, // sound registry holder 0
            1, // has consume particles
            0, // no consume effects
            36, 1, // death_protection type and delimited value length
            0, // no death effects
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_weapon_interaction_family_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 3,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                ],
                piercing_weapon: Some(PiercingWeaponSummary {
                    deals_knockback: true,
                    dismounts: false,
                    sound: None,
                    hit_sound: None,
                }),
                kinetic_weapon: Some(KineticWeaponSummary {
                    contact_cooldown_ticks: 10,
                    delay_ticks: 0,
                    dismount_conditions: None,
                    knockback_conditions: None,
                    damage_conditions: None,
                    forward_movement_bits: 0.0_f32.to_bits(),
                    damage_multiplier_bits: 1.0_f32.to_bits(),
                    sound: None,
                    hit_sound: None,
                }),
                swing_animation: Some(SwingAnimationSummary {
                    animation_type: SwingAnimationTypeSummary::Stab,
                    duration: 13,
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            3, 0, // patch counts
            38, 4, // piercing_weapon type and delimited value length
            1, 0, // deals knockback, does not dismount
            0, 0, // no sound or hit sound
            39, 15, // kinetic_weapon type and delimited value length
            10, 0, // contact cooldown and delay
            0, 0, 0, // no dismount, knockback, or damage conditions
            0, 0, 0, 0, // forward movement = 0.0
            0x3f, 0x80, 0, 0, // damage multiplier = 1.0
            0, 0, // no sound or hit sound
            40, 2, // swing_animation type and delimited value length
            2, 13, // stab, duration
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_trim_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_TRIM_TYPE_ID],
                armor_trim_material_id: Some(0),
                armor_trim_pattern_id: Some(17),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            56, 2, // trim type and delimited value length
            1, 18, // material holder 0 and pattern holder 17, both offset by one
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_direct_provides_trim_material_component() {
    let description_nbt = NbtSummaryValue::String("Ruby".to_string());
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID],
                provides_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                    asset_name: "ruby".to_string(),
                    override_armor_assets: BTreeMap::from([(
                        "minecraft:netherite".to_string(),
                        "ruby_darker".to_string(),
                    )]),
                    description: "Ruby".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Ruby".to_string(),
                            style: Default::default(),
                        }]
                        .into_boxed_slice(),
                    ),
                    description_nbt: Some(Box::new(description_nbt.clone())),
                })),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "provides_trim_material component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(value.read_string(usize::MAX).unwrap(), "ruby");
    assert_eq!(value.read_len().unwrap(), 1);
    assert_eq!(
        value.read_string(usize::MAX).unwrap(),
        "minecraft:netherite"
    );
    assert_eq!(value.read_string(usize::MAX).unwrap(), "ruby_darker");
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value).unwrap(),
        Some(description_nbt)
    );
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_direct_break_sound_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID],
                break_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:item.custom_break".to_string()),
                    fixed_range_bits: Some(12.5_f32.to_bits()),
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "break_sound component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(
        value.read_string(usize::MAX).unwrap(),
        "example:item.custom_break"
    );
    assert!(value.read_bool().unwrap());
    assert_eq!(value.read_f32().unwrap().to_bits(), 12.5_f32.to_bits());
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_map_id_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID],
                map_id: Some(42),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            46, 1,  // map_id type and delimited value length
            42, // map id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_map_post_processing_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID],
                map_post_processing: Some(MapPostProcessingSummary::Scale),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            48, 1, // map_post_processing type and delimited value length
            1, // SCALE
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_dye_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DYE_TYPE_ID],
                dye_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            43, 1,  // dye type and delimited value length
            14, // RED
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_dyed_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID],
                dyed_color: Some(0x12_34_56),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            44, 4, // dyed_color type and delimited value length
            0, 0x12, 0x34, 0x56, // fixed-width RGB int
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_map_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID],
                map_color: Some(0x45_67_89),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            45, 4, // map_color type and delimited value length
            0, 0x45, 0x67, 0x89, // fixed-width RGB int
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_firework_family_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
                    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
                ],
                firework_explosion_shape: Some(FireworkExplosionShapeSummary::Star),
                firework_explosion_colors: vec![0x11_22_33],
                firework_explosion_has_trail: Some(true),
                firework_explosion_has_twinkle: Some(false),
                fireworks_flight_duration: Some(2),
                fireworks_explosions_count: Some(1),
                fireworks_explosions: vec![FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::Burst,
                    colors: vec![-1],
                    fade_colors: vec![0x44_55_66],
                    has_trail: false,
                    has_twinkle: true,
                }],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            2, 0, // patch counts
            68, 9, // firework_explosion type and delimited value length
            2, 1, 0, 0x11, 0x22, 0x33, // star and one color
            0, 1, 0, // no fade colors, trail, no twinkle
            69, 15, // fireworks type and delimited value length
            2, 1, // flight duration and one explosion
            4, 1, 0xff, 0xff, 0xff, 0xff, // burst and one signed color
            1, 0, 0x44, 0x55, 0x66, // one fade color
            0, 1, // no trail, twinkle
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_potion_effect_family_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 3,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
                ],
                potion_id: Some(3),
                potion_custom_color: Some(0x11_22_33),
                potion_custom_effect_count: Some(1),
                potion_custom_effects: vec![MobEffectInstanceSummary {
                    effect_id: 2,
                    amplifier: 1,
                    duration: 200,
                    ambient: false,
                    show_particles: true,
                    show_icon: false,
                    hidden_effect: None,
                }],
                potion_custom_name: Some("swift".to_string()),
                potion_duration_scale_bits: Some(0.125_f32.to_bits()),
                suspicious_stew_effects: vec![
                    SuspiciousStewEffectSummary {
                        effect_id: 0,
                        duration: 160,
                    },
                    SuspiciousStewEffectSummary {
                        effect_id: 1,
                        duration: -1,
                    },
                ],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            3, 0, // patch counts
            51, 23, // potion_contents type and delimited value length
            1, 3, // present potion holder
            1, 0, 0x11, 0x22, 0x33, // present custom color
            1,    // one custom effect
            2, 1, 0xc8, 1, // effect holder, amplifier, duration
            0, 1, 0, 0, // ambient, particles, icon, no hidden effect
            1, 5, b's', b'w', b'i', b'f', b't', // present custom name
            52, 4, 0x3e, 0, 0, 0, // potion duration scale = 0.125
            53, 10, // suspicious_stew_effects type and delimited value length
            2, 0, 0xa0, 1, // two entries, first holder and duration
            1, 0xff, 0xff, 0xff, 0xff, 0x0f, // second holder and -1 duration
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_blocks_attacks_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID],
                blocks_attacks: Some(BlocksAttacksSummary {
                    block_delay_seconds_bits: 0.25_f32.to_bits(),
                    disable_cooldown_scale_bits: 1.0_f32.to_bits(),
                    damage_reductions: Vec::new(),
                    item_damage: ItemDamageFunctionSummary {
                        threshold_bits: 1.0_f32.to_bits(),
                        base_bits: 0.0_f32.to_bits(),
                        factor_bits: 1.0_f32.to_bits(),
                    },
                    bypassed_by: None,
                    block_sound: None,
                    disable_sound: None,
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            37, 24, // blocks_attacks type and delimited value length
            0x3e, 0x80, 0, 0, // block delay = 0.25
            0x3f, 0x80, 0, 0, // disable cooldown scale = 1.0
            0, // no damage reductions
            0x3f, 0x80, 0, 0, // item damage threshold = 1.0
            0, 0, 0, 0, // item damage base = 0.0
            0x3f, 0x80, 0, 0, // item damage factor = 1.0
            0, 0, 0, // no bypassed-by set or sounds
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_written_book_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID],
                written_book: Some(WrittenBookContentSummary {
                    title: "Guide".to_string(),
                    title_filter: None,
                    author: "Alex".to_string(),
                    generation: 1,
                    pages: vec!["Raw".to_string()],
                    page_filters: vec![Some("Safe".to_string())],
                    resolved: true,
                }),
                written_book_pages_nbt: vec![Some(NbtSummaryValue::String("Raw".to_string()))],
                written_book_page_filters_nbt: vec![Some(NbtSummaryValue::String(
                    "Safe".to_string(),
                ))],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            55, 29, // written_book_content type and delimited value length
            5, b'G', b'u', b'i', b'd', b'e', 0, // raw title, no filtered title
            4, b'A', b'l', b'e', b'x', // author
            1,    // generation
            1,    // page count
            8, 0, 3, b'R', b'a', b'w', // raw page Component NBT
            1, 8, 0, 4, b'S', b'a', b'f', b'e', // filtered page
            1,    // resolved
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_lore_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_LORE_TYPE_ID],
                lore: vec!["First".to_string(), "Second".to_string()],
                lore_styled: vec![
                    vec![crate::StyledTextRun {
                        text: "First".to_string(),
                        style: crate::ComponentStyle::default(),
                    }],
                    vec![crate::StyledTextRun {
                        text: "Second".to_string(),
                        style: crate::ComponentStyle::default(),
                    }],
                ],
                lore_nbt: vec![
                    NbtSummaryValue::String("First".to_string()),
                    NbtSummaryValue::String("Second".to_string()),
                ],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            11, 18, // lore type and delimited value length
            2,  // line count
            8, 0, 5, b'F', b'i', b'r', b's', b't', // first component
            8, 0, 6, b'S', b'e', b'c', b'o', b'n', b'd', // second component
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_name_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                    VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
                ],
                custom_name: Some("Custom".to_string()),
                custom_name_styled: Some(vec![crate::StyledTextRun {
                    text: "Custom".to_string(),
                    style: crate::ComponentStyle::default(),
                }]),
                custom_name_nbt: Some(NbtSummaryValue::String("Custom".to_string())),
                item_name: Some("Item".to_string()),
                item_name_styled: Some(vec![crate::StyledTextRun {
                    text: "Item".to_string(),
                    style: crate::ComponentStyle::default(),
                }]),
                item_name_nbt: Some(NbtSummaryValue::String("Item".to_string())),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            2, 0, // patch counts
            6, 9, // custom_name type and delimited value length
            8, 0, 6, b'C', b'u', b's', b't', b'o', b'm', // custom component
            9, 7, // item_name type and delimited value length
            8, 0, 4, b'I', b't', b'e', b'm', // item component
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_use_effects_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID],
                use_effects: Some(crate::packets::UseEffectsSummary {
                    can_sprint: true,
                    interact_vibrations: false,
                    speed_multiplier: 0.25,
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            5, 6, // use_effects type and delimited value length
            1, 0, // can sprint; does not emit interaction vibrations
            0x3e, 0x80, 0, 0, // speed multiplier = 0.25
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_minimum_attack_charge_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
                minimum_attack_charge_bits: Some(0.75_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            7, 4, // minimum_attack_charge type and delimited value length
            0x3f, 0x40, 0, 0, // minimum attack charge = 0.75
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_damage_type_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID],
                damage_type_id: Some(300),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            8, 2, // damage_type component and delimited value length
            0xac, 2, // session holder id 300
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_item_enchantment_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                    VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
                ],
                enchantments: vec![crate::packets::ItemEnchantmentSummary {
                    holder_id: 37,
                    level: 4,
                }],
                stored_enchantments: vec![crate::packets::ItemEnchantmentSummary {
                    holder_id: 13,
                    level: 5,
                }],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            2, 0, // patch counts
            13, 3, // enchantments type and delimited value length
            1, 37, 4, // one level-four enchantment
            42, 3, // stored_enchantments type and delimited value length
            1, 13, 5, // one stored level-five enchantment
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_writable_book_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID],
                writable_book_pages: vec!["raw".to_string(), "second".to_string()],
                writable_book_page_filters: vec![Some("safe".to_string()), None],
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            54, 19, // writable_book_content type and delimited value length
            2,  // page count
            3, b'r', b'a', b'w', // raw first page
            1, 4, b's', b'a', b'f', b'e', // filtered first page
            6, b's', b'e', b'c', b'o', b'n', b'd', // raw second page
            0,    // no filtered second page
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_profile_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
                ],
                profile: Some(ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::GameProfile,
                    uuid: Some(Uuid::from_u128(1)),
                    name: Some("Alex".to_string()),
                    properties: Vec::new(),
                    profile_textures: None,
                    skin_patch: PlayerSkinPatchSummary::default(),
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            70, 27, // profile type and value length
            1,  // Either.left(GameProfile)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
            4, b'A', b'l', b'e', b'x', // name
            0,    // property count
            0, 0, 0, 0, // empty skin patch
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_drop_empty_stack() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: -1,
        item: ItemStackSummary::empty(),
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), -1);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn set_creative_mode_slot_normalizes_air_item_to_empty_stack() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(crate::item_types::VANILLA_ITEM_AIR_ID),
            count: 99,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(99),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(payload, [0, 36, 0]);
}

#[test]
fn encodes_set_creative_mode_slot_adventure_mode_predicate_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
                can_place_on: Some(AdventureModePredicateSummary {
                    unknown: false,
                    block_registry_ids: vec![1],
                    unresolved_block_tag_count: 0,
                }),
                can_place_on_persistent: Some(AdventureModePredicateDataSummary {
                    predicates: vec![BlockPredicateSummary {
                        blocks: Some(BlockHolderSetSummary {
                            registry_ids: vec![1],
                            tag: None,
                        }),
                        ..BlockPredicateSummary::default()
                    }],
                }),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            14, 8, // can_place_on type and delimited value length
            1, // predicate count
            1, // blocks present
            2, 1, // one direct block holder
            0, // no state predicate
            0, // no nbt predicate
            0, // no exact component predicates
            0, // no partial component predicates
        ]
    );
}

#[test]
fn set_creative_mode_slot_rejects_values_rejected_by_the_untrusted_stack_codec() {
    let stack_with_patch = |count, component_patch| SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count,
            component_patch,
        },
    };
    let max_stack_size_patch = |value| DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
        max_stack_size: Some(value),
        ..DataComponentPatchSummary::default()
    };
    let max_damage_patch = |value| DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID],
        max_damage: Some(value),
        ..DataComponentPatchSummary::default()
    };
    let damage_patch = |value| DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
        damage: Some(value),
        ..DataComponentPatchSummary::default()
    };
    let damage_type_patch = |holder_id| DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID],
        damage_type_id: Some(holder_id),
        ..DataComponentPatchSummary::default()
    };
    let use_effects_patch = |speed_multiplier| DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID],
        use_effects: Some(crate::packets::UseEffectsSummary {
            can_sprint: false,
            interact_vibrations: true,
            speed_multiplier,
        }),
        ..DataComponentPatchSummary::default()
    };
    let minimum_attack_charge_patch = |value: f32| DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
        minimum_attack_charge_bits: Some(value.to_bits()),
        ..DataComponentPatchSummary::default()
    };
    let enchantments_patch = |type_id, level| {
        let mut patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![type_id],
            ..DataComponentPatchSummary::default()
        };
        let value = vec![crate::packets::ItemEnchantmentSummary {
            holder_id: 0,
            level,
        }];
        if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
            patch.enchantments = value;
        } else {
            patch.stored_enchantments = value;
        }
        patch
    };
    let fireworks_patch = |value| DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID],
        fireworks_flight_duration: Some(value),
        fireworks_explosions_count: Some(0),
        ..DataComponentPatchSummary::default()
    };
    let potion_duration_scale_patch = |bits| DataComponentPatchSummary {
        added: 1,
        added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID],
        potion_duration_scale_bits: Some(bits),
        ..DataComponentPatchSummary::default()
    };
    let adventure_mode_patch =
        |tooltip: AdventureModePredicateSummary, predicates: Vec<BlockPredicateSummary>| {
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
                can_place_on: Some(tooltip),
                can_place_on_persistent: Some(AdventureModePredicateDataSummary { predicates }),
                ..DataComponentPatchSummary::default()
            }
        };

    for (packet, expected_error) in [
        (
            stack_with_patch(100, DataComponentPatchSummary::default()),
            "count 100 exceeds",
        ),
        (
            stack_with_patch(1, max_stack_size_patch(0)),
            "max_stack_size must be in 1..=99",
        ),
        (
            stack_with_patch(1, max_stack_size_patch(100)),
            "max_stack_size must be in 1..=99",
        ),
        (
            stack_with_patch(1, max_damage_patch(0)),
            "max_damage must be positive",
        ),
        (
            stack_with_patch(1, max_damage_patch(-1)),
            "max_damage must be positive",
        ),
        (
            stack_with_patch(1, damage_patch(-1)),
            "damage must be non-negative",
        ),
        (
            stack_with_patch(1, damage_type_patch(-1)),
            "damage_type has negative holder id -1",
        ),
        (
            stack_with_patch(1, use_effects_patch(-0.0)),
            "use_effects speed_multiplier must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, use_effects_patch(-1.0)),
            "use_effects speed_multiplier must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, use_effects_patch(1.000_000_1)),
            "use_effects speed_multiplier must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, use_effects_patch(f32::INFINITY)),
            "use_effects speed_multiplier must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, use_effects_patch(f32::NAN)),
            "use_effects speed_multiplier must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, minimum_attack_charge_patch(-0.0)),
            "minimum_attack_charge must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, minimum_attack_charge_patch(-1.0)),
            "minimum_attack_charge must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, minimum_attack_charge_patch(1.000_000_1)),
            "minimum_attack_charge must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, minimum_attack_charge_patch(f32::INFINITY)),
            "minimum_attack_charge must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(1, minimum_attack_charge_patch(f32::NAN)),
            "minimum_attack_charge must be in 0.0..=1.0",
        ),
        (
            stack_with_patch(
                1,
                enchantments_patch(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, 0),
            ),
            "enchantments levels must be in 1..=255",
        ),
        (
            stack_with_patch(
                1,
                enchantments_patch(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, -1),
            ),
            "enchantments levels must be in 1..=255",
        ),
        (
            stack_with_patch(
                1,
                enchantments_patch(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, 256),
            ),
            "enchantments levels must be in 1..=255",
        ),
        (
            stack_with_patch(
                1,
                enchantments_patch(VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID, 0),
            ),
            "stored_enchantments levels must be in 1..=255",
        ),
        (
            stack_with_patch(1, fireworks_patch(-1)),
            "fireworks flight_duration must be in 0..=255",
        ),
        (
            stack_with_patch(1, fireworks_patch(256)),
            "fireworks flight_duration must be in 0..=255",
        ),
        (
            stack_with_patch(1, potion_duration_scale_patch((-0.0_f32).to_bits())),
            "potion_duration_scale must be in 0.0..=Float.MAX_VALUE",
        ),
        (
            stack_with_patch(1, potion_duration_scale_patch((-1.0_f32).to_bits())),
            "potion_duration_scale must be in 0.0..=Float.MAX_VALUE",
        ),
        (
            stack_with_patch(1, potion_duration_scale_patch(f32::INFINITY.to_bits())),
            "potion_duration_scale must be in 0.0..=Float.MAX_VALUE",
        ),
        (
            stack_with_patch(1, potion_duration_scale_patch(f32::NAN.to_bits())),
            "potion_duration_scale must be in 0.0..=Float.MAX_VALUE",
        ),
        (
            stack_with_patch(
                1,
                adventure_mode_patch(AdventureModePredicateSummary::default(), Vec::new()),
            ),
            "can_place_on predicate list must not be empty",
        ),
        (
            stack_with_patch(
                1,
                adventure_mode_patch(
                    AdventureModePredicateSummary {
                        unknown: true,
                        ..AdventureModePredicateSummary::default()
                    },
                    vec![BlockPredicateSummary {
                        state: Some(vec![
                            StatePropertyMatcherSummary {
                                name: "axis".to_string(),
                                value: StatePropertyValueMatcherSummary::Exact("x".to_string()),
                            },
                            StatePropertyMatcherSummary {
                                name: "axis".to_string(),
                                value: StatePropertyValueMatcherSummary::Exact("y".to_string()),
                            },
                        ]),
                        ..BlockPredicateSummary::default()
                    }],
                ),
            ),
            "duplicate state property `axis`",
        ),
    ] {
        let error = encode_play_set_creative_mode_slot(&packet).unwrap_err();
        assert!(
            error.to_string().contains(expected_error),
            "{error} did not contain {expected_error:?}"
        );
    }
}

#[test]
fn set_creative_mode_slot_accepts_untrusted_stack_codec_boundaries() {
    for (flight_duration, duration_scale_bits, bounded_float, enchantment_level) in [
        (0, 0.0_f32.to_bits(), 0.0, 1),
        (255, f32::MAX.to_bits(), 1.0, 255),
    ] {
        encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(42),
                count: 99,
                component_patch: DataComponentPatchSummary {
                    added: 9,
                    added_type_ids: vec![
                        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
                        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
                        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                    ],
                    max_stack_size: Some(99),
                    max_damage: Some(1),
                    damage: Some(0),
                    use_effects: Some(crate::packets::UseEffectsSummary {
                        can_sprint: false,
                        interact_vibrations: true,
                        speed_multiplier: bounded_float,
                    }),
                    minimum_attack_charge_bits: Some(bounded_float.to_bits()),
                    enchantments: vec![crate::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: enchantment_level,
                    }],
                    stored_enchantments: vec![crate::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: enchantment_level,
                    }],
                    fireworks_flight_duration: Some(flight_duration),
                    fireworks_explosions_count: Some(0),
                    potion_duration_scale_bits: Some(duration_scale_bits),
                    ..DataComponentPatchSummary::default()
                },
            },
        })
        .unwrap();
    }
}

#[test]
fn set_creative_mode_slot_enforces_persistent_counts_for_recursive_item_components() {
    fn recursive_patch(component_type_id: i32, count: i32) -> DataComponentPatchSummary {
        let template = crate::packets::ItemStackTemplateSummary {
            item_id: 42,
            count,
            component_patch: DataComponentPatchSummary::default(),
        };
        let mut patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![component_type_id],
            ..DataComponentPatchSummary::default()
        };
        match component_type_id {
            crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => {
                patch.use_remainder = Some(Box::new(template));
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
                patch.charged_projectiles_items = vec![template];
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
                patch.bundle_contents_items = vec![template];
                patch.bundle_contents_item_count = Some(1);
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => {
                patch.container_slots = vec![crate::packets::ItemContainerSlotSummary {
                    slot: 0,
                    item: template.clone(),
                }];
                patch.container_items = vec![template];
                patch.container_item_count = Some(1);
                patch.container_slot_count = Some(1);
            }
            _ => unreachable!("recursive component type"),
        }
        patch
    }

    let component_type_ids = [
        crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        crate::data_component_types::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        crate::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        crate::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
    ];
    for component_type_id in component_type_ids {
        for count in [1, 99] {
            encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
                slot_num: 36,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: recursive_patch(component_type_id, count),
                },
            })
            .unwrap();
        }
        for count in [-1, 0, 100] {
            let error = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
                slot_num: 36,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: recursive_patch(component_type_id, count),
                },
            })
            .unwrap_err();
            assert!(
                error.to_string().contains(&format!(
                    "nested item stack count {count} must be in 1..=99"
                )),
                "{error}"
            );
        }
    }
}

#[test]
fn set_creative_mode_slot_rejects_an_out_of_range_builtin_item_holder() {
    let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;
    let error = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(invalid_item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        },
    })
    .unwrap_err();

    assert!(error
        .to_string()
        .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));
}

#[test]
fn play_serverbound_inventory_packet_ids_match_vanilla_26_1_registration_order() {
    assert_eq!(ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED, 3);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK, 17);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_CLICK, 18);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_CLOSE, 19);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED, 20);
    assert_eq!(ids::play::SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST, 23);
    assert_eq!(ids::play::SERVERBOUND_EDIT_BOOK, 24);
    assert_eq!(ids::play::SERVERBOUND_RENAME_ITEM, 48);
    assert_eq!(ids::play::SERVERBOUND_SEEN_ADVANCEMENTS, 50);
    assert_eq!(ids::play::SERVERBOUND_SELECT_TRADE, 51);
    assert_eq!(ids::play::SERVERBOUND_SET_BEACON, 52);
    assert_eq!(ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT, 56);
    assert_eq!(ids::play::SERVERBOUND_SIGN_UPDATE, 61);
}

#[test]
fn play_serverbound_interaction_packet_ids_match_vanilla_26_1_registration_order() {
    assert_eq!(ids::play::SERVERBOUND_ATTACK, 1);
    assert_eq!(ids::play::SERVERBOUND_BLOCK_ENTITY_TAG_QUERY, 2);
    assert_eq!(ids::play::SERVERBOUND_CHANGE_DIFFICULTY, 4);
    assert_eq!(ids::play::SERVERBOUND_CHANGE_GAME_MODE, 5);
    assert_eq!(ids::play::SERVERBOUND_CHAT_ACK, 6);
    assert_eq!(ids::play::SERVERBOUND_CHAT_COMMAND, 7);
    assert_eq!(ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED, 8);
    assert_eq!(ids::play::SERVERBOUND_CHAT, 9);
    assert_eq!(ids::play::SERVERBOUND_ENTITY_TAG_QUERY, 25);
    assert_eq!(ids::play::SERVERBOUND_INTERACT, 26);
    assert_eq!(ids::play::SERVERBOUND_LOCK_DIFFICULTY, 29);
    assert_eq!(ids::play::SERVERBOUND_PADDLE_BOAT, 35);
    assert_eq!(ids::play::SERVERBOUND_PICK_ITEM_FROM_ENTITY, 37);
    assert_eq!(ids::play::SERVERBOUND_PING_REQUEST, 38);
    assert_eq!(ids::play::SERVERBOUND_SPECTATE_ENTITY, 62);
    assert_eq!(ids::play::SERVERBOUND_TELEPORT_TO_ENTITY, 64);
}

#[test]
fn play_serverbound_player_state_packet_ids_match_vanilla_26_1_registration_order() {
    assert_eq!(ids::play::SERVERBOUND_PLACE_RECIPE, 39);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_ABILITIES, 40);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_ACTION, 41);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_COMMAND, 42);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_INPUT, 43);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_LOADED, 44);
    assert_eq!(ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS, 46);
    assert_eq!(ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE, 47);
}

#[test]
fn encodes_container_inventory_packets() {
    let (id, payload) = encode_play_container_button_click(ContainerButtonClick {
        container_id: 7,
        button_id: 2,
    });
    assert_eq!(id, ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert!(decoder.is_empty());

    let mut added_components = BTreeMap::new();
    added_components.insert(10, 0x0102_0304);
    let mut removed_components = BTreeSet::new();
    removed_components.insert(20);
    let mut changed_slots = BTreeMap::new();
    changed_slots.insert(
        5,
        HashedStack::Item(HashedItemStack {
            item_id: 42,
            count: 64,
            components: HashedComponentPatch {
                added_components,
                removed_components,
            },
        }),
    );
    let (id, payload) = encode_play_container_click(ContainerClick {
        container_id: 7,
        state_id: 33,
        slot_num: 5,
        button_num: 1,
        input: ContainerInput::Pickup,
        changed_slots,
        carried_item: HashedStack::empty(),
    })
    .unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_CONTAINER_CLICK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(decoder.read_var_i32().unwrap(), 33);
    assert_eq!(decoder.read_i16().unwrap(), 5);
    assert_eq!(decoder.read_i8().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_i16().unwrap(), 5);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 64);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 10);
    assert_eq!(decoder.read_i32().unwrap(), 0x0102_0304);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 20);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_container_close(ContainerCloseRequest { container_id: 7 });
    assert_eq!(id, ids::play::SERVERBOUND_CONTAINER_CLOSE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_container_slot_state_changed(ContainerSlotStateChanged {
        slot_id: 12,
        container_id: 7,
        new_state: true,
    });
    assert_eq!(id, ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 12);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn container_click_rejects_vanilla_bounded_collections_before_encoding() {
    let click = |changed_slots, carried_item| ContainerClick {
        container_id: 7,
        state_id: 33,
        slot_num: 5,
        button_num: 0,
        input: ContainerInput::Pickup,
        changed_slots,
        carried_item,
    };

    let changed_slots = (0..=128).map(|slot| (slot, HashedStack::empty())).collect();
    assert!(encode_play_container_click(click(changed_slots, HashedStack::empty())).is_err());

    let added_components = (0..=256).map(|id| (id, id)).collect();
    let oversized_added = HashedStack::Item(HashedItemStack {
        item_id: 42,
        count: 1,
        components: HashedComponentPatch {
            added_components,
            removed_components: BTreeSet::new(),
        },
    });
    assert!(encode_play_container_click(click(BTreeMap::new(), oversized_added)).is_err());

    let removed_components = (0..=256).collect();
    let oversized_removed = HashedStack::Item(HashedItemStack {
        item_id: 42,
        count: 1,
        components: HashedComponentPatch {
            added_components: BTreeMap::new(),
            removed_components,
        },
    });
    assert!(encode_play_container_click(click(BTreeMap::new(), oversized_removed)).is_err());

    let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;
    let invalid_item = HashedStack::Item(HashedItemStack {
        item_id: invalid_item_id,
        count: 1,
        components: HashedComponentPatch::default(),
    });
    let error = encode_play_container_click(click(BTreeMap::new(), invalid_item)).unwrap_err();
    assert!(error
        .to_string()
        .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));
}

#[test]
fn encodes_place_recipe_packet() {
    let (id, payload) = encode_play_place_recipe(PlaceRecipeCommand {
        container_id: 7,
        recipe_index: 123,
        use_max_items: true,
    });
    assert_eq!(id, ids::play::SERVERBOUND_PLACE_RECIPE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_recipe_book_change_settings_packet() {
    let (id, payload) = encode_play_recipe_book_change_settings(RecipeBookChangeSettingsCommand {
        book_type: RecipeBookType::BlastFurnace,
        open: true,
        filtering: false,
    });
    assert_eq!(id, ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert!(decoder.read_bool().unwrap());
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_recipe_book_seen_recipe_packet() {
    let (id, payload) = encode_play_recipe_book_seen_recipe(RecipeBookSeenRecipeCommand {
        recipe: RecipeDisplayId { index: 321 },
    });
    assert_eq!(id, ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 321);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_change_difficulty_packet() {
    let (id, payload) = encode_play_change_difficulty(ChangeDifficultyCommand {
        difficulty: Difficulty::Hard,
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHANGE_DIFFICULTY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_change_game_mode_packet() {
    let (id, payload) = encode_play_change_game_mode(ChangeGameModeCommand {
        game_mode: GameType::Spectator,
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHANGE_GAME_MODE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_chat_acknowledgement_packet() {
    let (id, payload) = encode_play_chat_acknowledgement(ChatAcknowledgement { offset: 65 });

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_ACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 65);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_lock_difficulty_packet() {
    let (id, payload) = encode_play_lock_difficulty(LockDifficultyCommand { locked: true });

    assert_eq!(id, ids::play::SERVERBOUND_LOCK_DIFFICULTY);
    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_block_entity_tag_query_packet() {
    let pos = BlockPos {
        x: -5,
        y: 70,
        z: 12,
    };
    let (id, payload) = encode_play_block_entity_tag_query(BlockEntityTagQuery {
        transaction_id: 7,
        pos,
    });

    assert_eq!(id, ids::play::SERVERBOUND_BLOCK_ENTITY_TAG_QUERY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(
        decoder.read_i64().unwrap(),
        encode_block_pos(pos.x, pos.y, pos.z)
    );
    assert!(decoder.is_empty());
}

#[test]
fn encodes_entity_tag_query_packet() {
    let (id, payload) = encode_play_entity_tag_query(EntityTagQuery {
        transaction_id: 8,
        entity_id: 1234,
    });

    assert_eq!(id, ids::play::SERVERBOUND_ENTITY_TAG_QUERY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 8);
    assert_eq!(decoder.read_var_i32().unwrap(), 1234);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_edit_book_unsigned_packet() {
    let (id, payload) = encode_play_edit_book(&EditBook {
        slot: 3,
        pages: vec!["page 1".to_string(), "page 2".to_string()],
        title: None,
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_EDIT_BOOK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(decoder.read_string(1024).unwrap(), "page 1");
    assert_eq!(decoder.read_string(1024).unwrap(), "page 2");
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_edit_book_signed_packet() {
    let (id, payload) = encode_play_edit_book(&EditBook {
        slot: 5,
        pages: vec!["final page".to_string()],
        title: Some("Field Notes".to_string()),
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_EDIT_BOOK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_string(1024).unwrap(), "final page");
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_string(32).unwrap(), "Field Notes");
    assert!(decoder.is_empty());
}

#[test]
fn edit_book_rejects_vanilla_collection_and_utf16_limits() {
    let packet = |pages, title| EditBook {
        slot: 3,
        pages,
        title,
    };
    assert!(encode_play_edit_book(&packet(vec![String::new(); 101], None)).is_err());
    assert!(encode_play_edit_book(&packet(vec!["😀".repeat(513)], None)).is_err());
    assert!(encode_play_edit_book(&packet(vec![String::new()], Some("😀".repeat(17)))).is_err());
}

#[test]
fn encodes_rename_item_packet() {
    let (id, payload) = encode_play_rename_item(&RenameItem {
        name: "Sharp Pick".to_string(),
    });

    assert_eq!(id, ids::play::SERVERBOUND_RENAME_ITEM);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "Sharp Pick");
    assert!(decoder.is_empty());
}

#[test]
fn encodes_seen_advancements_opened_tab_packet() {
    let (id, payload) = encode_play_seen_advancements(&SeenAdvancements::OpenedTab {
        tab: "minecraft:story/root".to_string(),
    });

    assert_eq!(id, ids::play::SERVERBOUND_SEEN_ADVANCEMENTS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:story/root");
    assert!(decoder.is_empty());
}

#[test]
fn encodes_seen_advancements_closed_screen_packet() {
    let (id, payload) = encode_play_seen_advancements(&SeenAdvancements::ClosedScreen);

    assert_eq!(id, ids::play::SERVERBOUND_SEEN_ADVANCEMENTS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_select_trade_packet() {
    let (id, payload) = encode_play_select_trade(SelectTradeCommand { item: 2 });
    assert_eq!(id, ids::play::SERVERBOUND_SELECT_TRADE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_beacon_packet_with_effects() {
    let (id, payload) = encode_play_set_beacon(SetBeacon {
        primary_effect: Some(1),
        secondary_effect: Some(10),
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_BEACON);
    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 10);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_beacon_packet_without_secondary_effect() {
    let (id, payload) = encode_play_set_beacon(SetBeacon {
        primary_effect: Some(3),
        secondary_effect: None,
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_BEACON);
    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn set_beacon_rejects_holders_outside_the_builtin_mob_effect_registry() {
    let invalid_effect_id = crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32;
    let error = encode_play_set_beacon(SetBeacon {
        primary_effect: Some(1),
        secondary_effect: Some(invalid_effect_id),
    })
    .unwrap_err();

    assert!(error.to_string().contains(&format!(
        "invalid vanilla mob effect holder id {invalid_effect_id}"
    )));
}

#[test]
fn encodes_select_bundle_item_packet() {
    let (id, payload) = encode_play_select_bundle_item(SelectBundleItem {
        slot_id: 7,
        selected_item_index: 2,
    });
    assert_eq!(id, ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED);
    assert_eq!(payload, vec![0x07, 0x02]);
}

#[test]
fn encodes_select_bundle_item_unselect_packet() {
    let (id, payload) = encode_play_select_bundle_item(SelectBundleItem {
        slot_id: 7,
        selected_item_index: -1,
    });
    assert_eq!(id, ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED);
    assert_eq!(payload, vec![0x07, 0xff, 0xff, 0xff, 0xff, 0x0f]);
}

#[test]
fn encodes_player_input_flags() {
    let (id, payload) = encode_play_player_input(PlayerInput {
        forward: true,
        backward: false,
        left: true,
        right: false,
        jump: true,
        shift: true,
        sprint: false,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_INPUT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_u8().unwrap(), 0b0011_0101);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_player_abilities_flying_bit() {
    let (id, payload) = encode_play_player_abilities(PlayerAbilitiesCommand { flying: true });
    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ABILITIES);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_u8().unwrap(), 0x02);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_abilities(PlayerAbilitiesCommand { flying: false });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_u8().unwrap(), 0x00);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_player_command_actions() {
    let (id, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 5,
        action: PlayerCommandAction::StopSleeping,
        data: 0,
    });
    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_COMMAND);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 1234,
        action: PlayerCommandAction::StartSprinting,
        data: 0,
    });
    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_COMMAND);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1234);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_command(PlayerCommand {
        entity_id: -7,
        action: PlayerCommandAction::StopSprinting,
        data: 0,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), -7);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 99,
        action: PlayerCommandAction::StartRidingJump,
        data: 20,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 99);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(decoder.read_var_i32().unwrap(), 20);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 42,
        action: PlayerCommandAction::OpenInventory,
        data: 0,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 42,
        action: PlayerCommandAction::StartFallFlying,
        data: 0,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 6);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_player_action_packet() {
    let (id, payload) = encode_play_player_action(PlayerAction {
        action: PlayerActionKind::StartDestroyBlock,
        pos: BlockPos {
            x: 34,
            y: -12,
            z: -45,
        },
        direction: Direction::North,
        sequence: 7,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ACTION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos {
            x: 34,
            y: -12,
            z: -45,
        }
    );
    assert_eq!(decoder.read_u8().unwrap(), 2);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_all_player_action_kinds() {
    let pos = BlockPos {
        x: -12,
        y: 319,
        z: 45,
    };
    for entry in crate::player_action_kinds::VANILLA_PLAYER_ACTION_KINDS {
        let sequence = entry.ordinal + 10;
        let (id, payload) = encode_play_player_action(PlayerAction {
            action: entry.kind,
            pos,
            direction: Direction::East,
            sequence,
        });

        assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ACTION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), entry.ordinal);
        assert_eq!(
            chunks::decode_block_pos(decoder.read_i64().unwrap()),
            pos,
            "{}",
            entry.constant_name
        );
        assert_eq!(decoder.read_u8().unwrap(), 5, "{}", entry.constant_name);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            sequence,
            "{}",
            entry.constant_name
        );
        assert!(decoder.is_empty(), "{}", entry.constant_name);
    }
}

#[test]
fn encodes_stab_player_action_packet() {
    let (id, payload) = encode_play_player_action(PlayerAction {
        action: PlayerActionKind::Stab,
        pos: BlockPos { x: 0, y: 0, z: 0 },
        direction: Direction::Down,
        sequence: 0,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ACTION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos { x: 0, y: 0, z: 0 }
    );
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_chat_command_packet() {
    let (id, payload) = encode_play_chat_command(&ChatCommand {
        command: "give @p minecraft:stone".to_string(),
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_COMMAND);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        decoder.read_string(32767).unwrap(),
        "give @p minecraft:stone"
    );
    assert!(decoder.is_empty());
}

#[test]
fn encodes_signed_chat_command_packet_with_empty_argument_signatures() {
    let mut packet = ChatCommandSigned::unsigned_arguments(
        "say hello",
        1_717_986_918_300,
        0x0102_0304_0506_0708,
    );
    packet.last_seen_messages = LastSeenMessagesUpdate {
        offset: 3,
        acknowledged: (1 << 1) | (1 << 9),
        checksum: 0x42,
    };

    let (id, payload) = encode_play_chat_command_signed(&packet);

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "say hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0x02, 0x02, 0x00]
    );
    assert_eq!(decoder.read_u8().unwrap(), 0x42);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_signed_chat_command_argument_signatures() {
    let packet = ChatCommandSigned {
        command: "tell Steve hello".to_string(),
        timestamp_millis: 1_717_986_918_300,
        salt: 0x0102_0304_0506_0708,
        argument_signatures: ArgumentSignatures {
            entries: vec![ArgumentSignature {
                name: "message".to_string(),
                signature: MessageSignature {
                    bytes: vec![0xab; 256],
                },
            }],
        },
        last_seen_messages: LastSeenMessagesUpdate::default(),
    };

    let (id, payload) = encode_play_chat_command_signed(&packet);

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "tell Steve hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_string(16).unwrap(), "message");
    assert_eq!(
        decoder.read_exact(256, "argument signature").unwrap(),
        vec![0xab; 256].as_slice()
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0, 0, 0]
    );
    assert_eq!(decoder.read_u8().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_unsigned_chat_message_packet() {
    let (id, payload) = encode_play_chat_message(&ChatMessage::unsigned(
        "hello",
        1_717_986_918_300,
        0x0102_0304_0506_0708,
    ));

    assert_eq!(id, ids::play::SERVERBOUND_CHAT);
    assert_eq!(
        payload,
        vec![
            0x05, b'h', b'e', b'l', b'l', b'o', 0x00, 0x00, 0x01, 0x8f, 0xff, 0xff, 0xff, 0x9c,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ]
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(256).unwrap(), "hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0, 0, 0]
    );
    assert_eq!(decoder.read_u8().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
#[should_panic(expected = "chat message exceeds vanilla maximum of 256 characters")]
fn rejects_oversized_unsigned_chat_message_packet() {
    let _ = encode_play_chat_message(&ChatMessage::unsigned(
        "a".repeat(257),
        1_717_986_918_300,
        0x0102_0304_0506_0708,
    ));
}

#[test]
fn encodes_signed_chat_message_packet() {
    let mut packet = ChatMessage::unsigned("hello", 1_717_986_918_300, 0x0102_0304_0506_0708);
    packet.signature = Some(MessageSignature {
        bytes: vec![0xab; 256],
    });

    let (id, payload) = encode_play_chat_message(&packet);

    assert_eq!(id, ids::play::SERVERBOUND_CHAT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(256).unwrap(), "hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(
        decoder.read_exact(256, "chat message signature").unwrap(),
        vec![0xab; 256]
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0, 0, 0]
    );
    assert_eq!(decoder.read_u8().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
#[should_panic(expected = "chat message signature must be exactly 256 bytes")]
fn rejects_invalid_chat_message_signature_length() {
    let mut packet = ChatMessage::unsigned("hello", 1_717_986_918_300, 7);
    packet.signature = Some(MessageSignature {
        bytes: vec![0xab; 255],
    });

    let _ = encode_play_chat_message(&packet);
}

#[test]
fn encodes_chat_message_last_seen_update_bitset_little_endian() {
    let (id, payload) = encode_play_chat_message(&ChatMessage {
        message: "hello".to_string(),
        timestamp_millis: 1_717_986_918_300,
        salt: 0x0102_0304_0506_0708,
        signature: None,
        last_seen_messages: LastSeenMessagesUpdate {
            offset: 3,
            acknowledged: (1 << 0) | (1 << 8) | (1 << 19),
            checksum: 0x7f,
        },
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHAT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(256).unwrap(), "hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0x01, 0x01, 0x08]
    );
    assert_eq!(decoder.read_u8().unwrap(), 0x7f);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_chat_session_update_packet() {
    let session_id = Uuid::from_u128(0x12345678_90ab_cdef_1020_304050607080);
    let (id, payload) = encode_play_chat_session_update(&ChatSessionUpdate {
        session_id,
        expires_at_epoch_millis: 1_717_986_918_300,
        public_key: vec![0x30, 0x82, 0x01, 0x22],
        key_signature: vec![0xab; 256],
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_SESSION_UPDATE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), session_id);
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_var_i32().unwrap(), 4);
    assert_eq!(
        decoder.read_exact(4, "profile public key").unwrap(),
        &[0x30, 0x82, 0x01, 0x22]
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 256);
    assert_eq!(
        decoder
            .read_exact(256, "profile public key signature")
            .unwrap(),
        vec![0xab; 256]
    );
    assert!(decoder.is_empty());
}

#[test]
#[should_panic(expected = "profile public key exceeds vanilla maximum of 512 bytes")]
fn rejects_oversized_chat_session_public_key() {
    let _ = encode_play_chat_session_update(&ChatSessionUpdate {
        session_id: Uuid::nil(),
        expires_at_epoch_millis: 0,
        public_key: vec![0; 513],
        key_signature: Vec::new(),
    });
}

#[test]
#[should_panic(expected = "profile public key signature exceeds vanilla maximum of 4096 bytes")]
fn rejects_oversized_chat_session_key_signature() {
    let _ = encode_play_chat_session_update(&ChatSessionUpdate {
        session_id: Uuid::nil(),
        expires_at_epoch_millis: 0,
        public_key: Vec::new(),
        key_signature: vec![0; 4097],
    });
}

#[test]
fn encodes_entity_interaction_packets() {
    let (id, payload) = encode_play_attack_entity(AttackEntity { entity_id: 123 });
    assert_eq!(id, ids::play::SERVERBOUND_ATTACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_interact_entity(InteractEntity {
        entity_id: 123,
        hand: InteractionHand::OffHand,
        location: Vec3d::default(),
        using_secondary_action: true,
    });
    assert_eq!(id, ids::play::SERVERBOUND_INTERACT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_interact_entity(InteractEntity {
        entity_id: 5,
        hand: InteractionHand::MainHand,
        location: Vec3d {
            x: 1.0,
            y: 0.0,
            z: -1.0,
        },
        using_secondary_action: false,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_exact(6, "lp_vec3").unwrap(),
        &[0xf1, 0xff, 0x00, 0x00, 0xff, 0xff]
    );
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_jigsaw_generate_packet() {
    let packet = JigsawGenerate {
        pos: BlockPos {
            x: -12,
            y: 64,
            z: 9,
        },
        levels: 20,
        keep_jigsaws: true,
    };
    let (id, payload) = encode_play_jigsaw_generate(packet);

    assert_eq!(id, ids::play::SERVERBOUND_JIGSAW_GENERATE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        decoder.read_i64().unwrap(),
        encode_block_pos(packet.pos.x, packet.pos.y, packet.pos.z)
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 20);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_swing_hand() {
    let (id, payload) = encode_play_swing(InteractionHand::MainHand);
    assert_eq!(id, ids::play::SERVERBOUND_SWING);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_swing(InteractionHand::OffHand);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_use_item_on_packet() {
    let (id, payload) = encode_play_use_item_on(UseItemOn {
        hand: InteractionHand::MainHand,
        hit: BlockHitResult {
            pos: BlockPos {
                x: 34,
                y: -12,
                z: -45,
            },
            direction: Direction::Up,
            cursor_x: 0.25,
            cursor_y: 1.0,
            cursor_z: 0.75,
            inside: true,
            world_border_hit: false,
        },
        sequence: 11,
    });

    assert_eq!(id, ids::play::SERVERBOUND_USE_ITEM_ON);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos {
            x: 34,
            y: -12,
            z: -45,
        }
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_f32().unwrap(), 0.25);
    assert_eq!(decoder.read_f32().unwrap(), 1.0);
    assert_eq!(decoder.read_f32().unwrap(), 0.75);
    assert!(decoder.read_bool().unwrap());
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 11);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_use_item_packet() {
    let (id, payload) = encode_play_use_item(UseItem {
        hand: InteractionHand::OffHand,
        sequence: 12,
        y_rot: 180.0,
        x_rot: -30.0,
    });

    assert_eq!(id, ids::play::SERVERBOUND_USE_ITEM);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 12);
    assert_eq!(decoder.read_f32().unwrap(), 180.0);
    assert_eq!(decoder.read_f32().unwrap(), -30.0);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_pick_item_from_block_packet() {
    let (id, payload) = encode_play_pick_item_from_block(PickItemFromBlock {
        pos: BlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        include_data: true,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos {
            x: -5,
            y: 70,
            z: 12,
        }
    );
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_pick_item_from_entity_packet() {
    let (id, payload) = encode_play_pick_item_from_entity(PickItemFromEntity {
        entity_id: 123,
        include_data: true,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PICK_ITEM_FROM_ENTITY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_sign_update_packet() {
    let (id, payload) = encode_play_sign_update(&SignUpdate {
        pos: BlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        is_front_text: false,
        lines: [
            "line 0".to_string(),
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ],
    });

    assert_eq!(id, ids::play::SERVERBOUND_SIGN_UPDATE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos {
            x: -5,
            y: 70,
            z: 12,
        }
    );
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_string(384).unwrap(), "line 0");
    assert_eq!(decoder.read_string(384).unwrap(), "line 1");
    assert_eq!(decoder.read_string(384).unwrap(), "line 2");
    assert_eq!(decoder.read_string(384).unwrap(), "line 3");
    assert!(decoder.is_empty());
}

#[test]
fn encodes_spectator_entity_packets() {
    let (id, payload) = encode_play_spectate_entity(SpectateEntity { entity_id: 1234 });
    assert_eq!(id, ids::play::SERVERBOUND_SPECTATE_ENTITY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1234);
    assert!(decoder.is_empty());

    let uuid = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
    let (id, payload) = encode_play_teleport_to_entity(TeleportToEntity { uuid });
    assert_eq!(id, ids::play::SERVERBOUND_TELEPORT_TO_ENTITY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), uuid);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_award_stats() {
    assert_eq!(ids::play::CLIENTBOUND_AWARD_STATS, 3);

    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_var_i32(0);
    payload.write_var_i32(34);
    payload.write_var_i32(12);
    payload.write_var_i32(8);
    payload.write_var_i32(5);
    payload.write_var_i32(1);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_AWARD_STATS, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::AwardStats(AwardStats {
            stats: vec![
                StatUpdate {
                    stat_type_id: 0,
                    value_id: 34,
                    amount: 12,
                },
                StatUpdate {
                    stat_type_id: 8,
                    value_id: 5,
                    amount: 1,
                },
            ],
        })
    );

    let mut trailing = Encoder::new();
    trailing.write_var_i32(0);
    trailing.write_u8(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_AWARD_STATS, &trailing.into_inner())
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after award stats packet")
    );
}

#[test]
fn decodes_and_encodes_move_vehicle_packets() {
    let mut payload = Encoder::new();
    payload.write_f64(12.5);
    payload.write_f64(65.25);
    payload.write_f64(-8.75);
    payload.write_f32(135.0);
    payload.write_f32(-12.5);
    let payload = payload.into_inner();
    assert_eq!(payload.len(), 32);

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_MOVE_VEHICLE, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::MoveVehicle(MoveVehicle {
            position: Vec3d {
                x: 12.5,
                y: 65.25,
                z: -8.75,
            },
            y_rot: 135.0,
            x_rot: -12.5,
        })
    );

    let (id, payload) = encode_play_move_vehicle(12.5, 65.25, -8.75, 135.0, -12.5, true);
    assert_eq!(id, ids::play::SERVERBOUND_MOVE_VEHICLE);
    assert_eq!(id, 34);
    assert_eq!(payload.len(), 33);
    assert_eq!(payload[32], 1);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), 12.5);
    assert_eq!(decoder.read_f64().unwrap(), 65.25);
    assert_eq!(decoder.read_f64().unwrap(), -8.75);
    assert_eq!(decoder.read_f32().unwrap(), 135.0);
    assert_eq!(decoder.read_f32().unwrap(), -12.5);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());

    let mut trailing = Encoder::new();
    trailing.write_f64(12.5);
    trailing.write_f64(65.25);
    trailing.write_f64(-8.75);
    trailing.write_f32(135.0);
    trailing.write_f32(-12.5);
    trailing.write_u8(0);
    let error =
        decode_play_clientbound(ids::play::CLIENTBOUND_MOVE_VEHICLE, &trailing.into_inner())
            .unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after move vehicle packet"));
}

#[test]
fn encodes_paddle_boat() {
    let (id, payload) = encode_play_paddle_boat(PaddleBoat {
        left: true,
        right: false,
    });
    assert_eq!(id, ids::play::SERVERBOUND_PADDLE_BOAT);
    assert_eq!(id, 35);

    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_play_ping_request() {
    let (id, payload) = encode_play_ping_request(1_717_986_918_300);
    assert_eq!(id, ids::play::SERVERBOUND_PING_REQUEST);
    assert_eq!(id, 38);
    assert_eq!(payload.len(), 8);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_play_connection_lifecycle_packets() {
    let (id, payload) = encode_play_keep_alive(-123_456_789_012_345);
    assert_eq!(id, ids::play::SERVERBOUND_KEEP_ALIVE);
    assert_eq!(id, 28);
    assert_eq!(payload.len(), 8);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i64().unwrap(), -123_456_789_012_345);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_pong(-1_234_567);
    assert_eq!(id, ids::play::SERVERBOUND_PONG);
    assert_eq!(id, 45);
    assert_eq!(payload.len(), 4);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i32().unwrap(), -1_234_567);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_configuration_acknowledged();
    assert_eq!(id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
    assert_eq!(id, 16);
    assert!(payload.is_empty());
}

#[test]
fn decodes_play_keep_alive_and_ping_strictly() {
    let mut keep_alive = Encoder::new();
    keep_alive.write_i64(-123_456_789_012_345);
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_KEEP_ALIVE, &keep_alive.into_inner())
            .unwrap(),
        PlayClientbound::KeepAlive {
            id: -123_456_789_012_345,
        }
    );
    let mut keep_alive = Encoder::new();
    keep_alive.write_i64(-123_456_789_012_345);
    keep_alive.write_u8(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_KEEP_ALIVE, &keep_alive.into_inner())
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after play keep alive packet")
    );

    let mut ping = Encoder::new();
    ping.write_i32(-1_234_567);
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_PING, &ping.into_inner()).unwrap(),
        PlayClientbound::Ping { id: -1_234_567 }
    );
    let mut ping = Encoder::new();
    ping.write_i32(-1_234_567);
    ping.write_u8(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_PING, &ping.into_inner())
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after play ping packet")
    );
}

#[test]
fn encodes_player_loaded() {
    let (id, payload) = encode_play_player_loaded();
    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_LOADED);
    assert_eq!(id, 44);
    assert!(payload.is_empty());
}

#[test]
fn decodes_chunk_batch_and_encodes_client_play_status_packets() {
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_CHUNK_BATCH_START, &[]).unwrap();
    assert_eq!(packet, PlayClientbound::ChunkBatchStart);

    let mut payload = Encoder::new();
    payload.write_var_i32(9);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ChunkBatchFinished { batch_size: 9 }
    );

    let error =
        decode_play_clientbound(ids::play::CLIENTBOUND_CHUNK_BATCH_START, &[0]).unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after chunk batch start packet"));

    let mut trailing_finished = Encoder::new();
    trailing_finished.write_var_i32(9);
    trailing_finished.write_u8(0);
    let error = decode_play_clientbound(
        ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED,
        &trailing_finished.into_inner(),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after chunk batch finished packet"));

    let (id, payload) = encode_play_chunk_batch_received(9.0);
    assert_eq!(id, ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED);
    assert_eq!(payload.len(), 4);
    assert_eq!(Decoder::new(&payload).read_f32().unwrap(), 9.0);

    let (id, payload) = encode_play_client_tick_end();
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_TICK_END);
    assert!(payload.is_empty());

    let (id, payload) = encode_play_client_information_default().unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_INFORMATION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(16).unwrap(), "en_us");
    assert_eq!(decoder.read_i8().unwrap(), 10);
}

#[test]
fn encodes_client_information_from_struct() {
    let information = ClientInformation {
        language: "zh_cn".to_string(),
        view_distance: 12,
        chat_visibility: ClientChatVisibility::System,
        chat_colors: false,
        displayed_skin_parts: 0x15,
        main_hand: ClientMainHand::Left,
        text_filtering_enabled: true,
        allows_listing: true,
        particle_status: ClientParticleStatus::Minimal,
    };

    let (id, payload) = encode_client_information(&information).unwrap();
    assert_eq!(id, ids::configuration::SERVERBOUND_CLIENT_INFORMATION);
    assert_client_information_payload(&payload, &information);

    let (id, play_payload) = encode_play_client_information(&information).unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_INFORMATION);
    assert_eq!(play_payload, payload);

    let mut maximum_language = information.clone();
    maximum_language.language = "a".repeat(16);
    encode_client_information(&maximum_language).unwrap();
    encode_play_client_information(&maximum_language).unwrap();

    let mut oversized_language = information;
    oversized_language.language = "a".repeat(17);
    assert!(encode_client_information(&oversized_language).is_err());
    assert!(encode_play_client_information(&oversized_language).is_err());
    oversized_language.language = "😀".repeat(9);
    assert!(encode_client_information(&oversized_language).is_err());
    assert!(encode_play_client_information(&oversized_language).is_err());
}

#[test]
fn encodes_configuration_brand_custom_payload() {
    let (id, payload) = encode_configuration_brand_custom_payload("bbb-native").unwrap();
    assert_eq!(id, ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
    assert_eq!(decoder.read_string(32767).unwrap(), "bbb-native");
    assert!(decoder.is_empty());

    encode_configuration_brand_custom_payload(&"a".repeat(32_767)).unwrap();
    assert!(encode_configuration_brand_custom_payload(&"a".repeat(32_768)).is_err());
    assert!(encode_configuration_brand_custom_payload(&"😀".repeat(16_384)).is_err());
}

#[test]
fn encodes_play_brand_custom_payload() {
    let (id, payload) = encode_play_custom_payload(&ServerboundCustomPayload::Brand {
        brand: "bbb-native".to_string(),
    })
    .unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_CUSTOM_PAYLOAD);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
    assert_eq!(decoder.read_string(32767).unwrap(), "bbb-native");
    assert!(decoder.is_empty());
}

#[test]
fn encodes_custom_click_action_in_configuration_and_play() {
    let action = CustomClickAction {
        id: "dialog/submit".to_string(),
        payload: Some(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "byte".to_string(),
                value: NbtSummaryValue::Byte(-7),
            },
            NbtSummaryEntry {
                name: "short".to_string(),
                value: NbtSummaryValue::Short(-300),
            },
            NbtSummaryEntry {
                name: "int".to_string(),
                value: NbtSummaryValue::Int(70_000),
            },
            NbtSummaryEntry {
                name: "long".to_string(),
                value: NbtSummaryValue::Long(-9_000_000_000),
            },
            NbtSummaryEntry {
                name: "float".to_string(),
                value: NbtSummaryValue::Float(1.25f32.to_bits()),
            },
            NbtSummaryEntry {
                name: "double".to_string(),
                value: NbtSummaryValue::Double((-2.5f64).to_bits()),
            },
            NbtSummaryEntry {
                name: "bytes".to_string(),
                value: NbtSummaryValue::ByteArray(vec![-1, 0, 1]),
            },
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("A\0😀".to_string()),
            },
            NbtSummaryEntry {
                name: "values".to_string(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::Int(3),
                    NbtSummaryValue::Int(5),
                ]),
            },
            NbtSummaryEntry {
                name: "nested".to_string(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "ok".to_string(),
                    value: NbtSummaryValue::Byte(1),
                }]),
            },
            NbtSummaryEntry {
                name: "ints".to_string(),
                value: NbtSummaryValue::IntArray(vec![-1, 0, 1]),
            },
            NbtSummaryEntry {
                name: "longs".to_string(),
                value: NbtSummaryValue::LongArray(vec![-1, 0, 1]),
            },
        ])),
    };
    let (configuration_id, configuration_payload) =
        encode_configuration_custom_click_action(&action).unwrap();
    let (play_id, play_payload) = encode_play_custom_click_action(&action).unwrap();

    assert_eq!(
        configuration_id,
        ids::configuration::SERVERBOUND_CUSTOM_CLICK_ACTION
    );
    assert_eq!(play_id, ids::play::SERVERBOUND_CUSTOM_CLICK_ACTION);
    assert_eq!(configuration_payload, play_payload);

    let mut decoder = Decoder::new(&play_payload);
    assert_eq!(
        decoder.read_string(32_767).unwrap(),
        "minecraft:dialog/submit"
    );
    let tag_len = decoder.read_len().unwrap();
    let mut tag_decoder = Decoder::new(decoder.read_exact(tag_len, "custom click tag").unwrap());
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut tag_decoder).unwrap(),
        action.payload
    );
    assert!(tag_decoder.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_absent_custom_click_action_tag_as_length_prefixed_end_tag() {
    let action = CustomClickAction {
        id: "bbb:none".to_string(),
        payload: None,
    };
    let (_, payload) = encode_play_custom_click_action(&action).unwrap();
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32_767).unwrap(), "bbb:none");
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn rejects_invalid_oversized_and_excessively_deep_custom_click_actions() {
    let invalid_id = CustomClickAction {
        id: "BBB:submit".to_string(),
        payload: None,
    };
    assert!(encode_play_custom_click_action(&invalid_id).is_err());

    let oversized = CustomClickAction {
        id: "bbb:submit".to_string(),
        payload: Some(NbtSummaryValue::ByteArray(vec![
            0;
            super::client_common::MAX_CUSTOM_CLICK_ACTION_TAG_BYTES
        ])),
    };
    assert!(matches!(
        encode_play_custom_click_action(&oversized),
        Err(ProtocolError::PacketTooLarge(_, max))
            if max == super::client_common::MAX_CUSTOM_CLICK_ACTION_TAG_BYTES
    ));

    let mut too_deep = NbtSummaryValue::Compound(Vec::new());
    for _ in 0..super::client_common::MAX_CUSTOM_CLICK_ACTION_TAG_DEPTH {
        too_deep = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "child".to_string(),
            value: too_deep,
        }]);
    }
    let too_deep = CustomClickAction {
        id: "bbb:submit".to_string(),
        payload: Some(too_deep),
    };
    assert!(encode_configuration_custom_click_action(&too_deep)
        .unwrap_err()
        .to_string()
        .contains("network nbt exceeded max depth"));
}

#[test]
fn encodes_heterogeneous_custom_click_action_list_with_vanilla_wrappers() {
    let action = CustomClickAction {
        id: "bbb:submit".to_string(),
        payload: Some(NbtSummaryValue::List(vec![
            NbtSummaryValue::Int(1),
            NbtSummaryValue::String("two".to_string()),
            NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("three".to_string()),
            }]),
        ])),
    };
    let (_, payload) = encode_play_custom_click_action(&action).unwrap();
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32_767).unwrap(), "bbb:submit");
    let tag_len = decoder.read_len().unwrap();
    let mut tag_decoder = Decoder::new(decoder.read_exact(tag_len, "custom click tag").unwrap());
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut tag_decoder).unwrap(),
        action.payload
    );
    assert!(tag_decoder.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn play_custom_payload_rejects_invalid_ids_and_oversized_brand() {
    assert!(
        encode_play_custom_payload(&ServerboundCustomPayload::Unknown {
            id: "BBB:debug".to_string(),
            raw_payload: Vec::new(),
        })
        .is_err()
    );
    assert!(
        encode_play_custom_payload(&ServerboundCustomPayload::Brand {
            brand: "😀".repeat(16_384),
        })
        .is_err()
    );
}

#[test]
fn encodes_play_debug_subscription_request_as_validated_deterministic_set() {
    use crate::debug_subscription_types::VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID;

    let (id, payload) = encode_play_debug_subscription_request(&[
        15,
        VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID,
        8,
        15,
    ])
    .unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 8);
    assert_eq!(decoder.read_var_i32().unwrap(), 15);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_debug_subscription_request(&[]).unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    assert!(encode_play_debug_subscription_request(&[-1])
        .unwrap_err()
        .to_string()
        .contains("invalid debug subscription id -1"));
    assert!(encode_play_debug_subscription_request(&[16])
        .unwrap_err()
        .to_string()
        .contains("invalid debug subscription id 16"));
}

#[test]
fn rejects_oversized_play_unknown_custom_payload() {
    let err = encode_play_custom_payload(&ServerboundCustomPayload::Unknown {
        id: "bbb:debug".to_string(),
        raw_payload: vec![0; 32768],
    })
    .unwrap_err();

    assert!(matches!(err, ProtocolError::PacketTooLarge(32768, 32767)));
}

fn assert_client_information_payload(payload: &[u8], expected: &ClientInformation) {
    let mut decoder = Decoder::new(payload);
    assert_eq!(decoder.read_string(16).unwrap(), expected.language.as_str());
    assert_eq!(decoder.read_i8().unwrap(), expected.view_distance);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        match expected.chat_visibility {
            ClientChatVisibility::Full => 0,
            ClientChatVisibility::System => 1,
            ClientChatVisibility::Hidden => 2,
        }
    );
    assert_eq!(decoder.read_bool().unwrap(), expected.chat_colors);
    assert_eq!(decoder.read_u8().unwrap(), expected.displayed_skin_parts);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        match expected.main_hand {
            ClientMainHand::Left => 0,
            ClientMainHand::Right => 1,
        }
    );
    assert_eq!(
        decoder.read_bool().unwrap(),
        expected.text_filtering_enabled
    );
    assert_eq!(decoder.read_bool().unwrap(), expected.allows_listing);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        match expected.particle_status {
            ClientParticleStatus::All => 0,
            ClientParticleStatus::Decreased => 1,
            ClientParticleStatus::Minimal => 2,
        }
    );
    assert!(decoder.is_empty());
}

#[test]
fn encodes_configuration_accept_code_of_conduct_as_unit_packet() {
    let (id, payload) = encode_configuration_accept_code_of_conduct();
    assert_eq!(id, ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT);
    assert!(payload.is_empty());
}

#[test]
fn decodes_transfer_packets_in_configuration_and_play() {
    let mut payload = Encoder::new();
    payload.write_string("next.example.com");
    payload.write_var_i32(25566);
    let payload = payload.into_inner();

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_TRANSFER, &payload)
            .unwrap(),
        ConfigurationClientbound::Transfer(Transfer {
            host: "next.example.com".to_string(),
            port: 25566,
        })
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_TRANSFER, &payload).unwrap(),
        PlayClientbound::Transfer(Transfer {
            host: "next.example.com".to_string(),
            port: 25566,
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    for error in [
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_TRANSFER, &trailing)
            .unwrap_err(),
        decode_play_clientbound(ids::play::CLIENTBOUND_TRANSFER, &trailing).unwrap_err(),
    ] {
        assert!(error
            .to_string()
            .contains("trailing bytes after transfer packet"));
    }
}

#[test]
fn decodes_and_encodes_cookie_packets() {
    let mut request_payload = Encoder::new();
    request_payload.write_string("session");
    let request_payload = request_payload.into_inner();

    assert_eq!(
        decode_login_clientbound(ids::login::CLIENTBOUND_COOKIE_REQUEST, &request_payload).unwrap(),
        LoginClientbound::CookieRequest(CookieRequest {
            key: "minecraft:session".to_string(),
        })
    );
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_COOKIE_REQUEST,
            &request_payload,
        )
        .unwrap(),
        ConfigurationClientbound::CookieRequest(CookieRequest {
            key: "minecraft:session".to_string(),
        })
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_COOKIE_REQUEST, &request_payload).unwrap(),
        PlayClientbound::CookieRequest(CookieRequest {
            key: "minecraft:session".to_string(),
        })
    );

    let mut trailing_request = request_payload.clone();
    trailing_request.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_COOKIE_REQUEST, &trailing_request)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after cookie request packet")
    );

    let mut store_payload = Encoder::new();
    store_payload.write_string("session");
    store_payload.write_var_i32(3);
    store_payload.write_bytes(&[1, 2, 3]);
    let store_payload = store_payload.into_inner();

    let store = StoreCookie {
        key: "minecraft:session".to_string(),
        payload: vec![1, 2, 3],
    };
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_STORE_COOKIE,
            &store_payload,
        )
        .unwrap(),
        ConfigurationClientbound::StoreCookie(store.clone())
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_STORE_COOKIE, &store_payload).unwrap(),
        PlayClientbound::StoreCookie(store)
    );

    let mut trailing_store = store_payload.clone();
    trailing_store.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_STORE_COOKIE, &trailing_store)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after store cookie packet")
    );

    let (id, payload) = encode_login_cookie_response("bbb:session", Some(&[1, 2, 3])).unwrap();
    assert_eq!(id, ids::login::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:session", Some(&[1, 2, 3]));

    let (id, payload) = encode_configuration_cookie_response("bbb:missing", None).unwrap();
    assert_eq!(id, ids::configuration::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:missing", None);

    let (id, payload) = encode_play_cookie_response("bbb:session", Some(&[4, 5])).unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:session", Some(&[4, 5]));

    let maximum_payload = vec![7; 5120];
    let (_, payload) = encode_login_cookie_response("session", Some(&maximum_payload)).unwrap();
    assert_cookie_response_payload(&payload, "minecraft:session", Some(&maximum_payload));
    assert!(encode_configuration_cookie_response("bbb:session", Some(&vec![7; 5121])).is_err());
    assert!(encode_play_cookie_response("INVALID:session", None).is_err());
}

#[test]
fn login_unknown_packet_preserves_unsupported_diagnostics() {
    assert_eq!(
        decode_login_clientbound(0x7f, &[1, 2, 3]).unwrap(),
        LoginClientbound::Unknown {
            packet_id: 0x7f,
            len: 3,
        }
    );
}

#[test]
fn rejects_invalid_cookie_keys() {
    let mut request_payload = Encoder::new();
    request_payload.write_string("bbb:Session");
    let request_payload = request_payload.into_inner();

    let err = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_COOKIE_REQUEST,
        &request_payload,
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut store_payload = Encoder::new();
    store_payload.write_string("bbb:Session");
    store_payload.write_var_i32(1);
    store_payload.write_u8(0);
    let store_payload = store_payload.into_inner();

    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_STORE_COOKIE, &store_payload).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn configuration_store_cookie_enforces_vanilla_payload_limit() {
    let mut payload = Encoder::new();
    payload.write_string("bbb:session");
    payload.write_var_i32(5120);
    payload.write_bytes(&vec![7; 5120]);
    assert!(matches!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_STORE_COOKIE,
            &payload.into_inner()
        )
        .unwrap(),
        ConfigurationClientbound::StoreCookie(StoreCookie { payload, .. }) if payload.len() == 5120
    ));

    let mut payload = Encoder::new();
    payload.write_string("bbb:session");
    payload.write_var_i32(5121);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_STORE_COOKIE,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(5121, 5120)));
}

#[test]
fn decodes_configuration_common_packets() {
    let mut custom_payload = Encoder::new();
    custom_payload.write_string("minecraft:brand");
    custom_payload.write_string("bbb-native");
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
            &custom_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::CustomPayload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "bbb-native".to_string(),
            },
        })
    );

    let mut unknown_custom_payload = Encoder::new();
    unknown_custom_payload.write_string("bbb:configuration_probe");
    unknown_custom_payload.write_bytes(&[0xaa, 0xbb, 0xcc]);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
            &unknown_custom_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::CustomPayload(CustomPayload {
            id: "bbb:configuration_probe".to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: vec![0xaa, 0xbb, 0xcc],
            },
        })
    );

    let mut oversized_custom_payload = Encoder::new();
    oversized_custom_payload.write_string("bbb:oversized");
    oversized_custom_payload.write_bytes(&vec![0; 1_048_577]);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
        &oversized_custom_payload.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(1_048_577, 1_048_576)
    ));

    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_DISCONNECT,
            &nbt_string_root("Configuration closed"),
        )
        .unwrap(),
        ConfigurationClientbound::Disconnect {
            reason: "Configuration closed".to_string(),
            raw_reason: nbt_string_root("Configuration closed"),
        }
    );

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_RESET_CHAT, &[]).unwrap(),
        ConfigurationClientbound::ResetChat
    );

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION, &[])
            .unwrap(),
        ConfigurationClientbound::Finish
    );
    let mut keep_alive = Encoder::new();
    keep_alive.write_i64(-123_456_789);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_KEEP_ALIVE,
            &keep_alive.into_inner()
        )
        .unwrap(),
        ConfigurationClientbound::KeepAlive { id: -123_456_789 }
    );
    let mut trailing_keep_alive = Encoder::new();
    trailing_keep_alive.write_i64(-123_456_789);
    trailing_keep_alive.write_u8(0);
    assert!(decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_KEEP_ALIVE,
        &trailing_keep_alive.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after configuration keep alive packet"));
    let mut ping = Encoder::new();
    ping.write_i32(0x1234_5678);
    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_PING, &ping.into_inner())
            .unwrap(),
        ConfigurationClientbound::Ping { id: 0x1234_5678 }
    );
    let mut trailing_ping = Encoder::new();
    trailing_ping.write_i32(0x1234_5678);
    trailing_ping.write_u8(0);
    assert!(decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_PING,
        &trailing_ping.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after configuration ping packet"));

    assert_eq!(
        encode_configuration_finish(),
        (
            ids::configuration::SERVERBOUND_FINISH_CONFIGURATION,
            Vec::new()
        )
    );
    let (packet_id, payload) = encode_configuration_keep_alive(-123_456_789);
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_KEEP_ALIVE);
    assert_eq!(Decoder::new(&payload).read_i64().unwrap(), -123_456_789);
    let (packet_id, payload) = encode_configuration_pong(0x1234_5678);
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_PONG);
    assert_eq!(Decoder::new(&payload).read_i32().unwrap(), 0x1234_5678);

    let pack_id = Uuid::from_u128(0x11111111_2222_3333_4444_555555555555);
    let mut push_payload = Encoder::new();
    push_payload.write_uuid(pack_id);
    push_payload.write_string("https://example.invalid/server-pack.zip");
    push_payload.write_string("0123456789abcdef0123456789abcdef01234567");
    push_payload.write_bool(true);
    push_payload.write_bool(false);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_RESOURCE_PACK_PUSH,
            &push_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::ResourcePackPush(ResourcePackPush {
            id: pack_id,
            url: "https://example.invalid/server-pack.zip".to_string(),
            hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
            required: true,
            prompt: None,
        })
    );

    let mut pop_payload = Encoder::new();
    pop_payload.write_bool(true);
    pop_payload.write_uuid(pack_id);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_RESOURCE_PACK_POP,
            &pop_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::ResourcePackPop(ResourcePackPop { id: Some(pack_id) })
    );
    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_RESOURCE_PACK_POP, &[0])
            .unwrap(),
        ConfigurationClientbound::ResourcePackPop(ResourcePackPop { id: None })
    );

    let mut oversized_hash = Encoder::new();
    oversized_hash.write_uuid(pack_id);
    oversized_hash.write_string("https://example.invalid/server-pack.zip");
    oversized_hash.write_string(&"a".repeat(41));
    oversized_hash.write_bool(false);
    oversized_hash.write_bool(false);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_RESOURCE_PACK_PUSH,
        &oversized_hash.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong { max: 40, .. }
    ));

    let mut features_payload = Encoder::new();
    features_payload.write_var_i32(3);
    features_payload.write_string("minecraft:update_1_21");
    features_payload.write_string("vanilla");
    features_payload.write_string("minecraft:vanilla");
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
            &features_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::UpdateEnabledFeatures(UpdateEnabledFeatures {
            features: vec![
                "minecraft:update_1_21".to_string(),
                "minecraft:vanilla".to_string(),
            ],
        })
    );

    let mut hostile_features = Encoder::new();
    hostile_features.write_var_i32(i32::MAX);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
        &hostile_features.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::UnexpectedEof { .. }));

    let mut invalid_features_payload = Encoder::new();
    invalid_features_payload.write_var_i32(1);
    invalid_features_payload.write_string("minecraft:Vanilla");
    let err = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
        &invalid_features_payload.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut known_packs_payload = Encoder::new();
    known_packs_payload.write_var_i32(1);
    known_packs_payload.write_string("minecraft");
    known_packs_payload.write_string("core");
    known_packs_payload.write_string("26.1");
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS,
            &known_packs_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::SelectKnownPacks {
            known_packs: vec![KnownPack {
                namespace: "minecraft".to_string(),
                id: "core".to_string(),
                version: "26.1".to_string(),
            }],
        }
    );

    let mut hostile_known_packs = Encoder::new();
    hostile_known_packs.write_var_i32(i32::MAX);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS,
        &hostile_known_packs.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::UnexpectedEof { .. }));

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_CLEAR_DIALOG, &[])
            .unwrap(),
        ConfigurationClientbound::ClearDialog
    );
    let error =
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_CLEAR_DIALOG, &[0])
            .unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after clear dialog packet"));

    let mut show_dialog_payload = Encoder::new();
    show_dialog_payload.write_bytes(&nbt_compound_with_string("type", "minecraft:notice"));
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_SHOW_DIALOG,
            &show_dialog_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::ShowDialog(ShowDialog {
            dialog: DialogHolder::ContextFree {
                dialog: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "type".to_string(),
                    value: NbtSummaryValue::String("minecraft:notice".to_string()),
                }]),
            },
        })
    );

    let mut code_payload = Encoder::new();
    code_payload.write_string("Keep the server friendly.");
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CODE_OF_CONDUCT,
            &code_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::CodeOfConduct {
            text: "Keep the server friendly.".to_string(),
        }
    );

    let mut oversized_code = Encoder::new();
    oversized_code.write_string(&"x".repeat(32_768));
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_CODE_OF_CONDUCT,
        &oversized_code.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong { max: 32_767, .. }
    ));
}

#[test]
fn encodes_selected_known_packs_with_vanilla_limit_and_field_order() {
    let packs = vec![
        KnownPack {
            namespace: "minecraft".to_string(),
            id: "core".to_string(),
            version: "26.1".to_string(),
        },
        KnownPack {
            namespace: "example".to_string(),
            id: "feature".to_string(),
            version: "1".to_string(),
        },
    ];
    let (id, payload) = encode_select_known_packs(&packs).unwrap();
    assert_eq!(id, ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    for pack in &packs {
        assert_eq!(decoder.read_string(32_767).unwrap(), pack.namespace);
        assert_eq!(decoder.read_string(32_767).unwrap(), pack.id);
        assert_eq!(decoder.read_string(32_767).unwrap(), pack.version);
    }
    assert!(decoder.is_empty());

    let sixty_four = vec![packs[0].clone(); 64];
    encode_select_known_packs(&sixty_four).unwrap();
    let sixty_five = vec![packs[0].clone(); 65];
    assert!(encode_select_known_packs(&sixty_five)
        .unwrap_err()
        .to_string()
        .contains("maximum 64"));

    let oversized = [KnownPack {
        namespace: "😀".repeat(16_384),
        id: "core".to_string(),
        version: "26.1".to_string(),
    }];
    assert!(encode_select_known_packs(&oversized)
        .unwrap_err()
        .to_string()
        .contains("maximum is 32767"));
}

#[test]
fn decodes_configuration_registry_data_entries() {
    let raw_plains = nbt_compound_with_string("name", "minecraft:plains");
    let raw_temperature = nbt_list_of_floats(&[0.8, 0.4]);
    let mut payload = Encoder::new();
    payload.write_string("worldgen/biome");
    payload.write_var_i32(3);
    payload.write_string("plains");
    payload.write_bool(true);
    payload.write_bytes(&raw_plains);
    payload.write_string("minecraft:the_void");
    payload.write_bool(false);
    payload.write_string("bbb:raw_numeric_root");
    payload.write_bool(true);
    payload.write_bytes(&raw_temperature);
    let payload = payload.into_inner();

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_REGISTRY_DATA, &payload)
            .unwrap(),
        ConfigurationClientbound::RegistryData(RegistryData {
            registry: "minecraft:worldgen/biome".to_string(),
            entries: vec![
                RegistryDataEntry {
                    id: "minecraft:plains".to_string(),
                    raw_data: Some(raw_plains),
                },
                RegistryDataEntry {
                    id: "minecraft:the_void".to_string(),
                    raw_data: None,
                },
                RegistryDataEntry {
                    id: "bbb:raw_numeric_root".to_string(),
                    raw_data: Some(raw_temperature),
                },
            ],
            raw_payload_len: payload.len(),
        })
    );
}

#[test]
fn rejects_configuration_registry_data_entry_count_before_allocation() {
    let mut payload = Encoder::new();
    payload.write_string("minecraft:worldgen/biome");
    payload.write_var_i32(131_073);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(131_073, 131_072)
    ));
}

#[test]
fn rejects_invalid_configuration_registry_data_identifiers() {
    let mut invalid_registry = Encoder::new();
    invalid_registry.write_string("minecraft:Worldgen/Biome");
    invalid_registry.write_var_i32(0);
    let err = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &invalid_registry.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut invalid_entry = Encoder::new();
    invalid_entry.write_string("minecraft:worldgen/biome");
    invalid_entry.write_var_i32(1);
    invalid_entry.write_string("minecraft:Plains");
    invalid_entry.write_bool(false);
    let err = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &invalid_entry.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn rejects_invalid_configuration_registry_data_nbt() {
    let mut root_end = Encoder::new();
    root_end.write_string("minecraft:worldgen/biome");
    root_end.write_var_i32(1);
    root_end.write_string("minecraft:plains");
    root_end.write_bool(true);
    root_end.write_u8(0);
    assert!(decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &root_end.into_inner(),
    )
    .is_err());

    let mut trailing = Encoder::new();
    trailing.write_string("minecraft:worldgen/biome");
    trailing.write_var_i32(0);
    trailing.write_u8(99);
    assert!(decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &trailing.into_inner(),
    )
    .is_err());
}

#[test]
fn decodes_custom_report_details_in_configuration_and_play() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_string("Server");
    payload.write_string("bbb test shard");
    payload.write_string("Region");
    payload.write_string("local");
    let payload = payload.into_inner();

    let details = BTreeMap::from([
        ("Region".to_string(), "local".to_string()),
        ("Server".to_string(), "bbb test shard".to_string()),
    ]);
    let expected = CustomReportDetails { details };

    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
            &payload,
        )
        .unwrap(),
        ConfigurationClientbound::CustomReportDetails(expected.clone())
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_REPORT_DETAILS, &payload).unwrap(),
        PlayClientbound::CustomReportDetails(expected)
    );

    let mut trailing = payload;
    trailing.push(0);
    for error in [
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
            &trailing,
        )
        .unwrap_err(),
        decode_play_clientbound(ids::play::CLIENTBOUND_CUSTOM_REPORT_DETAILS, &trailing)
            .unwrap_err(),
    ] {
        assert!(error
            .to_string()
            .contains("trailing bytes after custom report details packet"));
    }

    let mut oversized = Encoder::new();
    oversized.write_var_i32(33);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
        &oversized.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(33, 32)));
}

#[test]
fn decodes_server_links_in_configuration_and_play() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_bool(true);
    payload.write_var_i32(0);
    payload.write_string("https://example.invalid/bugs");
    payload.write_bool(false);
    payload.write_bytes(&nbt_string_root("Rules"));
    payload.write_string("https://example.invalid/rules");
    let payload = payload.into_inner();

    let expected = ServerLinks {
        links: vec![
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
                url: "https://example.invalid/bugs".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Custom {
                    label: "Rules".to_string(),
                },
                url: "https://example.invalid/rules".to_string(),
            },
        ],
    };

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_SERVER_LINKS, &payload)
            .unwrap(),
        ConfigurationClientbound::ServerLinks(expected.clone())
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SERVER_LINKS, &payload).unwrap(),
        PlayClientbound::ServerLinks(expected)
    );

    let mut trailing = payload;
    trailing.push(0);
    for error in [
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_SERVER_LINKS, &trailing)
            .unwrap_err(),
        decode_play_clientbound(ids::play::CLIENTBOUND_SERVER_LINKS, &trailing).unwrap_err(),
    ] {
        assert!(error
            .to_string()
            .contains("trailing bytes after server links packet"));
    }

    let mut hostile = Encoder::new();
    hostile.write_var_i32(i32::MAX);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_SERVER_LINKS,
        &hostile.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::UnexpectedEof { .. }));
}

fn assert_cookie_response_payload(payload: &[u8], key: &str, expected: Option<&[u8]>) {
    let mut decoder = Decoder::new(payload);
    assert_eq!(decoder.read_string(32767).unwrap(), key);
    match expected {
        Some(expected) => {
            assert!(decoder.read_bool().unwrap());
            let len = decoder.read_len().unwrap();
            assert_eq!(
                decoder.read_exact(len, "cookie response").unwrap(),
                expected
            );
        }
        None => assert!(!decoder.read_bool().unwrap()),
    }
    assert!(decoder.is_empty());
}

fn encode_block_pos(x: i32, y: i32, z: i32) -> i64 {
    (((x as i64) & 0x3ffffff) << 38) | (((z as i64) & 0x3ffffff) << 12) | ((y as i64) & 0xfff)
}

fn nbt_string_root(text: &str) -> Vec<u8> {
    let mut payload = vec![8];
    payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
    payload.extend_from_slice(text.as_bytes());
    payload
}

fn nbt_compound_with_string(name: &str, value: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    write_nbt_string(&mut payload, name);
    write_nbt_string(&mut payload, value);
    payload.push(0);
    payload
}

fn nbt_list_of_floats(values: &[f32]) -> Vec<u8> {
    let mut payload = vec![9, 5];
    payload.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        payload.extend_from_slice(&value.to_be_bytes());
    }
    payload
}

fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
