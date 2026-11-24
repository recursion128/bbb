use super::*;
use bbb_server_types::food::FoodData;

struct Player {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    teleport_id: i32,
    spawn: Vec3d,
}

impl Player {
    fn new(game_type: GameType) -> Self {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type,
            ..ServerWorldConfig::default()
        })
        .unwrap();
        enqueue(
            &sender,
            WorldCommand::Join {
                client_information: Default::default(),
                context: JoinContext::default(),
                connection_id: ConnectionId(1),
                profile: profile("alice", 1),
            },
        )
        .unwrap();
        let joined = world.tick(receiver.drain());
        let (teleport_id, spawn) = joined
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(position),
                    ..
                } => Some((position.id, position.position)),
                _ => None,
            })
            .unwrap();
        Self {
            world,
            sender,
            receiver,
            teleport_id,
            spawn,
        }
    }

    fn packet(&mut self, packet: PlayServerbound) {
        play(&self.sender, &self.receiver, &mut self.world, 1, packet);
    }

    fn load(&mut self) {
        self.packet(PlayServerbound::AcceptTeleportation {
            id: self.teleport_id,
        });
        self.packet(PlayServerbound::PlayerLoaded);
    }

    /// A move that only rises or falls, from the position the join put the
    /// session at: `handleMovePlayer` refuses a step past the tick budget
    /// (`:1103`), so the horizontal part stays where the server has it.
    fn move_to(&mut self, y: f64, on_ground: bool) {
        self.packet(PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: self.spawn.x,
                y,
                z: self.spawn.z,
            }),
            rotation: None,
            on_ground,
            horizontal_collision: false,
        }));
    }

    fn sprint(&mut self) {
        self.packet(PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: entity_id_of(&self.world, 1),
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }));
    }

    fn jump(&mut self) {
        self.move_to(64.0, true);
        self.move_to(64.5, false);
    }

    fn jumps(&self) -> i32 {
        self.world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:jump"),
        )
    }

    fn food(&mut self) -> FoodData {
        self.world
            .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
            .unwrap()
            .player_patches[0]
            .created
            .food
    }
}

#[test]
fn player_jump_exhaustion_uses_sprinting_and_invulnerable_abilities() {
    for game_type in [GameType::Survival, GameType::Creative, GameType::Spectator] {
        for sprinting in [false, true] {
            let mut player = Player::new(game_type);
            player.load();
            if sprinting {
                player.sprint();
            }
            player.jump();
            let expected = if game_type == GameType::Survival {
                if sprinting {
                    0.2
                } else {
                    0.05
                }
            } else {
                0.0
            };
            // `Player.tick` (Player.java:233-235) takes a spectator off the
            // ground between the two packets, so its rise is no jump.
            let jumps = i32::from(game_type != GameType::Spectator);
            assert_eq!(player.jumps(), jumps, "{game_type:?}, sprint={sprinting}");
            assert_eq!(
                player.food().exhaustion_level(),
                expected,
                "{game_type:?}, sprint={sprinting}"
            );
        }
    }
}

#[test]
fn player_jump_exhaustion_requires_rising_ground_departure_once() {
    let mut player = Player::new(GameType::Survival);
    player.load();
    player.move_to(64.0, true);
    player.move_to(64.5, true);
    player.move_to(64.5, false);
    player.move_to(65.0, false);
    player.move_to(64.0, false);
    assert_eq!(player.jumps(), 0);
    assert_eq!(player.food().exhaustion_level(), 0.0);

    player.jump();
    player.move_to(65.0, false);
    player.move_to(65.0, false);
    player.move_to(64.0, false);
    assert_eq!(player.jumps(), 1);
    assert_eq!(player.food().exhaustion_level(), 0.05);
}

#[test]
fn player_jump_exhaustion_preserves_loaded_teleport_and_passenger_gates() {
    let mut player = Player::new(GameType::Survival);
    player.jump();
    assert_eq!(player.jumps(), 0);
    player.packet(PlayServerbound::PlayerLoaded);
    player.jump();
    assert_eq!(player.jumps(), 0);
    assert_eq!(player.food().exhaustion_level(), 0.0);

    player.packet(PlayServerbound::AcceptTeleportation {
        id: player.teleport_id,
    });
    player.move_to(64.0, true);
    player
        .world
        .set_external_player_last_vehicle_for_test(ConnectionId(1), Some(4242));
    player.move_to(64.5, false);
    assert_eq!(player.jumps(), 0);
    assert_eq!(player.food().exhaustion_level(), 0.0);
    player
        .world
        .set_external_player_last_vehicle_for_test(ConnectionId(1), None);
    player.jump();
    assert_eq!(player.jumps(), 1);
    assert_eq!(player.food().exhaustion_level(), 0.05);
}

#[test]
fn player_jump_exhaustion_reaches_food_tick_and_survives_player_save_restore() {
    let mut player = Player::new(GameType::Survival);
    player.load();
    player.sprint();
    for _ in 0..21 {
        player.jump();
    }
    assert_eq!(player.jumps(), 21);
    let food = player.food();
    assert_eq!(food.food_level(), 20);
    assert_eq!(food.saturation_level(), 4.0);
    assert!((food.exhaustion_level() - 0.2).abs() < 0.00001);

    let request = player
        .world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    let mut restored = ServerWorld::restore(
        ServerWorldConfig::default(),
        &restore_candidate(0, None, vec![(patch.uuid, parsed)]),
    )
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    let restored_food = restored
        .save_request(1_786_000_000_001, 2, RegionCompression::Zlib)
        .unwrap()
        .player_patches[0]
        .created
        .food;
    assert_eq!(restored_food, food);
}
