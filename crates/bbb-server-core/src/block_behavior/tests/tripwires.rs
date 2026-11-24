use super::*;

const LEFT_HOOK: BlockPos = BlockPos { x: -2, y: 70, z: 0 };
const LEFT_WIRE: BlockPos = BlockPos { x: -1, y: 70, z: 0 };
const SOURCE: BlockPos = BlockPos { x: 0, y: 70, z: 0 };
const RIGHT_WIRE: BlockPos = BlockPos { x: 1, y: 70, z: 0 };
const RIGHT_HOOK: BlockPos = BlockPos { x: 2, y: 70, z: 0 };

fn hook(facing: &str, attached: bool, powered: bool) -> i32 {
    state_of(
        "minecraft:tripwire_hook",
        &[
            ("facing", facing),
            ("attached", bool_name(attached)),
            ("powered", bool_name(powered)),
        ],
    )
}

fn wire(attached: bool, powered: bool) -> i32 {
    state_of(
        "minecraft:tripwire",
        &[
            ("north", "false"),
            ("east", "true"),
            ("south", "false"),
            ("west", "true"),
            ("attached", bool_name(attached)),
            ("disarmed", "false"),
            ("powered", bool_name(powered)),
        ],
    )
}

const fn bool_name(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[test]
fn placing_a_wire_attaches_both_hooks_and_the_whole_line() {
    let mut world = NativeWorld::new();
    world.set(relative(LEFT_HOOK, Direction::West), stone());
    world.set(relative(RIGHT_HOOK, Direction::East), stone());
    for (pos, state) in [
        (LEFT_HOOK, hook("east", false, false)),
        (RIGHT_HOOK, hook("west", false, false)),
        (LEFT_WIRE, wire(false, false)),
        (RIGHT_WIRE, wire(false, false)),
        (SOURCE, wire(false, false)),
    ] {
        world.world.set_block(
            bbb_server_types::save::Java26BuiltinDimension::Overworld,
            pos,
            state,
            2 | 16,
        );
    }

    assert_eq!(world.state(LEFT_HOOK), hook("east", true, false));
    assert_eq!(world.state(RIGHT_HOOK), hook("west", true, false));
    assert_eq!(world.state(LEFT_WIRE), wire(true, false));
    assert_eq!(world.state(SOURCE), wire(true, false));
    assert_eq!(world.state(RIGHT_WIRE), wire(true, false));
    let sounds = world.sounds();
    for pos in [LEFT_HOOK, RIGHT_HOOK] {
        assert!(sounds.iter().any(|sound| sound.pos == pos
            && sound.resource_id == "minecraft:block.tripwire.attach"
            && sound.volume == 0.4
            && sound.pitch == 0.7));
    }
}

#[test]
fn horizontal_shape_updates_track_wire_and_hook_connections() {
    let isolated = state_of(
        "minecraft:tripwire",
        &[
            ("north", "false"),
            ("east", "false"),
            ("south", "false"),
            ("west", "false"),
            ("attached", "false"),
            ("disarmed", "false"),
            ("powered", "false"),
        ],
    );
    let mut world = NativeWorld::new();
    world.set(SOURCE, isolated);
    world.set(relative(RIGHT_WIRE, Direction::East), stone());
    world.set(RIGHT_WIRE, hook("west", false, false));
    world.shape_update(SOURCE, Direction::East);
    assert_eq!(bool_property(world.state(SOURCE), "east"), Some(true));
    world.set(RIGHT_WIRE, stone());
    world.shape_update(SOURCE, Direction::East);
    assert_eq!(bool_property(world.state(SOURCE), "east"), Some(false));
}
