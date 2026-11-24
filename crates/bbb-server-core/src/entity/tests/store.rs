use super::*;
use bbb_facts::scoreboard_team_options::TeamCollisionRule;

#[test]
fn an_entity_is_filed_in_the_section_its_block_position_names() {
    let sand = state_id("minecraft:sand");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 20.5,
            y: 70.0,
            z: -3.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(sand, pos(20, 70, -4))),
    );
    assert_eq!(
        section_of_block(pos(20, 70, -4)),
        crate::entity::SectionKey::new(1, 4, -1)
    );
    assert_eq!(
        store.ids_in_section(crate::entity::SectionKey::new(1, 4, -1)),
        vec![id]
    );
    assert_eq!(store.ids_in_chunk((1, -1)), vec![id]);
    assert_eq!(store.ids_in_chunk((0, -1)), Vec::<i32>::new());
    assert_eq!(store.occupied_chunks(), vec![(1, -1)]);
}

#[test]
fn moving_an_entity_across_a_chunk_boundary_refiles_it() {
    let sand = state_id("minecraft:sand");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 15.5,
            y: 70.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(sand, pos(15, 70, 0))),
    );
    assert_eq!(store.ids_in_chunk((0, 0)), vec![id]);

    store.get_mut(id).expect("entity is live").set_pos(Vec3d {
        x: 16.5,
        y: 70.0,
        z: 0.5,
    });
    store.refile(id);

    assert_eq!(store.ids_in_chunk((0, 0)), Vec::<i32>::new());
    assert_eq!(store.ids_in_chunk((1, 0)), vec![id]);
}

#[test]
fn initialized_restore_files_and_publishes_the_completed_entity() {
    let sand = state_id("minecraft:sand");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store
        .restore_initialized(
            Uuid::from_u128(1),
            VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
            Vec3d {
                x: 15.5,
                y: 70.0,
                z: 0.5,
            },
            EntityData::FallingBlock(FallingBlockData::new(sand, pos(15, 70, 0))),
            |entity| {
                entity.set_pos(Vec3d {
                    x: 16.5,
                    y: 81.0,
                    z: -0.5,
                });
                entity.set_rot(81.5, -22.25);
                entity.set_delta_movement(Vec3d {
                    x: 0.25,
                    y: -0.5,
                    z: 0.75,
                });
                entity.fall_distance = 12.5;
                entity.remaining_fire_ticks = 37;
                entity.air_supply = 111;
                entity.on_ground = true;
                entity.invulnerable = true;
                entity.portal_cooldown = 42;
                entity.no_gravity = true;
                entity.set_needs_sync(true);
            },
        )
        .expect("the UUID and type are valid");

    assert_eq!(
        store.ids_in_section(crate::entity::SectionKey::new(1, 5, -1)),
        vec![id]
    );
    assert_single_added_matches_store(&mut store, id);
    store.refile(id);
    assert!(
        store.drain_lifecycle_events().is_empty(),
        "the initializer's final section is already the filed section"
    );
    store
        .remove(id, RemovalReason::Discarded)
        .expect("the initialized entity is live");
    assert!(store
        .ids_in_section(crate::entity::SectionKey::new(1, 5, -1))
        .is_empty());
}

#[test]
fn lifecycle_events_record_removal_during_a_detached_tick() {
    let sand = state_id("minecraft:sand");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 5.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(sand, pos(0, 5, 0))),
    );
    store.drain_lifecycle_events();

    let mut entity = store.detach(id).expect("entity is live");
    entity.set_removed(RemovalReason::Discarded);
    store.attach(entity);

    let events = store.drain_lifecycle_events();
    assert_eq!(events.len(), 1);
    match &events[0] {
        EntityLifecycleEvent::Removed {
            entity,
            tracked: true,
        } => {
            assert_eq!(entity.id(), id);
            assert_eq!(entity.removal_reason(), Some(RemovalReason::Discarded));
        }
        other => panic!("expected remove, found {other:?}"),
    }
    assert!(store.is_empty());
}

#[test]
fn removing_an_entity_frees_its_section_slot_and_its_uuid() {
    let sand = state_id("minecraft:sand");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 5.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(sand, pos(0, 5, 0))),
    );
    let uuid = store.get(id).expect("entity is live").uuid();
    assert!(store.contains_uuid(uuid));

    let removed = store
        .remove(id, RemovalReason::UnloadedToChunk)
        .expect("the entity was live");
    assert_eq!(
        removed.removal_reason(),
        Some(RemovalReason::UnloadedToChunk)
    );
    assert!(removed.removal_reason().is_some_and(|r| r.should_save()));
    assert!(!store.contains_uuid(uuid));
    assert!(store.is_empty());
    assert_eq!(store.ids_in_chunk((0, 0)), Vec::<i32>::new());
    assert_eq!(store.occupied_chunks(), Vec::new());
}

#[test]
fn a_record_whose_uuid_is_already_live_is_refused() {
    let sand = state_id("minecraft:sand");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 5.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(sand, pos(0, 5, 0))),
    );
    let uuid = store.get(id).expect("entity is live").uuid();
    assert!(store
        .restore(
            uuid,
            VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
            Vec3d {
                x: 0.5,
                y: 6.0,
                z: 0.5,
            },
            EntityData::FallingBlock(FallingBlockData::new(sand, pos(0, 6, 0))),
        )
        .is_none());
}

// ---------------------------------------------------------------------------
// Collision geometry
// ---------------------------------------------------------------------------

#[test]
fn the_vertical_axis_resolves_before_the_horizontal_axis_with_the_smaller_component() {
    assert_eq!(
        axis_step_order(Vec3d {
            x: 0.5,
            y: 0.0,
            z: 1.5
        }),
        [Axis::Y, Axis::Z, Axis::X]
    );
    assert_eq!(
        axis_step_order(Vec3d {
            x: 1.5,
            y: 0.0,
            z: 0.5
        }),
        [Axis::Y, Axis::X, Axis::Z]
    );
    // Equal magnitudes take the `YXZ` branch, because the comparison is a
    // strict `<` (`Direction.java:380`).
    assert_eq!(
        axis_step_order(Vec3d {
            x: 1.0,
            y: 0.0,
            z: -1.0
        }),
        [Axis::Y, Axis::X, Axis::Z]
    );
}
#[test]
fn the_axis_resolved_first_is_the_one_that_gets_blocked_against_a_diagonal_corner() {
    let unit = Aabb {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    let blocker = Aabb {
        min_x: 1.0,
        min_y: 0.0,
        min_z: 1.0,
        max_x: 2.0,
        max_y: 1.0,
        max_z: 2.0,
    };

    // |x| < |z|, so Z runs first, clears the corner, and X is then blocked.
    assert_eq!(
        collide_with_boxes(
            Vec3d {
                x: 0.5,
                y: 0.0,
                z: 1.5
            },
            unit,
            &[blocker]
        ),
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.5
        }
    );
    // The mirror image: X runs first and Z is the axis that loses.
    assert_eq!(
        collide_with_boxes(
            Vec3d {
                x: 1.5,
                y: 0.0,
                z: 0.5
            },
            unit,
            &[blocker]
        ),
        Vec3d {
            x: 1.5,
            y: 0.0,
            z: 0.0
        }
    );
}

/// `Scoreboard.getPlayersTeam` (`Scoreboard.java:308-310`) as
/// `Entity.getTeam` (`Entity.java:2692-2694`) reads it: one team per name, and
/// two names share a team exactly when they answer the same index.
#[test]
fn the_scoreboard_answers_one_team_per_name() {
    assert_eq!(
        crate::entity::ScoreboardTeams::default().team_of("alice"),
        None
    );

    let mut raw_only = crate::entity::level::ScoreboardTeams::default();
    raw_only.add_team(
        &[bbb_protocol::JavaString::from(vec![0xd800])],
        TeamCollisionRule::Always,
    );
    assert!(!raw_only.is_empty());
    assert_eq!(raw_only.team_of("\u{fffd}"), None);

    let mut teams = crate::entity::level::ScoreboardTeams::default();
    teams.add_team(&["alice".into(), "bob".into()], TeamCollisionRule::Always);
    teams.add_team(&["cara".into()], TeamCollisionRule::Always);

    let alice = teams.team_of("alice");
    assert_eq!(alice, teams.team_of("bob"));
    assert!(alice.is_some());
    assert_ne!(alice, teams.team_of("cara"));
    assert_eq!(teams.team_of("dave"), None);

    // `Scoreboard.loadPlayerTeam` adds every member of a restored team in
    // order (`Scoreboard.java:391-393`), so a name two teams carry ends up on
    // the later one, exactly as `teamsByPlayer.put` leaves it (`:277`).
    let mut teams = crate::entity::level::ScoreboardTeams::default();
    teams.add_team(&["alice".into()], TeamCollisionRule::Always);
    teams.add_team(&["alice".into()], TeamCollisionRule::Always);
    assert_eq!(teams.team_of("alice"), Some(1));
}

/// `Entity.isAlliedTo` (`Entity.java:2696-2706`) over the two indices one
/// capture assigned, which is the rule both the freeze path and the brain's
/// `TargetingConditions` combat arm read.
#[test]
fn two_indices_from_one_capture_are_allied_only_when_they_are_the_same_team() {
    let mut teams = crate::entity::level::ScoreboardTeams::default();
    teams.add_team(
        &["alice".into(), "creaking".into()],
        TeamCollisionRule::Always,
    );
    teams.add_team(&["bob".into()], TeamCollisionRule::Always);

    let creaking = teams.team_of("creaking");
    assert!(crate::entity::level::ScoreboardTeams::allied(
        creaking,
        teams.team_of("alice")
    ));
    assert!(!crate::entity::level::ScoreboardTeams::allied(
        creaking,
        teams.team_of("bob")
    ));
    // `Team.isAlliedTo(null)` is false (`Team.java:17-19`), so a side on no
    // team is allied to nobody -- not even to another side on no team, which
    // `None == None` would have answered true.
    assert!(!crate::entity::level::ScoreboardTeams::allied(
        creaking, None
    ));
    assert!(!crate::entity::level::ScoreboardTeams::allied(None, None));
    assert!(!crate::entity::level::ScoreboardTeams::allied(
        None, creaking
    ));
}
