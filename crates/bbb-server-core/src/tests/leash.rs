//! World wiring of `LeadItem.useOn`, `Entity.interact`'s leash branches,
//! the departed-session drop, and `AbstractBoat.remove`.

use super::*;

use crate::entity::{BoatData, EntityData, GeneralLivingData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
use bbb_protocol::item_types::VANILLA_ITEM_LEAD_ID;

fn oak_fence() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_fence",
        &BTreeMap::from([
            ("east".to_string(), "false".to_string()),
            ("north".to_string(), "false".to_string()),
            ("south".to_string(), "false".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
            ("west".to_string(), "false".to_string()),
        ]),
    )
    .expect("the default oak fence state exists")
}

fn lead_item(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_LEAD_ID),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn give_lead(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    count: i32,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: lead_item(count),
        }),
    );
}

fn use_on(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

fn interact_boat(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    boat: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: boat,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    )
}

fn spawn_oak_boat(world: &mut ServerWorld, position: Vec3d) -> i32 {
    let boat = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            position,
            EntityData::Boat(BoatData::new()),
        )
        .0;
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    boat
}

fn entity_links(tick: &TickResult) -> Vec<(i32, i32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityLink(update),
                ..
            } => Some((update.source_id, update.dest_id)),
            _ => None,
        })
        .collect()
}

fn swung_main_hand(tick: &TickResult) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityAnimation(update),
                ..
            } if update.action == 0
        )
    })
}

fn awarded_lead_used(tick: &TickResult) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AwardStats(update),
                ..
            } if update.stats.iter().any(|entry|
                entry.stat_type_id == bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID
                    && entry.value_id == VANILLA_ITEM_LEAD_ID
                    && entry.amount == 1)
        )
    })
}

fn request_stats(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    )
}

fn live_boat(world: &ServerWorld, boat: i32) -> &crate::entity::Entity {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .expect("the boat is live")
}

fn live_knots(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .is_some_and(|entity| {
                    matches!(entity.data(), EntityData::LeashKnot(_))
                        && entity.removal_reason().is_none()
                })
        })
        .collect()
}

fn dropped_lead_ids(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .is_some_and(|entity| match entity.data() {
                    EntityData::Item(item) => item.item().item_id() == Some(VANILLA_ITEM_LEAD_ID),
                    _ => false,
                })
        })
        .collect()
}

fn selected_lead_count(world: &ServerWorld) -> i32 {
    world.publication().players[0].inventory.slots[0].count
}

fn attach_nearby_boat(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> (i32, TickResult) {
    let player_position = world.publication().players[0].position;
    let boat = spawn_oak_boat(
        world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    give_lead(sender, receiver, world, 1);
    let attached = interact_boat(sender, receiver, world, boat);
    assert_eq!(
        crate::entity::leash::leash_holder(live_boat(world, boat)),
        Some(entity_id_of(world, 1))
    );
    assert!(
        entity_links(&attached).contains(&(boat, entity_id_of(world, 1))),
        "attach links: {:?}",
        entity_links(&attached)
    );
    (boat, attached)
}

fn clear_swing(world: &mut ServerWorld, receiver: &WorldCommandReceiver) {
    // `LivingEntity.getCurrentSwingDuration` is six ticks here.
    for _ in 0..6 {
        world.tick(receiver.drain());
    }
}

/// `LeadItem.useOn` (`LeadItem.java:23-35`) over `bindPlayerMobs` (`:37-65`).
#[test]
fn a_lead_on_a_fence_binds_held_boats_and_awards_item_used() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (boat, attached) = attach_nearby_boat(&sender, &receiver, &mut world);
    assert!(swung_main_hand(&attached), "SUCCESS_SERVER swings");
    clear_swing(&mut world, &receiver);
    let player_position = world.publication().players[0].position;
    let fence = BlockPos {
        x: player_position.x.floor() as i32,
        y: player_position.y.floor() as i32,
        z: player_position.z.floor() as i32 + 1,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        fence,
        oak_fence(),
        SET_BLOCK_UPDATE_ALL,
    ));
    // Sneak with a lead in hand skips `FenceBlock.useWithoutItem`
    // (`ServerPlayerGameMode.java:367-368`) so this click stays on `LeadItem.useOn`.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
    );

    let bound = use_on(&sender, &receiver, &mut world, fence, 1);
    let knots = live_knots(&world);
    assert_eq!(knots.len(), 1);
    assert_eq!(
        crate::entity::leash::leash_holder(live_boat(&world, boat)),
        Some(knots[0])
    );
    assert!(
        entity_links(&bound).contains(&(boat, knots[0])),
        "bind links: {:?}",
        entity_links(&bound)
    );
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(game_events.iter().any(|event| {
        event.game_event == VanillaGameEvent::BLOCK_ATTACH
            && event.pos == fence
            && event.source_entity_id == Some(entity_id_of(&world, 1))
    }));
    assert!(swung_main_hand(&bound));
    assert_eq!(selected_lead_count(&world), 1);
    assert!(awarded_lead_used(&request_stats(
        &sender, &receiver, &mut world
    )));
}

/// `:39-41` and `:60-62`: no held leashable is a plain pass; a fresh knot
/// every candidate is too far from is discarded.
#[test]
fn a_lead_on_a_fence_passes_and_discards_an_unused_knot() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let fence = BlockPos {
        x: player_position.x.floor() as i32,
        y: player_position.y.floor() as i32,
        z: player_position.z.floor() as i32 + 1,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        fence,
        oak_fence(),
        SET_BLOCK_UPDATE_ALL,
    ));
    give_lead(&sender, &receiver, &mut world, 1);

    let empty = use_on(&sender, &receiver, &mut world, fence, 1);
    assert!(live_knots(&world).is_empty());
    assert!(!swung_main_hand(&empty));
    assert!(!awarded_lead_used(&request_stats(
        &sender, &receiver, &mut world
    )));

    let (boat, _) = attach_nearby_boat(&sender, &receiver, &mut world);
    let boat_position = live_boat(&world, boat).position();
    world.set_stored_entity_position_for_test(
        Java26BuiltinDimension::Overworld,
        boat,
        Vec3d {
            x: boat_position.x + 13.0,
            ..boat_position
        },
    );

    let discarded = use_on(&sender, &receiver, &mut world, fence, 2);
    assert!(live_knots(&world).is_empty());
    // The fresh knot is added and discarded in the same use, so its
    // RemoveEntities follows its AddEntity (`LeadItem.java:60-62`).
    assert!(discarded.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::AddEntity(add),
            ..
        } if add.entity_type_id
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LEASH_KNOT_ID
    )));
    assert!(discarded.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::RemoveEntities(update),
            ..
        } if !update.entity_ids.is_empty()
    )));
    assert!(!swung_main_hand(&discarded));
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(!game_events
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_ATTACH));
}

/// `Entity.interact` (`Entity.java:2206-2238`): a lead attaches the boat, and
/// the holder's next click unleashes.
#[test]
fn interacting_with_a_boat_attaches_and_unleashes() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (boat, attached) = attach_nearby_boat(&sender, &receiver, &mut world);
    assert!(swung_main_hand(&attached), "SUCCESS_SERVER swings");
    // Creative `Player.interactOn` (`Player.java:836-838`) restores the shrink.
    assert_eq!(selected_lead_count(&world), 1);
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        None
    );

    let boat_before_unleash = live_boat(&world, boat).position();
    let unleashed = interact_boat(&sender, &receiver, &mut world, boat);
    assert_eq!(
        crate::entity::leash::leash_holder(live_boat(&world, boat)),
        None
    );
    assert!(
        entity_links(&unleashed).contains(&(boat, 0)),
        "unleash links: {:?}",
        entity_links(&unleashed)
    );
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(game_events.iter().any(|event| {
        event.game_event == VanillaGameEvent::ENTITY_INTERACT
            && event.source_entity_id == Some(entity_id_of(&world, 1))
    }));
    let exact_event = world
        .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|event| {
            event.recorded.game_event == VanillaGameEvent::ENTITY_INTERACT
                && event.recorded.source_entity_id == Some(entity_id_of(&world, 1))
        })
        .expect("unleashing raised an interaction event");
    assert_eq!(exact_event.position, boat_before_unleash);
    assert!(dropped_lead_ids(&world).is_empty());
}

/// The disconnect sweep drops every lead the leaver held
/// (`Leashable.java:148-153`).
#[test]
fn disconnecting_drops_leashes_the_leaver_held() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (boat, _) = attach_nearby_boat(&sender, &receiver, &mut world);

    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    let _ = world.tick(receiver.drain());

    assert_eq!(
        crate::entity::leash::leash_holder(live_boat(&world, boat)),
        None
    );
    assert_eq!(dropped_lead_ids(&world).len(), 1);
}

/// `AbstractBoat.remove` (`AbstractBoat.java:698-703`): a destroying removal
/// drops the lead; an unload keeps the leash for the save.
#[test]
fn destroying_a_leashed_boat_spawns_its_lead() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (boat, _) = attach_nearby_boat(&sender, &receiver, &mut world);

    assert!(world.remove_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        boat,
        crate::entity::RemovalReason::Killed,
    ));
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    assert_eq!(dropped_lead_ids(&world).len(), 1);

    let (boat, _) = attach_nearby_boat(&sender, &receiver, &mut world);
    assert!(world.remove_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        boat,
        crate::entity::RemovalReason::UnloadedToChunk,
    ));
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    assert_eq!(dropped_lead_ids(&world).len(), 1);
}

#[test]
fn iron_golem_repair_heals_consumes_and_syncs_on_the_packet_path() {
    use bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_IRON_INGOT_ID;
    use bbb_protocol::packets::EntityDataValueKind;

    let dimension = Java26BuiltinDimension::Overworld;
    let iron = |count| ItemStackSummary {
        item_id: Some(VANILLA_ITEM_IRON_INGOT_ID),
        count,
        component_patch: Default::default(),
    };
    let repair_sound = |tick: &TickResult| {
        tick.events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                    if matches!(&sound.sound, SoundEventHolder::Reference { registry_id }
                        if vanilla_sound_event_resource_id(*registry_id)
                            == Some("minecraft:entity.iron_golem.repair"))
            )
        })
    };
    for game_type in [GameType::Survival, GameType::Creative] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = game_type;
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "repairer", 1);
        let position = world.publication().players[0].position;
        let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID).unwrap();
        data.set_no_ai(true);
        let golem = world
            .spawn_stored_entity_for_test(
                dimension,
                VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
                Vec3d {
                    x: position.x + 1.0,
                    ..position
                },
                EntityData::GeneralLiving(data),
            )
            .0;
        let bbb_protocol::NbtSummaryValue::Compound(mut record) = world
            .stored_entity_command_data_for_test(dimension, golem)
            .unwrap()
        else {
            panic!("entity save is a compound")
        };
        record
            .iter_mut()
            .find(|entry| entry.name == "Health")
            .unwrap()
            .value = bbb_protocol::NbtSummaryValue::Float(60.0_f32.to_bits());
        world.load_stored_entity_record_for_test(dimension, golem, &record);
        world.apply_level_entity_lifecycle_for_test(dimension, &mut Vec::new());
        world.set_inventory_slot_for_test(ConnectionId(1), 0, iron(2));

        for (health, remaining) in [(85.0, 1), (100.0, 0)] {
            let repaired = interact_boat(&sender, &receiver, &mut world, golem);
            assert_eq!(live_boat(&world, golem).living().unwrap().health(), health);
            assert_eq!(
                repaired.snapshot.players[0].inventory.slots[0].count,
                if game_type == GameType::Creative {
                    2
                } else {
                    remaining
                }
            );
            assert!(repair_sound(&repaired));
            assert!(
                !swung_main_hand(&repaired),
                "SUCCESS leaves the swing to the client"
            );
            assert!(repaired.events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
                    if data.id == golem && data.values.iter().any(|value|
                        value.data_id == VANILLA_LIVING_ENTITY_HEALTH_DATA_ID
                            && value.value == EntityDataValueKind::Float(health))
            )));
            let (events, _) = world.game_events(dimension);
            assert!(events.iter().any(|event| event.game_event
                == VanillaGameEvent::ENTITY_INTERACT
                && event.source_entity_id == Some(entity_id_of(&world, 1))));
            let bbb_protocol::NbtSummaryValue::Compound(saved) = world
                .stored_entity_command_data_for_test(dimension, golem)
                .unwrap()
            else {
                panic!("entity save is a compound")
            };
            assert!(saved.iter().any(|entry| entry.name == "Health"
                && entry.value == bbb_protocol::NbtSummaryValue::Float(health.to_bits())));
            world.load_stored_entity_record_for_test(dimension, golem, &saved);
            assert_eq!(live_boat(&world, golem).living().unwrap().health(), health);
        }

        world.set_inventory_slot_for_test(ConnectionId(1), 0, iron(2));
        let interaction_count = |world: &ServerWorld| {
            world
                .game_events(dimension)
                .0
                .iter()
                .filter(|event| event.game_event == VanillaGameEvent::ENTITY_INTERACT)
                .count()
        };
        let interactions_before = interaction_count(&world);
        let full = interact_boat(&sender, &receiver, &mut world, golem);
        assert_eq!(full.snapshot.players[0].inventory.slots[0].count, 2);
        assert!(!repair_sound(&full));
        assert_eq!(interaction_count(&world), interactions_before);
    }
}

fn spawn_leash_mob(world: &mut ServerWorld, kind: i32, offset: f64) -> i32 {
    let position = world.publication().players[0].position;
    let mut data = GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    let id = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            kind,
            Vec3d {
                x: position.x + offset,
                ..position
            },
            EntityData::GeneralLiving(data),
        )
        .0;
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    id
}

#[test]
fn mob_lead_attachment_unleash_and_knot_transfer_use_real_packets() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    let dimension = Java26BuiltinDimension::Overworld;
    for game_type in [GameType::Survival, GameType::Creative] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = game_type;
        Arc::make_mut(config.datapacks.as_mut().unwrap())
            .advancements
            .advancements
            .push(bbb_server_datapack::advancement::AdvancementElement {
                id: "test:removed_knot".into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&serde_json::json!({
                    "criteria":{"interact":{
                        "trigger":"minecraft:player_interacted_with_entity",
                        "conditions":{
                            "entity":{"type":"minecraft:leash_knot"},
                            "item":{"items":"minecraft:stone","count":1}
                        }
                    }},
                    "rewards":{"experience":7}
                }))
                .unwrap(),
            });
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "leasher", 1);
        let cow = spawn_leash_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID, 1.0);
        let player = entity_id_of(&world, 1);
        world.set_inventory_slot_for_test(ConnectionId(1), 0, lead_item(3));
        let attached = interact_boat(&sender, &receiver, &mut world, cow);
        assert_eq!(
            crate::entity::leash::leash_holder(live_boat(&world, cow)),
            Some(player)
        );
        assert!(entity_links(&attached).contains(&(cow, player)));
        assert_eq!(
            selected_lead_count(&world),
            if game_type == GameType::Creative {
                3
            } else {
                2
            }
        );

        let unleashed = interact_boat(&sender, &receiver, &mut world, cow);
        assert_eq!(
            crate::entity::leash::leash_holder(live_boat(&world, cow)),
            None
        );
        assert!(entity_links(&unleashed).contains(&(cow, 0)));
        assert_eq!(
            dropped_lead_ids(&world).len(),
            usize::from(game_type == GameType::Survival)
        );
        interact_boat(&sender, &receiver, &mut world, cow);
        let position = world.publication().players[0].position;
        let fence = BlockPos {
            x: position.x.floor() as i32,
            y: position.y.floor() as i32,
            z: position.z.floor() as i32 + 1,
        };
        assert!(world.set_block(dimension, fence, oak_fence(), SET_BLOCK_UPDATE_ALL));
        let tied = use_on(&sender, &receiver, &mut world, fence, 1);
        let knot = live_knots(&world)[0];
        assert_eq!(
            crate::entity::leash::leash_holder(live_boat(&world, cow)),
            Some(knot)
        );
        assert!(entity_links(&tied).contains(&(cow, knot)));

        let bbb_protocol::NbtSummaryValue::Compound(saved) = world
            .stored_entity_command_data_for_test(dimension, cow)
            .unwrap()
        else {
            panic!("compound save")
        };
        assert!(saved.iter().any(|entry| entry.name == "leash"));
        world.load_stored_entity_record_for_test(dimension, cow, &saved);
        world.tick(Vec::new());
        assert_eq!(
            crate::entity::leash::leash_holder(live_boat(&world, cow)),
            Some(knot)
        );
        let other = spawn_leash_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID, 2.0);
        world.set_inventory_slot_for_test(ConnectionId(1), 0, lead_item(2));
        interact_boat(&sender, &receiver, &mut world, other);
        world.set_inventory_slot_for_test(ConnectionId(1), 0, ItemStackSummary::empty());
        let retied = interact_boat(&sender, &receiver, &mut world, knot);
        assert_eq!(
            crate::entity::leash::leash_holder(live_boat(&world, other)),
            Some(knot)
        );
        assert!(entity_links(&retied).contains(&(other, knot)));
        assert!(!world.player_advancement_is_done(ConnectionId(1), "test:removed_knot"));
        let stone = crate::entity::item::plain_stack("minecraft:stone", 1)
            .unwrap()
            .into_summary();
        world.set_inventory_slot_for_test(ConnectionId(1), 0, stone.clone());
        let transferred = interact_boat(&sender, &receiver, &mut world, knot);
        for id in [cow, other] {
            assert_eq!(
                crate::entity::leash::leash_holder(live_boat(&world, id)),
                Some(player)
            );
            assert!(entity_links(&transferred).contains(&(id, player)));
        }
        assert_eq!(
            world.publication().players[0].inventory.slots[0].item_id,
            stone.item_id
        );
        assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);
        assert!(live_knots(&world).is_empty());
        assert!(world.player_advancement_is_done(ConnectionId(1), "test:removed_knot"));
        assert_eq!(world.experience_total_for_test(ConnectionId(1)), 7);
        assert!(world
            .game_events(dimension)
            .0
            .iter()
            .any(|event| event.game_event == VanillaGameEvent::BLOCK_ATTACH
                && event.source_entity_id == Some(player)));
    }
}

#[test]
fn secondary_interaction_binds_held_mobs_to_an_adult_mob() {
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_PIG_ID};
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "leasher", 1);
    let cow = spawn_leash_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID, 1.0);
    let pig = spawn_leash_mob(&mut world, VANILLA_ENTITY_TYPE_PIG_ID, 2.0);
    world.set_inventory_slot_for_test(ConnectionId(1), 0, lead_item(2));
    interact_boat(&sender, &receiver, &mut world, cow);
    let bound = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: pig,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: true,
        }),
    );
    assert_eq!(
        crate::entity::leash::leash_holder(live_boat(&world, cow)),
        Some(pig)
    );
    assert!(entity_links(&bound).contains(&(cow, pig)));
    assert!(world
        .game_events(Java26BuiltinDimension::Overworld)
        .0
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::ENTITY_ACTION));
}

#[test]
fn invalid_mob_targets_do_not_accept_leads() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "leasher", 1);
    for kind in [
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_WOLF_ID,
    ] {
        let id = spawn_leash_mob(&mut world, kind, 1.0);
        if kind == VANILLA_ENTITY_TYPE_WOLF_ID {
            let bbb_protocol::NbtSummaryValue::Compound(mut saved) = world
                .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
            else {
                panic!("compound save")
            };
            saved.retain(|entry| entry.name != "anger_end_time");
            saved.push(bbb_protocol::NbtSummaryEntry {
                name: "anger_end_time".into(),
                value: bbb_protocol::NbtSummaryValue::Long(i64::MAX),
            });
            world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, id, &saved);
        }
        world.set_inventory_slot_for_test(ConnectionId(1), 0, lead_item(2));
        let refused = interact_boat(&sender, &receiver, &mut world, id);
        assert_eq!(
            crate::entity::leash::leash_holder(live_boat(&world, id)),
            None
        );
        assert!(!entity_links(&refused)
            .iter()
            .any(|(source, _)| *source == id));
        assert_eq!(selected_lead_count(&world), 2);
    }
}
