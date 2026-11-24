use super::*;
use crate::tests::parse_advancement;
use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
use bbb_server_types::item_stack::RuntimeItemStack;
use serde_json::{json, Value};

const PLAYER: ConnectionId = ConnectionId(1);
const STAND: BlockPos = BlockPos { x: 0, y: 64, z: 1 };

fn advancement(conditions: Value, experience: i32) -> Value {
    json!({"criteria":{"taken":{"trigger":"minecraft:brewed_potion","conditions":conditions}},
        "rewards":{"experience":experience}})
}

fn setup(definitions: &[(&str, Value)]) -> (ServerWorld, WorldCommandSender, WorldCommandReceiver) {
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
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
    let mut world = spawn_pinned_world(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    });
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        STAND,
        brewing_stand_state([true, false, false]),
        SET_BLOCK_UPDATE_ALL
    ));
    put_bottle(&mut world, water_potion());
    let opened = right_click(&sender, &receiver, &mut world, 1, STAND, 1);
    assert!(opened_screen(&opened).is_some());
    (world, sender, receiver)
}

fn put_bottle(world: &mut ServerWorld, stack: ItemStackSummary) {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, STAND)
        .and_then(|entity| entity.brewing_stand_mut())
        .unwrap()
        .items_mut()[0] = native_stack(stack);
}

fn click(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    input: ContainerInput,
    button_num: i8,
) {
    let inventory = world.player_inventory_for_test(PLAYER);
    let packet = ContainerClick {
        container_id: inventory.container_id(),
        state_id: inventory.state_id(),
        slot_num: 0,
        button_num,
        input,
        changed_slots: BTreeMap::new(),
        carried_item: HashedStack::empty(),
    };
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::ContainerClick(packet),
    );
}

#[test]
fn bottle_slot_takes_award_through_pickup_shift_hotbar_and_throw() {
    use bbb_protocol::item_types::VANILLA_ITEM_GLASS_BOTTLE_ID;
    for (input, hotbar_count) in [
        (ContainerInput::Pickup, None),
        (ContainerInput::QuickMove, None),
        (ContainerInput::Swap, None),
        (ContainerInput::Swap, Some(1)),
        (ContainerInput::Swap, Some(2)),
        (ContainerInput::Throw, None),
    ] {
        let (mut world, sender, receiver) =
            setup(&[("test:take", advancement(json!({"potion":"water"}), 5))]);
        if let Some(count) = hotbar_count {
            world.player_inventory_for_test(PLAYER).set_inventory_slot(
                0,
                native_stack(ItemStackSummary {
                    item_id: Some(VANILLA_ITEM_GLASS_BOTTLE_ID),
                    count,
                    component_patch: Default::default(),
                }),
            );
        }
        click(&mut world, &sender, &receiver, input, 0);
        assert_eq!(
            world.experience_total_for_test(PLAYER),
            5,
            "{input:?} {hotbar_count:?}"
        );
        assert!(world.player_advancement_is_done(PLAYER, "test:take"));
        assert_ne!(
            brewing_stand_payload(&mut world, STAND).items()[0].summary(),
            &water_potion()
        );
    }
}

#[test]
fn player_object_and_list_conditions_match_the_taking_player_and_reward_once() {
    let predicate = json!({"type":"minecraft:player","flags":{"is_on_ground":true}});
    let world_conditions = |score, clock| {
        json!({"player":[
            {"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"team":"brewers","flags":{"is_on_ground":true}}},
            {"condition":"minecraft:entity_scores","entity":"this","scores":{"brews":score}},
            {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
                "target":{"type":"minecraft:fixed","name":"#enabled"},"score":"brews"},"range":1},
            {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":clock},
            {"condition":"minecraft:environment_attribute_check",
                "attribute":"minecraft:gameplay/water_evaporates","value":false},
            {"condition":"minecraft:location_check","predicate":{"biomes":"minecraft:plains"}}
        ]})
    };
    let (mut world, sender, receiver) = setup(&[
        ("test:any", advancement(json!({}), 2)),
        (
            "test:object",
            advancement(json!({"potion":"minecraft:water","player":predicate}), 3),
        ),
        (
            "test:list",
            advancement(
                json!({"player":[{"condition":"minecraft:entity_properties",
            "entity":"this","predicate":predicate}]}),
                4,
            ),
        ),
        (
            "test:other",
            advancement(json!({"potion":"minecraft:awkward"}), 11),
        ),
        ("test:world", advancement(world_conditions(2, 107), 5)),
        (
            "test:wrong_score",
            advancement(world_conditions(3, 107), 13),
        ),
        (
            "test:wrong_clock",
            advancement(world_conditions(2, 108), 17),
        ),
    ]);
    for (index, input) in [
        "team add brewers",
        "team join brewers alice",
        "scoreboard objectives add brews dummy",
        "scoreboard players set alice brews 2",
        "scoreboard players set #enabled brews 1",
        "time of minecraft:the_end pause",
        "time of minecraft:the_end set 107",
    ]
    .into_iter()
    .enumerate()
    {
        let result = crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut world,
            index as u64 + 10,
            input,
        );
        assert!(crate::tests::entities::console_command_answer(&result)
            .0
            .is_some());
    }
    world.set_player_ground_for_test(PLAYER, false, 0.0);
    click(&mut world, &sender, &receiver, ContainerInput::QuickMove, 0);
    assert_eq!(world.experience_total_for_test(PLAYER), 2);
    put_bottle(&mut world, water_potion());
    world.set_player_ground_for_test(PLAYER, true, 0.0);
    click(&mut world, &sender, &receiver, ContainerInput::QuickMove, 0);
    assert_eq!(world.experience_total_for_test(PLAYER), 14);
    assert!(world.player_advancement_is_done(PLAYER, "test:object"));
    assert!(world.player_advancement_is_done(PLAYER, "test:list"));
    assert!(!world.player_advancement_is_done(PLAYER, "test:other"));
    assert!(world.player_advancement_is_done(PLAYER, "test:world"));
    assert!(!world.player_advancement_is_done(PLAYER, "test:wrong_score"));
    assert!(!world.player_advancement_is_done(PLAYER, "test:wrong_clock"));
    put_bottle(&mut world, water_potion());
    click(&mut world, &sender, &receiver, ContainerInput::QuickMove, 0);
    assert_eq!(world.experience_total_for_test(PLAYER), 14);
}

#[test]
fn placement_cursor_exchange_and_bottles_without_base_potions_do_not_award() {
    let (mut world, sender, receiver) = setup(&[("test:take", advancement(json!({}), 5))]);
    let mut awkward = water_potion();
    awkward.component_patch.potion_contents.potion_id = Some(
        bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:awkward")
            .unwrap()
            .id,
    );
    world
        .player_inventory_for_test(PLAYER)
        .set_carried(native_stack(awkward));
    click(&mut world, &sender, &receiver, ContainerInput::Pickup, 0);
    assert_eq!(world.experience_total_for_test(PLAYER), 0);
    world
        .player_inventory_for_test(PLAYER)
        .set_carried(RuntimeItemStack::empty());
    let glass = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_GLASS_BOTTLE_ID),
        count: 1,
        component_patch: Default::default(),
    };
    let mut custom_only = water_potion();
    custom_only.component_patch.potion_contents.potion_id = None;
    custom_only.component_patch.potion_contents.custom_effects =
        vec![bbb_protocol::packets::MobEffectInstanceSummary {
            effect_id: bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(
                "minecraft:speed",
            )
            .unwrap()
            .id,
            amplifier: 0,
            duration: 200,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        }];
    for stack in [glass, custom_only] {
        put_bottle(&mut world, stack);
        click(&mut world, &sender, &receiver, ContainerInput::QuickMove, 0);
        assert_eq!(world.experience_total_for_test(PLAYER), 0);
    }
    world
        .player_inventory_for_test(PLAYER)
        .set_carried(native_stack(water_potion()));
    click(&mut world, &sender, &receiver, ContainerInput::Pickup, 0);
    assert_eq!(world.experience_total_for_test(PLAYER), 0);
    assert_eq!(
        brewing_stand_payload(&mut world, STAND).items()[0].summary(),
        &water_potion()
    );
    click(&mut world, &sender, &receiver, ContainerInput::Pickup, 0);
    assert_eq!(world.experience_total_for_test(PLAYER), 5);
}
