//! A villager takes a profession from a free workstation, trades with a
//! player through the merchant menu, levels up and restocks at its job site
//! (`Villager.java`, `VillagerGoalPackages.java`).

use super::feeding::{interact_entity_packet, stack_of};
use super::*;
use crate::entity::{MerchantItemCost, MerchantOffer, MerchantOffers};
use bbb_facts::container_inputs::ContainerInput;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID as VILLAGER;
use bbb_protocol::packets::{
    ContainerClick, ContainerCloseRequest, HashedStack, MerchantOffers as OffersPacket,
    SelectTradeCommand,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn record(world: &ServerWorld, entity: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(OVERWORLD, entity)
        .expect("the villager has a record")
    else {
        panic!("an entity record is a compound")
    };
    entries
}

fn field(entries: &[NbtSummaryEntry], name: &str) -> Option<NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value.clone())
}

fn villager_data(world: &ServerWorld, villager: i32) -> (String, i32) {
    let Some(NbtSummaryValue::Compound(data)) = field(&record(world, villager), "VillagerData")
    else {
        panic!("a villager saves its data")
    };
    let Some(NbtSummaryValue::String(profession)) = field(&data, "profession") else {
        panic!("villager data names a profession")
    };
    let Some(NbtSummaryValue::Int(level)) = field(&data, "level") else {
        panic!("villager data has a level")
    };
    (profession.try_to_string().unwrap(), level)
}

fn offers(world: &ServerWorld, villager: i32) -> MerchantOffers {
    world
        .stored_entity_for_test(OVERWORLD, villager)
        .and_then(crate::entity::Entity::merchant_offers)
        .cloned()
        .unwrap_or_default()
}

fn offers_packet(tick: &TickResult) -> Option<OffersPacket> {
    tick.events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::MerchantOffers(offers),
            ..
        } => Some(offers.clone()),
        _ => None,
    })
}

fn tick_until(
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
    limit: usize,
    mut done: impl FnMut(&ServerWorld) -> bool,
) -> bool {
    for _ in 0..limit {
        world.tick(receiver.drain());
        if done(world) {
            return true;
        }
    }
    false
}

#[test]
fn a_villager_takes_a_lectern_trades_levels_up_and_restocks_at_it() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    // The restock wait outlasts the silent player's keep-alive window.
    config.keep_alive_interval_ticks = 100_000;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "trader", 0x7ade);
    console_command(&sender, &receiver, &mut world, 1, "time set noon");
    let position = world.publication().players[0].position;
    let (x, y, z) = (
        position.x.floor() as i32 + 3,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    let villager = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            VILLAGER,
            Vec3d {
                x: f64::from(x) - 0.5,
                y: position.y,
                z: f64::from(z) + 0.5,
            },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(VILLAGER).unwrap()),
        )
        .0;
    let lectern = format!("setblock {x} {y} {z} minecraft:lectern");
    let profession = |world: &ServerWorld| villager_data(world, villager).0;

    // `AcquirePoi` claims the free lectern and `AssignProfessionFromJobSite`
    // turns the villager into a librarian beside it.
    console_command(&sender, &receiver, &mut world, 2, &lectern);
    assert!(tick_until(&mut world, &receiver, 600, |world| {
        profession(world) == "minecraft:librarian"
    }));
    // Without the lectern a villager that never traded loses the job.
    console_command(
        &sender,
        &receiver,
        &mut world,
        3,
        &format!("setblock {x} {y} {z} air"),
    );
    assert!(tick_until(&mut world, &receiver, 200, |world| {
        profession(world) == "minecraft:none"
    }));
    console_command(&sender, &receiver, &mut world, 4, &lectern);
    assert!(tick_until(&mut world, &receiver, 600, |world| {
        profession(world) == "minecraft:librarian"
    }));

    // The librarian's level-1 trade set (`trade_set/librarian/level_1.json`).
    let opened = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(villager),
    );
    let packet = offers_packet(&opened).expect("a librarian opens its trades");
    // `Villager.getTypeName` (Villager.java:702-704) titles the screen.
    let title = opened
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::OpenScreen(screen),
                ..
            } => Some(format!("{:?}", screen.title.component())),
            _ => None,
        })
        .expect("the merchant menu opens");
    assert!(
        title.contains("entity.minecraft.villager.librarian"),
        "{title}"
    );
    assert_eq!(packet.offers.len(), 2);
    assert_eq!((packet.villager_level, packet.villager_xp), (1, 0));
    assert!(packet.show_progress && packet.can_restock);
    let sold = |name: &str| {
        Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
                .unwrap()
                .id,
        )
    };
    for offer in &packet.offers {
        assert!(
            [
                sold("minecraft:emerald"),
                sold("minecraft:enchanted_book"),
                sold("minecraft:bookshelf")
            ]
            .contains(&offer.sell.item_id),
            "{offer:?}"
        );
    }
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest {
            container_id: opened_container(&opened),
        }),
    );

    // One trade from 9 to 11 experience passes the level-2 threshold of 10.
    let one_paper = MerchantOffer::new(
        MerchantItemCost::new("minecraft:paper".into(), 1, Vec::new()).unwrap(),
        None,
        crate::entity::item::plain_stack("minecraft:emerald", 1).unwrap(),
        1,
        2,
        0.05,
    );
    *world
        .stored_entity_mut_for_test(OVERWORLD, villager)
        .and_then(crate::entity::Entity::merchant_offers_mut)
        .unwrap() = MerchantOffers::new(vec![one_paper]);
    let mut entries = record(&world, villager);
    entries.retain(|entry| entry.name != "Xp");
    entries.push(NbtSummaryEntry {
        name: "Xp".into(),
        value: NbtSummaryValue::Int(9),
    });
    world.load_stored_entity_record_for_test(OVERWORLD, villager, &entries);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:paper", 1));
    let opened = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(villager),
    );
    let container_id = opened_container(&opened);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SelectTrade(SelectTradeCommand { item: 0 }),
    );
    let state_id = world.player_inventory_for_test(ConnectionId(1)).state_id();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id,
            state_id,
            slot_num: 2,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    let traded = offers(&world, villager);
    assert!(traded.offers()[0].is_out_of_stock());
    assert_eq!(
        field(&record(&world, villager), "Xp"),
        Some(NbtSummaryValue::Int(11))
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClose(ContainerCloseRequest { container_id }),
    );

    // Forty ticks after the menu closes the villager reaches level 2 and adds
    // the level-2 trade set to its offers.
    assert!(tick_until(&mut world, &receiver, 60, |world| {
        villager_data(world, villager).1 == 2
    }));
    let levelled = offers(&world, villager);
    assert!(levelled.offers().len() > 1, "{levelled:?}");
    assert!(world
        .stored_entity_for_test(OVERWORLD, villager)
        .and_then(|entity| entity.living())
        .is_some_and(|living| living.active_effect("minecraft:regeneration").is_some()));

    // Working at the lectern restocks the used offer.
    assert!(tick_until(&mut world, &receiver, 2_000, |world| {
        !offers(world, villager).offers()[0].is_out_of_stock()
    }));
    let saved = record(&world, villager);
    assert_eq!(
        field(&saved, "RestocksToday"),
        Some(NbtSummaryValue::Int(1))
    );

    // The save round trip keeps the offers, the level and the experience.
    let reloaded = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            VILLAGER,
            Vec3d {
                x: f64::from(x) - 4.5,
                y: position.y,
                z: f64::from(z) + 0.5,
            },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(VILLAGER).unwrap()),
        )
        .0;
    world.load_stored_entity_record_for_test(OVERWORLD, reloaded, &saved);
    let reloaded_record = record(&world, reloaded);
    for name in [
        "Offers",
        "VillagerData",
        "Xp",
        "LastRestock",
        "RestocksToday",
    ] {
        assert_eq!(field(&reloaded_record, name), field(&saved, name), "{name}");
    }
    assert_eq!(offers(&world, reloaded), offers(&world, villager));
}

fn opened_container(tick: &TickResult) -> i32 {
    tick.events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::OpenScreen(screen),
                ..
            } => Some(screen.container_id),
            _ => None,
        })
        .expect("the merchant menu opens")
}

#[test]
fn unreachable_workstations_yield_to_a_reachable_one_and_are_retried_after_repair() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "trader", 0x7ade);
    let origin = world.publication().players[0].position;
    let (x, y, z) = (
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );
    let mut operation_id = 0;
    let mut command = |world: &mut ServerWorld, text: &str| {
        operation_id += 1;
        let result = console_command(&sender, &receiver, world, operation_id, text);
        let answer = console_command_answer(&result);
        assert!(answer.0.is_some(), "{text}: {answer:?}");
    };
    command(&mut world, "gamerule spawn_mobs false");
    command(&mut world, "gamerule advance_time false");
    command(&mut world, "time set noon");
    command(
        &mut world,
        &format!("forceload add {x} {} {} {}", z - 16, x + 31, z + 15),
    );
    for dz in [-12, -6, 0, 6, 12] {
        command(
            &mut world,
            &format!(
                "fill {} {y} {} {} {} {} minecraft:stone",
                x + 6,
                z + dz - 2,
                x + 10,
                y + 2,
                z + dz + 2
            ),
        );
        command(
            &mut world,
            &format!("setblock {} {y} {} minecraft:lectern", x + 8, z + dz),
        );
    }
    let available = format!("setblock {} {y} {z} minecraft:lectern", x + 24);
    command(&mut world, &available);
    let villager = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            VILLAGER,
            origin,
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(VILLAGER).unwrap()),
        )
        .0;
    let job_site_pos = |world: &ServerWorld| {
        let mut value = NbtSummaryValue::Compound(record(world, villager));
        for name in ["Brain", "memories", "minecraft:job_site", "value", "pos"] {
            let NbtSummaryValue::Compound(entries) = value else {
                return None;
            };
            value = field(&entries, name)?;
        }
        Some(value)
    };
    let employed_near = |world: &ServerWorld, target_x: i32| {
        let at = world
            .stored_entity_for_test(OVERWORLD, villager)
            .unwrap()
            .position();
        villager_data(world, villager).0 == "minecraft:librarian"
            && (at.x - (f64::from(target_x) + 0.5)).hypot(at.z - (f64::from(z) + 0.5)) < 3.0
    };
    assert!(
        tick_until(&mut world, &receiver, 600, |world| employed_near(
            world,
            x + 24
        )),
        "five unreachable stations must not hide a farther usable lectern"
    );
    assert_eq!(
        job_site_pos(&world),
        Some(NbtSummaryValue::IntArray(vec![x + 24, y, z]))
    );
    command(
        &mut world,
        &format!("setblock {} {y} {z} minecraft:air", x + 24),
    );
    assert!(tick_until(
        &mut world,
        &receiver,
        200,
        |world| villager_data(world, villager).0 == "minecraft:none"
    ));
    command(
        &mut world,
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0",
    );
    for _ in 0..100 {
        world.tick(Vec::new());
    }
    command(
        &mut world,
        &format!(
            "fill {} {y} {} {} {} {} minecraft:air",
            x + 6,
            z - 2,
            x + 10,
            y + 2,
            z + 2
        ),
    );
    command(
        &mut world,
        &format!("setblock {} {y} {z} minecraft:lectern", x + 8),
    );
    for _ in 0..450 {
        world.tick(Vec::new());
    }

    command(
        &mut world,
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0.5",
    );
    assert!(
        tick_until(&mut world, &receiver, 700, |world| employed_near(
            world,
            x + 8
        )),
        "a repaired station becomes eligible again: position={:?}, profession={:?}",
        world
            .stored_entity_for_test(OVERWORLD, villager)
            .unwrap()
            .position(),
        villager_data(&world, villager)
    );
    assert_eq!(
        job_site_pos(&world),
        Some(NbtSummaryValue::IntArray(vec![x + 8, y, z]))
    );
}

#[test]
fn a_converted_villagers_workstation_can_be_claimed_again() {
    for (lightning, cross_level) in [(true, false), (false, false), (true, true), (false, true)] {
        let (sender, receiver) = mailbox();
        let saved = cross_level.then(|| {
            let root = std::env::temp_dir().join(format!(
                "bbb-villager-cold-poi-{}-{lightning}",
                std::process::id()
            ));
            let storage = created_fixture_save(&root, "bbb-villager-cold-poi");
            let candidate = storage.load_candidate().unwrap();
            (storage, candidate, root)
        });
        let mut world = if let Some((_, candidate, _)) = &saved {
            spawn_pinned_restore(datapack_config(), candidate)
        } else {
            spawn_pinned_world(datapack_config())
        };
        join_loaded(&sender, &receiver, &mut world, 1, "trader", 0x7ade);
        let origin = world.publication().players[0].position;
        let (x, y, z) = (
            origin.x.floor() as i32 + 3,
            origin.y.floor() as i32,
            origin.z.floor() as i32,
        );
        let mut operation_id = 0;
        let mut command = |world: &mut ServerWorld, text: &str| {
            operation_id += 1;
            let result = console_command(&sender, &receiver, world, operation_id, text);
            let outcome = admin_outcome(&result);
            assert!(
                matches!(&outcome.result, Ok(AdminResult::Command(report)) if report.result.is_some()),
                "{text}: {outcome:?}"
            );
            result
        };
        command(&mut world, "gamerule spawn_mobs false");
        command(&mut world, "gamerule advance_time false");
        command(&mut world, "gamerule fire_spread_radius_around_player 0");
        command(&mut world, "difficulty hard");
        command(&mut world, "time set noon");
        command(
            &mut world,
            &format!("setblock {x} {y} {z} minecraft:lectern"),
        );
        let spawn = |world: &mut ServerWorld| {
            world
                .spawn_stored_entity_for_test(
                    OVERWORLD,
                    VILLAGER,
                    Vec3d {
                        x: f64::from(x) - 0.5,
                        y: f64::from(y),
                        z: f64::from(z) + 0.5,
                    },
                    crate::entity::EntityData::GeneralLiving(
                        GeneralLivingData::new(VILLAGER).unwrap(),
                    ),
                )
                .0
        };
        let resident = spawn(&mut world);
        assert!(tick_until(&mut world, &receiver, 600, |world| {
            villager_data(world, resident).0 == "minecraft:librarian"
        }));
        command(
            &mut world,
            "data merge entity @e[type=minecraft:villager,limit=1] {NoAI:1b}",
        );
        command(
            &mut world,
            &format!(
                "tp @e[type=minecraft:villager,limit=1] {} {y} {}",
                f64::from(x) - 0.5,
                f64::from(z) + 0.5
            ),
        );
        let dimension = if cross_level {
            command(
                &mut world,
                &format!("execute in minecraft:the_nether run forceload add {x} {z}"),
            );
            world.ensure_entity_transfer_full_chunk_for_test(
                Java26BuiltinDimension::Nether,
                (x.div_euclid(16), z.div_euclid(16)),
            );
            world
                .transfer_stored_entity_tree_cross_level(
                    OVERWORLD,
                    Java26BuiltinDimension::Nether,
                    resident,
                    crate::entity::StoredEntityTransition {
                        position: Vec3d {
                            x: f64::from(x) - 0.5,
                            y: f64::from(y),
                            z: f64::from(z) + 0.5,
                        },
                        delta_movement: Vec3d::default(),
                        y_rot: 0.0,
                        x_rot: 0.0,
                        relatives_mask: 0,
                    },
                )
                .expect("the employed villager moves to the nether");
            assert!(world.stored_entity_for_test(OVERWORLD, resident).is_none());
            command(
                &mut world,
                &format!("execute in minecraft:the_nether run tp trader {x} {y} {z}"),
            );
            let source_key = bbb_server_types::save::Java26LevelKey::from(OVERWORLD);
            let source_chunk = (x >> 4, z >> 4);
            assert!(tick_until(&mut world, &receiver, 100, |world| {
                world
                    .runtime_unload_columns()
                    .contains(&(source_key.clone(), source_chunk))
            }));
            let (storage, candidate, _) = saved.as_ref().unwrap();
            let unload = world
                .runtime_unload_save_request(100, RegionCompression::Zlib)
                .unwrap();
            storage.save_runtime_unloads(candidate, &unload).unwrap();
            let records = world
                .runtime_unload_columns()
                .into_iter()
                .filter(|(dimension, _)| dimension == &source_key)
                .map(|(_, chunk)| {
                    let record = storage
                        .read_runtime_column(candidate, &source_key, chunk.0, chunk.1)
                        .unwrap();
                    (chunk, record)
                })
                .collect();
            world.retain_runtime_unload_columns(&source_key, records);
            world.acknowledge_save_success(&unload);
            assert!(!world
                .chunk_store_for_test(OVERWORLD)
                .is_loaded(source_chunk.0, source_chunk.1));
            Java26BuiltinDimension::Nether
        } else {
            OVERWORLD
        };
        {
            let mut command = |world: &mut ServerWorld, text: &str| {
                if cross_level {
                    command(
                        world,
                        &format!("execute in minecraft:the_nether run {text}"),
                    );
                } else {
                    command(world, text);
                }
            };
            let converted_type = if lightning {
                command(
                    &mut world,
                    &format!(
                        "summon minecraft:lightning_bolt {} {y} {}",
                        f64::from(x) - 0.5,
                        f64::from(z) + 0.5
                    ),
                );
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID
            } else {
                command(
                    &mut world,
                    &format!("summon minecraft:zombie {} {y} {z}", x + 4),
                );
                command(
                    &mut world,
                    "data merge entity @e[type=minecraft:zombie,limit=1] {NoAI:1b}",
                );
                command(
                    &mut world,
                    "damage @e[type=minecraft:villager,limit=1] 100 minecraft:mob_attack by @e[type=minecraft:zombie,limit=1]",
                );
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
            };
            assert!(
                tick_until(&mut world, &receiver, 40, |world| {
                    world
                        .stored_entity_ids_for_test(dimension)
                        .into_iter()
                        .filter_map(|id| world.stored_entity_for_test(dimension, id))
                        .any(|entity| {
                            entity.entity_type_id() == converted_type
                                && entity.is_alive_with_health()
                        })
                }),
                "conversion: lightning={lightning}, cross_level={cross_level}, remaining={:?}",
                world
                    .stored_entity_ids_for_test(dimension)
                    .into_iter()
                    .filter_map(|id| world.stored_entity_for_test(dimension, id))
                    .map(|e| (e.entity_type_id(), e.position(), e.is_alive_with_health()))
                    .collect::<Vec<_>>()
            );
            command(&mut world, "kill @e[type=!minecraft:player]");
        }
        if cross_level {
            let returned = command(
                &mut world,
                &format!(
                    "execute in minecraft:overworld run tp trader {} {y} {z}",
                    x - 3
                ),
            );
            let teleport_id = returned
                .events
                .iter()
                .find_map(|event| match event {
                    WorldEvent::Send {
                        packet: ClientboundPlayPacket::PlayerPosition(update),
                        ..
                    } => Some(update.id),
                    _ => None,
                })
                .expect("the returning player receives a teleport");
            for packet in [
                PlayServerbound::AcceptTeleportation { id: teleport_id },
                PlayServerbound::PlayerLoaded,
            ] {
                enqueue(
                    &sender,
                    WorldCommand::Play {
                        connection_id: ConnectionId(1),
                        packet,
                    },
                )
                .unwrap();
            }
            world.tick(receiver.drain());
            let (storage, candidate, _) = saved.as_ref().unwrap();
            for _ in 0..20 {
                for (dimension, chunk) in world.pending_runtime_columns() {
                    let record =
                        storage.read_runtime_column(candidate, &dimension, chunk.0, chunk.1);
                    assert!(world
                        .complete_runtime_columns(&dimension, vec![(chunk, record)])
                        .is_empty());
                }
                for (dimension, chunk) in world.pending_runtime_entity_chunks() {
                    let record = storage
                        .read_runtime_entity_chunk(candidate, &dimension, chunk.0, chunk.1)
                        .unwrap();
                    world
                        .complete_runtime_entity_chunk(&dimension, chunk, record)
                        .unwrap();
                }
                world.tick(receiver.drain());
                if world
                    .entity_ticking_chunks(OVERWORLD)
                    .contains(&(x >> 4, z >> 4))
                {
                    break;
                }
            }
            assert!(
                world
                    .entity_ticking_chunks(OVERWORLD)
                    .contains(&(x >> 4, z >> 4)),
                "source resumes: pending_columns={:?}, pending_entities={:?}, player_level={:?}",
                world.pending_runtime_columns(),
                world.pending_runtime_entity_chunks(),
                world.player_dimension(ConnectionId(1))
            );
        }
        if world
            .block_state(OVERWORLD, BlockPos { x: x - 1, y, z })
            .is_some_and(crate::block_behavior::fire::is_fire)
        {
            command(
                &mut world,
                &format!("setblock {} {y} {z} minecraft:air", x - 1),
            );
        }
        let newcomer = spawn(&mut world);
        assert!(
            tick_until(&mut world, &receiver, 600, |world| {
                villager_data(world, newcomer).0 == "minecraft:librarian"
            }),
            "the converted villager left the lectern occupied: lightning={lightning}, cross_level={cross_level}"
        );
        if let Some((storage, _, root)) = saved {
            storage.close().unwrap();
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}

/// `Villager.customServerAiStep` (`Villager.java:249-251`) clears
/// `AssignProfessionWhenSpawned` after the first brain tick.
#[test]
fn assign_profession_when_spawned_lasts_one_brain_tick() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "watcher", 0x7adf);
    let position = world.publication().players[0].position;
    let villager = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            VILLAGER,
            Vec3d {
                x: position.x + 3.0,
                y: position.y,
                z: position.z,
            },
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(VILLAGER).unwrap()),
        )
        .0;
    let mut entries = record(&world, villager);
    entries.push(NbtSummaryEntry {
        name: "AssignProfessionWhenSpawned".into(),
        value: NbtSummaryValue::Byte(1),
    });
    world.load_stored_entity_record_for_test(OVERWORLD, villager, &entries);
    assert_eq!(
        field(&record(&world, villager), "AssignProfessionWhenSpawned"),
        Some(NbtSummaryValue::Byte(1))
    );
    for _ in 0..2 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        field(&record(&world, villager), "AssignProfessionWhenSpawned"),
        None
    );
}
