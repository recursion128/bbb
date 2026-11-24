//! `WrittenBookItem.use`, `KnowledgeBookItem.use` and `BundleItem.use`.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID;
use bbb_protocol::packets::{ItemStackTemplateSummary, OpenBook, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::*;

fn holding(stack: &RuntimeItemStack) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
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
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack.summary().clone());
    (sender, receiver, world)
}

fn with_component(item: &str, type_id: i32, value: NbtSummaryValue) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id(item)),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    })
    .with_component_nbt(
        type_id,
        Some(&value),
        crate::inventory::test_item_resolver(),
    )
    .unwrap()
}

fn held(world: &ServerWorld) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(world.publication().players[0].inventory.slots[0].clone())
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[test]
fn a_written_book_opens_and_is_marked_resolved() {
    let book = with_component(
        "minecraft:written_book",
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        NbtSummaryValue::Compound(vec![
            entry("title", NbtSummaryValue::String("Notes".into())),
            entry("author", NbtSummaryValue::String("alice".into())),
            entry(
                "pages",
                NbtSummaryValue::List(vec![NbtSummaryValue::String("hello".into())]),
            ),
        ]),
    );
    let (sender, receiver, mut world) = holding(&book);

    let used = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    assert!(used.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::OpenBook(OpenBook {
                hand: InteractionHand::MainHand
            }),
        }
    )));
    let Some(NbtSummaryValue::Compound(content)) = held(&world)
        .component_nbt(
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            crate::inventory::test_item_resolver(),
        )
        .unwrap()
    else {
        panic!("the book keeps its content");
    };
    assert!(content
        .iter()
        .any(|field| field.name == "resolved" && field.value == NbtSummaryValue::Byte(1)));
    assert!(content.iter().any(|field| field.name == "pages"));
}

/// `WrittenBookContent.resolve` (`WrittenBookContent.java:107-124`): a page
/// that resolves past 32767 JSON chars leaves every page unresolved.
#[test]
fn an_oversized_resolved_page_leaves_the_book_unresolved() {
    let nbt_page = NbtSummaryValue::Compound(vec![
        entry("nbt", NbtSummaryValue::String("big[]".into())),
        entry("storage", NbtSummaryValue::String("test:s".into())),
    ]);
    let selector = NbtSummaryValue::Compound(vec![entry(
        "selector",
        NbtSummaryValue::String("@s".into()),
    )]);
    let large = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string(String::new())),
        entry("extra", NbtSummaryValue::List(vec![nbt_page])),
    ]);
    let book = with_component(
        "minecraft:written_book",
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        NbtSummaryValue::Compound(vec![
            entry("title", NbtSummaryValue::String("Notes".into())),
            entry("author", NbtSummaryValue::String("alice".into())),
            entry("pages", NbtSummaryValue::List(vec![selector, large])),
        ]),
    );
    let (sender, receiver, mut world) = holding(&book);
    // 256 strings of 200 chars, doubled up under the 256-char command limit.
    let mut commands = vec![format!(
        "data modify storage test:s big set value [\"{}\"]",
        "x".repeat(200)
    )];
    commands.extend(std::iter::repeat_n(
        "data modify storage test:s big append from storage test:s big[]".to_string(),
        8,
    ));
    for (operation, command) in commands.iter().enumerate() {
        let tick = crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut world,
            90 + operation as u64,
            command,
        );
        assert!(
            crate::tests::entities::console_command_answer(&tick)
                .0
                .is_some(),
            "{command}"
        );
    }

    let content = |world: &ServerWorld| {
        let Some(NbtSummaryValue::Compound(content)) = held(world)
            .component_nbt(
                VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
                crate::inventory::test_item_resolver(),
            )
            .unwrap()
        else {
            panic!("the book keeps its content");
        };
        content
    };
    let pages = |content: &[NbtSummaryEntry]| {
        content
            .iter()
            .find(|field| field.name == "pages")
            .unwrap()
            .value
            .clone()
    };
    let written = pages(&content(&world));

    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    let content = content(&world);
    assert!(content
        .iter()
        .any(|field| field.name == "resolved" && field.value == NbtSummaryValue::Byte(1)));
    assert_eq!(pages(&content), written);
}

#[test]
fn a_knowledge_book_unlocks_its_recipes_and_is_used_up() {
    let book = with_component(
        "minecraft:knowledge_book",
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
        NbtSummaryValue::List(vec![NbtSummaryValue::string("minecraft:diamond_sword")]),
    );
    let (sender, receiver, mut world) = holding(&book);

    let used = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    assert!(used.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::RecipeBookAdd(add),
        } if !add.entries.is_empty()
    )));
    assert!(held(&world).is_empty());
}

#[test]
fn using_a_bundle_drops_its_stacks_one_by_one() {
    let template = |item: &str, count| ItemStackTemplateSummary {
        item_id: item_id(item),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let bundle = RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id("minecraft:bundle")),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    })
    .with_component_patch(
        &DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
            bundle_contents_items: vec![
                template("minecraft:dirt", 3),
                template("minecraft:stone", 5),
            ],
            ..DataComponentPatchSummary::default()
        },
        crate::inventory::test_item_resolver(),
    )
    .unwrap();
    let (sender, receiver, mut world) = holding(&bundle);
    let dropped = |world: &ServerWorld| {
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter_map(|id| world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id))
            .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ITEM_ID)
            .count()
    };

    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);
    world.tick(receiver.drain());
    assert_eq!(dropped(&world), 1);
    for _ in 0..12 {
        world.tick(receiver.drain());
    }

    assert_eq!(dropped(&world), 2);
    let held = held(&world);
    let contents = held
        .effective_component(
            VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            crate::inventory::test_item_resolver(),
        )
        .unwrap()
        .unwrap();
    assert!(contents.bundle_contents_items.is_empty());
}
