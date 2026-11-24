use super::*;
use crate::commands::CommandPlayerTarget;
use crate::world::LevelId;

impl ServerWorld {
    /// `hurtServer` on a session player a command names; `false` when the
    /// session is gone or the hurt did not land.
    pub(in crate::world) fn hurt_player_by_command(
        &mut self,
        player: &CommandPlayerTarget,
        damage: crate::world::combat::LevelPlayerDamage<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(connection_id) = self.command_player_connection(player) else {
            return false;
        };
        self.hurt_connected_player(
            connection_id,
            damage.damage_type,
            damage.amount,
            damage.causing,
            damage.direct,
            damage.source_position,
            events,
        )
    }

    /// `hurtServer` on a level-owned entity a command names.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn hurt_stored_entity_by_command(
        &mut self,
        dimension: LevelId,
        entity_id: i32,
        damage_type: &str,
        amount: f32,
        causing: Option<crate::commands::CommandDamageSourceEntity>,
        direct: Option<crate::commands::CommandDamageSourceEntity>,
        source_position: Option<[f64; 3]>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<StoredEntityHurtOutcome> {
        let (registry_tags, fall_damage_immune) =
            self.stored_command_damage_facts(&dimension, entity_id)?;
        self.hurt_stored_entity_from_level(
            dimension,
            entity_id,
            damage_type,
            amount,
            causing,
            direct,
            source_position,
            registry_tags,
            fall_damage_immune,
            None,
            events,
        )
    }

    /// The level's registry tags and `#minecraft:fall_damage_immune` membership
    /// of the entity's type.
    fn stored_command_damage_facts(
        &self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> Option<(EntityRegistryTagView, bool)> {
        let level = self.levels.get(dimension)?;
        let entity = level.entities.get(entity_id)?;
        let registry_tags = level.chunks.registry_tags_shared().map_or(
            EntityRegistryTagView::Builtin26_1,
            EntityRegistryTagView::Frozen,
        );
        let resolver = RuntimePersistentRegistryResolver {
            datapacks: self.cold.config.datapacks.as_deref(),
            content_registry: self.registries.content_registry(),
        };
        let fall_damage_immune = registry_tag_contains(
            &resolver,
            "minecraft:entity_type",
            "minecraft:fall_damage_immune",
            entity.data().entity_resource_id(),
        );
        Some((registry_tags, fall_damage_immune))
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn hurt_stored_entity_from_level(
        &mut self,
        dimension: LevelId,
        entity_id: i32,
        damage_type: &str,
        amount: f32,
        causing: Option<crate::commands::CommandDamageSourceEntity>,
        direct: Option<crate::commands::CommandDamageSourceEntity>,
        source_position: Option<[f64; 3]>,
        registry_tags: EntityRegistryTagView,
        fall_damage_immune: bool,
        caller: Option<&LevelId>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<StoredEntityHurtOutcome> {
        let registry = Arc::clone(&self.cold.damage_types);
        let damage_type = registry
            .iter()
            .find(|entry| entry.resource_id == damage_type)?;
        self.levels.get(&dimension)?.entities.get(entity_id)?;
        let mob_griefing = match self.admin.game_rule_value("minecraft:mob_griefing") {
            Some(GameRuleValue::Boolean(value)) => value,
            Some(GameRuleValue::Integer(_)) | None => true,
        };
        let block_drops = match self.admin.game_rule_value("minecraft:block_drops") {
            Some(GameRuleValue::Boolean(value)) => value,
            Some(GameRuleValue::Integer(_)) | None => true,
        };
        let entity_drops = match self.admin.game_rule_value("minecraft:entity_drops") {
            Some(GameRuleValue::Boolean(value)) => value,
            Some(GameRuleValue::Integer(_)) | None => true,
        };
        let attacker_may_build = causing
            .as_ref()
            .and_then(|entity| match entity {
                crate::commands::CommandDamageSourceEntity::Player { may_build, .. } => {
                    Some(*may_build)
                }
                crate::commands::CommandDamageSourceEntity::Stored { .. } => None,
            })
            .unwrap_or(true);
        let causing_entity_snapshot = causing.as_ref().and_then(command_explosion_source_entity);
        let causing_entity = causing.as_ref().map(|entity| match entity {
            crate::commands::CommandDamageSourceEntity::Player {
                user,
                entity_id,
                instabuild,
                ..
            } => StoredDamageCausingEntity::Player {
                uuid: user.id,
                entity_id: *entity_id,
                instabuild: *instabuild,
            },
            crate::commands::CommandDamageSourceEntity::Stored {
                entity_id, is_mob, ..
            } => {
                if *is_mob {
                    StoredDamageCausingEntity::Mob {
                        entity_id: *entity_id,
                    }
                } else {
                    StoredDamageCausingEntity::Other {
                        entity_id: *entity_id,
                    }
                }
            }
        });
        let rules = StoredEntityDamageRules {
            mob_griefing,
            entity_drops,
            tnt_explodes: self.game_rule_bool("minecraft:tnt_explodes", true),
            fall_damage_immune,
        };
        let direct_resource_id = direct.as_ref().map(|entity| match entity {
            crate::commands::CommandDamageSourceEntity::Player { .. } => "minecraft:player",
            crate::commands::CommandDamageSourceEntity::Stored {
                entity_resource_id, ..
            } => entity_resource_id.as_str(),
        });
        let effective_source_position = source_position
            .map(|[x, y, z]| Vec3d { x, y, z })
            .or_else(|| direct.as_ref().and_then(command_damage_source_position));
        let direct_entity_id = direct.as_ref().map(|entity| entity.entity_id());
        // `LootContextParams.ATTACKING_ENTITY` and `DIRECT_ATTACKING_ENTITY`
        // (`LivingEntity.java:1541-1542`) for the two entities the command
        // named, which are session players when `/damage … by <player>` said
        // so.
        for entity in causing.iter().chain(&direct) {
            if let crate::commands::CommandDamageSourceEntity::Player { user, .. } = entity {
                self.command_player_connection(user)?;
            }
        }
        let player_view = |entity: Option<&crate::commands::CommandDamageSourceEntity>| match entity
        {
            Some(crate::commands::CommandDamageSourceEntity::Player { user, .. }) => self
                .command_player_connection(user)
                .and_then(|target| self.session_loot_view(target)),
            _ => None,
        };
        let attacker_views = super::super::loot_context::SessionAttackerViews::new(
            player_view(causing.as_ref()),
            player_view(direct.as_ref()),
        );
        let cross_caller = caller.filter(|caller| *caller != &dimension);
        let result = self.with_stored_hurt_level_checked(
            &dimension,
            cross_caller,
            |entities, level, reader| {
                let request = StoredHurtRequest {
                    direct_entity_snapshot: None,
                    target_entity_id: entity_id,
                    damage_type_resource_id: &damage_type.resource_id,
                    amount,
                    direct_resource_id,
                    registry_tags: &registry_tags,
                    causing_entity_snapshot,
                    source_position: effective_source_position,
                    explicit_source_position: source_position.map(|[x, y, z]| Vec3d { x, y, z }),
                };
                let source = StoredEntityDamageSource {
                    damage_type,
                    direct_entity_id,
                    causing_entity,
                    attacker_views: attacker_views.views(),
                };
                let context = StoredEntityDamageContext {
                    block_drops,
                    attacker_may_build,
                    attacker_spectator: causing.as_ref().and_then(|entity| match entity {
                        crate::commands::CommandDamageSourceEntity::Player {
                            spectator, ..
                        } => Some(*spectator),
                        crate::commands::CommandDamageSourceEntity::Stored { .. } => None,
                    }),
                };
                if cross_caller.is_some() {
                    hurt_stored_entity_with_immunity_reader(
                        request,
                        None,
                        source,
                        rules,
                        context,
                        (
                            entities,
                            level,
                            reader.expect("distinct caller provides immunity reader"),
                        ),
                    )
                } else {
                    hurt_stored_entity(request, None, source, rules, context, entities, level)
                }
            },
        );
        let outcome = match result {
            Some(StoredHurtResult::Applied { outcome, .. }) => Some(outcome),
            Some(StoredHurtResult::DropNoop) | None => None,
        };
        let _ = self.run_entity_commands(&dimension, events);
        outcome
    }

    /// `KillCommand.kill`'s victim for the kinds `LivingEntity.kill`
    /// (`LivingEntity.java:303-305`) does not claim: `Entity.kill`
    /// (`Entity.java:399-402`), the removal plus the `ENTITY_DIE` it owes.
    ///
    /// The level seam is the stored-hurt one because it is the frame outside
    /// the entity tick that lends this level's live store and posts the
    /// `gameEvent`s raised against it; nothing here consults a registry, so an
    /// unavailable registry generation cannot drop the kill the way it can drop
    /// a hurt.
    pub(in crate::world) fn kill_stored_entity_by_command(
        &mut self,
        dimension: LevelId,
        entity_id: i32,
    ) -> bool {
        if !self.levels.contains_key(&dimension) {
            return false;
        }
        self.with_stored_hurt_level(&dimension, |entities, level| {
            crate::entity::kill::kill(entities, level, entity_id)
        })
    }

    #[cfg(test)]
    pub(crate) fn hurt_stored_entity_by_command_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        damage_type: &str,
        amount: f32,
        causing: Option<crate::commands::CommandDamageSourceEntity>,
    ) -> Option<StoredEntityHurtOutcome> {
        self.hurt_stored_entity_by_command(
            dimension.into(),
            entity_id,
            damage_type,
            amount,
            causing,
            None,
            None,
            &mut Vec::new(),
        )
    }

    #[cfg(test)]
    pub(crate) fn hurt_stored_entity_by_command_at_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        damage_type: &str,
        amount: f32,
        source_position: [f64; 3],
    ) -> Option<StoredEntityHurtOutcome> {
        self.hurt_stored_entity_by_command(
            dimension.into(),
            entity_id,
            damage_type,
            amount,
            None,
            None,
            Some(source_position),
            &mut Vec::new(),
        )
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn hurt_stored_entity_by_command_with_facts_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        damage_type: &str,
        amount: f32,
        causing: Option<crate::commands::CommandDamageSourceEntity>,
        direct: Option<crate::commands::CommandDamageSourceEntity>,
        source_position: Option<[f64; 3]>,
        registry_tags: EntityRegistryTagView,
        fall_damage_immune: bool,
    ) -> Option<StoredEntityHurtOutcome> {
        self.hurt_stored_entity_from_level(
            dimension.into(),
            entity_id,
            damage_type,
            amount,
            causing,
            direct,
            source_position,
            registry_tags,
            fall_damage_immune,
            None,
            &mut Vec::new(),
        )
    }

    #[cfg(test)]
    pub(crate) fn hurt_connected_player_by_command_for_test(
        &mut self,
        connection_id: ConnectionId,
        damage_type: &str,
        amount: f32,
        causing: Option<crate::commands::CommandDamageSourceEntity>,
        direct: Option<crate::commands::CommandDamageSourceEntity>,
        source_position: Option<[f64; 3]>,
    ) -> (bool, Vec<WorldEvent>) {
        let mut events = Vec::new();
        let succeeded = self.hurt_connected_player(
            connection_id,
            damage_type,
            amount,
            causing,
            direct,
            source_position,
            &mut events,
        );
        (succeeded, events)
    }

    #[cfg(test)]
    pub(crate) fn discard_stored_command_source_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> bool {
        self.level_mut_if_present(dimension)
            .and_then(|level| {
                level
                    .entities
                    .remove(entity_id, crate::entity::RemovalReason::Discarded)
            })
            .is_some()
    }
}

fn command_damage_source_position(
    source: &crate::commands::CommandDamageSourceEntity,
) -> Option<Vec3d> {
    match source {
        crate::commands::CommandDamageSourceEntity::Player { position, .. } => Some(*position),
        crate::commands::CommandDamageSourceEntity::Stored { position, .. } => *position,
    }
}

fn command_explosion_source_entity(
    source: &crate::commands::CommandDamageSourceEntity,
) -> Option<ExplosionSourceEntity> {
    match source {
        crate::commands::CommandDamageSourceEntity::Player {
            user,
            entity_id,
            position,
            instabuild,
            may_build,
            spectator,
            ..
        } => Some(ExplosionSourceEntity {
            custom_name: None,
            entity_id: *entity_id,
            entity_resource_id: "minecraft:player".to_string(),
            uuid: user.id,
            position: *position,
            kind: ExplosionSourceEntityKind::Player {
                instabuild: *instabuild,
                may_build: *may_build,
                spectator: *spectator,
            },
        }),
        crate::commands::CommandDamageSourceEntity::Stored {
            entity_id,
            entity_resource_id,
            uuid: Some(uuid),
            position: Some(position),
            is_mob,
            is_living_non_player,
            ..
        } => Some(ExplosionSourceEntity {
            custom_name: None,
            entity_id: *entity_id,
            entity_resource_id: entity_resource_id.clone(),
            uuid: *uuid,
            position: *position,
            kind: if *is_mob {
                ExplosionSourceEntityKind::Mob
            } else if *is_living_non_player {
                ExplosionSourceEntityKind::LivingOther
            } else {
                ExplosionSourceEntityKind::Other
            },
        }),
        crate::commands::CommandDamageSourceEntity::Stored { .. } => None,
    }
}
