use super::feeding::stack_of;
use super::horse_interaction::{hold, item_id, record};
use super::*;
use bbb_protocol::data_component_types::*;
use bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id;
use bbb_protocol::packets::UseItem;
use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Nbt};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const STANDING: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn field(name: &str, value: Nbt) -> Entry {
    Entry {
        name: name.into(),
        value,
    }
}

fn spawn(world: &mut ServerWorld, name: &str, fields: Vec<Entry>) -> i32 {
    let kind = vanilla_entity_type_id_for_resource_id(name).unwrap();
    let id = world
        .spawn_stored_entity_for_test(
            DIM,
            kind,
            Vec3d { z: 1.5, ..STANDING },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
        )
        .0;
    let mut entries = record(world, id);
    for field in fields.into_iter().chain([
        field("NoAI", Nbt::Byte(1)),
        field("NoGravity", Nbt::Byte(1)),
    ]) {
        entries.retain(|entry| entry.name != field.name);
        entries.push(field);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    id
}

fn interact(id: i32, hand: InteractionHand) -> PlayServerbound {
    PlayServerbound::Interact(InteractEntity {
        entity_id: id,
        hand,
        location: Vec3d::default(),
        using_secondary_action: false,
    })
}

fn sound(tick: &TickResult, name: &str) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.source == SoundSource::Neutral && sound.volume == 1.0 && sound.pitch == 1.0
        && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
            if vanilla_sound_event_resource_id(registry_id) == Some(name)))
    })
}

fn hunting_cooldown(time_to_live: i64) -> Nbt {
    use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue, PackedMemory};
    BrainPacked {
        memories: vec![PackedMemory {
            memory_type: MemoryModuleType::HasHuntingCooldown,
            value: MemoryValue::Bool(true),
            time_to_live: Some(time_to_live),
        }],
    }
    .encode()
}

#[test]
fn six_living_bucketable_mobs_capture_and_release_their_components_on_the_packet_path() {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;
    for (index, (name, bucket_name, pickup_sound)) in [
        (
            "minecraft:cod",
            "minecraft:cod_bucket",
            "minecraft:item.bucket.fill_fish",
        ),
        (
            "minecraft:salmon",
            "minecraft:salmon_bucket",
            "minecraft:item.bucket.fill_fish",
        ),
        (
            "minecraft:pufferfish",
            "minecraft:pufferfish_bucket",
            "minecraft:item.bucket.fill_fish",
        ),
        (
            "minecraft:tropical_fish",
            "minecraft:tropical_fish_bucket",
            "minecraft:item.bucket.fill_fish",
        ),
        (
            "minecraft:tadpole",
            "minecraft:tadpole_bucket",
            "minecraft:item.bucket.fill_tadpole",
        ),
        (
            "minecraft:axolotl",
            "minecraft:axolotl_bucket",
            "minecraft:item.bucket.fill_axolotl",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Survival;
        let mut startup = (*config.datapacks.take().unwrap()).clone();
        startup.advancements.advancements.push(AdvancementElement {
            id: "test:capture".into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&serde_json::json!({
                "criteria": {"fill": {"trigger": "minecraft:filled_bucket",
                    "conditions": {"item": {"items": bucket_name}}}}
            }))
            .unwrap(),
        });
        config.datapacks = Some(Arc::new(startup));
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world.set_player_position_for_test(ConnectionId(1), STANDING);
        let custom_name = Nbt::Compound(vec![
            field("text", Nbt::String("Bucket friend".into())),
            field("bold", Nbt::Byte(1)),
        ]);
        let mut retained = vec![
            field("Health", Nbt::Float(2.5_f32.to_bits())),
            field("Glowing", Nbt::Byte(1)),
            field("Invulnerable", Nbt::Byte(1)),
        ];
        let mut extra = Vec::new();
        match name {
            "minecraft:salmon" => extra.push(field("type", Nbt::String("large".into()))),
            "minecraft:pufferfish" => extra.push(field("PuffState", Nbt::Int(2))),
            "minecraft:tropical_fish" => extra.push(field("Variant", Nbt::Int(0x0e0b_0501))),
            "minecraft:axolotl" => {
                extra.extend([
                    field("Variant", Nbt::Int(4)),
                    field("Brain", hunting_cooldown(1234)),
                ]);
                retained.extend([
                    field("Age", Nbt::Int(-1200)),
                    field("AgeLocked", Nbt::Byte(1)),
                ]);
            }
            "minecraft:tadpole" => retained.extend([
                field("Age", Nbt::Int(321)),
                field("AgeLocked", Nbt::Byte(1)),
            ]),
            _ => {}
        }
        let mut source = retained.clone();
        source.extend(extra.clone());
        source.extend([
            field("CustomName", custom_name.clone()),
            field("FromBucket", Nbt::Byte(1)),
        ]);
        let id = spawn(&mut world, name, source);
        let position = world.stored_entity_for_test(DIM, id).unwrap().position();
        let (hand, slot) = if index % 2 == 0 {
            (InteractionHand::MainHand, 0)
        } else {
            (InteractionHand::OffHand, 40)
        };
        world.set_inventory_slot_for_test(
            ConnectionId(1),
            slot,
            stack_of("minecraft:water_bucket", 1),
        );
        let tick = play(&sender, &receiver, &mut world, 1, interact(id, hand));
        let bucket = tick.snapshot.players[0].inventory.slots[slot].clone();
        assert_eq!(bucket.item_id, Some(item_id(bucket_name)), "{name}");
        assert_eq!(bucket.count, 1);
        assert!(world.stored_entity_for_test(DIM, id).is_none());
        assert!(sound(&tick, pickup_sound), "{name}");
        assert!(world.player_advancement_is_done(ConnectionId(1), "test:capture"));
        assert_eq!(
            world.player_stat_for_test(
                ConnectionId(1),
                bbb_server_types::stats::StatKey::new(
                    bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
                    item_id("minecraft:water_bucket"),
                ),
            ),
            0,
        );
        assert!(!tick.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityAnimation(_),
                ..
            }
        )));
        assert!(world
            .positioned_game_events_for_test(DIM)
            .iter()
            .any(
                |event| event.recorded.game_event == VanillaGameEvent::ENTITY_INTERACT
                    && event.recorded.source_entity_id
                        == Some(world.player_entity_id_for_test(ConnectionId(1)))
                    && event.position == position
            ));
        let patch = &bucket.component_patch;
        assert_eq!(patch.custom_name_nbt, Some(custom_name.clone()));
        assert!(patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID));
        assert!(patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID));
        retained.extend([
            field("NoAI", Nbt::Byte(1)),
            field("NoGravity", Nbt::Byte(1)),
        ]);
        let mut bucket_data = retained.clone();
        if name == "minecraft:axolotl" {
            bucket_data.push(field("HuntingCooldown", Nbt::Long(1234)));
        }
        let fields = |entries: &[Entry]| {
            entries
                .iter()
                .map(|entry| (entry.name.clone(), entry.value.clone()))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let Some(Nbt::Compound(actual)) = patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID)
        else {
            panic!("bucket data")
        };
        assert_eq!(fields(actual), fields(&bucket_data), "{name}");
        match name {
            "minecraft:salmon" => assert_eq!(
                patch
                    .builtin_enum_component_ids
                    .get(&VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID),
                Some(&bbb_protocol::builtin_enum_component_values::VANILLA_SALMON_VARIANT_LARGE_ID)
            ),
            "minecraft:tropical_fish" => assert_eq!(
                (
                    patch.tropical_fish_pattern_id,
                    patch.tropical_fish_base_color_id,
                    patch.tropical_fish_pattern_color_id
                ),
                (Some(0x0501), Some(11), Some(14))
            ),
            "minecraft:axolotl" => assert_eq!(
                patch
                    .builtin_enum_component_ids
                    .get(&VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID),
                Some(&4)
            ),
            _ => {}
        }
        world.set_block(
            DIM,
            BlockPos { x: 0, y: 65, z: 2 },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL,
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::UseItem(UseItem {
                hand,
                sequence: 2,
                y_rot: 0.0,
                x_rot: 0.0,
            }),
        );
        let released = world
            .stored_entity_ids_for_test(DIM)
            .into_iter()
            .find(|id| {
                world
                    .stored_entity_for_test(DIM, *id)
                    .unwrap()
                    .entity_type_id()
                    == vanilla_entity_type_id_for_resource_id(name).unwrap()
            })
            .unwrap();
        let saved = fields(&record(&world, released));
        for entry in retained.into_iter().chain([
            field("FromBucket", Nbt::Byte(1)),
            field("CustomName", custom_name),
        ]) {
            assert_eq!(
                saved.get(&entry.name),
                Some(&entry.value),
                "{name}: {:?}",
                entry.name
            );
        }
        for entry in extra {
            if entry.name != "PuffState" {
                assert_eq!(
                    saved.get(&entry.name),
                    Some(&entry.value),
                    "{name}: {:?}",
                    entry.name
                );
            }
        }
    }
}

#[test]
fn creative_capture_repeats_existing_bucket_and_discards_the_result_when_inventory_is_full() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    hold(&mut world, "minecraft:water_bucket", 1);
    for _ in 0..2 {
        let id = spawn(&mut world, "minecraft:cod", Vec::new());
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact(id, InteractionHand::MainHand),
        );
        assert!(world.stored_entity_for_test(DIM, id).is_none());
    }
    let snapshot = world.publication();
    assert_eq!(
        snapshot.players[0].inventory.slots[0].item_id,
        Some(item_id("minecraft:water_bucket"))
    );
    let buckets: Vec<_> = snapshot.players[0]
        .inventory
        .slots
        .iter()
        .filter(|stack| stack.item_id == Some(item_id("minecraft:cod_bucket")))
        .collect();
    assert_eq!(buckets.len(), 2);
    assert_eq!(buckets[0], buckets[1]);
    for slot in 1..36 {
        world.set_inventory_slot_for_test(ConnectionId(1), slot, stack_of("minecraft:stone", 64));
    }
    let id = spawn(&mut world, "minecraft:cod", Vec::new());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(id, InteractionHand::MainHand),
    );
    assert!(world.stored_entity_for_test(DIM, id).is_none());
    assert!(!world
        .stored_entity_ids_for_test(DIM)
        .iter()
        .any(|id| matches!(
            world.stored_entity_for_test(DIM, *id).unwrap().data(),
            crate::entity::EntityData::Item(_)
        )));
}

#[test]
fn capture_requires_living_health_and_a_water_bucket_and_respects_silent() {
    let (sender, receiver, mut world) = super::horse_interaction::horse_world();
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    let dead = spawn(
        &mut world,
        "minecraft:cod",
        vec![field("Health", Nbt::Float(0.0_f32.to_bits()))],
    );
    hold(&mut world, "minecraft:water_bucket", 1);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(dead, InteractionHand::MainHand),
    );
    assert_eq!(
        tick.snapshot.players[0].inventory.slots[0].item_id,
        Some(item_id("minecraft:water_bucket"))
    );
    assert!(!sound(&tick, "minecraft:item.bucket.fill_fish"));
    let silent = spawn(
        &mut world,
        "minecraft:cod",
        vec![field("Silent", Nbt::Byte(1))],
    );
    hold(&mut world, "minecraft:bucket", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(silent, InteractionHand::MainHand),
    );
    assert!(world.stored_entity_for_test(DIM, silent).is_some());
    hold(&mut world, "minecraft:water_bucket", 1);
    let tick = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact(silent, InteractionHand::MainHand),
    );
    assert!(world.stored_entity_for_test(DIM, silent).is_none());
    assert!(!sound(&tick, "minecraft:item.bucket.fill_fish"));
    assert!(
        matches!(tick.snapshot.players[0].inventory.slots[0].component_patch.codec_backed_nbt.get(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID), Some(Nbt::Compound(entries)) if entries.iter().any(|entry| entry.name == "Silent" && entry.value == Nbt::Byte(1)))
    );
}
