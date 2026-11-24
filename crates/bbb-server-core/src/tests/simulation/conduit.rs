use super::helpers::state_with as state;
use super::*;
use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const CORE: BlockPos = BlockPos { x: 8, y: 70, z: 8 };

fn build_core(world: &mut ServerWorld) {
    let water = state("minecraft:water", &[("level", "0")]);
    for x in CORE.x - 1..=CORE.x + 1 {
        for y in CORE.y - 1..=CORE.y + 1 {
            for z in CORE.z - 1..=CORE.z + 1 {
                world.set_block(DIM, BlockPos { x, y, z }, water, SET_BLOCK_UPDATE_ALL);
            }
        }
    }
    let mut frame = std::collections::BTreeSet::new();
    for edge in [-2, 2] {
        for step in -2..=2 {
            frame.extend([
                (0, edge, step),
                (0, step, edge),
                (edge, 0, step),
                (step, 0, edge),
                (edge, step, 0),
                (step, edge, 0),
            ]);
        }
    }
    assert_eq!(frame.len(), 42);
    for (x, y, z) in frame {
        world.set_block(
            DIM,
            BlockPos {
                x: CORE.x + x,
                y: CORE.y + y,
                z: CORE.z + z,
            },
            state("minecraft:prismarine", &[]),
            SET_BLOCK_UPDATE_ALL,
        );
    }
    world.set_block(
        DIM,
        CORE,
        state("minecraft:conduit", &[("waterlogged", "true")]),
        SET_BLOCK_UPDATE_ALL,
    );
}

#[test]
fn conduit_world_tick_activates_effects_attacks_and_restores_target() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "diver", 1);
    build_core(&mut world);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 7.4,
            y: 70.0,
            z: 7.4,
        },
    );
    let mob = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 9.4,
            y: 70.0,
            z: 9.4,
        },
        EntityData::GeneralLiving(mob),
    );
    world.load_stored_entity_record_for_test(
        DIM,
        id,
        &[NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(1),
        }],
    );
    world
        .stored_entity_mut_for_test(DIM, id)
        .unwrap()
        .set_pos(Vec3d {
            x: 9.4,
            y: 70.0,
            z: 9.4,
        });
    world.set_game_time_for_test(38);
    world.tick(receiver.drain());
    assert!(world.stored_entity_for_test(DIM, id).unwrap().is_in_water());
    let active = world.tick(receiver.drain());
    let core = world
        .get_block_entity(DIM, CORE)
        .unwrap()
        .conduit()
        .unwrap();
    assert!(core.is_active() && core.is_hunting());
    assert_eq!(core.effect_blocks().len(), 42);
    assert_eq!(core.destroy_target_uuid(), Some(uuid));
    let health = world
        .stored_entity_for_test(DIM, id)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert_eq!(health, 16.0);
    let effect = world
        .player_active_effect_for_test(ConnectionId(1), "minecraft:conduit_power")
        .unwrap();
    assert_eq!(effect.duration(), 259);
    assert!(effect.ambient() && effect.show_particles() && effect.show_icon());
    let attack = vanilla_sound_event_registry_id("minecraft:block.conduit.attack.target").unwrap();
    let sounds = sounds_sent(&active);
    let hit = sounds
        .iter()
        .find(|sound| {
            sound.sound
                == SoundEventHolder::Reference {
                    registry_id: attack,
                }
        })
        .unwrap();
    assert_eq!(
        hit.position,
        world.stored_entity_for_test(DIM, id).unwrap().position()
    );
    assert_eq!(hit.source, SoundSource::Blocks);
    assert!(active.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::DamageEvent(packet), .. } if packet.entity_id == id)));
    let publication = world.tick(receiver.drain());
    assert!(publication.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::BlockEntityData(packet), .. } if packet.pos == CORE)));
    for _ in 0..38 {
        world.tick(receiver.drain());
    }
    let repeat = world.tick(receiver.drain());
    assert_eq!(
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        12.0
    );
    let ambient = vanilla_sound_event_registry_id("minecraft:block.conduit.ambient").unwrap();
    assert!(sounds_sent(&repeat).iter().any(|sound| sound.sound
        == SoundEventHolder::Reference {
            registry_id: ambient
        }));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .unwrap();
    let parsed = Java26ChunkData::parse(write.document.clone(), restore_chunk_context()).unwrap();
    let mut candidate = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    crate::tests::entities::restore_entity_writes(&mut candidate, &request.entity_writes);
    world = spawn_pinned_restore(datapack_config(), &candidate);
    let core = world
        .get_block_entity(DIM, CORE)
        .unwrap()
        .conduit()
        .unwrap();
    assert!(!core.is_active() && !core.is_hunting());
    assert_eq!(core.destroy_target_uuid(), Some(uuid));
    join_loaded(&sender, &receiver, &mut world, 1, "diver", 1);
    let id = world.live_entity_id_for_uuid_for_test(DIM, uuid).unwrap();
    world.set_game_time_for_test(119);
    world.tick(receiver.drain());
    assert_eq!(
        world
            .stored_entity_for_test(DIM, id)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        8.0
    );
    assert!(world
        .get_block_entity(DIM, CORE)
        .unwrap()
        .conduit()
        .unwrap()
        .is_hunting());
    assert!(world.set_block(
        DIM,
        BlockPos {
            x: CORE.x - 1,
            y: CORE.y - 1,
            z: CORE.z - 1
        },
        state("minecraft:stone", &[]),
        SET_BLOCK_UPDATE_ALL
    ));
    world.set_game_time_for_test(159);
    let stopped = world.tick(receiver.drain());
    let core = world
        .get_block_entity(DIM, CORE)
        .unwrap()
        .conduit()
        .unwrap();
    assert!(!core.is_active() && !core.is_hunting());
    assert_eq!(core.destroy_target_uuid(), Some(uuid));
    let deactivate = vanilla_sound_event_registry_id("minecraft:block.conduit.deactivate").unwrap();
    assert!(sounds_sent(&stopped).iter().any(|sound| sound.sound
        == SoundEventHolder::Reference {
            registry_id: deactivate
        }));
}

#[test]
fn conduit_world_tick_respects_published_water_tag() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    let mut startup = (**config.datapacks.as_ref().unwrap()).clone();
    startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:fluid")
        .unwrap()
        .tags
        .insert("minecraft:water".into(), Vec::new());
    config.datapacks = Some(Arc::new(startup));
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "diver", 1);
    build_core(&mut world);
    world.set_game_time_for_test(39);
    world.tick(receiver.drain());
    assert!(!world
        .get_block_entity(DIM, CORE)
        .unwrap()
        .conduit()
        .unwrap()
        .is_active());
}

#[test]
fn conduit_world_tick_resolves_saved_target_in_another_level() {
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:protection")
        .unwrap();
    let mut effects =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()["effects"]
            .clone();
    effects["minecraft:damage_immunity"] = serde_json::json!([{
        "effect":{}, "requirements":{"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107}
    }]);
    let config = datapack_config_with_enchantment_effects("minecraft:protection", effects);
    for (time, expected_health) in [(107, 10.0), (108, 6.16)] {
        assert_cross_level_conduit_hit(
            &config,
            time,
            "minecraft:plains",
            "minecraft:plains",
            expected_health,
            None,
        );
    }
}

#[test]
fn cross_level_conduit_immunity_reads_the_callers_environment() {
    let index = crate::enchantment_facts::builtin_enchantment_registry()
        .iter()
        .position(|id| id == "minecraft:protection")
        .unwrap();
    let mut effects =
        serde_json::to_value(&crate::enchantment_facts::builtin_enchantment_documents()[index])
            .unwrap()["effects"]
            .clone();
    effects["minecraft:damage_immunity"] = serde_json::json!([{"effect":{}, "requirements":{
        "condition":"minecraft:all_of", "terms":[
            {"condition":"minecraft:environment_attribute_check",
             "attribute":"minecraft:gameplay/water_evaporates", "value":true},
            {"condition":"minecraft:entity_properties", "entity":"this",
             "predicate":{"passenger":{"type":"minecraft:player",
                 "nbt":"{SelectedItem:{id:'minecraft:diamond',count:1},Dimension:'minecraft:the_nether',RootVehicle:{Entity:{id:'minecraft:cow'}}}"}}},
            {"condition":"minecraft:value_check", "value":{"type":"minecraft:storage",
                "storage":"test:conduit", "path":"enabled"}, "range":1}
        ]
    }}]);
    let config = datapack_config_with_enchantment_environment(
        "minecraft:protection",
        effects,
        &[("minecraft:plains", false), ("minecraft:desert", true)],
    );
    for (caller, target, selected_count, storage, expected_health) in [
        ("minecraft:desert", "minecraft:plains", 1, 1, 10.0),
        ("minecraft:plains", "minecraft:desert", 1, 1, 6.16),
        ("minecraft:desert", "minecraft:plains", 2, 1, 6.16),
        ("minecraft:desert", "minecraft:plains", 1, 2, 6.16),
    ] {
        assert_cross_level_conduit_hit(
            &config,
            107,
            caller,
            target,
            expected_health,
            Some((selected_count, storage)),
        );
    }

    let mut config = datapack_config();
    let mut document: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:protection")
            .unwrap()
            .source_json,
    )
    .unwrap();
    document["effects"] = serde_json::json!({"minecraft:damage_immunity":[{
        "effect":{},"requirements":{"condition":"minecraft:all_of","terms":[
            {"condition":"minecraft:location_check","predicate":{"dimension":"minecraft:overworld"}},
            {"condition":"minecraft:entity_properties","entity":"this","predicate":{
                "type":"minecraft:cow","vehicle":{"type":"minecraft:oak_chest_boat",
                    "slots":{"container.*":{"items":"minecraft:diamond","count":1}}}
            }}
        ]}
    }]});
    let cargo = serde_json::json!({"type":"minecraft:chest","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:location_check",
            "predicate":{"dimension":"minecraft:the_nether"}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties",
            "entity":"this","predicate":{}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
    ]});
    let root = std::env::temp_dir().join(format!("bbb-foreign-conduit-cargo-{}", Uuid::new_v4()));
    let pack = root.join("cargo");
    std::fs::create_dir_all(pack.join("data/minecraft/enchantment")).unwrap();
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Foreign conduit cargo","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    for (path, value) in [
        ("data/minecraft/enchantment/protection.json", document),
        ("data/test/loot_table/conduit_vehicle.json", cargo),
    ] {
        std::fs::write(pack.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "diver", 1);
    build_core(&mut world);
    let target_dimension = Java26BuiltinDimension::Nether;
    world.prepare_full_for_test(target_dimension, &[(0, 0)]);
    for x in 8..=11 {
        for z in 8..=11 {
            for y in 69..=74 {
                assert!(world.set_block(
                    target_dimension,
                    BlockPos { x, y, z },
                    state(
                        if y == 69 {
                            "minecraft:stone"
                        } else {
                            "minecraft:air"
                        },
                        &[]
                    ),
                    SET_BLOCK_UPDATE_ALL,
                ));
            }
        }
    }
    let boat_uuid = Uuid::from_u128(5101);
    let cow_uuid = Uuid::from_u128(5102);
    for (index, command) in [
        "execute in minecraft:the_nether run forceload add 8 8".to_string(),
        "execute in minecraft:the_nether run summon minecraft:oak_chest_boat 9.4 70 9.4 {UUID:[I;0,0,0,5101],NoGravity:1b,LootTable:'test:conduit_vehicle',LootTableSeed:37L}".to_string(),
        "execute in minecraft:the_nether run summon minecraft:cow 12.4 70 9.4 {UUID:[I;0,0,0,5102],NoAI:1b,NoGravity:1b}".to_string(),
        format!("execute in minecraft:the_nether run ride {cow_uuid} mount {boat_uuid}"),
        format!("execute in minecraft:the_nether run item replace entity {cow_uuid} armor.head with iron_helmet[enchantments={{protection:1}}]"),
    ].into_iter().enumerate() {
        let tick = console_command(&sender, &receiver, &mut world, index as u64 + 1, &command);
        assert_eq!(crate::tests::entities::console_command_answer(&tick).0, Some(1), "{command}");
    }
    assert!(world.is_execute_chunk_loaded(
        &Java26LevelKey::from(target_dimension),
        BlockPos { x: 9, y: 70, z: 9 },
    ));
    let boat = world
        .live_entity_id_for_uuid_for_test(target_dimension, boat_uuid)
        .unwrap();
    let cow = world
        .live_entity_id_for_uuid_for_test(target_dimension, cow_uuid)
        .unwrap();
    assert_eq!(
        world.stored_passengers_for_test(target_dimension, boat),
        &[cow]
    );
    let cow_body = world.stored_entity_for_test(target_dimension, cow).unwrap();
    assert_eq!(cow_body.uuid(), cow_uuid);
    assert_eq!(
        cow_body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID
    );
    assert_eq!(
        cow_body
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::Head)
            .unwrap()
            .item_resource_id(),
        Some("minecraft:iron_helmet")
    );
    let health = cow_body.living().unwrap().health();
    let boat_body = world
        .stored_entity_for_test(target_dimension, boat)
        .unwrap();
    assert_eq!(boat_body.uuid(), boat_uuid);
    assert_eq!(
        boat_body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
    );
    assert!(crate::entity::item_slots::container_has_loot(boat_body));
    for slot in 0..crate::entity::item_slots::container_size(boat_body).unwrap() {
        assert!(crate::entity::item_slots::read(boat_body, slot as i32)
            .unwrap()
            .is_some_and(|stack| stack.is_empty()));
    }
    let NbtSummaryValue::Compound(before) = world
        .stored_entity_command_data_for_test(target_dimension, boat)
        .unwrap()
    else {
        panic!("pending boat")
    };
    assert!(before.iter().any(|entry| entry.name == "LootTable"
        && entry.value == NbtSummaryValue::String("test:conduit_vehicle".into())));
    assert!(before
        .iter()
        .any(|entry| entry.name == "LootTableSeed" && entry.value == NbtSummaryValue::Long(37)));
    assert!(!before.iter().any(|entry| entry.name == "Items"));
    let mut attack = console_command(
        &sender,
        &receiver,
        &mut world,
        6,
        &format!(
            "data merge block {} {} {} {{Target:[I;0,0,0,5102]}}",
            CORE.x, CORE.y, CORE.z
        ),
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&attack).0,
        Some(1)
    );
    let attack_sound = SoundEventHolder::Reference {
        registry_id: vanilla_sound_event_registry_id("minecraft:block.conduit.attack.target")
            .unwrap(),
    };
    for _ in 0..80 {
        if sounds_sent(&attack)
            .iter()
            .any(|sound| sound.sound == attack_sound)
        {
            break;
        }
        attack = world.tick(receiver.drain());
    }
    let cow_body = world.stored_entity_for_test(target_dimension, cow).unwrap();
    assert_eq!(cow_body.uuid(), cow_uuid);
    assert_eq!(cow_body.living().unwrap().health(), health);
    assert!(sounds_sent(&attack)
        .iter()
        .any(|sound| sound.sound == attack_sound && sound.position == cow_body.position()));
    assert_eq!(
        world
            .get_block_entity(DIM, CORE)
            .unwrap()
            .conduit()
            .unwrap()
            .destroy_target_uuid(),
        Some(cow_uuid)
    );
    assert_eq!(
        world.stored_passengers_for_test(target_dimension, boat),
        &[cow]
    );
    let boat_body = world
        .stored_entity_for_test(target_dimension, boat)
        .unwrap();
    assert_eq!(boat_body.uuid(), boat_uuid);
    assert!(!crate::entity::item_slots::container_has_loot(boat_body));
    let mut counts = BTreeMap::new();
    for slot in 0..crate::entity::item_slots::container_size(boat_body).unwrap() {
        if let Some(stack) = crate::entity::item_slots::read(boat_body, slot as i32)
            .unwrap()
            .filter(|stack| !stack.is_empty())
        {
            *counts
                .entry(stack.item_resource_id().unwrap().to_owned())
                .or_insert(0) += stack.count();
        }
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("minecraft:apple".to_string(), 1),
            ("minecraft:diamond".to_string(), 1)
        ])
    );
    let NbtSummaryValue::Compound(after) = world
        .stored_entity_command_data_for_test(target_dimension, boat)
        .unwrap()
    else {
        panic!("unpacked boat")
    };
    assert!(!after
        .iter()
        .any(|entry| entry.name == "LootTable" || entry.name == "LootTableSeed"));
    assert!(after.iter().any(|entry| entry.name == "Items"));
}

fn assert_cross_level_conduit_hit(
    config: &ServerWorldConfig,
    time: i64,
    caller_biome: &str,
    target_biome: &str,
    expected_health: f32,
    rider_and_storage: Option<(i32, i32)>,
) {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config.clone());
    join_loaded(&sender, &receiver, &mut world, 1, "diver", 1);
    set_paused_test_clocks(&sender, &receiver, &mut world, time, 0);
    build_core(&mut world);
    let target_dimension = Java26BuiltinDimension::Nether;
    let position = Vec3d {
        x: 9.4,
        y: 70.0,
        z: 9.4,
    };
    assert!(world.set_noise_biome_for_test(DIM, (2, 17, 2), caller_biome));
    assert!(world.set_noise_biome_for_test(target_dimension, (2, 17, 2), target_biome));
    if let Some((selected_count, storage)) = rider_and_storage {
        join_loaded(&sender, &receiver, &mut world, 2, "rider", 2);
        for command in [
            format!("item replace entity rider weapon.mainhand with diamond {selected_count}"),
            format!("data modify storage test:conduit enabled set value {storage}"),
            "execute in minecraft:the_nether run tp rider 9.4 70 9.4".to_string(),
        ] {
            let result = console_command(&sender, &receiver, &mut world, 1, &command);
            assert_eq!(
                crate::tests::entities::console_command_answer(&result).0,
                Some(1),
                "{command}"
            );
        }
        for x in 8..=10 {
            for z in 8..=10 {
                for y in 69..=73 {
                    assert!(world.set_block(
                        target_dimension,
                        BlockPos { x, y, z },
                        state(
                            if y == 69 {
                                "minecraft:stone"
                            } else {
                                "minecraft:air"
                            },
                            &[]
                        ),
                        SET_BLOCK_UPDATE_ALL,
                    ));
                }
            }
        }
    }
    let (id, uuid) = world.spawn_stored_entity_for_test(
        target_dimension,
        VANILLA_ENTITY_TYPE_COW_ID,
        position,
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
    );
    let enchanted = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            nbt("id", NbtSummaryValue::string("minecraft:stone")),
            nbt("count", NbtSummaryValue::Int(1)),
            nbt(
                "components",
                NbtSummaryValue::Compound(vec![nbt(
                    "minecraft:enchantments",
                    NbtSummaryValue::Compound(vec![nbt(
                        "minecraft:protection",
                        NbtSummaryValue::Int(1),
                    )]),
                )]),
            ),
        ]),
        &world.item_stack_resolver_for_test(),
    )
    .unwrap();
    world
        .stored_entity_mut_for_test(target_dimension, id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::Head, Some(enchanted));
    if rider_and_storage.is_some() {
        assert!(world.attach_external_player_for_test(ConnectionId(2), id));
    }
    world
        .level_block_entity_for_test(DIM, CORE)
        .unwrap()
        .conduit_mut()
        .unwrap()
        .set_destroy_target(Some(uuid));
    world.set_game_time_for_test(39);
    let tick = world.tick(receiver.drain());
    let health = world
        .stored_entity_for_test(target_dimension, id)
        .unwrap()
        .living()
        .unwrap()
        .health();
    assert!(
        (health - expected_health).abs() < 0.001,
        "clock {time}, caller={caller_biome}, target={target_biome}, rider/storage={rider_and_storage:?}: {health}"
    );
    assert!(!tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::DamageEvent(packet), .. } if packet.entity_id == id)));
    assert!(sounds_sent(&tick).iter().any(|sound| sound.sound
        == SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.conduit.attack.target")
                .unwrap()
        }
        && sound.position == position));
    assert_eq!(
        world
            .get_block_entity(DIM, CORE)
            .unwrap()
            .conduit()
            .unwrap()
            .destroy_target_uuid(),
        Some(uuid)
    );
}
