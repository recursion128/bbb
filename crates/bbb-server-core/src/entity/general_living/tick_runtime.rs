//! The owned tick body and the per-family steps it dispatches inline.

use super::environment_damage::hurt_from_environment_outcome;
use super::horse_runtime::{
    tick_abstract_horse_counters, tick_abstract_horse_post_ai, tick_abstract_horse_pre_ai,
};
use super::*;
use crate::world::simulation_level::LevelCtx;

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    crate::entity::game_event_listener::warden_before_super(entity, store, level);
    crate::entity::skeleton::before_super(entity, store, level);
    crate::entity::zombie::before_super(entity, store, level);
    entity.tick_creaking_animations();
    crate::entity::endermite::before_super(entity);
    crate::entity::strider::tick_sounds(entity, store);
    if !crate::entity::strider::tick_environment(entity, level) {
        return false;
    }
    tick_creeper_before_super(entity, store, level);
    let is_vex = entity.entity_type_id() == VANILLA_ENTITY_TYPE_VEX_ID;
    let is_dragon =
        entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID;
    if is_vex || is_dragon {
        entity.set_no_physics(true);
    }
    living::snap_first_sleep(entity);
    store.refile_detached(entity);
    let ageable_water = AgeableWaterCreatureRetainedState::new(entity.entity_type_id());
    let ghast_contact = crate::entity::ghast::contact(entity, level);
    let slime_contact = crate::entity::slime::fluid_contact(entity, level);
    if matches!(
        ghast_contact,
        Some(crate::entity::ghast::GhastFluidContact::Unknown)
    ) || matches!(
        slime_contact,
        Some(crate::entity::slime::SlimeFluidContact::Unknown)
    ) {
        return false;
    }
    let phantom_contact = if matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.phantom_retained().is_some())
    {
        let contact = crate::entity::phantom::flying_contact(entity, level);
        if contact == living::FlyingContact::Unknown {
            return false;
        }
        contact
    } else {
        living::FlyingContact::Air
    };
    let bee_contact = if matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.bee_retained().is_some())
    {
        let Some(contact) = fluid_contact(entity, level, FluidTag::Water).known() else {
            return false;
        };
        Some(contact)
    } else {
        None
    };
    let fox_contact = if matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.fox_retained().is_some() && !data.is_no_ai())
    {
        let contact = fluid_contact(entity, level, FluidTag::Water);
        if contact == FluidContact::Unknown {
            return false;
        }
        contact
    } else {
        FluidContact::Known(false)
    };
    let pre_water_air = (is_water_animal_entity_type(entity.entity_type_id())
        || ageable_water.is_some())
    .then(|| entity.air_supply());
    let pre_dry_air = matches!(
        entity.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
    )
    .then(|| entity.air_supply());
    if !living_base_tick_checked(entity, store, level, |entity, store, level, damage| {
        let (damage_type, amount) = match damage {
            LivingBaseTickDamage::OnFire { amount } => ("minecraft:on_fire", amount),
            LivingBaseTickDamage::Cactus { amount } => ("minecraft:cactus", amount),
            LivingBaseTickDamage::SweetBerryBush { amount } => {
                ("minecraft:sweet_berry_bush", amount)
            }
            LivingBaseTickDamage::InWall { amount } => ("minecraft:in_wall", amount),
            LivingBaseTickDamage::Freeze { amount } => ("minecraft:freeze", amount),
            LivingBaseTickDamage::OutOfWorld { amount } => ("minecraft:out_of_world", amount),
        };
        hurt_from_environment(entity, store, level, damage_type, amount)
    }) {
        return false;
    }
    if !crate::entity::wandering_trader::tick_drinking(entity, store, level) {
        return false;
    }
    crate::entity::ambient_sound::tick(entity, store, level);
    if is_dragon {
        crate::entity::ender_dragon::tick(entity, store, level);
        return living::refresh_dirty_attributes_with_level(entity, level, store).is_some();
    }
    let water_contact = pre_water_air.map(|air| (air, FluidContact::Known(entity.is_in_water())));
    if let Some((pre_tick_air, contact)) = water_contact.filter(|_| {
        is_water_animal_entity_type(entity.entity_type_id())
            || matches!(
                ageable_water,
                Some(
                    AgeableWaterCreatureRetainedState::Squid
                        | AgeableWaterCreatureRetainedState::GlowSquid { .. }
                )
            )
    }) {
        if !tick_water_animal_air(entity, store, level, pre_tick_air, contact) {
            return false;
        }
    }
    if let Some(pre_tick_air) = pre_dry_air {
        tick_dry_out(entity, store, level, pre_tick_air);
    }
    tick_bat_ambient_sound(entity, store);
    tick_abstract_horse_ambient_sound(entity, store);
    crate::entity::ghast::tick_ambient_sound(entity, store);
    crate::entity::blaze::tick_ambient_sound(entity, store);
    crate::entity::witch::tick_ambient_sound(entity, store);
    crate::entity::parrot_sounds::tick(entity, store, level);
    crate::entity::wandering_trader::tick_ambient_sound(entity, store);
    crate::entity::villager::tick_ambient_sound(entity, store);
    crate::entity::villager::tick(entity, level);
    living::tick_stuck_counts(entity);
    publish_equipment_changes(entity, store, level);
    if !living::tick_sleep(entity, Some(&mut *store), level) {
        return false;
    }
    let position_before_ai = entity.position();
    if entity.is_alive() {
        if let Some(runtime) = store.mob_ai_mut(entity.id()) {
            runtime.install_fox_target_goals(entity);
        }
        crate::entity::happy_ghast::before_ai_step(entity);
        crate::entity::guardian::ai_step(entity, level);
        // `Monster.aiStep` (`Monster.java:43-46`), `Mannequin.java:176-179`.
        if entity.living().is_some_and(|living| living.is_swinging())
            && (crate::entity::ai::mob::spec::is_instance_of(entity.entity_type_id(), "Monster")
                || entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID)
        {
            if let Some(living) = entity.living_mut() {
                living.update_swing_time();
            }
        }
        // `Monster.updateNoActionTime` (`Monster.java:49-53`); raiders count
        // unconditionally in `raider::ai_step`.
        let entity_type_id = entity.entity_type_id();
        if crate::entity::ai::mob::spec::is_instance_of(entity_type_id, "Monster")
            && !crate::entity::ai::mob::spec::is_instance_of(entity_type_id, "Raider")
        {
            let eye = crate::entity::containing_block(Vec3d {
                y: entity.position().y + f64::from(entity.eye_height()),
                ..entity.position()
            });
            if crate::entity::level::light_level_dependent_magic_value(level, eye)
                .is_some_and(|brightness| brightness > 0.5)
            {
                if let Some(living) = entity.living_mut() {
                    living.no_action_time = living.no_action_time.wrapping_add(2);
                }
            }
        }
        if !crate::entity::ai::mob::target::sense::before_ai_step(entity, store, level) {
            return false;
        }
        if !crate::entity::witch::before_ai_step(entity, store, level) {
            return false;
        }
        if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID {
            let live_target =
                crate::entity::ai::mob::target::sense::body_has_live_target(entity, store, level);
            crate::entity::fox::before_ai_step(entity, live_target);
        }
        tick_abstract_horse_pre_ai(entity, store);
        crate::entity::goat::before_ai_step(entity);
        if let Some((_, contact)) = water_contact {
            tick_abstract_fish_flop(entity, store, contact);
        }
        crate::entity::wither::before_ai_step(entity, store, level);
        crate::entity::blaze::before_ai_step(entity);
        if !crate::entity::creaking::before_ai_step(entity, store, level) {
            return false;
        }
        if entity.is_alive_with_health()
            && !matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai())
        {
            if !crate::entity::zombification::tick(entity, store, level) {
                return false;
            }
            if entity.removal_reason().is_some() {
                return true;
            }
            crate::entity::piglin::before_ai(entity, store, level);
        }
        // `isEffectiveAi` is false under a controlling player, whose client
        // moves the mount (`LivingEntity.java:3015-3018`, `:3061-3064`).
        if let Some(rider) = crate::entity::item_steering::player_controller(entity, store, level) {
            crate::entity::item_steering::tick_ridden(entity, &rider);
        } else {
            let phantom_handled =
                match crate::entity::phantom::ai_step(entity, store, level, phantom_contact) {
                    Ok(handled) => handled,
                    Err(()) => return false,
                };
            if !phantom_handled
                && !ai_runtime::ai_step(
                    entity,
                    store,
                    level,
                    water_contact.map(|(_, contact)| contact).or_else(|| {
                        matches!(entity.data(), EntityData::GeneralLiving(data) if data.guardian_state().is_some())
                            .then(|| FluidContact::Known(entity.is_in_water()))
                    }),
                    ghast_contact,
                    slime_contact,
                    bee_contact,
                )
            {
                return false;
            }
        }
        if !matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai()) {
            if let Some(plugin_behavior) = entity
                .plugin_mob()
                .and_then(|_| store.plugins().mob_entry(entity.entity_type_id()))
                .map(|entry| std::sync::Arc::clone(&entry.behavior))
            {
                plugin_behavior.on_tick(entity, store, level);
                if entity.removal_reason().is_some() {
                    return true;
                }
            }
        }
        if !living::apply_living_inside_block_effects_and_freezing(
            entity,
            store,
            level,
            hurt_from_environment_outcome,
        ) {
            return false;
        }
        if crate::entity::push::push_entities(entity, store, level).is_none() {
            return false;
        }
        // `LivingEntity.aiStep` (`LivingEntity.java:3099-3100`).
        if entity.is_sensitive_to_water()
            && (entity.is_in_water()
                || level.is_raining_at(entity.block_position())
                || level.is_raining_at(BlockPos {
                    y: entity.bounding_box().max_y.floor() as i32,
                    ..entity.block_position()
                }))
            && !hurt_from_environment(entity, store, level, "minecraft:drown", 1.0)
        {
            return false;
        }
        if !crate::entity::daylight_burn::ai_step(entity, store, level) {
            return false;
        }
        if !crate::entity::mob_pickup::ai_step(entity, store, level) {
            return false;
        }
        if !crate::entity::fish::pufferfish::tick(entity, store, level) {
            return false;
        }
        crate::entity::ai::mob::interact_with_door::tick(entity, store, level);
        if !crate::entity::copper_golem::tick(entity, store, level) {
            return false;
        }
        if entity.removal_reason().is_some() {
            return true;
        }
        crate::entity::sniffer::tick(entity, store, level);
        crate::entity::piglin::tick(entity, store, level);
        crate::entity::hoglin::update_activity_sound(entity, store, level);
        crate::entity::wolf::tick(entity, store, level);
        crate::entity::wither::after_ai_step(entity, store);
        crate::entity::endermite::after_ai_step(entity);
        crate::entity::skeleton_horse::after_ai_step(entity);
        crate::entity::tadpole::after_ai_step(entity, store);
        if !crate::entity::ravager::after_ai_step(entity, store, level) {
            return false;
        }
        if !tick_ageable_mob(entity, store, level) {
            return false;
        }
        if matches!(
            ageable_water,
            Some(
                AgeableWaterCreatureRetainedState::Squid
                    | AgeableWaterCreatureRetainedState::GlowSquid { .. }
            )
        ) {
            let contact = Some(FluidContact::Known(entity.is_in_water()));
            tick_squid_runtime(entity, store, contact);
        }
        tick_glow_squid_dark_ticks(entity);
        tick_animal(entity);
        crate::entity::gift_loot::chicken_ai_step(entity, store, level);
        crate::entity::fox::after_ai_step(entity, store);
        if !crate::entity::ai::mob::target::sense::after_ai_step(entity, store, level) {
            return false;
        }
        let Some(bee_rolling) =
            crate::entity::ai::mob::target::sense::bee_rolling(entity, store, level)
        else {
            return false;
        };
        crate::entity::bee::after_ai_step(entity, store, bee_rolling);
        tick_flapping_slow_fall(entity);
        if !crate::entity::turtle::after_ai_step(entity, level) {
            return false;
        }
        crate::entity::rabbit::after_ai_step(entity, store);
        tick_abstract_horse_post_ai(entity, store, level);
        crate::entity::wandering_trader::after_ai_step(entity, store);
        crate::entity::villager::after_ai_step(entity, store);
        // Allay.java:213-217: healing continues while NoAI is set.
        if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID
            && entity.is_alive_with_health()
            && entity.tick_count() % 10 == 0
        {
            let health = entity.living().expect("allay living state").health();
            living::set_health(entity, health + 1.0);
        }
        crate::entity::game_event_listener::allay_after_ai_step(entity, store, level);
    }
    crate::entity::happy_ghast::continuous_heal(entity, level);
    if matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_mob())
        && !crate::entity::phantom::body_rotation(entity)
    {
        let id = entity.id();
        let position = entity.position();
        let y_rot = entity.y_rot();
        if let (Some(runtime), Some(living)) = (store.mob_ai_mut(id), entity.living_mut()) {
            runtime.tick_body_rotation(position_before_ai, position, y_rot, living);
        }
    }
    crate::entity::endermite::after_body_rotation(entity);
    if entity.sleeping_pos().is_some() {
        entity.set_rot(entity.y_rot(), 0.0);
    }
    if living::refresh_dirty_attributes_with_level(entity, level, store).is_none() {
        return false;
    }
    if let Some(living) = entity.living_mut() {
        living::tick_impulse_grace(&mut living.fall_impulse.reset_grace_time);
    }
    if matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_mob())
        && !ai_runtime::update_control_flags(entity, store, level)
    {
        return false;
    }
    if crate::entity::camel::tick_pose(entity, store, level).is_none() {
        return false;
    }
    crate::entity::camel::tick_dash(entity, store);
    tick_abstract_horse_counters(entity);
    crate::entity::nautilus::tick(entity, store);
    if matches!(
        ageable_water,
        Some(AgeableWaterCreatureRetainedState::Dolphin { .. })
    ) {
        let contact = FluidContact::Known(entity.is_in_water());
        if !tick_dolphin(entity, store, level, contact) {
            return false;
        }
    }
    tick_bat_after_super(entity);
    if !crate::entity::snow_golem::tick(entity, store, level) {
        return false;
    }
    if is_vex {
        entity.set_no_physics(false);
        entity.set_no_gravity(true);
        let starves = match &mut entity.data {
            EntityData::GeneralLiving(data) => data
                .vex_retained_mut()
                .is_some_and(VexRetainedState::tick_limited_life),
            _ => false,
        };
        if starves && !hurt_from_environment(entity, store, level, "minecraft:starve", 1.0) {
            return false;
        }
    }
    if !crate::entity::strider::float_strider(entity, level) {
        return false;
    }
    crate::entity::panda::tick(entity, store, level);
    crate::entity::ai::mob::cat::tick(entity, store, level);
    let has_target = store
        .mob_ai(entity.id())
        .is_some_and(crate::entity::ai::mob::MobAiRuntime::has_target);
    crate::entity::fox::after_super(entity, fox_contact, has_target, level.is_thundering());
    crate::entity::fox::tick_faceplant_effect(entity, store, level);
    crate::entity::bee::after_super(entity, store);
    crate::entity::slime::after_super(entity, store);
    if !crate::entity::ai::mob::target::sense::after_tick(entity, store, level) {
        return false;
    }
    let leash_holder_flipped = match &mut entity.data {
        EntityData::GeneralLiving(data) => data.tick_leash_holder(),
        _ => false,
    };
    if leash_holder_flipped {
        entity.mark_synched_dirty(
            bbb_protocol::entity_synched_data::VANILLA_HAPPY_GHAST_IS_LEASH_HOLDER_DATA_ID,
        );
    }
    crate::entity::shulker::tick(entity, store, level);
    crate::entity::game_event_listener::allay_after_super(entity, store, level);
    crate::entity::happy_ghast::after_super(entity, store)
}

fn tick_creeper_before_super(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if !entity.is_alive_with_health() {
        return;
    }
    let entity_id = entity.id();
    let position = entity.position();
    let block_position = entity.block_position();
    let silent = entity.is_silent();
    let (direction_changed, prime, explode, powered, explosion_radius) = {
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            return;
        };
        let Some(creeper) = data.creeper_retained_mut() else {
            return;
        };
        let (direction_changed, prime) = creeper.tick_before_super();
        (
            direction_changed,
            prime,
            creeper.swell() >= creeper.max_swell(),
            creeper.powered(),
            creeper.explosion_radius(),
        )
    };
    if direction_changed {
        entity.mark_synched_dirty(VANILLA_CREEPER_SWELL_DIR_DATA_ID);
    }
    if prime {
        if !silent {
            store.play_entity_sound(PendingEntitySound {
                position,
                resource_id: "minecraft:entity.creeper.primed".to_string(),
                source: SoundSource::Hostile,
                volume: 1.0,
                pitch: 0.5,
            });
        }
        level.game_event(
            VanillaGameEvent::PRIME_FUSE,
            block_position,
            Some(entity_id),
        );
    }
    if !explode {
        return;
    }

    if let Some(living) = entity.living_mut() {
        living.dead = true;
    }
    let direct = crate::entity::damage::explosion_source_entity(entity);
    store.queue_command(crate::entity::EntityCommand::Explosion(PendingExplosion {
        source: Some(ExplosionSource {
            indirect: Some(direct.clone()),
            causing: Some(direct.clone()),
            direct,
            in_water: entity.was_touching_water(),
            wind_charge: false,
        }),
        center: position,
        radius: explosion_radius as f32 * if powered { 2.0 } else { 1.0 },
        fire: false,
        interaction: ExplosionInteraction::Mob,
        damage_source: None,
        damage_profile: ExplosionDamageProfile::Default,
        presentation: ExplosionPresentation::Default,
    }));
    if powered {
        store.add_charged_creeper_blast(entity.id());
    }
    spawn_lingering_cloud(entity, store);
    // Creeper.java:236-237: the on-death effects fire as `KILLED` before the
    // discard, which then finds no effect left.
    entity.set_removed(crate::entity::RemovalReason::Killed);
}

/// `Creeper.spawnLingeringCloud` (Creeper.java:241-258).
fn spawn_lingering_cloud(entity: &Entity, store: &mut EntityStore) {
    let effects: Vec<bbb_protocol::packets::MobEffectInstanceSummary> =
        entity.living().map_or_else(Vec::new, |living| {
            living
                .active_effects()
                .map(crate::entity::living::ActiveMobEffect::to_summary)
                .collect()
        });
    if effects.is_empty() {
        return;
    }
    let cloud = crate::entity::AreaEffectCloudData::new()
        .with_radius(2.5)
        .with_radius_on_use(-0.5)
        .with_wait_time(10)
        .with_duration(300)
        .with_potion_duration_scale(0.25)
        .with_radius_per_tick(-2.5 / 300.0)
        .with_potion_contents(Some(bbb_protocol::packets::PotionContentsSummary {
            custom_effects: effects,
            ..Default::default()
        }));
    let _ = crate::entity::spawn_area_effect_cloud(store, entity.position(), cloud);
}

fn tick_water_animal_air(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pre_tick_air: i32,
    contact: FluidContact,
) -> bool {
    let alive = entity.is_alive() && entity.living().is_some_and(|living| living.health() > 0.0);
    let FluidContact::Known(in_water) = contact else {
        if alive {
            entity.set_air_supply(pre_tick_air);
        } else {
            entity.set_air_supply(300);
        }
        return false;
    };
    if !alive || in_water {
        entity.set_air_supply(300);
        return true;
    }
    let air = pre_tick_air.wrapping_sub(1);
    if air <= -20 {
        entity.set_air_supply(0);
        let _ = hurt_from_environment(entity, store, level, "minecraft:drown", 2.0);
    } else {
        entity.set_air_supply(air);
    }
    true
}

// Axolotl.java:181-186,253-262 (rain also wets); Nautilus.java:116-135.
fn tick_dry_out(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pre_tick_air: i32,
) {
    if matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai()) {
        return;
    }
    let wet = entity.is_in_water()
        || (entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID
            && entity.is_in_rain(level));
    if !entity.is_alive() || wet {
        entity.set_air_supply(crate::entity::living::max_air_supply(entity));
        return;
    }
    let air = pre_tick_air.wrapping_sub(1);
    if air <= -20 {
        entity.set_air_supply(0);
        let _ = hurt_from_environment(entity, store, level, "minecraft:dry_out", 2.0);
    } else {
        entity.set_air_supply(air);
    }
}

fn tick_glow_squid_dark_ticks(entity: &mut Entity) {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .ageable_water_retained_mut()
            .is_some_and(AgeableWaterCreatureRetainedState::tick_glow_squid),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID);
    }
}

fn tick_squid_runtime(entity: &mut Entity, store: &mut EntityStore, contact: Option<FluidContact>) {
    let id = entity.id();
    let broadcast_reset = store
        .mob_ai_mut(id)
        .and_then(|runtime| runtime.tick_squid(entity, contact.and_then(FluidContact::known)))
        .unwrap_or(false);
    if broadcast_reset {
        store.broadcast_to_tracking(
            id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: id,
                event_id: 19,
            }),
        );
    }
}

fn tick_dolphin(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    contact: FluidContact,
) -> bool {
    let no_ai = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai());
    if no_ai {
        entity.set_air_supply(4800);
        return true;
    }
    let block_pos = entity.block_position();
    let rain_at_feet = level.is_raining_at(block_pos);
    let rain_at_top = level.is_raining_at(BlockPos {
        y: entity.bounding_box().max_y.floor() as i32,
        ..block_pos
    });
    let wet = match contact {
        FluidContact::Known(in_water) => Some(in_water || rain_at_feet || rain_at_top),
        FluidContact::Unknown if rain_at_feet || rain_at_top => Some(true),
        FluidContact::Unknown => None,
    };
    let Some(wet) = wet else {
        return false;
    };
    let Some((_, moisture)) = (match &entity.data {
        EntityData::GeneralLiving(data) => data
            .ageable_water_retained()
            .and_then(AgeableWaterCreatureRetainedState::dolphin_fields),
        _ => None,
    }) else {
        return true;
    };
    let next = if wet { 2400 } else { moisture.wrapping_sub(1) };
    let dry_damage = !wet && next <= 0;
    let moisture_changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .ageable_water_retained_mut()
            .is_some_and(|state| state.set_dolphin_moistness(next)),
        _ => false,
    };
    if moisture_changed {
        entity.mark_synched_dirty(VANILLA_DOLPHIN_MOISTNESS_DATA_ID);
    }
    if dry_damage {
        let _ = hurt_from_environment(entity, store, level, "minecraft:dry_out", 1.0);
    }
    if wet || !entity.on_ground() {
        return true;
    }
    let Some((x_draw, z_draw, yaw_draw)) = store
        .mob_ai_mut(entity.id())
        .map(crate::entity::ai::mob::MobAiRuntime::dolphin_dry_jump_draws)
    else {
        return true;
    };
    let movement = entity.delta_movement();
    entity.set_delta_movement(Vec3d {
        x: movement.x + f64::from((x_draw * 2.0 - 1.0) * 0.2),
        y: movement.y + 0.5,
        z: movement.z + f64::from((z_draw * 2.0 - 1.0) * 0.2),
    });
    entity.set_rot(yaw_draw * 360.0, entity.x_rot());
    entity.on_ground = false;
    true
}

fn tick_ageable_mob(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if !entity.is_alive() {
        return true;
    }
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(ageable) = data.ageable_retained_mut() {
            ageable.tick_age_lock_particle_timer();
        }
    }
    if entity
        .living()
        .is_none_or(LivingEntityState::is_dead_or_dying)
    {
        return true;
    }
    let baby_changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let resource_id = data.entity_resource_id();
            let baby = data.is_baby();
            let Some(ageable) = data.ageable_retained_mut() else {
                return true;
            };
            ageable.tick(
                baby && !matches!(
                    resource_id,
                    "minecraft:skeleton_horse" | "minecraft:zombie_horse"
                ),
            )
        }
        _ => false,
    };
    if baby_changed {
        let previous = [entity.bb_width(), entity.bb_height()];
        publish_baby_change(entity);
        if living::dimensions_growth::resolve(entity, previous, level).is_none() {
            return false;
        }
        crate::entity::goat::age_boundary_reached(entity);
        crate::entity::gift_loot::turtle_age_boundary_reached(entity, store, level);
        // AgeableMob.java:230-234.
        if matches!(entity.data(), EntityData::GeneralLiving(data) if !data.is_baby()) {
            let too_wide = store.vehicle(entity.id()).is_some_and(|vehicle_id| {
                crate::entity::store::EntityBodyReadView::new(
                    store,
                    Some(entity),
                    level.detached_bodies(),
                )
                .get(vehicle_id)
                .is_some_and(|vehicle| {
                    crate::entity::boat::is_boat(vehicle.data())
                        && entity.bb_width() >= vehicle.bb_width()
                })
            });
            if too_wide {
                crate::entity::passenger::stop_riding_detached(entity, store, level);
            }
        }
    }
    true
}

// `Parrot.calculateFlapping` and `Chicken.aiStep` (`Chicken.java:124-127`).
fn tick_flapping_slow_fall(entity: &mut Entity) {
    if !matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_PARROT_ID | VANILLA_ENTITY_TYPE_CHICKEN_ID
    ) || entity.on_ground()
    {
        return;
    }
    let movement = entity.delta_movement();
    if movement.y < 0.0 {
        entity.set_delta_movement(Vec3d {
            y: movement.y * 0.6,
            ..movement
        });
    }
}

fn tick_animal(entity: &mut Entity) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(age) = data.ageable.map(AgeableMobRetainedState::age) else {
        return false;
    };
    data.animal
        .as_mut()
        .is_some_and(|animal| animal.ai_step(age))
}

fn tick_abstract_fish_flop(entity: &mut Entity, store: &mut EntityStore, contact: FluidContact) {
    if !is_water_animal_entity_type(entity.entity_type_id())
        || contact != FluidContact::Known(false)
        || !entity.on_ground()
        || !entity.vertical_collision()
    {
        return;
    }
    let Some((x_draw, z_draw)) = store
        .mob_ai_mut(entity.id())
        .map(crate::entity::ai::mob::MobAiRuntime::abstract_fish_flop_draws)
    else {
        return;
    };
    let movement = entity.delta_movement();
    entity.set_delta_movement(Vec3d {
        x: movement.x + f64::from((x_draw * 2.0 - 1.0) * 0.05),
        y: movement.y + 0.4,
        z: movement.z + f64::from((z_draw * 2.0 - 1.0) * 0.05),
    });
    entity.on_ground = false;
    entity.set_needs_sync(true);

    let Some(resource_id) = abstract_fish_flop_sound(entity.entity_type_id()) else {
        return;
    };
    let Some(pitch) = store
        .mob_ai_mut(entity.id())
        .map(|runtime| runtime.voice_pitch(false))
    else {
        return;
    };
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: resource_id.to_string(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch,
        });
    }
}

const fn abstract_fish_flop_sound(entity_type_id: i32) -> Option<&'static str> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_COD_ID => Some("minecraft:entity.cod.flop"),
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => Some("minecraft:entity.puffer_fish.flop"),
        VANILLA_ENTITY_TYPE_SALMON_ID => Some("minecraft:entity.salmon.flop"),
        VANILLA_ENTITY_TYPE_TADPOLE_ID => Some("minecraft:entity.tadpole.flop"),
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => Some("minecraft:entity.tropical_fish.flop"),
        _ => None,
    }
}
