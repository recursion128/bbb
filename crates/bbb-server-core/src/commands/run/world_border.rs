//! `/worldborder` (WorldBorderCommand.java:49-227).

use crate::tick_rate::java_fixed_decimals;
use bbb_protocol::packets::{
    SetBorderCenter, SetBorderLerpSize, SetBorderSize, SetBorderWarningDelay,
    SetBorderWarningDistance,
};
use bbb_protocol::{component_literal_nbt, NbtSummaryValue};
use bbb_server_types::interaction::WorldBorder;

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{translatable, CommandCx, CommandOrigin};
use crate::connection::ClientboundPlayPacket;
use crate::world::ServerWorld;

use super::{
    double_argument, float_argument, integer_argument, vec2_argument, CommandRun, Outcome,
};

/// `WorldBorderCommand.ERROR_TOO_BIG` (`:30-32`).
const MAX_SIZE: f32 = 5.999997E7;
/// `WorldBorderCommand.ERROR_TOO_FAR_OUT` (`:33-35`).
const MAX_CENTER: f64 = 2.9999984E7;

/// `WorldBorderCommand.setSize` (WorldBorderCommand.java:193-223).
pub(super) fn set_world_border_size(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    add: bool,
    with_time: bool,
) -> Outcome {
    write(run, cx, |border, now, _| {
        let current = border.size_at(now);
        let typed = double_argument(parse, "distance");
        let distance = if add { current + typed } else { typed };
        let ticks = if with_time {
            let time = match parse.argument("time") {
                Some(ArgumentValue::Time(ticks)) => i64::from(*ticks),
                other => unreachable!("time is a time argument, not {other:?}"),
            };
            if add {
                // `:62-67`: remaining lerp plus the typed duration.
                border.remaining_lerp_time(now) + time
            } else {
                time
            }
        } else {
            0
        };
        apply_set_size(border, now, current, distance, ticks)
    })
}

fn apply_set_size(
    border: WorldBorder,
    now: i64,
    current: f64,
    distance: f64,
    ticks: i64,
) -> Result<BorderWrite, NbtSummaryValue> {
    if current == distance {
        return Err(translatable(
            "commands.worldborder.set.failed.nochange",
            Vec::new(),
        ));
    }
    if distance < 1.0 {
        return Err(translatable(
            "commands.worldborder.set.failed.small",
            Vec::new(),
        ));
    }
    if distance > f64::from(MAX_SIZE) {
        return Err(translatable(
            "commands.worldborder.set.failed.big",
            vec![NbtSummaryValue::Float(MAX_SIZE.to_bits())],
        ));
    }
    let formatted = java_fixed_decimals(distance, 1);
    let rebuilt = |size, lerp_time| {
        reconstruct(
            now,
            border.center_x(),
            border.center_z(),
            border.damage_per_block(),
            border.safe_zone(),
            border.warning_blocks(),
            border.warning_time(),
            size,
            lerp_time,
            distance,
        )
    };
    let result = (distance - current) as i32;
    Ok(if ticks > 0 {
        BorderWrite {
            border: rebuilt(current, ticks),
            packet: Some(ClientboundPlayPacket::SetBorderLerpSize(
                SetBorderLerpSize {
                    old_size: current,
                    new_size: distance,
                    lerp_time: ticks,
                },
            )),
            key: if distance > current {
                "commands.worldborder.set.grow"
            } else {
                "commands.worldborder.set.shrink"
            },
            arguments: vec![
                component_literal_nbt(&formatted),
                component_literal_nbt(seconds(ticks)),
            ],
            result,
        }
    } else {
        BorderWrite {
            border: rebuilt(distance, 0),
            packet: Some(ClientboundPlayPacket::SetBorderSize(SetBorderSize {
                size: distance,
            })),
            key: "commands.worldborder.set.immediate",
            arguments: vec![component_literal_nbt(&formatted)],
            result,
        }
    })
}

/// `WorldBorderCommand.setCenter` (WorldBorderCommand.java:175-191).
pub(super) fn set_world_border_center(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    write(run, cx, |border, now, at| {
        let coordinates = vec2_argument(parse, "pos");
        let [x, _, z] = coordinates.position(at);
        // `Vec2Argument.getVec2` (Vec2Argument.java:38-41).
        let x = x as f32 as f64;
        let z = z as f32 as f64;
        if border.center_x() == x && border.center_z() == z {
            return Err(translatable(
                "commands.worldborder.center.failed",
                Vec::new(),
            ));
        }
        if !x.is_finite() || !z.is_finite() || x.abs() > MAX_CENTER || z.abs() > MAX_CENTER {
            return Err(translatable(
                "commands.worldborder.set.failed.far",
                vec![NbtSummaryValue::Double(MAX_CENTER.to_bits())],
            ));
        }
        Ok(BorderWrite {
            border: reconstruct(
                now,
                x,
                z,
                border.damage_per_block(),
                border.safe_zone(),
                border.warning_blocks(),
                border.warning_time(),
                border.size_at(now),
                border.remaining_lerp_time(now),
                border.lerp_target(),
            ),
            packet: Some(ClientboundPlayPacket::SetBorderCenter(SetBorderCenter {
                new_center_x: x,
                new_center_z: z,
            })),
            key: "commands.worldborder.center.success",
            arguments: vec![
                component_literal_nbt(java_fixed_decimals(x, 2)),
                component_literal_nbt(java_fixed_decimals(z, 2)),
            ],
            result: 0,
        })
    })
}

/// `WorldBorderCommand.setDamageAmount` (WorldBorderCommand.java:136-145).
pub(super) fn set_world_border_damage_amount(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let amount = float_argument(parse, "damagePerBlock");
    write(run, cx, |border, now, _| {
        if border.damage_per_block() == f64::from(amount) {
            return Err(translatable(
                "commands.worldborder.damage.amount.failed",
                Vec::new(),
            ));
        }
        Ok(BorderWrite {
            border: reconstruct(
                now,
                border.center_x(),
                border.center_z(),
                f64::from(amount),
                border.safe_zone(),
                border.warning_blocks(),
                border.warning_time(),
                border.size_at(now),
                border.remaining_lerp_time(now),
                border.lerp_target(),
            ),
            packet: None,
            key: "commands.worldborder.damage.amount.success",
            arguments: vec![component_literal_nbt(java_fixed_decimals(
                f64::from(amount),
                2,
            ))],
            result: amount as i32,
        })
    })
}

/// `WorldBorderCommand.setDamageBuffer` (WorldBorderCommand.java:125-134).
pub(super) fn set_world_border_damage_buffer(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let distance = float_argument(parse, "distance");
    write(run, cx, |border, now, _| {
        if border.safe_zone() == f64::from(distance) {
            return Err(translatable(
                "commands.worldborder.damage.buffer.failed",
                Vec::new(),
            ));
        }
        Ok(BorderWrite {
            border: reconstruct(
                now,
                border.center_x(),
                border.center_z(),
                border.damage_per_block(),
                f64::from(distance),
                border.warning_blocks(),
                border.warning_time(),
                border.size_at(now),
                border.remaining_lerp_time(now),
                border.lerp_target(),
            ),
            packet: None,
            key: "commands.worldborder.damage.buffer.success",
            arguments: vec![component_literal_nbt(java_fixed_decimals(
                f64::from(distance),
                2,
            ))],
            result: distance as i32,
        })
    })
}

/// `WorldBorderCommand.setWarningTime` (WorldBorderCommand.java:147-156).
pub(super) fn set_world_border_warning_time(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let ticks = match parse.argument("time") {
        Some(ArgumentValue::Time(ticks)) => *ticks,
        other => unreachable!("time is a time argument, not {other:?}"),
    };
    write(run, cx, |border, now, _| {
        if border.warning_time() == ticks {
            return Err(translatable(
                "commands.worldborder.warning.time.failed",
                Vec::new(),
            ));
        }
        Ok(BorderWrite {
            border: reconstruct(
                now,
                border.center_x(),
                border.center_z(),
                border.damage_per_block(),
                border.safe_zone(),
                border.warning_blocks(),
                ticks,
                border.size_at(now),
                border.remaining_lerp_time(now),
                border.lerp_target(),
            ),
            packet: Some(ClientboundPlayPacket::SetBorderWarningDelay(
                SetBorderWarningDelay {
                    warning_delay: ticks,
                },
            )),
            key: "commands.worldborder.warning.time.success",
            arguments: vec![component_literal_nbt(seconds(i64::from(ticks)))],
            result: ticks,
        })
    })
}

/// `WorldBorderCommand.setWarningDistance` (WorldBorderCommand.java:158-167).
pub(super) fn set_world_border_warning_distance(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let distance = integer_argument(parse, "distance");
    write(run, cx, |border, now, _| {
        if border.warning_blocks() == distance {
            return Err(translatable(
                "commands.worldborder.warning.distance.failed",
                Vec::new(),
            ));
        }
        Ok(BorderWrite {
            border: reconstruct(
                now,
                border.center_x(),
                border.center_z(),
                border.damage_per_block(),
                border.safe_zone(),
                distance,
                border.warning_time(),
                border.size_at(now),
                border.remaining_lerp_time(now),
                border.lerp_target(),
            ),
            packet: Some(ClientboundPlayPacket::SetBorderWarningDistance(
                SetBorderWarningDistance {
                    warning_blocks: distance,
                },
            )),
            key: "commands.worldborder.warning.distance.success",
            arguments: vec![NbtSummaryValue::Int(distance)],
            result: distance,
        })
    })
}

/// `WorldBorderCommand.getSize` (WorldBorderCommand.java:169-173).
pub(super) fn get_world_border_size(
    run: &mut CommandRun,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let size = world
        .command_world_border(&at.level)
        .size_at(world.game_time());
    run.succeed(
        translatable(
            "commands.worldborder.get",
            vec![component_literal_nbt(java_fixed_decimals(size, 0))],
        ),
        false,
    );
    Ok((size + 0.5).floor() as i32)
}

/// The border a `/worldborder` arm leaves behind, the packet that tells the
/// source level's players, and the line it reports.
struct BorderWrite {
    border: WorldBorder,
    packet: Option<ClientboundPlayPacket>,
    key: &'static str,
    arguments: Vec<NbtSummaryValue>,
    result: i32,
}

fn write(
    run: &mut CommandRun,
    cx: &mut CommandCx<'_>,
    plan: impl FnOnce(WorldBorder, i64, &CommandOrigin) -> Result<BorderWrite, NbtSummaryValue>,
) -> Outcome {
    let border = cx.world.command_world_border(&cx.origin.level);
    let write = plan(border, cx.world.game_time(), &cx.origin)?;
    cx.world
        .set_world_border(&cx.origin.level, write.border, write.packet, cx.events);
    run.succeed(translatable(write.key, write.arguments), true);
    Ok(write.result)
}

/// `WorldBorder.applyInitialSettings` (`WorldBorder.java:285-298`) via
/// [`WorldBorder::from_settings`].
#[allow(clippy::too_many_arguments)]
fn reconstruct(
    game_time: i64,
    center_x: f64,
    center_z: f64,
    damage_per_block: f64,
    safe_zone: f64,
    warning_blocks: i32,
    warning_time: i32,
    size: f64,
    lerp_time: i64,
    lerp_target: f64,
) -> WorldBorder {
    WorldBorder::from_settings(
        center_x,
        center_z,
        damage_per_block,
        safe_zone,
        warning_blocks,
        warning_time,
        size,
        lerp_time,
        lerp_target,
        game_time,
    )
}

/// `WorldBorderCommand.formatTicksToSeconds` (WorldBorderCommand.java:225-227).
fn seconds(ticks: i64) -> String {
    java_fixed_decimals(ticks as f64 / 20.0, 2)
}
