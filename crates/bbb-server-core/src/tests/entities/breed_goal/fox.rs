use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_FOX_TRUSTED_DATA_ID_0, VANILLA_FOX_TRUSTED_DATA_ID_1,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;
use bbb_protocol::packets::EntityDataValueKind;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

#[test]
fn fox_breeding_clears_parent_poses_and_saves_the_actual_feeders_as_child_trust() {
    for same_feeder in [false, true] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = GameType::Creative;
        let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
        let parent = serde_json::json!([{
            "condition":"minecraft:entity_properties", "entity":"this",
            "predicate":{"type":"minecraft:fox",
                "components":{"minecraft:fox/variant":"snow"}}
        }]);
        for (id, child_variant) in [
            ("test:breed_snow_fox", "snow"),
            ("test:breed_red_fox", "red"),
        ] {
            startup.advancements.advancements.push(
                bbb_server_datapack::advancement::AdvancementElement {
                    id: id.into(),
                    source_pack: "test".into(),
                    advancement: crate::tests::parse_advancement(
                        &serde_json::json!({"criteria":{"bred":{
                            "trigger":"minecraft:bred_animals",
                            "conditions":{
                                "parent":parent,
                                "partner":parent,
                                "child":[{
                                    "condition":"minecraft:entity_properties", "entity":"this",
                                    "predicate":{"type":"minecraft:fox",
                                        "components":{"minecraft:fox/variant":child_variant}}
                                }]
                            }
                        }}}),
                    )
                    .unwrap(),
                },
            );
        }
        let mut world = spawn_pinned_world(config);
        world.tick_rate_mut().set_frozen(true);
        join_loaded(&sender, &receiver, &mut world, 1, "first", 0xfeed);
        join_loaded(&sender, &receiver, &mut world, 2, "second", 0xbeef);
        let players = world.publication().players.clone();
        let first_player = players
            .iter()
            .find(|player| player.connection_id == ConnectionId(1))
            .unwrap();
        let second_player = players
            .iter()
            .find(|player| player.connection_id == ConnectionId(2))
            .unwrap();
        let position = first_player.position;
        let first_uuid = first_player.uuid;
        let second_uuid = second_player.uuid;
        let first =
            an_animal_beside_a_session(&mut world, VANILLA_ENTITY_TYPE_FOX_ID, position, -2.0);
        let second =
            an_animal_beside_a_session(&mut world, VANILLA_ENTITY_TYPE_FOX_ID, position, 2.0);
        for connection in [1, 2] {
            world.set_inventory_slot_for_test(
                ConnectionId(connection),
                0,
                stack_of("minecraft:sweet_berries", 2),
            );
        }
        for (parent, feeder) in [(first, 1), (second, if same_feeder { 1 } else { 2 })] {
            set_animal_record(
                &mut world,
                parent,
                "Type",
                NbtSummaryValue::String("snow".into()),
            );
            play(
                &sender,
                &receiver,
                &mut world,
                feeder,
                interact_entity_packet(parent),
            );
            assert!(animal_record_int(&world, parent, "InLove") > 0);
            for pose in ["Sleeping", "Sitting", "Crouching"] {
                set_animal_record(&mut world, parent, pose, NbtSummaryValue::Byte(1));
                assert_eq!(animal_record_byte(&world, parent, pose), 1);
            }
            let crate::entity::EntityData::GeneralLiving(data) =
                world.stored_entity_for_test(DIM, parent).unwrap().data()
            else {
                unreachable!()
            };
            assert!(!data.is_no_ai());
        }
        let parent_uuids =
            [first, second].map(|id| world.stored_entity_for_test(DIM, id).unwrap().uuid());
        world.tick_rate_mut().set_frozen(false);
        let mut approached = false;
        let mut cleared_before_birth = false;
        let mut child_id = None;
        for _ in 0..400 {
            world.tick(receiver.drain());
            let parents = [first, second].map(|id| world.stored_entity_for_test(DIM, id).unwrap());
            for (body, uuid) in parents.iter().zip(parent_uuids) {
                assert_eq!(body.uuid(), uuid);
                assert!(body.living().unwrap().health() > 0.0);
            }
            approached |= (parents[0].position().x - parents[1].position().x).abs() < 3.5;
            let babies: Vec<_> = stored_animals(&world, VANILLA_ENTITY_TYPE_FOX_ID)
                .into_iter()
                .filter(|(id, age)| *age < 0 && ![first, second].contains(id))
                .collect();
            if let Some((id, _)) = babies.first() {
                assert_eq!(babies.len(), 1);
                child_id = Some(*id);
                break;
            }
            cleared_before_birth |= [first, second].into_iter().all(|parent| {
                ["Sleeping", "Sitting", "Crouching"]
                    .into_iter()
                    .all(|pose| animal_record_byte(&world, parent, pose) == 0)
            });
        }
        assert!(
            approached && cleared_before_birth,
            "parents must stand and approach before birth"
        );
        let child_id = child_id.expect("the real fed Fox pair breeds a child");
        if same_feeder {
            assert!(world.player_advancement_is_done(ConnectionId(1), "test:breed_snow_fox"));
            assert!(!world.player_advancement_is_done(ConnectionId(1), "test:breed_red_fox"));
        }
        for parent in [first, second] {
            assert!(animal_record_int(&world, parent, "Age") > 0);
            assert_eq!(animal_record_int(&world, parent, "InLove"), 0);
        }
        assert!((-24000..0).contains(&animal_record_int(&world, child_id, "Age")));
        let NbtSummaryValue::Compound(record) = world
            .stored_entity_command_data_for_test(DIM, child_id)
            .unwrap()
        else {
            unreachable!()
        };
        assert_eq!(
            record
                .iter()
                .find(|field| field.name == "Type")
                .unwrap()
                .value,
            NbtSummaryValue::String("snow".into())
        );
        let mut trusted = vec![crate::entity::save::uuid_int_array(first_uuid)];
        if !same_feeder {
            trusted.push(crate::entity::save::uuid_int_array(second_uuid));
        }
        let NbtSummaryValue::List(saved) = &record
            .iter()
            .find(|field| field.name == "Trusted")
            .unwrap()
            .value
        else {
            unreachable!()
        };
        assert_eq!(saved.len(), trusted.len());
        assert!(trusted.iter().all(|uuid| saved.contains(uuid)));
        world.load_stored_entity_record_for_test(DIM, child_id, &record);
        let child = world.stored_entity_for_test(DIM, child_id).unwrap();
        let metadata = crate::entity::tracking::non_default_synched_values(child);
        let mut published: Vec<_> = metadata
            .iter()
            .filter_map(|value| {
                if ![VANILLA_FOX_TRUSTED_DATA_ID_0, VANILLA_FOX_TRUSTED_DATA_ID_1]
                    .contains(&value.data_id)
                {
                    return None;
                }
                match value.value {
                    EntityDataValueKind::OptionalLivingEntityReference(Some(uuid)) => Some(uuid),
                    _ => None,
                }
            })
            .collect();
        published.sort_unstable();
        let mut expected = vec![first_uuid];
        if !same_feeder {
            expected.push(second_uuid);
        }
        expected.sort_unstable();
        assert_eq!(published, expected);
    }
}
