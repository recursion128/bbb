//! `ServerPlayer.die`.

use super::*;

impl ServerWorld {
    /// `ServerPlayer.die` (ServerPlayer.java:869-929).
    pub(super) fn player_died(
        &mut self,
        connection_id: ConnectionId,
        source: &HurtSource<'_>,
        causing: Option<&crate::commands::CommandDamageSourceEntity>,
        direct: Option<&crate::commands::CommandDamageSourceEntity>,
        events: &mut Vec<WorldEvent>,
    ) {
        self.remove_player_shoulders(connection_id, events);
        let player = self.levels.player_at(connection_id);
        let entity_id = player.entity_id();
        let dimension = player.dimension;
        let victim = self.display_name_of(&player.profile);
        let player_credit = player.cold.last_hurt_by_player;
        let mob_credit = player.cold.last_hurt_by_mob;
        let position = player.position();
        self.player_game_event_at(connection_id, VanillaGameEvent::ENTITY_DIE, position, None);
        // `getKillCredit` (LivingEntity.java:1946-1952): a remembered player
        // reference wins outright and answers null when it cannot be resolved,
        // so an attacker who left credits nobody instead of falling through to
        // the remembered mob. Only with no player reference at all does the mob
        // resolve. Both arms read past the level the reference is held in; see
        // [`Self::player_reference`] and [`Self::entity_reference`].
        let killer = match player_credit {
            Some(uuid) => self
                .player_reference(uuid)
                .map(|killer| KillCredit::Player {
                    connection_id: killer,
                }),
            None => mob_credit.and_then(|uuid| {
                let (level_key, entity_id) = self.entity_reference(&dimension, uuid)?;
                let entity = self.level_by_id(&level_key).entities.get(entity_id)?;
                Some(KillCredit::Stored {
                    dimension: level_key,
                    entity_id,
                    entity_type_id: entity.entity_type_id(),
                    display_name: self.stored_display_name_of(entity),
                })
            }),
        };
        let kill_credit = match &killer {
            Some(KillCredit::Player { connection_id }) => {
                Some(self.display_name_of(&self.levels[*connection_id].profile))
            }
            Some(KillCredit::Stored { display_name, .. }) => Some(display_name.clone()),
            None => None,
        };
        let causing_name = causing.map(|entity| entity.display_name().clone());
        let direct_name = direct.map(|entity| entity.display_name().clone());
        // `:871-872`: the rule decides whether the message is built at all.
        // With it off the packet still goes out, carrying `CommonComponents.EMPTY`
        // (`:899`), and nobody is told.
        let show_death_message = matches!(
            self.admin.game_rule_value(RULE_SHOW_DEATH_MESSAGES),
            Some(GameRuleValue::Boolean(true))
        );
        let message = if show_death_message {
            bbb_server_datapack::damage::death_message_with_registry(
                &self.levels[connection_id].cold.combat,
                source,
                victim,
                kill_credit,
                causing_name,
                direct_name,
                self.plugins().content_registry(),
            )
        } else {
            // `CommonComponents.EMPTY` (CommonComponents.java:7).
            RichTextComponent::plain("")
        };
        // `:874-889`. The oversize fallback the send listener carries
        // (`:877-888`) needs a message longer than the packet can hold, which no
        // death message this server can build reaches.
        events.push(send(
            connection_id,
            ClientboundPlayPacket::PlayerCombatKill(PlayerCombatKill {
                player_id: entity_id,
                message: message.clone(),
            }),
        ));
        if show_death_message {
            self.broadcast_death_message(connection_id, message, events);
        }
        // `:903-905`.
        if self.game_rule_bool("minecraft:forgive_dead_players", true) {
            let player_uuid = self.levels[connection_id].profile.uuid;
            let block = BlockPos {
                x: position.x.floor() as i32,
                y: position.y.floor() as i32,
                z: position.z.floor() as i32,
            };
            self.with_stored_hurt_level(&dimension, |entities, level| {
                entities.forgive_dead_player(&*level, player_uuid, block);
            });
        }

        // `:907-909`.
        let spectator =
            self.levels[connection_id].game_type == bbb_protocol::game_types::GameType::Spectator;
        if !spectator {
            self.drop_player_death_loot(connection_id, source, causing, direct);
        }
        // `:911`.
        self.increment_death_count_scores(connection_id, events);

        if let Some(killer) = &killer {
            // `KilledTrigger.trigger`'s `killingBlow` (`KilledTrigger.java:21-23`):
            // the source the death was dealt by, whose two entity slots and
            // `isDirect` (`DamageSource.java:30-32`) the resolved command source
            // already carries.
            let killing_blow = KillingBlow {
                damage_type: source.damage_type.resource_id.clone(),
                direct_entity: Some(killed_blow_entity(self, &dimension, direct)),
                causing_entity: killed_blow_entity(self, &dimension, causing),
                is_direct: Some(
                    causing.map(crate::commands::CommandDamageSourceEntity::entity_id)
                        == direct.map(crate::commands::CommandDamageSourceEntity::entity_id),
                ),
            };
            // `:913-914`: the victim counts the killer's own entity type.
            let killed_by_type_id = match killer {
                KillCredit::Player { .. } => VANILLA_ENTITY_TYPE_PLAYER_ID,
                KillCredit::Stored { entity_type_id, .. } => *entity_type_id,
            };
            self.levels[connection_id].award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID, killed_by_type_id),
            );
            // The dying session as the other side of the kill, snapshotted
            // before the trigger takes the world.
            let died = self
                .session_loot_view(connection_id)
                .map_or(KillEntity::Opaque, |facts| {
                    KillEntity::Facts(Box::new(facts))
                });
            match killer {
                // `:915` into `ServerPlayer.awardKillScore` (`:941-954`), whose
                // body sits behind `victim != this` (`:942`): a session that
                // killed itself still counts the line above and nothing here.
                KillCredit::Player {
                    connection_id: killer,
                } => {
                    if *killer != connection_id {
                        let killer_entity = self
                            .session_loot_view(*killer)
                            .map_or(KillEntity::Opaque, |facts| {
                                KillEntity::Facts(Box::new(facts))
                            });
                        // `:943` into `Entity.awardKillScore`
                        // (`Entity.java:1931-1935`), whose one arm belongs to
                        // the dying session and names the killing session.
                        self.trigger_killed(
                            connection_id,
                            "minecraft:entity_killed_player",
                            killer_entity,
                            killing_blow.clone(),
                            events,
                        );
                        self.levels[*killer].award_stat_once(
                            &mut self.scoreboard,
                            custom_stat("minecraft:player_kills"),
                        );
                        self.increment_kill_count_scores(
                            *killer,
                            crate::world::score_criteria::KillScoreVictim::Player(connection_id),
                        );
                        // `:955`, the killer's own side of the same kill, whose
                        // other side is the session that died.
                        self.trigger_killed(
                            *killer,
                            "minecraft:player_killed_entity",
                            died,
                            killing_blow,
                            events,
                        );
                    }
                }
                // A mob killer reaches only `Entity.awardKillScore`
                // (`Entity.java:1931-1935`): the dying session's criterion,
                // with no killer statistics of its own.
                KillCredit::Stored {
                    dimension: killer_dimension,
                    entity_id,
                    entity_type_id,
                    ..
                } => {
                    let killer_dimension = *killer_dimension;
                    let wither = *entity_type_id
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_ID;
                    let entity = self.killed_entity_of_stored(&killer_dimension, *entity_id);
                    self.trigger_killed(
                        connection_id,
                        "minecraft:entity_killed_player",
                        entity,
                        killing_blow,
                        events,
                    );
                    // `:916` into `LivingEntity.createWitherRose` (`:1459-1475`).
                    if wither {
                        self.with_stored_hurt_level(&dimension, |entities, level| {
                            crate::entity::mob_death_loot::place_wither_rose(
                                entities,
                                level,
                                None,
                                block_position(position),
                                position,
                            );
                        });
                    }
                }
            }
        }

        // `:919`, `EntityEvent.DEATH` (EntityEvent.java:6) through
        // `ServerLevel.broadcastEntityEvent` (ServerLevel.java:1181-1183).
        self.send_to_player_tracking_and_self(
            connection_id,
            entity_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id: ENTITY_EVENT_DEATH,
            }),
            events,
        );
        let death_dimension = self.level_key(dimension).resource_id().to_string();
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        // `:920-922`.
        player.award_stat_once(&mut self.scoreboard, custom_stat(CUSTOM_STAT_DEATHS));
        player.reset_stat(
            &mut self.scoreboard,
            custom_stat(CUSTOM_STAT_TIME_SINCE_DEATH),
        );
        player.reset_stat(
            &mut self.scoreboard,
            custom_stat(CUSTOM_STAT_TIME_SINCE_REST),
        );
        player.clear_environment_on_death();
        // `:926-927`.
        let left_combat = player.cold.combat.recheck_status(player.tick_count, false);
        player.cold.last_death_location = Some(bbb_protocol::packets::GlobalPos {
            dimension: death_dimension,
            pos: block_position(player.position()),
        });
        player.mark_client_unloaded_after_death();
        if let Some(duration) = left_combat {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::PlayerCombatEnd(bbb_protocol::packets::PlayerCombatEnd {
                    duration,
                }),
            ));
        }
    }
}
