use super::*;
use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID;
use bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID;
use bbb_protocol::game_types::VANILLA_GAME_TYPE_ADVENTURE_ID;
use bbb_protocol::menu_types::VANILLA_MENU_TYPES;
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_REGENERATION_ID, VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_STRENGTH_ID,
};
use bbb_protocol::packets::{
    AddEntity, ContainerButtonClick, ContainerClick, ContainerCloseRequest, ContainerSetContent,
    ContainerSetData, ContainerSlotStateChanged, EntityDataValue, EntityDataValueKind, GameEvent,
    HashedComponentPatch, HashedItemStack, HashedStack, IngredientSummary, ItemCostSummary,
    ItemStackSummary, MerchantOffer, MerchantOffers, MountScreenOpen, OpenScreen,
    PlaceRecipeCommand, PlayerAbilities, PlayerExperience, RecipeBookChangeSettingsCommand,
    RecipeBookSettings, RecipeBookType, RecipeBookTypeSettings, RecipePropertySetSummary,
    RegistryData, RegistryDataEntry, RegistryTags, SelectBundleItem, SelectTradeCommand, SetBeacon,
    SetCursorItem, SetEntityData, SetPlayerInventory, SlotDisplaySummary,
    StonecutterSelectableRecipeSummary, TagNetworkPayload, UpdateRecipes, UpdateTags, Vec3d,
};
use bbb_protocol::recipe_wire_types::{
    VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
};
use uuid::Uuid;

const TEST_HASH_OPS_INT_7_HASH: i32 = -1_726_626_450;
const TEST_MAP_ID_7_HASH: i32 = TEST_HASH_OPS_INT_7_HASH;

fn baby_data(entity_type_id: i32, is_baby: bool) -> EntityDataValue {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
            .expect("generated nearest baby accessor must resolve");
    assert!(!default);
    bool_entity_data(data_id, is_baby)
}

fn stonecutter_recipe(item_ids: Vec<i32>) -> StonecutterSelectableRecipeSummary {
    StonecutterSelectableRecipeSummary {
        input: IngredientSummary {
            tag: None,
            item_ids,
        },
        option_display: SlotDisplaySummary {
            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
            raw_payload: vec![VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID as u8],
            item_stack: None,
            tag: None,
        },
    }
}

fn apply_item_tags(world: &mut WorldStore, tags: Vec<(&str, Vec<i32>)>) {
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: tags
                .into_iter()
                .map(|(tag, entries)| TagNetworkPayload {
                    tag: tag.to_string(),
                    entries,
                })
                .collect(),
        }],
    });
}

fn loom_world_with_banner_and_dye() -> WorldStore {
    let mut world = WorldStore::new();
    world.record_registry_data(RegistryData {
        registry: "minecraft:banner_pattern".to_string(),
        entries: (0..32)
            .map(|index| RegistryDataEntry {
                id: format!("example:pattern_{index}"),
                raw_data: None,
            })
            .collect(),
        raw_payload_len: 0,
    });
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:banner_pattern".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:no_item_required".to_string(),
                entries: (0..32).collect(),
            }],
        }],
    });
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LOOM_ID,
        title: bbb_protocol::RichTextComponent::plain("Loom"),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[0] = item_stack(42, 1);
    items[1] = item_stack(43, 1);
    world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 12,
        items,
        carried_item: ItemStackSummary::empty(),
    });
    world
}

fn add_entity_with_type(id: i32, entity_type_id: i32) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position: Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn byte_entity_data(data_id: u8, value: i8) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(value),
    }
}

fn bool_entity_data(data_id: u8, value: bool) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    }
}

fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn lectern_writable_book(page_count: usize) -> ItemStackSummary {
    let mut book = item_stack(42, 1);
    book.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);
    book.component_patch.writable_book_pages = (0..page_count)
        .map(|page| format!("Page {page}").into())
        .collect();
    book.component_patch.writable_book_page_filters = vec![None; page_count];
    book
}

fn crafting_recipe_book_entry(
    id: i32,
    category_id: i32,
    result_item_id: i32,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    crafting_recipe_book_entry_with_requirements(id, category_id, result_item_id, Vec::new())
}

fn crafting_recipe_book_entry_with_requirements(
    id: i32,
    category_id: i32,
    result_item_id: i32,
    requirements: Vec<Vec<i32>>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    crafting_recipe_book_entry_with_requirement_summaries(
        id,
        category_id,
        result_item_id,
        requirements
            .into_iter()
            .map(|item_ids| IngredientSummary {
                tag: None,
                item_ids,
            })
            .collect(),
    )
}

fn crafting_recipe_book_entry_with_requirement_summaries(
    id: i32,
    category_id: i32,
    result_item_id: i32,
    requirements: Vec<IngredientSummary>,
) -> bbb_protocol::packets::RecipeBookAddEntry {
    bbb_protocol::packets::RecipeBookAddEntry {
        contents: bbb_protocol::packets::RecipeDisplayEntry {
            id: bbb_protocol::packets::RecipeDisplayId { index: id },
            display: bbb_protocol::packets::RecipeDisplaySummary {
                display_type: bbb_protocol::packets::RecipeDisplayType::CraftingShapeless,
                raw_body: Vec::new(),
                crafting: Some(Box::new(
                    bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                        ingredients: Vec::new(),
                        result: SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                            raw_payload: Vec::new(),
                            item_stack: Some(item_stack(result_item_id, 1)),
                            tag: None,
                        },
                        crafting_station: SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                            raw_payload: Vec::new(),
                            item_stack: None,
                            tag: None,
                        },
                    },
                )),
                furnace: None,
                stonecutter: None,
                smithing: None,
            },
            group: None,
            category_id,
            crafting_requirements: (!requirements.is_empty()).then_some(requirements),
        },
        flags: 0,
        notification: false,
        highlight: false,
    }
}

fn map_id_item_stack(item_id: i32, count: i32, map_id: i32) -> ItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID);
    item.component_patch.map_id = Some(map_id);
    item
}

fn damage_item_stack(item_id: i32, count: i32, damage: i32) -> ItemStackSummary {
    let mut item = item_stack(item_id, count);
    item.component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    item.component_patch.damage = Some(damage);
    item
}

fn apply_instabuild_abilities(world: &mut WorldStore) {
    world.apply_player_abilities(PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
}

fn merchant_offers(container_id: i32, offer_count: usize) -> MerchantOffers {
    MerchantOffers {
        container_id,
        offers: (0..offer_count)
            .map(|index| MerchantOffer {
                buy_a: item_cost(42 + index as i32, 3),
                sell: item_stack(99 + index as i32, 1),
                buy_b: None,
                is_out_of_stock: false,
                uses: 1,
                max_uses: 12,
                xp: 8,
                special_price_diff: -2,
                price_multiplier: 0.05,
                demand: 6,
            })
            .collect(),
        villager_level: 3,
        villager_xp: 120,
        show_progress: true,
        can_restock: false,
    }
}

fn item_cost(item_id: i32, count: i32) -> ItemCostSummary {
    ItemCostSummary {
        item_id,
        count,
        component_predicate: Default::default(),
    }
}

fn bundle_stack(item_id: i32, count: i32, item_count: usize) -> ItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
    stack.component_patch.bundle_contents_items = (0..item_count)
        .map(|_| bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
            count: 1,
            component_patch: Default::default(),
        })
        .collect();
    stack
}

fn hashed_item(item_id: i32, count: i32) -> HashedItemStack {
    HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch::default(),
    }
}

fn hashed_int_component_item(
    item_id: i32,
    count: i32,
    component_type_id: i32,
    value: i32,
) -> HashedItemStack {
    let value_hash = match value {
        7 => TEST_HASH_OPS_INT_7_HASH,
        other => panic!("missing test HashOps int hash for {other}"),
    };
    HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch {
            added_components: BTreeMap::from([(component_type_id, value_hash)]),
            removed_components: BTreeSet::new(),
        },
    }
}

fn hashed_map_id_item(item_id: i32, count: i32, map_id: i32) -> HashedItemStack {
    let map_id_hash = match map_id {
        7 => TEST_MAP_ID_7_HASH,
        other => panic!("missing test HashOps map_id hash for {other}"),
    };
    HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch {
            added_components: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
                map_id_hash,
            )]),
            removed_components: BTreeSet::new(),
        },
    }
}

fn hashed_map_id_removed_component_item(
    item_id: i32,
    count: i32,
    map_id: i32,
    component_type_id: i32,
) -> HashedItemStack {
    let map_id_hash = match map_id {
        7 => TEST_MAP_ID_7_HASH,
        other => panic!("missing test HashOps map_id hash for {other}"),
    };
    HashedItemStack {
        item_id,
        count,
        components: HashedComponentPatch {
            added_components: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
                map_id_hash,
            )]),
            removed_components: BTreeSet::from([component_type_id]),
        },
    }
}

fn assert_quick_craft_click(
    rx: &mut mpsc::Receiver<NetCommand>,
    slot_num: i16,
    button_num: i8,
    changed_slots: BTreeMap<i16, HashedStack>,
    carried_item: HashedStack,
) {
    assert_quick_craft_click_for_container(
        rx,
        0,
        0,
        slot_num,
        button_num,
        changed_slots,
        carried_item,
    );
}

fn assert_quick_craft_click_for_container(
    rx: &mut mpsc::Receiver<NetCommand>,
    container_id: i32,
    state_id: i32,
    slot_num: i16,
    button_num: i8,
    changed_slots: BTreeMap<i16, HashedStack>,
    carried_item: HashedStack,
) {
    match rx.try_recv().unwrap() {
        NetCommand::ContainerClick(click) => {
            assert_eq!(click.container_id, container_id);
            assert_eq!(click.state_id, state_id);
            assert_eq!(click.slot_num, slot_num);
            assert_eq!(click.button_num, button_num);
            assert_eq!(click.input, ContainerInput::QuickCraft);
            assert_eq!(click.changed_slots, changed_slots);
            assert_eq!(click.carried_item, carried_item);
        }
        command => panic!("expected quick craft container click command, got {command:?}"),
    }
}

fn player_slot_item(world: &WorldStore, slot: i32) -> ItemStackSummary {
    world
        .inventory()
        .player_slots
        .iter()
        .find(|state| state.slot == slot)
        .map(|state| state.item.clone())
        .unwrap_or_else(ItemStackSummary::empty)
}

fn open_container_slot_bundle_selection(world: &WorldStore, slot: i16) -> Option<i32> {
    world
        .inventory()
        .container_slot(world.inventory().open_container.as_ref()?, slot)
        .map(|state| state.local_selected_bundle_item_index)
}

mod scrolling;

fn open_container_slots(world: &WorldStore) -> Vec<bbb_client_world::ContainerSlot> {
    let inventory = world.inventory();
    inventory
        .container_slots(inventory.open_container.as_ref().unwrap())
        .map(|slot| bbb_client_world::ContainerSlot {
            slot: slot.slot,
            item: slot.item.clone(),
            local_selected_bundle_item_index: slot.local_selected_bundle_item_index,
        })
        .collect()
}

mod drag_double_click_and_bundles;
mod layouts_and_hit_tests;
mod menu_buttons_and_recipe_book;
mod shift_click_predictions;
