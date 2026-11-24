//! The item projection every block-entity record is read and written through:
//! the world's own registry publication.

use super::*;

/// The publication is what keeps an item whole across a real save: a component
/// whose persistent form names a registry holder (`minecraft:enchantments`
/// writes `minecraft:sharpness`) can only be read back by the registry
/// generation that named it, so `ChestBlockEntity.loadAdditional` reaches it
/// through the same `PlayerItemStackResolver` `/item` writes with.
#[test]
fn a_chest_keeps_a_holder_resolved_component_across_a_real_save_and_reload() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chest_state("north", "single"),
        SET_BLOCK_UPDATE_ALL,
    );

    // The command surface is where such a stack comes from.
    let enchanted = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity alice weapon.mainhand with \
         diamond_sword[minecraft:enchantments={\"minecraft:sharpness\":3}]",
    );
    assert!(
        enchanted.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetSlot(_),
                ..
            }
        )),
        "{:?}",
        enchanted.events
    );
    let inventory = &world.publication().players[0].inventory;
    let given = inventory.slots[usize::from(inventory.selected_slot)].clone();
    let sharpness = bbb_server_datapack::enchantment::builtin_enchantment_registry()
        .iter()
        .find(|entry| entry.resource_id == "minecraft:sharpness")
        .expect("sharpness is in the built-in pack")
        .registry_id;
    assert_eq!(
        given.component_patch.enchantments,
        [bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: sharpness,
            level: 3,
        }]
    );

    // The stack the menu would have moved, reached the way a click reaches it.
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::chest_mut)
        .expect("the chest is standing")
        .items_mut()[0] = crate::tests::native_stack(given.clone());

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "data merge block 0 64 1 {lock:{count:{},components:{},predicates:{}}}",
    );
    let chest_entity = world
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .unwrap();
    assert!(chest_entity
        .chest()
        .unwrap()
        .can_open(
            false,
            &bbb_server_types::item_stack::RuntimeItemStack::empty(),
            crate::world::registry_runtime::item_stack_resolver_for_test()
        )
        .expect("the fixture lock resolves"));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 2);
    assert!(opened_screen(&opened).is_some(), "{:?}", opened.events);

    // Save, parse the document the storage would write, and reload it.
    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the chest's column is written");
    let parsed = Java26ChunkData::parse(created.document.clone(), restore_chunk_context())
        .expect("the runtime's own document is a valid Java 26.1 chunk");
    assert_eq!(parsed.block_entities().len(), 1);
    assert!(parsed.block_entities()[0]
        .iter()
        .any(|field| field.name == "lock"));
    let reloaded = ServerWorld::restore(
        ServerWorldConfig::default(),
        &restore_candidate(1_000, Some(parsed), Vec::new()),
    )
    .expect("the document this world wrote is one it reads back");

    let chest = reloaded
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::chest)
        .expect("the saved chest record became a runtime object");
    assert_eq!(chest.items()[0].summary().clone(), given);
    assert!(chest.items()[1].item_id().is_none());
    assert!(chest
        .can_open(
            false,
            &bbb_server_types::item_stack::RuntimeItemStack::empty(),
            crate::world::registry_runtime::item_stack_resolver_for_test()
        )
        .expect("the fixture lock resolves"));
}

/// The same round trip for a single-field carrier: a decorated pot's `item`
/// field is written through `DecoratedPotBlockEntity.saveAdditional` and read
/// back by `loadAdditional`, so it takes the publication too.
#[test]
fn a_decorated_pot_keeps_a_holder_resolved_component_across_a_real_save_and_reload() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 2, y: 64, z: 1 };
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_DECORATED_POT_ID,
        )
        .expect("the decorated pot has a default state"),
        SET_BLOCK_UPDATE_ALL,
    );

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity alice weapon.mainhand with \
         diamond_pickaxe[minecraft:enchantments={\"minecraft:fortune\":2}]",
    );
    let inventory = &world.publication().players[0].inventory;
    let given = inventory.slots[usize::from(inventory.selected_slot)].clone();
    assert_eq!(given.component_patch.enchantments.len(), 1);
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::decorated_pot_mut)
        .expect("the pot is standing")
        .set_item(crate::tests::native_stack(given.clone()));

    let request = world
        .save_request(1_786_000_000_000, 1, RegionCompression::Zlib)
        .unwrap();
    let created = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .expect("the pot's column is written");
    let parsed = Java26ChunkData::parse(created.document.clone(), restore_chunk_context())
        .expect("the runtime's own document is a valid Java 26.1 chunk");
    let reloaded = ServerWorld::restore(
        ServerWorldConfig::default(),
        &restore_candidate(1_000, Some(parsed), Vec::new()),
    )
    .expect("the document this world wrote is one it reads back");

    let pot = reloaded
        .get_block_entity(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::decorated_pot)
        .expect("the saved pot record became a runtime object");
    assert_eq!(pot.item().summary().clone(), given);
}
