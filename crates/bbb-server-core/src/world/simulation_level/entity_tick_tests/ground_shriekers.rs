use super::*;
use crate::entity::level::{SculkPlayerIdentity, SculkShriekerStepSource};
use crate::entity::warden_spawn_tracker::WardenSpawnTracker;
use crate::world::simulation_level::LevelCtx;

const SHRIEKER: BlockPos = BlockPos { x: 8, y: 64, z: 8 };
const SECOND: BlockPos = BlockPos { x: 9, ..SHRIEKER };

fn state(summon: bool, shrieking: bool) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        "minecraft:sculk_shrieker",
        &[
            ("can_summon".into(), summon.to_string()),
            ("shrieking".into(), shrieking.to_string()),
            ("waterlogged".into(), "false".into()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

fn install(level: &mut LevelCtx<'_, '_>, pos: BlockPos, summon: bool) {
    assert!(LevelCtx::set_block(
        level,
        pos,
        state(summon, false),
        2,
        crate::neighbor_updater::UPDATE_LIMIT
    ));
}

fn source(warden: Option<bool>) -> SculkShriekerStepSource {
    SculkShriekerStepSource {
        player: SculkPlayerIdentity {
            uuid: Uuid::from_u128(777),
            entity_id: 41,
        },
        warden_nearby: warden,
    }
}

fn warning(level: &LevelCtx<'_, '_>, pos: BlockPos) -> i32 {
    level
        .chunks
        .block_entities()
        .get(pos)
        .unwrap()
        .sculk_shrieker()
        .unwrap()
        .warning_level()
}

fn seed_warning(level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    level
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .unwrap()
        .sculk_shrieker_mut()
        .unwrap()
        .set_warning_level_for_test(4);
}

/// A level whose world holds a session for each of `uuids`, so their trackers
/// live on those players.
fn with_sessions(uuids: &[u128], test: impl FnOnce(&mut LevelCtx<'_, '_>)) {
    let mut world = ServerWorld::new(Default::default()).unwrap();
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    for (index, &uuid) in uuids.iter().enumerate() {
        crate::tests::join_loaded(
            &sender,
            &receiver,
            &mut world,
            index as u64 + 1,
            &format!("p{uuid}"),
            uuid,
        );
    }
    with_world_frame(world, &[], true, test);
}

fn store<'s>(level: &'s mut LevelCtx<'_, '_>) -> &'s mut EntityStore {
    level.structure_entities.as_mut().unwrap().store
}

fn tracker(level: &mut LevelCtx<'_, '_>, uuid: Uuid) -> WardenSpawnTracker {
    store(level)
        .player_by_uuid(uuid)
        .unwrap()
        .cold
        .warden_tracker
}

fn set_tracker(level: &mut LevelCtx<'_, '_>, uuid: Uuid, tracker: WardenSpawnTracker) {
    store(level)
        .players_mut()
        .find(|session| session.profile.uuid == uuid)
        .unwrap()
        .cold
        .warden_tracker = tracker;
}

#[test]
fn ground_shriekers_non_summoning_contact_needs_no_warning_runtime_or_warden_read() {
    with_level(|level| {
        install(level, SHRIEKER, false);
        level.sculk_warning = None;
        seed_warning(level, SHRIEKER);
        assert_eq!(
            LevelCtx::step_on_sculk_shrieker(
                level,
                crate::entity::level::EntityBlockContext::new(None, None),
                SHRIEKER,
                source(None)
            ),
            Some(())
        );
        assert_eq!(level.block_state(SHRIEKER), state(false, true));
        assert_eq!(warning(level, SHRIEKER), 0);
        assert!(level.changed_block_broadcasts.contains(&(8, 64, 8)));
        assert_eq!(block_tick_triggers_at(level, SHRIEKER), [100]);
        let events: Vec<_> = level.event_sink.drain_level_broadcasts().collect();
        assert_eq!(events.len(), 1);
        assert!(
            matches!(events[0], crate::level_event::PendingLevelBroadcast::LevelEvent(event)
            if event.event_type == 3007 && event.pos == SHRIEKER && event.data == 0)
        );
        let events = level.event_sink.drain_game_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].game_event, VanillaGameEvent::SHRIEK);
        assert_eq!(events[0].source_entity_id, Some(41));
        assert_eq!(events[0].affected_state, None);
    });
}

#[test]
fn ground_shriekers_warning_copies_highest_tracker_to_live_nearby_players_and_distant_actor() {
    with_sessions(&[1, 2, 3, 4, 5, 6, 777], |level| {
        {
            for session in store(level).players_mut() {
                let uuid = session.profile.uuid.as_u128();
                session.set_position(Vec3d {
                    x: match uuid {
                        2 => 24.499,
                        3 => 24.5,
                        777 => 200.5,
                        _ => 8.5,
                    },
                    y: 64.5,
                    z: 8.5,
                });
                if uuid == 4 {
                    session.game_type = bbb_protocol::game_types::GameType::Spectator;
                }
                if uuid == 5 || uuid == 6 {
                    session.damage.health = 0.0;
                }
            }
        }
        install(level, SHRIEKER, true);
        level.sculk_warning.as_mut().unwrap().difficulty = Difficulty::Normal;
        set_tracker(
            level,
            Uuid::from_u128(1),
            WardenSpawnTracker::new(123, 2, 0),
        );
        set_tracker(
            level,
            Uuid::from_u128(3),
            WardenSpawnTracker::new(321, 4, 0),
        );
        let excluded = [4, 5, 6].map(|uuid| tracker(level, Uuid::from_u128(uuid)));
        assert_eq!(
            LevelCtx::step_on_sculk_shrieker(
                level,
                crate::entity::level::EntityBlockContext::new(None, None),
                SHRIEKER,
                source(Some(false))
            ),
            Some(())
        );
        assert_eq!(warning(level, SHRIEKER), 3);
        for uuid in [1, 2, 777] {
            assert_eq!(
                tracker(level, Uuid::from_u128(uuid)),
                WardenSpawnTracker::new(0, 3, 200)
            );
        }
        assert_eq!(
            tracker(level, Uuid::from_u128(3)),
            WardenSpawnTracker::new(321, 4, 0)
        );
        assert_eq!(
            [4, 5, 6].map(|uuid| tracker(level, Uuid::from_u128(uuid))),
            excluded
        );
        assert_eq!(
            level.event_sink.drain_game_events()[0].source_entity_id,
            Some(41)
        );
    });
}

#[test]
fn ground_shriekers_warden_and_cooldown_are_resolved_rejections_that_reset_warning() {
    for warden in [false, true] {
        with_sessions(&[777], |level| {
            install(level, SHRIEKER, true);
            seed_warning(level, SHRIEKER);
            level.sculk_warning.as_mut().unwrap().difficulty = Difficulty::Normal;
            let before = WardenSpawnTracker::new(4, 2, 3);
            set_tracker(level, source(None).player.uuid, before);
            assert_eq!(
                LevelCtx::step_on_sculk_shrieker(
                    level,
                    crate::entity::level::EntityBlockContext::new(None, None),
                    SHRIEKER,
                    source(Some(warden))
                ),
                Some(())
            );
            assert_eq!(warning(level, SHRIEKER), 0);
            assert_eq!(level.block_state(SHRIEKER), state(true, false));
            assert_eq!(tracker(level, source(None).player.uuid), before);
            assert!(block_tick_triggers_at(level, SHRIEKER).is_empty());
            assert!(level.event_sink.drain_game_events().is_empty());
        });
    }
}

#[test]
fn ground_shriekers_peaceful_and_disabled_wardens_do_not_require_a_warden_read() {
    for peaceful in [false, true] {
        with_level(|level| {
            install(level, SHRIEKER, true);
            if peaceful {
                level.sculk_warning.as_mut().unwrap().difficulty = Difficulty::Peaceful;
            } else {
                level.game_rules.spawn_wardens = false;
                level.sculk_warning = None;
            }
            assert_eq!(
                LevelCtx::step_on_sculk_shrieker(
                    level,
                    crate::entity::level::EntityBlockContext::new(None, None),
                    SHRIEKER,
                    source(None)
                ),
                Some(())
            );
            assert_eq!(level.block_state(SHRIEKER), state(true, true));
        });
    }
}

#[test]
fn ground_shriekers_fresh_tracker_write_is_immediate_and_second_same_tick_contact_cools_down() {
    with_sessions(&[777], |level| {
        install(level, SHRIEKER, true);
        install(level, SECOND, true);
        seed_warning(level, SECOND);
        level.sculk_warning.as_mut().unwrap().difficulty = Difficulty::Normal;
        set_tracker(
            level,
            source(None).player.uuid,
            WardenSpawnTracker::default(),
        );
        assert_eq!(
            LevelCtx::step_on_sculk_shrieker(
                level,
                crate::entity::level::EntityBlockContext::new(None, None),
                SHRIEKER,
                source(Some(false))
            ),
            Some(())
        );
        assert_eq!(
            tracker(level, source(None).player.uuid),
            WardenSpawnTracker::new(0, 1, 200)
        );
        assert_eq!(
            LevelCtx::step_on_sculk_shrieker(
                level,
                crate::entity::level::EntityBlockContext::new(None, None),
                SECOND,
                source(Some(false))
            ),
            Some(())
        );
        assert_eq!(warning(level, SECOND), 0);
        assert_eq!(level.block_state(SECOND), state(true, false));
        assert_eq!(block_tick_triggers_at(level, SHRIEKER).len(), 1);
        assert!(block_tick_triggers_at(level, SECOND).is_empty());
    });
}
