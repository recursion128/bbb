//! The argument families whose whole answer is in the text.
//!
//! These are the vanilla argument types that need no registry, no datapack and
//! no world to decide whether a span is well formed: the enum-backed ones read
//! their names out of a generated catalog, and the rest are pure grammar. Each
//! scanner returns the absolute byte cursor it stopped at.

use crate::{
    chat_formattings::VANILLA_CHAT_FORMATTINGS,
    entity_anchors::VANILLA_ENTITY_ANCHORS,
    game_types::vanilla_game_type_for_serialized_name,
    heightmap_types::{HeightmapUsage, VANILLA_HEIGHTMAP_TYPES},
    packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID, VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID, VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ROTATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID, VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_UUID_ID, VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID, VANILLA_COMMAND_SLOTS, VANILLA_TEMPLATE_MIRRORS,
        VANILLA_TEMPLATE_ROTATIONS,
    },
    scoreboard_display_slots::VANILLA_SCOREBOARD_DISPLAY_SLOTS,
    CommandCoordinatesKind, CommandSnbtKind, JavaString,
};

use super::{CommandSyntaxError, CommandSyntaxErrorKind};

/// Catalog names are compared without projecting the command's UTF-16 units.
pub fn scan_command_catalog_argument_utf16(
    type_id: i32,
    command: &[u16],
    start: usize,
) -> Option<Result<usize, super::CommandSyntaxErrorUtf16>> {
    let family = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID => "item slot",
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID => "template mirror",
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID => "template rotation",
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID => "color",
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID => "scoreboard display slot",
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID => "entity anchor",
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID => "game mode",
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID => "heightmap type",
        _ => return None,
    };
    let end = if matches!(
        type_id,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID | VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID
    ) {
        super::scan_command_token_utf16(command, start)
    } else {
        super::scan_command_unquoted_string_utf16(command, start)
    };
    let raw = &command[start..end];
    let matches_name = |name: &str| name.encode_utf16().eq(raw.iter().copied());
    let valid = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID => {
            VANILLA_COMMAND_SLOTS.iter().any(|(name, single)| {
                matches_name(name)
                    && (type_id == VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID || *single)
            })
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID => VANILLA_TEMPLATE_MIRRORS
            .iter()
            .any(|name| matches_name(name)),
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID => VANILLA_TEMPLATE_ROTATIONS
            .iter()
            .any(|name| matches_name(name)),
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID => {
            let cleaned: Vec<_> = raw
                .iter()
                .copied()
                .filter(|unit| matches!(*unit, 65..=90 | 97..=122))
                .map(|unit| if unit <= 90 { unit + 32 } else { unit })
                .collect();
            VANILLA_CHAT_FORMATTINGS.iter().any(|entry| {
                !entry.is_format
                    && entry
                        .serialized_name
                        .encode_utf16()
                        .filter(|unit| matches!(*unit, 65..=90 | 97..=122))
                        .eq(cleaned.iter().copied())
            })
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID => VANILLA_SCOREBOARD_DISPLAY_SLOTS
            .iter()
            .any(|entry| matches_name(entry.serialized_name)),
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID => VANILLA_ENTITY_ANCHORS
            .iter()
            .any(|entry| matches_name(entry.serialized_name)),
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID => JavaString::with_utf8_units(raw, |name| {
            vanilla_game_type_for_serialized_name(name).is_some()
        })
        .expect("unquoted catalog words are ASCII"),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID => VANILLA_HEIGHTMAP_TYPES
            .iter()
            .filter(|entry| entry.usage != HeightmapUsage::Worldgen)
            .any(|entry| matches_name(&entry.serialization_key.to_ascii_lowercase())),
        _ => unreachable!(),
    };
    Some(if valid {
        Ok(end)
    } else {
        Err(super::CommandSyntaxErrorUtf16 {
            kind: CommandSyntaxErrorKind::InvalidArgument,
            cursor: if type_id == VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID {
                start
            } else {
                end
            },
            message: format!("invalid {family} ")
                .encode_utf16()
                .chain(raw.iter().copied())
                .collect::<Vec<_>>()
                .into(),
        })
    })
}

fn scan_catalog_argument(
    type_id: i32,
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        catalog_from_type(type_id, input, 0)
    })
}

/// Scans whichever context-free family owns `type_id`, or `None` when no
/// family here does -- the caller's cue to try a registry-aware family.
///
/// Every cursor is a UTF-16 unit index, which is what Brigadier's `StringReader`
/// counts; the `&str` entry below is the byte-positioned adapter over this one.
pub fn scan_command_context_free_argument_utf16(
    type_id: i32,
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
) -> Option<Result<usize, super::CommandSyntaxErrorUtf16>> {
    let result = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID => {
            entity_from_template_utf16(command, start, template)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID => {
            scan_command_game_profile_argument_utf16(command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID => {
            crate::scan_command_coordinates_argument_utf16(
                command,
                start,
                CommandCoordinatesKind::BlockPosition,
            )
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID => {
            crate::scan_command_coordinates_argument_utf16(
                command,
                start,
                CommandCoordinatesKind::ColumnPosition,
            )
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID => crate::scan_command_coordinates_argument_utf16(
            command,
            start,
            CommandCoordinatesKind::Position3,
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID => crate::scan_command_coordinates_argument_utf16(
            command,
            start,
            CommandCoordinatesKind::Position2,
        ),
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID => {
            scan_command_snbt_argument_utf16(command, start, CommandSnbtKind::Compound)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID => {
            scan_command_snbt_argument_utf16(command, start, CommandSnbtKind::Tag)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID => {
            scan_command_nbt_path_argument_utf16(command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID => catalog_from_type(type_id, command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID => {
            scan_command_hex_color_argument_utf16(command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID | VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID => {
            Ok(super::scan_command_unquoted_string_utf16(command, start))
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID => {
            scan_command_operation_argument_utf16(command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID => {
            crate::scan_command_angle_argument_utf16(command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID => {
            catalog_from_type(type_id, command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID => {
            crate::scan_command_swizzle_argument_utf16(command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID => {
            crate::scan_command_resource_location_argument_utf16(command, start, false)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID => {
            crate::scan_command_resource_location_argument_utf16(command, start, true)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID => {
            catalog_from_type(type_id, command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID => {
            crate::scan_command_range_argument_utf16(command, start, true)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID => {
            crate::scan_command_range_argument_utf16(command, start, false)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID => catalog_from_type(type_id, command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID => time_from_template_utf16(command, start, template),
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID => catalog_from_type(type_id, command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_ROTATION_ID => {
            crate::scan_command_coordinates_argument_utf16(
                command,
                start,
                CommandCoordinatesKind::Rotation,
            )
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID => {
            score_holder_from_template_utf16(command, start, template)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID => catalog_from_type(type_id, command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID => catalog_from_type(type_id, command, start),
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID => {
            catalog_from_type(type_id, command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID => {
            catalog_from_type(type_id, command, start)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_UUID_ID => scan_command_uuid_argument_utf16(command, start),
        _ => return None,
    };
    Some(result)
}

fn raw_error(
    cursor: usize,
    message: impl Into<crate::JavaString>,
) -> super::CommandSyntaxErrorUtf16 {
    super::CommandSyntaxErrorUtf16 {
        kind: CommandSyntaxErrorKind::InvalidArgument,
        cursor,
        message: message.into(),
    }
}

fn catalog_from_type(
    type_id: i32,
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    scan_command_catalog_argument_utf16(type_id, command, start).expect("catalog argument type")
}

/// The selector families (`minecraft:entity`, `minecraft:game_profile`,
/// `minecraft:score_holder`) read the buffer's own units.
fn selector_prefix_utf16(
    command: &[u16],
    start: usize,
    family: &str,
) -> Result<crate::CommandEntitySelectorPrefix, super::CommandSyntaxErrorUtf16> {
    crate::parse_command_entity_selector_prefix_utf16(&command[start..])
        .ok_or_else(|| raw_error(start, format!("invalid {family}")))
}

fn entity_from_template_utf16(
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    let CommandArgumentTemplate::Entity {
        single,
        players_only,
    } = template
    else {
        return Err(raw_error(start, "entity argument has the wrong template"));
    };
    let selector = selector_prefix_utf16(command, start, "entity selector")?;
    if *single && selector.multiple {
        return Err(raw_error(
            start,
            "entity selector selects multiple entities",
        ));
    }
    if *players_only && selector.includes_entities && !selector.self_selector {
        return Err(raw_error(
            start,
            "entity selector may select non-player entities",
        ));
    }
    Ok(start + selector.consumed)
}

/// `minecraft:game_profile`: a bare player name, or a selector that cannot
/// reach a non-player.
pub fn scan_command_game_profile_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    if command.get(start) != Some(&64) {
        return Ok(super::scan_command_token_utf16(command, start));
    }
    let selector = selector_prefix_utf16(command, start, "player selector")?;
    if selector.includes_entities {
        return Err(raw_error(
            start,
            "player selector may select non-player entities",
        ));
    }
    Ok(start + selector.consumed)
}

fn score_holder_from_template_utf16(
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    let CommandArgumentTemplate::ScoreHolder { multiple } = template else {
        return Err(raw_error(
            start,
            "score holder argument has the wrong template",
        ));
    };
    if command.get(start) != Some(&64) {
        return Ok(super::scan_command_token_utf16(command, start));
    }
    let selector = selector_prefix_utf16(command, start, "score-holder selector")?;
    if !*multiple && selector.multiple {
        return Err(raw_error(
            start,
            "score-holder selector selects multiple entities",
        ));
    }
    Ok(start + selector.consumed)
}

fn time_from_template_utf16(
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    let CommandArgumentTemplate::Time { minimum } = template else {
        return Err(raw_error(start, "time argument has the wrong template"));
    };
    crate::scan_command_time_argument_utf16(command, start, *minimum)
}

/// `minecraft:nbt_path`.
fn scan_command_nbt_path_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    crate::parse_command_nbt_path_value_prefix_utf16(&command[start..])
        .map(|(consumed, _)| start + consumed)
        .map_err(|error| raw_error(start + error.offset, error.context))
}

/// `minecraft:uuid`.
pub fn scan_command_uuid_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    crate::parse_command_uuid_prefix_utf16(&command[start..])
        .map(|consumed| start + consumed)
        .ok_or_else(|| raw_error(start, "invalid UUID"))
}

/// `minecraft:hex_color`.
pub fn scan_command_hex_color_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    let end = super::scan_command_unquoted_string_utf16(command, start);
    let raw = &command[start..end];
    (matches!(raw.len(), 3 | 6)
        && raw
            .iter()
            .all(|unit| matches!(*unit, 48..=57 | 65..=70 | 97..=102)))
    .then_some(end)
    .ok_or_else(|| {
        let mut message: Vec<u16> = "invalid hex color ".encode_utf16().collect();
        message.extend_from_slice(raw);
        raw_error(end, crate::JavaString::from(message))
    })
}

/// `minecraft:operation`: the scoreboard operators, which are punctuation and
/// so are read as a whole token rather than as a word.
pub fn scan_command_operation_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    if start == command.len() {
        return Err(raw_error(start, "invalid scoreboard operation"));
    }
    let end = super::scan_command_token_utf16(command, start);
    let token = &command[start..end];
    ["=", "+=", "-=", "*=", "/=", "%=", "<", ">", "><"]
        .iter()
        .any(|operator| operator.encode_utf16().eq(token.iter().copied()))
        .then_some(end)
        // `OperationArgument.parse` raises this one with the reader, so the
        // cursor is the position the token was read from.
        .ok_or_else(|| raw_error(start, "invalid scoreboard operation"))
}

/// Scans whichever context-free family owns `type_id`, or `None` when no
/// family here does -- the caller's cue to try a registry-aware family.
pub fn scan_command_context_free_argument(
    type_id: i32,
    command: &str,
    start: usize,
    template: &CommandArgumentTemplate,
) -> Option<Result<usize, CommandSyntaxError>> {
    let input: Vec<_> = command[start..].encode_utf16().collect();
    scan_command_context_free_argument_utf16(type_id, &input, 0, template).map(|result| {
        result
            .map(|end| start + super::strings::byte_cursor(&command[start..], end))
            .map_err(|error| super::strings::byte_error(command, start, error))
    })
}

/// `minecraft:entity`: an entity selector, held to the arity and the
/// player-only flag the argument was registered with.
pub fn scan_command_entity_argument(
    command: &str,
    start: usize,
    single: bool,
    players_only: bool,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        entity_from_template_utf16(
            input,
            0,
            &CommandArgumentTemplate::Entity {
                single,
                players_only,
            },
        )
    })
}

/// `minecraft:game_profile`: a bare player name, or a selector that cannot
/// reach a non-player.
pub fn scan_command_game_profile_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_game_profile_argument_utf16(input, 0)
    })
}

/// `minecraft:score_holder`: a bare name, or a selector held to the arity the
/// argument was registered with.
pub fn scan_command_score_holder_argument(
    command: &str,
    start: usize,
    multiple: bool,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        score_holder_from_template_utf16(
            input,
            0,
            &CommandArgumentTemplate::ScoreHolder { multiple },
        )
    })
}

/// The coordinate families (`block_pos`, `column_pos`, `vec2`, `vec3`,
/// `rotation`), which differ only in axis count and in what a bare axis means.
pub fn scan_command_coordinates_argument(
    command: &str,
    start: usize,
    kind: CommandCoordinatesKind,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        crate::scan_command_coordinates_argument_utf16(input, 0, kind)
    })
}

/// `minecraft:nbt_compound_tag` and `minecraft:nbt_tag`.
pub fn scan_command_snbt_argument_utf16(
    command: &[u16],
    start: usize,
    kind: CommandSnbtKind,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    let parsed = match kind {
        CommandSnbtKind::Compound => crate::parse_snbt_compound_prefix_utf16(&command[start..]),
        CommandSnbtKind::Tag => crate::parse_snbt_prefix_utf16(&command[start..]),
    };
    parsed
        .map(|(consumed, _)| start + consumed)
        .map_err(|_| super::CommandSyntaxErrorUtf16 {
            kind: CommandSyntaxErrorKind::InvalidArgument,
            cursor: start,
            message: "invalid SNBT argument".into(),
        })
}

pub fn scan_command_snbt_argument(
    command: &str,
    start: usize,
    kind: CommandSnbtKind,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_from_start_via_utf16(
        command,
        start,
        |units, unit_start| scan_command_snbt_argument_utf16(units, unit_start, kind),
        |end, to_byte| to_byte(end),
    )
}

/// `minecraft:nbt_path`.
pub fn scan_command_nbt_path_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_nbt_path_argument_utf16(input, 0)
    })
}

/// `minecraft:item_slot` (`single_only`) and `minecraft:item_slots`.
pub fn scan_command_item_slot_argument(
    command: &str,
    start: usize,
    single_only: bool,
) -> Result<usize, CommandSyntaxError> {
    scan_catalog_argument(
        if single_only {
            VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID
        } else {
            VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID
        },
        command,
        start,
    )
}

/// `minecraft:template_mirror`.
pub fn scan_command_template_mirror_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_catalog_argument(
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID,
        command,
        start,
    )
}

/// `minecraft:template_rotation`.
pub fn scan_command_template_rotation_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_catalog_argument(
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID,
        command,
        start,
    )
}

/// `minecraft:uuid`.
pub fn scan_command_uuid_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_uuid_argument_utf16(input, 0)
    })
}

/// `minecraft:color`: a `ChatFormatting` colour, compared the way vanilla's
/// name lookup does -- letters only, case-insensitive.
pub fn scan_command_color_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_catalog_argument(VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID, command, start)
}

/// `minecraft:hex_color`.
pub fn scan_command_hex_color_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_hex_color_argument_utf16(input, 0)
    })
}

/// `minecraft:operation`: the scoreboard operators, which are punctuation and
/// so are read as a whole token rather than as a word.
pub fn scan_command_operation_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_operation_argument_utf16(input, 0)
    })
    .map_err(|mut error| {
        if start < command.len() {
            error.cursor = None;
        }
        error
    })
}

/// `minecraft:angle`: a bare `~`, or a finite float with an optional `~`.
pub fn scan_command_angle_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_angle_argument_utf16(input, 0)
    })
}

pub fn scan_command_angle_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    if start == command.len() {
        return Err(super::CommandSyntaxErrorUtf16 {
            kind: CommandSyntaxErrorKind::InvalidArgument,
            cursor: start,
            message: "incomplete angle".into(),
        });
    }
    let cursor = start + usize::from(command.get(start) == Some(&126));
    if command.get(cursor).is_none_or(|unit| *unit == 32) {
        return Ok(cursor);
    }
    let (end, value) = scan_float_utf16(command, cursor, "angle")?;
    value
        .is_finite()
        .then_some(end)
        .ok_or_else(|| super::CommandSyntaxErrorUtf16 {
            kind: CommandSyntaxErrorKind::InvalidArgument,
            cursor: end,
            message: "angle must be finite".into(),
        })
}

fn scan_float_utf16(
    command: &[u16],
    start: usize,
    family: &str,
) -> Result<(usize, f32), super::CommandSyntaxErrorUtf16> {
    let (end, units) = super::scan_command_number_lexeme_utf16(command, start)?;
    JavaString::with_utf8_units(units, |raw| {
        raw.parse::<f32>()
            .map(|value| (end, value))
            .map_err(|_| super::CommandSyntaxErrorUtf16 {
                kind: CommandSyntaxErrorKind::InvalidNumber,
                cursor: start,
                message: format!("invalid {family} {raw}").into(),
            })
    })
    .expect("float lexeme is ASCII")
}

/// `minecraft:scoreboard_slot`.
pub fn scan_command_scoreboard_slot_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_catalog_argument(
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
        command,
        start,
    )
}

/// `minecraft:swizzle`: a non-repeating subset of the three axis letters.
pub fn scan_command_swizzle_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_swizzle_argument_utf16(input, 0)
    })
}

pub fn scan_command_swizzle_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    let mut cursor = start;
    let mut axes = 0_u8;
    while let Some(unit) = command.get(cursor).copied().filter(|unit| *unit != 32) {
        cursor += 1;
        let bit = match unit {
            120 => 1,
            121 => 2,
            122 => 4,
            _ => {
                return Err(super::CommandSyntaxErrorUtf16 {
                    kind: CommandSyntaxErrorKind::InvalidArgument,
                    cursor,
                    message: "invalid swizzle axis".into(),
                })
            }
        };
        if axes & bit != 0 {
            return Err(super::CommandSyntaxErrorUtf16 {
                kind: CommandSyntaxErrorKind::InvalidArgument,
                cursor,
                message: "duplicate swizzle axis".into(),
            });
        }
        axes |= bit;
    }
    Ok(cursor)
}

/// `minecraft:resource_location`, `minecraft:dimension` and (with `allow_tag`)
/// `minecraft:function`, none of which resolve what they name.
pub fn scan_command_resource_location_argument(
    command: &str,
    start: usize,
    allow_tag: bool,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_resource_location_argument_utf16(input, 0, allow_tag)
    })
}

pub fn scan_command_resource_location_argument_utf16(
    command: &[u16],
    start: usize,
    allow_tag: bool,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    let identifier_start = start + usize::from(allow_tag && command.get(start) == Some(&35));
    super::scan_command_resource_location_utf16(command, identifier_start).map(|(end, _)| end)
}

/// `minecraft:entity_anchor`.
pub fn scan_command_entity_anchor_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_catalog_argument(
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID,
        command,
        start,
    )
}

/// `minecraft:int_range`.
pub fn scan_command_int_range_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_range_argument(command, start, true)
}

/// `minecraft:float_range`.
pub fn scan_command_float_range_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_range_argument(command, start, false)
}

fn scan_range_argument(
    command: &str,
    start: usize,
    integer: bool,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_range_argument_utf16(input, 0, integer)
    })
}

pub fn scan_command_range_argument_utf16(
    command: &[u16],
    start: usize,
    integer: bool,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    if integer {
        parse_command_bounds_utf16(
            command,
            start,
            |raw| raw.parse::<i32>(),
            |min, max| min > max,
        )
        .map(|(end, _, _)| end)
    } else {
        parse_command_bounds_utf16(
            command,
            start,
            |raw| raw.parse::<f64>(),
            |min, max| min.total_cmp(&max).is_gt(),
        )
        .map(|(end, _, _)| end)
    }
}

/// `MinMaxBounds.FloatDegrees.fromReader` (`MinMaxBounds.java:260-265`) reads the
/// same bounds as the other two readers and is the only one that keeps swapped
/// ones.
pub fn scan_command_degrees_range_argument_utf16(
    command: &[u16],
    start: usize,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    parse_command_bounds_utf16(command, start, |raw| raw.parse::<f32>(), |_, _| false)
        .map(|(end, _, _)| end)
}

pub(in crate::component::command) fn parse_command_bounds_utf16<T: Copy, E>(
    command: &[u16],
    start: usize,
    parse_number: impl Fn(&str) -> Result<T, E>,
    swapped: impl Fn(T, T) -> bool,
) -> Result<(usize, Option<T>, Option<T>), super::CommandSyntaxErrorUtf16> {
    // MinMaxBounds.Bounds.fromReader rewinds errors from either endpoint.
    let error = |kind, message: String| super::CommandSyntaxErrorUtf16 {
        kind,
        cursor: start,
        message: message.into(),
    };
    let number = |cursor: usize| {
        let mut end = cursor;
        while let Some(unit) = command.get(end).copied() {
            if (unit == 46 && command.get(end + 1) == Some(&46))
                || !matches!(unit, 48..=57 | 45 | 46)
            {
                break;
            }
            end += 1;
        }
        if end == cursor {
            return Ok((end, None));
        }
        JavaString::with_utf8_units(&command[cursor..end], |raw| {
            parse_number(raw)
                .map(|value| (end, Some(value)))
                .map_err(|_| {
                    error(
                        CommandSyntaxErrorKind::InvalidNumber,
                        format!("invalid range number {raw}"),
                    )
                })
        })
        .expect("range number lexeme is ASCII")
    };
    let (mut end, min) = number(start)?;
    let max = if command.get(end..end.saturating_add(2)) == Some(&[46, 46]) {
        let (cursor, value) = number(end + 2)?;
        end = cursor;
        value
    } else {
        min
    };
    if min.is_none() && max.is_none() {
        return Err(error(
            CommandSyntaxErrorKind::InvalidArgument,
            "empty range".into(),
        ));
    }
    if min.zip(max).is_some_and(|(min, max)| swapped(min, max)) {
        return Err(error(
            CommandSyntaxErrorKind::InvalidArgument,
            "range minimum exceeds maximum".into(),
        ));
    }
    Ok((end, min, max))
}

/// `minecraft:gamemode`.
pub fn scan_command_game_mode_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_catalog_argument(VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, command, start)
}

/// `minecraft:time`: a float and an optional unit suffix, rounded to ticks and
/// held to the minimum the argument was registered with.
pub fn scan_command_time_argument(
    command: &str,
    start: usize,
    minimum: i32,
) -> Result<usize, CommandSyntaxError> {
    super::strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_time_argument_utf16(input, 0, minimum)
    })
}

pub fn scan_command_time_argument_utf16(
    command: &[u16],
    start: usize,
    minimum: i32,
) -> Result<usize, super::CommandSyntaxErrorUtf16> {
    let (number_end, value) = scan_float_utf16(command, start, "time")?;
    let end = super::scan_command_unquoted_string_utf16(command, number_end);
    let factor = match &command[number_end..end] {
        [] | [116] => 1.0,
        [115] => 20.0,
        [100] => 24_000.0,
        _ => {
            return Err(super::CommandSyntaxErrorUtf16 {
                kind: CommandSyntaxErrorKind::InvalidArgument,
                cursor: end,
                message: "invalid time unit".into(),
            })
        }
    };
    let ticks = java_round_to_i32(value * factor);
    (ticks >= minimum)
        .then_some(end)
        .ok_or_else(|| super::CommandSyntaxErrorUtf16 {
            kind: CommandSyntaxErrorKind::NumberTooLow,
            cursor: end,
            message: format!("time {ticks} is below minimum {minimum}").into(),
        })
}

/// `minecraft:heightmap`, which offers only the heightmaps a running world
/// keeps -- the worldgen-only ones are not spellable.
pub fn scan_command_heightmap_argument(
    command: &str,
    start: usize,
) -> Result<usize, CommandSyntaxError> {
    scan_catalog_argument(VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID, command, start)
}

fn java_round_to_i32(value: f32) -> i32 {
    // Math.round(float) avoids the extra rounding introduced by adding 0.5f.
    let bits = value.to_bits() as i32;
    let shift = 149 - ((bits & 0x7f80_0000) >> 23);
    if (0..32).contains(&shift) {
        let significand = (bits & 0x007f_ffff) | 0x0080_0000;
        let signed = if bits < 0 { -significand } else { significand };
        ((signed >> shift) + 1) >> 1
    } else {
        value as i32
    }
}
