use super::*;
use crate::entity::store::EntityIdCounter;
use crate::entity::tests::native_level::{state, NativeLevel};
use bbb_protocol::entity_types::*;
use bbb_protocol::packets::{BlockPos, Vec3d};

fn controlling_level() -> NativeLevel {
    let mut native = NativeLevel::new();
    native.install_tags(crate::registry_tags::RegistryTagSnapshot::new([(
        "minecraft:entity_type".to_owned(),
        "minecraft:non_controlling_rider".to_owned(),
        Vec::<String>::new(),
    )]));
    native
}

fn deny_riders(native: &mut NativeLevel) {
    native.install_tags(crate::registry_tags::RegistryTagSnapshot::new([(
        "minecraft:entity_type".to_owned(),
        "minecraft:non_controlling_rider".to_owned(),
        VANILLA_ENTITY_TYPES
            .iter()
            .map(|kind| kind.resource_id.to_owned())
            .collect::<Vec<_>>(),
    )]));
}
fn store() -> EntityStore {
    EntityStore::new(7, EntityIdCounter::new())
}
fn mob(store: &mut EntityStore, kind: i32) -> i32 {
    store
        .spawn_general_living(
            kind,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap()
}
fn link(store: &mut EntityStore, vehicle: i32, passenger: i32) {
    assert!(store.link_existing_passenger_forced(vehicle, passenger));
}
fn no_ai(store: &mut EntityStore, id: i32) {
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
        panic!("mob");
    };
    data.set_no_ai(true);
}

#[test]
fn mounted_frog_recompute_after_block_change_keeps_strider_effective_malus() {
    use crate::entity::ai::mob::navigation::{GroundPathType, MobPathingSnapshot};

    let mut store = store();
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let strider = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_STRIDER_ID, position)
        .unwrap();
    let frog = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_FROG_ID, position)
        .unwrap();
    link(&mut store, strider, frog);
    store.get_mut(frog).unwrap().on_ground = true;
    let mut native = NativeLevel::new();
    let (effective, inherited) = native
        .with_level(|level| {
            store.effective_pathfinding_malus_with_provenance(store.get(frog).unwrap(), level)
        })
        .unwrap();
    assert!(inherited);
    assert_eq!(effective.get(GroundPathType::Water), -1.0);
    let mob =
        MobPathingSnapshot::from_entity(store.get(frog).unwrap(), false, true, None, effective)
            .unwrap()
            .with_inherited_malus(inherited);
    let target = BlockPos { x: 4, y: 64, z: 0 };
    assert!(native.with_level(|level| store
        .mob_ai_mut(frog)
        .unwrap()
        .navigate_to(level, mob, target, 0, 1.0)));
    assert_eq!(
        store
            .mob_ai(frog)
            .unwrap()
            .pathfinding_malus()
            .get(GroundPathType::Water),
        0.0
    );

    let changed = BlockPos { x: 1, y: 64, z: 0 };
    native.set(changed, state("minecraft:water"));
    native.world.set_game_time_for_test(21);
    native.with_level(|level| store.recompute_ground_paths_affected_by(level, &[changed]));
    let path = store.ground_path(frog).unwrap();
    assert!(path.reached());
    assert!(path
        .nodes()
        .iter()
        .all(|node| node.path_type != GroundPathType::Water));
    assert_eq!(
        native
            .with_level(|level| store.effective_pathfinding_malus(store.get(frog).unwrap(), level))
            .unwrap()
            .get(GroundPathType::Water),
        -1.0
    );
    assert_eq!(
        store
            .mob_ai(frog)
            .unwrap()
            .pathfinding_malus()
            .get(GroundPathType::Water),
        0.0
    );
}

#[test]
fn three_level_chain_uses_detached_requester_and_checks_vehicle_no_ai() {
    let mut store = store();
    let bottom = mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    let middle = mob(&mut store, VANILLA_ENTITY_TYPE_PILLAGER_ID);
    let top = mob(&mut store, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    link(&mut store, bottom, middle);
    link(&mut store, middle, top);
    no_ai(&mut store, top);
    let requester = store.detach(top).unwrap();
    let mut native = controlling_level();
    assert_eq!(
        native.with_level(|level| store.mob_control_owner(&requester, level)),
        Some(bottom)
    );
    native = controlling_level();
    no_ai(&mut store, bottom);
    assert_eq!(
        native.with_level(|level| store.mob_control_owner(&requester, level)),
        Some(middle)
    );
    no_ai(&mut store, middle);
    assert_eq!(
        native.with_level(|level| store.mob_control_owner(&requester, level)),
        Some(top)
    );
}

#[test]
fn non_controlling_tag_blocks_control() {
    let mut native = controlling_level();
    deny_riders(&mut native);
    let mut store = store();
    let vehicle = mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    let first = mob(&mut store, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let second = mob(&mut store, VANILLA_ENTITY_TYPE_PILLAGER_ID);
    link(&mut store, vehicle, first);
    link(&mut store, vehicle, second);
    assert_eq!(
        native.with_level(|level| store.controlling_mob_passenger(store.get(vehicle).unwrap(), level)),
        Some(None)
    );
}

#[test]
fn mob_overrides_preserve_super_control_without_saddle_or_tame() {
    let mut native = controlling_level();
    for kind in [
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ] {
        let mut store = store();
        let vehicle = mob(&mut store, kind);
        let rider = mob(&mut store, VANILLA_ENTITY_TYPE_PILLAGER_ID);
        link(&mut store, vehicle, rider);
        assert_eq!(
            native.with_level(|level| store.mob_control_owner(store.get(rider).unwrap(), level)),
            Some(vehicle)
        );
    }
}

#[test]
fn non_mob_vehicle_and_an_actual_first_player_end_delegation() {
    let mut native = controlling_level();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "rider", 10000);
    let player = native
        .world
        .player_entity_id_for_test(crate::connection::ConnectionId(1));
    let counter = EntityIdCounter::new();
    for _ in 0..player {
        counter.next_id();
    }
    let mut store = EntityStore::new(7, counter);
    let vehicle = crate::entity::spawn_minecart(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        0.0,
    )
    .unwrap();
    let rider = mob(&mut store, VANILLA_ENTITY_TYPE_PILLAGER_ID);
    link(&mut store, vehicle, rider);
    assert_eq!(
        native.with_level(|level| store.mob_control_owner(store.get(rider).unwrap(), level)),
        Some(rider)
    );
    let pig = mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    assert!(store.attach_test_player(player, pig, crate::entity::ExternalPlayerCanRide::Allowed));
    assert_eq!(
        native.with_level(|level| store.controlling_mob_passenger(store.get(pig).unwrap(), level)),
        Some(None)
    );
}

#[test]
fn malus_inherits_only_the_direct_controlled_strider_map() {
    use crate::entity::ai::mob::navigation::GroundPathType;
    let mut store = store();
    let bottom = mob(&mut store, VANILLA_ENTITY_TYPE_STRIDER_ID);
    let middle = mob(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    let top = mob(&mut store, VANILLA_ENTITY_TYPE_PILLAGER_ID);
    link(&mut store, bottom, middle);
    link(&mut store, middle, top);
    let detached = store.detach(top).unwrap();
    let mut native = controlling_level();
    assert_eq!(
        native.with_level(|level| store.mob_control_owner(&detached, level)),
        Some(bottom)
    );
    assert_eq!(
        native
            .with_level(|level| store.effective_pathfinding_malus(&detached, level))
            .unwrap()
            .get(GroundPathType::Fire),
        16.0
    );
    let cow = store.get(middle).unwrap();
    assert_eq!(
        native
            .with_level(|level| store.effective_pathfinding_malus(cow, level))
            .unwrap()
            .get(GroundPathType::Fire),
        0.0
    );
    deny_riders(&mut native);
    assert_eq!(
        native
            .with_level(|level| store.effective_pathfinding_malus(cow, level))
            .unwrap()
            .get(GroundPathType::Fire),
        -1.0
    );
    native = controlling_level();
    no_ai(&mut store, bottom);
    assert_eq!(
        native.with_level(|level| store
            .effective_pathfinding_malus(store.get(middle).unwrap(), level))
            .unwrap()
            .get(GroundPathType::Fire),
        -1.0
    );
}

#[test]
fn detached_rider_inherits_and_noncontrolling_rider_keeps_own_malus() {
    let mut native = controlling_level();
    use crate::entity::ai::mob::navigation::GroundPathType;
    let mut store = store();
    let vehicle = mob(&mut store, VANILLA_ENTITY_TYPE_STRIDER_ID);
    let first = mob(&mut store, VANILLA_ENTITY_TYPE_FROG_ID);
    let second = mob(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    link(&mut store, vehicle, first);
    link(&mut store, vehicle, second);
    let detached = store.detach(first).unwrap();
    let (effective, inherited) = native
        .with_level(|level| store.effective_pathfinding_malus_with_provenance(&detached, level))
        .unwrap();
    assert!(inherited);
    assert_eq!(effective.get(GroundPathType::Water), -1.0);
    assert_eq!(
        store
            .mob_ai(first)
            .unwrap()
            .pathfinding_malus()
            .get(GroundPathType::Water),
        4.0
    );
    assert_eq!(
        native
            .with_level(|level| store.effective_pathfinding_malus(&detached, level))
            .unwrap()
            .get(GroundPathType::Lava),
        0.0
    );
    assert_eq!(
        native.with_level(|level| store
            .effective_pathfinding_malus(store.get(second).unwrap(), level))
            .unwrap()
            .get(GroundPathType::Fire),
        -1.0
    );
}

#[test]
fn a_bound_creaking_carries_the_transient_pathfinding_malus() {
    use crate::entity::ai::mob::navigation::GroundPathType;
    let mut store = store();
    let protector = mob(&mut store, VANILLA_ENTITY_TYPE_CREAKING_ID);
    let mut native = controlling_level();
    let unbound = native
        .with_level(|level| store.effective_pathfinding_malus(store.get(protector).unwrap(), level))
        .unwrap();
    assert_eq!(unbound.get(GroundPathType::Fire), 16.0);
    assert_eq!(unbound.get(GroundPathType::FireInNeighbor), 8.0);
    assert_eq!(unbound.get(GroundPathType::Damaging), -1.0);
    assert_eq!(unbound.get(GroundPathType::PowderSnow), -1.0);
    assert_eq!(unbound.get(GroundPathType::Lava), -1.0);

    assert!(store
        .get_mut(protector)
        .unwrap()
        .set_creaking_transient(BlockPos { x: 4, y: 64, z: -2 }));
    let bound = native
        .with_level(|level| store.effective_pathfinding_malus(store.get(protector).unwrap(), level))
        .unwrap();
    assert_eq!(bound.get(GroundPathType::Fire), 0.0);
    assert_eq!(bound.get(GroundPathType::FireInNeighbor), 0.0);
    assert_eq!(bound.get(GroundPathType::Damaging), 8.0);
    assert_eq!(bound.get(GroundPathType::PowderSnow), 8.0);
    assert_eq!(bound.get(GroundPathType::Lava), 8.0);
    assert_eq!(bound.get(GroundPathType::Water), 8.0);
    assert_eq!(bound.get(GroundPathType::Leaves), -1.0);
}
