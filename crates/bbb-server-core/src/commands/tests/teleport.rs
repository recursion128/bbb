//! `/teleport` and the rotation, coordinate and anchor arguments.

use bbb_protocol::packets::PlayerLookAt;

use super::*;
use crate::connection::ClientboundPlayPacket;

/// The `(position, [y_rot, x_rot], relatives_mask)` of every position packet
/// the run sent; relative axes carry the offset rather than the point.
fn moves(run: &CommandRun) -> Vec<([f64; 3], [f32; 2], i32)> {
    run.packets()
        .into_iter()
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::PlayerPosition(update) => Some((
                [update.position.x, update.position.y, update.position.z],
                [update.y_rot, update.x_rot],
                update.relatives_mask,
            )),
            _ => None,
        })
        .collect()
}

fn one_move(run: &CommandRun) -> ([f64; 3], [f32; 2], i32) {
    match moves(run).as_slice() {
        [single] => *single,
        other => panic!("one teleport, not {other:?}"),
    }
}

/// The look-at packet a facing teleport sent.
fn looked(run: &CommandRun) -> PlayerLookAt {
    match run.packets().as_slice() {
        [.., ClientboundPlayPacket::PlayerLookAt(look)] => *look,
        other => panic!("a facing teleport, not {other:?}"),
    }
}

/// Where a fixture player stands: level, position, rotation.
fn placed(world: &CommandTestWorld, name: &str) -> (Java26LevelKey, [f64; 3], [f32; 2]) {
    let player = world
        .world()
        .command_players()
        .find(|player| player.name == name)
        .expect("fixture player");
    (player.dimension.clone(), player.position, player.rotation)
}

/// The `/teleport` subtree the client is handed, node for node against the
/// derived table.
///
/// `TeleportCommand.register` (TeleportCommand.java:37-144) builds twelve nodes
/// under the root literal and one alias beside it, and the whole subtree is
/// published because [`catalog::IMPLEMENTED_COMMANDS`] is a per-root gate: a
/// node missing here would be a node the executor list can never be asked for.
#[test]
fn teleport_publishes_every_node_the_registration_class_builds() {
    let tree = usable_command_tree(CommandPermission::Owner);
    let index_of = |children: &[i32], name: &str| {
        children
            .iter()
            .copied()
            .find(|child| tree.nodes[*child as usize].name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name} is published"))
    };
    let teleport = index_of(&tree.nodes[0].children, "teleport");
    // The three children of the root literal: `<location>` (`:42`),
    // `<destination>` (`:55`) and `<targets>` (`:63`).
    let mut roots = tree.nodes[teleport as usize]
        .children
        .iter()
        .map(|child| tree.nodes[*child as usize].name.clone().unwrap())
        .collect::<Vec<_>>();
    roots.sort();
    assert_eq!(roots, ["destination", "location", "targets"]);

    let targets = index_of(&tree.nodes[teleport as usize].children, "targets");
    let mut under_targets = tree.nodes[targets as usize]
        .children
        .iter()
        .map(|child| tree.nodes[*child as usize].name.clone().unwrap())
        .collect::<Vec<_>>();
    under_targets.sort();
    // `<destination>` (`:138`) and `<location>` (`:65`).
    assert_eq!(under_targets, ["destination", "location"]);

    let location = index_of(&tree.nodes[targets as usize].children, "location");
    let mut under_location = tree.nodes[location as usize]
        .children
        .iter()
        .map(|child| tree.nodes[*child as usize].name.clone().unwrap())
        .collect::<Vec<_>>();
    under_location.sort();
    // `facing` (`:90`) and `<rotation>` (`:77`).
    assert_eq!(under_location, ["facing", "rotation"]);

    let facing = index_of(&tree.nodes[location as usize].children, "facing");
    let mut under_facing = tree.nodes[facing as usize]
        .children
        .iter()
        .map(|child| tree.nodes[*child as usize].name.clone().unwrap())
        .collect::<Vec<_>>();
    under_facing.sort();
    // `entity` (`:92`) and `<facingLocation>` (`:123`).
    assert_eq!(under_facing, ["entity", "facingLocation"]);

    let entity = index_of(&tree.nodes[facing as usize].children, "entity");
    let facing_entity = index_of(&tree.nodes[entity as usize].children, "facingEntity");
    let facing_anchor = index_of(&tree.nodes[facing_entity as usize].children, "facingAnchor");
    assert!(tree.nodes[facing_anchor as usize].children.is_empty());

    // The parsers the derived table records for the three argument types this
    // slice added a reader for.
    let parser = |index: i32| {
        tree.nodes[index as usize]
            .parser
            .as_ref()
            .map(|parser| parser.type_id)
    };
    assert_eq!(
        parser(location),
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID)
    );
    assert_eq!(
        parser(index_of(
            &tree.nodes[location as usize].children,
            "rotation"
        )),
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_ROTATION_ID)
    );
    assert_eq!(
        parser(facing_anchor),
        Some(VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID)
    );

    // Which nodes carry a command of their own: everything but the root
    // literal, `<targets>`, `facing` and `entity`
    // (TeleportCommand.java:43, :57, :66, :78, :95, :107, :124, :139).
    let executable = |index: i32| tree.nodes[index as usize].executable;
    assert!(!executable(teleport));
    assert!(!executable(targets));
    assert!(!executable(facing));
    assert!(!executable(entity));
    assert!(executable(location));
    assert!(executable(facing_entity));
    assert!(executable(facing_anchor));
}

/// `Commands.hasPermission(Commands.LEVEL_GAMEMASTERS)` on both root literals
/// (TeleportCommand.java:40, `:143`) and on `/gamemode`
/// (GameModeCommand.java:19, `:24`).
#[test]
fn teleport_and_gamemode_are_hidden_below_gamemaster() {
    let published = |permission| {
        usable_command_tree(permission).nodes[0]
            .children
            .iter()
            .filter_map(|child| {
                usable_command_tree(permission).nodes[*child as usize]
                    .name
                    .clone()
            })
            .collect::<Vec<_>>()
    };
    for name in ["teleport", "tp", "gamemode"] {
        assert!(
            !published(CommandPermission::Moderator).contains(&name.to_string()),
            "{name} is published to a moderator"
        );
        assert!(
            published(CommandPermission::GameMaster).contains(&name.to_string()),
            "{name} is missing from a gamemaster's tree"
        );
    }
    // A source that cannot see the node cannot parse it either: `parseNodes`
    // applies `canUse` where completion does not.
    let outcome = refused("teleport 0 64 0", CommandPermission::Moderator);
    assert_eq!(
        translate_key(failure_messages(&outcome)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

/// `RotationArgument.parse` (RotationArgument.java:25-40): the yaw is read
/// first and the pitch second, and `getRotation` (WorldCoordinates.java:19-22)
/// resolves each against the source's own.
#[test]
fn a_rotation_argument_reads_the_yaw_first_and_resolves_each_axis_on_its_own() {
    let alice = user("alice", 1);
    let facing = |command: &str| {
        let run = ran(
            command,
            world(std::slice::from_ref(&alice))
                .with_player_position("alice", [0.0, 0.0, 0.0])
                .with_player_rotation("alice", [45.0, 20.0])
                .with_position([0.0, 0.0, 0.0])
                .with_rotation([45.0, 20.0]),
        );
        let (_, [y_rot, x_rot], mask) = one_move(&run);
        (y_rot, x_rot, mask)
    };
    // Absolute on both axes: neither rotation bit is set and the values travel
    // as written (TeleportCommand.java:220 through `Relative.rotation`).
    let (y_rot, x_rot, mask) = facing("teleport @s 0 64 0 90 -10");
    assert_eq!((y_rot, x_rot), (90.0, -10.0));
    assert_eq!(mask & (PLAYER_RELATIVE_Y_ROT | PLAYER_RELATIVE_X_ROT), 0);
    // `~` on both: `getRotation` adds the source's own (45, 20), and
    // `performTeleport` then subtracts the victim's -- the same player here --
    // so what travels is the offset that was typed (`:248-251`).
    let (y_rot, x_rot, mask) = facing("teleport @s 0 64 0 ~10 ~");
    assert_eq!((y_rot, x_rot), (10.0, 0.0));
    assert_eq!(
        mask & (PLAYER_RELATIVE_Y_ROT | PLAYER_RELATIVE_X_ROT),
        PLAYER_RELATIVE_Y_ROT | PLAYER_RELATIVE_X_ROT
    );
    // One of each: only the axis written with `~` is marked.
    let (_, _, mask) = facing("teleport @s 0 64 0 ~10 5");
    assert_eq!(
        mask & (PLAYER_RELATIVE_Y_ROT | PLAYER_RELATIVE_X_ROT),
        PLAYER_RELATIVE_Y_ROT
    );
}

/// The two refusals `RotationArgument` can raise: its own `ERROR_NOT_COMPLETE`
/// (RotationArgument.java:15, thrown at `:28` and `:37`) and the mixed-form one
/// `WorldCoordinate.parseDouble` raises for a `^` (WorldCoordinate.java:18-20).
///
/// Both are reported only because the input reaches them as the *single*
/// exception `Commands.getParseException` (Commands.java:469-479) hands back
/// unchanged. Two shapes of the command have to cooperate for that. `@a` is
/// what keeps `<destination>` out of the way: it is `EntityArgument.entity()`
/// (TeleportCommand.java:55), which refuses a selector matching more than one
/// (EntityArgument.java:110-117), so the top-level branch that would otherwise
/// stop after the first word never becomes a candidate. A `~` destination is
/// what keeps the `<destination>` *under* `<targets>` out of the way, because
/// `parseNameOrUUID` (EntitySelectorParser.java:261-283) reads an unquoted
/// string and `~` is not in that character set. What is left is one child, and
/// so one exception.
#[test]
fn a_rotation_argument_refuses_a_half_pair_and_the_local_form() {
    let alice = user("alice", 1);
    let refuse = |command: &str| {
        let run = ran(command, world(std::slice::from_ref(&alice)));
        keys(&run)
            .into_iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>()
    };
    // One number is a yaw with no pitch after it (RotationArgument.java:36-37).
    assert_eq!(
        refuse("teleport @a ~ ~ ~ 90"),
        ["argument.rotation.incomplete", CURSOR_CONTEXT]
    );
    // A `^` is refused by the coordinate reader rather than by the node.
    assert_eq!(
        refuse("teleport @a ~ ~ ~ ^1 ^2"),
        ["argument.pos.mixed", CURSOR_CONTEXT]
    );
}

/// `Vec3Argument.parse` (Vec3Argument.java:46-48) picks the form from the first
/// character and never lets one triple hold both: an axis that switches is
/// `argument.pos.mixed` either way round (WorldCoordinate.java:18-20 for a `^`
/// after a world axis, LocalCoordinates.java:61-63 for the other order).
///
/// The publisher tested here is `/damage`'s `at <location>`
/// (DamageCommand.java:48-50) rather than `/teleport`'s, because the `at`
/// literal has exactly one child and so the coordinate reader's refusal is the
/// only exception in the parse. Under `/teleport <targets>` the same input also
/// reaches `<destination>`, which reads a bare `1` as a player name, and two
/// candidates is what `Commands.getParseException` (Commands.java:475-477)
/// answers with the unknown-argument error instead.
#[test]
fn a_coordinate_triple_may_not_mix_the_two_forms() {
    let alice = user("alice", 1);
    let refuse = |command: &str| {
        let run = ran(command, world(std::slice::from_ref(&alice)));
        keys(&run)
            .into_iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        refuse("damage alice 1 minecraft:generic at 1 ^2 3"),
        ["argument.pos.mixed", CURSOR_CONTEXT]
    );
    assert_eq!(
        refuse("damage alice 1 minecraft:generic at ^1 2 ^3"),
        ["argument.pos.mixed", CURSOR_CONTEXT]
    );
    // Two axes is not a triple: `ERROR_NOT_COMPLETE` (Vec3Argument.java:22).
    assert_eq!(
        refuse("damage alice 1 minecraft:generic at ^1 ^2"),
        ["argument.pos3d.incomplete", CURSOR_CONTEXT]
    );
}

/// `LocalCoordinates.getPosition` (LocalCoordinates.java:13-16) turns the
/// `^left ^up ^forwards` triple with the source's own rotation, and the source
/// this dispatcher builds always looks from its feet.
#[test]
fn a_local_teleport_turns_the_offset_with_the_sources_rotation() {
    let alice = user("alice", 1);
    let landed = |source_rotation: [f32; 2], command: &str| {
        let run = ran(
            command,
            world(std::slice::from_ref(&alice))
                .with_position([0.0, 0.0, 0.0])
                .with_rotation(source_rotation),
        );
        // The relative axes carry the difference from the victim, who stands at
        // the origin here, so the packet's numbers are the landing point.
        one_move(&run).0
    };
    // A source looking due south (yaw zero) has `forwards` along +Z.
    let [x, y, z] = landed([0.0, 0.0], "teleport @s ^ ^ ^5");
    assert!(x.abs() < 1.0e-3, "{x}");
    assert!(y.abs() < 1.0e-3, "{y}");
    assert!((z - 5.0).abs() < 1.0e-3, "{z}");
    // Turned a quarter to the west (yaw 90), the same `forwards` is along -X.
    let [x, y, z] = landed([90.0, 0.0], "teleport @s ^ ^ ^5");
    assert!((x + 5.0).abs() < 1.0e-3, "{x}");
    assert!(y.abs() < 1.0e-3, "{y}");
    assert!(z.abs() < 1.0e-3, "{z}");
    // Looking straight down (pitch 90), `up` is along +Z and `forwards` is -Y.
    let [x, y, z] = landed([0.0, 90.0], "teleport @s ^ ^ ^5");
    assert!(x.abs() < 1.0e-3, "{x}");
    assert!((y + 5.0).abs() < 1.0e-3, "{y}");
    assert!(z.abs() < 1.0e-3, "{z}");
}

/// `EntityAnchorArgument.parse` (EntityAnchorArgument.java:38-48) reads an
/// unquoted word and looks it up in `Anchor.BY_NAME`.
#[test]
fn an_entity_anchor_argument_reads_the_two_names_and_refuses_the_rest() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let players = [alice, bob];
    let anchor_of = |command: &str| {
        let run = ran(command, world(&players));
        looked(&run).target.expect("an entity look-at").to_anchor
    };
    assert_eq!(
        anchor_of("teleport @s 0 64 0 facing entity bob feet"),
        EntityAnchor::Feet
    );
    assert_eq!(
        anchor_of("teleport @s 0 64 0 facing entity bob eyes"),
        EntityAnchor::Eyes
    );
    // The node without the anchor supplies `FEET` itself
    // (TeleportCommand.java:102).
    assert_eq!(
        anchor_of("teleport @s 0 64 0 facing entity bob"),
        EntityAnchor::Feet
    );
    // `ERROR_INVALID` (EntityAnchorArgument.java:26-28) carries the word that
    // named no anchor. `@a` and the `~` destination are what leave the
    // `<facingAnchor>` refusal as the parse's only exception -- see
    // [`a_rotation_argument_refuses_a_half_pair_and_the_local_form`] for why.
    let run = ran("teleport @a ~ ~ ~ facing entity bob head", world(&players));
    assert_eq!(keys(&run), ["argument.anchor.invalid", CURSOR_CONTEXT]);
}

/// `EntityAnchorArgument.listSuggestions` (EntityAnchorArgument.java:51-53)
/// offers both names, and `RotationArgument` offers none because it declares no
/// `listSuggestions` at all (RotationArgument.java:13-45).
///
/// The `@a` and the `~` destination are load-bearing here for the same reason
/// they are in [`a_rotation_argument_refuses_a_half_pair_and_the_local_form`]:
/// `getCompletionSuggestions` offers the children of the node the *chosen*
/// parse reached, and a `/teleport <name> ..` whose first word also parses as
/// `<destination>` is a parse that stopped after that word.
#[test]
fn the_anchor_argument_offers_two_names_and_the_rotation_argument_offers_none() {
    let completions = completions_for(
        "teleport @a ~ ~ ~ facing entity bob ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["eyes", "feet"]);
    let completions = completions_for(
        "teleport @a ~ ~ ~ facing entity bob e",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["eyes"]);
    let completions = completions_for(
        "teleport @a ~ ~ ~ ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    // The two children of `<location>` are the `facing` literal and the
    // `<rotation>` argument; only the literal offers anything.
    assert_eq!(texts(&completions), ["facing"]);
}

/// `TeleportCommand.teleportToPos` (TeleportCommand.java:174-215) over the
/// source's own entity: the destination is reported with
/// `String.format(Locale.ROOT, "%f", ..)` (`:224-226`), which is six fraction
/// digits, and the centre correction has already moved a whole horizontal
/// coordinate onto the middle of its block (WorldCoordinate.java:34-36).
#[test]
fn teleport_to_a_position_reports_six_fraction_digits_of_the_corrected_point() {
    let alice = user("alice", 1);
    let mut moved = world(std::slice::from_ref(&alice));
    let run = ran("teleport 10 64 20", &mut moved);
    assert_eq!(keys(&run), ["commands.teleport.success.location.single"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Success]);
    assert_eq!(
        arguments(&run, 0),
        [
            display_name("alice", 1),
            text("10.500000"),
            text("64.000000"),
            text("20.500000"),
        ]
    );
    assert_eq!(
        one_move(&run),
        (
            [10.5, 64.0, 20.5],
            [0.0, 0.0],
            // No coordinate was written with `~`, so only the two rotation
            // axes are relative -- which is how "keep the rotation you had" is
            // spelled (`Relative.ROTATION`, Relative.java:22).
            PLAYER_RELATIVE_Y_ROT | PLAYER_RELATIVE_X_ROT,
        )
    );
    assert_eq!(
        placed(&moved, "alice"),
        (
            level_key(Java26BuiltinDimension::Overworld),
            [10.5, 64.0, 20.5],
            [0.0, 0.0]
        )
    );
    assert_eq!(run.result, Some(1));
}

/// `TeleportCommand.getRelatives` (TeleportCommand.java:217-222): a `~` axis
/// marks both the delta axis and the position axis, and the position half is
/// dropped for an entity that is changing level (`:219`).
#[test]
fn a_relative_teleport_marks_its_axes_and_drops_the_position_half_across_levels() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let players = [alice, bob];

    // Same level: every axis relative, and the packet carries the offset from
    // the victim rather than the point.
    let run = ran(
        "teleport alice ~5 ~ ~-2",
        world(&players)
            .with_player_position("alice", [1.0, 2.0, 3.0])
            .with_player_position("bob", [1.0, 2.0, 3.0])
            .with_player_level("bob", level_key(Java26BuiltinDimension::Nether))
            .with_position([1.0, 2.0, 3.0]),
    );
    assert_eq!(
        one_move(&run),
        (
            [5.0, 0.0, -2.0],
            [0.0, 0.0],
            PLAYER_RELATIVE_X
                | PLAYER_RELATIVE_Y
                | PLAYER_RELATIVE_Z
                | PLAYER_RELATIVE_DELTA_X
                | PLAYER_RELATIVE_DELTA_Y
                | PLAYER_RELATIVE_DELTA_Z
                | PLAYER_RELATIVE_Y_ROT
                | PLAYER_RELATIVE_X_ROT,
        )
    );
    // The point itself is what the feedback reports either way: `~` was
    // resolved against the source (WorldCoordinates.java:13-16).
    assert_eq!(
        arguments(&run, 0),
        [
            display_name("alice", 1),
            text("6.000000"),
            text("2.000000"),
            text("1.000000"),
        ]
    );

    // Bob is in the nether and the source is not, so the three position bits
    // are dropped and the destination travels absolute.
    let mut pulled = world(&players)
        .with_player_position("alice", [1.0, 2.0, 3.0])
        .with_player_position("bob", [1.0, 2.0, 3.0])
        .with_player_level("bob", level_key(Java26BuiltinDimension::Nether))
        .with_position([1.0, 2.0, 3.0]);
    let run = ran("teleport bob ~5 ~ ~-2", &mut pulled);
    assert_eq!(
        one_move(&run),
        (
            [6.0, 2.0, 1.0],
            [0.0, 0.0],
            PLAYER_RELATIVE_DELTA_X
                | PLAYER_RELATIVE_DELTA_Y
                | PLAYER_RELATIVE_DELTA_Z
                | PLAYER_RELATIVE_Y_ROT
                | PLAYER_RELATIVE_X_ROT,
        )
    );
    // `c.getSource().getLevel()` (TeleportCommand.java:70): the target is
    // pulled into the source's level, not left in its own.
    assert_eq!(
        placed(&pulled, "bob").0,
        level_key(Java26BuiltinDimension::Overworld)
    );
}

/// `TeleportCommand.performTeleport` (TeleportCommand.java:240-243) refuses a
/// destination outside `Level.isInSpawnableBounds` (Level.java:175-177) before
/// it moves anything.
#[test]
fn teleport_refuses_a_destination_outside_the_spawnable_bounds() {
    let alice = user("alice", 1);
    let refuse = |command: &str| {
        let run = ran(command, world(std::slice::from_ref(&alice)));
        assert!(moves(&run).is_empty(), "{:?}", run.packets());
        keys(&run)
            .into_iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>()
    };
    // `isOutsideSpawnableHeight` (`:189-191`) is a half-open window, so the
    // upper bound itself is out.
    assert_eq!(
        refuse("teleport 0 20000000 0"),
        ["commands.teleport.invalidPosition"]
    );
    assert_eq!(
        refuse("teleport 0 -20000001 0"),
        ["commands.teleport.invalidPosition"]
    );
    // `isInWorldBoundsHorizontal` (`:179-181`) is the same shape on X and Z.
    assert_eq!(
        refuse("teleport 30000000 64 0"),
        ["commands.teleport.invalidPosition"]
    );
    // One block inside the window is accepted.
    let run = ran("teleport 0 19999999 0", world(std::slice::from_ref(&alice)));
    assert_eq!(keys(&run), ["commands.teleport.success.location.single"]);
}

/// `TeleportCommand.teleportToEntity` (TeleportCommand.java:146-172): the whole
/// placement comes from the destination and nothing is relative (`:155`).
#[test]
fn teleport_to_an_entity_takes_its_level_position_and_rotation() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let players = [alice, bob];
    let mut joined = world(&players)
        .with_player_position("bob", [4.0, 5.0, 6.0])
        .with_player_rotation("bob", [30.0, -5.0])
        .with_player_level("bob", level_key(Java26BuiltinDimension::Nether));
    let run = ran("teleport bob", &mut joined);
    assert_eq!(one_move(&run), ([4.0, 5.0, 6.0], [30.0, -5.0], 0));
    assert_eq!(
        placed(&joined, "alice"),
        (
            level_key(Java26BuiltinDimension::Nether),
            [4.0, 5.0, 6.0],
            [30.0, -5.0]
        )
    );
    assert_eq!(keys(&run), ["commands.teleport.success.entity.single"]);
    assert_eq!(
        arguments(&run, 0),
        [display_name("alice", 1), display_name("bob", 2)]
    );
    assert_eq!(run.result, Some(1));
}

/// `EntityArgument.entities()` on `<targets>` (TeleportCommand.java:63) is the
/// one published node that accepts `@e`, and a run over more than one target
/// reports the count rather than the names (`:168`, `:206-211`).
#[test]
fn teleport_over_many_targets_counts_them_in_the_report() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let carol = user("carol", 3);
    let players = [alice, bob, carol];
    let run = ran("teleport @e 0 64 0", world(&players));
    assert_eq!(moves(&run).len(), 3);
    assert_eq!(keys(&run), ["commands.teleport.success.location.multiple"]);
    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::Int(3),
            text("0.500000"),
            text("64.000000"),
            text("0.500000"),
        ]
    );
    assert_eq!(run.result, Some(3));

    // `EntityArgument.getEntities` (EntityArgument.java:60-67) refuses an empty
    // result with the entity key rather than the player one.
    let run = ran("teleport nobody 0 64 0", world(&players));
    assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
    assert!(moves(&run).is_empty());
}

/// `LookAt.LookAtPosition` (LookAt.java:24-29) resolves its coordinates with
/// `Vec3Argument.getVec3` (Vec3Argument.java:38-40) -- the source applied at
/// parse time -- while `LookAtEntity` keeps the entity so the client can follow
/// it.
#[test]
fn teleport_facing_carries_a_resolved_point_or_the_entity_it_names() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let players = [alice, bob];
    let mut facing = world(&players);
    let bob_id = facing
        .world()
        .command_players()
        .find(|player| player.name == "bob")
        .expect("bob")
        .entity_id;
    let point = looked(&ran("teleport @s 0 64 0 facing 1 2 3", &mut facing));
    assert_eq!(
        [point.position.x, point.position.y, point.position.z],
        [1.5, 2.0, 3.5]
    );
    assert_eq!(point.target, None);
    let entity = looked(&ran(
        "teleport @s 0 64 0 facing entity bob eyes",
        &mut facing,
    ));
    let target = entity.target.expect("an entity look-at");
    assert_eq!(
        (target.entity_id, target.to_anchor),
        (bob_id, EntityAnchor::Eyes)
    );
    // The targets are resolved before the facing entity is, so a `<targets>`
    // that matches nobody is what a run naming two unknowns reports.
    let run = ran(
        "teleport nobody 0 64 0 facing entity nobody",
        world(&players),
    );
    assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
}

/// `ExecuteCommand.java:255-260` through `TeleportCommand.teleportToPos`
/// (`TeleportCommand.java:174-215`): `/execute in` retargets the source level
/// the position branch reads, so the victim lands in the named dimension.
#[test]
fn execute_in_retargets_teleport_onto_the_named_level() {
    let alice = user("alice", 1);

    let mut moved = world(std::slice::from_ref(&alice));
    let run = ran(
        "execute in minecraft:the_nether run tp @s 100 70 -20",
        &mut moved,
    );
    assert_eq!(keys(&run), ["commands.teleport.success.location.single"]);
    assert_eq!(run.result, Some(1));
    assert_eq!(
        placed(&moved, "alice"),
        (
            level_key(Java26BuiltinDimension::Nether),
            [100.5, 70.0, -19.5],
            [0.0, 0.0]
        )
    );
}

/// `c.getSource().getEntityOrException()` (TeleportCommand.java:46, `:58`),
/// whose `ERROR_NOT_ENTITY` (CommandSourceStack.java:53) is
/// `permissions.requires.entity`.
#[test]
fn teleport_without_a_source_entity_is_refused_before_anything_moves() {
    let bob = user("bob", 2);
    let offstage = source("console", CommandPermission::Owner);
    let run = ran_as(
        "teleport 0 64 0",
        &offstage,
        world(std::slice::from_ref(&bob)),
    );
    assert_eq!(keys(&run), ["permissions.requires.entity"]);
    assert!(moves(&run).is_empty());
    let run = ran_as("teleport bob", &offstage, world(std::slice::from_ref(&bob)));
    assert_eq!(keys(&run), ["permissions.requires.entity"]);
}

#[test]
fn teleport_player_faces_a_stored_target_with_source_and_target_anchors() {
    let players = [user("alice", 1), user("bob", 2)];
    let stored = [stored_entity(
        45,
        20,
        [10.0, 64.0, 0.0],
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    )];
    let mut fixture = world(&players)
        .with_entities(&stored)
        .with_player_pose(
            "alice",
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
        )
        .with_player_attributes("alice", |attributes| {
            attributes
                .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
                .unwrap()
                .set_base_value(2.0);
        })
        .with_player_position("bob", [10.0, 64.0, 0.0])
        .with_player_attributes("bob", |attributes| {
            attributes
                .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
                .unwrap()
                .set_base_value(0.5);
        });
    let feet = looked(&ran(
        "teleport alice 0.0 64.0 0.0 facing entity 0-0-0-0-2d",
        &mut fixture,
    ));
    assert_eq!(feet.from_anchor, EntityAnchor::Feet);
    assert_eq!(feet.position.y, 64.0);
    assert_eq!(feet.target.unwrap().to_anchor, EntityAnchor::Feet);
    let eyes = looked(&ran(
        "execute anchored eyes run teleport alice 0.0 64.0 0.0 facing entity 0-0-0-0-2d eyes",
        &mut fixture,
    ));
    assert_eq!(eyes.from_anchor, EntityAnchor::Eyes);
    let target = eyes.target.unwrap();
    assert_eq!(target.to_anchor, EntityAnchor::Eyes);
    let reads = fixture.world().entity_reads();
    let stand = reads.stored_entities().next().unwrap();
    assert_eq!(target.entity_id, stand.entity.id());
    assert_eq!(eyes.position.y, 64.0 + f64::from(stand.entity.eye_height()));
    let pitch = reads
        .players()
        .find(|player| player.name == "alice")
        .unwrap()
        .rotation[1];
    assert!(
        pitch > 4.0 && pitch < 5.0,
        "the player looks from its own eyes to the stand's eyes: {pitch}"
    );
    let player_eyes = looked(&ran(
        "execute anchored eyes run teleport alice 0.0 64.0 0.0 facing entity bob eyes",
        &mut fixture,
    ));
    let reads = fixture.world().entity_reads();
    let bob = reads.players().find(|player| player.name == "bob").unwrap();
    assert_eq!(player_eyes.position.y, 64.0 + f64::from(bob.eye_height));
    assert_eq!(player_eyes.from_anchor, EntityAnchor::Eyes);
    let pitch = reads
        .players()
        .find(|player| player.name == "alice")
        .unwrap()
        .rotation[1];
    assert!(
        pitch > 9.0 && pitch < 10.0,
        "crouched scaled source and scaled target agree with the packet: {pitch}"
    );
}
