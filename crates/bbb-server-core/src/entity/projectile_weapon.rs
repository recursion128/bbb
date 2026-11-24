//! `ProjectileWeaponItem` (`ProjectileWeaponItem.java:19-145`) and the
//! crossbow's aim (`CrossbowItem.java:107-169`): drawing ammo, the spread of
//! a volley and the projectile each drawn stack becomes, for players and mobs.

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
    VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::enchantment_weapon_effects::{
    process, projectile_spawned_conditions, WeaponEffectTable,
};
use crate::entity::abstract_arrow::{state_mut, ArrowPickup};
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityCommand, EntityData};
use crate::item_durability::ValueContext;
use crate::random::GameRandom;
use crate::world::ValueEffectLevel;

/// `ItemTags.ARROWS` membership.
pub(crate) fn is_arrow(stack: &RuntimeItemStack) -> bool {
    let Some(resource_id) = stack.item_resource_id() else {
        return false;
    };
    match vanilla_persistent_registry_tag_members_26_1("minecraft:item", "minecraft:arrows") {
        PersistentRegistryTagMembers::Present(members) => {
            members.iter().any(|member| member == resource_id)
        }
        _ => false,
    }
}

pub(crate) fn is_firework(stack: &RuntimeItemStack) -> bool {
    stack.item_resource_id() == Some("minecraft:firework_rocket")
}

/// `ProjectileWeaponItem.draw` (`ProjectileWeaponItem.java:105-122`) with
/// `useAmmo` (`:239-259`): `projectile` is the live ammo stack and shrinks by
/// what was used; a player whose stack runs out is left to clear it.
/// `shooter` is the entity context `processProjectileCount` tests in.
pub(crate) fn draw(
    table: &WeaponEffectTable,
    random: &GameRandom,
    shooter: ValueContext<'_>,
    weapon: &RuntimeItemStack,
    projectile: &mut RuntimeItemStack,
    infinite_materials: bool,
    value_level: &ValueEffectLevel<'_>,
) -> Vec<RuntimeItemStack> {
    if projectile.is_empty() {
        return Vec::new();
    }
    let count = process(
        table,
        weapon.summary(),
        shooter,
        |effects| effects.projectile_count.as_ref(),
        1.0,
        random,
        value_level,
    ) as i32;
    let mut copy = projectile.clone();
    let mut drawn = Vec::new();
    for index in 0..count.max(0) {
        let source = if index == 0 {
            &mut *projectile
        } else {
            &mut copy
        };
        let stack = use_ammo(
            table,
            random,
            weapon,
            source,
            infinite_materials || index > 0,
            value_level,
        );
        if !stack.is_empty() {
            drawn.push(stack);
        }
    }
    drawn
}

fn use_ammo(
    table: &WeaponEffectTable,
    random: &GameRandom,
    weapon: &RuntimeItemStack,
    projectile: &mut RuntimeItemStack,
    force_infinite: bool,
    value_level: &ValueEffectLevel<'_>,
) -> RuntimeItemStack {
    let ammo_to_use = if force_infinite {
        0
    } else {
        process(
            table,
            weapon.summary(),
            ValueContext::Item(projectile.summary()),
            |effects| effects.ammo_use.as_ref(),
            1.0,
            random,
            value_level,
        ) as i32
    };
    if ammo_to_use > projectile.count() {
        return RuntimeItemStack::empty();
    }
    if ammo_to_use == 0 {
        let mut copy = projectile.with_count(1);
        let summary = copy.summary_mut();
        summary.component_patch.intangible_projectile = true;
        bbb_server_types::item_stack::rules::add_patched_component(
            summary,
            VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        );
        summary.component_patch.added_type_ids.sort_unstable();
        return copy;
    }
    projectile.split(ammo_to_use)
}

fn intangible(stack: &RuntimeItemStack) -> bool {
    stack
        .summary()
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID)
}

/// The yaw offset of each projectile of a volley (`ProjectileWeaponItem.java:55-62`).
pub(crate) fn volley_angles(count: usize, max_angle: f32) -> Vec<f32> {
    let step = if count <= 1 {
        0.0
    } else {
        2.0 * max_angle / (count - 1) as f32
    };
    let offset = ((count.max(1) - 1) % 2) as f32 * step / 2.0;
    let mut direction = 1.0;
    (0..count)
        .map(|index| {
            let angle = offset + direction * index.div_ceil(2) as f32 * step;
            direction = -direction;
            angle
        })
        .collect()
}

/// `EnchantmentHelper.processProjectileSpread(weapon, 0)`.
pub(crate) fn volley_spread(
    table: &WeaponEffectTable,
    random: &GameRandom,
    shooter: ValueContext<'_>,
    weapon: &RuntimeItemStack,
    value_level: &ValueEffectLevel<'_>,
) -> f32 {
    process(
        table,
        weapon.summary(),
        shooter,
        |effects| effects.projectile_spread.as_ref(),
        0.0,
        random,
        value_level,
    )
    .max(0.0)
}

/// `getDurabilityUse` (`ProjectileWeaponItem.java:79-81`, `CrossbowItem.java:167-169`).
pub(crate) fn durability_use(crossbow: bool, projectile: &RuntimeItemStack) -> i32 {
    if crossbow && is_firework(projectile) {
        3
    } else {
        1
    }
}

/// `CrossbowItem.getShotPitch` (`CrossbowItem.java:192-199`).
pub(crate) fn crossbow_shot_pitch(random: &GameRandom, index: usize) -> f32 {
    if index == 0 {
        return 1.0;
    }
    let range = if index & 1 == 1 { 0.63 } else { 0.43 };
    1.0 / (random.next_float() * 0.5 + 1.8) + range
}

/// The entity firing a volley.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shooter {
    pub(crate) uuid: Uuid,
    pub(crate) player: bool,
    pub(crate) position: Vec3d,
    pub(crate) eye_y: f64,
}

/// `createProjectile` (`ProjectileWeaponItem.java:93-103`,
/// `CrossbowItem.java:151-164`) carrying `shot`, then `spawnProjectile`'s
/// `applyOnProjectileSpawned` (`Projectile.java:225-242`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_projectile(
    store: &mut EntityStore,
    shooter: Shooter,
    weapon: &RuntimeItemStack,
    ammo: RuntimeItemStack,
    crit: bool,
    crossbow: bool,
    shot: ProjectileShot,
    value_level: &ValueEffectLevel<'_>,
    initialize: impl FnOnce(&mut Entity),
) -> Option<i32> {
    let spawned_item = ammo.clone();
    if crossbow && is_firework(&ammo) {
        let position = Vec3d {
            y: shooter.eye_y - f64::from(0.15_f32),
            ..shooter.position
        };
        let entity_id = crate::entity::firework_rocket::spawn_shot_firework(
            store,
            position,
            ammo,
            Some(shooter.uuid),
            shot,
            initialize,
        )?;
        queue_spawn_effects(store, entity_id, spawned_item);
        return Some(entity_id);
    }
    let pierce = process(
        store.weapon_effects(),
        weapon.summary(),
        ValueContext::Item(ammo.summary()),
        |effects| effects.projectile_piercing.as_ref(),
        0.0,
        store.random(),
        value_level,
    ) as i32;
    let pickup = if intangible(&ammo) {
        ArrowPickup::CreativeOnly
    } else if shooter.player {
        ArrowPickup::Allowed
    } else {
        ArrowPickup::Disallowed
    };
    let (entity_type_id, mut data) = if ammo.item_resource_id() == Some("minecraft:spectral_arrow")
    {
        (
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
            EntityData::SpectralArrow(crate::entity::SpectralArrowData::new(ammo)),
        )
    } else {
        (
            VANILLA_ENTITY_TYPE_ARROW_ID,
            EntityData::Arrow(crate::entity::ArrowData::new(ammo)),
        )
    };
    let state = state_mut(&mut data)?;
    state.projectile.set_owner(Some(shooter.uuid));
    state.set_pickup(pickup);
    state.set_weapon(Some(weapon.clone()));
    state.set_pierce_level(pierce.clamp(0, 127) as i8);
    state.set_crit(crit);
    if crossbow {
        state.set_sound_event("minecraft:item.crossbow.hit");
    }
    let position = Vec3d {
        y: shooter.eye_y - f64::from(0.1_f32),
        ..shooter.position
    };
    let uuid = store.next_uuid();
    let entity_id = store.restore_initialized(uuid, entity_type_id, position, data, |entity| {
        shot.install(entity);
        initialize(entity);
    })?;
    queue_spawn_effects(store, entity_id, spawned_item);
    Some(entity_id)
}

pub(crate) fn queue_spawn_effects(store: &mut EntityStore, entity_id: i32, item: RuntimeItemStack) {
    let table = store.weapon_effects();
    let ammo_has_effects = projectile_spawned_conditions(table, item.summary())
        .next()
        .is_some();
    let weapon_has_effects = store
        .get(entity_id)
        .and_then(|entity| crate::entity::abstract_arrow::state(entity.data()))
        .and_then(|state| state.weapon())
        .filter(|weapon| weapon.item_id() != item.item_id())
        .is_some_and(|weapon| {
            projectile_spawned_conditions(table, weapon.summary())
                .next()
                .is_some()
        });
    if ammo_has_effects || weapon_has_effects {
        store.queue_command(EntityCommand::ProjectileSpawned { entity_id, item });
    }
}

/// The direction `CrossbowItem.shootProjectile` (`CrossbowItem.java:107-136`)
/// shoots along: a target's direction tilted by `angle` about the shot's
/// side axis, or the shooter's view turned by `angle` about its up axis.
pub(crate) fn crossbow_shot_direction(
    x_rot: f32,
    y_rot: f32,
    target_direction: Option<Vec3d>,
    angle: f32,
) -> Vec3d {
    let up = crate::entity::ai::mob::stroll::view_vector(x_rot - 90.0, y_rot);
    let radians = f64::from(angle * (std::f32::consts::PI / 180.0));
    let Some(target) = target_direction else {
        let view = crate::entity::ai::mob::stroll::view_vector(x_rot, y_rot);
        return rotate_about(view, up, radians);
    };
    let view = target.normalize();
    let mut right = cross(
        view,
        Vec3d {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
    );
    if right.length_sqr() <= 1.0e-7 {
        right = cross(view, up);
    }
    let side = rotate_about(view, right.normalize(), std::f64::consts::FRAC_PI_2);
    rotate_about(view, side.normalize(), radians)
}

/// Rodrigues' rotation of `vector` about the unit `axis`.
fn rotate_about(vector: Vec3d, axis: Vec3d, radians: f64) -> Vec3d {
    let (sin, cos) = radians.sin_cos();
    let dot = axis.x * vector.x + axis.y * vector.y + axis.z * vector.z;
    vector
        .scale(cos)
        .plus(cross(axis, vector).scale(sin))
        .plus(axis.scale(dot * (1.0 - cos)))
}

fn cross(a: Vec3d, b: Vec3d) -> Vec3d {
    Vec3d {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

/// `DataComponents.CHARGED_PROJECTILES` of a crossbow, empty by default.
pub(crate) fn charged_projectiles(stack: &RuntimeItemStack) -> Vec<RuntimeItemStack> {
    let patch = &stack.summary().component_patch;
    if !patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID)
    {
        return Vec::new();
    }
    patch
        .charged_projectiles_items
        .iter()
        .map(RuntimeItemStack::from_template)
        .collect()
}

/// `stack.set(DataComponents.CHARGED_PROJECTILES, ...)`; an empty list is the
/// prototype's value and leaves no patch entry.
pub(crate) fn with_charged_projectiles(
    stack: &RuntimeItemStack,
    projectiles: &[RuntimeItemStack],
) -> RuntimeItemStack {
    use bbb_server_types::item_stack::rules::{add_patched_component, remove_patched_component};
    let mut stack = stack.clone();
    let summary = stack.summary_mut();
    summary.component_patch.charged_projectiles_items = projectiles
        .iter()
        .filter_map(RuntimeItemStack::to_template)
        .collect();
    if projectiles.is_empty() {
        remove_patched_component(summary, VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID);
    } else {
        add_patched_component(summary, VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID);
        summary.component_patch.added_type_ids.sort_unstable();
    }
    stack
}
