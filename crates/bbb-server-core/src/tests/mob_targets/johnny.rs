use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_VINDICATOR_ID};
use bbb_protocol::item_types::VANILLA_ITEM_NAME_TAG_ID;
use bbb_protocol::RichTextComponent;

fn saved_johnny(world: &ServerWorld, vindicator: i32) -> bool {
    let Some(NbtSummaryValue::Compound(fields)) =
        world.stored_entity_command_data_for_test(DIMENSION, vindicator)
    else {
        panic!("the vindicator has a saved record");
    };
    match fields.iter().find(|field| field.name == "Johnny") {
        Some(NbtSummaryEntry {
            value: NbtSummaryValue::Byte(value),
            ..
        }) => *value != 0,
        None => false,
        other => panic!("the saved Johnny flag is a byte: {other:?}"),
    }
}

fn name_tag(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    vindicator: i32,
    component: RichTextComponent,
) {
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_NAME_TAG_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                ],
                custom_name_nbt: Some(component.component().clone()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: vindicator,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert_eq!(
        world
            .stored_entity_mut_for_test(DIMENSION, vindicator)
            .unwrap()
            .custom_name()
            .map(|name| name.component()),
        Some(component.component())
    );
}

fn creative_fixture() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
    Vec3d,
) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "namer", 1);
    let position = world.publication().players[0].position;
    let (vindicator, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        Vec3d {
            x: position.x + 1.0,
            ..position
        },
    );
    (sender, receiver, world, vindicator, uuid, position)
}

#[test]
fn a_real_name_tag_enables_johnny_and_renaming_or_clearing_keeps_the_cow_target_goal() {
    let (sender, receiver, mut world, vindicator, uuid, position) = creative_fixture();
    name_tag(
        &mut world,
        &sender,
        &receiver,
        vindicator,
        RichTextComponent::plain("Johnny"),
    );
    assert!(saved_johnny(&world, vindicator));
    name_tag(
        &mut world,
        &sender,
        &receiver,
        vindicator,
        RichTextComponent::plain("peaceful"),
    );
    assert!(saved_johnny(&world, vindicator));
    let cleared = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("data remove entity {uuid} CustomName"),
    );
    assert_eq!(console_command_answer(&cleared).0, Some(1));
    assert!(world
        .stored_entity_mut_for_test(DIMENSION, vindicator)
        .unwrap()
        .custom_name()
        .is_none());
    assert!(saved_johnny(&world, vindicator));
    let (_, cow_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: position.x + 5.0,
            ..position
        },
    );
    tick_until(&mut world, 200, |world| {
        target_of(world, vindicator) == Some(cow_uuid)
    });
}

#[test]
fn unnamed_lowercase_and_known_translation_with_johnny_fallback_do_not_target_a_cow() {
    let translated = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "translate".into(),
            value: NbtSummaryValue::String("entity.minecraft.cow".into()),
        },
        NbtSummaryEntry {
            name: "fallback".into(),
            value: NbtSummaryValue::String("Johnny".into()),
        },
    ]))
    .unwrap();
    for component in [
        None,
        Some(RichTextComponent::plain("johnny")),
        Some(translated),
    ] {
        let (sender, receiver, mut world, vindicator, _, position) = creative_fixture();
        if let Some(component) = component {
            name_tag(&mut world, &sender, &receiver, vindicator, component);
        }
        assert!(!saved_johnny(&world, vindicator));
        spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: position.x + 5.0,
                ..position
            },
        );
        for _ in 0..200 {
            world.tick(Vec::new());
            assert_eq!(target_of(&world, vindicator), None);
        }
    }
}

#[test]
fn data_load_overwrites_name_activation_and_the_final_flag_survives_a_cold_restart() {
    let config = datapack_config();
    for (index, (tail, expected)) in [
        ("merge entity {uuid} {CustomName:'Johnny'}", false),
        (
            "merge entity {uuid} {CustomName:'Johnny',Johnny:true}",
            true,
        ),
        (
            "merge entity {uuid} {CustomName:'Johnny',Johnny:false}",
            false,
        ),
        ("remove entity {uuid} Johnny", false),
    ]
    .into_iter()
    .enumerate()
    {
        let root =
            std::env::temp_dir().join(format!("bbb-johnny-restart-{}-{index}", std::process::id()));
        let storage = created_fixture_save(&root, "bbb-johnny-restart");
        let paths = storage.paths().clone();
        let mut candidate = storage.load_candidate().unwrap();
        install_forced_ticket(&mut candidate);
        let mut world = spawn_pinned_restore(config.clone(), &candidate);
        install_floor(&mut world);
        let (vindicator, uuid) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            Vec3d {
                x: 4.5,
                y: 65.0,
                z: 4.5,
            },
        );
        let (_, cow_uuid) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 9.5,
                y: 65.0,
                z: 4.5,
            },
        );
        let (sender, receiver) = mailbox();
        if index >= 2 {
            let enabled = console_command(
                &sender,
                &receiver,
                &mut world,
                1,
                &format!("data merge entity {uuid} {{CustomName:'Johnny',Johnny:true}}"),
            );
            assert_eq!(console_command_answer(&enabled).0, Some(1));
            assert!(saved_johnny(&world, vindicator));
        }
        let changed = console_command(
            &sender,
            &receiver,
            &mut world,
            2,
            &format!("data {}", tail.replace("{uuid}", &uuid.to_string())),
        );
        assert_eq!(console_command_answer(&changed).0, Some(1), "{tail}");
        assert_eq!(saved_johnny(&world, vindicator), expected, "{tail}");
        let request = world
            .save_request(1_786_000_004_000, 204, RegionCompression::Zlib)
            .unwrap();
        storage.save_snapshot(&candidate, &request).unwrap();
        world.acknowledge_save_success(&request);
        drop(world);
        storage.close().unwrap();
        let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
        let mut reloaded = storage.load_candidate().unwrap();
        install_forced_ticket(&mut reloaded);
        let mut restored = spawn_pinned_restore(config.clone(), &reloaded);
        let vindicator = restored
            .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
            .unwrap();
        assert_eq!(saved_johnny(&restored, vindicator), expected, "{tail}");
        assert_eq!(target_of(&restored, vindicator), None);
        if expected {
            tick_until(&mut restored, 200, |world| {
                target_of(world, vindicator) == Some(cow_uuid)
            });
        } else {
            for _ in 0..200 {
                restored.tick(Vec::new());
                assert_eq!(target_of(&restored, vindicator), None, "{tail}");
            }
        }
        drop(restored);
        storage.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
