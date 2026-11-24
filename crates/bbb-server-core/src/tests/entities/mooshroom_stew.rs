use super::feeding::{interact_entity_packet, stack_of};
use super::horse_interaction::{hold, item_id, record};
use super::*;
use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Nbt};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn spawn(world: &mut ServerWorld, fields: Vec<Entry>) -> i32 {
    let position = world.publication().players[0].position;
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID;
    let id = world
        .spawn_stored_entity_for_test(
            DIM,
            kind,
            Vec3d {
                z: position.z + 1.0,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
        )
        .0;
    let mut entries = record(world, id);
    for field in fields.into_iter().chain([Entry {
        name: "NoAI".into(),
        value: Nbt::Byte(1),
    }]) {
        entries.retain(|entry| entry.name != field.name);
        entries.push(field);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    id
}

fn effects(id: &str, duration: Option<i32>) -> Nbt {
    let mut fields = vec![Entry {
        name: "id".into(),
        value: Nbt::String(id.into()),
    }];
    if let Some(duration) = duration {
        fields.push(Entry {
            name: "duration".into(),
            value: Nbt::Int(duration),
        });
    }
    Nbt::Compound(fields)
}

fn sound(tick: &TickResult, name: &str, volume: f32) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.source == SoundSource::Neutral && sound.volume == volume && sound.pitch == 1.0
        && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
            if vanilla_sound_event_resource_id(registry_id) == Some(name)))
    })
}

#[test]
fn live_mooshroom_stew_reads_all_saved_effects_and_consumes_bowls() {
    let (sender, receiver, mut world) = super::horse_interaction::horse_world();
    let id = spawn(
        &mut world,
        vec![Entry {
            name: "stew_effects".into(),
            value: Nbt::List(vec![
                effects("minecraft:poison", Some(7)),
                effects("minecraft:blindness", None),
            ]),
        }],
    );
    hold(&mut world, "minecraft:bowl", 1);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    let soup = &tick.snapshot.players[0].inventory.slots[0];
    assert_eq!(soup.item_id, Some(item_id("minecraft:suspicious_stew")));
    assert_eq!(
        soup.component_patch
            .suspicious_stew_effects
            .iter()
            .map(|effect| (effect.effect_id, effect.duration))
            .collect::<Vec<_>>(),
        vec![
            (
                bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_POISON_ID,
                7
            ),
            (
                bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_BLINDNESS_ID,
                160
            )
        ]
    );
    assert!(soup.component_patch.added_type_ids.contains(
        &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID
    ));
    assert!(!record(&world, id)
        .iter()
        .any(|field| field.name == "stew_effects"));
    assert!(sound(
        &tick,
        "minecraft:entity.mooshroom.suspicious_milk",
        1.0
    ));
    assert!(!tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityAnimation(_),
            ..
        }
    )));
    assert!(world
        .game_events(DIM)
        .0
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::ENTITY_INTERACT));
    hold(&mut world, "minecraft:bowl", 3);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(tick.snapshot.players[0].inventory.slots[0].count, 2);
    assert!(tick.snapshot.players[0]
        .inventory
        .slots
        .iter()
        .any(|stack| stack.item_id == Some(item_id("minecraft:mushroom_stew"))));
    assert!(sound(&tick, "minecraft:entity.mooshroom.milk", 1.0));
}

#[test]
fn live_mooshroom_flowers_keep_first_effect_through_save_reload() {
    let (sender, receiver, mut world) = super::horse_interaction::horse_world();
    let id = spawn(
        &mut world,
        vec![Entry {
            name: "Type".into(),
            value: Nbt::String("brown".into()),
        }],
    );
    hold(&mut world, "minecraft:allium", 3);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(tick.snapshot.players[0].inventory.slots[0].count, 2);
    assert!(sound(&tick, "minecraft:entity.mooshroom.eat", 2.0));
    hold(&mut world, "minecraft:dandelion", 3);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(tick.snapshot.players[0].inventory.slots[0].count, 3);
    assert!(!sound(&tick, "minecraft:entity.mooshroom.eat", 2.0));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(10, None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let id = world.live_entity_id_for_uuid_for_test(DIM, uuid).unwrap();
    hold(&mut world, "minecraft:bowl", 1);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    let soup = tick.snapshot.players[0]
        .inventory
        .slots
        .iter()
        .find(|stack| stack.item_id == Some(item_id("minecraft:suspicious_stew")))
        .unwrap();
    let expected =
        bbb_protocol::item_types::vanilla_suspicious_stew_ingredient(item_id("minecraft:allium"))
            .unwrap();
    assert_eq!(
        soup.component_patch.suspicious_stew_effects,
        vec![bbb_protocol::packets::SuspiciousStewEffectSummary {
            effect_id: expected.effect_type_id,
            duration: expected.duration_ticks
        }]
    );
}

#[test]
fn live_mooshroom_creative_repeats_stew_and_discards_when_full() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let id = spawn(
        &mut world,
        vec![Entry {
            name: "Type".into(),
            value: Nbt::String("brown".into()),
        }],
    );
    hold(&mut world, "minecraft:allium", 2);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(tick.snapshot.players[0].inventory.slots[0].count, 2);
    hold(&mut world, "minecraft:bowl", 3);
    for _ in 0..3 {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
    }
    let snapshot = world.publication();
    assert_eq!(snapshot.players[0].inventory.slots[0].count, 3);
    assert_eq!(
        snapshot.players[0]
            .inventory
            .slots
            .iter()
            .filter(|stack| stack.item_id == Some(item_id("minecraft:mushroom_stew")))
            .count(),
        2
    );
    for slot in 1..36 {
        world.set_inventory_slot_for_test(ConnectionId(1), slot, stack_of("minecraft:stone", 64));
    }
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert!(!world.stored_entity_ids_for_test(DIM).iter().any(|id| matches!(world.stored_entity_for_test(DIM, *id).unwrap().data(), crate::entity::EntityData::Item(item) if item.item().item_id() == Some(item_id("minecraft:mushroom_stew")))));
}

#[test]
fn live_mooshroom_empty_effects_survival_full_drop_and_age_health_gates() {
    let (sender, receiver, mut world) = super::horse_interaction::horse_world();
    let id = spawn(
        &mut world,
        vec![
            Entry {
                name: "Silent".into(),
                value: Nbt::Byte(1),
            },
            Entry {
                name: "stew_effects".into(),
                value: Nbt::List(Vec::new()),
            },
        ],
    );
    hold(&mut world, "minecraft:bowl", 3);
    for slot in 1..36 {
        world.set_inventory_slot_for_test(ConnectionId(1), slot, stack_of("minecraft:stone", 64));
    }
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(tick.snapshot.players[0].inventory.slots[0].count, 2);
    assert!(!sound(
        &tick,
        "minecraft:entity.mooshroom.suspicious_milk",
        1.0
    ));
    assert!(!record(&world, id)
        .iter()
        .any(|field| field.name == "stew_effects"));
    assert!(world.stored_entity_ids_for_test(DIM).iter().any(|id| matches!(world.stored_entity_for_test(DIM, *id).unwrap().data(), crate::entity::EntityData::Item(item) if item.item().item_id() == Some(item_id("minecraft:suspicious_stew")) && item.item().summary().component_patch.added_type_ids.contains(&bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID))));
    for fields in [
        vec![Entry {
            name: "Age".into(),
            value: Nbt::Int(-100),
        }],
        vec![Entry {
            name: "Health".into(),
            value: Nbt::Float(0.0_f32.to_bits()),
        }],
    ] {
        let target = spawn(&mut world, fields);
        hold(&mut world, "minecraft:bowl", 3);
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(target),
        );
        assert_eq!(
            tick.snapshot.players[0].inventory.slots[0],
            stack_of("minecraft:bowl", 3)
        );
        assert!(!sound(&tick, "minecraft:entity.mooshroom.milk", 1.0));
    }
}
