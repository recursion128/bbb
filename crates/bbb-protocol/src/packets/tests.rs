use super::*;
use crate::{
    codec::{offline_player_uuid, Encoder},
    ids,
};

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
    payload.write_string("minecraft:overworld");
    payload.write_string("minecraft:the_nether");
    payload.write_string("minecraft:the_end");
    payload.write_var_i32(20);
    payload.write_var_i32(8);
    payload.write_var_i32(6);
    payload.write_bool(false);
    payload.write_bool(true);
    payload.write_bool(false);
    payload.write_var_i32(1);
    payload.write_string("minecraft:the_nether");
    payload.write_i64(12345);
    payload.write_i8(1);
    payload.write_i8(-1);
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_bool(true);
    payload.write_string("minecraft:overworld");
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
fn decodes_respawn_spawn_info() {
    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_string("minecraft:the_end");
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
fn encodes_set_carried_item() {
    let (id, payload) = encode_play_set_carried_item(6);
    assert_eq!(id, ids::play::SERVERBOUND_SET_CARRIED_ITEM);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 6);
    assert!(decoder.is_empty());
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
fn encodes_player_command_actions() {
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
    request_payload.write_string("bbb:session");
    let request_payload = request_payload.into_inner();

    assert_eq!(
        decode_login_clientbound(ids::login::CLIENTBOUND_COOKIE_REQUEST, &request_payload).unwrap(),
        LoginClientbound::CookieRequest(CookieRequest {
            key: "bbb:session".to_string(),
        })
    );
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_COOKIE_REQUEST,
            &request_payload,
        )
        .unwrap(),
        ConfigurationClientbound::CookieRequest(CookieRequest {
            key: "bbb:session".to_string(),
        })
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_COOKIE_REQUEST, &request_payload).unwrap(),
        PlayClientbound::CookieRequest(CookieRequest {
            key: "bbb:session".to_string(),
        })
    );

    let mut store_payload = Encoder::new();
    store_payload.write_string("bbb:session");
    store_payload.write_var_i32(3);
    store_payload.write_bytes(&[1, 2, 3]);
    let store_payload = store_payload.into_inner();

    let store = StoreCookie {
        key: "bbb:session".to_string(),
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
