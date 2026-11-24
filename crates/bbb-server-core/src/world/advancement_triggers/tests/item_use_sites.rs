use super::*;
use crate::entity::Entity;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
};
use bbb_protocol::NbtSummaryValue;

fn packet(world: &mut ServerWorld, packet: PlayServerbound) {
    world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
        world.last_command_sequence + 1,
        crate::mailbox::WorldProducerClass::Network,
        WorldCommand::Play {
            connection_id: CONNECTION,
            packet,
        },
    )]);
}

fn interact(world: &mut ServerWorld, entity_id: i32) {
    packet(
        world,
        PlayServerbound::Interact(bbb_protocol::packets::InteractEntity {
            entity_id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
}

fn entities_of<'w>(world: &'w ServerWorld, kind: &str) -> Vec<&'w Entity> {
    let store = &world.level(Java26BuiltinDimension::Overworld).entities;
    let mut ids = store.ids();
    ids.sort_unstable();
    ids.into_iter()
        .filter_map(|id| store.get(id))
        .filter(|entity| entity.data().entity_resource_id() == kind)
        .collect()
}

fn held(world: &ServerWorld) -> &ItemStackSummary {
    world.levels[CONNECTION]
        .inventory
        .selected_stack()
        .summary()
}

fn string(value: &str) -> Option<NbtSummaryValue> {
    Some(NbtSummaryValue::String(value.into()))
}

/// `BlockItem.useOn` (`BlockItem.java:41-46`): a carrot that cannot be
/// planted on stone is eaten instead.
#[test]
fn a_consumable_block_item_that_cannot_be_placed_is_eaten() {
    let mut world = world(&[]);
    player_loaded(&mut world);
    command(&mut world, "gamemode survival");
    world.levels[CONNECTION].food.set_food_level(10);
    let at = beside_player(&mut world, block_state("minecraft:stone", &[]));
    give(&mut world, "minecraft:carrot", 2);
    click(&mut world, at, 1);
    assert_eq!(
        world.block_state(
            Java26BuiltinDimension::Overworld,
            BlockPos { y: at.y + 1, ..at }
        ),
        Some(
            world
                .level(Java26BuiltinDimension::Overworld)
                .chunks
                .air_state_id()
        )
    );
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert_eq!(world.levels[CONNECTION].food.food_level(), 13);
    assert_eq!(held(&world).count, 1);
}

/// `HoneycombItem.java:148-150` and `AxeItem.java:73-75` fire before the
/// block changes, which is the only state the built-in criteria name.
#[test]
fn waxing_scraping_and_dewaxing_award_their_built_in_advancements() {
    let mut world = built_in_world();
    player_loaded(&mut world);
    command(&mut world, "gamemode survival");
    let at = beside_player(&mut world, block_state("minecraft:copper_block", &[]));
    give(&mut world, "minecraft:honeycomb", 1);
    click(&mut world, at, 1);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(block_state("minecraft:waxed_copper_block", &[]))
    );
    assert!(world.player_advancement_is_done(CONNECTION, "minecraft:husbandry/wax_on"));
    give(&mut world, "minecraft:iron_axe", 1);
    click(&mut world, at, 2);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(block_state("minecraft:copper_block", &[]))
    );
    assert!(world.player_advancement_is_done(CONNECTION, "minecraft:husbandry/wax_off"));
    assert!(!world.player_advancement_is_done(CONNECTION, "minecraft:adventure/lighten_up"));
    let bulb = |name| block_state(name, &[("lit", "true"), ("powered", "false")]);
    let at = beside_player(&mut world, bulb("minecraft:exposed_copper_bulb"));
    click(&mut world, at, 3);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(bulb("minecraft:copper_bulb"))
    );
    assert!(world.player_advancement_is_done(CONNECTION, "minecraft:adventure/lighten_up"));
}

/// `EntityType.java:1285-1287` and `Wolf.java:193-206`; a second egg on the
/// adult hands its components to the pup (`SpawnEggItem.java:175`).
#[test]
fn spawn_egg_components_reach_the_spawned_wolf_and_its_pup() {
    let mut world = world(&[]);
    player_loaded(&mut world);
    command(&mut world, "gamemode survival");
    let at = beside_player(&mut world, block_state("minecraft:stone", &[]));
    give(
        &mut world,
        "minecraft:wolf_spawn_egg[custom_name='Rex',wolf/variant='minecraft:ashen',wolf/collar='red',custom_data={tag:1b}]",
        1,
    );
    click(&mut world, at, 1);
    let wolves = entities_of(&world, "minecraft:wolf");
    assert_eq!(wolves.len(), 1);
    let wolf = wolves[0];
    assert_eq!(
        wolf.custom_name().map(|name| name.text().to_owned()),
        Some("Rex".into())
    );
    assert_eq!(
        wolf.loot_component(VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID)
            .unwrap(),
        string("minecraft:ashen")
    );
    assert_eq!(
        wolf.loot_component(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID)
            .unwrap(),
        string("red")
    );
    assert!(wolf.custom_data().iter().any(|entry| entry.name == "tag"));
    let parent = wolf.id();
    give(
        &mut world,
        "minecraft:wolf_spawn_egg[custom_name='Pup',wolf/variant='minecraft:black']",
        1,
    );
    interact(&mut world, parent);
    let pup = entities_of(&world, "minecraft:wolf")
        .into_iter()
        .find(|wolf| wolf.id() != parent)
        .expect("the egg spawned a pup");
    assert_eq!(
        pup.custom_name().map(|name| name.text().to_owned()),
        Some("Pup".into())
    );
    assert_eq!(
        pup.loot_component(VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID)
            .unwrap(),
        string("minecraft:black")
    );
}

/// `Mob.usePlayerItem` (`Mob.java:1154-1162`) leaves the food's remainder.
#[test]
fn feeding_the_last_food_leaves_its_use_remainder_in_hand() {
    let mut world = world(&[]);
    player_loaded(&mut world);
    command(&mut world, "gamemode survival");
    command(&mut world, "summon minecraft:cow ~ ~ ~1 {NoAI:1b}");
    let cow = entities_of(&world, "minecraft:cow")[0].id();
    give(
        &mut world,
        "minecraft:wheat[use_remainder={id:'minecraft:bowl'}]",
        1,
    );
    interact(&mut world, cow);
    assert_eq!(
        held(&world).item_id,
        Some(
            vanilla_item_type_for_resource_id("minecraft:bowl")
                .unwrap()
                .id
        )
    );
}

/// `PlayerPredicate.java:90-135` in `/execute if predicate`, a player trigger
/// and a `/loot` roll, all reading the online player's advancements.
#[test]
fn player_advancement_terms_answer_in_commands_triggers_and_loot() {
    let granted = json!({"criteria":{"done":{"trigger":"minecraft:impossible"}}});
    let player = json!({"type_specific":{"type":"minecraft:player",
        "advancements":{"test:granted":true}}});
    let mut world = world(&[
        ("test:granted", granted),
        (
            "test:tick",
            json!({"criteria":{"done":{"trigger":"minecraft:tick",
                "conditions":{"player":player}}}}),
        ),
    ]);
    player_loaded(&mut world);
    let predicate = "{condition:'minecraft:entity_properties',entity:'this',predicate:{type_specific:{type:'minecraft:player',advancements:{'test:granted':true}}}}";
    let check = |world: &mut ServerWorld| {
        command(world, "scoreboard players set @s probe 0");
        command(
            world,
            &format!("execute if predicate {predicate} run scoreboard players set @s probe 1"),
        );
        command(
            world,
            &format!("loot give @s loot {{pools:[{{rolls:1,entries:[{{type:'minecraft:item',name:'minecraft:diamond'}}],conditions:[{predicate}]}}]}}"),
        );
        world
            .scoreboard
            .board()
            .score("inventory_player", "probe")
            .map(|score| score.value)
    };
    command(&mut world, "scoreboard objectives add probe dummy");
    world.tick(Vec::new());
    assert!(!world.player_advancement_is_done(CONNECTION, "test:tick"));
    assert_eq!(check(&mut world), Some(0));
    assert_eq!(held(&world).item_id, None);
    command(&mut world, "advancement grant @s only test:granted");
    assert_eq!(check(&mut world), Some(1));
    assert_eq!(
        held(&world).item_id,
        Some(
            vanilla_item_type_for_resource_id("minecraft:diamond")
                .unwrap()
                .id
        )
    );
    world.tick(Vec::new());
    assert!(world.player_advancement_is_done(CONNECTION, "test:tick"));
}

/// `BottleItem.java:40-42`.
#[test]
fn bottling_dragon_breath_is_an_interaction_with_the_cloud() {
    let mut world = world(&[(
        "test:breath",
        json!({"criteria":{"done":{"trigger":"minecraft:player_interacted_with_entity",
            "conditions":{"item":{"items":"minecraft:glass_bottle"},
                "entity":{"type":"minecraft:area_effect_cloud"}}}}}),
    )]);
    player_loaded(&mut world);
    command(&mut world, "gamemode survival");
    command(
        &mut world,
        "summon minecraft:ender_dragon ~ ~20 ~ {NoAI:1b}",
    );
    let dragon = entities_of(&world, "minecraft:ender_dragon")[0].uuid();
    let words = dragon.as_u128();
    let owner = (0..4)
        .map(|i| ((words >> (96 - 32 * i)) as u32 as i32).to_string())
        .collect::<Vec<_>>()
        .join(",");
    command(
        &mut world,
        &format!(
            "summon minecraft:area_effect_cloud ~ ~ ~ {{Radius:3f,Duration:600,Owner:[I;{owner}]}}"
        ),
    );
    give(&mut world, "minecraft:glass_bottle", 1);
    packet(
        &mut world,
        PlayServerbound::UseItem(bbb_protocol::packets::UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    assert_eq!(
        held(&world).item_id,
        Some(
            vanilla_item_type_for_resource_id("minecraft:dragon_breath")
                .unwrap()
                .id
        )
    );
    assert!(world.player_advancement_is_done(CONNECTION, "test:breath"));
}
