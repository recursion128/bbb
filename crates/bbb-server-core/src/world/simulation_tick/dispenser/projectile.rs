//! `ProjectileDispenseBehavior`: `asProjectile` at the item's dispense
//! position, then `Projectile.shoot` along the facing.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed};
use crate::entity::abstract_arrow::{spawn as spawn_arrow, state_mut, ArrowPickup};
use crate::entity::projectile_shoot::{movement_to_shoot, ProjectileShot};
use crate::entity::store::EntityStore;
use crate::entity::{ArrowData, EntityData, SpectralArrowData};
use crate::world::ServerWorld;

/// `ProjectileItem.DispenseConfig` for one item, and which entity it becomes.
struct Config {
    /// `getDispensePosition(source, scale, (0, y_offset, 0))`.
    scale: f64,
    y_offset: f64,
    power: f32,
    uncertainty: f32,
    /// `overrideDispenseEvent`, else `1002`.
    sound: i32,
}

/// `DispenseConfig.DEFAULT` (`ProjectileItem.java`).
const DEFAULT: Config = Config {
    scale: 0.7,
    y_offset: 0.1,
    power: 1.1,
    uncertainty: 6.0,
    sound: 1002,
};

/// `ExperienceBottleItem` and `ThrowablePotionItem`: half the uncertainty,
/// a quarter more power.
const LOBBED: Config = Config {
    power: 1.1 * 1.25,
    uncertainty: 6.0 * 0.5,
    ..DEFAULT
};

/// `FireChargeItem` and `WindChargeItem`, with their own dispense event.
const fn charge(sound: i32) -> Config {
    Config {
        scale: 1.0,
        y_offset: 0.0,
        power: 1.0,
        uncertainty: 6.666_666_5,
        sound,
    }
}

/// `FireworkRocketItem.getEntityJustOutsideOfBlockPos`.
const FIREWORK: Config = Config {
    scale: 0.500_009_999_999_747_4,
    y_offset: 0.0,
    power: 0.5,
    uncertainty: 1.0,
    sound: 1004,
};

pub(super) fn dispense(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    use bbb_protocol::item_types::*;
    let Some(id) = stack.item_id() else {
        return Ok(Dispensed::optional(stack, false));
    };
    let config = match id {
        VANILLA_ITEM_EXPERIENCE_BOTTLE_ID
        | VANILLA_ITEM_SPLASH_POTION_ID
        | VANILLA_ITEM_LINGERING_POTION_ID => LOBBED,
        VANILLA_ITEM_FIRE_CHARGE_ID => charge(1018),
        VANILLA_ITEM_WIND_CHARGE_ID => charge(1051),
        VANILLA_ITEM_FIREWORK_ROCKET_ID => FIREWORK,
        _ => DEFAULT,
    };
    let position = source.dispense_position(config.scale, config.y_offset);
    let step = source.step();
    let item = stack.with_count(1);
    let input = item.clone();
    let store = &mut world.level_mut_by_id(&source.level).entities;
    let shot = ProjectileShot::new(movement_to_shoot(
        store,
        step,
        config.power,
        config.uncertainty,
    ));
    let spawned = match id {
        VANILLA_ITEM_ARROW_ID | VANILLA_ITEM_TIPPED_ARROW_ID => arrow(
            store,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            position,
            shot,
            EntityData::Arrow(ArrowData::new(item)),
        ),
        VANILLA_ITEM_SPECTRAL_ARROW_ID => arrow(
            store,
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
            position,
            shot,
            EntityData::SpectralArrow(SpectralArrowData::new(item)),
        ),
        VANILLA_ITEM_EGG_ID | VANILLA_ITEM_BLUE_EGG_ID | VANILLA_ITEM_BROWN_EGG_ID => {
            crate::entity::spawn_egg(store, position, shot, item, None)
        }
        VANILLA_ITEM_SNOWBALL_ID => {
            crate::entity::spawn_snowball(store, position, shot, item, None)
        }
        VANILLA_ITEM_EXPERIENCE_BOTTLE_ID => {
            crate::entity::spawn_experience_bottle(store, position, shot, item, None)
        }
        VANILLA_ITEM_SPLASH_POTION_ID => {
            crate::entity::spawn_splash_potion(store, position, shot, item, None)
        }
        VANILLA_ITEM_LINGERING_POTION_ID => {
            crate::entity::spawn_lingering_potion(store, position, shot, item, None)
        }
        VANILLA_ITEM_FIREWORK_ROCKET_ID => crate::entity::firework_rocket::spawn_shot_firework(
            store,
            position,
            item,
            None,
            shot,
            |_| {},
        ),
        // `FireChargeItem.asProjectile` keeps the dispensed stack (`FireChargeItem.java:70-71`).
        VANILLA_ITEM_FIRE_CHARGE_ID => {
            let uuid = store.next_uuid();
            store.restore_initialized(
                uuid,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
                position,
                EntityData::SmallFireball(crate::entity::SmallFireballData::new(
                    crate::entity::HurtingProjectileState::new(
                        None,
                        crate::entity::HurtingProjectileState::INITIAL_ACCELERATION_POWER,
                    ),
                    item,
                )),
                |entity| shot.install(entity),
            )
        }
        VANILLA_ITEM_WIND_CHARGE_ID => {
            crate::entity::spawn_wind_charge(store, position, shot, None)
        }
        _ => None,
    };
    if let Some(id) = spawned {
        crate::entity::projectile_weapon::queue_spawn_effects(store, id, input);
        stack.shrink(1);
    }
    Ok(Dispensed::new(stack, config.sound))
}

/// `ArrowItem.asProjectile`: an ownerless arrow anyone may pick up.
fn arrow(
    store: &mut EntityStore,
    entity_type_id: i32,
    position: bbb_protocol::packets::Vec3d,
    shot: ProjectileShot,
    mut data: EntityData,
) -> Option<i32> {
    state_mut(&mut data)?.set_pickup(ArrowPickup::Allowed);
    spawn_arrow(store, entity_type_id, position, shot.movement, data)
}
