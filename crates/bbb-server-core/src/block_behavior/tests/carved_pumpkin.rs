use super::native_world::NativeWorld;
use crate::level_event::LEVEL_EVENT_PARTICLES_DESTROY_BLOCK;
use bbb_protocol::packets::BlockPos;

use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID, VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID,
    VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
};
use bbb_protocol::NbtSummaryValue;

const HEAD: BlockPos = BlockPos { x: 0, y: 76, z: 0 };

fn pumpkin() -> i32 {
    default_state("minecraft:carved_pumpkin")
}

fn pumpkin_facing(facing: &str) -> i32 {
    state_of("minecraft:carved_pumpkin", &[("facing", facing)])
}

fn jack_o_lantern() -> i32 {
    default_state("minecraft:jack_o_lantern")
}

fn snow_block() -> i32 {
    default_state("minecraft:snow_block")
}

fn iron_block() -> i32 {
    default_state("minecraft:iron_block")
}

fn copper_block() -> i32 {
    default_state("minecraft:copper_block")
}

fn place(world: &mut NativeWorld, head: i32, old: i32) {
    world.world.set_block(
        bbb_server_types::save::Java26BuiltinDimension::Overworld,
        HEAD,
        old,
        18,
    );
    world.world.set_block(
        bbb_server_types::save::Java26BuiltinDimension::Overworld,
        HEAD,
        head,
        18,
    );
}

fn below_head(dy: i32) -> BlockPos {
    BlockPos {
        x: HEAD.x,
        y: HEAD.y + dy,
        z: HEAD.z,
    }
}

fn place_snow_body(world: &mut NativeWorld) {
    world.set(below_head(-1), snow_block());
    world.set(below_head(-2), snow_block());
}

fn place_iron_body(world: &mut NativeWorld) {
    world.set(
        BlockPos {
            x: HEAD.x - 1,
            y: HEAD.y - 1,
            z: HEAD.z,
        },
        iron_block(),
    );
    world.set(below_head(-1), iron_block());
    world.set(
        BlockPos {
            x: HEAD.x + 1,
            y: HEAD.y - 1,
            z: HEAD.z,
        },
        iron_block(),
    );
    world.set(below_head(-2), iron_block());
}

fn assert_pattern_cleared(world: &mut NativeWorld, cells: &[(BlockPos, i32)]) {
    let air = air();
    let events = world.level_events();
    for (pos, state) in cells {
        assert_eq!(world.state(*pos), air, "pattern cell {pos:?}");
        if *state != air {
            assert!(events.iter().any(|event| event.event_type
                == LEVEL_EVENT_PARTICLES_DESTROY_BLOCK
                && event.pos == *pos
                && event.data == *state));
        }
    }
}

#[test]
fn snow_golem_full_pattern_clears_and_spawns_a_golem() {
    let head = pumpkin();
    let snow = snow_block();
    let mut world = NativeWorld::new();
    world.set(HEAD, head);
    place_snow_body(&mut world);

    place(&mut world, head, air());

    let bottom = below_head(-2);
    let mid = below_head(-1);
    let entities = world.entities();
    let golem = entities
        .iter()
        .find(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID)
        .unwrap();
    assert_eq!(
        golem.position(),
        bbb_protocol::packets::Vec3d {
            x: f64::from(bottom.x) + 0.5,
            y: f64::from(bottom.y) + 0.05,
            z: f64::from(bottom.z) + 0.5,
        }
    );
    assert_eq!(golem.y_rot(), 0.0);

    assert_pattern_cleared(&mut world, &[(HEAD, head), (mid, snow), (bottom, snow)]);
}

#[test]
fn jack_o_lantern_is_a_pumpkin_head_for_snow_golem() {
    let head = jack_o_lantern();
    let mut world = NativeWorld::new();
    world.set(HEAD, head);
    place_snow_body(&mut world);

    place(&mut world, head, air());

    assert!(world
        .entities()
        .iter()
        .any(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID));
    assert_eq!(world.state(HEAD), air());
    assert_eq!(world.state(below_head(-1)), air());
    assert_eq!(world.state(below_head(-2)), air());
}

#[test]
fn iron_golem_full_pattern_sets_player_created() {
    let head = pumpkin();
    let iron = iron_block();
    let air = air();
    let mut world = NativeWorld::new();
    world.set(HEAD, head);
    place_iron_body(&mut world);

    place(&mut world, head, air);

    let bottom = below_head(-2);
    let mid = below_head(-1);
    let west = BlockPos {
        x: HEAD.x - 1,
        y: HEAD.y - 1,
        z: HEAD.z,
    };
    let east = BlockPos {
        x: HEAD.x + 1,
        y: HEAD.y - 1,
        z: HEAD.z,
    };
    let entities = world.entities();
    let golem = entities
        .iter()
        .find(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_IRON_GOLEM_ID)
        .unwrap();
    assert_eq!(
        golem.position(),
        bbb_protocol::packets::Vec3d {
            x: f64::from(bottom.x) + 0.5,
            y: f64::from(bottom.y) + 0.05,
            z: f64::from(bottom.z) + 0.5,
        }
    );
    assert_eq!(golem.y_rot(), 0.0);
    assert!(
        matches!(golem.data(), crate::entity::EntityData::GeneralLiving(data) if data.opaque_fields().iter().any(|field| field.name == "PlayerCreated" && field.value == NbtSummaryValue::Byte(1)))
    );
    assert_pattern_cleared(
        &mut world,
        &[
            (
                BlockPos {
                    x: HEAD.x + 1,
                    y: HEAD.y,
                    z: HEAD.z,
                },
                air,
            ),
            (east, iron),
            (
                BlockPos {
                    x: HEAD.x + 1,
                    y: HEAD.y - 2,
                    z: HEAD.z,
                },
                air,
            ),
            (HEAD, head),
            (mid, iron),
            (bottom, iron),
            (
                BlockPos {
                    x: HEAD.x - 1,
                    y: HEAD.y,
                    z: HEAD.z,
                },
                air,
            ),
            (west, iron),
            (
                BlockPos {
                    x: HEAD.x - 1,
                    y: HEAD.y - 2,
                    z: HEAD.z,
                },
                air,
            ),
        ],
    );
}

#[test]
fn copper_golem_full_pattern_replaces_copper_with_chest() {
    let head = pumpkin_facing("south");
    let copper = copper_block();
    let mut world = NativeWorld::new();
    world.set(HEAD, head);
    world.set(below_head(-1), copper);

    place(&mut world, head, air());

    let copper_pos = below_head(-1);
    let entities = world.entities();
    let golem = entities
        .iter()
        .find(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID)
        .unwrap();
    assert_eq!(
        golem.position(),
        bbb_protocol::packets::Vec3d {
            x: f64::from(HEAD.x) + 0.5,
            y: f64::from(HEAD.y) + 0.05,
            z: f64::from(HEAD.z) + 0.5,
        }
    );
    assert_eq!(golem.y_rot(), 0.0);
    assert!(
        matches!(golem.data(), crate::entity::EntityData::GeneralLiving(data) if crate::entity::copper_golem::weather_state(data) == VANILLA_COPPER_GOLEM_WEATHER_STATE_UNAFFECTED_ID)
    );

    assert_eq!(world.state(HEAD), air());
    assert_eq!(
        world.state(copper_pos),
        state_of(
            "minecraft:copper_chest",
            &[
                ("facing", "south"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        )
    );
}

#[test]
fn copper_golem_weather_follows_oxidized_and_waxed_bodies() {
    let head = pumpkin();
    let oxidized = default_state("minecraft:oxidized_copper");
    let mut oxidized_world = NativeWorld::new();
    oxidized_world.set(HEAD, head);
    oxidized_world.set(below_head(-1), oxidized);
    place(&mut oxidized_world, head, air());
    assert!(oxidized_world.entities().iter().any(|entity| matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data) if crate::entity::copper_golem::weather_state(data) == VANILLA_COPPER_GOLEM_WEATHER_STATE_OXIDIZED_ID)));
    assert_eq!(
        oxidized_world.state(below_head(-1)),
        state_of(
            "minecraft:oxidized_copper_chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        )
    );

    let waxed = default_state("minecraft:waxed_exposed_copper");
    let mut waxed_world = NativeWorld::new();
    waxed_world.set(HEAD, head);
    waxed_world.set(below_head(-1), waxed);
    place(&mut waxed_world, head, air());
    assert!(waxed_world.entities().iter().any(|entity| matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data) if crate::entity::copper_golem::weather_state(data) == VANILLA_COPPER_GOLEM_WEATHER_STATE_EXPOSED_ID)));
    assert_eq!(
        waxed_world.state(below_head(-1)),
        state_of(
            "minecraft:exposed_copper_chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        )
    );
}

#[test]
fn same_block_on_place_is_a_no_op_even_with_a_full_pattern() {
    let head = pumpkin();
    let mut world = NativeWorld::new();
    world.set(HEAD, head);
    place_snow_body(&mut world);

    place(&mut world, head, pumpkin_facing("south"));

    assert!(world.entities().is_empty());
    assert_eq!(world.state(HEAD), head);
    assert_eq!(world.state(below_head(-1)), snow_block());
    assert_eq!(world.state(below_head(-2)), snow_block());
    assert!(world.level_events().is_empty());
}

#[test]
fn incomplete_pattern_is_a_no_op() {
    let head = pumpkin();
    let mut world = NativeWorld::new();
    world.set(HEAD, head);
    world.set(below_head(-1), snow_block());

    place(&mut world, head, air());

    assert!(world.entities().is_empty());
    assert_eq!(world.state(HEAD), head);
    assert_eq!(world.state(below_head(-1)), snow_block());
    assert!(world.level_events().is_empty());
}
