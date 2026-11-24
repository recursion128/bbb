//! Saddled pigs and striders steered on a stick, a donkey's chest, and the
//! distance a rider's vehicle carries them (`Pig.java:157-226`,
//! `FoodOnAStickItem.java:24-39`, `AbstractChestedHorse.java:143-174`,
//! `ServerPlayer.java:1470-1490`).

use super::feeding::interact_entity_packet;
use super::horse_interaction::{
    hold, horse_world, item_id, mount_screen, played, record, tamed_equine, worn,
};
use super::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{MoveVehicleCommand, UseItem};
use bbb_server_types::stats::custom_stat;

const PIG: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
const STRIDER: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID;
const DONKEY: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID;

pub(super) fn spawn_beside(world: &mut ServerWorld, entity_type: i32) -> i32 {
    let position = world.publication().players[0].position;
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            entity_type,
            Vec3d {
                x: position.x + 1.5,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(entity_type).unwrap()),
        )
        .0
}

/// Clicks `mount` and acknowledges the rider teleport the mount sends.
pub(super) fn mount(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    mount: i32,
) {
    let mounted = play(sender, receiver, world, 1, interact_entity_packet(mount));
    let rider = world.player_entity_id_for_test(ConnectionId(1));
    assert_eq!(
        world
            .external_player_relation_in_dimension_for_test(
                Java26BuiltinDimension::Overworld,
                rider
            )
            .0,
        Some(mount),
        "the click mounted the player"
    );
    let teleport = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .expect("mounting places the rider");
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport },
    );
}

/// The rider's client moves its vehicle a quarter block east; answers where
/// the vehicle ended up.
pub(super) fn drive(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    vehicle: i32,
) -> (Vec3d, Vec3d) {
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    let target = Vec3d {
        x: baseline.x + 0.25,
        ..baseline
    };
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: target,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        }),
    );
    let position = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, vehicle)
        .expect("the vehicle is live")
        .position();
    (target, position)
}

fn use_held_item(y_rot: f32) -> PlayServerbound {
    PlayServerbound::UseItem(UseItem {
        hand: InteractionHand::MainHand,
        sequence: 1,
        y_rot,
        x_rot: 0.0,
    })
}

fn ridden_cm(world: &ServerWorld, stat: &str) -> i32 {
    world.player_stat_for_test(ConnectionId(1), custom_stat(stat))
}

fn held(world: &ServerWorld) -> ItemStackSummary {
    world.player_inventory_slot_for_test(ConnectionId(1), 0)
}

fn boost_time(world: &ServerWorld, entity: i32) -> i32 {
    match world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity)
        .expect("the steered mob is live")
        .data()
    {
        crate::entity::EntityData::GeneralLiving(data) => data
            .pig_steering()
            .or(data.strider_state().map(|strider| strider.steering))
            .expect("a steered mob")
            .boost_time_total(),
        _ => panic!("a steered mob is a living entity"),
    }
}

#[test]
fn a_saddled_pig_carries_a_rider_who_steers_and_boosts_it_on_a_carrot_on_a_stick() {
    let (sender, receiver, mut world) = horse_world();
    let pig = spawn_beside(&mut world, PIG);
    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(pig),
    );
    assert_eq!(
        worn(&world, pig, EquipmentSlot::Saddle),
        Some(item_id("minecraft:saddle"))
    );
    assert_eq!(held(&world), ItemStackSummary::empty());

    hold(&mut world, "minecraft:carrot_on_a_stick", 1);
    mount(&sender, &receiver, &mut world, pig);

    // The boost writes `DATA_BOOST_TIME` and costs the stick seven damage; a
    // second click while the boost runs changes nothing.
    play(&sender, &receiver, &mut world, 1, use_held_item(90.0));
    let boost = boost_time(&world, pig);
    assert!((140..=980).contains(&boost), "boost time {boost}");
    assert_eq!(held(&world).component_patch.damage, Some(7));
    play(&sender, &receiver, &mut world, 1, use_held_item(90.0));
    assert_eq!(boost_time(&world, pig), boost);
    assert_eq!(held(&world).component_patch.damage, Some(7));

    // `tickRidden` turns the pig to the rider's view.
    let pig_y_rot = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, pig)
        .map(crate::entity::Entity::y_rot);
    assert_eq!(pig_y_rot, Some(90.0));

    let (target, position) = drive(&sender, &receiver, &mut world, pig);
    assert_eq!(position, target, "the rider's client moved the pig");
    assert_eq!(ridden_cm(&world, "minecraft:pig_one_cm"), 25);
}

#[test]
fn a_worn_out_warped_fungus_on_a_stick_boosts_the_strider_and_turns_back_into_a_fishing_rod() {
    let (sender, receiver, mut world) = horse_world();
    let strider = spawn_beside(&mut world, STRIDER);
    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(strider),
    );
    let mut stick = super::feeding::stack_of("minecraft:warped_fungus_on_a_stick", 1);
    crate::item_durability::repair_to(&mut stick, 99);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stick);
    mount(&sender, &receiver, &mut world, strider);
    let (target, position) = drive(&sender, &receiver, &mut world, strider);
    assert_eq!(position, target);
    assert_eq!(ridden_cm(&world, "minecraft:strider_one_cm"), 25);

    play(&sender, &receiver, &mut world, 1, use_held_item(0.0));
    assert!(boost_time(&world, strider) >= 140);
    let rod = held(&world);
    assert_eq!(rod.item_id, Some(item_id("minecraft:fishing_rod")));
    assert_eq!(rod.count, 1);
    assert_eq!(rod.component_patch.damage.unwrap_or(0), 0);

    // Holding the rod, the rider no longer steers: the client's move is ignored.
    let (target, position) = drive(&sender, &receiver, &mut world, strider);
    assert_ne!(position, target);
}

#[test]
fn a_tamed_donkey_takes_a_chest_shows_its_cargo_and_carries_its_rider_as_a_horse() {
    let (sender, receiver, mut world) = horse_world();
    let donkey = tamed_equine(&mut world, DONKEY, 1.5);
    hold(&mut world, "minecraft:chest", 2);
    let chested = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(donkey),
    );
    assert!(played(&chested.events, "minecraft:entity.donkey.chest"));
    assert_eq!(held(&world), super::feeding::stack_of("minecraft:chest", 1));
    assert!(
        record(&world, donkey).contains(&bbb_protocol::NbtSummaryEntry {
            name: "ChestedHorse".into(),
            value: bbb_protocol::NbtSummaryValue::Byte(1),
        })
    );

    let opened = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: donkey,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: true,
        }),
    );
    let screen = mount_screen(&opened.events).expect("the donkey's inventory opened");
    assert_eq!(screen.inventory_columns, 5);
    let slots = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ContainerSetContent(content),
                ..
            } if content.container_id == screen.container_id => Some(content.items.len()),
            _ => None,
        })
        .expect("the menu sent its contents");
    // Saddle, body, fifteen cargo slots and the 36 player slots.
    assert_eq!(slots, 53);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(bbb_protocol::packets::ContainerCloseRequest {
            container_id: screen.container_id,
        }),
    );

    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(donkey),
    );
    mount(&sender, &receiver, &mut world, donkey);
    let (target, position) = drive(&sender, &receiver, &mut world, donkey);
    assert_eq!(position, target);
    assert_eq!(ridden_cm(&world, "minecraft:horse_one_cm"), 25);
}
