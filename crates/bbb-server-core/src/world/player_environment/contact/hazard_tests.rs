use super::super::tests::{block, world};
use super::*;
use crate::entity::living::FallImpulseContext;
use bbb_protocol::difficulties::Difficulty;
const PLAYER: ConnectionId = ConnectionId(1);

fn place(world: &mut ServerWorld, name: &str) -> BlockPos {
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.block_contact.old_position = player.position();
    let pos = crate::world::block_position(player.position());
    block(world, pos, name);
    pos
}

#[test]
fn player_environment_berry_reads_client_movement_and_preserves_flying_impulse_order() {
    let threshold = f64::from(0.003_f32);
    for flying in [false, true] {
        for grace in [-1, 0, 4] {
            for (speed, hurt) in [
                (threshold, true),
                (-threshold, true),
                (threshold - 1.0e-12, false),
                (0.0, false),
            ] {
                let mut world = world(GameType::Survival);
                let pos = place(&mut world, "minecraft:sweet_berry_bush");
                let dimension = world.levels[PLAYER].dimension;
                let state = world.level_by_id(&dimension).block_state(pos);
                let state = crate::fluid::with_property(state, "age", "3").unwrap();
                world
                    .level_mut_by_id(&dimension)
                    .chunks
                    .set_block_state(pos.x, pos.y, pos.z, state)
                    .unwrap();
                let player = world.levels.player_mut(PLAYER).unwrap();
                player.abilities.flying = flying;
                player.known_movement = Vec3d {
                    x: speed,
                    y: 100.0,
                    z: 0.0,
                };
                player.block_contact.fall_distance = 5.0;
                player.block_contact.stuck_multiplier = Vec3d {
                    x: 2.0,
                    y: 2.0,
                    z: 2.0,
                };
                player.cold.fall_impulse = FallImpulseContext {
                    reset_grace_time: grace,
                    impact_position: Some(player.position()),
                };
                world.tick_player_block_contacts(&mut Vec::new());
                let player = world.levels.player_at(PLAYER);
                assert_eq!(player.damage.health, if hurt { 19.0 } else { 20.0 });
                assert_eq!(
                    player.block_contact.fall_distance,
                    if flying { 5.0 } else { 0.0 }
                );
                assert_eq!(
                    player.block_contact.stuck_multiplier,
                    if flying {
                        Vec3d {
                            x: 2.0,
                            y: 2.0,
                            z: 2.0,
                        }
                    } else {
                        crate::block_behavior::sweet_berry_bush::stuck_multiplier()
                    }
                );
                assert_eq!(
                    player.cold.fall_impulse.impact_position.is_some(),
                    grace != 0
                );
            }
        }
    }
}

#[test]
fn player_environment_berry_age_zero_sticks_without_damage_and_vertical_motion_does_not_hurt() {
    for age in ["0", "3"] {
        let mut world = world(GameType::Survival);
        let pos = place(&mut world, "minecraft:sweet_berry_bush");
        let dimension = world.levels[PLAYER].dimension;
        let state = world.level_by_id(&dimension).block_state(pos);
        let state = crate::fluid::with_property(state, "age", age).unwrap();
        world
            .level_mut_by_id(&dimension)
            .chunks
            .set_block_state(pos.x, pos.y, pos.z, state)
            .unwrap();
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.known_movement = if age == "0" {
            Vec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }
        } else {
            Vec3d {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            }
        };
        world.tick_player_block_contacts(&mut Vec::new());
        assert_eq!(world.levels[PLAYER].damage.health, 20.0);
        assert_eq!(
            world.levels[PLAYER].block_contact.stuck_multiplier,
            crate::block_behavior::sweet_berry_bush::stuck_multiplier()
        );
    }
}

#[test]
fn player_environment_wither_rose_uses_readonly_immunity_and_existing_effect_tick() {
    for (mode, peaceful, invulnerable, expected) in [
        (GameType::Survival, false, false, true),
        (GameType::Creative, false, false, true),
        (GameType::Survival, true, false, false),
        (GameType::Survival, false, true, false),
        (GameType::Spectator, false, false, false),
    ] {
        let mut world = world(mode);
        world.admin.set_difficulty(if peaceful {
            Difficulty::Peaceful
        } else {
            Difficulty::Normal
        });
        let pos = place(&mut world, "minecraft:wither_rose");
        world.levels[PLAYER].entity_invulnerable = invulnerable;
        let mut events = Vec::new();
        world.tick_player_block_contacts(&mut events);
        let player = world.levels.player_at(PLAYER);
        assert_eq!(
            player
                .active_effect("minecraft:wither")
                .map(|effect| effect.duration()),
            expected.then_some(40)
        );
        assert_eq!(player.damage.health, 20.0);
        assert_eq!(events.iter().any(|event| matches!(event, WorldEvent::Send { connection_id, packet: crate::connection::ClientboundPlayPacket::UpdateMobEffect(_) } if *connection_id == PLAYER)), expected);
        if expected {
            block(&mut world, pos, "minecraft:air");
            world.tick(Vec::new());
            assert_eq!(
                world.levels[PLAYER]
                    .active_effect("minecraft:wither")
                    .unwrap()
                    .duration(),
                39
            );
            assert_eq!(
                world.levels[PLAYER].damage.health,
                if mode == GameType::Creative {
                    20.0
                } else {
                    19.0
                }
            );
        }
    }
}

#[test]
fn player_environment_cactus_is_immediate_and_stops_later_contacts_on_death() {
    let mut world = world(GameType::Survival);
    let pos = place(&mut world, "minecraft:cactus");
    block(
        &mut world,
        BlockPos {
            y: pos.y + 1,
            ..pos
        },
        "minecraft:fire",
    );
    world.levels[PLAYER].damage.health = 1.0;
    let mut events = Vec::new();
    world.tick_player_block_contacts(&mut events);
    assert!(world.levels[PLAYER].damage.is_dead_or_dying());
    let damage = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|row| row.resource_id == "minecraft:cactus")
        .unwrap()
        .registry_id;
    let sources: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: crate::connection::ClientboundPlayPacket::DamageEvent(event),
            } if *connection_id == PLAYER => Some(event.source_type_id),
            _ => None,
        })
        .collect();
    assert_eq!(sources, [damage]);
    assert!(world.levels[PLAYER].remaining_fire_ticks <= 0);
}

#[test]
fn player_environment_eyeblossom_contact_continues_into_later_hazards() {
    for flower in ["minecraft:open_eyeblossom", "minecraft:closed_eyeblossom"] {
        let mut world = world(GameType::Survival);
        let pos = place(&mut world, flower);
        let from = world.levels[PLAYER].position();
        let to = Vec3d {
            x: from.x + 2.0,
            ..from
        };
        block(
            &mut world,
            BlockPos {
                x: pos.x + 2,
                ..pos
            },
            "minecraft:cactus",
        );
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.set_position(to);
        crate::entity::record_body_movement(
            &mut player.block_contact.pending,
            crate::entity::EntityMovement::direct(from, to),
        );
        world.tick_player_block_contacts(&mut Vec::new());
        assert_eq!(world.levels[PLAYER].damage.health, 19.0, "{flower}");
        assert!(world.levels[PLAYER]
            .active_effect("minecraft:poison")
            .is_none());
    }
}

#[test]
fn player_environment_lava_cauldron_hurts_ignites_and_thaws() {
    let mut world = world(GameType::Survival);
    place(&mut world, "minecraft:lava_cauldron");
    world.levels[PLAYER].set_ticks_frozen(17);
    world.tick_player_block_contacts(&mut Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.damage.health, 16.0);
    assert_eq!(player.remaining_fire_ticks, 300);
    assert_eq!(player.ticks_frozen, 0);
}

#[test]
fn player_environment_layered_cauldron_consumes_for_creative_and_respects_spawn_protection() {
    use crate::admin::{NameAndId, OperatorEntry, StoredUserLists};
    use crate::block_behavior::cauldron;
    use crate::commands::CommandPermission;
    use crate::world::ServerWorldConfig;
    let mut lists = StoredUserLists::default();
    let operator = NameAndId {
        id: uuid::Uuid::from_u128(999),
        name: "Operator".into(),
    };
    lists.operators.insert(
        operator.id,
        OperatorEntry {
            user: operator,
            level: CommandPermission::All,
            bypasses_player_limit: false,
        },
    );
    for protected in [false, true] {
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: GameType::Creative,
            admin_lists: lists.clone(),
            spawn_protection_radius: if protected { 16 } else { 0 },
            ..ServerWorldConfig::default()
        })
        .unwrap();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let spawn = world.cold.default_spawn.position;
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.set_position(Vec3d {
            x: spawn.x.floor() + 0.5,
            y: 80.5,
            z: spawn.z.floor() + 0.5,
        });
        let pos = place(&mut world, "minecraft:powder_snow_cauldron");
        let dimension = world.levels[PLAYER].dimension;
        let full = cauldron::filled_powder_snow_cauldron_state();
        world
            .level_mut_by_id(&dimension)
            .chunks
            .set_block_state(pos.x, pos.y, pos.z, full)
            .unwrap();
        assert_eq!(
            world.player_is_under_spawn_protection(PLAYER, pos),
            protected
        );
        world.levels[PLAYER].remaining_fire_ticks = 100;
        world.tick_player_block_contacts(&mut Vec::new());
        assert!(world.levels[PLAYER].remaining_fire_ticks <= 0);
        assert_eq!(world.levels[PLAYER].ticks_frozen, 0);
        assert_eq!(
            world.level_by_id(&dimension).block_state(pos),
            if protected {
                full
            } else {
                cauldron::layered_cauldron_state(
                    bbb_protocol::block_types::VANILLA_BLOCK_WATER_CAULDRON_ID,
                    2,
                )
                .unwrap()
            }
        );

        let mut store = crate::entity::EntityStore::new(0, crate::entity::EntityIdCounter::new());
        let player = world.levels.player(PLAYER).unwrap();
        let mut data = crate::entity::ArrowData::new(
            crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
        );
        data.projectile.set_owner(Some(player.profile.uuid));
        let id = store.spawn(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
            player.position(),
            crate::entity::EntityData::Arrow(data),
        );
        let entity = store.get_mut(id).unwrap();
        entity.set_no_gravity(true);
        entity.set_remaining_fire_ticks(100);
        world.with_level(dimension, |level| {
            crate::entity::tick_entities(&mut store, level, true);
        });
        assert!(store.get(id).unwrap().remaining_fire_ticks() <= 0);
        assert_eq!(
            cauldron::cauldron_level(world.level_by_id(&dimension).block_state(pos)),
            Some(if protected { 3 } else { 1 })
        );
    }
}

/// PowderSnowBlock.java:84-93: a burning player melts the snow only where
/// `mayInteract` allows, which spawn protection refuses
/// (ServerPlayer.java:2054-2056).
#[test]
fn player_environment_burning_player_melts_powder_snow_outside_spawn_protection() {
    use crate::admin::{NameAndId, OperatorEntry, StoredUserLists};
    use crate::commands::CommandPermission;
    use crate::world::ServerWorldConfig;
    let mut lists = StoredUserLists::default();
    let operator = NameAndId {
        id: uuid::Uuid::from_u128(999),
        name: "Operator".into(),
    };
    lists.operators.insert(
        operator.id,
        OperatorEntry {
            user: operator,
            level: CommandPermission::All,
            bypasses_player_limit: false,
        },
    );
    for protected in [false, true] {
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: GameType::Creative,
            admin_lists: lists.clone(),
            spawn_protection_radius: if protected { 16 } else { 0 },
            ..ServerWorldConfig::default()
        })
        .unwrap();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let spawn = world.cold.default_spawn.position;
        world
            .levels
            .player_mut(PLAYER)
            .unwrap()
            .set_position(Vec3d {
                x: spawn.x.floor() + 0.5,
                y: 80.5,
                z: spawn.z.floor() + 0.5,
            });
        let pos = place(&mut world, "minecraft:powder_snow");
        let dimension = world.levels[PLAYER].dimension;
        let snow = world.level_by_id(&dimension).block_state(pos);
        world.levels[PLAYER].remaining_fire_ticks = 100;
        world.tick_player_block_contacts(&mut Vec::new());
        let melted = bbb_server_types::fluid::block_state_is_air(
            world.level_by_id(&dimension).block_state(pos),
        );
        assert_eq!(melted, !protected, "protected: {protected}");
        if protected {
            assert_eq!(world.level_by_id(&dimension).block_state(pos), snow);
        }
    }
}
