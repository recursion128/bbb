use super::*;
use crate::tests::{join_loaded, mailbox, operator_config};

#[test]
fn relative_teleport_keeps_gliders_airborne_and_grounds_other_players() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let connection = ConnectionId(1);
    let velocity = Vec3d {
        x: 0.25,
        y: -0.5,
        z: 0.125,
    };
    let mut events = Vec::new();
    for (gliding, command, dimension) in [
        (false, "tp @s ~1 ~2 ~", Java26BuiltinDimension::Overworld),
        (true, "tp @s ~1 ~2 ~", Java26BuiltinDimension::Overworld),
        (
            true,
            "execute in minecraft:the_nether run tp @s ~1 ~2 ~",
            Java26BuiltinDimension::Nether,
        ),
    ] {
        let player = &mut world.levels[connection];
        player.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, gliding);
        player.set_delta_movement(velocity);
        player.set_on_ground(false);
        let old_y = player.position().y;
        assert_eq!(
            world.apply_chat_command(connection, command, &mut events),
            None
        );
        let player = &world.levels[connection];
        assert_eq!(player.position().y, old_y + 2.0);
        assert_eq!(world.player_dimension(connection), dimension);
        assert_eq!(player.on_ground(), !gliding);
        assert_eq!(
            player.delta_movement(),
            Vec3d {
                y: if gliding { velocity.y } else { 0.0 },
                ..velocity
            }
        );
    }
}
