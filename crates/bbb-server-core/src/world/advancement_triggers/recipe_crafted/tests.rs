use std::sync::{Arc, OnceLock};

use crate::tests::parse_advancement;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{InteractionHand, ItemStackSummary};
use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use serde_json::{json, Value};

use super::*;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::ServerWorldConfig;

const PLAYER: ConnectionId = ConnectionId(1);
const RECIPE: &str = "minecraft:stone_button";

fn world(definitions: &[(&str, Value)]) -> ServerWorld {
    static BUILTIN: OnceLock<DatapackStartup> = OnceLock::new();
    let mut startup = BUILTIN
        .get_or_init(|| {
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .unwrap()
        })
        .clone();
    startup.advancements = AdvancementSet {
        advancements: definitions
            .iter()
            .map(|(id, body)| AdvancementElement {
                id: (*id).into(),
                source_pack: "test".into(),
                advancement: parse_advancement(body).unwrap(),
            })
            .collect(),
        warnings: Vec::new(),
    };
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "crafter", 1);
    {
        let mut position = world.levels[PLAYER].position();
        position.y = 64.0;
        world.levels[PLAYER].set_position(position);
    }
    world
}

fn advancement(conditions: Value, experience: i32) -> Value {
    json!({"criteria":{"crafted":{"trigger":"crafter_recipe_crafted","conditions":conditions},"player":{"trigger":"recipe_crafted","conditions":conditions}},
        "requirements":[["crafted"],["player"]],
        "rewards":{"experience":experience}})
}

fn grid(items: &[(&str, i32)]) -> Vec<RuntimeItemStack> {
    let mut grid = vec![RuntimeItemStack::empty(); 9];
    for (slot, (name, count)) in items.iter().enumerate() {
        grid[slot] = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(vanilla_item_type_for_resource_id(name).unwrap().id),
            count: *count,
            component_patch: Default::default(),
        });
    }
    grid
}

fn fire(world: &mut ServerWorld, recipe_id: &str, grid: &[RuntimeItemStack]) {
    world.trigger_recipe_crafted(
        PLAYER,
        CRAFTER_RECIPE_CRAFTED,
        recipe_id,
        grid,
        &mut Vec::new(),
    );
    world.trigger_recipe_crafted(PLAYER, RECIPE_CRAFTED, recipe_id, grid, &mut Vec::new());
}

#[test]
fn crafter_recipe_crafted_uses_recipe_and_greedy_distinct_input_slots_before_reward() {
    let mut world = world(&[
        (
            "test:any",
            advancement(json!({"recipe_id":"stone_button"}), 5),
        ),
        (
            "test:distinct",
            advancement(
                json!({"recipe_id":RECIPE,"ingredients":[{"items":"stone"},{"items":"stone"}]}),
                3,
            ),
        ),
        (
            "test:greedy",
            advancement(
                json!({"recipe_id":RECIPE,"ingredients":[
                    {"items":["stone","dirt"]},{"items":"stone"}
                ]}),
                7,
            ),
        ),
    ]);
    let first = grid(&[("minecraft:stone", 64), ("minecraft:dirt", 1)]);
    let original = first.clone();
    fire(&mut world, "minecraft:oak_planks", &first);
    assert_eq!(world.levels[PLAYER].experience.total(), 0);
    fire(&mut world, RECIPE, &first);
    assert_eq!(world.levels[PLAYER].experience.total(), 5);
    assert!(!world.player_advancement_is_done(PLAYER, "test:distinct"));
    assert!(!world.player_advancement_is_done(PLAYER, "test:greedy"));
    assert_eq!(first, original);

    let reordered = grid(&[("minecraft:dirt", 1), ("minecraft:stone", 64)]);
    fire(&mut world, RECIPE, &reordered);
    assert!(world.player_advancement_is_done(PLAYER, "test:greedy"));
    assert_eq!(world.levels[PLAYER].experience.total(), 12);
    let separate = grid(&[("minecraft:stone", 64), ("minecraft:stone", 1)]);
    fire(&mut world, RECIPE, &separate);
    fire(&mut world, RECIPE, &separate);
    assert!(world.player_advancement_is_done(PLAYER, "test:distinct"));
    assert_eq!(world.levels[PLAYER].experience.total(), 15);
    assert_eq!(
        world.levels[PLAYER]
            .cold
            .advancements
            .trigger_listeners(CRAFTER_RECIPE_CRAFTED)
            .count(),
        0
    );
}

#[test]
fn crafter_recipe_crafted_player_predicates_and_refusals_use_real_session_context() {
    let predicate = json!({"type":"minecraft:player","distance":{"absolute":{"max":0}},
        "nbt":"{SelectedItem:{id:'minecraft:diamond',count:1}}",
        "location":{"position":{"y":64}},"flags":{"is_on_ground":true}});
    let mut world = world(&[
        (
            "test:object",
            advancement(json!({"recipe_id":RECIPE,"player":predicate}), 2),
        ),
        (
            "test:list",
            advancement(
                json!({"recipe_id":RECIPE,"player":[{
                    "condition":"entity_properties","entity":"this","predicate":predicate
                }]}),
                3,
            ),
        ),
        (
            "test:refused",
            advancement(
                json!({"recipe_id":RECIPE,"player":{
                    "type_specific":{"type":"minecraft:fishing_hook","in_open_water":true}
                }}),
                99,
            ),
        ),
    ]);
    let ingredients = grid(&[("minecraft:stone", 1)]);
    let diamond = grid(&[("minecraft:diamond", 1)])[0].clone();
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(InteractionHand::MainHand, diamond.clone());
    world.levels[PLAYER].set_on_ground(false);
    fire(&mut world, "minecraft:oak_planks", &ingredients);
    fire(&mut world, RECIPE, &ingredients);
    assert_eq!(world.levels[PLAYER].experience.total(), 0);
    world.levels[PLAYER].set_on_ground(true);
    world.levels[PLAYER].inventory.set_item_in_hand(
        InteractionHand::MainHand,
        grid(&[("minecraft:stick", 1)])[0].clone(),
    );
    fire(&mut world, RECIPE, &ingredients);
    assert_eq!(world.levels[PLAYER].experience.total(), 0);
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(InteractionHand::MainHand, diamond);
    fire(&mut world, RECIPE, &ingredients);
    fire(&mut world, RECIPE, &ingredients);
    assert_eq!(world.levels[PLAYER].experience.total(), 5);
    assert!(world.player_advancement_is_done(PLAYER, "test:object"));
    assert!(world.player_advancement_is_done(PLAYER, "test:list"));
    assert!(!world.player_advancement_is_done(PLAYER, "test:refused"));
}
