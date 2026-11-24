use super::*;

#[test]
fn a_climbing_session_source_still_pushes_its_target_but_takes_no_impulse() {
    for flying in [false, true] {
        let (mut native, _, receiver) = session_level(4.8, GameType::Creative);
        let mut abilities =
            bbb_protocol::packets::PlayerAbilities::for_game_type(GameType::Creative);
        abilities.flying = flying;
        native.world.set_player_mode_and_abilities_for_test(
            ConnectionId(1),
            GameType::Creative,
            abilities,
        );
        native.world.set_block(
            DIMENSION,
            BlockPos { x: 4, y: 64, z: 9 },
            state("minecraft:stone"),
            2 | 16,
        );
        native.world.set_block(
            DIMENSION,
            BlockPos { x: 4, y: 64, z: 8 },
            state("minecraft:ladder"),
            2 | 16,
        );
        let target = spawn_world_mob(&mut native, 2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, position(5.1));
        native.world.tick(receiver.drain());
        assert!(world_delta(&native, target).x > 0.0);
        if flying {
            assert!(player_delta(&native).x < 0.0);
        } else {
            assert_eq!(player_delta(&native).x, 0.0);
        }
    }
}
