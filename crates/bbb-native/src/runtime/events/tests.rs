use super::*;
use crate::particle_runtime::ParticleEventSink;
use crate::runtime::{clear_color_for_day_time, clear_color_for_world};
use bbb_audio::{
    AudioCategory, AudioCommand, AudioCommandResolver, AudioResolveError, SoundEventRegistry,
};
use bbb_net::{NetCommand, NetEvent};
use bbb_pack::SoundCatalog;
use bbb_protocol::codec::Encoder;
use bbb_protocol::packets::{
    AddEntity, AdvancementCriterionProgressSummary, AdvancementProgressSummary, AdvancementSummary,
    AttributeSnapshot, AwardStats, BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate,
    ChatTypeBound, ChatTypeHolder, ChunkBiomeData, ChunkHeightmapData,
    ChunkPos as ProtocolChunkPos, ChunksBiomes, CommonPlayerSpawnInfo, ContainerClose,
    ContainerSetContent, ContainerSetData, ContainerSetSlot, CustomChatCompletions,
    CustomChatCompletionsAction, CustomPayload, CustomPayloadBody, DebugBlockValue,
    DebugChunkValue, DebugEntityValue, DebugEvent, DebugSample, DeleteChat, DialogHolder,
    DisguisedChat, EntityAnchor, EntityAnimation, EntityDataValue, EntityDataValueKind,
    EntityEvent, EntityMove, EntityPositionSync, EquipmentSlot, EquipmentSlotUpdate, Explosion,
    FilterMask, FilterMaskKind, ForgetLevelChunk, GameRuleValue, GameRuleValues,
    GameTestHighlightPos, HurtAnimation, IngredientSummary, InteractionHand, ItemCostSummary,
    ItemStackSummary, LevelChunkBlockEntity, LevelChunkData, LevelChunkWithLight, LevelParticles,
    LightUpdate, LightUpdateData, MapColorPatch, MapDecoration, MapItemData, MerchantOffer,
    MerchantOffers, MessageSignature, MinecartStep, MountScreenOpen, MoveMinecartAlongTrack,
    OpenBook, OpenScreen, OpenSignEditor, PackedMessageSignature, ParticlePayload,
    PlaceGhostRecipe, PlayLogin, PlayerChat, PlayerCombatEnd, PlayerCombatKill, PlayerLookAt,
    PlayerLookAtTarget, PongResponse, ProjectilePower, RecipeBookAdd, RecipeBookAddEntry,
    RecipeBookRemove, RecipeBookSettings, RecipeBookTypeSettings, RecipeDisplayEntry,
    RecipeDisplayId, RecipeDisplaySummary, RecipeDisplayType, RecipePropertySetSummary,
    RegistryData, RegistryDataEntry, RegistryTags, RemoteDebugSampleType, RemoveEntities, Respawn,
    RotateHead, SectionBlocksUpdate, SelectAdvancementsTab, ServerLinkEntry, ServerLinkKnownType,
    ServerLinkType, ServerLinks, SetChunkCacheCenter, SetChunkCacheRadius, SetCursorItem,
    SetEntityData, SetEntityLink, SetEntityMotion, SetEquipment, SetPassengers, SetPlayerInventory,
    ShowDialog, SignedMessageBody, SlotDisplaySummary, SoundEntityEvent, SoundEvent,
    SoundEventHolder, SoundSource, StatUpdate, StonecutterSelectableRecipeSummary, StopSound,
    TagNetworkPayload, TagQuery, TeleportEntity, TestInstanceBlockStatus, TrackedWaypoint,
    TrackedWaypointPacket, UpdateAdvancements, UpdateAttributes, UpdateRecipes, UpdateTags,
    Vec3d as ProtocolVec3d, Vec3i as ProtocolVec3i, WaypointData, WaypointIcon, WaypointIdentifier,
    WaypointOperation, WaypointVec3i,
};
use bbb_world::{BlockPos, ChunkPos, RegistryPacketEntry, WorldStore};
use std::collections::BTreeMap;
use tokio::sync::mpsc;
use uuid::Uuid;

#[test]
fn block_changed_ack_updates_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::BlockChangedAck(
        bbb_protocol::packets::BlockChangedAck { sequence: 17 },
    ))
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
    assert_eq!(counters.block_changed_ack_packets, 1);
    assert_eq!(counters.last_block_changed_ack_sequence, Some(17));
}

#[test]
fn chunk_cache_events_update_world_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::SetChunkCacheCenter(SetChunkCacheCenter {
        chunk_x: -4,
        chunk_z: 7,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetChunkCacheRadius(SetChunkCacheRadius {
        radius: 10,
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.chunk_cache_center(), Some(ChunkPos { x: -4, z: 7 }));
    assert_eq!(world.chunk_cache_radius(), Some(10));
    assert_eq!(counters.chunk_cache_center, Some(ChunkPos { x: -4, z: 7 }));
    assert_eq!(counters.chunk_cache_radius, Some(10));
    let world_counters = world.counters();
    assert_eq!(world_counters.chunk_cache_center_updates_received, 1);
    assert_eq!(world_counters.chunk_cache_radius_updates_received, 1);
    assert_eq!(counters.chunk_cache_center_updates_received, 1);
    assert_eq!(counters.chunk_cache_radius_updates_received, 1);
}

#[test]
fn terrain_chunk_events_update_world_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(7);
    tx.try_send(NetEvent::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    ))
    .unwrap();
    tx.try_send(NetEvent::BlockUpdate(BlockUpdate {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_state_id: 5,
    }))
    .unwrap();
    tx.try_send(NetEvent::SectionBlocksUpdate(SectionBlocksUpdate {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::BlockEntityData(BlockEntityData {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_entity_type_id: 7,
        raw_nbt: vec![0],
    }))
    .unwrap();
    tx.try_send(NetEvent::LightUpdate(LightUpdate {
        chunk_x: 1,
        chunk_z: -2,
        light_data: empty_light_update_data(),
    }))
    .unwrap();
    tx.try_send(NetEvent::ChunksBiomes(ChunksBiomes {
        chunks: vec![ChunkBiomeData {
            pos: ProtocolChunkPos { x: 1, z: -2 },
            raw_biomes: single_biome_payload(7),
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::ForgetLevelChunk(ForgetLevelChunk {
        pos: ProtocolChunkPos { x: 1, z: -2 },
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        7
    );
    assert_eq!(world.first_chunk(), Some(ChunkPos { x: 1, z: -2 }));
    assert_eq!(counters.first_chunk, Some(ChunkPos { x: 1, z: -2 }));
    assert!(world.probe_chunk(ChunkPos { x: 1, z: -2 }).is_none());

    let world_counters = world.counters();
    macro_rules! assert_chunk_counter {
        ($field:ident, $value:expr) => {
            assert_eq!(world_counters.$field, $value);
            assert_eq!(counters.$field, $value);
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
fn terrain_chunk_ignored_counters_are_projected() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::BlockUpdate(BlockUpdate {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_state_id: 5,
    }))
    .unwrap();
    tx.try_send(NetEvent::SectionBlocksUpdate(SectionBlocksUpdate {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::BlockEntityData(BlockEntityData {
        pos: ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        },
        block_entity_type_id: 7,
        raw_nbt: vec![0],
    }))
    .unwrap();
    tx.try_send(NetEvent::LightUpdate(LightUpdate {
        chunk_x: 1,
        chunk_z: -2,
        light_data: empty_light_update_data(),
    }))
    .unwrap();
    tx.try_send(NetEvent::ChunksBiomes(ChunksBiomes {
        chunks: vec![ChunkBiomeData {
            pos: ProtocolChunkPos { x: 1, z: -2 },
            raw_biomes: Vec::new(),
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::ForgetLevelChunk(ForgetLevelChunk {
        pos: ProtocolChunkPos { x: 1, z: -2 },
    }))
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
            assert_eq!(counters.$field, $value);
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
    assert_chunk_counter!(chunk_forgets_received, 1);
    assert_chunk_counter!(chunks_forgotten, 0);
    assert_chunk_counter!(chunk_forgets_ignored, 1);
}

#[test]
fn respawn_clears_projected_first_chunk_when_world_changes() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::LevelChunkWithLight(
        synthetic_native_level_chunk_packet(),
    ))
    .unwrap();

    let mut spawn_info = protocol_play_login(9).common_spawn_info;
    spawn_info.dimension_type_id = 1;
    spawn_info.dimension = "minecraft:the_nether".to_string();
    tx.try_send(NetEvent::Respawn(Respawn {
        common_spawn_info: spawn_info,
        data_to_keep: 0,
    }))
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
    let mut counters = NetCounters {
        entities_tracked: 99,
        active_mob_effects_tracked: 99,
        block_destructions_tracked: 99,
        block_events_tracked: 99,
        level_events_tracked: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(world.first_chunk(), None);
    assert_eq!(counters.first_chunk, None);
    assert_eq!(counters.respawns_received, 1);
    assert_eq!(counters.chunks_received, 1);
    assert_eq!(counters.chunks_decoded, 1);
    assert_eq!(world.counters().entities_tracked, 0);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
    assert_eq!(world.counters().block_destructions_tracked, 0);
    assert_eq!(world.counters().block_events_tracked, 0);
    assert_eq!(world.counters().level_events_tracked, 0);
    assert_eq!(counters.entities_tracked, 0);
    assert_eq!(counters.active_mob_effects_tracked, 0);
    assert_eq!(counters.block_destructions_tracked, 0);
    assert_eq!(counters.block_events_tracked, 0);
    assert_eq!(counters.level_events_tracked, 0);
}

#[test]
fn transfer_event_updates_world_and_snapshot_counters() {
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
        counters.last_transfer,
        Some(bbb_control::TransferTarget {
            host: "next.example.com".to_string(),
            port: 25566,
        })
    );
    assert_eq!(counters.transfer_packets, 1);
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
fn cookie_events_update_snapshot_counters() {
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
    let mut counters = NetCounters {
        last_cookie_key: Some("stale:key".to_string()),
        cookie_request_packets: 99,
        cookie_response_hits: 99,
        cookie_response_misses: 99,
        store_cookie_packets: 99,
        stored_cookie_count: 99,
        stored_cookie_bytes: 99,
        ..NetCounters::default()
    };

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
    assert_eq!(counters.last_cookie_key.as_deref(), Some("bbb:missing"));
    assert_eq!(counters.store_cookie_packets, 1);
    assert_eq!(counters.stored_cookie_count, 1);
    assert_eq!(counters.stored_cookie_bytes, 3);
    assert_eq!(counters.cookie_request_packets, 2);
    assert_eq!(counters.cookie_response_hits, 1);
    assert_eq!(counters.cookie_response_misses, 1);
}

#[test]
fn custom_report_details_event_updates_snapshot_counters() {
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
    assert_eq!(counters.custom_report_details, details);
    assert_eq!(counters.custom_report_detail_packets, 1);
    assert_eq!(counters.custom_report_details_tracked, 2);
    assert_eq!(world.custom_report_details(), &details);
    assert_eq!(world.counters().custom_report_detail_packets, 1);
    assert_eq!(world.counters().custom_report_details_tracked, 2);
}

#[test]
fn award_stats_event_updates_world_state_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::AwardStats(AwardStats {
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
    }))
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

    assert_eq!(counters.award_stats_packets, 1);
    assert_eq!(counters.award_stats_entries_received, 2);
    assert_eq!(counters.last_award_stats_entry_count, 2);
    assert_eq!(counters.stats_tracked, 2);
    assert_eq!(
        counters.last_award_stats,
        Some(bbb_control::AwardStatsState {
            entries: vec![
                bbb_control::StatValueState {
                    stat_type_id: 8,
                    value_id: 10,
                    amount: 3,
                },
                bbb_control::StatValueState {
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
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::UpdateEnabledFeatures(
        bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec![
                "minecraft:minecart_improvements".to_string(),
                "minecraft:vanilla".to_string(),
            ],
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::ResetChat).unwrap();
    tx.try_send(NetEvent::CodeOfConduct {
        text: "Keep the server friendly.".to_string(),
    })
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_player_chat(PlayerChat {
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
    let mut counters = NetCounters {
        last_player_chat: Some(bbb_control::ClientChatLine {
            content: "previous".to_string(),
            ..bbb_control::ClientChatLine::default()
        }),
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(counters.update_enabled_features_packets, 1);
    assert_eq!(
        counters.enabled_features,
        vec![
            "minecraft:minecart_improvements".to_string(),
            "minecraft:vanilla".to_string(),
        ]
    );
    assert_eq!(world.enabled_feature_list(), counters.enabled_features);
    assert!(world.is_feature_enabled("minecraft:vanilla"));
    assert_eq!(world.counters().update_enabled_features_packets, 1);
    assert_eq!(world.counters().enabled_features_tracked, 2);
    assert_eq!(counters.enabled_features_tracked, 2);
    assert_eq!(counters.enabled_features_ignored, 0);
    assert_eq!(counters.reset_chat_packets, 1);
    assert!(counters.last_player_chat.is_none());
    assert!(world.client_chat().messages.is_empty());
    assert!(world.client_chat().deleted_messages.is_empty());
    assert_eq!(world.client_chat().expected_player_chat_global_index, 0);
    assert_eq!(world.counters().reset_chat_packets, 1);
    assert_eq!(world.counters().chat_messages_tracked, 0);
    assert_eq!(world.counters().deleted_chat_messages_tracked, 0);
    assert_eq!(world.counters().chat_signature_cache_entries, 0);
    assert_eq!(counters.code_of_conduct_packets, 1);
    assert_eq!(
        counters.last_code_of_conduct_len,
        "Keep the server friendly.".len()
    );
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
        counters.last_code_of_conduct_len
    );
}

#[test]
fn registry_data_event_updates_world_state_and_snapshot_counters() {
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
    assert_eq!(counters.registries_seen, 1);
    assert_eq!(counters.registry_entries_seen, 2);
    assert_eq!(counters.registry_entries_with_data, 1);
    assert_eq!(counters.registry_entry_stubs, 1);
    assert_eq!(counters.registry_entry_payload_bytes, 24);
    assert_eq!(counters.registry_content_registries_tracked, 1);
    assert_eq!(counters.registry_content_packets_tracked, 1);
    assert_eq!(counters.registry_content_entries_tracked, 2);
    assert_eq!(counters.registry_duplicate_entries, 0);
    assert_eq!(counters.registry_duplicate_entry_ids_tracked, 0);
    assert_eq!(
        counters.last_registry_data_registry.as_deref(),
        Some("minecraft:chat_type")
    );
    assert_eq!(counters.last_registry_data_entry_count, 2);
}

#[test]
fn update_tags_event_updates_world_state_and_snapshot_counters() {
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
    assert_eq!(counters.update_tags_packets, 1);
    assert_eq!(counters.last_update_tags_registry_count, 1);
    assert_eq!(counters.last_update_tags_total_tag_count, 1);
    assert_eq!(counters.last_update_tags_total_value_count, 3);
    assert_eq!(counters.tag_registries_tracked, 1);
    assert_eq!(counters.tags_tracked, 1);
    assert_eq!(counters.tag_entries_tracked, 3);
}

#[test]
fn server_links_event_updates_world_and_snapshot_counters() {
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
    assert_eq!(counters.server_link_packets, 1);
    assert_eq!(counters.server_link_invalid_entries, 1);
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
    assert_eq!(
        counters.server_links,
        vec![
            bbb_control::ServerLinkState {
                label: "known_server_link.support".to_string(),
                url: "https://example.invalid/support".to_string(),
                known_type: Some("support".to_string()),
            },
            bbb_control::ServerLinkState {
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
    assert_eq!(counters.server_links_tracked, 2);
}

#[test]
fn client_ui_events_update_world_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::LowDiskSpaceWarning).unwrap();
    tx.try_send(NetEvent::MountScreenOpen(MountScreenOpen {
        container_id: 11,
        inventory_columns: 5,
        entity_id: 42,
    }))
    .unwrap();
    tx.try_send(NetEvent::OpenBook(OpenBook {
        hand: InteractionHand::OffHand,
    }))
    .unwrap();
    tx.try_send(NetEvent::OpenSignEditor(OpenSignEditor {
        pos: ProtocolBlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        is_front_text: false,
    }))
    .unwrap();
    tx.try_send(NetEvent::PongResponse(PongResponse { time: 123456789 }))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(counters.low_disk_space_warnings, 1);
    assert_eq!(world.low_disk_space_warning_count(), 1);
    assert_eq!(
        world.last_mount_screen(),
        Some(&bbb_world::MountScreenState {
            container_id: 11,
            inventory_columns: 5,
            entity_id: 42,
        })
    );
    assert_eq!(counters.mount_screen_open_packets, 1);
    assert_eq!(
        world.last_open_book(),
        Some(&bbb_world::OpenBookState {
            hand: "off_hand".to_string(),
        })
    );
    assert_eq!(counters.open_book_packets, 1);
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
    assert_eq!(counters.open_sign_editor_packets, 1);
    assert_eq!(counters.pong_response_packets, 1);
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
fn map_item_data_event_updates_world_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::MapItemData(MapItemData {
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
    }))
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

    assert_eq!(counters.map_item_data_packets, 1);
    assert_eq!(counters.maps_tracked, 1);
    assert_eq!(counters.map_decorations_tracked, 1);
    assert_eq!(counters.map_color_patches_applied, 1);
    assert_eq!(counters.map_color_patches_ignored, 0);
    assert_eq!(
        counters.last_map_color_patch,
        Some(bbb_control::MapColorPatchState {
            map_id: 42,
            start_x: 3,
            start_y: 4,
            width: 2,
            height: 2,
        })
    );
}

#[test]
fn take_item_entity_event_updates_snapshot_counter() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::TakeItemEntity(
        bbb_protocol::packets::TakeItemEntity {
            item_id: 10,
            player_id: 20,
            amount: 3,
        },
    ))
    .unwrap();
    drop(tx);

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        take_item_entity_packets: 99,
        take_item_entities_applied: 99,
        take_item_entities_ignored: 99,
        item_entity_stack_shrinks: 99,
        take_item_entities_removed: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(counters.take_item_entity_packets, 1);
    assert_eq!(counters.take_item_entities_applied, 0);
    assert_eq!(counters.take_item_entities_ignored, 1);
    assert_eq!(counters.item_entity_stack_shrinks, 0);
    assert_eq!(counters.take_item_entities_removed, 0);
    assert_eq!(world.counters().take_item_entities_received, 1);
    assert_eq!(world.counters().take_item_entities_applied, 0);
    assert_eq!(world.counters().take_item_entities_ignored, 1);
}

#[test]
fn clear_titles_event_updates_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::SetTitlesAnimation(
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetTitleText(
        bbb_protocol::packets::SetTitleText {
            content: "Quest complete".to_string(),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetSubtitleText(
        bbb_protocol::packets::SetSubtitleText {
            content: "Return to camp".to_string(),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::ClearTitles(bbb_protocol::packets::ClearTitles {
        reset_times: false,
    }))
    .unwrap();
    tx.try_send(NetEvent::ClearTitles(bbb_protocol::packets::ClearTitles {
        reset_times: true,
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(counters.title, bbb_control::TitleState::default());
    assert_eq!(counters.clear_titles_packets, 2);
    assert_eq!(counters.title_text_packets, 1);
    assert_eq!(counters.subtitle_text_packets, 1);
    assert_eq!(counters.titles_animation_packets, 1);
}

#[test]
fn command_suggestions_event_updates_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::CommandSuggestions(
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
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        command_suggestion_packets: 99,
        command_suggestion_entries_tracked: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(counters.command_suggestion_packets, 1);
    assert_eq!(counters.command_suggestion_entries_tracked, 2);
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
fn commands_event_updates_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::Commands(command_tree_packet("say")))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        command_tree_packets: 99,
        command_nodes_tracked: 99,
        command_literal_nodes_tracked: 99,
        command_argument_nodes_tracked: 99,
        command_redirect_nodes_tracked: 99,
        command_executable_nodes_tracked: 99,
        command_restricted_nodes_tracked: 99,
        last_command_root_index: Some(99),
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );
    assert_eq!(counters.command_tree_packets, 1);
    assert_eq!(counters.command_nodes_tracked, 3);
    assert_eq!(counters.command_literal_nodes_tracked, 1);
    assert_eq!(counters.command_argument_nodes_tracked, 1);
    assert_eq!(counters.command_redirect_nodes_tracked, 0);
    assert_eq!(counters.command_executable_nodes_tracked, 1);
    assert_eq!(counters.command_restricted_nodes_tracked, 1);
    assert_eq!(counters.last_command_root_index, Some(0));
    assert_eq!(world.counters().command_tree_packets, 1);
    assert_eq!(world.counters().command_nodes_tracked, 3);
    assert_eq!(world.counters().command_literal_nodes_tracked, 1);
    assert_eq!(world.counters().command_argument_nodes_tracked, 1);
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
fn client_chat_events_update_world_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    let sender = Uuid::from_u128(0x1234);
    let signature = MessageSignature {
        bytes: vec![9; 256],
    };
    let expected_signature_checksum = signature.checksum();
    tx.try_send(NetEvent::PlayerChat(PlayerChat {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::DeleteChat(DeleteChat {
        message_signature: PackedMessageSignature {
            cache_id: Some(0),
            full_signature: None,
        },
    }))
    .unwrap();
    tx.try_send(NetEvent::DisguisedChat(DisguisedChat {
        message: "server notice".to_string(),
        chat_type: protocol_chat_type("Server"),
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(world.client_chat().messages.len(), 2);
    assert_eq!(world.client_chat().deleted_messages.len(), 1);
    assert_eq!(counters.player_chat_packets, 1);
    assert_eq!(counters.disguised_chat_packets, 1);
    assert_eq!(counters.delete_chat_packets, 1);
    assert_eq!(counters.chat_messages_tracked, 2);
    assert_eq!(counters.deleted_chat_messages_tracked, 1);
    assert_eq!(counters.chat_signature_cache_entries, 1);
    assert_eq!(counters.player_chat_unsigned_content_packets, 1);
    assert_eq!(counters.player_chat_filtered_packets, 1);
    assert_eq!(
        counters.last_player_chat,
        Some(bbb_control::ClientChatLine {
            kind: "player".to_string(),
            content: "hello".to_string(),
            sender: Some(sender.to_string()),
            sender_name: "Alice".to_string(),
            target_name: None,
            global_index: Some(0),
            message_index: Some(2),
            chat_type_id: Some(0),
            signature_checksum: Some(expected_signature_checksum),
            unsigned_content_present: true,
            filter_mask: "partially_filtered".to_string(),
            validation_state: "unchecked".to_string(),
        })
    );
    assert_eq!(
        counters
            .last_disguised_chat
            .as_ref()
            .map(|chat| &chat.content),
        Some(&"server notice".to_string())
    );
    assert_eq!(
        counters.last_deleted_chat,
        Some(bbb_control::DeletedChatLine {
            signature_checksum: Some(expected_signature_checksum),
            cache_id: Some(0),
            resolved: true,
        })
    );
}

#[test]
fn client_feature_events_update_world_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::CustomChatCompletions(CustomChatCompletions {
        action: CustomChatCompletionsAction::Set,
        entries: vec!["/warp".to_string(), "/spawn".to_string()],
    }))
    .unwrap();
    tx.try_send(NetEvent::PlaceGhostRecipe(PlaceGhostRecipe {
        container_id: 9,
        recipe_display_type: RecipeDisplayType::Stonecutter,
        recipe_display_body: vec![1, 2, 3],
    }))
    .unwrap();
    tx.try_send(NetEvent::UpdateAdvancements(UpdateAdvancements {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::SelectAdvancementsTab(SelectAdvancementsTab {
        tab: Some("minecraft:story/root".to_string()),
    }))
    .unwrap();
    tx.try_send(NetEvent::TagQuery(TagQuery {
        transaction_id: 12,
        tag_present: true,
        raw_nbt: vec![10, 0],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(counters.custom_chat_completion_packets, 1);
    assert_eq!(
        counters.last_custom_chat_completion,
        Some(bbb_control::CustomChatCompletionState {
            action: "set".to_string(),
            entries: 2,
        })
    );
    assert!(world.custom_chat_completions().contains("/warp"));
    assert!(world.custom_chat_completions().contains("/spawn"));
    assert_eq!(world.counters().custom_chat_completion_packets, 1);
    assert_eq!(world.counters().custom_chat_completions_tracked, 2);
    assert_eq!(counters.custom_chat_completions_tracked, 2);
    assert_eq!(counters.ghost_recipe_packets, 1);
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
    assert_eq!(counters.select_advancements_tab_packets, 1);
    assert_eq!(
        counters.selected_advancements_tab.as_deref(),
        Some("minecraft:story/root")
    );
    assert_eq!(
        world.selected_advancements_tab(),
        Some("minecraft:story/root")
    );
    assert_eq!(world.counters().select_advancements_tab_packets, 1);
    assert_eq!(counters.tag_query_packets, 1);
    assert_eq!(
        world.last_tag_query(),
        Some(&bbb_world::TagQueryResponseState {
            transaction_id: 12,
            tag_present: true,
            raw_nbt: vec![10, 0],
        })
    );
    assert_eq!(world.counters().tag_query_packets, 1);
    assert_eq!(
        counters.last_tag_query,
        Some(bbb_control::TagQueryState {
            transaction_id: 12,
            tag_present: true,
            raw_nbt_len: 2,
        })
    );
}

#[test]
fn inventory_events_update_world_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::OpenScreen(OpenScreen {
        container_id: 7,
        menu_type_id: 18,
        title: "Inventory".to_string(),
    }))
    .unwrap();
    tx.try_send(NetEvent::ContainerSetContent(ContainerSetContent {
        container_id: 7,
        state_id: 1,
        items: vec![item_stack(42, 1), item_stack(43, 2)],
        carried_item: item_stack(99, 1),
    }))
    .unwrap();
    tx.try_send(NetEvent::ContainerSetSlot(ContainerSetSlot {
        container_id: 7,
        state_id: 2,
        slot: 1,
        item: item_stack(44, 3),
    }))
    .unwrap();
    tx.try_send(NetEvent::ContainerSetData(ContainerSetData {
        container_id: 7,
        id: 3,
        value: 11,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetPlayerInventory(SetPlayerInventory {
        slot: 5,
        item: item_stack(12, 1),
    }))
    .unwrap();
    tx.try_send(NetEvent::SetCursorItem(SetCursorItem {
        item: item_stack(100, 4),
    }))
    .unwrap();
    tx.try_send(NetEvent::ContainerClose(ContainerClose { container_id: 7 }))
        .unwrap();
    tx.try_send(NetEvent::ContainerClose(ContainerClose {
        container_id: 99,
    }))
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

    assert_eq!(counters.container_open_updates_received, 1);
    assert_eq!(counters.container_content_updates_received, 1);
    assert_eq!(counters.container_slot_updates_received, 1);
    assert_eq!(counters.container_data_updates_received, 1);
    assert_eq!(counters.container_close_updates_received, 2);
    assert_eq!(counters.container_close_updates_applied, 1);
    assert_eq!(counters.container_close_updates_ignored, 1);
    assert_eq!(counters.inventory_slot_updates_received, 1);
    assert_eq!(counters.inventory_slots_tracked, 1);
    assert_eq!(counters.cursor_item_updates_received, 1);
}

#[test]
fn entity_events_update_world_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(19);
    tx.try_send(NetEvent::AddEntity(protocol_add_entity(123)))
        .unwrap();
    tx.try_send(NetEvent::AddEntity(protocol_add_entity(456)))
        .unwrap();
    tx.try_send(NetEvent::EntityPositionSync(EntityPositionSync {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::MoveEntity(EntityMove {
        id: 123,
        delta_x: 4096,
        delta_y: 0,
        delta_z: -2048,
        y_rot: Some(-90.0),
        x_rot: Some(45.0),
        on_ground: false,
    }))
    .unwrap();
    tx.try_send(NetEvent::TeleportEntity(TeleportEntity {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::EntityPositionSync(EntityPositionSync {
        id: 999,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: false,
    }))
    .unwrap();
    tx.try_send(NetEvent::MoveEntity(EntityMove {
        id: 999,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: false,
    }))
    .unwrap();
    tx.try_send(NetEvent::TeleportEntity(TeleportEntity {
        id: 999,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        relatives_mask: 0,
        on_ground: false,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetEntityMotion(SetEntityMotion {
        id: 123,
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: 0.0,
            z: -0.1,
        },
    }))
    .unwrap();
    tx.try_send(NetEvent::RotateHead(RotateHead {
        id: 123,
        y_head_rot: 90.0,
    }))
    .unwrap();
    tx.try_send(NetEvent::EntityAnimation(EntityAnimation {
        id: 123,
        action: 3,
    }))
    .unwrap();
    tx.try_send(NetEvent::EntityEvent(EntityEvent {
        entity_id: 123,
        event_id: 35,
    }))
    .unwrap();
    tx.try_send(NetEvent::HurtAnimation(HurtAnimation {
        id: 123,
        yaw: 45.5,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetEntityData(SetEntityData {
        id: 123,
        values: vec![EntityDataValue {
            data_id: 0,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(0x20),
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::SetEquipment(SetEquipment {
        entity_id: 123,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: item_stack(42, 1),
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::UpdateAttributes(UpdateAttributes {
        entity_id: 123,
        attributes: vec![AttributeSnapshot {
            attribute_id: 21,
            base: 20.0,
            modifiers: Vec::new(),
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::SetEntityLink(SetEntityLink {
        source_id: 123,
        dest_id: 456,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetPassengers(SetPassengers {
        vehicle_id: 123,
        passenger_ids: vec![456],
    }))
    .unwrap();
    tx.try_send(NetEvent::RemoveEntities(RemoveEntities {
        entity_ids: vec![456, 999],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        19
    );

    let world_counters = world.counters();
    macro_rules! assert_entity_counter {
        ($field:ident, $value:expr) => {
            assert_eq!(world_counters.$field, $value);
            assert_eq!(counters.$field, $value);
        };
    }

    assert_entity_counter!(entities_received, 2);
    assert_entity_counter!(entities_tracked, 1);
    assert_entity_counter!(entity_position_syncs_received, 2);
    assert_entity_counter!(entity_position_syncs_applied, 1);
    assert_entity_counter!(entity_position_syncs_ignored, 1);
    assert_entity_counter!(entity_moves_received, 2);
    assert_entity_counter!(entity_moves_applied, 1);
    assert_entity_counter!(entity_moves_ignored, 1);
    assert_entity_counter!(entity_teleports_received, 2);
    assert_entity_counter!(entity_teleports_applied, 1);
    assert_entity_counter!(entity_teleports_ignored, 1);
    assert_entity_counter!(entity_motion_updates_received, 1);
    assert_entity_counter!(entity_motion_updates_applied, 1);
    assert_entity_counter!(entity_head_rotations_received, 1);
    assert_entity_counter!(entity_head_rotations_applied, 1);
    assert_entity_counter!(entity_animation_updates_received, 1);
    assert_entity_counter!(entity_animation_updates_applied, 1);
    assert_entity_counter!(entity_events_received, 1);
    assert_entity_counter!(entity_events_applied, 1);
    assert_entity_counter!(entity_hurt_animations_received, 1);
    assert_entity_counter!(entity_hurt_animations_applied, 1);
    assert_entity_counter!(entity_data_updates_received, 1);
    assert_entity_counter!(entity_data_values_received, 1);
    assert_entity_counter!(entity_data_updates_applied, 1);
    assert_entity_counter!(entity_equipment_updates_received, 1);
    assert_entity_counter!(entity_equipment_slots_received, 1);
    assert_entity_counter!(entity_equipment_updates_applied, 1);
    assert_entity_counter!(entity_attribute_updates_received, 1);
    assert_entity_counter!(entity_attributes_received, 1);
    assert_entity_counter!(entity_attribute_updates_applied, 1);
    assert_entity_counter!(entity_link_updates_received, 1);
    assert_entity_counter!(entity_link_updates_applied, 1);
    assert_entity_counter!(entity_passenger_updates_received, 1);
    assert_entity_counter!(entity_passenger_ids_received, 1);
    assert_entity_counter!(entity_passenger_updates_applied, 1);
    assert_entity_counter!(entity_removes_received, 2);
    assert_entity_counter!(entities_removed, 1);
    assert_entity_counter!(entity_removes_ignored, 1);
}

#[test]
fn transient_entity_event_ignored_counters_are_projected() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::EntityAnimation(EntityAnimation {
        id: 999,
        action: 4,
    }))
    .unwrap();
    tx.try_send(NetEvent::EntityEvent(EntityEvent {
        entity_id: 999,
        event_id: 21,
    }))
    .unwrap();
    tx.try_send(NetEvent::HurtAnimation(HurtAnimation {
        id: 999,
        yaw: 90.0,
    }))
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

    assert_eq!(counters.entity_animation_updates_received, 1);
    assert_eq!(counters.entity_animation_updates_applied, 0);
    assert_eq!(counters.entity_animation_updates_ignored, 1);
    assert_eq!(counters.entity_events_received, 1);
    assert_eq!(counters.entity_events_applied, 0);
    assert_eq!(counters.entity_events_ignored, 1);
    assert_eq!(counters.entity_hurt_animations_received, 1);
    assert_eq!(counters.entity_hurt_animations_applied, 0);
    assert_eq!(counters.entity_hurt_animations_ignored, 1);
}

#[test]
fn simple_entity_update_ignored_counters_are_projected() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::SetEntityMotion(SetEntityMotion {
        id: 999,
        delta_movement: ProtocolVec3d::default(),
    }))
    .unwrap();
    tx.try_send(NetEvent::RotateHead(RotateHead {
        id: 999,
        y_head_rot: 90.0,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetEntityLink(SetEntityLink {
        source_id: 999,
        dest_id: 123,
    }))
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

    assert_eq!(counters.entity_motion_updates_received, 1);
    assert_eq!(counters.entity_motion_updates_applied, 0);
    assert_eq!(counters.entity_motion_updates_ignored, 1);
    assert_eq!(counters.entity_head_rotations_received, 1);
    assert_eq!(counters.entity_head_rotations_applied, 0);
    assert_eq!(counters.entity_head_rotations_ignored, 1);
    assert_eq!(counters.entity_link_updates_received, 1);
    assert_eq!(counters.entity_link_updates_applied, 0);
    assert_eq!(counters.entity_link_updates_ignored, 1);
}

#[test]
fn entity_metadata_ignored_counters_are_projected() {
    const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;

    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::AddEntity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )))
    .unwrap();
    tx.try_send(NetEvent::SetEquipment(SetEquipment {
        entity_id: 124,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: item_stack(42, 1),
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::UpdateAttributes(UpdateAttributes {
        entity_id: 124,
        attributes: vec![AttributeSnapshot {
            attribute_id: 21,
            base: 20.0,
            modifiers: Vec::new(),
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::SetEntityData(SetEntityData {
        id: 999,
        values: vec![EntityDataValue {
            data_id: 0,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(0x20),
        }],
    }))
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

    assert_eq!(counters.entity_data_updates_received, 1);
    assert_eq!(counters.entity_data_values_received, 1);
    assert_eq!(counters.entity_data_updates_applied, 0);
    assert_eq!(counters.entity_data_updates_ignored, 1);
    assert_eq!(counters.entity_equipment_updates_received, 1);
    assert_eq!(counters.entity_equipment_slots_received, 1);
    assert_eq!(counters.entity_equipment_updates_applied, 0);
    assert_eq!(counters.entity_equipment_updates_ignored, 1);
    assert_eq!(counters.entity_attribute_updates_received, 1);
    assert_eq!(counters.entity_attributes_received, 1);
    assert_eq!(counters.entity_attribute_updates_applied, 0);
    assert_eq!(counters.entity_attribute_updates_ignored, 1);
}

#[test]
fn passenger_ignored_counters_are_projected() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::SetPassengers(SetPassengers {
        vehicle_id: 999,
        passenger_ids: vec![123, 124],
    }))
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

    assert_eq!(counters.entity_passenger_updates_received, 1);
    assert_eq!(counters.entity_passenger_ids_received, 2);
    assert_eq!(counters.entity_passenger_updates_applied, 0);
    assert_eq!(counters.entity_passenger_updates_ignored, 1);
}

#[test]
fn remove_entities_syncs_active_effect_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::RemoveEntities(RemoveEntities {
        entity_ids: vec![entity_id],
    }))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    world.apply_update_mob_effect(protocol_update_mob_effect(entity_id, 3));
    let mut counters = NetCounters {
        active_mob_effects_tracked: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.counters().entities_tracked, 0);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
    assert_eq!(counters.entities_tracked, 0);
    assert_eq!(counters.active_mob_effects_tracked, 0);
}

#[test]
fn add_entity_replacement_syncs_active_effect_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::AddEntity(protocol_add_entity(entity_id)))
        .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    world.apply_update_mob_effect(protocol_update_mob_effect(entity_id, 3));
    let mut counters = NetCounters {
        active_mob_effects_tracked: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(world.counters().entities_tracked, 1);
    assert_eq!(world.counters().active_mob_effects_tracked, 0);
    assert_eq!(counters.entities_tracked, 1);
    assert_eq!(counters.active_mob_effects_tracked, 0);
}

#[test]
fn mob_effect_ignored_counters_are_projected() {
    const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;

    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::AddEntity(protocol_add_entity_with_type(
        124,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )))
    .unwrap();
    tx.try_send(NetEvent::UpdateMobEffect(protocol_update_mob_effect(
        124, 3,
    )))
    .unwrap();
    tx.try_send(NetEvent::RemoveMobEffect(
        bbb_protocol::packets::RemoveMobEffect {
            entity_id: 124,
            effect_id: 3,
        },
    ))
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

    assert_eq!(counters.update_mob_effect_packets, 1);
    assert_eq!(counters.update_mob_effects_ignored, 1);
    assert_eq!(counters.remove_mob_effect_packets, 1);
    assert_eq!(counters.remove_mob_effects_ignored, 1);
    assert_eq!(counters.active_mob_effects_tracked, 0);
}

#[test]
fn merchant_offers_event_updates_world_inventory_state() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::OpenScreen(OpenScreen {
        container_id: 7,
        menu_type_id: 19,
        title: "Merchant".to_string(),
    }))
    .unwrap();
    tx.try_send(NetEvent::MerchantOffers(MerchantOffers {
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
    }))
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
    assert_eq!(world_counters.merchant_offers_tracked, 1);

    assert_eq!(counters.container_open_updates_received, 1);
    assert_eq!(counters.merchant_offer_packets_received, 1);
    assert_eq!(counters.merchant_offer_packets_applied, 1);
    assert_eq!(counters.merchant_offer_packets_ignored, 0);
    assert_eq!(counters.merchant_offers_tracked, 1);
}

#[test]
fn recipe_book_events_update_world_state() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::RecipeBookAdd(RecipeBookAdd {
        replace: true,
        entries: vec![
            recipe_book_entry(7, true, true),
            recipe_book_entry(8, false, false),
        ],
    }))
    .unwrap();
    tx.try_send(NetEvent::RecipeBookRemove(RecipeBookRemove {
        recipe_ids: vec![RecipeDisplayId { index: 8 }],
    }))
    .unwrap();
    tx.try_send(NetEvent::RecipeBookSettings(RecipeBookSettings {
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
    }))
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
    assert_eq!(world_counters.recipe_book_entries_received, 2);
    assert_eq!(world_counters.recipe_book_removed_entries_received, 1);
    assert_eq!(world_counters.recipe_book_entries_tracked, 1);
    assert_eq!(world_counters.recipe_book_highlights_tracked, 1);
    assert_eq!(world_counters.recipe_book_notifications_received, 1);

    assert_eq!(counters.recipe_book_add_packets, 1);
    assert_eq!(counters.recipe_book_remove_packets, 1);
    assert_eq!(counters.recipe_book_settings_packets, 1);
    assert_eq!(counters.recipe_book_replace_packets, 1);
    assert_eq!(counters.recipe_book_entries_received, 2);
    assert_eq!(counters.recipe_book_removed_entries_received, 1);
    assert_eq!(counters.recipe_book_entries_tracked, 1);
    assert_eq!(counters.recipe_book_highlights_tracked, 1);
    assert_eq!(counters.recipe_book_notifications_received, 1);
}

#[test]
fn update_advancements_event_updates_world_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::UpdateAdvancements(UpdateAdvancements {
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
    }))
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
    assert_eq!(world_counters.advancements_tracked, 1);
    assert_eq!(world_counters.advancement_roots_tracked, 1);
    assert_eq!(world_counters.advancement_progress_tracked, 1);
    assert_eq!(world_counters.advancement_progress_criteria_tracked, 2);

    assert_eq!(counters.update_advancements_packets, 1);
    assert_eq!(counters.update_advancements_reset_packets, 1);
    assert_eq!(counters.update_advancements_show_packets, 1);
    assert_eq!(counters.advancements_added_received, 1);
    assert_eq!(counters.advancements_removed_received, 0);
    assert_eq!(counters.advancements_adds_ignored, 0);
    assert_eq!(counters.advancement_progress_received, 1);
    assert_eq!(counters.advancement_progress_updates_ignored, 0);
    assert_eq!(counters.advancements_tracked, 1);
    assert_eq!(counters.advancement_roots_tracked, 1);
    assert_eq!(counters.advancement_progress_tracked, 1);
    assert_eq!(counters.advancement_progress_criteria_tracked, 2);
}

#[test]
fn update_recipes_event_replaces_world_recipe_access_state() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.try_send(NetEvent::UpdateRecipes(UpdateRecipes {
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
            },
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
        world.recipes().property_sets.get("minecraft:furnace_input"),
        Some(&vec![42, 43])
    );
    assert_eq!(world.recipes().stonecutter_recipes.len(), 1);
    let world_counters = world.counters();
    assert_eq!(world_counters.update_recipes_packets, 1);
    assert_eq!(world_counters.recipe_property_sets_tracked, 1);
    assert_eq!(world_counters.recipe_property_set_items_tracked, 2);
    assert_eq!(world_counters.stonecutter_recipes_tracked, 1);

    assert_eq!(counters.update_recipes_packets, 1);
    assert_eq!(counters.recipe_property_sets_tracked, 1);
    assert_eq!(counters.recipe_property_set_items_tracked, 2);
    assert_eq!(counters.stonecutter_recipes_tracked, 1);
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
    tx.try_send(NetEvent::Waypoint(TrackedWaypointPacket {
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
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(counters.custom_payload_packets, 1);
    assert_eq!(counters.custom_payload_brand_packets, 1);
    assert_eq!(counters.custom_payload_unknown_packets, 0);
    assert_eq!(
        counters.last_custom_payload,
        Some(bbb_control::CustomPayloadState {
            id: "minecraft:brand".to_string(),
            kind: "brand".to_string(),
            brand: Some("vanilla".to_string()),
            raw_payload_len: 0,
        })
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
    assert_eq!(counters.clear_dialog_packets, 1);
    assert_eq!(counters.show_dialog_packets, 1);
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
    assert_eq!(counters.waypoint_packets, 1);
    assert_eq!(counters.waypoints_tracked, 1);
    assert_eq!(counters.waypoint_updates_applied, 0);
    assert_eq!(counters.waypoint_updates_ignored, 0);
    assert_eq!(counters.waypoint_untracks_ignored, 0);
    assert_eq!(
        counters.last_waypoint,
        Some(bbb_control::WaypointState {
            operation: "track".to_string(),
            identifier_kind: "uuid".to_string(),
            identifier: waypoint_id.to_string(),
            icon_style: "minecraft:default".to_string(),
            icon_color_rgb: Some(0x112233),
            waypoint_kind: "position".to_string(),
            position: Some(bbb_control::NetVec3i {
                x: 10,
                y: 64,
                z: -5,
            }),
            chunk: None,
            azimuth: None,
        })
    );
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
    tx.try_send(NetEvent::PlayerCombatEnter).unwrap();
    tx.try_send(NetEvent::PlayerCombatEnd(PlayerCombatEnd { duration: 37 }))
        .unwrap();
    tx.try_send(NetEvent::PlayerCombatKill(PlayerCombatKill {
        player_id: 123,
        message: "You died".to_string(),
    }))
    .unwrap();
    tx.try_send(NetEvent::PlayerLookAt(PlayerLookAt {
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
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(counters.player_combat_enter_packets, 1);
    assert_eq!(counters.player_combat_end_packets, 1);
    assert_eq!(counters.player_combat_kill_packets, 1);
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
    assert_eq!(
        counters.last_player_combat,
        Some(bbb_control::PlayerCombatState {
            kind: "kill".to_string(),
            duration: None,
            player_id: Some(123),
            message: Some("You died".to_string()),
        })
    );
    assert_eq!(counters.player_look_at_packets, 1);
    assert_eq!(
        counters.last_player_look_at,
        Some(bbb_control::PlayerLookAtState {
            from_anchor: "eyes".to_string(),
            position: bbb_control::NetVec3 {
                x: 10.5,
                y: 64.0,
                z: -2.25,
            },
            target_entity_id: Some(456),
            to_anchor: Some("feet".to_string()),
        })
    );
}

#[test]
fn client_audio_events_update_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::Sound(SoundEvent {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::SoundEntity(SoundEntityEvent {
        sound: SoundEventHolder::Direct {
            location: "minecraft:entity.cat.ambient".to_string(),
            fixed_range: Some(32.0),
        },
        source: SoundSource::Neutral,
        entity_id: 123,
        volume: 1.0,
        pitch: 0.5,
        seed: -9,
    }))
    .unwrap();
    tx.try_send(NetEvent::StopSound(StopSound {
        source: Some(SoundSource::Music),
        name: Some("minecraft:music.menu".to_string()),
    }))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(123));
    let mut counters = NetCounters {
        sound_packets: 99,
        sound_entity_packets: 99,
        sound_entity_events_applied: 99,
        sound_entity_events_ignored: 99,
        stop_sound_packets: 99,
        last_stop_sound: Some(bbb_control::StopSoundState {
            source: Some("stale".to_string()),
            name: Some("stale".to_string()),
        }),
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(counters.sound_packets, 1);
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
        })
    );
    assert_eq!(
        counters.last_sound,
        Some(bbb_control::ClientSoundState {
            sound: bbb_control::SoundHolderState {
                kind: "reference".to_string(),
                registry_id: Some(41),
                location: None,
                fixed_range: None,
            },
            source: "block".to_string(),
            position: bbb_control::NetVec3 {
                x: 2.5,
                y: -1.0,
                z: 0.0,
            },
            volume: 0.75,
            pitch: 1.25,
            seed: 123456789,
        })
    );
    assert_eq!(counters.sound_entity_packets, 1);
    assert_eq!(counters.sound_entity_events_applied, 1);
    assert_eq!(counters.sound_entity_events_ignored, 0);
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
    assert_eq!(
        counters.last_sound_entity,
        Some(bbb_control::ClientSoundEntityState {
            sound: bbb_control::SoundHolderState {
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
    assert_eq!(counters.stop_sound_packets, 1);
    assert_eq!(world.counters().stop_sound_packets, 1);
    assert_eq!(
        world.last_stop_sound(),
        Some(&bbb_world::StopSoundEventState {
            source: Some("music".to_string()),
            name: Some("minecraft:music.menu".to_string()),
        })
    );
    assert_eq!(
        counters.last_stop_sound,
        Some(bbb_control::StopSoundState {
            source: Some("music".to_string()),
            name: Some("minecraft:music.menu".to_string()),
        })
    );
}

#[test]
fn client_audio_events_emit_runtime_commands_for_applied_events() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::Sound(SoundEvent {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::SoundEntity(SoundEntityEvent {
        sound: SoundEventHolder::Direct {
            location: "minecraft:entity.cat.ambient".to_string(),
            fixed_range: Some(32.0),
        },
        source: SoundSource::Neutral,
        entity_id: 123,
        volume: 1.0,
        pitch: 0.5,
        seed: -9,
    }))
    .unwrap();
    tx.try_send(NetEvent::SoundEntity(SoundEntityEvent {
        sound: SoundEventHolder::Direct {
            location: "minecraft:entity.cat.ambient".to_string(),
            fixed_range: None,
        },
        source: SoundSource::Neutral,
        entity_id: 404,
        volume: 0.2,
        pitch: 1.8,
        seed: 7,
    }))
    .unwrap();
    tx.try_send(NetEvent::StopSound(StopSound {
        source: Some(SoundSource::Music),
        name: Some("minecraft:music.menu".to_string()),
    }))
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
    assert_eq!(counters.sound_packets, 1);
    assert_eq!(counters.sound_entity_packets, 2);
    assert_eq!(counters.sound_entity_events_applied, 1);
    assert_eq!(counters.sound_entity_events_ignored, 1);
    assert_eq!(counters.stop_sound_packets, 1);

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
fn silent_entity_sound_events_do_not_emit_runtime_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::SetEntityData(SetEntityData {
        id: 123,
        values: vec![EntityDataValue {
            data_id: 4,
            serializer_id: 8,
            value: EntityDataValueKind::Boolean(true),
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::SoundEntity(SoundEntityEvent {
        sound: SoundEventHolder::Direct {
            location: "minecraft:entity.cat.ambient".to_string(),
            fixed_range: Some(32.0),
        },
        source: SoundSource::Neutral,
        entity_id: 123,
        volume: 1.0,
        pitch: 0.5,
        seed: -9,
    }))
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
    assert_eq!(counters.sound_entity_packets, 1);
    assert_eq!(counters.sound_entity_events_applied, 0);
    assert_eq!(counters.sound_entity_events_ignored, 1);
}

#[test]
fn world_effect_events_update_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::Explosion(Explosion {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::LevelParticles(LevelParticles {
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
    }))
    .unwrap();
    let mut world = WorldStore::new();
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
    assert_eq!(counters.explosion_packets, 1);
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
    assert_eq!(counters.level_particles_packets, 1);
}

#[test]
fn level_particles_emit_particle_runtime_batch_and_snapshot_counters() {
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
    tx.try_send(NetEvent::LevelParticles(packet.clone()))
        .unwrap();
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    let mut particles = RecordingParticleSink::default();

    assert_eq!(
        drain_net_events_with_sinks(
            &mut rx,
            &mut world,
            &mut counters,
            &None,
            None,
            Some(&mut particles),
            None,
        ),
        1
    );

    assert_eq!(particles.packets, vec![packet]);
    assert_eq!(particles.batches.len(), 1);
    assert_eq!(world.counters().level_particles_packets, 1);
    assert_eq!(world.last_level_particles().unwrap().count, 0);
    assert_eq!(counters.level_particles_packets, 1);
}

#[test]
fn projectile_power_updates_world_entity_state_and_snapshot_counters() {
    const VANILLA_ENTITY_TYPE_FIREBALL_ID: i32 = 52;

    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::ProjectilePower(ProjectilePower {
        entity_id: 123,
        acceleration_power: 0.75,
    }))
    .unwrap();
    tx.try_send(NetEvent::ProjectilePower(ProjectilePower {
        entity_id: 456,
        acceleration_power: 0.25,
    }))
    .unwrap();
    tx.try_send(NetEvent::ProjectilePower(ProjectilePower {
        entity_id: 404,
        acceleration_power: 0.5,
    }))
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
    assert_eq!(counters.projectile_power_packets, 3);
    assert_eq!(counters.projectile_power_updates_applied, 1);
    assert_eq!(counters.projectile_power_updates_ignored, 2);
}

#[test]
fn debug_game_events_update_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(8);
    tx.try_send(NetEvent::DebugBlockValue(DebugBlockValue {
        pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
        raw_update_payload: vec![5, 1, 0xaa],
    }))
    .unwrap();
    tx.try_send(NetEvent::DebugChunkValue(DebugChunkValue {
        pos: ProtocolChunkPos { x: 3, z: -4 },
        raw_update_payload: vec![7, 0],
    }))
    .unwrap();
    tx.try_send(NetEvent::DebugEntityValue(DebugEntityValue {
        entity_id: 123,
        raw_update_payload: vec![9, 1, 0xbb],
    }))
    .unwrap();
    tx.try_send(NetEvent::DebugEvent(DebugEvent {
        raw_event_payload: vec![4, 0xcc],
    }))
    .unwrap();
    tx.try_send(NetEvent::DebugSample(DebugSample {
        sample: vec![100, -50],
        sample_type: RemoteDebugSampleType::TickTime,
    }))
    .unwrap();
    tx.try_send(NetEvent::GameRuleValues(GameRuleValues {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::GameTestHighlightPos(GameTestHighlightPos {
        absolute_pos: ProtocolBlockPos {
            x: -10,
            y: 70,
            z: 22,
        },
        relative_pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
    }))
    .unwrap();
    tx.try_send(NetEvent::TestInstanceBlockStatus(TestInstanceBlockStatus {
        status: "Ready".to_string(),
        size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
    }))
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

    assert_eq!(counters.debug_block_value_packets, 1);
    assert_eq!(counters.debug_chunk_value_packets, 1);
    assert_eq!(counters.debug_entity_value_packets, 1);
    assert_eq!(counters.debug_event_packets, 1);
    assert_eq!(counters.debug_sample_packets, 1);
    assert_eq!(counters.game_rule_value_packets, 1);
    assert_eq!(counters.game_test_highlight_pos_packets, 1);
    assert_eq!(counters.test_instance_block_status_packets, 1);
}

#[test]
fn block_destruction_event_updates_world_and_counter() {
    let (tx, mut rx) = mpsc::channel(3);
    tx.try_send(NetEvent::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 10,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::BlockDestruction(
        bbb_protocol::packets::BlockDestruction {
            id: 99,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            progress: 255,
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        block_destruction_packets: 99,
        block_destructions_tracked: 99,
        block_destructions_removed: 99,
        block_destructions_ignored: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        3
    );
    assert_eq!(counters.block_destruction_packets, 3);
    assert_eq!(counters.block_destructions_tracked, 0);
    assert_eq!(counters.block_destructions_removed, 1);
    assert_eq!(counters.block_destructions_ignored, 1);
    assert_eq!(world.counters().block_destructions_received, 3);
    assert_eq!(world.counters().block_destructions_tracked, 0);
    assert_eq!(world.counters().block_destructions_removed, 1);
    assert_eq!(world.counters().block_destructions_ignored, 1);
    assert!(world.block_destruction(4).is_none());
}

#[test]
fn block_and_level_events_update_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::BlockEvent(bbb_protocol::packets::BlockEvent {
        pos: ProtocolBlockPos {
            x: 12,
            y: 65,
            z: -5,
        },
        b0: 2,
        b1: 9,
        block_id: 54,
    }))
    .unwrap();
    tx.try_send(NetEvent::LevelEvent(bbb_protocol::packets::LevelEvent {
        event_type: 1001,
        pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
        data: 42,
        global: true,
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        block_event_packets: 99,
        block_events_tracked: 99,
        level_event_packets: 99,
        level_events_tracked: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(counters.block_event_packets, 1);
    assert_eq!(counters.block_events_tracked, 1);
    assert_eq!(counters.level_event_packets, 1);
    assert_eq!(counters.level_events_tracked, 1);

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
fn border_events_update_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(6);
    tx.try_send(NetEvent::InitializeBorder(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderCenter(
        bbb_protocol::packets::SetBorderCenter {
            new_center_x: 3.0,
            new_center_z: 4.0,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderLerpSize(
        bbb_protocol::packets::SetBorderLerpSize {
            old_size: 200.0,
            new_size: 300.0,
            lerp_time: 50,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderSize(
        bbb_protocol::packets::SetBorderSize { size: 250.0 },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderWarningDelay(
        bbb_protocol::packets::SetBorderWarningDelay { warning_delay: 9 },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetBorderWarningDistance(
        bbb_protocol::packets::SetBorderWarningDistance { warning_blocks: 8 },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        initialize_border_packets: 99,
        set_border_center_packets: 99,
        set_border_lerp_size_packets: 99,
        set_border_size_packets: 99,
        set_border_warning_delay_packets: 99,
        set_border_warning_distance_packets: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        6
    );
    assert_eq!(counters.initialize_border_packets, 1);
    assert_eq!(counters.set_border_center_packets, 1);
    assert_eq!(counters.set_border_lerp_size_packets, 1);
    assert_eq!(counters.set_border_size_packets, 1);
    assert_eq!(counters.set_border_warning_delay_packets, 1);
    assert_eq!(counters.set_border_warning_distance_packets, 1);

    let border = world.world_border();
    assert_eq!(border.center_x, 3.0);
    assert_eq!(border.center_z, 4.0);
    assert_eq!(border.size, 250.0);
    assert_eq!(border.lerp_target, 250.0);
    assert_eq!(border.lerp_time, 0);
    assert_eq!(border.absolute_max_size, 500);
    assert_eq!(border.warning_blocks, 8);
    assert_eq!(border.warning_time, 9);
}

#[test]
fn scoreboard_events_update_world_and_counters() {
    let (tx, mut rx) = mpsc::channel(11);
    tx.try_send(NetEvent::SetObjective(
        bbb_protocol::packets::SetObjective {
            objective_name: "kills".to_string(),
            method: bbb_protocol::packets::SetObjectiveMethod::Add,
            parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                display_name: "Kills".to_string(),
                render_type: bbb_protocol::packets::ObjectiveRenderType::Integer,
                number_format: Some(vec![9]),
            }),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetDisplayObjective(
        bbb_protocol::packets::SetDisplayObjective {
            slot: bbb_protocol::packets::ScoreboardDisplaySlot::Sidebar,
            objective_name: Some("kills".to_string()),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetScore(bbb_protocol::packets::SetScore {
        owner: "Steve".to_string(),
        objective_name: "kills".to_string(),
        score: 4,
        display: Some("Four".to_string()),
        number_format: None,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetScore(bbb_protocol::packets::SetScore {
        owner: "Alex".to_string(),
        objective_name: "kills".to_string(),
        score: 1,
        display: None,
        number_format: None,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetPlayerTeam(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::ResetScore(bbb_protocol::packets::ResetScore {
        owner: "Alex".to_string(),
        objective_name: Some("kills".to_string()),
    }))
    .unwrap();
    tx.try_send(NetEvent::SetObjective(
        bbb_protocol::packets::SetObjective {
            objective_name: "missing".to_string(),
            method: bbb_protocol::packets::SetObjectiveMethod::Change,
            parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                display_name: "Missing".to_string(),
                render_type: bbb_protocol::packets::ObjectiveRenderType::Integer,
                number_format: None,
            }),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetDisplayObjective(
        bbb_protocol::packets::SetDisplayObjective {
            slot: bbb_protocol::packets::ScoreboardDisplaySlot::List,
            objective_name: Some("missing".to_string()),
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetScore(bbb_protocol::packets::SetScore {
        owner: "Nobody".to_string(),
        objective_name: "missing".to_string(),
        score: 9,
        display: None,
        number_format: None,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetPlayerTeam(
        bbb_protocol::packets::SetPlayerTeam {
            name: "missing".to_string(),
            method: bbb_protocol::packets::PlayerTeamMethod::Join,
            parameters: None,
            players: vec!["Nobody".to_string()],
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::ResetScore(bbb_protocol::packets::ResetScore {
        owner: "Nobody".to_string(),
        objective_name: Some("missing".to_string()),
    }))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        set_objective_packets: 99,
        set_objective_updates_applied: 99,
        set_objective_updates_ignored: 99,
        set_display_objective_packets: 99,
        set_display_objective_updates_applied: 99,
        set_display_objective_updates_ignored: 99,
        set_score_packets: 99,
        set_score_updates_applied: 99,
        set_score_updates_ignored: 99,
        set_player_team_packets: 99,
        set_player_team_updates_applied: 99,
        set_player_team_updates_ignored: 99,
        reset_score_packets: 99,
        reset_score_updates_applied: 99,
        reset_score_updates_ignored: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        11
    );
    assert_eq!(counters.set_objective_packets, 2);
    assert_eq!(counters.set_objective_updates_applied, 1);
    assert_eq!(counters.set_objective_updates_ignored, 1);
    assert_eq!(counters.set_display_objective_packets, 2);
    assert_eq!(counters.set_display_objective_updates_applied, 1);
    assert_eq!(counters.set_display_objective_updates_ignored, 1);
    assert_eq!(counters.set_score_packets, 3);
    assert_eq!(counters.set_score_updates_applied, 2);
    assert_eq!(counters.set_score_updates_ignored, 1);
    assert_eq!(counters.set_player_team_packets, 2);
    assert_eq!(counters.set_player_team_updates_applied, 1);
    assert_eq!(counters.set_player_team_updates_ignored, 1);
    assert_eq!(counters.reset_score_packets, 2);
    assert_eq!(counters.reset_score_updates_applied, 1);
    assert_eq!(counters.reset_score_updates_ignored, 1);

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
fn hud_session_events_update_world_and_counters() {
    let boss_id = Uuid::from_u128(1);
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::BossEvent(bbb_protocol::packets::BossEvent {
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
    }))
    .unwrap();
    tx.try_send(NetEvent::BossEvent(bbb_protocol::packets::BossEvent {
        id: boss_id,
        operation: bbb_protocol::packets::BossEventOperation::UpdateProgress { progress: 0.25 },
    }))
    .unwrap();
    tx.try_send(NetEvent::BossEvent(bbb_protocol::packets::BossEvent {
        id: Uuid::from_u128(99),
        operation: bbb_protocol::packets::BossEventOperation::UpdateProgress { progress: 1.0 },
    }))
    .unwrap();
    tx.try_send(NetEvent::TabList(bbb_protocol::packets::TabList {
        header: Some("Welcome".to_string()),
        footer: None,
    }))
    .unwrap();
    tx.try_send(NetEvent::ChangeDifficulty(
        bbb_protocol::packets::ChangeDifficulty {
            difficulty: bbb_protocol::packets::Difficulty::Hard,
            locked: true,
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        boss_event_packets: 99,
        boss_bars_tracked: 99,
        boss_events_ignored: 99,
        tab_list_packets: 99,
        change_difficulty_packets: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(counters.boss_event_packets, 3);
    assert_eq!(counters.boss_bars_tracked, 1);
    assert_eq!(counters.boss_events_ignored, 1);
    assert_eq!(counters.tab_list_packets, 1);
    assert_eq!(counters.change_difficulty_packets, 1);

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
fn player_info_events_update_world_and_counters() {
    let profile_id = Uuid::from_u128(1);
    let removed_profile_id = Uuid::from_u128(2);
    let (tx, mut rx) = mpsc::channel(2);
    tx.try_send(NetEvent::PlayerInfoUpdate(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::PlayerInfoRemove(
        bbb_protocol::packets::PlayerInfoRemove {
            profile_ids: vec![removed_profile_id],
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        player_info_update_packets: 99,
        player_info_remove_packets: 99,
        player_info_entries_tracked: 99,
        listed_players_tracked: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        2
    );
    assert_eq!(counters.player_info_update_packets, 1);
    assert_eq!(counters.player_info_remove_packets, 1);
    assert_eq!(counters.player_info_entries_tracked, 1);
    assert_eq!(counters.listed_players_tracked, 1);

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
fn server_presentation_events_update_world_and_counters() {
    let pack_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::ServerData(bbb_protocol::packets::ServerData {
        motd: "Native test server".to_string(),
        icon_bytes: Some(vec![1, 2, 3, 4]),
    }))
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
    tx.try_send(NetEvent::ResourcePackPop(
        bbb_protocol::packets::ResourcePackPop { id: None },
    ))
    .unwrap();
    tx.try_send(NetEvent::ResourcePackPop(
        bbb_protocol::packets::ResourcePackPop { id: Some(pack_id) },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        server_data_packets: 99,
        resource_pack_push_packets: 99,
        resource_pack_pop_packets: 99,
        resource_pack_pop_updates_applied: 99,
        resource_pack_pop_updates_ignored: 99,
        resource_packs_tracked: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
    );
    assert_eq!(counters.server_data_packets, 1);
    assert_eq!(counters.resource_pack_push_packets, 1);
    assert_eq!(counters.resource_pack_pop_packets, 2);
    assert_eq!(counters.resource_pack_pop_updates_applied, 1);
    assert_eq!(counters.resource_pack_pop_updates_ignored, 1);
    assert_eq!(counters.resource_packs_tracked, 0);

    let server_data = world.server_data().unwrap();
    assert_eq!(server_data.motd, "Native test server");
    assert_eq!(server_data.icon_byte_len(), Some(4));
    assert!(world.resource_packs().is_empty());

    let world_counters = world.counters();
    assert_eq!(world_counters.server_data_packets, 1);
    assert_eq!(world_counters.resource_pack_push_packets, 1);
    assert_eq!(world_counters.resource_pack_pop_packets, 2);
    assert_eq!(world_counters.resource_pack_pop_updates_applied, 1);
    assert_eq!(world_counters.resource_pack_pop_updates_ignored, 1);
    assert_eq!(world_counters.resource_packs_tracked, 0);
}

#[test]
fn entity_status_events_update_world_and_counters() {
    let entity_id = 55;
    let (tx, mut rx) = mpsc::channel(5);
    tx.try_send(NetEvent::Cooldown(bbb_protocol::packets::Cooldown {
        cooldown_group: "minecraft:ender_pearl".to_string(),
        duration: 20,
    }))
    .unwrap();
    tx.try_send(NetEvent::DamageEvent(bbb_protocol::packets::DamageEvent {
        entity_id,
        source_type_id: 5,
        source_cause_id: -1,
        source_direct_id: 42,
        source_position: Some(bbb_protocol::packets::Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }),
    }))
    .unwrap();
    tx.try_send(NetEvent::DamageEvent(bbb_protocol::packets::DamageEvent {
        entity_id: 99,
        source_type_id: 5,
        source_cause_id: -1,
        source_direct_id: -1,
        source_position: None,
    }))
    .unwrap();
    tx.try_send(NetEvent::UpdateMobEffect(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::RemoveMobEffect(
        bbb_protocol::packets::RemoveMobEffect {
            entity_id,
            effect_id: 99,
        },
    ))
    .unwrap();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(entity_id));
    let mut counters = NetCounters {
        cooldown_packets: 99,
        cooldowns_tracked: 99,
        damage_event_packets: 99,
        damage_events_applied: 99,
        damage_events_ignored: 99,
        update_mob_effect_packets: 99,
        update_mob_effects_ignored: 99,
        remove_mob_effect_packets: 99,
        remove_mob_effects_ignored: 99,
        active_mob_effects_tracked: 99,
        ..NetCounters::default()
    };

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        5
    );
    assert_eq!(counters.cooldown_packets, 1);
    assert_eq!(counters.cooldowns_tracked, 1);
    assert_eq!(counters.damage_event_packets, 2);
    assert_eq!(counters.damage_events_applied, 1);
    assert_eq!(counters.damage_events_ignored, 1);
    assert_eq!(counters.update_mob_effect_packets, 1);
    assert_eq!(counters.update_mob_effects_ignored, 0);
    assert_eq!(counters.remove_mob_effect_packets, 1);
    assert_eq!(counters.remove_mob_effects_ignored, 1);
    assert_eq!(counters.active_mob_effects_tracked, 1);

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
        .try_send(NetEvent::MoveVehicle(bbb_protocol::packets::MoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        }))
        .unwrap();

    let mut counters = NetCounters {
        move_vehicle_packets: 99,
        vehicle_moves_applied: 99,
        vehicle_moves_acked: 99,
        vehicle_moves_snapped: 99,
        vehicle_moves_ignored: 99,
        ..NetCounters::default()
    };
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &commands),
        1
    );

    assert_eq!(counters.move_vehicle_packets, 1);
    assert_eq!(counters.vehicle_moves_applied, 1);
    assert_eq!(counters.vehicle_moves_acked, 1);
    assert_eq!(counters.vehicle_moves_snapped, 1);
    assert_eq!(counters.vehicle_moves_ignored, 0);
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
fn move_vehicle_ignored_counters_are_projected() {
    let (event_tx, mut event_rx) = mpsc::channel(1);

    event_tx
        .try_send(NetEvent::MoveVehicle(bbb_protocol::packets::MoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        }))
        .unwrap();

    let mut world = WorldStore::new();
    let mut counters = NetCounters {
        move_vehicle_packets: 99,
        vehicle_moves_applied: 99,
        vehicle_moves_acked: 99,
        vehicle_moves_snapped: 99,
        vehicle_moves_ignored: 99,
        ..NetCounters::default()
    };
    assert_eq!(
        drain_net_events(&mut event_rx, &mut world, &mut counters, &None),
        1
    );

    assert_eq!(counters.move_vehicle_packets, 1);
    assert_eq!(counters.vehicle_moves_applied, 0);
    assert_eq!(counters.vehicle_moves_acked, 0);
    assert_eq!(counters.vehicle_moves_snapped, 0);
    assert_eq!(counters.vehicle_moves_ignored, 1);
    assert_eq!(world.counters().vehicle_moves_received, 1);
    assert_eq!(world.counters().vehicle_moves_applied, 0);
    assert_eq!(world.counters().vehicle_moves_acked, 0);
    assert_eq!(world.counters().vehicle_moves_snapped, 0);
    assert_eq!(world.counters().vehicle_moves_ignored, 1);
}

#[test]
fn minecart_along_track_event_updates_world_state() {
    let (event_tx, mut event_rx) = mpsc::channel(1);
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(10, 85));

    event_tx
        .try_send(NetEvent::MoveMinecartAlongTrack(MoveMinecartAlongTrack {
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
        }))
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
    assert_eq!(counters.minecart_moves_received, 1);
    assert_eq!(counters.minecart_moves_applied, 1);
    assert_eq!(counters.minecart_lerp_steps_received, 1);
    assert_eq!(counters.minecart_lerp_steps_tracked, 1);
}

#[test]
fn minecart_along_track_ignored_counters_are_projected() {
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
        .try_send(NetEvent::MoveMinecartAlongTrack(MoveMinecartAlongTrack {
            entity_id: 999,
            lerp_steps: vec![step],
        }))
        .unwrap();
    event_tx
        .try_send(NetEvent::MoveMinecartAlongTrack(MoveMinecartAlongTrack {
            entity_id: 20,
            lerp_steps: vec![step],
        }))
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
    assert_eq!(counters.minecart_moves_received, 2);
    assert_eq!(counters.minecart_moves_applied, 0);
    assert_eq!(counters.minecart_moves_ignored, 2);
    assert_eq!(counters.minecart_lerp_steps_received, 2);
    assert_eq!(counters.minecart_lerp_steps_tracked, 0);
}

#[test]
fn login_projects_local_player_id_from_world() {
    let (tx, mut rx) = mpsc::channel(2);
    let respawn_info = protocol_play_login(9).common_spawn_info;
    tx.try_send(NetEvent::Login(protocol_play_login(9)))
        .unwrap();
    tx.try_send(NetEvent::Respawn(Respawn {
        common_spawn_info: respawn_info,
        data_to_keep: 0,
    }))
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
    assert_eq!(counters.player_entity_id, Some(9));
    assert_eq!(counters.play_logins_received, 1);
    assert_eq!(counters.respawns_received, 1);
}

#[test]
fn local_player_events_update_world_state_and_snapshot_counters() {
    let (tx, mut rx) = mpsc::channel(10);
    tx.try_send(NetEvent::Login(protocol_play_login(9)))
        .unwrap();
    tx.try_send(NetEvent::PlayerAbilities(
        bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::PlayerHealth(
        bbb_protocol::packets::PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::PlayerExperience(
        bbb_protocol::packets::PlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::HeldSlot(bbb_protocol::packets::SetHeldSlot {
        slot: 5,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetDefaultSpawnPosition(
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
    ))
    .unwrap();
    tx.try_send(NetEvent::SetSimulationDistance(
        bbb_protocol::packets::SetSimulationDistance { distance: 12 },
    ))
    .unwrap();
    tx.try_send(NetEvent::SetCamera(bbb_protocol::packets::SetCamera {
        camera_id: 9,
    }))
    .unwrap();
    tx.try_send(NetEvent::SetCamera(bbb_protocol::packets::SetCamera {
        camera_id: 123,
    }))
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

    assert_eq!(counters.player_entity_id, Some(9));
    assert_eq!(counters.player_health.unwrap().food, 16);
    assert_eq!(counters.player_experience.unwrap().total, 123);
    assert_eq!(counters.selected_hotbar_slot, 5);
    assert_eq!(counters.default_spawn.as_ref().unwrap().yaw, 90.0);
    assert_eq!(counters.simulation_distance, Some(12));
    assert_eq!(
        counters.camera,
        bbb_control::CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(counters.player_abilities_packets, 1);
    assert_eq!(counters.player_health_packets, 1);
    assert_eq!(counters.player_experience_packets, 1);
    assert_eq!(counters.held_slot_packets, 1);
    assert_eq!(counters.held_slot_updates_applied, 1);
    assert_eq!(counters.held_slot_updates_ignored, 0);
    assert_eq!(counters.default_spawn_position_packets, 1);
    assert_eq!(counters.simulation_distance_packets, 1);
    assert_eq!(counters.set_camera_packets, 2);
    assert_eq!(counters.set_camera_updates_applied, 1);
    assert_eq!(counters.set_camera_updates_ignored, 1);
}

#[test]
fn world_time_and_weather_update_snapshot_and_clear_color() {
    let (tx, mut rx) = mpsc::channel(4);
    tx.try_send(NetEvent::SetTime(bbb_protocol::packets::PlayTime {
        game_time: 123,
        clock_updates: vec![bbb_protocol::packets::ClockUpdate {
            clock_id: 0,
            total_ticks: 6000,
            partial_tick: 0.0,
            rate: 1.0,
        }],
    }))
    .unwrap();
    tx.try_send(NetEvent::GameEvent(bbb_protocol::packets::GameEvent {
        event_id: 7,
        param: 0.5,
    }))
    .unwrap();
    tx.try_send(NetEvent::TickingState(
        bbb_protocol::packets::TickingState {
            tick_rate: 0.25,
            frozen: true,
        },
    ))
    .unwrap();
    tx.try_send(NetEvent::TickingStep(bbb_protocol::packets::TickingStep {
        tick_steps: 7,
    }))
    .unwrap();

    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    assert_eq!(
        drain_net_events(&mut rx, &mut world, &mut counters, &None),
        4
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
    assert_eq!(
        world.ticking(),
        bbb_world::WorldTickingState {
            tick_rate: 1.0,
            frozen: true,
            frozen_ticks_to_run: 7,
        }
    );

    assert_eq!(
        counters.world_time,
        Some(bbb_control::WorldTime {
            game_time: 123,
            day_time: 6000,
            clock_updates: 1,
        })
    );
    assert!(counters.weather.raining);
    assert_eq!(counters.weather.rain_level, 0.5);
    assert_eq!(counters.ticking.tick_rate, 1.0);
    assert!(counters.ticking.frozen);
    assert_eq!(counters.ticking.frozen_ticks_to_run, 7);
    assert_eq!(counters.world_time_packets, 1);
    assert_eq!(counters.game_event_packets, 1);
    assert_eq!(counters.ticking_state_packets, 1);
    assert_eq!(counters.ticking_step_packets, 1);

    let world_color = clear_color_for_world(&world);
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

struct RecordingAudioSink {
    catalog: SoundCatalog,
    registry: SoundEventRegistry,
    commands: Vec<AudioCommand>,
    errors: Vec<String>,
}

impl RecordingAudioSink {
    fn new(catalog: SoundCatalog, registry: SoundEventRegistry) -> Self {
        Self {
            catalog,
            registry,
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
    fn play_positioned_sound(&mut self, state: &bbb_world::SoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::new(&self.catalog, &self.registry);
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
            let resolver = AudioCommandResolver::new(&self.catalog, &self.registry);
            resolver.play_entity_sound_at(state, position)
        };
        self.record(command);
    }

    fn stop_sound(&mut self, state: &bbb_world::StopSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::new(&self.catalog, &self.registry);
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
    batches: Vec<bbb_renderer::ParticleSpawnBatch>,
}

impl ParticleEventSink for RecordingParticleSink {
    fn spawn_level_particles(
        &mut self,
        packet: &LevelParticles,
    ) -> bbb_renderer::ParticleSpawnBatch {
        self.packets.push(packet.clone());
        let batch = bbb_renderer::ParticleSpawnBatch {
            missing_definition_count: 1,
            ..bbb_renderer::ParticleSpawnBatch::default()
        };
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
