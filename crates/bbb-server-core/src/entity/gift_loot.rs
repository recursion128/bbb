//! Entity gifts drawn through `LivingEntity.dropFromGiftLootTable`.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMADILLO_ID;

use crate::entity::store::{EntityBodyReadView, EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;

const CHICKEN_LAY: &str = "minecraft:gameplay/chicken_lay";
const ARMADILLO_SHED: &str = "minecraft:gameplay/armadillo_shed";
const TURTLE_GROW: &str = "minecraft:gameplay/turtle_grow";

/// The tail of `Chicken.aiStep` (`Chicken.java:130-137`).
pub(crate) fn chicken_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if data.chicken_jockey() != Some(false) || is_baby(entity) {
        return;
    }
    count_down(
        entity,
        store,
        level,
        true,
        CHICKEN_LAY,
        "minecraft:entity.chicken.egg",
    );
}

/// The scute half of `Armadillo.customServerAiStep` (`Armadillo.java:145-152`):
/// `shouldDropLoot` (`LivingEntity.java:561-563`) only holds the drop back, the
/// countdown still resets.
pub(crate) fn armadillo_custom_server_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ARMADILLO_ID || !entity.is_alive() {
        return;
    }
    let drops = level.mob_drops();
    count_down(
        entity,
        store,
        level,
        drops,
        ARMADILLO_SHED,
        "minecraft:entity.armadillo.scute_drop",
    );
}

/// `Turtle.ageBoundaryReached` (`Turtle.java:255-260`).
pub(crate) fn turtle_age_boundary_reached(
    entity: &Entity,
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if data.turtle_retained().is_none() || is_baby(entity) || !level.mob_drops() {
        return;
    }
    drop_gift(entity, store, level, TURTLE_GROW, entity.position(), true);
}

/// `--time <= 0 && drops`, then the drop, its sound and `ENTITY_PLACE`, and
/// a redraw of `nextInt(6000) + 6000`.
fn count_down(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    drops: bool,
    table_id: &str,
    sound: &str,
) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(time) = data.gift_drop_time_mut() else {
        return;
    };
    *time = time.saturating_sub(1);
    if *time > 0 {
        return;
    }
    if drops && drop_gift(entity, store, level, table_id, entity.position(), true) {
        if !entity.is_silent() {
            let random = level.level_random();
            store.play_entity_sound(PendingEntitySound {
                position: entity.position(),
                resource_id: sound.to_owned(),
                source: crate::entity::living::living_sound_source(entity),
                volume: 1.0,
                pitch: (random.next_float() - random.next_float()) * 0.2 + 1.0,
            });
        }
        level.game_event(
            VanillaGameEvent::ENTITY_PLACE,
            entity.block_position(),
            Some(entity.id()),
        );
    }
    let next = level.level_random().next_int(6000) + 6000;
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(time) = data.gift_drop_time_mut() {
            *time = next;
        }
    }
}

/// Every drawn stack spawned at the supplied position.
pub(crate) fn drop_gift(
    entity: &Entity,
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
    table_id: &str,
    position: Vec3d,
    default_pickup_delay: bool,
) -> bool {
    let drawn = level.gift_loot_drops(
        EntityBodyReadView::new(store, Some(entity), None),
        entity,
        table_id,
    );
    let dropped = !drawn.is_empty();
    for stack in drawn {
        let stack = RuntimeItemStack::from_summary(stack);
        let _ = level.spawn_runtime_item(position, stack, default_pickup_delay);
    }
    dropped
}

fn is_baby(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.ageable_retained().is_some_and(|age| age.is_baby()))
}

// `Cat.CatRelaxOnOwnerGoal.giveMorningGift` (`Cat.java:594-613`).
pub(crate) fn cat_morning_gift(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    let chance = level.cat_waking_up_gift_chance(entity.block_position())?;
    if level.level_random().next_float().partial_cmp(&chance) != Some(std::cmp::Ordering::Less) {
        return Some(());
    }
    let origin = crate::entity::leash::leash_holder(entity)
        .and_then(|id| {
            store
                .get(id)
                .or_else(|| store.player(id).map(crate::world::PlayerState::body))
                .or_else(|| level.detached_entity(id))
        })
        .map_or_else(|| entity.block_position(), Entity::block_position);
    let random = level.level_random();
    let destination = Vec3d {
        x: f64::from(origin.x + random.next_int(11) - 5),
        y: f64::from(origin.y + random.next_int(5) - 2),
        z: f64::from(origin.z + random.next_int(11) - 5),
    };
    let _ =
        crate::entity::living::random_teleport::attempt(entity, store, level, destination, false)?;
    let position = entity.block_position();
    let yaw = entity.living()?.y_body_rot.to_radians();
    drop_gift(
        entity,
        store,
        level,
        "minecraft:gameplay/cat_morning_gift",
        Vec3d {
            x: f64::from(position.x) - f64::from(yaw.sin()),
            y: f64::from(position.y),
            z: f64::from(position.z) + f64::from(yaw.cos()),
        },
        false,
    );
    Some(())
}
