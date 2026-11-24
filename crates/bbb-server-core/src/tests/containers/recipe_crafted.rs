use super::*;
use crate::tests::parse_advancement;
use bbb_server_datapack::advancement::AdvancementElement;
use bbb_server_datapack::recipe::{ItemTagIndex, RecipeKind};
use serde_json::{json, Value};

pub(super) fn config(
    mut config: ServerWorldConfig,
    recipe: &str,
    ingredients: Value,
) -> ServerWorldConfig {
    let mut startup = (*config.datapacks.take().unwrap()).clone();
    startup.advancements.advancements.push(AdvancementElement {
        id: "test:crafted".into(),
        source_pack: "test".into(),
        advancement: parse_advancement(&json!({
            "criteria":{"crafted":{"trigger":"minecraft:recipe_crafted", "conditions":{
                "recipe_id":recipe,"ingredients":ingredients
            }}},"rewards":{"experience":5}
        }))
        .unwrap(),
    });
    config.datapacks = Some(Arc::new(startup));
    config
}

pub(super) fn utility_config(block: i32, mut config: ServerWorldConfig) -> ServerWorldConfig {
    use bbb_protocol::block_types::{
        VANILLA_BLOCK_SMITHING_TABLE_ID, VANILLA_BLOCK_STONECUTTER_ID,
    };
    if block == VANILLA_BLOCK_SMITHING_TABLE_ID {
        config = self::config(
            config,
            "minecraft:netherite_sword_smithing",
            json!([
                {"items":"minecraft:netherite_upgrade_smithing_template","count":2},
                {"items":"minecraft:diamond_sword"},
                {"items":"minecraft:netherite_ingot","count":2}
            ]),
        );
    } else if block == VANILLA_BLOCK_STONECUTTER_ID {
        let startup = config.datapacks.as_ref().unwrap();
        let tags = ItemTagIndex::from_loaded(startup.tags.registry("minecraft:item").unwrap());
        let stone = ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 2,
            component_patch: Default::default(),
        };
        let recipe = startup.recipes.recipes.iter().find(|element| {
            matches!(&element.recipe.kind, RecipeKind::Stonecutting(recipe) if recipe.ingredient.test(&stone, &tags))
        }).unwrap().id.clone();
        config = self::config(
            config,
            &recipe,
            json!([{"items":"minecraft:stone","count":2}]),
        );
    }
    config
}

#[test]
fn player_grid_takes_known_and_special_recipes_with_actual_components() {
    use bbb_protocol::item_types::VANILLA_ITEM_OAK_LOG_ID;
    for special in [false, true] {
        let startup = (*datapack_config().datapacks.unwrap()).clone();
        let recipe = if special {
            "minecraft:repair_item"
        } else {
            "minecraft:oak_planks"
        };
        let ingredients = if special {
            json!([{"items":"minecraft:stone_pickaxe", "count":1}])
        } else {
            json!([{"items":"minecraft:oak_log", "count":3,"components":{"minecraft:custom_name":"Recipe input"}}])
        };
        let mut world = spawn_pinned_world(config(
            ServerWorldConfig {
                datapacks: Some(Arc::new(startup)),
                ..Default::default()
            },
            recipe,
            ingredients,
        ));
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        if !special {
            let mut book = world
                .player_inventory_for_test(ConnectionId(1))
                .recipe_book()
                .clone();
            book.award(recipe);
            world
                .player_inventory_for_test(ConnectionId(1))
                .apply_recipe_book(book);
        }
        let mut item =
            if special {
                ItemStackSummary {
                    item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_PICKAXE_ID),
                    count: 1,
                    component_patch: bbb_protocol::packets::DataComponentPatchSummary {
                        added_type_ids: vec![
                        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                    ],
                        damage: Some(100),
                        ..Default::default()
                    }
                    .into(),
                }
            } else {
                ItemStackSummary {
                item_id: Some(VANILLA_ITEM_OAK_LOG_ID), count: 3,
                component_patch: bbb_registry_codec::read_item_component(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                    &bbb_protocol::NbtSummaryValue::String("Recipe input".into()),
                    crate::world::registry_runtime::item_stack_resolver_for_test(),
                ).unwrap().into(),
            }
            };
        world.stock_menu_slot_for_test(ConnectionId(1), 1, item.clone());
        if special {
            item.component_patch.damage = Some(90);
            world.stock_menu_slot_for_test(ConnectionId(1), 2, item);
        }
        // A packet take drains the menu's native take event in the world.
        for _ in 0..2 {
            let inventory = world.player_inventory_for_test(ConnectionId(1));
            let packet = ContainerClick {
                container_id: inventory.container_id(),
                state_id: inventory.state_id(),
                slot_num: 0,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            };
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::ContainerClick(packet),
            );
        }
        assert!(
            world.player_advancement_is_done(ConnectionId(1), "test:crafted"),
            "special={special}"
        );
        assert_eq!(world.experience_total_for_test(ConnectionId(1)), 5);
    }
}
