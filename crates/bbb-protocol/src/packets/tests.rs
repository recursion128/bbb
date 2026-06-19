use super::*;
use crate::{
    codec::{offline_player_uuid, Decoder, Encoder, ProtocolError},
    ids,
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

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
}

#[test]
fn encodes_login_hello() {
    let uuid = offline_player_uuid("bbb-client");
    let (id, payload) = encode_login_hello("bbb-client", uuid);
    assert_eq!(id, ids::login::SERVERBOUND_HELLO);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(16).unwrap(), "bbb-client");
    assert_eq!(decoder.read_uuid().unwrap(), uuid);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_play_disconnect_component() {
    let mut payload = Vec::new();
    payload.push(8);
    payload.extend_from_slice(&6u16.to_be_bytes());
    payload.extend_from_slice(b"Kicked");

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_DISCONNECT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Disconnect(Disconnect {
            reason: "Kicked".to_string(),
            raw_reason: payload,
        })
    );
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

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_LOGIN, &payload.into_inner()).unwrap();
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

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_RESPAWN, &payload.into_inner()).unwrap();
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
fn set_creative_mode_slot_rejects_summarized_component_patch() {
    let err = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![6],
                custom_name: Some("Renamed".to_string()),
                ..DataComponentPatchSummary::default()
            },
        },
    })
    .unwrap_err();

    assert!(matches!(err, ProtocolError::InvalidData(_)));
}

#[test]
fn play_serverbound_inventory_packet_ids_match_vanilla_26_1_registration_order() {
    assert_eq!(ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED, 3);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK, 17);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_CLICK, 18);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_CLOSE, 19);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED, 20);
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
    });
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
    });

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
    });

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
    });

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
    });

    assert_eq!(id, ids::play::SERVERBOUND_SET_BEACON);
    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
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
fn encodes_chat_message_last_seen_update_bitset_little_endian() {
    let (id, payload) = encode_play_chat_message(&ChatMessage {
        message: "hello".to_string(),
        timestamp_millis: 1_717_986_918_300,
        salt: 0x0102_0304_0506_0708,
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

    let (id, payload) = encode_play_chunk_batch_received(9.0);
    assert_eq!(id, ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED);
    assert_eq!(payload.len(), 4);
    assert_eq!(Decoder::new(&payload).read_f32().unwrap(), 9.0);

    let (id, payload) = encode_play_client_tick_end();
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_TICK_END);
    assert!(payload.is_empty());

    let (id, payload) = encode_play_client_information_default();
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

    let (id, payload) = encode_client_information(&information);
    assert_eq!(id, ids::configuration::SERVERBOUND_CLIENT_INFORMATION);
    assert_client_information_payload(&payload, &information);

    let (id, play_payload) = encode_play_client_information(&information);
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_INFORMATION);
    assert_eq!(play_payload, payload);
}

#[test]
fn encodes_configuration_brand_custom_payload() {
    let (id, payload) = encode_configuration_brand_custom_payload("bbb-native");
    assert_eq!(id, ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
    assert_eq!(decoder.read_string(32767).unwrap(), "bbb-native");
    assert!(decoder.is_empty());
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

    let (id, payload) = encode_login_cookie_response("bbb:session", Some(&[1, 2, 3]));
    assert_eq!(id, ids::login::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:session", Some(&[1, 2, 3]));

    let (id, payload) = encode_configuration_cookie_response("bbb:missing", None);
    assert_eq!(id, ids::configuration::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:missing", None);

    let (id, payload) = encode_play_cookie_response("bbb:session", Some(&[4, 5]));
    assert_eq!(id, ids::play::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:session", Some(&[4, 5]));
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

    let mut features_payload = Encoder::new();
    features_payload.write_var_i32(2);
    features_payload.write_string("minecraft:update_1_21");
    features_payload.write_string("vanilla");
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

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_CLEAR_DIALOG, &[])
            .unwrap(),
        ConfigurationClientbound::ClearDialog
    );

    let mut show_dialog_payload = Encoder::new();
    show_dialog_payload.write_bytes(&[1, 2, 3, 4]);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_SHOW_DIALOG,
            &show_dialog_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::ShowDialog(ShowDialog {
            dialog: DialogHolder::Direct {
                raw_dialog_payload: vec![1, 2, 3, 4],
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
