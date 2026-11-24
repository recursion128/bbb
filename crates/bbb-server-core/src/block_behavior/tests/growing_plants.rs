//! The growing-plant family: `GrowingPlantBlock`, `GrowingPlantHeadBlock` and
//! `GrowingPlantBodyBlock` over kelp, the two nether vines and cave vines.

use super::*;

// ---------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------

fn kelp(age: i32) -> i32 {
    state_of("minecraft:kelp", &[("age", &age.to_string())])
}

fn kelp_plant() -> i32 {
    state_of("minecraft:kelp_plant", &[])
}

fn twisting_vines(age: i32) -> i32 {
    state_of("minecraft:twisting_vines", &[("age", &age.to_string())])
}

fn weeping_vines(age: i32) -> i32 {
    state_of("minecraft:weeping_vines", &[("age", &age.to_string())])
}

fn weeping_vines_plant() -> i32 {
    state_of("minecraft:weeping_vines_plant", &[])
}

fn cave_vines(age: i32, berries: bool) -> i32 {
    state_of(
        "minecraft:cave_vines",
        &[
            ("age", &age.to_string()),
            ("berries", if berries { "true" } else { "false" }),
        ],
    )
}

fn cave_vines_plant(berries: bool) -> i32 {
    state_of(
        "minecraft:cave_vines_plant",
        &[("berries", if berries { "true" } else { "false" })],
    )
}

fn magma_block() -> i32 {
    state_of("minecraft:magma_block", &[])
}

fn netherrack() -> i32 {
    state_of("minecraft:netherrack", &[])
}

const SEED_GROWS_WITHOUT_BERRIES: i64 = 4096;

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// `GrowingPlantBlock.canSurvive`
// ---------------------------------------------------------------------------

#[test]
fn a_plant_hangs_off_a_sturdy_face_or_off_more_of_itself() {
    let below_is = |state: i32| {
        move |pos: BlockPos| {
            if pos == at(0, 69, 0) {
                state
            } else {
                air()
            }
        }
    };
    assert_eq!(
        can_survive(kelp(0), at(0, 70, 0), vanilla_tags(), &mut below_is(dirt())),
        Some(true)
    );
    assert_eq!(
        can_survive(
            kelp_plant(),
            at(0, 70, 0),
            vanilla_tags(),
            &mut below_is(kelp_plant())
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(kelp(0), at(0, 70, 0), vanilla_tags(), &mut below_is(air())),
        Some(false)
    );
    assert_eq!(
        can_survive(
            kelp(0),
            at(0, 70, 0),
            vanilla_tags(),
            &mut below_is(magma_block())
        ),
        Some(false)
    );
    assert_eq!(
        can_survive(
            twisting_vines(0),
            at(0, 70, 0),
            vanilla_tags(),
            &mut below_is(magma_block())
        ),
        Some(true)
    );
}

#[test]
fn a_downward_plant_reads_the_cell_above_it() {
    let above_is = |state: i32| {
        move |pos: BlockPos| {
            if pos == at(0, 71, 0) {
                state
            } else {
                air()
            }
        }
    };
    assert_eq!(
        can_survive(
            weeping_vines(0),
            at(0, 70, 0),
            vanilla_tags(),
            &mut above_is(netherrack())
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(
            cave_vines_plant(true),
            at(0, 70, 0),
            vanilla_tags(),
            &mut above_is(cave_vines(0, false))
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(
            cave_vines(0, false),
            at(0, 70, 0),
            vanilla_tags(),
            &mut above_is(air())
        ),
        Some(false)
    );
}

// ---------------------------------------------------------------------------
// `GrowingPlantHeadBlock.randomTick`
// ---------------------------------------------------------------------------

#[test]
fn kelp_grows_up_into_water_and_leaves_a_kelp_plant_behind() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), dirt());
    world.set(at(0, 70, 0), kelp(0));
    world.set(at(0, 71, 0), water(0));

    let seed = SEED_GROWS_WITHOUT_BERRIES;
    world.random_tick(at(0, 70, 0), seed);

    assert_eq!(world.state(at(0, 71, 0)), kelp(1));
    assert_eq!(world.state(at(0, 70, 0)), kelp_plant());
}

#[test]
fn weeping_vines_grow_down_into_air() {
    let mut world = NativeWorld::new();
    world.set(at(0, 74, 0), netherrack());
    world.set(at(0, 73, 0), weeping_vines(7));

    let seed = SEED_GROWS_WITHOUT_BERRIES;
    world.random_tick(at(0, 73, 0), seed);

    assert_eq!(world.state(at(0, 72, 0)), weeping_vines(8));
    assert_eq!(world.state(at(0, 73, 0)), weeping_vines_plant());
}

#[test]
fn a_head_at_max_age_never_grows() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), dirt());
    world.set(at(0, 70, 0), kelp(growing_plant::MAX_AGE));
    world.set(at(0, 71, 0), water(0));

    let seed = SEED_GROWS_WITHOUT_BERRIES;
    world.random_tick(at(0, 70, 0), seed);

    assert_eq!(world.state(at(0, 71, 0)), water(0));
}

#[test]
fn kelp_only_grows_into_water_and_the_vines_only_into_air() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), dirt());
    world.set(at(0, 70, 0), kelp(0));
    let seed = SEED_GROWS_WITHOUT_BERRIES;
    world.random_tick(at(0, 70, 0), seed);
    assert_eq!(world.state(at(0, 71, 0)), air());

    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), netherrack());
    world.set(at(0, 70, 0), twisting_vines(0));
    world.set(at(0, 71, 0), water(0));
    let seed = SEED_GROWS_WITHOUT_BERRIES;
    world.random_tick(at(0, 70, 0), seed);
    assert_eq!(world.state(at(0, 71, 0)), water(0));
}

#[test]
fn a_cave_vine_draws_its_berries_with_every_growth_and_hands_them_down() {
    // The new head carries berries at 0.11: about 33 of 300 growths, and both
    // outcomes miss with odds under 1e-14.
    let mut world = NativeWorld::new();
    world.set(at(0, 74, 0), stone());
    let mut berries = 0;
    for seed in 0..300 {
        world.set(at(0, 72, 0), air());
        world.set(at(0, 73, 0), cave_vines(3, true));
        world.random_tick(at(0, 73, 0), seed);
        let head = world.state(at(0, 72, 0));
        assert!(head == cave_vines(4, true) || head == cave_vines(4, false));
        berries += usize::from(head == cave_vines(4, true));
        assert_eq!(world.state(at(0, 73, 0)), cave_vines_plant(true));
    }
    assert!((1..300).contains(&berries), "{berries}");
}

// ---------------------------------------------------------------------------
// `updateShape`
// ---------------------------------------------------------------------------

#[test]
fn cutting_a_column_turns_the_body_at_the_cut_into_a_new_head() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), dirt());
    world.set(at(0, 70, 0), kelp_plant());
    world.set(at(0, 71, 0), kelp_plant());
    world.set(at(0, 72, 0), water(0));

    world.shape_update(at(0, 71, 0), Direction::Up);

    let head = world.state(at(0, 71, 0));
    assert_eq!(block_type_of(head), block_type_of(kelp(0)));
    assert!((0..25).contains(&int_property(head, "age").unwrap()));
    assert_eq!(world.state(at(0, 70, 0)), kelp_plant());
}

#[test]
fn a_cut_cave_vine_carries_its_berries_up_into_the_new_head() {
    let mut world = NativeWorld::new();
    world.set(at(0, 74, 0), stone());
    world.set(at(0, 73, 0), cave_vines_plant(true));
    world.set(at(0, 72, 0), air());

    world.shape_update(at(0, 73, 0), Direction::Down);

    let head = world.state(at(0, 73, 0));
    assert_eq!(block_type_of(head), block_type_of(cave_vines(0, true)));
    assert_eq!(bool_property(head, "berries"), Some(true));
    assert!((0..25).contains(&int_property(head, "age").unwrap()));
}

#[test]
fn losing_the_support_arms_the_tick_that_destroys_the_column() {
    let mut world = NativeWorld::ticking(at(0, 70, 0));
    world.set(at(0, 69, 0), dirt());
    world.set(at(0, 70, 0), kelp_plant());
    world.set(at(0, 71, 0), kelp(0));

    world.set(at(0, 69, 0), air());
    world.shape_update(at(0, 70, 0), Direction::Down);
    world.run_tick();
    // `Level.destroyBlock` leaves the cell holding whatever fluid the state
    // carried, which for `kelp_plant` is a water source
    // (`KelpPlantBlock.getFluidState`).
    assert_eq!(world.state(at(0, 70, 0)), water(0));
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:kelp" && *count > 0));
}

#[test]
fn a_head_that_meets_the_plant_ahead_of_it_becomes_body() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), dirt());
    world.set(at(0, 70, 0), kelp(4));
    world.set(at(0, 71, 0), kelp(5));

    world.shape_update(at(0, 70, 0), Direction::Down);

    assert_eq!(world.state(at(0, 70, 0)), kelp_plant());
}
