//! `/spawnpoint` (`SetSpawnCommand.java:23-97`).

use super::*;

/// Bare `/spawnpoint` (`SetSpawnCommand.java:27-34`) uses the source player
/// and `BlockPos.containing(source.getPosition())`.
#[test]
fn spawnpoint_sets_the_source_player() {
    let players = [user("alice", 1)];
    let source = source("alice", CommandPermission::Owner);
    let mut spawned = world(&players).with_position([10.7, 70.2, -3.1]);
    let run = ran_as("spawnpoint", &source, &mut spawned);
    assert_eq!(
        spawned.respawn("alice"),
        Some((
            level_key(Java26BuiltinDimension::Overworld),
            BlockPos {
                x: 10,
                y: 70,
                z: -4
            },
            0.0,
            0.0,
        ))
    );
    assert_eq!(keys(&run), ["commands.spawnpoint.success.single"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(1));
}

/// `/spawnpoint <targets> <pos> <rotation>` (`SetSpawnCommand.java:56-63`).
#[test]
fn spawnpoint_sets_named_targets_at_a_spawnable_pos() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut spawned = world(&players);
    let run = ran("spawnpoint @a 10 70 20 90 -20", &mut spawned);
    for name in ["alice", "bob"] {
        assert_eq!(
            spawned.respawn(name),
            Some((
                level_key(Java26BuiltinDimension::Overworld),
                BlockPos {
                    x: 10,
                    y: 70,
                    z: 20
                },
                90.0,
                -20.0,
            ))
        );
    }
    assert_eq!(keys(&run), ["commands.spawnpoint.success.multiple"]);
    assert_eq!(run.result, Some(2));
}

#[test]
fn spawnpoint_from_the_console_needs_a_player() {
    let run = ran("spawnpoint", CommandTestWorld::default());
    assert_eq!(keys(&run), ["permissions.requires.player"]);
}

/// `SetSpawnCommand.register` (`SetSpawnCommand.java:26`) gates the root on
/// `LEVEL_GAMEMASTERS`.
#[test]
fn spawnpoint_is_hidden_from_a_moderator() {
    let refusal = refused("spawnpoint", CommandPermission::Moderator);
    assert_eq!(keys(&refusal), ["command.unknown.command", CURSOR_CONTEXT]);
}
