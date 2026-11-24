use super::water::{pool, tick};
use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID, VANILLA_ENTITY_TYPE_SALMON_ID,
    VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
};

#[test]
fn fish_navigation_drives_each_fish_over_a_submerged_wall_and_keeps_no_ai_still() {
    let mut native = pool();
    for y in 64..=66 {
        for z in -3..=4 {
            native.set(BlockPos { x: 3, y, z }, state("minecraft:stone"));
        }
    }
    for kind in [
        VANILLA_ENTITY_TYPE_COD_ID,
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        VANILLA_ENTITY_TYPE_SALMON_ID,
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
    ] {
        let mut store = EntityStore::new(17, EntityIdCounter::new());
        let start = Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        };
        let id = store.spawn_general_living(kind, start).unwrap();
        let still_pos = Vec3d {
            x: 0.5,
            y: 65.0,
            z: 2.5,
        };
        let still = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_PUFFERFISH_ID, still_pos)
            .unwrap();
        let crate::entity::EntityData::GeneralLiving(data) =
            &mut store.get_mut(still).unwrap().data
        else {
            unreachable!()
        };
        data.set_no_ai(true);
        tick(&mut native, &mut store);
        store.mob_ai_mut(id).unwrap().goal_selector = Default::default();
        let mob = MobPathingSnapshot::from_entity(
            store.get(id).unwrap(),
            false,
            false,
            None,
            PathfindingMalus::for_entity_type(kind),
        )
        .unwrap();
        assert!(
            native.with_level(|level| store.mob_ai_mut(id).unwrap().navigate_to(
                level,
                mob,
                BlockPos { x: 7, y: 68, z: 0 },
                0,
                1.0
            ))
        );
        let mut crossed = false;
        for _ in 0..200 {
            tick(&mut native, &mut store);
            let pos = store.get(id).unwrap().position();
            if pos.x > 4.0 && pos.y > 67.0 {
                crossed = true;
                break;
            }
        }
        assert!(crossed, "{kind}: {:?}", store.get(id).unwrap().position());
        assert_eq!(store.get(still).unwrap().position(), still_pos);
    }
}

#[test]
fn fish_navigation_persistent_pufferfish_swims_without_an_explicit_path() {
    let mut native = pool();
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let start = Vec3d {
        x: 3.5,
        y: 66.0,
        z: 0.5,
    };
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PUFFERFISH_ID, start)
        .unwrap();
    let crate::entity::EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data
    else {
        unreachable!()
    };
    data.mob_retained_mut()
        .unwrap()
        .set_persistence_required(true);
    let mut swam = false;
    for _ in 0..400 {
        tick(&mut native, &mut store);
        let pos = store.get(id).unwrap().position();
        if (pos.x - start.x).hypot(pos.z - start.z) > 1.0 {
            swam = true;
            break;
        }
    }
    assert!(
        swam,
        "actual fish position: {:?}",
        store.get(id).unwrap().position()
    );
}

#[test]
fn schooling_fish_form_a_same_kind_group_follow_a_moving_leader_and_leave_when_far() {
    let mut native = pool();
    let mut store = EntityStore::new(29, EntityIdCounter::new());
    let first = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COD_ID,
            Vec3d {
                x: 0.5,
                y: 66.0,
                z: 0.5,
            },
        )
        .unwrap();
    let second = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COD_ID,
            Vec3d {
                x: 6.5,
                y: 66.0,
                z: 0.5,
            },
        )
        .unwrap();
    let salmon = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_SALMON_ID,
            Vec3d {
                x: 3.5,
                y: 66.0,
                z: 0.5,
            },
        )
        .unwrap();
    let mut pair = None;
    for _ in 0..400 {
        tick(&mut native, &mut store);
        for id in [first, second] {
            if let Some(uuid) = crate::entity::schooling_fish_state(store.get(id).unwrap())
                .unwrap()
                .leader_uuid()
            {
                pair = Some((id, store.entity_id_for_uuid(uuid).unwrap()));
            }
        }
        if pair.is_some() {
            break;
        }
    }
    let (follower, leader) = pair.expect("nearby cod form a school");
    assert!(
        crate::entity::schooling_fish_state(store.get(salmon).unwrap())
            .unwrap()
            .leader_uuid()
            .is_none()
    );
    let start = store.get(follower).unwrap().position();
    let target = Vec3d {
        x: start.x + if start.x < 4.0 { 4.0 } else { -4.0 },
        ..start
    };
    let mut body = store.detach(leader).unwrap();
    body.set_pos(target);
    let crate::entity::EntityData::GeneralLiving(data) = &mut body.data else {
        unreachable!()
    };
    data.set_no_ai(true);
    store.attach(body);
    let mut followed = false;
    for _ in 0..160 {
        tick(&mut native, &mut store);
        if store
            .get(follower)
            .unwrap()
            .position()
            .distance_to_sqr(target)
            < 4.0
        {
            followed = true;
            break;
        }
    }
    assert!(
        followed,
        "actual follower position: {:?}, target: {target:?}",
        store.get(follower).unwrap().position()
    );
    assert!((store.get(follower).unwrap().position().x - start.x).abs() > 1.0);
    let mut body = store.detach(leader).unwrap();
    body.set_pos(Vec3d {
        x: target.x + 24.0,
        ..target
    });
    store.attach(body);
    for _ in 0..5 {
        tick(&mut native, &mut store);
    }
    assert!(
        crate::entity::schooling_fish_state(store.get(follower).unwrap())
            .unwrap()
            .leader_uuid()
            .is_none()
    );
    assert_eq!(
        crate::entity::schooling_fish_state(store.get(leader).unwrap())
            .unwrap()
            .school_size(),
        1
    );
}
