use super::*;
use crate::entity::{EntityIdCounter, ItemEntityData};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID;
use bbb_protocol::packets::Vec3d;

fn item(store: &mut EntityStore, name: &str) -> i32 {
    store.spawn(
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        EntityData::Item(ItemEntityData::new(
            crate::entity::item::plain_stack(name, 1).unwrap(),
            0.0,
        )),
    )
}

fn flags(store: &EntityStore, id: i32) -> LootEntityFlags {
    LootEntity::Stored(StoredLootEntity {
        entity: store.get(id).unwrap(),
        bodies: Some(crate::entity::store::EntityBodyReadView::new(
            store, None, None,
        )),
        raid_at_position: None,
    })
    .flags()
}

#[test]
fn a_fire_resistant_item_is_not_on_fire() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    store.set_item_publication(crate::entity::ItemPublication::new(std::sync::Arc::new(
        crate::entity::test_items::BuiltinItems,
    )));
    let resistant = item(&mut store, "minecraft:netherite_sword");
    store
        .get_mut(resistant)
        .unwrap()
        .set_remaining_fire_ticks(40);
    let plain = item(&mut store, "minecraft:stone");
    store.get_mut(plain).unwrap().set_remaining_fire_ticks(40);
    assert_eq!(flags(&store, resistant).on_fire, Some(false));
    assert_eq!(flags(&store, resistant).on_ground, Some(false));
    assert_eq!(flags(&store, plain).on_fire, Some(true));
}
