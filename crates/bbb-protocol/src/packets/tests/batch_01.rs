use super::*;

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
            styled_reason: vec![StyledTextRun {
                text: "bye".to_string(),
                style: ComponentStyle::default(),
            }],
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
            styled_reason: vec![
                StyledTextRun {
                    text: "bye".to_string(),
                    style: ComponentStyle::default(),
                },
                StyledTextRun {
                    text: " now".to_string(),
                    style: ComponentStyle {
                        bold: Some(true),
                        ..ComponentStyle::default()
                    },
                },
            ],
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
            styled_reason: vec![StyledTextRun {
                text: "Kicked".to_string(),
                style: ComponentStyle::default(),
            }],
            raw_reason: payload.clone(),
        }
    );
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_DISCONNECT, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Disconnect(Disconnect {
            reason: "Kicked".to_string(),
            styled_reason: vec![StyledTextRun {
                text: "Kicked".to_string(),
                style: ComponentStyle::default(),
            }],
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
            component_patch: Default::default(),
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
            }
            .into(),
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
        name: "answer".into(),
        value: NbtSummaryValue::Int(42),
    }]);
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
                custom_data: Some(custom_data.clone()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
                ],
                custom_model_data_floats: vec![-0.0, nan].into(),
                custom_model_data_flags: vec![true, false],
                custom_model_data_strings: vec!["variant".into(), "😀".into(), "?".into()],
                custom_model_data_colors: vec![-1, 0x11_22_33],
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
    assert_eq!(value.read_len().unwrap(), 3);
    assert_eq!(value.read_string(32_767).unwrap(), "variant");
    assert_eq!(value.read_string(32_767).unwrap(), "😀");
    assert_eq!(value.read_string(32_767).unwrap(), "?");
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
            }
            .into(),
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
            }
            .into(),
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
            }
            .into(),
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
fn encodes_set_creative_mode_slot_repair_cost_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
                ],
                repair_cost: Some(5),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
            19, 1, 5, // repair_cost type, value length, and nonnegative value
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_creative_slot_lock_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
                ],
                creative_slot_lock: true,
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
            20, 0, // creative_slot_lock type and empty delimited Unit payload
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_additional_trade_cost_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
                ],
                additional_trade_cost: Some(-7),
                ..DataComponentPatchSummary::default()
            }.into(),
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
            41, 5, // additional_trade_cost type and value length
            0xf9, 0xff, 0xff, 0xff, 0x0f, // signed VarInt -7
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_enchantable_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
                ],
                enchantable_value: Some(15),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
            31, 1, 15, // enchantable type, value length, and positive value
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_equippable_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
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
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID],
                use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
                use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID],
                item_model: Some("example:models/variant".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_RARITY_TYPE_ID],
                rarity: Some(ItemRaritySummary::Rare),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID],
                enchantment_glint_override: Some(false),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID],
                intangible_projectile: true,
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID],
                instrument: InstrumentComponentSummary {
                    registry_id: None,
                    direct: Some(Box::new(InstrumentSummary {
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
                            "Custom horn".into(),
                        ))),
                    })),
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
        Some(NbtSummaryValue::String("Custom horn".into()))
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
                        translation_key: Some("block.minecraft.banner.stripe_bottom".into()),
                        color_id: VANILLA_DYE_COLOR_BLUE_ID,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BEES_TYPE_ID],
                bees_present: true,
                bees: vec![BeeOccupantSummary {
                    entity_type_id: VANILLA_ENTITY_TYPE_BEE_ID,
                    entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
                    ticks_in_hive: -5,
                    min_ticks_in_hive: 2400,
                }],
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID],
                lodestone_target: Some(target.clone()),
                lodestone_tracked: Some(false),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
    let title_nbt = NbtSummaryValue::String("Gallery".into());
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
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
            }
            .into(),
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
                    description_nbt: Some(Box::new(NbtSummaryValue::String("Custom song".into()))),
                    length_in_seconds_bits: 60.0_f32.to_bits(),
                    comparator_output: 7,
                })),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
        Some(NbtSummaryValue::String("Custom song".into()))
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
            }
            .into(),
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
                    display_text_nbt: Some(NbtSummaryValue::String("Override".into())),
                }],
                ..DataComponentPatchSummary::default()
            }.into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID],
                food_nutrition: Some(6),
                food_saturation_bits: Some(7.2_f32.to_bits()),
                food_can_always_eat: true,
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID],
                ..DataComponentPatchSummary::default()
            }
            .into(),
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
