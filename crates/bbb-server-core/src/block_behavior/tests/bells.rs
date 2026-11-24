use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::{BlockPos, Direction};

use super::{state_of, stone, NativeWorld};

const BELL: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const FLOOR: BlockPos = BlockPos { x: 0, y: 63, z: 0 };
const SIDE: BlockPos = BlockPos { x: 1, y: 64, z: 0 };

fn bell(attachment: &str, facing: &str, powered: bool) -> i32 {
    state_of(
        "minecraft:bell",
        &[
            ("facing", facing),
            ("attachment", attachment),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn powered(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("powered")
        .cloned()
}

fn redstone_block() -> i32 {
    super::default_state("minecraft:redstone_block")
}

#[test]
fn a_floor_bell_rings_from_its_facing_axis() {
    use crate::block_behavior::bell::is_proper_hit;
    let floor = bell("floor", "north", false);
    assert!(is_proper_hit(floor, Direction::North, 0.5));
    assert!(is_proper_hit(floor, Direction::South, 0.5));
    assert!(!is_proper_hit(floor, Direction::East, 0.5));
    assert!(!is_proper_hit(floor, Direction::Up, 0.5));
    assert!(!is_proper_hit(floor, Direction::North, 0.9));
}

#[test]
fn a_floor_bell_survives_on_stone() {
    use crate::block_behavior::can_survive;
    let mut at = |pos: BlockPos| {
        if pos == FLOOR {
            stone()
        } else {
            super::air()
        }
    };
    assert_eq!(
        can_survive(
            bell("floor", "north", false),
            BELL,
            super::vanilla_tags(),
            &mut at
        ),
        Some(true)
    );
}

#[test]
fn a_bell_powers_and_queues_a_ring_when_it_has_a_neighbour_signal() {
    let mut world = NativeWorld::ticking(BELL);
    world.set(FLOOR, stone());
    world.set(BELL, bell("floor", "north", false));
    world.set(SIDE, redstone_block());
    world.neighbor_changed(
        BELL,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(powered(world.state(BELL)).as_deref(), Some("true"));
    world.run_tick();
    assert!(world
        .world
        .get_block_entity(
            bbb_server_types::save::Java26BuiltinDimension::Overworld,
            BELL
        )
        .and_then(crate::block_entity::BlockEntity::bell)
        .unwrap()
        .shaking());
}
