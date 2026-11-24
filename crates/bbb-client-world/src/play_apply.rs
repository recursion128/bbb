//! Shared clientbound play-packet application.
//!
//! `WorldStore::apply_play_packet` is the single packet -> canonical-world
//! mapping used by both the offline probe and the online dispatcher. Callers
//! provide a [`PlayApplyEffects`] implementation for runtime side effects
//! (audio sinks, particle sinks, serverbound acknowledgements); world
//! mutation and the deterministic level-event random stream live here.

use bbb_protocol::packets::{
    ChatAcknowledgement, LevelEvent as ProtocolLevelEvent, PlayClientbound, Vec3d as ProtocolVec3d,
};

use crate::{
    AllayDuplicationParticleState, AnimalLoveParticleState, ArrowEffectParticleState,
    ChatDisconnectReason, ChunkPos, DolphinHappyParticleState, EntityTamingParticleState,
    FireworkRocketExplosionParticleState, FoxEatParticleState, HoneyBlockParticleState,
    JukeboxLevelEventState, LevelEventSoundRandomState, LivingEntityDrownParticleState,
    LivingEntityItemParticleState, LivingEntityPoofParticleState, LivingEntityPortalParticleState,
    LlamaSpitRecreationParticleState, LocalSoundEventState, RabbitSprintParticleState,
    RavagerRoarParticleState, RemovedPlayerVehicleTeleportRecovery, SnowballHitParticleState,
    SoundEntityEventState, SoundEventState, StopSoundEventState, TakeItemEntityPickupParticleState,
    ThrownEggHitParticleState, VehicleMoveReport, VillagerParticleState, WitchMagicParticleState,
    WorldStore,
};

mod chunk;
mod entities;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityTrackingEmitterParticleKind {
    Crit,
    EnchantedHit,
    TotemOfUndying,
}

/// Host screen context and runtime effects for shared play-packet application.
pub trait PlayApplyEffects {
    fn creative_inventory_context(&self) -> Option<bool> {
        None
    }

    fn server_data_updated(&mut self, _world: &WorldStore) {}
    fn positioned_sound(&mut self, _state: &SoundEventState) {}
    fn note_block_particle(&mut self, _world: &WorldStore, _state: crate::NoteBlockParticleState) {}
    fn local_sound(&mut self, _state: &LocalSoundEventState) {}
    fn entity_sound(&mut self, _state: &SoundEntityEventState, _position: Option<[f64; 3]>) {}
    fn stop_sound(&mut self, _state: &StopSoundEventState) {}
    fn stop_all_sounds(&mut self) {}
    fn stop_music(&mut self) {}
    fn jukebox_level_event(&mut self, _state: &JukeboxLevelEventState) {}
    fn chat_acknowledgement(&mut self, _command: ChatAcknowledgement) {}
    fn chat_validation_epoch_millis(&mut self) -> i64 {
        i64::MIN
    }
    fn chat_monotonic_millis(&mut self) -> u64 {
        u64::MAX
    }
    fn client_monotonic_millis(&mut self) -> u64 {
        0
    }
    fn chat_disconnect(&mut self, _reason: ChatDisconnectReason) {}
    fn container_close(&mut self, _container_id: i32) {}
    fn perform_respawn(&mut self) {}
    fn vehicle_move_report(&mut self, _report: VehicleMoveReport) {}
    fn removed_player_vehicle_teleport(&mut self, _recovery: RemovedPlayerVehicleTeleportRecovery) {
    }
    fn chunk_inserted(&mut self, _pos: ChunkPos) {}
    fn level_particles(
        &mut self,
        _world: &WorldStore,
        _packet: &bbb_protocol::packets::LevelParticles,
    ) {
    }
    fn explosion_particles(&mut self, _world: &WorldStore, _state: &crate::ExplosionEventState) {}
    fn firework_empty_explosion_particles(&mut self, _world: &WorldStore, _position: [f64; 3]) {}
    fn llama_spit_recreation_particles(
        &mut self,
        _world: &WorldStore,
        _state: LlamaSpitRecreationParticleState,
    ) {
    }
    fn firework_explosion_particles(
        &mut self,
        _world: &WorldStore,
        _state: &FireworkRocketExplosionParticleState,
    ) {
    }
    fn elder_guardian_effect_particles(&mut self, _world: &WorldStore, _position: ProtocolVec3d) {}
    fn tracking_emitter_particles(
        &mut self,
        _world: &WorldStore,
        _entity_id: i32,
        _kind: EntityTrackingEmitterParticleKind,
        _lifetime_ticks: u32,
    ) {
    }
    fn take_item_entity_pickup_particles(
        &mut self,
        _world: &WorldStore,
        _state: &TakeItemEntityPickupParticleState,
    ) {
    }
    fn ravager_roar_particles(&mut self, _world: &WorldStore, _state: RavagerRoarParticleState) {}
    fn witch_magic_particles(&mut self, _world: &WorldStore, _state: WitchMagicParticleState) {}
    fn living_entity_poof_particles(
        &mut self,
        _world: &WorldStore,
        _state: LivingEntityPoofParticleState,
    ) {
    }
    fn living_entity_item_particles(
        &mut self,
        _world: &WorldStore,
        _state: LivingEntityItemParticleState,
    ) {
    }
    fn living_entity_drown_particles(
        &mut self,
        _world: &WorldStore,
        _state: LivingEntityDrownParticleState,
    ) {
    }
    fn living_entity_portal_particles(
        &mut self,
        _world: &WorldStore,
        _state: LivingEntityPortalParticleState,
    ) {
    }
    fn arrow_effect_particles(&mut self, _world: &WorldStore, _state: ArrowEffectParticleState) {}
    fn entity_taming_particles(&mut self, _world: &WorldStore, _state: EntityTamingParticleState) {}
    fn villager_particles(&mut self, _world: &WorldStore, _state: VillagerParticleState) {}
    fn dolphin_happy_particles(&mut self, _world: &WorldStore, _state: DolphinHappyParticleState) {}
    fn fox_eat_particles(&mut self, _world: &WorldStore, _state: FoxEatParticleState) {}
    fn animal_love_particles(&mut self, _world: &WorldStore, _state: AnimalLoveParticleState) {}
    fn allay_duplication_particles(
        &mut self,
        _world: &WorldStore,
        _state: AllayDuplicationParticleState,
    ) {
    }
    fn snowball_hit_particles(&mut self, _world: &WorldStore, _state: SnowballHitParticleState) {}
    fn thrown_egg_hit_particles(&mut self, _world: &WorldStore, _state: ThrownEggHitParticleState) {
    }
    fn honey_block_particles(&mut self, _world: &WorldStore, _state: HoneyBlockParticleState) {}
    fn rabbit_sprint_particles(&mut self, _world: &WorldStore, _state: RabbitSprintParticleState) {}
    fn level_event_particles(
        &mut self,
        _world: &WorldStore,
        _event: &ProtocolLevelEvent,
        _random: &mut LevelEventSoundRandomState,
    ) {
    }
}

/// State-only application without runtime sinks.
pub struct NoPlayApplyEffects;

impl PlayApplyEffects for NoPlayApplyEffects {}

impl WorldStore {
    /// Applies a clientbound play packet's canonical world mutation.
    ///
    /// Returns `None` when the packet is fully world-owned. Returns the packet
    /// back when it needs caller-owned connection handling (keepalive/ping
    /// responses, chunk batch feedback, cookies, configuration handoff,
    /// resource-pack responses, movement acknowledgements, disconnects,
    /// unknown packets). `PlayerPosition`/`PlayerRotation` are applied to the
    /// world *and* returned so the caller can send movement responses.
    pub fn apply_play_packet(
        &mut self,
        packet: PlayClientbound,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) -> Option<PlayClientbound> {
        match packet {
            PlayClientbound::BundleDelimiter => None,

            // Entity packets
            PlayClientbound::AddEntity(entity) => {
                self.apply_play_add_entity(entity, effects);
                None
            }
            PlayClientbound::EntityAnimation(update) => {
                self.apply_play_entity_animation(update, effects);
                None
            }
            PlayClientbound::DamageEvent(update) => {
                self.apply_play_damage_event(update, random, effects);
                None
            }
            PlayClientbound::UpdateMobEffect(update) => {
                self.apply_update_mob_effect(update);
                None
            }
            PlayClientbound::RemoveMobEffect(update) => {
                self.apply_remove_mob_effect(update);
                None
            }
            PlayClientbound::MoveEntity(update) => {
                self.apply_entity_move(update);
                None
            }
            PlayClientbound::MoveMinecartAlongTrack(update) => {
                self.apply_move_minecart_along_track(update);
                None
            }
            PlayClientbound::MoveVehicle(update) => {
                if let Some(report) = self.apply_move_vehicle(update) {
                    effects.vehicle_move_report(report);
                }
                None
            }
            PlayClientbound::EntityPositionSync(update) => {
                self.apply_entity_position_sync(update);
                None
            }
            PlayClientbound::EntityEvent(update) => {
                self.apply_play_entity_event(update, random, effects);
                None
            }
            PlayClientbound::HurtAnimation(update) => {
                self.apply_hurt_animation(update);
                None
            }
            PlayClientbound::RemoveEntities(update) => {
                self.apply_remove_entities(update);
                None
            }
            PlayClientbound::RotateHead(update) => {
                self.apply_rotate_head(update);
                None
            }
            PlayClientbound::SetEntityMotion(update) => {
                self.apply_set_entity_motion(update);
                None
            }
            PlayClientbound::SetEntityLink(update) => {
                self.apply_set_entity_link(update);
                None
            }
            PlayClientbound::SetEquipment(update) => {
                self.apply_set_equipment(update);
                None
            }
            PlayClientbound::TakeItemEntity(update) => {
                self.apply_play_take_item_entity(update, random, effects);
                None
            }
            PlayClientbound::SetPassengers(update) => {
                self.apply_set_passengers(update);
                None
            }
            PlayClientbound::UpdateAttributes(update) => {
                self.apply_update_attributes(update);
                None
            }
            PlayClientbound::SetEntityData(update) => {
                self.apply_set_entity_data(update);
                None
            }
            PlayClientbound::TeleportEntity(update) => {
                self.apply_play_teleport_entity(update, effects);
                None
            }
            PlayClientbound::ProjectilePower(update) => {
                self.apply_projectile_power(update);
                None
            }

            // Container, inventory, and recipe packets
            PlayClientbound::MapItemData(update) => {
                self.apply_map_item_data(update);
                None
            }
            PlayClientbound::MountScreenOpen(update) => {
                self.apply_mount_screen_open(update);
                None
            }
            PlayClientbound::ContainerClose(update) => {
                self.apply_container_close(update);
                None
            }
            PlayClientbound::ContainerSetContent(update) => {
                self.apply_container_set_content(update);
                None
            }
            PlayClientbound::ContainerSetData(update) => {
                self.apply_container_set_data(update);
                None
            }
            PlayClientbound::ContainerSetSlot(update) => {
                self.apply_container_set_slot(update);
                None
            }
            PlayClientbound::MerchantOffers(update) => {
                self.apply_merchant_offers(update);
                None
            }
            PlayClientbound::OpenScreen(update) => {
                self.apply_open_screen(update);
                None
            }
            PlayClientbound::OpenBook(update) => {
                self.apply_open_book(update);
                None
            }
            PlayClientbound::OpenSignEditor(update) => {
                self.apply_open_sign_editor(update);
                None
            }
            PlayClientbound::PlaceGhostRecipe(update) => {
                self.apply_place_ghost_recipe(update);
                None
            }
            PlayClientbound::SetHeldSlot(update) => {
                self.apply_held_slot(update);
                None
            }
            PlayClientbound::SetCursorItem(update) => {
                self.apply_creative_aware_set_cursor_item(
                    update,
                    effects.creative_inventory_context().is_some(),
                );
                None
            }
            PlayClientbound::SetPlayerInventory(update) => {
                self.apply_set_player_inventory(update);
                None
            }
            PlayClientbound::RecipeBookAdd(update) => {
                self.apply_recipe_book_add(update);
                None
            }
            PlayClientbound::RecipeBookRemove(update) => {
                self.apply_recipe_book_remove(update);
                None
            }
            PlayClientbound::RecipeBookSettings(update) => {
                self.apply_recipe_book_settings(update);
                None
            }
            PlayClientbound::UpdateRecipes(update) => {
                self.apply_update_recipes(update);
                None
            }

            // Chunk and level packets
            PlayClientbound::BlockDestruction(update) => {
                self.apply_block_destruction(update);
                None
            }
            PlayClientbound::Explosion(update) => {
                self.apply_play_explosion(update, random, effects);
                None
            }
            PlayClientbound::SetSimulationDistance(update) => {
                self.apply_simulation_distance(update);
                None
            }
            PlayClientbound::BlockChangedAck(update) => {
                self.apply_block_changed_ack(update);
                None
            }
            PlayClientbound::BlockEntityData(update) => {
                let _ = self.apply_block_entity_data(update);
                None
            }
            PlayClientbound::BlockEvent(event) => {
                self.apply_block_event_with_effects(event, random, effects);
                None
            }
            PlayClientbound::LevelEvent(event) => {
                self.apply_level_event_with_effects(event, random, effects);
                None
            }
            PlayClientbound::GameEvent(update) => {
                self.apply_play_game_event(update, effects);
                None
            }
            PlayClientbound::LevelChunkWithLight(chunk) => {
                if let Ok(pos) = self.insert_level_chunk_with_light(chunk) {
                    effects.chunk_inserted(pos);
                }
                None
            }
            PlayClientbound::LevelParticles(update) => {
                self.apply_level_particles(update.clone());
                effects.level_particles(self, &update);
                None
            }
            PlayClientbound::LightUpdate(update) => {
                let _ = self.queue_light_update(update);
                None
            }
            PlayClientbound::ChunksBiomes(update) => {
                let _ = self.apply_biome_update(update);
                None
            }
            PlayClientbound::ForgetLevelChunk(update) => {
                self.forget_chunk(update.pos);
                None
            }
            PlayClientbound::BlockUpdate(update) => {
                self.apply_block_update(update);
                None
            }
            PlayClientbound::SectionBlocksUpdate(update) => {
                self.apply_section_blocks_update(update);
                None
            }
            PlayClientbound::SetChunkCacheCenter(update) => {
                self.apply_set_chunk_cache_center(update);
                None
            }
            PlayClientbound::SetChunkCacheRadius(update) => {
                self.apply_set_chunk_cache_radius(update);
                None
            }

            // Connection-owned packets: keepalive/ping responses, chunk batch
            // feedback, cookies, configuration handoff, disconnects, and
            // unknown packets stay with the caller's net context.
            packet @ (PlayClientbound::KeepAlive { .. }
            | PlayClientbound::Ping { .. }
            | PlayClientbound::ChunkBatchStart
            | PlayClientbound::ChunkBatchFinished { .. }
            | PlayClientbound::CookieRequest(_)
            | PlayClientbound::StoreCookie(_)
            | PlayClientbound::StartConfiguration
            | PlayClientbound::Disconnect(_)
            | PlayClientbound::Unknown { .. }) => Some(packet),

            // Session, HUD, chat, audio, border, scoreboard, and player packets
            PlayClientbound::AwardStats(update) => {
                self.apply_award_stats(update);
                None
            }
            PlayClientbound::BossEvent(update) => {
                self.apply_boss_event(update);
                None
            }
            PlayClientbound::ChangeDifficulty(update) => {
                self.apply_change_difficulty(update);
                None
            }
            PlayClientbound::Cooldown(update) => {
                self.apply_cooldown(update);
                None
            }
            PlayClientbound::CustomChatCompletions(update) => {
                self.apply_custom_chat_completions(update);
                None
            }
            PlayClientbound::CustomPayload(update) => {
                self.apply_custom_payload(update);
                None
            }
            PlayClientbound::CustomReportDetails(details) => {
                self.apply_custom_report_details(details);
                None
            }
            PlayClientbound::ServerLinks(links) => {
                self.apply_server_links(links);
                None
            }
            PlayClientbound::DebugBlockValue(update) => {
                self.apply_debug_block_value(update);
                None
            }
            PlayClientbound::DebugChunkValue(update) => {
                self.apply_debug_chunk_value(update);
                None
            }
            PlayClientbound::DebugEntityValue(update) => {
                self.apply_debug_entity_value(update);
                None
            }
            PlayClientbound::DebugEvent(update) => {
                self.apply_debug_event(update);
                None
            }
            PlayClientbound::DebugSample(update) => {
                self.apply_debug_sample(update);
                None
            }
            PlayClientbound::DeleteChat(update) => {
                if let Some(reason) = self.apply_delete_chat(update) {
                    effects.chat_disconnect(reason);
                }
                None
            }
            PlayClientbound::DisguisedChat(update) => {
                let now_epoch_millis = effects.chat_validation_epoch_millis();
                let now_monotonic_millis = effects.chat_monotonic_millis();
                self.apply_disguised_chat_at(update, now_epoch_millis, now_monotonic_millis);
                None
            }
            PlayClientbound::UpdateTags(update) => {
                self.apply_update_tags(update);
                None
            }
            PlayClientbound::LowDiskSpaceWarning => {
                self.apply_low_disk_space_warning();
                None
            }
            PlayClientbound::PongResponse(update) => {
                self.apply_pong_response(update);
                None
            }
            PlayClientbound::Login(login) => {
                self.apply_login(&login);
                None
            }
            PlayClientbound::Respawn(respawn) => {
                let result = self.apply_respawn(&respawn);
                if let Some(container_id) = result.closed_server_container_id {
                    effects.container_close(container_id);
                }
                if result.dimension_changed {
                    effects.stop_all_sounds();
                    effects.stop_music();
                }
                None
            }
            PlayClientbound::SetHealth(health) => {
                self.apply_player_health(health);
                None
            }
            PlayClientbound::PlayerAbilities(update) => {
                self.apply_player_abilities(update);
                None
            }
            PlayClientbound::PlayerChat(update) => {
                let now_epoch_millis = effects.chat_validation_epoch_millis();
                let now_monotonic_millis = effects.chat_monotonic_millis();
                let outcome =
                    self.apply_player_chat_at_times(update, now_epoch_millis, now_monotonic_millis);
                if let Some(command) = outcome.acknowledgement {
                    effects.chat_acknowledgement(command);
                }
                if let Some(reason) = outcome.disconnect {
                    effects.chat_disconnect(reason);
                }
                None
            }
            PlayClientbound::SetExperience(update) => {
                self.apply_player_experience(update);
                None
            }
            PlayClientbound::SetDefaultSpawnPosition(update) => {
                self.apply_default_spawn_position(update);
                None
            }
            PlayClientbound::SystemChat(update) => {
                let now_epoch_millis = effects.chat_validation_epoch_millis();
                self.apply_system_chat_at(update, now_epoch_millis);
                None
            }
            PlayClientbound::PlayerCombatEnd(update) => {
                self.apply_player_combat_end(update);
                None
            }
            PlayClientbound::PlayerCombatEnter => {
                self.apply_player_combat_enter();
                None
            }
            PlayClientbound::PlayerCombatKill(update) => {
                if self.apply_player_combat_kill(update)
                    == crate::PlayerCombatKillApplyOutcome::PerformRespawn
                {
                    effects.perform_respawn();
                }
                None
            }
            PlayClientbound::PlayerLookAt(update) => {
                self.apply_player_look_at(update);
                None
            }
            PlayClientbound::SetActionBarText(update) => {
                self.apply_action_bar_text(update);
                None
            }
            PlayClientbound::SetTitleText(update) => {
                self.apply_title_text(update);
                None
            }
            PlayClientbound::SetSubtitleText(update) => {
                self.apply_subtitle_text(update);
                None
            }
            PlayClientbound::ClearTitles(update) => {
                self.apply_clear_titles(update);
                None
            }
            PlayClientbound::SetTitlesAnimation(update) => {
                self.apply_titles_animation(update);
                None
            }
            PlayClientbound::Sound(update) => {
                let state = self.apply_sound_event(update);
                effects.positioned_sound(&state);
                None
            }
            PlayClientbound::SoundEntity(update) => {
                if let Some(state) = self.apply_sound_entity_event(update) {
                    let position = self
                        .probe_entity_transform(state.entity_id)
                        .map(|entity| [entity.position.x, entity.position.y, entity.position.z]);
                    effects.entity_sound(&state, position);
                }
                None
            }
            PlayClientbound::StopSound(update) => {
                let state = self.apply_stop_sound(update);
                effects.stop_sound(&state);
                None
            }
            PlayClientbound::TickingState(update) => {
                self.apply_ticking_state(update);
                None
            }
            PlayClientbound::TickingStep(update) => {
                self.apply_ticking_step(update);
                None
            }
            PlayClientbound::Transfer(update) => {
                // World state records the transfer; returning the packet lets
                // callers (online session, headless probe) execute the reconnect.
                self.apply_transfer(update.clone());
                Some(PlayClientbound::Transfer(update))
            }
            PlayClientbound::SetCamera(update) => {
                self.apply_set_camera(update);
                None
            }
            PlayClientbound::InitializeBorder(border) => {
                self.apply_initialize_border(border);
                None
            }
            PlayClientbound::SetBorderCenter(update) => {
                self.apply_set_border_center(update);
                None
            }
            PlayClientbound::SetBorderLerpSize(update) => {
                self.apply_set_border_lerp_size(update);
                None
            }
            PlayClientbound::SetBorderSize(update) => {
                self.apply_set_border_size(update);
                None
            }
            PlayClientbound::SetBorderWarningDelay(update) => {
                self.apply_set_border_warning_delay(update);
                None
            }
            PlayClientbound::SetBorderWarningDistance(update) => {
                self.apply_set_border_warning_distance(update);
                None
            }
            PlayClientbound::ResetScore(update) => {
                self.apply_reset_score(update);
                None
            }
            PlayClientbound::SetDisplayObjective(update) => {
                self.apply_set_display_objective(update);
                None
            }
            PlayClientbound::SetObjective(update) => {
                self.apply_set_objective(update);
                None
            }
            PlayClientbound::SetPlayerTeam(update) => {
                self.apply_set_player_team(update);
                None
            }
            PlayClientbound::SetScore(update) => {
                self.apply_set_score(update);
                None
            }
            PlayClientbound::Commands(update) => {
                self.apply_commands(update);
                None
            }
            PlayClientbound::CommandSuggestions(update) => {
                self.apply_command_suggestions(update);
                None
            }
            PlayClientbound::SelectAdvancementsTab(update) => {
                self.apply_select_advancements_tab(update);
                None
            }
            PlayClientbound::TagQuery(update) => {
                self.apply_tag_query(update);
                None
            }
            PlayClientbound::ClearDialog => {
                self.apply_clear_dialog();
                None
            }
            PlayClientbound::ShowDialog(update) => {
                self.apply_show_dialog(update);
                None
            }
            PlayClientbound::TestInstanceBlockStatus(update) => {
                self.apply_test_instance_block_status(update);
                None
            }
            PlayClientbound::TabList(update) => {
                self.apply_tab_list(update);
                None
            }
            PlayClientbound::GameRuleValues(update) => {
                self.apply_game_rule_values(update);
                None
            }
            PlayClientbound::GameTestHighlightPos(update) => {
                let now_monotonic_millis = effects.client_monotonic_millis();
                self.apply_game_test_highlight_pos_at(update, now_monotonic_millis);
                None
            }
            PlayClientbound::SetTime(update) => {
                self.apply_world_time(update);
                None
            }
            PlayClientbound::PlayerPosition(update) => {
                self.apply_player_position(update);
                Some(PlayClientbound::PlayerPosition(update))
            }
            PlayClientbound::PlayerRotation(update) => {
                self.apply_player_rotation(update);
                Some(PlayClientbound::PlayerRotation(update))
            }
            PlayClientbound::PlayerInfoUpdate(update) => {
                self.apply_player_info_update(update);
                None
            }
            PlayClientbound::PlayerInfoRemove(update) => {
                self.apply_player_info_remove(update);
                None
            }
            PlayClientbound::ServerData(update) => {
                self.apply_server_data(update);
                effects.server_data_updated(self);
                None
            }
            PlayClientbound::ResourcePackPop(update) => {
                self.apply_resource_pack_pop(update);
                None
            }
            PlayClientbound::Waypoint(update) => {
                self.apply_waypoint(update);
                None
            }
            PlayClientbound::UpdateAdvancements(update) => {
                self.apply_update_advancements(update);
                None
            }
            PlayClientbound::ResourcePackPush(update) => {
                // The push is world-owned; the response action and serverbound
                // reply stay with the caller's net context.
                self.apply_resource_pack_push(update.clone());
                Some(PlayClientbound::ResourcePackPush(update))
            }
        }
    }
}
