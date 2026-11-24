use super::feeding::{animal_record_int, interact_entity_packet, stack_of};
use super::horse_interaction::{hold, horse_world, item_id, mount_screen, played, record, worn};
use super::saddled_mounts::{drive, mount, spawn_beside};
use super::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{PlayerCommand, PlayerCommandAction};

const KINDS: [i32; 2] = [
    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
];

fn owned(world: &ServerWorld, nautilus: i32) -> bool {
    let owner = world.publication().players[0].uuid;
    let ints: Vec<i32> = owner
        .as_u128()
        .to_be_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&word| i32::from_be_bytes(word))
        .collect();
    record(world, nautilus).iter().any(|entry| {
        entry.name == "Owner"
            && entry.value == bbb_protocol::NbtSummaryValue::IntArray(ints.clone())
    })
}

pub(super) fn tame_with_buckets(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    nautilus: i32,
) {
    for _ in 0..64 {
        hold(world, "minecraft:pufferfish_bucket", 1);
        let fed = play(sender, receiver, world, 1, interact_entity_packet(nautilus));
        assert_eq!(
            fed.snapshot.players[0].inventory.slots[0],
            stack_of("minecraft:water_bucket", 1)
        );
        assert!(fed.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(status), .. }
                if status.entity_id == nautilus && matches!(status.event_id, 6 | 7))));
        if owned(world, nautilus) {
            return;
        }
    }
    panic!("repeated pufferfish feeding did not tame the nautilus");
}

#[test]
fn both_nautiluses_tame_equip_carry_a_rider_and_keep_their_owner_and_equipment() {
    for kind in KINDS {
        let (sender, receiver, mut world) = horse_world();
        let nautilus = spawn_beside(&mut world, kind);
        tame_with_buckets(&sender, &receiver, &mut world, nautilus);
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, nautilus)
            .unwrap()
            .living_mut()
            .unwrap()
            .set_health(10.0);
        hold(&mut world, "minecraft:cod_bucket", 1);
        let fed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(nautilus),
        );
        assert_eq!(
            fed.snapshot.players[0].inventory.slots[0],
            stack_of("minecraft:water_bucket", 1)
        );
        assert_eq!(animal_record_int(&world, nautilus, "InLove"), 0);
        // Cod buckets carry Foods.COD nutrition 2 (Items.java:1342-1345; Foods.java:13).
        assert_eq!(
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, nautilus)
                .unwrap()
                .living()
                .unwrap()
                .health(),
            14.0
        );
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, nautilus)
            .unwrap()
            .living_mut()
            .unwrap()
            .set_health(5.0);
        let mut custom_food = stack_of("minecraft:cod_bucket", 1);
        custom_food.component_patch = bbb_registry_codec::read_item_component(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            &bbb_protocol::NbtSummaryValue::Compound(vec![
                bbb_protocol::NbtSummaryEntry {
                    name: "nutrition".into(),
                    value: bbb_protocol::NbtSummaryValue::Int(3),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "saturation".into(),
                    value: bbb_protocol::NbtSummaryValue::Float(0.0_f32.to_bits()),
                },
            ]),
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .unwrap()
        .into();
        world.set_inventory_slot_for_test(ConnectionId(1), 0, custom_food);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(nautilus),
        );
        assert_eq!(
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, nautilus)
                .unwrap()
                .living()
                .unwrap()
                .health(),
            11.0
        );
        for resource in ["minecraft:saddle", "minecraft:iron_nautilus_armor"] {
            hold(&mut world, resource, 1);
            let equipped = play(
                &sender,
                &receiver,
                &mut world,
                1,
                interact_entity_packet(nautilus),
            );
            assert_eq!(
                equipped.snapshot.players[0].inventory.slots[0],
                ItemStackSummary::empty()
            );
            if resource == "minecraft:saddle" {
                assert!(played(
                    &equipped.events,
                    "minecraft:item.nautilus_saddle_equip"
                ));
            }
        }
        let saved = record(&world, nautilus);
        world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            nautilus,
            &saved,
        );
        assert!(owned(&world, nautilus));
        assert_eq!(
            worn(&world, nautilus, EquipmentSlot::Saddle),
            Some(item_id("minecraft:saddle"))
        );
        assert_eq!(
            worn(&world, nautilus, EquipmentSlot::Body),
            Some(item_id("minecraft:iron_nautilus_armor"))
        );
        hold(&mut world, "minecraft:air", 0);
        mount(&sender, &receiver, &mut world, nautilus);
        let (wanted, actual) = drive(&sender, &receiver, &mut world, nautilus);
        assert_eq!(
            actual, wanted,
            "the saddled nautilus accepts its rider's movement"
        );
        let rider = world.player_entity_id_for_test(ConnectionId(1));
        let opened = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerCommand(PlayerCommand {
                entity_id: rider,
                action: PlayerCommandAction::OpenInventory,
                data: 0,
            }),
        );
        let screen = mount_screen(&opened.events).expect("nautilus equipment screen");
        assert_eq!(screen.entity_id, nautilus);
        assert_eq!(screen.inventory_columns, 0);
        let content = opened
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::ContainerSetContent(content),
                    ..
                } if content.container_id == screen.container_id => Some(content.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(content.items.len(), 38);
        assert_eq!(content.items[0], stack_of("minecraft:saddle", 1));
        assert_eq!(
            content.items[1],
            stack_of("minecraft:iron_nautilus_armor", 1)
        );
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ContainerClick(ContainerClick {
                container_id: screen.container_id,
                state_id: content.state_id,
                slot_num: 1,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            }),
        );
        assert_eq!(worn(&world, nautilus, EquipmentSlot::Body), None);
        assert!(world.publication().players[0]
            .inventory
            .slots
            .contains(&stack_of("minecraft:iron_nautilus_armor", 1)));
        let state_id = world.player_inventory_for_test(ConnectionId(1)).state_id();
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ContainerClick(ContainerClick {
                container_id: screen.container_id,
                state_id,
                slot_num: 37,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            }),
        );
        assert_eq!(
            worn(&world, nautilus, EquipmentSlot::Body),
            Some(item_id("minecraft:iron_nautilus_armor"))
        );
        let saved = record(&world, nautilus);
        world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            nautilus,
            &saved,
        );
        assert_eq!(
            worn(&world, nautilus, EquipmentSlot::Body),
            Some(item_id("minecraft:iron_nautilus_armor"))
        );
    }
}

#[test]
fn creative_nautilus_feeding_keeps_the_fish_bucket_but_equipping_spends_the_saddle() {
    let (sender, receiver, mut world) = horse_world();
    console_command(&sender, &receiver, &mut world, 1, "gamemode creative alice");
    let nautilus = spawn_beside(&mut world, KINDS[0]);
    for _ in 0..64 {
        hold(&mut world, "minecraft:pufferfish_bucket", 1);
        let fed = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(nautilus),
        );
        assert_eq!(
            fed.snapshot.players[0].inventory.slots[0],
            stack_of("minecraft:pufferfish_bucket", 1)
        );
        if owned(&world, nautilus) {
            break;
        }
    }
    assert!(owned(&world, nautilus));
    assert_eq!(
        world.publication().players[0]
            .inventory
            .slots
            .iter()
            .filter(|stack| stack.item_id == Some(item_id("minecraft:water_bucket")))
            .map(|stack| stack.count)
            .sum::<i32>(),
        1
    );
    hold(&mut world, "minecraft:saddle", 2);
    let equipped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(nautilus),
    );
    assert_eq!(
        equipped.snapshot.players[0].inventory.slots[0],
        stack_of("minecraft:saddle", 1)
    );
    assert_eq!(
        worn(&world, nautilus, EquipmentSlot::Saddle),
        Some(item_id("minecraft:saddle"))
    );
}

fn load_young_age(world: &mut ServerWorld, nautilus: i32) {
    let mut saved = record(world, nautilus);
    saved
        .iter_mut()
        .find(|entry| entry.name == "Age")
        .unwrap()
        .value = bbb_protocol::NbtSummaryValue::Int(-24_000);
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, nautilus, &saved);
}

#[test]
fn a_baby_nautilus_eats_cod_buckets_to_grow_and_age_lock_stops_feeding() {
    let (sender, receiver, mut world) = horse_world();
    let nautilus = spawn_beside(&mut world, KINDS[0]);
    load_young_age(&mut world, nautilus);
    hold(&mut world, "minecraft:cod_bucket", 1);
    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(nautilus),
    );
    assert_eq!(
        fed.snapshot.players[0].inventory.slots[0],
        stack_of("minecraft:water_bucket", 1)
    );
    assert_eq!(animal_record_int(&world, nautilus, "ForcedAge"), 2_400);
    let age = animal_record_int(&world, nautilus, "Age");
    assert!((-21_600..-21_590).contains(&age));
    assert!(played(&fed.events, "minecraft:entity.baby_nautilus.eat"));
    assert!(!owned(&world, nautilus));
    let saved = record(&world, nautilus);
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, nautilus, &saved);
    assert_eq!(animal_record_int(&world, nautilus, "Age"), age);
    assert_eq!(animal_record_int(&world, nautilus, "ForcedAge"), 2_400);

    hold(&mut world, "minecraft:golden_dandelion", 1);
    let locked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(nautilus),
    );
    assert_eq!(
        locked.snapshot.players[0].inventory.slots[0],
        ItemStackSummary::empty()
    );
    assert_eq!(animal_record_int(&world, nautilus, "Age"), -24_000);
    hold(&mut world, "minecraft:cod_bucket", 1);
    let refused = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(nautilus),
    );
    assert_eq!(
        refused.snapshot.players[0].inventory.slots[0],
        stack_of("minecraft:cod_bucket", 1)
    );
    assert_eq!(animal_record_int(&world, nautilus, "Age"), -24_000);
    assert_eq!(animal_record_int(&world, nautilus, "ForcedAge"), 2_400);
    assert!(!played(
        &refused.events,
        "minecraft:entity.baby_nautilus.eat"
    ));
}

#[test]
fn a_zombie_nautilus_with_saved_negative_age_stays_adult_and_can_be_tamed_and_equipped() {
    let (sender, receiver, mut world) = horse_world();
    let nautilus = spawn_beside(&mut world, KINDS[1]);
    let adult_dimensions = {
        let entity = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, nautilus)
            .unwrap();
        (entity.bb_width(), entity.bb_height())
    };
    load_young_age(&mut world, nautilus);
    let entity = world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, nautilus)
        .unwrap();
    assert_eq!((entity.bb_width(), entity.bb_height()), adult_dimensions);
    entity.mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_BABY_DATA_ID);
    hold(&mut world, "minecraft:golden_dandelion", 1);
    let unchanged = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(nautilus),
    );
    assert_eq!(
        unchanged.snapshot.players[0].inventory.slots[0],
        stack_of("minecraft:golden_dandelion", 1)
    );
    assert!(!played(
        &unchanged.events,
        "minecraft:item.golden_dandelion.use"
    ));
    assert!(unchanged.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
            if data.id == nautilus && data.values.iter().any(|value|
                value.data_id == bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_BABY_DATA_ID
                    && value.value == bbb_protocol::packets::EntityDataValueKind::Boolean(true))
    )));
    assert_eq!(animal_record_int(&world, nautilus, "Age"), -24_000);
    tame_with_buckets(&sender, &receiver, &mut world, nautilus);
    hold(&mut world, "minecraft:saddle", 1);
    let equipped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(nautilus),
    );
    assert_eq!(
        equipped.snapshot.players[0].inventory.slots[0],
        ItemStackSummary::empty()
    );
    assert_eq!(
        worn(&world, nautilus, EquipmentSlot::Saddle),
        Some(item_id("minecraft:saddle"))
    );
    assert_eq!(animal_record_int(&world, nautilus, "Age"), -24_000);
}

#[test]
fn fed_nautiluses_court_but_only_the_living_pair_swims_together_and_has_a_baby() {
    for kind in KINDS {
        let (sender, receiver, mut world) = horse_world();
        let parents = [
            spawn_beside(&mut world, kind),
            spawn_beside(&mut world, kind),
        ];
        for parent in parents {
            tame_with_buckets(&sender, &receiver, &mut world, parent);
        }
        for parent in parents {
            hold(&mut world, "minecraft:saddle", 1);
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                interact_entity_packet(parent),
            );
        }
        let mut other_owner = record(&world, parents[1]);
        other_owner
            .iter_mut()
            .find(|entry| entry.name == "Owner")
            .unwrap()
            .value = bbb_protocol::NbtSummaryValue::IntArray(vec![1, 2, 3, 4]);
        world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            parents[1],
            &other_owner,
        );
        let center = world.publication().players[0].position;
        console_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "effect give alice minecraft:water_breathing 60",
        );
        let water = bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
        )
        .unwrap();
        for x in -5..=5 {
            for y in 0..=32 {
                for z in -3..=3 {
                    world.set_raw_block_state_for_test(
                        Java26BuiltinDimension::Overworld,
                        BlockPos {
                            x: center.x.floor() as i32 + x,
                            y: center.y.floor() as i32 + y,
                            z: center.z.floor() as i32 + z,
                        },
                        water,
                    );
                }
            }
        }
        for (index, parent) in parents.into_iter().enumerate() {
            world
                .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, parent)
                .unwrap()
                .set_pos(Vec3d {
                    x: center.x + if index == 0 { -2.5 } else { 2.5 },
                    ..center
                });
            hold(
                &mut world,
                if index == 0 {
                    "minecraft:cod_bucket"
                } else {
                    "minecraft:cod"
                },
                1,
            );
            let entity = world
                .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, parent)
                .unwrap();
            let max_health = entity.living().unwrap().max_health();
            crate::entity::living::set_health(entity, max_health);
            let fed = play(
                &sender,
                &receiver,
                &mut world,
                1,
                interact_entity_packet(parent),
            );
            assert_eq!(
                fed.snapshot.players[0].inventory.slots[0],
                if index == 0 {
                    stack_of("minecraft:water_bucket", 1)
                } else {
                    ItemStackSummary::empty()
                }
            );
            assert!(super::feeding::heart_events(&fed).contains(&parent));
            assert!(played(
                &fed.events,
                if kind == KINDS[0] {
                    "minecraft:entity.nautilus.eat"
                } else {
                    "minecraft:entity.zombie_nautilus.eat"
                }
            ));
            assert!(animal_record_int(&world, parent, "InLove") > 0);
            let saved = record(&world, parent);
            world.load_stored_entity_record_for_test(
                Java26BuiltinDimension::Overworld,
                parent,
                &saved,
            );
            assert!(animal_record_int(&world, parent, "InLove") > 0);
            hold(&mut world, "minecraft:cod", 1);
            let repeated = play(
                &sender,
                &receiver,
                &mut world,
                1,
                interact_entity_packet(parent),
            );
            assert_eq!(
                repeated.snapshot.players[0].inventory.slots[0],
                stack_of("minecraft:cod", 1)
            );
        }
        let distance = |world: &ServerWorld| {
            let a = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, parents[0])
                .unwrap()
                .position();
            let b = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, parents[1])
                .unwrap()
                .position();
            ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
        };
        assert!(distance(&world) > 3.0);
        let mut birth = None;
        for _ in 0..600 {
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
            let children = super::breed_goal::stored_animals(&world, kind);
            if let Some((child, _)) = children.into_iter().find(|(id, _)| !parents.contains(id)) {
                birth = Some((child, tick));
                break;
            }
        }
        if kind == KINDS[1] {
            assert!(birth.is_none(), "zombie nautiluses never produce offspring");
            assert_eq!(animal_record_int(&world, parents[0], "InLove"), 0);
            assert_eq!(animal_record_int(&world, parents[0], "Age"), 0);
            continue;
        }
        let (child, tick) = birth.unwrap_or_else(|| {
            panic!(
                "nautiluses failed to swim together; distance {}, parents {:?}",
                distance(&world),
                parents.map(|id| {
                    let entity = world
                        .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                        .unwrap();
                    (
                        entity.position(),
                        entity.delta_movement(),
                        entity.living().unwrap().speed(),
                        animal_record_int(&world, id, "InLove"),
                    )
                })
            )
        });
        assert!(distance(&world) < 3.0);
        assert_eq!(super::feeding::experience_orb_spawns(&tick), 1);
        assert!(animal_record_int(&world, child, "Age") <= -23_999);
        let creator = super::feeding::heart_events(&tick)
            .into_iter()
            .find(|id| parents.contains(id))
            .expect("creating parent's birth hearts");
        let child_record = record(&world, child);
        let creator_record = record(&world, creator);
        assert_eq!(
            child_record.iter().find(|entry| entry.name == "Owner"),
            creator_record.iter().find(|entry| entry.name == "Owner")
        );
        assert!(
            matches!(world.stored_entity_for_test(Java26BuiltinDimension::Overworld, child).unwrap().data(),
            crate::entity::EntityData::GeneralLiving(data) if data.is_tame())
        );
        assert_eq!(worn(&world, child, EquipmentSlot::Saddle), None);
        assert_eq!(worn(&world, child, EquipmentSlot::Body), None);
        for parent in parents {
            assert!(animal_record_int(&world, parent, "Age") > 5_990);
            assert_eq!(animal_record_int(&world, parent, "InLove"), 0);
        }
        for entity in [parents[0], parents[1], child] {
            let saved = record(&world, entity);
            world.load_stored_entity_record_for_test(
                Java26BuiltinDimension::Overworld,
                entity,
                &saved,
            );
            let reloaded = record(&world, entity);
            for key in ["Age", "InLove", "Owner", "equipment"] {
                assert_eq!(
                    reloaded.iter().find(|entry| entry.name == key),
                    saved.iter().find(|entry| entry.name == key),
                    "reloaded {key}"
                );
            }
            assert!(
                matches!(world.stored_entity_for_test(Java26BuiltinDimension::Overworld, entity).unwrap().data(),
                crate::entity::EntityData::GeneralLiving(data) if data.is_tame())
            );
        }
    }
}

fn saved_home(world: &ServerWorld, id: i32) -> Option<(Vec<i32>, i32)> {
    use bbb_protocol::NbtSummaryValue;
    let saved = record(world, id);
    let radius = saved.iter().find_map(|entry| match &entry.value {
        NbtSummaryValue::Int(value) if entry.name == "home_radius" => Some(*value),
        _ => None,
    })?;
    let position = saved
        .iter()
        .find_map(|entry| match &entry.value {
            NbtSummaryValue::IntArray(value) if entry.name == "home_pos" => Some(value.clone()),
            _ => None,
        })
        .expect("a saved home position");
    Some((position, radius))
}

#[test]
fn tame_nautilus_homes_follow_age_saddle_and_relocation_but_wait_for_leash_and_rider() {
    let dimension = Java26BuiltinDimension::Overworld;
    for kind in KINDS {
        let (sender, receiver, mut world) = horse_world();
        let id = spawn_beside(&mut world, kind);
        world.tick(Vec::new());
        assert_eq!(saved_home(&world, id), None);
        tame_with_buckets(&sender, &receiver, &mut world, id);
        world.tick(Vec::new());
        assert_eq!(saved_home(&world, id).unwrap().1, 32);
        if kind == KINDS[0] {
            load_young_age(&mut world, id);
            world.tick(Vec::new());
            assert_eq!(saved_home(&world, id).unwrap().1, 16);
            let mut saved = record(&world, id);
            saved
                .iter_mut()
                .find(|entry| entry.name == "Age")
                .unwrap()
                .value = bbb_protocol::NbtSummaryValue::Int(0);
            world.load_stored_entity_record_for_test(dimension, id, &saved);
            world.tick(Vec::new());
            assert_eq!(saved_home(&world, id).unwrap().1, 32);
        }
        hold(&mut world, "minecraft:saddle", 1);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert_eq!(
            worn(&world, id, EquipmentSlot::Saddle),
            Some(item_id("minecraft:saddle"))
        );
        assert_eq!(saved_home(&world, id).unwrap().1, 16);
        hold(&mut world, "minecraft:lead", 1);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert!(record(&world, id).iter().any(|entry| entry.name == "leash"));
        let mut saved = record(&world, id);
        saved
            .iter_mut()
            .find(|entry| entry.name == "home_radius")
            .unwrap()
            .value = bbb_protocol::NbtSummaryValue::Int(32);
        world.load_stored_entity_record_for_test(dimension, id, &saved);
        let leashed_home = saved_home(&world, id).unwrap();
        world.tick(Vec::new());
        assert_eq!(saved_home(&world, id), Some((leashed_home.0, 5)));
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert!(!record(&world, id).iter().any(|entry| entry.name == "leash"));
        world.tick(Vec::new());
        assert_eq!(saved_home(&world, id).unwrap().1, 16);

        let start = world
            .stored_entity_for_test(dimension, id)
            .unwrap()
            .position();
        let relocated = Vec3d {
            y: start.y + 50.0,
            ..start
        };
        world
            .stored_entity_mut_for_test(dimension, id)
            .unwrap()
            .set_pos(relocated);
        world.tick(Vec::new());
        let moved_home = saved_home(&world, id).unwrap();
        assert!((f64::from(moved_home.0[1]) - relocated.y).abs() < 2.0);
        let saved = record(&world, id);
        world.load_stored_entity_record_for_test(dimension, id, &saved);
        assert_eq!(saved_home(&world, id), Some(moved_home));

        world
            .stored_entity_mut_for_test(dimension, id)
            .unwrap()
            .set_pos(start);
        world.tick(Vec::new());
        hold(&mut world, "minecraft:air", 0);
        mount(&sender, &receiver, &mut world, id);
        let ridden_home = saved_home(&world, id).unwrap();
        world
            .stored_entity_mut_for_test(dimension, id)
            .unwrap()
            .set_pos(relocated);
        world.tick(Vec::new());
        assert_eq!(saved_home(&world, id), Some(ridden_home));
    }
}
