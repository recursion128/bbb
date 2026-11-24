//! `OminousItemSpawner`: a delayed, invulnerable item/projectile source.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_synched_data::VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
    VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID, VANILLA_ENTITY_TYPE_TRIDENT_ID,
    VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, SoundSource, Vec3d};
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::abstract_arrow::AbstractArrowState;
use crate::entity::arrow::{ArrowData, ArrowPickup};
use crate::entity::firework_rocket::{default_lifetime_for_item, FireworkRocketData};
use crate::entity::hurting_projectile::small_fireball::SmallFireballData;
use crate::entity::hurting_projectile::wind_charge::WindChargeData;
use crate::entity::hurting_projectile::{HurtingProjectileState, ProjectileState};
use crate::entity::item::{empty_stack, spawn_item_entity, stack_with_count};
use crate::entity::living::SynchedValue;
use crate::entity::projectile_shoot::{movement_to_shoot, ProjectileShot};
use crate::entity::spectral_arrow::{SpectralArrowData, DEFAULT_DURATION};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::trident::TridentData;
use crate::entity::{base_tick, Entity, EntityData, RemovalReason};
use crate::level_event::VanillaGameEvent;

const SPAWN_DELAY_MIN: i64 = 60;
const SPAWN_DELAY_RANGE: i32 = 61;
const ABOUT_TO_SPAWN_LEAD: i64 = 36;
const SPAWN_LEVEL_EVENT: i32 = 3021;

#[derive(Debug, Clone, PartialEq)]
pub struct OminousItemSpawnerData {
    item: RuntimeItemStack,
    spawn_item_after_ticks: i64,
}

impl OminousItemSpawnerData {
    pub fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    pub const fn spawn_item_after_ticks(&self) -> i64 {
        self.spawn_item_after_ticks
    }

    pub fn from_saved(item: RuntimeItemStack, spawn_item_after_ticks: i64) -> Self {
        Self {
            item,
            spawn_item_after_ticks,
        }
    }
}

pub fn spawn_ominous_item_spawner(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let delay = SPAWN_DELAY_MIN + i64::from(store.random().next_int(SPAWN_DELAY_RANGE));
    store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
        position,
        EntityData::OminousItemSpawner(OminousItemSpawnerData {
            item,
            spawn_item_after_ticks: delay,
        }),
    )
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if !base_tick(entity, store, level) {
        return false;
    }
    let EntityData::OminousItemSpawner(data) = entity.data() else {
        unreachable!("ominous item spawner ticks require spawner state");
    };
    let tick = i64::from(entity.tick_count());
    if tick == data.spawn_item_after_ticks - ABOUT_TO_SPAWN_LEAD {
        store.play_entity_sound(PendingEntitySound {
            position: block_center(entity.block_position()),
            resource_id: "minecraft:block.trial_spawner.about_to_spawn_item".to_string(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    if tick < data.spawn_item_after_ticks {
        return true;
    }

    if !data.item.is_empty() {
        let spawned_id = spawn_item(entity, store, level, &data.item);
        level.level_event(SPAWN_LEVEL_EVENT, entity.block_position(), 1);
        level.game_event(
            VanillaGameEvent::ENTITY_PLACE,
            entity.block_position(),
            spawned_id,
        );
        let EntityData::OminousItemSpawner(data) = &mut entity.data else {
            unreachable!("ominous item spawner ticks require spawner state");
        };
        data.item = empty_stack().expect("ItemStack.EMPTY has a canonical save form");
        entity.mark_synched_dirty(VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID);
    }
    // `:61`: `this.kill(level)` after the spawn, so the removal and its
    // `ENTITY_DIE` land on the same tick as the item.
    entity.set_removed(RemovalReason::Killed);
    crate::entity::kill::raise_entity_die(entity, level);
    true
}

fn spawn_item(
    spawner: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    item: &RuntimeItemStack,
) -> Option<i32> {
    let position = spawner.position();
    let owner = spawner.uuid();
    match projectile_kind(item_resource_id(item)) {
        Some(kind) => spawn_projectile(store, level, position, owner, item, kind),
        None => spawn_item_entity(store, position, item.clone()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectileKind {
    Arrow,
    SpectralArrow,
    Egg,
    ExperienceBottle,
    FireCharge,
    FireworkRocket,
    Snowball,
    SplashPotion,
    LingeringPotion,
    Trident,
    WindCharge,
}

fn projectile_kind(resource_id: &str) -> Option<ProjectileKind> {
    Some(match resource_id {
        "minecraft:arrow" | "minecraft:tipped_arrow" => ProjectileKind::Arrow,
        "minecraft:spectral_arrow" => ProjectileKind::SpectralArrow,
        "minecraft:egg" | "minecraft:blue_egg" | "minecraft:brown_egg" => ProjectileKind::Egg,
        "minecraft:experience_bottle" => ProjectileKind::ExperienceBottle,
        "minecraft:fire_charge" => ProjectileKind::FireCharge,
        "minecraft:firework_rocket" => ProjectileKind::FireworkRocket,
        "minecraft:snowball" => ProjectileKind::Snowball,
        "minecraft:splash_potion" => ProjectileKind::SplashPotion,
        "minecraft:lingering_potion" => ProjectileKind::LingeringPotion,
        "minecraft:trident" => ProjectileKind::Trident,
        "minecraft:wind_charge" => ProjectileKind::WindCharge,
        _ => return None,
    })
}

fn spawn_projectile(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    position: Vec3d,
    owner: Uuid,
    item: &RuntimeItemStack,
    kind: ProjectileKind,
) -> Option<i32> {
    let item = stack_with_count(item, 1)?;
    let input = item.clone();
    let id = match kind {
        ProjectileKind::Arrow => spawn_arrow(store, position, item, owner, false),
        ProjectileKind::SpectralArrow => spawn_arrow(store, position, item, owner, true),
        ProjectileKind::Egg => {
            let shot = standard_shot(store, 1.1, 6.0);
            crate::entity::spawn_egg(store, position, shot, item, Some(owner))
        }
        ProjectileKind::ExperienceBottle => {
            let shot = standard_shot(store, 1.375, 3.0);
            crate::entity::spawn_experience_bottle(store, position, shot, item, Some(owner))
        }
        ProjectileKind::FireCharge => {
            level.level_event(1018, containing_block(position), 0);
            spawn_fire_charge(store, position, item, owner)
        }
        ProjectileKind::FireworkRocket => {
            level.level_event(1004, containing_block(position), 0);
            spawn_firework(store, position, item, owner)
        }
        ProjectileKind::Snowball => {
            let shot = standard_shot(store, 1.1, 6.0);
            crate::entity::spawn_snowball(store, position, shot, item, Some(owner))
        }
        ProjectileKind::SplashPotion => {
            let shot = standard_shot(store, 1.375, 3.0);
            crate::entity::spawn_splash_potion(store, position, shot, item, Some(owner))
        }
        ProjectileKind::LingeringPotion => {
            let shot = standard_shot(store, 1.375, 3.0);
            crate::entity::spawn_lingering_potion(store, position, shot, item, Some(owner))
        }
        ProjectileKind::Trident => spawn_trident(store, position, item, owner),
        ProjectileKind::WindCharge => {
            level.level_event(1051, containing_block(position), 0);
            spawn_wind_charge(store, position, owner)
        }
    }?;
    crate::entity::projectile_weapon::queue_spawn_effects(store, id, input);
    Some(id)
}

fn standard_shot(store: &mut EntityStore, power: f32, uncertainty: f32) -> ProjectileShot {
    ProjectileShot::new(movement_to_shoot(
        store,
        Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        },
        power,
        uncertainty,
    ))
}

fn spawn_arrow(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    owner: Uuid,
    spectral: bool,
) -> Option<i32> {
    let shot = standard_shot(store, 1.1, 6.0);
    let uuid = store.next_uuid();
    let mut base = AbstractArrowState::new(item, "minecraft:entity.arrow.hit");
    base.set_pickup(ArrowPickup::Allowed);
    base.projectile.set_owner(Some(owner));
    let (entity_type, data) = if spectral {
        (
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
            EntityData::SpectralArrow(SpectralArrowData::from_state(base, DEFAULT_DURATION)),
        )
    } else {
        (
            VANILLA_ENTITY_TYPE_ARROW_ID,
            EntityData::Arrow(ArrowData::from_state(base)),
        )
    };
    store.restore_initialized(uuid, entity_type, position, data, |entity| {
        shot.install(entity)
    })
}

fn spawn_trident(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    owner: Uuid,
) -> Option<i32> {
    let shot = standard_shot(store, 1.1, 6.0);
    let uuid = store.next_uuid();
    let mut data = TridentData::new(item);
    data.set_pickup(ArrowPickup::Allowed);
    data.projectile.set_owner(Some(owner));
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        position,
        EntityData::Trident(data),
        |entity| shot.install(entity),
    )
}

/// `FireChargeItem.asProjectile` (`FireChargeItem.java:64-73`) then
/// `Projectile.shoot` at power 1.0 and uncertainty 6.6666665
/// (`OminousItemSpawner.java:90-103`, `FireChargeItem.java:80-86`).
fn spawn_fire_charge(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    owner: Uuid,
) -> Option<i32> {
    let shot = standard_shot(store, 1.0, 6.666_666_5);
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
        position,
        EntityData::SmallFireball(SmallFireballData::new(
            HurtingProjectileState::new(
                Some(owner),
                HurtingProjectileState::INITIAL_ACCELERATION_POWER,
            ),
            item,
        )),
        |entity| shot.install(entity),
    )
}

fn spawn_firework(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    owner: Uuid,
) -> Option<i32> {
    let lifetime =
        default_lifetime_for_item(&item) + store.random().next_int(6) + store.random().next_int(7);
    let shot = standard_shot(store, 0.5, 1.0);
    let uuid = store.next_uuid();
    let data =
        FireworkRocketData::from_saved(item, 0, lifetime, true, ProjectileState::new(Some(owner)));
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        position,
        EntityData::FireworkRocket(data),
        |entity| shot.install(entity),
    )
}

/// `OminousItemSpawner.spawnProjectile` (`OminousItemSpawner.java:90-106`)
/// calls `Projectile.spawnProjectileUsingShoot` with `WindChargeItem`'s
/// `DispenseConfig` (`WindChargeItem.java:73-79`: power 1.0, uncertainty 6.6666665).
fn spawn_wind_charge(store: &mut EntityStore, position: Vec3d, owner: Uuid) -> Option<i32> {
    let shot = standard_shot(store, 1.0, 6.666_666_5);
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        position,
        EntityData::WindCharge(WindChargeData::new(
            HurtingProjectileState::new(Some(owner), 0.0),
            5,
        )),
        |entity| shot.install(entity),
    )
}

fn item_resource_id(item: &RuntimeItemStack) -> &str {
    item.item_resource_id().unwrap_or("minecraft:air")
}

fn containing_block(position: Vec3d) -> bbb_protocol::packets::BlockPos {
    bbb_protocol::packets::BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn block_center(pos: bbb_protocol::packets::BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

pub fn synched_values(entity: &Entity, data: &OminousItemSpawnerData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    let stack = data.item().summary().clone();
    let is_default = stack.item_id.is_none();
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_OMINOUS_ITEM_SPAWNER_ITEM_DATA_ID,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack),
        },
        is_default,
    });
    rows
}
