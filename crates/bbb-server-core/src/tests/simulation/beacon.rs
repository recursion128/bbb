use super::block_state_id as state;
use super::*;
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const BEACON: BlockPos = BlockPos { x: 8, y: 70, z: 8 };

fn sounds(tick: &TickResult) -> Vec<i32> {
    sounds_sent(tick)
        .into_iter()
        .filter_map(|sound| match sound.sound {
            SoundEventHolder::Reference { registry_id } => Some(registry_id),
            _ => None,
        })
        .collect()
}

#[test]
fn beacon_world_tick_applies_effects_awards_construction_and_rebuilds_after_load() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    startup.advancements.advancements.push(bbb_server_datapack::advancement::AdvancementElement {
        id: "test:beacon".into(), source_pack: "test".into(),
        advancement: crate::tests::parse_advancement(&serde_json::json!({
            "criteria": {"built": {"trigger": "minecraft:construct_beacon", "conditions": {"level": 4}}},
                "rewards": {"recipes": ["minecraft:beacon"]}
        })).unwrap(),
    });
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 10.0,
            y: 70.0,
            z: 8.0,
        },
    );
    for layer in 1..=4 {
        for x in BEACON.x - layer..=BEACON.x + layer {
            for z in BEACON.z - layer..=BEACON.z + layer {
                world.set_block(
                    DIM,
                    BlockPos {
                        x,
                        y: BEACON.y - layer,
                        z,
                    },
                    block_state_id("minecraft:iron_block"),
                    SET_BLOCK_UPDATE_ALL,
                );
            }
        }
    }
    assert!(world.set_block(
        DIM,
        BEACON,
        block_state_id("minecraft:beacon"),
        SET_BLOCK_UPDATE_ALL
    ));
    let record = vec![
        nbt("id", NbtSummaryValue::string("minecraft:beacon")),
        nbt("primary_effect", NbtSummaryValue::string("minecraft:speed")),
        nbt(
            "secondary_effect",
            NbtSummaryValue::string("minecraft:regeneration"),
        ),
        nbt("Levels", NbtSummaryValue::Int(4)),
    ];
    *world.level_block_entity_for_test(DIM, BEACON).unwrap() =
        BlockEntity::load_static(BEACON, block_state_id("minecraft:beacon"), &record)
            .unwrap()
            .unwrap();
    assert_eq!(
        world
            .get_block_entity(DIM, BEACON)
            .unwrap()
            .beacon()
            .unwrap()
            .levels(),
        0
    );
    world.set_game_time_for_test(78);
    world.tick(receiver.drain());
    world.set_game_time_for_test(79);
    world.tick(receiver.drain());
    let beacon = world
        .get_block_entity(DIM, BEACON)
        .unwrap()
        .beacon()
        .unwrap();
    assert_eq!(beacon.levels(), 4);
    assert!(!beacon.beam_sections().is_empty());
    for name in ["minecraft:speed", "minecraft:regeneration"] {
        let effect = world
            .player_active_effect_for_test(ConnectionId(1), name)
            .unwrap();
        assert_eq!(effect.duration(), 339);
        assert_eq!(effect.amplifier(), 0);
        assert!(effect.ambient() && effect.show_particles() && effect.show_icon());
    }
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:beacon"));
    let saved = world.block_entity_save_for_test(DIM, BEACON).unwrap();
    assert!(saved
        .iter()
        .any(|field| field.name == "Levels" && field.value == NbtSummaryValue::Int(4)));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .unwrap();
    let parsed = Java26ChunkData::parse(write.document.clone(), restore_chunk_context()).unwrap();
    let candidate = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    world = spawn_pinned_restore(datapack_config(), &candidate);
    let loaded = world
        .get_block_entity(DIM, BEACON)
        .unwrap()
        .beacon()
        .unwrap();
    assert_eq!(loaded.levels(), 0);
    assert_eq!(loaded.primary_power(), Some("minecraft:speed"));
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    world.set_game_time_for_test(158);
    world.tick(receiver.drain());
    world.tick(receiver.drain());
    assert_eq!(
        world
            .get_block_entity(DIM, BEACON)
            .unwrap()
            .beacon()
            .unwrap()
            .levels(),
        4
    );
    let base = BlockPos {
        y: BEACON.y - 1,
        ..BEACON
    };
    assert!(world.set_block(DIM, base, state("minecraft:air"), SET_BLOCK_UPDATE_ALL));
    world.set_game_time_for_test(239);
    let inactive = world.tick(receiver.drain());
    assert_eq!(
        world
            .get_block_entity(DIM, BEACON)
            .unwrap()
            .beacon()
            .unwrap()
            .levels(),
        0
    );
    let deactivate = vanilla_sound_event_registry_id("minecraft:block.beacon.deactivate").unwrap();
    assert!(sounds(&inactive).contains(&deactivate));
}

#[test]
fn beacon_removal_plays_deactivate_even_when_removal_side_effects_are_skipped() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    assert!(world.set_block(DIM, BEACON, state("minecraft:beacon"), SET_BLOCK_UPDATE_ALL));
    world.tick(receiver.drain());
    assert!(world.set_block(
        DIM,
        BEACON,
        state("minecraft:air"),
        SET_BLOCK_UPDATE_ALL | 256
    ));
    let removed = world.tick(receiver.drain());
    assert!(world.get_block_entity(DIM, BEACON).is_none());
    let deactivate = vanilla_sound_event_registry_id("minecraft:block.beacon.deactivate").unwrap();
    assert_eq!(
        sounds(&removed)
            .iter()
            .filter(|&&id| id == deactivate)
            .count(),
        1
    );
}
