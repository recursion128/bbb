//! Sheared equipment advancements through the real entity interaction packet.

use super::*;
use crate::entity::{EntityData, GeneralLivingData};
use crate::tests::using_item::{stack, Fixture, ALICE, HOTBAR_MENU_SLOT};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use serde_json::json;

#[test]
fn shearing_matches_the_removed_stack_and_live_target_and_rewards_once() {
    let condition = json!({
        "player":{"slots":{"inventory.2":{"items":"minecraft:diamond"}}},
        "item":{"items":"minecraft:saddle","count":1,
            "components":{"minecraft:custom_data":{"marked":17}}},
        "entity":{"type":"minecraft:pig","slots":{"saddle":{"count":0}},
            "distance":{"absolute":{"min":0.5,"max":2}}}
    });
    let mut wrong_player = condition.clone();
    wrong_player["player"]["slots"]["inventory.2"]["items"] = json!("minecraft:emerald");
    let mut wrong_item = condition.clone();
    wrong_item["item"] = json!({"items":"minecraft:shears"});
    let mut wrong_component = condition.clone();
    wrong_component["item"]["components"]["minecraft:custom_data"]["marked"] = json!(18);
    let mut occupied_target = condition.clone();
    occupied_target["entity"]["slots"]["saddle"] = json!({"items":"minecraft:saddle"});
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (id, conditions, experience) in [
        ("test:sheared_equipment", condition, 7),
        ("test:any_equipment_shear", json!({}), 0),
        ("test:wrong_player", wrong_player, 0),
        ("test:held_shears", wrong_item, 0),
        ("test:wrong_component", wrong_component, 0),
        ("test:occupied_target", occupied_target, 0),
    ] {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&json!({
                    "criteria":{"sheared":{"trigger":"minecraft:player_sheared_equipment",
                        "conditions":conditions}},
                    "rewards":{"experience":experience}
                }))
                .unwrap(),
            },
        );
    }
    let mut fixture = Fixture::with_config(GameType::Survival, config);
    fixture.world.tick_rate_mut().set_frozen(true);
    fixture.command("item replace entity alice inventory.2 with diamond");
    let position = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIG_ID).unwrap();
    data.set_no_ai(true);
    let pig = fixture
        .world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_PIG_ID,
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
    let pig_uuid = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
        .unwrap()
        .uuid();
    let equip =
        format!("item replace entity {pig_uuid} saddle with saddle[custom_data={{marked:17b}}]");
    fixture.command(&equip);
    let original = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::Saddle)
        .unwrap()
        .clone();
    assert!(original.summary().component_patch.custom_data.is_some());
    let interact = PlayServerbound::Interact(InteractEntity {
        entity_id: pig,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: false,
    });
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:lead", 1));
    fixture.play(ALICE, interact.clone());
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:shears", 1));
    fixture.play(ALICE, interact.clone());
    assert!(!fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), "test:any_equipment_shear"));
    assert_eq!(
        fixture
            .world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
            .unwrap()
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::Saddle),
        Some(&original)
    );

    let tick = fixture.play(ALICE, interact.clone());
    for id in ["test:sheared_equipment", "test:any_equipment_shear"] {
        assert!(
            fixture
                .world
                .player_advancement_is_done(ConnectionId(ALICE), id),
            "{id}"
        );
    }
    for id in [
        "test:wrong_player",
        "test:held_shears",
        "test:wrong_component",
        "test:occupied_target",
    ] {
        assert!(
            !fixture
                .world
                .player_advancement_is_done(ConnectionId(ALICE), id),
            "{id}"
        );
    }
    assert!(update_advancements(&tick, ALICE).iter().any(|update| update
        .progress
        .iter()
        .any(|progress| progress.id == "test:sheared_equipment")));
    assert!(fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::Saddle)
        .is_none());
    assert_eq!(
        fixture.world.experience_total_for_test(ConnectionId(ALICE)),
        7
    );
    fixture.command(&equip);
    fixture.play(ALICE, interact);
    assert_eq!(
        fixture.world.experience_total_for_test(ConnectionId(ALICE)),
        7
    );
}

/// `Entity.interact` (`Entity.java:2195-2196`) and `attemptToShearEquipment`
/// (`:2280`) spend shears through `hurtAndBreak(1, player, hand)`: a last
/// point breaks them, empties the hand and counts `minecraft:broken`.
#[test]
fn cutting_a_leash_or_shearing_equipment_can_break_the_shears() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture.world.tick_rate_mut().set_frozen(true);
    let position = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIG_ID).unwrap();
    data.set_no_ai(true);
    let pig = fixture
        .world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_PIG_ID,
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
    let pig_uuid = fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
        .unwrap()
        .uuid();
    fixture.command(&format!(
        "item replace entity {pig_uuid} saddle with saddle"
    ));
    let interact = PlayServerbound::Interact(InteractEntity {
        entity_id: pig,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: false,
    });
    let broken = |fixture: &Fixture| {
        fixture.world.player_stat_for_test(
            ConnectionId(ALICE),
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_BROKEN_ID,
                item_id("minecraft:shears"),
            ),
        )
    };
    let saddled = |fixture: &Fixture| {
        fixture
            .world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
            .unwrap()
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::Saddle)
            .is_some()
    };
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:lead", 1));
    fixture.play(ALICE, interact.clone());
    fixture.command("item replace entity alice hotbar.0 with shears[damage=237]");
    fixture.play(ALICE, interact.clone());
    assert!(
        bbb_server_types::item_stack::RuntimeItemStack::from_summary(fixture.slot(0)).is_empty()
    );
    assert_eq!(broken(&fixture), 1);
    assert!(saddled(&fixture));

    fixture.command("item replace entity alice hotbar.0 with shears[damage=237]");
    fixture.play(ALICE, interact);
    assert!(!saddled(&fixture));
    assert!(
        bbb_server_types::item_stack::RuntimeItemStack::from_summary(fixture.slot(0)).is_empty()
    );
    assert_eq!(broken(&fixture), 2);
}
