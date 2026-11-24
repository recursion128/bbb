use super::*;
use crate::connection::ConnectionId;
use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{tick_entities, EntityData, GeneralLivingData};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID,
};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::geometry::Vec3Ops;

const POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 80.0,
    z: 0.5,
};
const SPAWN_COOLDOWN: i32 = 100;

fn frog_store(no_ai: bool, uuid: u128, cooldown: Option<i32>, used_id: i32) -> (EntityStore, i32) {
    let ids = EntityIdCounter::new();
    for _ in 0..used_id {
        ids.next_id();
    }
    let mut store = EntityStore::new(1, ids);
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_FROG_ID).unwrap();
    data.set_no_ai(no_ai);
    let id = store
        .restore(
            uuid::Uuid::from_u128(uuid),
            VANILLA_ENTITY_TYPE_FROG_ID,
            POSITION,
            EntityData::GeneralLiving(data),
        )
        .unwrap();
    if let Some(cooldown) = cooldown {
        set_cooldown(&mut store, id, cooldown);
    }
    (store, id)
}

fn stage(blocks: impl IntoIterator<Item = (BlockPos, i32)>) -> NativeLevel {
    let mut native = NativeLevel::new();
    for (pos, state) in blocks {
        native.set(pos, state);
    }
    native
}

fn step(native: &mut NativeLevel, store: &mut EntityStore, ticking: bool) {
    native
        .world
        .set_game_time_for_test(native.world.game_time() + 1);
    native.with_level(|level| tick_entities(store, level, ticking));
}

fn food_player(native: &mut NativeLevel) -> i32 {
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "food", 41);
    native
        .world
        .set_player_position_for_test(ConnectionId(1), Vec3d { x: 4.5, ..POSITION });
    native.world.stock_menu_slot_for_test(
        ConnectionId(1),
        36,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SLIME_BALL_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    native.world.player_entity_id_for_test(ConnectionId(1))
}

fn tempted(native: &mut NativeLevel, store: &mut EntityStore, id: i32) {
    for _ in 0..=2 * crate::entity::ai::DEFAULT_SCAN_RATE {
        step(native, store, true);
        if memory(
            store.live_brain(id).unwrap(),
            MemoryModuleType::TemptingPlayer,
        )
        .is_some()
        {
            return;
        }
    }
    panic!("the frog did not follow the player's food");
}

fn set_cooldown(store: &mut EntityStore, entity_id: i32, cooldown: i32) {
    let entity = store.get_mut(entity_id).expect("the frog is stored");
    assert!(crate::entity::set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::LongJumpCooldownTicks,
            value: MemoryValue::Int(cooldown),
            time_to_live: None,
        },
    ));
}

fn memory(live: &Brain, memory_type: MemoryModuleType) -> Option<MemoryValue> {
    live.memory(memory_type).cloned()
}

fn merge_brain_pack(store: &mut EntityStore, entity_id: i32, packed: &BrainPacked) {
    let NbtSummaryValue::Compound(mut entries) =
        store.command_data(entity_id).expect("the record reads")
    else {
        panic!("an entity record is a compound")
    };
    let brain = entries
        .iter_mut()
        .find(|entry| entry.name == "Brain")
        .expect("the record carries a brain");
    brain.value = packed.encode();
    store
        .load_command_data(entity_id, &entries)
        .expect("the merged record loads");
}

fn cooldown_pack(cooldown: i32) -> BrainPacked {
    BrainPacked {
        memories: vec![PackedMemory {
            memory_type: MemoryModuleType::LongJumpCooldownTicks,
            value: MemoryValue::Int(cooldown),
            time_to_live: None,
        }],
    }
}

fn foot_hold(pos: Vec3d) -> (BlockPos, i32) {
    (
        BlockPos {
            x: pos.x.floor() as i32,
            y: pos.y.floor() as i32 - 1,
            z: pos.z.floor() as i32,
        },
        state("minecraft:stone"),
    )
}

fn invisible_food_slime(
    store: &mut EntityStore,
    uuid: u128,
    distance: f64,
    pieces: &[(bbb_protocol::equipment_slots::EquipmentSlot, &str)],
) -> i32 {
    let mut data =
        GeneralLivingData::new(VANILLA_ENTITY_TYPE_SLIME_ID).expect("the slime is living");
    data.set_no_ai(true);
    let id = store
        .restore(
            uuid::Uuid::from_u128(uuid),
            VANILLA_ENTITY_TYPE_SLIME_ID,
            Vec3d {
                x: POSITION.x + distance,
                ..POSITION
            },
            EntityData::GeneralLiving(data),
        )
        .expect("the slime restores");
    let entity = store.get_mut(id).expect("the slime is stored");
    assert!(crate::entity::slime::set_size(entity, 1, true));
    entity.set_invisible(true);
    let living = entity.living_mut().expect("the slime is living");
    for (slot, item) in pieces {
        living.set_item_slot(
            *slot,
            Some(crate::entity::item::plain_stack(item, 1).expect("a built-in 26.1 item")),
        );
    }
    id
}

#[test]
fn the_production_tick_updates_the_cooldown_the_frog_saves_and_lays_frogspawn() {
    let (mut store, id) = frog_store(false, 1, Some(SPAWN_COOLDOWN), 0);
    let mut native = stage([foot_hold(POSITION)]);
    step(&mut native, &mut store, true);
    step(&mut native, &mut store, true);
    assert_eq!(
        store.live_brain_pack(id).unwrap().memories,
        vec![PackedMemory {
            memory_type: MemoryModuleType::LongJumpCooldownTicks,
            value: MemoryValue::Int(SPAWN_COOLDOWN - 2),
            time_to_live: None
        }]
    );
    assert!(store.set_mob_brain_memory(
        id,
        None,
        MemoryModuleType::IsPregnant,
        Some(MemoryValue::Unit)
    ));
    let water = BlockPos { x: 0, y: 79, z: -1 };
    native.set(water, state("minecraft:water"));
    native.set(BlockPos { x: 1, y: 79, z: 0 }, state("minecraft:stone"));
    native.set(BlockPos { x: 1, y: 79, z: -1 }, state("minecraft:water"));
    let mut laid = false;
    for _ in 0..100 {
        step(&mut native, &mut store, true);
        laid = [0, 1].into_iter().any(|x| {
            native
                .world
                .block_state(DIMENSION, BlockPos { x, y: 80, z: -1 })
                == Some(state("minecraft:frogspawn"))
        });
        if laid {
            break;
        }
    }
    assert!(laid);
    assert!(!store
        .live_brain_pack(id)
        .unwrap()
        .memories
        .iter()
        .any(|memory| memory.memory_type == MemoryModuleType::IsPregnant));
}

#[test]
fn the_frogs_water_sensor_tracks_the_actual_pond_and_dry_ground() {
    let (mut store, id) = frog_store(false, 2, Some(SPAWN_COOLDOWN), 0);
    let mut native = NativeLevel::new();
    load_pond(&mut native);
    for x in -8..=8 {
        for z in -8..=8 {
            native.set(BlockPos { x, y: 79, z }, state("minecraft:stone"));
            for y in 80..=84 {
                native.set(BlockPos { x, y, z }, state("minecraft:water"));
            }
        }
    }
    for _ in 0..crate::entity::ai::DEFAULT_SCAN_RATE {
        step(&mut native, &mut store, true);
    }
    assert!(store.get(id).unwrap().is_in_water());
    assert_eq!(
        memory(store.live_brain(id).unwrap(), MemoryModuleType::IsInWater),
        Some(MemoryValue::Unit)
    );
    for x in -8..=8 {
        for z in -8..=8 {
            for y in 80..=84 {
                native.world.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    state("minecraft:air"),
                    2 | 16,
                );
            }
        }
    }
    for _ in 0..=2 * crate::entity::ai::DEFAULT_SCAN_RATE {
        step(&mut native, &mut store, true);
        if memory(store.live_brain(id).unwrap(), MemoryModuleType::IsInWater).is_none() {
            break;
        }
    }
    assert!(!store.get(id).unwrap().is_in_water());
    assert_eq!(
        memory(store.live_brain(id).unwrap(), MemoryModuleType::IsInWater),
        None
    );
}

#[test]
fn no_ai_and_a_frozen_tick_preserve_the_frogs_saved_cooldown() {
    for (no_ai, ticking) in [(true, true), (false, false)] {
        let (mut store, id) = frog_store(no_ai, 3, Some(SPAWN_COOLDOWN), 0);
        let mut native = stage([foot_hold(POSITION)]);
        for _ in 0..3 {
            step(&mut native, &mut store, ticking);
        }
        assert_eq!(
            crate::entity::brain_packed(store.get(id).unwrap())
                .unwrap()
                .memories,
            vec![PackedMemory {
                memory_type: MemoryModuleType::LongJumpCooldownTicks,
                value: MemoryValue::Int(SPAWN_COOLDOWN),
                time_to_live: None
            }]
        );
    }
}

#[test]
fn merging_a_legal_brain_pack_changes_what_the_next_tick_saves() {
    for replacement in [cooldown_pack(7), BrainPacked::empty()] {
        let (mut store, id) = frog_store(false, 6, Some(SPAWN_COOLDOWN), 0);
        let mut native = stage([foot_hold(POSITION)]);
        step(&mut native, &mut store, true);
        merge_brain_pack(&mut store, id, &replacement);
        step(&mut native, &mut store, true);
        let expected = if replacement.memories.is_empty() {
            Vec::new()
        } else {
            vec![PackedMemory {
                memory_type: MemoryModuleType::LongJumpCooldownTicks,
                value: MemoryValue::Int(6),
                time_to_live: None,
            }]
        };
        assert_eq!(store.live_brain_pack(id).unwrap().memories, expected);
    }
}

#[test]
fn a_frog_follows_a_real_players_food_and_stops_when_the_hand_empties() {
    let mut native = stage([foot_hold(POSITION), foot_hold(Vec3d { x: 4.5, ..POSITION })]);
    let player = food_player(&mut native);
    let (mut store, id) = frog_store(false, 8, Some(SPAWN_COOLDOWN), player);
    tempted(&mut native, &mut store, id);
    let live = store.live_brain(id).unwrap();
    assert_eq!(
        memory(live, MemoryModuleType::TemptingPlayer),
        Some(MemoryValue::EntityRef(player))
    );
    assert_eq!(
        memory(live, MemoryModuleType::IsTempted),
        Some(MemoryValue::Bool(true))
    );
    assert!(
        matches!(memory(live, MemoryModuleType::WalkTarget), Some(MemoryValue::WalkTarget(target))
        if matches!(target.target, crate::entity::ai::MemoryPositionTracker::Entity { entity_id, .. } if entity_id == player))
    );
    native
        .world
        .stock_menu_slot_for_test(ConnectionId(1), 36, ItemStackSummary::empty());
    for _ in 0..=2 * crate::entity::ai::DEFAULT_SCAN_RATE {
        step(&mut native, &mut store, true);
        if memory(
            store.live_brain(id).unwrap(),
            MemoryModuleType::TemptingPlayer,
        )
        .is_none()
        {
            break;
        }
    }
    let live = store.live_brain(id).unwrap();
    for kind in [
        MemoryModuleType::TemptingPlayer,
        MemoryModuleType::IsTempted,
        MemoryModuleType::LookTarget,
        MemoryModuleType::WalkTarget,
    ] {
        assert_eq!(memory(live, kind), None);
    }
}

#[test]
fn a_burning_frog_panics_towards_the_actual_water() {
    let (mut store, id) = frog_store(false, 10, Some(SPAWN_COOLDOWN), 0);
    let water = BlockPos { x: 1, y: 80, z: 0 };
    let mut native = stage([foot_hold(POSITION), (water, state("minecraft:water"))]);
    let entity = store.get_mut(id).unwrap();
    entity.set_remaining_fire_ticks(200);
    entity.living_mut().unwrap().last_damage_type = Some("minecraft:mob_attack".into());
    let mut target = None;
    for _ in 0..=2 * crate::entity::ai::DEFAULT_SCAN_RATE {
        step(&mut native, &mut store, true);
        target = memory(store.live_brain(id).unwrap(), MemoryModuleType::WalkTarget);
        if target.is_some() {
            break;
        }
    }
    assert!(matches!(target, Some(MemoryValue::WalkTarget(target))
        if target.target == crate::entity::ai::MemoryPositionTracker::BlockPos(water)
            && target.speed_modifier == 2.0 && target.close_enough_dist == 0));
    assert_eq!(
        native.world.block_state(DIMENSION, water),
        Some(state("minecraft:water"))
    );
}

#[test]
fn a_frog_long_jumps_across_the_gap_and_lands_on_the_actual_island() {
    let (mut store, id) = frog_store(false, 77, None, 0);
    let passenger = crate::entity::spawn_armor_stand(&mut store, POSITION).unwrap();
    let NbtSummaryValue::Compound(mut entries) = store.command_data(passenger).unwrap() else {
        panic!("the armor stand record is a compound")
    };
    for name in ["Small", "NoGravity", "Invulnerable"] {
        entries.retain(|entry| entry.name != name);
        entries.push(bbb_protocol::NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Byte(1),
        });
    }
    store.load_command_data(passenger, &entries).unwrap();
    let stand = store.get(passenger).unwrap();
    assert!(crate::entity::armor_stand::armor_stand(stand)
        .unwrap()
        .is_small());
    assert!(stand.no_gravity());
    assert!(stand.invulnerable());
    assert!(store.link_existing_passenger(id, passenger));
    let assert_passenger = |store: &EntityStore| {
        assert_eq!(store.passengers(id), [passenger]);
        assert_eq!(store.vehicle(passenger), Some(id));
        let body = store.get(id).unwrap();
        let stand = store.get(passenger).unwrap();
        let scale = body
            .living()
            .unwrap()
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
            as f32;
        assert_eq!(scale, 1.0);
        let point = bbb_protocol::entity_types::vanilla_entity_type_passenger_attachments(
            VANILLA_ENTITY_TYPE_FROG_ID,
        )
        .unwrap()[0];
        let point = point.map(|coordinate| coordinate * scale);
        let rider = bbb_protocol::entity_types::vanilla_entity_type_vehicle_attachment(
            stand.entity_type_id(),
        )
        .unwrap();
        assert_eq!(rider, [0.0; 3]);
        let (sin, cos) = (-f64::from(body.y_rot()).to_radians()).sin_cos();
        let x = f64::from(point[0]);
        let z = f64::from(point[2]);
        let expected = body.position().plus(Vec3d {
            x: x * cos + z * sin - f64::from(rider[0]),
            y: f64::from(point[1]) - f64::from(rider[1]),
            z: z * cos - x * sin - f64::from(rider[2]),
        });
        let actual = stand.position();
        for (actual, expected) in [
            (actual.x, expected.x),
            (actual.y, expected.y),
            (actual.z, expected.z),
        ] {
            assert!((actual - expected).abs() < 1.0e-6);
        }
    };
    let island = BlockPos { x: 0, y: 79, z: 4 };
    let mut native = stage([foot_hold(POSITION), (island, state("minecraft:stone"))]);
    let mut launched = false;
    for _ in 0..400 {
        step(&mut native, &mut store, true);
        let body = store.get(id).unwrap();
        if body.pose_id() == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_LONG_JUMPING_ID
            && body.delta_movement().y > 0.0
            && !body.on_ground()
        {
            assert_passenger(&store);
            launched = true;
            break;
        }
    }
    assert!(launched);
    let mut landed = false;
    for _ in 0..60 {
        step(&mut native, &mut store, true);
        assert_passenger(&store);
        let body = store.get(id).unwrap();
        // The jump angle is random, so the frog lands anywhere on the island.
        if body.on_ground() && (3.75..5.25).contains(&body.position().z) {
            assert_eq!(body.position().y, POSITION.y);
            landed = true;
            break;
        }
    }
    assert!(landed);
    for _ in 0..20 {
        step(&mut native, &mut store, true);
        if store.get(id).unwrap().pose_id()
            == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID
        {
            break;
        }
    }
    assert_eq!(
        store.get(id).unwrap().pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID
    );
    assert_passenger(&store);
}

#[test]
fn an_airborne_frog_falls_without_long_jumping() {
    let (mut store, id) = frog_store(false, 78, None, 0);
    let mut native = NativeLevel::new();
    for y in -64..=63 {
        native.world.set_block(
            DIMENSION,
            BlockPos { x: 0, y, z: 0 },
            state("minecraft:air"),
            2 | 16,
        );
    }
    for _ in 0..40 {
        step(&mut native, &mut store, true);
    }
    let body = store.get(id).unwrap();
    assert!(body.position().y < POSITION.y);
    assert_ne!(
        body.pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_LONG_JUMPING_ID
    );
}

#[test]
fn a_frog_reaches_an_invisible_slime_only_while_it_wears_armour() {
    use bbb_protocol::equipment_slots::EquipmentSlot;
    const FULL_ARMOUR: [(EquipmentSlot, &str); 4] = [
        (EquipmentSlot::Head, "minecraft:diamond_helmet"),
        (EquipmentSlot::Chest, "minecraft:diamond_chestplate"),
        (EquipmentSlot::Legs, "minecraft:diamond_leggings"),
        (EquipmentSlot::Feet, "minecraft:diamond_boots"),
    ];
    const DISTANCE: f64 = 5.0;
    for (uuid, pieces, reached) in [(30, &FULL_ARMOUR[..0], false), (31, &FULL_ARMOUR[..], true)] {
        let (mut store, frog) = frog_store(false, uuid, Some(SPAWN_COOLDOWN), 0);
        let slime = invisible_food_slime(&mut store, uuid + 100, DISTANCE, pieces);
        let mut native = stage((0..=DISTANCE as i32 + 1).map(|step| {
            foot_hold(Vec3d {
                x: POSITION.x + f64::from(step),
                ..POSITION
            })
        }));
        let mut attackable = None;
        for _ in 0..=2 * crate::entity::ai::DEFAULT_SCAN_RATE {
            step(&mut native, &mut store, true);
            attackable = memory(
                store.live_brain(frog).unwrap(),
                MemoryModuleType::NearestAttackable,
            );
            if attackable.is_some() {
                break;
            }
        }
        assert_eq!(attackable == Some(MemoryValue::EntityRef(slime)), reached);
    }
}

fn load_pond(native: &mut NativeLevel) {
    let (sender, receiver) = crate::tests::mailbox();
    sender
        .try_send(crate::world::WorldCommand::Admin {
            envelope: crate::admin::AdminEnvelope {
                operation_id: 1,
                request_id: "pond".into(),
                principal: "fixture".into(),
                scope: crate::admin::AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 0,
            },
            request: crate::admin::AdminRequest::ExecuteCommand {
                command: "forceload add -16 -16 16 16".into(),
            },
        })
        .unwrap();
    native.world.tick(receiver.drain());
}

fn tadpole_pond() -> NativeLevel {
    let mut native = NativeLevel::new();
    for x in -3..=11 {
        for z in -3..=4 {
            native.set(BlockPos { x, y: 79, z }, state("minecraft:stone"));
            for y in 80..=85 {
                native.set(BlockPos { x, y, z }, state("minecraft:water"));
            }
        }
    }
    native
}

fn tadpole_store(used_id: i32) -> (EntityStore, i32) {
    let ids = EntityIdCounter::new();
    for _ in 0..used_id {
        ids.next_id();
    }
    let mut store = EntityStore::new(43, ids);
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_TADPOLE_ID, POSITION)
        .unwrap();
    (store, id)
}

#[test]
fn a_tadpole_brain_drives_autonomous_water_travel() {
    let mut native = tadpole_pond();
    load_pond(&mut native);
    let (mut store, id) = tadpole_store(0);
    let mut swam = false;
    for _ in 0..180 {
        step(&mut native, &mut store, true);
        let position = store.get(id).unwrap().position();
        if (position.x - POSITION.x).hypot(position.z - POSITION.z) > 1.0 {
            swam = true;
            break;
        }
    }
    assert!(
        swam,
        "actual tadpole position: {:?}",
        store.get(id).unwrap().position()
    );
}

#[test]
fn a_tadpole_follows_a_players_food_over_a_submerged_wall() {
    let mut native = tadpole_pond();
    for y in 80..=81 {
        for z in -3..=4 {
            native.set(BlockPos { x: 3, y, z }, state("minecraft:stone"));
        }
    }
    let player = food_player(&mut native);
    native.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 8.5,
            y: 82.0,
            z: 0.5,
        },
    );
    let (mut store, id) = tadpole_store(player);
    let mut followed = false;
    for _ in 0..240 {
        step(&mut native, &mut store, true);
        let position = store.get(id).unwrap().position();
        if position.x > 5.0 && position.y >= 81.9 {
            followed = true;
            break;
        }
    }
    assert!(
        followed,
        "actual tadpole position: {:?}",
        store.get(id).unwrap().position()
    );
    assert_eq!(
        memory(
            store.live_brain(id).unwrap(),
            MemoryModuleType::TemptingPlayer
        ),
        Some(MemoryValue::EntityRef(player))
    );
    native
        .world
        .stock_menu_slot_for_test(ConnectionId(1), 36, ItemStackSummary::empty());
    for _ in 0..=2 * crate::entity::ai::DEFAULT_SCAN_RATE {
        step(&mut native, &mut store, true);
    }
    let live = store.live_brain(id).unwrap();
    assert_eq!(memory(live, MemoryModuleType::TemptingPlayer), None);
    assert_ne!(
        memory(live, MemoryModuleType::IsTempted),
        Some(MemoryValue::Bool(true))
    );
}
