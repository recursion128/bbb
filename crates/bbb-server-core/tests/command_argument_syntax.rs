//! The command-argument grammar has to be reachable from the server crate.
//!
//! `/setblock` and `/fill` now read their arguments through this layer:
//! `commands::parse` scans a block name with `scan_command_resource_location`
//! and the property keys and values with `read_command_string_value`, so the
//! `Identifier.read` and `StringReader.readString` character classes have one
//! implementation in this workspace rather than two. What it does *not* share
//! is `scan_command_block_body`: that scanner answers the shape and nothing
//! else, while a server-side refusal has to name the translation key and the
//! cursor the argument type itself would have raised, which is a different
//! answer for each of `BlockStateParser`'s five property refusals.
//!
//! What this file pins is the reachability the whole arrangement rests on: the
//! grammar layer sits in `bbb-protocol`, which `bbb-server-world` already
//! depends on, so the server reaches it without depending on the client's
//! world crate and without a second copy of the grammar. The `/give` half is
//! still ahead of its command.

use bbb_protocol::{
    scan_command_block_body, scan_command_block_identifier, scan_command_coordinates_argument,
    scan_command_resource_location, CommandCoordinatesKind, CommandItemComponentPatchScanner,
};

#[test]
fn the_server_crate_can_scan_a_setblock_argument_list() {
    let command = "0 ~1 ^ minecraft:chest[facing=north]{Lock:'key'}";
    let position = scan_command_coordinates_argument(command, 0, CommandCoordinatesKind::Position3);
    // A world position and a local offset cannot be mixed, exactly as the
    // client's dispatcher already refuses them.
    assert!(position.is_err());

    let command = "0 1 2 minecraft:chest[facing=north]{Lock:'key'}";
    let end = scan_command_coordinates_argument(command, 0, CommandCoordinatesKind::BlockPosition)
        .expect("a block position");
    assert_eq!(end, "0 1 2".len());

    let identifier = scan_command_block_identifier(command, end + 1, false).expect("a block name");
    assert_eq!(identifier.resource_id, "minecraft:chest");
    let body = scan_command_block_body(command, identifier.end).expect("a block body");
    assert_eq!(
        body.properties.get("facing").map(String::as_str),
        Some("north")
    );
    assert_eq!(body.end, command.len());
}

#[test]
fn the_server_crate_can_scan_a_give_argument_list() {
    let command = "minecraft:stone[minecraft:damage=1]";
    let (end, item) = scan_command_resource_location(command, 0).expect("an item name");
    assert_eq!(item, "minecraft:stone");

    let mut patch = CommandItemComponentPatchScanner::new(command, end);
    let component = patch
        .next_component()
        .expect("a component scan")
        .expect("one component");
    assert_eq!(component.component.resource_id, "minecraft:damage");
    assert!(patch.next_component().expect("the patch end").is_none());
    assert_eq!(patch.end(), command.len());
}
