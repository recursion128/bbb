use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
use bbb_server_types::food::FoodData;

use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::{ServerWorldConfig, WorldEvent};
use bbb_server_persistence::RegionCompression;

fn world(difficulty: Difficulty, game_type: GameType) -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig {
        difficulty,
        game_type,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world
}

fn prepare(world: &mut ServerWorld, tick_count: i32, health: f32, food: FoodData) {
    let player = world.levels.player_mut(ConnectionId(1)).unwrap();
    player.tick_count = tick_count;
    player.damage.health = health;
    player.food = food;
}

fn state(world: &ServerWorld) -> (f32, FoodData) {
    let player = world.levels.player_at(ConnectionId(1));
    (player.damage.health, player.food)
}

#[test]
fn player_regeneration_uses_entity_tick_periods_without_clamping_saturation() {
    let mut world = world(Difficulty::Peaceful, GameType::Survival);
    prepare(&mut world, 9, 18.5, FoodData::from_saved(10, 7, 19.5, 2.0));
    let before = state(&world);
    world.tick_player_regeneration();
    assert_eq!(state(&world), before);

    world.levels[ConnectionId(1)].tick_count = 10;
    world.tick_player_regeneration();
    assert_eq!(
        state(&world),
        (18.5, FoodData::from_saved(11, 7, 19.5, 2.0))
    );

    world.levels[ConnectionId(1)].tick_count = 20;
    world.tick_player_regeneration();
    assert_eq!(
        state(&world),
        (19.5, FoodData::from_saved(12, 7, 20.5, 2.0))
    );
    world.levels[ConnectionId(1)].tick_count = 40;
    world.tick_player_regeneration();
    assert_eq!(
        state(&world),
        (20.0, FoodData::from_saved(13, 7, 20.5, 2.0))
    );
}

#[test]
fn player_regeneration_obeys_difficulty_and_rule_but_not_invulnerable_abilities() {
    for game_type in [GameType::Survival, GameType::Creative, GameType::Spectator] {
        let mut world = world(Difficulty::Peaceful, game_type);
        prepare(&mut world, 20, 18.0, FoodData::from_saved(10, 0, 0.0, 0.0));
        world.tick_player_regeneration();
        assert_eq!(state(&world), (19.0, FoodData::from_saved(11, 0, 1.0, 0.0)));
        assert!(world.admin.set_game_rule(
            "minecraft:natural_health_regeneration",
            GameRuleValue::Boolean(false),
        ));
        let before = state(&world);
        world.tick_player_regeneration();
        assert_eq!(state(&world), before);
    }
    for difficulty in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
        let mut world = world(difficulty, GameType::Survival);
        prepare(&mut world, 20, 18.0, FoodData::from_saved(10, 0, 0.0, 0.0));
        let before = state(&world);
        world.tick_player_regeneration();
        assert_eq!(state(&world), before);
    }
}

#[test]
fn player_regeneration_does_not_revive_or_tick_removed_avatars() {
    let mut world = world(Difficulty::Peaceful, GameType::Survival);
    prepare(&mut world, 20, 0.0, FoodData::from_saved(10, 0, 0.0, 0.0));
    world.tick_player_regeneration();
    assert_eq!(state(&world), (0.0, FoodData::from_saved(11, 0, 1.0, 0.0)));
    world.levels[ConnectionId(1)].avatar_removed = true;
    let before = state(&world);
    world.tick_player_regeneration();
    assert_eq!(state(&world), before);
}

#[test]
fn player_regeneration_precedes_food_tick_and_reaches_health_packet_and_save() {
    let mut world = world(Difficulty::Peaceful, GameType::Survival);
    prepare(&mut world, 19, 18.0, FoodData::from_saved(20, 9, 0.0, 0.0));
    let tick = world.tick(Vec::new());
    let (health, food) = state(&world);
    assert_eq!(health, 19.0 + 1.0 / 6.0);
    assert_eq!(food, FoodData::from_saved(20, 0, 1.0, 1.0));
    assert!(tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetHealth(packet) }
            if packet.health == health && packet.food == 20 && packet.saturation == 1.0
    )));

    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    assert_eq!(parsed.food_data(), food);
    assert_eq!(parsed.damage(20.0).health, health);
}
