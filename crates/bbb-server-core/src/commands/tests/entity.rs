//! Entity command feedback and native player rules.
use super::*;

use crate::connection::ClientboundPlayPacket;

fn player_state<R>(
    world: &CommandTestWorld,
    name: &str,
    read: impl FnOnce(crate::commands::CommandPlayer<'_>) -> R,
) -> R {
    read(
        world
            .world()
            .command_players()
            .find(|player| player.name == name)
            .expect("fixture player"),
    )
}

fn swings(run: &CommandRun) -> Vec<u8> {
    run.packets()
        .into_iter()
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::EntityAnimation(animation) => Some(animation.action),
            _ => None,
        })
        .collect()
}

/// The health each named player is left with after `command`.
fn health_after(command: &str, mut world: CommandTestWorld, names: &[&str]) -> Vec<f32> {
    dispatched(command, &mut world);
    names
        .iter()
        .map(|name| player_state(&world, name, |player| player.damage.health))
        .collect()
}

#[test]
fn a_landed_hit_reports_the_amount_and_the_target() {
    let players = [user("alice", 1)];
    let run = ran("damage alice 6", world(&players));
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.damage.success"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Float(6.0f32.to_bits()),
            display_name("alice", 1)
        ]
    );
}

#[test]
fn a_refused_hit_reports_invulnerable() {
    let players = [user("alice", 1)];
    let run = ran(
        "damage alice 6",
        world(&players).with_player_game_type("alice", GameType::Creative),
    );
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.damage.invulnerable"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
}

#[test]
fn a_damage_type_outside_the_registry_is_refused_by_the_argument() {
    let players = [user("alice", 1)];
    let run = ran(
        "damage alice 6 minecraft:not_a_damage_type",
        world(&players),
    );
    assert_eq!(
        failure_messages(&run)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["argument.resource.not_found"]
    );
}

#[test]
fn a_target_that_matches_nobody_reports_the_entity_key() {
    let run = ran("damage bob 6", world(&[]));
    assert_eq!(keys(&run), ["argument.entity.notfound.entity"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
}

#[test]
fn the_nearest_entity_selector_reaches_the_nearest_player() {
    let players = [user("alice", 1), user("bob", 2)];
    assert_eq!(
        health_after(
            "damage @n 3",
            world(&players)
                .with_player_position("alice", [4.0, 0.0, 0.0])
                .with_player_position("bob", [2.0, 0.0, 0.0]),
            &["alice", "bob"],
        ),
        [20.0, 17.0]
    );

    // With nobody online there is no nearest entity, which is
    // `findSingleEntity`'s empty arm (EntitySelector.java:128-129).
    let empty = ran("damage @n 3", world(&[]));
    assert_eq!(keys(&empty), ["argument.entity.notfound.entity"]);
}

#[test]
fn a_uuid_target_names_the_player_holding_that_id() {
    let players = [user("alice", 1), user("bob", 2)];
    for target in ["00000000-0000-0000-0000-000000000002", "0-0-0-0-2"] {
        assert_eq!(
            health_after(
                &format!("damage {target} 3"),
                world(&players),
                &["alice", "bob"]
            ),
            [20.0, 17.0],
            "{target}"
        );
    }

    // A UUID nobody holds is not read as a name either, so it matches nothing
    // rather than falling back to `getPlayerByName`.
    let missing = ran("damage 0-0-0-0-9 3", world(&players));
    assert_eq!(keys(&missing), ["argument.entity.notfound.entity"]);
}

#[test]
fn a_many_selector_is_refused_by_the_single_entity_argument() {
    let players = [user("alice", 1)];
    for command in ["damage @a 1", "damage @e 1"] {
        let run = ran(command, world(&players));
        assert_eq!(
            keys(&run),
            ["argument.entity.toomany", CURSOR_CONTEXT],
            "{command}"
        );
        assert_eq!(
            underlined_remainder(body(&run.feedback[1])).as_deref(),
            Some(command),
            "{command} quotes the wrong remainder"
        );
    }
}

#[test]
fn damage_needs_the_gamemaster_level() {
    let run = refused("damage alice 6", CommandPermission::Moderator);
    assert_eq!(
        failure_messages(&run)
            .into_iter()
            .filter_map(translate_key)
            .collect::<Vec<_>>(),
        ["command.unknown.command"]
    );
}

#[test]
fn the_damage_type_argument_suggests_the_registry_it_names() {
    let completions = completions_for(
        "damage alice 1 out_of",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["minecraft:out_of_world"]);
    // With a colon written the whole id is matched instead.
    let completions = completions_for(
        "damage alice 1 minecraft:generic_k",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["minecraft:generic_kill"]);
}

#[test]
fn the_location_argument_suggests_one_relative_triple() {
    let completions = completions_for(
        "damage alice 1 minecraft:generic at ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["~", "~ ~", "~ ~ ~"]);
    let completions = completions_for(
        "damage alice 1 minecraft:generic at ^",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    // `SuggestionsBuilder.suggest` drops the candidate equal to the remainder,
    // so the bare `^` the client already typed is not offered back.
    assert_eq!(texts(&completions), ["^ ^", "^ ^ ^"]);
    let completions = completions_for(
        "damage alice 1 minecraft:generic at 5",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["5 ~", "5 ~ ~"]);
}

#[test]
fn kill_hurts_every_victim_with_the_generic_kill_source_and_names_the_only_one() {
    let players = [user("alice", 1)];
    let mut killed = world(&players);
    let run = ran("kill alice", &mut killed);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.kill.success.single"]);
    assert_eq!(arguments(&run, 0), vec![display_name("alice", 1)]);
    assert!(player_state(&killed, "alice", |player| player.damage.health) <= 0.0);
}

#[test]
fn kill_over_many_reports_the_count_rather_than_the_names() {
    let players = [user("alice", 1), user("bob", 2)];
    let mut killed = world(&players);
    let run = ran("kill @e", &mut killed);
    assert_eq!(run.result, Some(2));
    assert_eq!(keys(&run), ["commands.kill.success.multiple"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(2)]);
    for name in ["alice", "bob"] {
        assert!(player_state(&killed, name, |player| player.damage.health) <= 0.0);
    }
}

#[test]
fn kill_hurts_a_creative_player_with_the_generic_kill_source() {
    let players = [user("alice", 1)];
    let mut killed = world(&players).with_player_game_type("alice", GameType::Creative);
    let run = ran("kill alice", &mut killed);
    assert_eq!(keys(&run), ["commands.kill.success.single"]);
    assert!(player_state(&killed, "alice", |player| player.damage.health) <= 0.0);
}

#[test]
fn kill_without_an_argument_kills_the_source_and_refuses_a_sourceless_one() {
    let players = [user("alice", 1)];
    let alice = source("alice", CommandPermission::Owner);
    let run = ran_as("kill", &alice, world(&players));
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.kill.success.single"]);

    let console = CommandSource::server("console", CommandPermission::Owner);
    let refused = ran_as("kill", &console, world(&players));
    assert_eq!(refused.result, None);
    assert_eq!(keys(&refused), ["permissions.requires.entity"]);
}

#[test]
fn swing_names_the_first_target_and_counts_the_living_ones() {
    let players = [user("alice", 1)];
    let run = ran("swing alice", world(&players));
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.swing.success.single"]);
    assert_eq!(arguments(&run, 0), vec![display_name("alice", 1)]);
    assert_eq!(swings(&run), [0]);

    let players = [user("alice", 1), user("bob", 2)];
    let run = ran("swing @e", world(&players));
    assert_eq!(run.result, Some(2));
    assert_eq!(keys(&run), ["commands.swing.success.multiple"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(2)]);

    let zombie = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    )
    .unwrap();
    let stored = [
        stored_entity(
            44,
            19,
            [1.0, 0.0, 0.0],
            crate::entity::EntityData::Minecart(crate::entity::MinecartData::new()),
        ),
        stored_entity(
            45,
            20,
            [2.0, 0.0, 0.0],
            crate::entity::EntityData::GeneralLiving(zombie),
        ),
    ];
    let mut fixture = world(&players[..1]).with_entities(&stored);
    let zombie_id = fixture
        .stored_id(
            &level_key(Java26BuiltinDimension::Overworld),
            Uuid::from_u128(45),
        )
        .unwrap();
    let run = ran(
        "swing @e[type=!minecraft:player,sort=nearest]",
        &mut fixture,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.swing.success.single"]);
    assert_eq!(
        arguments(&run, 0),
        [entity_display_name(
            translatable("entity.minecraft.minecart", Vec::new()),
            "minecraft:minecart",
            44
        )]
    );
    assert!(run.packets().iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::EntityAnimation(animation)
            if animation.id == zombie_id && animation.action == 0
    )));
    let refused = ran("swing @e[type=minecraft:minecart]", &mut fixture);
    assert_eq!(refused.result, None);
    assert_eq!(keys(&refused), ["commands.swing.failed.notliving"]);
    assert!(swings(&refused).is_empty());
}

#[test]
fn the_swing_hand_literals_pick_the_animation_the_arm_swings_with() {
    let players = [user("alice", 1)];
    // `ClientboundAnimatePacket`: `SWING_MAIN_HAND` is 0 and `SWING_OFF_HAND` 3.
    for (command, action) in [
        ("swing alice", 0),
        ("swing alice mainhand", 0),
        ("swing alice offhand", 3),
    ] {
        assert_eq!(
            swings(&ran(command, world(&players))),
            [action],
            "{command}"
        );
    }
    let zombie = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    )
    .unwrap();
    let stored = [stored_entity(
        45,
        20,
        [2.0, 0.0, 0.0],
        crate::entity::EntityData::GeneralLiving(zombie),
    )];
    for (command, action) in [
        ("swing @e[type=minecraft:zombie] mainhand", 0),
        ("swing 0-0-0-0-2d offhand", 3),
        ("execute as @e[type=minecraft:zombie] run swing", 0),
    ] {
        let mut fixture = world(&players).with_entities(&stored);
        let run = ran(command, &mut fixture);
        assert_eq!(run.result, Some(1), "{command}: {run:?}");
        assert_eq!(swings(&run), [action], "{command}");
        let selected = fixture
            .world()
            .entity_reads()
            .stored_entities()
            .find(|entity| entity.entity.uuid() == Uuid::from_u128(45))
            .unwrap();
        assert!(selected.entity.living().unwrap().is_swinging());
    }
}

#[test]
fn rotate_hands_over_the_difference_from_the_entity_on_a_relative_axis() {
    let players = [user("alice", 1)];
    let mut rotated = world(&players).with_player_rotation("alice", [30.0, 10.0]);
    let run = ran("rotate alice 90 -20", &mut rotated);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.rotate.success"]);
    assert_eq!(arguments(&run, 0), vec![display_name("alice", 1)]);
    assert_eq!(
        player_state(&rotated, "alice", |player| player.rotation),
        [90.0, -20.0]
    );

    // The source stands at yaw 0 / pitch 0 (the default source), so `~5 ~`
    // resolves against the source's facing rather than the entity's.
    let mut rotated = world(&players).with_player_rotation("alice", [30.0, 10.0]);
    ran("rotate alice ~5 ~", &mut rotated);
    assert_eq!(
        player_state(&rotated, "alice", |player| player.rotation),
        [5.0, 0.0]
    );
}

#[test]
fn rotate_facing_produces_a_look_and_never_a_teleport() {
    let players = [user("alice", 1), user("bob", 2)];
    for (command, position) in [
        // `Vec3Argument.vec3()` is the centre-correcting form
        // (Vec3Argument.java:30-32), and `WorldCoordinates.parseDouble`
        // (WorldCoordinates.java:59-77) passes that flag on the horizontal axes
        // and `false` on the vertical one (`:61-67`), so a bare `1 2 3` is the
        // block centre in x and z and the block face in y.
        ("rotate alice facing 1 2 3", Some([1.5, 2.0, 3.5])),
        ("rotate alice facing entity bob", None),
        ("rotate alice facing entity bob eyes", None),
    ] {
        let run = ran(command, world(&players));
        assert_eq!(keys(&run), ["commands.rotate.success"], "{command}");
        let [ClientboundPlayPacket::PlayerLookAt(look)] = run.packets()[..] else {
            panic!(
                "{command}: expected one look packet, got {:?}",
                run.packets()
            );
        };
        match position {
            Some([x, y, z]) => assert_eq!(
                (look.position.x, look.position.y, look.position.z),
                (x, y, z),
                "{command}"
            ),
            None => assert!(look.target.is_some(), "{command}"),
        }
    }
}

#[test]
fn spectate_refuses_itself_and_a_player_that_is_not_a_spectator() {
    let spectator = user("alice", 1);
    let players = [spectator.clone(), user("bob", 2)];
    let alice = source("alice", CommandPermission::Owner);

    // `:41-43` -- checked before the spectator test, so a survival player
    // spectating itself still gets the self refusal.
    let run = ran_as(
        "spectate alice",
        &alice,
        world(&players).with_player_game_type("alice", GameType::Spectator),
    );
    assert_eq!(keys(&run), ["commands.spectate.self"]);

    // `:45-47`, over the player doing the spectating rather than the target.
    let bob = source("bob", CommandPermission::Owner);
    let bob = CommandSource {
        uuid: Uuid::from_u128(2),
        ..bob
    };
    let run = ran_as(
        "spectate alice",
        &bob,
        world(&players).with_player_game_type("alice", GameType::Spectator),
    );
    assert_eq!(keys(&run), ["commands.spectate.not_spectator"]);
    assert_eq!(
        failure_messages(&run)[0],
        &compound(vec![
            ("translate", text("commands.spectate.not_spectator")),
            ("with", NbtSummaryValue::List(vec![display_name("bob", 2)])),
        ])
    );
}

#[test]
fn spectate_starts_and_stops_through_one_camera_effect() {
    let spectator = user("alice", 1);
    let players = [spectator, user("bob", 2)];
    let alice = source("alice", CommandPermission::Owner);

    let mut spectating = world(&players).with_player_game_type("alice", GameType::Spectator);
    let run = ran_as("spectate bob", &alice, &mut spectating);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.spectate.success.started"]);
    assert_eq!(arguments(&run, 0), vec![display_name("bob", 2)]);
    assert_eq!(
        player_state(&spectating, "alice", |player| player.camera.cloned()),
        Some(crate::commands::CommandCameraTarget::Player {
            uuid: Uuid::from_u128(2),
        })
    );

    let run = ran_as("spectate", &alice, &mut spectating);
    assert_eq!(keys(&run), ["commands.spectate.success.stopped"]);
    assert_eq!(arguments(&run, 0), Vec::new());
    assert_eq!(
        player_state(&spectating, "alice", |player| player.camera.cloned()),
        None
    );
}

#[test]
fn spectate_with_a_player_argument_moves_that_players_camera() {
    let spectator = user("bob", 2);
    let players = [user("alice", 1), spectator];
    let console = CommandSource::server("console", CommandPermission::Owner);
    let mut spectating = world(&players).with_player_game_type("bob", GameType::Spectator);
    ran_as("spectate alice bob", &console, &mut spectating);
    assert_eq!(
        player_state(&spectating, "bob", |player| player.camera.cloned()),
        Some(crate::commands::CommandCameraTarget::Player {
            uuid: Uuid::from_u128(1),
        })
    );
}

#[test]
fn ride_refuses_an_unmounted_rider_and_a_player_vehicle() {
    let players = [user("alice", 1), user("bob", 2)];
    let run = ran("ride alice dismount", world(&players));
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.ride.not_riding"]);
    assert_eq!(
        failure_messages(&run)[0],
        &compound(vec![
            ("translate", text("commands.ride.not_riding")),
            (
                "with",
                NbtSummaryValue::List(vec![display_name("alice", 1)])
            ),
        ])
    );

    let run = ran("ride alice mount bob", world(&players));
    assert_eq!(
        keys(&run),
        ["commands.ride.mount.failure.cant_ride_players"]
    );
}

#[test]
fn ride_forces_stored_and_player_passengers_and_dismounts_them() {
    let players = [user("alice", 1)];
    let entities = [
        stored_entity(
            45,
            20,
            [4.0, 64.0, 0.0],
            crate::entity::EntityData::Minecart(crate::entity::MinecartData::new()),
        ),
        stored_entity(
            46,
            21,
            [0.0, 64.0, 0.0],
            crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
        ),
        stored_entity(
            47,
            22,
            [8.0, 64.0, 0.0],
            crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
        ),
    ];
    let mut fixture = world(&players)
        .with_entities(&entities)
        .with_player_boarding_restrictions("alice");
    let mounted = ran("ride 0-0-0-0-2e mount 0-0-0-0-2d", &mut fixture);
    assert_eq!(mounted.result, Some(1));
    assert_eq!(keys(&mounted), ["commands.ride.mount.success"]);
    let position = |fixture: &CommandTestWorld| {
        fixture
            .world()
            .entity_reads()
            .stored_entities()
            .find(|e| e.entity.uuid() == Uuid::from_u128(46))
            .unwrap()
            .entity
            .position()
    };
    assert_eq!(
        ran(
            "execute as 0-0-0-0-2e on vehicle run tag @s add mounted",
            &mut fixture
        )
        .result,
        Some(1)
    );
    assert_eq!(
        keys(&ran("ride 0-0-0-0-2d mount 0-0-0-0-2e", &mut fixture)),
        ["commands.ride.mount.failure.loop"]
    );
    let mounted = ran("ride alice mount 0-0-0-0-2d", &mut fixture);
    assert_eq!(mounted.result, Some(1));
    assert_eq!(player_state(&fixture, "alice", |p| p.position[0]), 4.0);
    assert!(mounted
        .packets()
        .iter()
        .any(|p| matches!(p, ClientboundPlayPacket::PlayerPosition(_))));
    let stand_id = fixture
        .stored_id(
            &level_key(Java26BuiltinDimension::Overworld),
            Uuid::from_u128(46),
        )
        .unwrap();
    let player_id = player_state(&fixture, "alice", |p| p.entity_id);
    assert!(mounted.packets().iter().any(|p| matches!(p, ClientboundPlayPacket::SetPassengers(packet) if packet.passenger_ids == [player_id, stand_id])));
    let exit = ran("ride alice dismount", &mut fixture);
    assert_eq!(exit.result, Some(1));
    assert_eq!(keys(&exit), ["commands.ride.dismount.success"]);
    assert!(exit
        .packets()
        .iter()
        .any(|p| matches!(p, ClientboundPlayPacket::PlayerPosition(_))));
    let exit = ran("ride 0-0-0-0-2e dismount", &mut fixture);
    assert_eq!(exit.result, Some(1));
    // The command bypasses the boarding cooldown set by the preceding detach.
    assert_eq!(
        ran("ride 0-0-0-0-2e mount 0-0-0-0-2d", &mut fixture).result,
        Some(1)
    );
    assert_eq!(
        ran("ride alice mount 0-0-0-0-2d", &mut fixture).result,
        Some(1)
    );
    assert_eq!(
        keys(&ran("ride alice mount 0-0-0-0-2e", &mut fixture)),
        ["commands.ride.already_riding"]
    );
    ran("ride 0-0-0-0-2e dismount", &mut fixture);
    ran("fill 5 63 -3 11 63 3 stone", &mut fixture);
    ran("fill 5 64 -3 11 68 3 air", &mut fixture);
    assert_eq!(
        ran(
            "ride 0-0-0-0-2e mount @e[type=oak_boat,limit=1]",
            &mut fixture
        )
        .result,
        Some(1)
    );
    assert_eq!(
        ran("ride 0-0-0-0-2e dismount", &mut fixture).result,
        Some(1)
    );
    let landing = position(&fixture);
    assert_eq!(landing.x, 8.0);
    assert_eq!(landing.y, 64.0);
    assert!(
        landing.z.abs() > 1.0,
        "living passengers use the safe boat exit"
    );
}

#[test]
fn tag_add_counts_the_changed_targets_and_words_on_the_target_count() {
    let tagged = user("alice", 1);
    let players = [tagged, user("bob", 2)];
    let mut tagged = world(&players).with_player_tags("alice", vec!["marked".to_string()]);
    let run = ran("tag @e add marked", &mut tagged);
    // Only bob changed, so the count is one...
    assert_eq!(run.result, Some(1));
    // ...while the branch is the multiple one, because two targets were named.
    assert_eq!(keys(&run), ["commands.tag.add.success.multiple"]);
    assert_eq!(
        arguments(&run, 0),
        vec![text("marked"), NbtSummaryValue::Int(2)]
    );
    assert_eq!(
        player_state(&tagged, "bob", |player| player.tags.to_vec()),
        ["marked"]
    );
}

#[test]
fn tag_add_and_remove_refuse_a_run_that_changed_nothing() {
    let tagged = user("alice", 1);
    let players = [tagged];
    let run = ran(
        "tag alice add marked",
        world(&players).with_player_tags("alice", vec!["marked".to_string()]),
    );
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.tag.add.failed"]);

    let run = ran(
        "tag alice remove absent",
        world(&players).with_player_tags("alice", vec!["marked".to_string()]),
    );
    assert_eq!(keys(&run), ["commands.tag.remove.failed"]);

    let mut tagged = world(&players).with_player_tags("alice", vec!["marked".to_string()]);
    let run = ran("tag alice remove marked", &mut tagged);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.tag.remove.success.single"]);
    assert_eq!(
        arguments(&run, 0),
        vec![text("marked"), display_name("alice", 1)]
    );
    assert!(player_state(&tagged, "alice", |player| player
        .tags
        .is_empty()));
}

#[test]
fn tag_list_reports_two_different_counts_and_returns_the_tag_one() {
    let alice = user("alice", 1);
    let bob = user("bob", 2);
    let players = [alice, bob];

    let run = ran(
        "tag alice list",
        world(&players)
            .with_player_tags("alice", vec!["one".to_string(), "two".to_string()])
            .with_player_tags("bob", vec!["two".to_string(), "three".to_string()]),
    );
    assert_eq!(run.result, Some(2));
    assert_eq!(keys(&run), ["commands.tag.list.single.success"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Success]);
    let single = arguments(&run, 0);
    assert_eq!(single[0], display_name("alice", 1));
    assert_eq!(single[1], NbtSummaryValue::Int(2));

    let run = ran(
        "tag @e list",
        world(&players)
            .with_player_tags("alice", vec!["one".to_string(), "two".to_string()])
            .with_player_tags("bob", vec!["two".to_string(), "three".to_string()]),
    );
    // Three distinct tags across two entities: `targets.size()` is 2 and
    // `tags.size()` is 3, and the return value is the tag count.
    assert_eq!(run.result, Some(3));
    assert_eq!(keys(&run), ["commands.tag.list.multiple.success"]);
    let multiple = arguments(&run, 0);
    assert_eq!(multiple[0], NbtSummaryValue::Int(2));
    assert_eq!(multiple[1], NbtSummaryValue::Int(3));
}

#[test]
fn tag_list_over_untagged_entities_takes_the_empty_arms() {
    let players = [user("alice", 1), user("bob", 2)];
    let run = ran("tag alice list", world(&players));
    assert_eq!(run.result, Some(0));
    assert_eq!(keys(&run), ["commands.tag.list.single.empty"]);
    assert_eq!(arguments(&run, 0), vec![display_name("alice", 1)]);

    let run = ran("tag @e list", world(&players));
    assert_eq!(run.result, Some(0));
    assert_eq!(keys(&run), ["commands.tag.list.multiple.empty"]);
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(2)]);
}

#[test]
fn the_tag_remove_suggestion_offers_the_tags_the_targets_hold() {
    let alice = user("alice", 1);
    let players = [alice, user("bob", 2)];
    let alice_source = source("alice", CommandPermission::Owner);

    let offered = suggested_in(
        "tag @s remove ",
        &alice_source,
        world(&players).with_player_tags("alice", vec!["alpha".to_string(), "beta".to_string()]),
    );
    assert_eq!(texts(&offered), ["alpha", "beta"]);

    // `matchesSubStr` filters what is left to read
    // (SharedSuggestionProvider.java:271-283).
    let offered = suggested_in(
        "tag @s remove al",
        &alice_source,
        world(&players).with_player_tags("alice", vec!["alpha".to_string(), "beta".to_string()]),
    );
    assert_eq!(texts(&offered), ["alpha"]);

    // A target with no tags offers nothing, and neither does the `add` node,
    // which carries no `suggests` at all.
    let bob_source = CommandSource {
        uuid: Uuid::from_u128(2),
        ..source("bob", CommandPermission::Owner)
    };
    assert!(texts(&suggested_in(
        "tag @s remove ",
        &bob_source,
        world(&players).with_player_tags("alice", vec!["alpha".to_string(), "beta".to_string()])
    ))
    .is_empty());
    assert!(texts(&suggested_in(
        "tag @s add ",
        &alice_source,
        world(&players).with_player_tags("alice", vec!["alpha".to_string(), "beta".to_string()])
    ))
    .is_empty());
}

#[test]
fn stored_entity_sources_resolve_through_the_source_selector() {
    let stored = [stored_entity(
        44,
        19,
        [4.0, 65.0, -2.0],
        crate::entity::EntityData::Minecart(crate::entity::MinecartData::new()),
    )];
    let mut fixture = world(&[]).with_entities(&stored);
    let level = level_key(Java26BuiltinDimension::Overworld);
    let id = fixture.stored_id(&level, Uuid::from_u128(44)).unwrap();
    let source = CommandSource::stored_entity(
        "Command Cart",
        Uuid::from_u128(44),
        id,
        level,
        RichTextComponent::plain("Command Cart"),
        CommandPermission::GameMaster,
    );
    let selected = fixture.run("damage @s 1", &source);
    assert_eq!(keys(&selected), ["commands.damage.success"]);
    let run = ran_as("kill @s", &source, &mut fixture);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.kill.success.single"]);
    assert_eq!(
        arguments(&run, 0),
        [entity_display_name(
            translatable("entity.minecraft.minecart", Vec::new()),
            "minecraft:minecart",
            44
        )]
    );
    assert!(fixture
        .stored_id(
            &level_key(Java26BuiltinDimension::Overworld),
            Uuid::from_u128(44)
        )
        .is_none());
}

#[test]
fn tag_writes_reads_and_lists_a_native_stored_entity() {
    let entities = [stored_entity(
        9,
        99,
        [1.0, 64.0, 0.0],
        crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap()),
    )];
    let mut fixture = world(&[]).with_entities(&entities);
    let added = ran("tag @e add anyone", &mut fixture);
    assert_eq!(added.result, Some(1));
    assert_eq!(keys(&added), ["commands.tag.add.success.single"]);
    let repeated = ran("tag @e add anyone", &mut fixture);
    assert_eq!(keys(&repeated), ["commands.tag.add.failed"]);
    let listed = ran("tag @e list", &mut fixture);
    assert_eq!(listed.result, Some(1));
    assert_eq!(keys(&listed), ["commands.tag.list.single.success"]);
    assert_eq!(
        arguments(&listed, 0)[0],
        entity_display_name(
            translatable("entity.minecraft.armor_stand", Vec::new()),
            "minecraft:armor_stand",
            9
        )
    );
    assert_eq!(
        texts(&completions_for(
            "tag @e remove ",
            CommandPermission::Owner,
            &fixture
        )),
        ["anyone"]
    );
    let removed = ran("tag @e remove anyone", &mut fixture);
    assert_eq!(keys(&removed), ["commands.tag.remove.success.single"]);
    assert_eq!(ran("tag @e list", &mut fixture).result, Some(0));
}

fn suggested_in(
    command: &str,
    source: &CommandSource,
    fixture: CommandTestWorld,
) -> CommandCompletions {
    completion_suggestions_in_world(command, source, fixture.world(), fixture.origin())
}

#[test]
fn rotate_and_teleport_turn_stored_entities_from_the_requested_anchors() {
    let stand =
        || crate::entity::EntityData::ArmorStand(crate::entity::ArmorStandData::new().unwrap());
    let entities = [
        stored_entity(45, 20, [0.0, 64.0, 0.0], stand()),
        stored_entity(46, 21, [10.0, 64.0, 0.0], stand()),
    ];
    let mut fixture = world(&[]).with_entities(&entities);
    let rotation = |fixture: &CommandTestWorld| {
        let reads = fixture.world().entity_reads();
        let actor = reads
            .stored_entities()
            .find(|entity| entity.entity.uuid() == Uuid::from_u128(45))
            .unwrap();
        assert_eq!(
            actor.entity.position(),
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0
            }
        );
        assert_eq!(actor.entity.y_rot(), actor.entity.y_head_rot());
        [actor.entity.y_rot(), actor.entity.x_rot()]
    };
    assert_eq!(
        ran("rotate 0-0-0-0-2d 450 -20", &mut fixture).result,
        Some(1)
    );
    assert_eq!(rotation(&fixture), [90.0, -20.0]);
    ran(
        "execute rotated 45 10 run rotate 0-0-0-0-2d ~5 ~",
        &mut fixture,
    );
    assert_eq!(rotation(&fixture), [50.0, 10.0]);
    ran("rotate 0-0-0-0-2d facing entity 0-0-0-0-2e", &mut fixture);
    assert_eq!(rotation(&fixture), [-90.0, 0.0]);
    ran(
        "rotate 0-0-0-0-2d facing entity 0-0-0-0-2e eyes",
        &mut fixture,
    );
    assert!(rotation(&fixture)[1] < -9.0);
    ran(
        "execute anchored eyes run rotate 0-0-0-0-2d facing entity 0-0-0-0-2e",
        &mut fixture,
    );
    assert!(rotation(&fixture)[1] > 9.0);
    ran(
        "execute anchored eyes run teleport 0-0-0-0-2d 0.0 64.0 0.0 facing entity 0-0-0-0-2e eyes",
        &mut fixture,
    );
    assert_eq!(rotation(&fixture), [-90.0, 0.0]);
}
