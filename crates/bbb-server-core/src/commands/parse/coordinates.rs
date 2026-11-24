//! The coordinate argument grammar, moved out of the parse walk unchanged.
//!
//! `Coordinates` and its readers are one family: `minecraft:vec3`,
//! `minecraft:vec2`, `minecraft:block_pos`, `minecraft:column_pos` and
//! `minecraft:rotation` all parse into the same two shapes, and the rotation
//! maths the local form needs lives with them.

use bbb_protocol::packets::BlockPos;
use bbb_protocol::{component_literal_nbt, JavaString};

use super::super::CommandOrigin;
use super::{number_end, read_local_coordinate, translatable, ArgumentValue, ParseFailure};

/// What `Vec3Argument` parses into (`Coordinates`), in its two shapes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::commands) enum Coordinates {
    /// `WorldCoordinates` (WorldCoordinates.java:9).
    World {
        x: WorldCoordinate,
        y: WorldCoordinate,
        z: WorldCoordinate,
    },
    /// `LocalCoordinates` (LocalCoordinates.java:9).
    Local { left: f64, up: f64, forwards: f64 },
}

/// `WorldCoordinate` (WorldCoordinate.java:8), one axis of an absolute or
/// `~`-relative coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::commands) struct WorldCoordinate {
    relative: bool,
    value: f64,
}

impl WorldCoordinate {
    /// `WorldCoordinate.get` (WorldCoordinate.java:13-15).
    fn get(self, original: f64) -> f64 {
        if self.relative {
            self.value + original
        } else {
            self.value
        }
    }
}

impl Coordinates {
    /// `Coordinates.getRotation(CommandSourceStack)`
    /// (WorldCoordinates.java:19-22 and LocalCoordinates.java:19-21), as this
    /// world spells a rotation: yaw then pitch.
    ///
    /// The record's fields do not line up with that order.
    /// `RotationArgument.parse` (RotationArgument.java:30-34) reads the yaw
    /// first and stores it in `y`, the pitch second and stores it in `x`, and
    /// `getRotation` then reads `Vec2(x.get(rot.x), y.get(rot.y))` -- a `Vec2`
    /// whose `x` is the pitch, because `CommandSourceStack.getRotation` is
    /// `Vec2(entity.getXRot(), entity.getYRot())`. So the `y` field is the yaw
    /// on both sides and the `x` field is the pitch on both sides, which is
    /// what this reads back by name.
    ///
    /// `LocalCoordinates` answers `Vec2.ZERO` for every source; no `^` form can
    /// reach here, because `RotationArgument` builds a `WorldCoordinates` and
    /// nothing else.
    pub(in crate::commands) fn rotation(self, source_rotation: [f32; 2]) -> [f32; 2] {
        let [source_y_rot, source_x_rot] = source_rotation;
        match self {
            Self::World { x, y, .. } => [
                y.get(f64::from(source_y_rot)) as f32,
                x.get(f64::from(source_x_rot)) as f32,
            ],
            Self::Local { .. } => [0.0, 0.0],
        }
    }

    /// `Coordinates.isXRelative` as `RotationArgument`'s reader spells it: the
    /// `x` field of the record, which for a rotation is the pitch
    /// (`TeleportCommand.getRelatives`, TeleportCommand.java:220 passes it as
    /// `relativeXRot`).
    pub(in crate::commands) fn is_x_relative(self) -> bool {
        match self {
            Self::World { x, .. } => x.relative,
            // `LocalCoordinates.isXRelative` (LocalCoordinates.java:24-26).
            Self::Local { .. } => true,
        }
    }

    /// `Coordinates.isYRelative`, which for a rotation is the yaw.
    pub(in crate::commands) fn is_y_relative(self) -> bool {
        match self {
            Self::World { y, .. } => y.relative,
            Self::Local { .. } => true,
        }
    }

    /// `Coordinates.isZRelative`.
    pub(in crate::commands) fn is_z_relative(self) -> bool {
        match self {
            Self::World { z, .. } => z.relative,
            Self::Local { .. } => true,
        }
    }

    /// `Coordinates.getPosition(CommandSourceStack)`
    /// (WorldCoordinates.java:13-16 and LocalCoordinates.java:13-16).
    ///
    /// The world form is relative to `sender.getPosition()`; the local form is
    /// relative to `sender.getAnchor().apply(sender)`
    /// (LocalCoordinates.java:14), which `/execute anchored` moves off the
    /// source position.
    pub(in crate::commands) fn position(self, at: &CommandOrigin) -> [f64; 3] {
        self.position_from(at.position, at.rotation, at.anchored_position())
    }

    fn position_from(self, position: [f64; 3], rotation: [f32; 2], anchored: [f64; 3]) -> [f64; 3] {
        match self {
            Self::World { x, y, z } => [x.get(position[0]), y.get(position[1]), z.get(position[2])],
            Self::Local { left, up, forwards } => {
                let [dx, dy, dz] =
                    apply_local_coordinates_to_rotation(rotation, [left, up, forwards]);
                [anchored[0] + dx, anchored[1] + dy, anchored[2] + dz]
            }
        }
    }

    /// `Coordinates.getBlockPos` (Coordinates.java:13-15):
    /// `BlockPos.containing(getPosition(sender))`, which is `Mth.floor` on each
    /// axis (BlockPos.java:91-93).
    pub(in crate::commands) fn block_pos(self, at: &CommandOrigin) -> BlockPos {
        let [x, y, z] = self.position(at);
        BlockPos {
            x: x.floor() as i32,
            y: y.floor() as i32,
            z: z.floor() as i32,
        }
    }
    pub(in crate::commands) fn block_pos_from(
        self,
        position: [f64; 3],
        rotation: [f32; 2],
        anchored: [f64; 3],
    ) -> BlockPos {
        let [x, y, z] = self.position_from(position, rotation, anchored);
        BlockPos {
            x: x.floor() as i32,
            y: y.floor() as i32,
            z: z.floor() as i32,
        }
    }
}

/// `Vec3.applyLocalCoordinatesToRotation` (Vec3.java:323-337).
///
/// Every trigonometric step is `Mth.cos`/`Mth.sin` over a `float` angle and the
/// products are `float` multiplications, so the intermediate vectors are
/// single-precision before the three dot products widen them; reproducing that
/// order is what keeps `^ ^ ^1` landing on the same double vanilla reports.
///
/// `rotation` is `CommandSourceStack.getRotation()`, which is
/// `Vec2(entity.getXRot(), entity.getYRot())` -- pitch then yaw -- while this
/// world stores a player's rotation the way the movement packets carry it, yaw
/// then pitch, so the two halves are read back by name here rather than by
/// position.
pub(super) fn apply_local_coordinates_to_rotation(
    rotation: [f32; 2],
    direction: [f64; 3],
) -> [f64; 3] {
    let [y_rot, x_rot] = rotation;
    // `(rotation.y + 90.0F) * (float) (Math.PI / 180.0)` is float arithmetic
    // throughout, widened only by the `Mth.sin(double)` parameter itself.
    let radians = |degrees: f32| f64::from(degrees * (std::f32::consts::PI / 180.0));
    let y_cos = mth_cos(radians(y_rot + 90.0));
    let y_sin = mth_sin(radians(y_rot + 90.0));
    let x_cos = mth_cos(radians(-x_rot));
    let x_sin = mth_sin(radians(-x_rot));
    let x_cos_up = mth_cos(radians(-x_rot + 90.0));
    let x_sin_up = mth_sin(radians(-x_rot + 90.0));
    let forwards = [
        f64::from(y_cos * x_cos),
        f64::from(x_sin),
        f64::from(y_sin * x_cos),
    ];
    let up = [
        f64::from(y_cos * x_cos_up),
        f64::from(x_sin_up),
        f64::from(y_sin * x_cos_up),
    ];
    // `Vec3.cross` scaled by -1 (Vec3.java:332).
    let left = [
        -(forwards[1] * up[2] - forwards[2] * up[1]),
        -(forwards[2] * up[0] - forwards[0] * up[2]),
        -(forwards[0] * up[1] - forwards[1] * up[0]),
    ];
    [
        forwards[0] * direction[2] + up[0] * direction[1] + left[0] * direction[0],
        forwards[1] * direction[2] + up[1] * direction[1] + left[1] * direction[0],
        forwards[2] * direction[2] + up[2] * direction[1] + left[2] * direction[0],
    ]
}

/// `Mth.sin` (Mth.java:49-51) and its table (`:34-38`).
///
/// The table is the answer, not an optimisation of it: vanilla reads a
/// 65536-entry lookup rather than calling `Math.sin`, so the value a local
/// coordinate resolves to is the table entry and nothing closer to the true
/// sine. The entries are `(float) Math.sin(i / 10430.378350470453)`, computed
/// once on first use.
pub(super) fn mth_sin(radians: f64) -> f32 {
    sin_table()[((radians * SIN_TABLE_SCALE) as i64 & 0xFFFF) as usize]
}

/// `Mth.cos` (Mth.java:53-55): the same table a quarter turn along.
pub(super) fn mth_cos(radians: f64) -> f32 {
    sin_table()[((radians * SIN_TABLE_SCALE + 16384.0) as i64 & 0xFFFF) as usize]
}

/// The `10430.378350470453` of `Mth.sin`/`Mth.cos`, which is `65536 / (2π)`.
pub(super) const SIN_TABLE_SCALE: f64 = 10430.378350470453;

pub(super) fn sin_table() -> &'static [f32; 65536] {
    static TABLE: std::sync::OnceLock<Box<[f32; 65536]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = Box::new([0.0f32; 65536]);
        for (index, entry) in table.iter_mut().enumerate() {
            *entry = (index as f64 / SIN_TABLE_SCALE).sin() as f32;
        }
        table
    })
}

/// `BlockPosArgument.parse` (BlockPosArgument.java:61-63): a leading `^` reads
/// local coordinates and anything else reads the integer world form.
///
/// The local branch is `LocalCoordinates.parse`, which is the same reader
/// `minecraft:vec3` uses -- `BlockPosArgument` and `Vec3Argument` differ only
/// in which `WorldCoordinates` factory the other branch calls.
pub(in crate::commands) fn read_block_pos(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    if command[cursor..].starts_with(&[94]) {
        let (end, value) = read_local_coordinates(command, cursor)?;
        let ArgumentValue::Coordinates(coordinates) = value else {
            unreachable!("the local reader answers with coordinates");
        };
        return Ok((end, ArgumentValue::BlockPos(coordinates)));
    }
    read_int_world_coordinates(command, cursor)
}

/// `ColumnPosArgument.parse` (ColumnPosArgument.java:34-49).
///
/// There is no local branch. `BlockPosArgument` chooses between two readers on
/// a leading `^`; this one always calls `WorldCoordinate.parseInt`, which
/// refuses `^` with `argument.pos.mixed` (WorldCoordinate.java:42-44) -- so the
/// `^` forms in `getExamples` (`:22`) name a grammar the parser rejects.
///
/// The empty input is refused before the first axis is read (`:36-37`), and a
/// missing separator rewinds to the start of the argument (`:45-46`), so both
/// report `argument.pos2d.incomplete` where the argument began.
pub(in crate::commands) fn read_column_pos(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let incomplete = || ParseFailure {
        // `ERROR_NOT_COMPLETE` (`:23`).
        message: translatable("argument.pos2d.incomplete", Vec::new()),
        cursor: Some(cursor),
    };
    if cursor >= command.len() {
        return Err(incomplete());
    }
    let (after_x, x) = read_int_world_coordinate(command, cursor)?;
    if command.get(after_x) != Some(&32) {
        return Err(incomplete());
    }
    let (end, z) = read_int_world_coordinate(command, after_x + 1)?;
    Ok((
        end,
        ArgumentValue::ColumnPos(Coordinates::World {
            x,
            // `:43`: the vertical axis is filled in rather than read.
            y: WorldCoordinate {
                relative: true,
                value: 0.0,
            },
            z,
        }),
    ))
}

/// `WorldCoordinates.parseInt` (WorldCoordinates.java:39-57).
///
/// The two `setCursor(start)` calls before `ERROR_NOT_COMPLETE` (`:50`, `:54`)
/// are why an axis that never arrived is reported at the start of the argument
/// rather than at the end of the input.
pub(super) fn read_int_world_coordinates(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let incomplete = || ParseFailure {
        // `Vec3Argument.ERROR_NOT_COMPLETE` (Vec3Argument.java:22), which is
        // what `WorldCoordinates.parseInt` throws too.
        message: translatable("argument.pos3d.incomplete", Vec::new()),
        cursor: Some(cursor),
    };
    let (after_x, x) = read_int_world_coordinate(command, cursor)?;
    if command.get(after_x) != Some(&32) {
        return Err(incomplete());
    }
    let (after_y, y) = read_int_world_coordinate(command, after_x + 1)?;
    if command.get(after_y) != Some(&32) {
        return Err(incomplete());
    }
    let (end, z) = read_int_world_coordinate(command, after_y + 1)?;
    Ok((end, ArgumentValue::BlockPos(Coordinates::World { x, y, z })))
}

/// `WorldCoordinate.parseInt` (WorldCoordinate.java:41-59).
///
/// The one line that separates this from `parseDouble` is `:53`: a `~`-relative
/// axis still reads a double, so `~0.5` is legal, while an absolute axis reads
/// an int and `0.5` is not. There is no centre correction on either, which is
/// why `/setblock 0 0 0` names the block at the origin and `/teleport 0 0 0`
/// stands in the middle of it.
pub(super) fn read_int_world_coordinate(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, WorldCoordinate), ParseFailure> {
    if command[cursor..].starts_with(&[94]) {
        // `:42-44`: one axis may not switch to the local form.
        return Err(ParseFailure {
            message: translatable("argument.pos.mixed", Vec::new()),
            cursor: Some(cursor),
        });
    }
    if cursor >= command.len() {
        // `:46-48`, and the key is the int one rather than the double one.
        return Err(ParseFailure {
            message: translatable("argument.pos.missing.int", Vec::new()),
            cursor: Some(cursor),
        });
    }
    let relative = command[cursor..].starts_with(&[126]);
    let start = cursor + usize::from(relative);
    // `:52-56`: the number is only read when something other than a separator
    // follows, so `~` alone reads none and stands for the source's own axis.
    if !command.get(start).is_some_and(|byte| *byte != 32) {
        return Ok((
            start,
            WorldCoordinate {
                relative,
                value: 0.0,
            },
        ));
    }
    if relative {
        let (end, value) = read_double(command, start)?;
        return Ok((end, WorldCoordinate { relative, value }));
    }
    let (end, value) = read_int(command, start)?;
    Ok((
        end,
        WorldCoordinate {
            relative,
            value: f64::from(value),
        },
    ))
}

/// `StringReader.readInt` (com/mojang/brigadier/StringReader.java:97-115) as a
/// value, which is what a coordinate axis reads rather than a whole
/// `brigadier:integer` argument with its bounds.
pub(in crate::commands) fn read_int(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, i32), ParseFailure> {
    let end = number_end(command, cursor);
    let number = &command[cursor..end];
    if number.is_empty() {
        return Err(ParseFailure {
            message: translatable("parsing.int.expected", Vec::new()),
            cursor: Some(cursor),
        });
    }
    let Some(value) =
        JavaString::with_utf8_units(number, |number| number.parse::<i32>().ok()).flatten()
    else {
        return Err(ParseFailure {
            message: translatable("parsing.int.invalid", vec![component_literal_nbt(number)]),
            cursor: Some(cursor),
        });
    };
    Ok((end, value))
}

/// `Commands.createValidator(this::parse)` (Commands.java:447-456) over
/// `Vec3Argument.parse`: whether the candidate parses at all, trailing text
/// included, which is the predicate the coordinate suggestions filter on.
pub(in crate::commands) fn parses_as_vec3(text: &str) -> bool {
    JavaString::with_utf16(text, |text| read_vec3(text, 0, true).is_ok())
}

/// The same validator over `BlockPosArgument.parse`
/// (BlockPosArgument.java:61-63), which is the only thing that separates
/// `BlockPosArgument.listSuggestions` (`:66-80`) from `Vec3Argument`'s: a
/// coordinate this reader refuses is dropped from the offered triple even
/// though the same text would have stood as a `minecraft:vec3`.
pub(in crate::commands) fn parses_as_block_pos(text: &str) -> bool {
    JavaString::with_utf16(text, |text| read_block_pos(text, 0).is_ok())
}

/// The same validator over `ColumnPosArgument.parse`
/// (ColumnPosArgument.java:34-49), which `listSuggestions` hands
/// `suggest2DCoordinates` (`:65`). It refuses `^`, so the local coordinates the
/// suggestion path offers for a `^` remainder (`:59-60`) never survive it.
pub(in crate::commands) fn parses_as_column_pos(text: &str) -> bool {
    JavaString::with_utf16(text, |text| read_column_pos(text, 0).is_ok())
}

/// `Vec2Argument.parse` (Vec2Argument.java:43-58) as
/// `Commands.createValidator` hands it to `suggest2DCoordinates`.
pub(in crate::commands) fn parses_as_vec2(text: &str) -> bool {
    JavaString::with_utf16(text, |text| read_vec2(text, 0).is_ok())
}

/// `Vec2Argument.parse` (Vec2Argument.java:43-58): two centre-corrected world
/// axes and a relative-zero Y it never read (`:51`). A leading `^` is not a
/// local form here — `WorldCoordinate.parseDouble` refuses it as mixed.
pub(in crate::commands) fn read_vec2(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let incomplete = || ParseFailure {
        // `Vec2Argument.ERROR_NOT_COMPLETE` (Vec2Argument.java:23).
        message: translatable("argument.pos2d.incomplete", Vec::new()),
        cursor: Some(cursor),
    };
    if cursor >= command.len() {
        return Err(incomplete());
    }
    let (after_x, x) = read_world_coordinate(command, cursor, true)?;
    if command.get(after_x) != Some(&32) {
        return Err(incomplete());
    }
    let (end, z) = read_world_coordinate(command, after_x + 1, true)?;
    Ok((
        end,
        ArgumentValue::Vec2(Coordinates::World {
            x,
            y: WorldCoordinate {
                relative: true,
                value: 0.0,
            },
            z,
        }),
    ))
}

/// `Vec3Argument.parse` (Vec3Argument.java:46-48): a leading `^` reads local
/// coordinates and anything else reads world ones, centre-corrected unless the
/// node was built with `Vec3Argument.vec3(false)`.
pub(super) fn read_vec3(
    command: &[u16],
    cursor: usize,
    center: bool,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    if command[cursor..].starts_with(&[94]) {
        return read_local_coordinates(command, cursor);
    }
    read_world_coordinates(command, cursor, center)
}

/// `WorldCoordinates.parseDouble` (WorldCoordinates.java:59-77).
///
/// The centre correction applies to the two horizontal axes and never to the
/// vertical one (`:61`, `:64`, `:67`).
pub(super) fn read_world_coordinates(
    command: &[u16],
    cursor: usize,
    center: bool,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let incomplete = || ParseFailure {
        // `Vec3Argument.ERROR_NOT_COMPLETE` (Vec3Argument.java:22), thrown after
        // `reader.setCursor(start)` puts the reader back at the argument.
        message: translatable("argument.pos3d.incomplete", Vec::new()),
        cursor: Some(cursor),
    };
    let (after_x, x) = read_world_coordinate(command, cursor, center)?;
    if command.get(after_x) != Some(&32) {
        return Err(incomplete());
    }
    let (after_y, y) = read_world_coordinate(command, after_x + 1, false)?;
    if command.get(after_y) != Some(&32) {
        return Err(incomplete());
    }
    let (end, z) = read_world_coordinate(command, after_y + 1, center)?;
    Ok((
        end,
        ArgumentValue::Coordinates(Coordinates::World { x, y, z }),
    ))
}

/// `WorldCoordinate.parseDouble` (WorldCoordinate.java:17-39).
pub(super) fn read_world_coordinate(
    command: &[u16],
    cursor: usize,
    center: bool,
) -> Result<(usize, WorldCoordinate), ParseFailure> {
    if command[cursor..].starts_with(&[94]) {
        // `:18-20`: one axis may not switch to the local form.
        return Err(ParseFailure {
            message: translatable("argument.pos.mixed", Vec::new()),
            cursor: Some(cursor),
        });
    }
    if cursor >= command.len() {
        // `:22-24`.
        return Err(ParseFailure {
            message: translatable("argument.pos.missing.double", Vec::new()),
            cursor: Some(cursor),
        });
    }
    // `isRelative` (`:61-71`) consumes the `~` before the number starts.
    let relative = command[cursor..].starts_with(&[126]);
    let start = cursor + usize::from(relative);
    // `:28`: the number is only read when something other than a separator
    // follows, so `~` alone and an axis at the end of the input both read none.
    let (end, mut value) = if command.get(start).is_some_and(|byte| *byte != 32) {
        read_double(command, start)?
    } else {
        (start, 0.0)
    };
    let number = &command[start..end];
    if relative && number.is_empty() {
        // `:30-32`.
        return Ok((
            end,
            WorldCoordinate {
                relative: true,
                value: 0.0,
            },
        ));
    }
    // `:34-36`: a whole absolute number on a centre-corrected axis lands on the
    // middle of the block rather than on its corner.
    if !number.contains(&46) && !relative && center {
        value += 0.5;
    }
    Ok((end, WorldCoordinate { relative, value }))
}

/// `StringReader.readDouble` (com/mojang/brigadier/StringReader.java:137-155):
/// the `[0-9.-]` run, then the conversion, with the cursor put back at the
/// start of the run before the invalid-number refusal.
pub(in crate::commands) fn read_double(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, f64), ParseFailure> {
    let end = number_end(command, cursor);
    let number = &command[cursor..end];
    if number.is_empty() {
        return Err(ParseFailure {
            message: translatable("parsing.double.expected", Vec::new()),
            cursor: Some(cursor),
        });
    }
    JavaString::with_utf8_units(number, |number| number.parse::<f64>().ok())
        .flatten()
        .filter(|value| value.is_finite())
        .map(|value| (end, value))
        .ok_or_else(|| ParseFailure {
            message: translatable(
                "parsing.double.invalid",
                vec![component_literal_nbt(number)],
            ),
            cursor: Some(cursor),
        })
}

/// `LocalCoordinates.parse` (LocalCoordinates.java:38-56).
pub(super) fn read_local_coordinates(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let incomplete = || ParseFailure {
        message: translatable("argument.pos3d.incomplete", Vec::new()),
        cursor: Some(cursor),
    };
    let (after_left, left) = read_local_coordinate(command, cursor, cursor)?;
    if command.get(after_left) != Some(&32) {
        return Err(incomplete());
    }
    let (after_up, up) = read_local_coordinate(command, after_left + 1, cursor)?;
    if command.get(after_up) != Some(&32) {
        return Err(incomplete());
    }
    let (end, forwards) = read_local_coordinate(command, after_up + 1, cursor)?;
    Ok((
        end,
        ArgumentValue::Coordinates(Coordinates::Local { left, up, forwards }),
    ))
}

/// `RotationArgument.parse` (RotationArgument.java:25-40): two
/// `WorldCoordinate`s and no centre correction on either, stored yaw-first into
/// the record's `y` and pitch-second into its `x`, with a `~0` on the axis the
/// record has no rotation for (`:34`).
///
/// The refusal is `RotationArgument.ERROR_NOT_COMPLETE` (`:15`) rather than
/// `Vec3Argument`'s, and it is thrown after `reader.setCursor(start)` (`:36`)
/// puts the reader back at the argument, which is why both throws carry the
/// argument's own cursor.
pub(super) fn read_rotation(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let incomplete = || ParseFailure {
        message: translatable("argument.rotation.incomplete", Vec::new()),
        cursor: Some(cursor),
    };
    // `:27-28`: an empty remainder is refused before a coordinate is read at
    // all, which is what makes the missing-double refusal unreachable on the
    // first axis.
    if cursor >= command.len() {
        return Err(incomplete());
    }
    let (after_yaw, yaw) = read_world_coordinate(command, cursor, false)?;
    if command.get(after_yaw) != Some(&32) {
        return Err(incomplete());
    }
    let (end, pitch) = read_world_coordinate(command, after_yaw + 1, false)?;
    Ok((
        end,
        ArgumentValue::Rotation(Coordinates::World {
            x: pitch,
            y: yaw,
            z: WorldCoordinate {
                relative: true,
                value: 0.0,
            },
        }),
    ))
}
