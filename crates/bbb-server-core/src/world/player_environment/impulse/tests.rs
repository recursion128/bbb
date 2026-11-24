use super::super::tests::{block, world};
use super::*;
use crate::entity::living::FallImpulseContext;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    ExplosionSourceEntityKind, PendingExplosion,
};
use bbb_protocol::packets::{MovePlayer, Vec3d};
use bbb_server_types::save::Java26BuiltinDimension;

const PLAYER: ConnectionId = ConnectionId(1);

#[test]
fn player_impulse_creative_transition_resets_but_spectator_preserves() {
    let mut world = world(GameType::Survival);
    let position = world.levels[PLAYER].position();
    let context = FallImpulseContext {
        reset_grace_time: 40,
        impact_position: Some(position),
    };
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.cold.fall_impulse = context;
    player.cold.impulse_explosion_cause = Some((source("minecraft:wind_charge", position), None));
    assert!(world.set_player_game_mode(PLAYER, GameType::Spectator, &mut Vec::new()));
    assert_eq!(world.levels[PLAYER].cold.fall_impulse, context);
    assert!(world.levels[PLAYER].cold.impulse_explosion_cause.is_some());
    assert!(world.set_player_game_mode(PLAYER, GameType::Creative, &mut Vec::new()));
    assert_eq!(
        world.levels[PLAYER].cold.fall_impulse,
        FallImpulseContext::default()
    );
    assert!(world.levels[PLAYER].cold.impulse_explosion_cause.is_none());
}

#[test]
fn player_impulse_random_teleport_resets_only_after_success() {
    for collides in [false, true] {
        let mut world = world(GameType::Survival);
        let position = world.levels[PLAYER].position();
        let feet = crate::world::block_position(position);
        block(
            &mut world,
            bbb_protocol::packets::BlockPos {
                y: feet.y - 1,
                ..feet
            },
            "minecraft:stone",
        );
        block(
            &mut world,
            feet,
            if collides {
                "minecraft:stone"
            } else {
                "minecraft:air"
            },
        );
        block(
            &mut world,
            bbb_protocol::packets::BlockPos {
                y: feet.y + 1,
                ..feet
            },
            "minecraft:air",
        );
        let context = FallImpulseContext {
            reset_grace_time: 40,
            impact_position: Some(position),
        };
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.cold.fall_impulse = context;
        player.cold.impulse_explosion_cause =
            Some((source("minecraft:wind_charge", position), None));
        player.block_contact.fall_distance = 8.0;
        assert_eq!(
            world.teleport_randomly(PLAYER, 0.0, &mut Vec::new()),
            !collides
        );
        let player = world.levels.player_at(PLAYER);
        assert_eq!(
            player.cold.fall_impulse,
            if collides {
                context
            } else {
                FallImpulseContext::default()
            }
        );
        assert_eq!(player.cold.impulse_explosion_cause.is_some(), collides);
        assert_eq!(
            player.block_contact.fall_distance,
            if collides { 8.0 } else { 0.0 }
        );
    }
}

fn source(resource: &str, position: Vec3d) -> ExplosionSourceEntity {
    ExplosionSourceEntity {
        custom_name: None,
        entity_id: 10000,
        entity_resource_id: resource.to_owned(),
        uuid: uuid::Uuid::from_u128(10000),
        position,
        kind: ExplosionSourceEntityKind::Other,
    }
}

fn explode(world: &mut ServerWorld, resource: &str, multiplier: f32) {
    let position = world.levels[PLAYER].position();
    let request = PendingExplosion {
        source: Some(ExplosionSource {
            direct: source(resource, position),
            indirect: None,
            causing: None,
            in_water: false,
            wind_charge: true,
        }),
        center: Vec3d {
            y: position.y - 1.0,
            ..position
        },
        radius: 3.0,
        fire: false,
        interaction: ExplosionInteraction::None,
        damage_source: None,
        damage_profile: ExplosionDamageProfile::wind_charge(false, multiplier),
        presentation: ExplosionPresentation::Default,
    };
    world.explode(Java26BuiltinDimension::Overworld, request, &mut Vec::new());
}

#[test]
fn player_impulse_explosion_callback_runs_with_zero_hurt_and_zero_push() {
    for resource in [
        "minecraft:wind_charge",
        "minecraft:breeze_wind_charge",
        "minecraft:tnt",
    ] {
        let mut world = world(GameType::Survival);
        let position = world.levels[PLAYER].position();
        let old_impact = Vec3d {
            y: position.y + 2.0,
            ..position
        };
        world.levels[PLAYER].cold.fall_impulse = FallImpulseContext {
            reset_grace_time: 7,
            impact_position: Some(old_impact),
        };
        world.levels[PLAYER].needs_sync = false;
        explode(&mut world, resource, 0.0);
        let player = world.levels.player_at(PLAYER);
        assert_eq!(player.damage.health, 20.0);
        assert_eq!(player.delta_movement(), Vec3d::default());
        assert!(player.needs_sync);
        assert_eq!(
            player.cold.fall_impulse.reset_grace_time,
            if resource == "minecraft:wind_charge" {
                40
            } else {
                0
            }
        );
        assert_eq!(
            player.cold.fall_impulse.impact_position,
            Some(if resource == "minecraft:wind_charge" {
                position
            } else {
                old_impact
            })
        );
        assert_eq!(
            player
                .cold
                .impulse_explosion_cause
                .as_ref()
                .unwrap()
                .0
                .entity_resource_id,
            resource
        );
    }
}

#[test]
fn player_impulse_explosion_push_uses_live_resistance_and_excludes_spectators() {
    let mut world = world(GameType::Survival);
    let origin = world.levels[PLAYER].position();
    let position = Vec3d {
        x: origin.x + 1.0,
        ..origin
    };
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let cow = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            cow_type,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(cow_type).unwrap(),
            ),
        )
        .0;
    explode(&mut world, "minecraft:wind_charge", 1.0);
    assert!(world.levels[PLAYER].delta_movement().y > 0.0);
    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
            .unwrap()
            .delta_movement()
            .x
            > 0.0
    );
    let body = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, cow)
        .unwrap();
    body.set_delta_movement(Vec3d::default());
    body.living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_EXPLOSION_KNOCKBACK_RESISTANCE_ID,
        )
        .unwrap()
        .set_base_value(1.0);
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.set_delta_movement(Vec3d::default());
    player
        .attributes
        .get_instance(
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_EXPLOSION_KNOCKBACK_RESISTANCE_ID,
        )
        .unwrap()
        .set_base_value(1.0);
    explode(&mut world, "minecraft:wind_charge", 1.0);
    assert_eq!(world.levels[PLAYER].delta_movement(), Vec3d::default());
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
            .unwrap()
            .delta_movement(),
        Vec3d::default()
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.game_type = GameType::Spectator;
    player.reset_fall_impulse();
    explode(&mut world, "minecraft:wind_charge", 1.0);
    assert_eq!(
        world.levels[PLAYER].cold.fall_impulse,
        FallImpulseContext::default()
    );
    assert!(world.levels[PLAYER].cold.impulse_explosion_cause.is_none());
}

#[test]
fn player_impulse_landing_reduces_damage_but_preserves_unadjusted_stat() {
    for (height, grace, health, remains) in [
        (82.0, 40, 20.0, true),
        (82.0, 0, 20.0, false),
        (79.0, 40, 20.0, false),
        (85.0, 40, 18.0, false),
    ] {
        let mut world = world(GameType::Survival);
        let pos = world.levels[PLAYER].position();
        let feet = crate::world::block_position(pos);
        block(
            &mut world,
            bbb_protocol::packets::BlockPos {
                y: feet.y - 1,
                ..feet
            },
            "minecraft:stone",
        );
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.cold.fall_impulse = FallImpulseContext {
            reset_grace_time: grace,
            impact_position: Some(Vec3d { y: height, ..pos }),
        };
        player.cold.impulse_explosion_cause = Some((source("minecraft:wind_charge", pos), None));
        player.block_contact.fall_distance = 10.0;
        world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut Vec::new());
        let player = world.levels.player_at(PLAYER);
        assert_eq!(player.damage.health, health);
        assert_eq!(
            player
                .cold
                .stats
                .value(bbb_server_types::stats::custom_stat(
                    "minecraft:fall_one_cm"
                )),
            1000
        );
        assert_eq!(player.cold.fall_impulse.impact_position.is_some(), remains);
        assert_eq!(player.cold.impulse_explosion_cause.is_some(), remains);
    }
}

#[test]
fn player_impulse_grace_relaxes_wrong_distance_but_never_new_collision() {
    for (grace, collides, accepted) in [
        (1, false, true),
        (0, false, false),
        (-1, false, false),
        (1, true, false),
    ] {
        let mut world = world(GameType::Survival);
        let position = world.levels[PLAYER].position();
        world.levels[PLAYER].last_good = position;
        world.levels[PLAYER].cold.fall_impulse.reset_grace_time = grace;
        let movement = MovePlayer {
            position: Some(position),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        };
        let mut probe = world.player_move_probe(PLAYER, &movement).unwrap();
        probe.moved_distance = 0.25;
        probe.old_position_clear = true;
        probe.collides_with_new_geometry = collides;
        let outcome = crate::world::player_packets::apply_movement(
            world.levels.player_mut(PLAYER).unwrap(),
            &mut world.scoreboard,
            movement,
            probe,
        );
        assert_eq!(
            matches!(
                outcome,
                crate::world::player_packets::MovementOutcome::Accepted
            ),
            accepted
        );
    }
}

#[test]
fn player_impulse_tick_reaches_zero_without_clearing_until_a_reset_consumer() {
    let mut world = world(GameType::Survival);
    let position = world.levels[PLAYER].position();
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.cold.fall_impulse = FallImpulseContext {
        reset_grace_time: 1,
        impact_position: Some(position),
    };
    player.cold.impulse_explosion_cause = Some((source("minecraft:wind_charge", position), None));
    world.tick(Vec::new());
    assert_eq!(world.levels[PLAYER].cold.fall_impulse.reset_grace_time, 0);
    assert_eq!(
        world.levels[PLAYER].cold.fall_impulse.impact_position,
        Some(position)
    );
    world.try_reset_player_impulse_after_movement(PLAYER, true, false);
    assert_eq!(
        world.levels[PLAYER].cold.fall_impulse,
        FallImpulseContext::default()
    );
    assert!(world.levels[PLAYER].cold.impulse_explosion_cause.is_none());
}

#[test]
fn player_impulse_flying_powder_contact_still_clears_expired_context() {
    let mut world = world(GameType::Creative);
    let position = world.levels[PLAYER].position();
    let feet = crate::world::block_position(position);
    block(&mut world, feet, "minecraft:powder_snow");
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.abilities.flying = true;
    player.cold.fall_impulse.impact_position = Some(position);
    player.block_contact.fall_distance = 2.0;
    world.tick_player_block_contacts(&mut Vec::new());
    assert!(world.levels[PLAYER]
        .cold
        .fall_impulse
        .impact_position
        .is_none());
    assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 2.0);
}

#[test]
fn player_impulse_live_save_restores_context_and_drops_runtime_cause() {
    let mut world = world(GameType::Survival);
    let position = world.levels[PLAYER].position();
    let context = FallImpulseContext {
        reset_grace_time: -7,
        impact_position: Some(Vec3d {
            y: 95.25,
            ..position
        }),
    };
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.cold.fall_impulse = context;
    player.cold.spawn_extra_particles_on_fall = true;
    player.cold.impulse_explosion_cause = Some((source("minecraft:wind_charge", position), None));
    let save = world
        .save_request(
            1_786_000_000_000,
            1,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &save.player_patches[0];
    assert_eq!(patch.created.impulse_reset_grace_time, -7);
    assert_eq!(
        patch.created.impulse_impact_position,
        context.impact_position
    );
    let restored = crate::world::player_save::restored_player_from_save_patch(patch).unwrap();
    assert_eq!(restored.fall_impulse, context);
    let raw =
        bbb_server_persistence::created_player_document(patch, save.item_stack_resolver.as_dyn())
            .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(raw, patch.uuid).unwrap();
    assert_eq!(parsed.impulse_reset_grace_time(), -7);
    assert_eq!(parsed.impulse_impact_position(), context.impact_position);
    let root =
        std::env::temp_dir().join(format!("bbb-player-impulse-restore-{}", std::process::id()));
    let storage = crate::tests::created_fixture_save(&root, "impulse-restore");
    let mut candidate = storage.load_candidate().unwrap();
    candidate.players.insert(
        patch.uuid,
        (bbb_server_persistence::PlayerDataSource::Primary, parsed),
    );
    let mut next =
        ServerWorld::restore(crate::world::ServerWorldConfig::default(), &candidate).unwrap();
    assert_eq!(
        next.cold.restored_players[&patch.uuid].fall_impulse,
        context
    );
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut next, 1, "alice", 1);
    assert_eq!(next.levels[PLAYER].cold.fall_impulse, context);
    assert!(next.levels[PLAYER].cold.spawn_extra_particles_on_fall);
    assert!(next.levels[PLAYER].cold.impulse_explosion_cause.is_none());
    std::fs::remove_dir_all(root).unwrap();
}
