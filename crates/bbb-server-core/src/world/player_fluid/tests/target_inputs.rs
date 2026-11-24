use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DROWNED_ID;

#[test]
fn daytime_drowned_target_uses_real_session_water_contact_and_keeps_dry_negative() {
    for immersed in [false, true] {
        let mut world = world();
        let connection = ConnectionId(1);
        let dimension = world.levels[connection].dimension;
        let position = world.levels[connection].position();
        let uuid = world.levels[connection].profile.uuid;
        let stone =
            bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:stone")
                .unwrap();
        let stone =
            bbb_protocol::block_state_types::vanilla_block_default_state_id(stone.id).unwrap();
        let air =
            bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:air").unwrap();
        let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(air.id).unwrap();
        for x in position.x.floor() as i32 - 2..=position.x.floor() as i32 + 5 {
            for z in position.z.floor() as i32 - 2..=position.z.floor() as i32 + 2 {
                for y in 79..=84 {
                    assert!(world
                        .level_mut_by_id(&dimension)
                        .chunks
                        .set_block_state(x, y, z, if y == 79 { stone } else { air })
                        .is_some());
                }
            }
        }
        if immersed {
            pool(&mut world, 2);
        }
        world.tick_player_fluid_contact(&mut Vec::new());
        assert_eq!(
            world.levels[connection]
                .fluid_interaction
                .is_in_fluid(FluidTag::Water),
            immersed
        );
        assert_eq!(
            world.nearby_players_by_key(&dimension)[0].in_water,
            immersed
        );
        assert_eq!(world.is_bright_outside(&dimension), Some(true));
        let mob = world
            .level_mut_by_id(&dimension)
            .entities
            .spawn_general_living(
                VANILLA_ENTITY_TYPE_DROWNED_ID,
                Vec3d {
                    x: position.x + 3.0,
                    ..position
                },
            )
            .unwrap();
        let mut events = Vec::new();
        for _ in 0..64 {
            world.tick_level_entities(&dimension, true, &mut events);
            if world
                .level_by_id(&dimension)
                .entities
                .mob_target(mob)
                .is_some()
            {
                break;
            }
        }
        assert!(
            world
                .level_by_id(&dimension)
                .entities
                .get(mob)
                .unwrap()
                .tick_count()
                > 0
        );
        assert_eq!(
            world.level_by_id(&dimension).entities.mob_target(mob),
            immersed.then_some(uuid)
        );
    }
}
