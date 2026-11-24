//! The synched item-stack rows that are not the item entity's: an item
//! display and a thrown `ThrowableItemProjectile`, both reached through the
//! real tracking path and both carrying a component that names a registry
//! holder.

use super::*;

use crate::entity::display::ItemDisplayData;
use crate::entity::EntityData;
use bbb_protocol::packets::{
    DataComponentPatchSummary, EntityDataValueKind, InteractionHand, ItemEnchantmentSummary,
    ItemStackSummary, PlayServerbound, SetCreativeModeSlot, UseItem,
};

use crate::tests::entities::console_command;

/// Hotbar slot 0 in `InventoryMenu` numbering, which the fixture session
/// holds selected.
const HELD_MENU_SLOT: i16 = 36;

/// The `ITEM_STACK` row one tick's packets carry for an entity, as the session
/// receives it.
fn sent_stack_row(events: &[WorldEvent], entity_id: i32) -> Option<ItemStackSummary> {
    events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetEntityData(data),
            ..
        } if data.id == entity_id => data.values.iter().find_map(|value| match &value.value {
            EntityDataValueKind::ItemStack(stack) => Some(stack.clone()),
            _ => None,
        }),
        _ => None,
    })
}

/// The one stored entity `find` accepts, once the pairing pass has run.
fn tracked_entity(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    find: impl Fn(&EntityData) -> bool,
) -> i32 {
    for _ in 0..4 {
        world.tick(receiver.drain());
    }
    let mut ids = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| find(entity.data()))
        });
    let found = ids.next().expect("the entity is stored");
    assert!(ids.next().is_none(), "exactly one such entity");
    found
}

/// The id `minecraft:enchantment`'s `<id>` holder carries in the generation
/// this world loaded, which is the id a decoded component carries.
fn enchant_holder_id(world: &ServerWorld, enchantment: &str) -> i32 {
    let registry = world
        .datapacks()
        .expect("the fixture world has a datapack generation")
        .registries
        .registry("minecraft:enchantment")
        .expect("the generation carries the enchantment registry");
    i32::try_from(
        registry
            .elements
            .iter()
            .position(|element| element.id == enchantment)
            .unwrap_or_else(|| panic!("the loaded registry carries {enchantment}")),
    )
    .expect("the registry is small")
}

/// One stack with `minecraft:enchantments` on it, the way a decoded component
/// carries it: a published holder id, not a name.
fn enchanted_summary(
    world: &ServerWorld,
    item: &str,
    enchantment: &str,
    level: i32,
) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(item)
                .unwrap_or_else(|| panic!("{item} is a builtin item"))
                .id,
        ),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            ],
            enchantments: vec![ItemEnchantmentSummary {
                holder_id: enchant_holder_id(world, enchantment),
                level,
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

fn held_by_server(stack: &bbb_server_types::item_stack::RuntimeItemStack) -> ItemStackSummary {
    stack.summary().clone()
}

/// The `Display.ItemDisplay.DATA_ITEM_STACK_ID` row
/// on the real tracking path.
///
/// The stack is written through `/data merge entity`, whose overlay reads the
/// entity's rows back to decide which the load changed, so both halves go
/// through the level's publication: the load's comparison and the
/// `SetEntityData` the watching session receives. Vanilla writes the row
/// whatever the stack names (`SynchedEntityData.java:222-231`), so the row
/// must carry the published holder id rather than be left out.
#[test]
fn a_watching_session_is_sent_an_item_displays_holder_resolved_stack() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let position = snapshot_player(&world.tick(Vec::new()), 1).position;

    // `/summon` reaches armor stands and mobs (`EntityStore::spawn_summoned`),
    // so the display itself is a fixture; the stack on it is written through
    // the real `/data merge entity` path below.
    let (display, uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
        position,
        EntityData::ItemDisplay(ItemDisplayData::new()),
    );
    for _ in 0..4 {
        world.tick(receiver.drain());
    }

    let merged = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!(
            "data merge entity {uuid} {{item:{{id:\"minecraft:diamond_sword\",count:1,\
             components:{{\"minecraft:enchantments\":{{\"minecraft:sharpness\":3}}}}}}}}"
        ),
    );

    let mut events = merged.events;
    let mut shown = None;
    for _ in 0..4 {
        shown = shown.or_else(|| sent_stack_row(&events, display));
        if shown.is_some() {
            break;
        }
        events = world.tick(receiver.drain()).events;
    }
    let shown = shown.expect("the watching session is sent the display's item row");

    let persistent = match world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, display)
        .map(crate::entity::Entity::data)
    {
        Some(EntityData::ItemDisplay(data)) => data.item().clone(),
        other => panic!("the display is an item display: {other:?}"),
    };
    let held = held_by_server(&persistent);
    assert_eq!(
        shown, held,
        "the row the session is shown is the stack the server holds"
    );
    let expected = enchanted_summary(&world, "minecraft:diamond_sword", "minecraft:sharpness", 3);
    assert_eq!(
        shown, expected,
        "the row carries the component the load wrote, by published holder id"
    );
}

/// The `ThrowableItemProjectile.DATA_ITEM_STACK` row
/// on the real throw path:
/// `SnowballItem.use` projects `stack.copyWithCount(1)` through the level's
/// publication so enchanted stacks preserve their resolved holder id.
#[test]
fn a_watching_session_is_sent_a_thrown_snowballs_holder_resolved_stack() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let held = enchanted_summary(&world, "minecraft:snowball", "minecraft:sharpness", 3);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: HELD_MENU_SLOT,
            item: held.clone(),
        }),
    );

    let thrown = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 7,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    let snowball = tracked_entity(&receiver, &mut world, |data| {
        matches!(data, EntityData::Snowball(_))
    });

    let mut events = thrown.events;
    let mut shown = None;
    for _ in 0..4 {
        shown = shown.or_else(|| sent_stack_row(&events, snowball));
        if shown.is_some() {
            break;
        }
        events = world.tick(receiver.drain()).events;
    }
    let shown = shown.expect("the watching session is sent the projectile's item row");

    let persistent = match world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, snowball)
        .map(crate::entity::Entity::data)
    {
        Some(EntityData::Snowball(data)) => data.item().clone(),
        other => panic!("the projectile is a snowball: {other:?}"),
    };
    let stored = held_by_server(&persistent);
    assert_eq!(
        shown, stored,
        "the row the session is shown is the stack the server holds"
    );
    assert_eq!(
        shown, held,
        "the row carries the stack the thrower held, by published holder id"
    );
}
