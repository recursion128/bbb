//! A natural iron-golem flower offer equips and drops the copper recipient's gift.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation = world.game_time() as u64 + 10000;
    let output = console_command(sender, receiver, world, operation, text);
    let outcome = admin_outcome(&output);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {report:?}");
    keep_alive(sender, &output);
    output
}
fn keep_alive(sender: &WorldCommandSender, output: &TickResult) {
    for event in &output.events {
        if let WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::KeepAlive { id },
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: ALICE,
                    packet: PlayServerbound::KeepAlive { id: *id },
                },
            )
            .unwrap();
        }
    }
}
fn tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    let output = world.tick(receiver.drain());
    keep_alive(sender, &output);
    output
}
fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}
fn looks_at(body: &crate::entity::Entity, target: &crate::entity::Entity) -> bool {
    let from = body.position();
    let to = target.position();
    let yaw = (to.z - from.z).atan2(to.x - from.x).to_degrees() as f32 - 90.0;
    let error = (body.y_head_rot() - yaw + 180.0).rem_euclid(360.0) - 180.0;
    let pitch = (from.y + f64::from(body.eye_height()) - to.y - f64::from(target.eye_height()))
        .atan2(separation(from, to))
        .to_degrees() as f32;
    error.abs() < 5.0 && (body.x_rot() - pitch).abs() < 3.0
}
fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}

fn poppy_equipped(world: &ServerWorld, copper: i32) -> bool {
    world
        .stored_entity_for_test(DIMENSION, copper)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::Saddle)
        .is_some_and(|stack| stack.item_resource_id() == Some("minecraft:poppy"))
}
fn poppy_packet(output: &TickResult, copper: i32) -> bool {
    output.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::SetEquipment(update), .. }
        if update.entity_id == copper && update.slots.iter().any(|slot|
            slot.slot == EquipmentSlot::Saddle && slot.item.count == 1
            && slot.item.item_id.and_then(bbb_protocol::item_types::vanilla_item_type)
                .is_some_and(|item| item.resource_id == "minecraft:poppy"))))
}
fn dropped_poppy(world: &ServerWorld) -> bool {
    world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .any(|id| {
            world
                .stored_entity_for_test(DIMENSION, id)
                .is_some_and(|entity| {
                    let crate::entity::EntityData::Item(data) = entity.data() else {
                        return false;
                    };
                    data.item().item_resource_id() == Some("minecraft:poppy")
                })
        })
}

#[test]
fn a_natural_flower_offer_gives_a_copper_golem_a_guaranteed_poppy_drop() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "gamemode spectator alice",
        "forceload add -32 -16 48 32",
        "fill -32 64 -16 48 64 32 minecraft:stone",
        "gamerule advance_time false",
        "time set noon",
        "weather clear",
        "fill 3 65 3 3 68 13 minecraft:stone",
        "fill 13 65 3 13 68 13 minecraft:stone",
        "fill 4 65 3 12 68 3 minecraft:stone",
        "fill 4 65 13 12 68 13 minecraft:stone",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let (copper, copper_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {copper_uuid} {{NoAI:1b,next_weather_age:-2L}}"),
    );
    assert!(world
        .stored_entity_for_test(DIMENSION, copper)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::Saddle)
        .is_none());
    let mut golems = Vec::new();
    for (x, z) in [(5.5, 8.5), (11.5, 8.5), (8.5, 5.5), (8.5, 11.5)] {
        let (id, _) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            Vec3d { x, y: 65.0, z },
        );
        golems.push(id);
    }
    let mut offering = [false; 4];
    let mut looked = [false; 4];
    let mut completed = false;
    let mut equipped_packet = false;
    let mut gifted = false;
    let mut first_offer = None;
    world.tick_rate_mut().set_frozen(false);
    for elapsed in 0..64650 {
        if elapsed >= 64000 && !offering.iter().any(|active| *active) {
            break;
        }
        let output = tick(&sender, &receiver, &mut world);
        equipped_packet |= poppy_packet(&output, copper);
        for event in &output.events {
            if let WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::EntityEvent(update),
                ..
            } = event
            {
                if let Some(index) = golems.iter().position(|id| *id == update.entity_id) {
                    if update.event_id == 11 {
                        offering[index] = true;
                        looked[index] = false;
                        first_offer.get_or_insert(elapsed);
                    } else if update.event_id == 34 {
                        completed |= offering[index] && looked[index];
                        offering[index] = false;
                    }
                }
            }
        }
        let recipient = world.stored_entity_for_test(DIMENSION, copper).unwrap();
        for (index, id) in golems.iter().enumerate() {
            if offering[index] {
                let body = world.stored_entity_for_test(DIMENSION, *id).unwrap();
                looked[index] |= looks_at(body, recipient);
            }
        }
        if completed && equipped_packet && poppy_equipped(&world, copper) {
            gifted = true;
            break;
        }
    }
    assert!(gifted,
        "a positive natural offer completes with gaze and a real equipment update: first_offer={first_offer:?}, offering={offering:?}, looked={looked:?}, completed={completed}, equipment_packet={equipped_packet}");
    let record = world
        .stored_entity_command_data_for_test(DIMENSION, copper)
        .unwrap();
    let antenna = field(
        field(&record, "equipment").expect("saved equipment"),
        "saddle",
    )
    .expect("saved antenna poppy");
    assert_eq!(
        field(antenna, "id"),
        Some(&NbtSummaryValue::String("minecraft:poppy".into()))
    );
    assert_eq!(
        field(
            field(&record, "drop_chances").expect("saved drop chances"),
            "saddle"
        ),
        Some(&NbtSummaryValue::Float(2.0_f32.to_bits()))
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("kill {copper_uuid}"),
    );
    let mut dropped = dropped_poppy(&world);
    for _ in 0..60 {
        if dropped {
            break;
        }
        tick(&sender, &receiver, &mut world);
        dropped |= dropped_poppy(&world);
    }
    assert!(
        dropped,
        "the gifted poppy actually drops when its copper recipient dies"
    );
}
