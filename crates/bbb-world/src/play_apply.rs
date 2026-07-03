//! Shared clientbound play-packet application.
//!
//! `WorldStore::apply_play_packet` is the single packet -> canonical-world
//! mapping used by both the offline probe and the online dispatcher. Callers
//! provide a [`PlayApplyEffects`] implementation for runtime side effects
//! (audio sinks, particle sinks, serverbound acknowledgements); world
//! mutation and the deterministic level-event random stream live here.

use bbb_protocol::packets::{
    ChatAcknowledgement, GameEvent as ProtocolGameEvent, LevelEvent as ProtocolLevelEvent,
    PlayClientbound, Vec3d as ProtocolVec3d,
};

use crate::{
    advance_cobweb_place_particle_randoms, advance_vault_activation_particle_randoms,
    advance_vault_deactivation_particle_randoms, BlockPos, ChunkPos, JukeboxLevelEventState,
    LevelEventSoundRandomState, LocalSoundEventState, SoundEntityEventState, SoundEventState,
    StopSoundEventState, VehicleMoveReport, WorldStore,
};

const COBWEB_PLACE_LEVEL_EVENT: i32 = 3018;
const BLAZE_SMOKE_LEVEL_EVENT: i32 = 2004;
const DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT: i32 = 2006;
const DISPENSER_SMOKE_LEVEL_EVENT: i32 = 2000;
const DISPENSER_WHITE_SMOKE_LEVEL_EVENT: i32 = 2010;
const EGG_CRACK_LEVEL_EVENT: i32 = 3009;
const ELECTRIC_SPARK_LEVEL_EVENT: i32 = 3002;
const END_PORTAL_FRAME_FILL_LEVEL_EVENT: i32 = 1503;
const ENDER_EYE_BREAK_LEVEL_EVENT: i32 = 2003;
const INSTANT_POTION_BREAK_LEVEL_EVENT: i32 = 2007;
const LAVA_EXTINGUISH_LEVEL_EVENT: i32 = 1501;
const PLANT_GROWTH_LEVEL_EVENT: i32 = 1505;
const POTION_BREAK_LEVEL_EVENT: i32 = 2002;
const REDSTONE_TORCH_BURNOUT_LEVEL_EVENT: i32 = 1502;
const TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT: i32 = 3013;
const TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT: i32 = 3019;
const TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT: i32 = 3014;
const TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT: i32 = 3020;
const TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT: i32 = 3021;
const TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT: i32 = 3012;
const VAULT_ACTIVATE_LEVEL_EVENT: i32 = 3015;
const VAULT_DEACTIVATE_LEVEL_EVENT: i32 = 3016;
const SCULK_CHARGE_LEVEL_EVENT: i32 = 3006;
const SCULK_SHRIEKER_LEVEL_EVENT: i32 = 3007;
const SCRAPE_LEVEL_EVENT: i32 = 3005;
const SPLASH_CLOUD_LEVEL_EVENT: i32 = 2009;
const WAX_OFF_LEVEL_EVENT: i32 = 3004;
const WAX_ON_LEVEL_EVENT: i32 = 3003;
// Vanilla 26.1 BlockEntityType registry order in BlockEntityType.java.
const VANILLA_VAULT_BLOCK_ENTITY_TYPE_ID: i32 = 45;
const CRITICAL_HIT_ANIMATION_ACTION: u8 = 4;
const MAGIC_CRITICAL_HIT_ANIMATION_ACTION: u8 = 5;
const TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS: u32 = 3;
const TOTEM_TRACKING_EMITTER_LIFETIME_TICKS: u32 = 30;
const GUARDIAN_ELDER_EFFECT_GAME_EVENT: u8 = 10;

/// Growth level-event particle spawn mode; only the random-consumption shape
/// matters for callers without a particle sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelEventGrowthRandomMode {
    InBlock,
    WideNoFloating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityTrackingEmitterParticleKind {
    Crit,
    EnchantedHit,
    TotemOfUndying,
}

/// Runtime side effects of applying a play packet.
///
/// Methods default to no runtime sink plus world-owned read-only context
/// callbacks, so state-only callers (the offline probe, world tests) apply
/// identical canonical mutations and consume the identical deterministic random
/// stream as the online dispatcher.
pub trait PlayApplyEffects {
    fn positioned_sound(&mut self, _state: &SoundEventState) {}
    fn local_sound(&mut self, _state: &LocalSoundEventState) {}
    fn entity_sound(&mut self, _state: &SoundEntityEventState, _position: Option<[f64; 3]>) {}
    fn stop_sound(&mut self, _state: &StopSoundEventState) {}
    fn jukebox_level_event(&mut self, _state: &JukeboxLevelEventState) {}
    fn chat_acknowledgement(&mut self, _command: ChatAcknowledgement) {}
    fn vehicle_move_report(&mut self, _report: VehicleMoveReport) {}
    fn chunk_inserted(&mut self, _pos: ChunkPos) {}
    fn level_particles(
        &mut self,
        _world: &WorldStore,
        _packet: &bbb_protocol::packets::LevelParticles,
    ) {
    }
    fn firework_empty_explosion_particles(&mut self, _world: &WorldStore, _position: [f64; 3]) {}
    fn elder_guardian_effect_particles(&mut self, _world: &WorldStore, _position: ProtocolVec3d) {}
    fn tracking_emitter_particles(
        &mut self,
        _world: &WorldStore,
        _entity_id: i32,
        _kind: EntityTrackingEmitterParticleKind,
        _lifetime_ticks: u32,
    ) {
    }
    /// Spawn level-event particles through a sink. Return `true` when the sink
    /// consumed the particle randoms; `false` lets the world advance the
    /// deterministic random stream in the sink's place.
    fn level_event_particles(
        &mut self,
        _world: &WorldStore,
        _event: &ProtocolLevelEvent,
        _random: &mut LevelEventSoundRandomState,
    ) -> bool {
        false
    }
    /// Sculk-charge pop full-block context for sink-less random advancement.
    /// `None` matches vanilla's missing-block-probe fallback.
    fn sculk_charge_pop_full_block(
        &mut self,
        world: &WorldStore,
        event: &ProtocolLevelEvent,
    ) -> Option<bool> {
        sculk_charge_pop_full_block_context(world, event)
    }
    /// Plant-growth particle mode for sink-less random advancement. `None`
    /// matches vanilla's non-bonemealable/missing-block fallback.
    fn growth_particle_random_mode(
        &mut self,
        world: &WorldStore,
        event: &ProtocolLevelEvent,
    ) -> Option<LevelEventGrowthRandomMode> {
        growth_particle_random_mode_context(world, event)
    }
}

/// State-only application: canonical world mutation and deterministic random
/// stream advancement without any runtime sinks.
pub struct NoPlayApplyEffects;

impl PlayApplyEffects for NoPlayApplyEffects {}

fn tracking_emitter_for_entity_animation(action: u8) -> Option<EntityTrackingEmitterParticleKind> {
    match action {
        // Vanilla `ClientPacketListener.handleAnimate`: action 4 calls
        // `createTrackingEmitter(entity, ParticleTypes.CRIT)`.
        CRITICAL_HIT_ANIMATION_ACTION => Some(EntityTrackingEmitterParticleKind::Crit),
        // Vanilla action 5 calls
        // `createTrackingEmitter(entity, ParticleTypes.ENCHANTED_HIT)`.
        MAGIC_CRITICAL_HIT_ANIMATION_ACTION => {
            Some(EntityTrackingEmitterParticleKind::EnchantedHit)
        }
        _ => None,
    }
}

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
            PlayClientbound::BundleDelimiter => {}
            PlayClientbound::AddEntity(entity) => {
                self.apply_add_entity(entity);
            }
            PlayClientbound::EntityAnimation(update) => {
                let tracking_emitter = tracking_emitter_for_entity_animation(update.action);
                let applied = self.apply_entity_animation(update);
                if let (true, Some(kind)) = (applied, tracking_emitter) {
                    effects.tracking_emitter_particles(
                        self,
                        update.id,
                        kind,
                        TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS,
                    );
                }
            }
            PlayClientbound::AwardStats(update) => {
                self.apply_award_stats(update);
            }
            PlayClientbound::BlockDestruction(update) => {
                self.apply_block_destruction(update);
            }
            PlayClientbound::BossEvent(update) => {
                self.apply_boss_event(update);
            }
            PlayClientbound::ChangeDifficulty(update) => {
                self.apply_change_difficulty(update);
            }
            PlayClientbound::Cooldown(update) => {
                self.apply_cooldown(update);
            }
            PlayClientbound::CustomChatCompletions(update) => {
                self.apply_custom_chat_completions(update);
            }
            PlayClientbound::CustomPayload(update) => {
                self.apply_custom_payload(update);
            }
            PlayClientbound::CustomReportDetails(details) => {
                self.apply_custom_report_details(details);
            }
            PlayClientbound::ServerLinks(links) => {
                self.apply_server_links(links);
            }
            PlayClientbound::DamageEvent(update) => {
                self.apply_damage_event(update);
            }
            PlayClientbound::DebugBlockValue(update) => {
                self.apply_debug_block_value(update);
            }
            PlayClientbound::DebugChunkValue(update) => {
                self.apply_debug_chunk_value(update);
            }
            PlayClientbound::DebugEntityValue(update) => {
                self.apply_debug_entity_value(update);
            }
            PlayClientbound::DebugEvent(update) => {
                self.apply_debug_event(update);
            }
            PlayClientbound::DebugSample(update) => {
                self.apply_debug_sample(update);
            }
            PlayClientbound::DeleteChat(update) => {
                self.apply_delete_chat(update);
            }
            PlayClientbound::DisguisedChat(update) => {
                self.apply_disguised_chat(update);
            }
            PlayClientbound::UpdateMobEffect(update) => {
                self.apply_update_mob_effect(update);
            }
            PlayClientbound::UpdateTags(update) => {
                self.apply_update_tags(update);
            }
            PlayClientbound::RemoveMobEffect(update) => {
                self.apply_remove_mob_effect(update);
            }
            PlayClientbound::MoveEntity(update) => {
                self.apply_entity_move(update);
            }
            PlayClientbound::MoveMinecartAlongTrack(update) => {
                self.apply_move_minecart_along_track(update);
            }
            PlayClientbound::MoveVehicle(update) => {
                if let Some(report) = self.apply_move_vehicle(update) {
                    effects.vehicle_move_report(report);
                }
            }
            PlayClientbound::LowDiskSpaceWarning => {
                self.apply_low_disk_space_warning();
            }
            PlayClientbound::MapItemData(update) => {
                self.apply_map_item_data(update);
            }
            PlayClientbound::MountScreenOpen(update) => {
                self.apply_mount_screen_open(update);
            }
            PlayClientbound::ContainerClose(update) => {
                self.apply_container_close(update);
            }
            PlayClientbound::ContainerSetContent(update) => {
                self.apply_container_set_content(update);
            }
            PlayClientbound::ContainerSetData(update) => {
                self.apply_container_set_data(update);
            }
            PlayClientbound::ContainerSetSlot(update) => {
                self.apply_container_set_slot(update);
            }
            PlayClientbound::MerchantOffers(update) => {
                self.apply_merchant_offers(update);
            }
            PlayClientbound::OpenScreen(update) => {
                self.apply_open_screen(update);
            }
            PlayClientbound::OpenBook(update) => {
                self.apply_open_book(update);
            }
            PlayClientbound::OpenSignEditor(update) => {
                self.apply_open_sign_editor(update);
            }
            PlayClientbound::PlaceGhostRecipe(update) => {
                self.apply_place_ghost_recipe(update);
            }
            PlayClientbound::PongResponse(update) => {
                self.apply_pong_response(update);
            }
            PlayClientbound::Login(login) => {
                self.apply_login(&login);
            }
            PlayClientbound::Respawn(respawn) => {
                self.apply_respawn(&respawn);
            }
            PlayClientbound::SetHealth(health) => {
                self.apply_player_health(health);
            }
            PlayClientbound::EntityPositionSync(update) => {
                self.apply_entity_position_sync(update);
            }
            PlayClientbound::Explosion(update) => {
                self.apply_explosion(update);
            }
            PlayClientbound::EntityEvent(update) => {
                let firework_empty_explosions_position = if update.event_id == 17 {
                    self.firework_rocket_empty_explosions_position(update.entity_id)
                        .map(|position| [position.x, position.y, position.z])
                } else {
                    None
                };
                let applied = self.apply_entity_event(update);
                if let Some(position) = firework_empty_explosions_position {
                    effects.firework_empty_explosion_particles(self, position);
                }
                if applied && update.event_id == 35 {
                    effects.tracking_emitter_particles(
                        self,
                        update.entity_id,
                        EntityTrackingEmitterParticleKind::TotemOfUndying,
                        TOTEM_TRACKING_EMITTER_LIFETIME_TICKS,
                    );
                    if let Some(state) = self.totem_use_sound_for_entity(update.entity_id) {
                        effects.positioned_sound(&state);
                    }
                }
            }
            PlayClientbound::HurtAnimation(update) => {
                self.apply_hurt_animation(update);
            }
            PlayClientbound::RemoveEntities(update) => {
                self.apply_remove_entities(update);
            }
            PlayClientbound::RotateHead(update) => {
                self.apply_rotate_head(update);
            }
            PlayClientbound::SetEntityMotion(update) => {
                self.apply_set_entity_motion(update);
            }
            PlayClientbound::SetEntityLink(update) => {
                self.apply_set_entity_link(update);
            }
            PlayClientbound::SetEquipment(update) => {
                self.apply_set_equipment(update);
            }
            PlayClientbound::TakeItemEntity(update) => {
                let pickup_sound = self
                    .take_item_entity_pickup_sound_with_random(update.item_id, || {
                        random.next_float()
                    });
                if self.apply_take_item_entity(update) {
                    if let Some(state) =
                        pickup_sound.map(|state| self.record_positioned_sound(state))
                    {
                        effects.positioned_sound(&state);
                    }
                }
            }
            PlayClientbound::SetPassengers(update) => {
                self.apply_set_passengers(update);
            }
            PlayClientbound::UpdateAttributes(update) => {
                self.apply_update_attributes(update);
            }
            PlayClientbound::SetEntityData(update) => {
                self.apply_set_entity_data(update);
            }
            PlayClientbound::TeleportEntity(update) => {
                self.apply_teleport_entity(update);
            }
            PlayClientbound::PlayerAbilities(update) => {
                self.apply_player_abilities(update);
            }
            PlayClientbound::PlayerChat(update) => {
                if let Some(command) = self.apply_player_chat(update) {
                    effects.chat_acknowledgement(command);
                }
            }
            PlayClientbound::SetExperience(update) => {
                self.apply_player_experience(update);
            }
            PlayClientbound::SetHeldSlot(update) => {
                self.apply_held_slot(update);
            }
            PlayClientbound::SetCursorItem(update) => {
                self.apply_set_cursor_item(update);
            }
            PlayClientbound::SetPlayerInventory(update) => {
                self.apply_set_player_inventory(update);
            }
            PlayClientbound::SetDefaultSpawnPosition(update) => {
                self.apply_default_spawn_position(update);
            }
            PlayClientbound::SetSimulationDistance(update) => {
                self.apply_simulation_distance(update);
            }
            PlayClientbound::SystemChat(update) => {
                self.apply_system_chat(update);
            }
            PlayClientbound::PlayerCombatEnd(update) => {
                self.apply_player_combat_end(update);
            }
            PlayClientbound::PlayerCombatEnter => {
                self.apply_player_combat_enter();
            }
            PlayClientbound::PlayerCombatKill(update) => {
                self.apply_player_combat_kill(update);
            }
            PlayClientbound::PlayerLookAt(update) => {
                self.apply_player_look_at(update);
            }
            PlayClientbound::SetActionBarText(update) => {
                self.apply_action_bar_text(update);
            }
            PlayClientbound::SetTitleText(update) => {
                self.apply_title_text(update);
            }
            PlayClientbound::SetSubtitleText(update) => {
                self.apply_subtitle_text(update);
            }
            PlayClientbound::ClearTitles(update) => {
                self.apply_clear_titles(update);
            }
            PlayClientbound::SetTitlesAnimation(update) => {
                self.apply_titles_animation(update);
            }
            PlayClientbound::Sound(update) => {
                let state = self.apply_sound_event(update);
                effects.positioned_sound(&state);
            }
            PlayClientbound::SoundEntity(update) => {
                if let Some(state) = self.apply_sound_entity_event(update) {
                    let position = self
                        .probe_entity_transform(state.entity_id)
                        .map(|entity| [entity.position.x, entity.position.y, entity.position.z]);
                    effects.entity_sound(&state, position);
                }
            }
            PlayClientbound::StopSound(update) => {
                let state = self.apply_stop_sound(update);
                effects.stop_sound(&state);
            }
            PlayClientbound::TickingState(update) => {
                self.apply_ticking_state(update);
            }
            PlayClientbound::TickingStep(update) => {
                self.apply_ticking_step(update);
            }
            PlayClientbound::Transfer(update) => {
                self.apply_transfer(update);
            }
            PlayClientbound::SetCamera(update) => {
                self.apply_set_camera(update);
            }
            PlayClientbound::InitializeBorder(border) => {
                self.apply_initialize_border(border);
            }
            PlayClientbound::SetBorderCenter(update) => {
                self.apply_set_border_center(update);
            }
            PlayClientbound::SetBorderLerpSize(update) => {
                self.apply_set_border_lerp_size(update);
            }
            PlayClientbound::SetBorderSize(update) => {
                self.apply_set_border_size(update);
            }
            PlayClientbound::SetBorderWarningDelay(update) => {
                self.apply_set_border_warning_delay(update);
            }
            PlayClientbound::SetBorderWarningDistance(update) => {
                self.apply_set_border_warning_distance(update);
            }
            PlayClientbound::ResetScore(update) => {
                self.apply_reset_score(update);
            }
            PlayClientbound::SetDisplayObjective(update) => {
                self.apply_set_display_objective(update);
            }
            PlayClientbound::SetObjective(update) => {
                self.apply_set_objective(update);
            }
            PlayClientbound::SetPlayerTeam(update) => {
                self.apply_set_player_team(update);
            }
            PlayClientbound::SetScore(update) => {
                self.apply_set_score(update);
            }
            PlayClientbound::Commands(update) => {
                self.apply_commands(update);
            }
            PlayClientbound::CommandSuggestions(update) => {
                self.apply_command_suggestions(update);
            }
            PlayClientbound::SelectAdvancementsTab(update) => {
                self.apply_select_advancements_tab(update);
            }
            PlayClientbound::TagQuery(update) => {
                self.apply_tag_query(update);
            }
            PlayClientbound::ClearDialog => {
                self.apply_clear_dialog();
            }
            PlayClientbound::ShowDialog(update) => {
                self.apply_show_dialog(update);
            }
            PlayClientbound::TestInstanceBlockStatus(update) => {
                self.apply_test_instance_block_status(update);
            }
            PlayClientbound::TabList(update) => {
                self.apply_tab_list(update);
            }
            PlayClientbound::BlockChangedAck(update) => {
                self.apply_block_changed_ack(update);
            }
            PlayClientbound::BlockEntityData(update) => {
                let _ = self.apply_block_entity_data(update);
            }
            PlayClientbound::BlockEvent(event) => {
                self.apply_block_event(event);
            }
            PlayClientbound::LevelEvent(event) => {
                self.apply_level_event_with_effects(event, random, effects);
            }
            PlayClientbound::GameEvent(update) => {
                self.apply_game_event(update);
                if let Some(position) = self.elder_guardian_effect_particle_position(update) {
                    effects.elder_guardian_effect_particles(self, position);
                }
                if let Some(state) = self.game_event_positioned_sound(update) {
                    effects.positioned_sound(&state);
                }
            }
            PlayClientbound::GameRuleValues(update) => {
                self.apply_game_rule_values(update);
            }
            PlayClientbound::GameTestHighlightPos(update) => {
                self.apply_game_test_highlight_pos(update);
            }
            PlayClientbound::SetTime(update) => {
                self.apply_world_time(update);
            }
            PlayClientbound::PlayerPosition(update) => {
                self.apply_player_position(update);
                return Some(PlayClientbound::PlayerPosition(update));
            }
            PlayClientbound::PlayerRotation(update) => {
                self.apply_player_rotation(update);
                return Some(PlayClientbound::PlayerRotation(update));
            }
            PlayClientbound::PlayerInfoUpdate(update) => {
                self.apply_player_info_update(update);
            }
            PlayClientbound::PlayerInfoRemove(update) => {
                self.apply_player_info_remove(update);
            }
            PlayClientbound::ServerData(update) => {
                self.apply_server_data(update);
            }
            PlayClientbound::ResourcePackPop(update) => {
                self.apply_resource_pack_pop(update);
            }
            PlayClientbound::LevelChunkWithLight(chunk) => {
                if let Ok(pos) = self.insert_level_chunk_with_light(chunk) {
                    effects.chunk_inserted(pos);
                }
            }
            PlayClientbound::LevelParticles(update) => {
                self.apply_level_particles(update.clone());
                effects.level_particles(self, &update);
            }
            PlayClientbound::LightUpdate(update) => {
                let _ = self.apply_light_update(update);
            }
            PlayClientbound::ChunksBiomes(update) => {
                let _ = self.apply_biome_update(update);
            }
            PlayClientbound::ForgetLevelChunk(update) => {
                self.forget_chunk(ChunkPos {
                    x: update.pos.x,
                    z: update.pos.z,
                });
            }
            PlayClientbound::BlockUpdate(update) => {
                self.apply_block_update(update);
            }
            PlayClientbound::SectionBlocksUpdate(update) => {
                self.apply_section_blocks_update(update);
            }
            PlayClientbound::SetChunkCacheCenter(update) => {
                self.apply_set_chunk_cache_center(update);
            }
            PlayClientbound::SetChunkCacheRadius(update) => {
                self.apply_set_chunk_cache_radius(update);
            }
            PlayClientbound::ProjectilePower(update) => {
                self.apply_projectile_power(update);
            }
            PlayClientbound::Waypoint(update) => {
                self.apply_waypoint(update);
            }
            PlayClientbound::RecipeBookAdd(update) => {
                self.apply_recipe_book_add(update);
            }
            PlayClientbound::RecipeBookRemove(update) => {
                self.apply_recipe_book_remove(update);
            }
            PlayClientbound::RecipeBookSettings(update) => {
                self.apply_recipe_book_settings(update);
            }
            PlayClientbound::UpdateAdvancements(update) => {
                self.apply_update_advancements(update);
            }
            PlayClientbound::UpdateRecipes(update) => {
                self.apply_update_recipes(update);
            }
            PlayClientbound::ResourcePackPush(update) => {
                // The push is world-owned; the response action and serverbound
                // reply stay with the caller's net context.
                self.apply_resource_pack_push(update.clone());
                return Some(PlayClientbound::ResourcePackPush(update));
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
            | PlayClientbound::Unknown { .. }) => return Some(packet),
        }
        None
    }

    fn elder_guardian_effect_particle_position(
        &self,
        event: ProtocolGameEvent,
    ) -> Option<ProtocolVec3d> {
        (event.event_id == GUARDIAN_ELDER_EFFECT_GAME_EVENT)
            .then(|| self.local_player_pose().map(|pose| pose.position))
            .flatten()
    }

    /// Vanilla `LevelRenderer.levelEvent` client effects: canonical state,
    /// jukebox/global/local/positioned sounds, and the deterministic particle
    /// random stream shared between sink-less and sink-driven callers.
    fn apply_level_event_with_effects(
        &mut self,
        event: ProtocolLevelEvent,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let jukebox_event = self.apply_level_event(event);
        if let Some(jukebox_event) = jukebox_event {
            effects.jukebox_level_event(&jukebox_event);
        }
        if let Some(state) = camera_audio_position(self)
            .and_then(|camera_position| self.global_level_event_sound(event, camera_position))
        {
            let state = self.record_positioned_sound(with_level_event_sound_seed(state, random));
            effects.positioned_sound(&state);
        }
        if let Some(state) = self
            .level_event_local_sound_with_random(event, || random.next_float())
            .map(|state| self.record_local_sound(state))
        {
            effects.local_sound(&state);
        }
        if matches!(
            event.event_type,
            POTION_BREAK_LEVEL_EVENT | INSTANT_POTION_BREAK_LEVEL_EVENT
        ) {
            let particles_consumed_random = effects.level_event_particles(self, &event, random);
            if !particles_consumed_random {
                advance_potion_break_level_event_particle_randoms(random);
            }
            if let Some(state) = self.level_event_sound_with_random(event, || random.next_float()) {
                let state =
                    self.record_positioned_sound(with_level_event_sound_seed(state, random));
                effects.positioned_sound(&state);
            }
        } else if event.event_type == DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT {
            let particles_consumed_random = effects.level_event_particles(self, &event, random);
            if !particles_consumed_random {
                advance_dragon_fireball_explode_level_event_particle_randoms(random);
            }
            if let Some(state) = self.level_event_sound_with_random(event, || random.next_float()) {
                let state =
                    self.record_positioned_sound(with_level_event_sound_seed(state, random));
                effects.positioned_sound(&state);
            }
        } else if event.event_type == WAX_ON_LEVEL_EVENT {
            let particles_consumed_random = effects.level_event_particles(self, &event, random);
            if !particles_consumed_random {
                advance_wax_on_level_event_particle_randoms(random);
            }
            if let Some(state) = self.level_event_sound(event) {
                let state =
                    self.record_positioned_sound(with_level_event_sound_seed(state, random));
                effects.positioned_sound(&state);
            }
        } else if event.event_type == PLANT_GROWTH_LEVEL_EVENT {
            let particles_consumed_random = effects.level_event_particles(self, &event, random);
            if !particles_consumed_random {
                if let Some(mode) = effects.growth_particle_random_mode(self, &event) {
                    advance_growth_level_event_particle_randoms(event.data, mode, random);
                }
            }
            if let Some(state) = self.level_event_sound(event) {
                let state =
                    self.record_positioned_sound(with_level_event_sound_seed(state, random));
                effects.positioned_sound(&state);
            }
        } else if event.event_type == COBWEB_PLACE_LEVEL_EVENT {
            let particles_consumed_random = effects.level_event_particles(self, &event, random);
            if !particles_consumed_random {
                advance_cobweb_place_particle_randoms(random);
            }
            if let Some(state) =
                self.cobweb_place_level_event_sound_with_random(event, || random.next_float())
            {
                let state =
                    self.record_positioned_sound(with_level_event_sound_seed(state, random));
                effects.positioned_sound(&state);
            }
        } else if event.event_type == SCULK_SHRIEKER_LEVEL_EVENT {
            effects.level_event_particles(self, &event, random);
            if let Some(state) =
                self.sculk_shrieker_level_event_sound_with_random(event, || random.next_float())
            {
                let state =
                    self.record_positioned_sound(with_level_event_sound_seed(state, random));
                effects.positioned_sound(&state);
            }
        } else if matches!(
            event.event_type,
            VAULT_ACTIVATE_LEVEL_EVENT | VAULT_DEACTIVATE_LEVEL_EVENT
        ) {
            let vault_block_entity_at_event_pos = event.event_type == VAULT_ACTIVATE_LEVEL_EVENT
                && self.block_entity_type_id_at(BlockPos {
                    x: event.pos.x,
                    y: event.pos.y,
                    z: event.pos.z,
                }) == Some(VANILLA_VAULT_BLOCK_ENTITY_TYPE_ID);
            let should_advance_particle_random =
                event.event_type == VAULT_DEACTIVATE_LEVEL_EVENT || vault_block_entity_at_event_pos;
            let particles_consumed_random = effects.level_event_particles(self, &event, random);
            if should_advance_particle_random && !particles_consumed_random {
                match event.event_type {
                    VAULT_ACTIVATE_LEVEL_EVENT => advance_vault_activation_particle_randoms(random),
                    VAULT_DEACTIVATE_LEVEL_EVENT => {
                        advance_vault_deactivation_particle_randoms(random)
                    }
                    _ => {}
                }
            }
            if let Some(state) =
                self.vault_level_event_sound_with_random(event, || random.next_float())
            {
                let state =
                    self.record_positioned_sound(with_level_event_sound_seed(state, random));
                effects.positioned_sound(&state);
            }
        } else {
            if let Some(state) = self.level_event_sound_with_random(event, || random.next_float()) {
                let state =
                    self.record_positioned_sound(with_level_event_sound_seed(state, random));
                effects.positioned_sound(&state);
            }
            let particles_consumed_random = effects.level_event_particles(self, &event, random);
            if !particles_consumed_random {
                advance_level_event_particle_randoms_without_sink(self, effects, event, random);
            }
        }
    }
}

/// Vanilla camera-relative audio position: the current camera entity when it
/// is not following the player, otherwise the local player's eye position.
fn camera_audio_position(world: &WorldStore) -> Option<ProtocolVec3d> {
    let camera = world.local_player().camera;
    if let Some(camera_id) = camera.entity_id {
        if !camera.follows_player {
            if let Some(camera_pose) = world.probe_entity_camera_pose(camera_id) {
                return Some(ProtocolVec3d {
                    x: camera_pose.position.x,
                    y: camera_pose.position.y + f64::from(camera_pose.eye_height),
                    z: camera_pose.position.z,
                });
            }
        }
    }

    world.local_player_pose().map(|pose| ProtocolVec3d {
        x: pose.position.x,
        y: pose.position.y + pose.eye_height(),
        z: pose.position.z,
    })
}

fn with_level_event_sound_seed(
    mut state: SoundEventState,
    random: &mut LevelEventSoundRandomState,
) -> SoundEventState {
    state.seed = random.next_long();
    state
}

fn sculk_charge_pop_full_block_context(
    world: &WorldStore,
    event: &ProtocolLevelEvent,
) -> Option<bool> {
    if event.event_type != SCULK_CHARGE_LEVEL_EVENT || event.data >> 6 > 0 {
        return None;
    }
    // Vanilla `LevelEventHandler` event 3006 pop branch calls
    // `BlockState.isCollisionShapeFullBlock(level, pos)` before choosing
    // the 20 vs 40 particle random stream.
    let pos = protocol_to_world_block_pos(event.pos);
    world
        .probe_block(pos)
        .map(|probe| crate::client::block_collision_shape_is_full_block(&probe, pos))
}

fn growth_particle_random_mode_context(
    world: &WorldStore,
    event: &ProtocolLevelEvent,
) -> Option<LevelEventGrowthRandomMode> {
    if event.event_type != PLANT_GROWTH_LEVEL_EVENT || event.data <= 0 {
        return None;
    }
    let probe = world.probe_block(protocol_to_world_block_pos(event.pos))?;
    let block_name = probe.block_name.as_deref()?;

    // Vanilla `BoneMealItem.addGrowthParticles` branches on water or
    // `BonemealableBlock.Type`; sink-less callers only need that mode to
    // consume the same particle random count before the follow-up sound seed.
    if block_name == "minecraft:water" || is_neighbor_spreader_bonemealable_block_name(block_name) {
        return Some(LevelEventGrowthRandomMode::WideNoFloating);
    }
    if is_below_particle_pos_bonemealable_block_name(block_name)
        || is_grower_bonemealable_block_name(block_name)
    {
        return Some(LevelEventGrowthRandomMode::InBlock);
    }
    None
}

fn protocol_to_world_block_pos(pos: bbb_protocol::packets::BlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn is_neighbor_spreader_bonemealable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:grass_block"
            | "minecraft:netherrack"
            | "minecraft:warped_nylium"
            | "minecraft:crimson_nylium"
            | "minecraft:moss_block"
            | "minecraft:pale_moss_block"
    )
}

fn is_below_particle_pos_bonemealable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:rooted_dirt" | "minecraft:mangrove_leaves"
    )
}

fn is_grower_bonemealable_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:oak_sapling"
            | "minecraft:spruce_sapling"
            | "minecraft:birch_sapling"
            | "minecraft:jungle_sapling"
            | "minecraft:acacia_sapling"
            | "minecraft:cherry_sapling"
            | "minecraft:dark_oak_sapling"
            | "minecraft:pale_oak_sapling"
            | "minecraft:short_grass"
            | "minecraft:fern"
            | "minecraft:bush"
            | "minecraft:short_dry_grass"
            | "minecraft:tall_dry_grass"
            | "minecraft:seagrass"
            | "minecraft:sea_pickle"
            | "minecraft:wheat"
            | "minecraft:carrots"
            | "minecraft:potatoes"
            | "minecraft:beetroots"
            | "minecraft:pumpkin_stem"
            | "minecraft:melon_stem"
            | "minecraft:cocoa"
            | "minecraft:torchflower_crop"
            | "minecraft:pitcher_crop"
            | "minecraft:bamboo_sapling"
            | "minecraft:bamboo"
            | "minecraft:sweet_berry_bush"
            | "minecraft:warped_fungus"
            | "minecraft:crimson_fungus"
            | "minecraft:azalea"
            | "minecraft:flowering_azalea"
            | "minecraft:pink_petals"
            | "minecraft:wildflowers"
            | "minecraft:big_dripleaf"
            | "minecraft:big_dripleaf_stem"
            | "minecraft:small_dripleaf"
            | "minecraft:pale_moss_carpet"
            | "minecraft:pale_hanging_moss"
            | "minecraft:firefly_bush"
            | "minecraft:hanging_moss"
            | "minecraft:glow_lichen"
            | "minecraft:sunflower"
            | "minecraft:lilac"
            | "minecraft:rose_bush"
            | "minecraft:peony"
            | "minecraft:brown_mushroom"
            | "minecraft:red_mushroom"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:kelp"
            | "minecraft:kelp_plant"
    )
}

/// Advances the particle random stream exactly as the renderer particle sink
/// would for level events with data-only particle providers.
fn advance_level_event_particle_randoms_without_sink(
    world: &WorldStore,
    effects: &mut dyn PlayApplyEffects,
    event: ProtocolLevelEvent,
    random: &mut LevelEventSoundRandomState,
) {
    match event.event_type {
        LAVA_EXTINGUISH_LEVEL_EVENT => {
            for _ in 0..8 {
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        REDSTONE_TORCH_BURNOUT_LEVEL_EVENT => {
            for _ in 0..5 {
                let _ = random.next_double();
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        END_PORTAL_FRAME_FILL_LEVEL_EVENT => {
            for _ in 0..16 {
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        DISPENSER_SMOKE_LEVEL_EVENT | DISPENSER_WHITE_SMOKE_LEVEL_EVENT => {
            advance_shoot_particles_randoms(random);
        }
        ENDER_EYE_BREAK_LEVEL_EVENT => {
            advance_item_break_particle_randoms(random);
        }
        BLAZE_SMOKE_LEVEL_EVENT => {
            for _ in 0..20 {
                let _ = random.next_double();
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        SPLASH_CLOUD_LEVEL_EVENT => {
            for _ in 0..8 {
                let _ = random.next_double();
                let _ = random.next_double();
            }
        }
        ELECTRIC_SPARK_LEVEL_EVENT => {
            if matches!(event.data, 0..=2) {
                advance_axis_particles_randoms(10, 19, random);
            } else {
                advance_block_face_particle_randoms(3, 5, random);
            }
        }
        WAX_OFF_LEVEL_EVENT | SCRAPE_LEVEL_EVENT => {
            advance_block_face_particle_randoms(3, 5, random);
        }
        EGG_CRACK_LEVEL_EVENT => {
            advance_block_face_particle_randoms(3, 6, random);
        }
        TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT | TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT => {
            advance_trial_spawner_spawn_particle_randoms(random);
        }
        TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT
        | TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT => {
            advance_trial_spawner_detect_player_particle_randoms(event.data, random);
        }
        TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT => {
            advance_trial_spawner_eject_item_particle_randoms(random);
        }
        TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT => {
            advance_trial_spawner_detect_player_particle_randoms(0, random);
            advance_trial_spawner_become_ominous_particle_randoms(random);
        }
        SCULK_CHARGE_LEVEL_EVENT => {
            let pop_full_block = effects.sculk_charge_pop_full_block(world, &event);
            advance_sculk_charge_level_event_particle_randoms(event, pop_full_block, random);
        }
        _ => {}
    }
}

fn advance_shoot_particles_randoms(random: &mut LevelEventSoundRandomState) {
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

fn advance_item_break_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..8 {
        let _ = random.next_gaussian();
        let _ = random.next_double();
        let _ = random.next_gaussian();
    }
}

fn advance_block_face_particle_randoms(
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

fn advance_axis_particles_randoms(
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

fn advance_sculk_charge_level_event_particle_randoms(
    event: ProtocolLevelEvent,
    pop_full_block: Option<bool>,
    random: &mut LevelEventSoundRandomState,
) {
    let count = event.data >> 6;
    if count <= 0 {
        let particle_count = if pop_full_block.unwrap_or(false) {
            40
        } else {
            20
        };
        for _ in 0..particle_count {
            let _ = random.next_float();
            let _ = random.next_float();
            let _ = random.next_float();
        }
        return;
    }

    let particle_data = event.data & 63;
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

fn advance_trial_spawner_spawn_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
    }
}

fn advance_trial_spawner_detect_player_particle_randoms(
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

fn advance_trial_spawner_eject_item_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

fn advance_trial_spawner_become_ominous_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..20 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
    }
}

/// Advances the particle random stream for a sink-less plant-growth level
/// event with the given data count and spawn mode.
pub fn advance_growth_level_event_particle_randoms(
    data: i32,
    mode: LevelEventGrowthRandomMode,
    random: &mut LevelEventSoundRandomState,
) {
    let count = match mode {
        LevelEventGrowthRandomMode::InBlock => data,
        LevelEventGrowthRandomMode::WideNoFloating => data.wrapping_mul(3),
    };
    advance_particle_utils_spawn_particles_randoms(count, random);
}

pub fn advance_wax_on_level_event_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..6 {
        let particle_count = random.next_int_bound(3) + 3;
        for _ in 0..particle_count {
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
            let _ = random.next_double();
        }
    }
}

pub fn advance_dragon_fireball_explode_level_event_particle_randoms(
    random: &mut LevelEventSoundRandomState,
) {
    for _ in 0..200 {
        let _ = random.next_float();
        let _ = random.next_float();
        let _ = random.next_double();
    }
}

pub fn advance_potion_break_level_event_particle_randoms(random: &mut LevelEventSoundRandomState) {
    for _ in 0..8 {
        let _ = random.next_gaussian();
        let _ = random.next_double();
        let _ = random.next_gaussian();
    }
    for _ in 0..100 {
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_float();
    }
}

fn advance_particle_utils_spawn_particles_randoms(
    count: i32,
    random: &mut LevelEventSoundRandomState,
) {
    for _ in 0..count.max(0) {
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_gaussian();
        let _ = random.next_double();
        let _ = random.next_double();
        let _ = random.next_double();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalPlayerPoseState;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    use bbb_protocol::packets::{
        AddEntity, BlockPos as ProtocolBlockPos, EntityAnimation, EntityEvent, GameEvent,
        LevelEvent, PlayTime, SoundEvent, SoundEventHolder, SoundSource, TakeItemEntity, Vec3d,
    };
    use uuid::Uuid;

    #[derive(Default)]
    struct RecordingEffects {
        positioned_sounds: Vec<SoundEventState>,
        elder_guardian_effect_particles: Vec<Vec3d>,
        tracking_emitters: Vec<(i32, EntityTrackingEmitterParticleKind, u32)>,
    }

    impl PlayApplyEffects for RecordingEffects {
        fn positioned_sound(&mut self, state: &SoundEventState) {
            self.positioned_sounds.push(state.clone());
        }

        fn elder_guardian_effect_particles(&mut self, _world: &WorldStore, position: Vec3d) {
            self.elder_guardian_effect_particles.push(position);
        }

        fn tracking_emitter_particles(
            &mut self,
            _world: &WorldStore,
            entity_id: i32,
            kind: EntityTrackingEmitterParticleKind,
            lifetime_ticks: u32,
        ) {
            self.tracking_emitters
                .push((entity_id, kind, lifetime_ticks));
        }
    }

    fn level_event(event_type: i32, data: i32) -> LevelEvent {
        LevelEvent {
            event_type,
            pos: ProtocolBlockPos { x: 1, y: 64, z: -3 },
            data,
            global: false,
        }
    }

    fn add_entity(entity_id: i32, entity_type_id: i32, position: Vec3d) -> AddEntity {
        AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(entity_id as u128),
            entity_type_id,
            position,
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    #[test]
    fn apply_play_packet_applies_world_owned_packets_and_returns_none() {
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(0);

        let leftover = store.apply_play_packet(
            PlayClientbound::SetTime(PlayTime {
                game_time: 24000,
                clock_updates: Vec::new(),
            }),
            &mut random,
            &mut NoPlayApplyEffects,
        );

        assert!(leftover.is_none());
        assert_eq!(store.world_time().map(|time| time.game_time), Some(24000));
    }

    #[test]
    fn apply_play_packet_returns_connection_owned_packets_unapplied() {
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(0);

        let leftover = store.apply_play_packet(
            PlayClientbound::KeepAlive { id: 7 },
            &mut random,
            &mut NoPlayApplyEffects,
        );

        assert!(matches!(
            leftover,
            Some(PlayClientbound::KeepAlive { id: 7 })
        ));
    }

    #[test]
    fn apply_play_packet_applies_and_returns_player_position() {
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let update = bbb_protocol::packets::PlayerPositionUpdate {
            id: 3,
            position: Vec3d {
                x: 8.5,
                y: 65.0,
                z: -4.5,
            },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 90.0,
            x_rot: 0.0,
            relatives_mask: 0,
        };

        let leftover = store.apply_play_packet(
            PlayClientbound::PlayerPosition(update),
            &mut random,
            &mut NoPlayApplyEffects,
        );

        assert!(matches!(leftover, Some(PlayClientbound::PlayerPosition(_))));
        let pose = store.local_player_pose().expect("player pose");
        assert_eq!(pose.position.x, 8.5);
        assert_eq!(pose.position.z, -4.5);
    }

    #[test]
    fn apply_play_packet_forwards_positioned_sound_effects() {
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut effects = RecordingEffects::default();

        let leftover = store.apply_play_packet(
            PlayClientbound::Sound(SoundEvent {
                sound: SoundEventHolder::Direct {
                    location: "minecraft:block.stone.break".to_string(),
                    fixed_range: None,
                },
                source: SoundSource::Blocks,
                position: Vec3d {
                    x: 0.5,
                    y: 64.5,
                    z: 0.5,
                },
                volume: 1.0,
                pitch: 1.0,
                seed: 42,
            }),
            &mut random,
            &mut effects,
        );

        assert!(leftover.is_none());
        assert_eq!(effects.positioned_sounds.len(), 1);
        assert_eq!(effects.positioned_sounds[0].seed, 42);
    }

    #[test]
    fn take_item_entity_forwards_pickup_sounds() {
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut expected_random = LevelEventSoundRandomState::with_seed(0);
        let expected_item_pitch =
            (expected_random.next_float() - expected_random.next_float()) * 1.4 + 2.0;
        let expected_orb_pitch =
            (expected_random.next_float() - expected_random.next_float()) * 0.35 + 0.9;
        let expected_zombie_pitch =
            (expected_random.next_float() - expected_random.next_float()) * 1.4 + 2.0;
        let mut effects = RecordingEffects::default();

        for packet in [
            PlayClientbound::AddEntity(add_entity(
                10,
                VANILLA_ENTITY_TYPE_ITEM_ID,
                Vec3d {
                    x: 1.0,
                    y: 64.0,
                    z: -2.0,
                },
            )),
            PlayClientbound::AddEntity(add_entity(
                20,
                VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
                Vec3d {
                    x: 4.0,
                    y: 70.0,
                    z: 8.0,
                },
            )),
            PlayClientbound::AddEntity(add_entity(
                30,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                Vec3d {
                    x: -6.0,
                    y: 65.0,
                    z: 9.0,
                },
            )),
            PlayClientbound::TakeItemEntity(TakeItemEntity {
                item_id: 10,
                player_id: 99,
                amount: 1,
            }),
            PlayClientbound::TakeItemEntity(TakeItemEntity {
                item_id: 20,
                player_id: 99,
                amount: 1,
            }),
            PlayClientbound::TakeItemEntity(TakeItemEntity {
                item_id: 30,
                player_id: 99,
                amount: 1,
            }),
            PlayClientbound::TakeItemEntity(TakeItemEntity {
                item_id: 404,
                player_id: 99,
                amount: 1,
            }),
        ] {
            let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
            assert!(leftover.is_none());
        }

        assert_eq!(effects.positioned_sounds.len(), 3);
        assert_eq!(
            effects.positioned_sounds[0].sound.location.as_deref(),
            Some("minecraft:entity.item.pickup")
        );
        assert_eq!(effects.positioned_sounds[0].source, "player");
        assert_eq!(
            effects.positioned_sounds[0].position,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
        assert_eq!(effects.positioned_sounds[0].volume, 0.2);
        assert_eq!(effects.positioned_sounds[0].pitch, expected_item_pitch);
        assert_eq!(
            effects.positioned_sounds[1].sound.location.as_deref(),
            Some("minecraft:entity.experience_orb.pickup")
        );
        assert_eq!(effects.positioned_sounds[1].source, "player");
        assert_eq!(
            effects.positioned_sounds[1].position,
            Vec3d {
                x: 4.0,
                y: 70.0,
                z: 8.0,
            }
        );
        assert_eq!(effects.positioned_sounds[1].volume, 0.1);
        assert_eq!(effects.positioned_sounds[1].pitch, expected_orb_pitch);
        assert_eq!(
            effects.positioned_sounds[2].sound.location.as_deref(),
            Some("minecraft:entity.item.pickup")
        );
        assert_eq!(effects.positioned_sounds[2].source, "player");
        assert_eq!(
            effects.positioned_sounds[2].position,
            Vec3d {
                x: -6.0,
                y: 65.0,
                z: 9.0,
            }
        );
        assert_eq!(effects.positioned_sounds[2].volume, 0.2);
        assert_eq!(effects.positioned_sounds[2].pitch, expected_zombie_pitch);
        for sound in &effects.positioned_sounds {
            assert_eq!(sound.seed, 0);
            assert_eq!(sound.distance_delay, false);
        }
        assert_eq!(store.last_sound(), Some(&effects.positioned_sounds[2]));
        assert_eq!(store.counters().take_item_entities_received, 4);
        assert_eq!(store.counters().take_item_entities_applied, 3);
        assert_eq!(store.counters().take_item_entities_ignored, 1);
        assert_eq!(store.counters().take_item_entities_removed, 2);
        assert_eq!(store.entity_count(), 1);
    }

    #[test]
    fn game_events_forward_local_player_audio_and_particles() {
        let mut store = WorldStore::new();
        store.set_local_player_pose(LocalPlayerPoseState {
            position: Vec3d {
                x: 2.0,
                y: 64.0,
                z: -5.0,
            },
            ..LocalPlayerPoseState::default()
        });
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut effects = RecordingEffects::default();

        for packet in [
            PlayClientbound::GameEvent(GameEvent {
                event_id: 6,
                param: 0.0,
            }),
            PlayClientbound::GameEvent(GameEvent {
                event_id: 9,
                param: 0.0,
            }),
            PlayClientbound::GameEvent(GameEvent {
                event_id: 10,
                param: 1.75,
            }),
            PlayClientbound::GameEvent(GameEvent {
                event_id: 10,
                param: 0.0,
            }),
        ] {
            let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
            assert!(leftover.is_none());
        }

        assert_eq!(effects.positioned_sounds.len(), 3);
        assert_eq!(
            effects.positioned_sounds[0].sound.location.as_deref(),
            Some("minecraft:entity.arrow.hit_player")
        );
        assert_eq!(effects.positioned_sounds[0].source, "player");
        assert_eq!(
            effects.positioned_sounds[0].position,
            Vec3d {
                x: 2.0,
                y: 65.62,
                z: -5.0,
            }
        );
        assert_eq!(effects.positioned_sounds[0].volume, 0.18);
        assert_eq!(effects.positioned_sounds[0].pitch, 0.45);
        assert_eq!(
            effects.positioned_sounds[1].sound.location.as_deref(),
            Some("minecraft:entity.puffer_fish.sting")
        );
        assert_eq!(effects.positioned_sounds[1].source, "neutral");
        assert_eq!(
            effects.positioned_sounds[1].position,
            Vec3d {
                x: 2.0,
                y: 64.0,
                z: -5.0,
            }
        );
        assert_eq!(
            effects.positioned_sounds[2].sound.location.as_deref(),
            Some("minecraft:entity.elder_guardian.curse")
        );
        assert_eq!(effects.positioned_sounds[2].source, "hostile");
        assert_eq!(
            effects.elder_guardian_effect_particles,
            vec![
                Vec3d {
                    x: 2.0,
                    y: 64.0,
                    z: -5.0,
                },
                Vec3d {
                    x: 2.0,
                    y: 64.0,
                    z: -5.0,
                },
            ]
        );
        assert_eq!(store.last_sound(), Some(&effects.positioned_sounds[2]));
        assert_eq!(store.counters().game_event_packets, 4);
    }

    #[test]
    fn game_event_side_effects_require_local_player_pose() {
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut effects = RecordingEffects::default();

        for event_id in [6, 9, 10] {
            let leftover = store.apply_play_packet(
                PlayClientbound::GameEvent(GameEvent {
                    event_id,
                    param: 1.0,
                }),
                &mut random,
                &mut effects,
            );
            assert!(leftover.is_none());
        }

        assert!(effects.positioned_sounds.is_empty());
        assert!(effects.elder_guardian_effect_particles.is_empty());
        assert_eq!(store.counters().game_event_packets, 3);
    }

    #[test]
    fn entity_animation_forwards_crit_tracking_emitters() {
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut effects = RecordingEffects::default();

        for packet in [
            PlayClientbound::AddEntity(add_entity(
                10,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                Vec3d {
                    x: 1.0,
                    y: 64.0,
                    z: -2.0,
                },
            )),
            PlayClientbound::EntityAnimation(EntityAnimation { id: 10, action: 4 }),
            PlayClientbound::EntityAnimation(EntityAnimation { id: 10, action: 5 }),
            PlayClientbound::EntityAnimation(EntityAnimation { id: 10, action: 0 }),
            PlayClientbound::EntityAnimation(EntityAnimation { id: 404, action: 4 }),
        ] {
            let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
            assert!(leftover.is_none());
        }

        assert_eq!(
            effects.tracking_emitters,
            vec![
                (
                    10,
                    EntityTrackingEmitterParticleKind::Crit,
                    TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS,
                ),
                (
                    10,
                    EntityTrackingEmitterParticleKind::EnchantedHit,
                    TRACKING_EMITTER_DEFAULT_LIFETIME_TICKS,
                ),
            ]
        );
        assert_eq!(store.counters().entity_animation_updates_applied, 3);
        assert_eq!(store.counters().entity_animation_updates_ignored, 1);
    }

    #[test]
    fn totem_entity_event_forwards_positioned_use_sound_with_entity_source() {
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let mut effects = RecordingEffects::default();

        for packet in [
            PlayClientbound::AddEntity(add_entity(
                10,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                Vec3d {
                    x: 1.0,
                    y: 64.0,
                    z: -2.0,
                },
            )),
            PlayClientbound::AddEntity(add_entity(
                11,
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                Vec3d {
                    x: 4.5,
                    y: 70.0,
                    z: 8.25,
                },
            )),
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 10,
                event_id: 35,
            }),
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 11,
                event_id: 35,
            }),
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 404,
                event_id: 35,
            }),
        ] {
            let leftover = store.apply_play_packet(packet, &mut random, &mut effects);
            assert!(leftover.is_none());
        }

        assert_eq!(effects.positioned_sounds.len(), 2);
        assert_eq!(
            effects.positioned_sounds[0].sound.location.as_deref(),
            Some("minecraft:item.totem.use")
        );
        assert_eq!(effects.positioned_sounds[0].source, "hostile");
        assert_eq!(
            effects.positioned_sounds[0].position,
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
        assert_eq!(effects.positioned_sounds[1].source, "player");
        assert_eq!(
            effects.positioned_sounds[1].position,
            Vec3d {
                x: 4.5,
                y: 70.0,
                z: 8.25,
            }
        );
        assert_eq!(effects.positioned_sounds[0].volume, 1.0);
        assert_eq!(effects.positioned_sounds[0].pitch, 1.0);
        assert_eq!(effects.positioned_sounds[0].seed, 0);
        assert_eq!(effects.positioned_sounds[0].distance_delay, false);
        assert_eq!(
            effects.tracking_emitters,
            vec![
                (
                    10,
                    EntityTrackingEmitterParticleKind::TotemOfUndying,
                    TOTEM_TRACKING_EMITTER_LIFETIME_TICKS,
                ),
                (
                    11,
                    EntityTrackingEmitterParticleKind::TotemOfUndying,
                    TOTEM_TRACKING_EMITTER_LIFETIME_TICKS,
                ),
            ]
        );
        assert_eq!(store.last_sound(), Some(&effects.positioned_sounds[1]));
        assert_eq!(store.counters().entity_events_applied, 2);
        assert_eq!(store.counters().entity_events_ignored, 1);
    }

    #[test]
    fn sink_less_level_event_advances_particle_randoms_like_online_dispatcher() {
        // Lava extinguish (1501): the else-branch plays the randomized sound
        // first, then advances the sink-less particle random fallback.
        let mut store = WorldStore::new();
        let mut random = LevelEventSoundRandomState::with_seed(12345);
        store.apply_play_packet(
            PlayClientbound::LevelEvent(level_event(LAVA_EXTINGUISH_LEVEL_EVENT, 0)),
            &mut random,
            &mut NoPlayApplyEffects,
        );

        let mut expected_store = WorldStore::new();
        let mut expected = LevelEventSoundRandomState::with_seed(12345);
        let event = level_event(LAVA_EXTINGUISH_LEVEL_EVENT, 0);
        expected_store.apply_level_event(event);
        let state = expected_store
            .level_event_sound_with_random(event, || expected.next_float())
            .expect("lava extinguish sound");
        expected_store.record_positioned_sound(with_level_event_sound_seed(state, &mut expected));
        for _ in 0..8 {
            let _ = expected.next_double();
            let _ = expected.next_double();
        }

        assert_eq!(random.next_long(), expected.next_long());
        assert_eq!(
            store.counters().level_events_received,
            expected_store.counters().level_events_received
        );
    }
}
