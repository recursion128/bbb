use crate::enchantment_combat_effects::CombatValue;
use crate::entity::damage::DamageCauser;
use crate::world::simulation_level::LevelCtx;

use super::*;

pub(crate) fn hurt_server(
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
    rules: StoredEntityDamageRules,
    causer: DamageCauser,
) -> GeneralLivingHurtEffects {
    hurt_server_with_tags(
        entity,
        source,
        damage,
        rules,
        resolved_builtin_damage_tags(entity, source),
        crate::entity::witch::damage_multiplier(entity, source),
        causer,
        // No store or level to broadcast to or
        // apply the protection's effects against; the live spine runs both.
        None,
    )
}

struct LiveHurtHooks<'a, 's, 'reader, 'l, 'w> {
    store: &'a mut EntityStore,
    level: &'a mut LevelCtx<'l, 'w>,
    source: StoredEntityDamageSource<'s>,
    causing_entity_snapshot: Option<&'s crate::explosion::ExplosionSourceEntity>,
    direct_entity_snapshot: Option<&'s crate::explosion::ExplosionSourceEntity>,
    source_position: Option<Vec3d>,
    explicit_source_position: Option<Vec3d>,
    immunity_reader: Option<&'a mut crate::entity::enchanted_damage::DamageImmunityReader<'reader>>,
}

impl LiveHurtHooks<'_, '_, '_, '_, '_> {
    fn damage_event(&mut self, entity: &Entity) {
        self.store.broadcast_to_tracking(
            entity.id(),
            crate::connection::ClientboundPlayPacket::DamageEvent(
                bbb_protocol::packets::DamageEvent {
                    entity_id: entity.id(),
                    source_type_id: self.source.damage_type.registry_id,
                    source_cause_id: self
                        .source
                        .causing_entity
                        .map_or(-1, StoredDamageCausingEntity::entity_id),
                    source_direct_id: self.source.direct_entity_id.unwrap_or(-1),
                    source_position: self.explicit_source_position,
                },
            ),
        );
    }

    fn wake_sleeper(&mut self, entity: &mut Entity) -> bool {
        self.level
            .stop_sleeping(
                crate::entity::level::EntityBlockContext::new(Some(&mut *self.store), None),
                entity,
            )
            .is_some()
    }

    fn knockback(&mut self, entity: &mut Entity) {
        if !entity.creaking_can_move() {
            return;
        }
        if self.source_position.is_some() {
            let (xd, zd) = living::hurt_knockback_direction(
                entity.position(),
                None,
                None,
                self.source_position,
            );
            living::knockback(entity, self.store, 0.4, xd, zd);
        }
    }

    fn completed_hurt(
        &mut self,
        entity: &mut Entity,
        full_damage: bool,
        is_fire: bool,
        wolf_armor_absorbs: bool,
        crackiness_before: Option<u8>,
    ) {
        crate::entity::witch::remember_damage_with_is_fire(
            entity,
            StoredEntityHurtOutcome {
                applied: true,
                succeeded: true,
            },
            is_fire,
            self.store.game_time(),
        );
        if full_damage {
            crate::entity::ambient_sound::reset_clock(entity, self.store);
            super::hurt_sounds::play_hurt_or_death(entity, self.store, wolf_armor_absorbs);
            // LivingEntity.playSecondaryHurtSound (LivingEntity.java:1322-1327).
            if self.source.damage_type.resource_id == "minecraft:thorns" {
                self.store
                    .play_entity_sound(crate::entity::store::PendingEntitySound {
                        position: entity.position(),
                        resource_id: "minecraft:enchant.thorns.hit".to_owned(),
                        source: bbb_protocol::packets::SoundSource::Hostile,
                        volume: 1.0,
                        pitch: 1.0,
                    });
            }
        }
        super::hurt_sounds::play_iron_golem_crack(entity, self.store, crackiness_before);
    }

    fn death_protection(&mut self, entity: &mut Entity) -> bool {
        crate::entity::death_protection::check_totem_death_protection(
            entity,
            self.store,
            self.level,
            &self.source.damage_type.resource_id,
        )
    }

    fn enchantment_immune(&mut self, entity: &Entity) -> Result<bool, String> {
        crate::entity::enchanted_damage::stored_enchantment_immune_with_reader(
            self.store,
            self.level,
            entity,
            &self.source,
            self.immunity_reader.as_deref_mut(),
        )
    }

    fn armor_effectiveness(&mut self, entity: &Entity, fraction: f32) -> Result<f32, String> {
        let weapon = match self.source.causing_entity {
            Some(StoredDamageCausingEntity::Player { uuid, .. })
                if self.source.direct_entity_id
                    == self
                        .source
                        .causing_entity
                        .map(StoredDamageCausingEntity::entity_id) =>
            {
                crate::entity::enchanted_damage::EnchantedDamageItems::SessionWeapon { uuid }
            }
            _ => {
                let Some(direct) = self
                    .source
                    .direct_entity_id
                    .and_then(|id| self.store.get(id))
                else {
                    return Ok(fraction);
                };
                let enchantments = crate::entity::enchanted_damage::weapon_item(direct)
                    .map(|stack| {
                        crate::entity::enchanted_damage::stack_enchantments(
                            stack,
                            self.store.enchantment_registry(),
                        )
                    })
                    .unwrap_or_default();
                if enchantments.is_empty() {
                    return Ok(fraction);
                }
                return crate::entity::mob_death_loot::with_enchanted_damage_request(
                    self.store,
                    self.level,
                    entity,
                    &self.source,
                    crate::entity::enchanted_damage::EnchantedDamageItems::Listed(&enchantments),
                    |level, request| {
                        level.enchanted_damage_value(
                            request,
                            CombatValue::ArmorEffectiveness,
                            fraction,
                        )
                    },
                );
            }
        };
        crate::entity::mob_death_loot::with_enchanted_damage_request(
            self.store,
            self.level,
            entity,
            &self.source,
            weapon,
            |level, request| {
                level.enchanted_damage_value(request, CombatValue::ArmorEffectiveness, fraction)
            },
        )
    }

    fn damage_protection(&mut self, entity: &Entity) -> Result<f32, String> {
        let worn = crate::entity::enchanted_damage::worn_enchantments(self.store, entity)?;
        if worn.is_empty() {
            return Ok(0.0);
        }
        crate::entity::mob_death_loot::with_enchanted_damage_request(
            self.store,
            self.level,
            entity,
            &self.source,
            crate::entity::enchanted_damage::EnchantedDamageItems::Listed(&worn),
            |level, request| {
                level.enchanted_damage_value(request, CombatValue::DamageProtection, 0.0)
            },
        )
    }

    fn hurt_armor(&mut self, entity: &mut Entity, damage: f32) -> Result<(), String> {
        if !matches!(
            entity.data().entity_resource_id(),
            "minecraft:wolf" | "minecraft:horse"
        ) {
            return Ok(());
        }
        if damage <= 0.0 {
            return Ok(());
        }
        let Some(stack) = entity
            .living()
            .and_then(|living| living.item_by_slot(EquipmentSlot::Body))
            .filter(|stack| !stack.is_empty())
            .cloned()
        else {
            return Ok(());
        };
        let publication = self.level.item_stack_resolver();
        if !crate::entity::item_components::equippable_damage_on_hurt(&stack)?
            || crate::entity::item_components::damage_and_max(&stack)?.is_none()
            || crate::entity::damage::item_resists_damage(
                &stack,
                &self.source.damage_type.resource_id,
                publication,
            )
        {
            return Ok(());
        }
        // `(int)Math.max(1.0F, damage / 4.0F)`.
        let durability = (damage / 4.0).max(1.0) as i32;
        crate::entity::equipment_wear::hurt_and_break(
            entity,
            self.store,
            self.level,
            EquipmentSlot::Body,
            durability,
        )
        .map(|_| ())
    }

    fn wolf_armor_absorb(&mut self, entity: &mut Entity, damage: f32) -> Result<(), String> {
        crate::entity::equipment_wear::wolf_armor_absorb(entity, self.store, self.level, damage)
    }

    fn award_attacker_stat(&mut self, stat: &'static str, damage: f32) {
        let Some(StoredDamageCausingEntity::Player { uuid, .. }) = self.source.causing_entity
        else {
            return;
        };
        if damage > 0.0 && damage < 3.402_823_5e37 {
            self.store
                .queue_command(crate::entity::EntityCommand::PlayerStat(
                    crate::entity::store::PendingPlayerStat {
                        player_uuid: uuid,
                        stat,
                        // `Math.round(damage * 10.0F)`.
                        amount: (damage * 10.0 + 0.5).floor() as i32,
                    },
                ));
        }
    }
}

// Multi-arg Vanilla `LivingEntity.hurtServer` kernel over disjoint entity/world borrows.
#[allow(clippy::too_many_arguments)]
fn hurt_server_with_tags(
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
    rules: StoredEntityDamageRules,
    tags: ResolvedDamageTags,
    witch_damage_multiplier: f32,
    causer: DamageCauser,
    mut hooks: Option<&mut LiveHurtHooks<'_, '_, '_, '_, '_>>,
) -> GeneralLivingHurtEffects {
    if !matches!(entity.data(), EntityData::GeneralLiving(_))
        || causer.breeze
            && entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID
        // Allay.java:154-156.
        || source.causing_entity.is_some_and(|causing| {
            matches!(causing, StoredDamageCausingEntity::Player { uuid, .. }
                if crate::entity::allay::liked_player(entity) == Some(uuid))
        })
        || living::is_invulnerable_to_with_tags(
            entity,
            source.causing_entity.is_some_and(|causing| {
                matches!(
                    causing,
                    StoredDamageCausingEntity::Player {
                        instabuild: true,
                        ..
                    }
                )
            }),
            rules.fall_damage_immune,
            tags.bypasses_invulnerability,
            tags.is_fire,
            tags.is_fall,
        )
    {
        return refused();
    }
    if hooks
        .as_mut()
        .is_some_and(|hooks| hooks.enchantment_immune(entity) == Ok(true))
    {
        return refused();
    }
    // Bee.java:636-643.
    if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID {
        if let Some(runtime) = hooks
            .as_mut()
            .and_then(|hooks| hooks.store.mob_ai_mut(entity.id()))
        {
            runtime.bee_stop_pollinating();
        }
    }
    // Wolf.java:393-400, Parrot.java:397-404.
    if matches!(
        entity.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID
    ) {
        let stood = match &mut entity.data {
            EntityData::GeneralLiving(data) => data
                .tamable_retained_mut()
                .filter(|tamable| tamable.ordered_to_sit())
                .map(|tamable| tamable.set_ordered_to_sit(false))
                .is_some(),
            _ => false,
        };
        if stood {
            entity.mark_synched_dirty(
                bbb_protocol::entity_synched_data::VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID,
            );
        }
    }
    if tags.is_fire
        && entity
            .living()
            .is_some_and(|living| living.active_effect("minecraft:fire_resistance").is_some())
        || entity
            .living()
            .is_none_or(LivingEntityState::is_dead_or_dying)
    {
        return refused();
    }

    if entity.sleeping_pos().is_some()
        && hooks
            .as_mut()
            .is_some_and(|hooks| !hooks.wake_sleeper(entity))
    {
        return refused();
    }
    let crackiness_before = super::hurt_sounds::iron_golem_crackiness(entity);
    let damage = crate::entity::armadillo::scare::damage(entity, damage);
    let mut damage = if damage < 0.0 { 0.0 } else { damage };
    let original_damage = damage;
    if tags.is_freezing && tags.freeze_hurts_extra_types {
        damage *= 5.0;
    }
    if tags.damages_helmet
        && entity
            .living()
            .is_some_and(|living| living.has_item_in_slot(EquipmentSlot::Head))
    {
        damage *= 0.75;
    }
    if !damage.is_finite() {
        damage = f32::MAX;
    }
    let Some((dealt, took_full_damage)) = entity.living_mut().and_then(|living| {
        living.no_action_time = 0;
        living::hurt_cooldown(living, tags.bypasses_cooldown, damage)
    }) else {
        return refused();
    };

    let calls_living_hurt = calls_living_actually_hurt(entity, tags.bypasses_wolf_armor);
    if calls_living_hurt {
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            if let Some(animal) = data.animal_retained_mut() {
                animal.reset_love();
            }
        }
    }
    let damage_taken = if calls_living_hurt {
        actually_hurt(
            entity,
            tags,
            dealt,
            witch_damage_multiplier,
            hooks.as_deref_mut(),
        )
    } else {
        if let Some(hooks) = hooks.as_mut() {
            let _ = hooks.wolf_armor_absorb(entity, dealt);
        }
        0.0
    };
    if calls_living_hurt {
        if let Some(hooks) = hooks.as_mut() {
            crate::entity::armadillo::scare::was_hurt(
                entity,
                hooks.store,
                hooks.level,
                source,
                causer.living,
            );
        }
    }
    if damage_taken != 0.0 {
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_HEALTH_DATA_ID);
        if let Some(hooks) = hooks.as_mut() {
            crate::entity::taming::death_message::record_damage(
                entity,
                hooks.store,
                hooks.level,
                source,
                crate::entity::taming::death_message::PetDamageSourceSnapshots {
                    causing: hooks.causing_entity_snapshot,
                    direct: hooks.direct_entity_snapshot,
                },
                damage_taken,
                causer.living
                    || hooks.causing_entity_snapshot.is_some_and(|source| {
                        source.is_living_non_player() || source.player_facts().is_some()
                    }),
            );
        }
    }

    remember_damage_source(entity, source, tags, causer);
    if !tags.no_anger && !tags.no_anger_from_wind_charge && causer.living {
        if let Some(attacker) = causer.uuid {
            if crate::entity::villager_life::hurt_reputation(entity, attacker)
                && entity.is_alive()
                && matches!(
                    source.causing_entity,
                    Some(StoredDamageCausingEntity::Player { .. })
                )
            {
                if let Some(hooks) = hooks.as_mut() {
                    hooks.store.broadcast_to_tracking(
                        entity.id(),
                        ClientboundPlayPacket::EntityEvent(bbb_protocol::packets::EntityEvent {
                            entity_id: entity.id(),
                            event_id: 13,
                        }),
                    );
                }
            }
        }
    }
    if let Some(hooks) = hooks.as_mut().filter(|_| took_full_damage) {
        hooks.damage_event(entity);
    }
    // `LivingEntity.java:1222-1224`: stored bodies never block, so a zero hit still marks.
    if took_full_damage && !tags.no_impact {
        entity.mark_hurt();
    }
    if let Some(hooks) = hooks
        .as_mut()
        .filter(|_| took_full_damage && !tags.no_knockback)
    {
        hooks.knockback(entity);
    }
    let dying = entity
        .living()
        .is_some_and(LivingEntityState::is_dead_or_dying);
    let died = dying
        && !hooks
            .as_mut()
            .is_some_and(|hooks| hooks.death_protection(entity));
    if died {
        living::mark_dead(entity);
    }
    if let Some(hooks) = hooks {
        if let Some(StoredDamageCausingEntity::Player { uuid, .. }) = source.causing_entity {
            hooks
                .store
                .queue_command(crate::entity::EntityCommand::PlayerHurtEntity(
                    crate::entity::store::PendingPlayerHurtEntity {
                        player_uuid: uuid,
                        victim_entity_id: entity.id(),
                        damage_type_id: source.damage_type.registry_id,
                        direct_entity_id: source.direct_entity_id,
                        dealt: original_damage,
                        taken: damage,
                    },
                ));
        }
        hooks.completed_hurt(
            entity,
            took_full_damage && (!dying || died),
            tags.is_fire,
            !calls_living_hurt,
            crackiness_before,
        );
        if let Some(behavior) = entity
            .plugin_mob()
            .and_then(|_| hooks.store.plugins().mob_entry(entity.entity_type_id()))
            .map(|entry| std::sync::Arc::clone(&entry.behavior))
        {
            if took_full_damage {
                behavior.on_hurt(entity, hooks.store, hooks.level, damage);
            }
            if died {
                behavior.on_death(entity, hooks.store, hooks.level);
            }
        }
    }
    GeneralLivingHurtEffects {
        outcome: StoredEntityHurtOutcome {
            applied: true,
            succeeded: true,
        },
        emits_entity_damage: damage_taken != 0.0,
        game_event_source_id: Some(entity.id()),
        died,
        // Resolved by the store-owning spine beside the death it belongs to
        // (`hurt_live_entity`); this entry has no store to ask.
        perished: true,
    }
}

fn actually_hurt(
    entity: &mut Entity,
    tags: ResolvedDamageTags,
    dealt: f32,
    witch_damage_multiplier: f32,
    mut hooks: Option<&mut LiveHurtHooks<'_, '_, '_, '_, '_>>,
) -> f32 {
    // CopperGolem.java:448-451.
    crate::entity::copper_golem::set_animation_state(
        entity,
        crate::entity::copper_golem::AnimationState::Idle,
    );
    // Camel.java:483-485: damage ends sitting without a standing transition.
    if matches!(entity.data(), EntityData::GeneralLiving(data) if data.camel_state().is_some()) {
        if let Some(hooks) = hooks.as_mut() {
            let _ = crate::entity::camel::stand_up_instantly(entity, hooks.store, hooks.level);
        }
    }
    let mut damage = dealt;
    // `getDamageAfterArmorAbsorb` (`:1869-1876`).
    if !tags.bypasses_armor {
        if let Some(hooks) = hooks.as_mut() {
            let _ = hooks.hurt_armor(entity, damage);
        }
        let Some(living) = entity.living() else {
            return 0.0;
        };
        let armor = living::armor_value(living) as f32;
        let toughness = living
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_TOUGHNESS_ID)
            as f32;
        let toughness = 2.0 + toughness / 4.0;
        let real_armor = living::clamp_f32(armor - damage / toughness, armor * 0.2, 20.0);
        let fraction = real_armor / 25.0;
        let fraction = match hooks.as_mut().map_or(Ok(fraction), |hooks| {
            hooks.armor_effectiveness(entity, fraction)
        }) {
            Ok(modified) => living::clamp_f32(modified, 0.0, 1.0),
            Err(_) => fraction,
        };
        damage *= 1.0 - fraction;
    }
    damage = magic_absorb(entity, tags, damage, hooks.as_deref_mut()) * witch_damage_multiplier;
    let Some(living) = entity.living_mut() else {
        return 0.0;
    };
    let original = damage;
    damage = (damage - living.absorption_amount()).max(0.0);
    let absorption = living.absorption_amount();
    living.set_absorption_amount(absorption - (original - damage));
    if let Some(hooks) = hooks {
        hooks.award_attacker_stat("minecraft:damage_dealt_absorbed", original - damage);
    }
    if damage != 0.0 {
        let health = living.health();
        living.set_health(health - damage);
        let absorption = living.absorption_amount();
        living.set_absorption_amount(absorption - damage);
    }
    damage
}

fn magic_absorb(
    entity: &Entity,
    tags: ResolvedDamageTags,
    mut damage: f32,
    mut hooks: Option<&mut LiveHurtHooks<'_, '_, '_, '_, '_>>,
) -> f32 {
    if tags.bypasses_effects {
        return damage;
    }
    if !tags.bypasses_resistance {
        if let Some(effect) = entity
            .living()
            .and_then(|living| living.active_effect("minecraft:resistance"))
        {
            let absorb = 25_i32.wrapping_sub(effect.amplifier().wrapping_add(1).wrapping_mul(5));
            let old_damage = damage;
            damage = (damage * absorb as f32 / 25.0).max(0.0);
            if let Some(hooks) = hooks.as_mut() {
                hooks.award_attacker_stat("minecraft:damage_dealt_resisted", old_damage - damage);
            }
        }
    }
    if damage <= 0.0 {
        return 0.0;
    }
    if tags.bypasses_enchantments {
        return damage;
    }
    let protection = hooks
        .as_mut()
        .map_or(Ok(0.0), |hooks| hooks.damage_protection(entity))
        .unwrap_or(0.0);
    if protection > 0.0 {
        damage *= 1.0 - living::clamp_f32(protection, 0.0, 20.0) / 25.0;
    }
    damage
}

pub(crate) fn killed_entity(
    source: StoredEntityDamageSource<'_>,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &Entity,
) -> bool {
    let Some(causing) = source.causing_entity else {
        // `sourceEntity == null`: the gate has nothing to ask.
        return true;
    };
    crate::entity::villager_life::murder_reputation(victim, causing, store, level);
    match causing {
        StoredDamageCausingEntity::Player { .. } => true,
        StoredDamageCausingEntity::Mob { entity_id }
        | StoredDamageCausingEntity::Other { entity_id } => {
            crate::entity::mob_death_loot::drop_charged_creeper_loot(
                store, level, entity_id, victim, source,
            );
            let Some(killer) = store
                .get(entity_id)
                .or_else(|| level.detached_entity(entity_id))
            else {
                return true;
            };
            let (killer_type, killer_silent, killer_pos) = (
                killer.entity_type_id(),
                killer.is_silent(),
                killer.block_position(),
            );
            crate::entity::zombie::killed_entity(
                store,
                killer_type,
                killer_silent,
                killer_pos,
                victim,
            )
        }
    }
}

fn record_player_kill(
    source: StoredEntityDamageSource<'_>,
    store: &mut EntityStore,
    victim: &Entity,
) {
    let kill_credit_player_uuid = victim
        .living()
        .and_then(|living| living.last_hurt_by_player);
    let statistic_player_uuid = match source.causing_entity {
        Some(StoredDamageCausingEntity::Player { uuid, .. }) => Some(uuid),
        _ => None,
    };
    if kill_credit_player_uuid.is_none() && statistic_player_uuid.is_none() {
        return;
    }
    let causing_entity_id = source
        .causing_entity
        .map(StoredDamageCausingEntity::entity_id);
    store.queue_command(crate::entity::EntityCommand::PlayerKill(
        PendingPlayerKill {
            kill_credit_player_uuid,
            statistic_player_uuid,
            victim_type_id: victim.entity_type_id(),
            victim_entity_id: victim.id(),
            victim_equipment: kill_credit_player_uuid
                .and_then(|_| victim.living())
                .map(|living| {
                    living
                        .equipment()
                        .iter()
                        .map(|(slot, stack)| (slot, stack.clone()))
                        .collect()
                }),
            killing_blow: PendingKillingBlow {
                damage_type: source.damage_type.resource_id.clone(),
                direct_entity_id: source.direct_entity_id,
                causing_entity_id,
                is_direct: source.direct_entity_id == causing_entity_id,
            },
        },
    ));
}

pub(crate) fn emit_entity_die(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    effects: GeneralLivingHurtEffects,
) -> Option<()> {
    if effects.died {
        crate::entity::taming::death_message::queue_death(entity, store, source);
        crate::entity::villager::release_all_pois(entity, store, level);
        if effects.perished {
            level.game_event_at(
                VanillaGameEvent::ENTITY_DIE,
                entity.position(),
                Some(entity.id()),
            );
            crate::entity::mob_death_loot::drop_all_death_loot(store, level, entity, source)?;
        }
        broadcast_death(store, entity.id());
        entity.set_pose_id_with_level(
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DYING_ID,
            level,
            Some(&mut *store),
        )?;
    }
    Some(())
}

pub(crate) fn external_die(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
) -> bool {
    external_die_inner(entity, store, level, source).is_some()
}

fn external_die_inner(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
) -> Option<()> {
    if !entity.is_alive() || entity.living().is_none_or(|living| living.dead) {
        return None;
    }
    let tags = resolved_builtin_damage_tags(entity, source);
    let causer = DamageCauser::resolve(source.causing_entity, |id| {
        store.get(id).or_else(|| level.detached_entity(id))
    });
    blame_source_for_damage(entity, source, tags, causer);
    record_player_kill(source, store, entity);
    if entity.sleeping_pos().is_some() {
        if !level.can_stop_sleeping(
            crate::entity::level::EntityBlockContext::new(Some(&mut *store), None),
            entity,
        ) {
            return None;
        }
        level.stop_sleeping(
            crate::entity::level::EntityBlockContext::new(Some(&mut *store), None),
            entity,
        )?;
    }
    // `LivingEntity.die` marks dead before `killedEntity` and keeps the old
    // pose visible to death loot until after the death broadcast (`:1443-1455`).
    entity
        .living_mut()
        .expect("external die requires living")
        .dead = true;
    let perished = killed_entity(source, store, level, entity);
    emit_entity_die(
        store,
        level,
        entity,
        source,
        GeneralLivingHurtEffects {
            outcome: StoredEntityHurtOutcome {
                applied: true,
                succeeded: true,
            },
            emits_entity_damage: false,
            game_event_source_id: None,
            died: true,
            perished,
        },
    )
}

pub(crate) fn broadcast_death(store: &mut EntityStore, entity_id: i32) {
    store.broadcast_to_tracking(
        entity_id,
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id,
            event_id: crate::entity::living::ENTITY_EVENT_DEATH,
        }),
    );
}

#[cfg(test)]
pub(crate) fn hurt_server_with_runtime(
    entity: &mut Entity,
    store: &mut EntityStore,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
    rules: StoredEntityDamageRules,
) -> GeneralLivingHurtEffects {
    let is_wither = matches!(entity.data(), EntityData::GeneralLiving(data) if data.wither_retained().is_some());
    if is_wither {
        let mut membership = |registry: &str, tag: &str, value: &str| {
            matches!(
                bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1(registry, tag),
                bbb_protocol::PersistentRegistryTagMembers::Present(members)
                    if members.iter().any(|member| member == value)
            )
        };
        let bypasses_invulnerability = membership(
            "minecraft:damage_type",
            "minecraft:bypasses_invulnerability",
            &source.damage_type.resource_id,
        );
        if wither_refuses_hurt(
            entity,
            store,
            source,
            None,
            bypasses_invulnerability,
            &mut membership,
        ) {
            return refused();
        }
    }
    crate::entity::panda::before_hurt(entity);
    wake_bat_before_hurt(entity, source, rules, None);
    let effects = hurt_server(
        entity,
        source,
        damage,
        rules,
        DamageCauser::resolve(source.causing_entity, |id| store.get(id)),
    );
    if is_wither && effects.outcome.succeeded {
        if let Some(runtime) = store.mob_ai_mut(entity.id()) {
            runtime.accept_wither_hurt();
        }
    }
    crate::entity::living::mob_effects::on_mob_hurt(entity, store, effects.outcome.succeeded);
    crate::entity::witch::remember_damage(
        entity,
        store,
        source,
        effects.outcome,
        effects.emits_entity_damage,
    );
    apply_success_follow_up(entity, store, effects);
    effects
}

pub(crate) fn hurt_server_with_runtime_in_level(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
    rules: StoredEntityDamageRules,
) -> GeneralLivingHurtEffects {
    hurt_live_entity(
        entity, store, level, None, source, None, None, None, damage, rules, None, None, None,
    )
}

// Guardian.java:314-326: the direct living attacker is hurt before ordinary hurt gates.
fn guardian_thorns_before_hurt(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
) {
    if !matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.guardian_state().is_some_and(|state| !state.moving))
        || source.damage_type.resource_id == "minecraft:thorns"
        || level.registry_tag_membership(
            "minecraft:damage_type",
            "minecraft:avoids_guardian_thorns",
            &source.damage_type.resource_id,
        )
    {
        return;
    }
    let Some(target_id) = source.direct_entity_id else {
        return;
    };
    if let Some(player) = level.player_by_entity_id(target_id) {
        let attacker = crate::level_event::PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        level.hurt_player(crate::level_event::PendingPlayerHurt {
            player: player.uuid,
            damage_type: "minecraft:thorns",
            amount: 2.0,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: false,
            impulse: None,
        });
    } else if store
        .get(target_id)
        .or_else(|| level.detached_entity(target_id))
        .is_some_and(|target| target.living().is_some())
    {
        store.queue_command(crate::entity::EntityCommand::GuardianThorns {
            guardian_id: entity.id(),
            target_id,
        });
    }
}

// Multi-arg Vanilla `LivingEntity.hurtServer` runtime dispatch with live hooks and snapshots.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hurt_live_entity(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    registry_tags: Option<&crate::entity::EntityRegistryTagView>,
    source: StoredEntityDamageSource<'_>,
    causing_entity_snapshot: Option<&crate::explosion::ExplosionSourceEntity>,
    direct_entity_snapshot: Option<&crate::explosion::ExplosionSourceEntity>,
    direct_resource_id: Option<&str>,
    damage: f32,
    mut rules: StoredEntityDamageRules,
    source_position: Option<Vec3d>,
    explicit_source_position: Option<Vec3d>,
    immunity_reader: Option<&mut crate::entity::enchanted_damage::DamageImmunityReader<'_>>,
) -> GeneralLivingHurtEffects {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return refused();
    };
    // Raider.java:252-257.
    if let Some(crate::entity::raider::RaiderRaid::Current(raid_id)) =
        data.raider_retained().map(|state| state.raid())
    {
        store.queue_command(crate::entity::EntityCommand::RaiderHurt { raid_id });
    }
    // Shulker.java:412-418: a closed shell refuses every AbstractArrow subtype.
    if matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.shulker_state().is_some_and(|state| state.is_closed()))
        && direct_is_abstract_arrow(store, level, source.direct_entity_id, direct_resource_id)
    {
        return refused();
    }
    guardian_thorns_before_hurt(entity, store, level, source);
    let is_wither = matches!(entity.data(), EntityData::GeneralLiving(data) if data.wither_retained().is_some());
    let is_bat =
        matches!(entity.data(), EntityData::GeneralLiving(data) if data.bat_retained().is_some());
    let is_enderman = matches!(entity.data(), EntityData::GeneralLiving(data) if data.enderman_retained().is_some());
    let is_shulker =
        matches!(entity.data(), EntityData::GeneralLiving(data) if data.shulker_state().is_some());
    let is_silverfish = entity.data().entity_resource_id() == "minecraft:silverfish";
    let potion = is_enderman
        .then(|| {
            source.direct_entity_id.and_then(|id| {
                store
                    .get(id)
                    .or_else(|| level.detached_entity(id))
                    .and_then(crate::entity::thrown_potion::potion_data)
                    .map(|potion| crate::entity::thrown_potion::is_water(potion.item()))
            })
        })
        .flatten();
    let dragon_health = crate::entity::ender_dragon::state(entity)
        .and(entity.living().map(|living| living.health()));
    let (mut tags, witch_damage_multiplier, dragon_damage, projectile) = {
        let mut membership = |registry: &str, tag: &str, resource: &str| match registry_tags {
            Some(tags) => tags.contains(registry, tag, resource),
            None => level.registry_tag_membership(registry, tag, resource),
        };
        let tags = resolved_damage_tags(entity, source, &mut membership);
        if tags.is_fall && !rules.fall_damage_immune {
            rules.fall_damage_immune = membership(
                "minecraft:entity_type",
                "minecraft:fall_damage_immune",
                entity.data().entity_resource_id(),
            );
        }
        let creative = source
            .causing_entity
            .is_some_and(StoredDamageCausingEntity::is_creative_player);
        // Creaking.java:152-185: heart-bound protectors absorb qualifying hits without
        // taking damage, play event 66 + ENTITY_ACTION, and notify their heart.
        if let Some(home_pos) = entity
            .creaking_home_pos()
            .filter(|_| !tags.bypasses_invulnerability)
        {
            let invulnerable_to_source = living::is_invulnerable_to_with_tags(
                entity,
                creative,
                rules.fall_damage_immune,
                tags.bypasses_invulnerability,
                tags.is_fire,
                tags.is_fall,
            );
            let causer = DamageCauser::resolve(source.causing_entity, |id| {
                if id == entity.id() {
                    Some(&*entity)
                } else {
                    store.get(id).or_else(|| level.detached_entity(id))
                }
            });
            let responsible_player = matches!(
                source.causing_entity,
                Some(StoredDamageCausingEntity::Player { .. })
            ) || causer
                .tame_wolf_owner
                .flatten()
                .is_some_and(|owner| level.player(owner).is_some());
            let direct_is_living_or_projectile = direct_resource_id == Some("minecraft:player")
                || direct_entity_snapshot.is_some_and(|snap| {
                    matches!(
                        snap.kind,
                        crate::explosion::ExplosionSourceEntityKind::Player { .. }
                    )
                })
                || source.direct_entity_id.is_some_and(|id| {
                    id == entity.id()
                        || level.player_by_entity_id(id).is_some()
                        || store
                            .get(id)
                            .or_else(|| level.detached_entity(id))
                            .is_some_and(|direct| {
                                direct.living().is_some() || direct.is_projectile_class()
                            })
                });
            match entity.creaking_hurt_decision(
                tags.bypasses_invulnerability,
                invulnerable_to_source,
                direct_is_living_or_projectile,
                responsible_player,
            ) {
                crate::entity::CreakingHurtDecision::PassThrough => {}
                crate::entity::CreakingHurtDecision::Refuse => return refused(),
                crate::entity::CreakingHurtDecision::Absorb => {
                    entity.begin_creaking_hurt_animation();
                    store.broadcast_to_tracking(
                        entity.id(),
                        crate::connection::ClientboundPlayPacket::EntityEvent(
                            bbb_protocol::packets::EntityEvent {
                                entity_id: entity.id(),
                                event_id: 66,
                            },
                        ),
                    );
                    level.game_event_at(
                        crate::level_event::VanillaGameEvent::ENTITY_ACTION,
                        entity.position(),
                        Some(entity.id()),
                    );
                    store.queue_command(crate::entity::EntityCommand::CreakingHeartHurt {
                        home_pos,
                        protector_uuid: entity.uuid(),
                        entity_id: entity.id(),
                        center: entity.bounding_box().center(),
                        responsible_player,
                    });
                    return GeneralLivingHurtEffects {
                        outcome: StoredEntityHurtOutcome {
                            applied: true,
                            succeeded: true,
                        },
                        emits_entity_damage: false,
                        game_event_source_id: None,
                        died: false,
                        perished: false,
                    };
                }
            }
        }
        if (is_wither || is_bat || is_enderman || is_silverfish)
            && living::is_invulnerable_to_with_tags(
                entity,
                creative,
                rules.fall_damage_immune,
                tags.bypasses_invulnerability,
                tags.is_fire,
                tags.is_fall,
            )
        {
            return refused();
        }
        // Silverfish.java:83-93.
        if is_silverfish
            && (source.causing_entity.is_some()
                || membership(
                    "minecraft:damage_type",
                    "minecraft:always_triggers_silverfish",
                    &source.damage_type.resource_id,
                ))
        {
            if let Some(runtime) = store.mob_ai_mut(entity.id()) {
                runtime.notify_silverfish_hurt(level.game_time());
            }
        }
        if is_wither
            && wither_refuses_hurt(
                entity,
                store,
                source,
                direct_resource_id,
                tags.bypasses_invulnerability,
                &mut membership,
            )
        {
            return refused();
        }
        let dragon_damage = if dragon_health.is_some() {
            let reaches = matches!(
                source.causing_entity,
                Some(StoredDamageCausingEntity::Player { .. })
            ) || membership(
                "minecraft:damage_type",
                "minecraft:always_hurts_ender_dragons",
                &source.damage_type.resource_id,
            );
            let arrow =
                direct_is_arrow_or_wind_charge(store, source.direct_entity_id, direct_resource_id);
            // AbstractDragonSittingPhase.java:19-26.
            if arrow
                && crate::entity::ender_dragon::state(entity)
                    .is_some_and(|state| state.is_sitting())
            {
                if let Some(projectile) = source.direct_entity_id.and_then(|id| store.get_mut(id)) {
                    projectile.ignite_for_seconds(1.0);
                }
            }
            match crate::entity::ender_dragon::hurt_damage(entity, damage, reaches, arrow) {
                Some(damage) => Some(damage),
                None => return refused(),
            }
        } else {
            None
        };
        let multiplier = if entity.data().entity_resource_id() == "minecraft:witch" {
            let own = source
                .causing_entity
                .is_some_and(|causing| causing.entity_id() == entity.id());
            let resistant = membership(
                "minecraft:damage_type",
                "minecraft:witch_resistant_to",
                &source.damage_type.resource_id,
            );
            (if own { 0.0 } else { 1.0 }) * (if resistant { 0.15 } else { 1.0 })
        } else {
            1.0
        };
        let projectile = (is_enderman || is_shulker)
            && membership(
                "minecraft:damage_type",
                "minecraft:is_projectile",
                &source.damage_type.resource_id,
            );
        (tags, multiplier, dragon_damage, projectile)
    };
    // EnderMan.java:352-381: only clean water reaches ordinary hurt in the projectile branch.
    if is_enderman && (projectile || potion.is_some()) && potion != Some(true) {
        let mut effects = refused();
        if teleport_enderman_after_projectile(entity, store, level) {
            effects.outcome = StoredEntityHurtOutcome {
                applied: true,
                succeeded: true,
            };
        }
        return effects;
    }
    let damage = dragon_damage.unwrap_or(damage);
    // `Ghast.isReflectedFireball` (`Ghast.java:76-108`): a large fireball a
    // player sent back skips the ghast's fire immunity and hits for 1000.
    let damage = if entity.entity_type_id()
        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GHAST_ID
        && direct_resource_id == Some("minecraft:fireball")
        && matches!(
            source.causing_entity,
            Some(StoredDamageCausingEntity::Player { .. })
        ) {
        tags.is_fire = false;
        1000.0
    } else {
        damage
    };
    crate::entity::axolotl::before_hurt(entity, level, source, damage);
    crate::entity::panda::before_hurt(entity);
    if is_bat {
        wake_bat(entity);
    }
    let causer = DamageCauser::resolve(source.causing_entity, |id| {
        if id == entity.id() {
            Some(&*entity)
        } else {
            store.get(id).or_else(|| level.detached_entity(id))
        }
    });
    let mut effects = hurt_server_with_tags(
        entity,
        source,
        damage,
        rules,
        tags,
        witch_damage_multiplier,
        causer,
        Some(&mut LiveHurtHooks {
            store: &mut *store,
            level: &mut *level,
            source,
            causing_entity_snapshot,
            direct_entity_snapshot,
            source_position,
            explicit_source_position,
            immunity_reader,
        }),
    );
    crate::entity::warden::anger::after_hurt(entity, store, level, source);
    if effects.died {
        record_player_kill(source, store, entity);
        effects.perished = killed_entity(source, store, level, entity);
    }
    if let Some(health_before) = dragon_health {
        crate::entity::ender_dragon::after_hurt(entity, store, health_before);
    }
    if effects.outcome.succeeded {
        if is_wither {
            if let Some(runtime) = store.mob_ai_mut(entity.id()) {
                runtime.accept_wither_hurt();
            }
        }
        let retreat_attacker = source
            .causing_entity
            .filter(|_| causer.living)
            .zip(causer.uuid)
            .map(|(cause, uuid)| crate::entity::ai::mob::target::TargetRef {
                entity_id: cause.entity_id(),
                uuid,
            });
        crate::entity::hoglin::was_hurt(entity, store, level, retreat_attacker);
        crate::entity::zombie::call_reinforcements(entity, store, level, source);
        crate::entity::piglin::was_hurt(entity, store, level, source, causer.living);
        crate::entity::ai::mob::target::retaliation::was_hurt(
            entity,
            store,
            level,
            source,
            causer.living,
        );
        crate::entity::shulker::after_hurt(
            entity,
            store,
            level,
            projectile && direct_resource_id == Some("minecraft:shulker_bullet"),
        );
    }
    crate::entity::living::mob_effects::on_mob_hurt(entity, store, effects.outcome.succeeded);
    apply_success_follow_up(entity, store, effects);
    if is_enderman {
        let teleported = if potion == Some(true) {
            teleport_enderman_after_projectile(entity, store, level)
        } else {
            !causer.living
                && level.level_random().next_int(10) != 0
                && crate::entity::enderman::random_teleport(entity, store, level).unwrap_or(false)
        };
        if teleported {
            effects.outcome.applied = true;
            if potion == Some(true) {
                effects.outcome.succeeded = true;
            }
        }
    }
    effects
}

fn teleport_enderman_after_projectile(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    (0..64).any(|_| crate::entity::enderman::random_teleport(entity, store, level).unwrap_or(false))
}

fn apply_success_follow_up(
    entity: &mut Entity,
    store: &mut EntityStore,
    effects: GeneralLivingHurtEffects,
) {
    if !effects.outcome.succeeded {
        return;
    }
    abstract_horse_hurt_follow_up(entity, store);
    crate::entity::ai::mob::squid::spawn_ink(entity, store);
    let glow_changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .ageable_water_retained_mut()
            .is_some_and(|state| state.set_glow_squid_dark_ticks(100)),
        _ => false,
    };
    if glow_changed {
        entity.mark_synched_dirty(VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID);
    }
}

#[cfg(test)]
fn wake_bat_before_hurt(
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    tags: Option<ResolvedDamageTags>,
) {
    let invulnerable = tags.map_or_else(
        || {
            living::is_invulnerable_to(
                entity,
                source.damage_type,
                source.causing_entity.is_some_and(|causing| {
                    matches!(
                        causing,
                        StoredDamageCausingEntity::Player {
                            instabuild: true,
                            ..
                        }
                    )
                }),
                rules.fall_damage_immune,
            )
        },
        |tags| {
            living::is_invulnerable_to_with_tags(
                entity,
                source.causing_entity.is_some_and(|causing| {
                    matches!(
                        causing,
                        StoredDamageCausingEntity::Player {
                            instabuild: true,
                            ..
                        }
                    )
                }),
                rules.fall_damage_immune,
                tags.bypasses_invulnerability,
                tags.is_fire,
                tags.is_fall,
            )
        },
    );
    if !matches!(entity.data(), EntityData::GeneralLiving(data) if data.bat_retained().is_some())
        || invulnerable
    {
        return;
    }
    wake_bat(entity);
}

fn wake_bat(entity: &mut Entity) {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .bat_retained_mut()
            .is_some_and(|bat| bat.set_resting(false)),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_BAT_FLAGS_DATA_ID);
    }
}

pub(crate) fn abstract_horse_hurt_follow_up(entity: &mut Entity, store: &mut EntityStore) {
    let can_rear = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_DONKEY_ID
        | VANILLA_ENTITY_TYPE_HORSE_ID
        | VANILLA_ENTITY_TYPE_MULE_ID
        | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => true,
        VANILLA_ENTITY_TYPE_CAMEL_ID
        | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        | VANILLA_ENTITY_TYPE_LLAMA_ID
        | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => false,
        _ => return,
    };
    let rear = store
        .mob_ai_mut(entity.id())
        .is_some_and(|runtime| runtime.next_random_int(3) == 0);
    if !rear || !can_rear {
        return;
    }
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .abstract_horse_retained_mut()
            .is_some_and(|horse| horse.set_standing(20)),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
}

fn remember_damage_source(
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    tags: ResolvedDamageTags,
    causer: DamageCauser,
) {
    blame_source_for_damage(entity, source, tags, causer);
    let Some(living) = entity.living_mut() else {
        return;
    };
    living.last_damage_type = Some(source.damage_type.resource_id.clone());
    living.last_damage_source_entity = source
        .causing_entity
        .map(StoredDamageCausingEntity::entity_id);
    living.last_damage_memory_time = LAST_DAMAGE_SOURCE_MEMORY_TICKS;
}

fn blame_source_for_damage(
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    tags: ResolvedDamageTags,
    causer: DamageCauser,
) {
    let tick_count = entity.tick_count();
    let Some(living) = entity.living_mut() else {
        return;
    };
    // `resolvePlayerResponsibleForDamage` (`:1337-1351`): a tame wolf's
    // owner takes the blame, a stray one clears it.
    let responsible = match source.causing_entity {
        Some(StoredDamageCausingEntity::Player { uuid, .. }) => Some(Some(uuid)),
        _ => causer.tame_wolf_owner,
    };
    if let Some(player) = responsible {
        living.last_hurt_by_player = player;
        living.last_hurt_by_player_memory_time = if player.is_some() {
            LAST_HURT_BY_PLAYER_MEMORY_TICKS
        } else {
            0
        };
    }
    // `LivingEntity.resolveMobResponsibleForDamage` (`:1329-1335`): any living
    // causing entity, held back by the two anger tags.
    if !tags.no_anger && !tags.no_anger_from_wind_charge {
        match source.causing_entity {
            Some(StoredDamageCausingEntity::Player { uuid, .. }) => {
                living.set_last_hurt_by_mob(Some(LastHurtByMob::Uuid(uuid)), tick_count);
            }
            Some(causing @ StoredDamageCausingEntity::Mob { .. }) => living.set_last_hurt_by_mob(
                Some(LastHurtByMob::Stored {
                    entity_id: causing.entity_id(),
                    uuid: causer.uuid,
                }),
                tick_count,
            ),
            Some(causing @ StoredDamageCausingEntity::Other { .. }) if causer.living => living
                .set_last_hurt_by_mob(
                    Some(LastHurtByMob::Stored {
                        entity_id: causing.entity_id(),
                        uuid: causer.uuid,
                    }),
                    tick_count,
                ),
            Some(StoredDamageCausingEntity::Other { .. }) | None => {}
        }
    }
}

/// The window `getLastDamageSource` answers a source inside
/// (`LivingEntity.java:1395`).
const LAST_DAMAGE_SOURCE_MEMORY_TICKS: i32 = 40;

#[derive(Debug, Clone, Copy, Default)]
struct ResolvedDamageTags {
    bypasses_invulnerability: bool,
    is_fire: bool,
    is_fall: bool,
    is_freezing: bool,
    damages_helmet: bool,
    bypasses_cooldown: bool,
    bypasses_armor: bool,
    bypasses_effects: bool,
    bypasses_resistance: bool,
    bypasses_enchantments: bool,
    no_impact: bool,
    no_knockback: bool,
    bypasses_wolf_armor: bool,
    freeze_hurts_extra_types: bool,
    no_anger: bool,
    /// The second half of that gate: a `minecraft:wind_charge` source whose
    /// target is in `EntityTypeTags.NO_ANGER_FROM_WIND_CHARGE`.
    no_anger_from_wind_charge: bool,
}

fn resolved_builtin_damage_tags(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
) -> ResolvedDamageTags {
    resolved_damage_tags(entity, source, |registry, tag, value| {
        builtin_registry_tag_membership(registry, tag, value)
    })
}

fn resolved_damage_tags(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
    mut membership: impl FnMut(&str, &str, &str) -> bool,
) -> ResolvedDamageTags {
    let damage = source.damage_type.resource_id.as_str();
    let is_freezing = resolve_tag(
        &mut membership,
        "minecraft:damage_type",
        "minecraft:is_freezing",
        damage,
    );
    let freeze_hurts_extra_types = if is_freezing {
        resolve_tag(
            &mut membership,
            "minecraft:entity_type",
            "minecraft:freeze_hurts_extra_types",
            entity.data().entity_resource_id(),
        )
    } else {
        false
    };
    let no_anger = membership("minecraft:damage_type", "minecraft:no_anger", damage);
    let no_anger_from_wind_charge = damage == "minecraft:wind_charge"
        && membership(
            "minecraft:entity_type",
            "minecraft:no_anger_from_wind_charge",
            entity.data().entity_resource_id(),
        );
    ResolvedDamageTags {
        bypasses_invulnerability: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:bypasses_invulnerability",
            damage,
        ),
        is_fire: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:is_fire",
            damage,
        ),
        is_fall: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:is_fall",
            damage,
        ),
        is_freezing,
        damages_helmet: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:damages_helmet",
            damage,
        ),
        bypasses_cooldown: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:bypasses_cooldown",
            damage,
        ),
        bypasses_armor: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:bypasses_armor",
            damage,
        ),
        bypasses_effects: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:bypasses_effects",
            damage,
        ),
        bypasses_resistance: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:bypasses_resistance",
            damage,
        ),
        bypasses_enchantments: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:bypasses_enchantments",
            damage,
        ),
        no_impact: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:no_impact",
            damage,
        ),
        no_knockback: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:no_knockback",
            damage,
        ),
        bypasses_wolf_armor: resolve_tag(
            &mut membership,
            "minecraft:damage_type",
            "minecraft:bypasses_wolf_armor",
            damage,
        ),
        freeze_hurts_extra_types,
        no_anger,
        no_anger_from_wind_charge,
    }
}

fn resolve_tag(
    membership: &mut impl FnMut(&str, &str, &str) -> bool,
    registry_id: &str,
    tag_id: &str,
    element_resource_id: &str,
) -> bool {
    membership(registry_id, tag_id, element_resource_id)
}

fn wolf_has_armor(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.entity_resource_id == "minecraft:wolf"
        && data
            .living
            .item_by_slot(EquipmentSlot::Body)
            .is_some_and(|stack| stack.item_resource_id() == Some("minecraft:wolf_armor"))
}

fn calls_living_actually_hurt(entity: &Entity, bypasses_wolf_armor: bool) -> bool {
    !wolf_has_armor(entity) || bypasses_wolf_armor
}

fn wither_refuses_hurt(
    entity: &Entity,
    store: &EntityStore,
    source: StoredEntityDamageSource<'_>,
    direct_resource_id: Option<&str>,
    bypasses_invulnerability: bool,
    membership: &mut impl FnMut(&str, &str, &str) -> bool,
) -> bool {
    let invulnerable_ticks = match entity.data() {
        EntityData::GeneralLiving(data) => data.wither_retained().map_or(
            0,
            crate::entity::wither::WitherRetainedState::invulnerable_ticks,
        ),
        _ => 0,
    };
    membership(
        "minecraft:damage_type",
        "minecraft:wither_immune_to",
        &source.damage_type.resource_id,
    ) || causing_is_wither(entity, store, source.causing_entity)
        || invulnerable_ticks > 0 && !bypasses_invulnerability
        || is_powered_wither(entity)
            && direct_is_arrow_or_wind_charge(store, source.direct_entity_id, direct_resource_id)
        || causing_is_wither_friend(entity, store, source.causing_entity, membership)
}

fn causing_is_wither(
    entity: &Entity,
    store: &EntityStore,
    causing: Option<StoredDamageCausingEntity>,
) -> bool {
    let id = match causing {
        None | Some(StoredDamageCausingEntity::Player { .. }) => return false,
        Some(causing) => causing.entity_id(),
    };
    id == entity.id()
        || store
            .get(id)
            .is_some_and(|source| source.data().entity_resource_id() == "minecraft:wither")
}

fn causing_is_wither_friend<F>(
    entity: &Entity,
    store: &EntityStore,
    causing: Option<StoredDamageCausingEntity>,
    membership: &mut F,
) -> bool
where
    F: FnMut(&str, &str, &str) -> bool,
{
    let resource_id = match causing {
        None => return false,
        Some(StoredDamageCausingEntity::Player { .. }) => "minecraft:player",
        Some(StoredDamageCausingEntity::Mob { entity_id })
        | Some(StoredDamageCausingEntity::Other { entity_id }) => {
            if entity_id == entity.id() {
                "minecraft:wither"
            } else {
                let Some(source) = store.get(entity_id) else {
                    return false;
                };
                source.data().entity_resource_id()
            }
        }
    };
    membership(
        "minecraft:entity_type",
        "minecraft:wither_friends",
        resource_id,
    )
}

fn direct_is_abstract_arrow(
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    direct_id: Option<i32>,
    direct_resource_id: Option<&str>,
) -> bool {
    let Some(direct_id) = direct_id else {
        return false;
    };
    store
        .get(direct_id)
        .or_else(|| level.detached_entity(direct_id))
        .map_or_else(
            || {
                matches!(
                    direct_resource_id,
                    Some("minecraft:arrow" | "minecraft:spectral_arrow" | "minecraft:trident")
                )
            },
            |direct| crate::entity::abstract_arrow::state(direct.data()).is_some(),
        )
}

fn direct_is_arrow_or_wind_charge(
    store: &EntityStore,
    direct_id: Option<i32>,
    direct_resource_id: Option<&str>,
) -> bool {
    let Some(direct_id) = direct_id else {
        return false;
    };
    store.get(direct_id).map_or_else(
        || {
            matches!(
                direct_resource_id,
                Some(
                    "minecraft:arrow"
                        | "minecraft:spectral_arrow"
                        | "minecraft:trident"
                        | "minecraft:wind_charge"
                )
            )
        },
        |direct| {
            matches!(
                direct.data(),
                EntityData::Arrow(_)
                    | EntityData::SpectralArrow(_)
                    | EntityData::Trident(_)
                    | EntityData::WindCharge(_)
            )
        },
    )
}

fn is_powered_wither(entity: &Entity) -> bool {
    entity
        .living()
        .is_some_and(|living| living.health() <= living.max_health() / 2.0)
}
