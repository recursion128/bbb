use super::*;
use crate::entity::ai::mob::target::TargetRef;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::world::{ServerWorldConfig, WorldCommand};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID;
use bbb_protocol::item_types::{
    VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_HONEY_BOTTLE_ID, VANILLA_ITEM_SHEARS_ID,
};
use bbb_protocol::packets::{BlockHitResult, Direction, PlayServerbound, UseItemOn, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::Java26BuiltinDimension;

fn field(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[test]
fn beehive_harvest_smoke_preserves_occupants_and_unsmoked_harvest_releases_angry_bees() {
    let dim = Java26BuiltinDimension::Overworld;
    for item in [VANILLA_ITEM_SHEARS_ID, VANILLA_ITEM_GLASS_BOTTLE_ID] {
        for smoke in [false, true] {
            let (sender, receiver) = crate::tests::mailbox();
            let mut world = ServerWorld::new(ServerWorldConfig {
                game_type: bbb_protocol::game_types::GameType::Survival,
                ..crate::tests::datapack_config()
            })
            .unwrap();
            world.set_test_respawn_radius(0);
            crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "harvester", 1);
            crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bystander", 2);
            let floor = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
            )
            .unwrap();
            for x in -2..=4 {
                for z in -2..=4 {
                    world.set_block(dim, BlockPos { x, y: 299, z }, floor, SET_BLOCK_UPDATE_ALL);
                }
            }
            let pos = BlockPos { x: 0, y: 300, z: 1 };
            for id in [1, 2] {
                world.set_player_position_for_test(
                    ConnectionId(id),
                    Vec3d {
                        x: id as f64 - 0.5,
                        y: 300.0,
                        z: 0.5,
                    },
                );
            }
            let player = world.levels.player(ConnectionId(1)).unwrap();
            let target = TargetRef {
                entity_id: player.entity_id(),
                uuid: player.uuid(),
            };
            let player = world.levels.player(ConnectionId(2)).unwrap();
            let previous = TargetRef {
                entity_id: player.entity_id(),
                uuid: player.uuid(),
            };
            let state =
                bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                    "minecraft:bee_nest",
                    &std::collections::BTreeMap::from([
                        ("facing".into(), "east".into()),
                        ("honey_level".into(), "5".into()),
                    ]),
                )
                .unwrap();
            world.set_block(dim, pos, state, SET_BLOCK_UPDATE_ALL);
            world
                .level_mut(dim)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .unwrap()
                .beehive_mut()
                .unwrap()
                .load_additional(&[field(
                    "bees",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        field(
                            "entity_data",
                            NbtSummaryValue::Compound(vec![
                                field("id", NbtSummaryValue::String("minecraft:bee".into())),
                                field("anger_end_time", NbtSummaryValue::Long(10000)),
                                field("angry_at", NbtSummaryValue::IntArray(vec![0, 0, 0, 2])),
                            ]),
                        ),
                        field("ticks_in_hive", NbtSummaryValue::Int(37)),
                        field("min_ticks_in_hive", NbtSummaryValue::Int(600)),
                    ])]),
                )]);
            if smoke {
                let campfire = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                    bbb_protocol::block_types::VANILLA_BLOCK_CAMPFIRE_ID,
                )
                .unwrap();
                world.set_block(
                    dim,
                    BlockPos {
                        y: pos.y - 1,
                        ..pos
                    },
                    with_property(campfire, "lit", "true").unwrap(),
                    SET_BLOCK_UPDATE_ALL,
                );
            }
            let nearby = (item == VANILLA_ITEM_SHEARS_ID).then(|| {
                let idle = world
                    .level_mut(dim)
                    .entities
                    .spawn_general_living(
                        VANILLA_ENTITY_TYPE_BEE_ID,
                        Vec3d {
                            x: 6.0,
                            y: 300.0,
                            z: 6.0,
                        },
                    )
                    .unwrap();
                let busy = world
                    .level_mut(dim)
                    .entities
                    .spawn_general_living(
                        VANILLA_ENTITY_TYPE_BEE_ID,
                        Vec3d {
                            x: 7.0,
                            y: 300.0,
                            z: 7.0,
                        },
                    )
                    .unwrap();
                world.tick(receiver.drain());
                world.with_level_and_entities(dim, |level, entities| {
                    crate::entity::ai::mob::target::sense::set_mob_target(
                        entities,
                        level,
                        busy,
                        Some(previous),
                    );
                });
                assert_eq!(
                    world.level(dim).entities.mob_ai(busy).unwrap().target_ref(),
                    Some(previous)
                );
                (idle, busy)
            });
            world.stock_menu_slot_for_test(
                ConnectionId(1),
                36,
                ItemStackSummary {
                    item_id: Some(item),
                    count: 1,
                    component_patch: Default::default(),
                },
            );
            sender
                .try_send(WorldCommand::Play {
                    connection_id: ConnectionId(1),
                    packet: PlayServerbound::UseItemOn(UseItemOn {
                        hand: InteractionHand::MainHand,
                        hit: BlockHitResult {
                            pos,
                            direction: Direction::Up,
                            cursor_x: 0.5,
                            cursor_y: 0.5,
                            cursor_z: 0.5,
                            inside: false,
                            world_border_hit: false,
                        },
                        sequence: 1,
                    }),
                })
                .unwrap();
            world.tick(receiver.drain());
            assert_eq!(
                beehive_block::honey_level(world.block_state(dim, pos).unwrap()),
                0,
                "item={item}, smoke={smoke}"
            );
            let level = world.level(dim);
            let occupants = level
                .chunks
                .block_entities()
                .get(pos)
                .unwrap()
                .beehive()
                .unwrap()
                .release_entries();
            assert_eq!(occupants.len(), usize::from(smoke));
            if let Some((idle, busy)) = nearby {
                assert_eq!(
                    level.entities.mob_ai(busy).unwrap().target_ref(),
                    Some(previous)
                );
                let idle_target = level.entities.mob_ai(idle).unwrap().target_ref();
                if smoke {
                    assert_eq!(idle_target, None);
                } else {
                    assert!(
                        matches!(idle_target, Some(value) if value == target || value == previous)
                    );
                }
            }
            if !smoke {
                let released: Vec<_> = level
                    .entities
                    .ids()
                    .into_iter()
                    .filter(|id| {
                        nearby.is_none_or(|(idle, busy)| *id != idle && *id != busy)
                            && level
                                .entities
                                .get(*id)
                                .is_some_and(|e| e.entity_type_id() == VANILLA_ENTITY_TYPE_BEE_ID)
                    })
                    .collect();
                assert_eq!(released.len(), 1);
                assert!(
                    level.entities.mob_ai(released[0]).unwrap().target_ref() == Some(target)
                        || world.player_health_for_test(ConnectionId(1)) < 20.0,
                    "released bee targets or stings the harvester"
                );
            }
            if item == VANILLA_ITEM_GLASS_BOTTLE_ID {
                assert_eq!(
                    world
                        .levels
                        .player(ConnectionId(1))
                        .unwrap()
                        .inventory
                        .selected_stack()
                        .item_id(),
                    Some(VANILLA_ITEM_HONEY_BOTTLE_ID)
                );
            } else {
                let honeycomb: i32 = world
                    .level(dim)
                    .entities
                    .ids()
                    .into_iter()
                    .filter_map(|id| match world.level(dim).entities.get(id)?.data() {
                        crate::entity::EntityData::Item(drop)
                            if drop.item().item_id()
                                == Some(bbb_protocol::item_types::VANILLA_ITEM_HONEYCOMB_ID) =>
                        {
                            Some(drop.item().count())
                        }
                        _ => None,
                    })
                    .sum();
                assert_eq!(honeycomb, 3);
            }
            if !smoke && item == VANILLA_ITEM_GLASS_BOTTLE_ID {
                for _ in 0..400 {
                    if world.player_health_for_test(ConnectionId(1)) < 20.0 {
                        break;
                    }
                    world.tick(receiver.drain());
                }
                assert!(
                    world.player_health_for_test(ConnectionId(1)) < 20.0,
                    "released bees sting the harvester: {:?}",
                    world
                        .level(dim)
                        .entities
                        .ids()
                        .into_iter()
                        .filter_map(|id| {
                            let entity = world.level(dim).entities.get(id)?;
                            (entity.entity_type_id() == VANILLA_ENTITY_TYPE_BEE_ID).then(|| {
                                (
                                    entity.position(),
                                    world.level(dim).entities.mob_ai(id).unwrap().target_ref(),
                                    crate::entity::ai::mob::target::anger::read(entity),
                                )
                            })
                        })
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}
