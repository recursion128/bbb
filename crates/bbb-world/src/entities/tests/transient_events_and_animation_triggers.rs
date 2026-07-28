use super::*;
use crate::entities::schema::vanilla_entity_metadata_type_is_hoglin_base;
use crate::entities::{
    VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID as COPPER_GOLEM_STATE_GETTING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID as COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_IDLE_ID as COPPER_GOLEM_STATE_IDLE_ID,
};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_DYING_ID, VANILLA_ENTITY_POSE_INHALING_ID,
    VANILLA_ENTITY_POSE_LONG_JUMPING_ID, VANILLA_ENTITY_POSE_SHOOTING_ID,
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_SLIDING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;

fn entity_pose_data(pose_id: i32) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert_eq!(accessors.pose_default, VANILLA_ENTITY_POSE_STANDING_ID);
    protocol_pose_data(accessors.pose_data_id, pose_id)
}

fn living_entity_using_item_data(entity_type_id: i32, using: bool) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .expect("generated inherited LivingEntity metadata accessors must resolve");
    assert_eq!(accessors.flags_default, 0);
    protocol_byte_data(
        accessors.flags_data_id,
        if using {
            accessors.is_using_mask
        } else {
            accessors.flags_default
        },
    )
}

fn entity_pose(store: &WorldStore, entity_id: i32) -> i32 {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    store
        .probe_entity(entity_id)
        .expect("entity state")
        .data_values
        .iter()
        .find(|value| value.data_id == accessors.pose_data_id)
        .map_or(accessors.pose_default, |value| match value.value {
            EntityDataValueKind::Pose(pose) => pose,
            ref value => panic!("unexpected pose metadata value {value:?}"),
        })
}

fn living_sleeping_pos_data(
    entity_type_id: i32,
    pos: Option<ProtocolBlockPos>,
) -> ProtocolEntityDataValue {
    let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .expect("generated inherited LivingEntity metadata accessors must resolve");
    ProtocolEntityDataValue {
        data_id: accessors.sleeping_pos_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
        value: EntityDataValueKind::OptionalBlockPos(pos),
    }
}

fn living_sleeping_pos(store: &WorldStore, entity_id: i32) -> Option<ProtocolBlockPos> {
    let entity_type_id = store.entity_type_id(entity_id).expect("entity type");
    let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .expect("generated inherited LivingEntity metadata accessors must resolve");
    store
        .probe_entity(entity_id)
        .expect("entity state")
        .data_values
        .iter()
        .find(|value| value.data_id == accessors.sleeping_pos_data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::OptionalBlockPos(pos) => pos,
            ref value => panic!("unexpected sleeping-position metadata value {value:?}"),
        })
}

fn sniffer_state_data(state_id: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SNIFFER_ID,
    )
    .expect("generated Sniffer state accessor must resolve");
    protocol_enum_data(data_id, EntityDataEnumSerializer::SnifferState, state_id)
}

fn copper_golem_state_data_id() -> u8 {
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    )
    .expect("generated CopperGolem metadata accessors")
    .state_data_id
}

fn allay_dancing_data(dancing: bool) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ALLAY_ID,
    )
    .expect("generated Allay dancing accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(dancing),
    }
}

fn axolotl_playing_dead_data(playing_dead: bool) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    )
    .expect("generated Axolotl playing-dead accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(playing_dead),
    }
}

fn axolotl_baby_data(is_baby: bool) -> ProtocolEntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_baby_accessor_for_entity_type(VANILLA_ENTITY_TYPE_AXOLOTL_ID)
            .expect("generated Axolotl baby accessor must resolve");
    assert!(!default);
    protocol_bool_data(data_id, is_baby)
}

fn camel_dash_data(dashing: bool) -> ProtocolEntityDataValue {
    let (data_id, _) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAMEL_ID)
            .expect("generated Camel dash accessor must resolve");
    ProtocolEntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(dashing),
    }
}

fn warden_anger_data(anger: i32) -> ProtocolEntityDataValue {
    let (data_id, _) = vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    )
    .expect("generated Warden client-anger accessor must resolve");
    protocol_int_data(data_id, anger)
}

#[test]
fn tracks_entity_transient_events() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 123, action: 3 }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 123,
        event_id: 35,
    }));
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 123, yaw: 45.5 }));

    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.last_animation_action, Some(3));
    assert_eq!(entity.last_event_id, Some(35));
    assert_eq!(entity.last_hurt_yaw, Some(45.5));
    assert_eq!(
        store.entities.transient_events(123).unwrap(),
        EntityTransientEvents {
            last_animation_action: Some(3),
            last_event_id: Some(35),
            last_hurt_yaw: Some(45.5),
        }
    );

    assert!(!store.apply_entity_animation(ProtocolEntityAnimation { id: 999, action: 4 }));
    assert!(!store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 999,
        event_id: 21,
    }));
    assert!(!store.apply_hurt_animation(ProtocolHurtAnimation { id: 999, yaw: 90.0 }));

    assert_eq!(store.counters().entity_animation_updates_received, 2);
    assert_eq!(store.counters().entity_animation_updates_applied, 1);
    assert_eq!(store.counters().entity_animation_updates_ignored, 1);
    assert_eq!(store.counters().entity_events_received, 2);
    assert_eq!(store.counters().entity_events_applied, 1);
    assert_eq!(store.counters().entity_events_ignored, 1);
    assert_eq!(store.counters().entity_hurt_animations_received, 2);
    assert_eq!(store.counters().entity_hurt_animations_applied, 1);
    assert_eq!(store.counters().entity_hurt_animations_ignored, 1);
}

#[test]
fn living_entity_swap_hand_event_updates_canonical_equipment() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        69,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ));
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 68,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: item_stack(42, 1),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::OffHand,
                item: item_stack(51, 2),
            },
        ],
    }));

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 68,
        event_id: 55,
    }));
    assert_eq!(
        store
            .equipment_item(68, EquipmentSlot::MainHand)
            .expect("swapped main hand"),
        item_stack(51, 2)
    );
    assert_eq!(
        store
            .equipment_item(68, EquipmentSlot::OffHand)
            .expect("swapped off hand"),
        item_stack(42, 1)
    );

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 68,
        event_id: 55,
    }));
    assert_eq!(
        store
            .equipment_item(68, EquipmentSlot::MainHand)
            .expect("restored main hand"),
        item_stack(42, 1)
    );
    assert_eq!(
        store
            .equipment_item(68, EquipmentSlot::OffHand)
            .expect("restored off hand"),
        item_stack(51, 2)
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 70,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: item_stack(63, 1),
        }],
    }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: 55,
    }));
    assert!(store.equipment_item(70, EquipmentSlot::MainHand).is_none());
    assert_eq!(
        store
            .equipment_item(70, EquipmentSlot::OffHand)
            .expect("absent off hand receives main-hand item"),
        item_stack(63, 1)
    );

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 69,
        event_id: 55,
    }));
    assert!(store.equipment_item(69, EquipmentSlot::MainHand).is_none());
    assert!(!store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 999,
        event_id: 55,
    }));
    assert_eq!(store.counters().entity_events_applied, 4);
    assert_eq!(store.counters().entity_events_ignored, 1);
}

#[test]
fn living_death_event_sets_non_player_health_and_starts_death_animation() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        69,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID,
    ));
    let alive_generation = store.publication_snapshot();

    let death_time = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .death_time
    };

    // Vanilla `LivingEntity.handleEntityEvent(3)` sets every non-Player living entity's health to
    // zero and calls `die(generic)`. The existing health-driven client counter begins on the next
    // tick, just like `LivingEntity.tickDeath`.
    assert_eq!(store.entities.living_entity_health(68), Some(1.0));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 68,
        event_id: 3,
    }));
    assert_eq!(store.entities.living_entity_health(68), Some(0.0));
    assert_eq!(store.entities.living_entity_dead(68), Some(true));
    assert!(store.probe_entity(68).unwrap().living_dead);
    assert_eq!(entity_pose(&store, 68), VANILLA_ENTITY_POSE_DYING_ID);
    assert_eq!(death_time(&store, 68), 0.0);
    store.advance_entity_client_animations(1);
    assert_eq!(death_time(&store, 68), 1.0);
    assert_eq!(
        alive_generation.entities.living_entity_dead(68),
        Some(false),
        "retained publications keep the pre-event lifecycle generation"
    );
    assert_eq!(
        entity_pose(&alive_generation, 68),
        VANILLA_ENTITY_POSE_STANDING_ID
    );

    let encoded = serde_json::to_vec(&store).unwrap();
    let restored: WorldStore = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored.entities.living_entity_dead(68), Some(true));
    assert!(restored.probe_entity(68).unwrap().living_dead);
    assert_eq!(entity_pose(&restored, 68), VANILLA_ENTITY_POSE_DYING_ID);

    // The sound half of event 3 still applies to a Player, but vanilla deliberately skips its
    // health/death mutation. Non-living event-3 owners such as Snowball are unaffected too.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 69,
        event_id: 3,
    }));
    assert_eq!(store.entities.living_entity_health(69), Some(1.0));
    assert_eq!(store.entities.living_entity_dead(69), Some(false));
    assert_eq!(entity_pose(&store, 69), VANILLA_ENTITY_POSE_STANDING_ID);
    assert_eq!(death_time(&store, 69), 0.0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: 3,
    }));
    assert_eq!(store.entities.living_entity_health(70), None);
    assert_eq!(store.entities.living_entity_dead(70), None);
    assert_eq!(death_time(&store, 70), 0.0);

    // The base `LivingEntity.die` guard runs once. A later synchronized pose therefore survives a
    // repeated event while the unguarded health assignment still runs.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 68,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_STANDING_ID)],
    }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 68,
        event_id: 3,
    }));
    assert_eq!(store.entities.living_entity_health(68), Some(0.0));
    assert_eq!(store.entities.living_entity_dead(68), Some(true));
    assert_eq!(entity_pose(&store, 68), VANILLA_ENTITY_POSE_STANDING_ID);

    // A same-id packet recreation is a fresh object incarnation and resets the private guard.
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert_eq!(store.entities.living_entity_health(68), Some(1.0));
    assert_eq!(store.entities.living_entity_dead(68), Some(false));
    assert!(!store.probe_entity(68).unwrap().living_dead);
    assert_eq!(entity_pose(&store, 68), VANILLA_ENTITY_POSE_STANDING_ID);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 68,
        event_id: 3,
    }));
    assert_eq!(store.entities.living_entity_dead(68), Some(true));

    // Re-entering configuration destroys the client level and cannot leak the guard into the next
    // listener/session generation.
    store.clear_client_level();
    assert!(store.probe_entity(68).is_none());
    store.apply_add_entity(protocol_add_entity_with_type(
        68,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert_eq!(store.entities.living_entity_dead(68), Some(false));
}

#[test]
fn living_death_event_repeats_virtual_prehooks_outside_the_base_guard() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_WOLF_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_SNIFFER_ID,
    ));

    let sniffer_animation_id = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 72)
            .expect("Sniffer model source")
            .sniffer_animation_id
    };

    // Wolf.die clears the wet/shaking fields before the base `dead` guard.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: 8,
    }));
    assert!(store
        .probe_entity(71)
        .unwrap()
        .client_animations
        .wolf_wet
        .is_some());
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: 3,
    }));
    assert!(store
        .probe_entity(71)
        .unwrap()
        .client_animations
        .wolf_wet
        .is_none());
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: 8,
    }));
    assert!(store
        .probe_entity(71)
        .unwrap()
        .client_animations
        .wolf_wet
        .is_some());
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: 3,
    }));
    assert!(store
        .probe_entity(71)
        .unwrap()
        .client_animations
        .wolf_wet
        .is_none());
    assert_eq!(store.entities.living_entity_dead(71), Some(true));

    // Sniffer.die transitions DATA_STATE to IDLING before the same base guard, including on repeat.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 72,
        values: vec![sniffer_state_data(VANILLA_SNIFFER_STATE_DIGGING_ID)],
    }));
    assert_eq!(
        sniffer_animation_id(&store),
        VANILLA_SNIFFER_STATE_DIGGING_ID
    );
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 3,
    }));
    assert_eq!(sniffer_animation_id(&store), -1);
    assert_eq!(store.entities.living_entity_dead(72), Some(true));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 72,
        values: vec![sniffer_state_data(VANILLA_SNIFFER_STATE_DIGGING_ID)],
    }));
    assert_eq!(
        sniffer_animation_id(&store),
        VANILLA_SNIFFER_STATE_DIGGING_ID
    );
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 3,
    }));
    assert_eq!(sniffer_animation_id(&store), -1);
    assert_eq!(entity_pose(&store, 72), VANILLA_ENTITY_POSE_DYING_ID);
}

#[test]
fn living_death_event_stops_sleeping_at_vanilla_bed_position() {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());

    let mut bed_properties = std::collections::BTreeMap::new();
    bed_properties.insert("facing".to_string(), "north".to_string());
    bed_properties.insert("occupied".to_string(), "true".to_string());
    bed_properties.insert("part".to_string(), "foot".to_string());
    let registry = crate::registries::BlockStateRegistry::vanilla_26_1();
    let bed_state_id = registry
        .find_by_name_and_properties("minecraft:white_bed", &bed_properties)
        .expect("occupied north white-bed state")
        .id;
    let stone_state_id = registry
        .find_by_name_and_properties("minecraft:stone", &std::collections::BTreeMap::new())
        .expect("stone default state")
        .id;
    let bed_pos = ProtocolBlockPos { x: 2, y: 1, z: 2 };
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: bed_pos,
        block_state_id: bed_state_id,
    }));
    // Facing NORTH and yaw 20 selects EAST first. A full stone floor below that air cell makes it
    // the first safe `DismountHelper.findSafeDismountLocation` result.
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 3, y: 0, z: 2 },
        block_state_id: stone_state_id,
    }));

    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 73,
        values: vec![
            entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID),
            living_sleeping_pos_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, Some(bed_pos)),
        ],
    }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 73,
        event_id: 3,
    }));

    let entity = store.probe_entity(73).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 3.5,
            y: 1.0,
            z: 2.5,
        }
    );
    assert_eq!(entity.y_rot, 90.0);
    assert_eq!(entity.x_rot, 0.0);
    assert_eq!(living_sleeping_pos(&store, 73), None);
    assert_eq!(entity_pose(&store, 73), VANILLA_ENTITY_POSE_DYING_ID);
    assert!(entity.living_dead);

    let bed = store.probe_block(BlockPos { x: 2, y: 1, z: 2 }).unwrap();
    assert_eq!(
        bed.block_properties.get("occupied").map(String::as_str),
        Some("false")
    );
}

#[test]
fn sleeping_villager_death_records_and_round_trips_last_woken_brain_memory() {
    let mut store = WorldStore::new();
    store.apply_world_time(ProtocolPlayTime {
        game_time: 4_321,
        clock_updates: Vec::new(),
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 74,
        values: vec![living_sleeping_pos_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            Some(ProtocolBlockPos { x: 2, y: 1, z: 2 }),
        )],
    }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 3,
    }));

    let state = store.probe_entity(74).unwrap();
    assert_eq!(
        state.living_brain,
        Some(LivingEntityBrainState {
            liked_player: None,
            last_woken: Some(4_321),
        })
    );

    let cloned = store.clone();
    assert_eq!(
        cloned.probe_entity(74).unwrap().living_brain,
        state.living_brain
    );
    let restored: WorldStore =
        serde_json::from_value(serde_json::to_value(&store).unwrap()).unwrap();
    assert_eq!(
        restored.probe_entity(74).unwrap().living_brain,
        state.living_brain
    );

    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
    ));
    assert_eq!(
        store.probe_entity(74).unwrap().living_brain,
        Some(LivingEntityBrainState::default())
    );
}

#[test]
fn spawner_minecart_event_resets_delegated_base_spawner_delay() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
    ));

    // Vanilla constructs the embedded BaseSpawner with delay 20/minimum 200. Event 1 is delegated
    // by MinecartSpawner.handleEntityEvent and resets the client delay to that minimum.
    assert_eq!(store.entities.minecart_spawner_spawn_delay(71), Some(20));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: 1,
    }));
    assert_eq!(store.entities.minecart_spawner_spawn_delay(71), Some(200));

    // The same wire id on another minecart does not acquire embedded-spawner state.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 1,
    }));
    assert_eq!(store.entities.minecart_spawner_spawn_delay(72), None);
}

#[test]
fn sheep_eat_grass_event_drives_client_animation_tick() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let eat_tick = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .sheep_eat_animation_tick
    };

    // Vanilla Sheep.handleEntityEvent: event 10 resets eatAnimationTick to 40.
    assert_eq!(eat_tick(&store, 70), 0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: 10,
    }));
    assert_eq!(eat_tick(&store, 70), 40);

    // Vanilla Sheep.aiStep decrements eatAnimationTick once per client tick.
    store.advance_entity_client_animations(1);
    assert_eq!(eat_tick(&store, 70), 39);
    store.advance_entity_client_animations(38);
    assert_eq!(eat_tick(&store, 70), 1);
    store.advance_entity_client_animations(1);
    assert_eq!(eat_tick(&store, 70), 0);
    // It clamps at 0 and does not run negative.
    store.advance_entity_client_animations(5);
    assert_eq!(eat_tick(&store, 70), 0);

    // Only event 10 starts the animation; other sheep events do not.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: 6,
    }));
    assert_eq!(eat_tick(&store, 70), 0);

    // Event 10 on a non-sheep entity never starts the sheep eat animation.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: 10,
    }));
    assert_eq!(eat_tick(&store, 71), 0);
}

#[test]
fn goat_ram_events_drive_the_lower_head_tick_counter() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_GOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let lower_head = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .goat_lower_head_tick
    };

    // Vanilla Goat.handleEntityEvent: event 58 starts lowering the head; the counter then climbs +1 per
    // client tick (aiStep), clamped at 20.
    assert_eq!(lower_head(&store, 72), 0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 58,
    }));
    assert_eq!(lower_head(&store, 72), 0);
    store.advance_entity_client_animations(1);
    assert_eq!(lower_head(&store, 72), 1);
    store.advance_entity_client_animations(19);
    assert_eq!(lower_head(&store, 72), 20);
    // It clamps at the 20 cap.
    store.advance_entity_client_animations(5);
    assert_eq!(lower_head(&store, 72), 20);

    // Event 59 raises the head; the counter then decays -2 per tick down to 0, after which the state is
    // dropped (a resting goat projects 0).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 72,
        event_id: 59,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(lower_head(&store, 72), 18);
    store.advance_entity_client_animations(9);
    assert_eq!(lower_head(&store, 72), 0);
    store.advance_entity_client_animations(5);
    assert_eq!(lower_head(&store, 72), 0);

    // The ram events on a non-goat entity never start the counter.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 73,
        event_id: 58,
    }));
    store.advance_entity_client_animations(3);
    assert_eq!(lower_head(&store, 73), 0);
}

#[test]
fn iron_golem_attack_and_offer_events_drive_client_animation_timers() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    // Vanilla IronGolem.handleEntityEvent: event 4 sets attackAnimationTick to 10; the projection lerps
    // it with the partial tick (attackTicksRemaining = tick - partial).
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 4,
    }));
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        10.0
    );
    assert_eq!(
        source(&store, 74, 0.5).iron_golem_attack_ticks_remaining,
        9.5
    );
    store.advance_entity_client_animations(10);
    // After 10 ticks the attack timer has run out.
    assert_eq!(
        source(&store, 74, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );

    // Event 11 sets offerFlowerTick to 400; event 34 clears it.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 11,
    }));
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 400);
    store.advance_entity_client_animations(3);
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 397);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 74,
        event_id: 34,
    }));
    assert_eq!(source(&store, 74, 0.0).iron_golem_offer_flower_tick, 0);

    // The same events on a non-golem never start the golem timers.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 75,
        event_id: 4,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 75, 0.0).iron_golem_attack_ticks_remaining,
        0.0
    );
}

#[test]
fn ravager_attack_stun_and_roar_timers_advance_together() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_RAVAGER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    // Vanilla Ravager.handleEntityEvent: event 4 sets attackTick to 10 (partial-lerped projection).
    assert_eq!(source(&store, 76, 0.0).ravager_attack_ticks_remaining, 0.0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 76,
        event_id: 4,
    }));
    assert_eq!(source(&store, 76, 0.5).ravager_attack_ticks_remaining, 9.5);
    store.advance_entity_client_animations(10);
    assert_eq!(source(&store, 76, 0.0).ravager_attack_ticks_remaining, 0.0);

    // Event 39 sets stunnedTick to 40; when it decays to 0 the aiStep arms the post-stun roar (20).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 76,
        event_id: 39,
    }));
    assert_eq!(
        source(&store, 76, 0.0).ravager_stunned_ticks_remaining,
        40.0
    );
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);
    store.advance_entity_client_animations(40);
    // Stun has ended; the roar is now armed at 20 and the roarAnimation ramp begins (0 at tick 20).
    assert_eq!(source(&store, 76, 0.0).ravager_stunned_ticks_remaining, 0.0);
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);
    store.advance_entity_client_animations(5);
    // After 5 roar ticks: roarTick = 15, roarAnimation = (20 - 15)/20 = 0.25.
    assert!((source(&store, 76, 0.0).ravager_roar_animation - 0.25).abs() < 1.0e-6);
    store.advance_entity_client_animations(15);
    assert_eq!(source(&store, 76, 0.0).ravager_roar_animation, 0.0);

    // The ravager events on a non-ravager never start the timers.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 77,
        event_id: 39,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 77, 0.0).ravager_stunned_ticks_remaining, 0.0);
}

#[test]
fn evoker_fangs_attack_event_drives_the_bite_progress_ramp() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let progress = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .evoker_fangs_bite_progress
    };

    // An un-attacked fang is hidden underground (biteProgress 0).
    assert_eq!(progress(&store, 78, 1.0), 0.0);
    store.advance_entity_client_animations(5);
    assert_eq!(progress(&store, 78, 1.0), 0.0);

    // Vanilla `EvokerFangs.handleEntityEvent`: event 4 → `clientSideAttackStarted = true`,
    // and `lifeTicks` (22) begins counting down; `getAnimationProgress` at `lifeTicks`
    // 22 is `1 - (20 - partial)/20`, i.e. just above 0 and climbing.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 78,
        event_id: 4,
    }));
    let started = progress(&store, 78, 1.0);
    assert!(started > 0.0, "the attack ramp starts climbing: {started}");

    store.advance_entity_client_animations(1);
    let after_one = progress(&store, 78, 1.0);
    assert!(
        after_one > started,
        "the bite ramp keeps climbing: {started} -> {after_one}"
    );

    // After 20 ticks `lifeTicks` has reached 2, so `getAnimationProgress` saturates at
    // 1.0 (the fang has fully snapped shut and vanished) and holds there.
    store.advance_entity_client_animations(20);
    assert_eq!(progress(&store, 78, 1.0), 1.0);
    store.advance_entity_client_animations(5);
    assert_eq!(progress(&store, 78, 1.0), 1.0);

    // The fang event on a non-fang never starts a ramp.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 79,
        event_id: 4,
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(progress(&store, 79, 1.0), 0.0);
}

#[test]
fn ravager_stun_tick_particles_follow_the_vanilla_head_anchor() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_RAVAGER_ID,
    ));

    assert!(store.take_ravager_stun_particle_states().is_empty());
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 76,
        event_id: 39,
    }));
    store.advance_entity_client_animations(40);

    let particles = store.take_ravager_stun_particle_states();
    assert!(
        !particles.is_empty(),
        "the deterministic client RNG should emit at least one stun particle"
    );
    assert!(particles.len() <= 40);
    let y_body_rot = 20.0_f64.to_radians();
    let anchor_x = 1.0 - 1.95 * y_body_rot.sin();
    let anchor_z = -2.0 + 1.95 * y_body_rot.cos();
    for particle in particles {
        assert_eq!(particle.entity_id, 76);
        assert!((particle.position.y - 65.9).abs() < 1.0e-6);
        assert!(
            (particle.position.x - anchor_x).abs() <= 0.3,
            "x jitter stays inside vanilla +/-0.3 around the head anchor: {:?}",
            particle
        );
        assert!(
            (particle.position.z - anchor_z).abs() <= 0.3,
            "z jitter stays inside vanilla +/-0.3 around the head anchor: {:?}",
            particle
        );
    }
    assert!(store.take_ravager_stun_particle_states().is_empty());
}

#[test]
fn evoker_fangs_attack_tick_emits_twelve_crit_particles_once() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
    ));

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 78,
        event_id: 4,
    }));
    store.advance_entity_client_animations(7);
    assert!(store.take_evoker_fangs_crit_particle_states().is_empty());

    store.advance_entity_client_animations(1);
    let particles = store.take_evoker_fangs_crit_particle_states();
    assert_eq!(particles.len(), 12);
    for particle in particles {
        assert_eq!(particle.entity_id, 78);
        assert!((0.75..=1.25).contains(&particle.position.x));
        assert!((65.05..66.05).contains(&particle.position.y));
        assert!((-2.25..=-1.75).contains(&particle.position.z));
        assert!((-0.3..=0.3).contains(&particle.velocity.x));
        assert!((0.3..=0.6).contains(&particle.velocity.y));
        assert!((-0.3..=0.3).contains(&particle.velocity.z));
    }

    store.advance_entity_client_animations(8);
    assert!(store.take_evoker_fangs_crit_particle_states().is_empty());
}

#[test]
fn camel_dash_flag_drives_the_dash_animation_timer() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        90,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));

    let dash_seconds = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .camel_dash_seconds
    };
    let jump_cooldown = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .camel_jump_cooldown
    };
    let idle_seconds = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 90)
            .unwrap()
            .camel_idle_seconds
    };
    let set_dashing = |store: &mut WorldStore, dashing: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 90,
            values: vec![camel_dash_data(dashing)],
        }));
    };

    // A non-dashing camel projects the stopped-animation sentinel.
    assert_eq!(dash_seconds(&store, 1.0), -1.0);
    assert_eq!(jump_cooldown(&store, 1.0), 0.0);
    assert_eq!(idle_seconds(&store, 1.0), -1.0);
    store.advance_entity_client_animations(3);
    assert_eq!(dash_seconds(&store, 1.0), -1.0);
    assert_eq!(jump_cooldown(&store, 1.0), 0.0);
    assert!(
        (idle_seconds(&store, 1.0) - 0.15).abs() < 1.0e-6,
        "Camel.setupAnimationStates starts CAMEL_IDLE on the first ticking client frame"
    );

    // Vanilla `Camel.setupAnimationStates`: the synced DASH rising edge starts `dashAnimationState`,
    // and the elapsed seconds climb from there (1 tick = 0.05 s). Vanilla
    // `Camel.onSyncedDataUpdated(DASH)` also seeds `dashCooldown = 55`; the following tick decrements
    // it before `CamelRenderer.getJumpCooldown` subtracts `partialTicks`.
    set_dashing(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = dash_seconds(&store, 1.0);
    assert!(
        after_one >= 0.0,
        "a dashing camel starts the dash timer: {after_one}"
    );
    assert_eq!(jump_cooldown(&store, 1.0), 53.0);
    store.advance_entity_client_animations(2);
    let after_three = dash_seconds(&store, 1.0);
    assert!(
        after_three > after_one,
        "the dash timer keeps climbing: {after_one} -> {after_three}"
    );
    assert_eq!(jump_cooldown(&store, 1.0), 51.0);

    // Clearing the DASH flag stops the animation (back to the sentinel) but the cooldown keeps
    // counting down, just like the client-side `Camel.tick` field.
    set_dashing(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(
        dash_seconds(&store, 1.0),
        -1.0,
        "clearing DASH stops the dash animation"
    );
    assert_eq!(jump_cooldown(&store, 1.0), 50.0);
    store.advance_entity_client_animations(50);
    assert_eq!(jump_cooldown(&store, 1.0), 0.0);
}

#[test]
fn copper_golem_idle_state_drives_delayed_idle_animation_timer() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let idle_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 92)
            .unwrap()
            .copper_golem_idle_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 92,
            values: vec![protocol_enum_data(
                copper_golem_state_data_id(),
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    // Vanilla `CopperGolem.setupAnimationStates` first schedules a delayed `random.nextInt(200, 240)`
    // start while the synced state is IDLE; it does not start the head spin on the first client tick.
    assert_eq!(idle_seconds(&store), -1.0);
    store.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&store), -1.0);

    // bbb uses a deterministic Java-LCG-shaped client seed, but preserves vanilla's 200..239 tick
    // timeout range, so by 240 client ticks the first idle keyframe has started.
    store.advance_entity_client_animations(239);
    let after_timeout = idle_seconds(&store);
    assert!(
        after_timeout >= 0.0,
        "the delayed copper golem idle timer starts by the vanilla timeout window: {after_timeout}"
    );

    // Any non-IDLE interaction state stops the idle animation and clears the timer; returning to IDLE
    // schedules a later restart instead of immediately resuming the head spin.
    set_state(&mut store, COPPER_GOLEM_STATE_GETTING_ITEM_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&store), -1.0);

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&store), -1.0);
}

#[test]
fn copper_golem_getting_item_state_drives_interaction_timer() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        93,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let get_item_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 93)
            .unwrap()
            .copper_golem_get_item_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 93,
            values: vec![protocol_enum_data(
                copper_golem_state_data_id(),
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    assert_eq!(get_item_seconds(&store), -1.0);
    set_state(&mut store, COPPER_GOLEM_STATE_GETTING_ITEM_ID);
    store.advance_entity_client_animations(1);
    let after_one = get_item_seconds(&store);
    assert!(
        after_one >= 0.0,
        "GETTING_ITEM starts CopperGolem.interactionGetItemAnimationState: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = get_item_seconds(&store);
    assert!(
        after_three > after_one,
        "the GETTING_ITEM interaction timer advances: {after_one} -> {after_three}"
    );

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(get_item_seconds(&store), -1.0);
}

#[test]
fn copper_golem_getting_no_item_state_drives_interaction_timer() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        94,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let get_no_item_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 94)
            .unwrap()
            .copper_golem_get_no_item_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 94,
            values: vec![protocol_enum_data(
                copper_golem_state_data_id(),
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    assert_eq!(get_no_item_seconds(&store), -1.0);
    set_state(&mut store, COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID);
    store.advance_entity_client_animations(1);
    let after_one = get_no_item_seconds(&store);
    assert!(
        after_one >= 0.0,
        "GETTING_NO_ITEM starts CopperGolem.interactionGetNoItemAnimationState: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = get_no_item_seconds(&store);
    assert!(
        after_three > after_one,
        "the GETTING_NO_ITEM interaction timer advances: {after_one} -> {after_three}"
    );

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(get_no_item_seconds(&store), -1.0);
}

#[test]
fn copper_golem_dropping_item_state_drives_interaction_timer() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        95,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let drop_item_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 95)
            .unwrap()
            .copper_golem_drop_item_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 95,
            values: vec![protocol_enum_data(
                copper_golem_state_data_id(),
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    assert_eq!(drop_item_seconds(&store), -1.0);
    set_state(&mut store, COPPER_GOLEM_STATE_DROPPING_ITEM_ID);
    store.advance_entity_client_animations(1);
    let after_one = drop_item_seconds(&store);
    assert!(
        after_one >= 0.0,
        "DROPPING_ITEM starts CopperGolem.interactionDropItemAnimationState: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = drop_item_seconds(&store);
    assert!(
        after_three > after_one,
        "the DROPPING_ITEM interaction timer advances: {after_one} -> {after_three}"
    );

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(drop_item_seconds(&store), -1.0);
}

#[test]
fn copper_golem_dropping_no_item_state_drives_interaction_timer() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        96,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    ));

    let drop_no_item_seconds = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 96)
            .unwrap()
            .copper_golem_drop_no_item_seconds
    };
    let set_state = |store: &mut WorldStore, state_id: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 96,
            values: vec![protocol_enum_data(
                copper_golem_state_data_id(),
                EntityDataEnumSerializer::CopperGolemState,
                state_id,
            )],
        }));
    };

    assert_eq!(drop_no_item_seconds(&store), -1.0);
    set_state(&mut store, COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID);
    store.advance_entity_client_animations(1);
    let after_one = drop_no_item_seconds(&store);
    assert!(
        after_one >= 0.0,
        "DROPPING_NO_ITEM starts CopperGolem.interactionDropNoItemAnimationState: {after_one}"
    );
    store.advance_entity_client_animations(2);
    let after_three = drop_no_item_seconds(&store);
    assert!(
        after_three > after_one,
        "the DROPPING_NO_ITEM interaction timer advances: {after_one} -> {after_three}"
    );

    set_state(&mut store, COPPER_GOLEM_STATE_IDLE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(drop_no_item_seconds(&store), -1.0);
}

#[test]
fn allay_dancing_flag_drives_the_dance_spin_state() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        91,
        VANILLA_ENTITY_TYPE_ALLAY_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 91)
            .unwrap()
    };
    let set_dancing = |store: &mut WorldStore, dancing: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 91,
            values: vec![allay_dancing_data(dancing)],
        }));
    };

    // A non-dancing allay projects the inert dance state.
    let resting = source(&store, 1.0);
    assert!(!resting.allay_dancing);
    assert!(!resting.allay_spinning);
    assert_eq!(resting.allay_spinning_progress, 0.0);
    store.advance_entity_client_animations(3);
    assert!(!source(&store, 1.0).allay_dancing);

    // Vanilla `Allay.tick` (client): while DATA_DANCING is set, `dancingAnimationTicks` climbs and
    // the first 15 ticks of each 55-tick loop are the spin sub-window (`spinningAnimationTicks`
    // ramping 0->15, `spinningProgress` 0->1).
    set_dancing(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = source(&store, 1.0);
    assert!(
        after_one.allay_dancing,
        "the synced flag marks the allay dancing"
    );
    assert!(
        after_one.allay_spinning,
        "the dance opens in the spin sub-window"
    );
    assert!(after_one.allay_spinning_progress > 0.0);

    // Ten ticks into the spin window the progress has climbed further.
    store.advance_entity_client_animations(9);
    let after_ten = source(&store, 1.0);
    assert!(
        after_ten.allay_spinning_progress > after_one.allay_spinning_progress,
        "the spin ramp climbs: {} -> {}",
        after_one.allay_spinning_progress,
        after_ten.allay_spinning_progress
    );

    // Past the 15-tick spin window the allay is still dancing but no longer spinning, and the spin
    // progress unwinds back toward 0 (`spinningAnimationTicks` decrements once `isSpinning` is false).
    store.advance_entity_client_animations(10);
    let after_twenty = source(&store, 1.0);
    assert!(after_twenty.allay_dancing);
    assert!(
        !after_twenty.allay_spinning,
        "the spin sub-window has closed"
    );
    assert!(
        after_twenty.allay_spinning_progress < after_ten.allay_spinning_progress,
        "the spin ramp unwinds once spinning stops"
    );

    // Clearing the flag resets the dance entirely.
    set_dancing(&mut store, false);
    store.advance_entity_client_animations(1);
    let stopped = source(&store, 1.0);
    assert!(!stopped.allay_dancing);
    assert!(!stopped.allay_spinning);
    assert_eq!(stopped.allay_spinning_progress, 0.0);
}

#[test]
fn allay_main_hand_equipment_drives_holding_item_progress() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_ALLAY_ID,
    ));

    let holding_progress = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 92)
            .unwrap()
            .allay_holding_item_progress
    };
    let set_main_hand = |store: &mut WorldStore, item: ItemStackSummary| {
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id: 92,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item,
            }],
        }));
    };

    // Vanilla `Allay.tick`: empty-handed allays keep `holdingItemAnimationTicks`
    // settled at 0, so the projected render-state progress is inert.
    assert_eq!(holding_progress(&store, 1.0), 0.0);
    store.advance_entity_client_animations(2);
    assert_eq!(holding_progress(&store, 1.0), 0.0);

    // A non-empty main hand raises the held-item counter by 1 per client tick and
    // `getHoldingItemAnimationProgress(partialTick)` divides the lerp by 5.
    set_main_hand(&mut store, item_stack(42, 1));
    store.advance_entity_client_animations(1);
    assert!((holding_progress(&store, 0.5) - 0.1).abs() < 1.0e-6);
    assert!((holding_progress(&store, 1.0) - 0.2).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert!((holding_progress(&store, 1.0) - 1.0).abs() < 1.0e-6);

    // Clearing the main hand eases the arms back down before the idle state is dropped.
    set_main_hand(&mut store, ItemStackSummary::empty());
    store.advance_entity_client_animations(1);
    assert!((holding_progress(&store, 1.0) - 0.8).abs() < 1.0e-6);
    store.advance_entity_client_animations(4);
    assert_eq!(holding_progress(&store, 1.0), 0.0);
}

#[test]
fn pillager_charging_crossbow_flag_drives_the_use_item_tick_counter() {
    let mut store = WorldStore::new();
    let (charging_data_id, charging_default) =
        super::schema::vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
        )
        .expect("generated Pillager charging accessor");
    assert!(!charging_default);
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 77)
            .unwrap()
    };
    let set_charging = |store: &mut WorldStore, charging: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 77,
            values: vec![ProtocolEntityDataValue {
                data_id: charging_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(charging),
            }],
        }));
    };

    // A pillager that has never drawn its crossbow projects zero use-item ticks.
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
    store.advance_entity_client_animations(3);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);

    // Vanilla `LivingEntity` reconstructs `getTicksUsingItem()` as a per-tick count that rises while the
    // crossbow draw (use-item) is active; the renderer reads `getTicksUsingItem(partialTicks)`.
    set_charging(&mut store, true);
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 0.0).crossbow_charge_ticks,
        1.0,
        "one tick of charging counts as one use-item tick"
    );
    // The partial tick adds the in-between fraction, matching `getTicksUsingItem(partialTicks)`.
    assert_eq!(source(&store, 0.5).crossbow_charge_ticks, 1.5);

    store.advance_entity_client_animations(9);
    assert_eq!(
        source(&store, 0.0).crossbow_charge_ticks,
        10.0,
        "the counter keeps climbing while the draw is held"
    );

    // Clearing the charging flag resets the counter to zero (the draw stopped).
    set_charging(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 1.0).crossbow_charge_ticks,
        0.0,
        "releasing the crossbow resets the use-item counter"
    );
}

#[test]
fn piglin_charging_crossbow_flag_drives_the_shared_use_item_tick_counter() {
    let (charging_data_id, _) =
        vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin charging-crossbow accessor must resolve");

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        78,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 78)
            .unwrap()
    };
    let set_charging = |store: &mut WorldStore, charging: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 78,
            values: vec![ProtocolEntityDataValue {
                data_id: charging_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(charging),
            }],
        }));
    };

    // The regular piglin draws its crossbow with the SAME `animateCrossbowCharge`, so it shares the
    // pillager's `getTicksUsingItem` counter — each class still owns its distinct generated flag.
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
    store.advance_entity_client_animations(2);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);

    set_charging(&mut store, true);
    store.advance_entity_client_animations(4);
    assert_eq!(
        source(&store, 0.0).crossbow_charge_ticks,
        4.0,
        "four ticks of charging count as four use-item ticks"
    );
    assert_eq!(source(&store, 0.5).crossbow_charge_ticks, 4.5);

    // Releasing the crossbow resets the shared counter.
    set_charging(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
}

#[test]
fn player_using_item_flag_drives_the_shared_use_item_tick_counter() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        79,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 79)
            .unwrap()
    };
    let set_using = |store: &mut WorldStore, using: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 79,
            values: vec![living_entity_using_item_data(
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                using,
            )],
        }));
    };

    // Vanilla `getTicksUsingItem()` is item-agnostic, so the player drives the SAME draw counter off its
    // generated `isUsingItem` bit; the native layer applies the crossbow pose only when the using
    // item is an uncharged crossbow.
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
    store.advance_entity_client_animations(2);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);

    set_using(&mut store, true);
    store.advance_entity_client_animations(3);
    assert_eq!(
        source(&store, 0.0).crossbow_charge_ticks,
        3.0,
        "three ticks of using an item count as three use-item ticks"
    );
    assert_eq!(source(&store, 0.5).crossbow_charge_ticks, 3.5);

    // Stopping the use resets the shared counter (the draw stopped).
    set_using(&mut store, false);
    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 1.0).crossbow_charge_ticks, 0.0);
}

#[test]
fn axolotl_playing_dead_flag_drives_the_eased_factor() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        92,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    ));

    let factor = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 92)
            .unwrap()
            .axolotl_playing_dead_factor
    };
    let set_playing_dead = |store: &mut WorldStore, dead: bool| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 92,
            values: vec![axolotl_playing_dead_data(dead)],
        }));
    };

    // An awake axolotl projects no play-dead blend.
    assert_eq!(factor(&store, 1.0), 0.0);

    // Vanilla `Axolotl.playingDeadAnimator` (`BinaryAnimator(10, IN_OUT_SINE)`): the synced
    // `DATA_PLAYING_DEAD` flag eases the factor from 0 to a full 1.0 over the animator's 10 ticks.
    set_playing_dead(&mut store, true);
    store.advance_entity_client_animations(1);
    let after_one = factor(&store, 1.0);
    assert!(
        after_one > 0.0 && after_one < 1.0,
        "the play-dead factor eases up: {after_one}"
    );
    store.advance_entity_client_animations(9);
    assert!(
        (factor(&store, 1.0) - 1.0).abs() < 1.0e-6,
        "the factor saturates at 1.0 after the 10-tick animator length"
    );

    // Clearing the flag eases the factor back down to 0 over the next 10 ticks.
    set_playing_dead(&mut store, false);
    store.advance_entity_client_animations(1);
    let easing_down = factor(&store, 1.0);
    assert!(
        easing_down < 1.0,
        "the factor eases back down once awake: {easing_down}"
    );
    store.advance_entity_client_animations(10);
    assert_eq!(factor(&store, 1.0), 0.0, "fully awake again");
}

#[test]
fn baby_axolotl_ticks_one_animation_state_and_restarts_on_transition() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        93,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 93,
        values: vec![axolotl_baby_data(true)],
    }));

    let timers = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 93)
            .unwrap();
        [
            source.axolotl_baby_swim_seconds,
            source.axolotl_baby_walk_seconds,
            source.axolotl_baby_walk_underwater_seconds,
            source.axolotl_baby_idle_underwater_seconds,
            source.axolotl_baby_idle_underwater_on_ground_seconds,
            source.axolotl_baby_idle_on_ground_seconds,
            source.axolotl_baby_play_dead_seconds,
        ]
    };

    assert_eq!(timers(&store, 0.0), [-1.0; 7]);
    // With no water, movement, or ground contact, Vanilla selects idleOnGround. The selected
    // `AnimationState` starts at this tick and keeps its original start while the state persists.
    store.advance_entity_client_animations(1);
    assert_eq!(
        timers(&store, 0.0),
        [-1.0, -1.0, -1.0, -1.0, -1.0, 0.0, -1.0]
    );
    store.advance_entity_client_animations(2);
    assert_eq!(timers(&store, 1.0)[5], 0.15);

    // Synced play-dead wins over every movement/environment branch. `soloAnimation` stops the old
    // idle timer and starts play-dead from zero at the transition tick.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 93,
        values: vec![axolotl_playing_dead_data(true)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        timers(&store, 0.0),
        [-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, 0.0]
    );
    assert_eq!(timers(&store, 0.5)[6], 0.025);
}

#[test]
fn hoglin_attack_event_and_tick_match_generated_interface_graph() {
    // Vanilla HoglinBase implementors handle event 4 by setting the raw attack timer to 10, then
    // decrement it in aiStep. Compare both dispatch sites over the complete generated type catalog.
    for entry in VANILLA_ENTITY_TYPES {
        let entity_id = 1_000 + entry.id;
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(entity_id, entry.id));
        let attack_tick = |store: &WorldStore| {
            store
                .probe_entity(entity_id)
                .expect("catalog entity state")
                .client_animations
                .hoglin_attack_animation_tick()
        };

        assert_eq!(attack_tick(&store), 0, "{} initial", entry.resource_id);
        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id,
            event_id: 4,
        }));
        let expected = vanilla_entity_metadata_type_is_hoglin_base(entry.id);
        assert_eq!(
            attack_tick(&store),
            if expected { 10 } else { 0 },
            "{} event dispatch",
            entry.resource_id
        );
        store.advance_entity_client_animations(1);
        assert_eq!(
            attack_tick(&store),
            if expected { 9 } else { 0 },
            "{} tick dispatch",
            entry.resource_id
        );
    }
}

#[test]
fn rabbit_jump_event_drives_the_hop_window() {
    const RABBIT_JUMP_EVENT_ID: i8 = 1;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_RABBIT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let hop = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .rabbit_hop_seconds
    };

    // A resting rabbit projects the `-1.0` stopped sentinel.
    assert_eq!(hop(&store, 50, 1.0), -1.0);

    // Vanilla `Rabbit.handleEntityEvent(1)` seeds `jumpDuration = 15; jumpTicks = 0`. The hop is NOT
    // started yet — vanilla's `setupAnimationStates` (the hop branch) runs BEFORE `aiStep` lifts
    // `jumpTicks` past `0`, so the seed tick still reads the stopped sentinel.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 50,
        event_id: RABBIT_JUMP_EVENT_ID,
    }));
    assert_eq!(hop(&store, 50, 0.0), -1.0);
    // First tick: `jumpTicks` climbs to 1, but the hop only `startIfStopped`s on the NEXT tick's
    // `setupAnimationStates`, so it is still stopped here.
    store.advance_entity_client_animations(1);
    assert_eq!(hop(&store, 50, 0.0), -1.0);
    // Second tick: `jumpTicks > 0`, so the hop starts at the current age (elapsed begins at 0).
    store.advance_entity_client_animations(1);
    assert!((hop(&store, 50, 0.0) - 0.0).abs() < 1.0e-6);
    // The hop advances `1 / 20` per tick while the window runs.
    store.advance_entity_client_animations(5);
    assert!((hop(&store, 50, 0.0) - 0.25).abs() < 1.0e-6);
    // The partial tick folds into the live age.
    assert!((hop(&store, 50, 0.5) - 0.275).abs() < 1.0e-6);

    // The window is 15 ticks; the hop holds through its end (`jumpTicks` reaches `jumpDuration` and
    // resets), then stops on the following tick (`jumpTicks` back to 0).
    store.advance_entity_client_animations(9);
    assert!(
        (hop(&store, 50, 0.0) - 0.7).abs() < 1.0e-6,
        "still hopping at tick 14"
    );
    store.advance_entity_client_animations(1);
    assert_eq!(
        hop(&store, 50, 0.0),
        -1.0,
        "the hop stops when the jump window closes"
    );

    // The jump event on a non-rabbit never starts a hop.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 51,
        event_id: RABBIT_JUMP_EVENT_ID,
    }));
    store.advance_entity_client_animations(2);
    assert_eq!(hop(&store, 51, 0.0), -1.0);
}

#[test]
fn entity_model_sources_project_arrow_impact_shake() {
    let arrow_accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARROW_ID,
    )
    .expect("generated Arrow inherited metadata accessors");
    let spectral_arrow_accessors =
        vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
        )
        .expect("generated SpectralArrow inherited metadata accessors");
    let trident_accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .expect("generated ThrownTrident inherited metadata accessors");

    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        62,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    ));

    // Vanilla `AbstractArrow.onSyncedDataUpdated(IN_GROUND)` ignores the entity's first tick.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(arrow_accessors.in_ground_data_id, true)],
    }));
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 0.0);

    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 60, 0.0).arrow_shake,
        0.0,
        "the first-tick metadata update must not be replayed from stored state"
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(arrow_accessors.in_ground_data_id, false)],
    }));
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 0.0);

    // Past the first client tick, an `IN_GROUND` update to true starts `shakeTime = 7`;
    // `ArrowRenderer.extractRenderState` projects `shakeTime - partialTick`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(arrow_accessors.in_ground_data_id, true)],
    }));
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 7.0);
    assert_eq!(source(&store, 60, 0.5).arrow_shake, 6.5);

    store.advance_entity_client_animations(1);
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 6.0);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![protocol_bool_data(arrow_accessors.in_ground_data_id, true)],
    }));
    assert_eq!(
        source(&store, 60, 0.0).arrow_shake,
        6.0,
        "vanilla only restarts when the current shake has settled"
    );
    store.advance_entity_client_animations(6);
    assert_eq!(source(&store, 60, 0.0).arrow_shake, 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![protocol_bool_data(
            spectral_arrow_accessors.in_ground_data_id,
            true,
        )],
    }));
    assert_eq!(
        source(&store, 61, 0.25).arrow_shake,
        6.75,
        "spectral arrows share AbstractArrow.shakeTime"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![protocol_bool_data(
            trident_accessors.in_ground_data_id,
            true
        )],
    }));
    assert_eq!(
        source(&store, 62, 0.0).arrow_shake,
        0.0,
        "thrown tridents use their own renderer state and do not consume ArrowRenderState.shake"
    );
}

#[test]
fn creaking_combat_events_and_tearing_down_drive_the_keyframes() {
    const CREAKING_ATTACK_EVENT_ID: i8 = 4;
    const CREAKING_INVULNERABLE_EVENT_ID: i8 = 66;
    let (can_move_data_id, can_move_default) =
        super::schema::vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated Creaking can-move accessor");
    assert!(can_move_default);
    let (tearing_down_data_id, tearing_down_default) =
        super::schema::vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated Creaking tearing-down accessor");
    assert!(!tearing_down_default);
    let living_accessors =
        super::schema::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated inherited Creaking living accessors");
    assert_eq!(living_accessors.health_default, 1.0);

    let creaking_bool = |data_id: u8, value: bool| ProtocolEntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    };
    let source = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    // A resting creaking can move (the `CAN_MOVE` default) and projects the `-1.0` stopped sentinels.
    let rest = source(&store, 70, 1.0);
    assert!(rest.creaking_can_move, "default canMove is true");
    assert_eq!(rest.creaking_attack_seconds, -1.0);
    assert_eq!(rest.creaking_invulnerable_seconds, -1.0);
    assert_eq!(rest.creaking_death_seconds, -1.0);
    assert!(!rest.creaking_eyes_glowing);

    // The synced `CAN_MOVE = false` freezes the walk (a creaking observed mid-step turns to a statue).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(can_move_data_id, false)],
    }));
    assert!(!source(&store, 70, 1.0).creaking_can_move);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(can_move_data_id, true)],
    }));

    // Vanilla `Creaking.handleEntityEvent(4)`: `attackAnimationRemainingTicks = 15`. The one-shot is
    // NOT started yet — it only `animateWhen`s on the NEXT tick's `setupAnimationStates`, after
    // `aiStep` has decremented the counter (still positive). So the seed tick reads the stopped value.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: CREAKING_ATTACK_EVENT_ID,
    }));
    assert_eq!(source(&store, 70, 0.0).creaking_attack_seconds, -1.0);
    // First tick: vanilla decrements `15 -> 14` BEFORE `setupAnimationStates`, so the attack starts at
    // the current age this very tick (elapsed begins at 0), unlike the rabbit (which is animate-first).
    store.advance_entity_client_animations(1);
    assert!((source(&store, 70, 0.0).creaking_attack_seconds - 0.0).abs() < 1.0e-6);
    // It advances `1 / 20` per tick, with the partial folded into the live age.
    store.advance_entity_client_animations(5);
    assert!((source(&store, 70, 0.0).creaking_attack_seconds - 0.25).abs() < 1.0e-6);
    assert!((source(&store, 70, 0.5).creaking_attack_seconds - 0.275).abs() < 1.0e-6);
    // The window is 15 ticks (`attackTicks` 15 -> 0); the attack animates while it stays positive, so
    // it holds through tick 14 then stops when the counter hits 0 on tick 15.
    store.advance_entity_client_animations(8);
    assert!(
        (source(&store, 70, 0.0).creaking_attack_seconds - 0.65).abs() < 1.0e-6,
        "still attacking at tick 14"
    );
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 70, 0.0).creaking_attack_seconds,
        -1.0,
        "the attack stops when the 15-tick window closes"
    );

    // Vanilla `Creaking.handleEntityEvent(66)`: `invulnerabilityAnimationRemainingTicks = 8`, the same
    // decrement-first window (8 ticks).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: CREAKING_INVULNERABLE_EVENT_ID,
    }));
    store.advance_entity_client_animations(1);
    assert!((source(&store, 70, 0.0).creaking_invulnerable_seconds - 0.0).abs() < 1.0e-6);
    store.advance_entity_client_animations(7);
    assert_eq!(
        source(&store, 70, 0.0).creaking_invulnerable_seconds,
        -1.0,
        "the stagger stops when the 8-tick window closes"
    );

    // Vanilla `deathAnimationState.animateWhen(isTearingDown(), tickCount)`: the synced
    // `IS_TEARING_DOWN` boolean drives the collapse directly (no counter). The server sets health
    // to zero with teardown; on the first client tick `tickDeath` advances `deathTime` to 1 before
    // `checkEyeBlink`, so the false-eye 0..=0 delay toggles the emissive eyes on immediately. Tick 2
    // schedules from the glowing 2..=8 range and toggles them back off.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![
            creaking_bool(tearing_down_data_id, true),
            ProtocolEntityDataValue {
                data_id: living_accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(0.0),
            },
        ],
    }));
    store.advance_entity_client_animations(1);
    let first_teardown_tick = source(&store, 70, 0.0);
    assert!((first_teardown_tick.creaking_death_seconds - 0.0).abs() < 1.0e-6);
    assert!(first_teardown_tick.creaking_eyes_glowing);
    store.advance_entity_client_animations(1);
    assert!(!source(&store, 70, 0.0).creaking_eyes_glowing);
    store.advance_entity_client_animations(4);
    assert!((source(&store, 70, 0.0).creaking_death_seconds - 0.25).abs() < 1.0e-6);

    // The shared living death-tip counter stops at 20 because ordinary entities are removed then,
    // but heart-bound Creaking teardown lasts 45 ticks. Its private deathTime/flicker cursor must
    // therefore keep advancing and produce at least one eye transition after the shared cap.
    store.advance_entity_client_animations(14);
    let twentieth_teardown_tick = source(&store, 70, 0.0);
    assert_eq!(twentieth_teardown_tick.death_time, 20.0);
    let mut previous_eyes = twentieth_teardown_tick.creaking_eyes_glowing;
    let mut transitioned_after_shared_death_cap = false;
    for _ in 0..25 {
        store.advance_entity_client_animations(1);
        let eyes = source(&store, 70, 0.0).creaking_eyes_glowing;
        transitioned_after_shared_death_cap |= eyes != previous_eyes;
        previous_eyes = eyes;
    }
    assert!(
        transitioned_after_shared_death_cap,
        "teardown eye flicker must continue through the 45-tick Creaking death window"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![creaking_bool(tearing_down_data_id, false)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        source(&store, 70, 0.0).creaking_death_seconds,
        -1.0,
        "clearing isTearingDown stops the collapse"
    );
    assert!(!source(&store, 70, 0.0).creaking_eyes_glowing);

    // A non-creaking never gets a creaking state: its combat seconds stay stopped, and `canMove`
    // projects the gated `true` regardless of the event/metadata.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 71,
        event_id: CREAKING_ATTACK_EVENT_ID,
    }));
    store.advance_entity_client_animations(2);
    let chicken = source(&store, 71, 0.0);
    assert!(chicken.creaking_can_move);
    assert!(!chicken.creaking_eyes_glowing);
    assert_eq!(chicken.creaking_attack_seconds, -1.0);
    assert_eq!(chicken.creaking_death_seconds, -1.0);
}

#[test]
fn breeze_pose_drives_the_action_animations() {
    // Vanilla `Breeze.onSyncedDataUpdated(DATA_POSE)` + `tick`: the synced pose starts/stops the
    // shoot/inhale/slide/longJump one-shots (active while their pose holds), and LEAVING `Pose.SLIDING`
    // fires the brief `slideBack`. Each is projected as the elapsed seconds since it started, `-1.0`
    // when stopped. The looping idle is renderer-side and not projected.
    let actions = |store: &WorldStore, id: i32| {
        let s = store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap();
        (
            s.breeze_shoot_seconds,
            s.breeze_slide_seconds,
            s.breeze_slide_back_seconds,
            s.breeze_inhale_seconds,
            s.breeze_long_jump_seconds,
        )
    };
    let set_pose = |store: &mut WorldStore, id: i32, pose: i32| {
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![entity_pose_data(pose)],
        }));
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_BREEZE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    // A resting breeze projects the stopped sentinel for every action.
    assert_eq!(actions(&store, 60), (-1.0, -1.0, -1.0, -1.0, -1.0));

    // `Pose.SHOOTING` starts the shoot at the current age (elapsed begins at 0, advancing 1/20 per
    // tick); the others stay stopped.
    set_pose(&mut store, 60, VANILLA_ENTITY_POSE_SHOOTING_ID);
    assert_eq!(actions(&store, 60), (0.0, -1.0, -1.0, -1.0, -1.0));
    store.advance_entity_client_animations(5);
    assert!((actions(&store, 60).0 - 0.25).abs() < 1.0e-6);
    // Leaving SHOOTING stops the shoot (it is not a SLIDING leave, so no slideBack).
    set_pose(&mut store, 60, VANILLA_ENTITY_POSE_STANDING_ID);
    assert_eq!(actions(&store, 60), (-1.0, -1.0, -1.0, -1.0, -1.0));

    // `Pose.SLIDING` starts the slide; LEAVING it stops the slide AND fires `slideBack` at the leave.
    set_pose(&mut store, 60, VANILLA_ENTITY_POSE_SLIDING_ID);
    assert_eq!(actions(&store, 60).1, 0.0, "slide starts on SLIDING");
    store.advance_entity_client_animations(2);
    assert!((actions(&store, 60).1 - 0.1).abs() < 1.0e-6);
    set_pose(&mut store, 60, VANILLA_ENTITY_POSE_STANDING_ID);
    let (shoot, slide, slide_back, _, _) = actions(&store, 60);
    assert_eq!(shoot, -1.0);
    assert_eq!(slide, -1.0, "leaving SLIDING stops the slide");
    assert_eq!(
        slide_back, 0.0,
        "leaving SLIDING fires slideBack at the leave"
    );
    store.advance_entity_client_animations(3);
    assert!(
        (actions(&store, 60).2 - 0.15).abs() < 1.0e-6,
        "the slideBack return advances"
    );

    // `Pose.INHALING` starts the inhale; switching to `Pose.LONG_JUMPING` stops it and starts longJump.
    set_pose(&mut store, 60, VANILLA_ENTITY_POSE_INHALING_ID);
    assert_eq!(actions(&store, 60).3, 0.0, "inhale starts on INHALING");
    set_pose(&mut store, 60, VANILLA_ENTITY_POSE_LONG_JUMPING_ID);
    let (_, _, _, inhale, long_jump) = actions(&store, 60);
    assert_eq!(inhale, -1.0, "leaving INHALING stops the inhale");
    assert_eq!(long_jump, 0.0, "LONG_JUMPING starts the jump");

    // A non-breeze never gets a breeze state: every action stays stopped regardless of the pose.
    set_pose(&mut store, 61, VANILLA_ENTITY_POSE_SHOOTING_ID);
    store.advance_entity_client_animations(2);
    assert_eq!(actions(&store, 61), (-1.0, -1.0, -1.0, -1.0, -1.0));
}

#[test]
fn warden_tendril_event_drives_client_animation_pulse() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));

    let tendril = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .tendril_animation
    };

    // A warden at rest reports no tendril pulse.
    assert_eq!(tendril(&store, 80, 1.0), 0.0);

    // Vanilla Warden.handleEntityEvent: event 61 resets tendrilAnimation to 10. Vanilla
    // getTendrilAnimation lerps (tendrilAnimationO, tendrilAnimation) / 10, so right after the
    // event the lerp fades from the previous 0 (partialTick 0) to the new 10 (partialTick 1).
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 80,
        event_id: 61,
    }));
    assert!((tendril(&store, 80, 0.0) - 0.0).abs() < 1.0e-6);
    assert!((tendril(&store, 80, 0.5) - 0.5).abs() < 1.0e-6);
    assert!((tendril(&store, 80, 1.0) - 1.0).abs() < 1.0e-6);

    // Vanilla Warden.tick decrements tendrilAnimation once per client tick (lerp endpoint = current).
    store.advance_entity_client_animations(1);
    assert!((tendril(&store, 80, 1.0) - 0.9).abs() < 1.0e-6);
    store.advance_entity_client_animations(9);
    assert!((tendril(&store, 80, 1.0) - 0.0).abs() < 1.0e-6);
    // It settles at 0 and stays there.
    store.advance_entity_client_animations(5);
    assert!((tendril(&store, 80, 1.0) - 0.0).abs() < 1.0e-6);

    // Only event 61 starts the pulse; other warden events do not.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 80,
        event_id: 4,
    }));
    assert_eq!(tendril(&store, 80, 1.0), 0.0);

    // Event 61 on a non-warden entity never starts the tendril pulse.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 81,
        event_id: 61,
    }));
    assert_eq!(tendril(&store, 81, 1.0), 0.0);
}

#[test]
fn warden_client_anger_drives_heartbeat_period() {
    const WARDEN_ANGRY_MINIMUM_ANGER: i32 = 80;

    let heart = |store: &WorldStore, id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .heart_animation
    };

    let mut calm = WorldStore::new();
    calm.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));
    calm.advance_entity_client_animations(39);
    assert_eq!(heart(&calm, 82, 1.0), 0.0);
    calm.advance_entity_client_animations(1);
    assert_eq!(heart(&calm, 82, 0.0), 1.0);
    assert!((heart(&calm, 82, 1.0) - 0.9).abs() < 1.0e-6);

    let mut angry = WorldStore::new();
    angry.apply_add_entity(protocol_add_entity_with_type(
        83,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));
    assert!(angry.apply_set_entity_data(ProtocolSetEntityData {
        id: 83,
        values: vec![warden_anger_data(WARDEN_ANGRY_MINIMUM_ANGER)],
    }));
    angry.advance_entity_client_animations(9);
    assert_eq!(heart(&angry, 83, 1.0), 0.0);
    angry.advance_entity_client_animations(1);
    assert_eq!(heart(&angry, 83, 0.0), 1.0);
    assert!((heart(&angry, 83, 1.0) - 0.9).abs() < 1.0e-6);
}

#[test]
fn probes_entity_status_from_world_store() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity(123));

    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 123, action: 3 }));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 123,
        event_id: 35,
    }));
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 123, yaw: 45.5 }));

    let status = store.probe_entity_status(123).unwrap();

    assert_eq!(status.id, 123);
    assert_eq!(status.entity_type_id, VANILLA_ENTITY_TYPE_AXOLOTL_ID);
    assert_eq!(status.last_animation_action, Some(3));
    assert_eq!(status.last_event_id, Some(35));
    assert_eq!(status.last_hurt_yaw, Some(45.5));
    assert!(status.mob_effects.is_empty());
    assert!(status.last_damage.is_none());
    assert!(store.probe_entity_status(999).is_none());
}

#[test]
fn tracks_entity_link_updates() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity(20));
    store.apply_add_entity(protocol_add_entity_with_type(
        11,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID,
    ));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 20,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(20));
    assert_eq!(store.entities.leash(10).unwrap().holder_id, Some(20));

    // Turtle implements `Leashable` even though it does not override the
    // default attachment formula; generated interface membership must retain it.
    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 11,
        dest_id: 20,
    }));
    assert_eq!(store.entities.leash(11).unwrap().holder_id, Some(20));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 999,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(999));
    assert_eq!(store.entities.leash(10).unwrap().holder_id, Some(999));

    assert!(!store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 999,
        dest_id: 20,
    }));

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 0,
    }));
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);
    assert_eq!(store.entities.leash(10).unwrap().holder_id, None);

    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 10,
        dest_id: 20,
    }));
    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![20],
        }),
        1
    );
    assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);
    assert_eq!(store.entities.leash(10).unwrap().holder_id, None);

    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    assert!(!store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 30,
        dest_id: 20,
    }));
    assert_eq!(store.probe_entity(30).unwrap().leash_holder_id, None);

    assert_eq!(store.counters().entity_link_updates_received, 7);
    assert_eq!(store.counters().entity_link_updates_applied, 5);
    assert_eq!(store.counters().entity_link_updates_ignored, 2);
}

#[test]
fn entity_store_clone_matches_projected_states_component_for_component() {
    let mut store = WorldStore::new();
    // Plain mob, a minecart (conditional lerp component), a hurting
    // projectile (conditional acceleration component), and a re-added id.
    store.apply_add_entity(protocol_add_entity_with_type(1, VANILLA_ENTITY_TYPE_COW_ID));
    store.apply_add_entity(protocol_add_entity_with_type(
        2,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        3,
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(1, VANILLA_ENTITY_TYPE_COW_ID));

    let cloned = store.clone();
    assert_eq!(
        serde_json::to_string(&cloned).unwrap(),
        serde_json::to_string(&store).unwrap()
    );
}
