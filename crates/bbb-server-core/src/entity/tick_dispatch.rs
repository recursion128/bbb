use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;

#[cfg(test)]
use super::continue_entity_tick_after_portal_refusal;
use super::{
    abstract_arrow, area_effect_cloud, armor_stand, boat, display, egg, end_crystal, ender_pearl,
    entity_base_tick, evoker_fangs, experience_bottle, experience_orb, eye_of_ender, falling_block,
    firework_rocket, fishing_bobber, general_living, hanging, hurting_projectile, inside_blocks,
    interaction, item, lightning_bolt, llama_spit, marker, minecart, ominous_item_spawner,
    passenger, portal, shulker_bullet, snowball, thrown_potion, tnt, Entity, EntityData,
    EntityStore, EntityTickOutcome,
};
use crate::world::simulation_level::LevelCtx;

///
/// Vanilla walks `entityTickList`, an insertion-ordered map; this walks the
/// store's ids ascending, which is the same order for entities added in id
/// order and is deterministic for every other case, which an insertion-ordered
/// hash map is not once entities are removed and re-added.
///
///
/// The vehicle/passenger dispatch (`:419-426`) and recursive passenger tick
/// (`:810-839`) are [`passenger`]. The entity-ticking-range test (`:418`) is
/// kept, and the level answers it -- see [`LevelCtx::is_entity_ticking`].
/// Its other half, `entity instanceof ServerPlayer`, needs no arm here for the
/// same reason.
///
#[cfg(test)]
pub(crate) fn tick_entities(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    runs_normally: bool,
) {
    store.set_game_time(level.game_time());
    if !runs_normally {
        return;
    }
    for id in store.ticking_ids() {
        let mut outcome = tick_one_entity(store, level, id);
        while let EntityTickOutcome::Portal(pending) = outcome {
            let requester_entity_id = pending.requester_entity_id();
            let continuation = pending.finish_stayed(store);
            outcome = continue_entity_tick_after_portal_refusal(
                store,
                level,
                continuation,
                requester_entity_id,
            );
        }
    }
}

/// One iteration of [`tick_entities`]' loop, so the world driver can walk the
/// list one entity at a time and drain the explosions each queued between them
/// (`PrimedTnt.explode` runs inline in Vanilla's loop). The snapshot of ids
/// still belongs to the caller, so an entity a tick spawns is not walked until
/// the next tick, which is Vanilla's own behaviour.
#[cfg(test)]
pub(crate) fn tick_one_entity(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    id: i32,
) -> EntityTickOutcome {
    passenger::tick_root(store, level, id)
}

pub(crate) fn tick_one_entity_checked(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    id: i32,
) -> EntityTickOutcome {
    passenger::tick_root_checked(store, level, id)
}

pub(crate) fn tick_member(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    id: i32,
    outer_root: bool,
    request_portal_transition: bool,
    world_callbacks: bool,
) -> Result<
    (
        bool,
        Option<portal::PortalTransitionRequest>,
        Option<inside_blocks::NonlivingInsidePause>,
    ),
    (),
> {
    // Freeze the structural authority fact while the vehicle is still in the
    // store; its own tick temporarily detaches it below.
    let has_passengers = !store.passengers(id).is_empty();
    let external_player_controls_direct_boat =
        has_passengers && store.first_passenger_is_external_player(id);
    let has_external_player_passenger = has_passengers && store.has_external_player_passenger(id);
    let Some(mut entity) = store.detach(id) else {
        return Ok((false, None, None));
    };
    if outer_root && entity.is_alive() {
        match entity.data() {
            EntityData::GeneralLiving(_) => {
                general_living::check_despawn(&mut entity, store, level)
            }
            EntityData::ShulkerBullet(_) => {
                shulker_bullet::check_despawn(&mut entity, store.difficulty())
            }
            _ => {}
        }
    }
    if !entity.is_alive() {
        store.attach(entity);
        return Ok((false, None, None));
    }
    let block_position = entity.block_position();
    let ticked =
        !outer_root || level.is_entity_ticking(block_position.x >> 4, block_position.z >> 4);
    if !ticked {
        store.attach(entity);
        return Ok((false, None, None));
    }
    if outer_root && store.stop_invalid_riding(id) {
        entity.set_boarding_cooldown(60);
    }
    entity.set_old_pos_and_rot();
    entity.tick_count += 1;
    let is_passenger = store.vehicle(id).is_some();
    let portal_allowed = match entity.data() {
        EntityData::Snowball(_)
        | EntityData::Egg(_)
        | EntityData::EnderPearl(_)
        | EntityData::ExperienceBottle(_)
        | EntityData::SplashPotion(_)
        | EntityData::LingeringPotion(_) => true,
        EntityData::FishingBobber(_) => false,
        // WitherBoss.java:553-555, EnderDragon.java:845-847, Creaking.java:348-350,
        // LivingEntity.java:3686-3688.
        _ => {
            !is_passenger
                && !matches!(
                    entity.entity_type_id,
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_ID
                        | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
                )
                && !entity.is_heart_bound()
                && entity.sleeping_pos().is_none()
        }
    };
    level.begin_portal_tick(Some((
        portal_allowed,
        store.dimension_changing_delay(&entity),
        request_portal_transition,
    )));
    let mut contact = None;
    let outcome = if world_callbacks && matches!(entity.data(), EntityData::EndCrystal(_)) {
        end_crystal::tick_pausing(&mut entity, store, level).map(|paused| {
            contact = paused;
            None
        })
    } else {
        tick_entity(
            &mut entity,
            store,
            level,
            external_player_controls_direct_boat,
            has_external_player_passenger,
        )
    };
    let portal_request = level.take_portal_request();
    let post_tick_item = match outcome {
        Ok(post_tick_item) => post_tick_item,
        Err(()) if portal_request.is_some() => None,
        Err(()) => {
            store.attach(entity);
            return Err(());
        }
    };
    match post_tick_item {
        Some(EntityTickPostAction::Hanging(actions)) => {
            store.attach(entity);
            hanging::apply_hanging_post_actions_in_level(actions, store, level, None);
        }
        post_tick_item => {
            store.attach(entity);
            match post_tick_item {
                Some(EntityTickPostAction::SpawnItem(position, item)) => {
                    let _ = item::spawn_item_entity(store, position, item);
                }
                Some(EntityTickPostAction::EjectBoatPassengersAfterMove) => {
                    store.eject_passengers_reverse(id);
                }
                _ => {}
            }
        }
    }
    Ok((ticked, portal_request, contact))
}

enum EntityTickPostAction {
    SpawnItem(Vec3d, RuntimeItemStack),
    Hanging(hanging::ResolvedHangingPostActions),
    EjectBoatPassengersAfterMove,
}

fn tick_entity(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    external_player_controls_direct_boat: bool,
    has_external_player_passenger: bool,
) -> Result<Option<EntityTickPostAction>, ()> {
    match entity.data {
        EntityData::Player | EntityData::EnderDragonPart => return Ok(None),
        EntityData::GeneralLiving(_) => {
            return general_living::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::FallingBlock(_) => {
            return falling_block::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::Item(_) => return item::tick(entity, store, level).then_some(None).ok_or(()),
        EntityData::ExperienceOrb(_) => {
            return experience_orb::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::Tnt(_) => return tnt::tick(entity, store, level).then_some(None).ok_or(()),
        EntityData::Painting(_)
        | EntityData::ItemFrame(_)
        | EntityData::GlowItemFrame(_)
        | EntityData::LeashKnot(_) => {
            let actions = hanging::tick(entity, store, level).ok_or(())?;
            return Ok((!actions.is_empty()).then_some(EntityTickPostAction::Hanging(actions)));
        }
        EntityData::ArmorStand(_) => {
            return armor_stand::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::Marker(_) => marker::tick(entity),
        EntityData::Interaction(_) => interaction::tick(entity),
        EntityData::BlockDisplay(_) | EntityData::ItemDisplay(_) | EntityData::TextDisplay(_) => {
            display::tick(entity)
        }
        EntityData::Arrow(_) | EntityData::SpectralArrow(_) | EntityData::Trident(_) => {
            return abstract_arrow::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::FishingBobber(_) => {
            return fishing_bobber::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::Snowball(_) => {
            return snowball::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::Egg(_) => return egg::tick(entity, store, level).then_some(None).ok_or(()),
        EntityData::EnderPearl(_) => {
            return ender_pearl::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::EndCrystal(_) => {
            return end_crystal::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::EyeOfEnder(_) => {
            return eye_of_ender::tick(entity, store, level).map(|drop| {
                drop.map(|(position, item)| EntityTickPostAction::SpawnItem(position, item))
            })
        }
        EntityData::ExperienceBottle(_) => {
            return experience_bottle::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::FireworkRocket(_) => {
            return firework_rocket::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::LlamaSpit(_) => {
            return llama_spit::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::EvokerFangs(_) => {
            return evoker_fangs::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::LightningBolt(_) => {
            return lightning_bolt::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::LargeFireball(_)
        | EntityData::SmallFireball(_)
        | EntityData::DragonFireball(_)
        | EntityData::WitherSkull(_)
        | EntityData::WindCharge(_)
        | EntityData::BreezeWindCharge(_) => {
            return hurting_projectile::tick_entity(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::AreaEffectCloud(_) => {
            return area_effect_cloud::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::OminousItemSpawner(_) => {
            return ominous_item_spawner::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::SplashPotion(_) | EntityData::LingeringPotion(_) => {
            return thrown_potion::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::ShulkerBullet(_) => {
            return shulker_bullet::tick(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
        EntityData::Boat(_) | EntityData::ChestBoat(_) => {
            return boat::tick(
                entity,
                store,
                level,
                external_player_controls_direct_boat,
                has_external_player_passenger,
            )
            .map(|outcome| match outcome.ejection {
                boat::BoatEjectionOutcome::None | boat::BoatEjectionOutcome::TimeoutBeforeMove => {
                    None
                }
                boat::BoatEjectionOutcome::BubbleAfterMove
                | boat::BoatEjectionOutcome::TimeoutBeforeAndBubbleAfter => {
                    Some(EntityTickPostAction::EjectBoatPassengersAfterMove)
                }
            })
            .ok_or(());
        }
        EntityData::Minecart(_)
        | EntityData::FurnaceMinecart(_)
        | EntityData::ChestMinecart(_)
        | EntityData::HopperMinecart(_)
        | EntityData::CommandBlockMinecart(_)
        | EntityData::SpawnerMinecart(_)
        | EntityData::TntMinecart(_) => {
            return minecart::tick_stored(entity, store, level)
                .then_some(None)
                .ok_or(())
        }
    }
    Ok(None)
}

pub(crate) fn base_tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    base_tick_hurting_on_fire(entity, store, level, |_, _, _| ())
}

#[cfg(test)]
pub(super) fn base_tick_without_store_for_test(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    base_tick_hurting_without_store_for_test(entity, level, |_, _| ())
}

#[cfg(test)]
fn base_tick_hurting_without_store_for_test(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    hurt: impl FnOnce(&mut Entity, &mut LevelCtx<'_, '_>),
) -> bool {
    entity_base_tick::tick(entity, None, level, |entity, _, level| hurt(entity, level))
}

pub(crate) fn handle_portal(entity: &mut Entity, level: &mut LevelCtx<'_, '_>) -> bool {
    level.handle_portal(entity);
    !level.portal_transition_requested()
}

pub(crate) fn base_tick_hurting_on_fire(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    hurt: impl FnOnce(&mut Entity, &mut EntityStore, &mut LevelCtx<'_, '_>),
) -> bool {
    entity_base_tick::tick(entity, Some(store), level, |entity, store, level| {
        hurt(
            entity,
            store.expect("production base ticks have a store"),
            level,
        )
    })
}

pub(crate) fn process_portal_cooldown(entity: &mut Entity) {
    if entity.portal_cooldown > 0 {
        entity.portal_cooldown -= 1;
    }
}

/// `Entity.checkBelowWorld`; a living body's `onBelowWorld` is the
/// `out_of_world` hurt its own base tick deals (`LivingEntity.java:2141-2143`).
pub(crate) fn check_below_world(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    if is_below_world(entity, level) && entity.living().is_none() {
        entity.discard();
    }
}

pub(crate) fn is_below_world(entity: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    entity.position.y < f64::from(level.min_y()) - 64.0
}

pub fn apply_gravity(entity: &mut Entity) {
    let gravity = entity.gravity();
    if gravity != 0.0 {
        entity.delta_movement.y -= gravity;
    }
}
