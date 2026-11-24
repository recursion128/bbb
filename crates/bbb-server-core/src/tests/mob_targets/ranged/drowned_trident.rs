use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, VANILLA_MOB_FLAGS_DATA_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::EntityDataValueKind;

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation_id = 1000 + u64::try_from(world.game_time()).unwrap();
    let tick = console_command(sender, receiver, world, operation_id, text);
    assert!(
        matches!(&admin_outcome(&tick).result, Ok(AdminResult::Command(_))),
        "{text}: {:?}",
        admin_outcome(&tick)
    );
    assert_eq!(console_command_answer(&tick).0, Some(1), "{text}");
    tick
}

fn tridents(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(DIMENSION, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_TRIDENT_ID)
        })
        .collect()
}

fn flag_packet(tick: &TickResult, id: i32, data_id: u8, mask: i8, expected: i8) -> bool {
    tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::SetEntityData(update) }
        if update.id == id && update.values.iter().any(|value| value.data_id == data_id
            && matches!(value.value, EntityDataValueKind::Byte(flags) if flags & mask == expected))
    ))
}

#[test]
fn drowned_trident_respects_no_ai_hits_for_eight_and_keeps_the_weapon() {
    let (sender, receiver, mut world) = fixture();
    let (drowned, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_DROWNED_ID, east(2.0));
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.mainhand",
        "minecraft:trident",
    );
    let held = world
        .stored_entity_for_test(DIMENSION, drowned)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::MainHand)
        .unwrap()
        .clone();
    assert_eq!(
        first_added(&mut world, 100, VANILLA_ENTITY_TYPE_TRIDENT_ID),
        None
    );
    assert_eq!(health(&world), 20.0);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    assert_eq!(
        first_added(&mut world, 160, VANILLA_ENTITY_TYPE_TRIDENT_ID),
        Some(drowned)
    );
    let projectile = world
        .stored_entity_for_test(DIMENSION, tridents(&world)[0])
        .unwrap();
    let EntityData::Trident(data) = projectile.data() else {
        panic!("trident");
    };
    assert_eq!(data.projectile().owner(), Some(uuid));
    assert_eq!(data.pickup(), crate::entity::ArrowPickup::Disallowed);
    assert_eq!(data.item(), &held);
    let hits = hits(&mut world, 100);
    assert!(
        hits.first().is_some_and(|(_, lost, _)| *lost == 8.0),
        "{hits:?}"
    );
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, drowned)
            .unwrap()
            .living()
            .unwrap()
            .equipment()
            .get(EquipmentSlot::MainHand),
        Some(&held)
    );
}

#[test]
fn drowned_trident_uses_the_main_hand_components_and_releases_on_switch_or_target_loss() {
    let (sender, receiver, mut world) = fixture();
    let (drowned, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_DROWNED_ID, east(4.0));
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.offhand",
        "minecraft:trident",
    );
    let melee_hits = hits(&mut world, 90);
    assert!(
        !melee_hits.is_empty(),
        "an offhand trident leaves the melee goal available"
    );
    assert!(
        tridents(&world).is_empty(),
        "offhand tridents do not enable ranged attacks"
    );
    command(
        &sender,
        &receiver,
        &mut world,
        "effect give alice minecraft:instant_health 1 5",
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    let equip = [
        format!(
            r#"data merge entity {uuid} {{equipment:{{mainhand:{{id:"minecraft:trident",count:1,components:{{"minecraft:custom_data":{{marker:17}}}}}}}}}}"#
        ),
        format!(
            r#"data merge entity {uuid} {{equipment:{{mainhand:{{components:{{"minecraft:enchantments":{{"minecraft:loyalty":2,"minecraft:flame":1}}}}}}}}}}"#
        ),
        format!(
            r#"data merge entity {uuid} {{equipment:{{mainhand:{{components:{{"minecraft:use_effects":{{interact_vibrations:true}}}}}}}}}}"#
        ),
    ];
    for text in &equip {
        command(&sender, &receiver, &mut world, text);
    }
    let held = world
        .stored_entity_for_test(DIMENSION, drowned)
        .unwrap()
        .living()
        .unwrap()
        .equipment()
        .get(EquipmentSlot::MainHand)
        .unwrap()
        .clone();
    let tick = command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    let mut raised = flag_packet(&tick, drowned, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, 3, 1);
    let mut aggressive = flag_packet(
        &tick,
        drowned,
        VANILLA_MOB_FLAGS_DATA_ID,
        bbb_protocol::entity_synched_data::VANILLA_MOB_FLAG_AGGRESSIVE,
        bbb_protocol::entity_synched_data::VANILLA_MOB_FLAG_AGGRESSIVE,
    );
    for _ in 0..160 {
        let tick = world.tick(Vec::new());
        raised |= flag_packet(&tick, drowned, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, 3, 1);
        aggressive |= flag_packet(
            &tick,
            drowned,
            VANILLA_MOB_FLAGS_DATA_ID,
            bbb_protocol::entity_synched_data::VANILLA_MOB_FLAG_AGGRESSIVE,
            bbb_protocol::entity_synched_data::VANILLA_MOB_FLAG_AGGRESSIVE,
        );
        if !tridents(&world).is_empty() {
            break;
        }
    }
    assert!(
        raised && aggressive,
        "the tracked mob raises its main hand and becomes aggressive"
    );
    let projectile = world
        .stored_entity_for_test(DIMENSION, tridents(&world)[0])
        .unwrap();
    let EntityData::Trident(data) = projectile.data() else {
        panic!("trident");
    };
    assert_eq!(data.item(), &held);
    assert_eq!(data.loyalty(), 2);
    assert!(data.foil());
    assert!(
        projectile.remaining_fire_ticks() > 0,
        "projectile_spawned applies flame"
    );
    assert!(world
        .game_events(DIMENSION)
        .0
        .iter()
        .any(|event| event.game_event
            == crate::level_event::VanillaGameEvent::ITEM_INTERACT_START
            && event.source_entity_id == Some(drowned)));
    command(
        &sender,
        &receiver,
        &mut world,
        "kill @e[type=minecraft:trident]",
    );
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.mainhand",
        "minecraft:air",
    );
    world.tick(Vec::new());
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, drowned)
            .unwrap()
            .living()
            .unwrap()
            .living_entity_flags()
            & 1,
        0,
        "removing the main hand weapon releases use"
    );
    assert!(!hits(&mut world, 60).is_empty(), "the mob returns to melee");
    for text in &equip {
        command(&sender, &receiver, &mut world, text);
    }

    tick_until(&mut world, 20, |world| {
        world
            .stored_entity_for_test(DIMENSION, drowned)
            .unwrap()
            .living()
            .unwrap()
            .living_entity_flags()
            & 1
            != 0
    });
    let tick = command(&sender, &receiver, &mut world, "gamemode spectator alice");
    let mut released = flag_packet(&tick, drowned, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, 1, 0);
    let mut calm = flag_packet(
        &tick,
        drowned,
        VANILLA_MOB_FLAGS_DATA_ID,
        bbb_protocol::entity_synched_data::VANILLA_MOB_FLAG_AGGRESSIVE,
        0,
    );
    for _ in 0..20 {
        let tick = world.tick(Vec::new());
        released |= flag_packet(&tick, drowned, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID, 1, 0);
        calm |= flag_packet(
            &tick,
            drowned,
            VANILLA_MOB_FLAGS_DATA_ID,
            bbb_protocol::entity_synched_data::VANILLA_MOB_FLAG_AGGRESSIVE,
            0,
        );
        if released && calm {
            break;
        }
    }
    assert!(
        released && calm,
        "target loss clears the visible using and aggressive flags"
    );
    assert!(world
        .game_events(DIMENSION)
        .0
        .iter()
        .any(|event| event.game_event
            == crate::level_event::VanillaGameEvent::ITEM_INTERACT_FINISH
            && event.source_entity_id == Some(drowned)));
}
