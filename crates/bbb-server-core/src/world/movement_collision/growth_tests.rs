use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_SLIME_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;

use super::Vec3d;
use crate::admin::{AdminEnvelope, AdminRequest, AdminResult, AdminScope};
use crate::mailbox::{
    world_command_mailbox, WorldCommandReceiver, WorldCommandSender, WorldMailboxLimits,
};
use crate::world::ServerWorld;
use crate::world::{TickResult, WorldCommand, WorldEvent};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn console_command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation_id: u64,
    command: &str,
) -> TickResult {
    sender
        .try_send(WorldCommand::Admin {
            envelope: AdminEnvelope {
                operation_id,
                request_id: format!("growth-{operation_id}"),
                principal: "test-agent".into(),
                scope: AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 1234,
            },
            request: AdminRequest::ExecuteCommand {
                command: command.into(),
            },
        })
        .unwrap();
    world.tick(receiver.drain())
}

fn console_command_answer(tick: &TickResult) -> (Option<i32>, String) {
    let outcome = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Admin(outcome) => Some(outcome),
            _ => None,
        })
        .expect("admin outcome");
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{outcome:?}");
    };
    (report.result, format!("{:?}", report.messages))
}

fn fixture() -> (ServerWorld, WorldCommandSender, WorldCommandReceiver) {
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "growth", 1);
    world.level_mut(DIM).chunks.load_column(0, 0);
    (world, sender, receiver)
}

fn spawn(world: &mut ServerWorld, ty: i32) -> i32 {
    let (id, _) = world.spawn_stored_entity_for_test(
        DIM,
        ty,
        Vec3d {
            x: 8.5,
            y: 120.0,
            z: 8.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(ty).unwrap()),
    );
    let NbtSummaryValue::Compound(mut record) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        panic!("record");
    };
    for name in ["NoAI", "NoGravity", "PersistenceRequired"] {
        record.retain(|field| field.name != name);
        record.push(NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Byte(1),
        });
    }
    world.load_stored_entity_record_for_test(DIM, id, &record);
    world.tick(Vec::new());
    assert!(
        !world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .is_first_tick(),
        "fixture must reach actual entity tick"
    );
    id
}

fn assert_center_preserved(before_y: f64, before_height: f32, body: &crate::entity::Entity) {
    assert!(
        (body.position().y + f64::from(body.bb_height()) / 2.0
            - before_y
            - f64::from(before_height) / 2.0)
            .abs()
            < 1.0e-6,
        "old feet={before_y}, old height={before_height}, new feet={:?}, new height={}",
        body.position(),
        body.bb_height()
    );
}

#[test]
fn growth_world_age_boundary_moves_the_live_body() {
    let (mut world, _, _) = fixture();
    let id = spawn(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
    assert!(crate::entity::general_living::set_spawner_age(body, -1));
    let (y, height) = (body.position().y, body.bb_height());
    world.tick(Vec::new());
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(body.bb_height(), height * 2.0);
    assert_center_preserved(y, height, body);
}

#[test]
fn growth_world_dirty_scale_moves_once_when_attributes_are_consumed() {
    let (mut world, _, _) = fixture();
    let id = spawn(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
    let (y, height) = (body.position().y, body.bb_height());
    body.living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(1.5);
    assert_eq!(body.bb_height(), height);
    world.tick(Vec::new());
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(body.bb_height(), height * 1.5);
    assert_center_preserved(y, height, body);
    let position = body.position();
    world.tick(Vec::new());
    assert_eq!(
        world.stored_entity_for_test(DIM, id).unwrap().position(),
        position
    );
}

#[test]
fn growth_world_data_merge_slime_size_restores_original_position() {
    let (mut world, sender, receiver) = fixture();
    let id = spawn(&mut world, VANILLA_ENTITY_TYPE_SLIME_ID);
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    let (uuid, position, height) = (body.uuid(), body.position(), body.bb_height());
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        10,
        &format!("data merge entity {uuid} {{Size:1}}"),
    );
    assert_eq!(
        console_command_answer(&tick).0,
        Some(1),
        "{:?}",
        console_command_answer(&tick)
    );
    let body = world.stored_entity_for_test(DIM, id).unwrap();
    assert_eq!(body.bb_height(), height * 2.0);
    assert_eq!(body.position(), position);
}

#[test]
fn growth_world_size_reload_on_powder_snow_restores_position_and_replaces_equipment() {
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
    use bbb_protocol::equipment_slots::EquipmentSlot;
    use bbb_protocol::packets::BlockPos;

    for old_boots in [false, true] {
        let (mut world, sender, receiver) = fixture();
        let id = spawn(&mut world, VANILLA_ENTITY_TYPE_SLIME_ID);
        let powder = vanilla_block_default_state_id(
            vanilla_block_type_for_resource_id("minecraft:powder_snow")
                .unwrap()
                .id,
        )
        .unwrap();
        for x in 7..=9 {
            for z in 7..=9 {
                world.set_raw_block_state_for_test(DIM, BlockPos { x, y: 120, z }, powder);
            }
        }
        let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
        body.set_pos(Vec3d {
            x: 8.5,
            y: 121.0,
            z: 8.5,
        });
        body.living_mut().unwrap().set_item_slot(
            EquipmentSlot::Feet,
            old_boots
                .then(|| crate::entity::item::plain_stack("minecraft:leather_boots", 1).unwrap()),
        );
        let (uuid, position, height) = (body.uuid(), body.position(), body.bb_height());
        let equipment = if old_boots {
            "{feet:{id:\"minecraft:stone\",count:1}}"
        } else {
            "{feet:{id:\"minecraft:leather_boots\",count:1}}"
        };
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            12,
            &format!("data merge entity {uuid} {{Size:1,equipment:{equipment}}}"),
        );
        assert_eq!(
            console_command_answer(&tick).0,
            Some(1),
            "{:?}",
            console_command_answer(&tick)
        );
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        assert_eq!(body.bb_height(), height * 2.0);
        assert_eq!(
            body.living().unwrap().item_by_slot(EquipmentSlot::Feet),
            Some(
                &crate::entity::item::plain_stack(
                    if old_boots {
                        "minecraft:stone"
                    } else {
                        "minecraft:leather_boots"
                    },
                    1
                )
                .unwrap()
            )
        );
        assert_eq!(body.position(), position);
    }
}

#[test]
fn growth_world_slime_and_magma_cube_dirty_scale_keep_position() {
    for ty in [
        VANILLA_ENTITY_TYPE_SLIME_ID,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
    ] {
        let (mut world, _, _) = fixture();
        let id = spawn(&mut world, ty);
        let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
        let (position, height) = (body.position(), body.bb_height());
        body.living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(1.5);
        world.tick(Vec::new());
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        assert_eq!(body.bb_height(), height * 1.5);
        assert_eq!(body.position(), position);
    }
}

#[test]
fn growth_world_effect_clear_consumes_dirty_scale() {
    for single in [false, true] {
        let (mut world, sender, receiver) = fixture();
        let id = spawn(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
        let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            20,
            &format!("effect give {uuid} minecraft:resistance 100"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
        let body = world.stored_entity_mut_for_test(DIM, id).unwrap();
        let (position, height) = (body.position(), body.bb_height());
        body.living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(1.5);
        let suffix = if single { " minecraft:resistance" } else { "" };
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            21,
            &format!("effect clear {uuid}{suffix}"),
        );
        let _ = console_command_answer(&tick);
        let body = world.stored_entity_for_test(DIM, id).unwrap();
        assert_eq!(body.bb_height(), height * 1.5);
        assert_center_preserved(position.y, height, body);
        let NbtSummaryValue::Compound(record) =
            world.stored_entity_command_data_for_test(DIM, id).unwrap()
        else {
            panic!("record");
        };
        assert!(
            !record.iter().any(|field| field.name == "active_effects"),
            "effect was actually removed: {record:?}"
        );
    }
}
