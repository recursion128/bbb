//! Placement reads redstone power from block entities the way the tick does.

use super::*;

const TARGET: BlockPos = BlockPos { x: 6, y: 65, z: 6 };
const SUPPORT: BlockPos = BlockPos { x: 6, y: 64, z: 6 };
const COMPARATOR: BlockPos = BlockPos { x: 7, y: 65, z: 6 };

/// A builder facing south at the target, a comparator east of it pointing its
/// output west at the target cell, and a redstone block feeding it.
fn world_beside_a_powered_comparator(
    item: &str,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("builder", 1)]);
    config.force_game_mode = true;
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    select_placement_item(&sender, &receiver, &mut world, item);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 6.5,
            y: 66.0,
            z: 6.5,
        },
        Some(PlayerRotation {
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    for (pos, state) in [
        (SUPPORT, block_state_id("minecraft:stone")),
        (
            BlockPos {
                y: 64,
                ..COMPARATOR
            },
            block_state_id("minecraft:stone"),
        ),
        (
            BlockPos { x: 8, ..COMPARATOR },
            block_state_id("minecraft:redstone_block"),
        ),
        (
            COMPARATOR,
            placement_state(
                "minecraft:comparator",
                &[("facing", "east"), ("mode", "compare"), ("powered", "true")],
            ),
        ),
    ] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            state,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }
    assert!(
        world.with_level_for_test(Java26BuiltinDimension::Overworld, |level| {
            level.set_comparator_block_entity_output_signal(COMPARATOR, 15)
        })
    );
    (sender, receiver, world)
}

/// `RepeaterBlock.getStateForPlacement` (`RepeaterBlock.java:57-59`) locks
/// against a powered comparator on its side (`DiodeBlock.java:137-145`).
#[test]
fn a_repeater_placed_beside_a_powered_comparator_lands_locked() {
    let (sender, receiver, mut world) = world_beside_a_powered_comparator("minecraft:repeater");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        SUPPORT,
        Direction::Up,
        0.5,
        8,
    );
    assert_eq!(
        live_block(&mut world, TARGET),
        Some(placement_state(
            "minecraft:repeater",
            &[
                ("delay", "1"),
                ("facing", "north"),
                ("locked", "true"),
                ("powered", "false"),
            ],
        ))
    );
}

/// `RedstoneLampBlock.getStateForPlacement` reads `hasNeighborSignal`, which
/// sees the comparator's block-entity output.
#[test]
fn a_lamp_placed_beside_a_powered_comparator_lands_lit() {
    let (sender, receiver, mut world) =
        world_beside_a_powered_comparator("minecraft:redstone_lamp");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        SUPPORT,
        Direction::Up,
        0.5,
        8,
    );
    assert_eq!(
        live_block(&mut world, TARGET),
        Some(placement_state(
            "minecraft:redstone_lamp",
            &[("lit", "true")]
        ))
    );
}
