use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use uuid::Uuid;

use crate::entity::ai::{MemoryModuleType, MemoryValue};
use crate::entity::{Entity, EntityData, EntityStore};

pub(crate) mod dye_mix;
pub(crate) mod offspring;

/// `Animal.PARENT_AGE_AFTER_BREEDING` (`Animal.java:43`), the age both parents
/// take (`:224-225`).
const PARENT_AGE_AFTER_BREEDING: i32 = 6000;

/// `Animal.setInLove`'s timer literal (`Animal.java:173`). `setInLoveTime`
/// (`:181-183`) takes the same field from its caller and has no consumer here
/// (`BeehiveBlockEntity.java:420`, `Turtle.java:462`).
const IN_LOVE_TICKS: i32 = 600;

/// The `(byte)18` entity event `Animal.setInLove` and
/// `Animal.finalizeSpawnChildFromBreeding` broadcast (`Animal.java:178`,
/// `:228`), which `Animal.handleEntityEvent` turns into seven heart particles
/// on the client (`Animal.java:235-246`).
const LOVE_HEARTS_ENTITY_EVENT: i8 = 18;

pub(crate) fn is_in_love(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.animal_retained()
        .is_some_and(|animal| animal.is_in_love())
}

pub(crate) fn can_fall_in_love(entity: &Entity) -> bool {
    // Hoglin.java:308-310; HappyGhast.java:247-249.
    entity.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        && !is_in_love(entity)
        && !crate::entity::armadillo::is_scared(entity)
        && !crate::entity::hoglin::is_pacified(entity)
}

///
/// `false` is a body with no `Animal` state to write.
pub(crate) fn set_in_love(
    entity: &mut Entity,
    store: &mut EntityStore,
    cause: Option<Uuid>,
) -> bool {
    let id = entity.id();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(animal) = data.animal_retained_mut() else {
        return false;
    };
    animal.set_in_love(IN_LOVE_TICKS, cause);
    // `this.level().broadcastEntityEvent(this, (byte)18)` (`:178`).
    store.broadcast_to_tracking(
        id,
        crate::connection::ClientboundPlayPacket::EntityEvent(bbb_protocol::packets::EntityEvent {
            entity_id: id,
            event_id: LOVE_HEARTS_ENTITY_EVENT,
        }),
    );
    true
}

/// `false` is a store that could not pay one of the writes, which refuses the
/// tick rather than mating halfway.
pub(crate) fn spawn_child_from_breeding(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    partner_id: i32,
    random: &GameRandom,
) -> bool {
    if body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID {
        // Sniffers lay an item even when mob drops are disabled (`Sniffer.java:345-352`).
        let egg = crate::entity::item::plain_stack("minecraft:sniffer_egg", 1).unwrap();
        if crate::entity::item::spawn_dropped_item_entity(store, body.position(), egg).is_none()
            || !finalize_spawn_child_from_breeding(body, store, level, partner_id, random, None)
        {
            return false;
        }
        if !body.is_silent() {
            store.play_entity_sound(crate::entity::store::PendingEntitySound {
                position: body.position(),
                resource_id: "minecraft:block.sniffer_egg.plop".into(),
                source: bbb_protocol::packets::SoundSource::Neutral,
                volume: 1.0,
                pitch: (random.next_float() - random.next_float()) * 0.2 + 0.5,
            });
        }
        return true;
    }
    if body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID {
        let EntityData::GeneralLiving(data) = &mut body.data else {
            return false;
        };
        let Some(turtle) = data.turtle_retained_mut() else {
            return false;
        };
        // `TurtleBreedGoal.breed` (`Turtle.java:301-321`) carries an egg instead of a child.
        turtle.set_has_egg(true);
        body.mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_TURTLE_HAS_EGG_DATA_ID);
        return finalize_spawn_child_from_breeding(body, store, level, partner_id, random, None);
    }
    if !is_frog(body) {
        return spawn_child_from_breeding_base(body, store, level, partner_id, random);
    }
    if !finalize_spawn_child_from_breeding(body, store, level, partner_id, random, None) {
        return false;
    }
    store.set_mob_brain_memory(
        body.id(),
        Some(body),
        MemoryModuleType::IsPregnant,
        Some(MemoryValue::Unit),
    )
}

fn is_fox(body: &Entity) -> bool {
    body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID
}

fn is_frog(body: &Entity) -> bool {
    body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FROG_ID
}

/// A body whose type has no factory in [`offspring`] behaves like a subtype
/// whose `getBreedOffspring` answered null: the whole body is skipped, love
/// included (`Animal.java:211`).
fn spawn_child_from_breeding_base(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    partner_id: i32,
    random: &GameRandom,
) -> bool {
    let Some(plan) = offspring::plan_offspring(
        body,
        store,
        &mut |first, second| dye_mix::mixed_color(level, first, second),
        partner_id,
        random,
    ) else {
        return true;
    };
    let Some(variant) = offspring::parent_variant(plan, body, partner_id, store) else {
        return true;
    };
    let trusted = match store.get(partner_id) {
        Some(partner) if is_fox(body) => Some(crate::entity::fox::breeding::trusted_feeders(
            body, partner, store, level,
        )),
        _ => None,
    };
    let biome = level.biome_resource_id(body.block_position());
    let biome_has_tag = |tag: &str| {
        biome.is_some_and(|biome| {
            level.registry_tag_membership("minecraft:worldgen/biome", tag, biome)
        })
    };
    let Some(child_id) = offspring::spawn_offspring(
        plan,
        Some(&variant),
        body,
        partner_id,
        store,
        &biome_has_tag,
        random,
    ) else {
        return false;
    };
    if let (Some(trusted), Some(child)) = (trusted, store.get_mut(child_id)) {
        crate::entity::fox::breeding::set_child_trust(child, trusted);
    }
    finalize_spawn_child_from_breeding(body, store, level, partner_id, random, Some(child_id))
}

fn finalize_spawn_child_from_breeding(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    partner_id: i32,
    random: &GameRandom,
    child_id: Option<i32>,
) -> bool {
    let body_id = body.id();
    let position = body.position();
    // `Optional.ofNullable(this.getLoveCause()).or(...)` (`Animal.java:220-223`).
    let cause = love_cause(body).or_else(|| store.get(partner_id).and_then(love_cause));
    if let Some(player_uuid) = cause {
        store.queue_command(crate::entity::EntityCommand::BredAnimals(
            crate::entity::PendingBredAnimals {
                player_uuid,
                parent_id: body_id,
                partner_id,
                child_id,
            },
        ));
    }
    // `this.setAge(6000)`, `partner.setAge(6000)`, `this.resetLove()`,
    // `partner.resetLove()` (`:224-227`).
    if !crate::entity::set_spawner_age(body, PARENT_AGE_AFTER_BREEDING) {
        return false;
    }
    if !store
        .get_mut(partner_id)
        .is_some_and(|partner| crate::entity::set_spawner_age(partner, PARENT_AGE_AFTER_BREEDING))
    {
        return false;
    }
    if !reset_love(body) {
        return false;
    }
    if !store.get_mut(partner_id).is_some_and(reset_love) {
        return false;
    }
    // `level.broadcastEntityEvent(this, (byte)18)` (`:228`).
    if body.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID {
        store.broadcast_to_tracking(
            body_id,
            crate::connection::ClientboundPlayPacket::EntityEvent(
                bbb_protocol::packets::EntityEvent {
                    entity_id: body_id,
                    event_id: LOVE_HEARTS_ENTITY_EVENT,
                },
            ),
        );
    }
    // `if (level.getGameRules().get(GameRules.MOB_DROPS))` (`:229-231`).
    if level.mob_drops() {
        let value = random.next_int(7) + 1;
        let _ = crate::entity::experience_orb::spawn_experience_orb(store, position, value);
    }
    true
}

fn love_cause(entity: &Entity) -> Option<Uuid> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.animal_retained()?.love_cause()
}

fn reset_love(entity: &mut Entity) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(animal) = data.animal_retained_mut() else {
        return false;
    };
    animal.reset_love();
    true
}

#[cfg(test)]
mod tests;
