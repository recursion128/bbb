use std::collections::BTreeMap;

use bbb_protocol::block_entity_types::{
    VANILLA_BLOCK_ENTITY_TYPE_BARREL_ID, VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
};
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::NbtSummaryValue;

use bbb_protocol::item_types::{
    VANILLA_ITEM_GLOW_INK_SAC_ID, VANILLA_ITEM_HONEYCOMB_ID, VANILLA_ITEM_INK_SAC_ID,
    VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_STONE_ID,
};
use bbb_protocol::packets::ItemStackSummary;

use super::sign::{
    can_apply_to_sign, has_editable_text, is_facing_front_text, sign_applicator, strip_formatting,
    try_apply_to_sign, SignApplicator, SignBlockEntity, SignText, SignTextUpdate,
};
use super::*;
use crate::chunk::ChunkStore;
use bbb_server_types::item_stack::RuntimeItemStack;

fn runtime_stack(summary: ItemStackSummary) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(summary)
}

fn state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect::<BTreeMap<_, _>>();
    vanilla_block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("{name} {properties:?} is a vanilla 26.1 block state"))
}

fn oak_sign(rotation: &str) -> i32 {
    state_of(
        "minecraft:oak_sign",
        &[("rotation", rotation), ("waterlogged", "false")],
    )
}

fn oak_wall_sign(facing: &str) -> i32 {
    state_of(
        "minecraft:oak_wall_sign",
        &[("facing", facing), ("waterlogged", "false")],
    )
}

fn air() -> i32 {
    state_of("minecraft:air", &[])
}

fn comparator_state() -> i32 {
    state_of(
        "minecraft:comparator",
        &[
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "false"),
        ],
    )
}

/// `ComparatorBlock.newBlockEntity` and `ComparatorBlockEntity`
/// persistence (`ComparatorBlock.java:185-194`, `ComparatorBlockEntity.java:8-34`).
#[test]
fn comparator_output_defaults_and_round_trips() {
    let mut block_entity =
        new_block_entity(pos(1, 2, 3), comparator_state()).expect("a comparator object");
    assert_eq!(
        block_entity.type_id(),
        VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID
    );
    assert_eq!(block_entity.comparator().unwrap().output_signal(), 0);

    block_entity.comparator_mut().unwrap().set_output_signal(13);
    let record = block_entity
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert_eq!(
        unique_field(&record, "OutputSignal"),
        Some(&NbtSummaryValue::Int(13))
    );
    let loaded = crate::block_entity::load_test_fixture(pos(1, 2, 3), comparator_state(), &record)
        .expect("fixture codec succeeds")
        .expect("the record names a comparator");
    assert_eq!(loaded.comparator().unwrap().output_signal(), 13);
}

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

/// `BarrelBlock.newBlockEntity` (`BarrelBlock.java:69-71`) builds a
/// `BarrelBlockEntity` with 27 slots, the same `Items` schema a chest uses.
#[test]
fn the_barrel_type_builds_a_barrel_block_entity() {
    let default_state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_BARREL_ID,
    )
    .expect("minecraft:barrel has a default state");
    let built =
        new_block_entity(pos(0, 0, 0), default_state).expect("the barrel type builds an object");
    assert_eq!(built.type_id(), VANILLA_BLOCK_ENTITY_TYPE_BARREL_ID);
    let barrel = built.barrel().expect("the payload is a barrel");
    assert_eq!(barrel.items().len(), 27);
    assert!(built.chest().is_none());
}

/// `CopperChestBlock.shouldChangedStateKeepBlockEntity`
/// (`CopperChestBlock.java:142-144`): the oxidation step replaces the block and
/// keeps the container. Everything else loses its block entity when the block
/// changes.
#[test]
fn only_the_two_copper_families_keep_a_block_entity_across_a_block_change() {
    let copper_chest = state_of(
        "minecraft:copper_chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let exposed_copper_chest = state_of(
        "minecraft:exposed_copper_chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    assert!(should_changed_state_keep_block_entity(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        copper_chest,
        exposed_copper_chest
    ));
    assert!(!should_changed_state_keep_block_entity(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        copper_chest,
        state_of("minecraft:stone", &[])
    ));
    assert!(!should_changed_state_keep_block_entity(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        oak_sign("0"),
        oak_wall_sign("north")
    ));
}

#[test]
fn a_written_sign_record_loads_back_as_the_same_object() {
    let mut sign = SignBlockEntity::default();
    let uuid = uuid::Uuid::from_u128(3);
    sign.set_allowed_player_editor(Some(uuid));
    assert_eq!(
        sign.update_sign_text(
            uuid,
            false,
            true,
            &[
                "hello".to_string(),
                "world".to_string(),
                String::new(),
                String::new(),
            ],
        ),
        SignTextUpdate::Updated
    );
    let mut block_entity = new_block_entity(pos(1, -60, 2), oak_sign("4")).unwrap();
    *block_entity.sign_mut().unwrap() = sign;

    let record = saved_record(&block_entity).expect("fixture codec succeeds");
    assert_eq!(
        unique_field(&record, "id"),
        Some(&NbtSummaryValue::string("minecraft:sign"))
    );
    assert_eq!(unique_field(&record, "x"), Some(&NbtSummaryValue::Int(1)));
    assert_eq!(unique_field(&record, "y"), Some(&NbtSummaryValue::Int(-60)));
    assert_eq!(unique_field(&record, "z"), Some(&NbtSummaryValue::Int(2)));
    // `LevelChunk.getBlockEntityNbtForSaving` (`LevelChunk.java:456`).
    assert_eq!(
        unique_field(&record, "keepPacked"),
        Some(&NbtSummaryValue::Byte(0))
    );
    // `BlockEntity.saveWithoutMetadata` always stores the component map, and
    // the save schema always requires it.
    assert_eq!(
        unique_field(&record, "components"),
        Some(&NbtSummaryValue::Compound(Vec::new()))
    );

    let loaded = crate::block_entity::load_test_fixture(pos(1, -60, 2), oak_sign("4"), &record)
        .expect("fixture codec succeeds")
        .unwrap();
    assert_eq!(loaded, block_entity);
    assert_eq!(
        loaded.sign().unwrap().text(true).plain_lines(false),
        ["hello", "world", "", ""].map(str::to_string)
    );
    // The back face was never written, so it is still four empty lines.
    assert_eq!(
        loaded.sign().unwrap().text(false).plain_lines(false),
        ["", "", "", ""].map(str::to_string)
    );
}

/// `SignBlockEntity.setMessages` builds
/// `Component.literal(line).setStyle(currentTextStyle)`
/// (`SignBlockEntity.java:150-154`), so an edit keeps whatever style the line
/// it replaces carried and drops nothing else.
#[test]
fn an_edited_line_keeps_the_style_of_the_line_it_replaces() {
    let styled = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("old")),
        entry("color", NbtSummaryValue::string("red")),
        entry("bold", NbtSummaryValue::Byte(1)),
    ]);
    let text = SignText::load(&NbtSummaryValue::Compound(vec![
        entry(
            "messages",
            NbtSummaryValue::List(vec![
                styled,
                NbtSummaryValue::string(String::new()),
                NbtSummaryValue::string(String::new()),
                NbtSummaryValue::string(String::new()),
            ]),
        ),
        entry("color", NbtSummaryValue::string("black")),
        entry("has_glowing_text", NbtSummaryValue::Byte(0)),
    ]))
    .unwrap();

    let mut sign = SignBlockEntity::default();
    sign.set_text(text, true);
    let uuid = uuid::Uuid::from_u128(1);
    sign.set_allowed_player_editor(Some(uuid));
    sign.update_sign_text(
        uuid,
        false,
        true,
        &[
            "new".to_string(),
            "plain".to_string(),
            String::new(),
            String::new(),
        ],
    );

    assert_eq!(
        sign.text(true).message(0, false),
        &NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::string("new")),
            entry("color", NbtSummaryValue::string("red")),
            entry("bold", NbtSummaryValue::Byte(1)),
        ])
    );
    // A line whose predecessor had no style collapses back to a bare string,
    // which is what `Component.tryCollapseToString` produces.
    assert_eq!(
        sign.text(true).message(1, false),
        &NbtSummaryValue::string("plain")
    );
}

fn item(id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(id),
        count: 1,
        ..ItemStackSummary::empty()
    }
}

/// The four `SignApplicator` classes (`SignApplicator.java:8-14`) and the
/// default `canApplyToSign` that dyes and inks share.
#[test]
fn the_four_sign_applicators_are_the_dye_ink_glow_and_honeycomb_items() {
    assert_eq!(
        sign_applicator(&item(VANILLA_ITEM_RED_DYE_ID)),
        Some(SignApplicator::Dye {
            color: "red".to_string()
        })
    );
    assert_eq!(
        sign_applicator(&item(VANILLA_ITEM_INK_SAC_ID)),
        Some(SignApplicator::InkSac)
    );
    assert_eq!(
        sign_applicator(&item(VANILLA_ITEM_GLOW_INK_SAC_ID)),
        Some(SignApplicator::GlowInkSac)
    );
    assert_eq!(
        sign_applicator(&item(VANILLA_ITEM_HONEYCOMB_ID)),
        Some(SignApplicator::Honeycomb)
    );
    assert_eq!(sign_applicator(&item(VANILLA_ITEM_STONE_ID)), None);
}

#[test]
fn dyes_and_inks_need_a_message_and_honeycomb_does_not() {
    let empty = SignText::default();
    let dye = SignApplicator::Dye {
        color: "red".to_string(),
    };
    assert!(!can_apply_to_sign(&dye, &empty, false));
    assert!(!can_apply_to_sign(&SignApplicator::InkSac, &empty, false));
    assert!(!can_apply_to_sign(
        &SignApplicator::GlowInkSac,
        &empty,
        false
    ));
    assert!(can_apply_to_sign(&SignApplicator::Honeycomb, &empty, false));
}

/// `DyeItem`/`InkSacItem`/`GlowInkSacItem`/`HoneycombItem.tryApplyToSign`.
#[test]
fn applying_the_four_applicators_mutates_only_the_selected_face() {
    let mut sign = SignBlockEntity::default();
    sign.set_allowed_player_editor(Some(uuid::Uuid::from_u128(1)));
    assert_eq!(
        sign.update_sign_text(
            uuid::Uuid::from_u128(1),
            false,
            true,
            &[
                "front".to_string(),
                String::new(),
                String::new(),
                String::new()
            ]
        ),
        SignTextUpdate::Updated
    );
    let dye = SignApplicator::Dye {
        color: "blue".to_string(),
    };
    assert!(try_apply_to_sign(&mut sign, &dye, true));
    assert!(!try_apply_to_sign(&mut sign, &dye, true));
    assert_eq!(sign.text(true).color(), "blue");
    assert_eq!(sign.text(false).color(), "black");
    assert!(try_apply_to_sign(
        &mut sign,
        &SignApplicator::GlowInkSac,
        true
    ));
    assert!(sign.text(true).has_glowing_text());
    assert!(!sign.text(false).has_glowing_text());
    assert!(try_apply_to_sign(&mut sign, &SignApplicator::InkSac, true));
    assert!(!sign.text(true).has_glowing_text());
    assert!(try_apply_to_sign(
        &mut sign,
        &SignApplicator::Honeycomb,
        true
    ));
    assert!(sign.is_waxed());
    assert!(!try_apply_to_sign(
        &mut sign,
        &SignApplicator::Honeycomb,
        true
    ));
}

/// `SignBlockEntity.updateSignText` refuses a write from anyone but the player
/// who claimed the editor, and refuses every write to a waxed sign
/// (`SignBlockEntity.java:131-137`).
#[test]
fn a_sign_write_needs_the_editor_claim_and_an_unwaxed_sign() {
    let lines = ["a".to_string(), String::new(), String::new(), String::new()];
    let mut sign = SignBlockEntity::default();
    assert_eq!(
        sign.update_sign_text(uuid::Uuid::from_u128(1), false, true, &lines),
        SignTextUpdate::Refused,
        "no claim, no write"
    );
    sign.set_allowed_player_editor(Some(uuid::Uuid::from_u128(1)));
    assert_eq!(
        sign.update_sign_text(uuid::Uuid::from_u128(2), false, true, &lines),
        SignTextUpdate::Refused,
        "another player's claim is not this player's"
    );
    assert_eq!(
        sign.update_sign_text(uuid::Uuid::from_u128(1), false, true, &lines),
        SignTextUpdate::Updated
    );
    // `:133`: the claim is released by the write.
    assert_eq!(sign.player_who_may_edit(), None);
    assert_eq!(sign.text(true).plain_lines(false)[0], "a");
}

/// `SignBlockEntity.tick` (`SignBlockEntity.java:265-276`).
#[test]
fn the_editor_claim_survives_a_tick_in_range_and_not_one_out_of_range() {
    let mut sign = SignBlockEntity::default();
    sign.set_allowed_player_editor(Some(uuid::Uuid::from_u128(5)));
    sign.tick(|_| false);
    assert_eq!(sign.player_who_may_edit(), Some(uuid::Uuid::from_u128(5)));
    sign.tick(|_| true);
    assert_eq!(sign.player_who_may_edit(), None);
}

/// `SignBlock.hasEditableText` (`SignBlock.java:154-158`): a line whose
/// contents are not plain text closes the editor.
#[test]
fn a_translatable_line_makes_a_sign_uneditable() {
    let translatable = NbtSummaryValue::Compound(vec![entry(
        "translate",
        NbtSummaryValue::string("block.minecraft.stone"),
    )]);
    let text = SignText::load(&NbtSummaryValue::Compound(vec![
        entry(
            "messages",
            NbtSummaryValue::List(vec![
                translatable,
                NbtSummaryValue::string(String::new()),
                NbtSummaryValue::string(String::new()),
                NbtSummaryValue::string(String::new()),
            ]),
        ),
        entry("color", NbtSummaryValue::string("black")),
        entry("has_glowing_text", NbtSummaryValue::Byte(0)),
    ]))
    .unwrap();
    assert!(!has_editable_text(&text, false));
    assert!(has_editable_text(&SignText::default(), false));
}

/// `SignBlockEntity.isFacingFrontText` (`SignBlockEntity.java:60-71`) against
/// a standing sign turned to `rotation` 0, whose `getYRotationDegrees` is 0 —
/// the face looking south.
#[test]
fn a_player_south_of_a_rotation_zero_sign_faces_its_front() {
    let sign = pos(0, 64, 0);
    assert!(is_facing_front_text(oak_sign("0"), false, sign, 0.5, 4.5,));
    assert!(!is_facing_front_text(oak_sign("0"), false, sign, 0.5, -3.5,));
    // `rotation` 8 is 180 degrees, so the two answers swap.
    assert!(!is_facing_front_text(oak_sign("8"), false, sign, 0.5, 4.5));
    assert!(is_facing_front_text(oak_sign("8"), false, sign, 0.5, -3.5));
}

/// `WallSignBlock.getYRotationDegrees` is `FACING.toYRot()`
/// (`WallSignBlock.java:91-93`), and the hitbox centre it measures from sits
/// against the wall (`:95-98`).
#[test]
fn a_wall_sign_faces_the_player_standing_in_front_of_it() {
    let sign = pos(0, 64, 0);
    // Facing north means the sign's face points north, i.e. towards -z.
    assert!(is_facing_front_text(
        oak_wall_sign("north"),
        false,
        sign,
        0.5,
        -3.5,
    ));
    assert!(!is_facing_front_text(
        oak_wall_sign("north"),
        false,
        sign,
        0.5,
        4.5,
    ));
    assert!(is_facing_front_text(
        oak_wall_sign("east"),
        false,
        sign,
        4.5,
        0.5,
    ));
}

/// `ChatFormatting.stripFormatting` (`ChatFormatting.java:48`, `:107-109`).
#[test]
fn strip_formatting_removes_section_codes_and_nothing_else() {
    assert_eq!(strip_formatting("§chello §rworld"), "hello world");
    assert_eq!(strip_formatting("§Kobfuscated"), "obfuscated");
    // `§` followed by something outside the class stays put, and so does a
    // trailing `§`.
    assert_eq!(strip_formatting("§zkeep§"), "§zkeep§");
    assert_eq!(strip_formatting("plain"), "plain");
}

/// `LevelChunk.setBlockState`'s two block-entity arms, driven straight over
/// the map: a sign appears with its block, keeps its object when only a
/// property changes, and takes its record with it when the block goes.
#[test]
fn the_set_block_state_arms_create_keep_and_drop_a_signs_record() {
    let mut store = loaded_store();
    let at = pos(3, -60, 5);
    let changed = |store: &ChunkStore| store.save_dirty_epoch((0, 0)).is_some();

    create_block_entity_on_block_change(store.block_entities_mut(), at, oak_sign("0"));
    assert!(store.block_entities().get(at).is_some());
    assert!(changed(&store));

    // Waterlogging is a property-only write: `blockChanged` is false, so the
    // object stays and only its cached state moves.
    let waterlogged = state_of(
        "minecraft:oak_sign",
        &[("rotation", "0"), ("waterlogged", "true")],
    );
    assert_eq!(
        remove_block_entity_on_block_change(
            store.block_entities_mut(),
            at,
            oak_sign("0"),
            waterlogged,
            0,
            crate::registry_tags::RegistryTagSnapshot::builtin()
        ),
        BlockEntityRemovalDrops::None
    );
    create_block_entity_on_block_change(store.block_entities_mut(), at, waterlogged);
    assert_eq!(
        store.block_entities().get(at).map(BlockEntity::block_state),
        Some(waterlogged)
    );

    assert_eq!(
        remove_block_entity_on_block_change(
            store.block_entities_mut(),
            at,
            waterlogged,
            air(),
            0,
            crate::registry_tags::RegistryTagSnapshot::builtin()
        ),
        BlockEntityRemovalDrops::None
    );
    create_block_entity_on_block_change(store.block_entities_mut(), at, air());
    assert!(store.block_entities().get(at).is_none());
    assert!(changed(&store));
}

/// A store with the column at the origin loaded, which every fixture here uses.
fn loaded_store() -> ChunkStore {
    let mut store = ChunkStore::new(
        crate::world_creation::default_overworld_level_source().unwrap(),
        0,
    );
    store.try_ensure_served_full_column(0, 0).unwrap();
    store
}

/// A hanging sign is the same payload behind a different registry type, and
/// the type is what the wire and the save carry.
#[test]
fn a_hanging_sign_saves_under_its_own_registry_id() {
    let hanging = state_of(
        "minecraft:oak_hanging_sign",
        &[
            ("attached", "false"),
            ("rotation", "0"),
            ("waterlogged", "false"),
        ],
    );
    let block_entity = new_block_entity(pos(0, 0, 0), hanging).unwrap();
    assert_eq!(
        block_entity.type_id(),
        VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID
    );
    assert_eq!(
        unique_field(
            &saved_record(&block_entity).expect("fixture codec succeeds"),
            "id"
        ),
        Some(&NbtSummaryValue::string("minecraft:hanging_sign"))
    );
}

/// `ContainerHelper.saveAllItems` (`ContainerHelper.java:25-38`) writes one
/// `ItemStackWithSlot` per *non-empty* stack, and `loadAllItems` (`:40-46`)
/// reads them back by their `Slot` byte — so a sparse chest round-trips
/// through a list shorter than the container.
#[test]
fn a_chest_saves_only_its_non_empty_slots_and_reads_them_back() {
    let chest_state = state_of(
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let mut block_entity =
        new_block_entity(pos(4, 5, 6), chest_state).expect("a chest builds an object");
    let stone = runtime_stack(bbb_protocol::packets::ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 64,
        component_patch: Default::default(),
    });
    let mut items = vec![RuntimeItemStack::empty(); 27];
    items[0] = stone.clone();
    items[26] = runtime_stack(bbb_protocol::packets::ItemStackSummary {
        count: 1,
        ..stone.summary().clone()
    });
    assert!(block_entity
        .chest_mut()
        .expect("a chest object")
        .set_items(items.clone()));

    let record = block_entity
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    let Some(NbtSummaryValue::List(saved)) = unique_field(&record, "Items") else {
        panic!("a chest with no loot table saves its Items list: {record:?}");
    };
    assert_eq!(saved.len(), 2, "empty slots are skipped");
    let NbtSummaryValue::Compound(first) = &saved[0] else {
        panic!("each entry is a compound");
    };
    assert_eq!(unique_field(first, "Slot"), Some(&NbtSummaryValue::Byte(0)));
    assert_eq!(
        unique_field(first, "count"),
        Some(&NbtSummaryValue::Int(64)),
        "`ItemStack.MAP_CODEC`'s fields are inlined beside the slot"
    );
    let NbtSummaryValue::Compound(second) = &saved[1] else {
        panic!("each entry is a compound");
    };
    assert_eq!(
        unique_field(second, "Slot"),
        Some(&NbtSummaryValue::Byte(26))
    );

    let loaded = crate::block_entity::load_test_fixture(pos(4, 5, 6), chest_state, &record)
        .expect("fixture codec succeeds")
        .expect("the record names a chest standing on a chest");
    assert_eq!(loaded.chest().expect("a chest object").items(), items);
    assert_eq!(
        loaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver(),)
            .expect("fixture codec succeeds"),
        record
    );
}

/// `RandomizableContainer.trySaveLootTable` (`RandomizableContainer.java:57-70`)
/// answers true once it has stored a table, which is what makes
/// `ChestBlockEntity.saveAdditional` (`:92-97`) skip the items entirely — and
/// `tryLoadLootTable` (`:50-55`) does the same on the way back in. A chest
/// whose loot is still packed therefore keeps its table through a round trip
/// instead of being written out as an empty chest.
#[test]
fn a_chest_that_still_carries_a_loot_table_keeps_it_instead_of_its_items() {
    let chest_state = state_of(
        "minecraft:chest",
        &[
            ("facing", "east"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let record = vec![
        entry("id", NbtSummaryValue::string("minecraft:chest")),
        entry("x", NbtSummaryValue::Int(0)),
        entry("y", NbtSummaryValue::Int(0)),
        entry("z", NbtSummaryValue::Int(0)),
        entry("components", NbtSummaryValue::Compound(Vec::new())),
        entry(
            "LootTable",
            NbtSummaryValue::string("minecraft:chests/simple_dungeon"),
        ),
        entry("LootTableSeed", NbtSummaryValue::Long(-12345)),
    ];
    let loaded = crate::block_entity::load_test_fixture(pos(0, 0, 0), chest_state, &record)
        .expect("fixture codec succeeds")
        .expect("a chest record loads onto a chest");
    let chest = loaded.chest().expect("a chest object");
    assert!(chest
        .can_open(false, &RuntimeItemStack::empty(), test_item_resolver())
        .expect("fixture lock"));
    assert!(!chest
        .can_open(true, &RuntimeItemStack::empty(), test_item_resolver())
        .expect("fixture lock"));
    let saved = loaded
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert_eq!(
        unique_field(&saved, "LootTable"),
        Some(&NbtSummaryValue::string("minecraft:chests/simple_dungeon"))
    );
    assert_eq!(
        unique_field(&saved, "LootTableSeed"),
        Some(&NbtSummaryValue::Long(-12345))
    );
    assert_eq!(
        unique_field(&saved, "Items"),
        None,
        "`trySaveLootTable` answered true, so `saveAllItems` never ran"
    );
}

/// `LockCode.addToTag` (`LockCode.java:19-23`) stores nothing for
/// `LockCode.NO_LOCK`, and `BaseContainerBlockEntity.saveAdditional` (`:48`)
/// stores `CustomName` only when there is one. Both are kept as the record
/// wrote them so a save that has them does not lose them.
#[test]
fn a_chest_keeps_the_lock_and_the_custom_name_a_save_carries() {
    let chest_state = state_of(
        "minecraft:chest",
        &[
            ("facing", "south"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let plain = new_block_entity(pos(0, 0, 0), chest_state).expect("a chest builds an object");
    let saved = plain
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert_eq!(unique_field(&saved, "lock"), None);
    assert_eq!(unique_field(&saved, "CustomName"), None);
    assert!(plain
        .chest()
        .expect("a chest")
        .can_open(false, &RuntimeItemStack::empty(), test_item_resolver())
        .expect("fixture lock"));

    let mut record = saved;
    record.push(entry(
        "lock",
        NbtSummaryValue::Compound(vec![entry(
            "items",
            NbtSummaryValue::string("minecraft:tripwire_hook"),
        )]),
    ));
    record.push(entry("CustomName", NbtSummaryValue::string("Treasure")));
    let loaded = crate::block_entity::load_test_fixture(pos(0, 0, 0), chest_state, &record)
        .expect("fixture codec succeeds")
        .expect("a chest record loads onto a chest");
    assert!(
        !loaded
            .chest()
            .expect("a chest")
            .can_open(false, &RuntimeItemStack::empty(), test_item_resolver())
            .expect("fixture lock"),
        "an empty hand does not match the lock predicate"
    );
    assert!(loaded
        .chest()
        .expect("a chest")
        .can_open(true, &RuntimeItemStack::empty(), test_item_resolver())
        .expect("fixture lock"));
    let round_tripped = loaded
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert_eq!(
        unique_field(&round_tripped, "lock"),
        unique_field(&record, "lock")
    );
    assert_eq!(
        unique_field(&round_tripped, "CustomName"),
        unique_field(&record, "CustomName")
    );
}

/// `BlockEntity.preRemoveSideEffects` (`BlockEntity.java:233-237`) is
/// `Containers.dropContents` for a `Container`, so breaking a chest takes its
/// contents out of the record rather than leaving them at a position that is
/// now air.
#[test]
fn breaking_a_chest_hands_its_contents_back_and_drops_the_record() {
    let chest_state = state_of(
        "minecraft:chest",
        &[
            ("facing", "west"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let mut store = loaded_store();
    let mut block_entity =
        new_block_entity(pos(1, 2, 3), chest_state).expect("a chest builds an object");
    let mut items = vec![RuntimeItemStack::empty(); 27];
    items[3] = runtime_stack(bbb_protocol::packets::ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 12,
        component_patch: Default::default(),
    });
    block_entity
        .chest_mut()
        .expect("a chest")
        .set_items(items.clone());
    assert_eq!(
        block_entity.pre_remove_side_effects(block_entity.block_state()),
        BlockEntityRemovalDrops::ContainerSlots(items.to_vec())
    );
    store.block_entities_mut().add_and_register(block_entity);

    assert_eq!(
        remove_block_entity_on_block_change(
            store.block_entities_mut(),
            pos(1, 2, 3),
            chest_state,
            air(),
            0,
            crate::registry_tags::RegistryTagSnapshot::builtin()
        ),
        BlockEntityRemovalDrops::ContainerSlots(items.to_vec())
    );
    assert!(store.block_entities().get(pos(1, 2, 3)).is_none());
    assert!(store.save_dirty_epoch((0, 0)).is_some());
}

fn stone_stack(count: i32) -> RuntimeItemStack {
    runtime_stack(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count,
        component_patch: Default::default(),
    })
}

/// `HopperBlockEntity.saveAdditional` / `loadAdditional` (`:48-66`).
#[test]
fn hopper_save_round_trips_items_and_cooldown() {
    let mut hopper = hopper::HopperBlockEntity::default();
    hopper.items_mut()[2] = stone_stack(4);
    hopper.set_cooldown(7);
    let saved = hopper
        .save_additional(&mut BlockEntityItemSave::for_test())
        .expect("fixture payload codec");
    let mut loaded = hopper::HopperBlockEntity::default();
    loaded
        .load_additional(&saved, &mut BlockEntityItemLoad::for_test())
        .expect("fixture payload codec");
    assert_eq!(loaded.items()[2].count(), 4);
    assert_eq!(loaded.cooldown_time(), 7);
}

fn shulker_state(facing: &str) -> i32 {
    state_of("minecraft:shulker_box", &[("facing", facing)])
}

/// `ShulkerBoxBlockEntity.preRemoveSideEffects` is empty (`ShulkerBoxBlockEntity.java:163-164`).
#[test]
fn breaking_a_shulker_box_does_not_scatter_its_contents() {
    let mut block_entity = new_block_entity(pos(1, 2, 3), shulker_state("up"))
        .expect("a shulker box builds an object");
    let mut items = vec![RuntimeItemStack::empty(); 27];
    items[3] = stone_stack(12);
    assert!(block_entity
        .shulker_box_mut()
        .expect("a shulker box")
        .set_items(items));
    assert_eq!(
        block_entity.pre_remove_side_effects(block_entity.block_state()),
        BlockEntityRemovalDrops::None
    );
}

/// `ShulkerBoxBlockEntity.saveAdditional` / `loadAdditional` (`:202-220`).
#[test]
fn a_shulker_box_saves_only_its_non_empty_slots_and_reads_them_back() {
    let mut block_entity =
        new_block_entity(pos(4, 5, 6), shulker_state("north")).expect("a shulker box");
    let mut items = vec![RuntimeItemStack::empty(); 27];
    items[0] = stone_stack(64);
    items[26] = stone_stack(1);
    assert!(block_entity
        .shulker_box_mut()
        .expect("a shulker box")
        .set_items(items.clone()));

    let record = block_entity
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    let Some(NbtSummaryValue::List(saved)) = unique_field(&record, "Items") else {
        panic!("a shulker with no loot table saves its Items list: {record:?}");
    };
    assert_eq!(saved.len(), 2, "empty slots are skipped");

    let loaded =
        crate::block_entity::load_test_fixture(pos(4, 5, 6), shulker_state("north"), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a shulker standing on a shulker");
    assert_eq!(
        loaded.shulker_box().expect("a shulker object").items(),
        items
    );
}

/// `ShulkerBoxBlockEntity.triggerEvent` (`:140-155`) and `updateAnimation` (`:66-101`).
#[test]
fn opening_advances_the_lid_animation() {
    let mut shulker = shulker_box::ShulkerBoxBlockEntity::default();
    assert!(shulker.trigger_event(shulker_box::EVENT_SET_OPEN_COUNT, 1));
    assert_eq!(
        shulker.animation_status(),
        shulker_box::AnimationStatus::Opening
    );
    assert!(
        shulker.tick_animation(),
        "first opening tick fans neighbours"
    );
    let mut ticks = 1;
    while shulker.animation_status() == shulker_box::AnimationStatus::Opening {
        shulker.tick_animation();
        ticks += 1;
        assert!(ticks <= 10);
    }
    assert_eq!(
        shulker.animation_status(),
        shulker_box::AnimationStatus::Opened
    );
    assert_eq!(ticks, 10);
}

fn lectern_state(has_book: bool) -> i32 {
    state_of(
        "minecraft:lectern",
        &[
            ("facing", "north"),
            ("has_book", if has_book { "true" } else { "false" }),
            ("powered", "false"),
        ],
    )
}

fn two_page_book() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID),
        count: 1,
        component_patch: bbb_protocol::packets::DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
            ],
            writable_book_pages: vec!["one".into(), "two".into()],
            writable_book_page_filters: vec![None, None],
            ..Default::default()
        }.into(),
    }
}

/// `LecternBlockEntity.getRedstoneSignal` (`LecternBlockEntity.java:172-175`).
#[test]
fn a_lecterns_analog_follows_the_page() {
    let mut lectern = lectern::LecternBlockEntity::default();
    lectern
        .set_book(runtime_stack(two_page_book()))
        .expect("fixture book");
    assert_eq!(lectern.page_count(), 2);
    assert_eq!(lectern.redstone_signal().expect("fixture signal"), 1);
    assert!(lectern.set_page(1));
    assert_eq!(lectern.redstone_signal().expect("fixture signal"), 15);
}

/// `LecternBlockEntity.saveAdditional` / `loadAdditional` (`:204-218`).
///
/// Writable-book *pages* ride `writable_book_content`, which the persistent
/// item-stack projection does not yet encode; the book item and page index
/// still round-trip.
#[test]
fn a_lectern_saves_its_book_and_page() {
    let mut block_entity = new_block_entity(pos(1, 2, 3), lectern_state(true)).expect("a lectern");
    block_entity
        .lectern_mut()
        .expect("a lectern")
        .set_book(runtime_stack(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID),
            count: 1,
            component_patch: Default::default(),
        }))
        .expect("fixture book");
    let record = block_entity
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert!(unique_field(&record, "Book").is_some());
    assert_eq!(
        unique_field(&record, "Page"),
        Some(&NbtSummaryValue::Int(0))
    );
    let loaded = crate::block_entity::load_test_fixture(pos(1, 2, 3), lectern_state(true), &record)
        .expect("fixture codec succeeds")
        .expect("the record names a lectern");
    let lectern = loaded.lectern().expect("a lectern");
    assert!(lectern.has_book().expect("fixture operation succeeds"));
}

fn chiseled_bookshelf_state() -> i32 {
    state_of(
        "minecraft:chiseled_bookshelf",
        &[
            ("facing", "north"),
            ("slot_0_occupied", "false"),
            ("slot_1_occupied", "false"),
            ("slot_2_occupied", "false"),
            ("slot_3_occupied", "false"),
            ("slot_4_occupied", "false"),
            ("slot_5_occupied", "false"),
        ],
    )
}

fn book_stack() -> RuntimeItemStack {
    runtime_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID),
        count: 1,
        component_patch: Default::default(),
    })
}

/// `ChiseledBookShelfBlock.getAnalogOutputSignal` (`:191-196`).
#[test]
fn a_chiseled_bookshelfs_analog_is_last_interacted_slot_plus_one() {
    let mut bookshelf = chiseled_bookshelf::ChiseledBookShelfBlockEntity::default();
    assert!(bookshelf.set_slot(2, book_stack()));
    assert_eq!(bookshelf.last_interacted_slot(), 2);
    assert_eq!(bookshelf.redstone_signal(), 3);
    assert!(bookshelf.set_slot(2, RuntimeItemStack::empty()));
    assert_eq!(bookshelf.redstone_signal(), 3);
}

/// `ChiseledBookShelfBlockEntity.saveAdditional` / `loadAdditional` (`:54-66`).
#[test]
fn a_chiseled_bookshelf_saves_its_items_and_last_slot() {
    let mut block_entity =
        new_block_entity(pos(1, 2, 3), chiseled_bookshelf_state()).expect("a bookshelf");
    assert!(block_entity
        .chiseled_bookshelf_mut()
        .expect("a bookshelf")
        .set_slot(0, book_stack()));
    let record = block_entity
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert_eq!(
        unique_field(&record, "last_interacted_slot"),
        Some(&NbtSummaryValue::Int(0))
    );
    let loaded =
        crate::block_entity::load_test_fixture(pos(1, 2, 3), chiseled_bookshelf_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a chiseled bookshelf");
    let bookshelf = loaded.chiseled_bookshelf().expect("a bookshelf");
    assert_eq!(
        bookshelf.items()[0].item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID)
    );
    assert_eq!(bookshelf.last_interacted_slot(), 0);
}

fn decorated_pot_state() -> i32 {
    state_of(
        "minecraft:decorated_pot",
        &[
            ("facing", "north"),
            ("cracked", "false"),
            ("waterlogged", "false"),
        ],
    )
}

/// `DecoratedPotBlock.getAnalogOutputSignal` (`:241-243`).
#[test]
fn a_decorated_pots_analog_follows_its_one_slot() {
    let mut pot = decorated_pot::DecoratedPotBlockEntity::default();
    assert!(pot
        .try_insert_one(crate::block_entity::test_item_resolver(), &stone_stack(1))
        .expect("fixture operation succeeds"));
    assert!(pot.redstone_signal().unwrap() > 0);
    assert_eq!(pot.item().item_id(), Some(VANILLA_ITEM_STONE_ID));
    assert_eq!(pot.item().count(), 1);
    assert!(pot
        .try_insert_one(crate::block_entity::test_item_resolver(), &stone_stack(1))
        .expect("fixture operation succeeds"));
    assert_eq!(pot.item().count(), 2);
}

/// `DecoratedPotBlockEntity.saveAdditional` / `loadAdditional` (`:44-64`).
#[test]
fn a_decorated_pot_saves_its_item() {
    let mut block_entity = new_block_entity(pos(1, 2, 3), decorated_pot_state()).expect("a pot");
    block_entity
        .decorated_pot_mut()
        .expect("a pot")
        .set_item(stone_stack(3));
    let record = block_entity
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert!(unique_field(&record, "item").is_some());
    let loaded =
        crate::block_entity::load_test_fixture(pos(1, 2, 3), decorated_pot_state(), &record)
            .expect("fixture codec succeeds")
            .expect("the record names a decorated pot");
    let pot = loaded.decorated_pot().expect("a pot");
    assert_eq!(pot.item().item_id(), Some(VANILLA_ITEM_STONE_ID));
    assert_eq!(pot.item().count(), 3);
}

fn oak_shelf_state() -> i32 {
    state_of(
        "minecraft:oak_shelf",
        &[
            ("facing", "north"),
            ("powered", "false"),
            ("side_chain", "unconnected"),
            ("waterlogged", "false"),
        ],
    )
}

/// `ShelfBlockEntity.swapItemNoUpdate` (`:81-85`).
#[test]
fn a_shelf_swaps_the_hit_slot() {
    let mut shelf = shelf::ShelfBlockEntity::default();
    let taken = shelf.swap_item(0, stone_stack(4)).expect("slot 0 exists");
    assert!(taken.item_id().is_none());
    assert_eq!(shelf.items()[0].count(), 4);
    assert!(shelf.redstone_signal().unwrap() > 0);
}

/// `ShelfBlockEntity.saveAdditional` / `loadAdditional` (`:42-55`).
#[test]
fn a_shelf_saves_its_items() {
    let mut block_entity = new_block_entity(pos(1, 2, 3), oak_shelf_state()).expect("a shelf");
    block_entity
        .shelf_mut()
        .expect("a shelf")
        .swap_item(1, stone_stack(2));
    let record = block_entity
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    let loaded = crate::block_entity::load_test_fixture(pos(1, 2, 3), oak_shelf_state(), &record)
        .expect("fixture codec succeeds")
        .expect("the record names a shelf");
    let shelf = loaded.shelf().expect("a shelf");
    assert_eq!(shelf.items()[1].item_id(), Some(VANILLA_ITEM_STONE_ID));
    assert_eq!(shelf.items()[1].count(), 2);
}

/// `BellBlockEntity.tick` (`:56-66`) runs a 50-tick shake.
#[test]
fn a_rung_bell_stops_shaking_after_fifty_ticks() {
    let mut bell = bell::BellBlockEntity::default();
    bell.on_hit(bbb_protocol::packets::Direction::North);
    assert!(bell.shaking());
    for _ in 0..bell::DURATION {
        bell.tick();
    }
    assert!(!bell.shaking());
    assert_eq!(bell.ticks(), 0);
}

fn player_head_state() -> i32 {
    state_of(
        "minecraft:player_head",
        &[("powered", "false"), ("rotation", "0")],
    )
}

/// `SkullBlockEntity.saveAdditional` / `loadAdditional` (`:37-41`, `:44-49`).
#[test]
fn a_skull_round_trips_its_note_block_sound() {
    let mut skull = skull::SkullBlockEntity::default();
    skull.set_note_block_sound(Some("minecraft:block.note_block.harp".to_string()));
    let saved = skull.save_additional();
    let mut loaded = skull::SkullBlockEntity::default();
    loaded.load_additional(&saved);
    assert_eq!(
        loaded.note_block_sound(),
        Some("minecraft:block.note_block.harp")
    );
    let mut built = new_block_entity(pos(0, 0, 0), player_head_state()).expect("a player head");
    assert_eq!(
        built
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds"),
        Vec::new()
    );
    built
        .skull_mut()
        .expect("payload is a skull")
        .set_note_block_sound(Some("minecraft:block.note_block.harp".to_string()));
    assert_eq!(
        built
            .update_tag_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds"),
        vec![entry(
            "note_block_sound",
            NbtSummaryValue::string("minecraft:block.note_block.harp"),
        )]
    );
    assert!(
        crate::block_entity::block_entity_update_packet_with_resolver(
            &built,
            crate::block_entity::test_item_resolver()
        )
        .expect("fixture codec succeeds")
        .is_some()
    );
}

/// `AbstractFurnaceBlock.getAnalogOutputSignal` (`AbstractFurnaceBlock.java:67-69`)
/// and `BrewingStandBlock.getAnalogOutputSignal` (`BrewingStandBlock.java:95-97`)
/// are both `getRedstoneSignalFromBlockEntity`, so the answer is the container
/// fill over the slots each type declares and nothing else.
#[test]
fn the_furnace_and_brewing_stand_analog_outputs_are_their_container_fill() {
    let mut furnace = furnace::AbstractFurnaceBlockEntity::default();
    assert_eq!(furnace.items().len(), furnace::FURNACE_CONTAINER_SIZE);
    assert_eq!(furnace.redstone_signal().unwrap(), 0);
    // One full 64-stack of three slots is 1/3; floor(0.333 * 14) + 1.
    furnace.items_mut()[0] = stone_stack(64);
    assert_eq!(furnace.redstone_signal().unwrap(), 5);
    furnace.items_mut()[1] = stone_stack(64);
    furnace.items_mut()[2] = stone_stack(64);
    assert_eq!(furnace.redstone_signal().unwrap(), 15);

    let mut stand = brewing_stand::BrewingStandBlockEntity::default();
    assert_eq!(
        stand.items().len(),
        brewing_stand::BREWING_STAND_CONTAINER_SIZE
    );
    assert_eq!(stand.redstone_signal().unwrap(), 0);
    // 1/5 of the five slots; floor(0.2 * 14) + 1.
    stand.items_mut()[0] = stone_stack(64);
    assert_eq!(stand.redstone_signal().unwrap(), 3);
}

/// `CrafterBlockEntity.getRedstoneSignal` (`CrafterBlockEntity.java:251-262`)
/// counts slots rather than measuring a fill, and a disabled slot counts as a
/// taken one even though it is empty.
#[test]
fn a_crafters_analog_counts_taken_and_disabled_slots() {
    let mut crafter = crafter::CrafterBlockEntity::default();
    assert_eq!(crafter.redstone_signal(), 0);

    // One item in a slot is worth exactly one, whatever its count.
    crafter.items_mut()[0] = stone_stack(1);
    assert_eq!(crafter.redstone_signal(), 1);
    crafter.items_mut()[0] = stone_stack(64);
    assert_eq!(crafter.redstone_signal(), 1);

    // `slotCanBeDisabled` (`:264-266`) refuses a slot that holds something,
    // so only the empty ones can add to the count this way.
    assert!(!crafter.set_slot_disabled(0, true));
    assert!(crafter.set_slot_disabled(1, true));
    assert_eq!(crafter.redstone_signal(), 2);
    assert!(crafter.set_slot_disabled(1, false));
    assert_eq!(crafter.redstone_signal(), 1);

    for slot in 0..crafter::CRAFTER_CONTAINER_SIZE {
        crafter.items_mut()[slot] = stone_stack(1);
    }
    assert_eq!(
        crafter.redstone_signal(),
        crafter::CRAFTER_CONTAINER_SIZE as i32
    );
}

/// `CommandBlock.getAnalogOutputSignal` (`CommandBlock.java:146-149`) is the
/// saved success count, which is not clamped to a signal range here because
/// vanilla does not clamp it either.
#[test]
fn a_command_blocks_analog_is_its_saved_success_count() {
    let mut command_block = command_block::CommandBlockEntity::default();
    assert_eq!(command_block.success_count(), 0);
    command_block.set_success_count(7);
    assert_eq!(command_block.success_count(), 7);

    let saved = command_block.save_additional();
    let mut loaded = command_block::CommandBlockEntity::default();
    loaded.load_additional(&saved);
    assert_eq!(loaded.success_count(), 7);
}

fn sharp_sword() -> ItemStackSummary {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID;
    use bbb_protocol::packets::{DataComponentPatchSummary, ItemEnchantmentSummary};

    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID],
            enchantments: vec![ItemEnchantmentSummary {
                holder_id: bbb_registry_codec::PersistentItemStackHolderResolver::holder_id(
                    test_item_resolver(),
                    "minecraft:enchantment",
                    "minecraft:sharpness",
                )
                .expect("sharpness holder"),
                level: 5,
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

fn chest_state() -> i32 {
    state_of(
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    )
}

/// One `Items` list holding `stack` at `slot`, in the shape
/// `ContainerHelper.saveAllItems` writes.
fn chest_record(slot: u8, stack: NbtSummaryValue) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(stack) = stack else {
        panic!("a persistent stack is a compound");
    };
    let mut entries = vec![entry("Slot", NbtSummaryValue::Byte(slot as i8))];
    entries.extend(stack);
    vec![
        entry("id", NbtSummaryValue::string("minecraft:chest")),
        entry("x", NbtSummaryValue::Int(0)),
        entry("y", NbtSummaryValue::Int(64)),
        entry("z", NbtSummaryValue::Int(1)),
        entry("keepPacked", NbtSummaryValue::Byte(0)),
        entry(
            "Items",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
        ),
    ]
}

/// A chest record whose slot holds an item stack, written through the
/// publication the level published.
#[test]
fn a_chest_keeps_the_holder_resolved_component_the_publication_names() {
    let stack = sharp_sword();
    let saved = bbb_registry_codec::item_stack_summary_to_persistent_nbt_with_resolver(
        &stack,
        test_item_resolver(),
    )
    .expect("the publication names the sword's enchantment");
    let record = chest_record(3, saved);

    let loaded = BlockEntity::load_static_with_resolver(
        pos(0, 64, 1),
        chest_state(),
        &record,
        test_item_resolver(),
    )
    .expect("fixture codec succeeds")
    .expect("the record names a chest");
    let chest = loaded.chest().expect("a chest");
    assert_eq!(chest.items()[3].summary(), &stack);
    assert!(chest.items()[0].item_id().is_none());
}

#[test]
fn a_playing_jukebox_restores_an_explicit_song_component() {
    let NbtSummaryValue::Compound(mut record) = bbb_protocol::parse_snbt_compound(
        r#"{id:"minecraft:jukebox",RecordItem:{id:"minecraft:music_disc_13",count:1,components:{"minecraft:jukebox_playable":"minecraft:cat"}}}"#,
    )
    .unwrap()
    else {
        unreachable!()
    };
    let position = pos(0, 64, 1);
    let state = state_of("minecraft:jukebox", &[("has_record", "true")]);
    let resolver = test_item_resolver();
    let load = |record: &[NbtSummaryEntry]| {
        BlockEntity::load_static_with_resolver(position, state, record, resolver)
            .expect("a legal jukebox record loads")
            .expect("the record names a jukebox")
    };
    let stopped = load(&record);
    assert_eq!(
        stopped
            .jukebox()
            .unwrap()
            .comparator_output(resolver)
            .unwrap(),
        2
    );
    let item = stopped.jukebox().unwrap().item().clone();
    assert_eq!(item.item_resource_id(), Some("minecraft:music_disc_13"));
    assert_eq!(
        item.component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
            resolver,
        )
        .unwrap(),
        Some(NbtSummaryValue::String("minecraft:cat".into())),
    );

    record.push(entry("ticks_since_song_started", NbtSummaryValue::Long(0)));
    let playing = load(&record);
    let jukebox = playing.jukebox().unwrap();
    assert_eq!(jukebox.item(), &item);
    assert_eq!(jukebox.song_player().song_id(), Some("minecraft:cat"));
    assert_eq!(jukebox.song_player().ticks_since_song_started(), 0);
    let saved = playing
        .save_with_full_metadata_with_resolver(resolver)
        .unwrap();
    assert_eq!(
        unique_field(&saved, "ticks_since_song_started"),
        Some(&NbtSummaryValue::Long(0)),
    );
    let saved_item =
        RuntimeItemStack::try_read(unique_field(&saved, "RecordItem").unwrap(), resolver).unwrap();
    assert_eq!(saved_item, item);
}

fn text(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.into())
}

fn default_state(name: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is a vanilla 26.1 block"));
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id)
        .expect("a registered block has a default state")
}

/// `ShulkerBoxBlockEntity.loadAdditional` / `saveAdditional` (`:202-213`) keep
/// `BaseContainerBlockEntity`'s `CustomName` and the loot-table pair, and
/// `collectComponents` (`BlockEntity.java:309-314`) lays the implicit
/// components over the retained map: the name replaces the retained one, the
/// empty slots answer an empty `container`, and the pair `container_loot`.
#[test]
fn a_packed_shulker_box_keeps_its_name_and_seed_and_collects_them() {
    let record = vec![
        entry("id", text("minecraft:shulker_box")),
        entry("CustomName", text("Sealed")),
        entry("LootTable", text("chests/igloo_chest")),
        entry("LootTableSeed", NbtSummaryValue::Long(-42)),
        entry(
            "components",
            NbtSummaryValue::Compound(vec![
                entry("minecraft:lore", NbtSummaryValue::List(vec![text("kept")])),
                entry("minecraft:custom_name", text("replaced")),
            ]),
        ),
    ];
    let loaded = crate::block_entity::load_test_fixture(pos(1, 2, 3), shulker_state("up"), &record)
        .expect("fixture codec succeeds")
        .expect("a shulker record");

    let saved = loaded
        .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
        .expect("fixture codec succeeds");
    assert_eq!(unique_field(&saved, "CustomName"), Some(&text("Sealed")));
    assert_eq!(
        unique_field(&saved, "LootTable"),
        Some(&text("minecraft:chests/igloo_chest"))
    );
    assert_eq!(
        unique_field(&saved, "LootTableSeed"),
        Some(&NbtSummaryValue::Long(-42))
    );
    assert_eq!(unique_field(&saved, "Items"), None);
    let reloaded =
        crate::block_entity::load_test_fixture(pos(1, 2, 3), shulker_state("up"), &saved)
            .expect("fixture codec succeeds")
            .expect("the save reads back");
    assert_eq!(
        reloaded
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver(),)
            .expect("fixture codec succeeds"),
        saved
    );

    let collected = loaded.collect_components(BlockEntityItemProjection::with_resolver(
        test_item_resolver(),
    ));
    assert_eq!(collected.entries().len(), 4, "{collected:?}");
    assert_eq!(
        collected.get("minecraft:lore"),
        Some(&Ok(NbtSummaryValue::List(vec![text("kept")])))
    );
    assert_eq!(
        collected.get("minecraft:custom_name"),
        Some(&Ok(text("Sealed")))
    );
    assert_eq!(
        collected.get("minecraft:container"),
        Some(&Ok(NbtSummaryValue::List(Vec::new())))
    );
    assert_eq!(
        collected.get("minecraft:container_loot"),
        Some(&Ok(NbtSummaryValue::Compound(vec![
            entry("loot_table", text("minecraft:chests/igloo_chest")),
            entry("seed", NbtSummaryValue::Long(-42)),
        ])))
    );
    let layers = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        entry("pattern", text("minecraft:stripe_top")),
        entry("color", text("red")),
    ])]);
    let mut unpacked = record.clone();
    unpacked.retain(|field| field.name != "LootTable" && field.name != "LootTableSeed");
    unpacked
        .iter_mut()
        .find(|field| field.name == "components")
        .unwrap()
        .value =
        NbtSummaryValue::Compound(vec![entry("minecraft:banner_patterns", layers.clone())]);
    unpacked.push(entry(
        "lock",
        NbtSummaryValue::Compound(vec![entry(
            "components",
            NbtSummaryValue::Compound(vec![entry("minecraft:banner_patterns", layers.clone())]),
        )]),
    ));
    unpacked.push(entry(
        "Items",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry("Slot", NbtSummaryValue::Byte(4)),
            entry("id", text("minecraft:stone")),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![entry("minecraft:banner_patterns", layers)]),
            ),
        ])]),
    ));
    let source = load_test_fixture(pos(1, 2, 3), shulker_state("up"), &unpacked)
        .expect("FIRST fixture")
        .expect("unpacked shulker");
    let held = source.collect_components(BlockEntityItemProjection::with_resolver(
        test_item_resolver(),
    ));
    let copied = stone_stack(1)
        .with_component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
            Some(held.get("minecraft:container").unwrap().as_ref().unwrap()),
            test_item_resolver(),
        )
        .expect("copy applies");
    let contents = copied.container_contents().expect("container");
    assert!(contents[4].same_item_same_components(&source.shulker_box().unwrap().items()[4]));
}

/// `ShulkerBoxBlock.getDrops`' `contents` (`ShulkerBoxBlock.java:126-137`) is
/// every slot in order; other names are unregistered.
#[test]
fn an_unpacked_shulker_box_answers_its_slots_as_the_contents_drop() {
    let mut block_entity =
        new_block_entity(pos(0, 0, 0), shulker_state("up")).expect("a shulker box");
    let mut items = vec![RuntimeItemStack::empty(); 27];
    items[4] = stone_stack(9);
    assert!(block_entity
        .shulker_box_mut()
        .expect("a shulker box")
        .set_items(items.clone()));
    let contents = block_entity
        .dynamic_drop("minecraft:contents")
        .expect("registered contents")
        .expect("unpacked contents");
    assert_eq!(contents.len(), items.len());
    for (drop, expected) in contents.iter().zip(&items) {
        assert_eq!(drop, expected.summary());
    }
}

/// `DecoratedPotBlockEntity.collectImplicitComponents` (`:112-116`): the faces
/// in `PotDecorations.ordered` order with brick for a missing one, and the
/// held stack as the `container`.
#[test]
fn a_decorated_pot_collects_its_ordered_faces_and_its_stack() {
    let record = vec![
        entry("id", text("minecraft:decorated_pot")),
        entry(
            "sherds",
            NbtSummaryValue::List(vec![
                text("minecraft:angler_pottery_sherd"),
                text("minecraft:brick"),
                text("minecraft:arms_up_pottery_sherd"),
            ]),
        ),
        entry(
            "item",
            NbtSummaryValue::Compound(vec![
                entry("id", text("minecraft:stone")),
                entry("count", NbtSummaryValue::Int(3)),
            ]),
        ),
    ];
    let pot_state = default_state("minecraft:decorated_pot");
    let pot = crate::block_entity::load_test_fixture(pos(0, 0, 0), pot_state, &record)
        .expect("fixture codec succeeds")
        .expect("a pot record");
    let collected = pot.collect_components(BlockEntityItemProjection::with_resolver(
        test_item_resolver(),
    ));
    assert_eq!(
        collected.get("minecraft:pot_decorations"),
        Some(&Ok(NbtSummaryValue::List(vec![
            text("minecraft:angler_pottery_sherd"),
            text("minecraft:brick"),
            text("minecraft:arms_up_pottery_sherd"),
            text("minecraft:brick"),
        ])))
    );
    assert_eq!(
        collected.get("minecraft:container"),
        Some(&Ok(NbtSummaryValue::List(vec![NbtSummaryValue::Compound(
            vec![
                entry("slot", NbtSummaryValue::Int(0)),
                entry(
                    "item",
                    NbtSummaryValue::Compound(vec![
                        entry("id", text("minecraft:stone")),
                        entry("count", NbtSummaryValue::Int(3)),
                    ])
                ),
            ]
        )])))
    );
}

/// `BannerBlockEntity.collectImplicitComponents` (`:96-100`) always answers
/// the pattern layers, and the name only when there is one.
#[test]
fn a_banner_collects_its_patterns_and_its_name() {
    let patterns = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        entry("pattern", text("minecraft:stripe_top")),
        entry("color", text("red")),
    ])]);
    let banner_state = default_state("minecraft:white_banner");
    let named = crate::block_entity::load_test_fixture(
        pos(0, 0, 0),
        banner_state,
        &[
            entry("id", text("minecraft:banner")),
            entry("patterns", patterns.clone()),
            entry("CustomName", text("Flag")),
        ],
    )
    .expect("fixture codec succeeds")
    .expect("a banner record");
    let collected = named.collect_components(BlockEntityItemProjection::with_resolver(
        test_item_resolver(),
    ));
    assert_eq!(
        collected.get("minecraft:banner_patterns"),
        Some(&Ok(patterns))
    );
    assert_eq!(
        collected.get("minecraft:custom_name"),
        Some(&Ok(text("Flag")))
    );

    let plain = new_block_entity(pos(0, 0, 0), banner_state).expect("a banner");
    let collected = plain.collect_components(BlockEntityItemProjection::with_resolver(
        test_item_resolver(),
    ));
    assert_eq!(
        collected.get("minecraft:banner_patterns"),
        Some(&Ok(NbtSummaryValue::List(Vec::new())))
    );
    assert_eq!(collected.get("minecraft:custom_name"), None);
}

/// `Containers.dropContents` reads every slot through `getItem`, which
/// unpacks a pending table (`RandomizableContainerBlockEntity.java:54-57`);
/// the shulker box's `preRemoveSideEffects` is empty.
#[test]
fn only_a_container_whose_removal_drops_its_slots_unpacks_its_loot() {
    let loot = [
        entry("LootTable", text("minecraft:chests/igloo_chest")),
        entry("LootTableSeed", NbtSummaryValue::Long(5)),
    ];
    let chest_state = default_state("minecraft:chest");
    let with_loot = |id: &str, state: i32| {
        let mut record = vec![entry("id", text(id))];
        record.extend(loot.iter().cloned());
        crate::block_entity::load_test_fixture(pos(0, 0, 0), state, &record)
            .expect("fixture codec succeeds")
            .expect("a container record")
    };
    let chest = with_loot("minecraft:chest", chest_state);
    assert!(chest.removal_unpacks_loot(chest_state));
    let shulker = with_loot("minecraft:shulker_box", shulker_state("up"));
    assert!(!shulker.removal_unpacks_loot(shulker_state("up")));
    let empty_chest = new_block_entity(pos(0, 0, 0), chest_state).expect("a chest");
    assert!(!empty_chest.removal_unpacks_loot(chest_state));
}
