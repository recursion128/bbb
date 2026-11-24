//! `CopperGolemStatueBlock` identity, pose cycle and analog.

use crate::fluid::FlowingFluid;
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TYPES};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::relative;

use super::{state_of, stone, NativeWorld};

const STATUE_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn statue_state(name: &str, pose: &str, waterlogged: bool) -> i32 {
    state_of(
        name,
        &[
            ("facing", "north"),
            ("copper_golem_pose", pose),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn statue(name: &str, pose: &str) -> i32 {
    statue_state(name, pose, false)
}

fn statue_entries() -> Vec<&'static bbb_protocol::block_types::BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::CopperGolemStatueBlock)
        })
        .collect()
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(STATUE_POS, state);
    world.set(relative(STATUE_POS, direction), stone());
    world.shape_update(STATUE_POS, direction);
    world.state(STATUE_POS)
}

/// `CopperGolemStatueBlock.Pose.getNextPose` (`:204-206`).
#[test]
fn a_statue_pose_wraps_standing_sitting_running_star() {
    use crate::block_behavior::copper_golem_statue::{analog_output, next_state, pose};
    let standing = statue("minecraft:copper_golem_statue", "standing");
    assert_eq!(pose(standing), Some("standing"));
    assert_eq!(analog_output(standing), 1);
    let sitting = next_state(standing).expect("standing has a next pose");
    assert_eq!(pose(sitting), Some("sitting"));
    assert_eq!(analog_output(sitting), 2);
    let running = next_state(sitting).expect("sitting has a next pose");
    assert_eq!(pose(running), Some("running"));
    assert_eq!(analog_output(running), 3);
    let star = next_state(running).expect("running has a next pose");
    assert_eq!(pose(star), Some("star"));
    assert_eq!(analog_output(star), 4);
    let wrapped = next_state(star).expect("star wraps");
    assert_eq!(pose(wrapped), Some("standing"));
    assert_eq!(analog_output(wrapped), 1);
}

#[test]
fn axes_and_honeycomb_are_the_two_items_the_click_passes() {
    use crate::block_behavior::copper_golem_statue::{item_is_axe, item_is_honeycomb};
    assert!(item_is_axe(Some(
        bbb_protocol::item_types::VANILLA_ITEM_WOODEN_AXE_ID
    )));
    assert!(item_is_axe(Some(
        bbb_protocol::item_types::VANILLA_ITEM_NETHERITE_AXE_ID
    )));
    assert!(!item_is_axe(Some(
        bbb_protocol::item_types::VANILLA_ITEM_HONEYCOMB_ID
    )));
    assert!(!item_is_axe(None));
    assert!(item_is_honeycomb(Some(
        bbb_protocol::item_types::VANILLA_ITEM_HONEYCOMB_ID
    )));
    assert!(!item_is_honeycomb(Some(
        bbb_protocol::item_types::VANILLA_ITEM_WOODEN_AXE_ID
    )));
}

#[test]
fn waterlogged_statue_rearms_water_on_neighbour_update() {
    for entry in statue_entries().into_iter().take(1) {
        let state = statue_state(entry.resource_id, "running", true);
        let mut world = NativeWorld::new();
        assert_eq!(update(&mut world, state, Direction::North), state);
        assert!(world.saved_fluid_ticks().iter().any(|tick| {
            tick.id == "minecraft:water"
                && (tick.x, tick.y, tick.z) == (STATUE_POS.x, STATUE_POS.y, STATUE_POS.z)
                && tick.delay == FlowingFluid::WATER_TICK_DELAY
        }));
    }
}
