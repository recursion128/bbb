use super::*;

use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{ItemStackSummary, MovePlayer, PlayServerbound};
use std::sync::Arc;

fn living_vehicle(world: &mut ServerWorld, kind: i32) -> i32 {
    let position = world.levels[ALICE].position();
    let id = entities(world)
        .spawn_general_living(kind, position)
        .unwrap();
    entities(world)
        .get_mut(id)
        .unwrap()
        .set_delta_movement(VEHICLE);
    id
}

fn mount_living(world: &mut ServerWorld, player: ConnectionId, vehicle: i32) {
    let player_id = world.levels[player].entity_id();
    assert!(entities(world).attach_test_player(player_id, vehicle, ExternalPlayerCanRide::Allowed));
}

fn equip(
    world: &mut ServerWorld,
    vehicle: i32,
    slot: EquipmentSlot,
    resource: &str,
    equippable: Option<serde_json::Value>,
) {
    let mut value = serde_json::json!({"id":resource,"count":1});
    if let Some(equippable) = equippable {
        value["components"] = serde_json::json!({"minecraft:equippable":equippable});
    }
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &bbb_protocol::json_value_to_nbt_summary(&value).unwrap(),
        &crate::tests::LenientTags(&world.item_stack_resolver_for_test()),
    )
    .unwrap();
    entities(world)
        .get_mut(vehicle)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(slot, Some(stack));
}

fn tame(world: &mut ServerWorld, vehicle: i32) {
    let NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, vehicle)
        .unwrap()
    else {
        panic!()
    };
    entries.retain(|row| row.name != "Tame" && row.name != "Owner");
    entries.push(NbtSummaryEntry {
        name: "Tame".into(),
        value: NbtSummaryValue::Byte(1),
    });
    entries.push(NbtSummaryEntry {
        name: "Owner".into(),
        value: NbtSummaryValue::IntArray(
            world.levels[ALICE]
                .profile
                .uuid
                .as_u128()
                .to_be_bytes()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&word| i32::from_be_bytes(word))
                .collect(),
        ),
    });
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, vehicle, &entries);
}

fn held(world: &mut ServerWorld, off_hand: bool, resource: Option<&str>) {
    let stack = resource
        .map(|resource| ItemStackSummary {
            item_id: Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource)
                    .unwrap()
                    .id,
            ),
            count: 1,
            component_patch: Default::default(),
        })
        .unwrap_or_else(ItemStackSummary::empty);
    world.stock_menu_slot_for_test(ALICE, if off_hand { 45 } else { 36 }, stack);
}

#[test]
fn known_movement_equipped_horse_family_and_nautilus_read_live_session_input() {
    for kind in [
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_DONKEY_ID,
        VANILLA_ENTITY_TYPE_MULE_ID,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    ] {
        let mut world = world();
        let vehicle = living_vehicle(&mut world, kind);
        mount_living(&mut world, ALICE, vehicle);
        assert_eq!(world.player_known_movement(ALICE), Some(VEHICLE));
        tame(&mut world, vehicle);
        equip(
            &mut world,
            vehicle,
            EquipmentSlot::Saddle,
            "minecraft:stone",
            Some(serde_json::json!({"slot":"saddle"})),
        );
        assert_eq!(world.player_known_movement(ALICE), Some(RAW), "kind {kind}");
    }
}

#[test]
fn known_movement_pig_and_strider_read_both_session_hands_and_require_valid_saddle() {
    for (kind, steering) in [
        (VANILLA_ENTITY_TYPE_PIG_ID, "minecraft:carrot_on_a_stick"),
        (
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            "minecraft:warped_fungus_on_a_stick",
        ),
    ] {
        let mut world = world();
        let vehicle = living_vehicle(&mut world, kind);
        mount_living(&mut world, ALICE, vehicle);
        equip(
            &mut world,
            vehicle,
            EquipmentSlot::Saddle,
            "minecraft:saddle",
            None,
        );
        assert_eq!(world.player_known_movement(ALICE), Some(VEHICLE));
        for off_hand in [false, true] {
            held(&mut world, !off_hand, None);
            held(&mut world, off_hand, Some(steering));
            assert_eq!(world.player_known_movement(ALICE), Some(RAW));
        }
        equip(
            &mut world,
            vehicle,
            EquipmentSlot::Saddle,
            "minecraft:stone",
            None,
        );
        assert_eq!(world.player_known_movement(ALICE), Some(VEHICLE));
        equip(
            &mut world,
            vehicle,
            EquipmentSlot::Saddle,
            "minecraft:stone",
            Some(serde_json::json!({"slot":"body"})),
        );
        assert_eq!(world.player_known_movement(ALICE), Some(VEHICLE));
    }
}

#[test]
fn known_movement_happy_still_to_controller_and_rear_rider_share_gear_owner() {
    let mut world = world();
    let happy = living_vehicle(&mut world, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID);
    mount_living(&mut world, ALICE, happy);
    mount_living(&mut world, BOB, happy);
    equip(
        &mut world,
        happy,
        EquipmentSlot::Body,
        "minecraft:white_harness",
        None,
    );
    let set_timeout = |world: &mut ServerWorld, timeout: i32| {
        let NbtSummaryValue::Compound(mut entries) = world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, happy)
            .unwrap()
        else {
            panic!()
        };
        entries
            .iter_mut()
            .find(|row| row.name == "still_timeout")
            .unwrap()
            .value = NbtSummaryValue::Int(timeout);
        world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            happy,
            &entries,
        );
    };
    set_timeout(&mut world, 10);
    assert_eq!(world.player_known_movement(ALICE), Some(VEHICLE));
    assert_eq!(world.player_known_movement(BOB), Some(VEHICLE));
    set_timeout(&mut world, 0);
    assert_eq!(world.player_known_movement(ALICE), Some(RAW));
    assert_eq!(world.player_known_movement(BOB), Some(RAW));
}

#[test]
fn known_movement_loaded_equipment_tag_controls_membership() {
    let mut world = world();
    let horse = living_vehicle(&mut world, VANILLA_ENTITY_TYPE_HORSE_ID);
    mount_living(&mut world, ALICE, horse);
    equip(
        &mut world,
        horse,
        EquipmentSlot::Saddle,
        "minecraft:stone",
        Some(serde_json::json!({"slot":"saddle","allowed_entities":"#example:mounts"})),
    );
    for (tag, expected) in [
        (None, VEHICLE),
        (Some(vec!["minecraft:horse".to_owned()]), RAW),
        (Some(vec!["minecraft:pig".to_owned()]), VEHICLE),
    ] {
        let tags = tag.map(|members| {
            (
                "minecraft:entity_type".to_owned(),
                "example:mounts".to_owned(),
                members,
            )
        });
        world
            .level_mut(Java26BuiltinDimension::Overworld)
            .chunks
            .install_registry_tags(Arc::new(RegistryTagSnapshot::new(tags)));
        assert_eq!(world.player_known_movement(ALICE), Some(expected));
    }
}

#[test]
fn known_movement_leaf_horse_controller_keeps_self_input_when_vehicle_or_player_health_zero() {
    let mut world = world();
    let horse = living_vehicle(&mut world, VANILLA_ENTITY_TYPE_HORSE_ID);
    mount_living(&mut world, ALICE, horse);
    equip(
        &mut world,
        horse,
        EquipmentSlot::Saddle,
        "minecraft:stone",
        Some(serde_json::json!({"slot":"saddle"})),
    );
    world.set_player_health_for_test(ALICE, 0.0);
    entities(&mut world)
        .get_mut(horse)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    assert_eq!(world.player_known_movement(ALICE), Some(RAW));
    entities(&mut world)
        .get_mut(horse)
        .unwrap()
        .set_removed(RemovalReason::Killed);
    assert_eq!(world.player_known_movement(ALICE), Some(RAW));
}

#[test]
fn known_movement_zero_client_input_survives_nonzero_passenger_placement() {
    let mut world = world();
    world.apply_play(ALICE, PlayServerbound::ClientTickEnd, &mut Vec::new());
    assert_eq!(world.levels[ALICE].known_movement, Vec3d::default());
    let boat = boat(&mut world, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    mount(&mut world, ALICE, boat);
    let dimension = world.levels[ALICE].dimension;
    world.sync_external_players_after_vehicle_tick(&dimension, boat, &mut Vec::new());
    let before = world.levels[ALICE].position();
    let vehicle = entities(&mut world).get_mut(boat).unwrap();
    vehicle.set_pos(Vec3d {
        x: vehicle.position().x + 1.0,
        ..vehicle.position()
    });
    world.sync_external_players_after_vehicle_tick(&dimension, boat, &mut Vec::new());
    assert_ne!(world.levels[ALICE].position(), before);
    assert_eq!(world.player_known_movement(ALICE), Some(Vec3d::default()));
}

#[test]
fn known_movement_actual_move_packet_supplies_equipped_controller_input() {
    let mut world = world();
    let start = Vec3d {
        x: 0.5,
        y: 300.0,
        z: 0.5,
    };
    world.set_player_position_for_test(ALICE, start);
    world.apply_play(
        ALICE,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d { x: 0.75, ..start }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
        &mut Vec::new(),
    );
    let input = world.levels[ALICE].known_movement;
    assert_eq!(
        input,
        Vec3d {
            x: 0.25,
            y: 0.0,
            z: 0.0
        }
    );
    let horse = living_vehicle(&mut world, VANILLA_ENTITY_TYPE_HORSE_ID);
    mount_living(&mut world, ALICE, horse);
    equip(
        &mut world,
        horse,
        EquipmentSlot::Saddle,
        "minecraft:stone",
        Some(serde_json::json!({"slot":"saddle"})),
    );
    assert_eq!(world.player_known_movement(ALICE), Some(input));
}
