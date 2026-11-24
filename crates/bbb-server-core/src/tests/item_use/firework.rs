//! `FireworkRocketItem.use` and `useOn`: a rocket riding a gliding player,
//! and one launched off the clicked block.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID;
use bbb_protocol::packets::{BlockHitResult, BlockPos, Direction, ItemStackSummary, Vec3d};

use super::*;
use crate::entity::{Entity, EntityData};

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:firework_rocket")),
            count: 2,
            ..ItemStackSummary::empty()
        },
    );
    (sender, receiver, world)
}

fn rockets(world: &ServerWorld) -> Vec<Entity> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id))
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID)
        .cloned()
        .collect()
}

fn attached_to(rocket: &Entity) -> Option<i32> {
    match rocket.data() {
        EntityData::FireworkRocket(data) => data.attached_to_target(),
        _ => None,
    }
}

#[test]
fn a_gliding_player_launches_a_rocket_that_rides_along() {
    let (sender, receiver, mut world) = fixture();
    // Not gliding: `use` passes and nothing launches.
    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);
    assert!(rockets(&world).is_empty());

    world.set_player_fall_flying_for_test(ConnectionId(1), true);
    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 2);

    let launched = rockets(&world);
    assert_eq!(launched.len(), 1);
    assert_eq!(attached_to(&launched[0]), Some(entity_id_of(&world, 1)));
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);

    let moved = Vec3d {
        x: 3.5,
        y: 70.0,
        z: -2.5,
    };
    world.set_player_position_for_test(ConnectionId(1), moved);
    world.tick(receiver.drain());
    assert_eq!(rockets(&world)[0].position(), moved);
}

#[test]
fn a_rocket_used_on_a_block_launches_off_the_clicked_face() {
    let (sender, receiver, mut world) = fixture();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: BlockPos { x: 0, y: 63, z: 2 },
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );

    let launched = rockets(&world);
    assert_eq!(launched.len(), 1);
    assert_eq!(attached_to(&launched[0]), None);
    let position = launched[0].position();
    assert!((position.x - 0.5).abs() < 0.05 && (position.z - 2.5).abs() < 0.05);
    assert!(position.y > 64.1, "{position:?}");
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);
}
