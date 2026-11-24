//! `/worldborder`.

use crate::connection::ClientboundPlayPacket;
use crate::tick_rate::java_fixed_decimals;
use bbb_protocol::packets::{
    SetBorderCenter, SetBorderLerpSize, SetBorderSize, SetBorderWarningDelay,
    SetBorderWarningDistance,
};
use bbb_server_types::interaction::WorldBorder;

use super::*;

pub(super) fn static_border(size: f64) -> WorldBorder {
    WorldBorder::from_settings(0.0, 0.0, 0.2, 5.0, 5, 300, size, 0, size, 0)
}

/// A world with one overworld player to receive the border packets.
fn view_with(border: WorldBorder, game_time: i64) -> CommandTestWorld {
    world(&[user("bob", 2)])
        .with_game_time(game_time)
        .with_border(Java26BuiltinDimension::Overworld.into(), border)
}

fn sized(size: f64) -> CommandTestWorld {
    view_with(static_border(size), 0)
}

fn overworld_border(world: &CommandTestWorld) -> WorldBorder {
    world.border(Java26BuiltinDimension::Overworld.into())
}

fn border_packet(run: &CommandRun) -> Option<&ClientboundPlayPacket> {
    let packets: Vec<_> = run
        .packets()
        .into_iter()
        .filter(|packet| {
            matches!(
                packet,
                ClientboundPlayPacket::SetBorderSize(_)
                    | ClientboundPlayPacket::SetBorderLerpSize(_)
                    | ClientboundPlayPacket::SetBorderCenter(_)
                    | ClientboundPlayPacket::SetBorderWarningDelay(_)
                    | ClientboundPlayPacket::SetBorderWarningDistance(_)
            )
        })
        .collect();
    match packets.as_slice() {
        [] => None,
        [packet] => Some(packet),
        other => panic!("expected at most one border packet, got {other:?}"),
    }
}

/// `LEVEL_GAMEMASTERS` (WorldBorderCommand.java:52); below it the root is gone.
#[test]
fn worldborder_needs_gamemaster() {
    let refusal = refused("worldborder get", CommandPermission::Moderator);
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

/// `getSize` (WorldBorderCommand.java:169-173): floor(size+0.5), `sendSuccess`
/// with `%.0f` and no admin copy.
#[test]
fn worldborder_get_reports_the_live_size() {
    let size = 100.4;
    let run = ran("worldborder get", sized(size));
    assert_eq!(run.result, Some((size + 0.5).floor() as i32));
    assert_eq!(keys(&run), ["commands.worldborder.get"]);
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::string(java_fixed_decimals(size, 0))]
    );
    assert!(!run.feedback[0].inform_admins);
    assert_eq!(border_packet(&run), None);
}

/// `setSize` refusals (WorldBorderCommand.java:197-207).
#[test]
fn worldborder_set_refuses_same_size_too_small_and_too_big() {
    let same = ran("worldborder set 100", sized(100.0));
    assert_eq!(keys(&same), ["commands.worldborder.set.failed.nochange"]);
    assert_eq!(border_packet(&same), None);

    let small = ran("worldborder set 0.5", sized(100.0));
    assert_eq!(keys(&small), ["commands.worldborder.set.failed.small"]);

    let too_big = ran("worldborder add 1", CommandTestWorld::default());
    assert_eq!(keys(&too_big), ["commands.worldborder.set.failed.big"]);
    assert_eq!(
        arguments(&too_big, 0),
        vec![NbtSummaryValue::Float(5.999997E7_f32.to_bits())]
    );
}

/// Immediate `set` / `add` (WorldBorderCommand.java:217-222).
#[test]
fn worldborder_set_and_add_write_an_immediate_size() {
    let mut world = sized(100.0);
    let set = ran("worldborder set 80", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .size_at(0),
        80.0
    );
    assert_eq!(set.result, Some(-20));
    assert_eq!(keys(&set), ["commands.worldborder.set.immediate"]);
    assert_eq!(
        arguments(&set, 0),
        vec![NbtSummaryValue::string(java_fixed_decimals(80.0, 1))]
    );
    assert!(set.feedback[0].inform_admins);
    assert_eq!(overworld_border(&world), static_border(80.0));
    assert_eq!(
        border_packet(&set),
        Some(&ClientboundPlayPacket::SetBorderSize(SetBorderSize {
            size: 80.0
        }))
    );

    let mut world = sized(100.0);
    let add = ran("worldborder add -10", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .size_at(0),
        90.0
    );
    assert_eq!(add.result, Some(-10));
    assert_eq!(
        border_packet(&add),
        Some(&ClientboundPlayPacket::SetBorderSize(SetBorderSize {
            size: 90.0
        }))
    );
}

/// `ticks > 0` uses `lerpSizeBetween` (WorldBorderCommand.java:210-216).
#[test]
fn worldborder_set_with_time_lerps_and_words_grow_or_shrink() {
    let mut world = sized(100.0);
    let grow = ran("worldborder set 200 40t", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .size_at(20),
        150.0
    );
    assert_eq!(grow.result, Some(100));
    assert_eq!(keys(&grow), ["commands.worldborder.set.grow"]);
    assert_eq!(
        arguments(&grow, 0),
        vec![
            NbtSummaryValue::string(java_fixed_decimals(200.0, 1)),
            NbtSummaryValue::string(java_fixed_decimals(2.0, 2)),
        ]
    );
    assert_eq!(
        overworld_border(&world),
        WorldBorder::from_settings(0.0, 0.0, 0.2, 5.0, 5, 300, 100.0, 40, 200.0, 0)
    );
    assert_eq!(
        border_packet(&grow),
        Some(&ClientboundPlayPacket::SetBorderLerpSize(
            SetBorderLerpSize {
                old_size: 100.0,
                new_size: 200.0,
                lerp_time: 40,
            }
        ))
    );

    let mut world = sized(100.0);
    let shrink = ran("worldborder set 50 20t", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .size_at(10),
        75.0
    );
    assert_eq!(keys(&shrink), ["commands.worldborder.set.shrink"]);
    let Some(ClientboundPlayPacket::SetBorderLerpSize(SetBorderLerpSize { lerp_time, .. })) =
        border_packet(&shrink)
    else {
        panic!("set with time lerps");
    };
    assert_eq!(*lerp_time, 20);
}

/// `add` with time adds the remaining lerp (WorldBorderCommand.java:62-67).
#[test]
fn worldborder_add_with_time_extends_the_remaining_lerp() {
    let moving = WorldBorder::from_settings(0.0, 0.0, 0.2, 5.0, 5, 300, 100.0, 40, 200.0, 0);
    let mut world = view_with(moving, 0);
    let run = ran("worldborder add 10 10t", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .remaining_lerp_time(0),
        50
    );
    let Some(ClientboundPlayPacket::SetBorderLerpSize(SetBorderLerpSize {
        old_size,
        new_size,
        lerp_time,
    })) = border_packet(&run)
    else {
        panic!("add with time lerps");
    };
    assert_eq!((*old_size, *new_size, *lerp_time), (100.0, 110.0, 50));
    assert_eq!(overworld_border(&world).lerp_target(), 110.0);
}

/// `setCenter` (WorldBorderCommand.java:175-191). `0 0` centre-corrects.
#[test]
fn worldborder_center_refuses_same_and_too_far_and_writes_a_new_center() {
    let same = ran("worldborder center 0.0 0.0", sized(100.0));
    assert_eq!(keys(&same), ["commands.worldborder.center.failed"]);

    let relative = ran("worldborder center ~ ~", sized(100.0));
    assert_eq!(keys(&relative), ["commands.worldborder.center.failed"]);

    let far = ran("worldborder center 30000000 0", sized(100.0));
    assert_eq!(keys(&far), ["commands.worldborder.set.failed.far"]);
    assert_eq!(
        arguments(&far, 0),
        vec![NbtSummaryValue::Double(2.9999984E7_f64.to_bits())]
    );

    let mut world = sized(100.0);
    let moved = ran("worldborder center 0 0", &mut world);
    assert_eq!(
        (
            world
                .border(Java26BuiltinDimension::Overworld.into())
                .center_x(),
            world
                .border(Java26BuiltinDimension::Overworld.into())
                .center_z()
        ),
        (0.5, 0.5)
    );
    assert_eq!(moved.result, Some(0));
    assert_eq!(keys(&moved), ["commands.worldborder.center.success"]);
    assert_eq!(
        arguments(&moved, 0),
        vec![
            NbtSummaryValue::string(java_fixed_decimals(0.5, 2)),
            NbtSummaryValue::string(java_fixed_decimals(0.5, 2)),
        ]
    );
    assert_eq!(
        overworld_border(&world),
        WorldBorder::from_settings(0.5, 0.5, 0.2, 5.0, 5, 300, 100.0, 0, 100.0, 0)
    );
    assert_eq!(
        border_packet(&moved),
        Some(&ClientboundPlayPacket::SetBorderCenter(SetBorderCenter {
            new_center_x: 0.5,
            new_center_z: 0.5,
        }))
    );
}

/// Damage amount/buffer (WorldBorderCommand.java:125-145): no play packet.
#[test]
fn worldborder_damage_writes_amount_and_buffer_without_a_packet() {
    let mut world = sized(100.0);
    let amount = ran("worldborder damage amount 1.5", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .damage_per_block(),
        1.5
    );
    assert_eq!(amount.result, Some(1));
    assert_eq!(
        keys(&amount),
        ["commands.worldborder.damage.amount.success"]
    );
    assert_eq!(
        arguments(&amount, 0),
        vec![NbtSummaryValue::string(java_fixed_decimals(1.5, 2))]
    );
    assert_eq!(border_packet(&amount), None);

    let same_amount = ran(
        "worldborder damage amount 1.5",
        view_with(
            WorldBorder::from_settings(0.0, 0.0, 1.5, 5.0, 5, 300, 100.0, 0, 100.0, 0),
            0,
        ),
    );
    assert_eq!(
        keys(&same_amount),
        ["commands.worldborder.damage.amount.failed"]
    );

    let mut world = sized(100.0);
    let buffer = ran("worldborder damage buffer 8", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .safe_zone(),
        8.0
    );
    assert_eq!(
        keys(&buffer),
        ["commands.worldborder.damage.buffer.success"]
    );
    assert_eq!(border_packet(&buffer), None);

    let same_buffer = ran("worldborder damage buffer 5", sized(100.0));
    assert_eq!(
        keys(&same_buffer),
        ["commands.worldborder.damage.buffer.failed"]
    );
}

/// Warning time/distance (WorldBorderCommand.java:147-167).
#[test]
fn worldborder_warning_writes_time_and_distance() {
    let mut world = sized(100.0);
    let time = ran("worldborder warning time 40t", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .warning_time(),
        40
    );
    assert_eq!(time.result, Some(40));
    assert_eq!(keys(&time), ["commands.worldborder.warning.time.success"]);
    assert_eq!(
        arguments(&time, 0),
        vec![NbtSummaryValue::string(java_fixed_decimals(2.0, 2))]
    );
    assert_eq!(
        overworld_border(&world),
        WorldBorder::from_settings(0.0, 0.0, 0.2, 5.0, 5, 40, 100.0, 0, 100.0, 0)
    );
    assert_eq!(
        border_packet(&time),
        Some(&ClientboundPlayPacket::SetBorderWarningDelay(
            SetBorderWarningDelay { warning_delay: 40 }
        ))
    );

    let same_time = ran("worldborder warning time 15s", sized(100.0));
    assert_eq!(
        keys(&same_time),
        ["commands.worldborder.warning.time.failed"]
    );

    let mut world = sized(100.0);
    let distance = ran("worldborder warning distance 12", &mut world);
    assert_eq!(
        world
            .border(Java26BuiltinDimension::Overworld.into())
            .warning_blocks(),
        12
    );
    assert_eq!(
        keys(&distance),
        ["commands.worldborder.warning.distance.success"]
    );
    assert_eq!(
        overworld_border(&world),
        WorldBorder::from_settings(0.0, 0.0, 0.2, 5.0, 12, 300, 100.0, 0, 100.0, 0)
    );
    assert_eq!(
        border_packet(&distance),
        Some(&ClientboundPlayPacket::SetBorderWarningDistance(
            SetBorderWarningDistance { warning_blocks: 12 }
        ))
    );

    let same_distance = ran("worldborder warning distance 5", sized(100.0));
    assert_eq!(
        keys(&same_distance),
        ["commands.worldborder.warning.distance.failed"]
    );
}

/// `/execute in` retargets `source.getLevel()` (WorldBorderCommand.java:195).
#[test]
fn worldborder_reads_the_source_levels_border() {
    let mut world = CommandTestWorld::default()
        .with_border(
            Java26BuiltinDimension::Overworld.into(),
            static_border(100.0),
        )
        .with_border(Java26BuiltinDimension::Nether.into(), static_border(50.0));
    let overworld = ran("worldborder get", &mut world);
    assert_eq!(overworld.result, Some(100));
    let nether = ran(
        "execute in minecraft:the_nether run worldborder get",
        &mut world,
    );
    assert_eq!(nether.result, Some(50));
}

/// `Vec2Argument.parse` (Vec2Argument.java:43-58).
#[test]
fn worldborder_center_reads_a_vec2_and_refuses_an_incomplete_or_local_form() {
    let (end, value) = read_vec2(&("0 0").encode_utf16().collect::<Vec<_>>(), 0).expect("two axes");
    assert_eq!(end, "0 0".len());
    let ArgumentValue::Vec2(coordinates) = value else {
        panic!("vec2 reads as Vec2");
    };
    assert_eq!(
        coordinates.position(&CommandOrigin {
            position: [0.0, 64.0, 0.0],
            ..crate::world::ServerWorld::console_origin()
        }),
        [0.5, 64.0, 0.5]
    );
    assert_eq!(
        reader_failure(read_vec2(&("").encode_utf16().collect::<Vec<_>>(), 0)).as_deref(),
        Some("argument.pos2d.incomplete")
    );
    assert_eq!(
        reader_failure(read_vec2(&("0").encode_utf16().collect::<Vec<_>>(), 0)).as_deref(),
        Some("argument.pos2d.incomplete")
    );
    assert_eq!(
        reader_failure(read_vec2(&("^ ^").encode_utf16().collect::<Vec<_>>(), 0)).as_deref(),
        Some("argument.pos.mixed")
    );
}

/// `Vec2Argument.listSuggestions` (Vec2Argument.java:61-75).
#[test]
fn worldborder_center_suggests_a_two_axis_pair() {
    let completions = completions_for(
        "worldborder center ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["~", "~ ~"]);
    let completions = completions_for(
        "worldborder center 4",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["4 ~"]);
    let completions = completions_for(
        "worldborder center 4 5",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), [] as [&str; 0]);
    let completions = completions_for(
        "worldborder center ^",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), [] as [&str; 0]);
}
