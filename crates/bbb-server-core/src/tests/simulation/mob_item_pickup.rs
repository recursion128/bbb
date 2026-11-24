//! `Mob.aiStep`'s looting walk on the real tick: the
//! mob-griefing gate, `Mob.pickUpItem`'s equipment swap with its drop and the
//! fox's own `pickUpItem`.

use super::*;

use super::helpers::entity_command_record as record;
use super::nbt as entry;
use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::TakeItemEntity;

use crate::tests::entities::restore_entity_writes;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const MOB_POSITION: Vec3d = Vec3d {
    x: 2.5,
    y: 65.0,
    z: 4.5,
};

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    helpers::restored_world(
        helpers::operator_datapack_config(),
        &restore_candidate(0, None, Vec::new()),
    )
}

/// A NoAI mob that may pick up loot, so the walk is all that moves its
/// equipment: `aiStep` still runs without AI.
fn spawn_mob(
    world: &mut ServerWorld,
    entity_type_id: i32,
    extra: Vec<NbtSummaryEntry>,
) -> (i32, Uuid) {
    let (id, uuid) = helpers::spawn_living_at(world, entity_type_id, MOB_POSITION);
    let mut entries = vec![
        entry("CanPickUpLoot", NbtSummaryValue::Byte(1)),
        entry("NoAI", NbtSummaryValue::Byte(1)),
    ];
    entries.extend(extra);
    world.load_stored_entity_record_for_test(DIMENSION, id, &entries);
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_pos(MOB_POSITION);
    (id, uuid)
}

fn ground(world: &mut ServerWorld, item: &str, count: i32) -> i32 {
    world.spawn_ground_item_for_test(
        DIMENSION,
        MOB_POSITION,
        &ItemStackSummary {
            item_id: Some(item_id(item)),
            count,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    )
}

fn held(world: &ServerWorld, mob: i32, slot: EquipmentSlot) -> Option<(String, i32)> {
    let stack = world
        .stored_entity_for_test(DIMENSION, mob)?
        .living()?
        .item_by_slot(slot)?;
    Some((stack.item_resource_id()?.to_string(), stack.count()))
}

fn takes(tick: &TickResult) -> Vec<TakeItemEntity> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::TakeItemEntity(take),
            } => Some(*take),
            _ => None,
        })
        .collect()
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    helpers::nbt_field(entries, name).unwrap_or_else(|| panic!("the record writes {name}"))
}

fn mainhand_drop_chance(entries: &[NbtSummaryEntry]) -> Option<f32> {
    let NbtSummaryValue::Compound(chances) = field(entries, "drop_chances") else {
        return None;
    };
    chances
        .iter()
        .find(|entry| entry.name == "mainhand")
        .and_then(|entry| match entry.value {
            NbtSummaryValue::Float(bits) => Some(f32::from_bits(bits)),
            _ => None,
        })
}

/// `Mob` reads `mobGriefing`; with it on, `equipItemIfPossible`
/// prefers the sword with more attack damage, drops the
/// old one through `spawnAtLocation` because `max(r - 0.1, 0) < 1.0`
/// and `setItemSlotAndDropWhenKilled` guarantees the
/// new drop and pins the mob -- all of which the save
/// carries.
#[test]
fn a_zombie_takes_the_better_sword_drops_its_own_and_keeps_both_across_a_reload() {
    let (sender, receiver, mut world) = fixture();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule mob_griefing false",
    );
    let (zombie, uuid) = spawn_mob(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        vec![entry(
            "drop_chances",
            NbtSummaryValue::Compound(vec![entry(
                "mainhand",
                NbtSummaryValue::Float(1.0f32.to_bits()),
            )]),
        )],
    );
    world
        .stored_entity_mut_for_test(DIMENSION, zombie)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::MainHand,
            Some(crate::entity::item::plain_stack("minecraft:iron_sword", 1).unwrap()),
        );
    let sword = ground(&mut world, "minecraft:diamond_sword", 1);

    let tick = world.tick(receiver.drain());
    assert!(takes(&tick).is_empty());
    assert_eq!(
        held(&world, zombie, EquipmentSlot::MainHand),
        Some(("minecraft:iron_sword".into(), 1))
    );

    // The command may land before or after this tick's entity walk.
    let tick = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule mob_griefing true",
    );
    let mut taken = takes(&tick);
    if taken.is_empty() {
        taken = takes(&world.tick(receiver.drain()));
    }
    assert_eq!(
        taken,
        vec![TakeItemEntity {
            item_id: sword,
            player_id: zombie,
            amount: 1,
        }]
    );
    assert_eq!(
        held(&world, zombie, EquipmentSlot::MainHand),
        Some(("minecraft:diamond_sword".into(), 1))
    );
    let items = world.stored_items_for_test(DIMENSION);
    assert!(items.iter().all(|item| item.0 != sword), "{items:?}");
    let dropped: Vec<_> = items
        .iter()
        .map(|item| (item.2.as_str(), item.3, item.4))
        .collect();
    // `spawnAtLocation`'s `setDefaultPickUpDelay`.
    assert_eq!(dropped, vec![("minecraft:iron_sword", 1, 10)]);
    let entries = record(&world, uuid);
    assert_eq!(
        field(&entries, "PersistenceRequired"),
        &NbtSummaryValue::Byte(1)
    );
    assert_eq!(mainhand_drop_chance(&entries), Some(2.0));

    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the entity region saves");
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let restored = spawn_pinned_restore(datapack_config(), &candidate);
    let reloaded = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .expect("the zombie reloads");
    assert_eq!(
        held(&restored, reloaded, EquipmentSlot::MainHand),
        Some(("minecraft:diamond_sword".into(), 1))
    );
    let entries = record(&restored, uuid);
    assert_eq!(
        field(&entries, "PersistenceRequired"),
        &NbtSummaryValue::Byte(1)
    );
    assert_eq!(mainhand_drop_chance(&entries), Some(2.0));
}

/// `Fox.pickUpItem` mouths one item of a stack, leaves
/// the rest as a fresh item entity where the fox stands, guarantees the mouth
/// slot's drop, and reports a take of the emptied stack's count, zero.
#[test]
fn a_fox_mouths_one_berry_and_leaves_the_rest_of_the_stack_where_it_stood() {
    let (_sender, receiver, mut world) = fixture();
    let (fox, uuid) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_FOX_ID, Vec::new());
    let berries = ground(&mut world, "minecraft:sweet_berries", 3);

    let tick = world.tick(receiver.drain());
    assert_eq!(
        takes(&tick),
        vec![TakeItemEntity {
            item_id: berries,
            player_id: fox,
            amount: 0,
        }]
    );
    assert_eq!(
        held(&world, fox, EquipmentSlot::MainHand),
        Some(("minecraft:sweet_berries".into(), 1))
    );
    let items: Vec<_> = world
        .stored_items_for_test(DIMENSION)
        .into_iter()
        .map(|item| (item.0 == berries, item.1, item.2, item.3, item.4))
        .collect();
    assert_eq!(
        items,
        vec![(
            false,
            MOB_POSITION,
            "minecraft:sweet_berries".to_string(),
            2,
            0
        )]
    );
    assert_eq!(mainhand_drop_chance(&record(&world, uuid)), Some(2.0));
}

#[test]
fn a_villager_walks_to_wanted_food_and_collects_it_in_its_saved_inventory() {
    use bbb_protocol::packets::ItemRaritySummary;

    for (rarity, accepted) in [
        (ItemRaritySummary::Common, true),
        (ItemRaritySummary::Rare, false),
    ] {
        let (sender, receiver, mut world) = fixture();
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "fill -2 64 0 10 64 9 minecraft:stone",
        );
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "fill -2 65 0 10 67 9 minecraft:air",
        );
        let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
        let (villager, uuid) = world.spawn_stored_entity_for_test(
            DIMENSION,
            kind,
            MOB_POSITION,
            EntityData::GeneralLiving(GeneralLivingData::new(kind).unwrap()),
        );
        world.load_stored_entity_record_for_test(
            DIMENSION,
            villager,
            &[
                entry("CanPickUpLoot", NbtSummaryValue::Byte(1)),
                entry(
                    "Inventory",
                    NbtSummaryValue::List(
                        (0..8)
                            .map(|slot| {
                                NbtSummaryValue::Compound(vec![
                                    entry(
                                        "id",
                                        NbtSummaryValue::String(
                                            if slot == 0 {
                                                "minecraft:bread"
                                            } else {
                                                "minecraft:stone"
                                            }
                                            .into(),
                                        ),
                                    ),
                                    entry(
                                        "count",
                                        NbtSummaryValue::Int(if slot == 0 { 60 } else { 64 }),
                                    ),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ],
        );
        world
            .stored_entity_mut_for_test(DIMENSION, villager)
            .unwrap()
            .set_pos(MOB_POSITION);
        let food_position = Vec3d {
            x: MOB_POSITION.x + 3.25,
            ..MOB_POSITION
        };
        let food = world.spawn_ground_item_for_test(
            DIMENSION,
            food_position,
            &ItemStackSummary {
                item_id: Some(item_id("minecraft:bread")),
                count: 4,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![
                        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                    ],
                    rarity: Some(rarity),
                    ..Default::default()
                }
                .into(),
            },
        );
        let stone = world.spawn_ground_item_for_test(
            DIMENSION,
            Vec3d {
                x: MOB_POSITION.x - 2.0,
                ..MOB_POSITION
            },
            &ItemStackSummary {
                item_id: Some(item_id("minecraft:stone")),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        );
        let mut collected = false;
        for _ in 0..160 {
            collected |= takes(&world.tick(receiver.drain()))
                .iter()
                .any(|take| take.item_id == food && take.player_id == villager && take.amount == 4);
            if collected {
                break;
            }
        }
        let position = world
            .stored_entity_for_test(DIMENSION, villager)
            .unwrap()
            .position();
        assert_eq!(
            collected, accepted,
            "villager at {position:?}, food at {food_position:?}, rarity {rarity:?}"
        );
        if accepted {
            assert!(position.x > MOB_POSITION.x + 1.0, "{position:?}");
        }
        assert_eq!(
            world.stored_entity_for_test(DIMENSION, food).is_none(),
            accepted
        );
        assert!(world
            .stored_items_for_test(DIMENSION)
            .iter()
            .any(|item| item.0 == stone));
        let saved = record(&world, uuid);
        let NbtSummaryValue::List(values) = field(&saved, "Inventory") else {
            panic!("the villager saves its inventory");
        };
        assert_eq!(values.len(), 8);
        assert!(values.iter().any(|value| {
            let NbtSummaryValue::Compound(stack) = value else {
                return false;
            };
            field(stack, "id") == &NbtSummaryValue::String("minecraft:bread".into())
                && field(stack, "count") == &NbtSummaryValue::Int(if accepted { 64 } else { 60 })
        }));
    }
}

#[test]
fn a_fox_seeks_a_distant_nonfood_stack_while_no_ai_leaves_it_on_the_ground() {
    fn command(
        sender: &WorldCommandSender,
        receiver: &WorldCommandReceiver,
        world: &mut ServerWorld,
        line: &str,
    ) {
        let operation = world.publication().tick;
        enqueue(
            sender,
            WorldCommand::Admin {
                envelope: admin_envelope(operation, line, AdminScope::Command),
                request: AdminRequest::ExecuteCommand {
                    command: line.to_owned(),
                },
            },
        )
        .unwrap();
        let tick = world.tick(receiver.drain());
        let outcome = admin_outcome(&tick);
        assert!(!outcome.replayed, "{line}: {outcome:?}");
        let Ok(AdminResult::Command(report)) = &outcome.result else {
            panic!("{line}: {outcome:?}");
        };
        assert!(report.result.is_some(), "{line}: {report:?}");
    }

    for no_ai in [false, true] {
        let (sender, receiver, mut world) = fixture();
        world.tick_rate_mut().set_frozen(true);
        for line in [
            "gamemode creative alice",
            "gamerule spawn_mobs false",
            "gamerule advance_time false",
            "gamerule random_tick_speed 0",
            "time set midnight",
            "weather clear",
            "tp alice 28.5 65 28.5",
            "forceload add 0 0 31 31",
            "fill 0 64 0 15 68 15 minecraft:stone",
            "fill 1 65 4 12 66 4 minecraft:air",
        ] {
            command(&sender, &receiver, &mut world, line);
        }
        world.with_level_for_test(DIMENSION, |level| {
            assert!(level.has_ticking_column_at(BlockPos { x: 2, y: 65, z: 4 }));
            assert!(level.has_ticking_column_at(BlockPos { x: 7, y: 65, z: 4 }));
        });
        let (fox, uuid) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_FOX_ID, Vec::new());
        if !no_ai {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("attribute {uuid} minecraft:movement_speed base set 0"),
            );
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{NoAI:0b}}"),
            );
            world.tick_rate_mut().set_frozen(false);
            for _ in 0..20 {
                world.tick(receiver.drain());
                if world
                    .stored_entity_for_test(DIMENSION, fox)
                    .unwrap()
                    .on_ground()
                {
                    break;
                }
            }
            world.tick_rate_mut().set_frozen(true);
            assert!(world
                .stored_entity_for_test(DIMENSION, fox)
                .unwrap()
                .on_ground());
        }
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("attribute {uuid} minecraft:movement_speed base set 0.2"),
        );
        assert!(
            matches!(world.stored_entity_for_test(DIMENSION, fox).unwrap().data(),
            EntityData::GeneralLiving(data) if data.is_no_ai() == no_ai)
        );
        let start = world
            .stored_entity_for_test(DIMENSION, fox)
            .unwrap()
            .position();
        let stack = ItemStackSummary {
            item_id: Some(item_id("minecraft:stick")),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                ],
                custom_data: Some(NbtSummaryValue::Compound(vec![entry(
                    "fox_search_marker",
                    NbtSummaryValue::Int(7),
                )])),
                ..Default::default()
            }
            .into(),
        };
        let item = world.spawn_ground_item_for_test(
            DIMENSION,
            Vec3d {
                x: start.x + 5.0,
                ..start
            },
            &stack,
        );
        let EntityData::Item(ground) = world
            .stored_entity_for_test(DIMENSION, item)
            .unwrap()
            .data()
        else {
            panic!("a real ground item");
        };
        assert!(!ground.has_pickup_delay());
        let mut moved = false;
        let mut fast = false;
        world.tick_rate_mut().set_frozen(false);
        // Search admission is probabilistic; this bounds the normal-world observation.
        for _ in 0..600 {
            world.tick(receiver.drain());
            let body = world.stored_entity_for_test(DIMENSION, fox).unwrap();
            assert_eq!(body.uuid(), uuid);
            assert!(body.is_alive() && body.living().unwrap().health() > 0.0);
            let distance = (body.position().x - start.x).hypot(body.position().z - start.z);
            moved |= distance > 1.0;
            fast |= body.living().unwrap().speed() > 0.225;
            if no_ai {
                assert!(distance < 0.01, "NoAI fox moved: {:?}", body.position());
            } else if body
                .living()
                .unwrap()
                .item_by_slot(EquipmentSlot::MainHand)
                .is_some()
            {
                break;
            }
        }
        let mouth = world
            .stored_entity_for_test(DIMENSION, fox)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::MainHand);
        if no_ai {
            assert!(mouth.is_none());
            assert!(!moved && !fast);
            let EntityData::Item(ground) = world
                .stored_entity_for_test(DIMENSION, item)
                .unwrap()
                .data()
            else {
                panic!("NoAI leaves the actual ground item");
            };
            assert_eq!(ground.item().summary(), &stack);
        } else {
            assert!(moved && fast, "moved={moved}, fast={fast}");
            assert_eq!(
                mouth
                    .expect("the fox reaches and mouths the marker")
                    .summary(),
                &stack
            );
            assert!(world.stored_entity_for_test(DIMENSION, item).is_none());
        }
    }
}
