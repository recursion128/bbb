use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::decode_component_summary,
    ids,
    packets::{
        chat, chunks, client_audio, client_common, client_features, client_state, client_ui,
        command_suggestions, connection, debug_game, entities, inventory, maps, merchant,
        player_actions, player_info, scoreboard, server_presentation, tags, waypoints,
        world_border::{
            InitializeBorder, SetBorderCenter, SetBorderLerpSize, SetBorderSize,
            SetBorderWarningDelay, SetBorderWarningDistance,
        },
        world_effects, PlayerRotationUpdate,
    },
};

use super::{
    codecs::{decode_award_stats, decode_play_login, decode_player_position, decode_respawn},
    types::*,
};

pub fn decode_play_clientbound(packet_id: i32, payload: &[u8]) -> Result<PlayClientbound> {
    match packet_id {
        ids::play::CLIENTBOUND_BUNDLE_DELIMITER => Ok(PlayClientbound::BundleDelimiter),
        ids::play::CLIENTBOUND_ADD_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::AddEntity(entities::decode_add_entity(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_ANIMATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityAnimation(
                entities::decode_entity_animation(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_AWARD_STATS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::AwardStats(decode_award_stats(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockChangedAck(
                chunks::decode_block_changed_ack(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_BLOCK_DESTRUCTION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockDestruction(
                chunks::decode_block_destruction(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockEntityData(
                chunks::decode_block_entity_data(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_BLOCK_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockEvent(chunks::decode_block_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_BLOCK_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockUpdate(chunks::decode_block_update(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_BOSS_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BossEvent(client_state::decode_boss_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_CHANGE_DIFFICULTY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ChangeDifficulty(
                client_state::decode_change_difficulty(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CHUNK_BATCH_START => Ok(PlayClientbound::ChunkBatchStart),
        ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED => Ok(PlayClientbound::ChunkBatchFinished {
            batch_size: Decoder::new(payload).read_var_i32()?,
        }),
        ids::play::CLIENTBOUND_CHUNKS_BIOMES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ChunksBiomes(chunks::decode_chunks_biomes(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_CLEAR_DIALOG => {
            let decoder = Decoder::new(payload);
            client_common::decode_clear_dialog(&decoder)?;
            Ok(PlayClientbound::ClearDialog)
        }
        ids::play::CLIENTBOUND_CLEAR_TITLES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ClearTitles(
                client_state::decode_clear_titles(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::CommandSuggestions(
                command_suggestions::decode_command_suggestions(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_CLOSE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerClose(
                inventory::decode_container_close(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetContent(
                inventory::decode_container_set_content(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetData(
                inventory::decode_container_set_data(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_SLOT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetSlot(
                inventory::decode_container_set_slot(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_COOKIE_REQUEST => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::CookieRequest(
                connection::decode_cookie_request(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_COOLDOWN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Cooldown(client_state::decode_cooldown(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::CustomChatCompletions(
                client_features::decode_custom_chat_completions(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CUSTOM_PAYLOAD => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::CustomPayload(
                client_common::decode_custom_payload(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CUSTOM_REPORT_DETAILS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::CustomReportDetails(
                connection::decode_custom_report_details(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SERVER_LINKS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ServerLinks(
                connection::decode_server_links(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_DAMAGE_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DamageEvent(
                client_state::decode_damage_event(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DebugBlockValue(
                debug_game::decode_debug_block_value(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_DEBUG_CHUNK_VALUE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DebugChunkValue(
                debug_game::decode_debug_chunk_value(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_DEBUG_ENTITY_VALUE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DebugEntityValue(
                debug_game::decode_debug_entity_value(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_DEBUG_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DebugEvent(debug_game::decode_debug_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_DEBUG_SAMPLE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DebugSample(
                debug_game::decode_debug_sample(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_DELETE_CHAT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DeleteChat(chat::decode_delete_chat(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_DISCONNECT => Ok(PlayClientbound::Disconnect(Disconnect {
            reason: decode_component_summary(payload)?,
            raw_reason: payload.to_vec(),
        })),
        ids::play::CLIENTBOUND_DISGUISED_CHAT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DisguisedChat(chat::decode_disguised_chat(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_ENTITY_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityEvent(entities::decode_entity_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityPositionSync(
                entities::decode_entity_position_sync(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_EXPLODE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Explosion(world_effects::decode_explosion(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ForgetLevelChunk(
                chunks::decode_forget_level_chunk(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_GAME_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::GameEvent(client_state::decode_game_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_GAME_RULE_VALUES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::GameRuleValues(
                debug_game::decode_game_rule_values(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::GameTestHighlightPos(
                debug_game::decode_game_test_highlight_pos(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_HURT_ANIMATION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::HurtAnimation(
                entities::decode_hurt_animation(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_INITIALIZE_BORDER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::InitializeBorder(InitializeBorder {
                new_center_x: decoder.read_f64()?,
                new_center_z: decoder.read_f64()?,
                old_size: decoder.read_f64()?,
                new_size: decoder.read_f64()?,
                lerp_time: decoder.read_var_i64()?,
                new_absolute_max_size: decoder.read_var_i32()?,
                warning_blocks: decoder.read_var_i32()?,
                warning_time: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_KEEP_ALIVE => Ok(PlayClientbound::KeepAlive {
            id: Decoder::new(payload).read_i64()?,
        }),
        ids::play::CLIENTBOUND_LEVEL_PARTICLES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LevelParticles(
                world_effects::decode_level_particles(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_LOW_DISK_SPACE_WARNING => {
            let decoder = Decoder::new(payload);
            if !decoder.is_empty() {
                return Err(ProtocolError::InvalidData(
                    "trailing bytes after low disk space warning packet".to_string(),
                ));
            }
            Ok(PlayClientbound::LowDiskSpaceWarning)
        }
        ids::play::CLIENTBOUND_MAP_ITEM_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MapItemData(maps::decode_map_item_data(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_MERCHANT_OFFERS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MerchantOffers(
                merchant::decode_merchant_offers(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_MOUNT_SCREEN_OPEN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MountScreenOpen(
                client_ui::decode_mount_screen_open(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PING => Ok(PlayClientbound::Ping {
            id: Decoder::new(payload).read_i32()?,
        }),
        ids::play::CLIENTBOUND_LOGIN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Login(decode_play_login(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_OPEN_BOOK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::OpenBook(client_ui::decode_open_book(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_ENTITY_POS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveEntity(entities::decode_move_entity(
                &mut decoder,
                true,
                false,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveEntity(entities::decode_move_entity(
                &mut decoder,
                true,
                true,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_ENTITY_ROT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveEntity(entities::decode_move_entity(
                &mut decoder,
                false,
                true,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_VEHICLE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveVehicle(entities::decode_move_vehicle(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_OPEN_SCREEN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::OpenScreen(inventory::decode_open_screen(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_OPEN_SIGN_EDITOR => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::OpenSignEditor(
                client_ui::decode_open_sign_editor(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_ABILITIES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerAbilities(
                client_state::decode_player_abilities(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_CHAT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerChat(chat::decode_player_chat(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_PLAYER_COMBAT_END => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerCombatEnd(
                player_actions::decode_player_combat_end(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_COMBAT_ENTER => {
            let decoder = Decoder::new(payload);
            player_actions::decode_player_combat_enter(&decoder)?;
            Ok(PlayClientbound::PlayerCombatEnter)
        }
        ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerCombatKill(
                player_actions::decode_player_combat_kill(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerInfoRemove(
                player_info::decode_player_info_remove(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerInfoUpdate(
                player_info::decode_player_info_update(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PONG_RESPONSE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PongResponse(
                client_ui::decode_play_pong_response(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlaceGhostRecipe(
                client_features::decode_place_ghost_recipe(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_LOOK_AT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerLookAt(
                player_actions::decode_player_look_at(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_POSITION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerPosition(decode_player_position(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_PLAYER_ROTATION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerRotation(PlayerRotationUpdate {
                y_rot: decoder.read_f32()?,
                relative_y: decoder.read_bool()?,
                x_rot: decoder.read_f32()?,
                relative_x: decoder.read_bool()?,
            }))
        }
        ids::play::CLIENTBOUND_PROJECTILE_POWER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ProjectilePower(
                world_effects::decode_projectile_power(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_REMOVE_ENTITIES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RemoveEntities(
                entities::decode_remove_entities(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RemoveMobEffect(
                client_state::decode_remove_mob_effect(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_RESET_SCORE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResetScore(scoreboard::decode_reset_score(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_RESOURCE_PACK_POP => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResourcePackPop(
                server_presentation::decode_resource_pack_pop(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResourcePackPush(
                server_presentation::decode_resource_pack_push(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_RESPAWN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Respawn(decode_respawn(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_ROTATE_HEAD => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RotateHead(entities::decode_rotate_head(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SERVER_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ServerData(
                server_presentation::decode_server_data(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetActionBarText(
                client_state::decode_set_action_bar_text(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_BORDER_CENTER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderCenter(SetBorderCenter {
                new_center_x: decoder.read_f64()?,
                new_center_z: decoder.read_f64()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_BORDER_LERP_SIZE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderLerpSize(SetBorderLerpSize {
                old_size: decoder.read_f64()?,
                new_size: decoder.read_f64()?,
                lerp_time: decoder.read_var_i64()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_BORDER_SIZE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderSize(SetBorderSize {
                size: decoder.read_f64()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_BORDER_WARNING_DELAY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderWarningDelay(
                SetBorderWarningDelay {
                    warning_delay: decoder.read_var_i32()?,
                },
            ))
        }
        ids::play::CLIENTBOUND_SET_BORDER_WARNING_DISTANCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderWarningDistance(
                SetBorderWarningDistance {
                    warning_blocks: decoder.read_var_i32()?,
                },
            ))
        }
        ids::play::CLIENTBOUND_SET_CAMERA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetCamera(client_state::decode_set_camera(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_HEALTH => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetHealth(
                client_state::decode_player_health(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_HELD_SLOT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetHeldSlot(
                client_state::decode_set_held_slot(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_OBJECTIVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetObjective(
                scoreboard::decode_set_objective(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_PASSENGERS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPassengers(
                entities::decode_set_passengers(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SectionBlocksUpdate(
                chunks::decode_section_blocks_update(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SelectAdvancementsTab(
                client_features::decode_select_advancements_tab(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetChunkCacheCenter(
                chunks::decode_set_chunk_cache_center(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetChunkCacheRadius(
                chunks::decode_set_chunk_cache_radius(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_CURSOR_ITEM => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetCursorItem(
                inventory::decode_set_cursor_item(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetDefaultSpawnPosition(
                client_state::decode_default_spawn_position(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetDisplayObjective(
                scoreboard::decode_set_display_objective(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityData(
                entities::decode_set_entity_data(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_LINK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityLink(
                entities::decode_set_entity_link(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_MOTION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityMotion(
                entities::decode_set_entity_motion(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_EQUIPMENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEquipment(
                entities::decode_set_equipment(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_EXPERIENCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetExperience(
                client_state::decode_player_experience(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPlayerInventory(
                inventory::decode_set_player_inventory(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_PLAYER_TEAM => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPlayerTeam(
                scoreboard::decode_set_player_team(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_SCORE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetScore(scoreboard::decode_set_score(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetSimulationDistance(
                client_state::decode_set_simulation_distance(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetSubtitleText(
                client_state::decode_set_subtitle_text(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_TIME => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTime(client_state::decode_play_time(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_TITLE_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTitleText(
                client_state::decode_set_title_text(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_TITLES_ANIMATION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTitlesAnimation(
                client_state::decode_set_titles_animation(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SHOW_DIALOG => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ShowDialog(
                client_common::decode_show_dialog(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SOUND_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SoundEntity(
                client_audio::decode_sound_entity_event(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SOUND => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Sound(client_audio::decode_sound_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_START_CONFIGURATION => Ok(PlayClientbound::StartConfiguration),
        ids::play::CLIENTBOUND_STOP_SOUND => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::StopSound(client_audio::decode_stop_sound(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_STORE_COOKIE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::StoreCookie(
                connection::decode_store_cookie(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SYSTEM_CHAT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SystemChat(
                client_state::decode_system_chat(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TAB_LIST => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TabList(
                server_presentation::decode_tab_list(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TAG_QUERY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TagQuery(
                client_features::decode_tag_query(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TakeItemEntity(
                entities::decode_take_item_entity(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TELEPORT_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TeleportEntity(
                entities::decode_teleport_entity(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TestInstanceBlockStatus(
                debug_game::decode_test_instance_block_status(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TICKING_STATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TickingState(
                client_state::decode_ticking_state(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TICKING_STEP => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TickingStep(
                client_state::decode_ticking_step(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TRANSFER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Transfer(connection::decode_transfer(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_WAYPOINT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Waypoint(
                waypoints::decode_tracked_waypoint_packet(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::UpdateAttributes(
                entities::decode_update_attributes(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::UpdateMobEffect(
                client_state::decode_update_mob_effect(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_UPDATE_TAGS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::UpdateTags(tags::decode_update_tags(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_LEVEL_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LevelEvent(chunks::decode_level_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LevelChunkWithLight(
                chunks::decode_level_chunk_with_light(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_LIGHT_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LightUpdate(chunks::decode_light_update(
                &mut decoder,
            )?))
        }
        id => Ok(PlayClientbound::Unknown {
            packet_id: id,
            len: payload.len(),
        }),
    }
}
