//! A real trader drinks its night potion and daytime milk, then looks at a nearby player.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
    VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID;
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

fn flags_packet(output: &TickResult, id: i32, data_id: u8, mask: i8, set: bool) -> bool {
    output.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet:ClientboundPlayPacket::SetEntityData(update), .. }
        if update.id == id && update.values.iter().any(|value|
            value.data_id == data_id && matches!(value.value,
                EntityDataValueKind::Byte(flags) if (flags & mask != 0) == set))))
}
fn hand_packet(output: &TickResult, id: i32, item: Option<&str>) -> bool {
    output.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet:ClientboundPlayPacket::SetEquipment(update), .. }
        if update.entity_id == id && update.slots.iter().any(|slot|
            slot.slot == EquipmentSlot::MainHand
            && slot.item.item_id.and_then(bbb_protocol::item_types::vanilla_item_type)
                .map(|item| item.resource_id) == item)))
}
fn sound_packet(output: &TickResult, name: &str) -> bool {
    output.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet:ClientboundPlayPacket::Sound(sound), .. }
        if matches!(sound.sound, SoundEventHolder::Reference { registry_id }
            if vanilla_sound_event_resource_id(registry_id) == Some(name)))
    })
}
fn drink(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    trader: i32,
    item: &str,
    sound: &str,
    invisible: bool,
) {
    let mut actual_use = false;
    let mut equipped = false;
    let mut using_metadata = false;
    let mut visibility_metadata = false;
    let mut empty_packet = false;
    let mut heard = false;
    for _ in 0..240 {
        let output = tick(sender, receiver, world);
        equipped |= hand_packet(&output, trader, Some(item));
        using_metadata |= flags_packet(
            &output,
            trader,
            VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
            1,
            true,
        );
        visibility_metadata |= flags_packet(
            &output,
            trader,
            VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
            VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
            invisible,
        );
        empty_packet |= equipped && hand_packet(&output, trader, None);
        heard |= sound_packet(&output, sound);
        let body = world.stored_entity_for_test(DIMENSION, trader).unwrap();
        let living = body.living().unwrap();
        let hand = living.equipment().get(EquipmentSlot::MainHand);
        actual_use |= hand.is_some_and(|stack| stack.item_resource_id() == Some(item))
            && living.living_entity_flags() & 1 != 0;
        let effects_finished = if invisible {
            living.active_effect("minecraft:invisibility").is_some() && body.is_invisible()
        } else {
            living.active_effects().next().is_none() && !body.is_invisible()
        };
        if actual_use
            && equipped
            && using_metadata
            && visibility_metadata
            && empty_packet
            && heard
            && effects_finished
            && hand.is_none()
            && living.living_entity_flags() & 1 == 0
        {
            return;
        }
    }
    panic!("real {item} use did not finish: actual_use={actual_use}, equipped={equipped}, using_metadata={using_metadata}, visibility_metadata={visibility_metadata}, empty_packet={empty_packet}, heard={heard}");
}

#[test]
fn a_trader_drinks_a_night_potion_and_daytime_milk_then_watches_a_nontrading_player() {
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    for text in [
        "forceload add -32 -16 48 32",
        "fill -32 64 -16 48 64 32 minecraft:stone",
        "fill 3 65 3 3 68 12 minecraft:stone",
        "fill 12 65 3 12 68 12 minecraft:stone",
        "fill 4 65 3 11 68 3 minecraft:stone",
        "fill 4 65 12 11 68 12 minecraft:stone",
        "gamerule advance_time false",
        "time set midnight",
        "weather clear",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 6.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let (trader, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
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
        &format!("data merge entity {uuid} {{DespawnDelay:48000}}"),
    );
    assert!(!world
        .stored_entity_for_test(DIMENSION, trader)
        .unwrap()
        .is_invisible());
    world.tick_rate_mut().set_frozen(false);
    drink(
        &sender,
        &receiver,
        &mut world,
        trader,
        "minecraft:potion",
        "minecraft:entity.wandering_trader.disappeared",
        true,
    );
    world.tick_rate_mut().set_frozen(true);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("effect give {uuid} minecraft:haste 120 0"),
    );
    assert!(world
        .stored_entity_for_test(DIMENSION, trader)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:haste")
        .is_some());
    command(&sender, &receiver, &mut world, "time set noon");
    world.tick_rate_mut().set_frozen(false);
    drink(
        &sender,
        &receiver,
        &mut world,
        trader,
        "minecraft:milk_bucket",
        "minecraft:entity.wandering_trader.reappeared",
        false,
    );
    let mut looking = 0;
    for _ in 0..240 {
        let at = world
            .stored_entity_for_test(DIMENSION, trader)
            .unwrap()
            .position();
        world.set_player_position_for_test(
            ALICE,
            Vec3d {
                x: at.x + if at.x <= 8.0 { 2.0 } else { -2.0 },
                ..at
            },
        );
        tick(&sender, &receiver, &mut world);
        let body = world.stored_entity_for_test(DIMENSION, trader).unwrap();
        assert!(body.merchant_trading_player().is_none());
        let player = world.publication().players[0].position;
        let at = body.position();
        let yaw = (player.z - at.z).atan2(player.x - at.x).to_degrees() as f32 - 90.0;
        let yaw_error = (body.y_head_rot() - yaw + 180.0).rem_euclid(360.0) - 180.0;
        let pitch = (at.y + f64::from(body.eye_height()) - player.y - 1.62)
            .atan2(separation(at, player))
            .to_degrees() as f32;
        if separation(at, player) < 3.0
            && yaw_error.abs() < 5.0
            && (body.x_rot() - pitch).abs() < 3.0
        {
            looking += 1;
        }
        if looking >= 4 {
            return;
        }
    }
    panic!("the nontrading trader did not visibly look at the nearby real player");
}
