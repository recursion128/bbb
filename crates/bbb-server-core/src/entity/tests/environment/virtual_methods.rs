use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::packets::Direction;

fn bullet(store: &mut EntityStore) -> i32 {
    store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        FEET,
        EntityData::ShulkerBullet(crate::entity::ShulkerBulletData::from_saved(
            crate::entity::hurting_projectile::ProjectileState::new(None),
            None,
            None,
            0,
            Vec3d::default(),
        )),
    )
}

#[test]
fn blaze_fire_query_follows_production_charged_state_independently_of_fire_ticks() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:blaze", FEET);
    let mut entity = store.detach(id).unwrap();
    let mut level = NativeLevel::new();

    for charged in [false, true, false] {
        level.with_level(|ctx| {
            crate::entity::blaze::apply_ai_step(
                &mut entity,
                &mut store,
                ctx,
                crate::entity::blaze::BlazeAiStepResult {
                    charged,
                    delta_movement: Vec3d::default(),
                    action: None,
                },
            )
        });
        for fire in [0, 100] {
            entity.set_remaining_fire_ticks(fire);
            assert_eq!(entity.is_on_fire(), charged);
        }
    }
}

#[test]
fn fire_query_subtype_overrides_ignore_positive_fire_ticks() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let magma = living_at(&mut store, "minecraft:magma_cube", FEET);
    let strider = living_at(&mut store, "minecraft:strider", FEET);
    let bullet = bullet(&mut store);
    let skull = crate::entity::hurting_projectile::wither_skull::spawn_wither_skull(
        &mut store,
        FEET,
        crate::entity::projectile_shoot::ProjectileShot {
            movement: Vec3d::default(),
            source_movement: Vec3d::default(),
        },
        None,
        false,
    )
    .unwrap();
    let pig = living_at(&mut store, "minecraft:pig", FEET);
    for id in [magma, strider, bullet, skull, pig] {
        let entity = store.get_mut(id).unwrap();
        entity.set_remaining_fire_ticks(100);
        assert_eq!(entity.is_on_fire(), id == pig);
    }
}

#[test]
fn no_physics_subtypes_still_scan_piston_contacts_but_removed_bodies_do_not() {
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let vex = living_at(&mut store, "minecraft:vex", FEET);
    let bullet = bullet(&mut store);
    let pig = living_at(&mut store, "minecraft:pig", FEET);
    for id in [vex, bullet, pig] {
        let mut entity = store.detach(id).unwrap();
        entity.set_no_physics(true);
        entity.set_remaining_fire_ticks(100);
        let mut level = NativeLevel::new();
        level
            .world
            .set_block(DIMENSION, pos(1, 64, 0), state_id("minecraft:stone"), 18);
        level
            .world
            .set_block(DIMENSION, pos(2, 64, 0), state_id("minecraft:water"), 18);
        assert!(
            level.with_level(|ctx| crate::entity::movement::move_entity_by_piston(
                &mut entity,
                &mut store,
                ctx,
                Vec3d {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0
                },
                Direction::East
            ))
        );
        assert_eq!(entity.position().x, FEET.x + 2.0);
        assert_eq!(
            entity.remaining_fire_ticks(),
            if id == pig { 100 } else { 0 }
        );
        entity.set_remaining_fire_ticks(100);
        entity.discard();
        assert!(level.with_level(|ctx| {
            crate::entity::apply_nonliving_inside_block_effects_between(
                &mut entity,
                ctx,
                Some(&mut store),
                FEET,
                Vec3d { x: 2.5, ..FEET },
            )
        }));
        assert_eq!(entity.remaining_fire_ticks(), 100);
    }
}

#[test]
fn vex_tick_applies_cactus_contact_damage() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:vex", FEET);
    let entity = store.get_mut(id).unwrap();
    let health = entity.living().unwrap().health();
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.set_no_ai(true);
    }
    let mut level = NativeLevel::new();
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:cactus"), 18);
    level.with_level(|ctx| crate::entity::tick_entities(&mut store, ctx, true));
    let entity = store.get(id).unwrap();
    assert_eq!(entity.living().unwrap().health(), health - 1.0);
}
