//! `PlayerList.respawn`.

use super::*;

impl ServerWorld {
    /// `PlayerList.respawn`; `keep_all` is its `keepAllPlayerData`, the
    /// won-game respawn out of the End credits.
    pub(super) fn perform_respawn_checked(
        &mut self,
        connection_id: ConnectionId,
        keep_all: bool,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player_at(connection_id);
        if !keep_all && !player.damage.is_dead_or_dying() {
            // `:1872-1874`: a living player asking to respawn is ignored.
            return None;
        }
        if !keep_all {
            // The death's experience is settled before the reset below.
            let dimension = player.dimension;
            self.post_pending_game_events(&dimension);
            if !self.levels[connection_id].avatar_removed {
                // `PlayerList.java:388` removes a body `tickDeath` has not removed yet.
                self.drop_removed_player_menus(connection_id, events);
            }
        }
        let player = self.levels.player_at(connection_id);
        let entity_id = player.entity_id();
        let game_type = player.game_type;
        let player_respawn = player.cold.respawn.clone();
        let player_dimension = player.dimension;
        // `ServerPlayer.java:1605`: a spectator's death keeps them too.
        let keep_inventory = game_type == GameType::Spectator
            || matches!(
                self.admin.game_rule_value(RULE_KEEP_INVENTORY),
                Some(GameRuleValue::Boolean(true))
            );
        let inventory = player.inventory.respawned(keep_all || keep_inventory);
        let (spawn, missing_respawn_block) =
            match self.resolve_sleep_respawn(player_respawn.as_ref(), !keep_all) {
                Ok(spawn) => spawn,
                Err(error) => {
                    events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                        format!(
                            "player respawn deferred for connection {}: {error}",
                            connection_id.0
                        ),
                    )));
                    return None;
                }
            };
        let crate::world::WorldSpawn {
            position,
            rotation,
            dimension: dest_dimension,
        } = spawn;
        if !self.has_level_id(&dest_dimension) {
            return None;
        }
        if let Err(error) =
            self.install_full_chunks_by_key(&dest_dimension, &[player_chunk(position)])
        {
            events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                format!(
                    "player respawn deferred for connection {}: {error}",
                    connection_id.0
                ),
            )));
            return None;
        }
        // PlayerList replaces the ServerPlayer object (PlayerList.java:390-394).
        // `PlayerList.java:409`: the death respawn keeps nothing; the won-game
        // one keeps the attributes.
        let data_to_keep: i8 = if keep_all { 1 } else { 0 };
        // `:388` removes a body the death tick has not yet removed with `KILLED`.
        if !keep_all && !self.levels[connection_id].avatar_removed {
            self.trigger_player_on_death_effects(connection_id, events);
        }
        self.maps.remove_player(connection_id);
        self.waypoint_remove_player(connection_id, events);

        // `:387-388`. The tracking view goes to `ChunkTrackingView.EMPTY`
        // (ChunkMap.java:1056), which tells the client to forget every column
        // it was holding, and the avatar is unpaired from every tracker.
        self.apply_chunk_tracking_view(connection_id, None, events);
        self.remove_player_entity_tracking(
            &player_dimension,
            connection_id,
            entity_id,
            true,
            events,
        );
        self.remove_level_player(&player_dimension, connection_id);
        self.update_sleeping_player_list(&player_dimension, events);

        let now = self.tick;
        let teleport_id = self.next_teleport_id;
        self.next_teleport_id = self.next_teleport_id.wrapping_add(1);
        self.levels.move_player(connection_id, dest_dimension);
        {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the respawning player is still connected");
            // `:390-393`: a new `ServerPlayer` under the old id, so every
            // counter is the one a fresh `LivingEntity` holds and
            // `restoreFrom` (`:1604`) puts it at full health.
            player.inventory = inventory;
            player.cold.camera = None;
            // `PlayerList.java:395-397`.
            if missing_respawn_block {
                player.cold.respawn = None;
            }
            let health = player.damage.health;
            player.damage = PlayerDamageState::created();
            player.cold.portal_cooldown = 0;
            // `restoreFrom`'s `restoreAll` arm (ServerPlayer.java:1592-1602)
            // keeps health, food, effects and the portal process.
            if keep_all {
                player.damage.health = health;
                player.restart_absorption_effect();
            } else {
                player.cold.portal_process = None;
                // `restoreFrom(oldPlayer, false)` does not copy `foodData`
                // (ServerPlayer.java:1592-1595 is the `restoreAll` arm), so a
                // respawn after a death starts on a fresh one; `:1614-1615` then
                // forces the next `doTick` to announce it.
                player.food = FoodData::created();
                // The new `ServerPlayer` carries no effect either: only
                // `restoreFrom`'s `restoreAll` arm copies them (`:1597-1599`), and
                // a death respawn takes the other one (`:1603-1608`). The client
                // rebuilds its own player over the respawn packet, so no session is
                // owed a removal for them.
                player.effects = Default::default();
                // `assignBaseValues` (ServerPlayer.java:1591): the new player
                // keeps only base values, so every modifier is dropped.
                let mut attributes = crate::world::player_effects::player_attribute_map(&[]);
                for old in player.attributes.instances() {
                    if let Some(fresh) = attributes.get_instance(old.attribute_id()) {
                        fresh.set_base_value(old.base_value());
                    }
                }
                player.attributes = attributes;
                // ServerPlayer.java:1604: fill health after retaining attribute bases.
                player.damage.health = player.max_health();
            }
            player.tick_count = 0;
            player.cold.last_hurt_by_mob = None;
            player.cold.last_hurt_by_mob_timestamp = 0;
            player.cold.last_hurt_mob = None;
            player.cold.last_hurt_mob_timestamp = 0;
            player.cold.arrows = Default::default();
            player.cold.arrow_count_dirty = true;
            player.cold.stingers = Default::default();
            player.cold.stinger_count_dirty = true;
            player.reset_fall_impulse();
            player.cold.spawn_extra_particles_on_fall = false;
            player.cold.location_effects.last_block_pos = None;
            player.block_contact =
                crate::world::player_environment::contact::PlayerBlockContact::new(position, 0.0);
            // The new ServerPlayer starts from Abilities defaults before
            // `restoreFrom` reapplies its game type (ServerPlayer.java:1586-1590;
            // GameType.java:62-80).
            player.abilities = PlayerAbilities::for_game_type(game_type);
            player.last_sent_health = -1.0;
            player.last_sent_food = -1;
            // `:1613`, beside the two above.
            player.last_sent_exp = -1;
            // The new client-side player starts from the define-time score.
            player.last_sent_score = 0;
            player.cold.recorded_criteria = Default::default();
            // `restoreFrom` copies the three experience fields only through
            // `transferInventoryXpAndScore` (`:1627-1633`), which a death
            // reaches only through the gate the inventory above took.
            if !keep_all && !keep_inventory {
                // `restoreFrom` always carries `enchantmentSeed` (`:1610`).
                player.experience = ExperienceState::created()
                    .with_enchantment_seed(player.experience.enchantment_seed());
            }
            // `Player.takeXpDelay` is not among the copied fields either, so a
            // respawned session may absorb an orb on its first tick.
            player.take_xp_delay = 0;
            // `Entity.invulnerable` is not among the fields `restoreFrom`
            // copies (ServerPlayer.java:1586-1625), so the new player carries
            // the `false` the field is built with.
            player.entity_invulnerable = false;
            // `lastHurtByPlayer` is not copied either, so nobody is remembered.
            player.cold.last_hurt_by_player = None;
            player.cold.last_hurt_by_player_memory_time = 0;
            player.cold.skip_drop_experience = false;
            player.cold.combat = Default::default();
            player.avatar_removed = false;
            player.reset_respawn_entity_components();
            player.fluid_interaction = Default::default();
            player.body_tick_enabled = true;
            // `:404` snaps the new player to the respawn position.
            player.set_position(position);
            // `ServerGamePacketListenerImpl.java:1868,1877`: `resetPosition()`.
            player.first_good = position;
            player.last_good = position;
            player.set_rotation(rotation);
            player.set_on_ground(false);
            player.cold.last_section_pos = {
                let pos = block_position(position);
                (pos.x >> 4, pos.y >> 4, pos.z >> 4)
            };
            player.awaiting_teleport = Some(teleport_id);
            player.awaiting_teleport_tick = now;
            // `SynchedEntityData` is new with the entity, so every flag is back
            // at its default and clean.
            player.shared_flags = 0;
            player.shared_flags_dirty = false;
            player.effect_particles = Default::default();
            player.effects.mark_dirty();
            player.pose_id = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
            player.pose_dirty = false;
            player.cold.sleep_timer = 0;
            player.cold.sleeping_pos = None;
            player.cold.sleeping_pos_dirty = false;
            // PlayerList.java:390: the new avatar receives the current options.
            player.cold.main_hand_dirty = player.cold.client_information.main_hand
                != bbb_protocol::packets::ClientMainHand::Right;
            player.cold.model_customization_dirty =
                player.cold.client_information.displayed_skin_parts != 0;
            player.using_item = Default::default();
            player.hurt_marked = false;
            player.cold.last_equipment_items.clear();
            // The new Player starts at Entity's field default; `restoreFrom`
            // does not copy the old Player's counter.
            player.boarding_cooldown = 0;
            player.cold.destroy = DestroyProgress::default();
            player.inventory.suspend_menu_synchronizer();
            // `:424` re-marks the health, which `ServerPlayer.doTick`
            // (ServerPlayer.java:671-678) turns into the first
            // `ClientboundSetHealthPacket` the respawned session gets.
            player.last_sent_health = UNSENT_HEALTH;
        }
        // The session now counts as an entity of the respawn level for
        // `getEntityInAnyDimension` (`ServerLevel.java:1384-1391`).
        self.refresh_cross_level_entities_across(&player_dimension, &dest_dimension);

        // `restoreFrom`'s `onUpdateAbilities` (ServerPlayer.java:1590,
        // `:1723-1728`), which runs before the respawn packet because the
        // whole restore does (`PlayerList.java:392` ahead of `:412`).
        events.push(send(
            connection_id,
            ClientboundPlayPacket::PlayerAbilities(self.levels[connection_id].abilities),
        ));
        if missing_respawn_block {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::GameEvent(GameEvent {
                    event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID,
                    param: 0.0,
                }),
            ));
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::Respawn(Respawn {
                common_spawn_info: self.common_spawn_info(
                    &self.player_level_id(connection_id),
                    game_type,
                    self.levels[connection_id].cold.previous_game_type,
                    self.levels[connection_id].cold.portal_cooldown,
                    self.levels[connection_id].cold.last_death_location.clone(),
                ),
                data_to_keep,
            }),
        ));
        // `:413` through `ServerGamePacketListenerImpl.teleport` (:1242-1255):
        // a fresh teleport id with no relative axes and no motion.
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
        // `:415`.
        events.push(send(
            connection_id,
            ClientboundPlayPacket::ChangeDifficulty(ChangeDifficulty {
                difficulty: self.admin.settings().difficulty,
                locked: self.cold.difficulty_locked,
            }),
        ));
        // `:417`.
        self.send_active_player_effects(connection_id, events);
        // `:418` and `:419`.
        self.send_level_info(connection_id, events);
        self.send_player_permission_level(connection_id, events);
        // `:420` through `ChunkMap.updatePlayerStatus(player, true)`
        // (ChunkMap.java:1040-1048): the view is rebuilt from empty, so every
        // column the client just forgot is queued again; entity admission then
        // remains gated on the recipient actually having the target chunk.
        // `addRespawnedPlayer` appends the replacement object to the level
        // list (`PlayerList.java:420`).
        self.append_level_player(&dest_dimension, connection_id);
        self.update_sleeping_player_list(&dest_dimension, events);
        let _ = self.add_player_entity_tracking_checked(&dest_dimension, connection_id, events);
        self.update_chunk_tracking(connection_id, events);
        let _ = self.move_session_entity_tracking(connection_id, events);
        self.waypoint_add_player(connection_id, events);
        // PlayerList.java:420-424 attaches listeners before initial remote content.
        self.initialize_player_menu(connection_id, events);
        if !keep_all {
            self.send_respawn_anchor_deplete(connection_id, events);
        }
        // `:1878`.
        self.levels[connection_id].restart_client_load_timer_after_respawn();
        Some(())
    }
}
