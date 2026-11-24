use super::*;
use crate::tests::{join_loaded, mailbox, operator_config};
use crate::world::ConnectionId;

#[test]
fn give_native_animation_and_leftover_retain_first_components() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    let connection = ConnectionId(1);
    let mut events = Vec::new();
    assert_eq!(world.apply_chat_command(connection,
        "item replace entity op weapon.mainhand with stone[lock={predicates:{'minecraft:max_damage':{}}}]",
        &mut events), None);
    let prototype = world.levels[connection].inventory.selected_stack().clone();
    let level = world.levels[connection].dimension;
    world
        .give_native_items_to_player(connection, &prototype, 1, &mut events)
        .unwrap();
    let fake = world
        .level_by_id(&level)
        .entities
        .ids()
        .into_iter()
        .find_map(|id| {
            let entity = world.level_by_id(&level).entities.get(id)?;
            let crate::entity::EntityData::Item(data) = entity.data() else {
                return None;
            };
            (data.pickup_delay() == crate::entity::item::INFINITE_PICKUP_DELAY)
                .then(|| data.item().clone())
        })
        .expect("successful inventory add creates the fake original copy");
    assert_eq!(fake.count(), 1);
    assert!(fake.same_item_same_components(&prototype));
    world.set_player_game_mode(
        connection,
        bbb_protocol::game_types::GameType::Survival,
        &mut events,
    );
    for index in 0..36 {
        assert!(world.levels[connection]
            .inventory
            .set_inventory_slot(index, prototype.with_count(64)));
    }
    world
        .give_native_items_to_player(connection, &prototype, 1, &mut events)
        .unwrap();
    let leftover = world
        .level_by_id(&level)
        .entities
        .ids()
        .into_iter()
        .find_map(|id| {
            let entity = world.level_by_id(&level).entities.get(id)?;
            let crate::entity::EntityData::Item(data) = entity.data() else {
                return None;
            };
            (data.pickup_delay() == 0
                && data.target() == Some(world.levels[connection].profile.uuid))
            .then(|| data.item().clone())
        })
        .expect("full inventory publishes the original leftover with its pickup target");
    assert_eq!(leftover.count(), 1);
    assert!(leftover.same_item_same_components(&prototype));
}
