//! Item durability advancements through player actions and store interactions.

use super::*;
use crate::tests::using_item::{
    stack, Fixture, ALICE, BOB, HOTBAR_MENU_SLOT, HOTBAR_SLOT, OFFHAND_SLOT,
};
use serde_json::{json, Value};

fn config(criteria: &[(&str, Value)]) -> ServerWorldConfig {
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (id, conditions) in criteria {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: (*id).into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&json!({
                    "criteria":{"wear":{"trigger":"minecraft:item_durability_changed",
                        "conditions":conditions}}
                }))
                .unwrap(),
            },
        );
    }
    config
}

fn config_with_processed_wear(criteria: &[(&str, Value)], amount: i32) -> ServerWorldConfig {
    config_with_wear_effect(
        criteria,
        json!({"effect":{"type":"minecraft:set","value":amount}}),
    )
}

fn config_with_wear_effect(criteria: &[(&str, Value)], effect: Value) -> ServerWorldConfig {
    let mut config = config(criteria);
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let unbreaking = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:enchantment")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:unbreaking")
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&unbreaking.source_json).unwrap();
    json["effects"]["minecraft:item_damage"] = json!([effect]);
    unbreaking.source_json = serde_json::to_vec(&json).unwrap().into();
    unbreaking.source =
        Some(bbb_registry_codec::decode_direct_registry_json(&unbreaking.source_json).unwrap());
    config
}

fn done(fixture: &Fixture, id: &str) -> bool {
    fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), id)
}

fn release(fixture: &mut Fixture) -> TickResult {
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..12 {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem)
}

#[test]
fn bow_wear_conditions_read_the_named_clock_and_current_dimension() {
    let config = config_with_wear_effect(
        &[],
        json!({
            "effect":{"type":"minecraft:set","value":7},
            "requirements":{"condition":"minecraft:all_of","terms":[
                {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107},
                {"condition":"minecraft:environment_attribute_check",
                 "attribute":"minecraft:gameplay/fast_lava","value":false}
            ]}
        }),
    );
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.command("time of minecraft:the_end pause");
    for (time, damage) in [(107, 17), (108, 11)] {
        fixture.command(&format!("time of minecraft:the_end set {time}"));
        fixture.command("item replace entity alice weapon.mainhand with bow[damage=10,enchantments={unbreaking:1}]");
        fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 2));
        release(&mut fixture);
        assert_eq!(
            fixture.slot(HOTBAR_SLOT).component_patch.damage,
            Some(damage)
        );
    }
}

#[test]
fn a_bow_checks_the_original_complete_stack_and_negative_wear_delta() {
    let condition = json!({
        "player":{"slots":{"inventory.2":{"items":"minecraft:diamond"}}},
        "item":{"items":"minecraft:bow","count":1,
            "components":{"minecraft:damage":2,"minecraft:custom_data":{"marked":17}}},
        "delta":-1,"durability":2
    });
    let mut changed_stack = condition.clone();
    changed_stack["item"]["components"]["minecraft:damage"] = json!(3);
    let mut wrong_player = condition.clone();
    wrong_player["player"]["slots"]["inventory.2"]["items"] = json!("minecraft:emerald");
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config(&[
            ("test:bow_wear", condition),
            ("test:changed_stack", changed_stack),
            ("test:wrong_player", wrong_player),
        ]),
    );
    fixture.command("item replace entity alice inventory.2 with diamond");
    fixture.command("item replace entity alice weapon.mainhand with bow[damage=2,max_damage=5,custom_data={marked:17b}]");
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 2));
    let tick = release(&mut fixture);
    assert!(done(&fixture, "test:bow_wear"));
    assert!(!done(&fixture, "test:changed_stack"));
    assert!(!done(&fixture, "test:wrong_player"));
    assert_eq!(fixture.slot(HOTBAR_SLOT).component_patch.damage, Some(3));
    assert!(update_advancements(&tick, ALICE).iter().any(|update| update
        .progress
        .iter()
        .any(|progress| progress.id == "test:bow_wear")));
}

#[test]
fn a_shield_break_matches_the_old_stack_and_raw_overdamage() {
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config(&[
            (
                "test:raw_break",
                json!({"item":{"items":"minecraft:shield","count":1,
            "components":{"minecraft:damage":1,"minecraft:custom_data":{"marked":17}}},
            "delta":-5,"durability":-4}),
            ),
            ("test:clamped_break", json!({"durability":0})),
        ]),
    );
    let alice = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    fixture.world.set_player_position_for_test(
        ConnectionId(BOB),
        Vec3d {
            z: alice.z + 2.0,
            ..alice
        },
    );
    fixture.command("item replace entity alice weapon.offhand with shield[damage=1,max_damage=2,custom_data={marked:17b}]");
    fixture.use_item(InteractionHand::OffHand);
    for _ in 1..5 {
        fixture.idle();
    }
    fixture.command("damage alice 4 minecraft:arrow by bob");
    assert!(done(&fixture, "test:raw_break"));
    assert!(!done(&fixture, "test:clamped_break"));
    assert_eq!(fixture.slot(OFFHAND_SLOT), ItemStackSummary::empty());
}

#[test]
fn throwing_a_trident_uses_the_nonbreaking_damage_trigger() {
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config(&[(
            "test:trident_wear",
            json!({"item":{"items":"minecraft:trident",
            "components":{"minecraft:damage":1}},"delta":-1,"durability":1}),
        )]),
    );
    fixture
        .command("item replace entity alice weapon.mainhand with trident[damage=1,max_damage=3]");
    release(&mut fixture);
    assert!(done(&fixture, "test:trident_wear"));
    assert_eq!(fixture.slot(HOTBAR_SLOT), ItemStackSummary::empty());
}

#[test]
fn creative_and_zero_processed_wear_do_not_trigger() {
    let config = config_with_processed_wear(&[("test:wear", json!({}))], 0);
    for game_type in [GameType::Survival, GameType::Creative] {
        let mut fixture = Fixture::with_config(game_type, config.clone());
        fixture.command(if game_type == GameType::Creative {
            "item replace entity alice weapon.mainhand with bow"
        } else {
            "item replace entity alice weapon.mainhand with bow[enchantments={unbreaking:1}]"
        });
        fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 2));
        release(&mut fixture);
        assert!(!done(&fixture, "test:wear"));
        assert_eq!(
            crate::item_durability::damage_and_max(&fixture.slot(HOTBAR_SLOT))
                .unwrap()
                .0,
            0
        );
    }
}

#[test]
fn cutting_a_leash_carries_the_old_shears_from_store_to_world() {
    use crate::entity::{EntityData, GeneralLivingData};
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config(&[(
            "test:leash_wear",
            json!({"item":{"items":"minecraft:shears","count":1,
            "components":{"minecraft:damage":3,"minecraft:custom_data":{"marked":17}}},
            "delta":-1,"durability":6}),
        )]),
    );
    let position = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap();
    data.set_no_ai(true);
    let cow = fixture
        .world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: position.x + 1.0,
                ..position
            },
            EntityData::GeneralLiving(data),
        )
        .0;
    fixture
        .world
        .apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:lead", 1));
    let interact = PlayServerbound::Interact(InteractEntity {
        entity_id: cow,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: false,
    });
    fixture.play(ALICE, interact.clone());
    fixture.command("item replace entity alice weapon.mainhand with shears[damage=3,max_damage=10,custom_data={marked:17b}]");
    fixture.play(ALICE, interact);
    assert!(done(&fixture, "test:leash_wear"));
    assert_eq!(fixture.slot(HOTBAR_SLOT).component_patch.damage, Some(4));
    let cow = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
        .unwrap();
    assert_eq!(crate::entity::leash::leash_holder(cow), None);
}

#[test]
fn a_negative_processed_wear_matches_java_int_durability_without_overflowing() {
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config_with_processed_wear(
            &[(
                "test:negative_wear",
                json!({"item":{"items":"minecraft:bow",
                "components":{"minecraft:damage":0}},
                "durability":i32::MIN,"delta":1}),
            )],
            -1,
        ),
    );
    fixture.command("item replace entity alice weapon.mainhand with bow[max_damage=2147483647,enchantments={unbreaking:1}]");
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 1));
    release(&mut fixture);
    assert!(done(&fixture, "test:negative_wear"));
    assert_eq!(fixture.slot(HOTBAR_SLOT).component_patch.damage, Some(0));
}

#[test]
fn a_large_datapack_wear_value_keeps_the_tool_and_completes_its_criterion() {
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config_with_processed_wear(
            &[(
                "test:large_wear",
                json!({"item":{"items":"minecraft:bow",
                "components":{"minecraft:damage":1}},
                "durability":-1,"delta":-i32::MAX}),
            )],
            i32::MAX,
        ),
    );
    fixture.command("item replace entity alice weapon.mainhand with bow[damage=1,max_damage=2147483647,enchantments={unbreaking:1}]");
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 1));
    release(&mut fixture);
    assert!(done(&fixture, "test:large_wear"));
    assert_eq!(fixture.slot(HOTBAR_SLOT).component_patch.damage, Some(0));
}
