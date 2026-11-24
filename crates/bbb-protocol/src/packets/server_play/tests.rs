use crate::game_types::GameType;
use crate::packets::{
    decode_play_clientbound, encode_play_accept_teleportation, encode_play_change_difficulty,
    encode_play_chat_message, encode_play_chunk_batch_received, encode_play_client_tick_end,
    encode_play_clientbound_level_chunk_with_light, encode_play_clientbound_set_chunk_cache_center,
    encode_play_keep_alive, encode_play_lock_difficulty, encode_play_move_player_pos,
    encode_play_move_player_pos_rot, encode_play_move_player_rot,
    encode_play_move_player_status_only, encode_play_move_vehicle, encode_play_paddle_boat,
    encode_play_place_recipe, encode_play_player_abilities, encode_play_player_action,
    encode_play_player_loaded, encode_play_recipe_book_change_settings,
    encode_play_recipe_book_seen_recipe, encode_play_rename_item, encode_play_seen_advancements,
    encode_play_select_bundle_item, encode_play_select_trade, encode_play_set_beacon,
    encode_play_set_carried_item, encode_play_set_creative_mode_slot, encode_play_sign_update,
    encode_play_swing, encode_play_use_item_on, BlockHitResult, BlockPos, ChangeDifficultyCommand,
    ChatMessage, ClockUpdate, CommonPlayerSpawnInfo, ContainerSetContent, ContainerSetSlot,
    Cooldown, DamageEvent, DataComponentPatchSummary, Direction, EntityAnimation, EntityDataValue,
    GlobalPos, InteractionHand, ItemStackSummary, LevelChunkData, LevelChunkWithLight,
    LightUpdateData, LockDifficultyCommand, MobEffectFlags, PlaceRecipeCommand, PlayClientbound,
    PlayTime, PlayerAbilitiesCommand, PlayerAction, PlayerActionKind, PlayerCombatKill,
    PlayerHealth, PlayerRotationUpdate, RecipeBookType, RecipeDisplayId, SelectBundleItem,
    SetCamera, SetChunkCacheCenter, SetCreativeModeSlot, SetHeldSlot, SignUpdate, SystemChat,
    Transfer, UseItemOn, Vec3d,
};

use super::*;

#[test]
fn encodes_the_clientbound_bundle_delimiter_strictly() {
    let (id, payload) = encode_play_clientbound_bundle_delimiter();

    assert_eq!(id, ids::play::CLIENTBOUND_BUNDLE_DELIMITER);
    assert!(payload.is_empty());
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::BundleDelimiter
    );

    let mut trailing = payload;
    trailing.push(0);
    assert!(decode_play_clientbound(id, &trailing)
        .unwrap_err()
        .to_string()
        .contains("trailing bytes after bundle delimiter packet"));
}

#[test]
fn decodes_initial_bbb_client_play_packets_from_existing_encoders() {
    let encoded = [
        encode_play_accept_teleportation(7),
        encode_play_client_tick_end(),
        encode_play_keep_alive(-123),
        encode_play_move_player_pos(1.0, 65.0, -2.0, true, false),
        encode_play_move_player_pos_rot(1.5, 66.0, -2.5, 90.0, -15.0, false, true),
        encode_play_move_player_rot(45.0, 10.0, true, true),
        encode_play_move_player_status_only(false, false),
        encode_play_player_loaded(),
    ];
    let decoded = encoded
        .iter()
        .map(|(id, payload)| decode_play_serverbound(*id, payload).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(decoded[0], PlayServerbound::AcceptTeleportation { id: 7 });
    assert_eq!(decoded[1], PlayServerbound::ClientTickEnd);
    assert_eq!(decoded[2], PlayServerbound::KeepAlive { id: -123 });
    assert_eq!(
        decoded[3],
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: 1.0,
                y: 65.0,
                z: -2.0,
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        })
    );
    assert_eq!(
        decoded[4],
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: 1.5,
                y: 66.0,
                z: -2.5,
            }),
            rotation: Some(PlayerRotation {
                y_rot: 90.0,
                x_rot: -15.0,
            }),
            on_ground: false,
            horizontal_collision: true,
        })
    );
    assert_eq!(
        decoded[5],
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: Some(PlayerRotation {
                y_rot: 45.0,
                x_rot: 10.0,
            }),
            on_ground: true,
            horizontal_collision: true,
        })
    );
    assert_eq!(
        decoded[6],
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        })
    );
    assert_eq!(decoded[7], PlayServerbound::PlayerLoaded);
}

#[test]
fn rejects_trailing_and_unknown_serverbound_play_data() {
    let (id, mut payload) = encode_play_move_player_status_only(true, false);
    payload.push(0);
    assert!(decode_play_serverbound(id, &payload)
        .unwrap_err()
        .to_string()
        .contains("trailing bytes"));
    assert!(matches!(
        decode_play_serverbound(999, &[]),
        Err(ProtocolError::UnknownPacket {
            state: "play serverbound",
            id: 999
        })
    ));
    assert!(decode_play_serverbound(ids::play::SERVERBOUND_CHAT, &[1, 2]).is_err());
}

#[test]
fn decodes_bbb_client_interaction_packets_from_existing_encoders() {
    let chat = ChatMessage::unsigned("hello fixture", 42, 7);
    let action = PlayerAction {
        action: PlayerActionKind::StartDestroyBlock,
        pos: BlockPos { x: 1, y: 63, z: 2 },
        direction: Direction::Up,
        sequence: 3,
    };
    let use_item_on = UseItemOn {
        hand: InteractionHand::MainHand,
        hit: BlockHitResult {
            pos: BlockPos { x: 1, y: 63, z: 2 },
            direction: Direction::Up,
            cursor_x: 0.5,
            cursor_y: 1.0,
            cursor_z: 0.5,
            inside: false,
            world_border_hit: false,
        },
        sequence: 4,
    };
    let creative = SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(crate::item_types::VANILLA_ITEM_STONE_ID),
            count: 32,
            component_patch: Default::default(),
        },
    };
    let mut encoded = vec![
        encode_play_chat_message(&chat),
        encode_play_player_action(action),
        encode_play_set_carried_item(2),
        encode_play_swing(InteractionHand::MainHand),
        encode_play_use_item_on(use_item_on),
    ];
    encoded.push(encode_play_set_creative_mode_slot(&creative).unwrap());
    let decoded = encoded
        .iter()
        .map(|(id, payload)| decode_play_serverbound(*id, payload).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(decoded[0], PlayServerbound::Chat(chat));
    assert_eq!(decoded[1], PlayServerbound::PlayerAction(action));
    assert_eq!(decoded[2], PlayServerbound::SetCarriedItem { slot: 2 });
    assert_eq!(
        decoded[3],
        PlayServerbound::Swing {
            hand: InteractionHand::MainHand
        }
    );
    assert_eq!(decoded[4], PlayServerbound::UseItemOn(use_item_on));
    assert_eq!(decoded[5], PlayServerbound::SetCreativeModeSlot(creative));
}

#[test]
fn encodes_login_and_position_for_the_existing_client_decoder() {
    let login = PlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 2,
        simulation_distance: 2,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 7,
            game_type: 1,
            previous_game_type: -1,
            is_debug: false,
            is_flat: true,
            last_death_location: Some(GlobalPos {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos { x: 1, y: 64, z: 2 },
            }),
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    };
    let (id, payload) = encode_play_clientbound_login(&login).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::Login(login)
    );

    let position = PlayerPositionUpdate {
        id: 3,
        position: Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        },
        delta_movement: Vec3d::default(),
        y_rot: 180.0,
        x_rot: 0.0,
        relatives_mask: 0,
    };
    let (id, payload) = encode_play_clientbound_player_position(position);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::PlayerPosition(position)
    );

    let (id, payload) = encode_play_clientbound_keep_alive(-456);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::KeepAlive { id: -456 }
    );
}

#[test]
fn encodes_respawn_for_the_existing_client_decoder() {
    // The two data-to-keep bytes vanilla actually sends -- 0 or 1 from
    // `PlayerList.respawn` (`PlayerList.java:409`, `:412`) and 3 from
    // `ServerPlayer.teleport` across a dimension change (`ServerPlayer.java:1104`)
    // -- plus 2, the `KEEP_ENTITY_DATA` bit on its own, which `shouldKeep` is
    // written to answer (`ClientboundRespawnPacket.java:13`, `:34-36`).
    let spawn_info = CommonPlayerSpawnInfo {
        dimension_type_id: 3,
        dimension: "minecraft:the_nether".to_string(),
        seed: -12,
        game_type: 0,
        previous_game_type: 1,
        is_debug: false,
        is_flat: false,
        last_death_location: None,
        portal_cooldown: 300,
        sea_level: 32,
    };
    for data_to_keep in [0, 1, 2, 3] {
        let respawn = Respawn {
            common_spawn_info: spawn_info.clone(),
            data_to_keep,
        };
        let (id, payload) = encode_play_clientbound_respawn(&respawn).unwrap();
        assert_eq!(id, crate::ids::play::CLIENTBOUND_RESPAWN);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::Respawn(respawn)
        );
    }
}

#[test]
fn encodes_chunk_center_and_bounded_chunk_for_the_existing_client_decoder() {
    let center = SetChunkCacheCenter {
        chunk_x: 2,
        chunk_z: -3,
    };
    let (id, payload) = encode_play_clientbound_set_chunk_cache_center(center);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetChunkCacheCenter(center)
    );

    let chunk = LevelChunkWithLight {
        x: 2,
        z: -3,
        chunk_data: LevelChunkData {
            heightmaps: Vec::new(),
            section_data: vec![0, 0, 0, 0, 0, 0],
            block_entities: Vec::new(),
        },
        light_data: LightUpdateData {
            sky_y_mask: Vec::new(),
            block_y_mask: Vec::new(),
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: Vec::new(),
            block_updates: Vec::new(),
        },
    };
    let (id, payload) = encode_play_clientbound_level_chunk_with_light(&chunk).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::LevelChunkWithLight(chunk)
    );
}

#[test]
fn encodes_set_time_for_the_existing_client_decoder() {
    let update = PlayTime {
        game_time: 12345,
        clock_updates: vec![
            ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.25,
                rate: 1.0,
            },
            ClockUpdate {
                clock_id: 1,
                total_ticks: 18000,
                partial_tick: 0.5,
                rate: 0.0,
            },
        ],
    };
    let (id, payload) = encode_play_clientbound_set_time(&update).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_SET_TIME);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetTime(update)
    );

    let empty = PlayTime {
        game_time: -40,
        clock_updates: Vec::new(),
    };
    let (id, payload) = encode_play_clientbound_set_time(&empty).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetTime(empty)
    );
}

#[test]
fn encodes_interaction_results_for_the_existing_client_decoder() {
    let stone_state = crate::block_state_types::vanilla_block_default_state_id(
        crate::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    let block = BlockUpdate {
        pos: BlockPos { x: 1, y: 64, z: 2 },
        block_state_id: stone_state,
    };
    let (id, payload) = encode_play_clientbound_block_update(block).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::BlockUpdate(block)
    );

    let section = super::super::SectionBlocksUpdate {
        section_x: -1,
        section_y: -4,
        section_z: 2,
        updates: vec![
            BlockUpdate {
                pos: BlockPos {
                    x: -15,
                    y: -63,
                    z: 33,
                },
                block_state_id: stone_state,
            },
            BlockUpdate {
                pos: BlockPos {
                    x: -1,
                    y: -49,
                    z: 47,
                },
                block_state_id: 0,
            },
        ],
    };
    let (id, payload) = encode_play_clientbound_section_blocks_update(&section).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SectionBlocksUpdate(section)
    );

    let ack = BlockChangedAck { sequence: 17 };
    let (id, payload) = encode_play_clientbound_block_changed_ack(ack);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::BlockChangedAck(ack)
    );

    let held = SetHeldSlot { slot: 3 };
    let (id, payload) = encode_play_clientbound_set_held_slot(held);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetHeldSlot(held)
    );

    let cooldown = Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 20,
    };
    let (id, payload) = encode_play_clientbound_cooldown(&cooldown);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::Cooldown(cooldown)
    );

    // `sendEffectToPassengers` (`LivingEntity.java:1062-1068`) is the one
    // producer this server reaches, and it always passes `blend = false`.
    let effect = UpdateMobEffect {
        entity_id: 123,
        effect_id: 5,
        amplifier: 2,
        duration_ticks: 600,
        flags: MobEffectFlags {
            raw: 0,
            ambient: true,
            visible: false,
            show_icon: true,
            blend: false,
        },
    };
    let (id, payload) = encode_play_clientbound_update_mob_effect(&effect).unwrap();
    let PlayClientbound::UpdateMobEffect(decoded) = decode_play_clientbound(id, &payload).unwrap()
    else {
        panic!("the update encodes as an update");
    };
    // `raw` is what the decode read off the wire, so it answers the bits the
    // encode wrote rather than the zero the fixture carries.
    assert_eq!(decoded.flags.raw, 1 | 4);
    assert_eq!(
        decoded,
        UpdateMobEffect {
            flags: MobEffectFlags {
                raw: 1 | 4,
                ..effect.flags
            },
            ..effect
        }
    );

    let removal = RemoveMobEffect {
        entity_id: 123,
        effect_id: 5,
    };
    let (id, payload) = encode_play_clientbound_remove_mob_effect(&removal).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::RemoveMobEffect(removal)
    );

    // Both encoders refuse a holder the registry does not know, the way the
    // decode does (`client_state::validate_mob_effect_holder_id`).
    assert!(encode_play_clientbound_update_mob_effect(&UpdateMobEffect {
        effect_id: i32::MAX,
        ..effect
    })
    .is_err());
    assert!(encode_play_clientbound_remove_mob_effect(&RemoveMobEffect {
        effect_id: i32::MAX,
        ..removal
    })
    .is_err());

    // The always-open player inventory is published as a whole menu.
    let menu = ContainerSetContent {
        container_id: 0,
        state_id: 1,
        items: vec![
            ItemStackSummary {
                item_id: Some(crate::item_types::VANILLA_ITEM_STONE_ID),
                count: 64,
                component_patch: Default::default(),
            },
            ItemStackSummary::empty(),
        ],
        carried_item: ItemStackSummary::empty(),
    };
    let (id, payload) = encode_play_clientbound_container_set_content(&menu).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ContainerSetContent(menu)
    );

    let chat = SystemChat::plain("<alice> hello fixture", false);
    let (id, payload) = encode_play_clientbound_system_chat(&chat).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SystemChat(chat)
    );
}

#[test]
fn encodes_the_join_session_authority_for_the_existing_client_decoder() {
    // `PlayerList.placeNewPlayer` (PlayerList.java:181-182) announces both
    // right after the login.
    for difficulty in [
        Difficulty::Peaceful,
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
    ] {
        for locked in [false, true] {
            let update = ChangeDifficulty { difficulty, locked };
            let (id, payload) = encode_play_clientbound_change_difficulty(update);
            assert_eq!(
                decode_play_clientbound(id, &payload).unwrap(),
                PlayClientbound::ChangeDifficulty(update)
            );
        }
    }

    for game_type in [
        GameType::Survival,
        GameType::Creative,
        GameType::Adventure,
        GameType::Spectator,
    ] {
        let abilities = PlayerAbilities::for_game_type(game_type);
        let (id, payload) = encode_play_clientbound_player_abilities(abilities);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::PlayerAbilities(abilities)
        );
    }

    // `GameType.updatePlayerAbilities` (GameType.java:62-80) over the
    // `Abilities` defaults (Abilities.java:7-20), which is what the packet
    // carries. `mayBuild` is set there too but never written to the wire.
    assert_eq!(
        PlayerAbilities::for_game_type(GameType::Creative),
        PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }
    );
    assert_eq!(
        PlayerAbilities::for_game_type(GameType::Spectator),
        PlayerAbilities {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }
    );
    assert_eq!(
        PlayerAbilities::for_game_type(GameType::Survival),
        PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }
    );
    assert_eq!(
        PlayerAbilities::for_game_type(GameType::Adventure),
        PlayerAbilities::for_game_type(GameType::Survival)
    );
}

#[test]
fn round_trips_the_chunk_batch_brackets_and_their_acknowledgement() {
    // `PlayerChunkSender.sendNextChunks` brackets every chunk it releases, and
    // the client answers the closing bracket with the rate it wants next.
    let (id, payload) = encode_play_clientbound_chunk_batch_start();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ChunkBatchStart
    );

    for batch_size in [0, 1, 9, 64] {
        let (id, payload) = encode_play_clientbound_chunk_batch_finished(batch_size);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::ChunkBatchFinished { batch_size }
        );
    }

    for desired_chunks_per_tick in [0.01f32, 1.0, 9.0, 64.0] {
        let (id, payload) = encode_play_chunk_batch_received(desired_chunks_per_tick);
        assert_eq!(
            decode_play_serverbound(id, &payload).unwrap(),
            PlayServerbound::ChunkBatchReceived {
                desired_chunks_per_tick
            }
        );
    }
}

#[test]
fn decodes_the_session_authority_requests_from_existing_encoders() {
    let encoded = [
        encode_play_change_difficulty(ChangeDifficultyCommand {
            difficulty: Difficulty::Hard,
        }),
        encode_play_lock_difficulty(LockDifficultyCommand { locked: true }),
        encode_play_player_abilities(PlayerAbilitiesCommand { flying: true }),
        encode_play_player_abilities(PlayerAbilitiesCommand { flying: false }),
    ];
    let decoded = encoded
        .iter()
        .map(|(id, payload)| decode_play_serverbound(*id, payload).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        decoded[0],
        PlayServerbound::ChangeDifficulty {
            difficulty: Difficulty::Hard,
        }
    );
    assert_eq!(decoded[1], PlayServerbound::LockDifficulty { locked: true });
    assert_eq!(
        decoded[2],
        PlayServerbound::PlayerAbilities { flying: true }
    );
    assert_eq!(
        decoded[3],
        PlayServerbound::PlayerAbilities { flying: false }
    );

    // `ServerboundPlayerAbilitiesPacket` reads a bitfield and keeps only
    // `FLAG_FLYING`, so the bits a client has no business setting are ignored
    // rather than rejected.
    assert_eq!(
        decode_play_serverbound(ids::play::SERVERBOUND_PLAYER_ABILITIES, &[0x0f]).unwrap(),
        PlayServerbound::PlayerAbilities { flying: true }
    );
    assert_eq!(
        decode_play_serverbound(ids::play::SERVERBOUND_PLAYER_ABILITIES, &[0x0d]).unwrap(),
        PlayServerbound::PlayerAbilities { flying: false }
    );
    // `Difficulty.BY_ID` wraps out-of-range ids (Difficulty.java:19).
    assert_eq!(
        decode_play_serverbound(ids::play::SERVERBOUND_CHANGE_DIFFICULTY, &[4]).unwrap(),
        PlayServerbound::ChangeDifficulty {
            difficulty: Difficulty::Peaceful,
        }
    );
}

#[test]
fn rejects_invalid_interaction_ordinals_and_block_states() {
    let mut invalid_action = Encoder::new();
    invalid_action.write_var_i32(99);
    invalid_action.write_i64(0);
    invalid_action.write_u8(Direction::Up.id());
    invalid_action.write_var_i32(0);
    assert!(decode_play_serverbound(
        ids::play::SERVERBOUND_PLAYER_ACTION,
        &invalid_action.into_inner()
    )
    .unwrap_err()
    .to_string()
    .contains("invalid player action ordinal"));

    assert!(encode_play_clientbound_block_update(BlockUpdate {
        pos: BlockPos { x: 0, y: 64, z: 0 },
        block_state_id: crate::block_state_types::VANILLA_BLOCK_STATE_COUNT,
    })
    .unwrap_err()
    .to_string()
    .contains("unknown vanilla 26.1 block state id"));
}

#[test]
fn rejects_inconsistent_light_before_encoding() {
    let chunk = LevelChunkWithLight {
        x: 0,
        z: 0,
        chunk_data: LevelChunkData {
            heightmaps: Vec::new(),
            section_data: Vec::new(),
            block_entities: Vec::new(),
        },
        light_data: LightUpdateData {
            sky_y_mask: vec![1],
            block_y_mask: Vec::new(),
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: Vec::new(),
            block_updates: Vec::new(),
        },
    };
    assert!(encode_play_clientbound_level_chunk_with_light(&chunk)
        .unwrap_err()
        .to_string()
        .contains("1 data sections but 0 arrays"));
}

/// The requests whose vanilla handler acts only on a menu, a vehicle or a
/// per-player book this world does not open, decoded from the client encoders
/// that already send them.
#[test]
fn decodes_the_menu_vehicle_and_book_requests_from_existing_encoders() {
    let encoded = [
        encode_play_move_vehicle(MoveVehicleCommand {
            position: Vec3d {
                x: 1.5,
                y: 65.0,
                z: -2.5,
            },
            y_rot: 90.0,
            x_rot: -15.0,
            on_ground: true,
        }),
        encode_play_paddle_boat(PaddleBoat {
            left: true,
            right: false,
        }),
        encode_play_recipe_book_change_settings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::BlastFurnace,
            open: true,
            filtering: false,
        }),
        encode_play_recipe_book_seen_recipe(RecipeBookSeenRecipeCommand {
            recipe: RecipeDisplayId { index: -1 },
        }),
        encode_play_rename_item(&RenameItem {
            name: "bbb anvil".to_string(),
        }),
        encode_play_seen_advancements(&SeenAdvancements::OpenedTab {
            tab: "minecraft:story/root".to_string(),
        }),
        encode_play_seen_advancements(&SeenAdvancements::ClosedScreen),
        encode_play_select_trade(SelectTradeCommand { item: 2 }),
        encode_play_set_beacon(SetBeacon {
            primary_effect: Some(crate::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID),
            secondary_effect: None,
        })
        .unwrap(),
        encode_play_place_recipe(PlaceRecipeCommand {
            container_id: 0,
            recipe_index: -1,
            use_max_items: true,
        }),
        encode_play_select_bundle_item(SelectBundleItem {
            slot_id: 38,
            selected_item_index: 1,
        }),
        encode_play_sign_update(&SignUpdate {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            is_front_text: true,
            lines: [
                "first".to_string(),
                String::new(),
                "third".to_string(),
                String::new(),
            ],
        }),
    ];
    let decoded = encoded
        .iter()
        .map(|(id, payload)| decode_play_serverbound(*id, payload).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        decoded[0],
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: 1.5,
                y: 65.0,
                z: -2.5,
            },
            y_rot: 90.0,
            x_rot: -15.0,
            on_ground: true,
        })
    );
    assert_eq!(
        decoded[1],
        PlayServerbound::PaddleBoat(PaddleBoat {
            left: true,
            right: false,
        })
    );
    assert_eq!(
        decoded[2],
        PlayServerbound::RecipeBookChangeSettings(RecipeBookChangeSettingsCommand {
            book_type: RecipeBookType::BlastFurnace,
            open: true,
            filtering: false,
        })
    );
    assert_eq!(
        decoded[3],
        PlayServerbound::RecipeBookSeenRecipe(RecipeBookSeenRecipeCommand {
            recipe: RecipeDisplayId { index: -1 },
        })
    );
    assert_eq!(
        decoded[4],
        PlayServerbound::RenameItem(RenameItem {
            name: "bbb anvil".to_string(),
        })
    );
    assert_eq!(
        decoded[5],
        PlayServerbound::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:story/root".to_string(),
        })
    );
    assert_eq!(
        decoded[6],
        PlayServerbound::SeenAdvancements(SeenAdvancements::ClosedScreen)
    );
    assert_eq!(
        decoded[7],
        PlayServerbound::SelectTrade(SelectTradeCommand { item: 2 })
    );
    assert_eq!(
        decoded[8],
        PlayServerbound::SetBeacon(SetBeacon {
            primary_effect: Some(crate::mob_effect_types::VANILLA_MOB_EFFECT_SPEED_ID),
            secondary_effect: None,
        })
    );

    assert_eq!(
        decoded[9],
        PlayServerbound::PlaceRecipe(PlaceRecipeCommand {
            container_id: 0,
            recipe_index: -1,
            use_max_items: true,
        })
    );
    assert_eq!(
        decoded[10],
        PlayServerbound::SelectBundleItem(SelectBundleItem {
            slot_id: 38,
            selected_item_index: 1,
        })
    );
    assert_eq!(
        decoded[11],
        PlayServerbound::SignUpdate(SignUpdate {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            is_front_text: true,
            lines: [
                "first".to_string(),
                String::new(),
                "third".to_string(),
                String::new(),
            ],
        })
    );

    // `ServerboundSelectBundleItemPacket(FriendlyByteBuf)` (:13-19) rejects a
    // negative index that is not the no-selection sentinel.
    let mut invalid_bundle_index = Encoder::new();
    invalid_bundle_index.write_var_i32(38);
    invalid_bundle_index.write_var_i32(-2);
    assert!(decode_play_serverbound(
        ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED,
        &invalid_bundle_index.into_inner()
    )
    .unwrap_err()
    .to_string()
    .contains("invalid selectedItemIndex: -2"));

    // `readIdentifier` normalizes a bare path to the `minecraft` namespace, so
    // the tab a client sends without one resolves to the same advancement.
    let mut bare_tab = Encoder::new();
    bare_tab.write_var_i32(0);
    bare_tab.write_string("story/root");
    assert_eq!(
        decode_play_serverbound(
            ids::play::SERVERBOUND_SEEN_ADVANCEMENTS,
            &bare_tab.into_inner()
        )
        .unwrap(),
        PlayServerbound::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:story/root".to_string(),
        })
    );

    // `readEnum` and `RecipeBookType.STREAM_CODEC` are both ordinal mappers
    // with no fallback, so an ordinal outside the set fails the read.
    assert!(
        decode_play_serverbound(ids::play::SERVERBOUND_SEEN_ADVANCEMENTS, &[2])
            .unwrap_err()
            .to_string()
            .contains("invalid seen advancements action ordinal 2")
    );
    assert!(decode_play_serverbound(
        ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS,
        &[4, 0, 0]
    )
    .unwrap_err()
    .to_string()
    .contains("invalid recipe book type ordinal 4"));
}

/// The stack `handleEditBook` leaves behind must survive the round trip the
/// menu synchronizer puts it through, or the edit reaches the client as a
/// decode failure instead of a book.
#[test]
fn round_trips_the_stacks_the_book_editor_writes() {
    use crate::data_component_types::{
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    };
    use crate::item_types::{VANILLA_ITEM_WRITABLE_BOOK_ID, VANILLA_ITEM_WRITTEN_BOOK_ID};
    use crate::packets::WrittenBookContentSummary;

    let saved = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_WRITABLE_BOOK_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID],
            writable_book_pages: vec!["bbb book probe".into()],
            writable_book_page_filters: vec![None],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let signed = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_WRITTEN_BOOK_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID],
            written_book: Some(WrittenBookContentSummary {
                title: "bbb tales".into(),
                title_filter: None,
                author: "server-probe".into(),
                generation: 0,
                pages: vec!["bbb book probe".to_string()],
                page_filters: vec![None],
                resolved: true,
            }),
            written_book_pages_nbt: vec![Some(crate::component_literal_nbt("bbb book probe"))],
            written_book_page_filters_nbt: vec![None],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    for (label, stack) in [("saved", saved), ("signed", signed)] {
        let update = ContainerSetContent {
            container_id: 0,
            state_id: 1,
            items: vec![stack.clone()],
            carried_item: ItemStackSummary::empty(),
        };
        let (id, payload) = encode_play_clientbound_container_set_content(&update).unwrap();
        let decoded = decode_play_clientbound(id, &payload)
            .unwrap_or_else(|error| panic!("{label} book stack: {error}"));
        assert_eq!(
            decoded,
            PlayClientbound::ContainerSetContent(update),
            "{label}"
        );

        let request = SetCreativeModeSlot {
            slot_num: 36,
            item: stack,
        };
        let (id, payload) = encode_play_set_creative_mode_slot(&request).unwrap();
        let decoded = decode_play_serverbound(id, &payload)
            .unwrap_or_else(|error| panic!("{label} creative book stack: {error}"));
        assert_eq!(
            decoded,
            PlayServerbound::SetCreativeModeSlot(request),
            "{label}"
        );
    }
}

/// `ItemStack.OPTIONAL_STREAM_CODEC` (ItemStack.java:125) hands the component value
/// straight to its codec; `OPTIONAL_UNTRUSTED_STREAM_CODEC` (:126-128), which only
/// `ServerboundSetCreativeModeSlotPacket` uses, length-prefixes it instead.
#[test]
fn only_the_creative_slot_request_length_prefixes_its_component_values() {
    use crate::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
    use crate::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID;

    let stack = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_WRITABLE_BOOK_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
            damage: Some(7),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };

    let (_, direct) = encode_play_clientbound_container_set_slot(&ContainerSetSlot {
        container_id: 0,
        state_id: 1,
        slot: 36,
        item: stack.clone(),
    })
    .unwrap();
    let (_, delimited) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: stack,
    })
    .unwrap();

    let tail = |payload: &[u8]| payload[payload.len() - 3..].to_vec();
    assert_eq!(
        tail(&direct),
        vec![0, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID as u8, 7],
        "clientbound stacks write the damage value straight after its type id"
    );
    assert_eq!(
        tail(&delimited),
        vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID as u8, 1, 7],
        "the creative slot request prefixes the damage value with its length"
    );
}

#[test]
fn creative_slot_rejects_zero_enchantments_while_clientbound_stacks_preserve_them() {
    use crate::data_component_types::{
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    };
    use crate::item_types::VANILLA_ITEM_STONE_ID;
    use crate::packets::data_components::{
        encode_data_component_patch_summary_with_framing, ComponentValueFraming,
    };
    use crate::packets::ItemEnchantmentSummary;

    for type_id in [
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    ] {
        let mut patch = DataComponentPatchSummary {
            added_type_ids: vec![type_id],
            ..DataComponentPatchSummary::default()
        };
        let entries = vec![ItemEnchantmentSummary {
            holder_id: 13,
            level: 0,
        }];
        if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
            patch.enchantments = entries;
        } else {
            patch.stored_enchantments = entries;
        }

        let mut raw = Encoder::new();
        raw.write_i16(36);
        raw.write_var_i32(1);
        raw.write_var_i32(VANILLA_ITEM_STONE_ID);
        encode_data_component_patch_summary_with_framing(
            &mut raw,
            &patch,
            ComponentValueFraming::LengthPrefixed,
        )
        .unwrap();
        assert!(decode_play_serverbound(
            ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT,
            &raw.into_inner(),
        )
        .is_err());

        let update = ContainerSetSlot {
            container_id: 0,
            state_id: 1,
            slot: 36,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_STONE_ID),
                count: 1,
                component_patch: patch.into(),
            },
        };
        let (id, payload) = encode_play_clientbound_container_set_slot(&update).unwrap();
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::ContainerSetSlot(update)
        );
    }
}

#[test]
fn encodes_block_destruction_stages_for_the_existing_client_decoder() {
    // `ServerPlayerGameMode.incrementDestroyProgress` (:135-138) sends
    // `(int)(destroyProgress * 10.0F)`, so a live stage is 0..=9, and the three
    // clears (`:123`, `:224`, `:247`) send `-1`.
    for progress in [0_u8, 4, 9, 0xff] {
        let update = chunks::BlockDestruction {
            id: 42,
            pos: BlockPos { x: -3, y: 70, z: 9 },
            progress,
        };
        let (id, payload) = encode_play_clientbound_block_destruction(update);
        assert_eq!(id, crate::ids::play::CLIENTBOUND_BLOCK_DESTRUCTION);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::BlockDestruction(update)
        );
    }
}

#[test]
fn encodes_block_events_for_the_existing_client_decoder() {
    let note_block = chunks::BlockEvent {
        pos: BlockPos { x: -3, y: 70, z: 9 },
        b0: 0,
        b1: 12,
        block_id: crate::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID,
    };
    let (id, payload) = encode_play_clientbound_block_event(note_block).unwrap();
    assert_eq!(id, crate::ids::play::CLIENTBOUND_BLOCK_EVENT);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::BlockEvent(note_block)
    );

    assert!(encode_play_clientbound_block_event(chunks::BlockEvent {
        pos: BlockPos { x: 0, y: 0, z: 0 },
        b0: 0,
        b1: 0,
        block_id: i32::MAX,
    })
    .is_err());
}

#[test]
fn encodes_level_events_for_the_existing_client_decoder() {
    // `ClientboundLevelEventPacket.write` (`:32-37`) is two plain ints, a
    // packed block position and a boolean, in that order. The ids used here are
    // the two this server raises -- `LevelEvent.PARTICLES_DESTROY_BLOCK`
    // (`LevelEvent.java:53`) and `LevelEvent.LAVA_FIZZ` (`:47`) -- plus one the
    // client has no case for, because an unknown id is dropped by
    // `LevelEventHandler` rather than refused on the wire.
    for update in [
        chunks::LevelEvent {
            event_type: 2001,
            pos: BlockPos { x: -3, y: 70, z: 9 },
            data: 1234,
            global: false,
        },
        chunks::LevelEvent {
            event_type: 1501,
            pos: BlockPos {
                x: 30_000_000,
                y: -64,
                z: -30_000_000,
            },
            data: 0,
            global: true,
        },
        chunks::LevelEvent {
            event_type: i32::MIN,
            pos: BlockPos { x: 0, y: 0, z: 0 },
            data: i32::MAX,
            global: false,
        },
    ] {
        let (id, payload) = encode_play_clientbound_level_event(update);
        assert_eq!(id, crate::ids::play::CLIENTBOUND_LEVEL_EVENT);
        // Four bytes, eight bytes, four bytes, one byte: nothing is a var-int.
        assert_eq!(payload.len(), 17);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::LevelEvent(update),
            "level event {} did not round trip",
            update.event_type
        );
    }
}

#[test]
fn encodes_every_game_event_type_for_the_existing_client_decoder() {
    // `ClientboundGameEventPacket` declares fourteen `Type` constants
    // (`ClientboundGameEventPacket.java:14-27`) and the generated catalog is that
    // exact set, so the round trip has to hold for every one of them. The params
    // are the ones vanilla actually pairs with each event: a game type id for
    // `CHANGE_GAME_MODE` (`ClientPacketListener.java:1547`), the `DEMO_PARAM_*`
    // constants for `DEMO_EVENT` (`ClientboundGameEventPacket.java:28-32`), and a
    // weather level for the two level changes (`ClientPacketListener.java:1581`,
    // `:1583`).
    for entry in crate::game_event_types::VANILLA_CLIENT_GAME_EVENT_TYPES {
        for param in [0.0_f32, 1.0, 104.0, 0.75, -1.0] {
            let update = GameEvent {
                event_id: entry.id,
                param,
            };
            let (id, payload) = encode_play_clientbound_game_event(update).unwrap();
            assert_eq!(id, crate::ids::play::CLIENTBOUND_GAME_EVENT);
            assert_eq!(payload.len(), 5);
            assert_eq!(
                decode_play_clientbound(id, &payload).unwrap(),
                PlayClientbound::GameEvent(update),
                "game event {} did not round trip",
                entry.constant_name
            );
        }
    }
}

#[test]
fn rejects_game_event_ids_vanilla_has_no_type_for() {
    // `Type.TYPES` only holds the ids the packet's own constants registered
    // (`ClientboundGameEventPacket.java:68-76`), so the first free id past
    // `LEVEL_CHUNKS_LOAD_START` and the top of the unsigned byte both resolve to a
    // null `Type` the client silently drops
    // (`ClientPacketListener.java:1534-1598`).
    for event_id in [
        crate::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID + 1,
        u8::MAX,
    ] {
        assert!(encode_play_clientbound_game_event(GameEvent {
            event_id,
            param: 0.0,
        })
        .is_err());
    }
}

#[test]
fn encodes_ticking_state_for_the_existing_client_decoder() {
    // The default rate and both freeze states (`TickRateManager.java:9`, `:13`),
    // the `/tick rate` argument's two ends (`TickCommand.java:28`), and a rate
    // under `MIN_TICKRATE` that the wire still carries because the clamp lives in
    // `TickRateManager.setTickRate` (`TickRateManager.java:8`, `:15-18`) rather
    // than in the packet.
    for (tick_rate, frozen) in [
        (20.0_f32, false),
        (20.0, true),
        (1.0, true),
        (10000.0, false),
        (0.5, true),
    ] {
        let update = TickingState { tick_rate, frozen };
        let (id, payload) = encode_play_clientbound_ticking_state(update);
        assert_eq!(id, crate::ids::play::CLIENTBOUND_TICKING_STATE);
        assert_eq!(payload.len(), 5);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::TickingState(update)
        );
    }
}

#[test]
fn encodes_ticking_step_for_the_existing_client_decoder() {
    // `stopStepping` sends 0 (`ServerTickRateManager.java:51-59`), `/tick step`
    // defaults to 1 and otherwise passes a `TimeArgument.time(1)` tick count
    // (`TickCommand.java:35`, `:38`) such as one in-game day, and the var int
    // still carries the values outside that band because it is a plain
    // `writeVarInt` (`ClientboundTickingStepPacket.java:22-24`).
    for tick_steps in [0, 1, 24000, i32::MAX, -1, i32::MIN] {
        let update = TickingStep { tick_steps };
        let (id, payload) = encode_play_clientbound_ticking_step(update);
        assert_eq!(id, crate::ids::play::CLIENTBOUND_TICKING_STEP);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::TickingStep(update)
        );
    }
}

#[test]
fn encodes_set_health_for_the_existing_client_decoder() {
    // The band `/damage` and the death path move the health across: a full
    // player is at the `minecraft:max_health` attribute's twenty
    // (`LivingEntity.java:1954-1956`), `setHealth` clamps into `[0, maxHealth]`
    // (`LivingEntity.java:1146-1148`) so zero is the bottom, and the fractional
    // values in between are what a float carries and an int would lose. The
    // food and saturation are `FoodData`'s own defaults, which is what a server
    // that models no hunger sends beside the health.
    for health in [20.0, 19.5, 0.5, 0.0] {
        let update = PlayerHealth {
            health,
            food: 20,
            saturation: 5.0,
        };
        let (id, payload) = encode_play_clientbound_set_health(update);
        assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_HEALTH);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::SetHealth(update)
        );
    }
}

/// `ClientboundSetExperiencePacket` writes the *level* between the progress and
/// the total (`ClientboundSetExperiencePacket.java:28-32`), which is not the
/// constructor's argument order -- so the round trip is what proves the encoder
/// did not follow the constructor.
#[test]
fn encodes_set_experience_for_the_existing_client_decoder() {
    for (progress, level, total) in [(0.0, 0, 0), (0.5, 1, 12), (0.9999, 31, 1_395)] {
        let update = PlayerExperience {
            progress,
            level,
            total,
        };
        let (id, payload) = encode_play_clientbound_set_experience(update);
        assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_EXPERIENCE);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::SetExperience(update)
        );
    }
    // The byte after the float is the level, not the total: a swap round-trips
    // through this crate's own decoder just fine and only a real client would
    // notice, so the order is pinned against the bytes.
    let (_, payload) = encode_play_clientbound_set_experience(PlayerExperience {
        progress: 0.0,
        level: 1,
        total: 12,
    });
    assert_eq!(payload, vec![0, 0, 0, 0, 1, 12]);
}

#[test]
fn encodes_damage_event_for_the_existing_client_decoder() {
    // Both source shapes `DamageSource` can be built in
    // (`DamageSource.java:34-57`) and therefore both shapes
    // `ClientboundDamageEventPacket`'s entity constructor projects
    // (`ClientboundDamageEventPacket.java:22-30`): a source with no entity and
    // no position at all -- which is what `damageSources().generic()` is and
    // what `/damage <target> <amount>` uses -- writes `-1` for both ids, and a
    // source built from one entity writes that entity's id twice, because the
    // one-argument constructor passes it as both the direct and the causing
    // entity (`:51-53`).
    for update in [
        DamageEvent {
            entity_id: 42,
            source_type_id: 11,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        },
        DamageEvent {
            entity_id: 42,
            source_type_id: 11,
            source_cause_id: 7,
            source_direct_id: 7,
            source_position: None,
        },
        DamageEvent {
            entity_id: 42,
            source_type_id: 11,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: Some(Vec3d {
                x: 0.5,
                y: -64.0,
                z: 1.5,
            }),
        },
    ] {
        let (id, payload) = encode_play_clientbound_damage_event(update);
        assert_eq!(id, crate::ids::play::CLIENTBOUND_DAMAGE_EVENT);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::DamageEvent(update)
        );
    }
    // The absent id is the zero on the wire, not a negative varint: the sender
    // offsets by one (`ClientboundDamageEventPacket.java:42-44`) so that the
    // whole payload of an entity-less source stays five bytes.
    let (_, payload) = encode_play_clientbound_damage_event(DamageEvent {
        entity_id: 0,
        source_type_id: 0,
        source_cause_id: -1,
        source_direct_id: -1,
        source_position: None,
    });
    assert_eq!(payload, [0, 0, 0, 0, 0]);
}

#[test]
fn encodes_player_combat_kill_for_the_existing_client_decoder() {
    // The two messages `ServerPlayer.die` sends: the composed death message
    // when `showDeathMessages` is on (`ServerPlayer.java:871-876`) and
    // `CommonComponents.EMPTY` when it is off (`:898-900`). The empty one is
    // still a component on the wire, so both round-trip through the same codec.
    for message in ["alice was slain by bob", ""] {
        let update = PlayerCombatKill {
            player_id: 3,
            message: RichTextComponent::plain(message),
        };
        let (id, payload) = encode_play_clientbound_player_combat_kill(&update).unwrap();
        assert_eq!(id, crate::ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::PlayerCombatKill(update)
        );
    }
}

/// `EntityDataSerializers.ITEM_STACK` (`EntityDataSerializers.java:62-66`) is
/// `ItemStack.OPTIONAL_STREAM_CODEC`, so the stack an `ItemEntity` publishes
/// with its pairing data comes back off the wire unchanged.
#[test]
fn encodes_an_item_stack_entity_data_value_for_the_existing_client_decoder() {
    let update = SetEntityData {
        id: 91,
        values: vec![EntityDataValue {
            data_id: crate::entity_synched_data::VANILLA_ITEM_ENTITY_ITEM_DATA_ID,
            serializer_id:
                crate::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(ItemStackSummary {
                item_id: Some(crate::item_types::VANILLA_ITEM_DANDELION_ID),
                count: 3,
                component_patch: Default::default(),
            }),
        }],
    };
    let (id, payload) = encode_play_clientbound_set_entity_data(&update).unwrap();
    assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_ENTITY_DATA);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetEntityData(update)
    );
}

/// `BOOLEAN` is `ByteBufCodecs.BOOL` and `OPTIONAL_BLOCK_POS` applies the
/// standard optional wrapper to `BlockPos.STREAM_CODEC`
/// (`EntityDataSerializers.java:88`, `:95-97`).
#[test]
fn encodes_boolean_and_optional_block_pos_entity_data_values() {
    let beam_target = BlockPos {
        x: -9,
        y: 73,
        z: 14,
    };
    let update = SetEntityData {
        id: 45,
        values: vec![
            EntityDataValue {
                data_id: crate::entity_synched_data::VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
                serializer_id:
                    crate::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(false),
            },
            EntityDataValue {
                data_id: crate::entity_synched_data::VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID,
                serializer_id: crate::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                value: EntityDataValueKind::OptionalBlockPos(Some(beam_target)),
            },
            EntityDataValue {
                data_id: crate::entity_synched_data::VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID,
                serializer_id: crate::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                value: EntityDataValueKind::OptionalBlockPos(None),
            },
        ],
    };
    let (id, payload) = encode_play_clientbound_set_entity_data(&update).unwrap();
    assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_ENTITY_DATA);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetEntityData(update)
    );
}

/// Every value kind this server publishes travels: the encoder is the inverse
/// of the decoder the client already uses, so each one comes back identical.
#[test]
fn encodes_every_entity_data_value_kind_this_server_publishes() {
    use crate::entity_data_serializers::*;
    use crate::packets::{EntityDataRegistryHolder, EntityDataRegistryReference, ParticlePayload};

    let component = RichTextComponent::plain("bbb");
    let values = vec![
        (
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(-7),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
            EntityDataValueKind::Long(1 << 40),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(20.0),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID,
            EntityDataValueKind::String("bbb probe".into()),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
            EntityDataValueKind::Component(component.clone()),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
            EntityDataValueKind::OptionalComponent(Some(component.clone())),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
            EntityDataValueKind::OptionalComponent(None),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
            EntityDataValueKind::Rotations {
                x: 1.5,
                y: -2.0,
                z: 0.25,
            },
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
            EntityDataValueKind::Direction(3),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
            EntityDataValueKind::OptionalLivingEntityReference(Some(Uuid::from_u128(42))),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
            EntityDataValueKind::OptionalLivingEntityReference(None),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID,
            EntityDataValueKind::BlockState(11),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
            EntityDataValueKind::OptionalBlockState(Some(12)),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
            EntityDataValueKind::OptionalBlockState(None),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_PARTICLE_ID,
            EntityDataValueKind::Particle(ParticlePayload {
                particle_type_id: 5,
                raw_options: Vec::new(),
            }),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID,
            EntityDataValueKind::Particles(vec![
                ParticlePayload {
                    particle_type_id: 5,
                    raw_options: Vec::new(),
                },
                ParticlePayload {
                    particle_type_id: 6,
                    raw_options: Vec::new(),
                },
            ]),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_VILLAGER_DATA_ID,
            EntityDataValueKind::VillagerData {
                villager_type: EntityDataRegistryReference::unresolved(2),
                profession: EntityDataRegistryReference::unresolved(3),
                level: 4,
            },
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
            EntityDataValueKind::OptionalUnsignedInt(Some(0)),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
            EntityDataValueKind::OptionalUnsignedInt(None),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            EntityDataValueKind::Pose(1),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
            EntityDataValueKind::Vector3f {
                x: 0.5,
                y: 1.0,
                z: -1.5,
            },
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID,
            EntityDataValueKind::Quaternionf {
                x: 0.0,
                y: 0.5,
                z: 1.0,
                w: -0.5,
            },
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_CAT_VARIANT_ID,
            EntityDataValueKind::RegistryId {
                serializer: EntityDataRegistryHolder::CatVariant,
                value: EntityDataRegistryReference::unresolved(9),
            },
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
            EntityDataValueKind::PaintingVariant(PaintingVariantData {
                registry: Some(EntityDataRegistryReference::unresolved(4)),
                direct: None,
            }),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
            EntityDataValueKind::PaintingVariant(PaintingVariantData {
                registry: None,
                direct: Some(crate::packets::DirectPaintingVariantData {
                    width: 2,
                    height: 3,
                    asset_id: "minecraft:kebab".to_string(),
                    title: Some(component.clone()),
                    author: None,
                }),
            }),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
            EntityDataValueKind::HumanoidArm(0),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_RESOLVABLE_PROFILE_ID,
            EntityDataValueKind::ResolvableProfile(crate::packets::ResolvableProfileData {
                profile: crate::packets::ResolvableProfileKind::GameProfile {
                    id: Uuid::from_u128(7),
                    name: "Steve".to_string(),
                    properties: vec![crate::packets::GameProfileProperty {
                        name: "textures".to_string(),
                        value: "eyJ0ZXh0dXJlcyI6e319".to_string(),
                        signature: Some("signed-texture".to_string()),
                    }],
                },
                skin_patch: crate::packets::PlayerSkinPatchData {
                    body: Some("minecraft:entity/player/wide/steve".to_string()),
                    cape: None,
                    elytra: None,
                    slim_model: Some(true),
                },
            }),
        ),
        (
            VANILLA_ENTITY_DATA_SERIALIZER_RESOLVABLE_PROFILE_ID,
            EntityDataValueKind::ResolvableProfile(crate::packets::ResolvableProfileData {
                profile: crate::packets::ResolvableProfileKind::Partial {
                    name: Some("Alex".to_string()),
                    id: None,
                    properties: Vec::new(),
                },
                skin_patch: crate::packets::PlayerSkinPatchData {
                    body: None,
                    cape: None,
                    elytra: None,
                    slim_model: None,
                },
            }),
        ),
    ];

    for (serializer_id, value) in values {
        let update = SetEntityData {
            id: 77,
            values: vec![EntityDataValue {
                data_id: 9,
                serializer_id,
                value,
            }],
        };
        let (id, payload) = encode_play_clientbound_set_entity_data(&update).unwrap();
        assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_ENTITY_DATA);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::SetEntityData(update)
        );
    }
}

#[test]
fn encodes_animate_for_the_existing_client_decoder() {
    // The two actions `LivingEntity.swing` picks between (LivingEntity.java:1999,
    // ClientboundAnimatePacket.java:13, `:15`), plus the wake overlay the same
    // packet carries (`:14`) and the two crit actions (`:16-17`) -- the class
    // writes the action as a plain byte (`:33`), so every id it names travels.
    for action in [0_u8, 2, 3, 4, 5] {
        for id in [0_i32, 1, i32::MAX] {
            let update = EntityAnimation { id, action };
            let (packet_id, payload) = encode_play_clientbound_animate(update);
            assert_eq!(packet_id, crate::ids::play::CLIENTBOUND_ANIMATE);
            assert_eq!(
                decode_play_clientbound(packet_id, &payload).unwrap(),
                PlayClientbound::EntityAnimation(update)
            );
        }
    }
}

#[test]
fn encodes_set_camera_for_the_existing_client_decoder() {
    // `setCamera` sends the player's own id to stop spectating and the target's
    // to start (ServerPlayer.java:1918, `:1928`), so both are ordinary entity
    // ids and nothing else is representable.
    for camera_id in [0_i32, 1, 12345, i32::MAX] {
        let update = SetCamera { camera_id };
        let (packet_id, payload) = encode_play_clientbound_set_camera(update);
        assert_eq!(packet_id, crate::ids::play::CLIENTBOUND_SET_CAMERA);
        assert_eq!(
            decode_play_clientbound(packet_id, &payload).unwrap(),
            PlayClientbound::SetCamera(update)
        );
    }
}

#[test]
fn encodes_player_rotation_for_the_existing_client_decoder() {
    // The four combinations of the two relative flags `RotateCommand` produces
    // (RotateCommand.java:68-70), over a yaw outside `[-180, 180]` and a pitch
    // outside `[-90, 90]`: the packet carries what it was handed and the client
    // is what wraps and clamps (`Entity.forceSetRotation`, Entity.java:3091-3100).
    for relative_y in [false, true] {
        for relative_x in [false, true] {
            for (y_rot, x_rot) in [(0.0_f32, 0.0_f32), (270.5, -135.25), (-540.0, 91.0)] {
                let update = PlayerRotationUpdate {
                    y_rot,
                    relative_y,
                    x_rot,
                    relative_x,
                };
                let (packet_id, payload) = encode_play_clientbound_player_rotation(update);
                assert_eq!(packet_id, crate::ids::play::CLIENTBOUND_PLAYER_ROTATION);
                assert_eq!(payload.len(), 10);
                assert_eq!(
                    decode_play_clientbound(packet_id, &payload).unwrap(),
                    PlayClientbound::PlayerRotation(update)
                );
            }
        }
    }
}

#[test]
fn encodes_transfer_for_the_existing_client_decoder() {
    // The default port `/transfer <hostname>` fills in (TransferCommand.java:28)
    // and both ends of the `<port>` argument's range (`:30`).
    for (host, port) in [("localhost", 25565), ("example.invalid", 1), ("::1", 65535)] {
        let update = Transfer {
            host: host.into(),
            port,
        };
        let (packet_id, payload) = encode_play_clientbound_transfer(&update).unwrap();
        assert_eq!(packet_id, crate::ids::play::CLIENTBOUND_TRANSFER);
        assert_eq!(
            decode_play_clientbound(packet_id, &payload).unwrap(),
            PlayClientbound::Transfer(update)
        );
    }
}

#[test]
fn a_transfer_host_past_the_writeutf_bound_is_refused_rather_than_truncated() {
    // `Utf8String.write` throws above 32767 characters (Utf8String.java:35-37),
    // so the encoder refuses there rather than putting a string on the wire the
    // official server would never have written.
    let update = Transfer {
        host: "a".repeat(32767).into(),
        port: 25565,
    };
    assert!(encode_play_clientbound_transfer(&update).is_ok());
    let update = Transfer {
        host: "a".repeat(32768).into(),
        port: 25565,
    };
    assert!(encode_play_clientbound_transfer(&update).is_err());

    let update = Transfer {
        host: ("😀".repeat(16383) + "a").into(),
        port: 25565,
    };
    assert!(encode_play_clientbound_transfer(&update).is_ok());
    let update = Transfer {
        host: "😀".repeat(16384).into(),
        port: 25565,
    };
    assert!(encode_play_clientbound_transfer(&update).is_err());
}

/// `ClientboundOpenScreenPacket` and `ClientboundContainerClosePacket` are the
/// two halves of `ServerPlayer.openMenu` (`ServerPlayer.java:1337`) and
/// `ServerPlayer.closeContainer` (`ServerPlayer.java:1400`). Both are decoded
/// by the client half of this crate already, so the encoders are pinned
/// against those decoders rather than against a byte literal written here.
#[test]
fn the_menu_open_and_close_packets_round_trip_through_the_client_decoder() {
    use crate::component::{component_translatable_nbt, ComponentStyle, RichTextComponent};
    use crate::menu_types::{VANILLA_MENU_TYPE_GENERIC_9X3_ID, VANILLA_MENU_TYPE_GENERIC_9X6_ID};
    use crate::packets::{ContainerClose, OpenScreen};

    for (menu_type_id, key, container_id) in [
        // `ChestBlockEntity.DEFAULT_NAME` (`ChestBlockEntity.java:30`).
        (VANILLA_MENU_TYPE_GENERIC_9X3_ID, "container.chest", 1),
        // `ChestBlock.MENU_PROVIDER_COMBINER`'s double name
        // (`ChestBlock.java:112-118`).
        (
            VANILLA_MENU_TYPE_GENERIC_9X6_ID,
            "container.chestDouble",
            100,
        ),
    ] {
        let title = RichTextComponent::try_from_component_nbt(
            component_translatable_nbt(key, Vec::new(), &ComponentStyle::default()).unwrap(),
        )
        .unwrap();
        let update = OpenScreen {
            container_id,
            menu_type_id,
            title,
        };
        let (id, payload) = encode_play_clientbound_open_screen(&update).unwrap();
        assert_eq!(id, crate::ids::play::CLIENTBOUND_OPEN_SCREEN);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::OpenScreen(update)
        );

        let close = ContainerClose { container_id };
        let (id, payload) = encode_play_clientbound_container_close(close);
        assert_eq!(id, crate::ids::play::CLIENTBOUND_CONTAINER_CLOSE);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::ContainerClose(close)
        );
    }
}

/// `ByteBufCodecs.registry(Registries.MENU)` (`ByteBufCodecs.java:580-582`)
/// writes `mapper.getIdOrThrow(value)`, which throws for a value the registry
/// does not hold. Nothing but a registered menu type can reach the wire.
#[test]
fn the_open_screen_encoder_refuses_a_menu_type_the_registry_does_not_hold() {
    use crate::component::RichTextComponent;
    use crate::menu_types::VANILLA_MENU_TYPES;

    let past_the_end = VANILLA_MENU_TYPES.len() as i32;
    for menu_type_id in [-1, past_the_end] {
        let update = crate::packets::OpenScreen {
            container_id: 1,
            menu_type_id,
            title: RichTextComponent::plain("x"),
        };
        assert!(encode_play_clientbound_open_screen(&update).is_err());
    }
}

#[test]
fn round_trips_the_clientbound_set_objective_packet() {
    use crate::component::RichTextComponent;
    use crate::packets::{
        encode_play_clientbound_set_objective, NumberFormatSummary, ObjectiveRenderType,
        SetObjective, SetObjectiveMethod, SetObjectiveParameters,
    };

    let added = SetObjective {
        objective_name: "kills".into(),
        method: SetObjectiveMethod::Add.into(),
        parameters: Some(SetObjectiveParameters {
            display_name: RichTextComponent::plain("Kills"),
            render_type: ObjectiveRenderType::Hearts,
            number_format: Some(NumberFormatSummary::Fixed(RichTextComponent::plain("--"))),
        }),
    };
    let (id, payload) = encode_play_clientbound_set_objective(&added).unwrap();
    assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_OBJECTIVE);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetObjective(added.clone())
    );

    // The remove method carries no parameter payload at all
    // (`ClientboundSetObjectivePacket.java:54-58`).
    let removed = SetObjective {
        objective_name: "kills".into(),
        method: SetObjectiveMethod::Remove.into(),
        parameters: None,
    };
    let (id, payload) = encode_play_clientbound_set_objective(&removed).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetObjective(removed)
    );

    assert!(encode_play_clientbound_set_objective(&SetObjective {
        parameters: None,
        ..added
    })
    .is_err());
}

#[test]
fn round_trips_the_clientbound_set_score_packet() {
    use crate::component::{ComponentStyle, RichTextComponent, RichTextStyle};
    use crate::packets::{encode_play_clientbound_set_score, NumberFormatSummary, SetScore};

    let styled = RichTextStyle::try_from_style(ComponentStyle {
        bold: Some(true),
        ..ComponentStyle::default()
    })
    .unwrap();
    let score = SetScore {
        owner: "Steve".into(),
        objective_name: "kills".into(),
        score: 42,
        display: Some(RichTextComponent::plain("Forty two")),
        number_format: Some(NumberFormatSummary::Styled(styled)),
    };
    let (id, payload) = encode_play_clientbound_set_score(&score).unwrap();
    assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_SCORE);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetScore(score.clone())
    );

    let bare = SetScore {
        display: None,
        number_format: Some(NumberFormatSummary::Blank),
        ..score
    };
    let (id, payload) = encode_play_clientbound_set_score(&bare).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetScore(bare)
    );
}

#[test]
fn round_trips_the_clientbound_reset_score_packet() {
    use crate::packets::{encode_play_clientbound_reset_score, ResetScore};

    let one = ResetScore {
        owner: "Steve".into(),
        objective_name: Some("kills".into()),
    };
    let (id, payload) = encode_play_clientbound_reset_score(&one).unwrap();
    assert_eq!(id, crate::ids::play::CLIENTBOUND_RESET_SCORE);
    // The objective is a nullable string, so a present one is a true flag ahead
    // of its length (`ClientboundResetScorePacket.java:18-21`).
    assert_eq!(payload, b"\x05Steve\x01\x05kills".to_vec());
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ResetScore(one.clone())
    );

    // `onPlayerRemoved` (`ServerScoreboard.java:77-81`) resets the holder
    // against every objective at once, which is the absent name.
    let all = ResetScore {
        objective_name: None,
        ..one
    };
    let (id, payload) = encode_play_clientbound_reset_score(&all).unwrap();
    assert_eq!(payload, b"\x05Steve\x00".to_vec());
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ResetScore(all)
    );
}

#[test]
fn round_trips_the_clientbound_set_display_objective_packet() {
    use crate::packets::{
        encode_play_clientbound_set_display_objective, ScoreboardDisplaySlot, SetDisplayObjective,
    };

    let shown = SetDisplayObjective {
        slot: ScoreboardDisplaySlot::Sidebar,
        objective_name: Some("kills".into()),
    };
    let (id, payload) = encode_play_clientbound_set_display_objective(&shown).unwrap();
    assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE);
    // The objective is a plain string here, so its length follows the slot with
    // no nullability flag between
    // (`ClientboundSetDisplayObjectivePacket.java:33-36`).
    assert_eq!(payload, b"\x01\x05kills".to_vec());
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetDisplayObjective(shown)
    );

    // A cleared slot is the empty string
    // (`ClientboundSetDisplayObjectivePacket.java:19-26`, `:51-53`).
    let cleared = SetDisplayObjective {
        slot: ScoreboardDisplaySlot::BelowName,
        objective_name: None,
    };
    let (id, payload) = encode_play_clientbound_set_display_objective(&cleared).unwrap();
    assert_eq!(payload, b"\x02\x00".to_vec());
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetDisplayObjective(cleared)
    );
}

#[test]
fn round_trips_the_clientbound_set_player_team_packet() {
    use crate::component::RichTextComponent;
    use crate::packets::{
        encode_play_clientbound_set_player_team, ChatFormatting, PlayerTeamMethod,
        PlayerTeamParameters, SetPlayerTeam, TeamCollisionRule, TeamVisibility,
    };

    let created = SetPlayerTeam {
        name: "red".into(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: RichTextComponent::plain("Red Team"),
            options: 0b11,
            nametag_visibility: TeamVisibility::HideForOtherTeams,
            collision_rule: TeamCollisionRule::PushOwnTeam,
            // `PlayerTeam`'s own default colour, and the one case that tells the
            // two numberings apart: its ordinal is 21 and its `id` is -1
            // (`ChatFormatting.java:39`), and `writeEnum` writes the ordinal.
            color: ChatFormatting::Reset,
            player_prefix: RichTextComponent::plain("[R]"),
            player_suffix: RichTextComponent::plain("!"),
        }),
        players: vec!["Steve".into(), "Alex".into()],
    };
    let (id, payload) = encode_play_clientbound_set_player_team(&created).unwrap();
    assert_eq!(id, crate::ids::play::CLIENTBOUND_SET_PLAYER_TEAM);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetPlayerTeam(created.clone())
    );

    // Join carries the player list without the parameters, and remove carries
    // neither (`ClientboundSetPlayerTeamPacket.java:92-98`).
    let joined = SetPlayerTeam {
        method: PlayerTeamMethod::Join.into(),
        parameters: None,
        players: vec!["Sam".into()],
        ..created.clone()
    };
    let (id, payload) = encode_play_clientbound_set_player_team(&joined).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetPlayerTeam(joined)
    );

    let removed = SetPlayerTeam {
        method: PlayerTeamMethod::Remove.into(),
        parameters: None,
        players: Vec::new(),
        ..created.clone()
    };
    let (id, payload) = encode_play_clientbound_set_player_team(&removed).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetPlayerTeam(removed)
    );

    // `ClientboundSetPlayerTeamPacket.write` throws for this state
    // (`ClientboundSetPlayerTeamPacket.java:83-85`).
    assert!(encode_play_clientbound_set_player_team(&SetPlayerTeam {
        parameters: None,
        ..created
    })
    .is_err());
}

#[test]
fn edit_book_java_utf8_decode_replaces_surrogates_and_bounds_title_units() {
    let mut wire = Encoder::new();
    wire.write_var_i32(40);
    wire.write_var_i32(1);
    wire.write_var_i32(3);
    wire.write_bytes(&[0xed, 0xa0, 0x80]);
    wire.write_bool(true);
    wire.write_var_i32(2);
    wire.write_bytes(&[0xed, 0xb0]);
    assert_eq!(
        decode_play_serverbound(ids::play::SERVERBOUND_EDIT_BOOK, &wire.into_inner()).unwrap(),
        PlayServerbound::EditBook(crate::packets::EditBook {
            slot: 40,
            pages: vec!["�".to_string()],
            title: Some("�".to_string())
        })
    );
    let mut wire = Encoder::new();
    wire.write_var_i32(0);
    wire.write_var_i32(0);
    wire.write_bool(true);
    wire.write_string(&"😀".repeat(17));
    assert!(decode_play_serverbound(ids::play::SERVERBOUND_EDIT_BOOK, &wire.into_inner()).is_err());
}

#[test]
fn chat_wire_java_decoding_replaces_malformed_bytes_and_keeps_packet_fields() {
    fn string(out: &mut Encoder, bytes: &[u8]) {
        out.write_var_i32(bytes.len() as i32);
        out.write_bytes(bytes);
    }
    for bytes in [&[0xed, 0xa0, 0x80][..], &[0xf0, 0x90, 0x80][..]] {
        let mut out = Encoder::new();
        string(&mut out, bytes);
        assert!(
            matches!(decode_play_serverbound(ids::play::SERVERBOUND_CHAT_COMMAND, &out.into_inner()).unwrap(),
            PlayServerbound::ChatCommand { command } if command == "�")
        );
        let mut out = Encoder::new();
        string(&mut out, bytes);
        out.write_i64(123);
        out.write_i64(456);
        out.write_bool(false);
        out.write_var_i32(3);
        out.write_bytes(&[5, 0, 0, 7]);
        let PlayServerbound::Chat(chat) =
            decode_play_serverbound(ids::play::SERVERBOUND_CHAT, &out.into_inner()).unwrap()
        else {
            panic!("expected chat")
        };
        assert_eq!(chat.message, "�");
        assert_eq!((chat.timestamp_millis, chat.salt), (123, 456));
        assert_eq!(
            (
                chat.last_seen_messages.offset,
                chat.last_seen_messages.acknowledged,
                chat.last_seen_messages.checksum
            ),
            (3, 5, 7)
        );
        let mut out = Encoder::new();
        out.write_var_i32(9);
        string(&mut out, bytes);
        assert!(
            matches!(decode_play_serverbound(ids::play::SERVERBOUND_COMMAND_SUGGESTION, &out.into_inner()).unwrap(),
            PlayServerbound::CommandSuggestion(request) if request.id == 9 && request.command == "�")
        );
    }
    let mut out = Encoder::new();
    string(&mut out, &[0xed, 0xa0, 0x80]);
    out.write_i64(123);
    out.write_i64(456);
    out.write_var_i32(1);
    string(&mut out, &[0xed, 0xa0, 0x80]);
    out.write_bytes(&[0x55; 256]);
    out.write_var_i32(3);
    out.write_bytes(&[5, 0, 0, 7]);
    let PlayServerbound::ChatCommandSigned(chat) = decode_play_serverbound(
        ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED,
        &out.into_inner(),
    )
    .unwrap() else {
        panic!("expected signed command")
    };
    assert_eq!(chat.command, "�");
    assert_eq!(chat.argument_signatures.entries[0].name, "�");
    assert_eq!(
        chat.argument_signatures.entries[0].signature.bytes,
        vec![0x55; 256]
    );
    assert_eq!(chat.last_seen_messages.checksum, 7);
}

#[test]
fn chat_wire_java_limits_distinguish_commands_messages_and_suggestions() {
    use crate::packets::{
        encode_play_chat_command, encode_play_chat_command_signed, ChatCommand, ChatCommandSigned,
    };
    let command = "😀".repeat(16383) + "x";
    for (id, payload) in [
        encode_play_chat_command(&ChatCommand {
            command: command.clone(),
        }),
        encode_play_chat_command_signed(&ChatCommandSigned::unsigned_arguments(
            command.clone(),
            1,
            2,
        )),
    ] {
        assert!(decode_play_serverbound(id, &payload).is_ok());
    }
    let too_long = "😀".repeat(16384);
    let mut out = Encoder::new();
    out.write_string(&too_long);
    assert!(
        decode_play_serverbound(ids::play::SERVERBOUND_CHAT_COMMAND, &out.into_inner()).is_err()
    );
    assert!(
        std::panic::catch_unwind(|| encode_play_chat_command(&ChatCommand {
            command: too_long.clone()
        }))
        .is_err()
    );
    assert!(std::panic::catch_unwind(|| encode_play_chat_command_signed(
        &ChatCommandSigned::unsigned_arguments(too_long, 1, 2)
    ))
    .is_err());
    let message = ChatMessage::unsigned("😀".repeat(128), 1, 2);
    let (id, payload) = encode_play_chat_message(&message);
    assert!(decode_play_serverbound(id, &payload).is_ok());
    assert!(
        std::panic::catch_unwind(|| encode_play_chat_message(&ChatMessage::unsigned(
            "😀".repeat(129),
            1,
            2
        )))
        .is_err()
    );
    let mut out = Encoder::new();
    out.write_var_i32(1);
    out.write_string(&("😀".repeat(16250) + "x"));
    assert!(
        decode_play_serverbound(ids::play::SERVERBOUND_COMMAND_SUGGESTION, &out.into_inner())
            .is_err()
    );
}

#[test]
fn encodes_the_clientbound_hurt_animation_as_the_client_decodes_it() {
    let update = crate::packets::HurtAnimation {
        id: 300,
        yaw: -42.5,
    };
    let (id, payload) = encode_play_clientbound_hurt_animation(update);
    assert_eq!(id, ids::play::CLIENTBOUND_HURT_ANIMATION);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::HurtAnimation(update)
    );
}

#[test]
fn encodes_the_clientbound_tag_query_as_the_client_decodes_it() {
    let tag = crate::NbtSummaryValue::Compound(vec![crate::NbtSummaryEntry {
        name: "Lock".into(),
        value: crate::NbtSummaryValue::String("key".into()),
    }]);
    for (transaction_id, tag) in [(12, Some(&tag)), (-3, None)] {
        let update = crate::packets::TagQuery::from_tag(transaction_id, tag).unwrap();
        assert_eq!(update.tag_present, tag.is_some());
        let (id, payload) = encode_play_clientbound_tag_query(&update).unwrap();
        assert_eq!(id, ids::play::CLIENTBOUND_TAG_QUERY);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::TagQuery(update.clone())
        );
        assert_eq!(
            crate::decode_nbt_summary(&update.raw_nbt).unwrap().as_ref(),
            tag
        );
    }
    // `writeNbt(null)` is the bare `EndTag` (`FriendlyByteBuf.java:526-529`).
    let (_, payload) =
        encode_play_clientbound_tag_query(&crate::packets::TagQuery::from_tag(7, None).unwrap())
            .unwrap();
    assert_eq!(payload, [7, 0]);
}

#[test]
fn a_tag_query_refuses_a_root_that_is_not_a_compound() {
    assert!(crate::packets::TagQuery::from_tag(1, Some(&crate::NbtSummaryValue::Int(3))).is_err());
    let mut update = crate::packets::TagQuery::from_tag(1, None).unwrap();
    update.tag_present = true;
    assert!(encode_play_clientbound_tag_query(&update).is_err());
}

#[test]
fn encodes_the_clientbound_game_rule_values_as_the_client_decodes_it() {
    let update = crate::packets::GameRuleValues {
        values: vec![
            crate::packets::GameRuleValue {
                rule: "minecraft:advance_time".to_string(),
                value: "false".to_string(),
            },
            crate::packets::GameRuleValue {
                rule: "minecraft:random_tick_speed".to_string(),
                value: "3".to_string(),
            },
        ],
    };
    let (id, payload) = encode_play_clientbound_game_rule_values(&update).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_GAME_RULE_VALUES);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::GameRuleValues(update)
    );
    let oversized = crate::packets::GameRuleValues {
        values: vec![crate::packets::GameRuleValue {
            rule: "minecraft:advance_time".to_string(),
            value: "x".repeat(32768),
        }],
    };
    assert!(encode_play_clientbound_game_rule_values(&oversized).is_err());
}

#[test]
fn unsigned_player_chat_and_disguised_chat_round_trip_through_the_client_decoder() {
    use crate::packets::{ChatTypeHolder, FilterMaskKind};

    let sender = uuid::Uuid::from_u128(0x1234);
    let bound = BoundChatType {
        chat_type_id: 4,
        name: RichTextComponent::plain("alice"),
        target_name: Some(RichTextComponent::plain("bob")),
    };
    let chat = UnsignedPlayerChat {
        global_index: 7,
        sender,
        content: "hello".into(),
        timestamp_millis: 1_700_000_000_123,
        unsigned_content: None,
        chat_type: bound.clone(),
    };
    let (id, payload) = encode_play_clientbound_player_chat(&chat).unwrap();
    let PlayClientbound::PlayerChat(decoded) = decode_play_clientbound(id, &payload).unwrap()
    else {
        panic!("player chat");
    };
    assert_eq!(decoded.global_index, 7);
    assert_eq!(decoded.sender, sender);
    assert_eq!(decoded.index, 0);
    assert_eq!(decoded.signature, None);
    assert_eq!(decoded.body.content, "hello");
    assert_eq!(decoded.body.timestamp_millis, 1_700_000_000_123);
    assert_eq!(decoded.body.salt, 0);
    assert!(decoded.body.last_seen.is_empty());
    assert_eq!(decoded.unsigned_content, None);
    assert_eq!(decoded.filter_mask.kind, FilterMaskKind::PassThrough);
    assert_eq!(
        decoded.chat_type.chat_type,
        ChatTypeHolder::Registry { id: 4 }
    );
    assert_eq!(decoded.chat_type.name, "alice");
    assert_eq!(decoded.chat_type.target_name.as_deref(), Some("bob"));

    let disguised = DisguisedChatMessage {
        message: RichTextComponent::plain("from the console"),
        chat_type: BoundChatType {
            target_name: None,
            ..bound
        },
    };
    let (id, payload) = encode_play_clientbound_disguised_chat(&disguised).unwrap();
    let PlayClientbound::DisguisedChat(decoded) = decode_play_clientbound(id, &payload).unwrap()
    else {
        panic!("disguised chat");
    };
    assert_eq!(decoded.message, "from the console");
    assert_eq!(
        decoded.chat_type.chat_type,
        ChatTypeHolder::Registry { id: 4 }
    );
    assert_eq!(decoded.chat_type.target_name, None);
}

#[test]
fn decodes_serverbound_chat_session_update_round_trip() {
    let update = crate::packets::ChatSessionUpdate {
        session_id: Uuid::from_u128(0x12345678_90ab_cdef_1020_304050607080),
        expires_at_epoch_millis: 1_717_986_918_300,
        public_key: vec![0x30, 0x82, 0x01, 0x22],
        key_signature: vec![0xab; 256],
    };
    let (id, payload) = crate::packets::encode_play_chat_session_update(&update);
    assert_eq!(
        decode_play_serverbound(id, &payload).unwrap(),
        PlayServerbound::ChatSessionUpdate(update)
    );
}

#[test]
fn every_catalog_play_serverbound_packet_id_has_a_real_decoder() {
    for entry in ids::play::SERVERBOUND_PACKETS {
        assert!(
            !matches!(
                decode_play_serverbound(entry.id, &[]),
                Err(ProtocolError::UnknownPacket { .. })
            ),
            "play serverbound packet {} (0x{:02x}) fell into UnknownPacket",
            entry.class_name,
            entry.id
        );
    }
}

#[test]
fn remaining_play_clientbound_26_1_encoders_round_trip_through_client_decoder() {
    use std::collections::BTreeMap;

    use crate::{
        attribute_types::VANILLA_ATTRIBUTE_SCALE_ID,
        equipment_slots::EquipmentSlot,
        item_types::VANILLA_ITEM_STONE_ID,
        packets::{
            encode_play_clientbound_cookie_request,
            encode_play_clientbound_custom_chat_completions,
            encode_play_clientbound_custom_report_details, encode_play_clientbound_delete_chat,
            encode_play_clientbound_game_test_highlight_pos,
            encode_play_clientbound_low_disk_space_warning,
            encode_play_clientbound_move_minecart_along_track,
            encode_play_clientbound_player_combat_end, encode_play_clientbound_player_combat_enter,
            encode_play_clientbound_resource_pack_pop, encode_play_clientbound_rotate_head,
            encode_play_clientbound_server_links, encode_play_clientbound_set_entity_link,
            encode_play_clientbound_set_equipment, encode_play_clientbound_set_passengers,
            encode_play_clientbound_start_configuration, encode_play_clientbound_store_cookie,
            encode_play_clientbound_test_instance_block_status,
            encode_play_clientbound_update_attributes, unpack_degrees, AttributeModifier,
            AttributeSnapshot, BlockPos, CookieRequest, CustomChatCompletions,
            CustomChatCompletionsAction, CustomReportDetails, DeleteChat, EquipmentSlotUpdate,
            GameTestHighlightPos, ItemStackSummary, MessageSignature, MinecartStep,
            MoveMinecartAlongTrack, PackedMessageSignature, PlayerCombatEnd, ResourcePackPop,
            RotateHead, ServerLinkEntry, ServerLinkKnownType, ServerLinkType, ServerLinks,
            SetEntityLink, SetEquipment, SetPassengers, StoreCookie, TestInstanceBlockStatus,
            UpdateAttributes, Vec3d, Vec3i,
        },
        RichTextComponent,
    };

    let (id, payload) = encode_play_clientbound_start_configuration();
    assert_eq!(id, ids::play::CLIENTBOUND_START_CONFIGURATION);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::StartConfiguration
    );

    let minecart = MoveMinecartAlongTrack {
        entity_id: 19,
        lerp_steps: vec![MinecartStep {
            position: Vec3d {
                x: 1.5,
                y: 64.0,
                z: -2.25,
            },
            movement: Vec3d {
                x: 0.25,
                y: 0.0,
                z: -0.5,
            },
            y_rot: unpack_degrees(32),
            x_rot: unpack_degrees(-16),
            weight: 0.75,
        }],
    };
    let (id, payload) = encode_play_clientbound_move_minecart_along_track(&minecart).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_MOVE_MINECART_ALONG_TRACK);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::MoveMinecartAlongTrack(minecart)
    );

    for delete_chat in [
        DeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: Some(7),
                full_signature: None,
            },
        },
        DeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: None,
                full_signature: Some(MessageSignature {
                    bytes: vec![0xcd; 256],
                }),
            },
        },
    ] {
        let (id, payload) = encode_play_clientbound_delete_chat(&delete_chat).unwrap();
        assert_eq!(id, ids::play::CLIENTBOUND_DELETE_CHAT);
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::DeleteChat(delete_chat)
        );
    }

    let completions = CustomChatCompletions {
        action: CustomChatCompletionsAction::Add,
        entries: vec!["@alice".to_string(), "@bob".to_string()],
    };
    let (id, payload) = encode_play_clientbound_custom_chat_completions(&completions).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::CustomChatCompletions(completions)
    );

    let combat_end = PlayerCombatEnd { duration: 120 };
    let (id, payload) = encode_play_clientbound_player_combat_end(combat_end);
    assert_eq!(id, ids::play::CLIENTBOUND_PLAYER_COMBAT_END);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::PlayerCombatEnd(combat_end)
    );

    let (id, payload) = encode_play_clientbound_player_combat_enter();
    assert_eq!(id, ids::play::CLIENTBOUND_PLAYER_COMBAT_ENTER);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::PlayerCombatEnter
    );

    let (id, payload) = encode_play_clientbound_low_disk_space_warning();
    assert_eq!(id, ids::play::CLIENTBOUND_LOW_DISK_SPACE_WARNING);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::LowDiskSpaceWarning
    );

    let highlight = GameTestHighlightPos {
        absolute_pos: BlockPos {
            x: 10,
            y: 65,
            z: -4,
        },
        relative_pos: BlockPos { x: 1, y: 2, z: 3 },
    };
    let (id, payload) = encode_play_clientbound_game_test_highlight_pos(highlight);
    assert_eq!(id, ids::play::CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::GameTestHighlightPos(highlight)
    );

    let test_status = TestInstanceBlockStatus {
        status: RichTextComponent::plain("passed"),
        size: Some(Vec3i { x: 5, y: 4, z: 6 }),
    };
    let (id, payload) = encode_play_clientbound_test_instance_block_status(&test_status).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::TestInstanceBlockStatus(test_status)
    );

    let cookie_req = CookieRequest {
        key: "bbb:play_cookie".to_string(),
    };
    let (id, payload) = encode_play_clientbound_cookie_request(&cookie_req).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_COOKIE_REQUEST);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::CookieRequest(cookie_req)
    );

    let store_cookie = StoreCookie {
        key: "bbb:play_cookie".to_string(),
        payload: vec![1, 2, 3, 4],
    };
    let (id, payload) = encode_play_clientbound_store_cookie(&store_cookie).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_STORE_COOKIE);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::StoreCookie(store_cookie)
    );

    let mut report_map = BTreeMap::new();
    report_map.insert("shard".to_string(), "main".to_string());
    let report = CustomReportDetails {
        details: report_map,
    };
    let (id, payload) = encode_play_clientbound_custom_report_details(&report).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_CUSTOM_REPORT_DETAILS);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::CustomReportDetails(report)
    );

    let links = ServerLinks {
        links: vec![
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
                url: "https://example.org/bugs".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Custom {
                    label: "Rules".to_string(),
                },
                url: "https://example.org/rules".to_string(),
            },
        ],
    };
    let (id, payload) = encode_play_clientbound_server_links(&links).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_SERVER_LINKS);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ServerLinks(links)
    );

    let pop = ResourcePackPop {
        id: Some(Uuid::from_u128(0x99)),
    };
    let (id, payload) = encode_play_clientbound_resource_pack_pop(&pop);
    assert_eq!(id, ids::play::CLIENTBOUND_RESOURCE_PACK_POP);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::ResourcePackPop(pop)
    );

    let head = RotateHead {
        id: 11,
        y_head_rot: unpack_degrees(64),
    };
    let (id, payload) = encode_play_clientbound_rotate_head(&head);
    assert_eq!(id, ids::play::CLIENTBOUND_ROTATE_HEAD);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::RotateHead(head)
    );

    let link = SetEntityLink {
        source_id: 21,
        dest_id: -1,
    };
    let (id, payload) = encode_play_clientbound_set_entity_link(&link);
    assert_eq!(id, ids::play::CLIENTBOUND_SET_ENTITY_LINK);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetEntityLink(link)
    );

    let passengers = SetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![31, 32],
    };
    let (id, payload) = encode_play_clientbound_set_passengers(&passengers).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_SET_PASSENGERS);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetPassengers(passengers)
    );

    let equipment = SetEquipment {
        entity_id: 44,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    count: 3,
                    item_id: Some(VANILLA_ITEM_STONE_ID),
                    component_patch: Default::default(),
                },
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: ItemStackSummary::empty(),
            },
        ],
    };
    let (id, payload) = encode_play_clientbound_set_equipment(&equipment).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_SET_EQUIPMENT);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::SetEquipment(equipment)
    );

    let attributes = UpdateAttributes {
        entity_id: 55,
        attributes: vec![AttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 1.25,
            modifiers: vec![AttributeModifier {
                id: "minecraft:effect.boost".to_string(),
                amount: 0.5,
                operation_id: 0,
            }],
        }],
    };
    let (id, payload) = encode_play_clientbound_update_attributes(&attributes).unwrap();
    assert_eq!(id, ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES);
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::UpdateAttributes(attributes)
    );
}
