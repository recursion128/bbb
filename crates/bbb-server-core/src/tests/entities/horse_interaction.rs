//! A tamed horse eats, wears a saddle and armour, opens its inventory to its
//! rider, rears on a riding jump and breeds (`AbstractHorse.java:407-501`,
//! `:643-668`, `:892-901`, `Horse.java:177-222`).

use super::feeding::{interact_entity_packet, stack_of};
use super::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{PlayerCommand, PlayerCommandAction};

const HORSE: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID;
const DONKEY: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID;
const MULE: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MULE_ID;

pub(super) fn horse_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    (sender, receiver, world)
}

pub(super) fn record(world: &ServerWorld, entity: i32) -> Vec<bbb_protocol::NbtSummaryEntry> {
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, entity)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    entries
}

/// A tamed adult of `entity_type` `dx` blocks beside the session.
pub(super) fn tamed_equine(world: &mut ServerWorld, entity_type: i32, dx: f64) -> i32 {
    let position = world.publication().players[0].position;
    let (entity, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        entity_type,
        Vec3d {
            x: position.x + dx,
            ..position
        },
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(entity_type).unwrap()),
    );
    let mut entries = record(world, entity);
    entries.retain(|entry| entry.name != "Tame");
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "Tame".into(),
        value: bbb_protocol::NbtSummaryValue::Byte(1),
    });
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, entity, &entries);
    entity
}

pub(super) fn hold(world: &mut ServerWorld, resource_id: &str, count: i32) {
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of(resource_id, count));
}

pub(super) fn worn(world: &ServerWorld, entity: i32, slot: EquipmentSlot) -> Option<i32> {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity)
        .and_then(|entity| entity.living()?.item_by_slot(slot)?.item_id())
}

pub(super) fn item_id(resource_id: &str) -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
        .expect("a built-in item")
        .id
}

fn health(world: &ServerWorld, entity: i32) -> f32 {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity)
        .and_then(|entity| entity.living().map(|living| living.health()))
        .expect("a living entity")
}

pub(super) fn played(events: &[WorldEvent], resource_id: &str) -> bool {
    events.iter().any(|event| match event.sent_packet() {
        Some(ClientboundPlayPacket::Sound(sound)) => match sound.sound {
            SoundEventHolder::Reference { registry_id } => {
                vanilla_sound_event_resource_id(registry_id) == Some(resource_id)
            }
            SoundEventHolder::Direct { .. } => false,
        },
        _ => false,
    })
}

pub(super) fn mount_screen(
    events: &[WorldEvent],
) -> Option<bbb_protocol::packets::MountScreenOpen> {
    events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::MountScreenOpen(screen),
            ..
        } => Some(*screen),
        _ => None,
    })
}

fn player_command(world: &ServerWorld, action: PlayerCommandAction, data: i32) -> PlayServerbound {
    PlayServerbound::PlayerCommand(PlayerCommand {
        entity_id: world.player_entity_id_for_test(ConnectionId(1)),
        action,
        data,
    })
}

#[test]
fn a_tamed_horse_eats_wears_a_saddle_and_armour_and_keeps_them_across_a_reload() {
    let (sender, receiver, mut world) = horse_world();
    let horse = tamed_equine(&mut world, HORSE, 1.5);
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, horse)
        .and_then(|horse| horse.living_mut())
        .expect("a living horse")
        .set_health(5.0);

    // `handleEating`'s golden carrot: four health and the love timer
    // (`AbstractHorse.java:448-452`).
    hold(&mut world, "minecraft:golden_carrot", 2);
    let fed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(horse),
    );
    assert_eq!(
        fed.snapshot.players[0].inventory.slots[0],
        stack_of("minecraft:golden_carrot", 1)
    );
    assert_eq!(health(&world, horse), 9.0);
    assert!(played(&fed.events, "minecraft:entity.horse.eat"));
    assert!(super::feeding::animal_record_int(&world, horse, "InLove") > 0);
    assert!(world
        .game_events(Java26BuiltinDimension::Overworld)
        .0
        .iter()
        .any(|event| {
            event.game_event == VanillaGameEvent::EAT && event.source_entity_id == Some(horse)
        }));
    let mouth_open = |events: &[WorldEvent], open: bool| {
        events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
            if data.id == horse && data.values.iter().any(|value|
                value.data_id == bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID
                    && matches!(value.value, bbb_protocol::packets::EntityDataValueKind::Byte(flags)
                        if (flags & bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAG_OPEN_MOUTH != 0) == open))
    ))
    };
    let tracked = ticks_through_tracker_interval(&receiver, &mut world, fed);
    assert!(tracked.iter().any(|tick| mouth_open(&tick.events, true)));
    let mut closed = false;
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        closed |= mouth_open(&tick.events, false);
    }
    assert!(closed, "the feeding mouth animation clears on the tracker");

    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(horse),
    );
    hold(&mut world, "minecraft:iron_horse_armor", 1);
    let armoured = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(horse),
    );
    assert_eq!(
        armoured.snapshot.players[0].inventory.slots[0],
        ItemStackSummary::empty()
    );
    assert_eq!(
        worn(&world, horse, EquipmentSlot::Saddle),
        Some(item_id("minecraft:saddle"))
    );
    assert_eq!(
        worn(&world, horse, EquipmentSlot::Body),
        Some(item_id("minecraft:iron_horse_armor"))
    );

    // The saved record brings both back onto a fresh horse.
    let saved = record(&world, horse);
    let position = world.publication().players[0].position;
    let (reloaded, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        HORSE,
        position,
        crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(HORSE).unwrap()),
    );
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, reloaded, &saved);
    assert_eq!(
        worn(&world, reloaded, EquipmentSlot::Saddle),
        Some(item_id("minecraft:saddle"))
    );
    assert_eq!(
        worn(&world, reloaded, EquipmentSlot::Body),
        Some(item_id("minecraft:iron_horse_armor"))
    );
    assert_eq!(
        super::feeding::animal_record_int(&world, reloaded, "Temper"),
        { super::feeding::animal_record_int(&world, horse, "Temper") }
    );
}

#[test]
fn shift_clicking_a_tamed_horse_opens_its_inventory_and_takes_the_saddle_off() {
    let (sender, receiver, mut world) = horse_world();
    let horse = tamed_equine(&mut world, HORSE, 1.5);
    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(horse),
    );
    assert!(worn(&world, horse, EquipmentSlot::Saddle).is_some());

    let opened = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: horse,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: true,
        }),
    );
    let screen = mount_screen(&opened.events).expect("the horse's inventory opened");
    assert_eq!(screen.entity_id, horse);
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
        .expect("the menu sent its contents");
    // Saddle and body slots, then the 36 player slots.
    assert_eq!(content.items.len(), 38);
    assert_eq!(content.items[0], stack_of("minecraft:saddle", 1));

    // `quickMoveStack` sends the saddle to the player's slots
    // (`AbstractMountInventoryMenu.java:51-54`).
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ContainerClick(ContainerClick {
            container_id: screen.container_id,
            state_id: content.state_id,
            slot_num: 0,
            button_num: 0,
            input: ContainerInput::QuickMove,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        }),
    );
    assert_eq!(worn(&world, horse, EquipmentSlot::Saddle), None);
    let slots = &world.publication().players[0].inventory.slots;
    assert!(slots.contains(&stack_of("minecraft:saddle", 1)));
}

#[test]
fn a_rider_opens_the_saddled_horse_inventory_and_a_riding_jump_rears_it() {
    let (sender, receiver, mut world) = horse_world();
    let horse = tamed_equine(&mut world, HORSE, 1.5);
    hold(&mut world, "minecraft:saddle", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(horse),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(horse),
    );
    assert_eq!(
        world
            .external_player_relation_in_dimension_for_test(
                Java26BuiltinDimension::Overworld,
                world.player_entity_id_for_test(ConnectionId(1)),
            )
            .0,
        Some(horse),
        "the empty hand mounted the horse"
    );

    let command = player_command(&world, PlayerCommandAction::OpenInventory, 0);
    let opened = play(&sender, &receiver, &mut world, 1, command);
    assert_eq!(
        mount_screen(&opened.events).map(|screen| screen.entity_id),
        Some(horse)
    );
    assert_eq!(
        worn(&world, horse, EquipmentSlot::Saddle),
        Some(item_id("minecraft:saddle"))
    );
    let attachment_matches = |world: &ServerWorld, reared: bool| {
        let publication = world.publication();
        let player = publication
            .players
            .iter()
            .find(|player| player.connection_id == ConnectionId(1))
            .unwrap();
        assert_eq!(
            world.external_player_relation_in_dimension_for_test(
                Java26BuiltinDimension::Overworld,
                player.entity_id,
            ),
            (Some(horse), Some(horse))
        );
        let body = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, horse)
            .unwrap();
        assert!(body.is_alive());
        assert!(
            matches!(body.data(), crate::entity::EntityData::GeneralLiving(data)
            if !data.is_baby())
        );
        let scale = body
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
            as f32;
        let [x, y, z] =
            bbb_protocol::entity_types::vanilla_entity_type_passenger_attachments(HORSE).unwrap()
                [0]
            .map(|coordinate| coordinate * scale);
        let [rx, ry, rz] = bbb_protocol::entity_types::vanilla_entity_type_vehicle_attachment(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        )
        .unwrap();
        let (sin, cos) = (-f64::from(body.y_rot()).to_radians()).sin_cos();
        let (rider_sin, rider_cos) = (-f64::from(player.rotation.y_rot).to_radians()).sin_cos();
        let rise = if reared {
            f64::from(0.15_f32 * scale)
        } else {
            0.0
        };
        let forward = if reared {
            f64::from(-0.7_f32 * scale)
        } else {
            0.0
        };
        let expected = [
            f64::from(x) * cos + (f64::from(z) + forward) * sin
                - (f64::from(rx) * rider_cos + f64::from(rz) * rider_sin),
            f64::from(y) + rise - f64::from(ry),
            (f64::from(z) + forward) * cos
                - f64::from(x) * sin
                - (f64::from(rz) * rider_cos - f64::from(rx) * rider_sin),
        ];
        [
            player.position.x - body.position().x,
            player.position.y - body.position().y,
            player.position.z - body.position().z,
        ]
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() < 1.0e-5)
    };
    assert!(
        attachment_matches(&world, false),
        "the ordinary rider attachment"
    );

    let command = player_command(&world, PlayerCommandAction::StartRidingJump, 60);
    let jumped = play(&sender, &receiver, &mut world, 1, command);
    assert!(played(&jumped.events, "minecraft:entity.horse.jump"));
    let standing = record(&world, horse);
    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, horse)
            .is_some_and(
                |horse| matches!(horse.data(), crate::entity::EntityData::GeneralLiving(data)
                if data.abstract_horse_retained().is_some_and(|horse| horse.is_standing()))
            ),
        "the jump reared the horse: {standing:?}"
    );
    let mut fully_reared = false;
    for _ in 0..80 {
        world.tick(receiver.drain());
        if attachment_matches(&world, true) {
            fully_reared = true;
            break;
        }
    }
    assert!(
        fully_reared,
        "the actual rider reaches the fully reared attachment"
    );
    let mut restored = false;
    for _ in 0..120 {
        world.tick(receiver.drain());
        if attachment_matches(&world, false) {
            restored = true;
            break;
        }
    }
    assert!(
        restored,
        "the actual rider returns to the ordinary attachment"
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, horse)
        .is_some_and(
            |horse| matches!(horse.data(), crate::entity::EntityData::GeneralLiving(data)
                if data.abstract_horse_retained().is_some_and(|horse| !horse.is_standing()))
        ));
}

#[test]
fn a_fed_tamed_horse_and_donkey_breed_a_mule() {
    use bbb_protocol::builtin_enum_component_values::{
        builtin_enum_component_value_name, VANILLA_HORSE_VARIANT_BLACK_ID,
        VANILLA_HORSE_VARIANT_WHITE_ID,
    };
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID;

    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    for (suffix, coat) in [
        ("variant", VANILLA_HORSE_VARIANT_BLACK_ID),
        ("wrong_variant", VANILLA_HORSE_VARIANT_WHITE_ID),
    ] {
        let coat =
            builtin_enum_component_value_name(VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID, coat)
                .unwrap();
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: format!("test:breed_horse_{suffix}"),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(
                    &serde_json::json!({"criteria":{"bred":{
                        "trigger":"minecraft:bred_animals","conditions":{
                            "parent":[{"condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{"type":"minecraft:horse","components":{"minecraft:horse/variant":coat}}}],
                            "partner":[{"condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{"type":"minecraft:donkey"}}],
                            "child":[{"condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{"type":"minecraft:mule"}}]
                        }
                    }}}),
                ).unwrap(),
            },
        );
    }
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let horse = tamed_equine(&mut world, HORSE, 1.5);
    let donkey = tamed_equine(&mut world, DONKEY, -1.5);
    let mut entries = record(&world, horse);
    entries.retain(|entry| entry.name != "Variant");
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "Variant".into(),
        value: bbb_protocol::NbtSummaryValue::Int(VANILLA_HORSE_VARIANT_BLACK_ID),
    });
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, horse, &entries);
    assert!(record(&world, horse)
        .iter()
        .any(|entry| entry.name == "Variant"
            && entry.value == bbb_protocol::NbtSummaryValue::Int(VANILLA_HORSE_VARIANT_BLACK_ID)));
    for parent in [horse, donkey] {
        hold(&mut world, "minecraft:golden_carrot", 1);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(parent),
        );
    }
    let mules = |world: &ServerWorld| super::breed_goal::stored_animals(world, MULE).len();
    for _ in 0..200 {
        world.tick(receiver.drain());
        if mules(&world) > 0 {
            break;
        }
    }
    assert_eq!(mules(&world), 1, "the pair bred a mule");
    assert!(super::feeding::animal_record_int(&world, horse, "Age") > 0);
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:breed_horse_variant"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:breed_horse_wrong_variant"));
}

/// `ArmorSlot.mayPlace` reads the stack's own `equippable`
/// (`LivingEntity.isEquippableInSlot`, `LivingEntity.java:3821-3826`): a stick
/// made body-equippable goes onto the horse, one limited to llamas does not.
#[test]
fn the_body_slot_admits_by_the_stacks_own_equippable() {
    for (allowed, worn_after) in [("", true), (",allowed_entities:\"minecraft:llama\"", false)] {
        let (sender, receiver, mut world) = horse_world();
        let horse = tamed_equine(&mut world, HORSE, 1.5);
        let tick = crate::tests::entities::console_command(
            &sender,
            &receiver,
            &mut world,
            20,
            &format!("give alice stick[equippable={{slot:\"body\"{allowed}}}]"),
        );
        assert!(crate::tests::entities::console_command_answer(&tick)
            .0
            .is_some());
        let opened = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Interact(InteractEntity {
                entity_id: horse,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: true,
            }),
        );
        let screen = mount_screen(&opened.events).expect("the horse's inventory opened");
        let state_id = world.player_inventory_for_test(ConnectionId(1)).state_id();
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ContainerClick(ContainerClick {
                container_id: screen.container_id,
                state_id,
                slot_num: 2 + 27,
                button_num: 0,
                input: ContainerInput::QuickMove,
                changed_slots: BTreeMap::new(),
                carried_item: HashedStack::empty(),
            }),
        );
        assert_eq!(
            worn(&world, horse, EquipmentSlot::Body) == Some(item_id("minecraft:stick")),
            worn_after,
            "{allowed}"
        );
    }
}
