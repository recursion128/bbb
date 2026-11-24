use super::*;

/// `AbstractFurnaceBlockEntity.serverTick` (`AbstractFurnaceBlockEntity.java:138-206`)
/// through the live block ticker: the fuel lights the block, the recipe cooks
/// for its own 200 ticks, and the result lands in the output slot.
#[test]
fn a_furnace_burns_its_fuel_and_cooks_the_input() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_COAL_ID, VANILLA_ITEM_IRON_INGOT_ID, VANILLA_ITEM_RAW_IRON_ID,
    };

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let raw_iron = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_RAW_IRON_ID),
        count: 2,
        component_patch: Default::default(),
    };
    let coal = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_COAL_ID),
        count: 2,
        component_patch: Default::default(),
    };
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.furnace_mut())
        .expect("the furnace block entity is live")
        .set_items(
            crate::inventory::test_item_resolver(),
            &[
                crate::tests::native_stack(raw_iron),
                crate::tests::native_stack(coal),
                crate::tests::native_stack(ItemStackSummary::empty()),
            ],
            200,
        )
        .expect("the fixture furnace items are accepted");

    // The first tick lights the block and spends one coal.
    world.tick(receiver.drain());
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(furnace_state(true))
    );
    let furnace = furnace_payload(&mut world, at);
    assert_eq!(furnace.items()[1].count(), 1);
    assert_eq!(furnace.lit_time_remaining(), 1600);
    assert_eq!(furnace.cooking_time_spent(), 1);

    for _ in 0..199 {
        world.tick(receiver.drain());
    }
    let furnace = furnace_payload(&mut world, at);
    assert_eq!(furnace.cooking_time_spent(), 0);
    assert_eq!(furnace.items()[0].count(), 1);
    assert_eq!(
        furnace.items()[2].item_id(),
        Some(VANILLA_ITEM_IRON_INGOT_ID)
    );
    assert_eq!(furnace.items()[2].count(), 1);
    assert_eq!(
        furnace
            .recipes_used()
            .get("minecraft:iron_ingot_from_smelting_raw_iron"),
        Some(&1)
    );
}

/// `WorldlyContainer` face rules (`AbstractFurnaceBlockEntity.java:46-48`,
/// `:257-273`): a hopper beside a furnace feeds the fuel slot, one below takes
/// the output, and neither reaches the slot its face does not name.
#[test]
fn a_hopper_feeds_the_furnace_from_the_side_and_takes_its_output_from_below() {
    use bbb_protocol::item_types::{VANILLA_ITEM_COAL_ID, VANILLA_ITEM_IRON_INGOT_ID};

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let furnace_at = BlockPos { x: 0, y: 64, z: 1 };
    let beside = BlockPos { x: 0, y: 64, z: 0 };
    let below = BlockPos { x: 0, y: 63, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        furnace_at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    // A hopper facing south points at the furnace from the north, so the face
    // it touches is a side -- the fuel slot only.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        beside,
        hopper_south_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    // A hopper below the furnace takes from the block above it, whose face
    // toward the hopper is DOWN -- the output and fuel slots.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        below,
        hopper_down_enabled(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, beside)
        .and_then(|entity| entity.hopper_mut())
        .expect("the side hopper is live")
        .items_mut()[0] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_COAL_ID),
        count: 4,
        component_patch: Default::default(),
    });
    // A finished result the lower hopper may take.
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, furnace_at)
        .and_then(|entity| entity.furnace_mut())
        .expect("the furnace block entity is live")
        .set_items(
            crate::inventory::test_item_resolver(),
            &[
                crate::tests::native_stack(ItemStackSummary::empty()),
                crate::tests::native_stack(ItemStackSummary::empty()),
                crate::tests::native_stack(ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_IRON_INGOT_ID),
                    count: 1,
                    component_patch: Default::default(),
                }),
            ],
            200,
        )
        .expect("the fixture furnace items are accepted");

    world.tick(receiver.drain());
    // The side hopper put coal in the fuel slot, never the input slot.
    let furnace = furnace_payload(&mut world, furnace_at);
    assert_eq!(furnace.items()[1].item_id(), Some(VANILLA_ITEM_COAL_ID));
    assert!(furnace.items()[0].item_id().is_none());
    // The hopper below took the output.
    let below_items = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, below)
        .and_then(|entity| entity.hopper())
        .expect("the lower hopper is live")
        .items()
        .to_vec();
    assert_eq!(below_items[0].item_id(), Some(VANILLA_ITEM_IRON_INGOT_ID));
    assert!(furnace_payload(&mut world, furnace_at).items()[2]
        .item_id()
        .is_none());
}

/// `AbstractFurnaceBlock.useWithoutItem` (`AbstractFurnaceBlock.java:39-47`)
/// over `FurnaceBlock.openContainer` (`FurnaceBlock.java:45-51`) and
/// `FurnaceMenu` (`FurnaceMenu.java:10-14`): three container slots then the
/// standard inventory, the block entity's own name, the
/// `INTERACT_WITH_FURNACE` statistic, and the four `ContainerData` values
/// `setSynchronizer`'s `sendAllDataToRemote` (`AbstractContainerMenu.java:150-178`)
/// sends with a fresh menu.
#[test]
fn a_right_clicked_furnace_opens_its_menu_with_the_block_entitys_data_slots() {
    use bbb_protocol::item_types::{VANILLA_ITEM_COAL_ID, VANILLA_ITEM_RAW_IRON_ID};
    use bbb_protocol::menu_types::VANILLA_MENU_TYPE_FURNACE_ID;

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let items = [
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_RAW_IRON_ID),
            count: 2,
            component_patch: Default::default(),
        },
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_COAL_ID),
            count: 3,
            component_patch: Default::default(),
        },
        ItemStackSummary::empty(),
    ];
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.furnace_mut())
        .expect("the furnace block entity is live")
        .set_items(
            crate::inventory::test_item_resolver(),
            &items
                .iter()
                .cloned()
                .map(crate::tests::native_stack)
                .collect::<Vec<_>>(),
            200,
        )
        .expect("the fixture furnace items are accepted");

    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let screen = opened_screen(&opened).expect("clicking a furnace opens its screen");
    assert_eq!(screen.menu_type_id, VANILLA_MENU_TYPE_FURNACE_ID);
    assert_eq!(
        screen.title.component(),
        &bbb_protocol::component_translatable_nbt(
            "container.furnace",
            Vec::new(),
            &bbb_protocol::ComponentStyle::default(),
        )
        .unwrap(),
        "`FurnaceBlockEntity.DEFAULT_NAME` (`FurnaceBlockEntity.java:12`)"
    );
    let content = container_contents(&opened)
        .into_iter()
        .next()
        .expect("the menu is sent whole");
    assert_eq!(
        content.items.len(),
        3 + 36,
        "three container slots then `addStandardInventorySlots` \
         (`AbstractFurnaceMenu.java:38-73`)"
    );
    assert_eq!(content.items[..3], items);
    // The click is answered inside the tick's packet phase, so the same
    // `TickResult` also carries the block ticker's own changes afterwards.
    assert_eq!(
        container_data(&opened)
            .iter()
            .take(4)
            .map(|data| (data.id, data.value))
            .collect::<Vec<_>>(),
        vec![(0, 0), (1, 0), (2, 0), (3, 200)],
        "lit time, lit duration, cooking progress and cooking total time \
         (`AbstractFurnaceBlockEntity.java:45-52`)"
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:interact_with_furnace")
        ),
        1
    );
}

/// `AbstractContainerMenu.broadcastChanges`' data half (`:204-207`) over a
/// furnace: a value the client already holds is not sent again, and every one
/// that moves is.
#[test]
fn a_settled_furnace_menu_says_nothing_and_a_cooking_one_reports_each_move() {
    use bbb_protocol::item_types::{VANILLA_ITEM_COAL_ID, VANILLA_ITEM_RAW_IRON_ID};

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened_screen(&opened).is_some());
    assert_eq!(container_data(&opened).len(), 4);

    // Nothing burns, so all four values stay where the open left them.
    let settled = world.tick(receiver.drain());
    assert!(
        container_data(&settled).is_empty(),
        "a settled menu sends no data value: {:?}",
        container_data(&settled)
    );

    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.furnace_mut())
        .expect("the furnace block entity is live")
        .set_items(
            crate::inventory::test_item_resolver(),
            &[
                crate::tests::native_stack(ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_RAW_IRON_ID),
                    count: 1,
                    component_patch: Default::default(),
                }),
                crate::tests::native_stack(ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_COAL_ID),
                    count: 1,
                    component_patch: Default::default(),
                }),
                crate::tests::native_stack(ItemStackSummary::empty()),
            ],
            200,
        )
        .expect("the fixture furnace items are accepted");
    let burning = world.tick(receiver.drain());
    let furnace = furnace_payload(&mut world, at);
    let values = [
        furnace.lit_time_remaining(),
        furnace.lit_total_time(),
        furnace.cooking_time_spent(),
        furnace.cooking_total_time(),
    ];
    let sent = container_data(&burning);
    assert!(
        sent.iter().any(|data| data.id == 2),
        "the cooking progress moved: {sent:?}"
    );
    for data in &sent {
        assert_eq!(
            i32::from(data.value),
            values[usize::try_from(data.id).expect("a data slot id")],
            "data slot {} carries the block entity's own value",
            data.id
        );
    }
}

/// A click in a furnace menu writes through to the block entity the same way a
/// chest click does, and the input slot's `Container.setItem`
/// (`AbstractFurnaceBlockEntity.java:291-299`) resets the cooking timer, which
/// the menu reports on the same broadcast.
#[test]
fn a_click_in_a_furnace_menu_writes_through_to_the_block_entity() {
    use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let state_id = container_contents(&opened)[0].state_id;

    // Menu slot 30 is the first hotbar slot, which the fixture stocks with a
    // full stack of stone; menu slot 0 is the furnace's input slot.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 1,
            state_id,
            slot_num: 30,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    let placed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 1,
            state_id: state_id + 2,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );

    let furnace = furnace_payload(&mut world, at);
    assert_eq!(
        furnace.items()[0].summary().clone(),
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count: 64,
            component_patch: DataComponentPatchSummary::default().into(),
        },
        "the stack moved out of the player's hotbar and into the furnace"
    );
    assert_eq!(furnace.cooking_time_spent(), 0);
    assert_eq!(
        furnace.cooking_total_time(),
        crate::block_entity::furnace::DEFAULT_COOKING_TIME,
        "no recipe claims stone, so `getTotalCookTime` answers the default"
    );
    let data = container_data(&placed);
    assert!(
        data.iter()
            .any(|data| data.id == 3 && i32::from(data.value) == 200),
        "the new cooking total time is reported: {data:?}"
    );
}

/// `AbstractFurnaceMenu.stillValid` (`AbstractFurnaceMenu.java:87-89`) over
/// `Container.stillValidBlockEntity` (`Container.java:90-102`): walking out of
/// range closes the menu, which is the furnace arm of
/// [`ServerWorld::container_still_valid`].
#[test]
fn walking_out_of_range_closes_the_furnace_menu_from_the_server_side() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    assert!(opened_screen(&opened).is_some());

    // The walk out of range is the fixture; the close is the subject.
    let walked = place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 0.5,
            y: 65.0,
            z: 12.5,
        },
        None,
    );
    let closed = walked
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerClose(close),
                ..
            } => Some(*close),
            _ => None,
        })
        .expect("walking away closes the menu from the server side");
    assert_eq!(closed.container_id, 1);
    // No open counter and no lid: `AbstractFurnaceBlockEntity` is not a
    // `RandomizableContainerBlockEntity`, so nothing is signalled.
    assert!(
        sounds_sent(&walked).is_empty(),
        "{:?}",
        sounds_sent(&walked)
    );
    assert!(block_events_sent(&walked).is_empty());
}

/// `FurnaceResultSlot.checkTakeAchievements` (`FurnaceResultSlot.java:45-52`)
/// over `AbstractFurnaceBlockEntity.awardUsedRecipesAndPopExperience`
/// (`:334-343`): taking the result awards the recipes the furnace cooked,
/// pops their experience where the taker stands and clears `RecipesUsed`.
///
/// The recipe is gold ore smelting, whose experience is exactly 1.0, so the
/// pop is one orb whatever the roll does.
#[test]
fn taking_the_furnace_result_awards_the_recipes_and_pops_their_experience() {
    use bbb_protocol::item_types::{
        VANILLA_ITEM_COAL_ID, VANILLA_ITEM_GOLD_INGOT_ID, VANILLA_ITEM_GOLD_ORE_ID,
    };
    use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_CRAFTED_ID;
    use bbb_server_types::stats::StatKey;

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(recipe_crafted::config(
        datapack_config(),
        "minecraft:gold_ingot_from_smelting_gold_ore",
        serde_json::json!([{"items":"minecraft:gold_ore","count":1},{"items":"minecraft:coal","count":1},{"items":"minecraft:air","count":0}]),
    ));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.furnace_mut())
        .expect("the furnace block entity is live")
        .set_items(
            crate::inventory::test_item_resolver(),
            &[
                crate::tests::native_stack(ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_GOLD_ORE_ID),
                    count: 2,
                    component_patch: Default::default(),
                }),
                crate::tests::native_stack(ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_COAL_ID),
                    count: 2,
                    component_patch: Default::default(),
                }),
                crate::tests::native_stack(ItemStackSummary::empty()),
            ],
            200,
        )
        .expect("the fixture furnace items are accepted");
    for _ in 0..200 {
        world.tick(receiver.drain());
    }
    let furnace = furnace_payload(&mut world, at);
    assert_eq!(
        furnace.items()[2].item_id(),
        Some(VANILLA_ITEM_GOLD_INGOT_ID)
    );
    assert_eq!(
        furnace
            .recipes_used()
            .get("minecraft:gold_ingot_from_smelting_gold_ore"),
        Some(&1)
    );

    let opened = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let state_id = container_contents(&opened)[0].state_id;
    let taken = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 1,
            state_id,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );

    assert!(world.player_advancement_is_done(ConnectionId(1), "test:crafted"));
    let added = taken
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::RecipeBookAdd(add),
                ..
            } => Some(add.clone()),
            _ => None,
        })
        .expect("`ServerRecipeBook.addRecipes` publishes the recipe");
    assert!(!added.replace);
    let displays = world
        .datapacks()
        .expect("the test world has a datapack generation")
        .recipe_displays
        .displays_for_recipe("minecraft:gold_ingot_from_smelting_gold_ore");
    assert!(!displays.is_empty());
    assert_eq!(
        added
            .entries
            .iter()
            .map(|entry| &entry.contents)
            .collect::<Vec<_>>(),
        displays.iter().collect::<Vec<_>>()
    );
    assert!(
        furnace_payload(&mut world, at).recipes_used().is_empty(),
        "`awardUsedRecipesAndPopExperience` clears the map"
    );
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            StatKey::new(
                VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
                VANILLA_ITEM_GOLD_INGOT_ID
            )
        ),
        1,
        "`ItemStack.onCraftedBy` (`ItemStack.java:723-726`) counts the take"
    );
    // `createExperience` (`:353-360`) pops the orb at the taker's own feet, so
    // the session absorbs it the moment the orb touches it
    // (`ExperienceOrb.java:275-291`) and the bar is where the experience
    // shows.
    fn experience_updates(events: &[WorldEvent]) -> Vec<bbb_protocol::packets::PlayerExperience> {
        events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetExperience(update),
                    ..
                } => Some(*update),
                _ => None,
            })
            .collect()
    }
    let mut experience = experience_updates(&taken.events);
    for _ in 0..20 {
        if experience.last().is_some_and(|update| update.total >= 6) {
            break;
        }
        let tick = world.tick(receiver.drain());
        experience.extend(experience_updates(&tick.events));
    }
    assert_eq!(
        experience.last().map(|update| update.total),
        Some(6),
        "gold ore smelting plus criterion reward pays six experience: {experience:?}"
    );
}

/// `BlockEntity.setChanged` (`BlockEntity.java:198-203`) from the furnace
/// ticker (`AbstractFurnaceBlockEntity.java:203-205`): when a lit furnace
/// burns its next stick without changing its block state, the comparator
/// beside it still re-reads the emptier fuel slot.
#[test]
fn a_comparator_hears_a_lit_furnace_spend_its_next_fuel() {
    use bbb_protocol::item_types::{VANILLA_ITEM_RAW_IRON_ID, VANILLA_ITEM_STICK_ID};

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let comparator = BlockPos { x: -1, y: 64, z: 1 };
    let stone = crate::tests::block_state_id("minecraft:stone");
    let _ = world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            y: 63,
            ..comparator
        },
        stone,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let stack = |item_id, count| {
        crate::tests::native_stack(ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        })
    };
    // 64 + 33 items over three 64-stacks: the first stick leaves 96/192 (8),
    // the second, 100 ticks later, 95/192 (7).
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.furnace_mut())
        .expect("the furnace block entity is live")
        .set_items(
            crate::inventory::test_item_resolver(),
            &[
                stack(VANILLA_ITEM_RAW_IRON_ID, 64),
                stack(VANILLA_ITEM_STICK_ID, 33),
                crate::tests::native_stack(ItemStackSummary::empty()),
            ],
            200,
        )
        .expect("the fixture furnace items are accepted");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        comparator,
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:comparator",
            &BTreeMap::from([
                ("facing".to_string(), "east".to_string()),
                ("mode".to_string(), "compare".to_string()),
                ("powered".to_string(), "false".to_string()),
            ]),
        )
        .expect("built-in comparator state"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let output = |world: &mut ServerWorld| {
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, comparator)
            .and_then(|entity| entity.comparator())
            .expect("the comparator has its block entity")
            .output_signal()
    };
    for _ in 0..10 {
        world.tick(receiver.drain());
    }
    assert_eq!(output(&mut world), 8);
    for _ in 0..100 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(furnace_state(true))
    );
    assert_eq!(furnace_payload(&mut world, at).items()[1].count(), 31);
    assert_eq!(output(&mut world), 7);
}
