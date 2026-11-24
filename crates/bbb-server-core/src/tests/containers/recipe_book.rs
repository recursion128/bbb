use super::*;
use bbb_protocol::item_types::{
    VANILLA_ITEM_CHEST_ID, VANILLA_ITEM_CRAFTING_TABLE_ID, VANILLA_ITEM_OAK_PLANKS_ID,
    VANILLA_ITEM_RAW_IRON_ID, VANILLA_ITEM_STICK_ID,
};
use bbb_protocol::packets::{PlaceGhostRecipe, PlaceRecipeCommand, RecipeDisplayEntry};

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn display(world: &ServerWorld, recipe: &str) -> RecipeDisplayEntry {
    world
        .datapacks()
        .expect("the test world has a datapack generation")
        .recipe_displays
        .displays_for_recipe(recipe)[0]
        .clone()
}

fn place(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    recipe: &str,
    use_max_items: bool,
) -> TickResult {
    let container_id = world
        .player_inventory_for_test(ConnectionId(1))
        .container_id();
    let recipe_index = display(world, recipe).id.index;
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::PlaceRecipe(PlaceRecipeCommand {
            container_id,
            recipe_index,
            use_max_items,
        }),
    )
}

/// Menu slot `slot` as `(item, count)`.
fn slot(world: &mut ServerWorld, slot: i16) -> (Option<i32>, i32) {
    let item = world
        .player_inventory_for_test(ConnectionId(1))
        .slot_item(slot)
        .into_owned();
    (item.item_id(), item.count())
}

fn ghost(tick: &TickResult) -> Option<PlaceGhostRecipe> {
    tick.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::PlaceGhostRecipe(ghost),
            ..
        } => Some(ghost.clone()),
        _ => None,
    })
}

fn crafting_table_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    (sender, receiver, world)
}

/// `ServerPlaceRecipe.placeRecipe` in a `CraftingMenu`: a click places one
/// craft along the recipe's shape, a second click adds one more to each
/// cell, and the shift form fills the grid with as many crafts as the
/// inventory holds.
#[test]
fn the_recipe_book_fills_a_crafting_table_and_shift_places_the_most() {
    let (sender, receiver, mut world) = crafting_table_world();
    world.stock_menu_slot_for_test(ConnectionId(1), 36, stack(VANILLA_ITEM_OAK_PLANKS_ID, 64));
    world.award_player_recipe_for_test(ConnectionId(1), "minecraft:chest");
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let table = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_CRAFTING_TABLE_ID,
    )
    .unwrap();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        table,
        SET_BLOCK_UPDATE_ALL
    ));
    right_click(&sender, &receiver, &mut world, 1, at, 1);
    // `CraftingMenu`: result 0, grid 1..=9, hotbar from 37.
    let ring = [1, 2, 3, 4, 6, 7, 8, 9];

    let placed = place(&sender, &receiver, &mut world, "minecraft:chest", false);
    assert!(ghost(&placed).is_none());
    for cell in ring {
        assert_eq!(
            slot(&mut world, cell),
            (Some(VANILLA_ITEM_OAK_PLANKS_ID), 1)
        );
    }
    assert_eq!(slot(&mut world, 5), (None, 0));
    assert_eq!(slot(&mut world, 0), (Some(VANILLA_ITEM_CHEST_ID), 1));
    assert_eq!(slot(&mut world, 37), (Some(VANILLA_ITEM_OAK_PLANKS_ID), 56));
    assert!(placed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::ContainerSetSlot(update),
            ..
        } if update.container_id == 1 && update.slot == 0
            && update.item.item_id == Some(VANILLA_ITEM_CHEST_ID)
    )));

    place(&sender, &receiver, &mut world, "minecraft:chest", false);
    for cell in ring {
        assert_eq!(
            slot(&mut world, cell),
            (Some(VANILLA_ITEM_OAK_PLANKS_ID), 2)
        );
    }
    assert_eq!(slot(&mut world, 37), (Some(VANILLA_ITEM_OAK_PLANKS_ID), 48));

    place(&sender, &receiver, &mut world, "minecraft:chest", true);
    for cell in ring {
        assert_eq!(
            slot(&mut world, cell),
            (Some(VANILLA_ITEM_OAK_PLANKS_ID), 8)
        );
    }
    assert_eq!(slot(&mut world, 37), (None, 0));
}

/// `handlePlaceRecipe` in the player's own 2x2: missing ingredients answer
/// `ClientboundPlaceGhostRecipePacket` and move nothing; once they are there
/// the same request fills the grid.
#[test]
fn missing_ingredients_show_a_ghost_recipe_in_the_inventory_grid() {
    let (sender, receiver, mut world) = crafting_table_world();
    // The `minecraft:tick` criterion of `recipes/decorations/crafting_table`
    // teaches this recipe on join.
    assert!(world.player_knows_recipe_for_test(ConnectionId(1), "minecraft:crafting_table"));
    world.stock_menu_slot_for_test(ConnectionId(1), 36, stack(VANILLA_ITEM_OAK_PLANKS_ID, 3));

    let refused = place(
        &sender,
        &receiver,
        &mut world,
        "minecraft:crafting_table",
        false,
    );
    assert_eq!(
        ghost(&refused),
        Some(PlaceGhostRecipe {
            container_id: 0,
            recipe_display: display(&world, "minecraft:crafting_table").display,
        })
    );
    for cell in 1..=4 {
        assert_eq!(slot(&mut world, cell), (None, 0));
    }
    assert_eq!(slot(&mut world, 36), (Some(VANILLA_ITEM_OAK_PLANKS_ID), 3));

    world.stock_menu_slot_for_test(ConnectionId(1), 36, stack(VANILLA_ITEM_OAK_PLANKS_ID, 4));
    let placed = place(
        &sender,
        &receiver,
        &mut world,
        "minecraft:crafting_table",
        false,
    );
    assert!(ghost(&placed).is_none());
    for cell in 1..=4 {
        assert_eq!(
            slot(&mut world, cell),
            (Some(VANILLA_ITEM_OAK_PLANKS_ID), 1)
        );
    }
    assert_eq!(
        slot(&mut world, 0),
        (Some(VANILLA_ITEM_CRAFTING_TABLE_ID), 1)
    );
    assert_eq!(slot(&mut world, 36), (None, 0));
}

/// `AbstractFurnaceMenu.handlePlacement`: the 1x1 input slot takes the
/// cooking ingredient, and the block entity holds it after the request.
#[test]
fn the_recipe_book_fills_a_furnace_input() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use bbb_server_types::item_stack::RuntimeItemStack;

    for (name, value, expected) in [
        (
            "minecraft:rarity",
            NbtSummaryValue::String("common".into()),
            60,
        ),
        (
            "minecraft:rarity",
            NbtSummaryValue::String("uncommon".into()),
            40,
        ),
        ("!minecraft:rarity", NbtSummaryValue::Compound(vec![]), 40),
    ] {
        let (sender, receiver, mut world) = crafting_table_world();
        let resolver = world.item_stack_resolver();
        let incoming = RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String("minecraft:raw_iron".into()),
                },
                NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(20),
                },
                NbtSummaryEntry {
                    name: "components".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: name.into(),
                        value,
                    }]),
                },
            ]),
            &resolver,
        )
        .unwrap();
        world.stock_menu_slot_for_test(ConnectionId(1), 36, stack(VANILLA_ITEM_RAW_IRON_ID, 40));
        world.stock_menu_slot_for_test(ConnectionId(1), 37, incoming.summary().clone());
        world.award_player_recipe_for_test(
            ConnectionId(1),
            "minecraft:iron_ingot_from_smelting_raw_iron",
        );
        let at = BlockPos { x: 0, y: 64, z: 1 };
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            at,
            furnace_state(false),
            SET_BLOCK_UPDATE_ALL,
        ));
        right_click(&sender, &receiver, &mut world, 1, at, 1);

        let placed = place(
            &sender,
            &receiver,
            &mut world,
            "minecraft:iron_ingot_from_smelting_raw_iron",
            true,
        );
        assert!(ghost(&placed).is_none());
        assert_eq!(
            slot(&mut world, 0),
            (Some(VANILLA_ITEM_RAW_IRON_ID), expected)
        );
        // `AbstractFurnaceMenu`: input, fuel, result, then the inventory; the
        // first hotbar slot is 30.
        assert_eq!(slot(&mut world, 30), (None, 0));
        assert_eq!(
            slot(&mut world, 31),
            if expected == 60 {
                (None, 0)
            } else {
                (Some(VANILLA_ITEM_RAW_IRON_ID), 20)
            }
        );
        let input = furnace_payload(&mut world, at).items()[0].clone();
        assert_eq!(
            (input.item_id(), input.count()),
            (Some(VANILLA_ITEM_RAW_IRON_ID), expected)
        );
        assert!(input
            .same_item_same_effective_components(
                &RuntimeItemStack::from_summary(stack(VANILLA_ITEM_RAW_IRON_ID, 1)),
                &resolver,
            )
            .unwrap());
    }
}

/// `ResultSlot.checkTakeAchievements` learns the used recipe through
/// `ServerPlayer.awardRecipes`: the client is told with a notification and
/// the recipe's `recipe_unlocked` advancement completes.
#[test]
fn taking_a_craft_publishes_the_learned_recipe_and_its_advancement() {
    const STICK_ADVANCEMENT: &str = "minecraft:recipes/misc/stick";
    let (sender, receiver, mut world) = crafting_table_world();
    // Planks staged straight into the grid never pass through the inventory,
    // so `has_planks` cannot complete the advancement first.
    for cell in [1, 3] {
        world.stock_menu_slot_for_test(ConnectionId(1), cell, stack(VANILLA_ITEM_OAK_PLANKS_ID, 1));
    }
    assert!(!world.player_knows_recipe_for_test(ConnectionId(1), "minecraft:stick"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), STICK_ADVANCEMENT));

    let inventory = world.player_inventory_for_test(ConnectionId(1));
    let state_id = inventory.state_id();
    let taken = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::Pickup,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert_eq!(
        world
            .player_inventory_for_test(ConnectionId(1))
            .carried()
            .item_id(),
        Some(VANILLA_ITEM_STICK_ID)
    );
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
        .expect("the learned recipe reaches the client");
    assert!(!added.replace);
    assert_eq!(added.entries.len(), 1);
    assert_eq!(
        added.entries[0].contents,
        display(&world, "minecraft:stick")
    );
    assert!(added.entries[0].notification && added.entries[0].highlight);
    assert!(world.player_knows_recipe_for_test(ConnectionId(1), "minecraft:stick"));
    assert!(world.player_advancement_is_done(ConnectionId(1), STICK_ADVANCEMENT));
}
