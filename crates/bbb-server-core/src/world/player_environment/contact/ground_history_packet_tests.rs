use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::entity::{EntityData, GeneralLivingData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::packets::{MovePlayer, PlayServerbound, PlayerAbilities};
use bbb_server_types::save::Java26BuiltinDimension;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const PLAYER: ConnectionId = ConnectionId(1);
const OLD: BlockPos = BlockPos { x: 6, y: 65, z: 8 };
const START: Vec3d = Vec3d {
    x: 8.5,
    y: 65.25,
    z: 8.5,
};

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn write(world: &mut ServerWorld, pos: BlockPos, name: &str) {
    let value = state(name);
    if world.block_state(DIMENSION, pos) != Some(value) {
        assert!(world.set_block(DIMENSION, pos, value, SET_BLOCK_UPDATE_ALL));
    }
    assert_eq!(world.block_state(DIMENSION, pos), Some(value));
}

fn fixture(start_y: f64, distance: f64) -> ServerWorld {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert_eq!(world.levels.player_count(), 1);
    assert_eq!(world.levels[PLAYER].block_contact.last_climbable_pos, None);
    world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    for x in 5..=12 {
        for z in 7..=10 {
            for y in 64..=69 {
                write(
                    &mut world,
                    BlockPos { x, y, z },
                    if y == 64 {
                        "minecraft:stone"
                    } else {
                        "minecraft:air"
                    },
                );
            }
        }
    }
    write(&mut world, BlockPos { z: 9, ..OLD }, "minecraft:stone");
    write(&mut world, OLD, "minecraft:ladder");
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.game_type = GameType::Survival;
    player.abilities = PlayerAbilities::for_game_type(GameType::Survival);
    world.set_player_position_for_test(PLAYER, Vec3d { x: 6.8, ..START });
    let cow = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d { x: 7.1, ..START },
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
        )
        .0;
    world.tick_player_push(&DIMENSION.into());
    assert!(
        world
            .stored_entity_for_test(DIMENSION, cow)
            .unwrap()
            .delta_movement()
            .x
            > 0.0
    );
    assert_eq!(world.levels[PLAYER].delta_movement(), Vec3d::default());
    assert_eq!(
        world.levels[PLAYER].block_contact.last_climbable_pos,
        Some(OLD)
    );
    assert!(!world.levels[PLAYER].needs_sync);
    let mut cow = world.level_mut(DIMENSION).entities.detach(cow).unwrap();
    cow.set_pos(Vec3d { x: 12.5, ..START });
    world.level_mut(DIMENSION).entities.attach(cow);
    world.set_player_position_for_test(
        PLAYER,
        Vec3d {
            y: start_y,
            ..START
        },
    );
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.set_on_ground(false);
    player.known_movement = Vec3d::default();
    player.block_contact.fall_distance = distance;
    assert!(player.awaiting_teleport.is_none());
    assert_eq!(player.first_good, player.position());
    assert_eq!(player.last_good, player.position());
    assert_eq!(player.damage.health, 20.0);
    world
}

fn packet(world: &mut ServerWorld, target: Vec3d, on_ground: bool) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    // Drive the real packet dispatch without a later body tick obscuring this branch's effects.
    world.apply_command(
        crate::world::WorldCommand::Play {
            connection_id: PLAYER,
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(target),
                rotation: None,
                on_ground,
                horizontal_collision: false,
            }),
        },
        &mut events,
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, WorldEvent::Close { .. })));
    events
}

fn correction(events: &[WorldEvent]) -> Option<Vec3d> {
    events.iter().find_map(|event| match event {
        WorldEvent::Send {
            connection_id: PLAYER,
            packet: ClientboundPlayPacket::PlayerPosition(update),
        } => Some(update.position),
        _ => None,
    })
}

fn hit_ground_count(world: &ServerWorld) -> usize {
    let entity_id = world.levels[PLAYER].entity_id();
    world
        .game_events(DIMENSION)
        .0
        .iter()
        .filter(|event| {
            event.game_event == VanillaGameEvent::HIT_GROUND
                && event.source_entity_id == Some(entity_id)
        })
        .count()
}

#[test]
fn ground_history_packet_tests_accepted_known_ground_clears_zero_and_positive_history() {
    // ServerGamePacketListenerImpl.java:1158-1162; LivingEntity.java:371-374.
    for (start_y, distance, damage, hits) in [(65.0, 0.0, 0.0, 0), (65.25, 4.0, 1.0, 1)] {
        let mut world = fixture(start_y, distance);
        let before_hits = hit_ground_count(&world);
        let target = Vec3d { y: 65.0, ..START };
        let events = packet(&mut world, target, true);
        assert_eq!(correction(&events), None);
        let player = world.levels.player_at(PLAYER);
        assert_eq!(player.position(), target);
        assert_eq!(player.last_good, target);
        assert!(player.on_ground());
        assert!(player.awaiting_teleport.is_none());
        assert_eq!(player.block_contact.last_climbable_pos, None);
        assert_eq!(player.block_contact.fall_distance, 0.0);
        assert_eq!(player.damage.health, 20.0 - damage);
        assert_eq!(hit_ground_count(&world) - before_hits, hits);
        if hits != 0 {
            assert_eq!(
                world
                    .game_events(DIMENSION)
                    .0
                    .last()
                    .unwrap()
                    .affected_state,
                Some(state("minecraft:stone"))
            );
        }
    }
}

#[test]
fn ground_history_packet_tests_accepted_airborne_negative_query_retains_history() {
    let mut world = fixture(65.25, 4.0);
    let before_hits = hit_ground_count(&world);
    let target = Vec3d { y: 65.125, ..START };
    let events = packet(&mut world, target, false);
    assert_eq!(correction(&events), None);
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.position(), target);
    assert_eq!(player.last_good, target);
    assert!(!player.on_ground());
    assert_eq!(player.block_contact.last_climbable_pos, Some(OLD));
    assert_eq!(player.block_contact.fall_distance, 4.125);
    assert_eq!(player.damage.health, 20.0);
    assert_eq!(hit_ground_count(&world), before_hits);
}

#[test]
fn ground_history_packet_tests_too_quickly_and_awaiting_teleport_ignore_keep_history() {
    let mut world = fixture(65.25, 4.0);
    let before_hits = hit_ground_count(&world);
    let events = packet(
        &mut world,
        Vec3d {
            x: START.x + 200.0,
            ..START
        },
        true,
    );
    assert_eq!(correction(&events), Some(START));
    let pending = world.levels[PLAYER].awaiting_teleport;
    assert!(pending.is_some());
    assert_eq!(world.levels[PLAYER].position(), START);
    assert_eq!(
        world.levels[PLAYER].block_contact.last_climbable_pos,
        Some(OLD)
    );
    assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 4.0);
    let events = packet(&mut world, Vec3d { y: 65.0, ..START }, true);
    assert_eq!(correction(&events), None);
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.awaiting_teleport, pending);
    assert_eq!(player.position(), START);
    assert_eq!(player.received_move_packets, 1);
    assert_eq!(player.block_contact.last_climbable_pos, Some(OLD));
    assert_eq!(player.block_contact.fall_distance, 4.0);
    assert_eq!(player.damage.health, 20.0);
    assert_eq!(hit_ground_count(&world), before_hits);
}

#[test]
fn ground_history_packet_tests_moved_wrongly_corrected_ground_check_clears_history() {
    // ServerGamePacketListenerImpl.java:1180-1183: correction still performs a zero-delta fall check.
    let mut world = fixture(65.25, 4.0);
    let before_hits = hit_ground_count(&world);
    let events = packet(&mut world, Vec3d { y: 64.9, ..START }, true);
    assert_eq!(correction(&events), Some(START));
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.position(), START);
    assert_eq!(player.last_good, START);
    assert!(player.awaiting_teleport.is_some());
    assert!(!player.on_ground());
    assert_eq!(player.block_contact.last_climbable_pos, None);
    assert_eq!(player.block_contact.fall_distance, 0.0);
    assert_eq!(player.damage.health, 19.0);
    assert_eq!(hit_ground_count(&world) - before_hits, 1);
}

fn fall_advancement_fixture() -> ServerWorld {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
    use serde_json::json;
    let mut world = fixture(67.0, 0.0);
    let mut startup = (**world.cold.config.datapacks.as_ref().unwrap()).clone();
    let matched = json!({
        "start_position":{"position":{"y":{"min":66.5}}},
        "distance":{"y":{"min":1.5}},
        "player":{"type":"minecraft:player","distance":{"absolute":{"max":0}},
            "movement":{"fall_distance":{"min":1.5}},
            "location":{"position":{"y":{"max":65.0}}}}
    });
    let mut wrong_start = matched.clone();
    wrong_start["start_position"] = json!({"position":{"y":{"max":65.0}}});
    let mut wrong_distance = matched.clone();
    wrong_distance["distance"] = json!({"y":{"min":2.0}});
    let mut wrong_player = matched.clone();
    wrong_player["player"] = json!({"type":"minecraft:zombie"});
    startup.advancements = AdvancementSet {
        advancements: [
            ("test:fall", matched),
            ("test:wrong_distance", wrong_distance),
            ("test:wrong_player", wrong_player),
            ("test:wrong_start", wrong_start),
        ].into_iter().map(|(id, conditions)| AdvancementElement {
            id: id.into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&json!({
                "display":{"icon":{"id":"minecraft:stone"},"title":"Fall","description":"Fall"},
                "criteria":{"fall":{"trigger":"minecraft:fall_from_height","conditions":conditions}},
                "rewards":{"experience":7}
            })).unwrap(),
        }).collect(),
        warnings: Vec::new(),
    };
    world.cold.config.datapacks = Some(std::sync::Arc::new(startup));
    world.reload_player_advancement_stores();
    world.tick_player_advancement_flush(&mut Vec::new());
    packet(&mut world, Vec3d { y: 66.75, ..START }, false);
    world.tick_player_advancement_criteria(&mut Vec::new());
    packet(&mut world, Vec3d { y: 66.0, ..START }, false);
    packet(&mut world, Vec3d { y: 65.25, ..START }, false);
    assert!(!world.player_advancement_is_done(PLAYER, "test:fall"));
    assert_eq!(world.levels[PLAYER].experience.total(), 0);
    world
}

#[test]
fn fall_advancement_packet_landing_matches_start_distance_and_player_once() {
    let mut world = fall_advancement_fixture();
    let mut events = packet(&mut world, Vec3d { y: 65.0, ..START }, true);
    assert!(world.player_advancement_is_done(PLAYER, "test:fall"));
    for id in [
        "test:wrong_distance",
        "test:wrong_player",
        "test:wrong_start",
    ] {
        assert!(!world.player_advancement_is_done(PLAYER, id));
    }
    assert_eq!(world.levels[PLAYER].experience.total(), 7);
    world.tick_player_experience(&mut events);
    world.tick_player_advancement_flush(&mut events);
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: PLAYER, packet: ClientboundPlayPacket::SetExperience(xp) }
        if xp.total == 7
    )));
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: PLAYER, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
        if update.progress.iter().any(|progress| progress.id == "test:fall")
    )));
    events.clear();
    packet(&mut world, Vec3d { y: 65.0, ..START }, true);
    world.tick_player_advancement_flush(&mut events);
    assert_eq!(world.levels[PLAYER].experience.total(), 7);
    assert!(!events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::UpdateAdvancements(_),
            ..
        }
    )));
}

#[test]
fn fall_advancement_environment_resets_award_alive_player_without_landing() {
    for (alive, reset) in [
        (true, "water"),
        (false, "water"),
        (true, "flight"),
        (true, "slow_falling"),
        (true, "ladder"),
    ] {
        let mut world = fall_advancement_fixture();
        if !alive {
            world.levels[PLAYER].damage.health = 0.0;
        }
        let target = Vec3d { y: 65.0, ..START };
        packet(&mut world, target, false);
        assert!(!world.player_advancement_is_done(PLAYER, "test:fall"));
        let mut events = Vec::new();
        match reset {
            "flight" => world.levels[PLAYER].abilities.flying = true,
            "slow_falling" => {
                let effect =
                    crate::entity::ActiveMobEffect::new("minecraft:slow_falling", 200, 0).unwrap();
                world
                    .add_player_mob_effect(PLAYER, effect, &mut events)
                    .unwrap();
            }
            "ladder" => {
                write(
                    &mut world,
                    BlockPos { x: 8, y: 65, z: 9 },
                    "minecraft:stone",
                );
                write(
                    &mut world,
                    BlockPos { x: 8, y: 65, z: 8 },
                    "minecraft:ladder",
                );
            }
            "water" => {
                write(
                    &mut world,
                    BlockPos { x: 8, y: 65, z: 8 },
                    "minecraft:water",
                );
                assert!(world
                    .refresh_player_fluid_contact(PLAYER, &mut events)
                    .is_some());
                assert!(world.levels[PLAYER].was_touching_water);
            }
            _ => unreachable!(),
        }
        world.tick_player_advancement_criteria(&mut events);
        assert!(!world.levels[PLAYER].on_ground());
        assert_eq!(world.player_advancement_is_done(PLAYER, "test:fall"), alive);
        assert_eq!(
            world.levels[PLAYER].experience.total(),
            if alive { 7 } else { 0 }
        );
        world.refresh_player_fluid_contact(PLAYER, &mut events);
        assert_eq!(
            world.levels[PLAYER].experience.total(),
            if alive { 7 } else { 0 }
        );
    }
}
