use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

// ---- block / item / text display ------------------------------------------

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID, VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID, VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID, VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
    VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
};
use bbb_protocol::packets::EntityDataValueKind;
use bbb_protocol::{NbtSummaryEntry, RichTextComponent};

use crate::entity::display::{
    self, BillboardConstraints, BlockDisplayData, Brightness, DisplayData, ItemDisplayContext,
    ItemDisplayData, Quaternionf, TextAlign, TextDisplayData, Transformation, Vector3f,
    BLOCK_DISPLAY_BLOCK_STATE_DATA_ID, DISPLAY_BILLBOARD_RENDER_CONSTRAINTS_DATA_ID,
    DISPLAY_BRIGHTNESS_OVERRIDE_DATA_ID, DISPLAY_GLOW_COLOR_OVERRIDE_DATA_ID,
    DISPLAY_HEIGHT_DATA_ID, DISPLAY_LEFT_ROTATION_DATA_ID,
    DISPLAY_POS_ROT_INTERPOLATION_DURATION_DATA_ID, DISPLAY_RIGHT_ROTATION_DATA_ID,
    DISPLAY_SCALE_DATA_ID, DISPLAY_SHADOW_RADIUS_DATA_ID, DISPLAY_SHADOW_STRENGTH_DATA_ID,
    DISPLAY_TRANSFORMATION_INTERPOLATION_DURATION_DATA_ID,
    DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID, DISPLAY_TRANSLATION_DATA_ID,
    DISPLAY_VIEW_RANGE_DATA_ID, DISPLAY_WIDTH_DATA_ID, FLAG_ALIGN_LEFT, FLAG_SEE_THROUGH,
    FLAG_SHADOW, ITEM_DISPLAY_ITEM_DISPLAY_DATA_ID, ITEM_DISPLAY_ITEM_STACK_DATA_ID,
    TEXT_DISPLAY_BACKGROUND_COLOR_DATA_ID, TEXT_DISPLAY_LINE_WIDTH_DATA_ID,
    TEXT_DISPLAY_STYLE_FLAGS_DATA_ID, TEXT_DISPLAY_TEXT_DATA_ID, TEXT_DISPLAY_TEXT_OPACITY_DATA_ID,
};

fn origin() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn field<'a>(record: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    record
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

/// A `DisplayData` with every saved field away from its default.
fn loaded_display() -> DisplayData {
    DisplayData::new().with_saved_fields(
        Transformation {
            translation: Vector3f::new(0.5, -1.25, 2.0),
            left_rotation: Quaternionf::new(0.0, 0.0, 0.0, -1.0),
            scale: Vector3f::new(2.0, 4.0, 8.0),
            right_rotation: Quaternionf::new(0.0, 1.0, 0.0, 0.0),
        },
        5,
        3,
        100,
        BillboardConstraints::Center,
        2.5,
        1.5,
        0.75,
        3.0,
        2.0,
        0x00ff_00ff,
        Some(Brightness { block: 7, sky: 15 }),
    )
}

/// What [`loaded_display`] reads back as: `start_interpolation` is not a key
/// `addAdditionalSaveData` writes (`Display.java:238-250`), so the delay
/// reloads as the reader's `0` default (`:217`).
fn reloaded_display() -> DisplayData {
    DisplayData::new().with_saved_fields(
        loaded_display().transformation(),
        5,
        0,
        100,
        BillboardConstraints::Center,
        2.5,
        1.5,
        0.75,
        3.0,
        2.0,
        0x00ff_00ff,
        Some(Brightness { block: 7, sky: 15 }),
    )
}

fn spawn(store: &mut EntityStore, data: EntityData, position: Vec3d) -> i32 {
    match data {
        EntityData::BlockDisplay(data) => crate::entity::spawn_block_display(store, position, data),
        EntityData::ItemDisplay(data) => crate::entity::spawn_item_display(store, position, data),
        EntityData::TextDisplay(data) => crate::entity::spawn_text_display(store, position, data),
        _ => panic!("a display variant"),
    }
    .expect("a fresh UUID")
}

fn reload(store: &EntityStore) -> (EntityStore, i32) {
    let document = write_entity_chunk(store, (0, 0)).expect("the save form is well formed");
    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let [id] = ids.as_slice() else {
        panic!("one entity reloads")
    };
    (reloaded, *id)
}

#[test]
fn a_spawned_block_display_carries_the_defined_defaults() {
    // `Display.defineSynchedData` (`Display.java:195-211`) and
    // `BlockDisplay`'s air row (`:473-476`).
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = spawn(
        &mut store,
        EntityData::BlockDisplay(BlockDisplayData::new()),
        origin(),
    );
    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID
    );
    assert_eq!(entity.gravity(), 0.0);
    const _: () = assert!(!display::IS_PICKABLE);
    const _: () = assert!(!display::CAN_BE_HIT_BY_PROJECTILE);
    let rows = display::synched_values(entity);
    let entity_rows = crate::entity::living::entity_synched_values(entity).len();
    assert_eq!(entity_rows, 8);
    assert_eq!(rows.len(), entity_rows + 15 + 1);
    assert!(rows.iter().all(|row| row.is_default));
    assert!(crate::entity::tracking::non_default_synched_values(entity).is_empty());
    let expected: Vec<(u8, i32, EntityDataValueKind)> = vec![
        (
            DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(0),
        ),
        (
            DISPLAY_TRANSFORMATION_INTERPOLATION_DURATION_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(0),
        ),
        (
            DISPLAY_POS_ROT_INTERPOLATION_DURATION_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(0),
        ),
        (
            DISPLAY_TRANSLATION_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
            EntityDataValueKind::Vector3f {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ),
        (
            DISPLAY_SCALE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_VECTOR3_ID,
            EntityDataValueKind::Vector3f {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        ),
        (
            DISPLAY_LEFT_ROTATION_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID,
            EntityDataValueKind::Quaternionf {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        ),
        (
            DISPLAY_RIGHT_ROTATION_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_QUATERNION_ID,
            EntityDataValueKind::Quaternionf {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        ),
        (
            DISPLAY_BILLBOARD_RENDER_CONSTRAINTS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            EntityDataValueKind::Byte(0),
        ),
        (
            DISPLAY_BRIGHTNESS_OVERRIDE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(-1),
        ),
        (
            DISPLAY_VIEW_RANGE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(1.0),
        ),
        (
            DISPLAY_SHADOW_RADIUS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(0.0),
        ),
        (
            DISPLAY_SHADOW_STRENGTH_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(1.0),
        ),
        (
            DISPLAY_WIDTH_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(0.0),
        ),
        (
            DISPLAY_HEIGHT_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(0.0),
        ),
        (
            DISPLAY_GLOW_COLOR_OVERRIDE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(-1),
        ),
        (
            BLOCK_DISPLAY_BLOCK_STATE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID,
            EntityDataValueKind::BlockState(state_id("minecraft:air")),
        ),
    ];
    for (row, (data_id, serializer_id, value)) in rows[entity_rows..].iter().zip(&expected) {
        assert_eq!(row.value.data_id, *data_id);
        assert_eq!(row.value.serializer_id, *serializer_id, "row {data_id}");
        assert_eq!(&row.value.value, value, "row {data_id}");
    }
}

#[test]
fn the_item_and_text_rows_follow_the_shared_fifteen() {
    // `ItemDisplay.defineSynchedData` (`Display.java:568-572`) and
    // `TextDisplay`'s (`:693-700`).
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let item_id = spawn(
        &mut store,
        EntityData::ItemDisplay(ItemDisplayData::new().with_item(
            plain_stack("minecraft:diamond_sword", 1).expect("a built-in stack"),
            ItemDisplayContext::FirstPersonLeftHand,
        )),
        origin(),
    );
    let entity = store.get(item_id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID);
    let rows = display::synched_values(entity);
    let entity_rows = crate::entity::living::entity_synched_values(entity).len();
    assert_eq!(rows.len(), entity_rows + 15 + 2);
    let [stack, context] = &rows[entity_rows + 15..] else {
        panic!("two item display rows")
    };
    assert_eq!(stack.value.data_id, ITEM_DISPLAY_ITEM_STACK_DATA_ID);
    assert_eq!(
        stack.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID
    );
    assert!(!stack.is_default);
    let EntityDataValueKind::ItemStack(summary) = &stack.value.value else {
        panic!("an item stack row")
    };
    assert_eq!(summary.count, 1);
    assert_eq!(context.value.data_id, ITEM_DISPLAY_ITEM_DISPLAY_DATA_ID);
    assert_eq!(
        context.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
    );
    assert_eq!(
        context.value.value,
        EntityDataValueKind::Byte(ItemDisplayContext::FirstPersonLeftHand.id())
    );
    assert!(!context.is_default);

    let text_id = spawn(
        &mut store,
        EntityData::TextDisplay(TextDisplayData::new()),
        origin(),
    );
    let entity = store.get(text_id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID);
    let rows = display::synched_values(entity);
    assert_eq!(rows.len(), entity_rows + 15 + 5);
    assert!(rows.iter().all(|row| row.is_default));
    let expected: Vec<(u8, i32, EntityDataValueKind)> = vec![
        (
            TEXT_DISPLAY_TEXT_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
            EntityDataValueKind::Component(RichTextComponent::plain("")),
        ),
        (
            TEXT_DISPLAY_LINE_WIDTH_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(200),
        ),
        (
            TEXT_DISPLAY_BACKGROUND_COLOR_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(0x4000_0000),
        ),
        (
            TEXT_DISPLAY_TEXT_OPACITY_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            EntityDataValueKind::Byte(-1),
        ),
        (
            TEXT_DISPLAY_STYLE_FLAGS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            EntityDataValueKind::Byte(0),
        ),
    ];
    for (row, (data_id, serializer_id, value)) in rows[entity_rows + 15..].iter().zip(&expected) {
        assert_eq!(row.value.data_id, *data_id);
        assert_eq!(row.value.serializer_id, *serializer_id, "row {data_id}");
        assert_eq!(&row.value.value, value, "row {data_id}");
    }
    assert_ne!(
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
    );
}

#[test]
fn displays_never_tick_and_the_culling_size_rows_never_touch_the_box() {
    // `Display.tick()`'s server half is empty of owned effects (`:152-185`
    // runs interpolation bookkeeping for the client renderer and the
    // passenger check for a vehicle this crate does not model), and `width`/
    // `height` only feed `updateCulling` (`:562`, client-side): the collision
    // box stays the catalog's `0 x 0` whatever the rows say.
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let deep = Vec3d {
        x: 0.5,
        y: -164.0,
        z: 0.5,
    };
    let ids = [
        spawn(
            &mut store,
            EntityData::BlockDisplay(BlockDisplayData::new().with_display(loaded_display())),
            deep,
        ),
        spawn(
            &mut store,
            EntityData::ItemDisplay(ItemDisplayData::new().with_display(loaded_display())),
            deep,
        ),
        spawn(
            &mut store,
            EntityData::TextDisplay(TextDisplayData::new().with_display(loaded_display())),
            deep,
        ),
    ];
    for _ in 0..3 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    for id in ids {
        let entity = store.get(id).unwrap();
        assert!(entity.is_alive());
        assert_eq!(entity.position(), deep);
        assert_eq!(entity.bounding_box(), Aabb::of_entity(deep, 0.0, 0.0));
        assert_eq!(display::display(entity).unwrap().width(), 3.0);
    }
}

#[test]
fn a_display_cannot_be_hurt() {
    let mut level = NativeLevel::new();
    for data in [
        EntityData::BlockDisplay(BlockDisplayData::new()),
        EntityData::ItemDisplay(ItemDisplayData::new()),
        EntityData::TextDisplay(TextDisplayData::new()),
    ] {
        let entity_type =
            vanilla_entity_type_id_for_resource_id(data.entity_resource_id()).unwrap();
        let (id, _) =
            level
                .world
                .spawn_stored_entity_for_test(DIMENSION, entity_type, origin(), data);
        let outcome = level
            .world
            .hurt_stored_entity_by_command_for_test(
                DIMENSION,
                id,
                "minecraft:player_attack",
                1.0e9,
                None,
            )
            .unwrap();
        assert!(!outcome.applied && !outcome.succeeded);
        assert!(!level
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .hurt_marked());
        assert!(level
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .is_alive());
    }
}

#[test]
fn every_block_display_field_round_trips() {
    // `Display.addAdditionalSaveData` (`Display.java:238-250`) and
    // `BlockDisplay`'s `block_state` (`:501-504`).
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let slab = state_with(
        "minecraft:oak_slab",
        &[("type", "top"), ("waterlogged", "false")],
    );
    spawn(
        &mut store,
        EntityData::BlockDisplay(
            BlockDisplayData::new()
                .with_display(loaded_display())
                .with_block_state(slab),
        ),
        origin(),
    );
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:block_display"
    );
    let record = document.entities().next().unwrap().raw();
    assert_eq!(
        field(record, "billboard"),
        Some(&NbtSummaryValue::string("center"))
    );
    assert_eq!(
        field(record, "interpolation_duration"),
        Some(&NbtSummaryValue::Int(5))
    );
    // `Mth.clamp(teleportDuration, 0, 59)` (`Display.java:218-219`) already
    // clamped the builder's `100`.
    assert_eq!(
        field(record, "teleport_duration"),
        Some(&NbtSummaryValue::Int(59))
    );
    assert_eq!(
        field(record, "view_range"),
        Some(&NbtSummaryValue::Float(2.5_f32.to_bits()))
    );
    assert_eq!(
        field(record, "glow_color_override"),
        Some(&NbtSummaryValue::Int(0x00ff_00ff))
    );
    assert_eq!(
        field(record, "brightness"),
        Some(&NbtSummaryValue::Compound(vec![
            entry("block", NbtSummaryValue::Int(7)),
            entry("sky", NbtSummaryValue::Int(15)),
        ]))
    );
    assert_eq!(
        field(record, "transformation"),
        Some(&NbtSummaryValue::Compound(vec![
            entry(
                "translation",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(0.5_f32.to_bits()),
                    NbtSummaryValue::Float((-1.25_f32).to_bits()),
                    NbtSummaryValue::Float(2.0_f32.to_bits()),
                ])
            ),
            entry(
                "left_rotation",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                    NbtSummaryValue::Float((-1.0_f32).to_bits()),
                ])
            ),
            entry(
                "scale",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(2.0_f32.to_bits()),
                    NbtSummaryValue::Float(4.0_f32.to_bits()),
                    NbtSummaryValue::Float(8.0_f32.to_bits()),
                ])
            ),
            entry(
                "right_rotation",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                    NbtSummaryValue::Float(1.0_f32.to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                ])
            ),
        ]))
    );
    assert_eq!(
        field(record, "block_state"),
        Some(&NbtSummaryValue::Compound(vec![
            entry("Name", NbtSummaryValue::string("minecraft:oak_slab")),
            entry(
                "Properties",
                NbtSummaryValue::Compound(
                    bbb_protocol::block_state_types::vanilla_block_state_identity(slab)
                        .expect("the slab state is in the catalog")
                        .properties
                        .iter()
                        .map(|(name, value)| entry(name, NbtSummaryValue::string(value.clone())))
                        .collect(),
                ),
            ),
        ]))
    );

    // `start_interpolation` is read (`Display.java:217`) but never written
    // (`:238-250`), so a save/load cycle resets the delay to 0.
    assert_eq!(field(record, "start_interpolation"), None);

    let (mut reloaded, reloaded_id) = reload(&store);
    let after = reloaded.get_mut(reloaded_id).unwrap();
    assert_eq!(after.bounding_box(), Aabb::of_entity(origin(), 0.0, 0.0));
    let EntityData::BlockDisplay(after_data) = after.data().clone() else {
        panic!("a block display")
    };
    assert_eq!(
        after_data,
        BlockDisplayData::new()
            .with_display(reloaded_display())
            .with_block_state(slab)
    );
    assert_eq!(after_data.block_state_id(), slab);
    assert_eq!(
        after_data.display().brightness_override(),
        Some(Brightness { block: 7, sky: 15 })
    );
    assert_eq!(
        after_data.display().packed_brightness_override(),
        (7 << 4) | (15 << 20)
    );
    // Every loaded row was `set` away from its default, so every display row
    // is dirty for the first `sendChanges`, and row 8 is dirty regardless
    // (`Display.java:282-284` sets it with `force`).
    let dirty = after.take_synched_dirty();
    assert_eq!(
        dirty,
        (DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID
            ..=BLOCK_DISPLAY_BLOCK_STATE_DATA_ID)
            .collect::<Vec<u8>>()
    );

    // A default block display loads with only the forced row dirty.
    let mut fresh = EntityStore::new(9, EntityIdCounter::new());
    spawn(
        &mut fresh,
        EntityData::BlockDisplay(BlockDisplayData::new()),
        origin(),
    );
    let (mut reloaded, id) = reload(&fresh);
    assert_eq!(
        reloaded.get_mut(id).unwrap().take_synched_dirty(),
        vec![DISPLAY_TRANSFORMATION_INTERPOLATION_START_DELTA_TICKS_DATA_ID]
    );
}

#[test]
fn the_item_display_stack_and_context_round_trip_and_an_empty_stack_is_omitted() {
    // `ItemDisplay.addAdditionalSaveData` (`Display.java:606-614`) and its
    // reader (`:599-603`).
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    spawn(
        &mut store,
        EntityData::ItemDisplay(
            ItemDisplayData::new()
                .with_display(loaded_display())
                .with_item(
                    plain_stack("minecraft:diamond_sword", 1).expect("a built-in stack"),
                    ItemDisplayContext::Fixed,
                ),
        ),
        origin(),
    );
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:item_display"
    );
    let record = document.entities().next().unwrap().raw();
    assert!(matches!(
        field(record, "item"),
        Some(NbtSummaryValue::Compound(_))
    ));
    assert_eq!(
        field(record, "item_display"),
        Some(&NbtSummaryValue::string("fixed"))
    );
    let (reloaded, reloaded_id) = reload(&store);
    let EntityData::ItemDisplay(after) = reloaded.get(reloaded_id).unwrap().data() else {
        panic!("an item display")
    };
    assert_eq!(
        after,
        &ItemDisplayData::new()
            .with_display(reloaded_display())
            .with_item(
                plain_stack("minecraft:diamond_sword", 1).expect("a built-in stack"),
                ItemDisplayContext::Fixed,
            )
    );
    assert_eq!(after.item_display(), ItemDisplayContext::Fixed);
    assert!(!after.item().is_empty());

    // An empty stack writes no `item` (`:609-611`) and the default context
    // still writes `item_display` (`:613`).
    let mut fresh = EntityStore::new(9, EntityIdCounter::new());
    spawn(
        &mut fresh,
        EntityData::ItemDisplay(ItemDisplayData::new()),
        origin(),
    );
    let document = write_entity_chunk(&fresh, (0, 0)).expect("the save form is well formed");
    let record = document.entities().next().unwrap().raw();
    assert_eq!(field(record, "item"), None);
    assert_eq!(
        field(record, "item_display"),
        Some(&NbtSummaryValue::string("none"))
    );
    let (reloaded, id) = reload(&fresh);
    let EntityData::ItemDisplay(after) = reloaded.get(id).unwrap().data() else {
        panic!("an item display")
    };
    assert!(after.item().is_empty());
    assert_eq!(after.item_display(), ItemDisplayContext::None);

    // A context outside the codec's names is `NONE`.
    let read = display::read_item_display(
        &[entry("item_display", NbtSummaryValue::string("head_slot"))],
        &bbb_registry_codec::UnavailableHolderResolver,
    );
    assert_eq!(read.item_display(), ItemDisplayContext::None);
}

#[test]
fn the_text_display_fields_round_trip_through_the_flag_split() {
    // `TextDisplay.addAdditionalSaveData` (`Display.java:794-805`) splits the
    // flag byte into `shadow`/`see_through`/`default_background` booleans and
    // an `alignment` name; the reader (`:755-772`) reassembles it.
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let flags = FLAG_SHADOW | FLAG_SEE_THROUGH | FLAG_ALIGN_LEFT;
    spawn(
        &mut store,
        EntityData::TextDisplay(
            TextDisplayData::new()
                .with_display(loaded_display())
                .with_text_fields(RichTextComponent::plain("hello"), 120, 0, 127, flags),
        ),
        origin(),
    );
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:text_display"
    );
    let record = document.entities().next().unwrap().raw();
    assert_eq!(
        field(record, "text"),
        Some(&NbtSummaryValue::string("hello"))
    );
    assert_eq!(
        field(record, "line_width"),
        Some(&NbtSummaryValue::Int(120))
    );
    assert_eq!(field(record, "background"), Some(&NbtSummaryValue::Int(0)));
    assert_eq!(
        field(record, "text_opacity"),
        Some(&NbtSummaryValue::Byte(127))
    );
    assert_eq!(field(record, "shadow"), Some(&NbtSummaryValue::Byte(1)));
    assert_eq!(
        field(record, "see_through"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert_eq!(
        field(record, "default_background"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(
        field(record, "alignment"),
        Some(&NbtSummaryValue::string("left"))
    );

    let (reloaded, reloaded_id) = reload(&store);
    let EntityData::TextDisplay(after) = reloaded.get(reloaded_id).unwrap().data() else {
        panic!("a text display")
    };
    assert_eq!(
        after,
        &TextDisplayData::new()
            .with_display(reloaded_display())
            .with_text_fields(RichTextComponent::plain("hello"), 120, 0, 127, flags)
    );
    assert_eq!(after.flags(), flags);
    assert_eq!(after.align(), TextAlign::Left);

    // A center alignment is the absence of both align bits (`:765-770`).
    let read = display::read_text_display(&[entry("alignment", NbtSummaryValue::string("center"))]);
    assert_eq!(read.flags(), 0);
    assert_eq!(read.align(), TextAlign::Center);
    // A text that does not decode as a component is `Component.empty()`
    // (`input.read` answers nothing and the constructor default stands).
    let read = display::read_text_display(&[entry("text", NbtSummaryValue::Int(7))]);
    assert_eq!(read.text(), &RichTextComponent::plain(""));
}

#[test]
fn display_decode_alternatives_load() {
    // `ExtraCodecs.AXISANGLE4F` (`ExtraCodecs.java:116-119`) is a JOML
    // `Quaternionf(AxisAngle4f)`; the sixteen-float `MATRIX4F` alternative of
    // `Transformation.EXTENDED_CODEC` (`Transformation.java:33-35`) reads as
    // `IDENTITY`.
    let axis_angle = display::read_block_display(&[entry(
        "transformation",
        NbtSummaryValue::Compound(vec![
            entry(
                "translation",
                NbtSummaryValue::List(vec![NbtSummaryValue::Float(0); 3]),
            ),
            entry(
                "left_rotation",
                NbtSummaryValue::Compound(vec![
                    entry("angle", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    entry(
                        "axis",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Float(0); 3]),
                    ),
                ]),
            ),
            entry(
                "scale",
                NbtSummaryValue::List(vec![NbtSummaryValue::Float(0); 3]),
            ),
            entry(
                "right_rotation",
                NbtSummaryValue::List(vec![NbtSummaryValue::Float(0); 4]),
            ),
        ]),
    )]);
    assert_eq!(
        axis_angle.display().transformation().left_rotation,
        Quaternionf::new(0.0, 0.0, 0.0, 0.5_f32.cos())
    );

    let identity_matrix = display::read_text_display(&[entry(
        "transformation",
        NbtSummaryValue::List(
            [
                1.0_f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]
            .into_iter()
            .map(|v| NbtSummaryValue::Float(v.to_bits()))
            .collect(),
        ),
    )]);
    assert_eq!(
        identity_matrix.display().transformation(),
        Transformation::IDENTITY
    );
    let translated_matrix = display::read_text_display(&[entry(
        "transformation",
        NbtSummaryValue::List(
            [
                1.0_f32, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 1.0, 3.0, 0.0, 0.0, 0.0, 1.0,
            ]
            .into_iter()
            .map(|v| NbtSummaryValue::Float(v.to_bits()))
            .collect(),
        ),
    )]);
    assert_eq!(
        translated_matrix.display().transformation().translation,
        Vector3f {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        }
    );
}

#[test]
fn quaternions_are_normalized_the_way_joml_does() {
    // JOML 1.10.8 `Quaternionf.normalize()`: exact in the cases where the
    // float arithmetic is exact.
    assert_eq!(
        Quaternionf::new(0.0, 0.0, 0.0, 2.0).normalized(),
        Quaternionf::IDENTITY
    );
    assert_eq!(
        Quaternionf::new(0.0, -8.0, 0.0, 0.0).normalized(),
        Quaternionf::new(0.0, -1.0, 0.0, 0.0)
    );
    let normalized = Quaternionf::new(1.0, 2.0, 3.0, 4.0).normalized();
    let inv_norm = 1.0_f32
        / (f64::from(1.0_f32 * 1.0 + (2.0_f32 * 2.0 + (3.0_f32 * 3.0 + 4.0_f32 * 4.0))).sqrt()
            as f32);
    assert_eq!(
        normalized.components(),
        [
            1.0 * inv_norm,
            2.0 * inv_norm,
            3.0 * inv_norm,
            4.0 * inv_norm
        ]
    );
    // The reader normalizes what it reads (`ExtraCodecs.java:110-114`).
    let read = display::read_block_display(&[entry(
        "transformation",
        NbtSummaryValue::Compound(vec![
            entry(
                "translation",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(0); 3]),
            ),
            entry(
                "left_rotation",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                    NbtSummaryValue::Float(0.0_f32.to_bits()),
                    NbtSummaryValue::Float(2.0_f32.to_bits()),
                ]),
            ),
            entry(
                "scale",
                NbtSummaryValue::List(vec![NbtSummaryValue::Int(1); 3]),
            ),
            entry(
                "right_rotation",
                NbtSummaryValue::List(vec![NbtSummaryValue::Float(0); 4]),
            ),
        ]),
    )]);
    assert_eq!(
        read.display().transformation().left_rotation,
        Quaternionf::IDENTITY
    );
}

#[test]
fn the_billboard_and_context_and_align_names_are_the_codec_identifiers() {
    // `BillboardConstraints` (`Display.java:433-459`), `ItemDisplayContext`
    // (`ItemDisplayContext.java:9-18`) and `TextDisplay.Align`
    // (`Display.java:864-880`) all serialize by lowercase name; ids are the
    // enum ordinals the byte rows carry.
    for (constraints, id, name) in [
        (BillboardConstraints::Fixed, 0, "fixed"),
        (BillboardConstraints::Vertical, 1, "vertical"),
        (BillboardConstraints::Horizontal, 2, "horizontal"),
        (BillboardConstraints::Center, 3, "center"),
    ] {
        assert_eq!(constraints.id(), id);
        assert_eq!(constraints.serialized_name(), name);
        assert_eq!(
            BillboardConstraints::for_serialized_name(name),
            Some(constraints)
        );
    }
    assert_eq!(BillboardConstraints::for_serialized_name("CENTER"), None);
    for (context, id, name) in [
        (ItemDisplayContext::None, 0, "none"),
        (
            ItemDisplayContext::ThirdPersonLeftHand,
            1,
            "thirdperson_lefthand",
        ),
        (
            ItemDisplayContext::ThirdPersonRightHand,
            2,
            "thirdperson_righthand",
        ),
        (
            ItemDisplayContext::FirstPersonLeftHand,
            3,
            "firstperson_lefthand",
        ),
        (
            ItemDisplayContext::FirstPersonRightHand,
            4,
            "firstperson_righthand",
        ),
        (ItemDisplayContext::Head, 5, "head"),
        (ItemDisplayContext::Gui, 6, "gui"),
        (ItemDisplayContext::Ground, 7, "ground"),
        (ItemDisplayContext::Fixed, 8, "fixed"),
        (ItemDisplayContext::OnShelf, 9, "on_shelf"),
    ] {
        assert_eq!(context.id(), id);
        assert_eq!(context.serialized_name(), name);
        assert_eq!(ItemDisplayContext::for_serialized_name(name), Some(context));
    }
    for (align, name, flags) in [
        (TextAlign::Center, "center", 0),
        (TextAlign::Left, "left", FLAG_ALIGN_LEFT),
        (TextAlign::Right, "right", display::FLAG_ALIGN_RIGHT),
    ] {
        assert_eq!(align.serialized_name(), name);
        assert_eq!(TextAlign::for_serialized_name(name), Some(align));
        assert_eq!(TextAlign::of_flags(flags), align);
    }
    // Both bits set: `getAlign` tests LEFT first (`Display.java:715-723`).
    assert_eq!(
        TextAlign::of_flags(FLAG_ALIGN_LEFT | display::FLAG_ALIGN_RIGHT),
        TextAlign::Left
    );
    assert_eq!(
        Brightness::unpack(Brightness { block: 7, sky: 15 }.pack()),
        Brightness { block: 7, sky: 15 }
    );
}

mod no_physics;
