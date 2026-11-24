use bbb_protocol::{packets::CommandArgumentTemplate, scan_command_context_free_argument_utf16};

use super::{ParsedValue, RawCommandSyntaxErrorState};

/// The argument families the grammar layer answers on its own. Every span it
/// accepts is fully validated, because none of these families has anything left
/// to resolve against a registry.
pub(super) fn parse(
    type_id: i32,
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
) -> Option<Result<ParsedValue, RawCommandSyntaxErrorState>> {
    scan_command_context_free_argument_utf16(type_id, command, start, template)
        .map(|scanned| scanned.map(validated).map_err(Into::into))
}

fn validated(end: usize) -> ParsedValue {
    ParsedValue {
        end,
        validated: true,
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID, VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID, VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID, VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID, VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_UUID_ID, VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID,
    };

    use super::*;

    #[test]
    fn command_coordinate_errors_keep_axis_or_reader_start() {
        use bbb_protocol::{
            scan_command_coordinates_argument_utf16, CommandCoordinatesKind, CommandSyntaxErrorKind,
        };
        for (value, offset, error_kind) in [
            ("1 2", 0, CommandSyntaxErrorKind::InvalidArgument),
            ("1 2 ", 4, CommandSyntaxErrorKind::ExpectedNumber),
            ("1 ^ 3", 2, CommandSyntaxErrorKind::InvalidArgument),
            ("^ 1 ^", 0, CommandSyntaxErrorKind::InvalidArgument),
            ("^ ^ ", 4, CommandSyntaxErrorKind::ExpectedNumber),
            ("1 2147483648 3", 2, CommandSyntaxErrorKind::InvalidNumber),
            ("1 ~--2 3", 3, CommandSyntaxErrorKind::InvalidNumber),
        ] {
            let raw: Vec<_> = [0xd800].into_iter().chain(value.encode_utf16()).collect();
            let error = scan_command_coordinates_argument_utf16(
                &raw,
                1,
                CommandCoordinatesKind::BlockPosition,
            )
            .unwrap_err();
            assert_eq!(error.cursor, offset + 1, "{value}");
            assert_eq!(error.kind, error_kind);
            let command: Vec<u16> = "😀 ".encode_utf16().chain(value.encode_utf16()).collect();
            let error = parse(
                VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID,
                &command,
                3,
                &CommandArgumentTemplate::Unit,
            )
            .unwrap()
            .unwrap_err();
            assert_eq!(error.cursor, 3 + offset, "{value}");
            assert!(error.contextual);
        }
        for kind in [
            CommandCoordinatesKind::ColumnPosition,
            CommandCoordinatesKind::Position2,
            CommandCoordinatesKind::Rotation,
        ] {
            assert_eq!(
                scan_command_coordinates_argument_utf16(&[], 0, kind)
                    .unwrap_err()
                    .kind,
                CommandSyntaxErrorKind::InvalidArgument
            );
            assert_eq!(
                scan_command_coordinates_argument_utf16(&[49, 32, 94], 0, kind)
                    .unwrap_err()
                    .cursor,
                2
            );
        }
        let error = scan_command_coordinates_argument_utf16(
            &[49, 32, 0xd800, 32, 51],
            0,
            CommandCoordinatesKind::Position3,
        )
        .unwrap_err();
        assert_eq!(error.cursor, 2);
    }

    #[test]
    fn command_coordinate_uuid_prefixes_preserve_raw_suffixes() {
        use bbb_protocol::{
            parse_command_coordinates_prefix_utf16, parse_command_uuid_prefix_utf16,
            CommandCoordinatesKind,
        };
        for (kind, value) in [
            (CommandCoordinatesKind::BlockPosition, "~1.5 2 ~"),
            (CommandCoordinatesKind::ColumnPosition, "1 2"),
            (CommandCoordinatesKind::Position3, "^ ^-1.5 ^"),
            (CommandCoordinatesKind::Position2, "~ ~2.5"),
            (CommandCoordinatesKind::Rotation, "1.5 -2"),
        ] {
            let raw: Vec<_> = value.encode_utf16().chain([32, 0xd800]).collect();
            assert_eq!(
                parse_command_coordinates_prefix_utf16(&raw, kind),
                Some(value.len())
            );
        }
        for value in ["^ ~ ^", "1 2", "2147483648 2 3", "1.5 2 3"] {
            assert!(parse_command_coordinates_prefix_utf16(
                &value.encode_utf16().collect::<Vec<_>>(),
                CommandCoordinatesKind::BlockPosition
            )
            .is_none());
        }
        let uuid = "12345678-1234-1234-1234-123456789abc";
        let raw: Vec<_> = uuid.encode_utf16().chain([0xdc00]).collect();
        assert_eq!(parse_command_uuid_prefix_utf16(&raw), Some(uuid.len()));
        assert!(parse_command_uuid_prefix_utf16(&[0xd800]).is_none());
        assert!(
            parse_command_uuid_prefix_utf16(&"abc-def".encode_utf16().collect::<Vec<_>>())
                .is_none()
        );
        for (id, value) in [
            (VANILLA_COMMAND_ARGUMENT_TYPE_UUID_ID, uuid),
            (VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID, "~1.5 2 ~"),
        ] {
            let command: Vec<u16> = "😀 "
                .encode_utf16()
                .chain(value.encode_utf16())
                .chain(" tail".encode_utf16())
                .collect();
            assert_eq!(
                parse(id, &command, 3, &CommandArgumentTemplate::Unit)
                    .unwrap()
                    .unwrap()
                    .end,
                3 + value.len()
            );
        }
    }

    #[test]
    fn command_angle_time_raw_units_and_float_rounding() {
        use bbb_protocol::{
            scan_command_angle_argument_utf16, scan_command_time_argument_utf16,
            CommandSyntaxErrorKind,
        };
        let angle: Vec<_> = [0xdc00]
            .into_iter()
            .chain("~12.5".encode_utf16())
            .chain([0xd800])
            .collect();
        assert_eq!(scan_command_angle_argument_utf16(&angle, 1).unwrap(), 6);
        assert_eq!(scan_command_angle_argument_utf16(&[126, 32], 0).unwrap(), 1);
        assert_eq!(scan_command_angle_argument_utf16(&[32], 0).unwrap(), 0);
        assert_eq!(
            scan_command_angle_argument_utf16(&[], 0).unwrap_err().kind,
            CommandSyntaxErrorKind::InvalidArgument
        );
        assert_eq!(
            scan_command_angle_argument_utf16(&[126, 0xd800], 0)
                .unwrap_err()
                .cursor,
            1
        );
        for (value, minimum, valid) in [
            ("0.49999997t", 1, false),
            ("0.5t", 1, true),
            ("8388609t", 8388610, false),
            ("-0.5t", 0, true),
            ("-0.50000006t", 0, false),
            ("0.05s", 1, true),
        ] {
            let raw: Vec<_> = [0xdc00]
                .into_iter()
                .chain(value.encode_utf16())
                .chain([0xd800])
                .collect();
            let result = scan_command_time_argument_utf16(&raw, 1, minimum);
            assert_eq!(result.is_ok(), valid, "{value}");
            if let Err(error) = result {
                assert_eq!(error.cursor, raw.len() - 1);
            }
            let command: Vec<u16> = "😀 ".encode_utf16().chain(value.encode_utf16()).collect();
            let result = parse(
                VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
                &command,
                3,
                &CommandArgumentTemplate::Time { minimum },
            )
            .unwrap();
            assert_eq!(result.is_ok(), valid, "{value}");
        }
        let raw: Vec<_> = [0xdc00]
            .into_iter()
            .chain("2unknown".encode_utf16())
            .collect();
        assert_eq!(
            scan_command_time_argument_utf16(&raw, 1, 0)
                .unwrap_err()
                .cursor,
            raw.len()
        );
    }

    #[test]
    fn command_range_raw_boundaries_and_endpoint_error_rewind() {
        use bbb_protocol::{scan_command_range_argument_utf16, CommandSyntaxErrorKind};
        for integer in [true, false] {
            for value in ["1..3", "..3", "1..", "2", "-3..-1"] {
                let raw: Vec<_> = [0xd800]
                    .into_iter()
                    .chain(value.encode_utf16())
                    .chain([0xdc00])
                    .collect();
                assert_eq!(
                    scan_command_range_argument_utf16(&raw, 1, integer).unwrap(),
                    raw.len() - 1
                );
            }
            for (value, kind) in [
                ("1..--2", CommandSyntaxErrorKind::InvalidNumber),
                ("..", CommandSyntaxErrorKind::InvalidArgument),
                ("3..1", CommandSyntaxErrorKind::InvalidArgument),
            ] {
                let raw: Vec<_> = [0xd800].into_iter().chain(value.encode_utf16()).collect();
                let error = scan_command_range_argument_utf16(&raw, 1, integer).unwrap_err();
                assert_eq!(error.cursor, 1);
                assert_eq!(error.kind, kind);
                let id = if integer {
                    VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID
                } else {
                    VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID
                };
                let command: Vec<u16> = "😀 ".encode_utf16().chain(value.encode_utf16()).collect();
                let error = parse(id, &command, 3, &CommandArgumentTemplate::Unit)
                    .unwrap()
                    .unwrap_err();
                assert_eq!(error.cursor, 3);
            }
        }
        assert!(scan_command_range_argument_utf16(
            &"1..2147483648".encode_utf16().collect::<Vec<_>>(),
            0,
            true
        )
        .is_err());
        assert!(scan_command_range_argument_utf16(
            &"1..1.5".encode_utf16().collect::<Vec<_>>(),
            0,
            true
        )
        .is_err());
        assert_eq!(
            scan_command_range_argument_utf16(
                &"1..1.5".encode_utf16().collect::<Vec<_>>(),
                0,
                false
            )
            .unwrap(),
            6
        );
        assert_eq!(
            scan_command_range_argument_utf16(&[0xd800], 0, false)
                .unwrap_err()
                .cursor,
            0
        );
        for (value, valid) in [("0..-0", false), ("-0..0", true), ("-0..-0", true)] {
            let raw: Vec<_> = value.encode_utf16().collect();
            assert_eq!(
                scan_command_range_argument_utf16(&raw, 0, false).is_ok(),
                valid
            );
            assert!(scan_command_range_argument_utf16(&raw, 0, true).is_ok());
        }
    }

    #[test]
    fn command_resource_argument_raw_scanning_and_tag_error_start() {
        use bbb_protocol::scan_command_resource_location_argument_utf16;
        let input: Vec<_> = [0xdc00]
            .into_iter()
            .chain("#minecraft:tick".encode_utf16())
            .chain([0xd800])
            .collect();
        assert_eq!(
            scan_command_resource_location_argument_utf16(&input, 1, true).unwrap(),
            input.len() - 1
        );
        let invalid: Vec<_> = [0xdc00]
            .into_iter()
            .chain("#a:b:c".encode_utf16())
            .collect();
        assert_eq!(
            scan_command_resource_location_argument_utf16(&invalid, 1, true)
                .unwrap_err()
                .cursor,
            2
        );
        let command: Vec<u16> = "😀 #a:b:c".encode_utf16().collect();
        let error = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID,
            &command,
            3,
            &CommandArgumentTemplate::Unit,
        )
        .unwrap()
        .unwrap_err();
        assert_eq!(error.cursor, 3 + 1);
        for (allow_tag, text, end) in [
            (true, "#", 1),
            (false, "", 0),
            (false, ":", 1),
            (false, "#tick", 0),
        ] {
            assert_eq!(
                scan_command_resource_location_argument_utf16(
                    &text.encode_utf16().collect::<Vec<_>>(),
                    0,
                    allow_tag
                )
                .unwrap(),
                end
            );
        }
        for id in [
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
            bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID,
        ] {
            let command: Vec<u16> = "😀 minecraft:overworld tail".encode_utf16().collect();
            assert_eq!(
                parse(id, &command, 3, &CommandArgumentTemplate::Unit)
                    .unwrap()
                    .unwrap()
                    .end,
                3 + "minecraft:overworld".len()
            );
        }
    }

    #[test]
    fn command_swizzle_raw_errors_and_operation_context() {
        use bbb_protocol::scan_command_swizzle_argument_utf16;
        for (units, cursor) in [
            (vec![0xdc00, 120, 0xd800, 121], 3),
            (vec![0xdc00, 120, 120], 3),
        ] {
            assert_eq!(
                scan_command_swizzle_argument_utf16(&units, 1)
                    .unwrap_err()
                    .cursor,
                cursor
            );
        }
        assert_eq!(
            scan_command_swizzle_argument_utf16(&[0xd800, 120, 121, 122, 32, 0xdc00], 1).unwrap(),
            4
        );
        assert_eq!(scan_command_swizzle_argument_utf16(&[], 0).unwrap(), 0);
        let command: Vec<u16> = "😀 xé".encode_utf16().collect();
        let error = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID,
            &command,
            3,
            &CommandArgumentTemplate::Unit,
        )
        .unwrap()
        .unwrap_err();
        assert_eq!(error.cursor, command.len());
        for suffix in ["?", " ", "+=?"] {
            let command: Vec<u16> = "😀 ".encode_utf16().chain(suffix.encode_utf16()).collect();
            let error = parse(
                VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
                &command,
                3,
                &CommandArgumentTemplate::Unit,
            )
            .unwrap()
            .unwrap_err();
            // `OperationArgument.getSimpleOperation` raises the unknown symbol
            // without a reader (OperationArgument.java:88); the raw error keeps
            // the position the token was read from, so the buffer can be mapped
            // even for a raise Vanilla reports without a context.
            assert_eq!(error.cursor, 3, "{suffix}");
        }
        let command: Vec<u16> = "😀 ".encode_utf16().collect();
        let error = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
            &command,
            command.len(),
            &CommandArgumentTemplate::Unit,
        )
        .unwrap()
        .unwrap_err();
        // An unreadable position does raise with the reader
        // (OperationArgument.java:34-37).
        assert_eq!(error.cursor, command.len());
        assert!(error.contextual);
    }

    #[test]
    fn command_snbt_raw_arguments_keep_kind_and_consumed_boundaries() {
        use bbb_protocol::{scan_command_snbt_argument_utf16, CommandSnbtKind};
        let input: Vec<_> = [0xdc00, 32]
            .into_iter()
            .chain("{text:'".encode_utf16())
            .chain([0xd800])
            .chain("'}".encode_utf16())
            .chain([32, 0xdc00])
            .collect();
        for kind in [CommandSnbtKind::Compound, CommandSnbtKind::Tag] {
            assert_eq!(
                scan_command_snbt_argument_utf16(&input, 1, kind).unwrap(),
                input.len() - 2
            );
        }
        let input: Vec<_> = [0xdc00]
            .into_iter()
            .chain("'".encode_utf16())
            .chain([0xd800])
            .chain("'".encode_utf16())
            .chain([0xdc00])
            .collect();
        assert_eq!(
            scan_command_snbt_argument_utf16(&input, 1, CommandSnbtKind::Tag).unwrap(),
            4
        );
        assert_eq!(
            scan_command_snbt_argument_utf16(&input, 1, CommandSnbtKind::Compound)
                .unwrap_err()
                .cursor,
            1
        );
        for type_id in [
            VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID,
            VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID,
        ] {
            let command: Vec<u16> = "😀 {text:'😀'} tail".encode_utf16().collect();
            let result = parse(type_id, &command, 3, &CommandArgumentTemplate::Unit)
                .unwrap()
                .unwrap();
            assert!(result.validated);
            assert_eq!(result.end, 3 + "{text:'😀'}".encode_utf16().count());
        }
    }

    #[test]
    fn command_nbt_path_errors_retain_reader_positions() {
        let cases = [
            ("foo[1.2]", 4),
            ("foo[2147483648]", 4),
            ("foo[--1]", 4),
            ("foo[1x]", 5),
            ("foo.'bad\\q'", 9),
            ("foo.''", 6),
        ];
        for (path, offset) in cases {
            let command: Vec<u16> = "😀 ".encode_utf16().chain(path.encode_utf16()).collect();
            let error = parse(
                VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID,
                &command,
                3,
                &CommandArgumentTemplate::Unit,
            )
            .unwrap()
            .unwrap_err();
            assert_eq!(error.cursor, 3 + offset, "{path}");
        }
    }

    #[test]
    fn validates_catalog_backed_names_without_parallel_membership_tables() {
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID, "dark-blue");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID, "bold");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID, "F0a");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID, "FFFF");
        assert_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
            "sidebar.team.dark_aqua",
        );
        assert_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
            "sidebar.team.orange",
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, "spectator");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, "builder");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID, "ocean_floor");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID, "ocean_floor_wg");
    }

    #[test]
    fn validates_identifiers_functions_operations_angles_and_swizzles() {
        assert_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
            "example:path/value",
        );
        assert_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
            "Example:path",
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID, "#example:tick");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID, "><");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID, "==");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID, "~-5.5");
        assert_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_ANGLE_ID,
            "~9999999999999999999999999999999999999999",
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID, "zyx");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID, "xx");
    }

    #[test]
    fn range_and_time_grammar_matches_vanilla_boundaries() {
        for range in ["0..5", "0", "-5", "-100..", "..100"] {
            assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID, range);
        }
        for range in ["0..5.2", "-5.4", "0...2"] {
            assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID, range);
        }
        for range in ["..", "5..0", "1...2", "one"] {
            assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID, range);
        }

        assert_time_valid("1.5s", 0);
        assert_time_valid("20", 20);
        assert_time_invalid("19t", 20);
        assert_time_invalid("2m", 0);
    }

    #[test]
    fn validates_entity_profiles_score_holders_and_coordinates() {
        assert_parse_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
            "@s",
            CommandArgumentTemplate::Entity {
                single: true,
                players_only: true,
            },
        );
        assert_parse_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
            "@e",
            CommandArgumentTemplate::Entity {
                single: true,
                players_only: false,
            },
        );
        assert_parse_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
            "@n",
            CommandArgumentTemplate::Entity {
                single: false,
                players_only: true,
            },
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID, "@a");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID, "@e");
        assert_parse_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID,
            "@a",
            CommandArgumentTemplate::ScoreHolder { multiple: false },
        );
        assert_parse_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID,
            "@e[limit=2]",
            CommandArgumentTemplate::ScoreHolder { multiple: true },
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID, "^1 ^ ^-5");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID, "^1 ~ ^-5");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID, "1 ~ ~.5");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID, "1.5 2 3");
    }

    #[test]
    fn validates_snbt_paths_slots_template_transforms_and_uuids() {
        assert_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID,
            "{foo:[1,2],bar:'value'}",
        );
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID, "[1,2]");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID, "[I;1,2,3]");
        assert_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID,
            "foo[{id:'minecraft:pig'}].bar",
        );
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID, "foo[bad]");

        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID, "container.53");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID, "container.*");
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID, "armor.*");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID, "armor.unknown");

        assert_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID,
            "left_right",
        );
        assert_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID,
            "LEFT_RIGHT",
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID, "180");
        assert_invalid(
            VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID,
            "clockwise_45",
        );

        assert_valid(
            VANILLA_COMMAND_ARGUMENT_TYPE_UUID_ID,
            "dd12be42-52a9-4a91-a8a1-11c01849e498",
        );
        assert_valid(VANILLA_COMMAND_ARGUMENT_TYPE_UUID_ID, "1-2-3-4-5");
        assert_invalid(VANILLA_COMMAND_ARGUMENT_TYPE_UUID_ID, "not-a-uuid");
    }

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn assert_valid(type_id: i32, input: &str) {
        let input = units(input);
        let result = parse(type_id, &input, 0, &CommandArgumentTemplate::Unit)
            .expect("owned context-free parser")
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(result.end, input.len());
        assert!(result.validated);
    }

    fn assert_invalid(type_id: i32, input: &str) {
        let input = units(input);
        let result = parse(type_id, &input, 0, &CommandArgumentTemplate::Unit)
            .expect("owned context-free parser");
        assert!(
            result.is_err() || result.is_ok_and(|value| value.end != input.len()),
            "{input:?}"
        );
    }

    fn assert_parse_valid(type_id: i32, input: &str, template: CommandArgumentTemplate) {
        let input = units(input);
        let result = parse(type_id, &input, 0, &template)
            .expect("owned context-free parser")
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(result.end, input.len());
        assert!(result.validated);
    }

    fn assert_parse_invalid(type_id: i32, input: &str, template: CommandArgumentTemplate) {
        let input = units(input);
        let result = parse(type_id, &input, 0, &template).expect("owned context-free parser");
        assert!(
            result.is_err() || result.is_ok_and(|value| value.end != input.len()),
            "{input:?}"
        );
    }

    fn assert_time_valid(input: &str, minimum: i32) {
        let input = units(input);
        let result = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
            &input,
            0,
            &CommandArgumentTemplate::Time { minimum },
        )
        .expect("time parser")
        .unwrap();
        assert_eq!(result.end, input.len());
    }

    fn assert_time_invalid(input: &str, minimum: i32) {
        assert!(parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
            &units(input),
            0,
            &CommandArgumentTemplate::Time { minimum },
        )
        .expect("time parser")
        .is_err());
    }
}
