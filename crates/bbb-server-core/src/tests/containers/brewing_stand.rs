//! The brewing stand's tick, menu and hopper faces.

use super::*;

mod brewed_potion_advancement;

fn brewing_stand_state(bottles: [bool; 3]) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:brewing_stand",
        &BTreeMap::from([
            ("has_bottle_0".to_string(), bottles[0].to_string()),
            ("has_bottle_1".to_string(), bottles[1].to_string()),
            ("has_bottle_2".to_string(), bottles[2].to_string()),
        ]),
    )
    .expect("minecraft:brewing_stand is a vanilla 26.1 block")
}

fn brewing_stand_payload(
    world: &mut ServerWorld,
    at: BlockPos,
) -> crate::block_entity::brewing_stand::BrewingStandBlockEntity {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.brewing_stand())
        .expect("the brewing stand block entity is live")
        .clone()
}

fn water_potion() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_POTION_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            ],
            potion_contents: bbb_protocol::packets::PotionContentsSummary {
                potion_id: Some(
                    bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:water")
                        .expect("water is a built-in potion")
                        .id,
                ),
                ..Default::default()
            },
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

/// `BrewingStandBlockEntity.serverTick` (`BrewingStandBlockEntity.java:97-139`)
/// through the live block ticker: the fuel slot tops the fuel up, the brew
/// takes 400 ticks, the ingredient is spent, the three bottle slots become the
/// mixed potion, the `HAS_BOTTLE` bits follow the slots and the brew raises the
/// level event.
#[test]
fn a_brewing_stand_burns_fuel_and_brews_its_bottles() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_NETHER_WART_ID, VANILLA_ITEM_POTION_ID,
    };

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        brewing_stand_state([false; 3]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let payload = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.brewing_stand_mut())
        .expect("the brewing stand block entity is live");
    payload.items_mut()[0] = crate::tests::native_stack(water_potion());
    payload.items_mut()[3] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_NETHER_WART_ID),
        count: 1,
        component_patch: Default::default(),
    });
    payload.items_mut()[4] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_BLAZE_POWDER_ID),
        count: 2,
        component_patch: Default::default(),
    });

    // The first tick spends a fuel item, fills the fuel counter and starts the
    // brew; the bottle bit follows the slot in the same pass.
    world.tick(receiver.drain());
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(brewing_stand_state([true, false, false])),
        "`HAS_BOTTLE_0` follows slot 0 (`BrewingStandBlockEntity.java:125-138`)"
    );
    let brewing = brewing_stand_payload(&mut world, at);
    assert_eq!(brewing.fuel(), 19, "one use is spent on the brew");
    assert_eq!(brewing.items()[4].count(), 1, "one blaze powder is spent");
    assert_eq!(brewing.brew_time(), 400);

    // `flush_level_events` runs before `tick_block_entities`
    // (`simulation_tick.rs:501`, `:512`), so the brew's own event lands in the
    // next pass.
    let mut events = Vec::new();
    for _ in 0..401 {
        events.extend(world.tick(receiver.drain()).events);
    }
    let brewing = brewing_stand_payload(&mut world, at);
    assert_eq!(brewing.brew_time(), 0);
    assert!(brewing.items()[3].item_id().is_none(), "the wart is spent");
    assert_eq!(brewing.items()[0].item_id(), Some(VANILLA_ITEM_POTION_ID));
    assert_eq!(
        brewing.items()[0]
            .summary()
            .component_patch
            .potion_contents
            .potion_id,
        Some(
            bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:awkward")
                .expect("awkward is a built-in potion")
                .id
        ),
        "water plus nether wart is the awkward potion"
    );
    let brewed = events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::LevelEvent(event),
            ..
        } => Some(*event),
        _ => None,
    });
    assert_eq!(
        brewed,
        Some(bbb_protocol::packets::LevelEvent {
            event_type: 1035,
            pos: at,
            data: 0,
            global: false,
        }),
        "`level.levelEvent(1035, pos, 0)` (`BrewingStandBlockEntity.java:192`)"
    );
}

/// `BrewingStandBlock.useWithoutItem` (`BrewingStandBlock.java:65-74`) over
/// `BrewingStandMenu` (`BrewingStandMenu.java:41-55`): five container slots
/// then the standard inventory, the block entity's own name, the
/// `INTERACT_WITH_BREWINGSTAND` statistic and the two `ContainerData` values.
#[test]
fn a_right_clicked_brewing_stand_opens_its_menu_with_its_two_data_slots() {
    use bbb_protocol::item_types::{VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_NETHER_WART_ID};
    use bbb_protocol::menu_types::VANILLA_MENU_TYPE_BREWING_STAND_ID;

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        brewing_stand_state([false; 3]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let payload = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.brewing_stand_mut())
        .expect("the brewing stand block entity is live");
    payload.items_mut()[0] = crate::tests::native_stack(water_potion());
    payload.items_mut()[3] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_NETHER_WART_ID),
        count: 1,
        component_patch: Default::default(),
    });
    payload.items_mut()[4] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_BLAZE_POWDER_ID),
        count: 1,
        component_patch: Default::default(),
    });

    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("clicking a brewing stand opens its screen");
    assert_eq!(screen.menu_type_id, VANILLA_MENU_TYPE_BREWING_STAND_ID);
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.brewing",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap(),
        "`BrewingStandBlockEntity.DEFAULT_NAME` (`BrewingStandBlockEntity.java:40`)"
    );
    let content = container_contents(&opened)
        .into_iter()
        .next()
        .expect("the menu is sent whole");
    assert_eq!(
        content.items.len(),
        5 + 36,
        "five container slots then `addStandardInventorySlots` \
         (`BrewingStandMenu.java:53-54`)"
    );
    assert_eq!(content.items[0], water_potion());
    assert_eq!(content.items[3].item_id, Some(VANILLA_ITEM_NETHER_WART_ID));
    assert_eq!(
        container_data(&opened)
            .iter()
            .take(2)
            .map(|data| (data.id, data.value))
            .collect::<Vec<_>>(),
        vec![(0, 0), (1, 0)],
        "brew time and fuel (`BrewingStandBlockEntity.java:35-37`)"
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:interact_with_brewingstand")
        ),
        1
    );
}

/// `BrewingStandBlockEntity`'s `WorldlyContainer` faces
/// (`BrewingStandBlockEntity.java:31-33`, `:217-246`): a hopper beside a
/// brewing stand feeds the fuel slot, one below takes bottles, and the
/// ingredient slot gives up only an empty bottle.
#[test]
fn a_hopper_feeds_the_brewing_stand_from_the_side_and_takes_bottles_from_below() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_BLAZE_POWDER_ID, VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_NETHER_WART_ID,
        VANILLA_ITEM_POTION_ID,
    };

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let stand_at = BlockPos { x: 0, y: 64, z: 1 };
    let beside = BlockPos { x: 0, y: 64, z: 0 };
    let below = BlockPos { x: 0, y: 63, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        stand_at,
        brewing_stand_state([true, false, false]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        beside,
        hopper_south_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        below,
        hopper_down_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    // The side hopper holds brewing fuel.
    let side = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, beside)
        .and_then(|entity| entity.hopper_mut())
        .expect("the side hopper is live");
    side.items_mut()[0] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_BLAZE_POWDER_ID),
        count: 2,
        component_patch: Default::default(),
    });
    // The stand's bottle slot and ingredient slot are occupied.
    let payload = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, stand_at)
        .and_then(|entity| entity.brewing_stand_mut())
        .expect("the brewing stand block entity is live");
    payload.items_mut()[0] = crate::tests::native_stack(water_potion());
    payload.items_mut()[3] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_NETHER_WART_ID),
        count: 1,
        component_patch: Default::default(),
    });

    world.tick(receiver.drain());
    let stand = brewing_stand_payload(&mut world, stand_at);
    assert_eq!(
        stand.items()[4].item_id(),
        Some(VANILLA_ITEM_BLAZE_POWDER_ID),
        "the side face reaches the fuel slot (`:33`)"
    );
    let below_items = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, below)
        .and_then(|entity| entity.hopper())
        .expect("the lower hopper is live")
        .items()
        .to_vec();
    assert_eq!(
        below_items[0].item_id(),
        Some(VANILLA_ITEM_POTION_ID),
        "the face below takes the bottle (`:32`)"
    );
    assert!(
        below_items
            .iter()
            .all(|item| item.item_id() != Some(VANILLA_ITEM_NETHER_WART_ID)),
        "the ingredient slot gives up only an empty bottle (`:243-246`): {below_items:?}"
    );

    // The side face does not overwrite an occupied bottle slot (`:224-225`),
    // and the ingredient slot hands over an empty bottle.
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, stand_at)
        .and_then(|entity| entity.brewing_stand_mut())
        .expect("the brewing stand block entity is live")
        .items_mut()[3] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_GLASS_BOTTLE_ID),
        count: 1,
        component_patch: Default::default(),
    });
    // `HopperBlockEntity.tryMoveItems` sets an eight-tick cooldown after a
    // transfer (`HopperBlockEntity.java:124`), so the second move waits.
    for _ in 0..10 {
        world.tick(receiver.drain());
    }
    let below_items = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, below)
        .and_then(|entity| entity.hopper())
        .expect("the lower hopper is live")
        .items()
        .to_vec();
    assert!(
        below_items
            .iter()
            .any(|item| item.item_id() == Some(VANILLA_ITEM_GLASS_BOTTLE_ID)),
        "an empty bottle leaves the ingredient slot: {below_items:?}"
    );
}
