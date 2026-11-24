use super::super::tests::{block, world};
use super::*;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::packets::MovePlayer;

const PLAYER: ConnectionId = ConnectionId(1);

#[test]
fn player_environment_powder_contact_freezes_hurts_and_keeps_residual_frost() {
    let mut world = world(GameType::Survival);
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    block(&mut world, feet, "minecraft:powder_snow");
    block(
        &mut world,
        BlockPos {
            y: feet.y - 1,
            ..feet
        },
        "minecraft:stone",
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.set_ticks_frozen(139);
    player.tick_count = 39;
    world.tick(Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.ticks_frozen, 140);
    assert_eq!(player.damage.health, 19.0);
    assert!(
        (player
            .attributes
            .value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .unwrap()
            - (0.1 - f64::from(0.05_f32)))
        .abs()
            < 1.0e-7
    );
    block(&mut world, feet, "minecraft:air");
    world.tick(Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.ticks_frozen, 138);
    assert!(
        player
            .attributes
            .value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .unwrap()
            < 0.1
    );
    block(
        &mut world,
        BlockPos {
            y: feet.y - 1,
            ..feet
        },
        "minecraft:air",
    );
    world.tick(Vec::new());
    assert!(
        (world.levels[PLAYER]
            .attributes
            .value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .unwrap()
            - 0.1)
            .abs()
            < 1.0e-7
    );
}

#[test]
fn player_environment_fire_ignite_adds_one_or_two_ticks_only_to_a_nonnegative_clock() {
    let mut world = world(GameType::Survival);
    world.levels[PLAYER].remaining_fire_ticks = 165;
    assert!(world.apply_player_contact_effect(
        PLAYER,
        InsideBlockEffect::Type(InsideBlockEffectType::FireIgnite),
        &mut Vec::new()
    ));
    assert!([166, 167].contains(&world.levels[PLAYER].remaining_fire_ticks));
    world.levels[PLAYER].remaining_fire_ticks = -2;
    assert!(world.apply_player_contact_effect(
        PLAYER,
        InsideBlockEffect::Type(InsideBlockEffectType::FireIgnite),
        &mut Vec::new()
    ));
    assert_eq!(world.levels[PLAYER].remaining_fire_ticks, -1);
}

#[test]
fn player_environment_flying_powder_contact_freezes_without_stuck_multiplier() {
    let mut world = world(GameType::Creative);
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    block(&mut world, feet, "minecraft:powder_snow");
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.abilities.flying = true;
    player.block_contact.fall_distance = 2.0;
    world.tick_player_block_contacts(&mut Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.ticks_frozen, 1);
    assert_eq!(player.block_contact.stuck_multiplier, Vec3d::default());
    assert_eq!(player.block_contact.fall_distance, 2.0);
}

#[test]
fn player_environment_collision_movement_crosses_powder_without_ending_inside() {
    let mut world = world(GameType::Survival);
    let from = world.levels[PLAYER].position();
    let powder = BlockPos {
        x: from.x.floor() as i32 + 1,
        y: from.y.floor() as i32,
        z: from.z.floor() as i32,
    };
    block(&mut world, powder, "minecraft:powder_snow");
    world.levels[PLAYER].last_good = from;
    let movement = MovePlayer {
        position: Some(Vec3d {
            x: from.x + 3.0,
            ..from
        }),
        rotation: None,
        on_ground: false,
        horizontal_collision: false,
    };
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert!(matches!(
        crate::world::player_packets::apply_movement(
            world.levels.player_mut(PLAYER).unwrap(),
            &mut world.scoreboard,
            movement,
            probe
        ),
        crate::world::player_packets::MovementOutcome::Accepted
    ));
    world.tick_player_block_contacts(&mut Vec::new());
    assert_eq!(world.levels[PLAYER].ticks_frozen, 1);
    assert!(world.levels[PLAYER].block_contact.pending.is_empty());
}

#[test]
fn player_environment_teleport_discards_preceding_contact_path() {
    let mut world = world(GameType::Survival);
    let from = world.levels[PLAYER].position();
    let to = Vec3d {
        x: from.x + 3.0,
        ..from
    };
    block(
        &mut world,
        BlockPos {
            x: from.x.floor() as i32 + 1,
            y: from.y.floor() as i32,
            z: from.z.floor() as i32,
        },
        "minecraft:powder_snow",
    );
    crate::entity::record_body_movement(
        &mut world.levels[PLAYER].block_contact.pending,
        EntityMovement::direct(from, to),
    );
    world.levels[PLAYER].block_contact.supporting = Some(BlockPos {
        x: from.x.floor() as i32,
        y: (from.y - 0.2).floor() as i32,
        z: from.z.floor() as i32,
    });
    let player = world.levels.player_at(PLAYER);
    let transition = crate::world::TeleportTransition {
        new_level: player.dimension,
        position: to,
        y_rot: player.rotation().y_rot,
        x_rot: player.rotation().x_rot,
        relatives_mask: 0,
    };
    world.teleport_set_position(PLAYER, &transition, &mut Vec::new());
    assert!(world.levels[PLAYER].block_contact.pending.is_empty());
    assert_eq!(
        world.player_legacy_support_pos(PLAYER),
        Some(BlockPos {
            x: to.x.floor() as i32,
            y: (to.y - 0.2).floor() as i32,
            z: to.z.floor() as i32,
        })
    );
    world.tick_player_block_contacts(&mut Vec::new());
    assert_eq!(world.levels[PLAYER].ticks_frozen, 0);
}

#[test]
fn player_environment_fall_distance_accumulates_float_steps_and_saves_live_value() {
    let mut world = world(GameType::Survival);
    let dy = -0.123456789_f64;
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.block_contact.fall_distance = 2.75;
    player.known_movement = Vec3d {
        x: 0.0,
        y: dy,
        z: 0.0,
    };
    player.set_on_ground(false);
    world.update_player_contact_support(PLAYER, &mut Vec::new());
    let expected = 2.75 - f64::from(dy as f32);
    assert_eq!(world.levels[PLAYER].block_contact.fall_distance, expected);
    let save = world
        .save_request(
            1_786_000_000_000,
            1,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &save.player_patches[0];
    assert_eq!(patch.created.fall_distance, expected);
    let raw =
        bbb_server_persistence::created_player_document(patch, save.item_stack_resolver.as_dyn())
            .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(raw, patch.uuid).unwrap();
    assert_eq!(parsed.fall_distance(), expected);
    let restored = crate::world::player_save::restored_player_from_save_patch(patch).unwrap();
    assert_eq!(restored.fall_distance, expected);
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.known_movement.y = 0.1;
    world.update_player_contact_support(PLAYER, &mut Vec::new());
    assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 0.0);
    let save = world
        .save_request(
            1_786_000_000_001,
            2,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &save.player_patches[0];
    let raw = parsed
        .raw_with_player_save_patch(patch, save.item_stack_resolver.as_dyn())
        .unwrap();
    let reparsed = bbb_server_persistence::Java26PlayerData::parse(raw, patch.uuid).unwrap();
    assert_eq!(reparsed.fall_distance(), 0.0);
}

#[test]
fn player_environment_reset_ray_observes_flowing_water_height_and_portal_delay() {
    let mut world = world(GameType::Survival);
    let feet = crate::world::block_position(world.levels[PLAYER].position());
    let cell = BlockPos {
        x: feet.x + 1,
        ..feet
    };
    let dimension = world.levels[PLAYER].dimension;
    let from = Vec3d {
        y: feet.y as f64 + 0.4,
        ..world.levels[PLAYER].position()
    };
    let to = Vec3d {
        x: from.x + 3.0,
        ..from
    };
    block(&mut world, cell, "minecraft:water");
    let state = world.level_by_id(&dimension).block_state(cell);
    let flowing = crate::fluid::with_property(state, "level", "1").unwrap();
    world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(cell.x, cell.y, cell.z, flowing)
        .unwrap();
    let ray = |world: &ServerWorld, a, b, instant| {
        crate::world::pov_clip::fall_resetting_clip(
            &world.level_by_id(&dimension).chunks,
            a,
            b,
            instant,
        )
    };
    assert_eq!(ray(&world, from, to, false), Some(true));
    assert_eq!(
        ray(
            &world,
            Vec3d {
                y: feet.y as f64 + 0.99,
                ..from
            },
            Vec3d {
                y: feet.y as f64 + 0.99,
                ..to
            },
            false
        ),
        Some(false)
    );
    block(&mut world, cell, "minecraft:lava");
    assert_eq!(ray(&world, from, to, false), Some(false));
    block(&mut world, cell, "minecraft:nether_portal");
    assert_eq!(ray(&world, from, to, false), Some(false));
    assert_eq!(ray(&world, from, to, true), Some(true));
    block(&mut world, cell, "minecraft:end_portal");
    assert_eq!(ray(&world, from, to, false), Some(true));
}

#[test]
fn player_environment_reset_ray_uses_live_tags_and_preserves_unknown() {
    let mut world = world(GameType::Survival);
    let from = world.levels[PLAYER].position();
    let to = Vec3d {
        x: from.x + 3.0,
        ..from
    };
    let cell = BlockPos {
        x: from.x.floor() as i32 + 1,
        y: from.y.floor() as i32,
        z: from.z.floor() as i32,
    };
    block(&mut world, cell, "minecraft:stone");
    let dimension = world.levels[PLAYER].dimension;
    for (owned, expected) in [(true, Some(true)), (false, Some(false))] {
        let mut tags = Vec::new();
        if owned {
            tags.push((
                "minecraft:block".to_string(),
                "minecraft:fall_damage_resetting".to_string(),
                vec!["minecraft:stone".to_string()],
            ));
        }
        world.install_level_registry_tags(std::sync::Arc::new(
            crate::registry_tags::RegistryTagSnapshot::new(tags),
        ));
        assert_eq!(
            crate::world::pov_clip::fall_resetting_clip(
                &world.level_by_id(&dimension).chunks,
                from,
                to,
                false
            ),
            expected
        );
    }
}

#[test]
fn player_environment_long_move_reset_is_capped_and_precedes_collision_rejection() {
    let mut world = world(GameType::Survival);
    let from = world.levels[PLAYER].position();
    let feet = crate::world::block_position(from);
    let near = BlockPos {
        x: feet.x + 6,
        ..feet
    };
    let far = BlockPos {
        x: feet.x + 9,
        ..feet
    };
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.last_good = from;
    player.block_contact.fall_distance = 5.0;
    let movement = MovePlayer {
        position: Some(Vec3d {
            x: from.x + 12.0,
            ..from
        }),
        rotation: None,
        on_ground: false,
        horizontal_collision: false,
    };
    block(&mut world, far, "minecraft:water");
    assert!(
        !world
            .player_move_probe(PLAYER, &movement)
            .unwrap()
            .reset_fall_distance
    );
    block(&mut world, near, "minecraft:water");
    assert!(
        world
            .player_move_probe(PLAYER, &movement)
            .unwrap()
            .reset_fall_distance
    );
    block(
        &mut world,
        BlockPos {
            x: feet.x + 8,
            ..feet
        },
        "minecraft:stone",
    );
    let probe = world.player_move_probe(PLAYER, &movement).unwrap();
    assert!(probe.reset_fall_distance);
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.first_good = from;
    player.set_delta_movement(Vec3d {
        x: 12.0,
        ..Vec3d::default()
    });
    let mut events = Vec::new();
    world.apply_command(
        crate::world::WorldCommand::Play {
            connection_id: PLAYER,
            packet: bbb_protocol::packets::PlayServerbound::MovePlayer(movement),
        },
        &mut events,
    );
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: PLAYER,
            packet: crate::connection::ClientboundPlayPacket::PlayerPosition(update) }
        if update.position == from
    )));
    assert_eq!(world.levels[PLAYER].position(), from);
    assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 0.0);
}

#[test]
fn player_environment_rejected_ground_packet_clears_fall_after_teleport() {
    let mut world = world(GameType::Survival);
    let from = world.levels[PLAYER].position();
    let feet = crate::world::block_position(from);
    block(
        &mut world,
        BlockPos {
            x: feet.x + 2,
            ..feet
        },
        "minecraft:stone",
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.first_good = from;
    player.last_good = from;
    player.block_contact.fall_distance = 5.0;
    let mut events = Vec::new();
    world.apply_command(
        crate::world::WorldCommand::Play {
            connection_id: PLAYER,
            packet: bbb_protocol::packets::PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: from.x + 4.0,
                    ..from
                }),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        },
        &mut events,
    );
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.position(), from);
    assert!(player.awaiting_teleport.is_some());
    assert_eq!(player.block_contact.fall_distance, 0.0);
    assert!(player.block_contact.pending.is_empty());
    assert!(!player.on_ground());
}

#[test]
fn player_environment_empty_fluid_tag_does_not_create_reset_ray_shape() {
    let mut world = world(GameType::Survival);
    let from = world.levels[PLAYER].position();
    let to = Vec3d {
        x: from.x + 3.0,
        ..from
    };
    world.install_level_registry_tags(std::sync::Arc::new(
        crate::registry_tags::RegistryTagSnapshot::new(vec![(
            "minecraft:fluid".to_string(),
            "minecraft:water".to_string(),
            vec!["minecraft:empty".to_string()],
        )]),
    ));
    let dimension = &world.levels[PLAYER].dimension;
    assert_eq!(
        crate::world::pov_clip::fall_resetting_clip(
            &world.level_by_id(dimension).chunks,
            from,
            to,
            false
        ),
        Some(false)
    );
}

fn campfire_contact(world: &mut ServerWorld, name: &str, lit: bool) -> BodyBlockContact {
    let pos = crate::world::block_position(world.levels[PLAYER].position());
    block(world, pos, name);
    let dimension = world.levels[PLAYER].dimension;
    let state = world.level_by_id(&dimension).block_state(pos);
    let state =
        crate::fluid::with_property(state, "lit", if lit { "true" } else { "false" }).unwrap();
    world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(pos.x, pos.y, pos.z, state)
        .unwrap();
    BodyBlockContact {
        pos,
        state,
        resource_id: if name == "minecraft:campfire" {
            "minecraft:campfire"
        } else {
            "minecraft:soul_campfire"
        },
        environment_kind: EnvironmentBlockKind::Other,
        step: 0,
        precise: false,
        inside_block: true,
        fluid: None,
    }
}

#[test]
fn player_environment_campfire_stationary_real_tick_uses_full_inside_cube() {
    for (name, damage) in [
        ("minecraft:campfire", 1.0),
        ("minecraft:soul_campfire", 2.0),
    ] {
        for lit in [false, true] {
            let mut world = world(GameType::Survival);
            campfire_contact(&mut world, name, lit);
            let player = world.levels.player_mut(PLAYER).unwrap();
            {
                let mut position = player.position();
                position.y += 0.75;
                player.set_position(position);
            }
            player.block_contact.old_position = player.position();
            player.set_on_ground(false);
            world.tick(Vec::new());
            assert_eq!(
                world.levels[PLAYER].damage.health,
                20.0 - if lit { damage } else { 0.0 }
            );
            assert!(world.levels[PLAYER].remaining_fire_ticks < 0);
        }
    }
}

#[test]
fn player_environment_campfire_ignores_careful_precise_and_waterlogged() {
    for precise in [false, true] {
        let mut world = world(GameType::Survival);
        let mut contact = campfire_contact(&mut world, "minecraft:campfire", true);
        contact.precise = precise;
        contact.state = crate::fluid::with_property(contact.state, "waterlogged", "true").unwrap();
        world.levels[PLAYER].set_shared_flag(
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            true,
        );
        world.levels[PLAYER].set_ticks_frozen(30);
        let mut events = Vec::new();
        let mut visitor = SessionContact {
            world: &mut world,
            id: PLAYER,
            events: &mut events,
            collector: BlockEffectCollector::default(),
        };
        assert!(visitor.visit_contact(contact));
        assert_eq!(visitor.world.levels[PLAYER].damage.health, 19.0);
        assert_eq!(visitor.world.levels[PLAYER].ticks_frozen, 30);
        assert!(visitor.collector.take_effects().is_empty());
    }
}

#[test]
fn player_environment_campfire_rejected_damage_does_not_abort_later_contacts() {
    for game_type in [GameType::Survival, GameType::Creative, GameType::Spectator] {
        let mut world = world(game_type);
        campfire_contact(&mut world, "minecraft:soul_campfire", true);
        if game_type == GameType::Survival {
            assert_eq!(
                world.add_player_mob_effect(
                    PLAYER,
                    crate::entity::ActiveMobEffect::new("minecraft:fire_resistance", 200, 0)
                        .unwrap(),
                    &mut Vec::new()
                ),
                Some(true)
            );
        }
        let feet = crate::world::block_position(world.levels[PLAYER].position());
        block(
            &mut world,
            BlockPos {
                y: feet.y + 1,
                ..feet
            },
            "minecraft:powder_snow",
        );
        world.levels[PLAYER].block_contact.old_position = world.levels[PLAYER].position();
        world.tick_player_block_contacts(&mut Vec::new());
        assert_eq!(world.levels[PLAYER].damage.health, 20.0);
        assert_eq!(
            world.levels[PLAYER].ticks_frozen,
            if game_type == GameType::Spectator {
                0
            } else {
                1
            }
        );
    }
}

#[test]
fn player_environment_campfire_damage_precedes_previously_collected_fire_damage() {
    let mut world = world(GameType::Survival);
    let contact = campfire_contact(&mut world, "minecraft:soul_campfire", true);
    let dimension = world.levels[PLAYER].dimension;
    world
        .level_mut_by_id(&dimension)
        .chunks
        .set_block_state(
            contact.pos.x,
            contact.pos.y + 1,
            contact.pos.z,
            contact.state,
        )
        .unwrap();
    block(&mut world, contact.pos, "minecraft:fire");
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.damage.health = 1.0;
    player.block_contact.old_position = player.position();
    let mut events = Vec::new();
    world.tick_player_block_contacts(&mut events);
    let damage_types: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: crate::connection::ClientboundPlayPacket::DamageEvent(data),
            } if *connection_id == PLAYER => Some(data.source_type_id),
            _ => None,
        })
        .collect();
    let campfire_id = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|entry| entry.resource_id == "minecraft:campfire")
        .unwrap()
        .registry_id;
    assert_eq!(damage_types, [campfire_id]);
    assert!(world.levels[PLAYER].damage.is_dead_or_dying());
    assert_eq!(world.levels[PLAYER].remaining_fire_ticks, -20);
}

#[test]
fn player_end_gateway_contact_teleports_next_tick_and_saves_live_cooldown() {
    let mut world = world(GameType::Survival);
    let dimension = world.levels[PLAYER].dimension;
    let entry = crate::world::block_position(world.levels[PLAYER].position());
    let exit = BlockPos {
        x: entry.x + 64,
        y: 85,
        z: entry.z,
    };
    assert!(world.set_block(
        dimension.builtin().unwrap(),
        entry,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_END_GATEWAY_ID
        )
        .unwrap(),
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    world
        .level_mut_by_id(&dimension)
        .chunks
        .block_entities_mut()
        .get_mut(entry)
        .unwrap()
        .end_gateway_mut()
        .unwrap()
        .set_exit_position(exit, true);
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.set_y_rot(35.0);
    player.set_x_rot(12.0);
    player.set_delta_movement(Vec3d {
        x: 0.25,
        y: 0.0,
        z: 0.5,
    });
    world.tick(Vec::new());
    assert!(world.levels[PLAYER].cold.portal_process.is_some());
    assert_eq!(world.levels[PLAYER].cold.portal_cooldown, 0);
    assert!(world
        .level_by_id(&dimension)
        .chunks
        .block_entities()
        .get(entry)
        .unwrap()
        .end_gateway()
        .unwrap()
        .is_cooling_down());
    world.levels[PLAYER].set_delta_movement(Vec3d {
        x: 0.25,
        y: 0.0,
        z: 0.5,
    });
    let result = world.tick(Vec::new());
    assert!(result.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id, packet: crate::connection::ClientboundPlayPacket::PlayerPosition(update) }
            if *connection_id == PLAYER && update.relatives_mask == 504
    )));
    let player = world.levels.player_at(PLAYER);
    assert_eq!(
        player.position(),
        Vec3d {
            x: f64::from(exit.x) + 0.5,
            y: 85.0,
            z: f64::from(exit.z) + 0.5
        }
    );
    assert_eq!(player.cold.portal_cooldown, 10);
    assert_eq!(
        (player.rotation().y_rot, player.rotation().x_rot),
        (35.0, 12.0)
    );
    assert_eq!(
        (player.delta_movement().x, player.delta_movement().z),
        (0.0, 0.0)
    );
    assert!(player.awaiting_teleport.is_some());
    let save = world
        .save_request(
            1786000000000,
            1,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &save.player_patches[0];
    assert_eq!(patch.created.portal_cooldown, 10);
    let raw =
        bbb_server_persistence::created_player_document(patch, save.item_stack_resolver.as_dyn())
            .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(raw, patch.uuid).unwrap();
    assert_eq!(parsed.portal_cooldown(), 10);
    let restored = crate::world::player_save::restored_player_from_save_patch(patch).unwrap();
    assert_eq!(restored.portal_cooldown, 10);
    world.tick(Vec::new());
    assert_eq!(world.levels[PLAYER].cold.portal_cooldown, 9);
    let save = world
        .save_request(
            1786000000001,
            2,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &save.player_patches[0];
    let raw = parsed
        .raw_with_player_save_patch(patch, save.item_stack_resolver.as_dyn())
        .unwrap();
    assert_eq!(
        bbb_server_persistence::Java26PlayerData::parse(raw, patch.uuid)
            .unwrap()
            .portal_cooldown(),
        9
    );
    world.levels[PLAYER].is_changing_dimension = true;
    world.tick(Vec::new());
    assert_eq!(world.levels[PLAYER].cold.portal_cooldown, 9);
}

#[test]
fn player_environment_bubble_column_top_sends_particles_unless_flying() {
    for flying in [false, true] {
        let mut world = world(GameType::Survival);
        let feet = crate::world::block_position(world.levels[PLAYER].position());
        block(&mut world, feet, "minecraft:bubble_column");
        block(
            &mut world,
            BlockPos {
                y: feet.y + 1,
                ..feet
            },
            "minecraft:air",
        );
        world.levels.player_mut(PLAYER).unwrap().abilities.flying = flying;
        let tick = world.tick(Vec::new());
        let bubbles = tick
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    WorldEvent::Send {
                        connection_id,
                        packet: crate::connection::ClientboundPlayPacket::LevelParticles(packet),
                    } if *connection_id == PLAYER
                        && packet.particle.particle_type_id
                            == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BUBBLE_ID
                )
            })
            .count();
        assert_eq!(bubbles, if flying { 0 } else { 2 }, "flying={flying}");
    }
}
