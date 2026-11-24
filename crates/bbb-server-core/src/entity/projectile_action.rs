//! Ordered entity side effects that require world-owned execution.

use crate::world::simulation_level::LevelCtx;
pub mod mob_effect;

use bbb_protocol::packets::{BlockPos, Vec3d};
use uuid::Uuid;

use crate::explosion::ExplosionSourceEntity;
use crate::level_event::{PendingHurtEntity, VanillaGameEvent};
use bbb_server_datapack::damage::DamageTypeFacts;

use crate::entity::damage::{
    StoredDamageCausingEntity, StoredEntityDamageRules, StoredEntityDamageSource,
};
use crate::entity::{
    EntityData, EntityRegistryTagView, EntityStore, StoredEntityDamageContext,
    StoredEntityHurtOutcome,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PendingProjectileHurt {
    pub target: PendingHurtEntity,
    pub damage_type: &'static str,
    pub amount: f32,
    pub causing: Option<PendingHurtEntity>,
    pub direct: Option<PendingHurtEntity>,
    pub direct_resource_id: &'static str,
    pub registry_tags: EntityRegistryTagView,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingProjectileIgniteHurt {
    pub hurt: PendingProjectileHurt,
    pub fire_ticks: i32,
    pub restore_fire_on_refusal: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingProjectileMobEffect {
    pub resource_id: &'static str,
    pub amplifier: i32,
    pub easy_duration_ticks: i32,
    pub normal_duration_ticks: i32,
    pub hard_duration_ticks: i32,
    pub source_owner: Option<Uuid>,
    pub projectile_entity_id: i32,
}

impl PendingProjectileMobEffect {
    pub(crate) fn duration_for(
        &self,
        difficulty: Option<bbb_protocol::difficulties::Difficulty>,
    ) -> i32 {
        use bbb_protocol::difficulties::Difficulty;
        match difficulty {
            Some(Difficulty::Peaceful | Difficulty::Easy) => self.easy_duration_ticks,
            Some(Difficulty::Hard) => self.hard_duration_ticks,
            _ => self.normal_duration_ticks,
        }
    }
}

/// `AbstractArrow.onHitEntity`'s hurt and everything its outcome decides
/// (`AbstractArrow.java:419-505`, `ThrownTrident.java:119-144`).
#[derive(Debug, Clone, PartialEq)]
pub struct PendingArrowHit {
    /// `hurt.amount` is filled in from [`Self::damage`] by the world.
    pub hurt: PendingProjectileHurt,
    pub damage: ArrowDamage,
    pub arrow_entity_id: i32,
    /// A burning arrow sets the target alight for five seconds, undone when
    /// the hurt is refused (`:458-460`, `:501`).
    pub ignites: bool,
    /// An enderman that took the hit ends the hit there (`:463-465`).
    pub enderman: bool,
    /// `setArrowCount(getArrowCount() + 1)` (`:469-471`).
    pub sticks: bool,
    /// `doPostHurtEffects`: the arrow's potion or the spectral glow.
    pub effects: Vec<crate::entity::living::mob_effects::ActiveMobEffect>,
    /// The owner whose `setLastHurtMob` names the target (`:453-455`).
    pub owner: Option<Uuid>,
    /// `playSound(soundEvent, 1.0F, pitch)` after a taken hit (`:496`).
    pub hit_sound: Option<(String, f32)>,
    pub discard_on_success: bool,
    pub refusal: ArrowRefusal,
    /// A piercing arrow keeps `piercedAndKilledEntities` (`:431-446`).
    pub piercing: bool,
    /// `ThrownTrident.onHitEntity` runs the post-attack on any entity and
    /// without a living attacker (`ThrownTrident.java:134-138`).
    pub trident: bool,
}

/// The damage `onHitEntity` deals before and after the weapon's
/// `EnchantmentHelper.modifyDamage` (`AbstractArrow.java:425-431`,
/// `ThrownTrident.java:121-126`), and the motion `doKnockback` pushes along.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowDamage {
    pub base: f32,
    /// An arrow scales by its speed and rounds up; a trident does neither.
    pub speed: Option<f32>,
    pub crit: bool,
    pub weapon: Option<bbb_server_types::item_stack::RuntimeItemStack>,
    pub movement: Vec3d,
}

/// The weapon an arrow or trident that landed in a block was loosed from, for
/// its `hit_block` enchantment effects.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingProjectileHitBlock {
    pub projectile_entity_id: i32,
    pub weapon: bbb_server_types::item_stack::RuntimeItemStack,
    pub owner: Option<Uuid>,
    /// `BlockPos.clampLocationWithin` of the hit location.
    pub position: Vec3d,
    pub block_state_id: i32,
}

/// What a refused arrow hurt does to the arrow.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowRefusal {
    /// The trident already bounced whatever the outcome.
    Ignore,
    /// `deflect(REVERSE)` and `scale(0.2)`; an arrow too slow to bounce drops
    /// its pickup item if it may be picked up and is discarded (`:500-509`).
    Bounce {
        pickup: Option<bbb_server_types::item_stack::RuntimeItemStack>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectileLandOrigin {
    Entity(Vec3d),
    Block(BlockPos),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingProjectileAction {
    Deflect {
        target_entity_id: i32,
        new_owner: Option<Uuid>,
        direction: Option<Vec3d>,
    },
    Hurt(PendingProjectileHurt),
    ArrowHit(Box<PendingArrowHit>),
    IgniteThenHurt(PendingProjectileIgniteHurt),
    HurtThenMobEffect {
        hurt: PendingProjectileHurt,
        effect: PendingProjectileMobEffect,
    },
    PlaceFire {
        pos: BlockPos,
    },
    Land {
        origin: ProjectileLandOrigin,
        source_entity_id: i32,
    },
    Discard {
        entity_id: i32,
    },
    Destroy {
        entity_id: i32,
        position: Vec3d,
    },
    HitBlockEffects(Box<PendingProjectileHitBlock>),
    /// `CriteriaTriggers.TARGET_BLOCK_HIT` (`TargetBlock.java:45-48`).
    TargetHit {
        player: Uuid,
        projectile: Box<crate::entity::Entity>,
        signal: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredHurtResult {
    Applied {
        outcome: StoredEntityHurtOutcome,
        effect_applied: bool,
    },
    DropNoop,
}

#[derive(Debug, Clone)]
pub struct StoredHurtRequest<'a> {
    pub target_entity_id: i32,
    pub damage_type_resource_id: &'a str,
    pub amount: f32,
    pub direct_resource_id: Option<&'a str>,
    pub registry_tags: &'a EntityRegistryTagView,
    pub causing_entity_snapshot: Option<ExplosionSourceEntity>,
    pub direct_entity_snapshot: Option<&'a ExplosionSourceEntity>,
    pub source_position: Option<Vec3d>,
    pub explicit_source_position: Option<Vec3d>,
}

struct NonlivingDamage {
    damage_type: DamageTypeFacts,
    rules: StoredEntityDamageRules,
    causing_is_ender_dragon: bool,
    causing_entity_snapshot: Option<ExplosionSourceEntity>,
}

fn stored_hurt_request(hurt: &PendingProjectileHurt) -> Option<StoredHurtRequest<'_>> {
    let PendingHurtEntity::Stored { entity_id } = hurt.target else {
        return None;
    };
    Some(StoredHurtRequest {
        direct_entity_snapshot: None,
        target_entity_id: entity_id,
        damage_type_resource_id: hurt.damage_type,
        amount: hurt.amount,
        direct_resource_id: hurt.direct.is_some().then_some(hurt.direct_resource_id),
        registry_tags: &hurt.registry_tags,
        causing_entity_snapshot: None,
        source_position: None,
        explicit_source_position: None,
    })
}

pub(crate) fn hurt_stored_entity(
    request: StoredHurtRequest<'_>,
    effect: Option<&PendingProjectileMobEffect>,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> StoredHurtResult {
    hurt_stored_entity_inner(
        request,
        effect,
        source,
        rules,
        context,
        (store, level, None),
    )
}

pub(crate) fn hurt_stored_entity_with_immunity_reader(
    request: StoredHurtRequest<'_>,
    effect: Option<&PendingProjectileMobEffect>,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    target: (
        &mut EntityStore,
        &mut LevelCtx<'_, '_>,
        &mut crate::entity::enchanted_damage::DamageImmunityReader<'_>,
    ),
) -> StoredHurtResult {
    hurt_stored_entity_inner(
        request,
        effect,
        source,
        rules,
        context,
        (target.0, target.1, Some(target.2)),
    )
}

#[cfg(test)]
pub(crate) fn hurt_projectile_to_stored(
    hurt: &PendingProjectileHurt,
    effect: Option<&PendingProjectileMobEffect>,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> StoredHurtResult {
    hurt_projectile_to_stored_with_causing_snapshot(
        (hurt, None),
        effect,
        source,
        rules,
        context,
        store,
        level,
    )
}

pub(crate) fn hurt_projectile_to_stored_with_causing_snapshot(
    hurt_source: (&PendingProjectileHurt, Option<ExplosionSourceEntity>),
    effect: Option<&PendingProjectileMobEffect>,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> StoredHurtResult {
    let (hurt, causing_entity_snapshot) = hurt_source;
    let Some(mut request) = stored_hurt_request(hurt) else {
        return StoredHurtResult::DropNoop;
    };
    request.causing_entity_snapshot = causing_entity_snapshot;
    hurt_stored_entity(request, effect, source, rules, context, store, level)
}

fn hurt_stored_entity_inner(
    request: StoredHurtRequest<'_>,
    effect: Option<&PendingProjectileMobEffect>,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    (store, level, reader): (
        &mut EntityStore,
        &mut LevelCtx<'_, '_>,
        Option<&mut crate::entity::enchanted_damage::DamageImmunityReader<'_>>,
    ),
) -> StoredHurtResult {
    let StoredHurtRequest {
        target_entity_id: entity_id,
        damage_type_resource_id,
        amount,
        direct_resource_id,
        registry_tags,
        causing_entity_snapshot,
        direct_entity_snapshot,
        source_position,
        explicit_source_position,
    } = request;
    if source.damage_type.resource_id != damage_type_resource_id {
        return StoredHurtResult::DropNoop;
    }
    let dragon_part = crate::entity::ender_dragon::part_by_id(store, entity_id);
    let entity_id = dragon_part.map_or(entity_id, |(dragon_id, _, _)| dragon_id);
    let causing_entity_snapshot = if store.get(entity_id).is_some_and(|target| {
        matches!(
            target.data(),
            EntityData::EndCrystal(_) | EntityData::TntMinecart(_)
        )
    }) {
        causing_entity_snapshot.or_else(|| {
            source.causing_entity.and_then(|causing| {
                crate::entity::damage::damage_causing_explosion_source_entity(
                    causing,
                    store,
                    context.attacker_may_build,
                    None,
                    None,
                )
            })
        })
    } else {
        causing_entity_snapshot
    };
    let effect_duration = effect.map(|effect| effect.duration_for(store.difficulty()));
    let Some(mut target) = store.detach(entity_id) else {
        return StoredHurtResult::DropNoop;
    };
    if let Some((_, part, _)) = dragon_part {
        crate::entity::ender_dragon::set_hit_part(&mut target, Some(part));
    }
    let position = target.position();
    let mut hanging_actions = None;
    let mut entity_damage_after_attach = false;
    let mut died = false;
    let mut perished = true;
    let mut emits_entity_die = false;
    let (outcome, emits_entity_damage, game_event_source_id, destroyed_drops, removal_before_drops) =
        match target.data() {
            EntityData::GeneralLiving(_) => {
                let effects = crate::entity::general_living::hurt_live_entity(
                    &mut target,
                    store,
                    level,
                    Some(registry_tags),
                    source,
                    causing_entity_snapshot.as_ref(),
                    direct_entity_snapshot,
                    direct_resource_id,
                    amount,
                    rules,
                    source_position,
                    explicit_source_position,
                    reader,
                );
                died = effects.died;
                perished = effects.perished;
                (
                    effects.outcome,
                    effects.emits_entity_damage,
                    effects.game_event_source_id,
                    Vec::new(),
                    false,
                )
            }
            EntityData::ArmorStand(_) => {
                let Some(decision) = crate::entity::armor_stand::hurt_decision_with_registry_tags(
                    &target,
                    source,
                    rules,
                    context,
                    registry_tags,
                ) else {
                    store.attach(target);
                    return StoredHurtResult::DropNoop;
                };
                let Some(effects) = crate::entity::armor_stand::hurt_server_with_decision(
                    &mut target,
                    store,
                    decision,
                    context.block_drops,
                ) else {
                    store.attach(target);
                    return StoredHurtResult::DropNoop;
                };
                emits_entity_die = effects.emits_entity_die;
                (
                    effects.outcome,
                    effects.emits_entity_damage,
                    source
                        .causing_entity
                        .map(StoredDamageCausingEntity::entity_id),
                    Vec::new(),
                    false,
                )
            }
            EntityData::ShulkerBullet(_) => {
                crate::entity::shulker_bullet::hurt_server(&mut target, store);
                entity_damage_after_attach = true;
                (
                    StoredEntityHurtOutcome {
                        applied: true,
                        succeeded: true,
                    },
                    true,
                    Some(entity_id),
                    Vec::new(),
                    false,
                )
            }
            _ => {
                let resolved = resolve_nonliving_hurt(
                    &target,
                    store,
                    registry_tags,
                    source,
                    rules,
                    causing_entity_snapshot,
                );
                let NonlivingDamage {
                    damage_type,
                    rules,
                    causing_is_ender_dragon,
                    causing_entity_snapshot,
                } = resolved;
                let resolved_source = StoredEntityDamageSource {
                    damage_type: &damage_type,
                    direct_entity_id: source.direct_entity_id,
                    causing_entity: source.causing_entity,
                    attacker_views: Default::default(),
                };
                if matches!(target.data(), EntityData::EndCrystal(_)) {
                    let decision = crate::entity::end_crystal::hurt_decision_with_causing_type(
                        &mut target,
                        resolved_source,
                        rules,
                        causing_is_ender_dragon,
                        causing_entity_snapshot,
                    );
                    if let Some(explosion) = decision.pending_explosion {
                        store.queue_command(crate::entity::EntityCommand::Explosion(explosion));
                    }
                    (decision.outcome, false, None, Vec::new(), false)
                } else if matches!(target.data(), EntityData::TntMinecart(_)) {
                    let (tnt_source, burning_arrow_speed_squared) =
                        crate::entity::damage::tnt_minecart_damage_source(
                            resolved_source,
                            causing_entity_snapshot,
                            store,
                        );
                    let minecart_source = crate::entity::MinecartHurtSource {
                        creative_player: source
                            .causing_entity
                            .is_some_and(StoredDamageCausingEntity::is_creative_player),
                        bypasses_invulnerability: damage_type.bypasses_invulnerability,
                        is_fire: damage_type.is_fire,
                        fire_immune: target.plugin_mob().is_some_and(|plugin| plugin.fire_immune)
                            || bbb_protocol::entity_types::vanilla_entity_type_is_fire_immune(
                                target.entity_type_id(),
                            ),
                        is_fall: damage_type.is_fall,
                        fall_damage_immune: rules.fall_damage_immune,
                        causing_entity_id: source
                            .causing_entity
                            .map(StoredDamageCausingEntity::entity_id),
                    };
                    let effects = crate::entity::minecart::hurt_tnt_server(
                        &mut target,
                        store,
                        amount,
                        minecart_source,
                        tnt_source,
                        burning_arrow_speed_squared,
                        rules.tnt_explodes,
                        rules.entity_drops,
                    );
                    emits_entity_die = effects.emits_entity_die;
                    (
                        StoredEntityHurtOutcome {
                            applied: effects.applied,
                            succeeded: effects.succeeded,
                        },
                        effects.emits_entity_damage,
                        effects.game_event_source_id,
                        effects.drops,
                        true,
                    )
                } else if matches!(
                    target.data(),
                    EntityData::Painting(_)
                        | EntityData::ItemFrame(_)
                        | EntityData::GlowItemFrame(_)
                        | EntityData::LeashKnot(_)
                ) {
                    let Some(effects) = crate::entity::hanging::hurt_server_with_post_actions(
                        &mut target,
                        store,
                        rules.entity_drops,
                        hanging_hurt_source(resolved_source, rules),
                    ) else {
                        store.attach(target);
                        return StoredHurtResult::DropNoop;
                    };
                    emits_entity_die = effects.emits_entity_die;
                    hanging_actions = Some(effects.actions);
                    (
                        StoredEntityHurtOutcome {
                            applied: effects.outcome,
                            succeeded: effects.outcome,
                        },
                        false,
                        None,
                        Vec::new(),
                        false,
                    )
                } else {
                    let causer = crate::entity::damage::DamageCauser::resolve(
                        resolved_source.causing_entity,
                        |id| {
                            if id == target.id() {
                                Some(&target)
                            } else {
                                store.get(id)
                            }
                        },
                    );
                    let mutation = match crate::entity::damage::hurt_entity(
                        &mut target,
                        resolved_source,
                        amount,
                        rules,
                        context,
                        level.item_stack_resolver(),
                        causer,
                    ) {
                        Ok(mutation) => mutation,
                        Err(_) => {
                            store.attach(target);
                            return StoredHurtResult::DropNoop;
                        }
                    };
                    emits_entity_die = mutation.emits_entity_die;
                    crate::entity::damage::queue_container_vehicle_anger(
                        store,
                        &target,
                        emits_entity_die,
                        resolved_source,
                        rules,
                    );
                    (
                        mutation.outcome,
                        mutation.emits_entity_damage,
                        mutation.game_event_source_id,
                        mutation.destroyed_drops,
                        mutation.removal_before_drops,
                    )
                }
            }
        };
    if emits_entity_damage && !entity_damage_after_attach {
        level.game_event_at(
            VanillaGameEvent::ENTITY_DAMAGE,
            position,
            game_event_source_id,
        );
    }
    let vehicle_destroy = removal_before_drops
        && target.removal_reason().is_some()
        && (crate::entity::boat::is_boat(target.data())
            || crate::entity::minecart::is_minecart(target.data()));
    if vehicle_destroy
        && store
            .publish_detached_vehicle_removal(&mut target, level)
            .is_none()
    {
        store.attach(target);
        return StoredHurtResult::DropNoop;
    }
    // After `VehicleEntity.hurtServer`'s `ENTITY_DAMAGE` (`:48`) and before
    // each destroy body's drops, which is where `kill` sits in them.
    if emits_entity_die {
        level.game_event_at(VanillaGameEvent::ENTITY_DIE, position, Some(entity_id));
    }
    let effect_applied = if outcome.succeeded
        && matches!(
            target.data(),
            EntityData::GeneralLiving(_) | EntityData::ArmorStand(_)
        )
        && effect_duration.is_some_and(|duration| duration > 0)
        && effect.is_some_and(|effect| {
            !crate::entity::living::mob_effects::fixed_add_effect_rejection(
                target.data().entity_resource_id(),
            ) && crate::entity::living::mob_effects::can_be_affected_with_registry_tag_view(
                registry_tags,
                target.data().entity_resource_id(),
                effect.resource_id,
            )
        }) {
        let Some(effect) = effect else {
            unreachable!("effect admission requires an effect")
        };
        let duration = effect_duration.expect("effect admission resolved a duration");
        let target_id = target.id();
        crate::entity::living::mob_effects::ActiveMobEffect::new(
            effect.resource_id,
            duration,
            effect.amplifier,
        )
        .and_then(|effect| {
            target
                .living_mut()
                .and_then(|living| living.add_active_effect_published(effect))
        })
        .inspect(|published| {
            crate::entity::living::mob_effects::send_effect_update(store, target_id, published);
        })
        .is_some()
    } else {
        false
    };
    if let Some(actions) = hanging_actions {
        store.attach(target);
        crate::entity::hanging::apply_hanging_post_actions_in_level(
            actions,
            store,
            level,
            source
                .causing_entity
                .map(StoredDamageCausingEntity::entity_id),
        );
    } else if vehicle_destroy {
        let drops = destroyed_drops
            .into_iter()
            .map(|stack| {
                crate::entity::named_entity_drop::finish_named_entity_drop(&target, stack, level)
            })
            .collect::<Option<Vec<_>>>();
        let _ = drops
            .and_then(|drops| crate::entity::damage::spawn_destroyed_drops(store, position, drops))
            .and_then(|()| {
                if level.entity_drops() {
                    store.finish_detached_vehicle_destroy(&mut target)
                } else {
                    Some(())
                }
            });
        store.attach(target);
    } else if removal_before_drops {
        store.attach(target);
        let _ = crate::entity::damage::spawn_destroyed_drops(store, position, destroyed_drops);
    } else {
        let _ = crate::entity::damage::spawn_destroyed_drops(store, position, destroyed_drops);
        store.attach(target);
    }
    if emits_entity_damage && entity_damage_after_attach {
        level.game_event_at(
            VanillaGameEvent::ENTITY_DAMAGE,
            position,
            game_event_source_id,
        );
    }
    if died {
        if let Some(mut victim) = store.detach(entity_id) {
            let _ = crate::entity::general_living::emit_entity_die(
                store,
                level,
                &mut victim,
                source,
                crate::entity::general_living::GeneralLivingHurtEffects {
                    outcome,
                    emits_entity_damage: false,
                    game_event_source_id,
                    died,
                    perished,
                },
            );
            store.attach(victim);
        }
    }
    StoredHurtResult::Applied {
        outcome,
        effect_applied,
    }
}

fn hanging_hurt_source(
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
) -> crate::entity::HangHurtSource {
    crate::entity::HangHurtSource {
        is_explosion: source.damage_type.is_explosion,
        is_fire: source.damage_type.is_fire,
        is_fall: source.damage_type.is_fall,
        fall_damage_immune: rules.fall_damage_immune,
        from_mob: source
            .causing_entity
            .is_some_and(StoredDamageCausingEntity::is_mob),
        mob_griefing: rules.mob_griefing,
        caused_by_entity: source.causing_entity.is_some(),
        creative_player: source.causing_entity.is_some_and(|causing| {
            matches!(
                causing,
                StoredDamageCausingEntity::Player {
                    instabuild: true,
                    ..
                }
            )
        }),
        bypasses_invulnerability: source.damage_type.bypasses_invulnerability,
    }
}

pub(crate) enum DetachedNonlivingHurt {
    Mutation(crate::entity::damage::EntityHurtMutation),
    Hanging(crate::entity::hanging::ResolvedHangingHurtEffects),
}

pub(crate) fn hurt_detached_nonliving_with_facts(
    target: &mut crate::entity::Entity,
    store: &mut EntityStore,
    registry_tags: &EntityRegistryTagView,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
    rules: StoredEntityDamageRules,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Option<DetachedNonlivingHurt> {
    let NonlivingDamage {
        damage_type, rules, ..
    } = resolve_nonliving_hurt(target, store, registry_tags, source, rules, None);
    let resolved_source = StoredEntityDamageSource {
        damage_type: &damage_type,
        ..source
    };
    if matches!(
        target.data(),
        EntityData::Painting(_)
            | EntityData::ItemFrame(_)
            | EntityData::GlowItemFrame(_)
            | EntityData::LeashKnot(_)
    ) {
        return crate::entity::hanging::hurt_server_with_post_actions(
            target,
            store,
            rules.entity_drops,
            hanging_hurt_source(resolved_source, rules),
        )
        .map(DetachedNonlivingHurt::Hanging);
    }
    Some(DetachedNonlivingHurt::Mutation(
        crate::entity::damage::hurt_entity(
            target,
            resolved_source,
            damage,
            rules,
            StoredEntityDamageContext::default(),
            items,
            Default::default(),
        )
        .ok()?,
    ))
}

fn resolve_nonliving_hurt(
    target: &crate::entity::Entity,
    store: &EntityStore,
    registry_tags: &EntityRegistryTagView,
    source: StoredEntityDamageSource<'_>,
    mut rules: StoredEntityDamageRules,
    causing_entity_snapshot: Option<ExplosionSourceEntity>,
) -> NonlivingDamage {
    let damage_type = resolved_damage_type_facts(registry_tags, source.damage_type);
    rules.fall_damage_immune |= damage_type.is_fall
        && registry_tags.contains(
            "minecraft:entity_type",
            "minecraft:fall_damage_immune",
            target.data().entity_resource_id(),
        );
    let causing_is_ender_dragon = causing_entity_snapshot
        .as_ref()
        .is_some_and(|causing| causing.entity_resource_id == "minecraft:ender_dragon")
        || causing_is_ender_dragon(source.causing_entity, store);
    NonlivingDamage {
        damage_type,
        rules,
        causing_is_ender_dragon,
        causing_entity_snapshot,
    }
}

fn resolved_damage_type_facts(
    registry_tags: &EntityRegistryTagView,
    damage_type: &DamageTypeFacts,
) -> DamageTypeFacts {
    let mut resolved = damage_type.clone();
    let resource_id = damage_type.resource_id.as_str();
    resolved.bypasses_invulnerability = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:bypasses_invulnerability",
        resource_id,
    );
    resolved.bypasses_cooldown = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:bypasses_cooldown",
        resource_id,
    );
    resolved.is_drowning = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:is_drowning",
        resource_id,
    );
    resolved.is_fall = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:is_fall",
        resource_id,
    );
    resolved.is_fire = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:is_fire",
        resource_id,
    );
    resolved.is_freezing = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:is_freezing",
        resource_id,
    );
    resolved.is_explosion = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:is_explosion",
        resource_id,
    );
    resolved.ignites_armor_stands = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:ignites_armor_stands",
        resource_id,
    );
    resolved.burns_armor_stands = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:burns_armor_stands",
        resource_id,
    );
    resolved.can_break_armor_stand = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:can_break_armor_stand",
        resource_id,
    );
    resolved.always_kills_armor_stands = registry_member(
        registry_tags,
        "minecraft:damage_type",
        "minecraft:always_kills_armor_stands",
        resource_id,
    );
    resolved
}

fn registry_member(
    registry_tags: &EntityRegistryTagView,
    registry_id: &str,
    tag_id: &str,
    resource_id: &str,
) -> bool {
    registry_tags.contains(registry_id, tag_id, resource_id)
}

fn causing_is_ender_dragon(
    causing: Option<StoredDamageCausingEntity>,
    store: &EntityStore,
) -> bool {
    let Some(
        causing @ (StoredDamageCausingEntity::Mob { .. } | StoredDamageCausingEntity::Other { .. }),
    ) = causing
    else {
        return false;
    };
    store
        .get(causing.entity_id())
        .is_some_and(|entity| entity.data().entity_resource_id() == "minecraft:ender_dragon")
}

/// The stored `DamageSource.getEntity()` runtime kind used by mob-griefing gates.
pub fn resolve_stored_damage_causing_entity(
    store: &EntityStore,
    entity_id: i32,
) -> Option<StoredDamageCausingEntity> {
    let source = store.get(entity_id)?;
    Some(
        if matches!(source.data(), EntityData::GeneralLiving(data) if data.is_mob()) {
            StoredDamageCausingEntity::Mob { entity_id }
        } else {
            StoredDamageCausingEntity::Other { entity_id }
        },
    )
}
