use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID;
use bbb_protocol::packets::SoundEventHolder;
use bbb_server_types::geometry::Vec3Ops;

const ALICE: ConnectionId = ConnectionId(1);

fn interact_entity_packet(entity_id: i32) -> PlayServerbound {
    PlayServerbound::Interact(InteractEntity {
        entity_id,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: false,
    })
}

fn stack_of(name: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
                .unwrap()
                .id,
        ),
        count,
        ..ItemStackSummary::empty()
    }
}

fn compound_mut<'a>(
    entries: &'a mut [NbtSummaryEntry],
    name: &str,
) -> &'a mut Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(entries) = &mut entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
    else {
        panic!("compound {name}");
    };
    entries
}

fn set_entry(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    entries.retain(|entry| entry.name != name);
    entries.push(NbtSummaryEntry {
        name: name.into(),
        value,
    });
}

fn shipwreck_world() -> ServerWorld {
    let mut candidate = restore_candidate_without_tickets(0, None, Vec::new());
    let mut document = candidate.world_gen_settings.document.clone();
    let data = compound_mut(document.entries_mut(), "data");
    set_entry(data, "generate_structures", NbtSummaryValue::Byte(1));
    let settings = compound_mut(
        compound_mut(
            compound_mut(compound_mut(data, "dimensions"), "minecraft:overworld"),
            "generator",
        ),
        "settings",
    );
    set_entry(
        settings,
        "biome",
        NbtSummaryValue::String("minecraft:ocean".into()),
    );
    set_entry(
        settings,
        "structure_overrides",
        NbtSummaryValue::List(vec![NbtSummaryValue::String("minecraft:shipwrecks".into())]),
    );
    candidate.world_gen_settings =
        bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
            document, 0,
        )
        .unwrap();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    spawn_pinned_restore(config, &candidate)
}

fn got_fish(world: &ServerWorld, id: i32) -> bool {
    let NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(DIMENSION, id)
        .unwrap()
    else {
        panic!("dolphin save");
    };
    matches!(
        entries
            .iter()
            .find(|entry| entry.name == "GotFish")
            .map(|entry| &entry.value),
        Some(NbtSummaryValue::Byte(1))
    )
}

#[test]
fn dolphin_eats_fish_grows_its_baby_and_saves_the_adults_fed_state() {
    for (age, silent) in [(-24_000, true), (0, false)] {
        let (sender, receiver, mut world) = super::melee::fixture();
        let position = Vec3d {
            x: 9.5,
            ..super::melee::PLAYER_POSITION
        };
        let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_DOLPHIN_ID).unwrap();
        data.set_no_ai(true);
        data.ageable_retained_mut().unwrap().set_age(age);
        let id = world
            .spawn_stored_entity_for_test(
                DIMENSION,
                VANILLA_ENTITY_TYPE_DOLPHIN_ID,
                position,
                EntityData::GeneralLiving(data),
            )
            .0;
        world
            .stored_entity_mut_for_test(DIMENSION, id)
            .unwrap()
            .set_silent(silent);
        world.set_inventory_slot_for_test(ALICE, 0, stack_of("minecraft:cod", 2));
        let fed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert_eq!(fed.snapshot.players[0].inventory.slots[0].count, 1);
        assert_eq!(got_fish(&world, id), age == 0);
        if age < 0 {
            let entity = world.stored_entity_for_test(DIMENSION, id).unwrap();
            let EntityData::GeneralLiving(data) = entity.data() else {
                unreachable!();
            };
            assert!(data.ageable_retained().unwrap().age() > age + 2000);
        }
        assert_eq!(fed.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:entity.dolphin.eat"))
        )), !silent);
    }
}

#[test]
fn dolphin_fed_on_real_ticks_leads_towards_a_locatable_structure() {
    use crate::commands::{LocateKind, LocateRequest, LocateSearchResult};
    let (sender, receiver) = mailbox();
    let mut world = shipwreck_world();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let allowed_ids = world
        .level_registry_tags_for_test(DIMENSION)
        .members("minecraft:worldgen/structure", "minecraft:dolphin_located")
        .iter()
        .map(ToString::to_string)
        .collect();
    let LocateSearchResult::Found {
        pos: destination, ..
    } = world.locate_live_target(&LocateRequest {
        kind: LocateKind::Structure,
        level: DIMENSION.into(),
        origin: BlockPos { x: 0, y: 66, z: 0 },
        target_display: "#minecraft:dolphin_located".into(),
        target_is_tag: true,
        allowed_ids,
    })
    else {
        panic!("the ocean flat world generates a real shipwreck");
    };
    let command = format!(
        "forceload add {} {} {} {}",
        destination.x - 48,
        destination.z - 48,
        destination.x + 48,
        destination.z + 48
    );
    let loaded = console_command(&sender, &receiver, &mut world, 100, &command);
    assert!(console_command_answer(&loaded).0.is_some());
    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &BTreeMap::from([("level".to_owned(), "0".to_owned())]),
    )
    .unwrap();
    let stone = block_state_id("minecraft:stone");
    for x in destination.x - 24..=destination.x + 32 {
        for z in destination.z - 24..=destination.z + 24 {
            world.set_block(
                DIMENSION,
                BlockPos { x, y: 64, z },
                stone,
                SET_BLOCK_UPDATE_ALL,
            );
            for y in 65..=70 {
                let pos = BlockPos { x, y, z };
                world.set_block(DIMENSION, pos, water, SET_BLOCK_UPDATE_ALL);
                assert_eq!(world.block_state(DIMENSION, pos), Some(water));
            }
        }
    }
    let start = Vec3d {
        x: f64::from(destination.x) + 18.0,
        y: 66.0,
        z: f64::from(destination.z) + 0.5,
    };
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: start.x - 1.0,
            ..start
        },
    );
    let id = spawn(&mut world, VANILLA_ENTITY_TYPE_DOLPHIN_ID, start).0;
    world.set_inventory_slot_for_test(ALICE, 0, stack_of("minecraft:salmon", 2));
    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(id),
    );
    assert_eq!(fed.snapshot.players[0].inventory.slots[0].count, 1);
    assert!(got_fish(&world, id));
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_air_supply(99);
    world.tick(Vec::new());
    assert!(
        got_fish(&world, id),
        "breathing pauses the trip without losing the fish"
    );
    let target = Vec3d {
        x: f64::from(destination.x) + 0.5,
        y: 66.0,
        z: f64::from(destination.z) + 0.5,
    };
    let mut closest = start.distance_to_sqr(target);
    for _ in 0..400 {
        world.tick(Vec::new());
        let dolphin = world.stored_entity_for_test(DIMENSION, id).unwrap();
        closest = closest.min(dolphin.position().distance_to_sqr(Vec3d {
            y: dolphin.position().y,
            ..target
        }));
        if !got_fish(&world, id) {
            break;
        }
    }
    assert!(
        closest < 16.0,
        "dolphin reaches structure {destination:?}; closest distance²={closest}, position={:?}",
        world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .position()
    );
    assert!(!got_fish(&world, id), "arrival finishes the trip");
}
