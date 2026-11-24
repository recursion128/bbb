use super::*;
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::Entity;

fn item_fire_body(stack: RuntimeItemStack) -> Entity {
    Entity::new(
        31,
        Uuid::from_u128(31),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        FEET,
        EntityData::Item(crate::entity::ItemEntityData::new(stack, 0.0)),
    )
}

#[test]
fn removing_damage_resistant_lets_a_netherite_item_burn() {
    let raw = bbb_protocol::NbtSummaryValue::Compound(vec![
        bbb_protocol::NbtSummaryEntry {
            name: "id".into(),
            value: bbb_protocol::NbtSummaryValue::String("minecraft:netherite_sword".into()),
        },
        bbb_protocol::NbtSummaryEntry {
            name: "count".into(),
            value: bbb_protocol::NbtSummaryValue::Int(1),
        },
        bbb_protocol::NbtSummaryEntry {
            name: "components".into(),
            value: bbb_protocol::NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                name: "!minecraft:damage_resistant".into(),
                value: bbb_protocol::NbtSummaryValue::Compound(vec![]),
            }]),
        },
    ]);
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &raw,
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap();
    let mut body = item_fire_body(stack);
    let mut level = NativeLevel::new();

    assert!(!level.with_level(|ctx| body.fire_immune_in(ctx)).unwrap());
    assert!(!level.with_level(|ctx| body.is_on_fire_in(ctx)).unwrap());
    body.set_remaining_fire_ticks(40);
    assert!(level.with_level(|ctx| body.is_on_fire_in(ctx)).unwrap());
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    assert!(
        level.with_level(|ctx| crate::entity::damage::base_tick_from_environment(
            &mut body, &mut store, ctx
        ))
    );
    let EntityData::Item(item) = body.data() else {
        panic!()
    };
    assert_eq!(item.health(), 4);
    assert_eq!(body.remaining_fire_ticks(), 39);
}
