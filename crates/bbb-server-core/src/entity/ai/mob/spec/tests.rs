use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{tick_one_entity, EntityIdCounter, EntityStore};

fn wandered(native: &mut NativeLevel, entity_type_id: i32, start: Vec3d, ticks: usize) -> f64 {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let id = store.spawn_general_living(entity_type_id, start).unwrap();
    let mut farthest = 0.0_f64;
    for _ in 0..ticks {
        native.with_level(|level| {
            tick_one_entity(&mut store, level, id);
        });
        let position = store.get(id).unwrap().position();
        farthest = farthest.max((position.x - start.x).hypot(position.z - start.z));
    }
    farthest
}

fn loaded_level() -> NativeLevel {
    let mut native = NativeLevel::new();
    let chunks = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |z| bbb_protocol::packets::ChunkPos { x, z }))
        .collect::<Vec<_>>();
    native
        .world
        .set_chunks_forced(DIMENSION.into(), true, &chunks);
    native
}

// The fixture has no light data, so land strolls cannot score positions;
// the registered goals are what the goal debug stream shows.
#[test]
fn armadillos_zoglins_shulkers_and_pandas_register_their_idle_goals() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
    };
    for (kind, wanted) in [
        (
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            &[
                "FollowParentGoal",
                "LookAtPlayerGoal",
                "RandomStrollGoal",
                "RandomLookAroundGoal",
            ][..],
        ),
        (VANILLA_ENTITY_TYPE_ZOGLIN_ID, &["HoglinIdleGoal"][..]),
        (
            VANILLA_ENTITY_TYPE_SHULKER_ID,
            &["LookAtPlayerGoal", "RandomLookAroundGoal"][..],
        ),
        (VANILLA_ENTITY_TYPE_PANDA_ID, &["LookAtPlayerGoal"][..]),
    ] {
        let runtime = crate::entity::ai::mob::MobAiRuntime::for_entity(
            kind,
            crate::random::GameRandom::with_seed(0),
        );
        let names: Vec<_> = runtime
            .goal_debug_info()
            .goals
            .into_iter()
            .map(|goal| goal.name)
            .collect();
        for name in wanted {
            assert!(names.iter().any(|have| have == name), "{kind}: {names:?}");
        }
    }
}

#[test]
fn swimming_turtles_travel() {
    let mut native = loaded_level();
    let water = state("minecraft:water");
    for x in -12..=28 {
        for y in 70..=73 {
            for z in -12..=28 {
                native.set(BlockPos { x, y, z }, water);
            }
        }
    }
    let start = Vec3d {
        x: 8.5,
        y: 71.0,
        z: 8.5,
    };
    let distance = wandered(&mut native, VANILLA_ENTITY_TYPE_TURTLE_ID, start, 300);
    assert!(distance > 3.0, "{distance}");
}
