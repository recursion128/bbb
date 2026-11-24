//! Datapack recipe result components through crafting and player persistence.

use super::*;
use bbb_protocol::item_types::{VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_FLINT_ID};

#[test]
fn styled_recipe_result_components_survive_crafting_and_player_restart() {
    let name = serde_json::json!({
        "text": "Smith's tool", "color": "gold", "bold": true,
        "extra": [{"text": "!", "italic": true}]
    });
    let lore = serde_json::json!({
        "translate": "item.minecraft.flint_and_steel", "color": "blue",
        "extra": [{"text": " forged", "underlined": true}]
    });
    let display = serde_json::json!({
        "text": "Power", "color": "red", "italic": false,
        "extra": [{"text": " +3.5", "bold": true}]
    });
    let recipe = serde_json::json!({
        "type": "minecraft:crafting_shaped",
        "pattern": ["IF"],
        "key": {"I": "minecraft:diamond", "F": "minecraft:flint"},
        "result": {
            "id": "minecraft:flint_and_steel",
            "components": {
                "minecraft:custom_name": name,
                "minecraft:lore": [lore],
                "minecraft:attribute_modifiers": [{
                    "type": "minecraft:attack_damage", "id": "test:forged",
                    "amount": 3.5, "operation": "add_value", "slot": "mainhand",
                    "display": {"type": "override", "value": display}
                }]
            }
        }
    });
    let root = std::env::temp_dir().join(format!("bbb-recipe-components-{}", uuid::Uuid::new_v4()));
    let pack = root.join("custom");
    std::fs::create_dir_all(pack.join("data/test/recipe")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"recipe components","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        pack.join("data/test/recipe/forged.json"),
        serde_json::to_vec(&recipe).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    assert!(startup.recipes.recipe("test:forged").is_some());
    std::fs::remove_dir_all(root).unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..creative_fixture_config()
    };
    let (sender, receiver) = mailbox();
    let mut world =
        ServerWorld::restore(config.clone(), &restore_candidate(0, None, vec![])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 9, "smith", 9);
    for (slot_num, item_id) in [(1, VANILLA_ITEM_DIAMOND_ID), (2, VANILLA_ITEM_FLINT_ID)] {
        play(
            &sender,
            &receiver,
            &mut world,
            9,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
                slot_num,
                item: ItemStackSummary {
                    item_id: Some(item_id),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
            }),
        );
    }
    play(
        &sender,
        &receiver,
        &mut world,
        9,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    let request = world
        .save_request(1_786_000_002_000, 202, RegionCompression::Zlib)
        .unwrap();
    let patch = &request.player_patches[0];
    let crafted = patch
        .created
        .inventory
        .runtime_slots()
        .into_iter()
        .find(|stack| stack.summary().component_patch.custom_name_nbt.is_some())
        .expect("the custom recipe result was quick-moved into the inventory");
    let components = &crafted.summary().component_patch;
    let expected_name = bbb_protocol::nbt_summary::json_value_to_nbt_summary(&name).unwrap();
    let expected_lore = bbb_protocol::nbt_summary::json_value_to_nbt_summary(&lore).unwrap();
    let expected_display = bbb_protocol::nbt_summary::json_value_to_nbt_summary(&display).unwrap();
    assert_eq!(components.custom_name_nbt, Some(expected_name));
    assert_eq!(components.lore_nbt, [expected_lore]);
    let [modifier] = components.attribute_modifiers.as_slice() else {
        panic!("the crafted result retains its attribute modifier");
    };
    assert_eq!(
        bbb_protocol::attribute_types::vanilla_attribute_type(modifier.attribute_id)
            .unwrap()
            .resource_id,
        "minecraft:attack_damage"
    );
    assert_eq!(modifier.modifier_id, "test:forged");
    assert_eq!(modifier.amount_bits, 3.5_f64.to_bits());
    assert_eq!(modifier.operation_id, 0);
    assert_eq!(
        bbb_protocol::equipment_slots::vanilla_equipment_slot_group(modifier.slot_id)
            .unwrap()
            .serialized_name,
        "mainhand"
    );
    assert_eq!(modifier.display_text_nbt, Some(expected_display));
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    let mut restarted = ServerWorld::restore(
        config,
        &restore_candidate(0, None, vec![(patch.uuid, parsed)]),
    )
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut restarted, 9, "smith", 9);
    let restored_request = restarted
        .save_request(1_786_000_003_000, 203, RegionCompression::Zlib)
        .unwrap();
    let restored = restored_request.player_patches[0]
        .created
        .inventory
        .runtime_slots()
        .into_iter()
        .find(|stack| stack.summary().component_patch.custom_name_nbt.is_some())
        .expect("the crafted result is restored to the player's inventory");
    assert!(crafted.same_item_same_components(&restored));
    assert_eq!(restored.count(), 1);
}

#[test]
fn transmute_only_offers_an_effective_component_change_through_the_player_menu() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_GOAT_HORN_ID;
    use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
    use bbb_server_datapack::recipe::{parse_recipe_source_with_lookup, RecipeElement};

    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let lookup = FrozenDatapackRegistryLookup::from_startup(startup);
    let source = bbb_registry_codec::decode_direct_registry_json(
        br#"{
        "type": "minecraft:crafting_transmute",
        "input": "minecraft:goat_horn", "material": "minecraft:flint",
        "result": {"id": "minecraft:goat_horn", "components": {
            "minecraft:instrument": "minecraft:ponder_goat_horn"
        }}
    }"#,
    )
    .unwrap();
    let recipe = parse_recipe_source_with_lookup(source.root(), &lookup).unwrap();
    std::sync::Arc::make_mut(&mut startup.recipes).recipes = vec![RecipeElement {
        id: "test:retune".into(),
        source_pack: "test".into(),
        recipe,
    }];
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config, &restore_candidate(0, None, vec![])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 9, "smith", 9);
    let resolver = world.item_stack_resolver();
    let horn = bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
        count: 1,
        component_patch: Default::default(),
    });
    let put = |world: &mut ServerWorld, slot_num, item| {
        play(
            &sender,
            &receiver,
            world,
            9,
            PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot { slot_num, item }),
        );
    };
    put(
        &mut world,
        2,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_FLINT_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    put(&mut world, 1, horn.summary().clone());
    assert!(world
        .player_inventory_for_test(ConnectionId(9))
        .slot_item(0)
        .is_empty());
    let changed = horn
        .clone()
        .with_component_nbt(
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            Some(&bbb_protocol::NbtSummaryValue::String(
                "minecraft:sing_goat_horn".into(),
            )),
            &resolver,
        )
        .unwrap();
    put(&mut world, 1, changed.summary().clone());
    let menu = world.player_inventory_for_test(ConnectionId(9));
    assert_eq!(menu.slot_item(0).item_id(), Some(VANILLA_ITEM_GOAT_HORN_ID));
    play(
        &sender,
        &receiver,
        &mut world,
        9,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: 0,
            state_id: 1,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    let menu = world.player_inventory_for_test(ConnectionId(9));
    assert!(menu.slot_item(1).is_empty());
    assert!(menu.slot_item(2).is_empty());
    let result = menu
        .inventory_slots()
        .into_iter()
        .find(|stack| stack.item_id() == Some(VANILLA_ITEM_GOAT_HORN_ID))
        .unwrap();
    assert!(result
        .same_item_same_effective_components(&horn, &resolver)
        .unwrap());
    assert_eq!(result.count(), 1);
}
