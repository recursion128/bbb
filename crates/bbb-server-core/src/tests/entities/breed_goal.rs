//! The goal-side entry of the breeding chain: `BreedGoal`'s admission and
//! `breed()` (`BreedGoal.java:32-80`) on the real mob tick, sharing
//! `animal_breeding::spawn_child_from_breeding` with the brain's
//! `AnimalMakeLove`.

use super::feeding::{
    animal_record_int, experience_orb_spawns, heart_events, interact_entity_packet, stack_of,
};
use super::*;

mod fox;

#[test]
fn unhurt_adult_and_baby_turtles_walk_around_a_wall_into_water() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID;

    for age in [0, -24000] {
        let (sender, receiver) = mailbox();
        let mut world =
            spawn_pinned_restore(datapack_config(), &restore_candidate(0, None, Vec::new()));
        join_loaded(&sender, &receiver, &mut world, 1, "swimmer", 1);
        let position = world.publication().players[0].position;
        let base = BlockPos {
            x: position.x.floor() as i32,
            y: position.y.floor() as i32,
            z: position.z.floor() as i32,
        };
        for dx in -3..=13 {
            for dz in -5..=5 {
                for dy in -2..=2 {
                    let block = if dy < 0 {
                        if (8..=11).contains(&dx) && (-2..=2).contains(&dz) {
                            "minecraft:water"
                        } else {
                            "minecraft:stone"
                        }
                    } else if dx == 4 && (-2..=2).contains(&dz) && dy < 2 {
                        "minecraft:stone"
                    } else {
                        "minecraft:air"
                    };
                    world.set_raw_block_state_for_test(
                        Java26BuiltinDimension::Overworld,
                        BlockPos {
                            x: base.x + dx,
                            y: base.y + dy,
                            z: base.z + dz,
                        },
                        if block == "minecraft:water" {
                            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                                bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
                            )
                            .unwrap()
                        } else {
                            block_state_id(block)
                        },
                    );
                }
            }
        }
        let turtle =
            an_animal_beside_a_session(&mut world, VANILLA_ENTITY_TYPE_TURTLE_ID, position, 1.5);
        let NbtSummaryValue::Compound(mut record) = world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, turtle)
            .unwrap()
        else {
            unreachable!()
        };
        record.retain(|field| field.name != "Age");
        record.push(nbt("Age", NbtSummaryValue::Int(age)));
        world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            turtle,
            &record,
        );
        let mut entered = false;
        for _ in 0..600 {
            world.tick(receiver.drain());
            let body = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, turtle)
                .unwrap();
            if body.is_in_water() {
                entered = true;
                assert!(body.position().x > position.x + 5.0);
                assert_eq!(body.living().unwrap().health(), 30.0);
                break;
            }
        }
        assert!(entered, "turtle age {age} did not enter the pool");
    }
}

#[test]
fn fed_sniffers_lay_one_saved_egg_with_or_without_mob_drops() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID;
    use bbb_protocol::packets::{SoundEventHolder, SoundSource};

    for mob_drops in [true, false] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Survival;
        let root = std::env::temp_dir().join(format!(
            "bbb-sniffer-breeding-{}-{mob_drops}",
            std::process::id()
        ));
        let storage = created_fixture_save(&root, "Sniffer breeding");
        let candidate = storage.load_candidate().unwrap();
        let mut world = spawn_pinned_restore(config, &candidate);
        join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
        console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("gamerule mob_drops {mob_drops}"),
        );
        let position = world.publication().players[0].position;
        let parents = [
            an_animal_beside_a_session(&mut world, VANILLA_ENTITY_TYPE_SNIFFER_ID, position, 1.0),
            an_animal_beside_a_session(&mut world, VANILLA_ENTITY_TYPE_SNIFFER_ID, position, 2.5),
        ];
        world.set_inventory_slot_for_test(
            ConnectionId(1),
            0,
            stack_of("minecraft:torchflower_seeds", 2),
        );
        for parent in parents {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                interact_entity_packet(parent),
            );
            assert!(animal_record_int(&world, parent, "InLove") > 0);
        }
        let mut orbs = 0;
        let mut hearts = 0;
        let mut plops = 0;
        for _ in 0..300 {
            let tick = world.tick(receiver.drain());
            orbs += experience_orb_spawns(&tick);
            hearts += heart_events(&tick).len();
            for event in tick.events {
                if let WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(sound),
                    ..
                } = event
                {
                    if matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                        if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:block.sniffer_egg.plop"))
                    {
                        assert_eq!(sound.source, SoundSource::Neutral);
                        assert_eq!(sound.volume, 1.0);
                        assert!((0.3..=0.7).contains(&sound.pitch));
                        plops += 1;
                    }
                }
            }
            if plops > 0 {
                break;
            }
        }
        assert_eq!(plops, 1);
        assert_eq!(hearts, 1);
        assert_eq!(orbs, usize::from(mob_drops));
        assert_eq!(
            stored_animals(&world, VANILLA_ENTITY_TYPE_SNIFFER_ID).len(),
            2
        );
        let eggs: Vec<_> = world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter(|id| matches!(
                world.stored_entity_for_test(Java26BuiltinDimension::Overworld, *id).unwrap().data(),
                crate::entity::EntityData::Item(item)
                    if item.item().item_resource_id() == Some("minecraft:sniffer_egg")
                        && item.item().count() == 1 && item.pickup_delay() > 0
            ))
            .collect();
        assert_eq!(eggs.len(), 1);
        let saved: Vec<_> = parents
            .into_iter()
            .chain(eggs)
            .map(|id| {
                let entity = world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                    .unwrap();
                entity.uuid()
            })
            .collect();
        let ages = parents.map(|id| {
            assert_eq!(animal_record_int(&world, id, "InLove"), 0);
            let age = animal_record_int(&world, id, "Age");
            assert!(age > 5900);
            age
        });
        assert_eq!(
            world.player_stat_for_test(
                ConnectionId(1),
                bbb_server_types::stats::custom_stat("minecraft:animals_bred")
            ),
            1
        );
        let request = world
            .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
            .unwrap();
        storage.save_snapshot(&candidate, &request).unwrap();
        let candidate = storage.load_candidate().unwrap();
        let restored = spawn_pinned_restore(datapack_config(), &candidate);
        for (uuid, age) in saved.iter().take(2).zip(ages) {
            let id = restored
                .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, *uuid)
                .unwrap();
            assert_eq!(animal_record_int(&restored, id, "Age"), age);
            assert_eq!(animal_record_int(&restored, id, "InLove"), 0);
        }
        let egg_id = restored
            .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, saved[2])
            .unwrap();
        let egg = restored
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, egg_id)
            .unwrap();
        assert!(matches!(egg.data(), crate::entity::EntityData::Item(item)
            if item.item().item_resource_id() == Some("minecraft:sniffer_egg")
                && item.item().count() == 1 && item.pickup_delay() > 0));
        assert_eq!(
            stored_animals(&restored, VANILLA_ENTITY_TYPE_SNIFFER_ID).len(),
            2
        );
        storage.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn fed_turtles_approach_and_breed_one_saved_egg_instead_of_a_baby() {
    use bbb_protocol::entity_synched_data::VANILLA_TURTLE_HAS_EGG_DATA_ID;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID;
    use bbb_protocol::packets::EntityDataValueKind;

    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let root = std::env::temp_dir().join(format!("bbb-turtle-home-eggs-{}", std::process::id()));
    let storage = created_fixture_save(&root, "Turtle home eggs");
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config, &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let position = world.publication().players[0].position;
    let home = BlockPos {
        x: position.x.floor() as i32 + 20,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    for dx in -3..=24 {
        for dz in -8..=8 {
            let pos = BlockPos {
                x: position.x.floor() as i32 + dx,
                y: home.y - 1,
                z: home.z + dz,
            };
            world.set_raw_block_state_for_test(
                Java26BuiltinDimension::Overworld,
                pos,
                block_state_id(if dx >= 17 {
                    "minecraft:sand"
                } else {
                    "minecraft:stone"
                }),
            );
            for dy in 0..=2 {
                world.set_raw_block_state_for_test(
                    Java26BuiltinDimension::Overworld,
                    BlockPos {
                        y: home.y + dy,
                        ..pos
                    },
                    if dy == 0 && (5..=13).contains(&dx) {
                        bbb_protocol::block_state_types::vanilla_block_default_state_id(
                            bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
                        )
                        .unwrap()
                    } else {
                        block_state_id("minecraft:air")
                    },
                );
            }
        }
    }
    let parents = [
        an_animal_beside_a_session(&mut world, VANILLA_ENTITY_TYPE_TURTLE_ID, position, 2.5),
        an_animal_beside_a_session(
            &mut world,
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            Vec3d {
                z: position.z + 2.5,
                ..position
            },
            0.0,
        ),
    ];
    for parent in parents {
        let NbtSummaryValue::Compound(mut entries) = world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, parent)
            .unwrap()
        else {
            unreachable!()
        };
        entries
            .iter_mut()
            .find(|entry| entry.name == "home_pos")
            .unwrap()
            .value = NbtSummaryValue::IntArray(vec![home.x, home.y, home.z]);
        world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            parent,
            &entries,
        );
    }
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:seagrass", 2));
    for parent in parents {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(parent),
        );
        assert!(animal_record_int(&world, parent, "InLove") > 0);
    }
    let mut orbs = 0;
    let mut hearts = 0;
    let mut synced_carriers = std::collections::BTreeSet::new();
    for _ in 0..300 {
        let tick = world.tick(receiver.drain());
        orbs += experience_orb_spawns(&tick);
        hearts += heart_events(&tick).len();
        for event in &tick.events {
            if let WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(data),
                ..
            } = event
            {
                if parents.contains(&data.id)
                    && data.values.iter().any(|value| {
                        value.data_id == VANILLA_TURTLE_HAS_EGG_DATA_ID
                            && value.value == EntityDataValueKind::Boolean(true)
                    })
                {
                    synced_carriers.insert(data.id);
                }
            }
        }
        if orbs > 0 && !synced_carriers.is_empty() {
            break;
        }
    }
    assert_eq!(
        orbs,
        1,
        "parents: {:?}",
        parents.map(|id| (
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .map(|entity| entity.position()),
            animal_record_int(&world, id, "InLove"),
            animal_record_byte(&world, id, "has_egg")
        ))
    );
    assert_eq!(hearts, 0);
    assert_eq!(
        stored_animals(&world, VANILLA_ENTITY_TYPE_TURTLE_ID).len(),
        2
    );
    let carriers: Vec<_> = parents
        .into_iter()
        .filter(|parent| animal_record_byte(&world, *parent, "has_egg") == 1)
        .collect();
    assert_eq!(carriers.len(), 1);
    assert_eq!(synced_carriers.into_iter().collect::<Vec<_>>(), carriers);
    for parent in parents {
        assert!(animal_record_int(&world, parent, "Age") > 0);
        assert_eq!(animal_record_int(&world, parent, "InLove"), 0);
        let bbb_protocol::NbtSummaryValue::Compound(entries) = world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, parent)
            .unwrap()
        else {
            panic!("turtle save is a compound")
        };
        world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            parent,
            &entries,
        );
        assert_eq!(
            animal_record_byte(&world, parent, "has_egg") == 1,
            carriers.contains(&parent)
        );
        assert!(animal_record_int(&world, parent, "Age") > 0);
        assert_eq!(animal_record_int(&world, parent, "InLove"), 0);
    }
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:animals_bred")
        ),
        1
    );
    assert!(
        world.player_advancement_is_done(ConnectionId(1), "minecraft:husbandry/breed_an_animal")
    );
    let carrier = carriers[0];
    let uuid = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, carrier)
        .unwrap()
        .uuid();
    let mut previous = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, carrier)
        .unwrap()
        .position();
    assert!((previous.x - f64::from(home.x)).abs() > 9.0);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: f64::from(home.x) + 0.5,
            y: f64::from(home.y),
            z: f64::from(home.z) + 5.5,
        },
    );
    let mut swam = false;
    let mut digging = false;
    let mut cleared = false;
    let mut lay_sound = false;
    for _ in 0..2400 {
        let tick = world.tick(receiver.drain());
        let turtle = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, carrier)
            .unwrap();
        let current = turtle.position();
        assert!(
            (current.x - previous.x).abs() < 1.0 && (current.z - previous.z).abs() < 1.0,
            "the turtle walks home"
        );
        previous = current;
        swam |= turtle.is_in_water();
        for event in tick.events {
            match event {
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::KeepAlive { id },
                } => {
                    enqueue(
                        &sender,
                        WorldCommand::Play {
                            connection_id,
                            packet: PlayServerbound::KeepAlive { id },
                        },
                    )
                    .unwrap();
                }
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetEntityData(data),
                    ..
                } if data.id == carrier => {
                    for value in data.values {
                        digging |= value.data_id
                            == bbb_protocol::entity_synched_data::VANILLA_TURTLE_LAYING_EGG_DATA_ID
                            && value.value == EntityDataValueKind::Boolean(true);
                        cleared |= value.data_id == VANILLA_TURTLE_HAS_EGG_DATA_ID
                            && value.value == EntityDataValueKind::Boolean(false);
                    }
                }
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(sound),
                    ..
                } => {
                    lay_sound |= matches!(sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id } if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.turtle.lay_egg"));
                }
                _ => {}
            }
        }
        if animal_record_byte(&world, carrier, "has_egg") == 0 && cleared && lay_sound {
            break;
        }
    }
    assert_eq!(
        animal_record_byte(&world, carrier, "has_egg"),
        0,
        "turtle ended at {previous:?}, home {home:?}"
    );
    assert!(
        swam && digging && cleared && lay_sound,
        "swam={swam}, digging={digging}, cleared={cleared}, sound={lay_sound}"
    );
    let eggs: Vec<_> = (home.x - 4..=home.x + 4)
        .flat_map(|x| (home.z - 8..=home.z + 8).map(move |z| BlockPos { x, y: home.y, z }))
        .filter_map(|pos| {
            let state = world.block_state(Java26BuiltinDimension::Overworld, pos)?;
            (bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
                == Some(bbb_protocol::block_types::VANILLA_BLOCK_TURTLE_EGG_ID))
            .then_some((pos, state))
        })
        .collect();
    assert_eq!(eggs.len(), 1);
    let (egg_pos, egg_state) = eggs[0];
    assert!((1..=4)
        .contains(&bbb_server_types::block_queries::int_property(egg_state, "eggs").unwrap()));
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    let carrier = restored
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, uuid)
        .unwrap();
    assert_eq!(animal_record_byte(&restored, carrier, "has_egg"), 0);
    assert_eq!(
        restored.block_state(Java26BuiltinDimension::Overworld, egg_pos),
        Some(egg_state)
    );
    let NbtSummaryValue::Compound(entries) = restored
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, carrier)
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.name == "home_pos")
            .unwrap()
            .value,
        NbtSummaryValue::IntArray(vec![home.x, home.y, home.z])
    );
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// One cow of the breeding fixture, standing next to the session.
fn a_cow_beside_a_session(world: &mut ServerWorld, player_position: Vec3d, dx: f64) -> i32 {
    let cow_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let (cow, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        cow_type,
        Vec3d {
            x: player_position.x + dx,
            ..player_position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(cow_type).unwrap(),
        ),
    );
    cow
}

/// Every stored animal of one type, oldest id first, as `(entity id, Age)`.
pub(super) fn stored_animals(world: &ServerWorld, entity_type_id: i32) -> Vec<(i32, i32)> {
    (1..64)
        .filter(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == entity_type_id)
        })
        .map(|id| (id, animal_record_int(world, id, "Age")))
        .collect()
}

/// The hearts and experience orbs one window of ticks produced.
struct MatingWindow {
    ticks: usize,
    hearts: usize,
    orbs: usize,
}

/// Ticks until the first experience orb, or until `limit` ticks have run.
fn tick_until_first_orb(
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
    limit: usize,
) -> MatingWindow {
    let mut window = MatingWindow {
        ticks: 0,
        hearts: 0,
        orbs: 0,
    };
    for _ in 0..limit {
        let result = world.tick(receiver.drain());
        window.ticks += 1;
        window.hearts += heart_events(&result).len();
        window.orbs += experience_orb_spawns(&result);
        if window.orbs > 0 {
            break;
        }
    }
    window
}

/// The goal-driven half of the breeding chain on the real tick: a player feeds
/// two cows `#minecraft:cow_food`, and `BreedGoal` walks the pair together and
/// lets the shared owner pay the mating.
#[test]
fn a_player_feeding_two_cows_makes_them_mate_once_through_the_breed_goal() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let variant = "minecraft:cold";
    let sound =
        bbb_protocol::animal_sound_variant_types::CowSoundVariantIdentity::Moody.resource_id();
    let predicate = |variant| {
        serde_json::json!([{
            "condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"type":"minecraft:cow","components":{
                "minecraft:cow/variant":variant,"minecraft:cow/sound_variant":sound
            }}
        }])
    };
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (suffix, expected_variant) in [("components", variant), ("wrong_variant", "minecraft:warm")]
    {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: format!("test:breed_cow_{suffix}"),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(
                    &serde_json::json!({"criteria":{"bred":{
                        "trigger":"minecraft:bred_animals","conditions":{
                            "parent":predicate(expected_variant),"partner":predicate(variant)
                        }
                    }}}),
                )
                .unwrap(),
            },
        );
    }
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let first = a_cow_beside_a_session(&mut world, player_position, 1.0);
    let second = a_cow_beside_a_session(&mut world, player_position, 2.5);
    for cow in [first, second] {
        for (field, value) in [("variant", variant), ("sound_variant", sound)] {
            set_animal_record(
                &mut world,
                cow,
                field,
                NbtSummaryValue::String(value.into()),
            );
            assert_eq!(
                animal_record_field(&world, cow, field),
                Some(NbtSummaryValue::String(value.into()))
            );
        }
    }
    // `#minecraft:cow_food` names wheat (`AbstractCow.java:51-53`).
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:wheat", 4));

    // The feeding entry of the same chain, which is the only thing that puts an
    // animal in love (`Animal.setInLove`, `Animal.java:172-179`).
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(first),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(second),
    );
    assert!(animal_record_int(&world, first, "InLove") > 0);
    assert!(animal_record_int(&world, second, "InLove") > 0);

    // `this.loveTime >= this.adjustedTickDelay(60)` (`BreedGoal.java:57`): the
    // goal admits on the tick each cow falls in love and breeds thirty selector
    // ticks -- sixty game ticks -- later, within the three-block mating
    // distance.
    let window = tick_until_first_orb(&mut world, &receiver, 300);

    assert_eq!(
        window.orbs, 1,
        "the pair mated {} times in {} ticks",
        window.orbs, window.ticks
    );
    assert_eq!(
        window.hearts, 1,
        "the mating broadcast {} heart events",
        window.hearts
    );
    // `this.setAge(6000)`/`partner.setAge(6000)` and `resetLove` on both
    // (`Animal.java:224-227`).
    for cow in [first, second] {
        assert_eq!(animal_record_int(&world, cow, "InLove"), 0);
        assert!(animal_record_int(&world, cow, "Age") > 0);
    }
    // `Animal.spawnChildFromBreeding`'s offspring (`Animal.java:210-215`): one
    // baby cow of the parents' own type.
    let cows = stored_animals(
        &world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
    );
    assert_eq!(cows.len(), 3, "the parents and their child: {cows:?}");
    let babies: Vec<i32> = cows
        .iter()
        .filter(|(id, age)| *age < 0 && *id != first && *id != second)
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(babies.len(), 1, "one child: {cows:?}");
    // The love cause's `Stats.ANIMALS_BRED` and `BRED_ANIMALS`
    // (`Animal.java:220-223`), whose child predicate names the cow.
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:animals_bred"),
        ),
        1
    );
    assert!(
        world.player_advancement_is_done(ConnectionId(1), "minecraft:husbandry/breed_an_animal")
    );
    let progress = world.player_advancements_to_data(ConnectionId(1));
    assert!(
        format!("{progress:?}").contains("minecraft:cow"),
        "bred_all_animals records the cow child: {progress:?}"
    );
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:breed_cow_components"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:breed_cow_wrong_variant"));
}

/// The love-time gate before the pair can breed (`BreedGoal.java:57`).
#[test]
fn two_in_love_cows_below_the_love_time_gate_do_not_mate_yet() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let first = a_cow_beside_a_session(&mut world, player_position, 1.0);
    let second = a_cow_beside_a_session(&mut world, player_position, 2.5);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:wheat", 2));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(first),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(second),
    );

    // Forty ticks is under the sixty the delay needs, so the goal has walked
    // the pair together and paid nothing.
    let window = tick_until_first_orb(&mut world, &receiver, 40);
    assert_eq!(window.orbs, 0, "nothing is paid before the delay");
    assert_eq!(window.hearts, 0);
    assert!(animal_record_int(&world, first, "InLove") > 0);
    assert_eq!(animal_record_int(&world, first, "Age"), 0);

    // The same pair does mate once the delay has passed.
    let window = tick_until_first_orb(&mut world, &receiver, 300);
    assert_eq!(window.orbs, 1);
}

/// `Animal.canMate`'s class test (`Animal.java:201-207`): a cow and a sheep
/// neither search for each other nor share a partner class.
#[test]
fn a_cow_and_a_sheep_in_love_never_mate() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let cow = a_cow_beside_a_session(&mut world, player_position, 1.0);
    let sheep_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID;
    let (sheep, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        sheep_type,
        Vec3d {
            x: player_position.x + 2.0,
            ..player_position
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(sheep_type).unwrap(),
        ),
    );
    // `#minecraft:sheep_food` names wheat too (`Sheep.java:80-82`).
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:wheat", 2));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(cow),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(sheep),
    );
    assert!(animal_record_int(&world, cow, "InLove") > 0);
    assert!(animal_record_int(&world, sheep, "InLove") > 0);

    let window = tick_until_first_orb(&mut world, &receiver, 200);

    assert_eq!(window.orbs, 0, "a mixed pair pays nothing");
    assert_eq!(window.hearts, 0);
    // Both halves keep the timer they were fed (`Animal.java:172-179`).
    assert!(animal_record_int(&world, cow, "InLove") > 0);
    assert!(animal_record_int(&world, sheep, "InLove") > 0);
    assert_eq!(animal_record_int(&world, cow, "Age"), 0);
    assert_eq!(animal_record_int(&world, sheep, "Age"), 0);
}

/// One byte field of a stored animal's saved record.
fn animal_record_byte(world: &ServerWorld, entity_id: i32, name: &str) -> i8 {
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    match entries.iter().find(|entry| entry.name == name) {
        Some(bbb_protocol::NbtSummaryEntry {
            value: bbb_protocol::NbtSummaryValue::Byte(value),
            ..
        }) => *value,
        other => panic!("{name} is a byte field: {other:?}"),
    }
}

/// Rewrites one field of a stored animal's saved record through the load half
/// of the same save pair, which is how `/data merge entity` hands one in.
fn set_animal_record(
    world: &mut ServerWorld,
    entity_id: i32,
    name: &str,
    value: bbb_protocol::NbtSummaryValue,
) {
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("an animal record carries {name}"))
        .value = value;
    world.load_stored_entity_record_for_test(
        Java26BuiltinDimension::Overworld,
        entity_id,
        &entries,
    );
}

/// One animal of `entity_type_id`, standing next to the session.
fn an_animal_beside_a_session(
    world: &mut ServerWorld,
    entity_type_id: i32,
    player_position: Vec3d,
    dx: f64,
) -> i32 {
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            entity_type_id,
            Vec3d {
                x: player_position.x + dx,
                ..player_position
            },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(entity_type_id).unwrap(),
            ),
        )
        .0
}

/// Feeds both animals `#minecraft:cow_food`/`#minecraft:sheep_food` -- wheat
/// for either -- and ticks until the mating pays its experience orb.
fn feed_and_mate(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pair: [i32; 2],
    food: &str,
) -> MatingWindow {
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of(food, 4));
    for animal in pair {
        play(sender, receiver, world, 1, interact_entity_packet(animal));
    }
    for animal in pair {
        assert!(animal_record_int(world, animal, "InLove") > 0);
    }
    tick_until_first_orb(world, receiver, 300)
}

/// `Sheep.getBreedOffspring` (`Sheep.java:267-276`) through
/// `DyeColor.getMixedColor` (`DyeColor.java:120-127`): the lamb of a red and a
/// yellow parent is *orange*, a colour neither parent carries, because the
/// level's own `RecipeType.CRAFTING` answers `red_dye + yellow_dye` with
/// `orange_dye` (`recipe/orange_dye_from_red_yellow.json`). A coin flip could
/// never produce it, so this pins the recipe arm and not the one at `:125`.
#[test]
fn two_dyed_sheep_breed_the_lamb_their_dyes_craft() {
    let sheep_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID;
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let first = an_animal_beside_a_session(&mut world, sheep_type, player_position, 1.0);
    let second = an_animal_beside_a_session(&mut world, sheep_type, player_position, 2.5);
    // `DyeColor.RED` is 14 and `DyeColor.YELLOW` is 4.
    set_animal_record(
        &mut world,
        first,
        "Color",
        bbb_protocol::NbtSummaryValue::Byte(14),
    );
    set_animal_record(
        &mut world,
        second,
        "Color",
        bbb_protocol::NbtSummaryValue::Byte(4),
    );

    let window = feed_and_mate(
        &sender,
        &receiver,
        &mut world,
        [first, second],
        "minecraft:wheat",
    );
    assert_eq!(
        window.orbs, 1,
        "the pair mated once in {} ticks",
        window.ticks
    );

    let sheep = stored_animals(&world, sheep_type);
    let babies: Vec<i32> = sheep
        .iter()
        .filter(|(id, age)| *age < 0 && *id != first && *id != second)
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(babies.len(), 1, "one lamb: {sheep:?}");
    assert_eq!(
        animal_record_byte(&world, babies[0], "Color"),
        1,
        "`DyeColor.ORANGE`, which is what the two parents' dyes craft"
    );
    // The parents kept their own colours.
    assert_eq!(animal_record_byte(&world, first, "Color"), 14);
    assert_eq!(animal_record_byte(&world, second, "Color"), 4);
}

/// `MushroomCow.getOffspringVariant` (`MushroomCow.java:246-257`): two brown
/// parents hand the calf brown, which the default red construction would never
/// produce on its own.
#[test]
fn two_brown_mooshrooms_breed_a_brown_calf() {
    let mooshroom_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID;
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let first = an_animal_beside_a_session(&mut world, mooshroom_type, player_position, 1.0);
    let second = an_animal_beside_a_session(&mut world, mooshroom_type, player_position, 2.5);
    for parent in [first, second] {
        set_animal_record(
            &mut world,
            parent,
            "Type",
            bbb_protocol::NbtSummaryValue::string("brown"),
        );
    }

    let window = feed_and_mate(
        &sender,
        &receiver,
        &mut world,
        [first, second],
        "minecraft:wheat",
    );
    assert_eq!(
        window.orbs, 1,
        "the pair mated once in {} ticks",
        window.ticks
    );

    let herd = stored_animals(&world, mooshroom_type);
    let babies: Vec<i32> = herd
        .iter()
        .filter(|(id, age)| *age < 0 && *id != first && *id != second)
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(babies.len(), 1, "one calf: {herd:?}");
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, babies[0])
        .expect("the calf has a record")
    else {
        panic!("an entity record is a compound")
    };
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.name == "Type")
            .map(|entry| &entry.value),
        Some(&bbb_protocol::NbtSummaryValue::string("brown")),
        "the calf of two brown parents"
    );
}

/// One optional field of a stored animal's saved record.
fn animal_record_field(
    world: &ServerWorld,
    entity_id: i32,
    name: &str,
) -> Option<bbb_protocol::NbtSummaryValue> {
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value.clone())
}

/// `TamableAnimal.addAdditionalSaveData` writes `Owner` only for a tamed body
/// (`TamableAnimal.java:58-63`), which is how a record says it is tame.
fn is_tame(world: &ServerWorld, entity_id: i32) -> bool {
    animal_record_field(world, entity_id, "Owner").is_some()
}

/// Clicks `animal` with the held stack until it is tame, which each click has
/// a one-in-three chance of doing (`Wolf.java:501`, `Cat.java:475`).
fn tame_by_clicking(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    animal: i32,
    food: &str,
) {
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of(food, 64));
    for _ in 0..48 {
        if is_tame(world, animal) {
            return;
        }
        play(sender, receiver, world, 1, interact_entity_packet(animal));
    }
    panic!("the taming roll never took in 48 clicks");
}

/// The babies of one type that neither parent is, oldest id first.
fn babies_of(world: &ServerWorld, entity_type_id: i32, parents: [i32; 2]) -> Vec<i32> {
    stored_animals(world, entity_type_id)
        .into_iter()
        .filter(|(id, age)| *age < 0 && !parents.contains(id))
        .map(|(id, _)| id)
        .collect()
}

fn add_tamable_breeding_advancements(
    config: &mut ServerWorldConfig,
    species: &str,
    variant: &str,
    sound_variant: &str,
) {
    use bbb_protocol::builtin_enum_component_values::{
        vanilla_dye_color_name, VANILLA_DYE_COLOR_ORANGE_ID, VANILLA_DYE_COLOR_RED_ID,
        VANILLA_DYE_COLOR_YELLOW_ID,
    };

    let predicate = |collar, parent| {
        let mut components = serde_json::Map::from_iter([
            (
                format!("minecraft:{species}/variant"),
                serde_json::json!(variant),
            ),
            (
                format!("minecraft:{species}/collar"),
                serde_json::json!(vanilla_dye_color_name(collar)),
            ),
        ]);
        if parent {
            components.insert(
                format!("minecraft:{species}/sound_variant"),
                serde_json::json!(sound_variant),
            );
        }
        serde_json::json!([{
            "condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"type":format!("minecraft:{species}"),"components":components}
        }])
    };
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (suffix, child_collar) in [
        ("components", VANILLA_DYE_COLOR_ORANGE_ID),
        ("wrong_collar", VANILLA_DYE_COLOR_YELLOW_ID),
    ] {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: format!("test:breed_{species}_{suffix}"),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&serde_json::json!({
                    "criteria":{"bred":{
                        "trigger":"minecraft:bred_animals",
                        "conditions":{
                            "parent":predicate(VANILLA_DYE_COLOR_RED_ID, true),
                            "partner":predicate(VANILLA_DYE_COLOR_YELLOW_ID, true),
                            "child":predicate(child_collar, false)
                        }
                    }}
                }))
                .unwrap(),
            },
        );
    }
}

/// The whole tame-wolf chain on the real seams: the bone roll tames both
/// wolves and orders them to sit (`Wolf.java:491-495`, `:500-510`), the sit
/// toggle behind `super.mobInteract` (`:473-479`) stands them back up past
/// `Wolf.canMate`'s sitting-pose term (`:618`), and the pup is born tame with
/// the collar colour its parents' dyes craft (`:589-594`).
#[test]
fn two_tame_wolves_toggled_out_of_their_sit_order_breed_a_tame_pup() {
    let variant = "minecraft:snowy";
    let sound_variant =
        bbb_protocol::animal_sound_variant_types::WolfSoundVariantIdentity::Cute.resource_id();
    let wolf_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID;
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    add_tamable_breeding_advancements(&mut config, "wolf", variant, sound_variant);
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let first = an_animal_beside_a_session(&mut world, wolf_type, player_position, 1.0);
    let second = an_animal_beside_a_session(&mut world, wolf_type, player_position, 2.5);

    for wolf in [first, second] {
        for (field, value) in [("variant", variant), ("sound_variant", sound_variant)] {
            set_animal_record(
                &mut world,
                wolf,
                field,
                NbtSummaryValue::String(value.into()),
            );
            assert_eq!(
                animal_record_field(&world, wolf, field),
                Some(NbtSummaryValue::String(value.into()))
            );
        }
        tame_by_clicking(&sender, &receiver, &mut world, wolf, "minecraft:bone");
        // `this.setOrderedToSit(true)` (`Wolf.java:505`).
        assert_eq!(animal_record_byte(&world, wolf, "Sitting"), 1);
        // `DEFAULT_COLLAR_COLOR = DyeColor.RED` (`Wolf.java:102`).
        assert_eq!(animal_record_byte(&world, wolf, "CollarColor"), 14);
    }

    // `Wolf.mobInteract`'s collar arm (`:485-489`); `DyeColor.YELLOW` is 4.
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:yellow_dye", 1));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(second),
    );
    assert_eq!(animal_record_byte(&world, second, "CollarColor"), 4);

    // A bone is neither food nor a collar dye, so the same click that tamed
    // them now reaches the sit toggle (`:473-479`) and stands them back up.
    // `Wolf.canMate` refuses a partner still in the sitting pose (`:618`), so
    // this is the mating's precondition and not a nicety; the goal-side test
    // `two_tame_wolves_refuse_a_partner_still_in_the_sitting_pose` pins the
    // refusal itself.
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:bone", 2));
    for wolf in [first, second] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(wolf),
        );
        assert_eq!(animal_record_byte(&world, wolf, "Sitting"), 0);
    }

    // `#minecraft:wolf_food` names raw beef, and a wolf tamed to full health
    // falls past the healing arm (`:448`) into `Animal.mobInteract`.
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:beef", 4));
    for wolf in [first, second] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(wolf),
        );
        assert!(animal_record_int(&world, wolf, "InLove") > 0);
    }

    let window = tick_until_first_orb(&mut world, &receiver, 300);
    assert_eq!(
        window.orbs, 1,
        "the pair mated {} times in {} ticks",
        window.orbs, window.ticks
    );
    assert_eq!(window.hearts, 1);

    let pups = babies_of(&world, wolf_type, [first, second]);
    assert_eq!(pups.len(), 1, "one pup");
    // `baby.setTame(true, true)` (`Wolf.java:591`).
    assert!(is_tame(&world, pups[0]));
    assert_eq!(
        animal_record_field(&world, pups[0], "Owner"),
        animal_record_field(&world, first, "Owner"),
        "`baby.setOwnerReference(this.getOwnerReference())` (`:590`)"
    );
    // `DyeColor.getMixedColor` (`DyeColor.java:120-127`) crafts red and yellow
    // into `DyeColor.ORANGE`, which is 1 -- a colour no coin flip could give.
    assert_eq!(animal_record_byte(&world, pups[0], "CollarColor"), 1);
    assert_eq!(animal_record_byte(&world, first, "CollarColor"), 14);
    assert_eq!(animal_record_byte(&world, second, "CollarColor"), 4);
    assert_eq!(
        animal_record_field(&world, pups[0], "variant"),
        Some(NbtSummaryValue::String(variant.into()))
    );
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:breed_wolf_components"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:breed_wolf_wrong_collar"));
}

/// `Cat.canMate` (`Cat.java:377-383`) wants only two tame parents, and
/// `Cat.getBreedOffspring` (`:364-370`) hands the kitten the same three writes
/// the pup takes.
#[test]
fn two_tame_cats_breed_a_kitten_wearing_the_collar_their_dyes_craft() {
    let variant = "minecraft:siamese";
    let sound_variant =
        bbb_protocol::animal_sound_variant_types::CatSoundVariantIdentity::Royal.resource_id();
    let cat_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID;
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    add_tamable_breeding_advancements(&mut config, "cat", variant, sound_variant);
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let first = an_animal_beside_a_session(&mut world, cat_type, player_position, 1.0);
    let second = an_animal_beside_a_session(&mut world, cat_type, player_position, 2.5);

    // `#minecraft:cat_food` names raw cod (`Cat.java:447-449`).
    for cat in [first, second] {
        for (field, value) in [("variant", variant), ("sound_variant", sound_variant)] {
            set_animal_record(
                &mut world,
                cat,
                field,
                NbtSummaryValue::String(value.into()),
            );
            assert_eq!(
                animal_record_field(&world, cat, field),
                Some(NbtSummaryValue::String(value.into()))
            );
        }
        tame_by_clicking(&sender, &receiver, &mut world, cat, "minecraft:cod");
        assert_eq!(animal_record_byte(&world, cat, "CollarColor"), 14);
        // `this.setPersistenceRequired()` (`Cat.java:431`).
        assert_eq!(animal_record_byte(&world, cat, "PersistenceRequired"), 1);
    }

    // `Cat.mobInteract`'s collar arm (`:400-409`).
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:yellow_dye", 1));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(second),
    );
    assert_eq!(animal_record_byte(&world, second, "CollarColor"), 4);

    // `tryToTame` ordered both to sit (`Cat.java:477`), and
    // `SitWhenOrderedToGoal` holds a sitting cat's movement; the owner's empty
    // hand stands them up (`:419-423`).
    world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
    for cat in [first, second] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(cat),
        );
        assert_eq!(animal_record_byte(&world, cat, "Sitting"), 0);
    }

    // The same food now falls past `:411` -- both cats are whole -- into
    // `Animal.mobInteract`'s love branch.
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:cod", 4));
    for cat in [first, second] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(cat),
        );
        assert!(animal_record_int(&world, cat, "InLove") > 0);
    }

    let window = tick_until_first_orb(&mut world, &receiver, 300);
    assert_eq!(
        window.orbs, 1,
        "the pair mated {} times in {} ticks",
        window.orbs, window.ticks
    );

    let kittens = babies_of(&world, cat_type, [first, second]);
    assert_eq!(kittens.len(), 1, "one kitten");
    assert!(is_tame(&world, kittens[0]));
    assert_eq!(
        animal_record_field(&world, kittens[0], "Owner"),
        animal_record_field(&world, first, "Owner")
    );
    assert_eq!(animal_record_byte(&world, kittens[0], "CollarColor"), 1);
    assert_eq!(
        animal_record_field(&world, kittens[0], "variant"),
        Some(NbtSummaryValue::String(variant.into()))
    );
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:breed_cat_components"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:breed_cat_wrong_collar"));

    assert_eq!(
        animal_record_field(&world, first, "variant"),
        Some(NbtSummaryValue::String(variant.into()))
    );
    let owner = animal_record_field(&world, first, "Owner");
    let cats = [first, kittens[0]].map(|id| {
        (
            id,
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
                .uuid(),
        )
    });
    for ((id, uuid), operation_id, saved_variant, saved_sound) in [
        (cats[0], 4001, "minecraft:tabby", sound_variant),
        (
            cats[1],
            4002,
            "tabby",
            sound_variant.strip_prefix("minecraft:").unwrap(),
        ),
    ] {
        let tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation_id,
            &format!(
                "data merge entity {uuid} {{variant:'{saved_variant}',sound_variant:'{saved_sound}'}}"
            ),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
        assert_eq!(
            animal_record_field(&world, id, "variant"),
            Some(NbtSummaryValue::String("minecraft:tabby".into()))
        );
        assert_eq!(
            animal_record_field(&world, id, "sound_variant"),
            Some(NbtSummaryValue::String(sound_variant.into()))
        );
    }

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_restore(config, &candidate);
    let (joined, loaded) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    for (index, (_, uuid)) in cats.into_iter().enumerate() {
        let id = world
            .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, uuid)
            .unwrap();
        assert_eq!(animal_record_field(&world, id, "Owner"), owner);
        assert_eq!(
            animal_record_byte(&world, id, "CollarColor"),
            if index == 0 { 14 } else { 1 }
        );
        assert_eq!(animal_record_int(&world, id, "Age") < 0, index == 1);
        for (field, registry, resource, holder) in [
            (
                "variant",
                "minecraft:cat_variant",
                "minecraft:tabby",
                bbb_protocol::packets::EntityDataRegistryHolder::CatVariant,
            ),
            (
                "sound_variant",
                "minecraft:cat_sound_variant",
                sound_variant,
                bbb_protocol::packets::EntityDataRegistryHolder::CatSoundVariant,
            ),
        ] {
            assert_eq!(
                animal_record_field(&world, id, field),
                Some(NbtSummaryValue::String(resource.into()))
            );
            let expected = world
                .synchronized_registries()
                .registry_element_index(registry, resource)
                .unwrap();
            assert!(joined.events.iter().chain(&loaded.events).any(|event| {
                matches!(event,
                    WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                    if data.id == id && data.values.iter().any(|value| matches!(
                        &value.value,
                        EntityDataValueKind::RegistryId { serializer, value }
                        if *serializer == holder && value.holder_id() == expected
                    ))
                )
            }));
        }
    }

    let uuid = cats[0].1;
    let id = world
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, uuid)
        .unwrap();
    let variant_holder = world
        .synchronized_registries()
        .registry_element_index("minecraft:cat_variant", variant)
        .unwrap();
    for (operation_id, fields, current_sound) in [
        (
            5001,
            format!("variant:'{variant}',sound_variant:'minecraft:classic'"),
            "minecraft:classic",
        ),
        (
            5002,
            format!("sound_variant:'{sound_variant}'"),
            sound_variant,
        ),
    ] {
        let sound_holder = world
            .synchronized_registries()
            .registry_element_index("minecraft:cat_sound_variant", current_sound)
            .unwrap();
        let mut tick = console_command(
            &sender,
            &receiver,
            &mut world,
            operation_id,
            &format!("data merge entity {uuid} {{{fields}}}"),
        );
        assert_eq!(console_command_answer(&tick).0, Some(1));
        assert_eq!(
            animal_record_field(&world, id, "variant"),
            Some(NbtSummaryValue::String(variant.into()))
        );
        assert_eq!(
            animal_record_field(&world, id, "sound_variant"),
            Some(NbtSummaryValue::String(current_sound.into()))
        );
        let mut seen = [operation_id == 5002, false];
        for _ in 0..20 {
            for event in &tick.events {
                if let WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SetEntityData(data),
                    ..
                } = event
                {
                    if data.id != id {
                        continue;
                    }
                    for (index, (holder, expected)) in [
                        (
                            bbb_protocol::packets::EntityDataRegistryHolder::CatVariant,
                            variant_holder,
                        ),
                        (
                            bbb_protocol::packets::EntityDataRegistryHolder::CatSoundVariant,
                            sound_holder,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        seen[index] |= data.values.iter().any(|value| {
                            matches!(
                                &value.value,
                                EntityDataValueKind::RegistryId { serializer, value }
                                if *serializer == holder && value.holder_id() == expected
                            )
                        });
                    }
                }
            }
            if seen.into_iter().all(|seen| seen) {
                break;
            }
            tick = world.tick(receiver.drain());
        }
        assert!(seen.into_iter().all(|seen| seen));
        assert_eq!(
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
                .uuid(),
            uuid
        );
        assert_eq!(animal_record_field(&world, id, "Owner"), owner);
        assert_eq!(animal_record_byte(&world, id, "CollarColor"), 14);
        assert!(animal_record_int(&world, id, "Age") >= 0);
    }
}

/// Rabbits, which breed through a `BreedGoal` row and copy a parent's
/// variant (`Rabbit.java:340-352`).
#[test]
fn two_fed_rabbits_breed_a_kit_of_their_variant() {
    let rabbit_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID;
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (id, child_variant) in [
        ("test:breed_gold_rabbit", "gold"),
        ("test:breed_brown_rabbit", "brown"),
    ] {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&serde_json::json!({
                    "criteria":{"bred":{
                        "trigger":"minecraft:bred_animals",
                        "conditions":{
                            "parent":[{
                                "condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{
                                    "type":"minecraft:rabbit",
                                    "components":{"minecraft:rabbit/variant":"gold"}
                                }
                            }],
                            "partner":[{
                                "condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{
                                    "type":"minecraft:rabbit",
                                    "components":{"minecraft:rabbit/variant":"gold"}
                                }
                            }],
                            "child":[{
                                "condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{
                                    "type":"minecraft:rabbit",
                                    "components":{"minecraft:rabbit/variant":child_variant}
                                }
                            }]
                        }
                    }}
                }))
                .unwrap(),
            },
        );
    }
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let player_position = world.publication().players[0].position;
    let first = an_animal_beside_a_session(&mut world, rabbit_type, player_position, 1.0);
    let second = an_animal_beside_a_session(&mut world, rabbit_type, player_position, 2.0);
    for parent in [first, second] {
        // `Rabbit.Variant.GOLD`.
        set_animal_record(
            &mut world,
            parent,
            "RabbitType",
            bbb_protocol::NbtSummaryValue::Int(4),
        );
    }
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:carrot", 2));
    for parent in [first, second] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(parent),
        );
        assert!(animal_record_int(&world, parent, "InLove") > 0);
    }

    let window = tick_until_first_orb(&mut world, &receiver, 400);
    assert_eq!(window.orbs, 1, "mated once in {} ticks", window.ticks);
    let kits: Vec<i32> = stored_animals(&world, rabbit_type)
        .into_iter()
        .filter(|(id, age)| *age < 0 && *id != first && *id != second)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(kits.len(), 1);
    assert_eq!(animal_record_int(&world, kits[0], "RabbitType"), 4);
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:breed_gold_rabbit"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:breed_brown_rabbit"));
}

#[test]
fn feeding_bees_and_camels_breeds_native_babies_with_saved_ages_and_cooldowns() {
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_CAMEL_ID};
    for (entity_type, food) in [
        (VANILLA_ENTITY_TYPE_BEE_ID, "minecraft:dandelion"),
        (VANILLA_ENTITY_TYPE_CAMEL_ID, "minecraft:cactus"),
    ] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Survival;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
        let position = world.publication().players[0].position;
        let first = an_animal_beside_a_session(&mut world, entity_type, position, 1.0);
        let second = an_animal_beside_a_session(&mut world, entity_type, position, 2.5);
        let window = feed_and_mate(&sender, &receiver, &mut world, [first, second], food);
        assert_eq!(
            window.orbs, 1,
            "type {entity_type} mated once in {} ticks",
            window.ticks
        );
        if entity_type == VANILLA_ENTITY_TYPE_CAMEL_ID {
            for parent in [first, second] {
                assert!(world
                    .game_events(Java26BuiltinDimension::Overworld)
                    .0
                    .iter()
                    .any(
                        |event| event.game_event == crate::level_event::VanillaGameEvent::EAT
                            && event.source_entity_id == Some(parent)
                    ));
            }
        }
        let herd = stored_animals(&world, entity_type);
        assert_eq!(herd.len(), 3, "the parents and one native child");
        let babies = babies_of(&world, entity_type, [first, second]);
        assert_eq!(babies.len(), 1);
        assert!(animal_record_int(&world, babies[0], "Age") <= -23990);
        for parent in [first, second] {
            assert!(animal_record_int(&world, parent, "Age") > 5900);
            assert_eq!(animal_record_int(&world, parent, "InLove"), 0);
        }
        for entity in [first, second, babies[0]] {
            let age = animal_record_int(&world, entity, "Age");
            let bbb_protocol::NbtSummaryValue::Compound(entries) = world
                .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, entity)
                .unwrap()
            else {
                panic!("an animal record")
            };
            world.load_stored_entity_record_for_test(
                Java26BuiltinDimension::Overworld,
                entity,
                &entries,
            );
            assert_eq!(animal_record_int(&world, entity, "Age"), age);
            assert_eq!(animal_record_int(&world, entity, "InLove"), 0);
        }
        assert_eq!(
            world.player_stat_for_test(
                ConnectionId(1),
                bbb_server_types::stats::custom_stat("minecraft:animals_bred")
            ),
            1
        );
    }
}

#[test]
fn panda_breed_goal_requires_bamboo_and_saves_the_child_genes() {
    use bbb_protocol::entity_synched_data::VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID;
    use bbb_protocol::packets::{EntityDataValueKind, SoundEventHolder};
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "breeder", 0xfeed);
    let position = world.publication().players[0].position;
    let parents = [
        an_animal_beside_a_session(&mut world, VANILLA_ENTITY_TYPE_PANDA_ID, position, 1.0),
        an_animal_beside_a_session(&mut world, VANILLA_ENTITY_TYPE_PANDA_ID, position, 2.5),
    ];
    for parent in parents {
        for gene in ["MainGene", "HiddenGene"] {
            set_animal_record(
                &mut world,
                parent,
                gene,
                bbb_protocol::NbtSummaryValue::String("lazy".into()),
            );
        }
    }
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:bamboo", 2));
    for parent in parents {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(parent),
        );
        assert!(animal_record_int(&world, parent, "InLove") > 0);
    }
    let mut unhappy = false;
    let mut cant_breed = false;
    for _ in 0..100 {
        let tick = world.tick(receiver.drain());
        assert_eq!(experience_orb_spawns(&tick), 0);
        for event in &tick.events {
            match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetEntityData(data),
                    ..
                } if parents.contains(&data.id) => {
                    unhappy |= data.values.iter().any(|value| value.data_id == VANILLA_PANDA_UNHAPPY_COUNTER_DATA_ID
                        && matches!(value.value, EntityDataValueKind::Int(counter) if counter > 0));
                }
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(sound),
                    ..
                } => {
                    cant_breed |= matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                        if vanilla_sound_event_resource_id(*registry_id) == Some("minecraft:entity.panda.cant_breed"));
                }
                _ => {}
            }
        }
    }
    assert!(unhappy && cant_breed);
    assert_eq!(
        stored_animals(&world, VANILLA_ENTITY_TYPE_PANDA_ID).len(),
        2
    );
    let bamboo = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_BAMBOO_ID,
    )
    .unwrap();
    world.set_raw_block_state_for_test(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: position.x.floor() as i32 + 4,
            y: position.y.floor() as i32,
            z: position.z.floor() as i32,
        },
        bamboo,
    );
    let window = tick_until_first_orb(&mut world, &receiver, 300);
    assert_eq!(window.orbs, 1);
    assert_eq!(window.hearts, 1);
    let babies = babies_of(&world, VANILLA_ENTITY_TYPE_PANDA_ID, parents);
    assert_eq!(babies.len(), 1);
    for parent in parents {
        assert!(animal_record_int(&world, parent, "Age") > 5900);
        assert_eq!(animal_record_int(&world, parent, "InLove"), 0);
    }
    let child = babies[0];
    let saved: Vec<_> = ["Age", "MainGene", "HiddenGene"]
        .into_iter()
        .map(|name| (name, animal_record_field(&world, child, name).unwrap()))
        .collect();
    assert!(animal_record_int(&world, child, "Age") < 0);
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, child)
        .unwrap()
    else {
        panic!("child record");
    };
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, child, &entries);
    for (name, value) in saved {
        assert_eq!(animal_record_field(&world, child, name), Some(value));
    }
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::custom_stat("minecraft:animals_bred")
        ),
        1
    );
    assert!(
        world.player_advancement_is_done(ConnectionId(1), "minecraft:husbandry/breed_an_animal")
    );
}
