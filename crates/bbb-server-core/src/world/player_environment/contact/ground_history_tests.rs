use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::entity::{EntityData, GeneralLivingData};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::packets::PlayerAbilities;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::stats::custom_stat;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const PLAYER: ConnectionId = ConnectionId(1);
const CLIMBED: BlockPos = BlockPos { x: 8, y: 65, z: 8 };
const SOURCE: Vec3d = Vec3d {
    x: 8.8,
    y: 65.25,
    z: 8.5,
};
const LANDING: Vec3d = Vec3d {
    x: 10.5,
    y: 65.0,
    z: 8.5,
};
const SUPPORT: BlockPos = BlockPos { x: 10, y: 64, z: 8 };

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn write(world: &mut ServerWorld, pos: BlockPos, name: &str) {
    assert!(world
        .level_mut(DIMENSION)
        .chunks
        .set_block_state(pos.x, pos.y, pos.z, state(name))
        .is_some());
}

fn history(world: &ServerWorld) -> Option<BlockPos> {
    world.levels[PLAYER].block_contact.last_climbable_pos
}

fn fixture() -> ServerWorld {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    for x in 5..=12 {
        for z in 6..=10 {
            for y in 63..=69 {
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
    write(&mut world, BlockPos { z: 9, ..CLIMBED }, "minecraft:stone");
    write(&mut world, CLIMBED, "minecraft:ladder");
    world.set_player_position_for_test(PLAYER, SOURCE);
    let player = world.levels.player_mut(PLAYER).unwrap();
    player.game_type = GameType::Survival;
    player.abilities = PlayerAbilities::for_game_type(GameType::Survival);
    player.set_delta_movement(Vec3d::default());
    player.needs_sync = false;
    assert_eq!(history(&world), None);
    let cow = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d { x: 9.1, ..SOURCE },
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
    assert_eq!(history(&world), Some(CLIMBED));
    let mut body = world.level_mut(DIMENSION).entities.detach(cow).unwrap();
    body.set_pos(Vec3d { x: 100.0, ..SOURCE });
    world.level_mut(DIMENSION).entities.attach(body);
    world.set_player_position_for_test(PLAYER, LANDING);
    assert_eq!(history(&world), Some(CLIMBED));
    world.level_mut(DIMENSION).event_sink.drain_game_events();
    world
}

fn distance_stat(world: &ServerWorld) -> i32 {
    world.levels[PLAYER]
        .cold
        .stats
        .value(custom_stat("minecraft:fall_one_cm"))
}

fn damage_packets(events: &[WorldEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DamageEvent(packet),
            } if *connection_id == PLAYER => Some(packet.source_type_id),
            _ => None,
        })
        .collect()
}

fn ground_events(world: &ServerWorld) -> usize {
    world
        .level(DIMENSION)
        .event_sink
        .recent_game_events()
        .filter(|event| event.game_event == VanillaGameEvent::HIT_GROUND)
        .count()
}

#[test]
fn ground_history_tests_flying_and_creative_known_callback_still_clear_history() {
    for game_type in [GameType::Survival, GameType::Creative] {
        let mut world = fixture();
        let player = world.levels.player_mut(PLAYER).unwrap();
        player.game_type = game_type;
        player.abilities = PlayerAbilities::for_game_type(game_type);
        if game_type == GameType::Survival {
            player.abilities.can_fly = true;
            player.abilities.flying = true;
        }
        player.block_contact.fall_distance = 7.0;
        let mut events = Vec::new();
        assert_eq!(
            world.player_block_fall_on(PLAYER, state("minecraft:stone"), SUPPORT, 7.0, &mut events),
            Some(())
        );
        assert_eq!(history(&world), Some(CLIMBED));
        assert_eq!(ground_events(&world), 0);
        world.check_player_fall_state(PLAYER, Vec3d::default(), true, &mut events);
        assert_eq!(history(&world), None);
        assert_eq!(world.levels[PLAYER].block_contact.fall_distance, 0.0);
        assert_eq!(world.levels[PLAYER].damage.health, 20.0);
        assert_eq!(distance_stat(&world), 0);
        assert!(damage_packets(&events).is_empty());
        assert_eq!(ground_events(&world), 1);
    }
}
