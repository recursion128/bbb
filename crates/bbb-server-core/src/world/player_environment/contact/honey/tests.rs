use super::super::super::tests::{block, world};
use super::*;
use crate::connection::ConnectionId;
use crate::entity::living::FallImpulseContext;
use crate::entity::{BlockEffectCollector, BodyBlockContact, EnvironmentBlockKind};
use crate::world::{ServerWorld, WorldEvent};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{SoundEventHolder, SoundSource, Vec3d};

const PLAYER: ConnectionId = ConnectionId(1);

fn prepare(world: &mut ServerWorld) -> BlockPos {
    let pos = crate::world::block_position(world.levels[PLAYER].position());
    block(world, pos, "minecraft:honey_block");
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.set_position(Vec3d {
        x: f64::from(pos.x) - 0.25,
        y: f64::from(pos.y) + 0.2,
        z: f64::from(pos.z) + 0.5,
    });
    player.block_contact.old_position = player.position();
    player.set_on_ground(false);
    player.set_delta_movement(Vec3d {
        x: 0.2,
        y: -0.3,
        z: 0.4,
    });
    player.block_contact.fall_distance = 9.0;
    pos
}

fn slide(world: &mut ServerWorld, pos: BlockPos) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    assert_eq!(
        apply(
            &mut SessionContact {
                world,
                id: PLAYER,
                events: &mut events,
                collector: BlockEffectCollector::default(),
            },
            pos
        ),
        Some(())
    );
    events
}

#[test]
fn player_environment_honey_stationary_contact_resets_distance_without_impulse() {
    for mode in [GameType::Survival, GameType::Creative] {
        for flying in [false, true] {
            for grace in [0, 4] {
                let mut world = world(mode);
                prepare(&mut world);
                let player = world.levels.player_mut(PLAYER).unwrap();
                player.abilities.flying = flying;
                let impulse = FallImpulseContext {
                    reset_grace_time: grace,
                    impact_position: Some(player.position()),
                };
                player.cold.fall_impulse = impulse;
                world.tick_player_block_contacts(&mut Vec::new());
                let player = world.levels.player_at(PLAYER);
                assert_eq!(player.block_contact.fall_distance, 0.0);
                assert_eq!(player.cold.fall_impulse.reset_grace_time, grace);
                assert_eq!(
                    player.cold.fall_impulse.impact_position,
                    impulse.impact_position
                );
                assert_eq!(
                    player.delta_movement().y,
                    (-0.05 - 0.08) * f64::from(0.98_f32)
                );
                let factor = -0.05 / (-0.3 / f64::from(0.98_f32) + 0.08);
                assert_eq!(player.delta_movement().x, 0.2 * factor);
                assert_eq!(player.delta_movement().z, 0.4 * factor);
            }
        }
    }
}

#[test]
fn player_environment_honey_uses_current_pose_width_before_drawing_effects() {
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    };
    for (pose, slides) in [
        (VANILLA_ENTITY_POSE_STANDING_ID, false),
        (VANILLA_ENTITY_POSE_SLEEPING_ID, true),
    ] {
        let mut world = world(GameType::Survival);
        let pos = prepare(&mut world);
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.pose_id = pose;
        {
            let mut position = player.position();
            position.x = f64::from(pos.x) - 0.1;
            player.set_position(position);
        }
        slide(&mut world, pos);
        assert_eq!(
            world.levels[PLAYER].block_contact.fall_distance == 0.0,
            slides
        );
    }
}

#[test]
fn player_environment_honey_swept_move_slides_after_leaving_the_block() {
    use bbb_protocol::packets::MovePlayer;
    let mut world = world(GameType::Survival);
    let pos = prepare(&mut world);
    let player = world.levels.player_mut(PLAYER).unwrap();
    {
        let mut position = player.position();
        position.z = f64::from(pos.z) - 1.0;
        player.set_position(position);
    }
    player.last_good = player.position();
    player.block_contact.old_position = player.position();
    let target = Vec3d {
        z: f64::from(pos.z) + 2.0,
        ..player.position()
    };
    let movement = MovePlayer {
        position: Some(target),
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
            probe,
        ),
        crate::world::player_packets::MovementOutcome::Accepted
    ));
    world.tick_player_block_contacts(&mut Vec::new());
    let player = world.levels.player_at(PLAYER);
    assert_eq!(player.position(), target);
    assert_eq!(player.block_contact.fall_distance, 0.0);
    assert_eq!(
        player.delta_movement().y,
        (-0.05 - 0.08) * f64::from(0.98_f32)
    );
}

#[test]
fn player_environment_honey_effects_roll_separately_and_sound_excludes_self() {
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        game_type: GameType::Survival,
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    {
        let mut position = world.levels[PLAYER].position();
        position.y = 80.0;
        world.levels[PLAYER].set_position(position);
    }
    // Sound and particles are separate `nextInt(5) == 0` rolls: about 40 of
    // 200 slides each (sd ~5.7).
    let (mut sounds, mut particles) = (0, 0);
    let start = world.levels[PLAYER].position();
    for _ in 0..200 {
        world.levels[PLAYER].set_position(start);
        let pos = prepare(&mut world);
        let position = world.levels[PLAYER].position();
        world.levels[ConnectionId(2)].set_position(position);
        world.reconcile_player_tracking_for_test(PLAYER);
        world.reconcile_player_tracking_for_test(ConnectionId(2));
        let entity_id = world.levels[PLAYER].entity_id();
        let events = slide(&mut world, pos);
        let mut observed = Vec::new();
        for output in events {
            match output {
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::Sound(packet),
                } => {
                    assert_eq!(connection_id, ConnectionId(2));
                    assert_eq!(packet.source, SoundSource::Players);
                    assert_eq!(packet.position, position);
                    assert_eq!((packet.volume, packet.pitch), (1.0, 1.0));
                    assert_eq!(
                        packet.sound,
                        SoundEventHolder::Reference {
                            registry_id:
                                bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                                    "minecraft:block.honey_block.slide"
                                )
                                .unwrap(),
                        }
                    );
                    observed.push((0, connection_id));
                }
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::EntityEvent(packet),
                } => {
                    assert_eq!(packet.entity_id, entity_id);
                    assert_eq!(packet.event_id, 53);
                    observed.push((1, connection_id));
                }
                other => panic!("unexpected honey output: {other:?}"),
            }
        }
        let sound = observed.iter().any(|(kind, _)| *kind == 0);
        let event = observed.iter().any(|(kind, _)| *kind == 1);
        sounds += usize::from(sound);
        particles += usize::from(event);
        let mut wanted = Vec::new();
        if sound {
            wanted.push((0, ConnectionId(2)));
        }
        if event {
            let mut recipients: Vec<_> = observed
                .iter()
                .filter_map(|(kind, id)| (*kind == 1).then_some(*id))
                .collect();
            recipients.sort_by_key(|id| id.0);
            assert_eq!(recipients, [PLAYER, ConnectionId(2)]);
            wanted.extend(observed.iter().copied().filter(|(kind, _)| *kind == 1));
        }
        assert_eq!(observed, wanted);
    }
    assert!((15..=70).contains(&sounds), "{sounds}");
    assert!((15..=70).contains(&particles), "{particles}");
}

fn criterion_world(block_name: &str, rewards: serde_json::Value) -> ServerWorld {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    startup.advancements.advancements = vec![AdvancementElement {
        id: "test:slide".to_owned(),
        source_pack: "test".to_owned(),
        advancement: parse_advancement(&serde_json::json!({
            "criteria": { "slide": {
                "trigger": "minecraft:slide_down_block",
                "conditions": { "block": block_name }
            }},
            "rewards": rewards
        }))
        .unwrap(),
    }];
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        datapacks: Some(std::sync::Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    {
        let mut position = world.levels[PLAYER].position();
        position.y = 80.0;
        world.levels[PLAYER].set_position(position);
    }
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world
}

#[test]
fn player_environment_honey_criterion_uses_live_state_only_every_twenty_ticks() {
    let mut world = criterion_world("minecraft:stone", serde_json::json!({}));
    let pos = prepare(&mut world);
    let dimension = world.levels[PLAYER].dimension;
    let state = world.level_by_id(&dimension).block_state(pos);
    block(&mut world, pos, "minecraft:stone");
    for tick in [19, 20] {
        world.game_time = tick;
        {
            let mut delta = world.levels[PLAYER].delta_movement();
            delta.y = -0.3;
            world.levels[PLAYER].set_delta_movement(delta);
        }
        let mut events = Vec::new();
        let mut contact = SessionContact {
            world: &mut world,
            id: PLAYER,
            events: &mut events,
            collector: BlockEffectCollector::default(),
        };
        assert!(contact.visit_contact(BodyBlockContact {
            pos,
            state,
            environment_kind: EnvironmentBlockKind::Other,
            resource_id: "minecraft:honey_block",
            step: 0,
            precise: true,
            inside_block: true,
            fluid: None,
        }));
        assert_eq!(
            world.player_advancement_is_done(PLAYER, "test:slide"),
            tick == 20
        );
    }
}
