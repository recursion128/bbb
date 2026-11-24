use super::native_level::{NativeLevel, DIMENSION};
use super::*;
#[test]
fn a_spawned_orb_has_rotation_and_scattered_motion() {
    let mut store = EntityStore::new(20250818, EntityIdCounter::new());
    let id = crate::entity::experience_orb::spawn_experience_orb(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        7,
    )
    .expect("a freshly drawn UUID is free");
    let entity = store.get(id).expect("entity is live");
    assert!((0.0..360.0).contains(&entity.y_rot()));
    assert_eq!(entity.x_rot(), 0.0);
    assert!((-0.2..0.2).contains(&entity.delta_movement().x));
    assert!((0.0..0.4).contains(&entity.delta_movement().y));
    assert!((-0.2..0.2).contains(&entity.delta_movement().z));
}

#[test]
fn the_award_split_conserves_the_amount_and_uses_the_vanilla_rungs() {
    for amount in [1, 17, 2_500] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let random = crate::random::GameRandom::with_seed(9);
        let position = Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        };
        crate::entity::award_experience(&mut store, &random, position, amount);
        let mut total = 0;
        for id in store.ids() {
            let entity = store.get(id).expect("a listed entity is in the store");
            let EntityData::ExperienceOrb(orb) = entity.data() else {
                panic!("an award spawns experience orbs only");
            };
            assert_eq!(
                bbb_server_types::experience::experience_value(orb.value()),
                orb.value(),
                "value {} is a rung",
                orb.value()
            );
            total += orb.value() * orb.count();
        }
        assert_eq!(total, amount, "the split conserved {amount}");
    }
}

#[test]
fn an_orb_discards_itself_once_it_reaches_its_lifetime() {
    let mut level = NativeLevel::new();
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        vanilla_entity_type_id_for_resource_id("minecraft:experience_orb")
            .expect("experience_orb is a built-in type"),
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        EntityData::ExperienceOrb(ExperienceOrbData::from_saved(
            5,
            bbb_server_types::experience::LIFETIME - 2,
            3,
            1,
        )),
    );
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let EntityData::ExperienceOrb(orb) = store
        .get(id)
        .expect("the orb survived one tick short of its lifetime")
        .data()
    else {
        panic!("the store holds an orb");
    };
    assert_eq!(orb.age(), bbb_server_types::experience::LIFETIME - 1);
    assert!(store.get(id).unwrap().is_alive());

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(
        store.get(id).is_none(),
        "the orb discarded itself at its lifetime"
    );
}

#[test]
fn a_moving_orb_contacts_a_portal_crossed_by_its_path() {
    let mut store = EntityStore::new(19, EntityIdCounter::new());
    let id = crate::entity::experience_orb::spawn_experience_orb(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        1,
    )
    .unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 3.0,
        y: 0.0,
        z: 0.0,
    });
    let entry = pos(2, 64, 0);
    let mut level = NativeLevel::new();
    assert!(level.world.set_block(
        DIMENSION,
        entry,
        state_id("minecraft:nether_portal"),
        2 | 16
    ));

    level.with_level(|ctx| crate::entity::experience_orb::tick(&mut entity, &mut store, ctx));

    assert_eq!(entity.portal_process().unwrap().entry_position(), entry);
}

#[test]
fn an_orb_in_lava_is_kicked_upward_and_a_dry_orb_is_not() {
    let mut store = EntityStore::new(18, EntityIdCounter::new());
    let id = store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        EntityData::ExperienceOrb(ExperienceOrbData::new(1)),
    );
    store.drain_lifecycle_events();
    let mut entity = store.detach(id).unwrap();
    entity.set_no_physics(true);
    let mut level = NativeLevel::new();
    level.set(pos(0, 68, 0), state_id("minecraft:lava"));
    level.install_tags(registry_tag_snapshot(&[
        ("minecraft:fluid", "minecraft:water", &[]),
        ("minecraft:fluid", "minecraft:lava", &["minecraft:lava"]),
    ]));

    level.with_level(|ctx| crate::entity::experience_orb::tick(&mut entity, &mut store, ctx));
    assert_eq!(entity.delta_movement().y, 0.2 * f64::from(0.98_f32));

    let mut dry = entity.clone();
    dry.set_pos(Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    });
    dry.set_delta_movement(Vec3d::default());
    level.install_tags(registry_tag_snapshot(&[
        ("minecraft:fluid", "minecraft:water", &[]),
        ("minecraft:fluid", "minecraft:lava", &[]),
    ]));
    level.with_level(|ctx| crate::entity::experience_orb::tick(&mut dry, &mut store, ctx));
    assert!(dry.delta_movement().y < 0.0);
}

#[test]
fn an_orb_stuck_in_a_block_is_pushed_out_and_resynced() {
    let mut store = EntityStore::new(18, EntityIdCounter::new());
    let id = store.spawn(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        Vec3d {
            x: 0.5,
            y: 66.3,
            z: 0.5,
        },
        EntityData::ExperienceOrb(ExperienceOrbData::new(1)),
    );
    store.drain_lifecycle_events();
    let mut entity = store.detach(id).unwrap();
    entity.set_delta_movement(Vec3d::default());
    let mut level = NativeLevel::new();
    level.set(pos(0, 66, 0), state_id("minecraft:stone"));
    let block = bbb_server_types::geometry::Aabb::of_block(pos(0, 66, 0));

    level.with_level(|ctx| crate::entity::experience_orb::tick(&mut entity, &mut store, ctx));
    assert!(entity.needs_sync());
    for _ in 0..20 {
        level.with_level(|ctx| crate::entity::experience_orb::tick(&mut entity, &mut store, ctx));
    }
    assert!(
        !entity.bounding_box().intersects(block),
        "{:?}",
        entity.position()
    );
}
