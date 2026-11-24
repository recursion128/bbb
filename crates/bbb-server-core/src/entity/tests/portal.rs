use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use bbb_server_types::save::Java26BuiltinDimension;

const ENTRY: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn portal_level() -> NativeLevel {
    let mut level = NativeLevel::new();
    level.world.place_portal_ticket(DIMENSION, ENTRY).unwrap();
    for (pos, block) in [
        (BlockPos { y: 63, ..ENTRY }, "minecraft:obsidian"),
        (ENTRY, "minecraft:nether_portal"),
    ] {
        assert!(level
            .world
            .set_block(DIMENSION, pos, state_id(block), 2 | 16));
    }
    level
}

#[test]
fn an_item_inside_a_portal_transfers_its_stack_and_uuid_to_the_nether() {
    let mut level = portal_level();
    let stack = stack_of("minecraft:stone", 1);
    let (id, uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        POSITION,
        EntityData::Item(ItemEntityData::new(stack.clone(), 0.0)),
    );
    for _ in 0..3 {
        level.world.tick(Vec::new());
    }
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    let nether = Java26BuiltinDimension::Nether;
    let replacement = level
        .world
        .stored_entity_ids_for_test(nether)
        .into_iter()
        .find(|id| {
            level
                .world
                .stored_entity_for_test(nether, *id)
                .unwrap()
                .uuid()
                == uuid
        })
        .expect("the item is in the Nether");
    let body = level
        .world
        .stored_entity_for_test(nether, replacement)
        .unwrap();
    assert!(body.portal_cooldown() > 0);
    assert_eq!(
        level
            .world
            .stored_item_stack_for_test(nether, replacement)
            .unwrap(),
        stack
    );
    let saved = level
        .world
        .stored_entity_command_data_for_test(nether, replacement)
        .unwrap();
    let NbtSummaryValue::Compound(fields) = saved else {
        panic!("an entity has a compound save");
    };
    assert!(fields.iter().any(|field| field.name == "Item"));
    assert!(fields.iter().any(|field| field.name == "PortalCooldown"
        && matches!(field.value, NbtSummaryValue::Int(value) if value > 0)));
}

#[test]
fn an_in_ground_arrow_stays_in_its_source_dimension() {
    let mut level = portal_level();
    let mut data = crate::entity::ArrowData::new(stack_of("minecraft:arrow", 1));
    data.set_in_ground(true);
    let (id, uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
        POSITION,
        EntityData::Arrow(data),
    );
    for _ in 0..3 {
        level.world.tick(Vec::new());
    }
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_some());
    assert!(level
        .world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Nether, uuid)
        .is_none());
}

#[test]
fn a_fishing_hook_with_a_live_owner_never_transfers_to_the_nether() {
    let mut level = portal_level();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "angler", 7);
    level
        .world
        .set_player_position_for_test(crate::world::ConnectionId(1), Vec3d { x: 2.5, ..POSITION });
    level.world.stock_menu_slot_for_test(
        crate::world::ConnectionId(1),
        36,
        stack_of("minecraft:fishing_rod", 1).into_summary(),
    );
    let (id, uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        POSITION,
        EntityData::FishingBobber(crate::entity::FishingBobberData::new(
            Uuid::from_u128(7),
            0,
            0,
        )),
    );
    level.world.tick(receiver.drain());
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_some());
    for _ in 0..3 {
        level.world.tick(receiver.drain());
    }
    assert!(level
        .world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Nether, uuid)
        .is_none());
}

fn living_crosses_portal(entity_type_id: i32, heart: Option<BlockPos>) -> bool {
    let mut level = portal_level();
    let (id, uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        entity_type_id,
        POSITION,
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(entity_type_id).expect("living type"),
        ),
    );
    if let Some(heart) = heart {
        assert!(level
            .world
            .stored_entity_mut_for_test(DIMENSION, id)
            .unwrap()
            .set_creaking_transient(heart));
    }
    for _ in 0..3 {
        level.world.tick(Vec::new());
    }
    level
        .world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Nether, uuid)
        .is_some()
}

#[test]
fn bosses_and_heart_bound_creakings_never_use_portals() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
    };
    assert!(living_crosses_portal(VANILLA_ENTITY_TYPE_PIG_ID, None));
    assert!(living_crosses_portal(VANILLA_ENTITY_TYPE_CREAKING_ID, None));
    assert!(!living_crosses_portal(VANILLA_ENTITY_TYPE_WITHER_ID, None));
    assert!(!living_crosses_portal(
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        None
    ));
    assert!(!living_crosses_portal(
        VANILLA_ENTITY_TYPE_CREAKING_ID,
        Some(BlockPos { x: 4, y: 64, z: 4 })
    ));
}
