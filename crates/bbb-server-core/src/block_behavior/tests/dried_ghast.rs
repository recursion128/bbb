use super::native_world::NativeWorld;

use super::*;

const GHAST: BlockPos = BlockPos { x: 2, y: 70, z: 3 };

const HYDRATION_TICK_DELAY: i64 = 5_000;

const MAX_HYDRATION_LEVEL: i32 = 3;

fn dried_ghast(hydration: i32, waterlogged: bool) -> i32 {
    dried_ghast_facing(hydration, waterlogged, "north")
}

fn dried_ghast_facing(hydration: i32, waterlogged: bool, facing: &str) -> i32 {
    state_of(
        "minecraft:dried_ghast",
        &[
            ("facing", facing),
            ("hydration", &hydration.to_string()),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

#[test]
fn a_soaking_ghast_saves_its_hydration_tick() {
    let mut world = NativeWorld::new();
    let soaking = dried_ghast(0, true);
    world.set(GHAST, soaking);

    world.random_tick(GHAST, 0);

    assert!(world
        .saved_block_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:dried_ghast"
            && tick.x == GHAST.x
            && tick.y == GHAST.y
            && tick.z == GHAST.z
            && i64::from(tick.delay) == HYDRATION_TICK_DELAY));
    assert_eq!(world.state(GHAST), soaking);
}

#[test]
fn a_drying_ghast_saves_the_hydration_tick_and_a_spent_one_stays_dry() {
    {
        let hydration = MAX_HYDRATION_LEVEL;
        let mut world = NativeWorld::new();
        world.set(GHAST, dried_ghast(hydration, false));

        world.random_tick(GHAST, 0);

        assert!(world
            .saved_block_ticks()
            .iter()
            .any(|tick| tick.id == "minecraft:dried_ghast"
                && tick.x == GHAST.x
                && tick.y == GHAST.y
                && tick.z == GHAST.z
                && i64::from(tick.delay) == HYDRATION_TICK_DELAY));
    }

    let mut spent = NativeWorld::new();
    spent.set(GHAST, dried_ghast(0, false));

    spent.random_tick(GHAST, 0);
    assert_eq!(spent.state(GHAST), dried_ghast(0, false));
}

#[test]
fn hydration_climbs_in_water_with_the_transition_sound() {
    for hydration in 0..MAX_HYDRATION_LEVEL {
        let mut world = NativeWorld::new();
        let before = dried_ghast(hydration, true);
        let after = dried_ghast(hydration + 1, true);
        world.set(GHAST, before);

        world.tick_block(GHAST);

        assert_eq!(world.state(GHAST), after, "hydration {hydration}");
        assert!(world.sounds().iter().any(|sound| sound.pos == GHAST
            && sound.resource_id == "minecraft:block.dried_ghast.transition"));
    }
}

#[test]
fn hydration_falls_out_of_water_silently_and_stops_at_zero() {
    for hydration in 1..=MAX_HYDRATION_LEVEL {
        let mut world = NativeWorld::new();
        let before = dried_ghast(hydration, false);
        let after = dried_ghast(hydration - 1, false);
        world.set(GHAST, before);

        world.tick_block(GHAST);

        assert_eq!(world.state(GHAST), after, "hydration {hydration}");
        assert!(world.sounds().is_empty(), "hydration {hydration}");
    }

    let mut spent = NativeWorld::new();
    let dry = dried_ghast(0, false);
    spent.set(GHAST, dry);

    spent.tick_block(GHAST);

    assert_eq!(spent.state(GHAST), dry);
}

#[test]
fn a_fully_hydrated_ghast_becomes_a_happy_ghast_facing_its_way() {
    for (facing, y_rot) in [("north", 180.0), ("east", -90.0)] {
        let mut world = NativeWorld::new();
        let ready = dried_ghast_facing(MAX_HYDRATION_LEVEL, true, facing);
        world.set(GHAST, ready);

        world.tick_block(GHAST);

        assert_eq!(world.state(GHAST), water(0), "facing {facing}");
        let entities = world.entities();
        let ghast = entities
            .iter()
            .find(|entity| {
                entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            })
            .unwrap();
        assert_eq!(
            ghast.position(),
            bbb_protocol::packets::Vec3d {
                x: f64::from(GHAST.x) + 0.5,
                y: f64::from(GHAST.y),
                z: f64::from(GHAST.z) + 0.5
            }
        );
        assert_eq!(ghast.y_rot(), y_rot);
        assert!(
            matches!(ghast.data(), crate::entity::EntityData::GeneralLiving(data) if data.is_baby())
        );
        assert!(world.items().is_empty(), "facing {facing}");
    }
}
