//! The ally term of the creaking's two readers over the live scoreboard:
//! `Creaking.checkCanMove`'s own test and
//! `TargetingConditions.test`'s combat arm behind the player
//! sensor, both through `Entity.isAlliedTo` on the
//! real tick.

use std::collections::BTreeMap;

use bbb_protocol::entity_synched_data::{
    VANILLA_CREAKING_CAN_MOVE_DATA_ID, VANILLA_CREAKING_IS_ACTIVE_DATA_ID,
};
use bbb_protocol::packets::EntityDataValueKind;

use uuid::Uuid;

use super::helpers::{
    creaking_fixture_with_teams as fixture_with_teams, creaking_lay_corridor as lay_corridor,
    creaking_move_player as move_player, creaking_place_heart, creaking_protector as protector,
    creaking_spawn_protector as spawn_protector, creaking_synched_rows as synched_rows,
    level_sounds, uuid_scanning_players_immediately, CREAKING_HEART as HEART,
    CREAKING_LOOKING_WEST as LOOKING_WEST, CREAKING_PROTECTOR_POS as PROTECTOR_POSITION,
    CREAKING_WATCHER_POS as WATCHER_POSITION,
};
use super::ServerWorld;

fn place_heart(world: &mut ServerWorld, at: bbb_protocol::packets::BlockPos) {
    creaking_place_heart(world, at, "awake");
}

/// What one watch left behind.
struct Watch {
    /// The sounds of the tick `checkCanMove` decides on.
    sounds: Vec<String>,
    /// The synched rows the two ticks after it published for the protector.
    rows: BTreeMap<u8, EntityDataValueKind>,
    can_move: bool,
    is_active: bool,
}

/// The one watch every case runs: a protector bound to the heart of `HEART`
/// and a watcher four blocks east of it staring straight at it, over a save
/// whose scoreboard carries one member list per team.
fn watch_with_teams(uuid: Uuid, teams: Vec<Vec<String>>) -> Watch {
    let (sender, receiver, mut world) = fixture_with_teams(teams);
    lay_corridor(&mut world);
    place_heart(&mut world, HEART);
    let entity_id = spawn_protector(&mut world, PROTECTOR_POSITION, uuid, HEART);

    // Tick one: the watcher's own move packet lands, the brain is built and
    // both sensors scan with a zero delay, so `checkCanMove` reads her from the
    // next tick on.
    let first = move_player(
        &sender,
        &receiver,
        &mut world,
        Some(WATCHER_POSITION),
        LOOKING_WEST,
    );
    assert!(
        level_sounds(&first).is_empty(),
        "{:?}",
        level_sounds(&first)
    );

    // Tick two is the decision; tick three publishes the rows it wrote.
    let decision = world.tick(receiver.drain());
    let row_tick = world.tick(receiver.drain());
    let entity = protector(&world, entity_id);
    Watch {
        sounds: level_sounds(&decision),
        rows: synched_rows(&row_tick, entity_id),
        can_move: entity.creaking_can_move(),
        is_active: entity.creaking_is_active(),
    }
}

/// `!this.isAlliedTo(player)`: a player on the
/// protector's own scoreboard team is not a target, so her stare neither
/// activates nor freezes it.
///
/// The two names `Entity.getTeam` is keyed by are the
/// protector's UUID string and the player's profile name
/// which is why the team below carries both.
#[test]
fn a_same_team_player_does_not_freeze_the_protector() {
    let uuid = uuid_scanning_players_immediately();
    let watch = watch_with_teams(uuid, vec![vec![uuid.to_string(), "alice".to_string()]]);
    assert!(
        watch.sounds.is_empty(),
        "an ally is no potential target, so nothing activates: {:?}",
        watch.sounds
    );
    assert!(
        watch.can_move,
        "`CAN_MOVE` never moved off its define-time true"
    );
    assert!(
        !watch.is_active,
        "`IS_ACTIVE` never moved off its define-time false"
    );
    assert_eq!(
        watch.rows.get(&VANILLA_CREAKING_CAN_MOVE_DATA_ID),
        None,
        "no row was written, so none was published"
    );
    assert_eq!(watch.rows.get(&VANILLA_CREAKING_IS_ACTIVE_DATA_ID), None);
}

/// `Team.isAlliedTo` is one and the same team, so a player
/// on no team and a player on a different team from the protector are both
/// non-allies and the stare freezes it.
#[test]
fn a_player_on_another_team_still_freezes_the_protector() {
    let uuid = uuid_scanning_players_immediately();
    for teams in [
        vec![vec!["alice".to_string()]],
        vec![vec![uuid.to_string()], vec!["alice".to_string()]],
    ] {
        let watch = watch_with_teams(uuid, teams);
        assert_eq!(
            watch.sounds,
            vec![
                "minecraft:entity.creaking.activate".to_string(),
                "minecraft:entity.creaking.freeze".to_string(),
            ],
            "a non-ally still activates and freezes the protector"
        );
        assert!(!watch.can_move, "`checkCanMove` froze the body");
        assert!(watch.is_active, "`activate` wrote `IS_ACTIVE`");
        assert_eq!(
            watch.rows.get(&VANILLA_CREAKING_CAN_MOVE_DATA_ID),
            Some(&EntityDataValueKind::Boolean(false))
        );
        assert_eq!(
            watch.rows.get(&VANILLA_CREAKING_IS_ACTIVE_DATA_ID),
            Some(&EntityDataValueKind::Boolean(true))
        );
    }
}

/// The same watch with no scoreboard at all: neither body is on a team, so the
/// pair is not allied and the protector freezes the way it did before the ally
/// term existed.
#[test]
fn a_watch_with_no_scoreboard_still_freezes_the_protector() {
    let watch = watch_with_teams(uuid_scanning_players_immediately(), Vec::new());
    assert_eq!(
        watch.sounds,
        vec![
            "minecraft:entity.creaking.activate".to_string(),
            "minecraft:entity.creaking.freeze".to_string(),
        ]
    );
    assert!(!watch.can_move);
    assert!(watch.is_active);
}
