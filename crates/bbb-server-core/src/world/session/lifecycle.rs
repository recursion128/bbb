//! Session join, initial state publication, recipe-book sync, and departure.

use std::sync::Arc;

use bbb_protocol::game_types::{GameType, VANILLA_GAME_TYPE_NOT_SET_ID};
use bbb_protocol::packets::{
    ChangeDifficulty, ClientInformation, CommonPlayerSpawnInfo, EntityEvent, GameProfile,
    PlayLogin, PlayerAbilities, PlayerInfoRemove, PlayerInput, PlayerPositionUpdate, RecipeBookAdd,
    RecipeBookAddEntry, RecipeBookSettings, RecipeBookTypeSettings as WireRecipeBookTypeSettings,
    SetHeldSlot, SystemChat, Vec3d,
};
use bbb_protocol::RichTextComponent;
use bbb_server_datapack::damage::PlayerDamageState;
use bbb_server_datapack::player_advancements::PlayerAdvancements;
use bbb_server_datapack::recipe::ItemTagIndex;
use bbb_server_types::experience::PlayerExperience as ExperienceState;
use bbb_server_types::food::FoodData;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::stats::{custom_stat, CUSTOM_STAT_LEAVE_GAME};

use crate::admin::NameAndId;
use crate::block_destroy::DestroyProgress;
use crate::chunk_sender::PlayerChunkSender;
use crate::commands;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::save::read_external_player_root_vehicle_input_with_context;
use crate::inventory::PlayerInventoryMenu;
use crate::registry_sync::SynchronizedRegistries;
use crate::world::player_save::{
    live_warden_spawn_tracker, restored_player_from_save_patch, DepartedPlayerSnapshot,
};
use crate::world::{
    block_position, initializing_player_info, initializing_player_info_entry, player_chunk, send,
    JoinContext, LevelId, PlayerSession, PlayerState, ServerWorld, WorldCloseReason, WorldEvent,
    CLIENT_LOADED_TIMEOUT_TICKS, LAST_SEEN_MESSAGES_MAX_LENGTH, UNSENT_EXPERIENCE, UNSENT_FOOD,
    UNSENT_HEALTH,
};

use super::abilities_after_game_type;

impl ServerWorld {
    pub(in crate::world) fn append_level_player(
        &mut self,
        dimension: &LevelId,
        connection_id: ConnectionId,
    ) {
        let ignored = self.player_loading_ticket_ignored(connection_id);
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.cold.filed_ticket_ignored = ignored;
        }
        let order = &mut self.level_mut_by_id(dimension).player_order;
        order.retain(|candidate| *candidate != connection_id);
        order.push(connection_id);
        self.update_player_query_section(connection_id);
    }

    pub(in crate::world) fn remove_level_player(
        &mut self,
        dimension: &LevelId,
        connection_id: ConnectionId,
    ) {
        let level = self.level_mut_by_id(dimension);
        level
            .player_order
            .retain(|candidate| *candidate != connection_id);
    }

    pub(in crate::world) fn join(
        &mut self,
        connection_id: ConnectionId,
        profile: GameProfile,
        client_information: ClientInformation,
        context: JoinContext,
        events: &mut Vec<WorldEvent>,
    ) {
        // The login carried the instant `canPlayerLogin` compares a ban expiry
        // against, which is the newest one this world has been told about.
        self.cold.wall_clock_millis = self.cold.wall_clock_millis.max(context.now);
        let user = NameAndId {
            id: profile.uuid,
            name: (&profile.name).into(),
        };
        let refusal = if self.stopping {
            Some(WorldCloseReason::ServerShutdown)
        } else if self.levels.contains_player(connection_id) {
            Some(WorldCloseReason::DuplicateConnection)
        } else {
            // `PlayerList.canPlayerLogin` (PlayerList.java:334-362) runs the
            // ban and allowlist checks before the capacity check, and its own
            // capacity branch exempts an operator who bypasses the limit.
            self.admin
                .login_refusal(&user, context.remote_address.as_deref(), context.now)
                .map(WorldCloseReason::Administrative)
        };
        // `PlayerList.disconnectAllPlayersWithProfile` (PlayerList.java:364-383)
        // kicks the session already online under this profile; its removal
        // saves it, which is what this session then loads.
        if refusal.is_none() {
            let duplicates = self
                .levels
                .player_entries()
                .filter(|(_, player)| player.profile.uuid == profile.uuid)
                .map(|(&duplicate, _)| duplicate)
                .collect::<Vec<_>>();
            for duplicate in duplicates {
                self.remove_player(duplicate, events);
                events.push(WorldEvent::Close {
                    connection_id: duplicate,
                    reason: WorldCloseReason::DuplicatePlayer,
                });
            }
        }
        let rejection = if refusal.is_some() {
            refusal
        } else if self
            .levels
            .players()
            .any(|player| player.profile.uuid == profile.uuid)
        {
            // `ServerConfigurationPacketListenerImpl.java:167-169`: a session
            // whose removal could not complete still holds the profile.
            Some(WorldCloseReason::DuplicatePlayer)
        } else if self.levels.player_count() >= self.max_players()
            && !self.admin.bypasses_player_limit(profile.uuid)
        {
            Some(WorldCloseReason::PlayerCapacity)
        } else {
            None
        };
        if let Some(reason) = rejection {
            events.push(WorldEvent::Close {
                connection_id,
                reason,
            });
            return;
        }

        // `Entity`'s constructor draws the avatar's id from the same
        // process-wide counter every other entity uses (`Entity.java:210`).
        let entity_id = self.entity_ids.next_id();
        let teleport_id = self.next_teleport_id;
        self.next_teleport_id = self.next_teleport_id.wrapping_add(1);
        let departed_patch = self
            .cold
            .departed_player_snapshots
            .get(&profile.uuid)
            .cloned();
        if departed_patch.is_none() {
            if let Err(error) = self.ensure_restored_player_loaded(profile.uuid) {
                tracing::error!(
                    event = "world.player_data_load_failed",
                    uuid = %profile.uuid,
                    %error,
                );
                events.push(WorldEvent::Close {
                    connection_id,
                    reason: WorldCloseReason::Administrative(RichTextComponent::plain(format!(
                        "Failed to load player data: {error}"
                    ))),
                });
                return;
            }
            if let Err(error) = self.ensure_restored_player_stats_loaded(profile.uuid) {
                tracing::error!(
                    event = "world.player_stats_load_failed",
                    uuid = %profile.uuid,
                    %error,
                );
                events.push(WorldEvent::Close {
                    connection_id,
                    reason: WorldCloseReason::Administrative(RichTextComponent::plain(format!(
                        "Failed to load player statistics: {error}"
                    ))),
                });
                return;
            }
            if self.has_advancement_manager() {
                if let Err(error) = self.ensure_restored_player_advancements_loaded(profile.uuid) {
                    tracing::error!(
                        event = "world.player_advancements_load_failed",
                        uuid = %profile.uuid,
                        %error,
                    );
                    events.push(WorldEvent::Close {
                        connection_id,
                        reason: WorldCloseReason::Administrative(RichTextComponent::plain(
                            format!("Failed to load player advancements: {error}"),
                        )),
                    });
                    return;
                }
            }
        }
        let departed_restored = match departed_patch.as_ref() {
            Some(snapshot) => {
                let restored =
                    restored_player_from_save_patch(&snapshot.patch).and_then(|mut restored| {
                        self.restore_player_items_for_rejoin(&snapshot.patch, &mut restored)?;
                        Ok(restored)
                    });
                match restored {
                    Ok(restored) => Some(restored),
                    Err(error) => {
                        events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                            format!("player rejoin deferred for {}: {error}", profile.uuid),
                        )));
                        return;
                    }
                }
            }
            None => None,
        };
        let restored = departed_restored
            .or_else(|| self.cold.restored_players.get(&profile.uuid).cloned())
            .map(|mut restored| {
                restored.dimension = restored
                    .dimension
                    .filter(|dimension| self.has_level_key(dimension));
                restored
            });
        // `ServerPlayer`'s tracker comes from the document it loaded
        // (`ServerPlayer.java:387`, seeded at the restore) or is the fresh one
        // its constructor built (`:265`); a profile that left this run has no
        // file write yet, so the value its departure froze is the one the
        // session continues from.
        let warden_tracker = departed_patch
            .as_ref()
            .and_then(|snapshot| snapshot.patch.warden_spawn_tracker.as_ref())
            .map(live_warden_spawn_tracker)
            .or_else(|| {
                self.cold
                    .restored_warden_trackers
                    .get(&profile.uuid)
                    .copied()
            })
            .unwrap_or_default();
        // `PrepareSpawnTask.start` (`PrepareSpawnTask.java:55-63`) uses a
        // valid saved pose directly and otherwise completes PlayerSpawnFinder
        // before the session enters play.
        let restored_dimension = restored
            .as_ref()
            .and_then(|restored| {
                restored
                    .dimension
                    .as_ref()
                    .and_then(|key| self.level_id(key))
            })
            .unwrap_or(self.cold.default_spawn.dimension);
        let restored_pose = restored
            .as_ref()
            .filter(|restored| {
                self.is_in_world_bounds_by_key(
                    &restored_dimension,
                    block_position(restored.position),
                )
            })
            .map(|restored| (restored.position, restored.rotation, restored.on_ground));
        let default_spawn = if restored_pose.is_none() {
            match self.find_default_spawn() {
                Ok(spawn) => spawn,
                Err(error) => {
                    events.push(WorldEvent::Close {
                        connection_id,
                        reason: WorldCloseReason::ChunkGenerationFailed {
                            detail: error.to_string(),
                        },
                    });
                    return;
                }
            }
        } else {
            self.cold.default_spawn.clone()
        };
        let dimension = if restored_pose.is_some() {
            restored_dimension
        } else {
            default_spawn.dimension
        };
        let (position, rotation, on_ground) =
            restored_pose.unwrap_or((default_spawn.position, default_spawn.rotation, false));
        if let Err(error) = self.install_full_chunks_by_key(&dimension, &[player_chunk(position)]) {
            events.push(WorldEvent::Close {
                connection_id,
                reason: WorldCloseReason::ChunkGenerationFailed {
                    detail: error.to_string(),
                },
            });
            return;
        }
        // `PlayerList.getPlayerStats` (`PlayerList.java:757-763`) is lazy per
        // profile and reads the file each time it has to build one; this world
        // read every `players/stats/<uuid>.json` the save held when it opened
        // and keeps that map as the stand-in for the files. A session that
        // departed in this run instead carries the newer counter inside its
        // full frozen patch.
        let baseline_stats = self.cold.restored_player_stats.get(&profile.uuid).cloned();
        let restored_stats = departed_patch
            .as_ref()
            .map(|snapshot| snapshot.patch.stats.clone())
            .or(baseline_stats);
        let restored_advancements = self
            .has_advancement_manager()
            .then(|| {
                departed_patch
                    .as_ref()
                    .map(|snapshot| snapshot.advancements.clone())
                    .or_else(|| {
                        self.cold
                            .restored_player_advancements
                            .get(&profile.uuid)
                            .cloned()
                    })
            })
            .flatten();
        // `ServerPlayer.calculateGameModeForNewPlayer` (`ServerPlayer.java:2029-2036`)
        // over the save's `playerGameType` -- read whatever the position did,
        // since vanilla reads the two out of the same document and only the
        // position is bounded.
        let game_type = self.calculate_game_mode_for_new_player(
            restored.as_ref().and_then(|restored| restored.game_type),
        );
        let abilities = restored
            .as_ref()
            .map(|restored| abilities_after_game_type(restored.abilities, game_type))
            .unwrap_or_else(|| PlayerAbilities::for_game_type(game_type));
        // A session with no save data carries the inventory `Player`'s
        // constructor builds (Player.java:174-180), whose 36 slots are
        // `ItemStack.EMPTY` (Inventory.java:55).
        let mut inventory = restored
            .as_ref()
            .map(|restored| restored.inventory.clone())
            .unwrap_or_else(PlayerInventoryMenu::empty);
        inventory.set_limited_crafting(self.login_rule_flags().do_limited_crafting);
        // `PlayerList.placeNewPlayer` (PlayerList.java:184-190) attaches the
        // server `RecipeManager` and sends the book after abilities. Install
        // after the restored book so `rematch_crafting_grid` sees known recipes.
        if let Err(error) = Self::install_join_crafting_catalog(
            &self.item_stack_resolver(),
            Some(&mut self.maps),
            &mut inventory,
            self.cold.config.datapacks.as_ref(),
            self.cold.crafting_item_tags.as_ref(),
        ) {
            events.push(WorldEvent::Close {
                connection_id,
                reason: WorldCloseReason::Administrative(RichTextComponent::plain(format!(
                    "Inventory crafting catalog admission failed: {error}"
                ))),
            });
            return;
        }
        inventory.suspend_menu_synchronizer();
        let initial_selected_slot = inventory.selected_slot();
        let initial_main_hand = inventory.selected_stack().clone();
        self.cold.restored_warden_trackers.remove(&profile.uuid);
        let mut body = crate::entity::Entity::new(
            entity_id,
            profile.uuid,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            position,
            crate::entity::EntityData::Player,
        );
        body.set_rot(rotation.y_rot, rotation.x_rot);
        body.set_on_ground(on_ground);
        self.sessions.insert(
            connection_id,
            PlayerSession {
                chunk_sender: PlayerChunkSender::new(),
                // `updatePlayerStatus(player, true)` sets `EMPTY` and then
                // `updatePlayerPos` records where the player actually is, both
                // before the first `updateChunkTracking`.
                chunk_tracking_view: None,
            },
        );
        self.levels.insert_player(PlayerState {
            connection_id,
            body,
            has_visual_fire: restored.as_ref().is_some_and(|p| p.has_visual_fire),
            environment_first_tick: true,
            was_touching_water: false,
            block_contact: crate::world::player_environment::contact::PlayerBlockContact::new(
                position,
                restored.as_ref().map_or(0.0, |p| p.fall_distance),
            ),
            silent: restored.as_ref().is_some_and(|p| p.silent),
            silent_dirty: restored.as_ref().is_some_and(|p| p.silent),
            remaining_fire_ticks: restored.as_ref().map_or(0, |p| p.remaining_fire_ticks),
            air_supply: restored
                .as_ref()
                .map_or(crate::entity::living::MAX_AIR_SUPPLY, |p| p.air_supply),
            air_supply_dirty: restored
                .as_ref()
                .is_some_and(|p| p.air_supply != crate::entity::living::MAX_AIR_SUPPLY),
            ticks_frozen: restored.as_ref().map_or(0, |p| p.ticks_frozen),
            ticks_frozen_dirty: restored.as_ref().is_some_and(|p| p.ticks_frozen != 0),
            fluid_interaction: Default::default(),
            body_tick_enabled: true,
            profile,
            dimension,
            avatar_removed: false,
            // `ServerGamePacketListenerImpl`'s constructor opens the
            // client-load window (:280) before the join packets go out, so
            // a session is not loaded until it says so or the timer runs
            // out.
            is_changing_dimension: false,
            waiting_for_respawn: false,
            client_loaded_timeout_timer: CLIENT_LOADED_TIMEOUT_TICKS,
            awaiting_teleport: Some(teleport_id),
            awaiting_teleport_tick: self.tick,
            game_type,
            abilities,
            // `LivingEntity.readAdditionalSaveData` restores shared flag
            // 7 before the player is published (`LivingEntity.java:803`), and
            // `setGlowingTag` flag 6 (`Entity.java:2085`).
            shared_flags: restored.as_ref().map_or(0, |restored| {
                use bbb_protocol::entity_synched_data::{
                    VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING as FALL_FLYING,
                    VANILLA_ENTITY_SHARED_FLAG_GLOWING as GLOWING,
                };
                (if restored.fall_flying { FALL_FLYING } else { 0 })
                    | (if restored.glowing { GLOWING } else { 0 })
            }),
            shared_flags_dirty: restored
                .as_ref()
                .is_some_and(|restored| restored.fall_flying || restored.glowing),
            pose_id: bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID,
            pose_dirty: false,
            boarding_cooldown: 0,
            first_good: Vec3d::default(),
            last_good: Vec3d::default(),
            received_move_packets: 0,
            known_move_packets: 0,
            movement: Default::default(),
            last_client_input: PlayerInput::default(),
            inventory,
            // `Player`'s own fields start at zero and at whatever the
            // freshly built inventory is holding.
            attack_strength_ticker: 0,
            speed: 0.0,
            item_swap_ticker: 0,
            last_item_in_main_hand: initial_main_hand,
            // `LivingEntity.readAdditionalSaveData` (LivingEntity.java:778,
            // :791-793) over the saved document, and the constructor's own
            // full health (`:275`) for a profile the save has never held.
            damage: restored
                .as_ref()
                .map_or_else(PlayerDamageState::created, |restored| restored.damage),
            // `LivingEntity.<init>` (`LivingEntity.java:274`) with the
            // packed instances `readAdditionalSaveData` applies over it
            // (`:779-781`).
            attributes: match restored.as_ref() {
                Some(restored) => crate::world::player_effects::restored_player_attribute_map(
                    &restored.attributes,
                    restored.abilities.walking_speed,
                ),
                None => crate::world::player_effects::player_attribute_map(&[]),
            },
            // `readAdditionalSaveData` (`LivingEntity.java:783-789`), and
            // `effectsDirty`'s initializer (`:240`).
            effects: {
                let mut effects = restored
                    .as_ref()
                    .map(|restored| restored.effects.clone())
                    .unwrap_or_default();
                effects.mark_dirty();
                effects
            },
            effect_particles: Default::default(),
            tick_count: 0,
            entity_invulnerable: restored
                .as_ref()
                .is_some_and(|restored| restored.entity_invulnerable),
            // `LivingEntity.swinging` and `swingTime`
            // (LivingEntity.java:1993-2008 reads both, `updateSwingTime`
            // drives them): a fresh entity is not swinging.
            swinging: false,
            swing_time: 0,
            swinging_arm: bbb_protocol::packets::InteractionHand::MainHand,
            known_movement: Vec3d::default(),
            needs_sync: false,
            hurt_marked: false,
            received_movement_this_tick: false,
            // `FoodData.readAdditionalSaveData` (`FoodData.java:74-79`)
            // over the saved document, and the four field initializers
            // (`:14-17`) for a profile the save has never held.
            food: restored
                .as_ref()
                .map_or_else(FoodData::created, |restored| restored.food),
            last_sent_health: UNSENT_HEALTH,
            last_sent_food: UNSENT_FOOD,
            last_food_saturation_zero: true,
            // `Player.readAdditionalSaveData` (`Player.java:629-631`)
            // over the saved document, and the three field initialisers
            // (`:160-162`) for a profile the save has never held.
            experience: restored
                .as_ref()
                .map_or_else(ExperienceState::created, |restored| restored.experience),
            last_sent_exp: UNSENT_EXPERIENCE,
            last_sent_score: 0,
            take_xp_delay: 0,
            using_item: crate::world::using_item::UsingItemState::default(),
            cold: Box::new(super::super::PlayerCold {
                shoulders: restored
                    .as_ref()
                    .map_or_else(Default::default, |player| player.shoulders.clone()),
                chat_spam: 0,
                next_chat_index: 0,
                portal_cooldown: restored.as_ref().map_or(0, |p| p.portal_cooldown),
                portal_process: None,
                // `ServerPlayer.java:392-394` wakes a player saved asleep, which
                // resets the counter (`Player.stopSleepInBed`).
                sleep_timer: restored
                    .as_ref()
                    .filter(|saved| saved.sleeping_pos.is_none())
                    .map_or(0, |saved| saved.sleep_timer),
                sleeping_pos: None,
                sleeping_pos_dirty: false,
                arrows: Default::default(),
                arrow_count_dirty: false,
                stingers: Default::default(),
                stinger_count_dirty: false,
                spawn_extra_particles_on_fall: restored
                    .as_ref()
                    .is_some_and(|p| p.spawn_extra_particles_on_fall),
                seen_credits: restored.as_ref().is_some_and(|p| p.seen_credits),
                won_game: false,
                fall_impulse: restored
                    .as_ref()
                    .map_or_else(Default::default, |p| p.fall_impulse),
                impulse_explosion_cause: None,
                glowing_tag: restored.as_ref().is_some_and(|p| p.glowing),
                custom_name: restored.as_ref().and_then(|p| p.custom_name.clone()),
                custom_data: restored
                    .as_ref()
                    .map_or_else(Vec::new, |p| p.custom_data.clone()),
                drop_spam_throttler: crate::world::player_drop::DropSpamThrottler::default(),
                entered_nether_position: restored
                    .as_ref()
                    .and_then(|restored| restored.entered_nether_position),
                raid_omen_position: restored
                    .as_ref()
                    .and_then(|restored| restored.raid_omen_position),
                last_death_location: restored
                    .as_ref()
                    .and_then(|restored| restored.last_death_location.clone()),
                skip_drop_experience: false,
                combat: Default::default(),
                levitation_start_position: None,
                entered_lava_on_vehicle_position: None,
                levitation_start_tick: 0,
                fall_fly_ticks: 0,
                respawn: restored
                    .as_ref()
                    .and_then(|restored| restored.respawn.clone()),
                pending_block_ack: None,
                pending_keep_alive: None,
                keep_alive_sent_millis: crate::world::idle_timeout::monotonic_millis(),
                latency: 0,
                last_action_millis: crate::world::idle_timeout::monotonic_millis(),
                remote_address: context.remote_address,
                main_hand_dirty: client_information.main_hand
                    != bbb_protocol::packets::ClientMainHand::Right,
                model_customization_dirty: client_information.displayed_skin_parts != 0,
                client_information,
                previous_game_type: restored
                    .as_ref()
                    .and_then(|restored| restored.previous_game_type),
                last_section_pos: {
                    let pos = block_position(position);
                    (pos.x >> 4, pos.y >> 4, pos.z >> 4)
                },
                filed_ticket_ignored: false,
                last_seen_tracked: LAST_SEEN_MESSAGES_MAX_LENGTH,
                requested_debug_subscriptions: Default::default(),
                last_vehicle: None,
                vehicle_first_good: Vec3d::default(),
                vehicle_last_good: Vec3d::default(),
                client_vehicle_is_floating: false,
                above_ground_vehicle_tick_count: 0,
                recipe_book_settings: restored
                    .as_ref()
                    .map(|restored| restored.recipe_book_settings)
                    .unwrap_or_default(),
                // `PlayerList.getPlayerAdvancements` (`PlayerList.java:788-799`)
                // reads `players/advancements/<uuid>.json` the first time a
                // session asks for it. A profile that departed in this run
                // carries the newer store on its frozen snapshot instead.
                advancements: match restored_advancements {
                    Some(data) => PlayerAdvancements::load(&self.cold.advancement_tree, &data),
                    None => PlayerAdvancements::created(&self.cold.advancement_tree),
                },
                advancement_update_refusal: None,
                pending_book_edits: Vec::new(),
                destroy: DestroyProgress::default(),
                locator_bar_icon: restored
                    .as_ref()
                    .map_or_else(crate::world::waypoints::default_waypoint_icon, |restored| {
                        restored.locator_bar_icon.clone()
                    }),
                last_hurt_by_player: restored
                    .as_ref()
                    .and_then(|saved| saved.last_hurt_by.player)
                    .map(|(uuid, _)| uuid),
                last_hurt_by_player_memory_time: restored
                    .as_ref()
                    .and_then(|saved| saved.last_hurt_by.player)
                    .map_or(0, |(_, ticks)| ticks),
                last_hurt_by_mob: restored
                    .as_ref()
                    .and_then(|saved| saved.last_hurt_by.mob)
                    .map(|(uuid, _)| uuid),
                last_hurt_by_mob_timestamp: restored
                    .as_ref()
                    .and_then(|saved| saved.last_hurt_by.mob)
                    .map_or(0, |(_, ticks)| ticks),
                last_hurt_mob: None,
                last_hurt_mob_timestamp: 0,
                // `PlayerList.getPlayerStats` (`PlayerList.java:757-763`) reads
                // `players/stats/<uuid>.json` the first time a session asks for
                // it, and `placeNewPlayer` (`:189`) then marks every counter it
                // found dirty so the first `sendStats` publishes the lot.
                stats: {
                    let mut stats = restored_stats.unwrap_or_default();
                    stats.mark_all_dirty();
                    stats
                },
                warden_tracker,
                last_level_up_time: 0,
                recorded_criteria: Default::default(),
                // `Entity.load` (Entity.java:2090) adds the document's `Tags`
                // to a set that starts empty, so a profile the save has never
                // held joins with none.
                tags: restored
                    .as_ref()
                    .map(|restored| restored.tags.clone())
                    .unwrap_or_default(),
                // `Entity.camera` starts as the entity itself
                // (`Entity.getCamera`), which is what `None` means here.
                camera: None,
                cooldowns: crate::item_cooldowns::ItemCooldowns::default(),
                last_equipment_items: std::collections::BTreeMap::new(),
                location_effects: Default::default(),
            }),
        });
        // Only a published PlayerState has taken ownership. Any earlier spawn
        // failure leaves this latest snapshot available for a retry.
        let joined_uuid = self.levels[connection_id].profile.uuid;
        self.cold.departed_player_snapshots.remove(&joined_uuid);
        self.cold.restored_players.remove(&joined_uuid);
        self.cold.restored_player_stats.remove(&joined_uuid);
        self.cold.restored_player_advancements.remove(&joined_uuid);
        self.append_level_player(&dimension, connection_id);
        self.update_sleeping_player_list(&dimension, events);

        let rules = self.login_rule_flags();
        let login = PlayLogin {
            player_id: entity_id,
            hardcore: self.cold.hardcore,
            // `this.server.levelKeys()` (`PlayerList.java:170`), which is
            // `MinecraftServer.levels.keySet()` (`MinecraftServer.java:1189-1191`):
            // every level this server runs, not just the one the session
            // starts in, because the client has to know the set before it can
            // be told to move between them.
            levels: self
                .level_ids()
                .into_iter()
                .map(|dimension| self.level_key(dimension).resource_id().to_string())
                .collect(),
            max_players: self.admin.settings().max_players,
            chunk_radius: self.admin.settings().view_distance,
            simulation_distance: self.admin.settings().simulation_distance,
            reduced_debug_info: rules.reduced_debug_info,
            show_death_screen: rules.show_death_screen,
            do_limited_crafting: rules.do_limited_crafting,
            common_spawn_info: self.common_spawn_info(
                &dimension,
                game_type,
                self.levels[connection_id].cold.previous_game_type,
                self.levels[connection_id].cold.portal_cooldown,
                self.levels[connection_id].cold.last_death_location.clone(),
            ),
            enforces_secure_chat: false,
        };
        events.push(send(connection_id, ClientboundPlayPacket::Login(login)));
        // `PlayerList.placeNewPlayer` (PlayerList.java:181-183) sends the world
        // difficulty and the player's abilities immediately after the login,
        // before the held slot: the client needs both before it can present a
        // playable session.
        events.push(send(
            connection_id,
            ClientboundPlayPacket::ChangeDifficulty(ChangeDifficulty {
                difficulty: self.admin.settings().difficulty,
                locked: self.cold.difficulty_locked,
            }),
        ));
        events.push(send(
            connection_id,
            ClientboundPlayPacket::PlayerAbilities(self.levels[connection_id].abilities),
        ));
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SetHeldSlot(SetHeldSlot {
                slot: i32::from(initial_selected_slot),
            }),
        ));
        if let Some(startup) = self.cold.config.datapacks.as_deref() {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::UpdateRecipes(
                    startup.recipe_displays.update_recipes().clone(),
                ),
            ));
        }
        // `PlayerList.sendPlayerPermissionLevel` (PlayerList.java:188), after
        // the selected slot and client recipe catalog.
        self.send_player_permission_level(connection_id, events);
        self.send_initial_recipe_book(connection_id, events);
        // `PlayerList.updateEntireScoreboard` (PlayerList.java:191, `:220-237`),
        // which vanilla sends here rather than with the level info below.
        for packet in self.scoreboard.join_packets() {
            events.push(send(connection_id, packet));
        }
        self.send_level_info(connection_id, events);
        // `PlayerList.placeNewPlayer` (PlayerList.java:193-200) announces the
        // arrival just before the spawn teleport, and to the player list as it
        // stands — the joiner is added to it further down (:208), so the one
        // client that never hears about a join is the one joining.
        let joined = SystemChat::from_component(
            commands::player_joined_announcement(
                &self.command_player_view(connection_id),
                self.scoreboard(),
            ),
            false,
        );
        for &other in self.levels.player_keys() {
            if other != connection_id {
                self.send_system_message(other, joined.clone(), events);
            }
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::PlayerPosition(PlayerPositionUpdate {
                id: teleport_id,
                position,
                delta_movement: Vec3d::default(),
                y_rot: rotation.y_rot,
                x_rot: rotation.x_rot,
                relatives_mask: 0,
            }),
        ));
        // `PlayerList.java:202-205`: the status's MOTD; this server has no icon.
        if !context.transferred {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::ServerData(bbb_protocol::packets::ServerData {
                    motd: bbb_protocol::RichTextComponent::plain(
                        self.admin.settings().motd.clone(),
                    ),
                    icon_bytes: None,
                }),
            ));
        }
        // `ChunkMap.updatePlayerStatus(player, true)` (ChunkMap.java:1037-1048)
        // enables the chunk view before the later entity-tracker viewer sweep.
        self.update_chunk_tracking(connection_id, events);
        self.waypoint_add_player(connection_id, events);
        // Player visibility (`PlayerList.placeNewPlayer`, PlayerList.java:
        // 207-212): the initializing PlayerInfoUpdate strictly precedes every
        // AddEntity pairing or clients drop the spawn
        // (`ClientPacketListener.java:592-599`). Vanilla wraps pairings in a
        // bundle.
        let existing: Vec<ConnectionId> = self
            .levels
            .player_keys()
            .copied()
            .filter(|&other| other != connection_id)
            .collect();
        if !existing.is_empty() {
            // To the new player only: every existing player (PlayerList.java:207).
            // The list is `PlayerList.players`, which a death does not leave —
            // only the respawn does (`PlayerList.java:387`) — so a session
            // waiting on the death screen is still announced.
            let entries = existing
                .iter()
                .map(|&other| initializing_player_info_entry(self.levels.player_at(other)))
                .collect();
            events.push(send(
                connection_id,
                ClientboundPlayPacket::PlayerInfoUpdate(initializing_player_info(entries)),
            ));
        }
        // To everyone, including the new player (PlayerList.java:210).
        let announcement = ClientboundPlayPacket::PlayerInfoUpdate(initializing_player_info(vec![
            initializing_player_info_entry(self.levels.player_at(connection_id)),
        ]));
        for &other in self.levels.player_keys() {
            events.push(send(other, announcement.clone()));
        }
        // Pairing failures leave the session unpaired rather than refusing the join.
        let _ = self.add_player_entity_tracking_checked(&dimension, connection_id, events);
        let _ = self.move_session_entity_tracking(connection_id, events);
        // `PlayerList.java:213` connects persistent custom boss-bar members.
        let player_id = self.levels[connection_id].profile.uuid;
        for packet in self
            .cold
            .boss_events
            .on_player_connect(connection_id, player_id)
        {
            events.push(send(connection_id, packet));
        }
        // `PlayerList.java:214`: whatever the document restored is announced
        // to the arriving session, since nothing called `onEffectAdded` for it.
        self.send_active_player_effects(connection_id, events);
        // PlayerList.java:213-215 registers inventory listeners after player publication.
        self.initialize_player_menu(connection_id, events);
        // `PrepareSpawnTask.java:184-187` publishes the Player through
        // `placeNewPlayer` before loading and attaching its saved parent tree.
        if let Some(root_vehicle) = restored
            .as_ref()
            .and_then(|restored| restored.root_vehicle.as_ref())
        {
            let input = self.entity_nbt_read_snapshot();
            let restored = read_external_player_root_vehicle_input_with_context(
                &mut self.level_mut_by_id(&dimension).entities,
                root_vehicle.entity(),
                root_vehicle.attach_input(),
                entity_id,
                input.context(),
            );
            if let Ok(restored_root) = restored {
                if let Some(root_id) = restored_root {
                    self.resolve_raider_raids_in_tree(&dimension, root_id);
                }
                self.apply_level_entity_lifecycle_by_key(&dimension, events);
            }
        }
        self.restore_player_ender_pearls(
            player_id,
            restored
                .as_ref()
                .map_or(&[], |restored| restored.ender_pearls.as_slice()),
            events,
        );
    }

    /// `PlayerList.sendPlayerPermissionLevel` (PlayerList.java:463-466 into
    /// `:559-572`): the session is told its own level, and only its own, and
    /// the announcement closes with `sendCommands` (`:571`) so the tree that
    /// level may use arrives with it.
    pub(in crate::world) fn send_player_permission_level(
        &self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let permission = self.player_permission(connection_id);
        events.push(send(
            connection_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: self.levels[connection_id].entity_id(),
                event_id: permission.entity_event_id(),
            }),
        ));
        events.push(send(
            connection_id,
            ClientboundPlayPacket::Commands(commands::tree::usable_command_tree(permission)),
        ));
    }

    /// The dynamic registries the configuration phase publishes for this world.
    pub fn synchronized_registries(&self) -> &SynchronizedRegistries {
        &self.registries
    }

    /// The wire index of one level's dimension type (`ServerPlayer.java:2151-2164`).
    pub(in crate::world) fn dimension_type_id(&self, dimension: &LevelId) -> i32 {
        let element_id = self
            .cold
            .level_dimension_types
            .get(dimension)
            .expect("every level this world runs declared a dimension type");
        self.registries
            .dimension_type_index(element_id)
            .expect("every level's dimension type was published when the world was built")
    }

    /// `ServerPlayer.createCommonSpawnInfo` (ServerPlayer.java:2151-2164), which
    /// the play login and every `ClientboundRespawnPacket` carry.
    pub(in crate::world) fn common_spawn_info(
        &self,
        dimension: &LevelId,
        game_type: GameType,
        previous_game_type: Option<GameType>,
        portal_cooldown: i32,
        last_death_location: Option<bbb_protocol::packets::GlobalPos>,
    ) -> CommonPlayerSpawnInfo {
        let overworld_is_flat = self
            .level(Java26BuiltinDimension::Overworld)
            .chunks
            .flat_generator()
            .is_some();
        CommonPlayerSpawnInfo {
            dimension_type_id: self.dimension_type_id(dimension),
            dimension: self.level_key(*dimension).resource_id().to_string(),
            seed: bbb_server_worldgen::noise::biome::biome_zoom_seed(self.cold.seed),
            game_type: i8::try_from(game_type.id())
                .expect("Vanilla game type ids fit the protocol byte"),
            previous_game_type: i8::try_from(
                previous_game_type.map_or(VANILLA_GAME_TYPE_NOT_SET_ID, GameType::id),
            )
            .expect("Vanilla nullable game type ids fit the protocol byte"),
            is_debug: self.is_debug(),
            is_flat: overworld_is_flat,
            last_death_location,
            portal_cooldown,
            sea_level: self.level_by_id(dimension).chunks.sea_level(),
        }
    }

    pub(crate) fn install_join_crafting_catalog(
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        maps: Option<&mut crate::world::maps::ServerMapStore>,
        inventory: &mut PlayerInventoryMenu,
        datapacks: Option<&Arc<bbb_server_datapack::startup::DatapackStartup>>,
        tags: Option<&Arc<ItemTagIndex>>,
    ) -> bbb_server_types::Result<()> {
        let (Some(datapacks), Some(tags)) = (datapacks, tags) else {
            return Ok(());
        };
        inventory
            .recipe_book_mut()
            .retain_known(|id| datapacks.recipes.recipe(id).is_some());
        inventory.install_crafting_publication(
            resolver,
            maps,
            Arc::clone(datapacks),
            Arc::clone(tags),
        )
    }

    pub(crate) fn reload_player_crafting_catalog(
        inventory: &mut PlayerInventoryMenu,
        datapacks: &Arc<bbb_server_datapack::startup::DatapackStartup>,
        tags: Option<&Arc<ItemTagIndex>>,
    ) {
        let Some(tags) = tags else {
            return;
        };
        inventory
            .recipe_book_mut()
            .retain_known(|id| datapacks.recipes.recipe(id).is_some());
        inventory.reload_crafting_publication(Arc::clone(datapacks), Arc::clone(tags));
    }

    /// `ServerRecipeBook.sendInitialRecipeBook` (`ServerRecipeBook.java:112-120`).
    pub(in crate::world) fn send_initial_recipe_book(
        &self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(startup) = self.cold.config.datapacks.as_deref() else {
            return;
        };
        let player = self.levels.player_at(connection_id);
        let settings = player.cold.recipe_book_settings;
        let pair = |index: usize| WireRecipeBookTypeSettings {
            open: settings[index].open,
            filtering: settings[index].filtering,
        };
        events.push(send(
            connection_id,
            ClientboundPlayPacket::RecipeBookSettings(RecipeBookSettings {
                crafting: pair(0),
                furnace: pair(1),
                blast_furnace: pair(2),
                smoker: pair(3),
            }),
        ));

        let book = player.inventory.recipe_book();
        let highlighted = book
            .to_be_displayed()
            .collect::<std::collections::BTreeSet<_>>();
        let entries = book
            .known()
            .flat_map(|recipe_id| {
                startup
                    .recipe_displays
                    .displays_for_recipe(recipe_id)
                    .iter()
                    .cloned()
                    .map(|display| {
                        RecipeBookAddEntry::new(display, false, highlighted.contains(recipe_id))
                    })
            })
            .collect();
        events.push(send(
            connection_id,
            ClientboundPlayPacket::RecipeBookAdd(RecipeBookAdd {
                entries,
                replace: true,
            }),
        ));
    }

    /// `Level.getEntityOrPart`'s player arm.
    pub(in crate::world) fn connection_by_entity_id(&self, entity_id: i32) -> Option<ConnectionId> {
        self.levels
            .player_entries()
            .find(|(_, player)| player.entity_id() == entity_id && !player.avatar_removed)
            .map(|(connection_id, _)| *connection_id)
    }

    /// Removes a player and tells every survivor to drop the tab-list entry
    /// (`PlayerList.remove`, PlayerList.java:331) and the entity
    /// (`ServerEntity.removePairing`, ServerEntity.java:257-260).
    pub(in crate::world) fn remove_player(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        self.remove_player_inner(connection_id, false, events);
    }

    /// Snapshots and cleans up an online player during graceful server shutdown
    /// without emitting leave chat or tracking broadcasts to closing clients.
    pub(in crate::world) fn remove_player_for_shutdown(&mut self, connection_id: ConnectionId) {
        let mut discarded = Vec::new();
        self.remove_player_inner(connection_id, true, &mut discarded);
    }

    fn remove_player_inner(
        &mut self,
        connection_id: ConnectionId,
        shutting_down: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.levels.contains_player(connection_id) {
            return;
        }
        if self.levels[connection_id].cold.sleeping_pos.is_some() {
            self.stop_player_sleep(connection_id, true, false, events);
        }
        let mut departed_patch = match self.player_save_patch(
            self.levels
                .player(connection_id)
                .expect("presence was just checked"),
        ) {
            Ok(patch) => Some(patch),
            Err(error) => {
                if shutting_down {
                    tracing::warn!(
                        event = "world.shutdown_player_snapshot_failed",
                        connection_id = connection_id.0,
                        %error,
                    );
                    None
                } else {
                    events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                        format!(
                            "player removal deferred for connection {}: {error}",
                            connection_id.0
                        ),
                    )));
                    return;
                }
            }
        };
        let uuid = self.levels[connection_id].profile.uuid;
        // PlayerList.java:301-302.
        self.levels[connection_id]
            .award_stat_once(&mut self.scoreboard, custom_stat(CUSTOM_STAT_LEAVE_GAME));
        self.flush_scoreboard_effects(events);
        if let Some(patch) = &mut departed_patch {
            patch.stats = self.levels[connection_id].cold.stats.clone();
        }
        self.unload_player_ender_pearls(uuid, events);
        self.maps.remove_player(connection_id);
        if !shutting_down {
            let left = SystemChat::from_component(
                commands::player_left_announcement(
                    &self.command_player_view(connection_id),
                    self.scoreboard(),
                ),
                false,
            );
            for &other in self.levels.player_keys() {
                if other != connection_id {
                    self.send_system_message(other, left.clone(), events);
                }
            }
        }
        let dimension = self.player_level_id(connection_id);
        let entity_id = self.levels[connection_id].entity_id();
        self.apply_chunk_tracking_view(connection_id, None, &mut Vec::new());
        self.remove_player_entity_tracking(&dimension, connection_id, entity_id, false, events);
        let mut waypoint_removals = Vec::new();
        self.waypoint_remove_player(connection_id, &mut waypoint_removals);
        waypoint_removals.retain(|event| {
            !matches!(
                event,
                WorldEvent::Send {
                    connection_id: recipient,
                    ..
                } if *recipient == connection_id
            )
        });
        self.drop_removed_player_menus(connection_id, events);
        self.remove_level_player(&dimension, connection_id);
        self.update_sleeping_player_list(&dimension, events);
        self.unload_external_player_ride(&dimension, connection_id, events);
        let player = self
            .levels
            .remove_player(connection_id)
            .expect("presence was just checked");
        self.sessions.remove(&connection_id);
        let entity_id = player.entity_id();
        let advancements = player.cold.advancements;
        self.cold
            .boss_events
            .on_player_disconnect(connection_id, uuid);
        for level in self.levels.values_mut() {
            if let Some(fight) = &mut level.dragon_fight {
                fight.forget_player(connection_id);
            }
        }
        if let Some(departed_patch) = departed_patch {
            self.cold.departed_player_snapshots.insert(
                uuid,
                DepartedPlayerSnapshot {
                    patch: departed_patch,
                    advancements: advancements.to_data(&self.cold.advancement_tree),
                },
            );
        }
        let entity_drops = self.game_rule_bool("minecraft:entity_drops", true);
        for level_dimension in self.level_ids() {
            let dropped = crate::entity::leash::drop_leashes_held_by_departed_player(
                &mut self.level_mut_by_id(&level_dimension).entities,
                entity_id,
                entity_drops,
            );
            if level_dimension == dimension || dropped {
                self.apply_level_entity_lifecycle_by_key(&level_dimension, events);
            }
        }
        if !shutting_down {
            let info_remove = ClientboundPlayPacket::PlayerInfoRemove(PlayerInfoRemove {
                profile_ids: vec![uuid],
            });
            events.extend(waypoint_removals);
            for &other in self.levels.player_keys() {
                events.push(send(other, info_remove.clone()));
            }
        }
    }
}
