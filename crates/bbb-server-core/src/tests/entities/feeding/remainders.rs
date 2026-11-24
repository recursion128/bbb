use super::*;

fn with_remainder(mut food: ItemStackSummary) -> ItemStackSummary {
    let remainder = stack_of("minecraft:glass_bottle", 1);
    food.component_patch = DataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        ],
        use_cooldown_seconds_bits: Some(1.0_f32.to_bits()),
        use_remainder: Some(Box::new(bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: remainder.item_id.unwrap(),
            count: remainder.count,
            component_patch: remainder.component_patch,
        })),
        ..Default::default()
    }
    .into();
    food
}

struct Fixture {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    target: i32,
}

impl Fixture {
    fn new(entity: &str, game_type: GameType) -> Self {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = game_type;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "feeder", 1);
        let position = world.publication().players[0].position;
        let kind = bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(entity)
            .unwrap()
            .id;
        let mut data = GeneralLivingData::new(kind).unwrap();
        data.set_no_ai(true);
        let target = world
            .spawn_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                kind,
                Vec3d {
                    x: position.x + 1.0,
                    ..position
                },
                EntityData::GeneralLiving(data),
            )
            .0;
        Self {
            world,
            sender,
            receiver,
            target,
        }
    }

    fn feed(&mut self, hand: InteractionHand) -> TickResult {
        let mut packet = interact_entity_packet(self.target);
        let PlayServerbound::Interact(interact) = &mut packet else {
            unreachable!()
        };
        interact.hand = hand;
        play(&self.sender, &self.receiver, &mut self.world, 1, packet)
    }

    fn hold(&mut self, slot: usize, stack: ItemStackSummary) {
        self.world
            .set_inventory_slot_for_test(ConnectionId(1), slot, stack);
    }
}

#[test]
fn animal_use_remainders_replace_the_last_food_or_enter_inventory_and_drop_when_full() {
    for (count, hand, full, creative) in [
        (1, InteractionHand::MainHand, false, false),
        (2, InteractionHand::MainHand, false, false),
        (1, InteractionHand::OffHand, false, false),
        (2, InteractionHand::OffHand, true, false),
        (1, InteractionHand::MainHand, false, true),
    ] {
        let game_type = if creative {
            GameType::Creative
        } else {
            GameType::Survival
        };
        let mut fixture = Fixture::new("minecraft:cow", game_type);
        if full {
            for slot in 0..36 {
                fixture.hold(slot, stack_of("minecraft:stone", 64));
            }
        }
        let slot = if hand == InteractionHand::OffHand {
            40
        } else {
            0
        };
        let food = with_remainder(stack_of("minecraft:wheat", count));
        fixture.hold(slot, food.clone());
        let tick = fixture.feed(hand);
        assert!(!fixture
            .world
            .player_is_on_cooldown_for_test(ConnectionId(1), &food));
        let inventory = &tick.snapshot.players[0].inventory;
        let bottle = stack_of("minecraft:glass_bottle", 1);
        if creative {
            assert_eq!(inventory.slots[slot], food);
            assert!(!inventory
                .slots
                .iter()
                .any(|stack| stack.item_id == bottle.item_id));
        } else if count == 1 {
            assert_eq!(inventory.slots[slot], bottle);
        } else {
            let mut remaining = food;
            remaining.count = 1;
            assert_eq!(inventory.slots[slot], remaining);
            assert_eq!(
                inventory
                    .slots
                    .iter()
                    .filter(|stack| **stack == bottle)
                    .count(),
                usize::from(!full)
            );
        }
        let drops = fixture
            .world
            .stored_items_for_test(Java26BuiltinDimension::Overworld);
        assert_eq!(drops.len(), usize::from(full));
        if full {
            assert_eq!(drops[0].2, "minecraft:glass_bottle");
        }
        assert!(animal_record_int(&fixture.world, fixture.target, "InLove") > 0);
    }
}

#[test]
fn taming_and_parrot_poison_food_use_remainders_but_wolf_bones_do_not() {
    for (entity, food, remainder) in [
        ("minecraft:cat", "minecraft:cod", true),
        ("minecraft:parrot", "minecraft:wheat_seeds", true),
        ("minecraft:parrot", "minecraft:cookie", true),
        ("minecraft:nautilus", "minecraft:pufferfish", true),
        ("minecraft:wolf", "minecraft:bone", false),
    ] {
        let mut fixture = Fixture::new(entity, GameType::Survival);
        fixture.hold(0, with_remainder(stack_of(food, 1)));
        let tick = fixture.feed(InteractionHand::MainHand);
        let held = &tick.snapshot.players[0].inventory.slots[0];
        if remainder {
            assert_eq!(
                *held,
                stack_of("minecraft:glass_bottle", 1),
                "{entity}: {food}"
            );
        } else {
            assert!(held.item_id.is_none(), "{entity}: {food}");
        }
    }
}

#[test]
fn healing_a_tame_wolf_uses_its_food_remainder() {
    let mut fixture = Fixture::new("minecraft:wolf", GameType::Survival);
    let uuid = fixture.world.publication().players[0].uuid;
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = fixture
        .world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, fixture.target)
        .unwrap()
    else {
        panic!("entity record is a compound")
    };
    entries.retain(|entry| entry.name != "Owner" && entry.name != "Health");
    let uuid = uuid.as_u128();
    entries.extend([
        bbb_protocol::NbtSummaryEntry {
            name: "Owner".into(),
            value: bbb_protocol::NbtSummaryValue::IntArray(vec![
                (uuid >> 96) as i32,
                (uuid >> 64) as i32,
                (uuid >> 32) as i32,
                uuid as i32,
            ]),
        },
        bbb_protocol::NbtSummaryEntry {
            name: "Health".into(),
            value: bbb_protocol::NbtSummaryValue::Float(1.0_f32.to_bits()),
        },
    ]);
    fixture.world.load_stored_entity_record_for_test(
        Java26BuiltinDimension::Overworld,
        fixture.target,
        &entries,
    );
    fixture.hold(0, with_remainder(stack_of("minecraft:beef", 1)));
    let tick = fixture.feed(InteractionHand::MainHand);
    assert_eq!(
        tick.snapshot.players[0].inventory.slots[0],
        stack_of("minecraft:glass_bottle", 1)
    );
    assert!(
        fixture
            .world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, fixture.target)
            .unwrap()
            .living()
            .unwrap()
            .health()
            > 1.0
    );
}

#[test]
fn bucket_overrides_and_direct_food_consumption_do_not_add_custom_remainders() {
    for (entity, food, expected) in [
        (
            "minecraft:axolotl",
            "minecraft:tropical_fish_bucket",
            Some("minecraft:water_bucket"),
        ),
        (
            "minecraft:nautilus",
            "minecraft:tropical_fish_bucket",
            Some("minecraft:water_bucket"),
        ),
        ("minecraft:dolphin", "minecraft:cod", None),
        ("minecraft:horse", "minecraft:apple", None),
        ("minecraft:cow", "minecraft:golden_dandelion", None),
    ] {
        let mut fixture = Fixture::new(entity, GameType::Survival);
        set_animal_record_int(&mut fixture.world, fixture.target, "Age", -2_400);
        fixture.hold(0, with_remainder(stack_of(food, 1)));
        let tick = fixture.feed(InteractionHand::MainHand);
        assert_eq!(
            tick.snapshot.players[0].inventory.slots[0],
            expected.map_or_else(ItemStackSummary::empty, |item| stack_of(item, 1)),
            "{entity}: {food}"
        );
        assert!(!tick.snapshot.players[0]
            .inventory
            .slots
            .iter()
            .any(|stack| stack.item_id == stack_of("minecraft:glass_bottle", 1).item_id));
        assert!(fixture
            .world
            .stored_items_for_test(Java26BuiltinDimension::Overworld)
            .is_empty());
    }
}

#[test]
fn a_given_componentful_food_remainder_reaches_inventory_and_survives_player_save() {
    let mut fixture = Fixture::new("minecraft:cow", GameType::Survival);
    let given = console_command(
        &fixture.sender,
        &fixture.receiver,
        &mut fixture.world,
        1,
        r#"give feeder wheat[use_remainder={id:"minecraft:glass_bottle",count:3,components:{"minecraft:custom_data":{marker:7},"minecraft:custom_name":{text:"Feed bottle"}}}] 2"#,
    );
    assert_eq!(console_command_answer(&given).0, Some(1));
    let food = &given.snapshot.players[0].inventory.slots[0];
    assert_eq!(food.count, 2);
    let template = food.component_patch.use_remainder.as_ref().unwrap();
    let expected = ItemStackSummary {
        item_id: Some(template.item_id),
        count: template.count,
        component_patch: template.component_patch.clone(),
    };
    let custom_data =
        bbb_protocol::NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
            name: "marker".into(),
            value: bbb_protocol::NbtSummaryValue::Int(7),
        }]);
    assert_eq!(expected.count, 3);
    assert_eq!(
        expected.component_patch.custom_data.as_ref(),
        Some(&custom_data)
    );
    assert_eq!(
        expected.component_patch.custom_name.as_deref(),
        Some("Feed bottle")
    );
    let fed = fixture.feed(InteractionHand::MainHand);
    assert_eq!(fed.snapshot.players[0].inventory.slots[0].count, 1);
    assert_eq!(fed.snapshot.players[0].inventory.slots[1], expected);
    assert!(animal_record_int(&fixture.world, fixture.target, "InLove") > 0);

    let request = fixture
        .world
        .save_request(
            1_786_000_005_000,
            202,
            bbb_server_persistence::RegionCompression::Zlib,
        )
        .unwrap();
    let patch = &request.player_patches[0];
    let document = bbb_server_persistence::created_player_document(
        patch,
        request.item_stack_resolver.as_dyn(),
    )
    .unwrap();
    let parsed = bbb_server_persistence::Java26PlayerData::parse(document, patch.uuid).unwrap();
    let Some(bbb_protocol::NbtSummaryValue::List(inventory)) = parsed.inventory_input() else {
        panic!("saved inventory list");
    };
    let saved = inventory
        .iter()
        .find(|value| {
            bbb_protocol::nbt_summary::value_field(value, "Slot")
                == Some(&bbb_protocol::NbtSummaryValue::Byte(1))
        })
        .unwrap();
    let restored = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        saved,
        crate::world::registry_runtime::item_stack_resolver_for_test(),
    )
    .unwrap()
    .into_summary();
    assert_eq!(restored.count, expected.count);
    assert_eq!(restored.item_id, expected.item_id);
    assert_eq!(
        restored.component_patch.custom_data,
        expected.component_patch.custom_data
    );
    assert_eq!(
        restored.component_patch.custom_name_nbt,
        expected.component_patch.custom_name_nbt
    );
}
