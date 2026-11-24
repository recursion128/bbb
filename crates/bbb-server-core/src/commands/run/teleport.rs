//! `/teleport` (`/tp`) and the look-at helpers `/rotate` shares.

use bbb_protocol::{
    component_literal_nbt,
    entity_anchors::EntityAnchor,
    packets::{
        PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z,
        PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y, PLAYER_RELATIVE_Y_ROT,
        PLAYER_RELATIVE_Z,
    },
    NbtSummaryValue,
};

use crate::tick_rate::java_fixed_decimals;
use crate::world::wrap_degrees;
use bbb_server_types::save::Java26LevelKey;

use crate::commands::parse::{ArgumentValue, Coordinates, ParseResult};
use crate::commands::types::{CommandLookAt, TeleportMove};
use crate::commands::{translatable, CommandCx, CommandSource};

use super::{
    coordinates_argument, entity_anchor_argument, rotation_argument, select_damage_entity_states,
    single_damage_entity_argument, source_entity, CommandRun, Outcome,
};
use crate::commands::types::CommandDamageEntityRef;
use crate::commands::CommandOrigin;

/// Which node named the entities a `/teleport` moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TeleportTargets {
    /// `Collections.singleton(c.getSource().getEntityOrException())`
    /// (TeleportCommand.java:46, `:58`).
    SourceEntity,
    /// `EntityArgument.getEntities(c, "targets")` (`:69`).
    Argument,
}

/// Which `facing ..` branch the reached node came through, and therefore which
/// `LookAt` it builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FacingShape {
    /// `facing entity <facingEntity>` (TeleportCommand.java:102), whose anchor
    /// is the `FEET` the node itself supplies.
    Entity,
    /// `facing entity <facingEntity> <facingAnchor>` (`:114-116`).
    AnchoredEntity,
    /// `facing <facingLocation>` (`:131`), which resolves the coordinates
    /// against the source right away -- `Vec3Argument.getVec3`
    /// (Vec3Argument.java:38-40) rather than `getCoordinates`.
    Position,
}

/// The entities a `/teleport` node named.
///
/// `getEntityOrException` (CommandSourceStack.java:393-399) throws
/// `permissions.requires.entity`, and `EntityArgument.getEntities`
/// (EntityArgument.java:60-67) throws `NO_ENTITIES_FOUND` (`:35`) for a
/// selector that matched nobody.
fn teleport_targets<'a>(
    parse: &ParseResult,
    source: &CommandSource,
    world: &crate::world::EntityReads<'a>,
    at: &CommandOrigin,
    targets: TeleportTargets,
) -> Result<Vec<CommandDamageEntityRef<'a>>, NbtSummaryValue> {
    match targets {
        TeleportTargets::SourceEntity => Ok(vec![source_entity(source, world)?]),
        TeleportTargets::Argument => {
            let Some(ArgumentValue::Players(selector)) = parse.argument("targets") else {
                unreachable!("targets is an entity argument");
            };
            let found = select_damage_entity_states(selector, source, at, world)?;
            if found.is_empty() {
                return Err(translatable("argument.entity.notfound.entity", Vec::new()));
            }
            Ok(found)
        }
    }
}

/// `TeleportCommand.teleportToEntity` (TeleportCommand.java:146-172).
///
/// The destination hands over its whole placement -- its level, its position
/// and its rotation (`:151-157`) -- and the relative set is empty (`:155`), so
/// nothing about the moved entity survives the move.
pub(super) fn teleport_to_entity(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    targets: TeleportTargets,
) -> Outcome {
    let (moves, report) = cx.world.with_entity_reads(|world| {
        let at = &cx.origin;
        let entities = teleport_targets(parse, source, world, at, targets)?;
        let destination = single_damage_entity_argument(parse, "destination", source, at, world)?;
        let moves = entities
            .iter()
            .map(|entity| {
                teleport_move(
                    *entity,
                    destination.dimension(),
                    destination.position(),
                    0,
                    destination.rotation(),
                    None,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let count = i32::try_from(entities.len()).unwrap_or(i32::MAX);
        let report = if entities.len() == 1 {
            // `:162-166`.
            translatable(
                "commands.teleport.success.entity.single",
                vec![
                    entities[0].display_name(world),
                    destination.display_name(world),
                ],
            )
        } else {
            // `:168`. The count is a boxed `int`, which `TranslatableContents`
            // serializes as the primitive it is.
            translatable(
                "commands.teleport.success.entity.multiple",
                vec![NbtSummaryValue::Int(count), destination.display_name(world)],
            )
        };
        Ok::<_, NbtSummaryValue>((moves, report))
    })?;
    Ok(perform_teleports(run, cx, moves, report))
}

/// `TeleportCommand.teleportToPos` (TeleportCommand.java:174-215).
///
/// The destination is resolved once against the source (`:182-183`) and the
/// relative set is rebuilt per entity (`:186`), because the one thing that
/// varies between them is whether they are already in the level the source is
/// in.
pub(super) fn teleport_to_pos(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    targets: TeleportTargets,
    rotation: bool,
    facing: Option<FacingShape>,
) -> Outcome {
    let (moves, report) = cx.world.with_entity_reads(|reads| {
        plan_teleport_to_pos(parse, source, reads, &cx.origin, targets, rotation, facing)
    })?;
    Ok(perform_teleports(run, cx, moves, report))
}

fn plan_teleport_to_pos(
    parse: &ParseResult,
    source: &CommandSource,
    world: &crate::world::EntityReads<'_>,
    at: &CommandOrigin,
    targets: TeleportTargets,
    rotation: bool,
    facing: Option<FacingShape>,
) -> Result<(Vec<TeleportMove>, NbtSummaryValue), NbtSummaryValue> {
    let entities = teleport_targets(parse, source, world, at, targets)?;
    let destination = coordinates_argument(parse, "location");
    let rotation = rotation.then(|| rotation_argument(parse, "rotation"));
    // The `LookAt` is the last argument of the executor's own call, so a
    // `facingEntity` that resolves to nobody is reported after the targets
    // were resolved and before anything moved.
    let look_at = match facing {
        Some(shape) => Some(look_at_target(parse, source, world, at, shape)?),
        None => None,
    };
    let position = destination.position(at);
    let rotation_values = rotation.map(|coordinates| coordinates.rotation(at.rotation));
    let mut moves = Vec::with_capacity(entities.len());
    for entity in &entities {
        let relatives_mask =
            teleport_relatives(destination, rotation, entity.dimension() == &at.level);
        // `:188` keeps the entity's own rotation when none was typed and `:190`
        // takes the one that was.
        let rotation_values = rotation_values.unwrap_or_else(|| entity.rotation());
        moves.push(teleport_move(
            *entity,
            &at.level,
            position,
            relatives_mask,
            rotation_values,
            look_at.clone(),
        )?);
    }
    let count = i32::try_from(entities.len()).unwrap_or(i32::MAX);
    let [x, y, z] = position.map(format_double);
    let report = if entities.len() == 1 {
        // `:195-204`.
        translatable(
            "commands.teleport.success.location.single",
            vec![
                entities[0].display_name(world),
                component_literal_nbt(&x),
                component_literal_nbt(&y),
                component_literal_nbt(&z),
            ],
        )
    } else {
        // `:206-211`.
        translatable(
            "commands.teleport.success.location.multiple",
            vec![
                NbtSummaryValue::Int(count),
                component_literal_nbt(&x),
                component_literal_nbt(&y),
                component_literal_nbt(&z),
            ],
        )
    };
    Ok((moves, report))
}

/// Moves every entity, then reports once (TeleportCommand.java:146-172,
/// `:185-212`).
fn perform_teleports(
    run: &mut CommandRun,
    cx: &mut CommandCx<'_>,
    moves: Vec<TeleportMove>,
    report: NbtSummaryValue,
) -> i32 {
    let count = i32::try_from(moves.len()).unwrap_or(i32::MAX);
    for teleport in moves {
        cx.world.teleport_by_command(teleport, cx.events);
    }
    run.succeed(report, true);
    count
}

/// The `LookAt` a `facing ..` node builds (TeleportCommand.java:102, `:114-116`
/// and `:131`).
pub(super) fn look_at_target(
    parse: &ParseResult,
    source: &CommandSource,
    world: &crate::world::EntityReads<'_>,
    at: &CommandOrigin,
    shape: FacingShape,
) -> Result<CommandLookAt, NbtSummaryValue> {
    match shape {
        FacingShape::Entity | FacingShape::AnchoredEntity => Ok(CommandLookAt::Entity {
            target: single_damage_entity_argument(parse, "facingEntity", source, at, world)?
                .attribute_target(),
            from_anchor: at.anchor,
            anchor: match shape {
                FacingShape::AnchoredEntity => entity_anchor_argument(parse, "facingAnchor"),
                _ => EntityAnchor::Feet,
            },
        }),
        FacingShape::Position => Ok(CommandLookAt::Position {
            position: coordinates_argument(parse, "facingLocation").position(at),
            from_anchor: at.anchor,
        }),
    }
}

/// `TeleportCommand.getRelatives` (TeleportCommand.java:217-222), already
/// packed the way `Relative.pack` (Relative.java:109-117) packs it for the
/// wire.
///
/// The three sets it unions are not the same question asked three times. The
/// direction set (`Relative.direction`, Relative.java:68-83) marks the delta
/// axes, which is what keeps a `~`-relative teleport from zeroing the motion
/// the entity already had; the position set (`:51-66`) marks the position axes
/// and is dropped entirely when the entity is changing level (`:219`), because
/// a coordinate relative to a position in another level would mean nothing; and
/// the rotation set is `Relative.ROTATION` (`:22`) -- both axes relative, which
/// is how "keep the rotation you had" is spelled -- unless a `<rotation>` was
/// typed, in which case each axis is relative only if it was written with `~`.
fn teleport_relatives(
    destination: Coordinates,
    rotation: Option<Coordinates>,
    same_dimension: bool,
) -> i32 {
    let mut mask = 0;
    if destination.is_x_relative() {
        mask |= PLAYER_RELATIVE_DELTA_X;
    }
    if destination.is_y_relative() {
        mask |= PLAYER_RELATIVE_DELTA_Y;
    }
    if destination.is_z_relative() {
        mask |= PLAYER_RELATIVE_DELTA_Z;
    }
    if same_dimension {
        if destination.is_x_relative() {
            mask |= PLAYER_RELATIVE_X;
        }
        if destination.is_y_relative() {
            mask |= PLAYER_RELATIVE_Y;
        }
        if destination.is_z_relative() {
            mask |= PLAYER_RELATIVE_Z;
        }
    }
    match rotation {
        None => mask |= PLAYER_RELATIVE_Y_ROT | PLAYER_RELATIVE_X_ROT,
        Some(rotation) => {
            if rotation.is_y_relative() {
                mask |= PLAYER_RELATIVE_Y_ROT;
            }
            if rotation.is_x_relative() {
                mask |= PLAYER_RELATIVE_X_ROT;
            }
        }
    }
    mask
}

/// `TeleportCommand.performTeleport` (TeleportCommand.java:228-266).
///
/// The move is what `Entity.teleportTo` is handed at `:252`: the axes the relative set marks
/// carry the difference from where the victim is now (`:245-249`) rather than
/// the destination itself, and both rotations are wrapped (`:250-251`).
fn teleport_move(
    victim: CommandDamageEntityRef<'_>,
    level: &Java26LevelKey,
    position: [f64; 3],
    relatives_mask: i32,
    rotation: [f32; 2],
    look_at: Option<CommandLookAt>,
) -> Result<TeleportMove, NbtSummaryValue> {
    let [x, y, z] = position;
    // `BlockPos.containing` (BlockPos.java:91-93) floors each axis with
    // `Mth.floor` (Mth.java:65-67), whose `(int)` narrowing saturates the way
    // this cast does. Reject non-finite coordinates before narrowing, because
    // `f64::NAN as i32 == 0` would falsely pass spawnable bounds.
    if !x.is_finite()
        || !y.is_finite()
        || !z.is_finite()
        || !is_in_spawnable_bounds([x.floor() as i32, y.floor() as i32, z.floor() as i32])
    {
        // `INVALID_POSITION` (TeleportCommand.java:33-35), thrown at `:242`.
        return Err(translatable(
            "commands.teleport.invalidPosition",
            Vec::new(),
        ));
    }
    let relative = |value: f64, current: f64, bit: i32| {
        if relatives_mask & bit != 0 {
            value - current
        } else {
            value
        }
    };
    let [y_rot, x_rot] = rotation;
    let relative_rotation = |value: f32, current: f32, bit: i32| {
        if relatives_mask & bit != 0 {
            value - current
        } else {
            value
        }
    };
    let current_position = victim.position();
    let current_rotation = victim.rotation();
    let position = [
        relative(x, current_position[0], PLAYER_RELATIVE_X),
        relative(y, current_position[1], PLAYER_RELATIVE_Y),
        relative(z, current_position[2], PLAYER_RELATIVE_Z),
    ];
    if !position.iter().all(|axis| axis.is_finite()) {
        return Err(translatable(
            "commands.teleport.invalidPosition",
            Vec::new(),
        ));
    }
    let raw_y_rot = relative_rotation(y_rot, current_rotation[0], PLAYER_RELATIVE_Y_ROT);
    let raw_x_rot = relative_rotation(x_rot, current_rotation[1], PLAYER_RELATIVE_X_ROT);
    let y_rot = if raw_y_rot.is_finite() {
        wrap_degrees(raw_y_rot)
    } else {
        0.0
    };
    let x_rot = if raw_x_rot.is_finite() {
        wrap_degrees(raw_x_rot)
    } else {
        0.0
    };
    Ok(TeleportMove {
        victim: victim.attribute_target(),
        level: level.clone(),
        position,
        rotation: [y_rot, x_rot],
        relatives_mask,
        look_at,
        teleport_command_tail: true,
    })
}

/// `Level.isInSpawnableBounds` (Level.java:175-177): a height inside
/// `isOutsideSpawnableHeight`'s window (`:189-191`) and a column inside
/// `isInWorldBoundsHorizontal`'s square (`:179-181`).
///
/// The two methods spell their bounds as literals rather than reading the
/// constants beside them (`Level.java:98`, `:102-103`), which is why the
/// numbers are repeated here the way vanilla repeats them.
pub(super) fn is_in_spawnable_bounds(pos: [i32; 3]) -> bool {
    let [x, y, z] = pos;
    let inside_height = (-20_000_000..20_000_000).contains(&y);
    inside_height && x >= -30_000_000 && z >= -30_000_000 && x < 30_000_000 && z < 30_000_000
}

/// `TeleportCommand.formatDouble` (TeleportCommand.java:224-226):
/// `String.format(Locale.ROOT, "%f", value)`, which is six fraction digits.
fn format_double(value: f64) -> String {
    java_fixed_decimals(value, 6)
}
