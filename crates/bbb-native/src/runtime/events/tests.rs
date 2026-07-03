use super::*;
use crate::audio_runtime::resolver::{AudioCommandResolver, AudioResolveError};
use crate::particle_runtime::{
    LevelEventDripstoneDripParticle, LevelEventGrowthParticleContext, LevelEventGrowthParticleMode,
    LevelEventGrowthParticleSupport, LevelEventParticleContext, LevelParticleSpawnContext,
    ParticleEventSink,
};
use crate::runtime::{clear_color_for_day_time, clear_color_for_world};
use bbb_audio::{AudioCategory, AudioCommand};
use bbb_control::{AudioCounters, NetCounters};
use bbb_net::{NetCommand, NetEvent};
use bbb_pack::{JukeboxSongRegistry, SoundCatalog, SoundEventRegistry};
use bbb_protocol::codec::Encoder;
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::PlayClientbound;
use bbb_protocol::packets::{
    AddEntity, AdvancementCriterionProgressSummary, AdvancementProgressSummary, AdvancementSummary,
    AttributeSnapshot, AwardStats, BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate,
    ChatTypeBound, ChatTypeHolder, ChunkBiomeData, ChunkHeightmapData,
    ChunkPos as ProtocolChunkPos, ChunksBiomes, CommonPlayerSpawnInfo, ContainerClose,
    ContainerSetContent, ContainerSetData, ContainerSetSlot, CustomChatCompletions,
    CustomChatCompletionsAction, CustomPayload, CustomPayloadBody, DataComponentPatchSummary,
    DebugBlockValue, DebugChunkValue, DebugEntityValue, DebugEvent, DebugSample, DeleteChat,
    DialogHolder, DisguisedChat, EntityAnchor, EntityAnimation, EntityDataValue,
    EntityDataValueKind, EntityEvent, EntityMove, EntityPositionSync, EquipmentSlot,
    EquipmentSlotUpdate, Explosion, FilterMask, FilterMaskKind, ForgetLevelChunk, GameRuleValue,
    GameRuleValues, GameTestHighlightPos, HurtAnimation, IngredientSummary, InteractionHand,
    ItemCostSummary, ItemStackSummary, LevelChunkBlockEntity, LevelChunkData, LevelChunkWithLight,
    LevelEvent, LevelParticles, LightUpdate, LightUpdateData, MapColorPatch, MapDecoration,
    MapItemData, MerchantOffer, MerchantOffers, MessageSignature, MinecartStep, MountScreenOpen,
    MoveMinecartAlongTrack, OpenBook, OpenScreen, OpenSignEditor, PackedMessageSignature,
    ParticlePayload, PlaceGhostRecipe, PlayLogin, PlayerChat, PlayerCombatEnd, PlayerCombatKill,
    PlayerLookAt, PlayerLookAtTarget, PlayerPositionUpdate, PlayerRotationUpdate, PongResponse,
    ProjectilePower, RecipeBookAdd, RecipeBookAddEntry, RecipeBookRemove, RecipeBookSettings,
    RecipeBookTypeSettings, RecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary,
    RecipeDisplayType, RecipePropertySetSummary, RegistryData, RegistryDataEntry, RegistryTags,
    RemoteDebugSampleType, RemoveEntities, Respawn, RotateHead, SectionBlocksUpdate,
    SelectAdvancementsTab, ServerLinkEntry, ServerLinkKnownType, ServerLinkType, ServerLinks,
    SetChunkCacheCenter, SetChunkCacheRadius, SetCursorItem, SetEntityData, SetEntityLink,
    SetEntityMotion, SetEquipment, SetPassengers, SetPlayerInventory, ShowDialog,
    SignedMessageBody, SlotDisplaySummary, SoundEntityEvent, SoundEvent, SoundEventHolder,
    SoundSource, StatUpdate, StonecutterSelectableRecipeSummary, StopSound, TagNetworkPayload,
    TagQuery, TeleportEntity, TestInstanceBlockStatus, TrackedWaypoint, TrackedWaypointPacket,
    UpdateAdvancements, UpdateAttributes, UpdateRecipes, UpdateTags, Vec3d as ProtocolVec3d,
    Vec3i as ProtocolVec3i, WaypointData, WaypointIcon, WaypointIdentifier, WaypointOperation,
    WaypointVec3i, WrittenBookContentSummary,
};
use bbb_world::{
    advance_cobweb_place_particle_randoms, BlockPos, ChunkPos, LocalPlayerPoseState,
    RavagerRoarParticleState, RegistryPacketEntry, TakeItemEntityPickupParticleState,
    WorldBlockSoundProfile, WorldStore,
};
use std::collections::BTreeMap;
use tokio::sync::mpsc;
use uuid::Uuid;

fn advance_growth_randoms_for_context(
    event: &bbb_protocol::packets::LevelEvent,
    context: &LevelEventParticleContext,
    random: &mut LevelEventSoundRandomState,
) {
    if let Some(growth) = &context.growth_particles {
        let mode = match growth.mode {
            LevelEventGrowthParticleMode::InBlock { .. } => {
                bbb_world::LevelEventGrowthRandomMode::InBlock
            }
            LevelEventGrowthParticleMode::WideNoFloating { .. } => {
                bbb_world::LevelEventGrowthRandomMode::WideNoFloating
            }
        };
        advance_growth_level_event_particle_randoms(event.data, mode, random);
    }
}

#[test]
fn block_changed_ack_updates_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockChangedAck(
        bbb_protocol::packets::BlockChangedAck { sequence: 17 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(
        world.last_block_changed_ack(),
        Some(&bbb_world::BlockChangedAckState { sequence: 17 })
    );
    assert_eq!(world.counters().block_changed_ack_packets, 1);
}

#[test]
fn configuration_reentry_clears_online_client_level_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(55, 1));
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(counters.state.as_deref(), Some("Configuration"));
    assert_eq!(world.entity_count(), 0);
    assert_eq!(
        world.local_player(),
        &bbb_world::LocalPlayerState::default()
    );
}

#[test]
fn chunk_cache_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::SetChunkCacheCenter(
        SetChunkCacheCenter {
            chunk_x: -4,
            chunk_z: 7,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetChunkCacheRadius(
        SetChunkCacheRadius { radius: 10 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.chunk_cache_center(), Some(ChunkPos { x: -4, z: 7 }));
    assert_eq!(world.chunk_cache_radius(), Some(10));
    let world_counters = world.counters();
    assert_eq!(world_counters.chunk_cache_center_updates_received, 1);
    assert_eq!(world_counters.chunk_cache_radius_updates_received, 1);
}

#[test]
fn terrain_chunk_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(7);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_state_id: 5,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SectionBlocksUpdate(
        SectionBlocksUpdate {
            section_x: 1,
            section_y: 0,
            section_z: -2,
            updates: vec![BlockUpdate {
                pos: ProtocolBlockPos {
                    x: 17,
                    y: -64,
                    z: -31,
                },
                block_state_id: 6,
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_entity_type_id: 7,
            raw_nbt: vec![0],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LightUpdate(LightUpdate {
        chunk_x: 1,
        chunk_z: -2,
        light_data: empty_light_update_data(),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChunksBiomes(
        ChunksBiomes {
            chunks: vec![ChunkBiomeData {
                pos: ProtocolChunkPos { x: 1, z: -2 },
                raw_biomes: single_biome_payload(7),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ForgetLevelChunk(
        ForgetLevelChunk {
            pos: ProtocolChunkPos { x: 1, z: -2 },
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        7
    );
    assert_eq!(world.first_chunk(), Some(ChunkPos { x: 1, z: -2 }));
    assert!(world.probe_chunk(ChunkPos { x: 1, z: -2 }).is_none());

    let world_counters = world.counters();
    macro_rules! assert_chunk_counter {
        ($field:ident, $value:expr) => {
            assert_eq!(world_counters.$field, $value);
        };
    }

    assert_chunk_counter!(chunks_received, 1);
    assert_chunk_counter!(chunks_decoded, 1);
    assert_chunk_counter!(sections_decoded, 1);
    assert_chunk_counter!(block_entities_seen, 1);
    assert_chunk_counter!(light_arrays_seen, 0);
    assert_chunk_counter!(block_updates_received, 2);
    assert_chunk_counter!(block_updates_applied, 2);
    assert_chunk_counter!(block_updates_ignored, 0);
    assert_chunk_counter!(block_entity_updates_received, 1);
    assert_chunk_counter!(block_entity_updates_applied, 1);
    assert_chunk_counter!(block_entity_updates_ignored, 0);
    assert_chunk_counter!(light_updates_received, 1);
    assert_chunk_counter!(light_updates_applied, 1);
    assert_chunk_counter!(light_updates_ignored, 0);
    assert_chunk_counter!(biome_updates_received, 1);
    assert_chunk_counter!(biome_updates_applied, 1);
    assert_chunk_counter!(biome_updates_ignored, 0);
    assert_chunk_counter!(chunk_forgets_received, 1);
    assert_chunk_counter!(chunks_forgotten, 1);
    assert_chunk_counter!(chunk_forgets_ignored, 0);
}

#[test]
fn block_entity_data_sign_text_updates_world_through_event_dispatcher() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_entity_type_id: 7,
            raw_nbt: sign_text_nbt(
                ["Front A", "Front B", "Front C", "Front D"],
                ["Back A", "Back B", "Back C", "Back D"],
            ),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    let pos = BlockPos {
        x: 16,
        y: -64,
        z: -32,
    };
    assert_eq!(
        world.sign_text_lines(pos, true),
        Some(&[
            "Front A".to_string(),
            "Front B".to_string(),
            "Front C".to_string(),
            "Front D".to_string(),
        ])
    );
    assert_eq!(
        world.sign_text_lines(pos, false),
        Some(&[
            "Back A".to_string(),
            "Back B".to_string(),
            "Back C".to_string(),
            "Back D".to_string(),
        ])
    );
    assert_eq!(world.counters().block_entity_updates_applied, 1);
}

#[test]
fn terrain_chunk_ignored_counters_stay_in_world_store() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_state_id: 5,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SectionBlocksUpdate(
        SectionBlocksUpdate {
            section_x: 1,
            section_y: 0,
            section_z: -2,
            updates: vec![BlockUpdate {
                pos: ProtocolBlockPos {
                    x: 17,
                    y: -64,
                    z: -31,
                },
                block_state_id: 6,
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: ProtocolBlockPos {
                x: 16,
                y: -64,
                z: -32,
            },
            block_entity_type_id: 7,
            raw_nbt: vec![0],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LightUpdate(LightUpdate {
        chunk_x: 1,
        chunk_z: -2,
        light_data: empty_light_update_data(),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChunksBiomes(
        ChunksBiomes {
            chunks: vec![ChunkBiomeData {
                pos: ProtocolChunkPos { x: 1, z: -2 },
                raw_biomes: Vec::new(),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ForgetLevelChunk(
        ForgetLevelChunk {
            pos: ProtocolChunkPos { x: 1, z: -2 },
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        6
    );
    assert!(world.probe_chunk(ChunkPos { x: 1, z: -2 }).is_none());

    let world_counters = world.counters();
    macro_rules! assert_chunk_counter {
        ($field:ident, $value:expr) => {
            assert_eq!(world_counters.$field, $value);
        };
    }

    assert_chunk_counter!(block_updates_received, 2);
    assert_chunk_counter!(block_updates_applied, 0);
    assert_chunk_counter!(block_updates_ignored, 2);
    assert_chunk_counter!(block_entity_updates_received, 1);
    assert_chunk_counter!(block_entity_updates_applied, 0);
    assert_chunk_counter!(block_entity_updates_ignored, 1);
    assert_chunk_counter!(light_updates_received, 1);
    assert_chunk_counter!(light_updates_applied, 0);
    assert_chunk_counter!(light_updates_ignored, 1);
    assert_chunk_counter!(biome_updates_received, 1);
    assert_chunk_counter!(biome_updates_applied, 0);
    assert_chunk_counter!(biome_updates_ignored, 1);
    assert_chunk_counter!(world_apply_errors, 0);
    assert_chunk_counter!(chunk_forgets_received, 1);
    assert_chunk_counter!(chunks_forgotten, 0);
    assert_chunk_counter!(chunk_forgets_ignored, 1);
    assert!(world.apply_diagnostics().apply_errors.is_empty());
    assert!(counters.last_error.is_none());
}

#[test]
fn terrain_apply_errors_stay_in_world_store() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: ProtocolBlockPos { x: 0, y: 64, z: 0 },
            block_entity_type_id: 7,
            raw_nbt: vec![1],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert!(counters.last_error.is_none());
    assert_eq!(world.counters().world_apply_errors, 1);
    let diagnostics = world.apply_diagnostics();
    assert_eq!(diagnostics.apply_errors.len(), 1);
    assert_eq!(diagnostics.apply_errors[0].source, "block_entity_data");
    assert!(
        diagnostics.apply_errors[0].message.contains("nbt byte"),
        "{:?}",
        diagnostics.apply_errors
    );
}

#[test]
fn terrain_chunk_decode_errors_stay_in_world_store() {
    let (tx, mut rx) = mpsc::channel(1);
    let mut chunk = synthetic_native_level_chunk_packet();
    chunk.chunk_data.section_data = vec![0xff];
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(chunk)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert!(counters.last_error.is_none());
    assert_eq!(world.counters().chunks_received, 0);
    assert_eq!(world.counters().world_apply_errors, 1);
    let diagnostics = world.apply_diagnostics();
    assert_eq!(diagnostics.apply_errors.len(), 1);
    assert_eq!(diagnostics.apply_errors[0].source, "level_chunk_with_light");
}

#[test]
fn terrain_biome_decode_errors_stay_in_world_store() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChunksBiomes(
        ChunksBiomes {
            chunks: vec![ChunkBiomeData {
                pos: ProtocolChunkPos { x: 1, z: -2 },
                raw_biomes: vec![65],
            }],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    assert!(counters.last_error.is_none());
    assert_eq!(world.counters().chunks_received, 1);
    assert_eq!(world.counters().biome_updates_received, 1);
    assert_eq!(world.counters().world_apply_errors, 1);
    let diagnostics = world.apply_diagnostics();
    assert_eq!(diagnostics.apply_errors.len(), 1);
    assert_eq!(diagnostics.apply_errors[0].source, "chunks_biomes");
}

#[test]
fn respawn_clears_world_first_chunk_when_world_changes() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();

    let mut spawn_info = protocol_play_login(9).common_spawn_info;
    spawn_info.dimension_type_id = 1;
    spawn_info.dimension = "minecraft:the_nether".to_string();
    tx.try_send(NetEvent::Play(PlayClientbound::Respawn(Respawn {
        common_spawn_info: spawn_info,
        data_to_keep: 0,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(55));
    world.apply_update_mob_effect(protocol_update_mob_effect(55, 3));
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        })
    );
    world.apply_block_event(bbb_protocol::packets::BlockEvent {
        pos: ProtocolBlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        b0: 2,
        b1: 9,
        block_id: 54,
    });
    world.apply_level_event(bbb_protocol::packets::LevelEvent {
        event_type: 1001,
        pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
        data: 42,
        global: true,
    });
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.first_chunk(), None);
    assert_eq!(world.counters().respawns_received, 1);
    assert_eq!(world.counters().chunks_received, 1);
    assert_eq!(world.counters().chunks_decoded, 1);
    assert_eq!(world.counters().entities_tracked, 0);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
    assert_eq!(world.counters().block_destructions_tracked, 0);
    assert_eq!(world.counters().block_events_tracked, 0);
    assert_eq!(world.counters().level_events_tracked, 0);
}

#[test]
fn configuration_state_change_clears_client_level_state() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::StateChanged {
        state: bbb_net::ConnectionState::Configuration,
    })
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(55));
    assert!(
        world.apply_block_destruction(bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        })
    );
    world.apply_block_event(bbb_protocol::packets::BlockEvent {
        pos: ProtocolBlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        b0: 2,
        b1: 9,
        block_id: 54,
    });
    world.apply_level_event(bbb_protocol::packets::LevelEvent {
        event_type: 1001,
        pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
        data: 42,
        global: true,
    });
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    assert_eq!(counters.state.as_deref(), Some("Configuration"));
    assert_eq!(world.first_chunk(), None);
    assert_eq!(world.entity_count(), 0);
    assert_eq!(
        world.local_player(),
        &bbb_world::LocalPlayerState::default()
    );
    assert_eq!(world.counters().chunks_received, 1);
    assert_eq!(world.counters().chunks_decoded, 1);
    assert_eq!(world.counters().entities_tracked, 0);
    assert_eq!(world.counters().block_destructions_tracked, 0);
    assert_eq!(world.counters().block_events_tracked, 0);
    assert_eq!(world.counters().level_events_tracked, 0);
}

#[test]
fn start_configuration_flushes_pending_chat_acknowledgement_before_clearing_level() {
    let (tx, mut rx) = mpsc::channel(1);
    let (ack_tx, mut ack_rx) = tokio::sync::oneshot::channel();
    tx.try_send(NetEvent::StartConfiguration {
        pending_chat_acknowledgement: ack_tx,
    })
    .unwrap();

    let mut world = WorldStore::new();
    assert_eq!(
        world.apply_player_chat(player_chat_with_signature(
            0,
            MessageSignature {
                bytes: vec![7; 256],
            },
        )),
        None
    );
    world.apply_add_entity(protocol_add_entity(55));
    world.set_local_using_item(true);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(
        ack_rx.try_recv().unwrap(),
        Some(bbb_protocol::packets::ChatAcknowledgement { offset: 1 })
    );
    assert_eq!(world.entity_count(), 0);
    assert_eq!(
        world.local_player(),
        &bbb_world::LocalPlayerState::default()
    );
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
    assert_eq!(world.counters().player_chat_acknowledgement_packets, 1);
}

#[test]
fn transfer_event_updates_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Transfer(bbb_protocol::packets::Transfer {
        host: "next.example.com".to_string(),
        port: 25566,
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(
        world.last_transfer(),
        Some(&bbb_world::TransferTargetState {
            host: "next.example.com".to_string(),
            port: 25566,
        })
    );
    assert_eq!(world.counters().transfer_packets, 1);
}

#[test]
fn cookie_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::StoreCookie {
        key: "bbb:session".to_string(),
        payload_len: 3,
        stored_cookie_count: 1,
    })
    .unwrap();
    tx.try_send(NetEvent::CookieRequest {
        key: "bbb:session".to_string(),
        response_payload_present: true,
    })
    .unwrap();
    tx.try_send(NetEvent::CookieRequest {
        key: "bbb:missing".to_string(),
        response_payload_present: false,
    })
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.last_cookie_key(), Some("bbb:missing"));
    assert_eq!(world.stored_cookie_count(), 1);
    let world_counters = world.counters();
    assert_eq!(world_counters.store_cookie_packets, 1);
    assert_eq!(world_counters.stored_cookie_bytes, 3);
    assert_eq!(world_counters.cookie_request_packets, 2);
    assert_eq!(world_counters.cookie_response_hits, 1);
    assert_eq!(world_counters.cookie_response_misses, 1);
}

#[test]
fn custom_report_details_event_updates_world_counters() {
    let details = BTreeMap::from([
        ("Region".to_string(), "local".to_string()),
        ("Server".to_string(), "bbb test shard".to_string()),
    ]);
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::CustomReportDetails(
        bbb_protocol::packets::CustomReportDetails {
            details: details.clone(),
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(world.custom_report_details(), &details);
    assert_eq!(world.counters().custom_report_detail_packets, 1);
    assert_eq!(world.counters().custom_report_details_tracked, 2);
}

#[test]
fn award_stats_event_updates_world_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::AwardStats(AwardStats {
        stats: vec![
            StatUpdate {
                stat_type_id: 8,
                value_id: 10,
                amount: 3,
            },
            StatUpdate {
                stat_type_id: 0,
                value_id: 4,
                amount: 11,
            },
        ],
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.stat_value(8, 10), Some(3));
    assert_eq!(world.stat_value(0, 4), Some(11));
    assert_eq!(world.counters().award_stats_packets, 1);
    assert_eq!(world.counters().award_stats_entries_received, 2);
    assert_eq!(world.counters().last_award_stats_entry_count, 2);
    assert_eq!(world.counters().stats_tracked, 2);
    assert_eq!(
        world.last_stats_update(),
        Some(&bbb_world::StatsUpdateState {
            entries: vec![
                bbb_world::StatValueState {
                    stat_type_id: 8,
                    value_id: 10,
                    amount: 3,
                },
                bbb_world::StatValueState {
                    stat_type_id: 0,
                    value_id: 4,
                    amount: 11,
                },
            ],
        })
    );
}

#[test]
fn configuration_state_events_update_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::UpdateEnabledFeatures(
        bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec![
                "minecraft:minecart_improvements".to_string(),
                "minecraft:vanilla".to_string(),
            ],
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SelectKnownPacks {
        known_packs: vec![bbb_protocol::packets::KnownPack {
            namespace: "minecraft".to_string(),
            id: "core".to_string(),
            version: "26.1".to_string(),
        }],
        selected_packs: Vec::new(),
    })
    .unwrap();
    tx.try_send(NetEvent::ResetChat).unwrap();
    tx.try_send(NetEvent::CodeOfConduct {
        text: "Keep the server friendly.".to_string(),
    })
    .unwrap();

    let mut world = WorldStore::new();
    let _ = world.apply_player_chat(PlayerChat {
        global_index: 0,
        sender: Uuid::from_u128(1),
        index: 0,
        signature: Some(MessageSignature {
            bytes: vec![7; 256],
        }),
        body: SignedMessageBody {
            content: "previous".to_string(),
            timestamp_millis: 1,
            salt: 2,
            last_seen: Vec::new(),
        },
        unsigned_content: None,
        filter_mask: FilterMask {
            kind: FilterMaskKind::PassThrough,
            mask_words: Vec::new(),
        },
        chat_type: ChatTypeBound {
            chat_type: ChatTypeHolder::Registry { id: 0 },
            name: "Alice".to_string(),
            target_name: None,
        },
    });
    assert_eq!(world.counters().chat_messages_tracked, 1);
    assert_eq!(world.counters().chat_signature_cache_entries, 1);
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(
        world.enabled_feature_list(),
        vec![
            "minecraft:minecart_improvements".to_string(),
            "minecraft:vanilla".to_string(),
        ]
    );
    assert!(world.is_feature_enabled("minecraft:vanilla"));
    assert_eq!(world.counters().update_enabled_features_packets, 1);
    assert_eq!(world.counters().enabled_features_tracked, 2);
    assert_eq!(world.counters().enabled_features_ignored, 0);
    assert_eq!(world.known_packs().offered.len(), 1);
    assert_eq!(world.known_packs().offered[0].namespace, "minecraft");
    assert_eq!(world.known_packs().offered[0].id, "core");
    assert_eq!(world.known_packs().offered[0].version, "26.1");
    assert!(world.known_packs().selected.is_empty());
    assert_eq!(world.counters().select_known_packs_packets, 1);
    assert_eq!(world.counters().known_packs_offered, 1);
    assert_eq!(world.counters().known_packs_selected, 0);
    assert!(world.client_chat().messages.is_empty());
    assert!(world.client_chat().deleted_messages.is_empty());
    assert_eq!(world.client_chat().expected_player_chat_global_index, 0);
    assert_eq!(world.counters().reset_chat_packets, 1);
    assert_eq!(world.counters().chat_messages_tracked, 0);
    assert_eq!(world.counters().deleted_chat_messages_tracked, 0);
    assert_eq!(world.counters().chat_signature_cache_entries, 0);
    assert_eq!(
        world.last_code_of_conduct(),
        Some(&bbb_world::CodeOfConductState {
            text: "Keep the server friendly.".to_string(),
            text_hash: bbb_world::code_of_conduct_text_hash("Keep the server friendly."),
        })
    );
    assert_eq!(world.counters().code_of_conduct_packets, 1);
    assert_eq!(
        world.counters().last_code_of_conduct_len,
        "Keep the server friendly.".len()
    );
}

#[test]
fn registry_data_event_updates_world_state_and_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::RegistryData(RegistryData {
        registry: "minecraft:chat_type".to_string(),
        raw_payload_len: 96,
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:chat".to_string(),
                raw_data: Some(vec![10; 24]),
            },
            RegistryDataEntry {
                id: "minecraft:raw".to_string(),
                raw_data: None,
            },
        ],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    let packet = &world.registries().registries[0];
    assert_eq!(packet.name, "minecraft:chat_type");
    assert_eq!(packet.raw_payload_len, 96);
    assert_eq!(
        packet.entries,
        vec![
            RegistryPacketEntry::with_raw_data("minecraft:chat", vec![10; 24]),
            RegistryPacketEntry::stub("minecraft:raw"),
        ]
    );
    assert_eq!(world.counters().registries_seen, 1);
    assert_eq!(world.counters().registry_entries_seen, 2);
    assert_eq!(world.counters().registry_entries_with_data, 1);
    assert_eq!(world.counters().registry_entry_stubs, 1);
    assert_eq!(world.counters().registry_entry_payload_bytes, 24);
    assert_eq!(world.counters().registry_content_registries_tracked, 1);
    assert_eq!(world.counters().registry_content_packets_tracked, 1);
    assert_eq!(world.counters().registry_content_entries_tracked, 2);
    assert_eq!(world.counters().registry_duplicate_entries, 0);
    assert_eq!(world.counters().registry_duplicate_entry_ids_tracked, 0);
    assert_eq!(
        world.counters().last_registry_data_registry.as_deref(),
        Some("minecraft:chat_type")
    );
    assert_eq!(world.counters().last_registry_data_entry_count, 2);
}

#[test]
fn update_tags_event_updates_world_state_and_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::UpdateTags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![5, 6, 7],
            }],
        }],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(
        world.registry_tags("minecraft:item").unwrap().tags["minecraft:logs"],
        vec![5, 6, 7]
    );
    assert_eq!(world.counters().update_tags_packets, 1);
    assert_eq!(world.counters().last_update_tags_registry_count, 1);
    assert_eq!(world.counters().last_update_tags_total_tag_count, 1);
    assert_eq!(world.counters().last_update_tags_total_value_count, 3);
    assert_eq!(world.counters().tag_registries_tracked, 1);
    assert_eq!(world.counters().tags_tracked, 1);
    assert_eq!(world.counters().tag_entries_tracked, 3);
}

#[test]
fn server_links_event_updates_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::ServerLinks(ServerLinks {
        links: vec![
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                url: "https://example.invalid/support".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Custom {
                    label: "Rules".to_string(),
                },
                url: "http://example.invalid/rules".to_string(),
            },
            ServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                url: "ftp://example.invalid/file".to_string(),
            },
        ],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(
        world.server_links(),
        &[
            bbb_world::ServerLinkState {
                label: "known_server_link.support".to_string(),
                url: "https://example.invalid/support".to_string(),
                known_type: Some("support".to_string()),
            },
            bbb_world::ServerLinkState {
                label: "Rules".to_string(),
                url: "http://example.invalid/rules".to_string(),
                known_type: None,
            },
        ]
    );
    let world_counters = world.counters();
    assert_eq!(world_counters.server_link_packets, 1);
    assert_eq!(world_counters.server_link_invalid_entries, 1);
    assert_eq!(world_counters.server_links_tracked, 2);
}

#[test]
fn client_ui_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::LowDiskSpaceWarning))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::MountScreenOpen(
        MountScreenOpen {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::OpenBook(OpenBook {
        hand: InteractionHand::OffHand,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::OpenSignEditor(
        OpenSignEditor {
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PongResponse(
        PongResponse { time: 123456789 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(world.low_disk_space_warning_count(), 1);
    assert_eq!(
        world.last_mount_screen(),
        Some(&bbb_world::MountScreenState {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        })
    );
    assert_eq!(
        world.last_open_book(),
        Some(&bbb_world::OpenBookState {
            hand: "off_hand".to_string(),
        })
    );
    assert_eq!(
        world.last_open_sign_editor(),
        Some(&bbb_world::OpenSignEditorState {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            is_front_text: false,
        })
    );
    assert_eq!(
        world.last_pong_response(),
        Some(&bbb_world::PongResponseState { time: 123456789 })
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.low_disk_space_warnings, 1);
    assert_eq!(world_counters.mount_screen_open_packets, 1);
    assert_eq!(world_counters.open_book_packets, 1);
    assert_eq!(world_counters.open_sign_editor_packets, 1);
    assert_eq!(world_counters.pong_response_packets, 1);
}

#[test]
fn open_book_event_uses_held_written_book_for_active_screen() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::SetPlayerInventory(
        SetPlayerInventory {
            slot: 40,
            item: written_book_stack(vec!["Native first", "Native second"]),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::OpenBook(OpenBook {
        hand: InteractionHand::OffHand,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(
        world.last_open_book(),
        Some(&bbb_world::OpenBookState {
            hand: "off_hand".to_string(),
        })
    );
    assert_eq!(
        world.current_book(),
        Some(&bbb_world::BookScreenState {
            hand: "off_hand".to_string(),
            pages: vec!["Native first".to_string(), "Native second".to_string()],
            current_page: 0,
        })
    );
}

#[test]
fn map_item_data_event_updates_world_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::MapItemData(MapItemData {
        map_id: 42,
        scale: 2,
        locked: true,
        decorations: Some(vec![MapDecoration {
            type_id: 4,
            x: -20,
            y: 30,
            rot: 7,
            name: Some("Village".to_string()),
        }]),
        color_patch: Some(MapColorPatch {
            start_x: 3,
            start_y: 4,
            width: 2,
            height: 2,
            colors: vec![1, 2, 3, 4],
        }),
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    let map = world.map_item(42).expect("map state is tracked");
    assert_eq!(map.scale, 2);
    assert!(map.locked);
    assert_eq!(map.decorations.len(), 1);
    assert_eq!(map.decorations[0].name.as_deref(), Some("Village"));
    assert_eq!(map.colors[3 + 4 * 128], 1);
    assert_eq!(map.colors[4 + 5 * 128], 4);

    let world_counters = world.counters();
    assert_eq!(world_counters.map_item_data_packets, 1);
    assert_eq!(world_counters.maps_tracked, 1);
    assert_eq!(world_counters.map_decorations_tracked, 1);
    assert_eq!(world_counters.map_color_patches_applied, 1);
    assert_eq!(world_counters.map_color_patches_ignored, 0);
    assert_eq!(
        world.last_map_color_patch(),
        Some(&bbb_world::LastMapColorPatchState {
            map_id: 42,
            start_x: 3,
            start_y: 4,
            width: 2,
            height: 2,
        })
    );
}

#[test]
fn take_item_entity_event_updates_world_counter() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::TakeItemEntity(
        bbb_protocol::packets::TakeItemEntity {
            item_id: 10,
            player_id: 20,
            amount: 3,
        },
    )))
    .unwrap();
    drop(tx);

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(world.counters().take_item_entities_received, 1);
    assert_eq!(world.counters().take_item_entities_applied, 0);
    assert_eq!(world.counters().take_item_entities_ignored, 1);
    assert_eq!(world.counters().item_entity_stack_shrinks, 0);
    assert_eq!(world.counters().take_item_entities_removed, 0);
}

#[test]
fn take_item_entity_event_emits_pickup_sounds() {
    let (tx, mut rx) = mpsc::channel(7);
    let add_entity_at = |id: i32, entity_type_id: i32, position: ProtocolVec3d| {
        let mut entity = protocol_add_entity_with_type(id, entity_type_id);
        entity.position = position;
        PlayClientbound::AddEntity(entity)
    };
    for packet in [
        add_entity_at(
            10,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        ),
        add_entity_at(
            20,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            ProtocolVec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            },
        ),
        add_entity_at(
            30,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ProtocolVec3d {
                x: -6.0,
                y: 65.0,
                z: 9.0,
            },
        ),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 10,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 20,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 30,
            player_id: 99,
            amount: 1,
        }),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 404,
            player_id: 99,
            amount: 1,
        }),
    ] {
        tx.try_send(NetEvent::Play(packet)).unwrap();
    }
    drop(tx);

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_item_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 1.4 + 2.0;
    let expected_orb_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 0.35 + 0.9;
    let expected_zombie_pitch =
        (expected_random.next_float() - expected_random.next_float()) * 1.4 + 2.0;

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        7
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 3);
    let expected = [
        (
            "minecraft:entity.item.pickup",
            AudioCategory::Players,
            [1.0, 64.0, -2.0],
            0.2,
            expected_item_pitch,
        ),
        (
            "minecraft:entity.experience_orb.pickup",
            AudioCategory::Players,
            [4.0, 70.0, 8.0],
            0.1,
            expected_orb_pitch,
        ),
        (
            "minecraft:entity.item.pickup",
            AudioCategory::Players,
            [-6.0, 65.0, 9.0],
            0.2,
            expected_zombie_pitch,
        ),
    ];
    for (command, (event_id, category, position, volume, pitch)) in
        audio.commands.iter().zip(expected)
    {
        let AudioCommand::PlayPositionedSound(command) = command else {
            panic!("expected positioned sound, got {command:?}");
        };
        assert_eq!(command.sound.event_id, event_id);
        assert_eq!(command.category, category);
        assert_eq!(command.position, position);
        assert_close(command.packet_volume, volume);
        assert_close(command.packet_pitch, pitch);
        assert_eq!(command.seed, 0);
        assert!(!command.distance_delay);
    }
    assert_eq!(world.counters().take_item_entities_received, 4);
    assert_eq!(world.counters().take_item_entities_applied, 3);
    assert_eq!(world.counters().take_item_entities_ignored, 1);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:entity.item.pickup")
    );
}

#[test]
fn take_item_entity_event_emits_pickup_particle_state_before_removal() {
    let (tx, mut rx) = mpsc::channel(5);
    let mut item = protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_ITEM_ID);
    item.position = ProtocolVec3d {
        x: 1.0,
        y: 64.0,
        z: -2.0,
    };
    item.delta_movement = ProtocolVec3d {
        x: 0.1,
        y: 0.2,
        z: -0.3,
    };
    let mut target = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_PLAYER_ID);
    target.position = ProtocolVec3d {
        x: 4.0,
        y: 70.0,
        z: 8.0,
    };
    for packet in [
        PlayClientbound::AddEntity(item),
        PlayClientbound::SetEntityData(SetEntityData {
            id: 10,
            values: vec![EntityDataValue {
                data_id: 8,
                serializer_id: 7,
                value: EntityDataValueKind::ItemStack(item_stack(42, 5)),
            }],
        }),
        PlayClientbound::AddEntity(target),
        PlayClientbound::TakeItemEntity(bbb_protocol::packets::TakeItemEntity {
            item_id: 10,
            player_id: 20,
            amount: 5,
        }),
    ] {
        tx.try_send(NetEvent::Play(packet)).unwrap();
    }
    drop(tx);

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut random,
        ),
        4
    );

    assert_eq!(particles.take_item_entity_pickup_states.len(), 1);
    let state = &particles.take_item_entity_pickup_states[0];
    assert_eq!(state.item_entity_id, 10);
    assert_eq!(state.item_entity_type_id, VANILLA_ENTITY_TYPE_ITEM_ID);
    assert_eq!(state.item_position.x, 1.0);
    assert_eq!(state.item_position.y, 64.0);
    assert_eq!(state.item_position.z, -2.0);
    assert_eq!(state.item_delta_movement.x, 0.1);
    assert_eq!(state.item_delta_movement.y, 0.2);
    assert_eq!(state.item_delta_movement.z, -0.3);
    assert_eq!(state.target_entity_id, 20);
    assert_eq!(state.target_position.x, 4.0);
    assert_eq!(state.target_position.y, 70.0);
    assert_eq!(state.target_position.z, 8.0);
    assert_close(state.target_eye_height, 1.62);
    assert_eq!(state.item_stack, Some(item_stack(42, 5)));
    assert!(world.probe_entity(10).is_none());
    assert_eq!(world.counters().take_item_entities_removed, 1);
}

#[test]
fn clear_titles_event_updates_world_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::SetTitlesAnimation(
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetTitleText(
        bbb_protocol::packets::SetTitleText {
            content: "Quest complete".to_string(),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetSubtitleText(
        bbb_protocol::packets::SetSubtitleText {
            content: "Return to camp".to_string(),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ClearTitles(
        bbb_protocol::packets::ClearTitles { reset_times: false },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ClearTitles(
        bbb_protocol::packets::ClearTitles { reset_times: true },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(world.title().title.as_deref(), None);
    assert_eq!(world.title().subtitle.as_deref(), None);
    assert_eq!(world.title().fade_in, 10);
    assert_eq!(world.title().stay, 70);
    assert_eq!(world.title().fade_out, 20);
    assert_eq!(world.title().title_time, 0);
    let world_counters = world.counters();
    assert_eq!(world_counters.clear_titles_packets, 2);
    assert_eq!(world_counters.title_text_packets, 1);
    assert_eq!(world_counters.subtitle_text_packets, 1);
    assert_eq!(world_counters.titles_animation_packets, 1);
}

#[test]
fn command_suggestions_event_updates_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::CommandSuggestions(
        bbb_protocol::packets::CommandSuggestions {
            id: 7,
            start: 1,
            length: 4,
            suggestions: vec![
                bbb_protocol::packets::CommandSuggestion {
                    text: "give".to_string(),
                    tooltip: Some("Run give".to_string()),
                },
                bbb_protocol::packets::CommandSuggestion {
                    text: "gamemode".to_string(),
                    tooltip: None,
                },
            ],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(world.counters().command_suggestion_packets, 1);
    assert_eq!(world.counters().command_suggestion_entries_tracked, 2);

    let result = world.command_suggestions_by_id(7).unwrap();
    assert_eq!(result.start, 1);
    assert_eq!(result.length, 4);
    assert_eq!(result.suggestions.len(), 2);
    assert_eq!(result.suggestions[0].text, "give");
    assert_eq!(result.suggestions[0].tooltip.as_deref(), Some("Run give"));
    assert_eq!(world.last_command_suggestions(), Some(result));
}

#[test]
fn commands_event_updates_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::Commands(
        command_tree_packet("say"),
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(world.counters().command_tree_packets, 1);
    assert_eq!(world.counters().command_nodes_tracked, 3);
    assert_eq!(world.counters().command_literal_nodes_tracked, 1);
    assert_eq!(world.counters().command_argument_nodes_tracked, 1);
    assert_eq!(world.counters().command_redirect_nodes_tracked, 0);
    assert_eq!(world.counters().command_executable_nodes_tracked, 1);
    assert_eq!(world.counters().command_restricted_nodes_tracked, 1);

    let commands = world.commands();
    assert_eq!(commands.root_index, 0);
    assert_eq!(commands.nodes[1].name.as_deref(), Some("say"));
    assert_eq!(commands.nodes[2].name.as_deref(), Some("message"));
    assert_eq!(
        commands.nodes[2].parser.as_ref().unwrap().name,
        "brigadier:string"
    );
    assert_eq!(
        commands.nodes[2].suggestions.as_deref(),
        Some("minecraft:ask_server")
    );
}

#[test]
fn client_chat_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    let sender = Uuid::from_u128(0x1234);
    let signature = MessageSignature {
        bytes: vec![9; 256],
    };
    let expected_signature_checksum = signature.checksum();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerChat(PlayerChat {
        global_index: 0,
        sender,
        index: 2,
        signature: Some(signature),
        body: SignedMessageBody {
            content: "hello".to_string(),
            timestamp_millis: 1,
            salt: 2,
            last_seen: Vec::new(),
        },
        unsigned_content: Some("unsigned hello".to_string()),
        filter_mask: FilterMask {
            kind: FilterMaskKind::PartiallyFiltered,
            mask_words: vec![1],
        },
        chat_type: protocol_chat_type("Alice"),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DeleteChat(DeleteChat {
        message_signature: PackedMessageSignature {
            cache_id: Some(0),
            full_signature: None,
        },
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DisguisedChat(
        DisguisedChat {
            message: "server notice".to_string(),
            chat_type: protocol_chat_type("Server"),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.client_chat().messages.len(), 2);
    assert_eq!(world.client_chat().deleted_messages.len(), 1);
    let player_chat = &world.client_chat().messages[0];
    assert_eq!(player_chat.kind, bbb_world::ChatMessageKind::Player);
    assert_eq!(player_chat.content, "hello");
    assert_eq!(player_chat.sender, Some(sender));
    assert_eq!(player_chat.sender_name, "Alice");
    assert_eq!(player_chat.global_index, Some(0));
    assert_eq!(player_chat.message_index, Some(2));
    assert_eq!(player_chat.chat_type.registry_id, Some(0));
    assert_eq!(
        player_chat
            .signature
            .as_ref()
            .map(|signature| signature.checksum),
        Some(expected_signature_checksum)
    );
    assert_eq!(
        player_chat.unsigned_content.as_deref(),
        Some("unsigned hello")
    );
    assert_eq!(player_chat.filter_mask, "partially_filtered");
    assert_eq!(
        player_chat.validation_state,
        bbb_world::ChatValidationState::Unchecked
    );
    let disguised_chat = &world.client_chat().messages[1];
    assert_eq!(disguised_chat.kind, bbb_world::ChatMessageKind::Disguised);
    assert_eq!(disguised_chat.content, "server notice");
    let deleted_chat = &world.client_chat().deleted_messages[0];
    assert_eq!(
        deleted_chat
            .signature
            .as_ref()
            .map(|signature| signature.checksum),
        Some(expected_signature_checksum)
    );
    assert_eq!(deleted_chat.cache_id, Some(0));
    assert!(deleted_chat.resolved);
    let world_counters = world.counters();
    assert_eq!(world_counters.player_chat_packets, 1);
    assert_eq!(world_counters.disguised_chat_packets, 1);
    assert_eq!(world_counters.delete_chat_packets, 1);
    assert_eq!(world_counters.chat_messages_tracked, 2);
    assert_eq!(world_counters.deleted_chat_messages_tracked, 1);
    assert_eq!(world_counters.chat_signature_cache_entries, 1);
    assert_eq!(world_counters.player_chat_unsigned_content_packets, 1);
    assert_eq!(world_counters.player_chat_filtered_packets, 1);
}

#[test]
fn player_chat_events_queue_vanilla_threshold_acknowledgement() {
    let (tx, mut rx) = mpsc::channel(80);
    for index in 0..65 {
        tx.try_send(NetEvent::Play(PlayClientbound::PlayerChat(
            player_chat_with_signature(
                index,
                MessageSignature {
                    bytes: vec![index as u8; 256],
                },
            ),
        )))
        .unwrap();
    }

    let (command_tx, mut command_rx) = mpsc::channel(2);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &Some(command_tx)),
        65
    );
    assert_eq!(world.counters().player_chat_packets, 65);
    assert_eq!(world.counters().player_chat_acknowledgement_packets, 1);
    assert_eq!(
        world.counters().player_chat_acknowledgement_pending_offset,
        0
    );
    assert_eq!(counters.chat_acknowledgement_commands_queued, 1);
    assert_eq!(
        command_rx.try_recv().unwrap(),
        NetCommand::ChatAcknowledgement(bbb_protocol::packets::ChatAcknowledgement { offset: 65 })
    );
    assert!(command_rx.try_recv().is_err());
}

#[test]
fn client_feature_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::CustomChatCompletions(
        CustomChatCompletions {
            action: CustomChatCompletionsAction::Set,
            entries: vec!["/warp".to_string(), "/spawn".to_string()],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlaceGhostRecipe(
        PlaceGhostRecipe {
            container_id: 9,
            recipe_display_type: RecipeDisplayType::Stonecutter,
            recipe_display_body: vec![1, 2, 3],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateAdvancements(
        UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: None,
                requirements: Vec::new(),
                sends_telemetry_event: false,
            }],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SelectAdvancementsTab(
        SelectAdvancementsTab {
            tab: Some("minecraft:story/root".to_string()),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TagQuery(TagQuery {
        transaction_id: 12,
        tag_present: true,
        raw_nbt: vec![10, 0],
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(
        world.last_custom_chat_completion_update(),
        Some(&bbb_world::CustomChatCompletionUpdateState {
            action: "set".to_string(),
            entries: 2,
        })
    );
    assert!(world.custom_chat_completions().contains("/warp"));
    assert!(world.custom_chat_completions().contains("/spawn"));
    assert_eq!(world.counters().custom_chat_completion_packets, 1);
    assert_eq!(world.counters().custom_chat_completions_tracked, 2);
    assert_eq!(
        world.last_ghost_recipe(),
        Some(&bbb_world::GhostRecipeState {
            container_id: 9,
            recipe_display_type_id: 3,
            recipe_display_type: "stonecutter".to_string(),
            recipe_display_body_len: 3,
        })
    );
    assert_eq!(world.counters().ghost_recipe_packets, 1);
    assert_eq!(
        world.selected_advancements_tab(),
        Some("minecraft:story/root")
    );
    assert_eq!(world.counters().select_advancements_tab_packets, 1);
    assert_eq!(
        world.last_tag_query(),
        Some(&bbb_world::TagQueryResponseState {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: vec![10, 0],
        })
    );
    assert_eq!(world.counters().tag_query_packets, 1);
}

#[test]
fn inventory_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::OpenScreen(OpenScreen {
        container_id: 7,
        menu_type_id: 18,
        title: "Inventory".to_string(),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerSetContent(
        ContainerSetContent {
            container_id: 7,
            state_id: 1,
            items: vec![item_stack(42, 1), item_stack(43, 2)],
            carried_item: item_stack(99, 1),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerSetSlot(
        ContainerSetSlot {
            container_id: 7,
            state_id: 2,
            slot: 1,
            item: item_stack(44, 3),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerSetData(
        ContainerSetData {
            container_id: 7,
            id: 3,
            value: 11,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetPlayerInventory(
        SetPlayerInventory {
            slot: 5,
            item: item_stack(12, 1),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetCursorItem(
        SetCursorItem {
            item: item_stack(100, 4),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerClose(
        ContainerClose { container_id: 7 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ContainerClose(
        ContainerClose { container_id: 99 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        8
    );

    assert!(world.inventory().open_container.is_none());
    assert_eq!(world.inventory().cursor_item, item_stack(100, 4));
    assert_eq!(world.inventory().player_slots.len(), 1);
    assert_eq!(world.inventory().player_slots[0].slot, 5);
    assert_eq!(world.inventory().player_slots[0].item, item_stack(12, 1));

    let world_counters = world.counters();
    assert_eq!(world_counters.container_open_updates_received, 1);
    assert_eq!(world_counters.container_content_updates_received, 1);
    assert_eq!(world_counters.container_slot_updates_received, 1);
    assert_eq!(world_counters.container_data_updates_received, 1);
    assert_eq!(world_counters.container_close_updates_received, 2);
    assert_eq!(world_counters.container_close_updates_applied, 1);
    assert_eq!(world_counters.container_close_updates_ignored, 1);
    assert_eq!(world_counters.inventory_slot_updates_received, 1);
    assert_eq!(world_counters.inventory_slots_tracked, 1);
    assert_eq!(world_counters.cursor_item_updates_received, 1);
}

#[test]
fn entity_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(19);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity(123),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity(456),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityPositionSync(
        EntityPositionSync {
            id: 123,
            position: ProtocolVec3d {
                x: 2.0,
                y: 65.0,
                z: -3.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.25,
                z: 0.0,
            },
            y_rot: 180.0,
            x_rot: 30.0,
            on_ground: true,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::MoveEntity(EntityMove {
        id: 123,
        delta_x: 4096,
        delta_y: 0,
        delta_z: -2048,
        y_rot: Some(-90.0),
        x_rot: Some(45.0),
        on_ground: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TeleportEntity(
        TeleportEntity {
            id: 123,
            position: ProtocolVec3d {
                x: 0.5,
                y: 70.0,
                z: -4.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.2,
                z: 0.0,
            },
            y_rot: 10.0,
            x_rot: -120.0,
            relatives_mask: 0,
            on_ground: true,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityPositionSync(
        EntityPositionSync {
            id: 999,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::MoveEntity(EntityMove {
        id: 999,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TeleportEntity(
        TeleportEntity {
            id: 999,
            position: ProtocolVec3d::default(),
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
            on_ground: false,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityMotion(
        SetEntityMotion {
            id: 123,
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: 0.0,
                z: -0.1,
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RotateHead(RotateHead {
        id: 123,
        y_head_rot: 90.0,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 123, action: 3 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 123,
        event_id: 35,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::HurtAnimation(
        HurtAnimation { id: 123, yaw: 45.5 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 123,
            values: vec![EntityDataValue {
                data_id: 0,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(0x20),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEquipment(
        SetEquipment {
            entity_id: 123,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: item_stack(42, 1),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateAttributes(
        UpdateAttributes {
            entity_id: 123,
            attributes: vec![AttributeSnapshot {
                attribute_id: 21,
                base: 20.0,
                modifiers: Vec::new(),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityLink(
        SetEntityLink {
            source_id: 123,
            dest_id: 456,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetPassengers(
        SetPassengers {
            vehicle_id: 123,
            passenger_ids: vec![456],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RemoveEntities(
        RemoveEntities {
            entity_ids: vec![456, 999],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        19
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entities_received, 2);
    assert_eq!(world_counters.entities_tracked, 1);
    assert_eq!(world_counters.entity_position_syncs_received, 2);
    assert_eq!(world_counters.entity_position_syncs_applied, 1);
    assert_eq!(world_counters.entity_position_syncs_ignored, 1);
    assert_eq!(world_counters.entity_moves_received, 2);
    assert_eq!(world_counters.entity_moves_applied, 1);
    assert_eq!(world_counters.entity_moves_ignored, 1);
    assert_eq!(world_counters.entity_teleports_received, 2);
    assert_eq!(world_counters.entity_teleports_applied, 1);
    assert_eq!(world_counters.entity_teleports_ignored, 1);
    assert_eq!(world_counters.entity_motion_updates_received, 1);
    assert_eq!(world_counters.entity_motion_updates_applied, 1);
    assert_eq!(world_counters.entity_head_rotations_received, 1);
    assert_eq!(world_counters.entity_head_rotations_applied, 1);
    assert_eq!(world_counters.entity_animation_updates_received, 1);
    assert_eq!(world_counters.entity_animation_updates_applied, 1);
    assert_eq!(world_counters.entity_events_received, 1);
    assert_eq!(world_counters.entity_events_applied, 1);
    assert_eq!(world_counters.entity_hurt_animations_received, 1);
    assert_eq!(world_counters.entity_hurt_animations_applied, 1);
    assert_eq!(world_counters.entity_data_updates_received, 1);
    assert_eq!(world_counters.entity_data_values_received, 1);
    assert_eq!(world_counters.entity_data_updates_applied, 1);
    assert_eq!(world_counters.entity_equipment_updates_received, 1);
    assert_eq!(world_counters.entity_equipment_slots_received, 1);
    assert_eq!(world_counters.entity_equipment_updates_applied, 1);
    assert_eq!(world_counters.entity_attribute_updates_received, 1);
    assert_eq!(world_counters.entity_attributes_received, 1);
    assert_eq!(world_counters.entity_attribute_updates_applied, 1);
    assert_eq!(world_counters.entity_link_updates_received, 1);
    assert_eq!(world_counters.entity_link_updates_applied, 1);
    assert_eq!(world_counters.entity_passenger_updates_received, 1);
    assert_eq!(world_counters.entity_passenger_ids_received, 1);
    assert_eq!(world_counters.entity_passenger_updates_applied, 1);
    assert_eq!(world_counters.entity_removes_received, 2);
    assert_eq!(world_counters.entities_removed, 1);
    assert_eq!(world_counters.entity_removes_ignored, 1);
}

#[test]
fn firework_entity_event_with_empty_explosions_emits_poof_particles() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(100, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        firework_item_entity_data(100, Some(0)),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 100,
        event_id: 17,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(200, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        firework_item_entity_data(200, Some(1)),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 200,
        event_id: 17,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(300, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 300,
        event_id: 17,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        8
    );

    assert_eq!(
        particles.firework_empty_explosion_positions,
        vec![[1.0, 64.0, -2.0]]
    );
    assert_eq!(
        particles.firework_empty_explosion_camera_positions,
        vec![None]
    );
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 3);
}

#[test]
fn totem_entity_event_emits_tracking_emitter_particles() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(123, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 123,
        event_id: 35,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 35,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        3
    );

    assert_eq!(particles.tracking_emitter_states.len(), 1);
    let state = particles.tracking_emitter_states[0];
    assert_eq!(
        state.particle_type_id,
        crate::particle_runtime::TOTEM_OF_UNDYING_PARTICLE_TYPE_ID
    );
    assert_eq!(state.position, [1.0, 64.0, -2.0]);
    assert_close(state.width, 0.6);
    assert_close(state.height, 1.95);
    assert_eq!(state.lifetime_ticks, 30);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Hostile);
            assert_eq!(command.position, [1.0, 64.0, -2.0]);
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.packet_pitch, 1.0);
            assert_eq!(command.seed, 0);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:item.totem.use");
            assert_eq!(command.sound.sound_name, "minecraft:item/totem/use");
        }
        other => panic!("expected totem positioned sound command, got {other:?}"),
    }
    assert_eq!(world.counters().entity_events_applied, 1);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn ravager_roar_entity_event_emits_poof_particle_state() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(77, VANILLA_ENTITY_TYPE_RAVAGER_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 77,
        event_id: 69,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(78, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 78,
        event_id: 69,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 69,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert_eq!(particles.ravager_roar_states.len(), 1);
    let state = particles.ravager_roar_states[0];
    assert_eq!(state.entity_id, 77);
    assert_eq!(state.center.x, 1.0);
    assert_close64(state.center.y, 65.1);
    assert_eq!(state.center.z, -2.0);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().entity_events_applied, 2);
    assert_eq!(world.counters().entity_events_ignored, 1);
}

#[test]
fn entity_animation_emits_crit_tracking_emitter_particles() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(123, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 123, action: 4 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 123, action: 5 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 999, action: 4 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    assert_eq!(particles.tracking_emitter_states.len(), 2);
    assert_eq!(
        particles.tracking_emitter_states[0].particle_type_id,
        crate::particle_runtime::CRIT_PARTICLE_TYPE_ID
    );
    assert_eq!(
        particles.tracking_emitter_states[1].particle_type_id,
        crate::particle_runtime::ENCHANTED_HIT_PARTICLE_TYPE_ID
    );
    for state in &particles.tracking_emitter_states {
        assert_eq!(state.position, [1.0, 64.0, -2.0]);
        assert_close(state.width, 0.6);
        assert_close(state.height, 1.95);
        assert_eq!(state.lifetime_ticks, 3);
    }
    assert_eq!(world.counters().entity_animation_updates_applied, 2);
    assert_eq!(world.counters().entity_animation_updates_ignored, 1);
}

#[test]
fn entity_events_materialize_ender_dragon_part_pick_targets() {
    const ENDER_DRAGON_TYPE_ID: i32 = 43;

    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(100, ENDER_DRAGON_TYPE_ID),
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    world.advance_entity_client_animations(1);

    assert_eq!(world.probe_entity_pick_bounds(100), None);
    let targets = world.entity_pick_targets();
    assert_eq!(
        targets
            .iter()
            .map(|target| target.entity_id)
            .collect::<Vec<_>>(),
        vec![101, 102, 103, 104, 105, 106, 107, 108]
    );
    assert_eq!(
        targets[0].bounds,
        bbb_world::EntityPickBoundsState::from_base_size(1.0, 1.0, 0.0)
    );
    assert_eq!(
        targets[2].bounds,
        bbb_world::EntityPickBoundsState::from_base_size(5.0, 3.0, 0.0)
    );
    assert_eq!(
        targets[6].bounds,
        bbb_world::EntityPickBoundsState::from_base_size(4.0, 2.0, 0.0)
    );
    assert_eq!(world.counters().entities_received, 1);
    assert_eq!(world.counters().entities_tracked, 1);
}

#[test]
fn transient_entity_event_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::EntityAnimation(
        EntityAnimation { id: 999, action: 4 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 21,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::HurtAnimation(
        HurtAnimation { id: 999, yaw: 90.0 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entity_animation_updates_received, 1);
    assert_eq!(world_counters.entity_animation_updates_applied, 0);
    assert_eq!(world_counters.entity_animation_updates_ignored, 1);
    assert_eq!(world_counters.entity_events_received, 1);
    assert_eq!(world_counters.entity_events_applied, 0);
    assert_eq!(world_counters.entity_events_ignored, 1);
    assert_eq!(world_counters.entity_hurt_animations_received, 1);
    assert_eq!(world_counters.entity_hurt_animations_applied, 0);
    assert_eq!(world_counters.entity_hurt_animations_ignored, 1);
}

#[test]
fn simple_entity_update_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityMotion(
        SetEntityMotion {
            id: 999,
            delta_movement: ProtocolVec3d::default(),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RotateHead(RotateHead {
        id: 999,
        y_head_rot: 90.0,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityLink(
        SetEntityLink {
            source_id: 999,
            dest_id: 123,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entity_motion_updates_received, 1);
    assert_eq!(world_counters.entity_motion_updates_applied, 0);
    assert_eq!(world_counters.entity_motion_updates_ignored, 1);
    assert_eq!(world_counters.entity_head_rotations_received, 1);
    assert_eq!(world_counters.entity_head_rotations_applied, 0);
    assert_eq!(world_counters.entity_head_rotations_ignored, 1);
    assert_eq!(world_counters.entity_link_updates_received, 1);
    assert_eq!(world_counters.entity_link_updates_applied, 0);
    assert_eq!(world_counters.entity_link_updates_ignored, 1);
}

#[test]
fn entity_metadata_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(124, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEquipment(
        SetEquipment {
            entity_id: 124,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: item_stack(42, 1),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateAttributes(
        UpdateAttributes {
            entity_id: 124,
            attributes: vec![AttributeSnapshot {
                attribute_id: 21,
                base: 20.0,
                modifiers: Vec::new(),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 999,
            values: vec![EntityDataValue {
                data_id: 0,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(0x20),
            }],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entity_data_updates_received, 1);
    assert_eq!(world_counters.entity_data_values_received, 1);
    assert_eq!(world_counters.entity_data_updates_applied, 0);
    assert_eq!(world_counters.entity_data_updates_ignored, 1);
    assert_eq!(world_counters.entity_equipment_updates_received, 1);
    assert_eq!(world_counters.entity_equipment_slots_received, 1);
    assert_eq!(world_counters.entity_equipment_updates_applied, 0);
    assert_eq!(world_counters.entity_equipment_updates_ignored, 1);
    assert_eq!(world_counters.entity_attribute_updates_received, 1);
    assert_eq!(world_counters.entity_attributes_received, 1);
    assert_eq!(world_counters.entity_attribute_updates_applied, 0);
    assert_eq!(world_counters.entity_attribute_updates_ignored, 1);
}

#[test]
fn passenger_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::SetPassengers(
        SetPassengers {
            vehicle_id: 999,
            passenger_ids: vec![123, 124],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.entity_passenger_updates_received, 1);
    assert_eq!(world_counters.entity_passenger_ids_received, 2);
    assert_eq!(world_counters.entity_passenger_updates_applied, 0);
    assert_eq!(world_counters.entity_passenger_updates_ignored, 1);
}

#[test]
fn remove_entities_updates_world_active_effect_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::RemoveEntities(
        RemoveEntities {
            entity_ids: vec![entity_id],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    world.apply_update_mob_effect(protocol_update_mob_effect(entity_id, 3));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.counters().entities_tracked, 0);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
}

#[test]
fn add_entity_replacement_updates_world_active_effect_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity(entity_id),
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    world.apply_update_mob_effect(protocol_update_mob_effect(entity_id, 3));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.counters().entities_tracked, 1);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
}

#[test]
fn mob_effect_ignored_counters_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        protocol_add_entity_with_type(124, VANILLA_ENTITY_TYPE_ITEM_ID),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateMobEffect(
        protocol_update_mob_effect(124, 3),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RemoveMobEffect(
        bbb_protocol::packets::RemoveMobEffect {
            entity_id: 124,
            effect_id: 3,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.update_mob_effect_packets, 1);
    assert_eq!(world_counters.update_mob_effects_ignored, 1);
    assert_eq!(world_counters.remove_mob_effect_packets, 1);
    assert_eq!(world_counters.remove_mob_effects_ignored, 1);
    assert_eq!(world_counters.active_mob_effects_tracked, 0);
}

#[test]
fn merchant_offers_event_updates_world_inventory_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::OpenScreen(OpenScreen {
        container_id: 7,
        menu_type_id: 19,
        title: "Merchant".to_string(),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::MerchantOffers(
        MerchantOffers {
            container_id: 7,
            offers: vec![MerchantOffer {
                buy_a: item_cost(42, 3),
                sell: item_stack(99, 1),
                buy_b: Some(item_cost(43, 2)),
                is_out_of_stock: true,
                uses: 4,
                max_uses: 12,
                xp: 8,
                special_price_diff: -2,
                price_multiplier: 0.05,
                demand: 6,
            }],
            villager_level: 3,
            villager_xp: 120,
            show_progress: true,
            can_restock: false,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    let container = world.inventory().open_container.as_ref().unwrap();
    let offers = container.merchant_offers.as_ref().unwrap();
    assert_eq!(offers.container_id, 7);
    assert_eq!(offers.offers.len(), 1);
    assert_eq!(offers.offers[0].buy_a, item_cost(42, 3));
    assert_eq!(offers.offers[0].sell, item_stack(99, 1));
    assert_eq!(offers.villager_level, 3);
    assert_eq!(offers.villager_xp, 120);
    assert!(offers.show_progress);
    assert!(!offers.can_restock);

    let world_counters = world.counters();
    assert_eq!(world_counters.container_open_updates_received, 1);
    assert_eq!(world_counters.merchant_offer_packets_received, 1);
    assert_eq!(world_counters.merchant_offer_packets_applied, 1);
    assert_eq!(world_counters.merchant_offer_packets_ignored, 0);
    assert_eq!(world_counters.merchant_offers_tracked, 1);
}

#[test]
fn recipe_book_events_update_world_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::RecipeBookAdd(
        RecipeBookAdd {
            replace: true,
            entries: vec![
                recipe_book_entry(7, true, true),
                recipe_book_entry(8, false, false),
            ],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RecipeBookRemove(
        RecipeBookRemove {
            recipe_ids: vec![RecipeDisplayId { index: 8 }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RecipeBookSettings(
        RecipeBookSettings {
            crafting: RecipeBookTypeSettings {
                open: true,
                filtering: false,
            },
            furnace: RecipeBookTypeSettings {
                open: false,
                filtering: true,
            },
            blast_furnace: RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
            smoker: RecipeBookTypeSettings {
                open: false,
                filtering: false,
            },
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert!(world.recipe_book().known.contains_key(&7));
    assert!(!world.recipe_book().known.contains_key(&8));
    assert!(world.recipe_book().highlights.contains(&7));
    assert_eq!(world.recipe_book().notification_ids, vec![7]);
    assert!(world.recipe_book().settings.crafting.open);
    assert!(world.recipe_book().settings.furnace.filtering);

    let world_counters = world.counters();
    assert_eq!(world_counters.recipe_book_add_packets, 1);
    assert_eq!(world_counters.recipe_book_remove_packets, 1);
    assert_eq!(world_counters.recipe_book_settings_packets, 1);
    assert_eq!(world_counters.recipe_book_replace_packets, 1);
    assert_eq!(world_counters.recipe_book_entries_received, 2);
    assert_eq!(world_counters.recipe_book_removed_entries_received, 1);
    assert_eq!(world_counters.recipe_book_entries_tracked, 1);
    assert_eq!(world_counters.recipe_book_highlights_tracked, 1);
    assert_eq!(world_counters.recipe_book_notifications_received, 1);
}

#[test]
fn update_advancements_event_updates_world_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateAdvancements(
        UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "minecraft:story/root".to_string(),
                parent: None,
                display: None,
                requirements: vec![vec!["mine_stone".to_string(), "get_log".to_string()]],
                sends_telemetry_event: true,
            }],
            removed: Vec::new(),
            progress: vec![AdvancementProgressSummary {
                id: "minecraft:story/root".to_string(),
                criteria: vec![AdvancementCriterionProgressSummary {
                    name: "mine_stone".to_string(),
                    obtained_epoch_millis: Some(1_700_000_000_000),
                }],
            }],
            show_advancements: true,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert!(world
        .advancements()
        .advancements
        .contains_key("minecraft:story/root"));
    let progress = world
        .advancements()
        .progress
        .get("minecraft:story/root")
        .unwrap();
    assert_eq!(progress.criteria.len(), 2);

    let world_counters = world.counters();
    assert_eq!(world_counters.update_advancements_packets, 1);
    assert_eq!(world_counters.update_advancements_reset_packets, 1);
    assert_eq!(world_counters.update_advancements_show_packets, 1);
    assert_eq!(world_counters.advancements_added_received, 1);
    assert_eq!(world_counters.advancements_removed_received, 0);
    assert_eq!(world_counters.advancements_adds_ignored, 0);
    assert_eq!(world_counters.advancement_progress_received, 1);
    assert_eq!(world_counters.advancement_progress_updates_ignored, 0);
    assert_eq!(world_counters.advancements_tracked, 1);
    assert_eq!(world_counters.advancement_roots_tracked, 1);
    assert_eq!(world_counters.advancement_progress_tracked, 1);
    assert_eq!(world_counters.advancement_progress_criteria_tracked, 2);
}

#[test]
fn update_recipes_event_replaces_world_recipe_access_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateRecipes(
        UpdateRecipes {
            property_sets: vec![RecipePropertySetSummary {
                key: "minecraft:furnace_input".to_string(),
                item_ids: vec![42, 43],
            }],
            stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                input: IngredientSummary {
                    tag: None,
                    item_ids: vec![11, 12],
                },
                option_display: SlotDisplaySummary {
                    display_type_id: 4,
                    raw_payload: vec![4, 77],
                    item_stack: None,
                },
            }],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(
        world.recipes().property_sets.get("minecraft:furnace_input"),
        Some(&vec![42, 43])
    );
    assert_eq!(world.recipes().stonecutter_recipes.len(), 1);
    let world_counters = world.counters();
    assert_eq!(world_counters.update_recipes_packets, 1);
    assert_eq!(world_counters.recipe_property_sets_tracked, 1);
    assert_eq!(world_counters.recipe_property_set_items_tracked, 2);
    assert_eq!(world_counters.stonecutter_recipes_tracked, 1);
}

#[test]
fn client_common_waypoint_events_update_world_and_snapshot_counters() {
    let waypoint_id = Uuid::from_u128(0x00112233445566778899aabbccddeeff);
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::CustomPayload(CustomPayload {
        id: "minecraft:brand".to_string(),
        payload: CustomPayloadBody::Brand {
            brand: "vanilla".to_string(),
        },
    }))
    .unwrap();
    tx.try_send(NetEvent::ClearDialog).unwrap();
    tx.try_send(NetEvent::ShowDialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 11 },
    }))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::Waypoint(
        TrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: TrackedWaypoint {
                identifier: WaypointIdentifier::Uuid(waypoint_id),
                icon: WaypointIcon {
                    style: "minecraft:default".to_string(),
                    color_rgb: Some(0x112233),
                },
                data: WaypointData::Position(WaypointVec3i {
                    x: 10,
                    y: 64,
                    z: -5,
                }),
            },
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(world.server_brand(), Some("vanilla"));
    assert_eq!(
        world.last_custom_payload(),
        Some(&bbb_world::CustomPayloadState {
            id: "minecraft:brand".to_string(),
            kind: "brand".to_string(),
            brand: Some("vanilla".to_string()),
            raw_payload_len: 0,
        })
    );
    assert_eq!(
        world.current_dialog(),
        Some(&bbb_world::DialogState {
            holder_kind: "reference".to_string(),
            registry_id: Some(11),
            raw_dialog_payload_len: 0,
        })
    );
    let world_counters = world.counters();
    assert_eq!(world_counters.custom_payload_packets, 1);
    assert_eq!(world_counters.custom_payload_brand_packets, 1);
    assert_eq!(world_counters.custom_payload_unknown_packets, 0);
    assert_eq!(world_counters.clear_dialog_packets, 1);
    assert_eq!(world_counters.show_dialog_packets, 1);
    assert_eq!(world_counters.waypoint_packets, 1);
    assert_eq!(world_counters.waypoints_tracked, 1);
    assert_eq!(world_counters.waypoint_updates_applied, 0);
    assert_eq!(world_counters.waypoint_updates_ignored, 0);
    assert_eq!(world_counters.waypoint_untracks_ignored, 0);
    let waypoint_key = format!("uuid:{waypoint_id}");
    let tracked_waypoint = world
        .tracked_waypoints()
        .get(&waypoint_key)
        .expect("tracked waypoint is stored in world");
    assert_eq!(tracked_waypoint.identifier_kind, "uuid");
    assert_eq!(tracked_waypoint.identifier, waypoint_id.to_string());
    assert_eq!(tracked_waypoint.icon_style, "minecraft:default");
    assert_eq!(tracked_waypoint.icon_color_rgb, Some(0x112233));
    assert_eq!(
        tracked_waypoint.data,
        bbb_world::WaypointDataState {
            kind: "position".to_string(),
            position: Some(bbb_world::WaypointVec3iState {
                x: 10,
                y: 64,
                z: -5,
            }),
            chunk: None,
            azimuth: None,
        }
    );
    assert_eq!(
        world.last_waypoint_event(),
        Some(&bbb_world::WaypointEventState {
            operation: "track".to_string(),
            waypoint: tracked_waypoint.clone(),
            applied: true,
        })
    );
}

#[test]
fn player_action_events_update_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerCombatEnter))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerCombatEnd(
        PlayerCombatEnd { duration: 37 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerCombatKill(
        PlayerCombatKill {
            player_id: 123,
            message: "You died".to_string(),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerLookAt(
        PlayerLookAt {
            from_anchor: EntityAnchor::Eyes,
            position: ProtocolVec3d {
                x: 10.5,
                y: 64.0,
                z: -2.25,
            },
            target: Some(PlayerLookAtTarget {
                entity_id: 456,
                to_anchor: EntityAnchor::Feet,
            }),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(world.counters().player_combat_enter_packets, 1);
    assert_eq!(world.counters().player_combat_end_packets, 1);
    assert_eq!(world.counters().player_combat_kill_packets, 1);
    assert_eq!(
        world.last_player_combat(),
        Some(&bbb_world::PlayerCombatEventState {
            kind: "kill".to_string(),
            duration: None,
            player_id: Some(123),
            message: Some("You died".to_string()),
        })
    );
    assert_eq!(world.counters().player_look_at_packets, 1);
    assert_eq!(
        world.local_player().last_look_at,
        Some(bbb_world::LocalPlayerLookAtState {
            from_anchor: EntityAnchor::Eyes,
            position: ProtocolVec3d {
                x: 10.5,
                y: 64.0,
                z: -2.25,
            },
            target_entity_id: Some(456),
            to_anchor: Some(EntityAnchor::Feet),
        })
    );
}

#[test]
fn client_audio_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::Sound(SoundEvent {
        sound: SoundEventHolder::Reference { registry_id: 41 },
        source: SoundSource::Blocks,
        position: ProtocolVec3d {
            x: 2.5,
            y: -1.0,
            z: 0.0,
        },
        volume: 0.75,
        pitch: 1.25,
        seed: 123456789,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: Some(32.0),
            },
            source: SoundSource::Neutral,
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::StopSound(StopSound {
        source: Some(SoundSource::Music),
        name: Some("minecraft:music.menu".to_string()),
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.counters().sound_packets, 1);
    assert_eq!(
        world.last_sound(),
        Some(&bbb_world::SoundEventState {
            sound: bbb_world::SoundHolderState {
                kind: "reference".to_string(),
                registry_id: Some(41),
                location: None,
                fixed_range: None,
            },
            source: "block".to_string(),
            position: ProtocolVec3d {
                x: 2.5,
                y: -1.0,
                z: 0.0,
            },
            volume: 0.75,
            pitch: 1.25,
            seed: 123456789,
            distance_delay: false,
        })
    );
    assert_eq!(world.counters().sound_entity_packets, 1);
    assert_eq!(world.counters().sound_entity_events_applied, 1);
    assert_eq!(world.counters().sound_entity_events_ignored, 0);
    assert_eq!(
        world.last_sound_entity(),
        Some(&bbb_world::SoundEntityEventState {
            sound: bbb_world::SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some("minecraft:entity.cat.ambient".to_string()),
                fixed_range: Some(32.0),
            },
            source: "neutral".to_string(),
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        })
    );
    assert_eq!(world.counters().stop_sound_packets, 1);
    assert_eq!(
        world.last_stop_sound(),
        Some(&bbb_world::StopSoundEventState {
            source: Some("music".to_string()),
            name: Some("minecraft:music.menu".to_string()),
        })
    );
}

#[test]
fn client_audio_events_emit_runtime_commands_for_applied_events() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::Sound(SoundEvent {
        sound: SoundEventHolder::Reference { registry_id: 0 },
        source: SoundSource::Ambient,
        position: ProtocolVec3d {
            x: 2.5,
            y: -1.0,
            z: 0.0,
        },
        volume: 0.75,
        pitch: 1.25,
        seed: 123456789,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: Some(32.0),
            },
            source: SoundSource::Neutral,
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: None,
            },
            source: SoundSource::Neutral,
            entity_id: 404,
            volume: 0.2,
            pitch: 1.8,
            seed: 7,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::StopSound(StopSound {
        source: Some(SoundSource::Music),
        name: Some("minecraft:music.menu".to_string()),
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(
        test_sound_catalog(),
        SoundEventRegistry::from_ids(["minecraft:ambient.cave"]),
    );

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 3);
    let world_counters = world.counters();
    assert_eq!(world_counters.sound_packets, 1);
    assert_eq!(world_counters.sound_entity_packets, 2);
    assert_eq!(world_counters.sound_entity_events_applied, 1);
    assert_eq!(world_counters.sound_entity_events_ignored, 1);
    assert_eq!(world_counters.stop_sound_packets, 1);

    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.category, AudioCategory::Ambient);
            assert_eq!(command.position, [2.5, -1.0, 0.0]);
            assert_eq!(command.packet_volume, 0.75);
            assert_eq!(command.packet_pitch, 1.25);
            assert_eq!(command.seed, 123456789);
            assert_eq!(command.fixed_range, None);
            assert_eq!(command.sound.event_id, "minecraft:ambient.cave");
            assert_eq!(command.sound.sound_name, "minecraft:ambient/cave/cave1");
        }
        other => panic!("expected positioned sound command, got {other:?}"),
    }
    match &audio.commands[1] {
        AudioCommand::PlayEntitySound(command) => {
            assert_eq!(command.category, AudioCategory::Neutral);
            assert_eq!(command.entity_id, 123);
            assert_eq!(command.position, Some([1.0, 64.0, -2.0]));
            assert_eq!(command.packet_volume, 1.0);
            assert_eq!(command.packet_pitch, 0.5);
            assert_eq!(command.seed, -9);
            assert_eq!(command.fixed_range, Some(32.0));
            assert_eq!(command.sound.event_id, "minecraft:entity.cat.ambient");
            assert_eq!(command.sound.sound_name, "minecraft:mob/cat/meow1");
        }
        other => panic!("expected entity sound command, got {other:?}"),
    }
    match &audio.commands[2] {
        AudioCommand::StopSound(command) => {
            assert_eq!(command.category, Some(AudioCategory::Music));
            assert_eq!(command.name.as_deref(), Some("minecraft:music.menu"));
        }
        other => panic!("expected stop sound command, got {other:?}"),
    }
}

#[test]
fn sound_event_registry_data_updates_audio_reference_resolution() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::RegistryData(RegistryData {
        registry: "minecraft:sound_event".to_string(),
        raw_payload_len: 64,
        entries: vec![RegistryDataEntry {
            id: "minecraft:ambient.cave".to_string(),
            raw_data: None,
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::Sound(SoundEvent {
        sound: SoundEventHolder::Reference { registry_id: 0 },
        source: SoundSource::Ambient,
        position: ProtocolVec3d {
            x: 2.5,
            y: -1.0,
            z: 0.0,
        },
        volume: 0.75,
        pitch: 1.25,
        seed: 123456789,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(world.counters().registries_seen, 1);
    assert_eq!(
        world
            .registry_content("minecraft:sound_event")
            .unwrap()
            .entries[0]
            .id,
        "minecraft:ambient.cave"
    );
    match &audio.commands[0] {
        AudioCommand::PlayPositionedSound(command) => {
            assert_eq!(command.sound.event_id, "minecraft:ambient.cave");
            assert_eq!(command.sound.sound_name, "minecraft:ambient/cave/cave1");
        }
        other => panic!("expected positioned sound command, got {other:?}"),
    }
}

#[test]
fn silent_entity_sound_events_do_not_emit_runtime_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::SetEntityData(
        SetEntityData {
            id: 123,
            values: vec![EntityDataValue {
                data_id: 4,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SoundEntity(
        SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: Some(32.0),
            },
            source: SoundSource::Neutral,
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty(), "{:?}", audio.commands);
    assert_eq!(world.last_sound_entity(), None);
    assert_eq!(world.counters().sound_entity_packets, 1);
    assert_eq!(world.counters().sound_entity_events_applied, 0);
    assert_eq!(world.counters().sound_entity_events_ignored, 1);
}

#[test]
fn world_effect_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::Explosion(Explosion {
        center: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        radius: 4.5,
        block_count: 7,
        player_knockback: Some(ProtocolVec3d {
            x: 0.25,
            y: -0.5,
            z: 1.5,
        }),
        raw_effect_payload: vec![0x2d, 0x2a, 0x01, 0x00],
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelParticles(
        LevelParticles {
            override_limiter: true,
            always_show: false,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: ProtocolVec3d {
                x: f64::from(0.1_f32),
                y: f64::from(0.2_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 1.5,
            count: 16,
            particle: ParticlePayload {
                particle_type_id: 45,
                raw_options: vec![0xaa, 0xbb],
            },
        },
    )))
    .unwrap();
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        delta_movement: ProtocolVec3d {
            x: 0.5,
            y: -0.25,
            z: 1.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.counters().explosion_packets, 1);
    assert_eq!(
        world.last_explosion(),
        Some(&bbb_world::ExplosionEventState {
            center: ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            radius: 4.5,
            block_count: 7,
            player_knockback: Some(ProtocolVec3d {
                x: 0.25,
                y: -0.5,
                z: 1.5,
            }),
            raw_effect_payload_len: 4,
        })
    );
    assert_eq!(
        world.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d {
            x: 0.75,
            y: -0.75,
            z: 2.5,
        }
    );
    assert_eq!(world.counters().level_particles_packets, 1);
    assert_eq!(
        world.last_level_particles(),
        Some(&bbb_world::LevelParticlesEventState {
            override_limiter: true,
            always_show: false,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: ProtocolVec3d {
                x: f64::from(0.1_f32),
                y: f64::from(0.2_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 1.5,
            count: 16,
            particle_type_id: 45,
            raw_options_len: 2,
        })
    );
}

#[test]
fn level_particles_emit_particle_runtime_batch_and_world_counters() {
    let packet = LevelParticles {
        override_limiter: false,
        always_show: true,
        position: ProtocolVec3d {
            x: 10.0,
            y: 64.5,
            z: -3.25,
        },
        offset: ProtocolVec3d {
            x: f64::from(0.1_f32),
            y: f64::from(0.2_f32),
            z: f64::from(0.3_f32),
        },
        max_speed: 1.5,
        count: 0,
        particle: ParticlePayload {
            particle_type_id: 4,
            raw_options: vec![0xcc],
        },
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelParticles(
        packet.clone(),
    )))
    .unwrap();
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.packets, vec![packet]);
    assert_eq!(
        particles.contexts,
        vec![LevelParticleSpawnContext {
            camera_position: Some([1.0, 3.62, 3.0]),
            vibration_entity_position: None,
        }]
    );
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_particles_packets, 1);
    assert_eq!(world.last_level_particles().unwrap().count, 0);
}

#[test]
fn level_particles_context_resolves_vibration_entity_position_source() {
    let mut raw_options = vec![1, 123];
    raw_options.extend_from_slice(&0.75_f32.to_be_bytes());
    raw_options.push(27);
    let packet = LevelParticles {
        override_limiter: false,
        always_show: true,
        position: ProtocolVec3d {
            x: 10.0,
            y: 64.5,
            z: -3.25,
        },
        offset: ProtocolVec3d::default(),
        max_speed: 0.0,
        count: 0,
        particle: ParticlePayload {
            particle_type_id: 48,
            raw_options,
        },
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelParticles(
        packet.clone(),
    )))
    .unwrap();
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.packets, vec![packet]);
    assert_eq!(
        particles.contexts,
        vec![LevelParticleSpawnContext {
            camera_position: None,
            vibration_entity_position: Some(crate::particle_runtime::LevelParticleEntityPosition {
                entity_id: 123,
                position: [1.0, 64.0, -2.0],
            }),
        }]
    );
}

#[test]
fn level_event_smoke_particles_emit_particle_runtime_batch_and_world_counters() {
    let event = LevelEvent {
        event_type: 1502,
        pos: ProtocolBlockPos { x: 4, y: 65, z: -7 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
    assert_eq!(world.level_events().last().unwrap().event_type, 1502);
}

#[test]
fn post_sound_smoke_level_events_advance_particle_randoms_without_particle_sink() {
    fn advance_expected_particle_randoms(event_type: i32, random: &mut LevelEventSoundRandomState) {
        match event_type {
            1501 => {
                for _ in 0..8 {
                    let _ = random.next_double();
                    let _ = random.next_double();
                }
            }
            1502 => {
                for _ in 0..5 {
                    let _ = random.next_double();
                    let _ = random.next_double();
                    let _ = random.next_double();
                }
            }
            1503 => {
                for _ in 0..16 {
                    let _ = random.next_double();
                    let _ = random.next_double();
                }
            }
            _ => unreachable!("unexpected level event type {event_type}"),
        }
    }

    let cases = [
        (1501, "minecraft:block.lava.extinguish", true),
        (1502, "minecraft:block.redstone_torch.burnout", true),
        (1503, "minecraft:block.end_portal_frame.fill", false),
    ];

    for (event_type, expected_sound, randomized_pitch) in cases {
        let event = LevelEvent {
            event_type,
            pos: ProtocolBlockPos {
                x: event_type - 1500,
                y: 65,
                z: -7,
            },
            data: 0,
            global: false,
        };
        let followup = LevelEvent {
            event_type: 1004,
            pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
            data: 0,
            global: false,
        };
        let (tx, mut rx) = mpsc::channel(2);
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
            .unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
            .unwrap();

        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let expected_pitch = if randomized_pitch {
            2.6 + (expected_random.next_float() - expected_random.next_float()) * 0.8
        } else {
            1.0
        };
        let expected_seed = expected_random.next_long();
        advance_expected_particle_randoms(event_type, &mut expected_random);
        let expected_followup_seed = expected_random.next_long();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut audio =
            RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
        let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

        assert_eq!(
            drain_net_events_with_sinks(
                &mut rx,
                &mut world,
                &mut counters,
                &None,
                Some(&mut audio),
                None,
                None,
                None,
                &mut level_event_sound_random,
            ),
            2
        );

        assert!(audio.errors.is_empty(), "{:?}", audio.errors);
        assert_eq!(audio.commands.len(), 2);
        let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
            panic!(
                "expected positioned smoke-side sound, got {:?}",
                audio.commands[0]
            );
        };
        assert_eq!(sound.sound.event_id, expected_sound);
        assert_close(sound.packet_pitch, expected_pitch);
        assert_eq!(sound.seed, expected_seed);

        let AudioCommand::PlayPositionedSound(followup_sound) = &audio.commands[1] else {
            panic!(
                "expected positioned followup sound, got {:?}",
                audio.commands[1]
            );
        };
        assert_eq!(
            followup_sound.sound.event_id,
            "minecraft:entity.firework_rocket.shoot"
        );
        assert_eq!(followup_sound.seed, expected_followup_seed);
        assert_eq!(world.counters().level_events_received, 2);
        assert_eq!(world.counters().level_events_tracked, 2);
    }
}

fn advance_expected_simple_particle_only_level_event_randoms(
    event_type: i32,
    random: &mut LevelEventSoundRandomState,
) {
    match event_type {
        2000 | 2010 => advance_expected_shoot_particles_randoms(random),
        2003 => advance_expected_item_break_particle_randoms(random),
        2004 => {
            for _ in 0..20 {
                let _ = random.next_double();
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        2009 => {
            for _ in 0..8 {
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        _ => unreachable!("unexpected simple particle-only event type {event_type}"),
    }
}

fn advance_expected_shoot_particles_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..10 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

fn advance_expected_item_break_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..8 {
        let _ = random.next_gaussian();
        let _ = random.next_double();
        let _ = random.next_gaussian();
    }
}

fn advance_expected_block_face_axis_level_event_randoms(
    event_type: i32,
    data: i32,
    random: &mut LevelEventSoundRandomState,
) {
    match event_type {
        3002 if matches!(data, 0..=2) => advance_expected_axis_particles_randoms(10, 19, random),
        3002 => advance_expected_block_face_particle_randoms(3, 5, random),
        3004 | 3005 => advance_expected_block_face_particle_randoms(3, 5, random),
        3009 => advance_expected_block_face_particle_randoms(3, 6, random),
        _ => unreachable!("unexpected block-face/axis event type {event_type}"),
    }
}

fn advance_expected_block_face_particle_randoms(
    min_particles_per_face: i32,
    max_particles_per_face: i32,
    random: &mut LevelEventSoundRandomState,
) {
    for _ in 0..6 {
        let particle_count = random
            .next_int_bound(max_particles_per_face - min_particles_per_face + 1)
            + min_particles_per_face;
        for _ in 0..particle_count {
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
        }
    }
}

fn advance_expected_axis_particles_randoms(
    min_particles: i32,
    max_particles: i32,
    random: &mut LevelEventSoundRandomState,
) {
    let particle_count = random.next_int_bound(max_particles - min_particles + 1) + min_particles;
    for _ in 0..particle_count {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
    }
}

#[test]
fn particle_only_level_events_advance_randoms_without_particle_sink_before_followup_sound() {
    for event_type in [2000, 2003, 2004, 2009, 2010] {
        let event = LevelEvent {
            event_type,
            pos: ProtocolBlockPos {
                x: event_type - 1990,
                y: 64,
                z: -3,
            },
            data: 1,
            global: false,
        };
        let followup = LevelEvent {
            event_type: 1004,
            pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
            data: 0,
            global: false,
        };
        let (tx, mut rx) = mpsc::channel(2);
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
            .unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
            .unwrap();

        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        advance_expected_simple_particle_only_level_event_randoms(event_type, &mut expected_random);
        let expected_followup_seed = expected_random.next_long();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut audio =
            RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
        let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

        assert_eq!(
            drain_net_events_with_sinks(
                &mut rx,
                &mut world,
                &mut counters,
                &None,
                Some(&mut audio),
                None,
                None,
                None,
                &mut level_event_sound_random,
            ),
            2
        );

        assert!(audio.errors.is_empty(), "{:?}", audio.errors);
        assert_eq!(audio.commands.len(), 1);
        let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
            panic!(
                "expected positioned followup sound, got {:?}",
                audio.commands[0]
            );
        };
        assert_eq!(
            sound.sound.event_id,
            "minecraft:entity.firework_rocket.shoot"
        );
        assert_eq!(sound.seed, expected_followup_seed);
        assert_eq!(world.counters().level_events_received, 2);
        assert_eq!(world.counters().level_events_tracked, 2);
    }
}

#[test]
fn block_face_axis_level_events_advance_randoms_without_particle_sink_before_followup_sound() {
    for (event_type, data) in [(3002, 1), (3002, 99), (3004, 0), (3005, 0), (3009, 0)] {
        let event = LevelEvent {
            event_type,
            pos: ProtocolBlockPos {
                x: event_type - 3000,
                y: 64,
                z: -3,
            },
            data,
            global: false,
        };
        let followup = LevelEvent {
            event_type: 1004,
            pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
            data: 0,
            global: false,
        };
        let (tx, mut rx) = mpsc::channel(2);
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
            .unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
            .unwrap();

        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        advance_expected_block_face_axis_level_event_randoms(
            event_type,
            data,
            &mut expected_random,
        );
        let expected_followup_seed = expected_random.next_long();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut audio =
            RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
        let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

        assert_eq!(
            drain_net_events_with_sinks(
                &mut rx,
                &mut world,
                &mut counters,
                &None,
                Some(&mut audio),
                None,
                None,
                None,
                &mut level_event_sound_random,
            ),
            2
        );

        assert!(audio.errors.is_empty(), "{:?}", audio.errors);
        assert_eq!(audio.commands.len(), 1);
        let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
            panic!(
                "expected positioned followup sound, got {:?}",
                audio.commands[0]
            );
        };
        assert_eq!(
            sound.sound.event_id,
            "minecraft:entity.firework_rocket.shoot"
        );
        assert_eq!(sound.seed, expected_followup_seed);
        assert_eq!(world.counters().level_events_received, 2);
        assert_eq!(world.counters().level_events_tracked, 2);
    }
}

#[test]
fn sculk_charge_pop_level_event_threads_full_block_context_to_particles() {
    let full_block_event = LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let empty_block_event = LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos {
            x: 17,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChunksBiomes(
        ChunksBiomes {
            chunks: vec![ChunkBiomeData {
                pos: ProtocolChunkPos { x: 1, z: -2 },
                raw_biomes: single_biome_payload(7),
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: full_block_event.pos,
        block_state_id: 1,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(
        full_block_event,
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(
        empty_block_event,
    )))
    .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert_eq!(
        world
            .probe_block(bbb_world::BlockPos {
                x: 16,
                y: -64,
                z: -32,
            })
            .and_then(|probe| probe.block_name),
        Some("minecraft:stone".to_string())
    );
    assert_eq!(
        particles.level_events,
        vec![full_block_event, empty_block_event]
    );
    assert_eq!(
        particles.level_event_contexts,
        vec![
            LevelEventParticleContext {
                sculk_charge_pop_full_block: Some(true),
                block_state_id_at_event_pos: Some(1),
                biome_id_at_event_pos: Some(7),
                vault_block_entity_at_event_pos: false,
                vault_connection_particles: None,
                dripstone_drip_particle: None,
                growth_particles: None,
                in_block_particle_spread_height: None,
                composter_fill_center_shape_max_y: None,
            },
            LevelEventParticleContext {
                sculk_charge_pop_full_block: Some(false),
                block_state_id_at_event_pos: Some(0),
                biome_id_at_event_pos: Some(7),
                vault_block_entity_at_event_pos: false,
                vault_connection_particles: None,
                dripstone_drip_particle: None,
                growth_particles: None,
                in_block_particle_spread_height: None,
                composter_fill_center_shape_max_y: None,
            },
        ]
    );
}

#[test]
fn vault_activation_level_event_threads_block_entity_context_to_particles() {
    let vault_event = LevelEvent {
        event_type: 3015,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let non_vault_event = LevelEvent {
        event_type: 3015,
        pos: ProtocolBlockPos {
            x: 17,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: vault_event.pos,
            block_entity_type_id: 45,
            raw_nbt: vec![0],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(vault_event)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(non_vault_event)))
        .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    bbb_world::advance_vault_activation_particle_randoms(&mut expected_random);
    let expected_pitch = (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
    let expected_seed = expected_random.next_long();

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    assert_eq!(
        world.block_entity_type_id_at(bbb_world::BlockPos {
            x: 16,
            y: -64,
            z: -32,
        }),
        Some(45)
    );
    assert_eq!(particles.level_events, vec![vault_event, non_vault_event]);
    assert_eq!(
        particles.level_event_contexts[0].vault_block_entity_at_event_pos,
        true
    );
    assert_eq!(
        particles.level_event_contexts[1].vault_block_entity_at_event_pos,
        false
    );
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.vault.activate");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [16.5, -63.5, -31.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert!(command.distance_delay);
    assert_eq!(command.seed, expected_seed);
    let sound = world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.vault.activate")
    );
    assert_eq!(sound.seed, expected_seed);
    assert!(sound.distance_delay);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn vault_activation_level_event_threads_connection_particle_context() {
    let event = LevelEvent {
        event_type: 3015,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let player_uuid = Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff);
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: event.pos,
        block_state_id: vault_block_state_id("east"),
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEntityData(
        BlockEntityData {
            pos: event.pos,
            block_entity_type_id: 45,
            raw_nbt: vault_shared_data_nbt(&[player_uuid], Some(3.0)),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::AddEntity(
        vault_test_player(
            77,
            player_uuid,
            ProtocolVec3d {
                x: 18.25,
                y: -64.0,
                z: -32.0,
            },
        ),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    let context = particles.level_event_contexts[0].clone();
    assert!(context.vault_block_entity_at_event_pos);
    let connection = context.vault_connection_particles.unwrap();
    assert_eq!(connection.origin, [17.0, -62.25, -31.5]);
    assert_eq!(connection.targets.len(), 1);
    assert_eq!(connection.targets[0].entity_id, 77);
    assert_eq!(connection.targets[0].uuid, player_uuid);
    assert_close64(connection.targets[0].target_position[0], 18.25);
    assert_close64(connection.targets[0].target_position[1], -63.1);
    assert_close64(connection.targets[0].target_position[2], -32.0);
}

#[test]
fn vault_deactivation_level_event_emits_sound_after_particles() {
    let event = LevelEvent {
        event_type: 3016,
        pos: ProtocolBlockPos { x: -3, y: 72, z: 5 },
        data: 1,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    bbb_world::advance_vault_deactivation_particle_randoms(&mut expected_random);
    let expected_pitch = (expected_random.next_float() - expected_random.next_float()) * 0.2 + 1.0;
    let expected_seed = expected_random.next_long();

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.level_events, vec![event]);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.vault.deactivate");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [-2.5, 72.5, 5.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert!(command.distance_delay);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:block.vault.deactivate")
    );
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn dripstone_drip_level_event_context_uses_fluid_and_dimension_default() {
    let tip_id = vanilla_block_state_id(
        "minecraft:pointed_dripstone",
        [
            ("thickness", "tip"),
            ("vertical_direction", "down"),
            ("waterlogged", "false"),
        ],
    );
    let waterlogged_tip_id = vanilla_block_state_id(
        "minecraft:pointed_dripstone",
        [
            ("thickness", "tip"),
            ("vertical_direction", "down"),
            ("waterlogged", "true"),
        ],
    );
    let stone_id = vanilla_block_state_id("minecraft:stone", []);
    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    let lava_id = vanilla_block_state_id("minecraft:lava", [("level", "0")]);

    let mut world = WorldStore::new();
    world
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut world, block_pos(16, -64, -32), tip_id);
    set_test_block(&mut world, block_pos(16, -63, -32), stone_id);
    set_test_block(&mut world, block_pos(16, -62, -32), water_id);
    set_test_block(&mut world, block_pos(17, -64, -32), tip_id);
    set_test_block(&mut world, block_pos(17, -63, -32), stone_id);
    set_test_block(&mut world, block_pos(17, -62, -32), lava_id);
    set_test_block(&mut world, block_pos(18, -64, -32), waterlogged_tip_id);

    assert_eq!(
        level_event_particle_context(&world, &level_event_at(1504, 16, -64, -32))
            .dripstone_drip_particle,
        Some(LevelEventDripstoneDripParticle::Water)
    );
    assert_eq!(
        level_event_particle_context(&world, &level_event_at(1504, 17, -64, -32))
            .dripstone_drip_particle,
        Some(LevelEventDripstoneDripParticle::Lava)
    );
    assert_eq!(
        level_event_particle_context(&world, &level_event_at(1504, 18, -64, -32))
            .dripstone_drip_particle,
        None
    );

    let mut default_overworld = WorldStore::new();
    default_overworld
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut default_overworld, block_pos(19, -64, -32), tip_id);
    set_test_block(&mut default_overworld, block_pos(19, -63, -32), stone_id);
    assert_eq!(
        level_event_particle_context(&default_overworld, &level_event_at(1504, 19, -64, -32))
            .dripstone_drip_particle,
        Some(LevelEventDripstoneDripParticle::Water)
    );

    let mut nether = WorldStore::new();
    let mut login = protocol_play_login(1);
    login.levels = vec!["minecraft:the_nether".to_string()];
    login.common_spawn_info.dimension_type_id = 1;
    login.common_spawn_info.dimension = "minecraft:the_nether".to_string();
    nether.apply_login(&login);
    nether
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut nether, block_pos(16, 0, -32), tip_id);
    set_test_block(&mut nether, block_pos(16, 1, -32), stone_id);
    assert_eq!(
        level_event_particle_context(&nether, &level_event_at(1504, 16, 0, -32))
            .dripstone_drip_particle,
        Some(LevelEventDripstoneDripParticle::Lava)
    );
}

#[test]
fn plant_growth_level_event_context_uses_bonemeal_particle_branches() {
    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    let grass_id = vanilla_block_state_id("minecraft:grass_block", [("snowy", "false")]);
    let rooted_dirt_id = vanilla_block_state_id("minecraft:rooted_dirt", []);
    let short_grass_id = vanilla_block_state_id("minecraft:short_grass", []);
    let bottom_slab_id = vanilla_block_state_id(
        "minecraft:oak_slab",
        [("type", "bottom"), ("waterlogged", "false")],
    );
    let composter_6_id = vanilla_block_state_id("minecraft:composter", [("level", "6")]);
    let stone_id = vanilla_block_state_id("minecraft:stone", []);

    let mut world = WorldStore::new();
    world
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();

    set_test_block(&mut world, block_pos(16, -63, -32), water_id);
    set_test_block(&mut world, block_pos(16, -64, -32), stone_id);
    let mut water_support = LevelEventGrowthParticleSupport::empty();
    water_support.insert(0, 0);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 16, -63, -32, 15))
            .growth_particles,
        Some(LevelEventGrowthParticleContext {
            pos: block_pos(16, -63, -32),
            mode: LevelEventGrowthParticleMode::WideNoFloating {
                support: water_support
            },
        })
    );

    set_test_block(&mut world, block_pos(17, -64, -32), grass_id);
    let mut grass_support = LevelEventGrowthParticleSupport::empty();
    grass_support.insert(-1, 0);
    grass_support.insert(0, 0);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 17, -64, -32, 15))
            .growth_particles,
        Some(LevelEventGrowthParticleContext {
            pos: block_pos(17, -63, -32),
            mode: LevelEventGrowthParticleMode::WideNoFloating {
                support: grass_support
            },
        })
    );

    set_test_block(&mut world, block_pos(18, -63, -32), rooted_dirt_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 18, -63, -32, 15))
            .growth_particles,
        Some(LevelEventGrowthParticleContext {
            pos: block_pos(18, -64, -32),
            mode: LevelEventGrowthParticleMode::InBlock { spread_height: 1.0 },
        })
    );

    set_test_block(&mut world, block_pos(19, -63, -32), stone_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 19, -63, -32, 15))
            .growth_particles,
        None
    );

    set_test_block(&mut world, block_pos(20, -63, -32), short_grass_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1505, 20, -63, -32, 15))
            .growth_particles,
        Some(LevelEventGrowthParticleContext {
            pos: block_pos(20, -63, -32),
            mode: LevelEventGrowthParticleMode::InBlock {
                spread_height: 13.0 / 16.0
            },
        })
    );

    set_test_block(&mut world, block_pos(21, -63, -32), bottom_slab_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(2011, 21, -63, -32, 3))
            .in_block_particle_spread_height,
        Some(0.5)
    );

    set_test_block(&mut world, block_pos(22, -63, -32), composter_6_id);
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1500, 22, -63, -32, 1))
            .composter_fill_center_shape_max_y,
        Some(13.0 / 16.0)
    );
    assert_eq!(
        level_event_particle_context(&world, &level_event_at_with_data(1500, 23, -63, -32, 1))
            .composter_fill_center_shape_max_y,
        Some(1.0)
    );
}

#[test]
fn projectile_power_updates_world_entity_state_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::ProjectilePower(
        ProjectilePower {
            entity_id: 123,
            acceleration_power: 0.75,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ProjectilePower(
        ProjectilePower {
            entity_id: 456,
            acceleration_power: 0.25,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ProjectilePower(
        ProjectilePower {
            entity_id: 404,
            acceleration_power: 0.5,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
    ));
    world.apply_add_entity(protocol_add_entity_with_type(456, 7));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(
        world.hurting_projectile(123),
        Some(bbb_world::HurtingProjectileState {
            acceleration_power: 0.75,
        })
    );
    assert_eq!(world.hurting_projectile(456), None);
    assert_eq!(world.counters().projectile_power_packets, 3);
    assert_eq!(world.counters().projectile_power_updates_applied, 1);
    assert_eq!(world.counters().projectile_power_updates_ignored, 2);
    assert_eq!(
        world.last_projectile_power_update(),
        Some(&bbb_world::ProjectilePowerUpdateState {
            entity_id: 404,
            acceleration_power: 0.5,
            applied: false,
        })
    );
}

#[test]
fn debug_game_events_update_world_counters() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::Play(PlayClientbound::DebugBlockValue(
        DebugBlockValue {
            pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
            raw_update_payload: vec![5, 1, 0xaa],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugChunkValue(
        DebugChunkValue {
            pos: ProtocolChunkPos { x: 3, z: -4 },
            raw_update_payload: vec![7, 0],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugEntityValue(
        DebugEntityValue {
            entity_id: 123,
            raw_update_payload: vec![9, 1, 0xbb],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugEvent(DebugEvent {
        raw_event_payload: vec![4, 0xcc],
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DebugSample(DebugSample {
        sample: vec![100, -50],
        sample_type: RemoteDebugSampleType::TickTime,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameRuleValues(
        GameRuleValues {
            values: vec![
                GameRuleValue {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                GameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameTestHighlightPos(
        GameTestHighlightPos {
            absolute_pos: ProtocolBlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TestInstanceBlockStatus(
        TestInstanceBlockStatus {
            status: "Ready".to_string(),
            size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        8
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.debug_block_value_packets, 1);
    assert_eq!(world_counters.debug_chunk_value_packets, 1);
    assert_eq!(world_counters.debug_entity_value_packets, 1);
    assert_eq!(world_counters.debug_event_packets, 1);
    assert_eq!(world_counters.debug_sample_packets, 1);
    assert_eq!(world_counters.game_rule_value_packets, 1);
    assert_eq!(world_counters.game_test_highlight_pos_packets, 1);
    assert_eq!(world_counters.test_instance_block_status_packets, 1);
    assert_eq!(
        world.last_debug_block_value(),
        Some(&bbb_world::DebugBlockValueState {
            pos: BlockPos { x: 1, y: 64, z: -2 },
            raw_update_payload_len: 3,
        })
    );
    assert_eq!(
        world.last_debug_chunk_value(),
        Some(&bbb_world::DebugChunkValueState {
            pos: ChunkPos { x: 3, z: -4 },
            raw_update_payload_len: 2,
        })
    );
    assert_eq!(
        world.last_debug_entity_value(),
        Some(&bbb_world::DebugEntityValueState {
            entity_id: 123,
            raw_update_payload_len: 3,
        })
    );
    assert_eq!(
        world.last_debug_event(),
        Some(&bbb_world::DebugEventState {
            raw_event_payload_len: 2,
        })
    );
    assert_eq!(
        world.last_debug_sample(),
        Some(&bbb_world::DebugSampleState {
            sample_len: 2,
            sample_type: "tick_time".to_string(),
        })
    );
    assert_eq!(
        world.last_game_rule_values(),
        Some(&bbb_world::GameRuleValuesState {
            values: vec![
                bbb_world::GameRuleValueState {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                bbb_world::GameRuleValueState {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        })
    );
    assert_eq!(
        world.last_game_test_highlight_pos(),
        Some(&bbb_world::GameTestHighlightPosState {
            absolute_pos: BlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: BlockPos { x: 1, y: 2, z: 3 },
        })
    );
    assert_eq!(
        world.last_test_instance_block_status(),
        Some(&bbb_world::TestInstanceBlockStatusState {
            status: "Ready".to_string(),
            size: Some(bbb_world::DebugVec3iState { x: 3, y: 4, z: 5 }),
        })
    );
}

#[test]
fn game_events_emit_local_player_sounds_and_elder_guardian_particles() {
    let (tx, mut rx) = mpsc::channel(4);
    for (event_id, param) in [(6, 0.0), (9, 0.0), (10, 1.75), (10, 0.0)] {
        tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
            bbb_protocol::packets::GameEvent { event_id, param },
        )))
        .unwrap();
    }

    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 4.0,
            y: 70.0,
            z: -1.0,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 3);
    let expected = [
        (
            "minecraft:entity.arrow.hit_player",
            AudioCategory::Players,
            [4.0, 71.62, -1.0],
            0.18,
            0.45,
        ),
        (
            "minecraft:entity.puffer_fish.sting",
            AudioCategory::Neutral,
            [4.0, 70.0, -1.0],
            1.0,
            1.0,
        ),
        (
            "minecraft:entity.elder_guardian.curse",
            AudioCategory::Hostile,
            [4.0, 70.0, -1.0],
            1.0,
            1.0,
        ),
    ];
    for (command, (event_id, category, position, volume, pitch)) in
        audio.commands.iter().zip(expected)
    {
        let AudioCommand::PlayPositionedSound(command) = command else {
            panic!("expected positioned sound, got {command:?}");
        };
        assert_eq!(command.sound.event_id, event_id);
        assert_eq!(command.category, category);
        assert_close(command.position[0] as f32, position[0] as f32);
        assert_close(command.position[1] as f32, position[1] as f32);
        assert_close(command.position[2] as f32, position[2] as f32);
        assert_close(command.packet_volume, volume);
        assert_close(command.packet_pitch, pitch);
        assert_eq!(command.seed, 0);
    }

    assert_eq!(particles.packets.len(), 2);
    assert_eq!(particles.contexts.len(), 2);
    for packet in &particles.packets {
        assert_eq!(
            packet.particle.particle_type_id,
            crate::particle_runtime::ELDER_GUARDIAN_PARTICLE_TYPE_ID
        );
        assert_eq!(
            packet.position,
            ProtocolVec3d {
                x: 4.0,
                y: 70.0,
                z: -1.0,
            }
        );
        assert_eq!(packet.offset, ProtocolVec3d::default());
        assert_eq!(packet.max_speed, 1.0);
        assert_eq!(packet.count, 0);
        assert!(!packet.override_limiter);
        assert!(!packet.always_show);
    }
    for context in &particles.contexts {
        let camera_position = context.camera_position.expect("camera position");
        assert_close(camera_position[0] as f32, 4.0);
        assert_close(camera_position[1] as f32, 71.62);
        assert_close(camera_position[2] as f32, -1.0);
    }
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(world.counters().game_event_packets, 4);
}

#[test]
fn block_destruction_event_updates_world_and_counter() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 10,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 99,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            progress: 255,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.counters().block_destructions_received, 3);
    assert_eq!(world.counters().block_destructions_tracked, 0);
    assert_eq!(world.counters().block_destructions_removed, 1);
    assert_eq!(world.counters().block_destructions_ignored, 1);
    assert!(world.block_destruction(4).is_none());
}

#[test]
fn block_and_level_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::BlockEvent(
        bbb_protocol::packets::BlockEvent {
            pos: ProtocolBlockPos {
                x: 12,
                y: 65,
                z: -5,
            },
            b0: 2,
            b1: 9,
            block_id: 54,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(
        bbb_protocol::packets::LevelEvent {
            event_type: 1001,
            pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
            data: 42,
            global: true,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    let world_counters = world.counters();
    assert_eq!(world_counters.block_events_received, 1);
    assert_eq!(world_counters.block_events_tracked, 1);
    assert_eq!(world_counters.level_events_received, 1);
    assert_eq!(world_counters.level_events_tracked, 1);

    let block_event = world.block_events().first().unwrap();
    assert_eq!(
        block_event.pos,
        BlockPos {
            x: 12,
            y: 65,
            z: -5
        }
    );
    assert_eq!(block_event.b0, 2);
    assert_eq!(block_event.b1, 9);
    assert_eq!(block_event.block_id, 54);

    let level_event = world.level_events().first().unwrap();
    assert_eq!(level_event.event_type, 1001);
    assert_eq!(level_event.pos, BlockPos { x: 3, y: 4, z: 5 });
    assert_eq!(level_event.data, 42);
    assert!(level_event.global);
}

#[test]
fn level_event_2001_emits_vanilla_block_break_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 2001,
        pos: ProtocolBlockPos { x: 2, y: 3, z: -4 },
        data: 9,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.set_default_block_sound_profiles(BTreeMap::from([(
        "minecraft:grass_block".to_string(),
        WorldBlockSoundProfile {
            break_sound: "minecraft:block.grass.break".to_string(),
            hit_sound: "minecraft:block.grass.hit".to_string(),
            volume: 0.8,
            pitch: 1.2,
        },
    )]));
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.grass.break");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [2.5, 3.5, -3.5]);
    assert_close(command.packet_volume, 0.9);
    assert_close(command.packet_pitch, 0.96);
    assert_eq!(command.seed, -4_962_768_465_676_381_896);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn level_event_3008_emits_brushable_completion_sound_and_particles() {
    let suspicious_sand = vanilla_block_state_id("minecraft:suspicious_sand", [("dusted", "2")]);
    let event = LevelEvent {
        event_type: 3008,
        pos: ProtocolBlockPos { x: 4, y: 65, z: -6 },
        data: suspicious_sand,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:item.brush.brushing.sand");
    assert_eq!(command.category, AudioCategory::Players);
    assert_eq!(command.position, [4.5, 65.5, -5.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, -4_962_768_465_676_381_896);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:item.brush.brushing.sand")
    );
    assert_eq!(world.last_sound().unwrap().source, "player");
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn plant_growth_level_event_plays_bone_meal_sound_after_particles() {
    let short_grass_id = vanilla_block_state_id("minecraft:short_grass", []);
    let event = level_event_at_with_data(1505, 20, -63, -32, 4);
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    world
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut world, block_pos(20, -63, -32), short_grass_id);
    let context = level_event_particle_context(&world, &event);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_growth_randoms_for_context(&event, &context, &mut expected_random);
    let expected_seed = expected_random.next_long();

    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.level_event_contexts, vec![context]);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:item.bone_meal.use");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [20.5, -62.5, -31.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, expected_seed);
    let sound = world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:item.bone_meal.use")
    );
    assert_eq!(sound.source, "block");
    assert_eq!(sound.seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn plant_growth_level_event_audio_only_advances_particle_randoms_before_sound_seed() {
    let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);
    let stone_id = vanilla_block_state_id("minecraft:stone", []);
    let event = level_event_at_with_data(1505, 16, -63, -32, 3);
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    world
        .insert_level_chunk_with_light(synthetic_native_level_chunk_packet())
        .unwrap();
    set_test_block(&mut world, block_pos(16, -63, -32), water_id);
    set_test_block(&mut world, block_pos(16, -64, -32), stone_id);
    let context = level_event_particle_context(&world, &event);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_growth_randoms_for_context(&event, &context, &mut expected_random);
    let expected_seed = expected_random.next_long();

    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            None,
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:item.bone_meal.use");
    assert_eq!(command.seed, expected_seed);
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn fixed_level_event_emits_vanilla_positioned_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1004,
        pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:entity.firework_rocket.shoot"
    );
    assert_eq!(command.category, AudioCategory::Neutral);
    assert_eq!(command.position, [8.5, 64.5, -1.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.2);
    assert_eq!(command.seed, -4_962_768_465_676_381_896);
    let recorded = world.last_sound().unwrap();
    assert_eq!(
        recorded.sound.location.as_deref(),
        Some("minecraft:entity.firework_rocket.shoot")
    );
    assert_eq!(recorded.source, "neutral");
    assert_eq!(
        recorded.position,
        ProtocolVec3d {
            x: 8.5,
            y: 64.5,
            z: -1.5,
        }
    );
    assert_close(recorded.volume, 1.0);
    assert_close(recorded.pitch, 1.2);
    assert_eq!(recorded.seed, -4_962_768_465_676_381_896);
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn randomized_level_event_emits_vanilla_positioned_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1015,
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:entity.ghast.warn");
    assert_eq!(command.category, AudioCategory::Hostile);
    assert_eq!(command.position, [-3.5, 70.5, 9.5]);
    assert_close(command.packet_volume, 10.0);
    assert_close(command.packet_pitch, 0.979_905_37);
    assert_eq!(command.seed, 4_437_113_781_045_784_766);
    let recorded = world.last_sound().unwrap();
    assert_eq!(
        recorded.sound.location.as_deref(),
        Some("minecraft:entity.ghast.warn")
    );
    assert_eq!(recorded.source, "hostile");
    assert_eq!(
        recorded.position,
        ProtocolVec3d {
            x: -3.5,
            y: 70.5,
            z: 9.5,
        }
    );
    assert_close(recorded.volume, 10.0);
    assert_close(recorded.pitch, 0.979_905_37);
    assert_eq!(recorded.seed, 4_437_113_781_045_784_766);
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn potion_and_dragon_fireball_level_events_emit_vanilla_sounds() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 2002,
        pos: ProtocolBlockPos { x: 1, y: 64, z: -3 },
        data: 0x3366cc,
        global: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 2006,
        pos: ProtocolBlockPos { x: -2, y: 70, z: 4 },
        data: 1,
        global: false,
    })))
    .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_potion_break_level_event_particle_randoms(&mut expected_random);
    let expected_potion_pitch = 0.9 + expected_random.next_float().clamp(0.0, 1.0) * 0.1;
    let expected_potion_seed = expected_random.next_long();
    advance_dragon_fireball_explode_level_event_particle_randoms(&mut expected_random);
    let expected_dragon_pitch = 0.9 + expected_random.next_float().clamp(0.0, 1.0) * 0.1;
    let expected_dragon_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayPositionedSound(potion) = &audio.commands[0] else {
        panic!(
            "expected positioned potion sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(
        potion.sound.event_id,
        "minecraft:entity.splash_potion.break"
    );
    assert_eq!(potion.category, AudioCategory::Neutral);
    assert_eq!(potion.position, [1.5, 64.5, -2.5]);
    assert_close(potion.packet_volume, 1.0);
    assert_close(potion.packet_pitch, expected_potion_pitch);
    assert_eq!(potion.seed, expected_potion_seed);

    let AudioCommand::PlayPositionedSound(dragon) = &audio.commands[1] else {
        panic!(
            "expected positioned dragon sound, got {:?}",
            audio.commands[1]
        );
    };
    assert_eq!(
        dragon.sound.event_id,
        "minecraft:entity.dragon_fireball.explode"
    );
    assert_eq!(dragon.category, AudioCategory::Hostile);
    assert_eq!(dragon.position, [-1.5, 70.5, 4.5]);
    assert_close(dragon.packet_volume, 1.0);
    assert_close(dragon.packet_pitch, expected_dragon_pitch);
    assert_eq!(dragon.seed, expected_dragon_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:entity.dragon_fireball.explode")
    );
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn potion_break_level_event_plays_sound_after_particles() {
    let event = LevelEvent {
        event_type: 2007,
        pos: ProtocolBlockPos { x: 1, y: 64, z: -3 },
        data: 0x3366cc,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_potion_break_level_event_particle_randoms(&mut expected_random);
    let expected_pitch = 0.9 + expected_random.next_float().clamp(0.0, 1.0) * 0.1;
    let expected_seed = expected_random.next_long();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.level_events, vec![event]);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:entity.splash_potion.break"
    );
    assert_eq!(command.category, AudioCategory::Neutral);
    assert_eq!(command.position, [1.5, 64.5, -2.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:entity.splash_potion.break")
    );
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn dragon_fireball_level_event_plays_sound_after_particles() {
    let event = LevelEvent {
        event_type: 2006,
        pos: ProtocolBlockPos { x: -2, y: 70, z: 4 },
        data: 1,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_dragon_fireball_explode_level_event_particle_randoms(&mut expected_random);
    let expected_pitch = 0.9 + expected_random.next_float().clamp(0.0, 1.0) * 0.1;
    let expected_seed = expected_random.next_long();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert_eq!(particles.level_events, vec![event]);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:entity.dragon_fireball.explode"
    );
    assert_eq!(command.category, AudioCategory::Hostile);
    assert_eq!(command.position, [-1.5, 70.5, 4.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:entity.dragon_fireball.explode")
    );
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

fn advance_expected_trial_spawner_level_event_particle_randoms(
    event_type: i32,
    data: i32,
    random: &mut LevelEventSoundRandomState,
) {
    match event_type {
        3012 | 3021 => advance_expected_trial_spawner_spawn_particle_randoms(random),
        3013 | 3019 => advance_expected_trial_spawner_detect_player_particle_randoms(data, random),
        3014 => advance_expected_trial_spawner_eject_item_particle_randoms(random),
        3020 => {
            advance_expected_trial_spawner_detect_player_particle_randoms(0, random);
            advance_expected_trial_spawner_become_ominous_particle_randoms(random);
        }
        _ => unreachable!("unexpected trial spawner level event type {event_type}"),
    }
}

fn advance_expected_trial_spawner_spawn_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
    }
}

fn advance_expected_trial_spawner_detect_player_particle_randoms(
    data: i32,
    random: &mut LevelEventSoundRandomState,
) {
    let count = 30_i64 + i64::from(data.min(10)) * 5;
    for _ in 0..count.max(0) {
        let _ = random.next_float();
        let _ = random.next_float();
        let _ = random.next_float();
    }
}

fn advance_expected_trial_spawner_eject_item_particle_randoms(
    random: &mut LevelEventSoundRandomState,
) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

fn advance_expected_trial_spawner_become_ominous_particle_randoms(
    random: &mut LevelEventSoundRandomState,
) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

#[test]
fn trial_spawner_level_events_emit_distance_delayed_vanilla_sounds() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 3012,
        pos: ProtocolBlockPos { x: 4, y: 65, z: -6 },
        data: 0,
        global: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 3020,
        pos: ProtocolBlockPos { x: -8, y: 70, z: 2 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_spawn_pitch =
        1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2;
    let expected_spawn_seed = expected_random.next_long();
    advance_expected_trial_spawner_level_event_particle_randoms(3012, 0, &mut expected_random);
    let expected_ominous_pitch =
        1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2;
    let expected_ominous_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayPositionedSound(spawn) = &audio.commands[0] else {
        panic!(
            "expected positioned trial spawner sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(
        spawn.sound.event_id,
        "minecraft:block.trial_spawner.spawn_mob"
    );
    assert_eq!(spawn.category, AudioCategory::Blocks);
    assert_eq!(spawn.position, [4.5, 65.5, -5.5]);
    assert_close(spawn.packet_volume, 1.0);
    assert_close(spawn.packet_pitch, expected_spawn_pitch);
    assert_eq!(spawn.seed, expected_spawn_seed);
    assert!(spawn.distance_delay);

    let AudioCommand::PlayPositionedSound(ominous) = &audio.commands[1] else {
        panic!(
            "expected positioned trial spawner ominous sound, got {:?}",
            audio.commands[1]
        );
    };
    assert_eq!(
        ominous.sound.event_id,
        "minecraft:block.trial_spawner.ominous_activate"
    );
    assert_eq!(ominous.category, AudioCategory::Blocks);
    assert_eq!(ominous.position, [-7.5, 70.5, 2.5]);
    assert_close(ominous.packet_volume, 0.3);
    assert_close(ominous.packet_pitch, expected_ominous_pitch);
    assert_eq!(ominous.seed, expected_ominous_seed);
    assert!(ominous.distance_delay);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:block.trial_spawner.ominous_activate")
    );
    assert!(world.last_sound().unwrap().distance_delay);
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn trial_spawner_audio_only_events_advance_post_sound_particle_randoms() {
    let cases = [
        (3012, 1, "minecraft:block.trial_spawner.spawn_mob"),
        (3013, 2, "minecraft:block.trial_spawner.detect_player"),
        (3014, 0, "minecraft:block.trial_spawner.eject_item"),
        (3019, 10, "minecraft:block.trial_spawner.detect_player"),
        (3020, 3, "minecraft:block.trial_spawner.ominous_activate"),
        (3021, 1, "minecraft:block.trial_spawner.spawn_item"),
    ];

    for (event_type, data, expected_sound) in cases {
        let event = LevelEvent {
            event_type,
            pos: ProtocolBlockPos {
                x: event_type - 3000,
                y: 65,
                z: -6,
            },
            data,
            global: false,
        };
        let followup = LevelEvent {
            event_type: 1004,
            pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
            data: 0,
            global: false,
        };
        let (tx, mut rx) = mpsc::channel(2);
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
            .unwrap();
        tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
            .unwrap();

        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let expected_pitch =
            1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2;
        let expected_seed = expected_random.next_long();
        advance_expected_trial_spawner_level_event_particle_randoms(
            event_type,
            data,
            &mut expected_random,
        );
        let expected_followup_seed = expected_random.next_long();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();
        let mut audio =
            RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
        let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

        assert_eq!(
            drain_net_events_with_sinks(
                &mut rx,
                &mut world,
                &mut counters,
                &None,
                Some(&mut audio),
                None,
                None,
                None,
                &mut level_event_sound_random,
            ),
            2
        );

        assert!(audio.errors.is_empty(), "{:?}", audio.errors);
        assert_eq!(audio.commands.len(), 2);
        let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
            panic!(
                "expected positioned trial spawner sound, got {:?}",
                audio.commands[0]
            );
        };
        assert_eq!(sound.sound.event_id, expected_sound);
        assert_close(sound.packet_pitch, expected_pitch);
        assert_eq!(sound.seed, expected_seed);
        assert!(sound.distance_delay);

        let AudioCommand::PlayPositionedSound(followup_sound) = &audio.commands[1] else {
            panic!(
                "expected positioned followup sound, got {:?}",
                audio.commands[1]
            );
        };
        assert_eq!(
            followup_sound.sound.event_id,
            "minecraft:entity.firework_rocket.shoot"
        );
        assert_eq!(followup_sound.seed, expected_followup_seed);
        assert_eq!(world.counters().level_events_received, 2);
        assert_eq!(world.counters().level_events_tracked, 2);
    }
}

#[test]
fn trial_spawner_level_event_emits_sound_and_particle_side_effects() {
    let event = LevelEvent {
        event_type: 3012,
        pos: ProtocolBlockPos { x: 4, y: 65, z: -6 },
        data: 1,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = 1.0 + (expected_random.next_float() - expected_random.next_float()) * 0.2;
    let expected_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(sound) = &audio.commands[0] else {
        panic!(
            "expected positioned trial spawner sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(
        sound.sound.event_id,
        "minecraft:block.trial_spawner.spawn_mob"
    );
    assert_eq!(sound.category, AudioCategory::Blocks);
    assert_close(sound.packet_pitch, expected_pitch);
    assert_eq!(sound.seed, expected_seed);
    assert!(sound.distance_delay);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

fn advance_expected_sculk_charge_level_event_particle_randoms(
    data: i32,
    full_block_pop: bool,
    random: &mut LevelEventSoundRandomState,
) {
    let count = data >> 6;
    if count <= 0 {
        let particle_count = if full_block_pop { 40 } else { 20 };
        for _ in 0..particle_count {
            let _ = random.next_float();
            let _ = random.next_float();
            let _ = random.next_float();
        }
        return;
    }

    let particle_data = data & 63;
    let face_count = if particle_data == 0 {
        6
    } else {
        particle_data.count_ones()
    };
    for _ in 0..face_count {
        let particle_count = random.next_int_bound(count + 1);
        for _ in 0..particle_count {
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
        }
    }
}

#[test]
fn sculk_charge_level_event_emits_vanilla_randomized_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos { x: -2, y: 68, z: 3 },
        data: 5 << 6,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.sculk.charge");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [-1.5, 68.5, 3.5]);
    assert_close(command.packet_volume, 0.565_720_5);
    assert_close(command.packet_pitch, 0.760_804_6);
    assert_eq!(command.seed, -7_261_648_964_369_397_258);
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn sculk_charge_audio_only_charged_event_advances_particle_randoms() {
    let data = (5 << 6) | 0b001011;
    let event = LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos { x: -2, y: 68, z: 3 },
        data,
        global: false,
    };
    let followup = LevelEvent {
        event_type: 1004,
        pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let count = (data >> 6) as f32;
    assert!(expected_random.next_float() < 0.3 + count * 0.1);
    let expected_volume = 0.15 + 0.02 * count * count * expected_random.next_float();
    let expected_pitch = 0.4 + 0.3 * count * expected_random.next_float();
    let expected_seed = expected_random.next_long();
    advance_expected_sculk_charge_level_event_particle_randoms(data, false, &mut expected_random);
    let expected_followup_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            None,
            None,
            None,
            &mut level_event_sound_random,
        ),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayPositionedSound(sculk) = &audio.commands[0] else {
        panic!(
            "expected positioned sculk sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(sculk.sound.event_id, "minecraft:block.sculk.charge");
    assert_close(sculk.packet_volume, expected_volume);
    assert_close(sculk.packet_pitch, expected_pitch);
    assert_eq!(sculk.seed, expected_seed);

    let AudioCommand::PlayPositionedSound(followup_sound) = &audio.commands[1] else {
        panic!(
            "expected positioned followup sound, got {:?}",
            audio.commands[1]
        );
    };
    assert_eq!(
        followup_sound.sound.event_id,
        "minecraft:entity.firework_rocket.shoot"
    );
    assert_eq!(followup_sound.seed, expected_followup_seed);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn sculk_charge_audio_only_pop_event_uses_full_block_particle_context() {
    let event = LevelEvent {
        event_type: 3006,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let followup = LevelEvent {
        event_type: 1004,
        pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: event.pos,
        block_state_id: 1,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(followup)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_seed = expected_random.next_long();
    advance_expected_sculk_charge_level_event_particle_randoms(0, true, &mut expected_random);
    let expected_followup_seed = expected_random.next_long();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            None,
            None,
            None,
            &mut level_event_sound_random,
        ),
        4
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayPositionedSound(sculk) = &audio.commands[0] else {
        panic!(
            "expected positioned sculk sound, got {:?}",
            audio.commands[0]
        );
    };
    assert_eq!(sculk.sound.event_id, "minecraft:block.sculk.charge");
    assert_close(sculk.packet_volume, 1.0);
    assert_close(sculk.packet_pitch, 1.0);
    assert_eq!(sculk.seed, expected_seed);

    let AudioCommand::PlayPositionedSound(followup_sound) = &audio.commands[1] else {
        panic!(
            "expected positioned followup sound, got {:?}",
            audio.commands[1]
        );
    };
    assert_eq!(
        followup_sound.sound.event_id,
        "minecraft:entity.firework_rocket.shoot"
    );
    assert_eq!(followup_sound.seed, expected_followup_seed);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn sculk_shrieker_level_event_emits_waterlogged_gated_sound_after_particles() {
    let dry_event = LevelEvent {
        event_type: 3007,
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let wet_event = LevelEvent {
        event_type: 3007,
        pos: ProtocolBlockPos {
            x: 17,
            y: -64,
            z: -32,
        },
        data: 0,
        global: false,
    };
    let dry_shrieker = vanilla_block_state_id(
        "minecraft:sculk_shrieker",
        [
            ("can_summon", "false"),
            ("shrieking", "false"),
            ("waterlogged", "false"),
        ],
    );
    let wet_shrieker = vanilla_block_state_id(
        "minecraft:sculk_shrieker",
        [
            ("can_summon", "false"),
            ("shrieking", "false"),
            ("waterlogged", "true"),
        ],
    );
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: dry_event.pos,
        block_state_id: dry_shrieker,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BlockUpdate(BlockUpdate {
        pos: wet_event.pos,
        block_state_id: wet_shrieker,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(dry_event)))
        .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(wet_event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    let expected_pitch = 0.6 + expected_random.next_float() * 0.4;
    let expected_seed = expected_random.next_long();

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        5
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(particles.level_events, vec![dry_event, wet_event]);
    assert_eq!(particles.batches.len(), 2);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:block.sculk_shrieker.shriek"
    );
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [16.5, -63.5, -31.5]);
    assert_close(command.packet_volume, 2.0);
    assert_close(command.packet_pitch, expected_pitch);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:block.sculk_shrieker.shriek")
    );
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn end_gateway_level_event_emits_vanilla_sound_and_particles() {
    let event = LevelEvent {
        event_type: 3000,
        pos: ProtocolBlockPos { x: 8, y: 64, z: -2 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.end_gateway.spawn");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [8.5, 64.5, -1.5]);
    assert_close(command.packet_volume, 10.0);
    assert_close(command.packet_pitch, 0.685_933_77);
    assert_eq!(command.seed, 4_437_113_781_045_784_766);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn cobweb_place_level_event_emits_particles_before_distance_delayed_sound() {
    let event = LevelEvent {
        event_type: 3018,
        pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.cobweb.place");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [2.5, 64.5, -4.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.013_698_2);
    assert_eq!(command.seed, 536_938_910_405_906_015);
    assert!(command.distance_delay);
    let sound = world.last_sound().unwrap();
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.cobweb.place")
    );
    assert_eq!(sound.seed, 536_938_910_405_906_015);
    assert!(sound.distance_delay);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn wax_on_level_event_emits_vanilla_sound_and_particles() {
    let event = LevelEvent {
        event_type: 3003,
        pos: ProtocolBlockPos { x: -3, y: 72, z: 5 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particles = RecordingParticleSink::default();
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);
    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_wax_on_level_event_particle_randoms(&mut expected_random);
    let expected_seed = expected_random.next_long();

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            Some(&mut particles),
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:item.honeycomb.wax_on");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [-2.5, 72.5, 5.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(particles.level_events, vec![event]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(
        world.last_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:item.honeycomb.wax_on")
    );
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn wax_on_level_event_audio_only_advances_particles_before_sound_seed() {
    let event = LevelEvent {
        event_type: 3003,
        pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
        data: 0,
        global: false,
    };
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(event)))
        .unwrap();

    let mut expected_random = LevelEventSoundRandomState::with_seed(0);
    advance_wax_on_level_event_particle_randoms(&mut expected_random);
    let expected_seed = expected_random.next_long();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut level_event_sound_random = LevelEventSoundRandomState::with_seed(0);

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            Some(&mut audio),
            None,
            None,
            None,
            &mut level_event_sound_random,
        ),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:item.honeycomb.wax_on");
    assert_eq!(command.category, AudioCategory::Blocks);
    assert_eq!(command.position, [1.5, 64.5, -1.5]);
    assert_close(command.packet_volume, 1.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, expected_seed);
    assert_eq!(world.last_sound().unwrap().seed, expected_seed);
    assert_eq!(world.counters().level_events_received, 1);
    assert_eq!(world.counters().level_events_tracked, 1);
}

#[test]
fn global_level_event_emits_vanilla_camera_relative_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1028,
        pos: ProtocolBlockPos { x: 10, y: 0, z: 0 },
        data: 0,
        global: true,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.5,
            y: -1.12,
            z: 0.5,
        },
        ..LocalPlayerPoseState::default()
    });
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayPositionedSound(command) = &audio.commands[0] else {
        panic!("expected positioned sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(
        command.sound.event_id,
        "minecraft:entity.ender_dragon.death"
    );
    assert_eq!(command.category, AudioCategory::Hostile);
    assert!((command.position[0] - 2.5).abs() < 1.0e-6);
    assert!((command.position[1] - 0.5).abs() < 1.0e-6);
    assert!((command.position[2] - 0.5).abs() < 1.0e-6);
    assert_close(command.packet_volume, 5.0);
    assert_close(command.packet_pitch, 1.0);
    assert_eq!(command.seed, -4_962_768_465_676_381_896);
    let recorded = world.last_sound().unwrap();
    assert_eq!(
        recorded.sound.location.as_deref(),
        Some("minecraft:entity.ender_dragon.death")
    );
    assert_eq!(recorded.source, "hostile");
    assert!((recorded.position.x - 2.5).abs() < 1.0e-6);
    assert!((recorded.position.y - 0.5).abs() < 1.0e-6);
    assert!((recorded.position.z - 0.5).abs() < 1.0e-6);
    assert_close(recorded.volume, 5.0);
    assert_close(recorded.pitch, 1.0);
    assert_eq!(recorded.seed, -4_962_768_465_676_381_896);
    assert_eq!(world.counters().sound_packets, 0);
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn global_level_event_without_camera_does_not_emit_runtime_sound() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1023,
        pos: ProtocolBlockPos { x: 10, y: 0, z: 0 },
        data: 0,
        global: true,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert!(audio.commands.is_empty());
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn portal_travel_level_event_emits_vanilla_local_ambience() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1032,
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        1
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 1);
    let AudioCommand::PlayLocalSound(command) = &audio.commands[0] else {
        panic!("expected local sound, got {:?}", audio.commands[0]);
    };
    assert_eq!(command.sound.event_id, "minecraft:block.portal.travel");
    assert_eq!(command.sound.sound_name, "minecraft:portal/travel");
    assert_eq!(command.category, AudioCategory::Ambient);
    assert_close(command.packet_volume, 0.25);
    assert_close(command.packet_pitch, 1.092_387_1);
    assert_eq!(command.seed, 0);
    assert_eq!(
        world.last_local_sound().unwrap().sound.location.as_deref(),
        Some("minecraft:block.portal.travel")
    );
    assert_eq!(world.counters().level_events_received, 1);
}

#[test]
fn jukebox_level_events_update_world_audio_state() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1010,
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        data: 27,
        global: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1011,
        pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    assert!(world.playing_jukebox_songs().is_empty());
    let event = world.last_jukebox_event().unwrap();
    assert_eq!(event.action, bbb_world::JukeboxLevelEventAction::Stop);
    assert_eq!(event.pos, BlockPos { x: -4, y: 70, z: 9 });
    assert_eq!(event.song_registry_id, None);
    assert!(event.stopped_existing);
    assert_eq!(world.counters().level_events_received, 2);
    assert_eq!(world.counters().level_events_tracked, 2);
}

#[test]
fn jukebox_level_events_emit_runtime_audio_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1010,
        pos: ProtocolBlockPos { x: 4, y: 64, z: -7 },
        data: 1,
        global: false,
    })))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1011,
        pos: ProtocolBlockPos { x: 4, y: 64, z: -7 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(audio.commands.len(), 2);
    let AudioCommand::PlayJukeboxSong(play) = &audio.commands[0] else {
        panic!("expected jukebox play command, got {:?}", audio.commands[0]);
    };
    assert_eq!(play.sound.event_id, "minecraft:music_disc.cat");
    assert_eq!(play.sound.sound_name, "minecraft:records/cat");
    assert_eq!(play.category, AudioCategory::Records);
    assert_eq!(play.position, [4.5, 64.5, -6.5]);
    assert_eq!(play.jukebox_pos, [4, 64, -7]);
    assert_close(play.packet_volume, 4.0);
    assert_close(play.packet_pitch, 1.0);

    let AudioCommand::StopJukeboxSong(stop) = &audio.commands[1] else {
        panic!("expected jukebox stop command, got {:?}", audio.commands[1]);
    };
    assert_eq!(stop.jukebox_pos, [4, 64, -7]);
    assert!(world.playing_jukebox_songs().is_empty());
    assert_eq!(world.counters().level_events_received, 2);
}

#[test]
fn jukebox_song_registry_data_updates_audio_resolution() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::RegistryData(RegistryData {
        registry: "minecraft:jukebox_song".to_string(),
        raw_payload_len: 48,
        entries: vec![RegistryDataEntry {
            id: "minecraft:tears".to_string(),
            raw_data: None,
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::LevelEvent(LevelEvent {
        event_type: 1010,
        pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        data: 0,
        global: false,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());

    assert_eq!(
        drain_net_events_with_audio(&mut rx, &mut world, &mut counters, &None, Some(&mut audio)),
        2
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    assert_eq!(world.counters().registries_seen, 1);
    assert_eq!(
        world
            .registry_content("minecraft:jukebox_song")
            .unwrap()
            .entries[0]
            .id,
        "minecraft:tears"
    );
    let AudioCommand::PlayJukeboxSong(play) = &audio.commands[0] else {
        panic!("expected jukebox play command, got {:?}", audio.commands[0]);
    };
    assert_eq!(play.sound.event_id, "minecraft:music_disc.tears");
    assert_eq!(play.sound.sound_name, "minecraft:records/tears");
    assert_eq!(play.jukebox_pos, [1, 2, 3]);
}

#[test]
fn border_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::Play(PlayClientbound::InitializeBorder(
        bbb_protocol::packets::InitializeBorder {
            new_center_x: 1.0,
            new_center_z: 2.0,
            old_size: 100.0,
            new_size: 200.0,
            lerp_time: 40,
            new_absolute_max_size: 500,
            warning_blocks: 6,
            warning_time: 7,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderCenter(
        bbb_protocol::packets::SetBorderCenter {
            new_center_x: 3.0,
            new_center_z: 4.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderLerpSize(
        bbb_protocol::packets::SetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderSize(
        bbb_protocol::packets::SetBorderSize { size: 250.0 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderWarningDelay(
        bbb_protocol::packets::SetBorderWarningDelay { warning_delay: 9 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetBorderWarningDistance(
        bbb_protocol::packets::SetBorderWarningDistance { warning_blocks: 8 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        6
    );

    let border = world.world_border();
    assert_eq!(border.center_x, 3.0);
    assert_eq!(border.center_z, 4.0);
    assert_eq!(border.size, 250.0);
    assert_eq!(border.lerp_target, 250.0);
    assert_eq!(border.lerp_time, 0);
    assert_eq!(border.absolute_max_size, 500);
    assert_eq!(border.warning_blocks, 8);
    assert_eq!(border.warning_time, 9);

    let world_counters = world.counters();
    assert_eq!(world_counters.world_border_initializes_received, 1);
    assert_eq!(world_counters.world_border_center_updates_received, 1);
    assert_eq!(world_counters.world_border_lerp_size_updates_received, 1);
    assert_eq!(world_counters.world_border_size_updates_received, 1);
    assert_eq!(
        world_counters.world_border_warning_delay_updates_received,
        1
    );
    assert_eq!(
        world_counters.world_border_warning_distance_updates_received,
        1
    );
}

#[test]
fn scoreboard_events_update_world_and_world_counters() {
    let (tx, mut rx) = mpsc::channel(11);
    tx.try_send(NetEvent::Play(PlayClientbound::SetObjective(
        bbb_protocol::packets::SetObjective {
            objective_name: "kills".to_string(),
            method: bbb_protocol::packets::SetObjectiveMethod::Add,
            parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                display_name: "Kills".to_string(),
                render_type: bbb_protocol::packets::ObjectiveRenderType::Integer,
                number_format: Some(vec![9]),
            }),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetDisplayObjective(
        bbb_protocol::packets::SetDisplayObjective {
            slot: bbb_protocol::packets::ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("kills".to_string()),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetScore(
        bbb_protocol::packets::SetScore {
            owner: "Steve".to_string(),
            objective_name: "kills".to_string(),
            score: 4,
            display: Some("Four".to_string()),
            number_format: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetScore(
        bbb_protocol::packets::SetScore {
            owner: "Alex".to_string(),
            objective_name: "kills".to_string(),
            score: 1,
            display: None,
            number_format: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetPlayerTeam(
        bbb_protocol::packets::SetPlayerTeam {
            name: "red".to_string(),
            method: bbb_protocol::packets::PlayerTeamMethod::Add,
            parameters: Some(bbb_protocol::packets::PlayerTeamParameters {
                display_name: "Red Team".to_string(),
                options: 0b11,
                nametag_visibility: bbb_protocol::packets::TeamVisibility::Always,
                collision_rule: bbb_protocol::packets::TeamCollisionRule::Never,
                color: bbb_protocol::packets::ChatFormatting::Red,
                player_prefix: "[R]".to_string(),
                player_suffix: "!".to_string(),
            }),
            players: vec!["Steve".to_string()],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ResetScore(
        bbb_protocol::packets::ResetScore {
            owner: "Alex".to_string(),
            objective_name: Some("kills".to_string()),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetObjective(
        bbb_protocol::packets::SetObjective {
            objective_name: "missing".to_string(),
            method: bbb_protocol::packets::SetObjectiveMethod::Change,
            parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                display_name: "Missing".to_string(),
                render_type: bbb_protocol::packets::ObjectiveRenderType::Integer,
                number_format: None,
            }),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetDisplayObjective(
        bbb_protocol::packets::SetDisplayObjective {
            slot: bbb_protocol::packets::ScoreboardDisplaySlot::List,
            objective_name: Some("missing".to_string()),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetScore(
        bbb_protocol::packets::SetScore {
            owner: "Nobody".to_string(),
            objective_name: "missing".to_string(),
            score: 9,
            display: None,
            number_format: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetPlayerTeam(
        bbb_protocol::packets::SetPlayerTeam {
            name: "missing".to_string(),
            method: bbb_protocol::packets::PlayerTeamMethod::Join,
            parameters: None,
            players: vec!["Nobody".to_string()],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ResetScore(
        bbb_protocol::packets::ResetScore {
            owner: "Nobody".to_string(),
            objective_name: Some("missing".to_string()),
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        11
    );
    let world_counters = world.counters();
    assert_eq!(world_counters.set_objective_packets, 2);
    assert_eq!(world_counters.set_objective_updates_applied, 1);
    assert_eq!(world_counters.set_objective_updates_ignored, 1);
    assert_eq!(world_counters.set_display_objective_packets, 2);
    assert_eq!(world_counters.set_display_objective_updates_applied, 1);
    assert_eq!(world_counters.set_display_objective_updates_ignored, 1);
    assert_eq!(world_counters.set_score_packets, 3);
    assert_eq!(world_counters.set_score_updates_applied, 2);
    assert_eq!(world_counters.set_score_updates_ignored, 1);
    assert_eq!(world_counters.set_player_team_packets, 2);
    assert_eq!(world_counters.set_player_team_updates_applied, 1);
    assert_eq!(world_counters.set_player_team_updates_ignored, 1);
    assert_eq!(world_counters.reset_score_packets, 2);
    assert_eq!(world_counters.reset_score_updates_applied, 1);
    assert_eq!(world_counters.reset_score_updates_ignored, 1);

    let scoreboard = world.scoreboard();
    let objective = scoreboard.objectives.get("kills").unwrap();
    assert_eq!(objective.display_name, "Kills");
    assert_eq!(objective.render_type, "integer");
    assert_eq!(objective.number_format, Some(vec![9]));
    assert_eq!(
        scoreboard.display_slots.get("sidebar").map(String::as_str),
        Some("kills")
    );

    let steve_scores = scoreboard.scores.get("Steve").unwrap();
    let steve_kills = steve_scores.get("kills").unwrap();
    assert_eq!(steve_kills.value, 4);
    assert_eq!(steve_kills.display.as_deref(), Some("Four"));
    assert!(!scoreboard.scores.contains_key("Alex"));

    let team = scoreboard.teams.get("red").unwrap();
    assert!(team.players.contains("Steve"));
    let parameters = team.parameters.as_ref().unwrap();
    assert_eq!(parameters.display_name, "Red Team");
    assert_eq!(parameters.color, "red");
}

#[test]
fn hud_session_events_update_world_and_world_counters() {
    let boss_id = Uuid::from_u128(1);
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::BossEvent(
        bbb_protocol::packets::BossEvent {
            id: boss_id,
            operation: bbb_protocol::packets::BossEventOperation::Add {
                name: "Ender Dragon".to_string(),
                progress: 0.75,
                color: bbb_protocol::packets::BossBarColor::Purple,
                overlay: bbb_protocol::packets::BossBarOverlay::Progress,
                flags: bbb_protocol::packets::BossEventFlags {
                    darken_screen: true,
                    play_music: false,
                    create_world_fog: true,
                },
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BossEvent(
        bbb_protocol::packets::BossEvent {
            id: boss_id,
            operation: bbb_protocol::packets::BossEventOperation::UpdateProgress { progress: 0.25 },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::BossEvent(
        bbb_protocol::packets::BossEvent {
            id: Uuid::from_u128(99),
            operation: bbb_protocol::packets::BossEventOperation::UpdateProgress { progress: 1.0 },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TabList(
        bbb_protocol::packets::TabList {
            header: Some("Welcome".to_string()),
            footer: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::ChangeDifficulty(
        bbb_protocol::packets::ChangeDifficulty {
            difficulty: bbb_protocol::packets::Difficulty::Hard,
            locked: true,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );

    let boss = world.boss_bars().get(&boss_id).unwrap();
    assert_eq!(boss.name, "Ender Dragon");
    assert_eq!(boss.progress, 0.25);
    assert_eq!(boss.color, "purple");
    assert_eq!(boss.overlay, "progress");
    assert!(boss.darken_screen);
    assert!(boss.create_world_fog);
    assert_eq!(world.tab_list().header.as_deref(), Some("Welcome"));
    assert_eq!(world.tab_list().footer, None);
    assert_eq!(world.difficulty().difficulty, "hard");
    assert!(world.difficulty().difficulty_locked);

    let world_counters = world.counters();
    assert_eq!(world_counters.boss_event_packets, 3);
    assert_eq!(world_counters.boss_bars_tracked, 1);
    assert_eq!(world_counters.boss_events_ignored, 1);
    assert_eq!(world_counters.tab_list_packets, 1);
    assert_eq!(world_counters.change_difficulty_packets, 1);
}

#[test]
fn player_info_events_update_world_and_world_counters() {
    let profile_id = Uuid::from_u128(1);
    let removed_profile_id = Uuid::from_u128(2);
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerInfoUpdate(
        bbb_protocol::packets::PlayerInfoUpdate {
            actions: vec![
                bbb_protocol::packets::PlayerInfoAction::AddPlayer,
                bbb_protocol::packets::PlayerInfoAction::InitializeChat,
                bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
                bbb_protocol::packets::PlayerInfoAction::UpdateListed,
                bbb_protocol::packets::PlayerInfoAction::UpdateLatency,
                bbb_protocol::packets::PlayerInfoAction::UpdateDisplayName,
                bbb_protocol::packets::PlayerInfoAction::UpdateListOrder,
                bbb_protocol::packets::PlayerInfoAction::UpdateHat,
            ],
            entries: vec![
                bbb_protocol::packets::PlayerInfoEntry {
                    profile_id,
                    profile: Some(bbb_protocol::packets::GameProfile {
                        uuid: profile_id,
                        name: "Ada".to_string(),
                        properties: vec![bbb_protocol::packets::GameProfileProperty {
                            name: "textures".to_string(),
                            value: "skin".to_string(),
                            signature: Some("signature".to_string()),
                        }],
                    }),
                    listed: true,
                    latency: 42,
                    game_mode: bbb_protocol::packets::GameType::Creative,
                    display_name: Some("Ada Lovelace".to_string()),
                    show_hat: true,
                    list_order: 3,
                    chat_session: Some(bbb_protocol::packets::PlayerInfoChatSession {
                        session_id: Uuid::from_u128(3),
                        expires_at_epoch_millis: 99,
                        public_key: vec![1, 2],
                        key_signature: vec![3, 4],
                    }),
                },
                bbb_protocol::packets::PlayerInfoEntry {
                    profile_id: removed_profile_id,
                    profile: Some(bbb_protocol::packets::GameProfile {
                        uuid: removed_profile_id,
                        name: "Removed".to_string(),
                        properties: Vec::new(),
                    }),
                    listed: true,
                    latency: 7,
                    game_mode: bbb_protocol::packets::GameType::Survival,
                    display_name: None,
                    show_hat: false,
                    list_order: 0,
                    chat_session: None,
                },
            ],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerInfoRemove(
        bbb_protocol::packets::PlayerInfoRemove {
            profile_ids: vec![removed_profile_id],
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    let entry = world.player_info_entry(profile_id).unwrap();
    assert_eq!(entry.profile.uuid, profile_id);
    assert_eq!(entry.profile.name, "Ada");
    assert_eq!(entry.profile.properties.len(), 1);
    assert!(entry.listed);
    assert_eq!(entry.latency, 42);
    assert_eq!(entry.game_mode, "creative");
    assert_eq!(entry.display_name.as_deref(), Some("Ada Lovelace"));
    assert!(entry.show_hat);
    assert_eq!(entry.list_order, 3);
    assert!(entry.chat_session_present);
    assert!(world.listed_players().contains(&profile_id));
    assert!(world.player_info_entry(removed_profile_id).is_none());
    assert!(!world.listed_players().contains(&removed_profile_id));

    let world_counters = world.counters();
    assert_eq!(world_counters.player_info_update_packets, 1);
    assert_eq!(world_counters.player_info_remove_packets, 1);
    assert_eq!(world_counters.player_info_entries_tracked, 1);
    assert_eq!(world_counters.listed_players_tracked, 1);
}

#[test]
fn server_presentation_events_update_world_and_world_counters() {
    let pack_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::ServerData(
        bbb_protocol::packets::ServerData {
            motd: "Native test server".to_string(),
            icon_bytes: Some(vec![1, 2, 3, 4]),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::ResourcePackPush(
        bbb_protocol::packets::ResourcePackPush {
            id: pack_id,
            url: "https://example.invalid/pack.zip".to_string(),
            hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
            required: true,
            prompt: Some("Install pack?".to_string()),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::ResourcePackResponse {
        id: pack_id,
        action: bbb_protocol::packets::ResourcePackResponseAction::Declined,
    })
    .unwrap();
    tx.try_send(NetEvent::ResourcePackPop(
        bbb_protocol::packets::ResourcePackPop { id: None },
    ))
    .unwrap();
    tx.try_send(NetEvent::ResourcePackPop(
        bbb_protocol::packets::ResourcePackPop { id: Some(pack_id) },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );

    let server_data = world.server_data().unwrap();
    assert_eq!(server_data.motd, "Native test server");
    assert_eq!(server_data.icon_byte_len(), Some(4));
    assert!(world.resource_packs().is_empty());

    let world_counters = world.counters();
    assert_eq!(world_counters.server_data_packets, 1);
    assert_eq!(world_counters.resource_pack_push_packets, 1);
    assert_eq!(world_counters.resource_pack_response_packets, 1);
    assert_eq!(world_counters.resource_pack_response_updates_applied, 1);
    assert_eq!(world_counters.resource_pack_response_updates_ignored, 0);
    assert_eq!(world_counters.resource_pack_required_declines, 1);
    assert_eq!(world_counters.resource_pack_pop_packets, 2);
    assert_eq!(world_counters.resource_pack_pop_updates_applied, 1);
    assert_eq!(world_counters.resource_pack_pop_updates_ignored, 1);
    assert_eq!(world_counters.resource_packs_tracked, 0);
}

#[test]
fn entity_status_events_update_world_and_world_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Play(PlayClientbound::Cooldown(
        bbb_protocol::packets::Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DamageEvent(
        bbb_protocol::packets::DamageEvent {
            entity_id,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: 42,
            source_position: Some(bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }),
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::DamageEvent(
        bbb_protocol::packets::DamageEvent {
            entity_id: 99,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::UpdateMobEffect(
        bbb_protocol::packets::UpdateMobEffect {
            entity_id,
            effect_id: 3,
            amplifier: 2,
            duration_ticks: 400,
            flags: bbb_protocol::packets::MobEffectFlags {
                raw: 0b1011,
                ambient: true,
                visible: true,
                show_icon: false,
                blend: true,
            },
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::RemoveMobEffect(
        bbb_protocol::packets::RemoveMobEffect {
            entity_id,
            effect_id: 99,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    let cooldown = world.cooldown("minecraft:ender_pearl").unwrap();
    assert_eq!(cooldown.duration, 20);

    let damage = world.entity_last_damage(entity_id).unwrap();
    assert_eq!(damage.source_type_id, 5);
    assert_eq!(damage.source_cause_id, -1);
    assert_eq!(damage.source_direct_id, 42);
    assert_eq!(
        damage.source_position,
        Some(bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        })
    );

    let effect = world.entity_effect(entity_id, 3).unwrap();
    assert_eq!(effect.amplifier, 2);
    assert_eq!(effect.duration_ticks, 400);
    assert!(effect.ambient);
    assert!(effect.visible);
    assert!(!effect.show_icon);
    assert!(effect.blend);
    assert!(world.entity_effect(entity_id, 99).is_none());

    let world_counters = world.counters();
    assert_eq!(world_counters.cooldown_packets, 1);
    assert_eq!(world_counters.cooldowns_tracked, 1);
    assert_eq!(world_counters.damage_event_packets, 2);
    assert_eq!(world_counters.damage_events_applied, 1);
    assert_eq!(world_counters.damage_events_ignored, 1);
    assert_eq!(world_counters.update_mob_effect_packets, 1);
    assert_eq!(world_counters.update_mob_effects_ignored, 0);
    assert_eq!(world_counters.remove_mob_effect_packets, 1);
    assert_eq!(world_counters.remove_mob_effects_ignored, 1);
    assert_eq!(world_counters.active_mob_effects_tracked, 1);
}

#[test]
fn move_vehicle_event_updates_world_and_queues_ack() {
    let (event_tx, mut event_rx) = mpsc::channel(1);
    let (command_tx, mut command_rx) = mpsc::channel(1);
    let commands = Some(command_tx);
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(99));
    world.apply_add_entity(protocol_add_entity(10));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));

    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveVehicle(
            bbb_protocol::packets::MoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            },
        )))
        .unwrap();

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &commands),
        1
    );

    assert_eq!(counters.move_vehicle_commands_queued, 1);
    assert_eq!(world.counters().vehicle_moves_received, 1);
    assert_eq!(world.counters().vehicle_moves_applied, 1);
    assert_eq!(world.counters().vehicle_moves_acked, 1);
    assert_eq!(world.counters().vehicle_moves_snapped, 1);
    assert_eq!(world.counters().vehicle_moves_ignored, 0);
    let vehicle = world.probe_entity(10).unwrap();
    assert_eq!(
        vehicle.position,
        bbb_world::EntityVec3 {
            x: 5.0,
            y: 66.0,
            z: -7.0,
        }
    );
    match command_rx.try_recv().unwrap() {
        NetCommand::MoveVehicle(command) => {
            assert_eq!(command.position.x, 5.0);
            assert_eq!(command.position.y, 66.0);
            assert_eq!(command.position.z, -7.0);
            assert_eq!(command.y_rot, 45.0);
            assert_eq!(command.x_rot, -5.0);
            assert!(!command.on_ground);
        }
        other => panic!("expected move vehicle command, got {other:?}"),
    }
}

#[test]
fn move_vehicle_ignored_counters_update_world_counters() {
    let (event_tx, mut event_rx) = mpsc::channel(1);

    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveVehicle(
            bbb_protocol::packets::MoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            },
        )))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.counters().vehicle_moves_received, 1);
    assert_eq!(world.counters().vehicle_moves_applied, 0);
    assert_eq!(world.counters().vehicle_moves_acked, 0);
    assert_eq!(world.counters().vehicle_moves_snapped, 0);
    assert_eq!(world.counters().vehicle_moves_ignored, 1);
}

#[test]
fn minecart_along_track_event_updates_world_state_and_world_counters() {
    let (event_tx, mut event_rx) = mpsc::channel(1);
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(10, 85));

    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 10,
                lerp_steps: vec![MinecartStep {
                    position: ProtocolVec3d {
                        x: 2.0,
                        y: 64.25,
                        z: -3.0,
                    },
                    movement: ProtocolVec3d {
                        x: 0.3,
                        y: 0.0,
                        z: -0.3,
                    },
                    y_rot: 90.0,
                    x_rot: 5.0,
                    weight: 1.0,
                }],
            },
        )))
        .unwrap();

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &None),
        1
    );

    let entity = world.probe_entity(10).unwrap();
    assert_eq!(
        entity.position,
        bbb_world::EntityVec3 {
            x: 2.0,
            y: 64.25,
            z: -3.0,
        }
    );
    assert_eq!(entity.y_rot, 90.0);
    assert_eq!(entity.x_rot, 5.0);
    assert_eq!(world.counters().minecart_moves_received, 1);
    assert_eq!(world.counters().minecart_moves_applied, 1);
    assert_eq!(world.counters().minecart_lerp_steps_received, 1);
    assert_eq!(world.counters().minecart_lerp_steps_tracked, 1);
}

#[test]
fn minecart_along_track_ignored_counters_update_world_counters() {
    let (event_tx, mut event_rx) = mpsc::channel(2);
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(20));

    let step = MinecartStep {
        position: ProtocolVec3d {
            x: 2.0,
            y: 64.25,
            z: -3.0,
        },
        movement: ProtocolVec3d {
            x: 0.3,
            y: 0.0,
            z: -0.3,
        },
        y_rot: 90.0,
        x_rot: 5.0,
        weight: 1.0,
    };

    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 999,
                lerp_steps: vec![step],
            },
        )))
        .unwrap();
    event_tx
        .try_send(NetEvent::Play(PlayClientbound::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: 20,
                lerp_steps: vec![step],
            },
        )))
        .unwrap();

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &None),
        2
    );

    assert_eq!(world.counters().minecart_moves_received, 2);
    assert_eq!(world.counters().minecart_moves_applied, 0);
    assert_eq!(world.counters().minecart_moves_ignored, 2);
    assert_eq!(world.counters().minecart_lerp_steps_received, 2);
    assert_eq!(world.counters().minecart_lerp_steps_tracked, 0);
}

#[test]
fn login_tracks_local_player_id_in_world() {
    let (tx, mut rx) = mpsc::channel(2);
    let respawn_info = protocol_play_login(9).common_spawn_info;
    tx.try_send(NetEvent::Play(PlayClientbound::Login(protocol_play_login(
        9,
    ))))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::Respawn(Respawn {
        common_spawn_info: respawn_info,
        data_to_keep: 0,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.local_player_id(), Some(9));
    assert_eq!(world.counters().play_logins_received, 1);
    assert_eq!(world.counters().respawns_received, 1);
}

#[test]
fn respawn_event_resets_local_player_runtime_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Play(PlayClientbound::Respawn(Respawn {
        common_spawn_info: protocol_play_login(9).common_spawn_info,
        data_to_keep: 0,
    })))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(9));
    world.apply_add_entity(protocol_add_entity(9));
    world.apply_add_entity(protocol_add_entity(55));
    world.apply_player_health(bbb_protocol::packets::PlayerHealth {
        health: 4.0,
        food: 7,
        saturation: 0.5,
    });
    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.25,
        level: 3,
        total: 40,
    });
    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 55 }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 10.0,
            y: 65.0,
            z: -4.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: -0.2,
            z: 0.3,
        },
        on_ground: true,
        horizontal_collision: true,
        fall_distance: 8.0,
        sneaking: true,
        swimming: true,
        y_rot: 90.0,
        x_rot: 20.0,
        last_teleport_id: 77,
    });
    world.set_local_destroying_block(BlockPos { x: 1, y: 2, z: 3 });
    world.set_local_using_item(true);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 9,
        values: vec![EntityDataValue {
            data_id: 0,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(0x02),
        }],
    }));
    assert!(world.apply_update_mob_effect(protocol_update_mob_effect(9, 3)));

    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.local_player_id(), Some(9));
    assert_eq!(world.counters().respawns_received, 1);
    assert!(world.local_player().health.is_none());
    assert!(world.local_player().experience.is_none());
    assert_eq!(world.local_player_pose(), None);
    assert_eq!(
        world.local_player().camera,
        bbb_world::CameraState::default()
    );
    assert_eq!(
        world.local_player().interaction,
        bbb_world::LocalPlayerInteractionState::default()
    );
    let entity = world.probe_entity(9).unwrap();
    assert!(entity.data_values.is_empty());
    assert!(entity.mob_effects.is_empty());
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
}

#[test]
fn player_position_and_rotation_events_update_world_pose() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerPosition(
        PlayerPositionUpdate {
            id: 23,
            position: ProtocolVec3d {
                x: 10.0,
                y: 64.0,
                z: -5.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.125,
                y: 0.0,
                z: 0.25,
            },
            y_rot: 90.0,
            x_rot: 15.0,
            relatives_mask: 0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerRotation(
        PlayerRotationUpdate {
            y_rot: 10.0,
            relative_y: true,
            x_rot: -5.0,
            relative_x: false,
        },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );

    let pose = world.local_player_pose().unwrap();
    assert_eq!(
        pose.position,
        ProtocolVec3d {
            x: 10.0,
            y: 64.0,
            z: -5.0,
        }
    );
    assert_eq!(
        pose.delta_movement,
        ProtocolVec3d {
            x: 0.125,
            y: 0.0,
            z: 0.25,
        }
    );
    assert_eq!(pose.y_rot, 100.0);
    assert_eq!(pose.x_rot, -5.0);
    assert_eq!(pose.last_teleport_id, 23);

    let world_counters = world.counters();
    assert_eq!(world_counters.player_position_packets, 1);
    assert_eq!(world_counters.player_rotation_packets, 1);
}

#[test]
fn local_player_events_update_world_state_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(10);
    tx.try_send(NetEvent::Play(PlayClientbound::Login(protocol_play_login(
        9,
    ))))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::PlayerAbilities(
        bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetHealth(
        bbb_protocol::packets::PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetExperience(
        bbb_protocol::packets::PlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetHeldSlot(
        bbb_protocol::packets::SetHeldSlot { slot: 5 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetDefaultSpawnPosition(
        bbb_protocol::packets::SetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetSimulationDistance(
        bbb_protocol::packets::SetSimulationDistance { distance: 12 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetCamera(
        bbb_protocol::packets::SetCamera { camera_id: 9 },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::SetCamera(
        bbb_protocol::packets::SetCamera { camera_id: 123 },
    )))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        9
    );

    let local = world.local_player();
    assert_eq!(local.health.unwrap().health, 7.5);
    assert_eq!(local.experience.unwrap().level, 8);
    assert_eq!(local.selected_hotbar_slot, 5);
    assert_eq!(local.simulation_distance, Some(12));
    assert_eq!(
        local.default_spawn.as_ref().map(|spawn| spawn.pos),
        Some(BlockPos {
            x: -5,
            y: 70,
            z: 12,
        })
    );
    assert_eq!(
        local.camera,
        bbb_world::CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );

    let world_counters = world.counters();
    assert_eq!(world_counters.player_abilities_packets, 1);
    assert_eq!(world_counters.player_health_packets, 1);
    assert_eq!(world_counters.player_experience_packets, 1);
    assert_eq!(world_counters.held_slot_packets, 1);
    assert_eq!(world_counters.held_slot_updates_applied, 1);
    assert_eq!(world_counters.held_slot_updates_ignored, 0);
    assert_eq!(world_counters.default_spawn_position_packets, 1);
    assert_eq!(world_counters.simulation_distance_packets, 1);
    assert_eq!(world_counters.set_camera_packets, 2);
    assert_eq!(world_counters.set_camera_updates_applied, 1);
    assert_eq!(world_counters.set_camera_updates_ignored, 1);
}

#[test]
fn world_time_and_weather_update_world_counters_and_clear_color() {
    let (tx, mut rx) = mpsc::channel(7);
    tx.try_send(NetEvent::Play(PlayClientbound::SetTime(
        bbb_protocol::packets::PlayTime {
            game_time: 123,
            clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.0,
                rate: 1.0,
            }],
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
        bbb_protocol::packets::GameEvent {
            event_id: 7,
            param: 0.5,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
        bbb_protocol::packets::GameEvent {
            event_id: 3,
            param: 3.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
        bbb_protocol::packets::GameEvent {
            event_id: 11,
            param: 1.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::GameEvent(
        bbb_protocol::packets::GameEvent {
            event_id: 12,
            param: 1.0,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TickingState(
        bbb_protocol::packets::TickingState {
            tick_rate: 0.25,
            frozen: true,
        },
    )))
    .unwrap();
    tx.try_send(NetEvent::Play(PlayClientbound::TickingStep(
        bbb_protocol::packets::TickingStep { tick_steps: 7 },
    )))
    .unwrap();

    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        7
    );

    assert_eq!(
        world.world_time(),
        Some(&bbb_world::WorldTimeState {
            game_time: 123,
            clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.0,
                rate: 1.0,
            }
            .into()],
            day_time: 6000,
        })
    );
    assert_eq!(world.weather().rain_level, 0.5);
    assert_eq!(world.gameplay().game_type, 3);
    assert_eq!(world.gameplay().game_type_name, "spectator");
    assert_eq!(world.gameplay().previous_game_type, Some(0));
    assert!(!world.gameplay().show_death_screen);
    assert!(world.gameplay().do_limited_crafting);
    assert_eq!(
        world.ticking(),
        bbb_world::WorldTickingState {
            tick_rate: 1.0,
            frozen: true,
            frozen_ticks_to_run: 7,
        }
    );

    assert_eq!(world.counters().world_time_packets, 1);
    assert_eq!(world.counters().game_event_packets, 4);
    assert_eq!(world.counters().ticking_state_packets, 1);
    assert_eq!(world.counters().ticking_step_packets, 1);

    let world_color = clear_color_for_world(&world, false);
    let expected_world_color = clear_color_for_day_time(6000, 0.5, 0.0);
    assert_eq!(world_color, expected_world_color);

    let day = clear_color_for_day_time(6000, 0.0, 0.0);
    let night = clear_color_for_day_time(18000, 0.0, 0.0);
    let storm = clear_color_for_day_time(6000, 1.0, 1.0);
    assert!(day.b > night.b);
    assert!(storm.r < day.r);
    assert!(storm.g < day.g);
    assert!(storm.b < day.b);
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn firework_item_entity_data(entity_id: i32, explosions_count: Option<usize>) -> SetEntityData {
    let mut stack = item_stack(901, 1);
    stack.component_patch = DataComponentPatchSummary {
        fireworks_flight_duration: Some(1),
        fireworks_explosions_count: explosions_count,
        ..DataComponentPatchSummary::default()
    };
    SetEntityData {
        id: entity_id,
        values: vec![EntityDataValue {
            data_id: 8,
            serializer_id: 7,
            value: EntityDataValueKind::ItemStack(stack),
        }],
    }
}

fn written_book_stack(pages: Vec<&str>) -> ItemStackSummary {
    let mut stack = item_stack(42, 1);
    let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
    let page_filters = vec![None; pages.len()];
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".to_string(),
        title_filter: None,
        author: "Alex".to_string(),
        generation: 0,
        pages,
        page_filters,
        resolved: true,
    });
    stack
}

fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
    ItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

fn recipe_book_entry(id: i32, notification: bool, highlight: bool) -> RecipeBookAddEntry {
    RecipeBookAddEntry {
        contents: RecipeDisplayEntry {
            id: RecipeDisplayId { index: id },
            display: RecipeDisplaySummary {
                display_type: RecipeDisplayType::Stonecutter,
                raw_body: vec![3, 0, 0, 0],
                crafting: None,
            },
            group: None,
            category_id: 10,
            crafting_requirements: Some(vec![IngredientSummary {
                tag: None,
                item_ids: vec![42],
            }]),
        },
        flags: (u8::from(notification)) | (u8::from(highlight) << 1),
        notification,
        highlight,
    }
}

fn protocol_play_login(player_id: i32) -> PlayLogin {
    PlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    }
}

fn command_tree_packet(literal: &str) -> bbb_protocol::packets::Commands {
    bbb_protocol::packets::Commands {
        root_index: 0,
        nodes: vec![
            bbb_protocol::packets::CommandNode {
                node_type: bbb_protocol::packets::CommandNodeType::Root,
                flags: 0,
                children: vec![1],
                redirect: None,
                name: None,
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            bbb_protocol::packets::CommandNode {
                node_type: bbb_protocol::packets::CommandNodeType::Literal,
                flags: 1,
                children: vec![2],
                redirect: None,
                name: Some(literal.to_string()),
                parser: None,
                suggestions: None,
                executable: false,
                restricted: false,
            },
            bbb_protocol::packets::CommandNode {
                node_type: bbb_protocol::packets::CommandNodeType::Argument,
                flags: 54,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(bbb_protocol::packets::CommandArgumentParser {
                    type_id: 5,
                    name: "brigadier:string".to_string(),
                    properties: vec![2],
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: true,
            },
        ],
    }
}

fn protocol_chat_type(name: &str) -> ChatTypeBound {
    ChatTypeBound {
        chat_type: ChatTypeHolder::Registry { id: 0 },
        name: name.to_string(),
        target_name: None,
    }
}

fn player_chat_with_signature(global_index: i32, signature: MessageSignature) -> PlayerChat {
    PlayerChat {
        global_index,
        sender: Uuid::from_u128(0x1234),
        index: global_index,
        signature: Some(signature),
        body: SignedMessageBody {
            content: format!("message {global_index}"),
            timestamp_millis: i64::from(global_index),
            salt: i64::from(global_index) + 1,
            last_seen: Vec::new(),
        },
        unsigned_content: None,
        filter_mask: FilterMask {
            kind: FilterMaskKind::PassThrough,
            mask_words: Vec::new(),
        },
        chat_type: protocol_chat_type("Alice"),
    }
}

struct RecordingAudioSink {
    catalog: SoundCatalog,
    registry: SoundEventRegistry,
    jukebox_registry: JukeboxSongRegistry,
    commands: Vec<AudioCommand>,
    errors: Vec<String>,
}

impl RecordingAudioSink {
    fn new(catalog: SoundCatalog, registry: SoundEventRegistry) -> Self {
        Self {
            catalog,
            registry,
            jukebox_registry: JukeboxSongRegistry::vanilla_26_1(),
            commands: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn record(&mut self, command: std::result::Result<AudioCommand, AudioResolveError>) {
        match command {
            Ok(command) => self.commands.push(command),
            Err(err) => self.errors.push(err.to_string()),
        }
    }
}

impl crate::audio_runtime::AudioEventSink for RecordingAudioSink {
    fn counters(&self) -> AudioCounters {
        AudioCounters {
            enabled: true,
            catalog_events: self.catalog.len(),
            registry_entries: self.registry.len(),
            commands_submitted: self.commands.len() as u64,
            resolve_failures: self.errors.len() as u64,
            last_resolve_error: self.errors.last().cloned(),
            ..AudioCounters::default()
        }
    }

    fn set_sound_event_registry(&mut self, registry: SoundEventRegistry) {
        self.registry = registry;
    }

    fn set_jukebox_song_registry(&mut self, registry: JukeboxSongRegistry) {
        self.jukebox_registry = registry;
    }

    fn play_local_sound(&mut self, state: &bbb_world::LocalSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_local_sound(state)
        };
        self.record(command);
    }

    fn play_positioned_sound(&mut self, state: &bbb_world::SoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_positioned_sound(state)
        };
        self.record(command);
    }

    fn play_entity_sound(
        &mut self,
        state: &bbb_world::SoundEntityEventState,
        position: Option<[f64; 3]>,
    ) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_entity_sound_at(state, position)
        };
        self.record(command);
    }

    fn play_jukebox_song(&mut self, state: &bbb_world::JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_jukebox_song(state)
        };
        self.record(command);
    }

    fn stop_jukebox_song(&mut self, state: &bbb_world::JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.stop_jukebox_song(state)
        };
        self.commands.push(command);
    }

    fn stop_sound(&mut self, state: &bbb_world::StopSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.stop_sound(state)
        };
        self.commands.push(command);
    }

    fn tick_entity_sound_positions(&mut self, command: bbb_audio::TickEntitySoundPositionsCommand) {
        self.commands
            .push(AudioCommand::TickEntitySoundPositions(command));
    }
}

#[derive(Default)]
struct RecordingParticleSink {
    packets: Vec<LevelParticles>,
    contexts: Vec<LevelParticleSpawnContext>,
    level_events: Vec<LevelEvent>,
    level_event_contexts: Vec<LevelEventParticleContext>,
    firework_empty_explosion_positions: Vec<[f64; 3]>,
    firework_empty_explosion_camera_positions: Vec<Option<[f64; 3]>>,
    tracking_emitter_states: Vec<crate::particle_runtime::TrackingEmitterParticleState>,
    take_item_entity_pickup_states: Vec<TakeItemEntityPickupParticleState>,
    ravager_roar_states: Vec<RavagerRoarParticleState>,
    batches: Vec<bbb_renderer::ParticleSpawnBatch>,
}

impl ParticleEventSink for RecordingParticleSink {
    fn spawn_level_particles(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
        _biome_sampler: Option<&dyn crate::particle_runtime::ParticleBiomeSampler>,
        _item_runtime: Option<&bbb_item_model::NativeItemRuntime>,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.packets.push(packet.clone());
        self.contexts.push(context);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_definition_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_level_event_particles(
        &mut self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        if event.event_type == 3018 {
            advance_cobweb_place_particle_randoms(random);
        } else if matches!(event.event_type, 2002 | 2007) {
            advance_potion_break_level_event_particle_randoms(random);
        } else if event.event_type == 2006 {
            advance_dragon_fireball_explode_level_event_particle_randoms(random);
        } else if event.event_type == 3003 {
            advance_wax_on_level_event_particle_randoms(random);
        } else if event.event_type == 1505 {
            advance_growth_randoms_for_context(event, &context, random);
        } else if event.event_type == 3015 && context.vault_block_entity_at_event_pos {
            bbb_world::advance_vault_activation_particle_randoms_with_connections(
                random,
                context
                    .vault_connection_particles
                    .as_ref()
                    .map(|state| state.targets.len())
                    .unwrap_or(0),
            );
        } else if event.event_type == 3016 {
            bbb_world::advance_vault_deactivation_particle_randoms(random);
        }
        self.level_events.push(*event);
        self.level_event_contexts.push(context);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_sprite_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_firework_empty_explosion_particles(
        &mut self,
        position: [f64; 3],
        camera_position: Option<[f64; 3]>,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.firework_empty_explosion_positions.push(position);
        self.firework_empty_explosion_camera_positions
            .push(camera_position);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_sprite_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_tracking_emitter_particles(
        &mut self,
        state: crate::particle_runtime::TrackingEmitterParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.tracking_emitter_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_sprite_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_take_item_entity_pickup_particles(
        &mut self,
        state: &TakeItemEntityPickupParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.take_item_entity_pickup_states.push(state.clone());
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }

    fn spawn_ravager_roar_particles(
        &mut self,
        state: RavagerRoarParticleState,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.ravager_roar_states.push(state);
        let batch = bbb_renderer::ParticleSpawnBatch::default();
        self.batches.push(batch.clone());
        batch
    }
}

fn test_sound_catalog() -> SoundCatalog {
    let assets_dir = std::env::temp_dir().join("bbb-native-audio-test-assets");
    SoundCatalog::from_json_bytes(
        "minecraft",
        &assets_dir,
        br#"{
            "ambient.cave": {
                "sounds": ["ambient/cave/cave1"]
            },
            "entity.cat.ambient": {
                "sounds": ["mob/cat/meow1"]
            },
            "block.grass.break": {
                "sounds": ["dig/grass1"]
            },
            "item.bone_meal.use": {
                "sounds": ["item/bone_meal/use"]
            },
            "item.totem.use": {
                "sounds": ["item/totem/use"]
            },
            "entity.item.pickup": {
                "sounds": ["random/pop"]
            },
            "entity.experience_orb.pickup": {
                "sounds": ["random/orb"]
            },
            "entity.arrow.hit_player": {
                "sounds": ["random/bowhit"]
            },
            "entity.puffer_fish.sting": {
                "sounds": ["mob/puffer_fish/sting"]
            },
            "entity.elder_guardian.curse": {
                "sounds": ["mob/elderguardian/curse"]
            },
            "entity.firework_rocket.shoot": {
                "sounds": ["fireworks/launch1"]
            },
            "block.lava.extinguish": {
                "sounds": ["random/fizz"]
            },
            "block.redstone_torch.burnout": {
                "sounds": ["random/fizz"]
            },
            "block.end_portal_frame.fill": {
                "sounds": ["block/end_portal_frame/fill"]
            },
            "entity.splash_potion.break": {
                "sounds": ["random/glass"]
            },
            "entity.dragon_fireball.explode": {
                "sounds": ["mob/enderdragon/fireball"]
            },
            "entity.ghast.warn": {
                "sounds": ["mob/ghast/affectionate_scream"]
            },
            "block.portal.travel": {
                "sounds": ["portal/travel"]
            },
            "entity.wither.spawn": {
                "sounds": ["mob/wither/spawn"]
            },
            "entity.ender_dragon.death": {
                "sounds": ["mob/enderdragon/end"]
            },
            "entity.ender_dragon.growl": {
                "sounds": ["mob/enderdragon/growl"]
            },
            "block.end_gateway.spawn": {
                "sounds": ["block/end_gateway/spawn"]
            },
            "block.trial_spawner.spawn_mob": {
                "sounds": ["block/trial_spawner/spawn_mob"]
            },
            "block.trial_spawner.detect_player": {
                "sounds": ["block/trial_spawner/detect_player"]
            },
            "block.trial_spawner.eject_item": {
                "sounds": ["block/trial_spawner/eject_item"]
            },
            "block.trial_spawner.ominous_activate": {
                "sounds": ["block/trial_spawner/ominous_activate"]
            },
            "block.trial_spawner.spawn_item": {
                "sounds": ["block/trial_spawner/spawn_item"]
            },
            "item.honeycomb.wax_on": {
                "sounds": ["item/honeycomb/wax_on"]
            },
            "item.brush.brushing.sand": {
                "sounds": ["item/brush/brushing/sand"]
            },
            "item.brush.brushing.gravel": {
                "sounds": ["item/brush/brushing/gravel"]
            },
            "block.sculk.charge": {
                "sounds": ["block/sculk/charge"]
            },
            "block.sculk_shrieker.shriek": {
                "sounds": ["block/sculk_shrieker/shriek"]
            },
            "block.cobweb.place": {
                "sounds": ["block/cobweb/place"]
            },
            "block.vault.activate": {
                "sounds": ["block/vault/activate"]
            },
            "block.vault.deactivate": {
                "sounds": ["block/vault/deactivate"]
            },
            "block.end_portal.spawn": {
                "sounds": ["portal/endportal"]
            },
            "music_disc.cat": {
                "sounds": ["records/cat"]
            },
            "music_disc.tears": {
                "sounds": ["records/tears"]
            }
        }"#,
    )
    .unwrap()
}

fn protocol_add_entity(id: i32) -> AddEntity {
    protocol_add_entity_with_type(id, 7)
}

fn protocol_update_mob_effect(
    entity_id: i32,
    effect_id: i32,
) -> bbb_protocol::packets::UpdateMobEffect {
    bbb_protocol::packets::UpdateMobEffect {
        entity_id,
        effect_id,
        amplifier: 2,
        duration_ticks: 400,
        flags: bbb_protocol::packets::MobEffectFlags {
            raw: 0b1011,
            ambient: true,
            visible: true,
            show_icon: false,
            blend: true,
        },
    }
}

fn level_event_at(event_type: i32, x: i32, y: i32, z: i32) -> LevelEvent {
    level_event_at_with_data(event_type, x, y, z, 0)
}

fn level_event_at_with_data(event_type: i32, x: i32, y: i32, z: i32, data: i32) -> LevelEvent {
    LevelEvent {
        event_type,
        pos: block_pos(x, y, z),
        data,
        global: false,
    }
}

fn set_test_block(world: &mut WorldStore, pos: ProtocolBlockPos, block_state_id: i32) {
    assert!(world.apply_block_update(BlockUpdate {
        pos,
        block_state_id,
    }));
}

fn block_pos(x: i32, y: i32, z: i32) -> ProtocolBlockPos {
    ProtocolBlockPos { x, y, z }
}

fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
    let properties = BTreeMap::from(props.map(|(key, value)| (key.to_string(), value.to_string())));
    bbb_world::RegistrySet::vanilla_26_1()
        .block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("missing vanilla block state {name} {properties:?}"))
}

fn synthetic_native_level_chunk_packet() -> LevelChunkWithLight {
    let mut sections = Encoder::new();
    sections.write_i16(0);
    sections.write_i16(0);
    sections.write_u8(0);
    sections.write_var_i32(0);
    sections.write_u8(0);
    sections.write_var_i32(0);

    LevelChunkWithLight {
        x: 1,
        z: -2,
        chunk_data: LevelChunkData {
            heightmaps: vec![ChunkHeightmapData {
                kind_id: 1,
                data: vec![42],
            }],
            section_data: sections.into_inner(),
            block_entities: vec![LevelChunkBlockEntity {
                packed_xz: 0,
                y: -64,
                block_entity_type_id: 7,
                raw_nbt: vec![0],
            }],
        },
        light_data: empty_light_update_data(),
    }
}

fn empty_light_update_data() -> LightUpdateData {
    LightUpdateData {
        sky_y_mask: Vec::new(),
        block_y_mask: Vec::new(),
        empty_sky_y_mask: Vec::new(),
        empty_block_y_mask: Vec::new(),
        sky_updates: Vec::new(),
        block_updates: Vec::new(),
    }
}

fn single_biome_payload(biome_id: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_u8(0);
    payload.write_var_i32(biome_id);
    payload.into_inner()
}

fn sign_text_nbt(front: [&str; 4], back: [&str; 4]) -> Vec<u8> {
    let mut payload = vec![10];
    write_sign_text_side(&mut payload, "front_text", front);
    write_sign_text_side(&mut payload, "back_text", back);
    payload.push(0);
    payload
}

fn vault_shared_data_nbt(players: &[Uuid], connected_particles_range: Option<f64>) -> Vec<u8> {
    let mut payload = vec![10, 10];
    write_nbt_string(&mut payload, "shared_data");
    if !players.is_empty() {
        payload.push(9);
        write_nbt_string(&mut payload, "connected_players");
        payload.push(11);
        payload.extend_from_slice(&(players.len() as i32).to_be_bytes());
        for player in players {
            write_nbt_uuid_int_array(&mut payload, *player);
        }
    }
    if let Some(range) = connected_particles_range {
        payload.push(6);
        write_nbt_string(&mut payload, "connected_particles_range");
        payload.extend_from_slice(&range.to_be_bytes());
    }
    payload.push(0);
    payload.push(0);
    payload
}

fn write_sign_text_side(out: &mut Vec<u8>, name: &str, lines: [&str; 4]) {
    out.push(10);
    write_nbt_string(out, name);
    out.push(9);
    write_nbt_string(out, "messages");
    out.push(8);
    out.extend_from_slice(&4i32.to_be_bytes());
    for line in lines {
        write_nbt_string(out, line);
    }
    out.push(0);
}

fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn write_nbt_uuid_int_array(out: &mut Vec<u8>, uuid: Uuid) {
    let value = uuid.as_u128();
    let ints = [
        (value >> 96) as u32,
        (value >> 64) as u32,
        (value >> 32) as u32,
        value as u32,
    ];
    out.extend_from_slice(&4i32.to_be_bytes());
    for value in ints {
        out.extend_from_slice(&(value as i32).to_be_bytes());
    }
}

fn vault_block_state_id(facing: &str) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("ominous".to_string(), "false".to_string()),
        ("vault_state".to_string(), "active".to_string()),
    ]);
    bbb_world::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:vault", &properties)
        .unwrap()
        .id
}

fn vault_test_player(id: i32, uuid: Uuid, position: ProtocolVec3d) -> AddEntity {
    AddEntity {
        id,
        uuid,
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position,
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn assert_close64(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "expected {expected}, got {actual}"
    );
}

fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id,
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data: 99,
    }
}
