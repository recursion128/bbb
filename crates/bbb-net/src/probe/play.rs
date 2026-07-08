use anyhow::{bail, Result};
use bbb_protocol::packets::{self, ChatAcknowledgement, PlayClientbound};
use bbb_world::{ChunkPos, PlayApplyEffects, VehicleMoveReport};

use crate::{probe::ProbeContext, resource_pack::response_action_for_push, types::ConnectionState};

/// Buffered play-apply side effects: the probe finishes the serverbound
/// responses after the synchronous world application returns.
#[derive(Default)]
struct ProbePlayEffects {
    chat_acknowledgement: Option<ChatAcknowledgement>,
    vehicle_move_report: Option<VehicleMoveReport>,
    inserted_chunk_pos: Option<ChunkPos>,
}

impl PlayApplyEffects for ProbePlayEffects {
    fn chat_acknowledgement(&mut self, command: ChatAcknowledgement) {
        self.chat_acknowledgement = Some(command);
    }

    fn vehicle_move_report(&mut self, report: VehicleMoveReport) {
        self.vehicle_move_report = Some(report);
    }

    fn chunk_inserted(&mut self, pos: ChunkPos) {
        self.inserted_chunk_pos = Some(pos);
    }
}

impl ProbeContext {
    pub(super) async fn handle_play_packet(
        &mut self,
        packet: PlayClientbound,
    ) -> Result<Option<ChunkPos>> {
        let mut effects = ProbePlayEffects::default();
        let connection_packet =
            self.world
                .apply_play_packet(packet, &mut self.level_event_sound_random, &mut effects);

        if let Some(command) = effects.chat_acknowledgement.take() {
            let (id, payload) = packets::encode_play_chat_acknowledgement(command);
            self.conn.send_packet(id, &payload).await?;
        }
        if let Some(report) = effects.vehicle_move_report.take() {
            let (id, payload) = packets::encode_play_move_vehicle(
                report.position.x,
                report.position.y,
                report.position.z,
                report.y_rot,
                report.x_rot,
                report.on_ground,
            );
            self.conn.send_packet(id, &payload).await?;
        }

        let Some(packet) = connection_packet else {
            return Ok(effects.inserted_chunk_pos);
        };
        match packet {
            PlayClientbound::KeepAlive { id } => {
                let (id, payload) = packets::encode_play_keep_alive(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::Ping { id } => {
                let (id, payload) = packets::encode_play_pong(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::ChunkBatchStart => {
                self.chunk_batch_size.on_batch_start();
            }
            PlayClientbound::ChunkBatchFinished { batch_size } => {
                let desired_chunks_per_tick = self.chunk_batch_size.on_batch_finished(batch_size);
                let (id, payload) =
                    packets::encode_play_chunk_batch_received(desired_chunks_per_tick);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let payload_present = payload.is_some();
                let (id, response) = packets::encode_play_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
                self.world
                    .apply_cookie_request(request.key, payload_present);
            }
            PlayClientbound::StoreCookie(cookie) => {
                let key = cookie.key;
                let payload_len = cookie.payload.len();
                self.server_cookies.insert(key.clone(), cookie.payload);
                self.world
                    .apply_store_cookie(key, payload_len, self.server_cookies.len());
            }
            PlayClientbound::StartConfiguration => {
                if let Some(command) = self.world.take_pending_player_chat_acknowledgement() {
                    let (id, payload) = packets::encode_play_chat_acknowledgement(command);
                    self.conn.send_packet(id, &payload).await?;
                }
                let (id, payload) = packets::encode_play_configuration_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.play_tick = None;
                self.seen_code_of_conduct = false;
                self.world.clear_client_level();
            }
            PlayClientbound::Disconnect(disconnect) => {
                bail!("play disconnected: {}", disconnect.reason)
            }
            PlayClientbound::ResourcePackPush(update) => {
                // The world push apply already ran in apply_play_packet.
                let pack_id = update.id;
                let action = response_action_for_push(&update);
                let (id, payload) = packets::encode_play_resource_pack_response(pack_id, action);
                self.conn.send_packet(id, &payload).await?;
                self.world.apply_resource_pack_response(pack_id, action);
            }
            PlayClientbound::PlayerPosition(update) => {
                self.player_position_state = update.apply_to_state(self.player_position_state);
                let (id, payload) = packets::encode_play_accept_teleportation(update.id);
                self.conn.send_packet(id, &payload).await?;
                let (id, payload) = packets::encode_play_move_player_pos_rot(
                    self.player_position_state.position.x,
                    self.player_position_state.position.y,
                    self.player_position_state.position.z,
                    self.player_position_state.y_rot,
                    self.player_position_state.x_rot,
                    false,
                    false,
                );
                self.conn.send_packet(id, &payload).await?;
                if !self.player_loaded_sent {
                    let (id, payload) = packets::encode_play_player_loaded();
                    self.conn.send_packet(id, &payload).await?;
                    self.player_loaded_sent = true;
                }
            }
            PlayClientbound::PlayerRotation(update) => {
                self.player_position_state = update.apply_to_state(self.player_position_state);
                let (id, payload) = packets::encode_play_move_player_rot(
                    self.player_position_state.y_rot,
                    self.player_position_state.x_rot,
                    false,
                    false,
                );
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::Unknown { packet_id, len } => {
                self.record_unsupported_packet(self.state, packet_id, len);
            }
            _ => {}
        }
        Ok(effects.inserted_chunk_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::RawConnection;
    use bbb_protocol::entity_types::*;
    use bbb_protocol::packets::{
        AddEntity, AdvancementCriterionProgressSummary, AdvancementProgressSummary,
        AdvancementSummary, AttributeModifier, AttributeSnapshot, AwardStats, BlockChangedAck,
        BlockEntityData, BlockEvent, BlockPos as ProtocolBlockPos, BlockUpdate, BossBarColor,
        BossBarOverlay, BossEvent, BossEventFlags, BossEventOperation, ChangeDifficulty,
        ChatFormatting, ChatTypeBound, ChatTypeHolder, ChunkBiomeData, ChunkHeightmapData,
        ChunkPos as ProtocolChunkPos, ChunksBiomes, ClearTitles, ClockUpdate,
        CommandArgumentParser, CommandNode, CommandNodeType, CommandSuggestion, CommandSuggestions,
        Commands, CommonPlayerSpawnInfo, ContainerClose, ContainerSetContent, ContainerSetData,
        ContainerSetSlot, CookieRequest, Cooldown, CustomChatCompletions,
        CustomChatCompletionsAction, CustomPayload, CustomPayloadBody, CustomReportDetails,
        DamageEvent, DebugBlockValue, DebugChunkValue, DebugEntityValue, DebugEvent, DebugSample,
        DeleteChat, DialogHolder, Difficulty, Disconnect, DisguisedChat, EntityAnchor,
        EntityAnimation, EntityDataValue, EntityDataValueKind, EntityEvent, EntityMove,
        EntityPositionSync, EquipmentSlot, EquipmentSlotUpdate, Explosion, FilterMask,
        FilterMaskKind, ForgetLevelChunk, GameEvent, GameProfile, GameProfileProperty,
        GameRuleValue, GameRuleValues, GameTestHighlightPos, GameType, HurtAnimation,
        IngredientSummary, InitializeBorder, InteractionHand, ItemCostSummary, ItemStackSummary,
        LevelChunkBlockEntity, LevelChunkData, LevelChunkWithLight, LevelEvent, LevelParticles,
        LightUpdate, LightUpdateData, MapColorPatch, MapDecoration, MapItemData, MerchantOffer,
        MerchantOffers, MessageSignature, MinecartStep, MobEffectFlags, MountScreenOpen,
        MoveMinecartAlongTrack, MoveVehicle, ObjectiveRenderType, OpenBook, OpenScreen,
        OpenSignEditor, PackedMessageSignature, ParticlePayload, PlaceGhostRecipe, PlayLogin,
        PlayTime, PlayerAbilities, PlayerChat, PlayerCombatEnd, PlayerCombatKill, PlayerExperience,
        PlayerHealth, PlayerInfoAction, PlayerInfoChatSession, PlayerInfoEntry, PlayerInfoRemove,
        PlayerInfoUpdate, PlayerLookAt, PlayerPositionUpdate, PlayerRotationUpdate,
        PlayerTeamMethod, PlayerTeamParameters, PongResponse, ProjectilePower, RecipeBookAdd,
        RecipeBookAddEntry, RecipeBookRemove, RecipeBookSettings, RecipeBookTypeSettings,
        RecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary, RecipeDisplayType,
        RecipePropertySetSummary, RegistryTags, RemoteDebugSampleType, RemoveEntities,
        RemoveMobEffect, ResetScore, ResourcePackPop, ResourcePackPush, ResourcePackResponseAction,
        Respawn, RotateHead, ScoreboardDisplaySlot, SectionBlocksUpdate, SelectAdvancementsTab,
        ServerData, ServerLinkEntry, ServerLinkKnownType, ServerLinkType, ServerLinks,
        SetActionBarText, SetBorderCenter, SetBorderLerpSize, SetBorderSize, SetBorderWarningDelay,
        SetBorderWarningDistance, SetCamera, SetChunkCacheCenter, SetChunkCacheRadius,
        SetCursorItem, SetDefaultSpawnPosition, SetDisplayObjective, SetEntityData, SetEntityLink,
        SetEntityMotion, SetEquipment, SetHeldSlot, SetObjective, SetObjectiveMethod,
        SetObjectiveParameters, SetPassengers, SetPlayerInventory, SetPlayerTeam, SetScore,
        SetSimulationDistance, SetSubtitleText, SetTitleText, SetTitlesAnimation, ShowDialog,
        SignedMessageBody, SlotDisplaySummary, SoundEntityEvent, SoundEvent, SoundEventHolder,
        SoundSource, StatUpdate, StonecutterSelectableRecipeSummary, StopSound, StoreCookie,
        SystemChat, TabList, TagNetworkPayload, TagQuery, TakeItemEntity, TeamCollisionRule,
        TeamVisibility, TeleportEntity, TestInstanceBlockStatus, TickingState, TickingStep,
        TrackedWaypoint, TrackedWaypointPacket, Transfer, UpdateAdvancements, UpdateAttributes,
        UpdateMobEffect, UpdateRecipes, UpdateTags, Vec3d as ProtocolVec3d, Vec3i as ProtocolVec3i,
        WaypointData, WaypointIcon, WaypointIdentifier, WaypointOperation, WaypointVec3i,
        WrittenBookContentSummary,
    };
    use bbb_protocol::{
        codec::{Decoder, Encoder},
        ids,
    };
    use bbb_world::{
        advance_growth_level_event_particle_randoms, BlockPos, ChunkPos,
        LevelEventGrowthRandomMode, LevelEventSoundRandomState, LocalPlayerPoseState, RegistrySet,
    };
    use bytes::BytesMut;
    use std::{collections::BTreeMap, time::Duration};
    use tokio::net::TcpListener;
    use tokio::time::timeout;
    use uuid::Uuid;

    #[tokio::test]
    async fn probe_applies_debug_game_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::DebugBlockValue(DebugBlockValue {
                pos: ProtocolBlockPos { x: 1, y: 64, z: -2 },
                raw_update_payload: vec![5, 1, 0xaa],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DebugChunkValue(DebugChunkValue {
                pos: ProtocolChunkPos { x: 3, z: -4 },
                raw_update_payload: vec![7, 0],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DebugEntityValue(DebugEntityValue {
                entity_id: 123,
                raw_update_payload: vec![9, 1, 0xbb],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DebugEvent(DebugEvent {
                raw_event_payload: vec![4, 0xcc],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DebugSample(DebugSample {
                sample: vec![100, -50],
                sample_type: RemoteDebugSampleType::TickTime,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameRuleValues(GameRuleValues {
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
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameTestHighlightPos(
                GameTestHighlightPos {
                    absolute_pos: ProtocolBlockPos {
                        x: -10,
                        y: 70,
                        z: 22,
                    },
                    relative_pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TestInstanceBlockStatus(
                TestInstanceBlockStatus {
                    status: "Ready".to_string(),
                    size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
                },
            ))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.debug_block_value_packets, 1);
        assert_eq!(report.world_counters.debug_chunk_value_packets, 1);
        assert_eq!(report.world_counters.debug_entity_value_packets, 1);
        assert_eq!(report.world_counters.debug_event_packets, 1);
        assert_eq!(report.world_counters.debug_sample_packets, 1);
        assert_eq!(report.world_counters.game_rule_value_packets, 1);
        assert_eq!(report.world_counters.game_test_highlight_pos_packets, 1);
        assert_eq!(report.world_counters.test_instance_block_status_packets, 1);
        assert_eq!(
            report.world.last_debug_block_value(),
            Some(&bbb_world::DebugBlockValueState {
                pos: BlockPos { x: 1, y: 64, z: -2 },
                raw_update_payload_len: 3,
            })
        );
        assert_eq!(
            report
                .world
                .last_game_rule_values()
                .map(|state| state.len()),
            Some(2)
        );
        assert_eq!(
            report.world.last_test_instance_block_status(),
            Some(&bbb_world::TestInstanceBlockStatusState {
                status: "Ready".to_string(),
                size: Some(bbb_world::DebugVec3iState { x: 3, y: 4, z: 5 }),
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_award_stats_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AwardStats(AwardStats {
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
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world.stat_value(8, 10), Some(3));
        assert_eq!(report.world.stat_value(0, 4), Some(11));
        assert_eq!(report.world_counters.award_stats_packets, 1);
        assert_eq!(report.world_counters.award_stats_entries_received, 2);
        assert_eq!(report.world_counters.last_award_stats_entry_count, 2);
        assert_eq!(report.world_counters.stats_tracked, 2);
    }

    #[tokio::test]
    async fn probe_applies_item_cooldown_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Cooldown(Cooldown {
                cooldown_group: "minecraft:ender_pearl".to_string(),
                duration: 20,
            }))
            .await
            .unwrap();

        let cooldown = probe
            .world
            .cooldown("minecraft:ender_pearl")
            .expect("cooldown should be tracked after positive duration");
        assert_eq!(cooldown.duration, 20);
        assert_eq!(cooldown.remaining_ticks, 20);
        assert_eq!(
            probe
                .world
                .item_cooldown_percent("minecraft:ender_pearl", 0.5),
            0.975
        );

        probe
            .handle_play_packet(PlayClientbound::Cooldown(Cooldown {
                cooldown_group: "minecraft:ender_pearl".to_string(),
                duration: 0,
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert!(report.world.cooldown("minecraft:ender_pearl").is_none());
        assert_eq!(
            report
                .world
                .item_cooldown_percent("minecraft:ender_pearl", 0.0),
            0.0
        );
        assert_eq!(report.world_counters.cooldown_packets, 2);
        assert_eq!(report.world_counters.cooldowns_tracked, 0);
    }

    #[tokio::test]
    async fn probe_applies_tag_query_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::TagQuery(TagQuery {
                transaction_id: 12,
                tag_present: true,
                raw_nbt: vec![10, 0],
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.last_tag_query(),
            Some(&bbb_world::TagQueryResponseState {
                transaction_id: 12,
                tag_present: true,
                raw_nbt: vec![10, 0],
            })
        );
        assert_eq!(report.world.last_tag_query().unwrap().raw_nbt_len(), 2);
        assert_eq!(report.world_counters.tag_query_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_command_suggestion_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::CustomChatCompletions(
                CustomChatCompletions {
                    action: CustomChatCompletionsAction::Set,
                    entries: vec!["/spawn".to_string(), "/warp".to_string()],
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::CustomChatCompletions(
                CustomChatCompletions {
                    action: CustomChatCompletionsAction::Remove,
                    entries: vec!["/spawn".to_string()],
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::CommandSuggestions(CommandSuggestions {
                id: 77,
                start: 1,
                length: 4,
                suggestions: vec![
                    CommandSuggestion {
                        text: "give".to_string(),
                        tooltip: Some("Run give".to_string()),
                    },
                    CommandSuggestion {
                        text: "gamemode".to_string(),
                        tooltip: None,
                    },
                ],
            }))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });
        let completions = report.world.custom_chat_completions();
        assert_eq!(completions.len(), 1);
        assert!(completions.contains("/warp"));
        assert_eq!(
            report.world.last_custom_chat_completion_update(),
            Some(&bbb_world::CustomChatCompletionUpdateState {
                action: "remove".to_string(),
                entries: 1,
            })
        );

        let suggestions = report
            .world
            .command_suggestions_by_id(77)
            .expect("suggestions tracked by id");
        assert_eq!(suggestions.start, 1);
        assert_eq!(suggestions.length, 4);
        assert_eq!(suggestions.suggestions.len(), 2);
        assert_eq!(suggestions.suggestions[0].text, "give");
        assert_eq!(
            suggestions.suggestions[0].tooltip.as_deref(),
            Some("Run give")
        );
        assert_eq!(suggestions.suggestions[1].text, "gamemode");
        assert_eq!(suggestions.suggestions[1].tooltip, None);
        assert_eq!(report.world.last_command_suggestions(), Some(suggestions));
        assert_eq!(report.world_counters.custom_chat_completion_packets, 2);
        assert_eq!(report.world_counters.custom_chat_completions_tracked, 1);
        assert_eq!(report.world_counters.command_suggestion_packets, 1);
        assert_eq!(report.world_counters.command_suggestion_entries_tracked, 2);
    }

    #[tokio::test]
    async fn probe_applies_command_tree_and_update_tags_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Commands(command_tree_packet("say")))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::UpdateTags(UpdateTags {
                registries: vec![RegistryTags {
                    registry: "minecraft:item".to_string(),
                    tags: vec![TagNetworkPayload {
                        tag: "minecraft:logs".to_string(),
                        entries: vec![5, 6, 7],
                    }],
                }],
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        let commands = report.world.commands();
        assert_eq!(commands.root_index, 0);
        assert_eq!(commands.nodes.len(), 3);
        assert_eq!(commands.nodes[1].name.as_deref(), Some("say"));
        assert_eq!(commands.nodes[2].name.as_deref(), Some("message"));
        assert_eq!(
            commands.nodes[2].parser.as_ref().unwrap().name,
            "minecraft:message"
        );
        assert_eq!(
            commands.nodes[2].suggestions.as_deref(),
            Some("minecraft:ask_server")
        );
        assert!(commands.command_requires_signed_arguments("say hello world"));
        assert_eq!(
            report.world.registry_tags("minecraft:item").unwrap().tags["minecraft:logs"],
            vec![5, 6, 7]
        );
        assert_eq!(report.world_counters.command_tree_packets, 1);
        assert_eq!(report.world_counters.command_nodes_tracked, 3);
        assert_eq!(report.world_counters.command_literal_nodes_tracked, 1);
        assert_eq!(report.world_counters.command_argument_nodes_tracked, 1);
        assert_eq!(report.world_counters.command_executable_nodes_tracked, 1);
        assert_eq!(report.world_counters.command_restricted_nodes_tracked, 1);
        assert_eq!(report.world_counters.update_tags_packets, 1);
        assert_eq!(report.world_counters.last_update_tags_registry_count, 1);
        assert_eq!(report.world_counters.last_update_tags_total_tag_count, 1);
        assert_eq!(report.world_counters.last_update_tags_total_value_count, 3);
        assert_eq!(report.world_counters.tag_registries_tracked, 1);
        assert_eq!(report.world_counters.tags_tracked, 1);
        assert_eq!(report.world_counters.tag_entries_tracked, 3);
    }

    #[tokio::test]
    async fn probe_applies_map_item_data_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::MapItemData(MapItemData {
                map_id: 12,
                scale: 1,
                locked: false,
                decorations: Some(vec![MapDecoration {
                    type_id: 4,
                    x: -8,
                    y: 9,
                    rot: 3,
                    name: Some("Camp".to_string()),
                }]),
                color_patch: Some(MapColorPatch {
                    start_x: 5,
                    start_y: 6,
                    width: 2,
                    height: 1,
                    colors: vec![11, 12],
                }),
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        let map = report.world.map_item(12).expect("map state is tracked");
        assert_eq!(map.decorations.len(), 1);
        assert_eq!(map.decorations[0].name.as_deref(), Some("Camp"));
        assert_eq!(map.colors[5 + 6 * 128], 11);
        assert_eq!(map.colors[6 + 6 * 128], 12);
        assert_eq!(
            report.world.last_map_color_patch(),
            Some(&bbb_world::LastMapColorPatchState {
                map_id: 12,
                start_x: 5,
                start_y: 6,
                width: 2,
                height: 1,
            })
        );
        assert_eq!(report.world_counters.map_item_data_packets, 1);
        assert_eq!(report.world_counters.maps_tracked, 1);
        assert_eq!(report.world_counters.map_decorations_tracked, 1);
        assert_eq!(report.world_counters.map_color_patches_applied, 1);
    }

    #[tokio::test]
    async fn probe_applies_audio_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(123)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::Sound(SoundEvent {
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
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SoundEntity(SoundEntityEvent {
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
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::StopSound(StopSound {
                source: Some(SoundSource::Music),
                name: Some("minecraft:music.menu".to_string()),
            }))
            .await
            .unwrap();

        let report = probe.finish(4, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.sound_packets, 1);
        assert_eq!(
            report.world.last_sound(),
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
        assert_eq!(report.world_counters.sound_entity_packets, 1);
        assert_eq!(report.world_counters.sound_entity_events_applied, 1);
        assert_eq!(report.world_counters.sound_entity_events_ignored, 0);
        assert_eq!(
            report.world.last_sound_entity(),
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
        assert_eq!(report.world_counters.stop_sound_packets, 1);
        assert_eq!(
            report.world.last_stop_sound(),
            Some(&bbb_world::StopSoundEventState {
                source: Some("music".to_string()),
                name: Some("minecraft:music.menu".to_string()),
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_visual_effect_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.world.set_local_player_pose(LocalPlayerPoseState {
            delta_movement: ProtocolVec3d {
                x: 0.5,
                y: -0.25,
                z: 1.0,
            },
            ..LocalPlayerPoseState::default()
        });

        probe
            .handle_play_packet(PlayClientbound::Explosion(Explosion {
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
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::LevelParticles(LevelParticles {
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
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.explosion_packets, 1);
        assert_eq!(
            report.world.last_explosion(),
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
            report.world.local_player_pose().unwrap().delta_movement,
            ProtocolVec3d {
                x: 0.75,
                y: -0.75,
                z: 2.5,
            }
        );
        assert_eq!(report.world_counters.level_particles_packets, 1);
        assert_eq!(
            report.world.last_level_particles(),
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

    #[tokio::test]
    async fn probe_chunk_batch_feedback_uses_vanilla_calculator() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::ChunkBatchStart)
            .await
            .unwrap();
        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "chunk batch start must only update the local calculator"
        );

        probe
            .handle_play_packet(PlayClientbound::ChunkBatchFinished { batch_size: 0 })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("chunk batch received packet should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED);
        let desired = Decoder::new(&payload).read_f32().unwrap();
        assert_eq!(desired, 3.5);
    }

    #[tokio::test]
    async fn probe_play_keepalive_and_ping_send_common_responses() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::KeepAlive {
                id: 0x1122_3344_5566_7788,
            })
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::Ping { id: 0x0a0b_0c0d })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("keepalive response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_KEEP_ALIVE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_i64().unwrap(), 0x1122_3344_5566_7788);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("pong response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PONG);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_i32().unwrap(), 0x0a0b_0c0d);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn probe_player_chat_sends_chat_acknowledgement_after_threshold() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        for index in 0..65 {
            probe
                .handle_play_packet(PlayClientbound::PlayerChat(
                    protocol_player_chat_with_signature(
                        index,
                        MessageSignature {
                            bytes: vec![index as u8; 256],
                        },
                    ),
                ))
                .await
                .unwrap();
        }

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("chat acknowledgement packet should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_ACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 65);
        assert!(decoder.is_empty());
        assert_eq!(probe.world.counters().player_chat_packets, 65);
        assert_eq!(
            probe.world.counters().player_chat_acknowledgement_packets,
            1
        );
    }

    #[tokio::test]
    async fn probe_applies_delete_and_disguised_chat_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let signature = MessageSignature {
            bytes: vec![9; 256],
        };
        let expected_signature_checksum = signature.checksum();

        probe
            .handle_play_packet(PlayClientbound::PlayerChat(
                protocol_player_chat_with_signature(0, signature),
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DeleteChat(DeleteChat {
                message_signature: PackedMessageSignature {
                    cache_id: Some(0),
                    full_signature: None,
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::DisguisedChat(DisguisedChat {
                message: "server notice".to_string(),
                chat_type: ChatTypeBound {
                    chat_type: ChatTypeHolder::Registry { id: 0 },
                    name: "Server".to_string(),
                    target_name: None,
                },
            }))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });
        let chat = report.world.client_chat();

        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.deleted_messages.len(), 1);
        assert_eq!(chat.messages[0].kind, bbb_world::ChatMessageKind::Player);
        assert_eq!(chat.messages[0].content, "message 0");
        assert_eq!(chat.messages[1].kind, bbb_world::ChatMessageKind::Disguised);
        assert_eq!(chat.messages[1].content, "server notice");
        assert_eq!(chat.messages[1].sender_name, "Server");

        let deleted = &chat.deleted_messages[0];
        assert_eq!(deleted.cache_id, Some(0));
        assert!(deleted.resolved);
        assert_eq!(
            deleted
                .signature
                .as_ref()
                .map(|signature| signature.checksum),
            Some(expected_signature_checksum)
        );

        assert_eq!(report.world_counters.player_chat_packets, 1);
        assert_eq!(report.world_counters.delete_chat_packets, 1);
        assert_eq!(report.world_counters.disguised_chat_packets, 1);
        assert_eq!(report.world_counters.chat_messages_tracked, 2);
        assert_eq!(report.world_counters.deleted_chat_messages_tracked, 1);
        assert_eq!(report.world_counters.chat_signature_cache_entries, 1);
        assert_eq!(report.world_counters.chat_unknown_packed_signatures, 0);
    }

    #[tokio::test]
    async fn probe_start_configuration_acknowledges_and_resets_configuration_dedup_state() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Play;
        probe.play_tick = Some(crate::connection::play_tick_interval());
        probe.seen_code_of_conduct = true;
        probe.world.apply_add_entity(protocol_add_entity(55));
        probe.world.set_local_using_item(true);

        probe
            .handle_play_packet(PlayClientbound::StartConfiguration)
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("configuration acknowledgement should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
        assert!(payload.is_empty());
        assert_eq!(probe.state, ConnectionState::Configuration);
        assert!(probe.play_tick.is_none());
        assert!(!probe.seen_code_of_conduct);
        assert_eq!(probe.world.entity_count(), 0);
        assert_eq!(
            probe.world.local_player(),
            &bbb_world::LocalPlayerState::default()
        );

        probe
            .handle_configuration_packet(packets::ConfigurationClientbound::CodeOfConduct {
                text: "Fresh configuration rules.".to_string(),
            })
            .await
            .unwrap();

        assert!(probe.seen_code_of_conduct);
        assert_eq!(
            probe.world.last_code_of_conduct().unwrap().text,
            "Fresh configuration rules."
        );
        assert_eq!(probe.world.counters().code_of_conduct_packets, 1);
    }

    #[tokio::test]
    async fn probe_start_configuration_flushes_pending_chat_acknowledgement_first() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Play;

        probe
            .handle_play_packet(PlayClientbound::PlayerChat(
                protocol_player_chat_with_signature(
                    0,
                    MessageSignature {
                        bytes: vec![9; 256],
                    },
                ),
            ))
            .await
            .unwrap();

        probe
            .handle_play_packet(PlayClientbound::StartConfiguration)
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("chat acknowledgement should be sent first")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CHAT_ACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("configuration acknowledgement should be sent second")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
        assert!(payload.is_empty());
        assert_eq!(probe.state, ConnectionState::Configuration);
        assert_eq!(
            probe
                .world
                .counters()
                .player_chat_acknowledgement_pending_offset,
            0
        );
    }

    #[tokio::test]
    async fn probe_records_nonfatal_chunk_update_errors() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::BlockEntityData(BlockEntityData {
                pos: ProtocolBlockPos { x: 0, y: 64, z: 0 },
                block_entity_type_id: 9,
                raw_nbt: vec![1],
            }))
            .await
            .unwrap();

        let counters = probe.world.counters();
        assert_eq!(counters.block_entity_updates_received, 1);
        assert_eq!(counters.block_entity_updates_applied, 0);
        let diagnostics = probe.world.apply_diagnostics();
        assert_eq!(diagnostics.apply_errors.len(), 1);
        assert_eq!(diagnostics.apply_errors[0].source, "block_entity_data");
        assert!(
            diagnostics.apply_errors[0].message.contains("nbt byte"),
            "{:?}",
            diagnostics.apply_errors
        );
        assert_eq!(counters.world_apply_errors, 1);

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.world_apply_errors.len(), 1);
        assert!(report.world_apply_errors[0].contains("nbt byte"));
        assert_eq!(report.world_counters.world_apply_errors, 1);
        assert_eq!(report.world.apply_diagnostics().apply_errors.len(), 1);
    }

    #[tokio::test]
    async fn probe_applies_block_entity_sign_text_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(
                synthetic_probe_level_chunk_packet(),
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BlockEntityData(BlockEntityData {
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
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 1, z: -2 });
        let pos = BlockPos {
            x: 16,
            y: -64,
            z: -32,
        };

        assert_eq!(
            report.world.sign_text_lines(pos, true),
            Some([
                "Front A".to_string(),
                "Front B".to_string(),
                "Front C".to_string(),
                "Front D".to_string(),
            ])
        );
        assert_eq!(
            report.world.sign_text_lines(pos, false),
            Some([
                "Back A".to_string(),
                "Back B".to_string(),
                "Back C".to_string(),
                "Back D".to_string(),
            ])
        );
        assert_eq!(report.world_counters.block_entity_updates_applied, 1);
    }

    #[tokio::test]
    async fn probe_records_bad_chunk_and_continues_to_good_chunk() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let mut bad_chunk = synthetic_probe_level_chunk_packet();
        bad_chunk.chunk_data.section_data = vec![0xff];

        let first_chunk = probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(bad_chunk))
            .await
            .unwrap();

        assert_eq!(first_chunk, None);
        assert_eq!(probe.world.counters().world_apply_errors, 1);
        assert_eq!(
            probe.world.apply_diagnostics().apply_errors[0].source,
            "level_chunk_with_light"
        );

        let first_chunk = probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(
                synthetic_probe_level_chunk_packet(),
            ))
            .await
            .unwrap();

        assert_eq!(first_chunk, Some(ChunkPos { x: 1, z: -2 }));
        assert_eq!(probe.world.counters().chunks_received, 1);
        assert_eq!(probe.world.counters().world_apply_errors, 1);
    }

    #[tokio::test]
    async fn probe_applies_chunk_view_and_block_updates_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let chunk_pos = ChunkPos { x: 1, z: -2 };

        probe
            .handle_play_packet(PlayClientbound::SetChunkCacheCenter(SetChunkCacheCenter {
                chunk_x: chunk_pos.x,
                chunk_z: chunk_pos.z,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetChunkCacheRadius(SetChunkCacheRadius {
                radius: 7,
            }))
            .await
            .unwrap();

        let first_chunk = probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(
                synthetic_probe_level_chunk_packet(),
            ))
            .await
            .unwrap();

        assert_eq!(first_chunk, Some(chunk_pos));
        assert_eq!(probe.world.chunk_cache_center(), Some(chunk_pos));
        assert_eq!(probe.world.chunk_cache_radius(), Some(7));
        assert_eq!(probe.world.first_chunk(), Some(chunk_pos));
        assert_eq!(probe.world.chunk_positions(), vec![chunk_pos]);

        probe
            .handle_play_packet(PlayClientbound::BlockUpdate(BlockUpdate {
                pos: ProtocolBlockPos {
                    x: 16,
                    y: -64,
                    z: -32,
                },
                block_state_id: 9,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SectionBlocksUpdate(SectionBlocksUpdate {
                section_x: 1,
                section_y: -4,
                section_z: -2,
                updates: vec![
                    BlockUpdate {
                        pos: ProtocolBlockPos {
                            x: 17,
                            y: -64,
                            z: -32,
                        },
                        block_state_id: 9,
                    },
                    BlockUpdate {
                        pos: ProtocolBlockPos {
                            x: 18,
                            y: -64,
                            z: -32,
                        },
                        block_state_id: 9,
                    },
                ],
            }))
            .await
            .unwrap();

        for x in 16..=18 {
            assert_eq!(
                probe
                    .world
                    .probe_block(BlockPos { x, y: -64, z: -32 })
                    .unwrap()
                    .block_state_id,
                9
            );
        }
        assert_eq!(probe.world.counters().chunks_received, 1);
        assert_eq!(probe.world.counters().chunks_decoded, 1);
        assert_eq!(probe.world.counters().block_updates_received, 3);
        assert_eq!(probe.world.counters().block_updates_applied, 3);
        assert_eq!(probe.world.counters().block_updates_ignored, 0);
        assert_eq!(
            probe.world.counters().chunk_cache_center_updates_received,
            1
        );
        assert_eq!(
            probe.world.counters().chunk_cache_radius_updates_received,
            1
        );

        probe
            .handle_play_packet(PlayClientbound::ForgetLevelChunk(ForgetLevelChunk {
                pos: ProtocolChunkPos {
                    x: chunk_pos.x,
                    z: chunk_pos.z,
                },
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world.chunk_count(), 0);
        assert_eq!(report.world.first_chunk(), Some(chunk_pos));
        assert_eq!(report.world_counters.chunk_forgets_received, 1);
        assert_eq!(report.world_counters.chunks_forgotten, 1);
        assert_eq!(report.world_counters.chunk_forgets_ignored, 0);
    }

    #[tokio::test]
    async fn probe_applies_light_and_biome_updates_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let chunk_pos = ChunkPos { x: 1, z: -2 };
        let mut sky = vec![0; LIGHT_ARRAY_BYTES];
        let mut block = vec![0; LIGHT_ARRAY_BYTES];
        set_light_nibble(&mut sky, 0, 4);
        set_light_nibble(&mut block, 0, 13);

        probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(
                synthetic_probe_level_chunk_packet(),
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::LightUpdate(LightUpdate {
                chunk_x: chunk_pos.x,
                chunk_z: chunk_pos.z,
                light_data: LightUpdateData {
                    sky_y_mask: vec![0b10],
                    block_y_mask: vec![0b10],
                    empty_sky_y_mask: Vec::new(),
                    empty_block_y_mask: Vec::new(),
                    sky_updates: vec![sky],
                    block_updates: vec![block],
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ChunksBiomes(ChunksBiomes {
                chunks: vec![ChunkBiomeData {
                    pos: ProtocolChunkPos {
                        x: chunk_pos.x,
                        z: chunk_pos.z,
                    },
                    raw_biomes: single_biome_payload(7),
                }],
            }))
            .await
            .unwrap();

        let chunk = probe.world.probe_chunk(chunk_pos).unwrap();
        assert_eq!(chunk.light.sky_y_mask, vec![0b10]);
        assert_eq!(chunk.light.block_y_mask, vec![0b10]);
        assert_eq!(chunk.light.sky_updates.len(), 1);
        assert_eq!(chunk.light.block_updates.len(), 1);
        assert_eq!(chunk.light.sky_updates[0][0] & 0x0f, 4);
        assert_eq!(chunk.light.block_updates[0][0] & 0x0f, 13);
        assert_eq!(
            probe
                .world
                .probe_block(BlockPos {
                    x: 16,
                    y: -64,
                    z: -32,
                })
                .unwrap()
                .biome_id,
            Some(7)
        );

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.chunks_received, 1);
        assert_eq!(report.world_counters.light_updates_received, 1);
        assert_eq!(report.world_counters.light_updates_applied, 1);
        assert_eq!(report.world_counters.light_updates_ignored, 0);
        assert_eq!(report.world_counters.biome_updates_received, 1);
        assert_eq!(report.world_counters.biome_updates_applied, 1);
        assert_eq!(report.world_counters.biome_updates_ignored, 0);
    }

    #[tokio::test]
    async fn probe_applies_block_destruction_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::BlockDestruction(
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
            .await
            .unwrap();
        assert_eq!(
            probe
                .world
                .block_destruction(4)
                .map(|progress| progress.progress),
            Some(6)
        );

        probe
            .handle_play_packet(PlayClientbound::BlockDestruction(
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
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BlockDestruction(
                bbb_protocol::packets::BlockDestruction {
                    id: 99,
                    pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                    progress: 255,
                },
            ))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.block_destructions_received, 3);
        assert_eq!(report.world_counters.block_destructions_tracked, 0);
        assert_eq!(report.world_counters.block_destructions_removed, 1);
        assert_eq!(report.world_counters.block_destructions_ignored, 1);
        assert!(report.world.block_destruction(4).is_none());
    }

    #[tokio::test]
    async fn probe_play_cookie_events_update_world_and_respond() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::StoreCookie(StoreCookie {
                key: "bbb:session".to_string(),
                payload: vec![4, 5, 6],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::CookieRequest(CookieRequest {
                key: "bbb:session".to_string(),
            }))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("cookie response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_COOKIE_RESPONSE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "bbb:session");
        assert!(decoder.read_bool().unwrap());
        let len = decoder.read_len().unwrap();
        assert_eq!(
            decoder.read_exact(len, "cookie response").unwrap(),
            &[4, 5, 6]
        );
        assert!(decoder.is_empty());

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.world.last_cookie_key(), Some("bbb:session"));
        assert_eq!(report.world.stored_cookie_count(), 1);
        assert_eq!(report.world_counters.store_cookie_packets, 1);
        assert_eq!(report.world_counters.stored_cookie_bytes, 3);
        assert_eq!(report.world_counters.cookie_request_packets, 1);
        assert_eq!(report.world_counters.cookie_response_hits, 1);
        assert_eq!(report.world_counters.cookie_response_misses, 0);
    }

    #[tokio::test]
    async fn probe_play_server_presentation_packets_update_world_and_decline_resource_pack() {
        let pack_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
        let missing_pack_id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::ServerData(ServerData {
                motd: "Native test server".to_string(),
                icon_bytes: Some(vec![1, 2, 3, 4]),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ResourcePackPush(ResourcePackPush {
                id: pack_id,
                url: "https://example.invalid/pack.zip".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: true,
                prompt: Some("Install pack?".to_string()),
            }))
            .await
            .unwrap();

        let pack = probe
            .world
            .resource_pack(pack_id)
            .expect("resource pack should be tracked after push");
        assert_eq!(pack.url, "https://example.invalid/pack.zip");
        assert_eq!(pack.hash, "0123456789abcdef0123456789abcdef01234567");
        assert!(pack.required);
        assert_eq!(pack.prompt.as_deref(), Some("Install pack?"));
        assert_eq!(
            pack.last_response.as_ref().map(|response| response.action),
            Some(ResourcePackResponseAction::Declined)
        );

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            ResourcePackResponseAction::Declined.ordinal()
        );
        assert!(decoder.is_empty());

        probe
            .handle_play_packet(PlayClientbound::ResourcePackPop(ResourcePackPop {
                id: Some(pack_id),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ResourcePackPop(ResourcePackPop {
                id: Some(missing_pack_id),
            }))
            .await
            .unwrap();

        let report = probe.finish(4, ChunkPos { x: 0, z: 0 });
        let server_data = report
            .world
            .server_data()
            .expect("server data should be tracked");
        assert_eq!(server_data.motd, "Native test server");
        assert_eq!(server_data.icon_byte_len(), Some(4));
        assert!(report.world.resource_packs().is_empty());
        assert_eq!(report.world_counters.server_data_packets, 1);
        assert_eq!(report.world_counters.resource_pack_push_packets, 1);
        assert_eq!(report.world_counters.resource_pack_response_packets, 1);
        assert_eq!(
            report.world_counters.resource_pack_response_updates_applied,
            1
        );
        assert_eq!(
            report.world_counters.resource_pack_response_updates_ignored,
            0
        );
        assert_eq!(report.world_counters.resource_pack_required_declines, 1);
        assert_eq!(report.world_counters.resource_pack_pop_packets, 2);
        assert_eq!(report.world_counters.resource_pack_pop_updates_applied, 1);
        assert_eq!(report.world_counters.resource_pack_pop_updates_ignored, 1);
        assert_eq!(report.world_counters.resource_packs_tracked, 0);
    }

    #[tokio::test]
    async fn probe_play_common_server_presentation_packets_update_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let details = BTreeMap::from([
            ("Region".to_string(), "local".to_string()),
            ("Server".to_string(), "probe-play".to_string()),
        ]);

        probe
            .handle_play_packet(PlayClientbound::CustomReportDetails(CustomReportDetails {
                details: details.clone(),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ServerLinks(ServerLinks {
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
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.world.custom_report_details(), &details);
        assert_eq!(report.world.server_links().len(), 2);
        assert_eq!(
            report.world.server_links()[0].label,
            "known_server_link.support"
        );
        assert_eq!(
            report.world.server_links()[0].known_type.as_deref(),
            Some("support")
        );
        assert_eq!(
            report.world.server_links()[0].url,
            "https://example.invalid/support"
        );
        assert_eq!(report.world.server_links()[1].label, "Rules");
        assert_eq!(report.world.server_links()[1].known_type, None);
        assert_eq!(
            report.world.server_links()[1].url,
            "http://example.invalid/rules"
        );
        assert_eq!(report.world_counters.custom_report_detail_packets, 1);
        assert_eq!(report.world_counters.custom_report_details_tracked, 2);
        assert_eq!(report.world_counters.server_link_packets, 1);
        assert_eq!(report.world_counters.server_links_tracked, 2);
        assert_eq!(report.world_counters.server_link_invalid_entries, 1);
    }

    #[tokio::test]
    async fn probe_applies_inventory_container_packets_to_world() {
        const VANILLA_MERCHANT_MENU_TYPE_ID: i32 = 19;

        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetPlayerInventory(SetPlayerInventory {
                slot: 36,
                item: item_stack(42, 1),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::OpenScreen(OpenScreen {
                container_id: 7,
                menu_type_id: VANILLA_MERCHANT_MENU_TYPE_ID,
                title: "Merchant".to_string(),
                title_styled: Vec::new(),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ContainerSetContent(ContainerSetContent {
                container_id: 7,
                state_id: 12,
                items: vec![ItemStackSummary::empty(), item_stack(43, 2)],
                carried_item: item_stack(98, 1),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ContainerSetSlot(ContainerSetSlot {
                container_id: 7,
                state_id: 13,
                slot: 1,
                item: item_stack(44, 3),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ContainerSetData(ContainerSetData {
                container_id: 7,
                id: 2,
                value: 10,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MerchantOffers(merchant_offers(7, 2)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetCursorItem(SetCursorItem {
                item: item_stack(99, 1),
            }))
            .await
            .unwrap();

        let inventory = probe.world.inventory();
        assert_eq!(inventory.player_slots.len(), 1);
        assert_eq!(inventory.player_slots[0].slot, 36);
        assert_eq!(inventory.player_slots[0].item, item_stack(42, 1));
        assert_eq!(inventory.cursor_item, item_stack(99, 1));

        let container = inventory.open_container.as_ref().unwrap();
        assert_eq!(container.container_id, 7);
        assert_eq!(container.menu_type_id, Some(VANILLA_MERCHANT_MENU_TYPE_ID));
        assert_eq!(container.title.as_deref(), Some("Merchant"));
        assert_eq!(container.state_id, 13);
        assert_eq!(container.slots.len(), 2);
        assert_eq!(container.slots[0].item, ItemStackSummary::empty());
        assert_eq!(container.slots[1].item, item_stack(44, 3));
        assert_eq!(container.data_values.len(), 1);
        assert_eq!(container.data_values[0].id, 2);
        assert_eq!(container.data_values[0].value, 10);
        let offers = container.merchant_offers.as_ref().unwrap();
        assert_eq!(offers.container_id, 7);
        assert_eq!(offers.offers.len(), 2);
        assert_eq!(offers.villager_level, 3);
        assert_eq!(offers.villager_xp, 120);
        assert!(offers.show_progress);
        assert!(!offers.can_restock);
        assert_eq!(offers.offers[0].buy_a, item_cost(42, 3));
        assert_eq!(offers.offers[0].sell, item_stack(99, 1));

        probe
            .handle_play_packet(PlayClientbound::ContainerClose(ContainerClose {
                container_id: 7,
            }))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });

        assert!(report.world.inventory().open_container.is_none());
        assert_eq!(
            report.world.inventory().player_slots[0].item,
            item_stack(42, 1)
        );
        assert_eq!(report.world.inventory().cursor_item, item_stack(99, 1));
        assert_eq!(report.world_counters.inventory_slot_updates_received, 1);
        assert_eq!(report.world_counters.inventory_slots_tracked, 1);
        assert_eq!(report.world_counters.cursor_item_updates_received, 1);
        assert_eq!(report.world_counters.container_open_updates_received, 1);
        assert_eq!(report.world_counters.container_content_updates_received, 1);
        assert_eq!(report.world_counters.container_slot_updates_received, 1);
        assert_eq!(report.world_counters.container_data_updates_received, 1);
        assert_eq!(report.world_counters.merchant_offer_packets_received, 1);
        assert_eq!(report.world_counters.merchant_offer_packets_applied, 1);
        assert_eq!(report.world_counters.merchant_offer_packets_ignored, 0);
        assert_eq!(report.world_counters.merchant_offers_tracked, 0);
        assert_eq!(report.world_counters.container_close_updates_received, 1);
        assert_eq!(report.world_counters.container_close_updates_applied, 1);
        assert_eq!(report.world_counters.container_close_updates_ignored, 0);
    }

    #[tokio::test]
    async fn probe_play_custom_payload_updates_world_brand_and_unknown_payload() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::CustomPayload(CustomPayload {
                id: "minecraft:brand".to_string(),
                payload: CustomPayloadBody::Brand {
                    brand: "vanilla".to_string(),
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::CustomPayload(CustomPayload {
                id: "example:diagnostic".to_string(),
                payload: CustomPayloadBody::Unknown {
                    raw_payload: vec![1, 2, 3, 4],
                },
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world.server_brand(), Some("vanilla"));
        assert_eq!(
            report.world.last_custom_payload(),
            Some(&bbb_world::CustomPayloadState {
                id: "example:diagnostic".to_string(),
                kind: "unknown".to_string(),
                brand: None,
                raw_payload_len: 4,
            })
        );
        assert_eq!(report.world_counters.custom_payload_packets, 2);
        assert_eq!(report.world_counters.custom_payload_brand_packets, 1);
        assert_eq!(report.world_counters.custom_payload_unknown_packets, 1);
    }

    #[tokio::test]
    async fn probe_play_resource_pack_invalid_url_updates_world_and_response() {
        let pack_id = Uuid::from_u128(0x77777777_3333_4444_5555_666666666666);
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::ResourcePackPush(ResourcePackPush {
                id: pack_id,
                url: "not a valid resource pack url".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: true,
                prompt: Some("Install pack?".to_string()),
            }))
            .await
            .unwrap();

        let pack = probe
            .world
            .resource_pack(pack_id)
            .expect("resource pack should be tracked after push");
        assert_eq!(
            pack.last_response.as_ref().map(|response| response.action),
            Some(ResourcePackResponseAction::InvalidUrl)
        );

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            ResourcePackResponseAction::InvalidUrl.ordinal()
        );
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn probe_play_transfer_updates_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Transfer(Transfer {
                host: "next.example.invalid".to_string(),
                port: 25566,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.transfer_packets, 1);
        assert_eq!(
            report.world.last_transfer(),
            Some(&bbb_world::TransferTargetState {
                host: "next.example.invalid".to_string(),
                port: 25566,
            })
        );
    }

    #[tokio::test]
    async fn probe_move_vehicle_sends_vehicle_ack() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Login(protocol_play_login(99)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(10)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetPassengers(SetPassengers {
                vehicle_id: 10,
                passenger_ids: vec![99],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MoveVehicle(MoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            }))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("move vehicle ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_VEHICLE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 5.0);
        assert_eq!(decoder.read_f64().unwrap(), 66.0);
        assert_eq!(decoder.read_f64().unwrap(), -7.0);
        assert_eq!(decoder.read_f32().unwrap(), 45.0);
        assert_eq!(decoder.read_f32().unwrap(), -5.0);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let report = probe.finish(4, ChunkPos { x: 0, z: 0 });
        let vehicle = report.world.probe_entity(10).unwrap();
        assert_eq!(vehicle.position.x, 5.0);
        assert_eq!(vehicle.position.y, 66.0);
        assert_eq!(vehicle.position.z, -7.0);
        assert_eq!(report.world_counters.vehicle_moves_received, 1);
        assert_eq!(report.world_counters.vehicle_moves_applied, 1);
        assert_eq!(report.world_counters.vehicle_moves_acked, 1);
        assert_eq!(report.world_counters.vehicle_moves_snapped, 1);
    }

    #[tokio::test]
    async fn probe_move_vehicle_without_mount_does_not_ack() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Login(protocol_play_login(99)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(99)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MoveVehicle(MoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            }))
            .await
            .unwrap();

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "ignored vehicle move must not send a vehicle ack"
        );

        let report = probe.finish(4, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.world_counters.vehicle_moves_received, 1);
        assert_eq!(report.world_counters.vehicle_moves_applied, 0);
        assert_eq!(report.world_counters.vehicle_moves_acked, 0);
        assert_eq!(report.world_counters.vehicle_moves_snapped, 0);
        assert_eq!(report.world_counters.vehicle_moves_ignored, 1);
    }

    #[tokio::test]
    async fn probe_applies_minecart_along_track_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
                10,
                VANILLA_ENTITY_TYPE_MINECART_ID,
            )))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(20)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MoveMinecartAlongTrack(
                MoveMinecartAlongTrack {
                    entity_id: 10,
                    lerp_steps: vec![
                        minecart_step(1.25, 64.1, -2.25, 0.2, 0.0, -0.2, 45.0, -10.0, 0.5),
                        minecart_step(1.75, 64.2, -2.75, 0.4, 0.0, -0.4, 90.0, 5.0, 1.25),
                    ],
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MoveMinecartAlongTrack(
                MoveMinecartAlongTrack {
                    entity_id: 999,
                    lerp_steps: vec![minecart_step(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)],
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MoveMinecartAlongTrack(
                MoveMinecartAlongTrack {
                    entity_id: 20,
                    lerp_steps: vec![minecart_step(3.0, 64.0, -4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)],
                },
            ))
            .await
            .unwrap();

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });
        let minecart = report.world.probe_entity(10).unwrap();
        assert_eq!(minecart.position.x, 1.75);
        assert_eq!(minecart.position.y, 64.2);
        assert_eq!(minecart.position.z, -2.75);
        assert_eq!(minecart.delta_movement.x, 0.4);
        assert_eq!(minecart.delta_movement.y, 0.0);
        assert_eq!(minecart.delta_movement.z, -0.4);
        assert_eq!(minecart.y_rot, 90.0);
        assert_eq!(minecart.x_rot, 5.0);
        assert_eq!(minecart.minecart_lerp_steps.len(), 2);

        let non_minecart = report.world.probe_entity(20).unwrap();
        assert_eq!(non_minecart.position.x, 1.0);
        assert_eq!(non_minecart.position.y, 64.0);
        assert_eq!(non_minecart.position.z, -2.0);
        assert!(report.world.probe_entity(999).is_none());

        assert_eq!(report.world_counters.entities_received, 2);
        assert_eq!(report.world_counters.entities_tracked, 2);
        assert_eq!(report.world_counters.minecart_moves_received, 3);
        assert_eq!(report.world_counters.minecart_moves_applied, 1);
        assert_eq!(report.world_counters.minecart_moves_ignored, 2);
        assert_eq!(report.world_counters.minecart_lerp_steps_received, 4);
        assert_eq!(report.world_counters.minecart_lerp_steps_tracked, 2);
    }

    #[tokio::test]
    async fn probe_applies_entity_motion_and_transient_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(123)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetEntityMotion(SetEntityMotion {
                id: 123,
                delta_movement: ProtocolVec3d {
                    x: 0.1,
                    y: 0.0,
                    z: -0.1,
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RotateHead(RotateHead {
                id: 123,
                y_head_rot: 90.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::EntityAnimation(EntityAnimation {
                id: 123,
                action: 3,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 123,
                event_id: 35,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::HurtAnimation(HurtAnimation {
                id: 123,
                yaw: 45.5,
            }))
            .await
            .unwrap();

        let report = probe.finish(6, ChunkPos { x: 0, z: 0 });
        let entity = report.world.probe_entity(123).unwrap();

        assert_eq!(entity.delta_movement.x, 0.1);
        assert_eq!(entity.delta_movement.y, 0.0);
        assert_eq!(entity.delta_movement.z, -0.1);
        assert_eq!(entity.y_head_rot, 90.0);
        assert_eq!(entity.last_animation_action, Some(3));
        assert_eq!(entity.last_event_id, Some(35));
        assert_eq!(entity.last_hurt_yaw, Some(45.5));

        assert_eq!(report.world_counters.entities_received, 1);
        assert_eq!(report.world_counters.entities_tracked, 1);
        assert_eq!(report.world_counters.entity_motion_updates_received, 1);
        assert_eq!(report.world_counters.entity_motion_updates_applied, 1);
        assert_eq!(report.world_counters.entity_motion_updates_ignored, 0);
        assert_eq!(report.world_counters.entity_head_rotations_received, 1);
        assert_eq!(report.world_counters.entity_head_rotations_applied, 1);
        assert_eq!(report.world_counters.entity_head_rotations_ignored, 0);
        assert_eq!(report.world_counters.entity_animation_updates_received, 1);
        assert_eq!(report.world_counters.entity_animation_updates_applied, 1);
        assert_eq!(report.world_counters.entity_animation_updates_ignored, 0);
        assert_eq!(report.world_counters.entity_events_received, 1);
        assert_eq!(report.world_counters.entity_events_applied, 1);
        assert_eq!(report.world_counters.entity_events_ignored, 0);
        assert_eq!(report.world_counters.entity_hurt_animations_received, 1);
        assert_eq!(report.world_counters.entity_hurt_animations_applied, 1);
        assert_eq!(report.world_counters.entity_hurt_animations_ignored, 0);
    }

    #[tokio::test]
    async fn probe_counts_missing_entity_updates_as_ignored() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::EntityPositionSync(EntityPositionSync {
                id: 999,
                position: ProtocolVec3d {
                    x: 1.0,
                    y: 65.0,
                    z: -1.0,
                },
                delta_movement: ProtocolVec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: false,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MoveEntity(EntityMove {
                id: 999,
                delta_x: 4096,
                delta_y: 0,
                delta_z: -2048,
                y_rot: Some(90.0),
                x_rot: Some(10.0),
                on_ground: true,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TeleportEntity(TeleportEntity {
                id: 999,
                position: ProtocolVec3d {
                    x: 2.0,
                    y: 70.0,
                    z: -2.0,
                },
                delta_movement: ProtocolVec3d::default(),
                y_rot: 180.0,
                x_rot: 15.0,
                relatives_mask: 0,
                on_ground: true,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::EntityAnimation(EntityAnimation {
                id: 999,
                action: 4,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 999,
                event_id: 21,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::HurtAnimation(HurtAnimation {
                id: 999,
                yaw: 90.0,
            }))
            .await
            .unwrap();

        let report = probe.finish(6, ChunkPos { x: 0, z: 0 });

        assert!(report.world.probe_entity(999).is_none());
        assert_eq!(report.world_counters.entities_received, 0);
        assert_eq!(report.world_counters.entities_tracked, 0);
        assert_eq!(report.world_counters.entity_position_syncs_received, 1);
        assert_eq!(report.world_counters.entity_position_syncs_applied, 0);
        assert_eq!(report.world_counters.entity_position_syncs_ignored, 1);
        assert_eq!(report.world_counters.entity_moves_received, 1);
        assert_eq!(report.world_counters.entity_moves_applied, 0);
        assert_eq!(report.world_counters.entity_moves_ignored, 1);
        assert_eq!(report.world_counters.entity_teleports_received, 1);
        assert_eq!(report.world_counters.entity_teleports_applied, 0);
        assert_eq!(report.world_counters.entity_teleports_ignored, 1);
        assert_eq!(report.world_counters.entity_animation_updates_received, 1);
        assert_eq!(report.world_counters.entity_animation_updates_applied, 0);
        assert_eq!(report.world_counters.entity_animation_updates_ignored, 1);
        assert_eq!(report.world_counters.entity_events_received, 1);
        assert_eq!(report.world_counters.entity_events_applied, 0);
        assert_eq!(report.world_counters.entity_events_ignored, 1);
        assert_eq!(report.world_counters.entity_hurt_animations_received, 1);
        assert_eq!(report.world_counters.entity_hurt_animations_applied, 0);
        assert_eq!(report.world_counters.entity_hurt_animations_ignored, 1);
    }

    #[tokio::test]
    async fn probe_applies_entity_state_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(123)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(456)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
                300,
                VANILLA_ENTITY_TYPE_ITEM_ID,
            )))
            .await
            .unwrap();

        probe
            .handle_play_packet(PlayClientbound::EntityPositionSync(EntityPositionSync {
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
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MoveEntity(EntityMove {
                id: 123,
                delta_x: 4096,
                delta_y: 0,
                delta_z: -2048,
                y_rot: Some(-90.0),
                x_rot: Some(45.0),
                on_ground: false,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TeleportEntity(TeleportEntity {
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
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetEntityData(SetEntityData {
                id: 123,
                values: vec![
                    EntityDataValue {
                        data_id: 0,
                        serializer_id: 0,
                        value: EntityDataValueKind::Byte(0x20),
                    },
                    EntityDataValue {
                        data_id: 2,
                        serializer_id: 1,
                        value: EntityDataValueKind::Int(301),
                    },
                ],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetEquipment(SetEquipment {
                entity_id: 123,
                slots: vec![EquipmentSlotUpdate {
                    slot: EquipmentSlot::Head,
                    item: item_stack(42, 1),
                }],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::UpdateAttributes(UpdateAttributes {
                entity_id: 123,
                attributes: vec![AttributeSnapshot {
                    attribute_id: 21,
                    base: 20.0,
                    modifiers: vec![AttributeModifier {
                        id: "minecraft:health_bonus".to_string(),
                        amount: 4.0,
                        operation_id: 0,
                    }],
                }],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetEntityLink(SetEntityLink {
                source_id: 123,
                dest_id: 456,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetPassengers(SetPassengers {
                vehicle_id: 123,
                passenger_ids: vec![456],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetEntityData(SetEntityData {
                id: 300,
                values: vec![item_stack_entity_data(item_stack(99, 2))],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TakeItemEntity(TakeItemEntity {
                item_id: 300,
                player_id: 123,
                amount: 1,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::UpdateMobEffect(UpdateMobEffect {
                entity_id: 123,
                effect_id: 3,
                amplifier: 1,
                duration_ticks: 200,
                flags: MobEffectFlags {
                    raw: 0b0110,
                    ambient: false,
                    visible: true,
                    show_icon: true,
                    blend: false,
                },
            }))
            .await
            .unwrap();
        assert_eq!(
            probe.world.entity_effect(123, 3).unwrap().duration_ticks,
            200
        );
        probe
            .handle_play_packet(PlayClientbound::DamageEvent(DamageEvent {
                entity_id: 123,
                source_type_id: 5,
                source_cause_id: 456,
                source_direct_id: 300,
                source_position: Some(ProtocolVec3d {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                }),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RemoveMobEffect(RemoveMobEffect {
                entity_id: 123,
                effect_id: 3,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RemoveEntities(RemoveEntities {
                entity_ids: vec![456, 404],
            }))
            .await
            .unwrap();

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });
        let entity = report.world.probe_entity(123).unwrap();
        assert_eq!(entity.position.x, 0.5);
        assert_eq!(entity.position.y, 70.0);
        assert_eq!(entity.position.z, -4.0);
        assert_eq!(entity.delta_movement.y, 0.2);
        assert_eq!(entity.y_rot, 10.0);
        assert_eq!(entity.x_rot, -90.0);
        assert_eq!(entity.on_ground, Some(true));
        assert_eq!(
            entity.data_values,
            vec![
                EntityDataValue {
                    data_id: 0,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(0x20),
                },
                EntityDataValue {
                    data_id: 2,
                    serializer_id: 1,
                    value: EntityDataValueKind::Int(301),
                },
            ]
        );
        assert_eq!(
            entity.equipment,
            vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: item_stack(42, 1),
            }]
        );
        assert_eq!(
            entity.attributes,
            vec![AttributeSnapshot {
                attribute_id: 21,
                base: 20.0,
                modifiers: vec![AttributeModifier {
                    id: "minecraft:health_bonus".to_string(),
                    amount: 4.0,
                    operation_id: 0,
                }],
            }]
        );
        assert_eq!(entity.leash_holder_id, None);
        assert!(entity.passengers.is_empty());
        assert!(report.world.probe_entity(456).is_none());
        assert_eq!(
            report.world.probe_entity(300).unwrap().data_values,
            vec![item_stack_entity_data(item_stack(99, 1))]
        );
        assert!(report.world.entity_effect(123, 3).is_none());
        let damage = report.world.entity_last_damage(123).unwrap();
        assert_eq!(damage.source_type_id, 5);
        assert_eq!(damage.source_cause_id, 456);
        assert_eq!(damage.source_direct_id, 300);
        assert_eq!(
            damage.source_position,
            Some(ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );

        assert_eq!(report.world_counters.entities_received, 3);
        assert_eq!(report.world_counters.entities_tracked, 2);
        assert_eq!(report.world_counters.entity_position_syncs_received, 1);
        assert_eq!(report.world_counters.entity_position_syncs_applied, 1);
        assert_eq!(report.world_counters.entity_position_syncs_ignored, 0);
        assert_eq!(report.world_counters.entity_moves_received, 1);
        assert_eq!(report.world_counters.entity_moves_applied, 1);
        assert_eq!(report.world_counters.entity_moves_ignored, 0);
        assert_eq!(report.world_counters.entity_teleports_received, 1);
        assert_eq!(report.world_counters.entity_teleports_applied, 1);
        assert_eq!(report.world_counters.entity_teleports_ignored, 0);
        assert_eq!(report.world_counters.entity_data_updates_received, 2);
        assert_eq!(report.world_counters.entity_data_values_received, 3);
        assert_eq!(report.world_counters.entity_data_updates_applied, 2);
        assert_eq!(report.world_counters.entity_data_updates_ignored, 0);
        assert_eq!(report.world_counters.entity_equipment_updates_received, 1);
        assert_eq!(report.world_counters.entity_equipment_slots_received, 1);
        assert_eq!(report.world_counters.entity_equipment_updates_applied, 1);
        assert_eq!(report.world_counters.entity_equipment_updates_ignored, 0);
        assert_eq!(report.world_counters.entity_attribute_updates_received, 1);
        assert_eq!(report.world_counters.entity_attributes_received, 1);
        assert_eq!(report.world_counters.entity_attribute_updates_applied, 1);
        assert_eq!(report.world_counters.entity_attribute_updates_ignored, 0);
        assert_eq!(report.world_counters.entity_link_updates_received, 1);
        assert_eq!(report.world_counters.entity_link_updates_applied, 1);
        assert_eq!(report.world_counters.entity_link_updates_ignored, 0);
        assert_eq!(report.world_counters.entity_passenger_updates_received, 1);
        assert_eq!(report.world_counters.entity_passenger_ids_received, 1);
        assert_eq!(report.world_counters.entity_passenger_updates_applied, 1);
        assert_eq!(report.world_counters.entity_passenger_updates_ignored, 0);
        assert_eq!(report.world_counters.take_item_entities_received, 1);
        assert_eq!(report.world_counters.take_item_entities_applied, 1);
        assert_eq!(report.world_counters.take_item_entities_ignored, 0);
        assert_eq!(report.world_counters.item_entity_stack_shrinks, 1);
        assert_eq!(report.world_counters.take_item_entities_removed, 0);
        assert_eq!(report.world_counters.update_mob_effect_packets, 1);
        assert_eq!(report.world_counters.update_mob_effects_ignored, 0);
        assert_eq!(report.world_counters.remove_mob_effect_packets, 1);
        assert_eq!(report.world_counters.remove_mob_effects_ignored, 0);
        assert_eq!(report.world_counters.active_mob_effects_tracked, 0);
        assert_eq!(report.world_counters.damage_event_packets, 1);
        assert_eq!(report.world_counters.damage_events_applied, 1);
        assert_eq!(report.world_counters.damage_events_ignored, 0);
        assert_eq!(report.world_counters.entity_removes_received, 2);
        assert_eq!(report.world_counters.entities_removed, 1);
        assert_eq!(report.world_counters.entity_removes_ignored, 1);
    }

    #[tokio::test]
    async fn probe_projectile_power_updates_world_entity_state_and_counters() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
                123,
                VANILLA_ENTITY_TYPE_FIREBALL_ID,
            )))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity_with_type(
                456, 7,
            )))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
                entity_id: 123,
                acceleration_power: 0.75,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
                entity_id: 456,
                acceleration_power: 0.25,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ProjectilePower(ProjectilePower {
                entity_id: 404,
                acceleration_power: 0.5,
            }))
            .await
            .unwrap();

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.hurting_projectile(123),
            Some(bbb_world::HurtingProjectileState {
                acceleration_power: 0.75,
            })
        );
        assert_eq!(report.world.hurting_projectile(456), None);
        assert_eq!(report.world_counters.projectile_power_packets, 3);
        assert_eq!(report.world_counters.projectile_power_updates_applied, 1);
        assert_eq!(report.world_counters.projectile_power_updates_ignored, 2);
        assert_eq!(
            report.world.last_projectile_power_update(),
            Some(&bbb_world::ProjectilePowerUpdateState {
                entity_id: 404,
                acceleration_power: 0.5,
                applied: false,
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_waypoint_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let waypoint_id = Uuid::from_u128(0x00112233445566778899aabbccddeeff);

        probe
            .handle_play_packet(PlayClientbound::Waypoint(TrackedWaypointPacket {
                operation: WaypointOperation::Track,
                waypoint: TrackedWaypoint {
                    identifier: WaypointIdentifier::Uuid(waypoint_id),
                    icon: WaypointIcon {
                        style: "minecraft:default".to_string(),
                        color_rgb: Some(0x112233),
                    },
                    data: WaypointData::Position(WaypointVec3i {
                        x: 12,
                        y: 70,
                        z: -8,
                    }),
                },
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        let key = format!("uuid:{waypoint_id}");
        let waypoint = report
            .world
            .tracked_waypoints()
            .get(&key)
            .expect("waypoint should be tracked");

        assert_eq!(report.world_counters.waypoint_packets, 1);
        assert_eq!(report.world_counters.waypoints_tracked, 1);
        assert_eq!(waypoint.identifier_kind, "uuid");
        assert_eq!(waypoint.identifier, waypoint_id.to_string());
        assert_eq!(waypoint.icon_style, "minecraft:default");
        assert_eq!(waypoint.icon_color_rgb, Some(0x112233));
        assert_eq!(waypoint.data.kind, "position");
        assert_eq!(
            waypoint.data.position,
            Some(bbb_world::WaypointVec3iState {
                x: 12,
                y: 70,
                z: -8,
            })
        );
        assert_eq!(
            report
                .world
                .last_waypoint_event()
                .map(|event| event.applied),
            Some(true)
        );
    }

    #[tokio::test]
    async fn probe_applies_client_ui_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::LowDiskSpaceWarning)
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ShowDialog(ShowDialog {
                dialog: DialogHolder::Direct {
                    raw_dialog_payload: vec![0xaa, 0xbb, 0xcc],
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ClearDialog)
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::MountScreenOpen(MountScreenOpen {
                container_id: 11,
                inventory_columns: 5,
                entity_id: 42,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::OpenBook(OpenBook {
                hand: InteractionHand::OffHand,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::OpenSignEditor(OpenSignEditor {
                pos: ProtocolBlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                is_front_text: false,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlaceGhostRecipe(PlaceGhostRecipe {
                container_id: 9,
                recipe_display: packets::RecipeDisplaySummary {
                    display_type: packets::RecipeDisplayType::Stonecutter,
                    raw_body: vec![3, 4, 100, 4, 101, 4, 102],
                    crafting: None,
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PongResponse(PongResponse {
                time: 123456789,
            }))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });
        let ui = report.world.client_ui();

        assert_eq!(ui.low_disk_space_warning_count, 1);
        assert_eq!(ui.current_dialog, None);
        assert_eq!(
            ui.last_mount_screen,
            Some(bbb_world::MountScreenState {
                container_id: 11,
                inventory_columns: 5,
                entity_id: 42,
            })
        );
        assert_eq!(
            ui.last_open_book,
            Some(bbb_world::OpenBookState {
                hand: "off_hand".to_string()
            })
        );
        assert_eq!(
            ui.last_open_sign_editor,
            Some(bbb_world::OpenSignEditorState {
                pos: BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                is_front_text: false,
            })
        );
        assert_eq!(
            ui.last_ghost_recipe,
            Some(bbb_world::GhostRecipeState {
                container_id: 9,
                recipe_display_type_id: 3,
                recipe_display_type: "stonecutter".to_string(),
                recipe_display_body_len: 7,
                recipe_display: Some(packets::RecipeDisplaySummary {
                    display_type: packets::RecipeDisplayType::Stonecutter,
                    raw_body: vec![3, 4, 100, 4, 101, 4, 102],
                    crafting: None,
                }),
            })
        );
        assert_eq!(
            ui.last_pong_response,
            Some(bbb_world::PongResponseState { time: 123456789 })
        );
        assert_eq!(report.world_counters.low_disk_space_warnings, 1);
        assert_eq!(report.world_counters.show_dialog_packets, 1);
        assert_eq!(report.world_counters.clear_dialog_packets, 1);
        assert_eq!(report.world_counters.mount_screen_open_packets, 1);
        assert_eq!(report.world_counters.open_book_packets, 1);
        assert_eq!(report.world_counters.open_sign_editor_packets, 1);
        assert_eq!(report.world_counters.ghost_recipe_packets, 1);
        assert_eq!(report.world_counters.pong_response_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_recipe_book_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let settings = RecipeBookSettings {
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
        };

        probe
            .handle_play_packet(PlayClientbound::RecipeBookAdd(RecipeBookAdd {
                replace: true,
                entries: vec![
                    probe_recipe_entry(7, true, true),
                    probe_recipe_entry(8, false, false),
                ],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RecipeBookAdd(RecipeBookAdd {
                replace: false,
                entries: vec![probe_recipe_entry(9, true, false)],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RecipeBookRemove(RecipeBookRemove {
                recipe_ids: vec![RecipeDisplayId { index: 7 }],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::RecipeBookSettings(settings))
            .await
            .unwrap();

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });
        let recipe_book = report.world.recipe_book();

        assert!(!recipe_book.known.contains_key(&7));
        assert_eq!(recipe_book.known.get(&8).unwrap().category_id, 18);
        assert_eq!(recipe_book.known.get(&9).unwrap().category_id, 19);
        assert!(recipe_book.highlights.is_empty());
        assert_eq!(recipe_book.notification_ids, vec![9]);
        assert_eq!(recipe_book.settings, settings);

        assert_eq!(report.world_counters.recipe_book_add_packets, 2);
        assert_eq!(report.world_counters.recipe_book_replace_packets, 1);
        assert_eq!(report.world_counters.recipe_book_remove_packets, 1);
        assert_eq!(report.world_counters.recipe_book_settings_packets, 1);
        assert_eq!(report.world_counters.recipe_book_entries_received, 3);
        assert_eq!(
            report.world_counters.recipe_book_removed_entries_received,
            1
        );
        assert_eq!(report.world_counters.recipe_book_entries_tracked, 2);
        assert_eq!(report.world_counters.recipe_book_highlights_tracked, 0);
        assert_eq!(report.world_counters.recipe_book_notifications_received, 2);
    }

    #[tokio::test]
    async fn probe_applies_advancement_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::UpdateAdvancements(UpdateAdvancements {
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
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SelectAdvancementsTab(
                SelectAdvancementsTab {
                    tab: Some("minecraft:story/root".to_string()),
                },
            ))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.selected_advancements_tab(),
            Some("minecraft:story/root")
        );
        assert!(report
            .world
            .advancements()
            .advancements
            .contains_key("minecraft:story/root"));
        let progress = report
            .world
            .advancements()
            .progress
            .get("minecraft:story/root")
            .unwrap();
        assert_eq!(progress.criteria.len(), 2);

        assert_eq!(report.world_counters.update_advancements_packets, 1);
        assert_eq!(report.world_counters.update_advancements_reset_packets, 1);
        assert_eq!(report.world_counters.update_advancements_show_packets, 1);
        assert_eq!(report.world_counters.advancements_added_received, 1);
        assert_eq!(report.world_counters.advancements_removed_received, 0);
        assert_eq!(report.world_counters.advancements_adds_ignored, 0);
        assert_eq!(report.world_counters.advancement_progress_received, 1);
        assert_eq!(
            report.world_counters.advancement_progress_updates_ignored,
            0
        );
        assert_eq!(report.world_counters.advancements_tracked, 1);
        assert_eq!(report.world_counters.advancement_roots_tracked, 1);
        assert_eq!(report.world_counters.advancement_progress_tracked, 1);
        assert_eq!(
            report.world_counters.advancement_progress_criteria_tracked,
            2
        );
        assert_eq!(report.world_counters.select_advancements_tab_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_update_recipes_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::UpdateRecipes(UpdateRecipes {
                property_sets: vec![
                    RecipePropertySetSummary {
                        key: "minecraft:furnace_input".to_string(),
                        item_ids: vec![42, 43],
                    },
                    RecipePropertySetSummary {
                        key: "minecraft:smithing_base".to_string(),
                        item_ids: vec![99],
                    },
                ],
                stonecutter_recipes: vec![StonecutterSelectableRecipeSummary {
                    input: IngredientSummary {
                        tag: None,
                        item_ids: vec![11, 12],
                    },
                    option_display: SlotDisplaySummary {
                        display_type_id: 4,
                        raw_payload: vec![4, 77],
                        item_stack: None,
                        tag: None,
                    },
                }],
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        let recipes = report.world.recipes();

        assert_eq!(
            recipes.property_sets.get("minecraft:furnace_input"),
            Some(&vec![42, 43])
        );
        assert_eq!(
            recipes.property_sets.get("minecraft:smithing_base"),
            Some(&vec![99])
        );
        assert_eq!(recipes.stonecutter_recipes.len(), 1);
        assert_eq!(recipes.stonecutter_recipes[0].input.item_ids, vec![11, 12]);
        assert_eq!(
            recipes.stonecutter_recipes[0]
                .option_display
                .display_type_id,
            4
        );
        assert_eq!(
            recipes.stonecutter_recipes[0].option_display.raw_payload,
            vec![4, 77]
        );

        assert_eq!(report.world_counters.update_recipes_packets, 1);
        assert_eq!(report.world_counters.recipe_property_sets_tracked, 2);
        assert_eq!(report.world_counters.recipe_property_set_items_tracked, 3);
        assert_eq!(report.world_counters.stonecutter_recipes_tracked, 1);
    }

    #[tokio::test]
    async fn probe_applies_world_time_weather_and_ticking_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetTime(PlayTime {
                game_time: 123,
                clock_updates: vec![ClockUpdate {
                    clock_id: 0,
                    total_ticks: 6000,
                    partial_tick: 0.25,
                    rate: 1.0,
                }],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 7,
                param: 0.5,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 8,
                param: 0.75,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 3,
                param: 2.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 11,
                param: 1.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::GameEvent(GameEvent {
                event_id: 12,
                param: 1.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TickingState(TickingState {
                tick_rate: 0.25,
                frozen: true,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TickingStep(TickingStep { tick_steps: 7 }))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });
        let time = report.world.world_time().unwrap();

        assert_eq!(time.game_time, 123);
        assert_eq!(time.day_time, 6000);
        assert_eq!(
            time.clock_updates,
            vec![bbb_world::ClockUpdateState {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.25,
                rate: 1.0,
            }]
        );
        assert_eq!(
            report.world.weather(),
            bbb_world::WorldWeatherState {
                raining: true,
                rain_level: 0.5,
                thunder_level: 0.75,
                last_game_event_id: Some(12),
                last_game_event_param: 1.0,
            }
        );
        assert_eq!(report.world.gameplay().game_type, 2);
        assert_eq!(report.world.gameplay().game_type_name, "adventure");
        assert_eq!(report.world.gameplay().previous_game_type, Some(0));
        assert!(!report.world.gameplay().show_death_screen);
        assert!(report.world.gameplay().do_limited_crafting);
        assert_eq!(
            report.world.ticking(),
            bbb_world::WorldTickingState {
                tick_rate: 1.0,
                frozen: true,
                frozen_ticks_to_run: 7,
            }
        );
        assert_eq!(report.world_counters.world_time_packets, 1);
        assert_eq!(report.world_counters.game_event_packets, 5);
        assert_eq!(report.world_counters.ticking_state_packets, 1);
        assert_eq!(report.world_counters.ticking_step_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_world_border_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::InitializeBorder(InitializeBorder {
                new_center_x: 1.0,
                new_center_z: 2.0,
                old_size: 100.0,
                new_size: 200.0,
                lerp_time: 30,
                new_absolute_max_size: 500,
                warning_blocks: 6,
                warning_time: 7,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetBorderCenter(SetBorderCenter {
                new_center_x: 3.0,
                new_center_z: 4.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetBorderLerpSize(SetBorderLerpSize {
                old_size: 200.0,
                new_size: 300.0,
                lerp_time: 50,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetBorderSize(SetBorderSize {
                size: 250.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetBorderWarningDelay(
                SetBorderWarningDelay { warning_delay: 9 },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetBorderWarningDistance(
                SetBorderWarningDistance { warning_blocks: 8 },
            ))
            .await
            .unwrap();

        let report = probe.finish(7, ChunkPos { x: 0, z: 0 });
        let border = report.world.world_border();

        assert_eq!(border.center_x, 3.0);
        assert_eq!(border.center_z, 4.0);
        assert_eq!(border.size, 250.0);
        assert_eq!(border.lerp_target, 250.0);
        assert_eq!(border.lerp_time, 0);
        assert_eq!(border.absolute_max_size, 500);
        assert_eq!(border.warning_blocks, 8);
        assert_eq!(border.warning_time, 9);

        assert_eq!(report.world_counters.world_border_initializes_received, 1);
        assert_eq!(
            report.world_counters.world_border_center_updates_received,
            1
        );
        assert_eq!(
            report
                .world_counters
                .world_border_lerp_size_updates_received,
            1
        );
        assert_eq!(report.world_counters.world_border_size_updates_received, 1);
        assert_eq!(
            report
                .world_counters
                .world_border_warning_delay_updates_received,
            1
        );
        assert_eq!(
            report
                .world_counters
                .world_border_warning_distance_updates_received,
            1
        );
    }

    #[tokio::test]
    async fn probe_open_book_uses_held_written_book_for_active_screen() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetPlayerInventory(SetPlayerInventory {
                slot: 40,
                item: written_book_stack(vec!["Probe first", "Probe second"]),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::OpenBook(OpenBook {
                hand: InteractionHand::OffHand,
            }))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.last_open_book(),
            Some(&bbb_world::OpenBookState {
                hand: "off_hand".to_string(),
            })
        );
        assert_eq!(
            report.world.current_book(),
            Some(&bbb_world::BookScreenState {
                hand: "off_hand".to_string(),
                pages: vec!["Probe first".to_string(), "Probe second".to_string()],
                current_page: 0,
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_hud_text_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SystemChat(SystemChat {
                content: "Server restart soon".to_string(),
                overlay: false,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SystemChat(SystemChat {
                content: "Now entering camp".to_string(),
                overlay: true,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetActionBarText(SetActionBarText {
                content: "Action ready".to_string(),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetTitlesAnimation(SetTitlesAnimation {
                fade_in: 5,
                stay: 40,
                fade_out: 15,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetTitleText(SetTitleText {
                content: "Quest complete".to_string(),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetSubtitleText(SetSubtitleText {
                content: "Return to camp".to_string(),
            }))
            .await
            .unwrap();

        let system_chat = probe.world.system_chat().unwrap();
        assert_eq!(system_chat.content, "Now entering camp");
        assert!(system_chat.overlay);
        let action_bar = probe.world.action_bar().unwrap();
        assert_eq!(action_bar.content, "Action ready");
        assert_eq!(action_bar.display_ticks, 60);
        assert_eq!(probe.world.title().title.as_deref(), Some("Quest complete"));
        assert_eq!(
            probe.world.title().subtitle.as_deref(),
            Some("Return to camp")
        );
        assert_eq!(probe.world.title().fade_in, 5);
        assert_eq!(probe.world.title().stay, 40);
        assert_eq!(probe.world.title().fade_out, 15);
        assert_eq!(probe.world.title().title_time, 60);

        probe
            .handle_play_packet(PlayClientbound::ClearTitles(ClearTitles {
                reset_times: true,
            }))
            .await
            .unwrap();

        let report = probe.finish(7, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world.title().title, None);
        assert_eq!(report.world.title().subtitle, None);
        assert_eq!(report.world.title().fade_in, 10);
        assert_eq!(report.world.title().stay, 70);
        assert_eq!(report.world.title().fade_out, 20);
        assert_eq!(report.world.title().title_time, 0);
        assert_eq!(report.world_counters.system_chat_packets, 2);
        assert_eq!(report.world_counters.action_bar_packets, 1);
        assert_eq!(report.world_counters.titles_animation_packets, 1);
        assert_eq!(report.world_counters.title_text_packets, 1);
        assert_eq!(report.world_counters.subtitle_text_packets, 1);
        assert_eq!(report.world_counters.clear_titles_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_hud_session_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let boss_id = Uuid::from_u128(1);

        probe
            .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
                id: boss_id,
                operation: BossEventOperation::Add {
                    name: "Ender Dragon".to_string(),
                    progress: 0.75,
                    color: BossBarColor::Purple,
                    overlay: BossBarOverlay::Progress,
                    flags: BossEventFlags {
                        darken_screen: true,
                        play_music: false,
                        create_world_fog: true,
                    },
                },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
                id: boss_id,
                operation: BossEventOperation::UpdateProgress { progress: 0.25 },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BossEvent(BossEvent {
                id: Uuid::from_u128(99),
                operation: BossEventOperation::UpdateProgress { progress: 1.0 },
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::TabList(TabList {
                header: Some("Welcome".to_string()),
                footer: None,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ChangeDifficulty(ChangeDifficulty {
                difficulty: Difficulty::Hard,
                locked: true,
            }))
            .await
            .unwrap();

        let report = probe.finish(5, ChunkPos { x: 0, z: 0 });
        let boss = report.world.boss_bars().get(&boss_id).unwrap();

        assert_eq!(boss.name, "Ender Dragon");
        assert_eq!(boss.progress, 0.25);
        assert_eq!(boss.color, "purple");
        assert_eq!(boss.overlay, "progress");
        assert!(boss.darken_screen);
        assert!(boss.create_world_fog);
        assert_eq!(report.world.tab_list().header.as_deref(), Some("Welcome"));
        assert_eq!(report.world.tab_list().footer, None);
        assert_eq!(report.world.difficulty().difficulty, "hard");
        assert!(report.world.difficulty().difficulty_locked);

        assert_eq!(report.world_counters.boss_event_packets, 3);
        assert_eq!(report.world_counters.boss_bars_tracked, 1);
        assert_eq!(report.world_counters.boss_events_ignored, 1);
        assert_eq!(report.world_counters.tab_list_packets, 1);
        assert_eq!(report.world_counters.change_difficulty_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_scoreboard_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetObjective(SetObjective {
                objective_name: "kills".to_string(),
                method: SetObjectiveMethod::Add,
                parameters: Some(SetObjectiveParameters {
                    display_name: "Kills".to_string(),
                    render_type: ObjectiveRenderType::Integer,
                    number_format: Some(vec![9]),
                }),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetDisplayObjective(SetDisplayObjective {
                slot: ScoreboardDisplaySlot::Sidebar,
                objective_name: Some("kills".to_string()),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetScore(SetScore {
                owner: "Steve".to_string(),
                objective_name: "kills".to_string(),
                score: 4,
                display: Some("Four".to_string()),
                number_format: None,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetScore(SetScore {
                owner: "Alex".to_string(),
                objective_name: "kills".to_string(),
                score: 1,
                display: None,
                number_format: None,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Add,
                parameters: Some(PlayerTeamParameters {
                    display_name: "Red Team".to_string(),
                    options: 0b11,
                    nametag_visibility: TeamVisibility::Always,
                    collision_rule: TeamCollisionRule::Never,
                    color: ChatFormatting::Red,
                    player_prefix: "[R]".to_string(),
                    player_suffix: "!".to_string(),
                }),
                players: vec!["Steve".to_string()],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ResetScore(ResetScore {
                owner: "Alex".to_string(),
                objective_name: Some("kills".to_string()),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetObjective(SetObjective {
                objective_name: "missing".to_string(),
                method: SetObjectiveMethod::Change,
                parameters: Some(SetObjectiveParameters {
                    display_name: "Missing".to_string(),
                    render_type: ObjectiveRenderType::Integer,
                    number_format: None,
                }),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetDisplayObjective(SetDisplayObjective {
                slot: ScoreboardDisplaySlot::List,
                objective_name: Some("missing".to_string()),
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetScore(SetScore {
                owner: "Nobody".to_string(),
                objective_name: "missing".to_string(),
                score: 9,
                display: None,
                number_format: None,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "missing".to_string(),
                method: PlayerTeamMethod::Join,
                parameters: None,
                players: vec!["Nobody".to_string()],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::ResetScore(ResetScore {
                owner: "Nobody".to_string(),
                objective_name: Some("missing".to_string()),
            }))
            .await
            .unwrap();

        let report = probe.finish(11, ChunkPos { x: 0, z: 0 });
        let counters = &report.world_counters;
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

        let scoreboard = report.world.scoreboard();
        let objective = scoreboard.objectives.get("kills").unwrap();
        assert_eq!(objective.display_name, "Kills");
        assert_eq!(objective.render_type, "integer");
        assert_eq!(objective.number_format, Some(vec![9]));
        assert_eq!(
            scoreboard.display_slots.get("sidebar").map(String::as_str),
            Some("kills")
        );

        let steve_score = &scoreboard.scores["Steve"]["kills"];
        assert_eq!(steve_score.value, 4);
        assert_eq!(steve_score.display.as_deref(), Some("Four"));
        assert!(!scoreboard.scores.contains_key("Alex"));

        let team = scoreboard.teams.get("red").unwrap();
        assert!(team.players.contains("Steve"));
        let parameters = team.parameters.as_ref().unwrap();
        assert_eq!(parameters.display_name, "Red Team");
        assert_eq!(parameters.color, "red");
    }

    #[tokio::test]
    async fn probe_applies_block_changed_ack_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::BlockChangedAck(BlockChangedAck {
                sequence: 17,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.block_changed_ack_packets, 1);
        assert_eq!(
            report.world.last_block_changed_ack(),
            Some(&bbb_world::BlockChangedAckState { sequence: 17 })
        );
    }

    #[tokio::test]
    async fn probe_applies_block_and_level_events_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::BlockEvent(BlockEvent {
                pos: ProtocolBlockPos {
                    x: 12,
                    y: 65,
                    z: -5,
                },
                b0: 2,
                b1: 9,
                block_id: 54,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 1001,
                pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
                data: 42,
                global: true,
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.block_events_received, 1);
        assert_eq!(report.world_counters.block_events_tracked, 1);
        assert_eq!(report.world_counters.level_events_received, 1);
        assert_eq!(report.world_counters.level_events_tracked, 1);

        let block_event = report.world.block_events().first().unwrap();
        assert_eq!(
            block_event.pos,
            BlockPos {
                x: 12,
                y: 65,
                z: -5,
            }
        );
        assert_eq!(block_event.b0, 2);
        assert_eq!(block_event.b1, 9);
        assert_eq!(block_event.block_id, 54);

        let level_event = report.world.level_events().first().unwrap();
        assert_eq!(level_event.event_type, 1001);
        assert_eq!(level_event.pos, BlockPos { x: 3, y: 4, z: 5 });
        assert_eq!(level_event.data, 42);
        assert!(level_event.global);
    }

    #[tokio::test]
    async fn probe_records_portal_travel_level_event_as_local_sound() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 1032,
                pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
                data: 0,
                global: false,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.level_events_received, 1);
        let sound = report.world.last_local_sound().unwrap();
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:block.portal.travel")
        );
        assert_eq!(sound.source, "ambient");
        assert_close(sound.volume, 0.25);
        assert_close(sound.pitch, 1.092_387_1);
        assert_eq!(sound.seed, 0);
    }

    #[tokio::test]
    async fn probe_records_positioned_level_event_sound_in_world_audio_state() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 1015,
                pos: ProtocolBlockPos { x: -4, y: 70, z: 9 },
                data: 0,
                global: false,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.level_events_received, 1);
        assert_eq!(report.world_counters.sound_packets, 0);
        let sound = report.world.last_sound().unwrap();
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:entity.ghast.warn")
        );
        assert_eq!(sound.source, "hostile");
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: -3.5,
                y: 70.5,
                z: 9.5,
            }
        );
        assert_close(sound.volume, 10.0);
        assert_close(sound.pitch, 0.979_905_37);
        assert_eq!(sound.seed, 4_437_113_781_045_784_766);
    }

    #[tokio::test]
    async fn probe_uses_loaded_sculk_charge_pop_shape_context_for_level_event_randoms() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let event_pos = ProtocolBlockPos {
            x: 16,
            y: -64,
            z: -32,
        };
        let stone_id = vanilla_block_state_id("minecraft:stone", []);

        probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(
                synthetic_probe_level_chunk_packet(),
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BlockUpdate(BlockUpdate {
                pos: event_pos,
                block_state_id: stone_id,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 3006,
                pos: event_pos,
                data: 0,
                global: false,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 1004,
                pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
                data: 0,
                global: false,
            }))
            .await
            .unwrap();

        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let _sculk_sound_seed = expected_random.next_long();
        for _ in 0..40 {
            let _ = expected_random.next_float();
            let _ = expected_random.next_float();
            let _ = expected_random.next_float();
        }
        let expected_followup_seed = expected_random.next_long();

        let report = probe.finish(4, ChunkPos { x: 1, z: -2 });
        assert_eq!(report.world_counters.level_events_received, 2);
        let sound = report.world.last_sound().unwrap();
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:entity.firework_rocket.shoot")
        );
        assert_eq!(sound.seed, expected_followup_seed);
    }

    #[tokio::test]
    async fn probe_uses_loaded_growth_context_for_level_event_randoms() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let event_pos = ProtocolBlockPos {
            x: 16,
            y: -63,
            z: -32,
        };
        let water_id = vanilla_block_state_id("minecraft:water", [("level", "0")]);

        probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(
                synthetic_probe_level_chunk_packet(),
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BlockUpdate(BlockUpdate {
                pos: event_pos,
                block_state_id: water_id,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 1505,
                pos: event_pos,
                data: 3,
                global: false,
            }))
            .await
            .unwrap();

        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        advance_growth_level_event_particle_randoms(
            3,
            LevelEventGrowthRandomMode::WideNoFloating,
            &mut expected_random,
        );
        let expected_seed = expected_random.next_long();

        let report = probe.finish(3, ChunkPos { x: 1, z: -2 });
        assert_eq!(report.world_counters.level_events_received, 1);
        let sound = report.world.last_sound().unwrap();
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:item.bone_meal.use")
        );
        assert_eq!(sound.seed, expected_seed);
    }

    #[tokio::test]
    async fn probe_records_cobweb_place_sound_after_particle_randoms() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 3018,
                pos: ProtocolBlockPos { x: 2, y: 64, z: -5 },
                data: 0,
                global: false,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.level_events_received, 1);
        assert_eq!(report.world_counters.sound_packets, 0);
        let sound = report.world.last_sound().unwrap();
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:block.cobweb.place")
        );
        assert_eq!(sound.source, "block");
        assert_eq!(
            sound.position,
            ProtocolVec3d {
                x: 2.5,
                y: 64.5,
                z: -4.5,
            }
        );
        assert_close(sound.volume, 1.0);
        assert_close(sound.pitch, 1.013_698_2);
        assert_eq!(sound.seed, 536_938_910_405_906_015);
        assert!(sound.distance_delay);
    }

    #[tokio::test]
    async fn probe_records_global_level_event_sound_when_camera_pose_is_known() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 0.5,
                y: -1.12,
                z: 0.5,
            },
            ..LocalPlayerPoseState::default()
        });

        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 1028,
                pos: ProtocolBlockPos { x: 10, y: 0, z: 0 },
                data: 0,
                global: true,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.level_events_received, 1);
        assert_eq!(report.world_counters.sound_packets, 0);
        let sound = report.world.last_sound().unwrap();
        assert_eq!(
            sound.sound.location.as_deref(),
            Some("minecraft:entity.ender_dragon.death")
        );
        assert_eq!(sound.source, "hostile");
        assert!((sound.position.x - 2.5).abs() < 1.0e-6);
        assert!((sound.position.y - 0.5).abs() < 1.0e-6);
        assert!((sound.position.z - 0.5).abs() < 1.0e-6);
        assert_close(sound.volume, 5.0);
        assert_close(sound.pitch, 1.0);
        assert_eq!(sound.seed, -4_962_768_465_676_381_896);
    }

    #[tokio::test]
    async fn probe_records_jukebox_level_events_in_world_audio_state() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::LevelEvent(LevelEvent {
                event_type: 1010,
                pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
                data: 27,
                global: false,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.level_events_received, 1);
        assert_eq!(report.world.playing_jukebox_songs().len(), 1);
        let song = report.world.playing_jukebox_songs()[0];
        assert_eq!(song.pos, BlockPos { x: 3, y: 4, z: 5 });
        assert_eq!(song.song_registry_id, 27);
        let event = report.world.last_jukebox_event().unwrap();
        assert_eq!(event.action, bbb_world::JukeboxLevelEventAction::Start);
        assert_eq!(event.song_registry_id, Some(27));
    }

    #[tokio::test]
    async fn probe_applies_player_look_at_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::PlayerLookAt(PlayerLookAt {
                from_anchor: EntityAnchor::Eyes,
                position: ProtocolVec3d {
                    x: 12.0,
                    y: 65.0,
                    z: -7.0,
                },
                target: None,
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.player_look_at_packets, 1);
        assert_eq!(
            report.world.local_player().last_look_at,
            Some(bbb_world::LocalPlayerLookAtState {
                from_anchor: EntityAnchor::Eyes,
                position: ProtocolVec3d {
                    x: 12.0,
                    y: 65.0,
                    z: -7.0,
                },
                target_entity_id: None,
                to_anchor: None,
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_player_combat_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::PlayerCombatEnter)
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlayerCombatEnd(PlayerCombatEnd {
                duration: 37,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlayerCombatKill(PlayerCombatKill {
                player_id: 123,
                message: "You died".to_string(),
            }))
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });

        assert_eq!(report.world_counters.player_combat_enter_packets, 1);
        assert_eq!(report.world_counters.player_combat_end_packets, 1);
        assert_eq!(report.world_counters.player_combat_kill_packets, 1);
        assert_eq!(
            report.world.last_player_combat(),
            Some(&bbb_world::PlayerCombatEventState {
                kind: "kill".to_string(),
                duration: None,
                player_id: Some(123),
                message: Some("You died".to_string()),
            })
        );
    }

    #[tokio::test]
    async fn probe_applies_player_health_position_and_rotation_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
                health: 7.5,
                food: 16,
                saturation: 2.0,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlayerPosition(PlayerPositionUpdate {
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
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlayerRotation(PlayerRotationUpdate {
                y_rot: 10.0,
                relative_y: true,
                x_rot: -5.0,
                relative_x: false,
            }))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });
        let local = report.world.local_player();
        assert_eq!(
            local.health,
            Some(bbb_world::LocalPlayerHealthState {
                health: 7.5,
                food: 16,
                saturation: 2.0,
            })
        );
        assert_eq!(report.world_counters.player_health_packets, 1);
        assert_eq!(report.world_counters.player_position_packets, 1);
        assert_eq!(report.world_counters.player_rotation_packets, 1);

        let pose = report.world.local_player_pose().unwrap();
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
    }

    #[tokio::test]
    async fn probe_player_position_sends_vanilla_teleport_ack_and_player_loaded_once() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::PlayerPosition(PlayerPositionUpdate {
                id: 17,
                position: ProtocolVec3d {
                    x: 1.25,
                    y: 64.5,
                    z: -8.75,
                },
                delta_movement: ProtocolVec3d::default(),
                y_rot: 90.0,
                x_rot: -15.0,
                relatives_mask: 0,
            }))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("teleport ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 17);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("move player pos/rot ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 1.25);
        assert_eq!(decoder.read_f64().unwrap(), 64.5);
        assert_eq!(decoder.read_f64().unwrap(), -8.75);
        assert_eq!(decoder.read_f32().unwrap(), 90.0);
        assert_eq!(decoder.read_f32().unwrap(), -15.0);
        assert_eq!(decoder.read_u8().unwrap(), 0);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("player loaded should be sent after first position sync")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_LOADED);
        assert!(payload.is_empty());
        assert!(probe.player_loaded_sent);

        probe
            .handle_play_packet(PlayClientbound::PlayerPosition(PlayerPositionUpdate {
                id: 18,
                position: ProtocolVec3d {
                    x: 2.0,
                    y: 70.0,
                    z: -9.0,
                },
                delta_movement: ProtocolVec3d::default(),
                y_rot: 100.0,
                x_rot: 5.0,
                relatives_mask: 0,
            }))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("second teleport ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 18);
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("second move player pos/rot ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 2.0);
        assert_eq!(decoder.read_f64().unwrap(), 70.0);
        assert_eq!(decoder.read_f64().unwrap(), -9.0);
        assert_eq!(decoder.read_f32().unwrap(), 100.0);
        assert_eq!(decoder.read_f32().unwrap(), 5.0);
        assert_eq!(decoder.read_u8().unwrap(), 0);
        assert!(decoder.is_empty());

        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "player loaded must only be sent for the first position sync"
        );

        assert_eq!(
            probe.player_position_state.position,
            ProtocolVec3d {
                x: 2.0,
                y: 70.0,
                z: -9.0,
            }
        );
        assert_eq!(probe.player_position_state.y_rot, 100.0);
        assert_eq!(probe.player_position_state.x_rot, 5.0);
    }

    #[tokio::test]
    async fn probe_player_rotation_sends_vanilla_rot_ack() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe.player_position_state.y_rot = 30.0;
        probe.player_position_state.x_rot = 5.0;
        probe
            .handle_play_packet(PlayClientbound::PlayerRotation(PlayerRotationUpdate {
                y_rot: 15.0,
                relative_y: true,
                x_rot: -10.0,
                relative_x: false,
            }))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("move player rot ack should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_ROT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f32().unwrap(), 45.0);
        assert_eq!(decoder.read_f32().unwrap(), -10.0);
        assert_eq!(decoder.read_u8().unwrap(), 0);
        assert!(decoder.is_empty());
        assert_eq!(probe.player_position_state.y_rot, 45.0);
        assert_eq!(probe.player_position_state.x_rot, -10.0);
    }

    #[tokio::test]
    async fn probe_applies_local_player_status_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::PlayerAbilities(PlayerAbilities {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: false,
                flying_speed: 0.08,
                walking_speed: 0.12,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetExperience(PlayerExperience {
                progress: 0.5,
                level: 12,
                total: 345,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 5 }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 99 }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetDefaultSpawnPosition(
                SetDefaultSpawnPosition {
                    dimension: "minecraft:overworld".to_string(),
                    pos: ProtocolBlockPos {
                        x: -12,
                        y: 70,
                        z: 44,
                    },
                    yaw: 180.0,
                    pitch: -15.0,
                },
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetSimulationDistance(
                SetSimulationDistance { distance: 12 },
            ))
            .await
            .unwrap();

        let report = probe.finish(6, ChunkPos { x: 0, z: 0 });
        let local = report.world.local_player();

        assert_eq!(
            local.abilities,
            Some(bbb_world::LocalPlayerAbilitiesState {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: false,
                flying_speed: 0.08,
                walking_speed: 0.12,
            })
        );
        assert_eq!(
            local.experience,
            Some(bbb_world::LocalPlayerExperienceState {
                progress: 0.5,
                level: 12,
                total: 345,
            })
        );
        assert_eq!(local.selected_hotbar_slot, 5);
        assert_eq!(
            local.default_spawn,
            Some(bbb_world::DefaultSpawnState {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos {
                    x: -12,
                    y: 70,
                    z: 44,
                },
                yaw: 180.0,
                pitch: -15.0,
            })
        );
        assert_eq!(local.simulation_distance, Some(12));
        assert_eq!(report.world_counters.player_abilities_packets, 1);
        assert_eq!(report.world_counters.player_experience_packets, 1);
        assert_eq!(report.world_counters.held_slot_packets, 2);
        assert_eq!(report.world_counters.held_slot_updates_applied, 1);
        assert_eq!(report.world_counters.held_slot_updates_ignored, 1);
        assert_eq!(report.world_counters.default_spawn_position_packets, 1);
        assert_eq!(report.world_counters.simulation_distance_packets, 1);
    }

    #[tokio::test]
    async fn probe_applies_player_info_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let profile_id = Uuid::from_u128(1);
        let removed_profile_id = Uuid::from_u128(2);

        probe
            .handle_play_packet(PlayClientbound::PlayerInfoUpdate(PlayerInfoUpdate {
                actions: vec![
                    PlayerInfoAction::AddPlayer,
                    PlayerInfoAction::InitializeChat,
                    PlayerInfoAction::UpdateGameMode,
                    PlayerInfoAction::UpdateListed,
                    PlayerInfoAction::UpdateLatency,
                    PlayerInfoAction::UpdateDisplayName,
                    PlayerInfoAction::UpdateListOrder,
                    PlayerInfoAction::UpdateHat,
                ],
                entries: vec![
                    PlayerInfoEntry {
                        profile_id,
                        profile: Some(GameProfile {
                            uuid: profile_id,
                            name: "Ada".to_string(),
                            properties: vec![GameProfileProperty {
                                name: "textures".to_string(),
                                value: "skin".to_string(),
                                signature: Some("signature".to_string()),
                            }],
                        }),
                        listed: true,
                        latency: 42,
                        game_mode: GameType::Creative,
                        display_name: Some("Ada Lovelace".to_string()),
                        show_hat: true,
                        list_order: 3,
                        chat_session: Some(PlayerInfoChatSession {
                            session_id: Uuid::from_u128(3),
                            expires_at_epoch_millis: 99,
                            public_key: vec![1, 2],
                            key_signature: vec![3, 4],
                        }),
                    },
                    PlayerInfoEntry {
                        profile_id: removed_profile_id,
                        profile: Some(GameProfile {
                            uuid: removed_profile_id,
                            name: "Removed".to_string(),
                            properties: Vec::new(),
                        }),
                        listed: true,
                        latency: 7,
                        game_mode: GameType::Survival,
                        display_name: None,
                        show_hat: false,
                        list_order: 0,
                        chat_session: None,
                    },
                ],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::PlayerInfoRemove(PlayerInfoRemove {
                profile_ids: vec![removed_profile_id],
            }))
            .await
            .unwrap();

        let report = probe.finish(2, ChunkPos { x: 0, z: 0 });
        let entry = report.world.player_info_entry(profile_id).unwrap();
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
        assert!(report.world.listed_players().contains(&profile_id));
        assert!(report.world.player_info_entry(removed_profile_id).is_none());
        assert!(!report.world.listed_players().contains(&removed_profile_id));

        assert_eq!(report.world_counters.player_info_update_packets, 1);
        assert_eq!(report.world_counters.player_info_remove_packets, 1);
        assert_eq!(report.world_counters.player_info_entries_tracked, 1);
        assert_eq!(report.world_counters.listed_players_tracked, 1);
    }

    #[tokio::test]
    async fn probe_applies_set_camera_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Login(protocol_play_login(9)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetCamera(SetCamera { camera_id: 9 }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetCamera(SetCamera { camera_id: 123 }))
            .await
            .unwrap();

        let report = probe.finish(3, ChunkPos { x: 0, z: 0 });

        assert_eq!(
            report.world.local_player().camera,
            bbb_world::CameraState {
                entity_id: Some(9),
                follows_player: true,
                entity_known: true,
            }
        );
        assert_eq!(report.world_counters.set_camera_packets, 2);
        assert_eq!(report.world_counters.set_camera_updates_applied, 1);
        assert_eq!(report.world_counters.set_camera_updates_ignored, 1);
    }

    #[tokio::test]
    async fn probe_respawn_updates_world_and_clears_level_bound_state_when_dimension_changes() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Login(protocol_play_login(9)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(9)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(55)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetCamera(SetCamera { camera_id: 55 }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
                health: 4.0,
                food: 7,
                saturation: 0.5,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetExperience(PlayerExperience {
                progress: 0.25,
                level: 3,
                total: 40,
            }))
            .await
            .unwrap();
        probe.world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 10.0,
                y: 65.0,
                z: -4.0,
            },
            on_ground: true,
            horizontal_collision: true,
            fall_distance: 2.0,
            ..LocalPlayerPoseState::default()
        });
        probe
            .handle_play_packet(PlayClientbound::LevelChunkWithLight(
                synthetic_probe_level_chunk_packet(),
            ))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::BlockDestruction(
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
            .await
            .unwrap();

        let mut spawn_info = protocol_play_login(9).common_spawn_info;
        spawn_info.dimension_type_id = 1;
        spawn_info.dimension = "minecraft:the_nether".to_string();
        spawn_info.sea_level = 32;
        probe
            .handle_play_packet(PlayClientbound::Respawn(Respawn {
                common_spawn_info: spawn_info,
                data_to_keep: 0,
            }))
            .await
            .unwrap();

        let report = probe.finish(9, ChunkPos { x: 0, z: 0 });
        let level = report.world.level_info().unwrap();

        assert_eq!(level.dimension, "minecraft:the_nether");
        assert_eq!(level.dimension_type_id, 1);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_nether")
        );
        assert_eq!(level.sea_level, 32);
        assert_eq!(report.world.local_player_id(), Some(9));
        assert_eq!(report.world.entity_count(), 0);
        assert_eq!(report.world.chunk_count(), 0);
        assert_eq!(report.world.first_chunk(), None);
        assert_eq!(report.world.local_player().health, None);
        assert_eq!(report.world.local_player().experience, None);
        assert_eq!(
            report.world.local_player().camera,
            bbb_world::CameraState::default()
        );
        assert_eq!(report.world.local_player_pose(), None);
        assert!(report.world.block_destructions().is_empty());

        assert_eq!(report.world_counters.play_logins_received, 1);
        assert_eq!(report.world_counters.respawns_received, 1);
        assert_eq!(report.world_counters.chunks_received, 1);
        assert_eq!(report.world_counters.block_destructions_received, 1);
        assert_eq!(report.world_counters.block_destructions_tracked, 0);
        assert_eq!(report.world_counters.entities_tracked, 0);
    }

    #[tokio::test]
    async fn probe_respawn_keep_all_data_preserves_local_pose_entity_data_and_attribute_modifiers()
    {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::Login(protocol_play_login(9)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::AddEntity(protocol_add_entity(9)))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
                health: 4.0,
                food: 7,
                saturation: 0.5,
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::SetExperience(PlayerExperience {
                progress: 0.25,
                level: 3,
                total: 40,
            }))
            .await
            .unwrap();
        probe.world.set_local_player_pose(LocalPlayerPoseState {
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
        let old_pose = probe.world.local_player_pose().unwrap();
        probe
            .world
            .set_local_destroying_block(BlockPos { x: 1, y: 2, z: 3 });
        probe.world.set_local_using_item(true);
        probe
            .handle_play_packet(PlayClientbound::SetEntityData(SetEntityData {
                id: 9,
                values: vec![EntityDataValue {
                    data_id: 0,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(0x02),
                }],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::UpdateAttributes(UpdateAttributes {
                entity_id: 9,
                attributes: vec![AttributeSnapshot {
                    attribute_id: 21,
                    base: 0.1,
                    modifiers: vec![AttributeModifier {
                        id: "minecraft:test_speed".to_string(),
                        amount: 0.25,
                        operation_id: 1,
                    }],
                }],
            }))
            .await
            .unwrap();
        probe
            .handle_play_packet(PlayClientbound::UpdateMobEffect(UpdateMobEffect {
                entity_id: 9,
                effect_id: 3,
                amplifier: 1,
                duration_ticks: 200,
                flags: MobEffectFlags::default(),
            }))
            .await
            .unwrap();
        assert!(probe.world.entity_effect(9, 3).is_some());

        probe
            .handle_play_packet(PlayClientbound::Respawn(Respawn {
                common_spawn_info: protocol_play_login(9).common_spawn_info,
                data_to_keep: 3,
            }))
            .await
            .unwrap();

        let report = probe.finish(8, ChunkPos { x: 0, z: 0 });

        assert!(report.world.local_player().health.is_none());
        assert!(report.world.local_player().experience.is_none());
        assert_eq!(
            report.world.local_player_pose(),
            Some(LocalPlayerPoseState {
                on_ground: false,
                horizontal_collision: false,
                fall_distance: 0.0,
                ..old_pose
            })
        );
        assert_eq!(
            report.world.local_player().camera,
            bbb_world::CameraState::default()
        );
        assert_eq!(
            report.world.local_player().interaction,
            bbb_world::LocalPlayerInteractionState::default()
        );

        let entity = report.world.probe_entity(9).unwrap();
        assert_eq!(entity.data_values.len(), 1);
        assert_eq!(entity.data_values[0].data_id, 0);
        assert_eq!(entity.data_values[0].serializer_id, 0);
        assert_eq!(entity.data_values[0].value, EntityDataValueKind::Byte(0x02));
        assert!(entity.mob_effects.is_empty());
        assert_eq!(entity.attributes.len(), 1);
        assert_eq!(entity.attributes[0].base, 0.1);
        assert_eq!(
            entity.attributes[0].modifiers,
            vec![AttributeModifier {
                id: "minecraft:test_speed".to_string(),
                amount: 0.25,
                operation_id: 1,
            }]
        );

        assert_eq!(report.world_counters.play_logins_received, 1);
        assert_eq!(report.world_counters.respawns_received, 1);
        assert_eq!(report.world_counters.entity_data_updates_received, 1);
        assert_eq!(report.world_counters.entity_data_updates_applied, 1);
        assert_eq!(report.world_counters.entity_attribute_updates_received, 1);
        assert_eq!(report.world_counters.entity_attribute_updates_applied, 1);
        assert_eq!(report.world_counters.update_mob_effect_packets, 1);
        assert_eq!(report.world_counters.update_mob_effects_ignored, 0);
        assert_eq!(report.world_counters.active_mob_effects_tracked, 0);
    }

    #[tokio::test]
    async fn probe_set_health_records_dead_health_without_auto_respawn() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_play_packet(PlayClientbound::SetHealth(PlayerHealth {
                health: 0.0,
                food: 3,
                saturation: 0.5,
            }))
            .await
            .unwrap();

        assert!(
            timeout(Duration::from_millis(100), server.read_packet())
                .await
                .is_err(),
            "dead health alone must not send perform respawn"
        );

        assert_eq!(
            probe.world.local_player().health,
            Some(bbb_world::LocalPlayerHealthState {
                health: 0.0,
                food: 3,
                saturation: 0.5,
            })
        );
        assert!(probe.world.local_player_is_dead());
        assert_eq!(probe.world.counters().player_health_packets, 1);
    }

    #[tokio::test]
    async fn probe_play_unknown_packets_update_report_diagnostics() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Play;

        probe
            .handle_play_packet(PlayClientbound::Unknown {
                packet_id: 0x7e,
                len: 9,
            })
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.unsupported_packets, 1);
        assert_eq!(
            report.last_unsupported_packet_state.as_deref(),
            Some("Play")
        );
        assert_eq!(report.last_unsupported_packet_id, Some(0x7e));
        assert_eq!(report.last_unsupported_packet_len, Some(9));
        assert_eq!(report.world_counters.play_logins_received, 0);
    }

    #[tokio::test]
    async fn probe_bundle_delimiter_is_transport_noop() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Play;

        let first_chunk = probe
            .handle_play_packet(PlayClientbound::BundleDelimiter)
            .await
            .unwrap();

        assert_eq!(first_chunk, None);
        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "bundle delimiter must not send serverbound packets"
        );

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.unsupported_packets, 0);
        assert_eq!(report.world_counters.play_logins_received, 0);
    }

    #[tokio::test]
    async fn probe_play_disconnect_returns_disconnect_error() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Play;

        let err = probe
            .handle_play_packet(PlayClientbound::Disconnect(Disconnect {
                reason: "Kicked".to_string(),
                raw_reason: Vec::new(),
            }))
            .await
            .unwrap_err();

        assert_eq!(err.to_string(), "play disconnected: Kicked");
        assert!(
            timeout(Duration::from_millis(50), server.read_packet())
                .await
                .is_err(),
            "disconnect handling must not send serverbound packets"
        );
    }

    fn probe_recipe_entry(id: i32, notification: bool, highlight: bool) -> RecipeBookAddEntry {
        RecipeBookAddEntry {
            contents: RecipeDisplayEntry {
                id: RecipeDisplayId { index: id },
                display: RecipeDisplaySummary {
                    display_type: RecipeDisplayType::Stonecutter,
                    raw_body: vec![3, id as u8],
                    crafting: None,
                },
                group: None,
                category_id: id + 10,
                crafting_requirements: Some(vec![IngredientSummary {
                    tag: None,
                    item_ids: vec![40 + id],
                }]),
            },
            flags: (u8::from(notification)) | (u8::from(highlight) << 1),
            notification,
            highlight,
        }
    }

    async fn raw_connection_pair() -> (RawConnection, RawConnection) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = tokio::spawn(async move {
            RawConnection::connect(&addr.to_string(), None)
                .await
                .unwrap()
        });
        let (server_stream, _) = listener.accept().await.unwrap();
        let client = client.await.unwrap();
        let server = RawConnection {
            stream: server_stream,
            read_buf: BytesMut::with_capacity(8192),
            compression_threshold: None,
        };
        (client, server)
    }

    const LIGHT_ARRAY_BYTES: usize = 2048;

    fn synthetic_probe_level_chunk_packet() -> LevelChunkWithLight {
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
            light_data: LightUpdateData {
                sky_y_mask: Vec::new(),
                block_y_mask: Vec::new(),
                empty_sky_y_mask: Vec::new(),
                empty_block_y_mask: Vec::new(),
                sky_updates: Vec::new(),
                block_updates: Vec::new(),
            },
        }
    }

    fn vanilla_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
        let properties =
            BTreeMap::from(props.map(|(key, value)| (key.to_string(), value.to_string())));
        RegistrySet::vanilla_26_1()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("missing vanilla block state {name} {properties:?}"))
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

    fn set_light_nibble(layer: &mut [u8], nibble_index: usize, value: u8) {
        let byte = layer.get_mut(nibble_index / 2).unwrap();
        let shift = (nibble_index % 2) * 4;
        *byte = (*byte & !(0x0f << shift)) | ((value & 0x0f) << shift);
    }

    fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
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

    fn item_stack_entity_data(item: ItemStackSummary) -> EntityDataValue {
        EntityDataValue {
            data_id: 8,
            serializer_id: 7,
            value: EntityDataValueKind::ItemStack(item),
        }
    }

    fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
        ItemCostSummary {
            item_id,
            count,
            component_predicate: Default::default(),
        }
    }

    fn merchant_offers(container_id: i32, offer_count: usize) -> MerchantOffers {
        MerchantOffers {
            container_id,
            offers: (0..offer_count)
                .map(|index| MerchantOffer {
                    buy_a: item_cost(42 + index as i32, 3),
                    sell: item_stack(99 + index as i32, 1),
                    buy_b: None,
                    is_out_of_stock: false,
                    uses: 1,
                    max_uses: 12,
                    xp: 8,
                    special_price_diff: -2,
                    price_multiplier: 0.05,
                    demand: 6,
                })
                .collect(),
            villager_level: 3,
            villager_xp: 120,
            show_progress: true,
            can_restock: false,
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

    fn command_tree_packet(literal: &str) -> Commands {
        Commands {
            root_index: 0,
            nodes: vec![
                CommandNode {
                    node_type: CommandNodeType::Root,
                    flags: 0,
                    children: vec![1],
                    redirect: None,
                    name: None,
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Literal,
                    flags: 1,
                    children: vec![2],
                    redirect: None,
                    name: Some(literal.to_string()),
                    parser: None,
                    suggestions: None,
                    executable: false,
                    restricted: false,
                },
                CommandNode {
                    node_type: CommandNodeType::Argument,
                    flags: 54,
                    children: Vec::new(),
                    redirect: None,
                    name: Some("message".to_string()),
                    parser: Some(CommandArgumentParser {
                        type_id: 20,
                        name: "minecraft:message".to_string(),
                        properties: vec![2],
                    }),
                    suggestions: Some("minecraft:ask_server".to_string()),
                    executable: true,
                    restricted: true,
                },
            ],
        }
    }

    fn protocol_player_chat_with_signature(
        global_index: i32,
        signature: MessageSignature,
    ) -> PlayerChat {
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
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Registry { id: 0 },
                name: "Alice".to_string(),
                target_name: None,
            },
        }
    }

    fn protocol_add_entity(id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: 7,
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

    fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
        AddEntity {
            entity_type_id,
            ..protocol_add_entity(id)
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    }

    fn minecart_step(
        position_x: f64,
        position_y: f64,
        position_z: f64,
        movement_x: f64,
        movement_y: f64,
        movement_z: f64,
        y_rot: f32,
        x_rot: f32,
        weight: f32,
    ) -> MinecartStep {
        MinecartStep {
            position: ProtocolVec3d {
                x: position_x,
                y: position_y,
                z: position_z,
            },
            movement: ProtocolVec3d {
                x: movement_x,
                y: movement_y,
                z: movement_z,
            },
            y_rot,
            x_rot,
            weight,
        }
    }
}
