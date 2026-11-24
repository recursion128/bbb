//! `Entity.getVisibilityPercent`'s player-side terms
//! over the live `PlayerSensor`: the crouch flag a session's own input packet
//! writes and the invisibility flag its effect list drives, both read through
//! `TargetingConditions.test`'s range term
//! and the `minecraft:brains` dump the protector publishes.

use super::*;

use bbb_protocol::debug_subscription_types::VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID;
use bbb_protocol::packets::{DebugBrainDump, PlayerInput, PlayerRotation};

use crate::tests::{chat_command, place_player, play, PlayServerbound};

use super::helpers::{
    brain_dumps, creaking_fixture_with_teams as fixture_with_teams, creaking_lay_floor,
    creaking_memory_line as memory_line, creaking_place_heart, creaking_protector as protector,
    creaking_spawn_protector as spawn_protector, level_sounds, uuid_scanning_players_immediately,
    CREAKING_HEART as HEART, CREAKING_LOOKING_WEST as LOOKING_WEST,
    CREAKING_PROTECTOR_POS as PROTECTOR_POSITION,
};

fn lay_floor(world: &mut ServerWorld, xs: std::ops::RangeInclusive<i32>) {
    creaking_lay_floor(world, xs, 66);
}

fn place_heart(world: &mut ServerWorld, at: BlockPos) {
    creaking_place_heart(world, at, "awake");
}

/// How the watcher's own session state is moved before the sensors scan.
enum Watcher {
    /// The control: nothing is written to the session.
    Plain,
    /// `Entity.setShiftKeyDown` from the input packet
    /// handler, which is
    /// `Entity.isDiscrete`.
    Crouching,
    /// `/effect give <player> invisibility`,
    /// which `updateInvisibilityStatus` turns into the shared flag
    Invisible,
    /// The same invisibility with these `armor.*` slots filled, which is what
    /// `LivingEntity.getArmorCoverPercentage` counts
    InvisibleWearing(&'static [(&'static str, &'static str)]),
}

/// What one watch read out of the protector.
struct Watch {
    /// The protector's brain at its first scan, kept before any walk this
    /// window allows moved the body.
    dump: DebugBrainDump,
    /// Every level sound the three ticks published.
    sounds: Vec<String>,
    /// `Creaking.canMove`, which `checkCanMove` writes.
    can_move: bool,
}

impl Watch {
    /// `NEAREST_PLAYERS`, the scan before any
    /// condition.
    fn nearest_players(&self) -> bool {
        self.holds("nearest_players")
    }

    /// `NEAREST_VISIBLE_PLAYER`, the non-combat condition's answer (`:37`).
    fn visible(&self) -> bool {
        self.holds("nearest_visible_player")
    }

    /// `NEAREST_VISIBLE_ATTACKABLE_PLAYERS`, the combat condition's (`:39`).
    fn attackable(&self) -> bool {
        self.holds("nearest_visible_targetable_players")
    }

    /// One memory line, as "does it hold anything".
    fn holds(&self, module: &str) -> bool {
        memory_line(&self.dump, module).is_some_and(|line| !line.ends_with(": -"))
    }
}

/// The first dump of a tick whose player scan is present: the sensors scan on
/// the protector's first tick (`uuid_scanning_players_immediately`) and only
/// rewrite their memories on the next scan, so the lines keep the geometry the
/// decision was made at.
fn note_scan(slot: &mut Option<DebugBrainDump>, tick: &TickResult, entity_id: i32) {
    if slot.is_some() {
        return;
    }
    if let Some(dump) = brain_dumps(tick).remove(&entity_id) {
        if memory_line(&dump, "nearest_players").is_some() {
            *slot = Some(dump);
        }
    }
}

/// A protector at `PROTECTOR_POSITION` bound to the heart of `HEART`, and one
/// watcher `distance` blocks east of it staring straight back over a corridor
/// that reaches her.
fn watch(watcher: Watcher, distance: f64) -> Watch {
    let uuid = uuid_scanning_players_immediately();
    let (sender, receiver, mut world) = fixture_with_teams(Vec::new());
    chat_command(&sender, &receiver, &mut world, 1, "gamemode survival alice");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::DebugSubscriptionRequest {
            subscription_ids: vec![VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID],
        },
    );
    lay_floor(&mut world, -2..=34);
    place_heart(&mut world, HEART);
    // The session state is written before the protector exists, so its first
    // tick is the one the watcher's own move packet lands on.
    match watcher {
        Watcher::Plain => {}
        Watcher::Crouching => {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::PlayerInput(PlayerInput {
                    shift: true,
                    ..PlayerInput::default()
                }),
            );
        }
        Watcher::Invisible => {
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "effect give alice invisibility",
            );
        }
        Watcher::InvisibleWearing(pieces) => {
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "effect give alice invisibility",
            );
            for (slot, item) in pieces {
                chat_command(
                    &sender,
                    &receiver,
                    &mut world,
                    1,
                    &format!("item replace entity alice {slot} with {item}"),
                );
            }
        }
    }
    let entity_id = spawn_protector(&mut world, PROTECTOR_POSITION, uuid, HEART);
    let position = Vec3d {
        x: PROTECTOR_POSITION.x + distance,
        y: PROTECTOR_POSITION.y,
        z: PROTECTOR_POSITION.z,
    };
    let first = place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        position,
        Some(PlayerRotation {
            y_rot: LOOKING_WEST,
            x_rot: 0.0,
        }),
    );
    // Every distance below is measured from where the watcher's own packet put
    // her, so a refused one must not pass silently.
    assert_eq!(
        world
            .publication()
            .players
            .iter()
            .find(|player| player.connection_id == ConnectionId(1))
            .map(|player| player.position),
        Some(position),
        "the watcher's move packet landed"
    );
    let mut dump = None;
    let mut sounds = level_sounds(&first);
    note_scan(&mut dump, &first, entity_id);
    for _ in 0..2 {
        let tick = world.tick(receiver.drain());
        sounds.extend(level_sounds(&tick));
        note_scan(&mut dump, &tick, entity_id);
    }
    Watch {
        dump: dump.expect("the protector publishes its player scan"),
        sounds,
        can_move: protector(&world, entity_id).creaking_can_move(),
    }
}

/// The 0.8 `isDiscrete` term: the same watcher the scan keeps is dropped by
/// both targeting conditions once she crouches at a distance the follow range
/// alone would have reached.
///
/// `Sensor.updateTargetingConditionRanges` moves both
/// conditions to the protector's 32-block `FOLLOW_RANGE`, and the range term is
/// `max(32 * 0.8, 2.0)` = 25.6 blocks.
#[test]
fn a_crouching_player_leaves_the_target_lists_at_eighty_percent_of_the_range() {
    let control = watch(Watcher::Plain, 26.0);
    assert!(
        control.nearest_players() && control.visible() && control.attackable(),
        "the control at twenty-six blocks is inside every list"
    );

    let crouching = watch(Watcher::Crouching, 26.0);
    assert!(
        crouching.nearest_players(),
        "`PlayerSensor` filters spectators and the follow range only (`PlayerSensor.java:28-42`)"
    );
    assert!(
        !crouching.visible(),
        "`forNonCombat` reads `getVisibilityPercent` (`TargetingConditions.java:82`)"
    );
    assert!(
        !crouching.attackable(),
        "`forCombat` reads the same range term"
    );
}

/// The invisible term `0.7 * getArmorCoverPercentage` over the same sensor, and
/// the freeze path that does not read it: an invisible watcher four blocks away
/// still stares the protector to a stop while both target lists drop her.
#[test]
fn an_invisible_player_leaves_the_target_lists_but_still_freezes_the_protector() {
    let control = watch(Watcher::Plain, 4.0);
    assert!(
        control.visible() && control.attackable(),
        "four blocks are inside the follow range for a visible watcher"
    );

    let invisible = watch(Watcher::Invisible, 4.0);
    assert!(
        invisible.nearest_players(),
        "`checkCanMove` reads the unfiltered scan (`Creaking.java:454-489`)"
    );
    assert!(
        !invisible.visible() && !invisible.attackable(),
        "`max(32 * 0.7 * 0.1, 2.0)` = 2.24 blocks"
    );
    assert_eq!(
        invisible.sounds,
        vec![
            "minecraft:entity.creaking.activate".to_string(),
            "minecraft:entity.creaking.freeze".to_string(),
        ],
        "the stare still activates and freezes the body"
    );
    assert!(!invisible.can_move);
}

/// Every `HUMANOID_ARMOR` slot of `EquipmentSlotGroup.ARMOR`
/// (`EquipmentSlotGroup.java:23`, `EquipmentSlot.java:15-18`), which is the
/// whole denominator of `getArmorCoverPercentage`.
const FULL_ARMOUR: &[(&str, &str)] = &[
    ("armor.head", "diamond_helmet"),
    ("armor.chest", "diamond_chestplate"),
    ("armor.legs", "diamond_leggings"),
    ("armor.feet", "diamond_boots"),
];

/// `0.7 * getArmorCoverPercentage` over the same
/// live `PlayerSensor`: an invisible watcher in full armour is inside both
/// target lists twenty blocks away -- `max(32 * 0.7 * 1.0, 2.0)` = 22.4 -- where
/// the same watcher naked falls to the 0.1 floor and is dropped at 2.24.
#[test]
fn an_invisible_player_in_full_armour_is_seen_much_further_than_a_naked_one() {
    let naked = watch(Watcher::Invisible, 20.0);
    assert!(
        naked.nearest_players(),
        "the unfiltered scan still holds her"
    );
    assert!(
        !naked.visible() && !naked.attackable(),
        "20 > max(32 * 0.7 * 0.1, 2.0)"
    );

    let armoured = watch(Watcher::InvisibleWearing(FULL_ARMOUR), 20.0);
    assert!(
        armoured.visible() && armoured.attackable(),
        "20 <= 32 * 0.7 * 1.0"
    );
}
