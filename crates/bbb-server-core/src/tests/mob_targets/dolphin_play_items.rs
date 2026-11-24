use super::melee::{fixture, ALICE};
use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::geometry::Vec3Ops;

#[test]
fn dolphin_play_items_chases_picks_up_and_throws_the_whole_component_stack() {
    let (sender, receiver, mut world) = fixture();
    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &BTreeMap::from([("level".to_owned(), "0".to_owned())]),
    )
    .unwrap();
    for x in 1..=14 {
        for z in 7..=9 {
            assert!(world.set_block(
                DIMENSION,
                BlockPos { x, y: 65, z },
                water,
                SET_BLOCK_UPDATE_ALL
            ));
        }
    }
    let stone = block_state_id("minecraft:stone");
    for x in 0..=15 {
        for z in 6..=10 {
            if x != 0 && x != 15 && z != 6 && z != 10 {
                continue;
            }
            for y in 65..=68 {
                assert!(world.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    stone,
                    SET_BLOCK_UPDATE_ALL
                ));
            }
        }
    }
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 8.5,
            y: 80.0,
            z: 8.5,
        },
    );
    let start = Vec3d {
        x: 3.5,
        y: 65.0,
        z: 8.5,
    };
    let destination = Vec3d { x: 6.5, ..start };
    let summary = ItemStackSummary {
        item_id: Some(item_id("minecraft:diamond")),
        count: 7,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            ],
            custom_name_nbt: Some(
                bbb_protocol::RichTextComponent::plain("Dolphin toy")
                    .component()
                    .clone(),
            ),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let original_id = world.spawn_ground_item_for_test(DIMENSION, destination, &summary);
    let original = match world
        .stored_entity_for_test(DIMENSION, original_id)
        .unwrap()
        .data()
    {
        EntityData::Item(item) => item.item().clone(),
        _ => unreachable!(),
    };
    world.tick(Vec::new());
    let (dolphin, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_DOLPHIN_ID, start);
    let mut approached = false;
    let mut took = false;
    for _ in 0..1200 {
        let tick = world.tick(receiver.drain());
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::KeepAlive { id },
            } = event
            {
                enqueue(
                    &sender,
                    WorldCommand::Play {
                        connection_id: *connection_id,
                        packet: PlayServerbound::KeepAlive { id: *id },
                    },
                )
                .unwrap();
            }
        }
        let body = world.stored_entity_for_test(DIMENSION, dolphin).unwrap();
        approached |=
            body.position().distance_to_sqr(destination) < start.distance_to_sqr(destination) - 4.0;
        took |= tick.events.iter().any(|event| {
            matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::TakeItemEntity(take), .. }
                if take.item_id == original_id && take.player_id == dolphin && take.amount == 7)
        });
        let thrown = world
            .stored_entity_ids_for_test(DIMENSION)
            .into_iter()
            .find_map(|id| {
                if id == original_id {
                    return None;
                }
                let item = match world.stored_entity_for_test(DIMENSION, id)?.data() {
                    EntityData::Item(item) if item.thrower() == Some(uuid) => item,
                    _ => return None,
                };
                Some(item)
            });
        if let Some(thrown) = thrown {
            assert!(approached, "dolphin swims toward the distant toy");
            assert!(took, "canonical pickup broadcasts the whole stack");
            assert_eq!(thrown.item(), &original);
            assert!((39..=40).contains(&thrown.pickup_delay()));
            assert!(body
                .living()
                .unwrap()
                .item_by_slot(EquipmentSlot::MainHand)
                .is_none());
            assert!(world
                .stored_entity_for_test(DIMENSION, original_id)
                .is_none());
            return;
        }
    }
    panic!(
        "dolphin did not throw its toy; approached={approached}, took={took}, body={:?}",
        world
            .stored_entity_for_test(DIMENSION, dolphin)
            .unwrap()
            .position()
    );
}
