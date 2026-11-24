use super::environment::{joined_level, unoccupied_ids};
use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

use crate::connection::ClientboundPlayPacket;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
    VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use crate::entity::hanging::spawn_leash_knot;
use crate::entity::leash::{self, LeashAttachment};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::tracking::ServerEntity;
use crate::entity::{spawn_boat, RemovalReason};

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(
        NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                entry(
                    "Entities",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
                ),
            ],
        ),
        0,
        0,
    )
    .expect("leash chunk document")
}

fn boat_position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

/// An oak boat record the writer produced itself, so only the `leash` field
/// under test is written by hand.
fn oak_boat_record() -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x0b_0a_71, EntityIdCounter::new());
    spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        boat_position(),
        0.0,
    )
    .expect("a fresh oak boat");
    write_entity_chunk(&store, (0, 0))
        .expect("boat record")
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

fn boat_leashed_to(attachment: NbtSummaryValue, ids: EntityIdCounter) -> (EntityStore, i32) {
    let mut entries = oak_boat_record();
    entries.push(entry("leash", attachment));
    let mut store = EntityStore::new(0x1ea5, ids);
    let id = read_entity_chunk(&mut store, &chunk(entries)).expect("a leashed boat record")[0];
    (store, id)
}

fn knot_at(x: i32, y: i32, z: i32) -> NbtSummaryValue {
    NbtSummaryValue::IntArray(vec![x, y, z])
}

fn resting_level() -> NativeLevel {
    NativeLevel::new()
}

fn leash_holder(store: &EntityStore, id: i32) -> Option<i32> {
    leash::leash_data(store.get(id).expect("entity is live"))
        .and_then(crate::entity::leash::LeashData::leash_holder)
}

fn knot_ids(store: &EntityStore) -> Vec<i32> {
    store
        .ids()
        .into_iter()
        .filter(|id| {
            matches!(
                store.get(*id).expect("entity is live").data(),
                EntityData::LeashKnot(_)
            )
        })
        .collect()
}

fn broadcast_links(events: &[EntityLifecycleEvent]) -> Vec<(i32, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Broadcast {
                packet: ClientboundPlayPacket::SetEntityLink(update),
                ..
            } => Some((update.source_id, update.dest_id)),
            _ => None,
        })
        .collect()
}

fn dropped_lead_positions(native: &NativeLevel) -> Vec<Vec3d> {
    native
        .world
        .stored_items_for_test(DIMENSION)
        .into_iter()
        .filter(|item| item.2 == "minecraft:lead")
        .map(|item| item.1)
        .collect()
}

#[test]
fn a_saved_knot_position_creates_its_knot_and_publishes_the_link() {
    let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), EntityIdCounter::new());
    assert_eq!(leash_holder(&store, boat), None);
    store.drain_lifecycle_events();

    resting_level().with_level(|ctx| tick_entities(&mut store, ctx, true));

    let knots = knot_ids(&store);
    assert_eq!(knots.len(), 1);
    assert_eq!(leash_holder(&store, boat), Some(knots[0]));
    assert_eq!(
        broadcast_links(&store.drain_lifecycle_events()),
        vec![(boat, knots[0])]
    );
}

#[test]
fn a_saved_knot_position_reuses_the_knot_already_there() {
    let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), EntityIdCounter::new());
    let knot = spawn_leash_knot(&mut store, BlockPos { x: 3, y: 64, z: 0 }).expect("a fresh knot");
    let other =
        spawn_leash_knot(&mut store, BlockPos { x: 9, y: 64, z: 0 }).expect("a second knot");

    resting_level().with_level(|ctx| tick_entities(&mut store, ctx, true));

    assert_eq!(leash_holder(&store, boat), Some(knot));
    let mut knots = knot_ids(&store);
    knots.sort_unstable();
    let mut expected = vec![knot, other];
    expected.sort_unstable();
    assert_eq!(knots, expected);
}

#[test]
fn a_resolved_knot_holder_writes_back_as_its_block_position() {
    let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), EntityIdCounter::new());
    resting_level().with_level(|ctx| tick_entities(&mut store, ctx, true));

    let written = write_entity_chunk(&store, (0, 0)).expect("the leashed boat saves");
    let record = written
        .entities()
        .find(|entity| {
            entity
                .raw()
                .iter()
                .any(|field| field.value == NbtSummaryValue::String("minecraft:oak_boat".into()))
        })
        .expect("the boat record");
    assert_eq!(
        record
            .raw()
            .iter()
            .find(|field| field.name == "leash")
            .map(|field| &field.value),
        Some(&knot_at(3, 64, 0))
    );
    let _ = boat;
}

#[test]
fn an_unresolvable_holder_uuid_drops_a_lead_after_a_hundred_ticks() {
    let (mut store, boat) = boat_leashed_to(
        NbtSummaryValue::Compound(vec![entry(
            "UUID",
            NbtSummaryValue::IntArray(vec![0, 0, 0, 9]),
        )]),
        EntityIdCounter::new(),
    );
    let mut level = resting_level();

    for _ in 0..crate::entity::leash::DELAYED_RESTORE_TICKS {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert!(dropped_lead_positions(&level).is_empty());
        assert_eq!(
            leash::leash_data(store.get(boat).expect("the boat is live"))
                .and_then(crate::entity::leash::LeashData::delayed_leash_info),
            Some(LeashAttachment::Holder(uuid::Uuid::from_u128(9)))
        );
    }

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert_eq!(dropped_lead_positions(&level).len(), 1);
    assert_eq!(
        leash::leash_data(store.get(boat).expect("the boat is live")),
        None
    );
}

#[test]
fn saved_holder_uuid_waits_for_visibility_then_keeps_its_held_identity_when_hidden() {
    let mut store = EntityStore::new_managed(
        crate::random::GameRandom::with_seed(0x1ea6),
        EntityIdCounter::new(),
    );
    let holder = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 1.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let uuid = store.get(holder).unwrap().uuid();
    let mut entries = oak_boat_record();
    entries.push(entry(
        "leash",
        NbtSummaryValue::Compound(vec![entry(
            "UUID",
            NbtSummaryValue::IntArray(
                uuid.as_bytes()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&bytes| i32::from_be_bytes(bytes))
                    .collect(),
            ),
        )]),
    ));
    let boat = read_entity_chunk(&mut store, &chunk(entries)).unwrap()[0];
    let mut level = resting_level();
    let tick = |store: &mut EntityStore, level: &mut NativeLevel| {
        let mut body = store.detach(boat).unwrap();
        assert!(level.with_level(|ctx| leash::tick_leash(&mut body, store, ctx)));
        store.attach(body);
    };
    store.drain_lifecycle_events();
    assert_eq!(store.entity_id_for_uuid(uuid), Some(holder));
    tick(&mut store, &mut level);
    assert_eq!(leash_holder(&store, boat), None);
    assert_eq!(
        leash::leash_data(store.get(boat).unwrap())
            .and_then(crate::entity::leash::LeashData::delayed_leash_info),
        Some(LeashAttachment::Holder(uuid)),
    );
    assert!(broadcast_links(&store.drain_lifecycle_events()).is_empty());

    store.apply_chunk_visibility((0, 0), crate::entity::EntityVisibility::Tracked);
    tick(&mut store, &mut level);
    assert_eq!(leash_holder(&store, boat), Some(holder));
    assert_eq!(
        broadcast_links(&store.drain_lifecycle_events()),
        [(boat, holder)],
    );

    store.apply_chunk_visibility((0, 0), crate::entity::EntityVisibility::Hidden);
    tick(&mut store, &mut level);
    assert_eq!(leash_holder(&store, boat), Some(holder));
    assert!(broadcast_links(&store.drain_lifecycle_events()).is_empty());
    assert!(dropped_lead_positions(&level).is_empty());
}

#[test]
fn a_holder_past_twelve_blocks_snaps_the_lead() {
    let (mut store, boat) = boat_leashed_to(knot_at(20, 64, 0), EntityIdCounter::new());
    let knot = spawn_leash_knot(&mut store, BlockPos { x: 20, y: 64, z: 0 }).expect("a fresh knot");
    let knot_position = store.get(knot).expect("the knot is live").position();
    let mut level = resting_level();

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    assert_eq!(leash_holder(&store, boat), None);
    assert_eq!(dropped_lead_positions(&level).len(), 1);
    let events = store.drain_lifecycle_events();
    let sounds: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Sound { sound } => Some(sound),
            _ => None,
        })
        .collect();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].position, knot_position);
    assert_eq!(
        sounds[0].source,
        bbb_protocol::packets::SoundSource::Neutral
    );
    // The pairing packet that clears the link follows the drop.
    assert_eq!(broadcast_links(&events).last(), Some(&(boat, 0)));
    assert!(knot_ids(&store).is_empty());
}

#[test]
fn a_holder_that_left_the_level_obeys_the_entity_drops_gamerule() {
    for entity_drops in [true, false] {
        let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), EntityIdCounter::new());
        let mut level = resting_level();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::console_command(
            &sender,
            &receiver,
            &mut level.world,
            1,
            &format!("gamerule minecraft:entity_drops {entity_drops}"),
        );
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let knot = leash_holder(&store, boat).expect("the knot resolved");
        store.remove(knot, RemovalReason::Discarded);

        level.with_level(|ctx| tick_entities(&mut store, ctx, true));

        assert_eq!(leash_holder(&store, boat), None);
        assert_eq!(
            dropped_lead_positions(&level).len(),
            usize::from(entity_drops)
        );
    }
}

/// `ServerEntity.sendPairingData` (`ServerEntity.java:311-313`).
#[test]
fn the_pairing_bundle_publishes_the_leashed_link() {
    let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), EntityIdCounter::new());
    resting_level().with_level(|ctx| tick_entities(&mut store, ctx, true));
    let knot = leash_holder(&store, boat).expect("the knot resolved");

    let entity = store.get(boat).expect("the boat is live");
    let packets = ServerEntity::new(entity, 1, true).send_pairing_data(entity, &[]);
    assert_eq!(
        packets.last(),
        Some(&ClientboundPlayPacket::SetEntityLink(
            bbb_protocol::packets::SetEntityLink {
                source_id: boat,
                dest_id: knot,
            }
        ))
    );

    let knot_entity = store.get(knot).expect("the knot is live");
    assert!(ServerEntity::new(knot_entity, 1, true)
        .send_pairing_data(knot_entity, &[])
        .iter()
        .all(|packet| !matches!(packet, ClientboundPlayPacket::SetEntityLink(_))));
}

#[test]
fn a_holder_outside_the_elastic_distance_pulls_the_boat_toward_it() {
    let (mut store, boat) = boat_leashed_to(knot_at(8, 64, 0), EntityIdCounter::new());
    let mut level = resting_level();

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    let entity = store.get(boat).expect("the boat is live");
    // The knot is at +x, so the spring displacement is toward +x and the
    // vertical component is damped by `AXIS_SPECIFIC_ELASTICITY`.
    assert!(entity.delta_movement().x > 0.0);
    assert!(entity.delta_movement().z < 0.0);
    let momentum = leash::leash_data(entity)
        .expect("the boat is still leashed")
        .angular_momentum();
    assert!(momentum != 0.0);
    assert!(leash_holder(&store, boat).is_some());
    assert!(dropped_lead_positions(&level).is_empty());
}

#[test]
fn a_holder_inside_the_elastic_distance_leaves_the_boat_alone() {
    let (mut store, boat) = boat_leashed_to(knot_at(2, 64, 0), EntityIdCounter::new());
    let (mut free_store, free_boat) = {
        let mut store = EntityStore::new(0x1ea5, EntityIdCounter::new());
        let id = read_entity_chunk(&mut store, &chunk(oak_boat_record()))
            .expect("an unleashed boat record")[0];
        (store, id)
    };
    let mut level = resting_level();
    let mut free_level = resting_level();

    for _ in 0..3 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        free_level.with_level(|ctx| tick_entities(&mut free_store, ctx, true));
    }

    let leashed = store.get(boat).expect("the boat is live");
    let free = free_store.get(free_boat).expect("the boat is live");
    assert_eq!(leashed.delta_movement(), free.delta_movement());
    assert_eq!(leashed.position(), free.position());
    assert_eq!(leashed.y_rot(), free.y_rot());
    assert!(leash_holder(&store, boat).is_some());
}

fn player_uuid_attachment() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![entry(
        "UUID",
        NbtSummaryValue::IntArray(vec![0, 0, 0, 1]),
    )])
}

fn lead_item_id() -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:lead")
        .expect("lead is a built-in 26.1 item")
        .id
}

fn store_lead_item_positions(store: &EntityStore) -> Vec<Vec3d> {
    store
        .ids()
        .into_iter()
        .filter_map(|id| {
            let entity = store.get(id).expect("entity is live");
            match entity.data() {
                EntityData::Item(_) => Some(entity.position()),
                _ => None,
            }
        })
        .collect()
}

fn sound_ids(events: &[EntityLifecycleEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            EntityLifecycleEvent::Sound { sound }
            | EntityLifecycleEvent::SoundExcept { sound, .. } => match sound.sound {
                bbb_protocol::packets::SoundEventHolder::Reference { registry_id } => {
                    Some(registry_id)
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn sound_registry_id(resource_id: &str) -> i32 {
    bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
        .expect("a built-in 26.1 sound")
}

fn interacting_player<'a>(
    native: &NativeLevel,
    hand_stack: &'a mut ItemStackSummary,
) -> leash::LeashInteractingPlayer<'a> {
    let player = &native.world.publication().players[0];
    leash::LeashInteractingPlayer {
        mob_item_used: false,
        hand_stack,
        entity_id: player.entity_id,
        uuid: player.uuid,
        position: player.position,
        secondary_use_active: false,
        infinite_materials: native
            .world
            .player_abilities_for_test(ConnectionId(1))
            .instabuild,
    }
}

fn lead_stack(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(lead_item_id()),
        count,
        component_patch: Default::default(),
    }
}

fn shears_stack() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID),
        count: 1,
        component_patch: Default::default(),
    }
}

fn bucket_stack() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID),
        count: 1,
        component_patch: Default::default(),
    }
}

fn spawn_mob(store: &mut EntityStore, entity_type_id: i32) -> i32 {
    store
        .spawn_general_living(entity_type_id, boat_position())
        .expect("a stored mob")
}

fn equip(store: &mut EntityStore, id: i32, slot: EquipmentSlot, stack: RuntimeItemStack) {
    store
        .get_mut(id)
        .expect("the mob is filed")
        .living_mut()
        .expect("the mob is living")
        .set_item_slot(slot, Some(stack));
}

fn equipped_item_id(store: &EntityStore, id: i32, slot: EquipmentSlot) -> Option<i32> {
    store
        .get(id)
        .and_then(crate::entity::Entity::living)
        .and_then(|living| living.item_by_slot(slot))
        .and_then(|stack| stack.item_id())
}

fn store_dropped_item_ids(store: &EntityStore) -> Vec<i32> {
    store
        .ids()
        .into_iter()
        .filter_map(|id| {
            let entity = store.get(id).expect("entity is live");
            match entity.data() {
                EntityData::Item(data) => data.item().item_id(),
                _ => None,
            }
        })
        .collect()
}

fn store_dropped_item_positions(store: &EntityStore) -> Vec<Vec3d> {
    store
        .ids()
        .into_iter()
        .filter_map(|id| {
            let entity = store.get(id).expect("entity is live");
            match entity.data() {
                EntityData::Item(_) => Some(entity.position()),
                _ => None,
            }
        })
        .collect()
}

fn enchanted_equipment(item: &str, enchantment: &str) -> RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string(item)),
            entry("count", NbtSummaryValue::Int(1)),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![entry(
                    "minecraft:enchantments",
                    NbtSummaryValue::Compound(vec![entry(enchantment, NbtSummaryValue::Int(1))]),
                )]),
            ),
        ]),
        &crate::entity::test_items::BuiltinItems,
    )
    .expect("an enchanted stack")
}

fn set_ageable_baby(store: &mut EntityStore, id: i32) {
    match store.get_mut(id).map(|entity| &mut entity.data) {
        Some(EntityData::GeneralLiving(data)) => {
            assert!(data.ageable_retained_mut().unwrap().set_age(-24_000));
        }
        _ => panic!("expected an ageable mob"),
    }
}

fn mob_persistence_required(store: &EntityStore, id: i32) -> bool {
    match store.get(id).map(crate::entity::Entity::data) {
        Some(EntityData::GeneralLiving(data)) => data
            .mob_retained()
            .is_some_and(|mob| mob.persistence_required()),
        _ => false,
    }
}

fn passenger_attachment_average(entity_type_id: i32) -> Vec3d {
    let points =
        bbb_protocol::entity_types::vanilla_entity_type_passenger_attachments(entity_type_id)
            .expect("the mob declares a passenger attachment");
    let n = f64::from(u32::try_from(points.len()).expect("attachment count"));
    Vec3d {
        x: points.iter().map(|point| f64::from(point[0])).sum::<f64>() / n,
        y: points.iter().map(|point| f64::from(point[1])).sum::<f64>() / n,
        z: points.iter().map(|point| f64::from(point[2])).sum::<f64>() / n,
    }
}

#[test]
fn a_saved_player_uuid_resolves_against_the_sessions_and_pulls() {
    let (player_world, _sender, _receiver) = joined_level();
    let player_entity_id = player_world
        .world
        .player_entity_id_for_test(ConnectionId(1));

    let (mut store, boat) =
        boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
    store.drain_lifecycle_events();
    let mut level = {
        let (mut native, _sender, _receiver) = joined_level();
        native.world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 0.5,
            },
        );
        native.world.tick(Vec::new());
        native
    };

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    assert_eq!(leash_holder(&store, boat), Some(player_entity_id));
    assert_eq!(
        broadcast_links(&store.drain_lifecycle_events()),
        vec![(boat, player_entity_id)]
    );
    let entity = store.get(boat).expect("the boat is live");
    assert!(entity.delta_movement().x > 0.0);

    let written = write_entity_chunk(&store, (0, 0)).expect("the leashed boat saves");
    assert_eq!(
        written
            .entities()
            .next()
            .unwrap()
            .raw()
            .iter()
            .find(|field| field.name == "leash")
            .map(|field| &field.value),
        Some(&player_uuid_attachment())
    );
}

#[test]
fn a_dead_or_spectating_player_holder_drops_the_lead() {
    let (player_world, sender, receiver) = joined_level();
    let player_entity_id = player_world
        .world
        .player_entity_id_for_test(ConnectionId(1));

    for make_dead in [true, false] {
        let (mut store, boat) =
            boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
        let mut level = {
            let (mut native, _sender, _receiver) = joined_level();
            native.world.set_player_position_for_test(
                ConnectionId(1),
                Vec3d {
                    x: 3.5,
                    y: 64.0,
                    z: 0.5,
                },
            );
            native
        };
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        assert_eq!(leash_holder(&store, boat), Some(player_entity_id));
        if make_dead {
            level.world.set_player_dead_for_test(ConnectionId(1));
        } else {
            crate::tests::console_command(
                &sender,
                &receiver,
                &mut level.world,
                1,
                "gamemode spectator alice",
            );
        }

        level.with_level(|ctx| tick_entities(&mut store, ctx, true));

        assert_eq!(leash_holder(&store, boat), None);
        assert_eq!(dropped_lead_positions(&level).len(), 1);
    }
}

#[test]
fn a_player_holder_in_another_level_keeps_the_leash_without_a_pull() {
    let (mut native, sender, receiver) = joined_level();
    let player = native.world.player_entity_id_for_test(ConnectionId(1));
    let (mut store, boat) = boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&native));
    native.with_level(|level| tick_entities(&mut store, level, true));
    assert_eq!(leash_holder(&store, boat), Some(player));
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        1,
        "execute in minecraft:the_nether run teleport alice 0.5 80 0.5",
    );
    assert!(native
        .with_level(|level| level.player_by_entity_id(player))
        .is_none());
    for _ in 0..3 {
        native.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert_eq!(leash_holder(&store, boat), Some(player));
    assert!(dropped_lead_positions(&native).is_empty());
    assert_eq!(store.get(boat).unwrap().delta_movement().x, 0.0);
}

#[test]
fn the_departed_session_sweep_drops_every_held_lead() {
    let (player_world, _sender, _receiver) = joined_level();
    let player_entity_id = player_world
        .world
        .player_entity_id_for_test(ConnectionId(1));

    let (mut store, boat) =
        boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
    let mut level = {
        let (mut native, _sender, _receiver) = joined_level();
        native.world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 3.5,
                y: 64.0,
                z: 0.5,
            },
        );
        native.world.tick(Vec::new());
        native
    };
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    store.drain_lifecycle_events();

    leash::drop_leashes_held_by_departed_player(&mut store, player_entity_id, true);

    assert_eq!(leash_holder(&store, boat), None);
    assert_eq!(store_lead_item_positions(&store).len(), 1);
    assert_eq!(
        broadcast_links(&store.drain_lifecycle_events()),
        vec![(boat, 0)]
    );

    let (mut removed_store, removed_boat) =
        boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
    level.with_level(|ctx| tick_entities(&mut removed_store, ctx, true));
    leash::drop_leashes_held_by_departed_player(&mut removed_store, player_entity_id, false);
    assert_eq!(leash_holder(&removed_store, removed_boat), None);
    assert!(store_lead_item_positions(&removed_store).is_empty());
}

#[test]
fn a_destroyed_boat_drops_its_lead_and_an_unloaded_one_keeps_it() {
    let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), EntityIdCounter::new());
    let mut level = resting_level();
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let knot = leash_holder(&store, boat).expect("the knot resolved");
    store.drain_lifecycle_events();

    store.remove(boat, RemovalReason::Killed);

    assert_eq!(store_lead_item_positions(&store).len(), 1);
    let events = store.drain_lifecycle_events();
    events
        .iter()
        .find(|event| {
            matches!(
                event,
                EntityLifecycleEvent::Broadcast {
                    packet: ClientboundPlayPacket::SetEntityLink(update),
                    ..
                } if update.source_id == boat && update.dest_id == 0
            )
        })
        .expect("the cleared link is published");
    events
        .iter()
        .find(|event| {
            matches!(event, EntityLifecycleEvent::Removed { entity, .. } if entity.id() == boat)
        })
        .expect("the removal edge is published");

    assert!(store.get(knot).is_none(), "the knot discards at once");

    let (mut unload_store, unload_boat) =
        boat_leashed_to(knot_at(3, 64, 0), EntityIdCounter::new());
    resting_level().with_level(|ctx| tick_entities(&mut unload_store, ctx, true));
    let unloaded = unload_store
        .remove(unload_boat, RemovalReason::UnloadedToChunk)
        .expect("the boat is handed back for the save");
    assert!(store_lead_item_positions(&unload_store).is_empty());
    assert!(leash::leash_data(&unloaded)
        .and_then(crate::entity::leash::LeashData::leash_holder)
        .is_some());
}

#[test]
fn a_lead_in_hand_attaches_the_boat_to_the_player() {
    let (player_world, _sender, _receiver) = joined_level();
    let player_entity_id = player_world
        .world
        .player_entity_id_for_test(ConnectionId(1));

    let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), unoccupied_ids(&player_world));
    resting_level().with_level(|ctx| tick_entities(&mut store, ctx, true));
    let knot = leash_holder(&store, boat).expect("the knot resolved");
    store.drain_lifecycle_events();
    let mut hand = lead_stack(2);

    let outcome = leash::interact_leash(
        &mut store,
        boat,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::AttachedToPlayer);
    assert_eq!(leash_holder(&store, boat), Some(player_entity_id));
    assert_eq!(hand.count, 1);
    // The knot's lead dropped before the new attachment.
    assert_eq!(store_lead_item_positions(&store).len(), 1);
    assert!(store.get(knot).is_none(), "the knot discards at once");
    let events = store.drain_lifecycle_events();
    assert_eq!(
        broadcast_links(&events),
        vec![(boat, 0), (boat, player_entity_id)]
    );
    assert_eq!(
        sound_ids(&events),
        vec![sound_registry_id("minecraft:item.lead.tied")]
    );
}

#[test]
fn a_lead_in_hand_does_not_steal_from_another_player() {
    let (mut player_world, sender, receiver) = joined_level();
    let player_entity_id = player_world
        .world
        .player_entity_id_for_test(ConnectionId(1));

    let (mut store, boat) =
        boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
    let mut level = {
        let (mut native, _sender, _receiver) = joined_level();
        native.world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 3.5,
                y: 64.0,
                z: 0.5,
            },
        );
        native.world.tick(Vec::new());
        native
    };
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    store.drain_lifecycle_events();
    let mut hand = lead_stack(1);
    let mut player = interacting_player(&player_world, &mut hand);

    crate::tests::join_loaded(&sender, &receiver, &mut player_world.world, 2, "bob", 2);
    let bob = player_world
        .world
        .publication()
        .players
        .iter()
        .find(|p| p.connection_id == ConnectionId(2))
        .unwrap()
        .clone();
    player.entity_id = bob.entity_id;
    player.uuid = bob.uuid;

    let outcome = leash::interact_leash(&mut store, boat, &mut player);

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(leash_holder(&store, boat), Some(player_entity_id));
    assert_eq!(hand.count, 1);
}

#[test]
fn the_holders_click_unleashes_by_the_game_mode() {
    let (mut player_world, sender, receiver) = joined_level();

    for infinite_materials in [false, true] {
        let (mut store, boat) =
            boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
        let mut level = {
            let (mut native, _sender, _receiver) = joined_level();
            native.world.set_player_position_for_test(
                ConnectionId(1),
                Vec3d {
                    x: 3.5,
                    y: 64.0,
                    z: 0.5,
                },
            );
            native
        };
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        store.drain_lifecycle_events();
        let mut hand = ItemStackSummary::empty();

        crate::tests::console_command(
            &sender,
            &receiver,
            &mut player_world.world,
            90,
            if infinite_materials {
                "gamemode creative alice"
            } else {
                "gamemode survival alice"
            },
        );
        let mut player = interacting_player(&player_world, &mut hand);

        let outcome = leash::interact_leash(&mut store, boat, &mut player);

        assert_eq!(outcome, leash::LeashInteraction::Unleashed);
        assert_eq!(leash_holder(&store, boat), None);
        assert_eq!(
            store_lead_item_positions(&store).len(),
            usize::from(!infinite_materials)
        );
        let events = store.drain_lifecycle_events();
        assert_eq!(broadcast_links(&events), vec![(boat, 0)]);
        assert_eq!(
            sound_ids(&events),
            vec![sound_registry_id("minecraft:item.lead.untied")]
        );
    }
}

#[test]
fn secondary_use_binds_the_held_boats_to_the_target_boat() {
    let (player_world, _sender, _receiver) = joined_level();

    let (mut store, held) =
        boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
    let mut level = {
        let (mut native, _sender, _receiver) = joined_level();
        native.world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 3.5,
                y: 64.0,
                z: 0.5,
            },
        );
        native.world.tick(Vec::new());
        native
    };
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let target = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
        0.0,
    )
    .expect("a target boat");
    store.drain_lifecycle_events();
    let mut hand = ItemStackSummary::empty();
    let mut player = interacting_player(&player_world, &mut hand);
    player.secondary_use_active = true;

    let outcome = leash::interact_leash(&mut store, target, &mut player);

    assert_eq!(outcome, leash::LeashInteraction::BoundHeld);
    assert_eq!(leash_holder(&store, held), Some(target));
    let events = store.drain_lifecycle_events();
    assert_eq!(broadcast_links(&events), vec![(held, target)]);
    assert_eq!(
        sound_ids(&events),
        vec![sound_registry_id("minecraft:item.lead.tied")]
    );
}

#[test]
fn shears_cut_a_boat_leashed_to_a_knot() {
    let (player_world, _sender, _receiver) = joined_level();

    let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), unoccupied_ids(&player_world));
    resting_level().with_level(|ctx| tick_entities(&mut store, ctx, true));
    let knot = leash_holder(&store, boat).expect("the knot resolved");
    store.drain_lifecycle_events();
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        boat,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Sheared);
    assert_eq!(leash_holder(&store, boat), None);
    assert_eq!(store_lead_item_positions(&store).len(), 1);
    let events = store.drain_lifecycle_events();
    assert_eq!(broadcast_links(&events), vec![(boat, 0)]);
    assert_eq!(
        sound_ids(&events),
        vec![sound_registry_id("minecraft:item.shears.snip")]
    );
    assert!(store.get(knot).is_none(), "the knot discards at once");
}

#[test]
fn shears_on_a_holder_drop_inbound_leashes() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5ea7, unoccupied_ids(&player_world));
    let holder = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 0.5,
        },
        0.0,
    )
    .expect("a holder boat");
    let inbound = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        boat_position(),
        0.0,
    )
    .expect("an inbound boat");
    {
        let mut entity = store.detach(inbound).expect("the inbound boat is live");
        assert!(leash::set_leashed_to(&mut entity, &mut store, holder, true));
        store.attach(entity);
    }
    store.drain_lifecycle_events();
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        holder,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Sheared);
    assert_eq!(leash_holder(&store, inbound), None);
    assert_eq!(store_lead_item_positions(&store).len(), 1);
    let events = store.drain_lifecycle_events();
    assert_eq!(broadcast_links(&events), vec![(inbound, 0)]);
    assert_eq!(
        sound_ids(&events),
        vec![sound_registry_id("minecraft:item.shears.snip")]
    );
}

#[test]
fn shears_on_a_mob_holder_cut_inbound_without_equipment_refuse() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5ea8, unoccupied_ids(&player_world));
    let pig = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, boat_position())
        .expect("a stored pig");
    let inbound = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
        0.0,
    )
    .expect("an inbound boat");
    {
        let mut entity = store.detach(inbound).expect("the inbound boat is live");
        assert!(leash::set_leashed_to(&mut entity, &mut store, pig, true));
        store.attach(entity);
    }
    store.drain_lifecycle_events();
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        pig,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Sheared);
    assert_eq!(leash_holder(&store, inbound), None);
    assert_eq!(store_lead_item_positions(&store).len(), 1);
}

#[test]
fn shears_on_an_unleashed_target_do_not_consume() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5ea9, unoccupied_ids(&player_world));
    let boat = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        boat_position(),
        0.0,
    )
    .expect("an unleashed boat");
    let mut boat_hand = shears_stack();
    assert_eq!(
        leash::interact_leash(
            &mut store,
            boat,
            &mut interacting_player(&player_world, &mut boat_hand)
        ),
        leash::LeashInteraction::Pass
    );
    assert_eq!(boat_hand.component_patch.damage, None);
    assert!(store_lead_item_positions(&store).is_empty());

    let pig = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_PIG_ID,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .expect("a stored pig");
    let mut pig_hand = shears_stack();
    assert_eq!(
        leash::interact_leash(
            &mut store,
            pig,
            &mut interacting_player(&player_world, &mut pig_hand)
        ),
        leash::LeashInteraction::Pass
    );
    assert_eq!(pig_hand.component_patch.damage, None);
    assert!(store_lead_item_positions(&store).is_empty());
}

#[test]
fn creative_shears_cut_without_durability() {
    let (mut player_world, sender, receiver) = joined_level();

    let (mut store, boat) = boat_leashed_to(knot_at(3, 64, 0), unoccupied_ids(&player_world));
    resting_level().with_level(|ctx| tick_entities(&mut store, ctx, true));
    store.drain_lifecycle_events();
    let mut hand = shears_stack();

    crate::tests::console_command(
        &sender,
        &receiver,
        &mut player_world.world,
        90,
        "gamemode creative alice",
    );
    let mut player = interacting_player(&player_world, &mut hand);

    let outcome = leash::interact_leash(&mut store, boat, &mut player);

    assert_eq!(outcome, leash::LeashInteraction::Sheared);
    assert_eq!(leash_holder(&store, boat), None);
    assert_eq!(store_lead_item_positions(&store).len(), 1);
    assert_eq!(hand.component_patch.damage, None);
    assert_eq!(hand.count, 1);
    assert_eq!(
        sound_ids(&store.drain_lifecycle_events()),
        vec![sound_registry_id("minecraft:item.shears.snip")]
    );
}

#[test]
fn shears_drop_a_pig_saddle_and_clear_the_slot() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5eaa, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    equip(
        &mut store,
        pig,
        EquipmentSlot::Saddle,
        crate::entity::item::plain_stack("minecraft:saddle", 1).expect("saddle"),
    );
    store.drain_lifecycle_events();
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        pig,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert!(matches!(
        outcome,
        leash::LeashInteraction::ShearedEquipment { .. }
    ));
    assert_eq!(equipped_item_id(&store, pig, EquipmentSlot::Saddle), None);
    assert_eq!(
        store_dropped_item_ids(&store),
        vec![bbb_protocol::item_types::VANILLA_ITEM_SADDLE_ID]
    );
    assert_eq!(
        store_dropped_item_positions(&store),
        vec![boat_position().plus(passenger_attachment_average(VANILLA_ENTITY_TYPE_PIG_ID))]
    );
    assert!(mob_persistence_required(&store, pig));
    assert_eq!(
        sound_ids(&store.drain_lifecycle_events()),
        vec![sound_registry_id("minecraft:item.saddle.unequip")]
    );
}

/// Wolf body armor is `can_be_sheared` (`Equippable` prototype).
#[test]
fn shears_drop_wolf_body_armor() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5eab, unoccupied_ids(&player_world));
    let wolf = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID);
    equip(
        &mut store,
        wolf,
        EquipmentSlot::Body,
        crate::entity::item::plain_stack("minecraft:wolf_armor", 1).expect("wolf armor"),
    );
    store.drain_lifecycle_events();
    let mut hand = shears_stack();

    // `Wolf.canShearEquipment` (`Wolf.java:440-441`): only the owner shears.
    let outcome = leash::interact_leash(
        &mut store,
        wolf,
        &mut interacting_player(&player_world, &mut hand),
    );
    assert!(!matches!(
        outcome,
        leash::LeashInteraction::ShearedEquipment { .. }
    ));
    assert_eq!(
        equipped_item_id(&store, wolf, EquipmentSlot::Body),
        Some(bbb_protocol::item_types::VANILLA_ITEM_WOLF_ARMOR_ID)
    );
    let owner = interacting_player(&player_world, &mut hand).uuid;
    let EntityData::GeneralLiving(data) = &mut store.get_mut(wolf).unwrap().data else {
        unreachable!()
    };
    data.tamable_retained_mut().unwrap().set_owner(Some(owner));
    store.drain_lifecycle_events();

    let outcome = leash::interact_leash(
        &mut store,
        wolf,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert!(matches!(
        outcome,
        leash::LeashInteraction::ShearedEquipment { .. }
    ));
    assert_eq!(equipped_item_id(&store, wolf, EquipmentSlot::Body), None);
    assert_eq!(
        store_dropped_item_ids(&store),
        vec![bbb_protocol::item_types::VANILLA_ITEM_WOLF_ARMOR_ID]
    );
    assert_eq!(
        sound_ids(&store.drain_lifecycle_events()),
        vec![sound_registry_id("minecraft:item.armor.unequip_wolf")]
    );
}

#[test]
fn shears_the_first_matching_slot_in_values_order() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5ead, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    equip(
        &mut store,
        pig,
        EquipmentSlot::Body,
        crate::entity::item::plain_stack("minecraft:wolf_armor", 1).expect("wolf armor"),
    );
    equip(
        &mut store,
        pig,
        EquipmentSlot::Saddle,
        crate::entity::item::plain_stack("minecraft:saddle", 1).expect("saddle"),
    );
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        pig,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert!(matches!(
        outcome,
        leash::LeashInteraction::ShearedEquipment { .. }
    ));
    assert_eq!(equipped_item_id(&store, pig, EquipmentSlot::Body), None);
    assert_eq!(
        equipped_item_id(&store, pig, EquipmentSlot::Saddle),
        Some(bbb_protocol::item_types::VANILLA_ITEM_SADDLE_ID)
    );
    assert_eq!(
        store_dropped_item_ids(&store),
        vec![bbb_protocol::item_types::VANILLA_ITEM_WOLF_ARMOR_ID]
    );
}

/// A creeper has no shearable equipment, so `attemptToShearEquipment` false
/// PASSes into `Creeper.mobInteract` (`PASS`).
#[test]
fn shears_on_a_creeper_pass_without_a_match() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5eae, unoccupied_ids(&player_world));
    let creeper = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_CREEPER_ID);
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        creeper,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(hand.component_patch.damage, None);
    assert!(store_dropped_item_ids(&store).is_empty());
}

#[test]
fn shears_on_an_adult_unsheared_sheep_drop_wool_and_damage_the_live_tool() {
    let (native, sheep, tick) = shear_sheep(false, false, 0);
    assert!(live_sheep_sheared(&native, sheep));
    let drops = native.world.stored_items_for_test(DIMENSION);
    assert!(!drops.is_empty());
    assert!(drops
        .iter()
        .all(|item| item.2 == "minecraft:white_wool" && item.3 == 1));
    assert!(tick.events.iter().any(|event|matches!(event,crate::world::WorldEvent::Send{packet:ClientboundPlayPacket::AddEntity(packet),..}|crate::world::WorldEvent::Broadcast{packet:ClientboundPlayPacket::AddEntity(packet),..} if packet.entity_type_id==bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID&&packet.position.y==65.0)));
    let player = &native.world.publication().players[0];
    let tool = &player.inventory.slots[usize::from(player.inventory.selected_slot)];
    assert_eq!(tool.component_patch.damage, Some(1));
    assert!(has_sound(&tick, "minecraft:entity.sheep.shear"));
}

#[test]
fn shears_on_an_already_sheared_or_baby_sheep_do_not_shear() {
    for baby in [false, true] {
        let (native, sheep, _) = shear_sheep(baby, !baby, 0);
        assert_eq!(live_sheep_sheared(&native, sheep), !baby);
        assert!(native.world.stored_items_for_test(DIMENSION).is_empty());
        let player = &native.world.publication().players[0];
        assert_eq!(
            player.inventory.slots[usize::from(player.inventory.selected_slot)]
                .component_patch
                .damage,
            None
        );
    }
}

#[test]
fn sheep_colour_selects_the_nested_wool_table() {
    let (native, sheep, tick) = shear_sheep(false, false, 14);
    assert!(live_sheep_sheared(&native, sheep));
    let drops = native.world.stored_items_for_test(DIMENSION);
    assert!(!drops.is_empty());
    assert!(drops.iter().all(|item| item.2 == "minecraft:red_wool"));
    assert!(has_sound(&tick, "minecraft:entity.sheep.shear"));
}

#[test]
fn live_mooshroom_loot_entity_reads_its_variant_component_and_is_not_a_sheep() {
    let mut store = EntityStore::new(0x5eba, EntityIdCounter::new());
    let mooshroom = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_MOOSHROOM_ID);
    let entity = crate::loot_runtime::LootEntity::Stored(crate::loot_runtime::StoredLootEntity {
        entity: store.get(mooshroom).expect("the mooshroom is live"),
        bodies: Some(crate::entity::store::EntityBodyReadView::new(
            &store, None, None,
        )),
        raid_at_position: None,
    });
    assert_eq!(
        entity
            .component(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
            )
            .unwrap(),
        Some(bbb_protocol::NbtSummaryValue::String("red".into()))
    );
    assert_eq!(entity.sheared(), None);
}

/// A pig saddle still wins `Entity.attemptToShearEquipment` before `mobInteract`.
#[test]
fn shears_on_a_saddled_pig_still_take_equipment_first() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5eb8, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    equip(
        &mut store,
        pig,
        EquipmentSlot::Saddle,
        crate::entity::item::plain_stack("minecraft:saddle", 1).expect("saddle"),
    );
    store.drain_lifecycle_events();
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        pig,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert!(matches!(
        outcome,
        leash::LeashInteraction::ShearedEquipment { .. }
    ));
    assert_eq!(equipped_item_id(&store, pig, EquipmentSlot::Saddle), None);
    assert_eq!(
        store_dropped_item_ids(&store),
        vec![bbb_protocol::item_types::VANILLA_ITEM_SADDLE_ID]
    );
}

#[test]
fn shears_on_a_vehicle_mob_pass_so_leash_may_proceed() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5eaf, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    let chicken = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    equip(
        &mut store,
        pig,
        EquipmentSlot::Saddle,
        crate::entity::item::plain_stack("minecraft:saddle", 1).expect("saddle"),
    );
    assert!(store.link_existing_passenger_forced(pig, chicken));
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        pig,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(
        equipped_item_id(&store, pig, EquipmentSlot::Saddle),
        Some(bbb_protocol::item_types::VANILLA_ITEM_SADDLE_ID)
    );
    assert_eq!(hand.component_patch.damage, None);
    assert!(store_dropped_item_ids(&store).is_empty());
}

#[test]
fn secondary_use_skips_equipment_shear() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5eb0, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    equip(
        &mut store,
        pig,
        EquipmentSlot::Saddle,
        crate::entity::item::plain_stack("minecraft:saddle", 1).expect("saddle"),
    );
    let mut hand = shears_stack();
    let mut player = interacting_player(&player_world, &mut hand);
    player.secondary_use_active = true;

    let outcome = leash::interact_leash(&mut store, pig, &mut player);

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(
        equipped_item_id(&store, pig, EquipmentSlot::Saddle),
        Some(bbb_protocol::item_types::VANILLA_ITEM_SADDLE_ID)
    );
}

#[test]
fn binding_curse_blocks_survival_equipment_shear() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5eb1, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    equip(
        &mut store,
        pig,
        EquipmentSlot::Saddle,
        enchanted_equipment("minecraft:saddle", "minecraft:binding_curse"),
    );
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        pig,
        &mut interacting_player(&player_world, &mut hand),
    );

    // The refused shear falls through to the saddled pig's ride arm
    // (`Pig.java:158-164`).
    assert_eq!(
        outcome,
        leash::LeashInteraction::Mounted {
            vehicle_entity_id: pig,
            rotation: None,
        }
    );
    assert_eq!(
        equipped_item_id(&store, pig, EquipmentSlot::Saddle),
        Some(bbb_protocol::item_types::VANILLA_ITEM_SADDLE_ID)
    );
    assert_eq!(hand.component_patch.damage, None);
    assert!(store_dropped_item_ids(&store).is_empty());
}

#[test]
fn creative_shears_binding_cursed_equipment() {
    let (mut player_world, sender, receiver) = joined_level();

    let mut store = EntityStore::new(0x5eb2, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    equip(
        &mut store,
        pig,
        EquipmentSlot::Saddle,
        enchanted_equipment("minecraft:saddle", "minecraft:binding_curse"),
    );
    let mut hand = shears_stack();

    crate::tests::console_command(
        &sender,
        &receiver,
        &mut player_world.world,
        90,
        "gamemode creative alice",
    );
    let mut player = interacting_player(&player_world, &mut hand);

    let outcome = leash::interact_leash(&mut store, pig, &mut player);

    assert!(matches!(
        outcome,
        leash::LeashInteraction::ShearedEquipment { .. }
    ));
    assert_eq!(equipped_item_id(&store, pig, EquipmentSlot::Saddle), None);
    assert_eq!(hand.component_patch.damage, None);
    assert_eq!(
        store_dropped_item_ids(&store),
        vec![bbb_protocol::item_types::VANILLA_ITEM_SADDLE_ID]
    );
}

/// Unenchanted shearable stacks always run; a known non-binding curse does too.
#[test]
fn unbreaking_does_not_block_equipment_shear() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0x5eb3, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    equip(
        &mut store,
        pig,
        EquipmentSlot::Saddle,
        enchanted_equipment("minecraft:saddle", "minecraft:unbreaking"),
    );
    let mut hand = shears_stack();

    let outcome = leash::interact_leash(
        &mut store,
        pig,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert!(matches!(
        outcome,
        leash::LeashInteraction::ShearedEquipment { .. }
    ));
    assert_eq!(equipped_item_id(&store, pig, EquipmentSlot::Saddle), None);
}

#[test]
fn bind_player_mobs_ties_the_held_boats_to_a_fence_knot() {
    let (player_world, _sender, _receiver) = joined_level();
    let player_entity_id = player_world
        .world
        .player_entity_id_for_test(ConnectionId(1));

    let (mut store, held) =
        boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
    let mut level = {
        let (mut native, _sender, _receiver) = joined_level();
        native.world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 3.5,
                y: 64.0,
                z: 0.5,
            },
        );
        native.world.tick(Vec::new());
        native
    };
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    store.drain_lifecycle_events();

    let outcome =
        leash::bind_player_mobs(&mut store, player_entity_id, BlockPos { x: 4, y: 64, z: 0 });

    assert_eq!(outcome, leash::LeadBindOutcome::Bound);
    let knots = knot_ids(&store);
    assert_eq!(knots.len(), 1);
    assert_eq!(leash_holder(&store, held), Some(knots[0]));
    let events = store.drain_lifecycle_events();
    assert_eq!(broadcast_links(&events), vec![(held, knots[0])]);
    assert_eq!(
        sound_ids(&events),
        vec![sound_registry_id("minecraft:item.lead.tied")]
    );

    let mut empty_store = EntityStore::new(0x1ea6, unoccupied_ids(&player_world));
    assert_eq!(
        leash::bind_player_mobs(
            &mut empty_store,
            player_entity_id,
            BlockPos { x: 4, y: 64, z: 0 }
        ),
        leash::LeadBindOutcome::Pass
    );
    assert!(knot_ids(&empty_store).is_empty());

    let (mut far_store, far_held) =
        boat_leashed_to(player_uuid_attachment(), unoccupied_ids(&player_world));
    level.with_level(|ctx| tick_entities(&mut far_store, ctx, true));
    assert_eq!(
        leash::bind_player_mobs(
            &mut far_store,
            player_entity_id,
            BlockPos { x: 15, y: 64, z: 0 }
        ),
        leash::LeadBindOutcome::Pass
    );
    assert!(knot_ids(&far_store).is_empty());
    assert_eq!(leash_holder(&far_store, far_held), Some(player_entity_id));
}

#[test]
fn a_happy_ghast_holder_publishes_its_leash_holder_flag() {
    let mut store = EntityStore::new(0x6087, EntityIdCounter::new());
    let ghast = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            Vec3d {
                x: 8.5,
                y: 70.0,
                z: 0.5,
            },
        )
        .expect("a stored happy ghast");
    let boat = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        boat_position(),
        0.0,
    )
    .expect("a fresh oak boat");
    {
        let mut entity = store.detach(boat).expect("the boat is live");
        assert!(leash::set_leashed_to(&mut entity, &mut store, ghast, false));
        store.attach(entity);
    }
    let mut level = resting_level();

    let flag = |store: &EntityStore| match store.get(ghast).expect("the ghast is live").data() {
        EntityData::GeneralLiving(data) => data
            .happy_ghast_is_leash_holder()
            .expect("the happy ghast retains the holder clock"),
        _ => unreachable!(),
    };
    // The ghast ticks before the boat's notify, so the flag turns on when the
    // decay next runs with a rewound clock.
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(flag(&store));
    // The pull itself ran too: the ghast is at +x beyond the elastic
    // distance, and both support the quad leash.
    assert!(
        store
            .get(boat)
            .expect("the boat is live")
            .delta_movement()
            .x
            > 0.0
    );

    {
        let mut entity = store.detach(boat).expect("the boat is live");
        assert!(leash::drop_leash_store_spawn(
            &mut entity,
            &mut store,
            false,
            false
        ));
        store.attach(entity);
    }
    for _ in 0..5 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    assert!(!flag(&store));
}

#[test]
fn a_rowed_leashed_boat_reads_the_controllers_known_movement() {
    let (player_world, _sender, _receiver) = joined_level();
    let player_entity_id = player_world
        .world
        .player_entity_id_for_test(ConnectionId(1));

    let (mut store, boat) = boat_leashed_to(knot_at(8, 64, 0), unoccupied_ids(&player_world));
    spawn_leash_knot(&mut store, BlockPos { x: 8, y: 64, z: 0 }).expect("a fresh knot");
    let (mut level, sender, receiver) = joined_level();
    level.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: -3.5,
            y: 64.0,
            z: 0.5,
        },
    );
    level.world.tick(Vec::new());
    let start = level.world.publication().players[0].position;
    sender
        .try_send(crate::world::WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: bbb_protocol::packets::PlayServerbound::MovePlayer(
                bbb_protocol::packets::MovePlayer {
                    position: Some(Vec3d {
                        x: start.x + 0.1,
                        ..start
                    }),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                },
            ),
        })
        .unwrap();
    level.world.tick(receiver.drain());

    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    store.place_test_player(player_entity_id, start);
    assert!(store.admit_external_player_to_boat(
        player_entity_id,
        boat,
        crate::entity::ExternalPlayerBoatAdmissionFacts {
            can_ride: crate::entity::ExternalPlayerCanRide::Allowed,
            secondary_use_active: false,
            eye_in_water: false,
        }
    ));

    let before = store.get(boat).expect("the boat is live").y_rot();
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));

    let entity = store.get(boat).expect("the boat is live");
    assert!(leash_holder(&store, boat).is_some());
    let momentum = leash::leash_data(entity)
        .expect("the boat is still leashed")
        .angular_momentum();
    assert!(momentum != 0.0);
    assert!(entity.y_rot() != before);
    assert_eq!(entity.delta_movement(), Vec3d::default());
}

#[test]
fn bucket_on_an_adult_cow_fills_milk() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0xc011, unoccupied_ids(&player_world));
    let cow = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    store.drain_lifecycle_events();
    let mut hand = bucket_stack();

    let outcome = leash::interact_leash(
        &mut store,
        cow,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Milked);
    assert_eq!(
        hand.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_MILK_BUCKET_ID)
    );
    assert_eq!(hand.count, 1);
    assert_eq!(
        sound_ids(&store.drain_lifecycle_events()),
        vec![sound_registry_id("minecraft:entity.cow.milk")]
    );
}

#[test]
fn creative_bucket_on_a_cow_keeps_the_bucket() {
    let (mut player_world, sender, receiver) = joined_level();

    let mut store = EntityStore::new(0xc012, unoccupied_ids(&player_world));
    let cow = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    store.drain_lifecycle_events();
    let mut hand = bucket_stack();

    crate::tests::console_command(
        &sender,
        &receiver,
        &mut player_world.world,
        90,
        "gamemode creative alice",
    );
    let mut player = interacting_player(&player_world, &mut hand);

    let outcome = leash::interact_leash(&mut store, cow, &mut player);

    assert_eq!(outcome, leash::LeashInteraction::Milked);
    assert_eq!(
        hand.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
    );
    assert_eq!(hand.count, 1);
    assert_eq!(
        sound_ids(&store.drain_lifecycle_events()),
        vec![sound_registry_id("minecraft:entity.cow.milk")]
    );
}

#[test]
fn bucket_on_a_baby_cow_passes() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0xc013, unoccupied_ids(&player_world));
    let cow = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    set_ageable_baby(&mut store, cow);
    let mut hand = bucket_stack();

    let outcome = leash::interact_leash(
        &mut store,
        cow,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(
        hand.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
    );
    assert!(sound_ids(&store.drain_lifecycle_events()).is_empty());
}

#[test]
fn bucket_on_a_pig_passes() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0xc014, unoccupied_ids(&player_world));
    let pig = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    let mut hand = bucket_stack();

    let outcome = leash::interact_leash(
        &mut store,
        pig,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(
        hand.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_BUCKET_ID)
    );
}

#[test]
fn bucket_on_an_adult_goat_fills_milk() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0xc016, unoccupied_ids(&player_world));
    let goat = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_GOAT_ID);
    store.drain_lifecycle_events();
    let mut hand = bucket_stack();

    let outcome = leash::interact_leash(
        &mut store,
        goat,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Milked);
    assert_eq!(
        hand.item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_MILK_BUCKET_ID)
    );
    assert_eq!(hand.count, 1);
    assert_eq!(
        sound_ids(&store.drain_lifecycle_events()),
        vec![sound_registry_id("minecraft:entity.goat.milk")]
    );
}

#[test]
fn bucket_on_a_screaming_goat_uses_the_screaming_milk_sound() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0xc018, unoccupied_ids(&player_world));
    let goat = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_GOAT_ID);
    assert!(crate::entity::goat::apply_spawn_traits(
        store.get_mut(goat).expect("goat"),
        true,
        crate::entity::goat::SpawnHornRemoval::None,
    ));
    store.drain_lifecycle_events();
    let mut hand = bucket_stack();

    let outcome = leash::interact_leash(
        &mut store,
        goat,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Milked);
    assert_eq!(
        sound_ids(&store.drain_lifecycle_events()),
        vec![sound_registry_id("minecraft:entity.goat.screaming.milk")]
    );
}

#[test]
fn non_bucket_on_an_adult_cow_passes() {
    let (player_world, _sender, _receiver) = joined_level();

    let mut store = EntityStore::new(0xc017, unoccupied_ids(&player_world));
    let cow = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    let mut hand = ItemStackSummary::empty();

    let outcome = leash::interact_leash(
        &mut store,
        cow,
        &mut interacting_player(&player_world, &mut hand),
    );

    assert_eq!(outcome, leash::LeashInteraction::Pass);
}

fn shear_sheep(
    baby: bool,
    sheared: bool,
    color: i8,
) -> (NativeLevel, i32, crate::world::TickResult) {
    let (mut native, sender, receiver) = joined_level();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        1,
        "item replace entity alice weapon.mainhand with minecraft:shears",
    );
    let (id, _) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_SHEEP_ID).unwrap(),
        ),
    );
    let body = native
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap();
    if baby {
        assert!(crate::entity::general_living::set_spawner_age(body, -100));
    }
    crate::entity::sheep::set_spawn_color(body, color);
    assert_eq!(crate::entity::sheep::color_of(body), Some(color));
    let EntityData::GeneralLiving(data) = &mut body.data else {
        panic!()
    };
    data.set_no_ai(true);
    data.sheep_retained_mut().unwrap().set_sheared(sheared);
    native
        .world
        .apply_level_entity_lifecycle_for_test(DIMENSION, &mut Vec::new());
    sender
        .try_send(crate::world::WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: bbb_protocol::packets::PlayServerbound::Interact(
                bbb_protocol::packets::InteractEntity {
                    entity_id: id,
                    hand: bbb_protocol::packets::InteractionHand::MainHand,
                    location: Vec3d::default(),
                    using_secondary_action: false,
                },
            ),
        })
        .unwrap();
    let tick = native.world.tick(receiver.drain());
    (native, id, tick)
}
fn live_sheep_sheared(native: &NativeLevel, id: i32) -> bool {
    native
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .is_some()
        && match native
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .data()
        {
            EntityData::GeneralLiving(data) => data.sheep_is_sheared() == Some(true),
            _ => false,
        }
}
fn has_sound(tick: &crate::world::TickResult, resource: &str) -> bool {
    let expected = bbb_protocol::packets::SoundEventHolder::Reference {
        registry_id: sound_registry_id(resource),
    };
    tick.events.iter().any(|event| {
        let packet = match event {
            crate::world::WorldEvent::Send { packet, .. }
            | crate::world::WorldEvent::Broadcast { packet, .. } => packet,
            _ => return false,
        };
        match packet {
            ClientboundPlayPacket::Sound(sound) => sound.sound == expected,
            ClientboundPlayPacket::SoundEntity(sound) => sound.sound == expected,
            _ => false,
        }
    })
}

#[test]
fn a_lead_attaches_a_riding_mob_and_dismounts_it() {
    let (player_world, _, _) = joined_level();
    let mut store = EntityStore::new(0x1ea5, unoccupied_ids(&player_world));
    let boat = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        boat_position(),
        0.0,
    )
    .unwrap();
    let cow = spawn_mob(&mut store, VANILLA_ENTITY_TYPE_COW_ID);
    assert!(store.link_existing_passenger_forced(boat, cow));
    store.drain_lifecycle_events();
    let mut hand = lead_stack(2);
    let result = leash::interact_leash(
        &mut store,
        cow,
        &mut interacting_player(&player_world, &mut hand),
    );
    let player = player_world
        .world
        .player_entity_id_for_test(ConnectionId(1));
    assert_eq!(result, leash::LeashInteraction::AttachedToPlayer);
    assert_eq!(leash_holder(&store, cow), Some(player));
    assert_eq!(store.vehicle(cow), None);
    assert!(store.passengers(boat).is_empty());
    let events = store.drain_lifecycle_events();
    assert_eq!(broadcast_links(&events), vec![(cow, player)]);
    assert!(events.iter().any(|event| matches!(event,
        crate::entity::store::EntityLifecycleEvent::PassengersChanged { vehicle_id, passenger_ids }
            if *vehicle_id == boat && passenger_ids.is_empty())));
}
