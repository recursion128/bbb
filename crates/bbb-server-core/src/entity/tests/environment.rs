use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

#[test]
fn body_freeze_immunity_covers_spectators_leather_and_immune_types() {
    use crate::entity::environment::can_body_freeze;
    assert_eq!(
        can_body_freeze(
            true,
            "minecraft:player",
            ["minecraft:leather_helmet"],
            |_, _, _| false
        ),
        Some(false)
    );
    let lookup = |registry: &str, tag: &str, id: &str| {
        matches!(
            (registry, tag, id),
            (
                "minecraft:item",
                "minecraft:freeze_immune_wearables",
                "minecraft:leather_helmet"
            ) | (
                "minecraft:entity_type",
                "minecraft:freeze_immune_entity_types",
                "minecraft:polar_bear"
            )
        )
    };
    assert_eq!(
        can_body_freeze(false, "minecraft:pig", ["minecraft:leather_helmet"], lookup),
        Some(false)
    );
    assert_eq!(
        can_body_freeze(false, "minecraft:player", ["minecraft:iron_helmet"], lookup),
        Some(true)
    );
    assert_eq!(
        can_body_freeze(false, "minecraft:polar_bear", [], lookup),
        Some(false)
    );
    assert_eq!(
        can_body_freeze(false, "minecraft:skeleton", [], lookup),
        Some(false)
    );
}

#[test]
fn body_freeze_counter_and_damage_follow_the_vanilla_thresholds() {
    use crate::entity::environment::{body_freeze_damage_due, tick_body_freeze};
    assert_eq!(tick_body_freeze(142, false, || Some(false)), Some(140));
    assert_eq!(tick_body_freeze(1, false, || None), Some(0));
    assert_eq!(tick_body_freeze(140, true, || Some(false)), Some(138));
    assert_eq!(tick_body_freeze(140, true, || Some(true)), Some(140));
    assert_eq!(body_freeze_damage_due(139, 40, || Some(false)), Some(false));
    assert_eq!(body_freeze_damage_due(140, 39, || Some(false)), Some(false));
    assert_eq!(body_freeze_damage_due(140, 40, || Some(false)), Some(false));
    assert_eq!(body_freeze_damage_due(140, 40, || Some(true)), Some(true));
}

#[test]
fn body_frost_replaces_transient_float_modifier_and_removes_it_over_air() {
    use crate::entity::attributes::{AttributeMap, AttributeSupplier};
    use crate::entity::environment::update_body_frost;
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
    let mut attributes =
        AttributeMap::new(AttributeSupplier::for_entity_type("minecraft:player").unwrap());
    let base = attributes
        .value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap();
    update_body_frost(&mut attributes, 70, false);
    let modifier = attributes
        .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap()
        .modifier("minecraft:powder_snow")
        .unwrap();
    assert_eq!(modifier.amount, -0.02500000037252903);
    assert_eq!(
        modifier.operation,
        crate::entity::attributes::AttributeOperation::AddValue
    );
    assert_eq!(
        attributes
            .value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .unwrap(),
        base - 0.02500000037252903
    );
    assert!(attributes.pack().iter().all(|attribute| attribute
        .modifiers
        .iter()
        .all(|modifier| modifier.id != "minecraft:powder_snow")));
    update_body_frost(&mut attributes, 200, false);
    assert_eq!(
        attributes
            .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .unwrap()
            .modifier("minecraft:powder_snow")
            .unwrap()
            .amount,
        -0.05000000074505806
    );
    update_body_frost(&mut attributes, 140, true);
    assert_eq!(
        attributes.value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        Some(base)
    );
    update_body_frost(&mut attributes, 70, false);
    update_body_frost(&mut attributes, 0, false);
    assert_eq!(
        attributes.value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        Some(base)
    );
}

#[test]
fn stored_body_armor_immunity_still_thaws_and_retains_residual_frost() {
    use crate::entity::environment::{apply_living_freeze_suffix, can_freeze, KnownContact};
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let entity = store.get_mut(id).unwrap();
    entity.living_mut().unwrap().set_item_slot(
        bbb_protocol::equipment_slots::EquipmentSlot::Body,
        crate::entity::item::plain_stack("minecraft:leather_horse_armor", 1),
    );
    entity.set_ticks_frozen(72);
    entity.set_is_in_powder_snow(true);
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    assert_eq!(
        level.with_level(|ctx| can_freeze(entity, ctx)),
        KnownContact::Known(false)
    );
    assert_eq!(
        level.with_level(|ctx| apply_living_freeze_suffix(entity, ctx)),
        KnownContact::Known(false)
    );
    assert_eq!(entity.ticks_frozen(), 70);
    assert_eq!(
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .unwrap()
            .modifier("minecraft:powder_snow")
            .unwrap()
            .amount,
        -0.02500000037252903
    );
}

fn item_with_portal_cooldown(store: &mut EntityStore, x: f64, cooldown: i32) -> i32 {
    let id = spawn_item_entity(
        store,
        Vec3d { x, y: 64.0, z: 0.5 },
        stack_of("minecraft:stone", 1),
    )
    .expect("a fresh UUID");
    store.get_mut(id).unwrap().portal_cooldown = cooldown;
    id
}

#[test]
fn portal_cooldown_ticks_only_while_positive_through_tick_one_entity() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let positive = item_with_portal_cooldown(&mut store, 0.5, 2);
    let negative = item_with_portal_cooldown(&mut store, 8.5, -2);
    let mut level = NativeLevel::new();

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, positive));
    assert_eq!(store.get(positive).unwrap().portal_cooldown(), 1);

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, positive));
    assert_eq!(store.get(positive).unwrap().portal_cooldown(), 0);

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, positive));
    assert_eq!(store.get(positive).unwrap().portal_cooldown(), 0);

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, negative));
    assert_eq!(store.get(negative).unwrap().portal_cooldown(), -2);
}

#[test]
fn saved_portal_cooldown_continues_through_tick_entities_after_reload() {
    let mut source = EntityStore::new(0, EntityIdCounter::new());
    item_with_portal_cooldown(&mut source, 0.5, 3);
    let document = write_entity_chunk(&source, (0, 0)).expect("the item save is canonical");

    let mut restored = EntityStore::new(0, EntityIdCounter::new());
    let [id] = read_entity_chunk(&mut restored, &document)
        .expect("the item save restores")
        .try_into()
        .expect("one item restores");
    assert_eq!(restored.get(id).unwrap().portal_cooldown(), 3);

    NativeLevel::new().with_level(|ctx| tick_entities(&mut restored, ctx, true));
    assert_eq!(restored.get(id).unwrap().portal_cooldown(), 2);
}

// ---------------------------------------------------------------------------
// `Entity.baseTick`'s fluid, fire and powder-snow half
// ---------------------------------------------------------------------------

use crate::entity::environment::{
    FluidTag, FAST_LAVA_CURRENT_SCALE, LAVA_CURRENT_SCALE, WATER_CURRENT_SCALE,
};
use bbb_facts::fluid::{
    FLUID_CURRENT_TAPER_HEIGHT as CURRENT_TAPER_HEIGHT, MIN_CURRENT_IMPULSE,
    RESTING_CURRENT_DELTA as RESTING_DELTA,
};

const FEET: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn sloping_pool(family: &str) -> NativeLevel {
    fluid_pool(family, false)
}
fn fluid_pool(family: &str, fast_lava: bool) -> NativeLevel {
    let mut native = if fast_lava {
        let root = std::env::temp_dir().join(format!("bbb-fluid-env-{}", Uuid::new_v4()));
        let pack = root.join("datapacks/fluid");
        std::fs::create_dir_all(pack.join("data/minecraft/dimension_type")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"),r#"{"pack":{"description":"fluid environment","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
        let mut dimension = serde_json::from_slice::<serde_json::Value>(
            &(crate::registry_sync::builtin_datapack_startup_shared()
                .registries
                .element("minecraft:dimension_type", "minecraft:overworld")
                .unwrap())
            .source_json,
        )
        .unwrap();
        dimension["attributes"]["minecraft:gameplay/fast_lava"] = serde_json::json!(true);
        std::fs::write(
            pack.join("data/minecraft/dimension_type/overworld.json"),
            serde_json::to_vec(&dimension).unwrap(),
        )
        .unwrap();
        let startup = bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.join("datapacks")),
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            },
        )
        .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        NativeLevel::with_datapacks(std::sync::Arc::new(startup))
    } else {
        NativeLevel::new()
    };
    native
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id(family), 18);
    native.world.set_block(
        DIMENSION,
        pos(1, 64, 0),
        state_with(family, &[("level", "1")]),
        18,
    );
    native
}

fn item_at(store: &mut EntityStore, position: Vec3d) -> i32 {
    let id =
        spawn_item_entity(store, position, stack_of("minecraft:stone", 1)).expect("a fresh UUID");
    store
        .get_mut(id)
        .unwrap()
        .set_delta_movement(Vec3d::default());
    id
}

fn living_at(store: &mut EntityStore, resource_id: &str, position: Vec3d) -> i32 {
    let entity_type_id = vanilla_entity_type_id_for_resource_id(resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in 26.1 entity type"));
    store
        .spawn_general_living(entity_type_id, position)
        .unwrap_or_else(|| panic!("{resource_id} constructs"))
}

#[test]
fn water_contact_resets_fall_distance_and_applies_the_current_once_per_tick() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut level = sloping_pool("minecraft:water");
    let mut entity = store.detach(id).unwrap();
    entity.fall_distance = 7.0;

    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert_eq!(entity.fall_distance(), 0.0);
    assert!(entity.is_in_water());
    // The slope runs east, so the impulse is the unit east vector scaled.
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: WATER_CURRENT_SCALE,
            y: 0.0,
            z: 0.0
        }
    );

    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert_eq!(entity.delta_movement().x, WATER_CURRENT_SCALE * 2.0);
}

#[test]
fn the_touching_water_flag_rises_and_falls_with_the_scan() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut wet = sloping_pool("minecraft:water");
    let mut dry = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            dry.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    let mut entity = store.detach(id).unwrap();

    assert!(!entity.is_in_water());
    wet.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(entity.is_in_water());
    wet.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(entity.is_in_water());
    dry.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(!entity.is_in_water());
}

fn frog_water_step(tag: bool, passenger: bool, water_state: i32) -> (Vec3d, Vec3d) {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let frog = living_at(&mut store, "minecraft:frog", FEET);
    store.get_mut(frog).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.1,
        z: 0.0,
    });
    if passenger {
        let rider = living_at(&mut store, "minecraft:pig", FEET);
        assert!(store.link_existing_passenger_forced(frog, rider));
    }
    let mut native = NativeLevel::new();
    native
        .world
        .set_block(DIMENSION, pos(0, 64, 0), water_state, 18);
    let members: &[&str] = if tag { &["minecraft:frog"] } else { &[] };
    native.install_tags(registry_tag_snapshot(&[
        (
            "minecraft:entity_type",
            "minecraft:can_float_while_ridden",
            members,
        ),
        (
            "minecraft:fluid",
            "minecraft:water",
            &["minecraft:water", "minecraft:flowing_water"],
        ),
    ]));
    let mut body = store.detach(frog).unwrap();
    native.with_level(|level| {
        assert!(crate::entity::base_tick(&mut body, &mut store, level));
        crate::entity::general_living::frog_travel::travel_in_water(
            &mut body,
            &mut store,
            level,
            Vec3d::default(),
        )
        .unwrap();
    });
    (body.position(), body.delta_movement())
}

#[test]
fn frog_water_travel_uses_riding_tag_and_fluid_jump_threshold_after_damping() {
    let water = state_id("minecraft:water");
    let (plain_position, plain_delta) = frog_water_step(false, true, water);
    let (tagged_position, tagged_delta) = frog_water_step(true, true, water);
    let (_, no_rider_delta) = frog_water_step(true, false, water);
    assert_eq!(plain_position, tagged_position);
    assert_eq!(plain_position.y, FEET.y + 0.1);
    assert_eq!(plain_delta.y, 0.1 * 0.9);
    assert_eq!(tagged_delta.y, plain_delta.y + f64::from(0.04_f32));
    assert_eq!(no_rider_delta, plain_delta);

    let shallow = state_with("minecraft:water", &[("level", "7")]);
    let (_, shallow_delta) = frog_water_step(true, true, shallow);
    assert_eq!(shallow_delta, plain_delta);
}

#[test]
fn the_lava_current_scale_follows_the_fast_lava_dimension_attribute() {
    // A resting entity would take the minimum-impulse floor instead of the
    // scale, so both runs start with a horizontal delta above it.
    let drift = Vec3d {
        x: RESTING_DELTA * 10.0,
        y: 0.0,
        z: 0.0,
    };
    let mut slow = sloping_pool("minecraft:lava");
    let mut fast = fluid_pool("minecraft:lava", true);

    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut entity = store.detach(id).unwrap();
    entity.set_delta_movement(drift);
    slow.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert_eq!(entity.delta_movement().x, drift.x + LAVA_CURRENT_SCALE);

    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut entity = store.detach(id).unwrap();
    entity.set_delta_movement(drift);
    fast.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert_eq!(entity.delta_movement().x, drift.x + FAST_LAVA_CURRENT_SCALE);
    const _: () = assert!(FAST_LAVA_CURRENT_SCALE > LAVA_CURRENT_SCALE);
}

#[test]
fn a_resting_entity_takes_the_minimum_current_impulse_instead_of_the_scale() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut level = sloping_pool("minecraft:lava");
    let mut entity = store.detach(id).unwrap();

    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    const _: () = assert!(LAVA_CURRENT_SCALE < MIN_CURRENT_IMPULSE);
    assert_eq!(entity.delta_movement().x, MIN_CURRENT_IMPULSE);
}

#[test]
fn an_entity_that_is_not_pushed_by_fluid_keeps_its_delta_in_the_same_current() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let pushed = living_at(&mut store, "minecraft:pig", FEET);
    let unpushed = living_at(&mut store, "minecraft:squid", FEET);
    let mut level = sloping_pool("minecraft:water");

    let mut entity = store.detach(pushed).unwrap();
    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert_eq!(entity.delta_movement().x, WATER_CURRENT_SCALE);

    let mut entity = store.detach(unpushed).unwrap();
    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert_eq!(entity.delta_movement().x, 0.0);
    // The trackers are still filled; only the current is skipped.
    assert!(entity.is_in_water());
}

#[test]
fn the_scan_reports_the_fluid_height_over_the_entity_floor_and_the_eye_contact() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:water"), 18);
    let mut entity = store.detach(id).unwrap();

    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    // A source under air keeps its own height, measured from the entity floor.
    let own_height = f64::from(8.0_f32 / 9.0_f32);
    assert_eq!(entity.fluid_height(FluidTag::Water), own_height);
    assert!(entity.fluid_height(FluidTag::Water) > CURRENT_TAPER_HEIGHT);
    assert_eq!(entity.fluid_height(FluidTag::Lava), 0.0);
    assert!(entity.is_eye_in_fluid(FluidTag::Water));
    assert!(!entity.is_eye_in_fluid(FluidTag::Lava));
}

#[test]
fn an_unloaded_neighbour_column_hides_the_whole_fluid_scan() {
    let mut native = NativeLevel::new();
    let edge = native
        .with_level(|level| {
            (0..16)
                .map(|chunk| pos(chunk * 16 + 15, 64, 0))
                .find(|pos| {
                    level.has_chunk_at(*pos)
                        && !level.has_chunk_at(crate::entity::tests::pos(pos.x + 1, 64, 0))
                })
        })
        .expect("a loaded column next to an unavailable column");
    assert!(native
        .world
        .set_block(DIMENSION, edge, state_id("minecraft:water"), 18));
    assert_eq!(
        native.with_level(|level| level.block_state(edge)),
        state_id("minecraft:water")
    );
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(
        &mut store,
        Vec3d {
            x: f64::from(edge.x) + 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let mut body = store.detach(id).unwrap();
    assert!(native.with_level(|level| crate::entity::base_tick(&mut body, &mut store, level)));
    assert!(!body.is_in_water());
    assert_eq!(body.fluid_height(FluidTag::Water), 0.0);
}

#[test]
fn fire_hurts_every_twentieth_tick_while_it_counts_down_each_tick() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut body = store.detach(id).unwrap();
    body.set_remaining_fire_ticks(40);
    let mut native = NativeLevel::new();
    for _ in 0..41 {
        assert!(
            native.with_level(|level| crate::entity::damage::base_tick_from_environment(
                &mut body, &mut store, level
            ))
        );
    }
    let EntityData::Item(item) = body.data() else {
        panic!()
    };
    assert_eq!(item.health(), 3);
    assert_eq!(body.remaining_fire_ticks(), 0);
    assert!(!body
        .get_shared_flag(bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_ON_FIRE));
}

#[test]
fn lava_suppresses_the_fire_damage_without_stopping_the_countdown() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut body = store.detach(id).unwrap();
    body.set_remaining_fire_ticks(21);
    let mut native = sloping_pool("minecraft:lava");
    for _ in 0..2 {
        assert!(
            native.with_level(|level| crate::entity::damage::base_tick_from_environment(
                &mut body, &mut store, level
            ))
        );
    }
    let EntityData::Item(item) = body.data() else {
        panic!()
    };
    assert_eq!(item.health(), 5);
    assert_eq!(body.remaining_fire_ticks(), 19);
    assert!(body.is_in_lava());
    assert!(
        body.get_shared_flag(bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_ON_FIRE)
    );
}

#[test]
fn a_fire_immune_entity_clears_its_fire_instead_of_burning_down() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:blaze", FEET);
    let mut body = store.detach(id).unwrap();
    let health = body.living().unwrap().health();
    body.set_remaining_fire_ticks(40);
    let mut native = NativeLevel::new();
    assert!(native.with_level(|level| crate::entity::base_tick(&mut body, &mut store, level)));
    assert_eq!(body.remaining_fire_ticks(), 0);
    assert_eq!(body.living().unwrap().health(), health);
}

#[test]
fn lava_contact_halves_fall_distance() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut level = sloping_pool("minecraft:lava");
    let mut entity = store.detach(id).unwrap();
    entity.fall_distance = 8.0;

    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert_eq!(entity.fall_distance(), 4.0);
}

#[test]
fn water_contact_allows_sprinting_to_swim() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:water"), 18);
    level
        .world
        .set_block(DIMENSION, pos(0, 65, 0), state_id("minecraft:water"), 18);
    let mut entity = store.detach(id).unwrap();
    entity.set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        true,
    );

    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(entity.is_swimming());

    entity.set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        false,
    );
    level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(!entity.is_swimming());
}

#[test]
fn water_and_rain_and_powder_snow_extinguish_fire_after_the_countdown() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut wet = sloping_pool("minecraft:water");
    store.get_mut(id).unwrap().set_remaining_fire_ticks(21);

    wet.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
    assert_eq!(store.get(id).unwrap().remaining_fire_ticks(), 0);

    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut rain = rainy_level();
    store.get_mut(id).unwrap().set_remaining_fire_ticks(21);
    rain.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
    assert_eq!(store.get(id).unwrap().remaining_fire_ticks(), 0);

    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut snow = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            snow.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    snow.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:powder_snow"),
        18,
    );
    store.get_mut(id).unwrap().set_remaining_fire_ticks(21);
    snow.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
    let entity = store.get(id).unwrap();
    assert_eq!(entity.remaining_fire_ticks(), 0);
    assert!(entity.is_in_powder_snow());
    assert_eq!(entity.ticks_frozen(), 1);
}

#[test]
fn powder_snow_freezes_a_non_leather_living_entity_and_thaws_out_of_it() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut snow = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            snow.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    snow.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:powder_snow"),
        18,
    );
    let mut dry = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            dry.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }

    let mut entity = store.detach(id).unwrap();
    snow.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(snow.with_level(|ctx| apply_inside_for_test(&mut entity, &mut store, ctx)));
    assert_eq!(
        snow.with_level(
            |ctx| crate::entity::environment::apply_living_freeze_suffix(&mut entity, ctx)
        ),
        crate::entity::environment::KnownContact::Known(false)
    );
    assert!(entity.is_in_powder_snow());
    assert_eq!(entity.ticks_frozen(), 1);

    dry.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(dry.with_level(|ctx| apply_inside_for_test(&mut entity, &mut store, ctx)));
    assert_eq!(
        dry.with_level(
            |ctx| crate::entity::environment::apply_living_freeze_suffix(&mut entity, ctx)
        ),
        crate::entity::environment::KnownContact::Known(false)
    );
    assert!(!entity.is_in_powder_snow());
    assert_eq!(entity.ticks_frozen(), 0);
}

#[test]
fn leather_wearables_and_freeze_immune_types_do_not_gain_frozen_ticks() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let pig = living_at(&mut store, "minecraft:pig", FEET);
    store
        .get_mut(pig)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            bbb_protocol::equipment_slots::EquipmentSlot::Head,
            crate::entity::item::plain_stack("minecraft:leather_helmet", 1),
        );
    let bear = living_at(&mut store, "minecraft:polar_bear", FEET);
    let mut snow = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            snow.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    snow.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:powder_snow"),
        18,
    );

    store.get_mut(pig).unwrap().set_ticks_frozen(4);
    let mut entity = store.detach(pig).unwrap();
    snow.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(snow.with_level(|ctx| apply_inside_for_test(&mut entity, &mut store, ctx)));
    assert_eq!(
        snow.with_level(
            |ctx| crate::entity::environment::apply_living_freeze_suffix(&mut entity, ctx)
        ),
        crate::entity::environment::KnownContact::Known(false)
    );
    assert!(entity.is_in_powder_snow());
    assert_eq!(entity.ticks_frozen(), 2);

    store.get_mut(bear).unwrap().set_ticks_frozen(4);
    let mut entity = store.detach(bear).unwrap();
    snow.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx));
    assert!(snow.with_level(|ctx| apply_inside_for_test(&mut entity, &mut store, ctx)));
    assert_eq!(
        snow.with_level(
            |ctx| crate::entity::environment::apply_living_freeze_suffix(&mut entity, ctx)
        ),
        crate::entity::environment::KnownContact::Known(false)
    );
    assert!(entity.is_in_powder_snow());
    assert_eq!(entity.ticks_frozen(), 2);
}

#[test]
fn fully_frozen_living_entities_take_freeze_damage_on_the_fortieth_tick() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut body = store.detach(id).unwrap();
    body.set_ticks_frozen(140);
    body.tick_count = 40;
    let before = body.living().unwrap().health();
    let mut native = NativeLevel::new();
    native.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:powder_snow"),
        18,
    );
    assert!(native.with_level(|level| crate::entity::base_tick(&mut body, &mut store, level)));
    assert!(native.with_level(|level| {
        crate::entity::living::apply_living_inside_block_effects_and_freezing(
            &mut body,
            &mut store,
            level,
            crate::entity::general_living::hurt_from_environment_outcome,
        )
    }));
    assert_eq!(body.living().unwrap().health(), before - 1.0);
}

#[test]
fn in_wall_hurts_when_the_eye_box_hits_a_suffocating_collision() {
    for no_physics in [false, true] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = living_at(&mut store, "minecraft:pig", FEET);
        let mut body = store.detach(id).unwrap();
        body.set_no_physics(no_physics);
        let health = body.living().unwrap().health();
        let EntityData::GeneralLiving(data) = &mut body.data else {
            panic!()
        };
        data.set_no_ai(true);
        let mut native = NativeLevel::new();
        native
            .world
            .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:stone"), 18);
        native
            .world
            .set_block(DIMENSION, pos(0, 65, 0), state_id("minecraft:stone"), 18);
        assert!(native
            .with_level(|level| crate::entity::general_living::tick(&mut body, &mut store, level)));
        assert_eq!(
            body.living().unwrap().health(),
            health - if no_physics { 0.0 } else { 1.0 }
        );
    }
}

fn contact_health_loss(
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
    id: i32,
) -> f32 {
    let mut body = store.detach(id).unwrap();
    let before = body.living().unwrap().health();
    assert!(
        crate::entity::living::apply_living_inside_block_effects_and_freezing(
            &mut body,
            store,
            level,
            crate::entity::general_living::hurt_from_environment_outcome
        )
    );
    let lost = before - body.living().unwrap().health();
    store.attach(body);
    lost
}

#[test]
fn cactus_hurts_a_living_mob_inside_the_block_once() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:cactus"), 18);
    assert_eq!(
        level.with_level(|ctx| contact_health_loss(&mut store, ctx, id)),
        1.0
    );
}

#[test]
fn air_and_cobweb_only_do_not_ask_for_cactus() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let air = living_at(&mut store, "minecraft:pig", FEET);
    let cobweb = living_at(&mut store, "minecraft:pig", FEET);
    let mut dry = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            dry.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    let mut web = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            web.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    web.world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:cobweb"), 18);
    assert_eq!(
        dry.with_level(|ctx| contact_health_loss(&mut store, ctx, air)),
        0.0
    );
    assert_eq!(
        web.with_level(|ctx| contact_health_loss(&mut store, ctx, cobweb)),
        0.0
    );
}

#[test]
fn cactus_beside_the_deflated_box_does_not_hurt() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let bb = store.get(id).unwrap().bounding_box();
    let epsilon = f64::from(1.0e-5_f32);
    let max_x = (bb.max_x - epsilon).ceil() as i32 - 1;
    let cactus_x = max_x + 1;
    assert!(cactus_x > max_x);
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level.world.set_block(
        DIMENSION,
        pos(cactus_x, 64, 0),
        state_id("minecraft:cactus"),
        18,
    );
    assert_eq!(
        level.with_level(|ctx| contact_health_loss(&mut store, ctx, id)),
        0.0
    );
}

#[test]
fn cobweb_makes_a_non_living_entity_stuck_and_scales_the_next_move() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut level = NativeLevel::new();
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:cobweb"), 18);
    store.get_mut(id).unwrap().fall_distance = 6.0;
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.8,
        y: 0.0,
        z: 0.4,
    });

    level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id));
    let entity = store.get_mut(id).unwrap();
    assert_eq!(entity.fall_distance(), 0.0);
    assert_eq!(
        entity.stuck_speed_multiplier(),
        Vec3d {
            x: 0.25,
            y: f64::from(0.05_f32),
            z: 0.25,
        }
    );

    entity.set_delta_movement(Vec3d {
        x: 0.8,
        y: 0.0,
        z: 0.4,
    });
    let before = entity.position();
    let mut entity = store.detach(id).expect("the entity is live");
    let requested = entity.delta_movement();
    level.with_level(|ctx| {
        crate::entity::move_entity(
            &mut entity,
            &mut store,
            ctx,
            crate::entity::MoverType::Self_,
            requested,
        )
    });
    assert_eq!(
        entity.position(),
        Vec3d {
            x: before.x + 0.8 * 0.25,
            y: before.y,
            z: before.z + 0.4 * 0.25,
        }
    );
    assert_eq!(entity.delta_movement(), Vec3d::default());
    assert_eq!(entity.stuck_speed_multiplier(), Vec3d::default());
}

#[test]
fn spiders_ignore_cobwebs_and_the_wither_is_never_stuck() {
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    for (block, kind, stuck) in [
        ("minecraft:cobweb", "minecraft:spider", false),
        ("minecraft:cobweb", "minecraft:cave_spider", false),
        ("minecraft:cobweb", "minecraft:wither", false),
        ("minecraft:powder_snow", "minecraft:wither", false),
        ("minecraft:powder_snow", "minecraft:spider", true),
    ] {
        level
            .world
            .set_block(DIMENSION, pos(0, 64, 0), state_id(block), 18);
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = living_at(&mut store, kind, FEET);
        let mut entity = store.detach(id).unwrap();
        level.with_level(|ctx| apply_inside_for_test(&mut entity, &mut store, ctx));
        assert_eq!(
            entity.stuck_speed_multiplier() != Vec3d::default(),
            stuck,
            "{kind} in {block}"
        );
    }
}

#[test]
fn cobweb_weaving_uses_the_living_effect_store() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut level = NativeLevel::new();
    let mut entity = store.detach(id).unwrap();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:cobweb"), 18);

    assert!(level.with_level(|ctx| apply_inside_for_test(&mut entity, &mut store, ctx)));
    assert_eq!(
        entity.stuck_speed_multiplier(),
        Vec3d {
            x: 0.25,
            y: f64::from(0.05_f32),
            z: 0.25,
        }
    );

    entity.living_mut().unwrap().add_active_effect(
        crate::entity::ActiveMobEffect::new("minecraft:weaving", 200, 0).unwrap(),
    );
    assert!(level.with_level(|ctx| apply_inside_for_test(&mut entity, &mut store, ctx)));
    assert_eq!(
        entity.stuck_speed_multiplier(),
        Vec3d {
            x: 0.5,
            y: 0.25,
            z: 0.5,
        }
    );
}

fn sliding_item(store: &mut EntityStore, delta_y: f64) -> i32 {
    let id = item_at(
        store,
        Vec3d {
            x: -0.1,
            y: 64.2,
            z: 0.5,
        },
    );
    let entity = store.get_mut(id).unwrap();
    entity.on_ground = false;
    entity.fall_distance = 5.0;
    entity.set_delta_movement(Vec3d {
        x: 0.2,
        y: delta_y,
        z: 0.0,
    });
    id
}

#[test]
fn honey_slide_throttles_a_fast_fall_and_resets_fall_distance() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = sliding_item(&mut store, -0.3);
    let mut level = NativeLevel::new();
    level.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:honey_block"),
        18,
    );
    let entity = store.get_mut(id).unwrap();

    assert!(level
        .with_level(|ctx| crate::entity::environment::apply_inside_block_environment(entity, ctx)));
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.04422382788003191,
            y: -0.12740000247955321,
            z: 0.0,
        }
    );
    assert_eq!(entity.fall_distance(), 0.0);
}

#[test]
fn honey_slide_keeps_horizontal_speed_when_the_fall_is_already_slow() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = sliding_item(&mut store, -0.18);
    let mut level = NativeLevel::new();
    level.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:honey_block"),
        18,
    );
    let entity = store.get_mut(id).unwrap();

    assert!(level
        .with_level(|ctx| crate::entity::environment::apply_inside_block_environment(entity, ctx)));
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.2,
            y: -0.12740000247955321,
            z: 0.0,
        }
    );
    assert_eq!(entity.fall_distance(), 0.0);
}

#[test]
fn honey_does_not_slide_when_grounded_centered_or_too_slow() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let mut honey = NativeLevel::new();
    honey.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:honey_block"),
        18,
    );
    let falling = Vec3d {
        x: 0.2,
        y: -0.3,
        z: 0.0,
    };

    let grounded = sliding_item(&mut store, -0.3);
    store.get_mut(grounded).unwrap().on_ground = true;
    honey.with_level(|ctx| {
        crate::entity::environment::apply_inside_block_environment(
            store.get_mut(grounded).unwrap(),
            ctx,
        )
    });
    assert_eq!(store.get(grounded).unwrap().delta_movement(), falling);
    assert_eq!(store.get(grounded).unwrap().fall_distance(), 5.0);

    let centered = item_at(&mut store, FEET);
    store.get_mut(centered).unwrap().on_ground = false;
    store.get_mut(centered).unwrap().fall_distance = 5.0;
    store.get_mut(centered).unwrap().set_delta_movement(falling);
    honey.with_level(|ctx| {
        crate::entity::environment::apply_inside_block_environment(
            store.get_mut(centered).unwrap(),
            ctx,
        )
    });
    assert_eq!(store.get(centered).unwrap().delta_movement(), falling);
    assert_eq!(store.get(centered).unwrap().fall_distance(), 5.0);

    let slow = sliding_item(&mut store, -0.05);
    honey.with_level(|ctx| {
        crate::entity::environment::apply_inside_block_environment(
            store.get_mut(slow).unwrap(),
            ctx,
        )
    });
    assert_eq!(
        store.get(slow).unwrap().delta_movement(),
        Vec3d {
            x: 0.2,
            y: -0.05,
            z: 0.0,
        }
    );
    assert_eq!(store.get(slow).unwrap().fall_distance(), 5.0);
}

fn berry_state(age: i32) -> i32 {
    let age = age.to_string();
    state_with("minecraft:sweet_berry_bush", &[("age", &age)])
}

fn berry_level(age: i32) -> NativeLevel {
    {
        let mut native = NativeLevel::new();
        for x in -3..=3 {
            for z in -3..=3 {
                native.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    18,
                );
            }
        }
        native
            .world
            .set_block(DIMENSION, pos(0, 64, 0), berry_state(age), 18);
        native
    }
}

fn berry_stuck() -> Vec3d {
    Vec3d {
        x: f64::from(0.8_f32),
        y: 0.75,
        z: f64::from(0.8_f32),
    }
}

fn shift_living_horizontally(store: &mut EntityStore, id: i32, dx: f64, dz: f64) {
    let position = store.get(id).unwrap().position();
    store.get_mut(id).unwrap().set_pos(Vec3d {
        x: position.x + dx,
        y: position.y,
        z: position.z + dz,
    });
}

#[test]
fn fox_and_bee_skip_living_berry_inside() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let fox = living_at(&mut store, "minecraft:fox", FEET);
    let bee = living_at(&mut store, "minecraft:bee", FEET);
    let threshold = f64::from(0.003_f32);
    shift_living_horizontally(&mut store, fox, threshold, 0.0);
    shift_living_horizontally(&mut store, bee, 0.0, threshold);
    let mut fox_level = berry_level(3);
    let mut bee_level = berry_level(3);
    assert_eq!(
        fox_level.with_level(|ctx| contact_health_loss(&mut store, ctx, fox)),
        0.0
    );
    assert_eq!(
        bee_level.with_level(|ctx| contact_health_loss(&mut store, ctx, bee)),
        0.0
    );
    assert_eq!(
        store.get(fox).unwrap().stuck_speed_multiplier(),
        Vec3d::default()
    );
    assert_eq!(
        store.get(bee).unwrap().stuck_speed_multiplier(),
        Vec3d::default()
    );
}

#[test]
fn age_zero_living_berry_sticks_without_hurt() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    shift_living_horizontally(&mut store, id, f64::from(0.003_f32), 0.0);
    let mut level = berry_level(0);
    assert_eq!(
        level.with_level(|ctx| contact_health_loss(&mut store, ctx, id)),
        0.0
    );
    assert_eq!(
        store.get(id).unwrap().stuck_speed_multiplier(),
        berry_stuck()
    );
}

#[test]
fn tiny_living_berry_movement_does_not_hurt() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    shift_living_horizontally(&mut store, id, 0.002, 0.0);
    let mut level = berry_level(2);
    assert_eq!(
        level.with_level(|ctx| contact_health_loss(&mut store, ctx, id)),
        0.0
    );
    assert_eq!(
        store.get(id).unwrap().stuck_speed_multiplier(),
        berry_stuck()
    );
}

#[test]
fn living_berry_hurts_when_horizontal_delta_reaches_threshold() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let along_x = living_at(&mut store, "minecraft:pig", FEET);
    let along_z = living_at(&mut store, "minecraft:pig", FEET);
    let threshold = f64::from(0.003_f32);
    shift_living_horizontally(&mut store, along_x, threshold, 0.0);
    shift_living_horizontally(&mut store, along_z, 0.0, threshold);
    let mut x_level = berry_level(1);
    let mut z_level = berry_level(3);
    assert_eq!(
        x_level.with_level(|ctx| contact_health_loss(&mut store, ctx, along_x)),
        1.0
    );
    assert_eq!(
        z_level.with_level(|ctx| contact_health_loss(&mut store, ctx, along_z)),
        1.0
    );
    assert_eq!(
        store.get(along_x).unwrap().stuck_speed_multiplier(),
        berry_stuck()
    );
    assert_eq!(
        store.get(along_z).unwrap().stuck_speed_multiplier(),
        berry_stuck()
    );
}

#[test]
fn item_is_not_a_living_berry_target() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let entity = store.get_mut(id).unwrap();
    let position = entity.position();
    entity.set_pos(Vec3d {
        x: position.x + f64::from(0.003_f32),
        y: position.y,
        z: position.z,
    });
    let mut level = berry_level(3);
    assert!(level
        .with_level(|ctx| crate::entity::environment::apply_inside_block_environment(entity, ctx)));
    assert_eq!(entity.stuck_speed_multiplier(), Vec3d::default());
}

fn apply_inside_for_test(
    entity: &mut crate::entity::Entity,
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
) -> bool {
    crate::entity::inside_blocks::apply_living_inside_block_effects(
        entity,
        level,
        Some(store),
        |entity, level, store, callback| {
            crate::entity::living::apply_detached_inside_callback(
                entity,
                store.expect("actual store"),
                level,
                callback,
            )
        },
    )
}

#[test]
fn swept_inside_block_hits_intermediate_cactus_and_cobweb() {
    for negative in [false, true] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = living_at(&mut store, "minecraft:pig", FEET);
        let mut body = store.detach(id).unwrap();
        let before = body.living().unwrap().health();
        let to = Vec3d {
            x: if negative { -7.5 } else { 8.5 },
            ..FEET
        };
        body.record_movement(crate::entity::inside_blocks::EntityMovement::direct(
            FEET, to,
        ));
        body.set_pos(to);
        let mut native = NativeLevel::new();
        native.world.set_block(
            DIMENSION,
            pos(if negative { -3 } else { 3 }, 64, 0),
            state_id("minecraft:cactus"),
            18,
        );
        if !negative {
            native
                .world
                .set_block(DIMENSION, pos(5, 64, 0), state_id("minecraft:cobweb"), 18);
        }
        assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
        assert_eq!(body.living().unwrap().health(), before - 1.0);
        if !negative {
            assert_eq!(
                body.stuck_speed_multiplier(),
                Vec3d {
                    x: 0.25,
                    y: f64::from(0.05_f32),
                    z: 0.25
                }
            );
        }
    }
}

#[test]
fn stationary_inside_block_scans_the_final_box() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut body = store.detach(id).unwrap();
    let before = body.living().unwrap().health();
    let mut native = NativeLevel::new();
    native
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:cactus"), 18);
    assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
    assert_eq!(body.living().unwrap().health(), before - 1.0);
}

#[test]
fn swept_bubble_column_contact_lifts_a_living_body() {
    for (distance, expected) in [(0.99, 0.0), (1.0, 0.06)] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = living_at(&mut store, "minecraft:pig", FEET);
        let mut body = store.detach(id).unwrap();
        let to = Vec3d {
            x: FEET.x + distance,
            ..FEET
        };
        body.record_movement(crate::entity::inside_blocks::EntityMovement::direct(
            FEET, to,
        ));
        body.set_pos(to);
        let mut native = NativeLevel::new();
        native.world.set_block(
            DIMENSION,
            pos(0, 64, 0),
            state_with("minecraft:bubble_column", &[("drag", "false")]),
            18,
        );
        native
            .world
            .set_block(DIMENSION, pos(0, 65, 0), state_id("minecraft:water"), 18);
        assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
        assert_eq!(body.delta_movement().y, expected);
    }
}

fn apply_portal_contact(
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
    id: i32,
) -> bool {
    let mut entity = store.detach(id).expect("the living entity is in the store");
    let resolved = crate::entity::living::apply_living_inside_block_effects_and_freezing(
        &mut entity,
        store,
        level,
        crate::entity::general_living::hurt_from_environment_outcome,
    );
    store.attach(entity);
    resolved
}

#[test]
fn living_nether_and_end_portal_contacts_create_typed_processors() {
    for (resource_id, kind) in [
        (
            "minecraft:nether_portal",
            crate::entity::portal::PortalKind::Nether,
        ),
        (
            "minecraft:end_portal",
            crate::entity::portal::PortalKind::End,
        ),
    ] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = living_at(&mut store, "minecraft:pig", FEET);
        let entry = pos(0, 64, 0);
        let mut level = NativeLevel::new();
        level
            .world
            .set_block(DIMENSION, entry, state_id(resource_id), 18);

        assert!(level.with_level(|ctx| apply_portal_contact(&mut store, ctx, id)));

        let processor = store
            .get(id)
            .unwrap()
            .portal_process()
            .expect("the portal contact creates a processor");
        assert_eq!(processor.kind(), kind);
        assert_eq!(processor.entry_position(), entry);
    }
}

/// `EndPortalBlock.java:34,56-58`: only the 6/16..12/16 band of the cell is inside.
#[test]
fn an_end_portal_only_takes_a_body_inside_its_band() {
    for (feet_y, inside) in [(64.0, true), (64.76, false)] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = living_at(&mut store, "minecraft:pig", Vec3d { y: feet_y, ..FEET });
        let mut level = NativeLevel::new();
        level.world.set_block(
            DIMENSION,
            pos(0, 64, 0),
            state_id("minecraft:end_portal"),
            18,
        );
        level.with_level(|ctx| apply_portal_contact(&mut store, ctx, id));
        assert_eq!(
            store.get(id).unwrap().portal_process().is_some(),
            inside,
            "feet at {feet_y}"
        );
    }
}

#[test]
fn living_passenger_portal_contact_succeeds_without_a_processor() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let vehicle = living_at(&mut store, "minecraft:chicken", FEET);
    let passenger = living_at(&mut store, "minecraft:pig", FEET);
    assert!(store.link_existing_passenger(vehicle, passenger));
    let mut level = NativeLevel::new();
    level.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:end_portal"),
        18,
    );

    assert!(level.with_level(|ctx| apply_portal_contact(&mut store, ctx, passenger)));
    assert!(store.get(passenger).unwrap().portal_process().is_none());
}

#[test]
fn portal_contact_uses_the_direct_external_player_delay() {
    let (mut native, _sender, _receiver) = joined_level();
    let player = native
        .world
        .player_entity_id_for_test(crate::connection::ConnectionId(1));
    let mut store = EntityStore::new(0, unoccupied_ids(&native));
    let id = living_at(&mut store, "minecraft:chicken", FEET);
    assert!(store.attach_test_player(player, id, crate::entity::ExternalPlayerCanRide::Allowed));
    store.get_mut(id).unwrap().portal_cooldown = 1;
    native.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:nether_portal"),
        18,
    );
    assert!(native.with_level(|level| apply_portal_contact(&mut store, level, id)));
    assert_eq!(store.get(id).unwrap().portal_cooldown(), 10);
    assert!(store.get(id).unwrap().portal_process().is_none());
    let mut ordinary = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut ordinary, "minecraft:pig", FEET);
    ordinary.get_mut(id).unwrap().portal_cooldown = 1;
    let mut native = NativeLevel::new();
    native.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:nether_portal"),
        18,
    );
    assert!(native.with_level(|level| apply_portal_contact(&mut ordinary, level, id)));
    assert_eq!(ordinary.get(id).unwrap().portal_cooldown(), 300);
    assert!(ordinary.get(id).unwrap().portal_process().is_none());
}

#[test]
fn living_honey_slide_emits_the_slide_sound_and_event() {
    for silent in [false, true] {
        let mut store = EntityStore::new(24, EntityIdCounter::new());
        let id = living_at(
            &mut store,
            "minecraft:cow",
            Vec3d {
                x: -0.44,
                y: 64.2,
                z: 0.5,
            },
        );
        let mut body = store.detach(id).unwrap();
        body.set_silent(silent);
        body.on_ground = false;
        body.set_delta_movement(Vec3d {
            x: 0.2,
            y: -0.3,
            z: 0.0,
        });
        body.fall_distance = 5.0;
        let mut native = NativeLevel::new();
        native.world.set_block(
            DIMENSION,
            pos(0, 64, 0),
            state_id("minecraft:honey_block"),
            18,
        );
        assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
        assert_eq!(body.fall_distance(), 0.0);
        let mut events = store.drain_lifecycle_events();
        for _ in 0..100 {
            native
                .with_level(|level| {
                    crate::entity::inside_blocks::honey_slide_effects(&body, &mut store, level)
                })
                .unwrap();
            events.extend(store.drain_lifecycle_events());
        }
        assert!(events.iter().any(|event|matches!(event,EntityLifecycleEvent::Broadcast{packet:crate::connection::ClientboundPlayPacket::EntityEvent(event),..} if event.entity_id==id&&event.event_id==53)));
        let sound = bbb_protocol::packets::SoundEventHolder::Reference {
            registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:block.honey_block.slide",
            )
            .unwrap(),
        };
        assert_eq!(events.iter().any(|event|matches!(event,EntityLifecycleEvent::Sound{sound:packet} if packet.sound==sound)),!silent);
    }
}

#[test]
fn bubble_column_above_emits_four_particles() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:cow", FEET);
    store.drain_lifecycle_events();
    let mut entity = store.detach(id).unwrap();
    let bubble = state_with("minecraft:bubble_column", &[("drag", "false")]);
    let mut level = {
        let mut native = NativeLevel::new();
        native.world.set_block(DIMENSION, pos(0, 64, 0), bubble, 18);
        native
    };

    assert!(level.with_level(|ctx| {
        crate::entity::living::apply_living_inside_block_effects_and_freezing(
            &mut entity,
            &mut store,
            ctx,
            crate::entity::general_living::hurt_from_environment_outcome,
        )
    }));
    store.attach(entity);

    assert_eq!(
        store
            .drain_lifecycle_events()
            .iter()
            .filter(|event| matches!(event, EntityLifecycleEvent::Particles { .. }))
            .count(),
        4
    );
}

#[test]
fn inside_block_damage_kills_a_living_body() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:ravager", FEET);
    let mut body = store.detach(id).unwrap();
    body.living_mut().unwrap().set_health(1.0);
    let mut native = NativeLevel::new();
    native
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:fire"), 18);
    native.world.set_block(
        DIMENSION,
        pos(1, 64, 0),
        state_id("minecraft:soul_fire"),
        18,
    );
    assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
    assert_eq!(body.living().unwrap().health(), 0.0);
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event
            == crate::level_event::VanillaGameEvent::ENTITY_DIE
            && event.recorded.source_entity_id == Some(id)
            && event.recorded.pos == pos(0, 64, 0)));
}

#[test]
fn powder_snow_sticks_living_entities_and_breaks_when_on_fire() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut body = store.detach(id).unwrap();
    body.set_remaining_fire_ticks(20);
    let powder = pos(0, 64, 0);
    let mut native = NativeLevel::new();
    native
        .world
        .set_block(DIMENSION, powder, state_id("minecraft:powder_snow"), 18);
    assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
    assert_eq!(
        body.stuck_speed_multiplier(),
        Vec3d {
            x: f64::from(0.9_f32),
            y: 1.5,
            z: f64::from(0.9_f32)
        }
    );
    assert_eq!(
        native.with_level(|level| level.block_state(powder)),
        state_id("minecraft:air")
    );
    assert!(native
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.event_type == crate::level_event::LEVEL_EVENT_PARTICLES_DESTROY_BLOCK
                && event.pos == powder
                && event.data == state_id("minecraft:powder_snow")
        ));
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event
            == crate::level_event::VanillaGameEvent::BLOCK_DESTROY
            && event.recorded.pos == powder
            && event.recorded.source_entity_id.is_none()
            && event.recorded.affected_state == Some(state_id("minecraft:powder_snow"))));
    assert_eq!(body.remaining_fire_ticks(), 0);
}

#[test]
fn falling_entity_inside_block_only_meets_powder_snow_below_its_top_tenth() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(
        &mut store,
        "minecraft:pig",
        Vec3d {
            x: 0.5,
            y: 64.95,
            z: 0.5,
        },
    );
    let mut entity = store.detach(id).unwrap();
    entity.fall_distance = 3.0;
    let mut level = NativeLevel::new();
    level.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:powder_snow"),
        18,
    );

    assert!(level.with_level(|ctx| apply_inside_for_test(&mut entity, &mut store, ctx)));
    assert!(!entity.is_in_powder_snow());
    assert_eq!(entity.ticks_frozen(), 0);
}

#[test]
fn honey_ordinary_item_slides() {
    let mut store = EntityStore::new(24, EntityIdCounter::new());
    let id = item_at(
        &mut store,
        Vec3d {
            x: -0.08,
            y: 64.2,
            z: 0.5,
        },
    );
    let mut entity = store.detach(id).unwrap();
    entity.on_ground = false;
    entity.fall_distance = 7.0;
    let delta = Vec3d {
        x: 0.2,
        y: -0.3,
        z: 0.4,
    };
    entity.set_delta_movement(delta);
    let mut level = {
        let mut native = NativeLevel::new();
        native.world.set_block(
            DIMENSION,
            pos(0, 64, 0),
            state_id("minecraft:honey_block"),
            18,
        );
        native
    };
    assert!(
        level.with_level(|ctx| crate::entity::apply_nonliving_inside_block_effects(
            &mut entity,
            ctx,
            Some(&mut store)
        ))
    );
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.04422382788003191,
            y: -0.12740000247955321,
            z: 0.08844765576006382
        }
    );
    assert_eq!(entity.fall_distance(), 0.0);
}

#[test]
fn honey_effects_allow_boat_minecart_and_tnt_ticks_to_finish() {
    for family in ["boat", "minecart", "tnt"] {
        let mut store = EntityStore::new(24, EntityIdCounter::new());
        let at = Vec3d {
            x: 0.5,
            y: 64.6,
            z: 0.5,
        };
        let id = match family {
            "boat" => crate::entity::boat::spawn_boat(
                &mut store,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
                at,
                0.0,
            ),
            "minecart" => crate::entity::minecart::spawn_minecart(&mut store, at, 0.0),
            "tnt" => crate::entity::tnt::spawn_tnt(&mut store, &GameRandom::with_seed(1), at, None),
            _ => unreachable!(),
        }
        .unwrap();
        let mut entity = store.detach(id).unwrap();
        entity.set_pos(Vec3d {
            x: 0.03125 - f64::from(entity.bb_width() / 2.0),
            ..at
        });
        entity.set_old_pos_and_rot();
        entity.fall_distance = 6.0;
        entity.set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.3,
            z: 0.0,
        });
        store.drain_lifecycle_events();
        let mut level = {
            let mut native = NativeLevel::new();
            native.world.set_block(
                DIMENSION,
                pos(0, 64, 0),
                state_id("minecraft:honey_block"),
                18,
            );
            native
        };
        let finished = match family {
            "boat" => level
                .with_level(|ctx| {
                    crate::entity::boat::tick(&mut entity, &mut store, ctx, false, false)
                })
                .is_some(),
            "minecart" => {
                level.with_level(|ctx| crate::entity::minecart::tick(&mut entity, &mut store, ctx))
            }
            "tnt" => level.with_level(|ctx| crate::entity::tnt::tick(&mut entity, &mut store, ctx)),
            _ => unreachable!(),
        };
        assert!(finished, "{family}");
        assert_eq!(entity.fall_distance(), 0.0, "{family}");
        assert!(entity.delta_movement().y > -0.14, "{family}");
        if let EntityData::Tnt(data) = entity.data() {
            assert_eq!(data.fuse(), crate::entity::tnt::DEFAULT_FUSE_TIME - 1);
        }
        let mut outputs = store.drain_lifecycle_events();
        for _ in 0..100 {
            level
                .with_level(|ctx| {
                    crate::entity::inside_blocks::honey_slide_effects(&entity, &mut store, ctx)
                })
                .unwrap();
            outputs.extend(store.drain_lifecycle_events());
        }
        assert!(outputs.iter().any(|event|matches!(event,EntityLifecycleEvent::Sound{sound} if sound.sound==bbb_protocol::packets::SoundEventHolder::Reference{registry_id:bbb_protocol::sound_event_types::vanilla_sound_event_registry_id("minecraft:block.honey_block.slide").unwrap()})),"{family}");
        assert!(outputs.iter().any(|event|matches!(event,EntityLifecycleEvent::Broadcast{packet:crate::connection::ClientboundPlayPacket::EntityEvent(event),..} if event.entity_id==id&&event.event_id==53)),"{family}");
    }
}

mod eyeblossom;

#[test]
fn piston_wither_rose_withers_living_outside_peaceful() {
    use bbb_protocol::difficulties::Difficulty;
    for difficulty in [Difficulty::Peaceful, Difficulty::Normal] {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        store.set_difficulty(difficulty);
        let id = living_at(&mut store, "minecraft:cow", FEET);
        let mut entity = store.detach(id).unwrap();
        let mut level = {
            let mut native = NativeLevel::new();
            native.world.set_block(
                DIMENSION,
                pos(1, 64, 0),
                state_id("minecraft:wither_rose"),
                18,
            );
            native
        };
        level.with_level(|ctx| {
            crate::entity::movement::move_entity_by_piston(
                &mut entity,
                &mut store,
                ctx,
                Vec3d {
                    x: 0.4,
                    y: 0.0,
                    z: 0.0,
                },
                bbb_protocol::packets::Direction::East,
            )
        });
        assert_eq!(
            entity
                .living()
                .unwrap()
                .active_effect("minecraft:wither")
                .map(|effect| effect.duration()),
            (difficulty != Difficulty::Peaceful).then_some(40),
        );
    }
}

mod extinguish;
mod piston_contacts;

mod virtual_methods;

mod item_fire_immunity;

mod fall_fluid_refresh;

mod crop_contacts;
mod fox_targets;

mod bee_targets;
mod enderman_freeze;

mod water_sensitive;
mod water_splash;

pub(super) fn joined_level() -> (
    NativeLevel,
    crate::mailbox::WorldCommandSender,
    crate::mailbox::WorldCommandReceiver,
) {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        100,
        "gamemode survival alice",
    );
    native
        .world
        .set_player_position_for_test(crate::connection::ConnectionId(1), FEET);
    native.world.tick(Vec::new());
    (native, sender, receiver)
}
pub(super) fn unoccupied_ids(native: &NativeLevel) -> EntityIdCounter {
    let ids = EntityIdCounter::new();
    let highest = native
        .world
        .publication()
        .players
        .iter()
        .map(|p| p.entity_id)
        .chain(native.world.stored_entity_ids_for_test(DIMENSION))
        .max()
        .unwrap_or(0);
    for _ in 0..highest {
        ids.next_id();
    }
    ids
}

fn rainy_level() -> NativeLevel {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(&sender, &receiver, &mut native.world, 1, "weather rain");
    for _ in 0..100 {
        if native.with_level(|level| level.is_raining_at(pos(0, 64, 0))) {
            return native;
        }
        native.world.tick(Vec::new());
    }
    panic!("rain reaches the open loaded scene");
}

#[test]
fn a_long_contact_movement_keeps_the_final_cobweb_effect() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut body = store.detach(id).unwrap();
    let to = Vec3d { x: 25.5, ..FEET };
    body.record_movement(crate::entity::inside_blocks::EntityMovement::direct(
        FEET, to,
    ));
    body.set_pos(to);
    let mut native = NativeLevel::new();
    assert!(native
        .world
        .set_block(DIMENSION, pos(20, 64, 0), state_id("minecraft:cactus"), 18));
    assert!(native
        .world
        .set_block(DIMENSION, pos(25, 64, 0), state_id("minecraft:cobweb"), 18));
    assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
    assert_ne!(body.stuck_speed_multiplier(), Vec3d::default());
}
#[test]
fn swept_web_and_powder_contacts_leave_the_body_frozen() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let from = Vec3d { y: 64.95, ..FEET };
    let id = living_at(&mut store, "minecraft:pig", from);
    let mut body = store.detach(id).unwrap();
    body.fall_distance = 3.0;
    let to = Vec3d { x: 3.5, ..from };
    body.record_movement(crate::entity::inside_blocks::EntityMovement::direct(
        from, to,
    ));
    body.set_pos(to);
    let mut native = NativeLevel::new();
    native
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:cobweb"), 18);
    native.world.set_block(
        DIMENSION,
        pos(2, 64, 0),
        state_id("minecraft:powder_snow"),
        18,
    );
    assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
    assert!(body.is_in_powder_snow());
    assert_eq!(body.ticks_frozen(), 1);
}
#[test]
fn axis_movement_contacts_hurt_and_apply_wither() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    store.set_difficulty(bbb_protocol::difficulties::Difficulty::Normal);
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut body = store.detach(id).unwrap();
    let before = body.living().unwrap().health();
    let to = Vec3d {
        x: 2.5,
        y: 66.0,
        z: 1.5,
    };
    body.record_movement(
        crate::entity::inside_blocks::EntityMovement::axis_dependent(
            FEET,
            to,
            Vec3d {
                x: 2.0,
                y: 2.0,
                z: 1.0,
            },
        ),
    );
    body.set_pos(to);
    let mut native = NativeLevel::new();
    native
        .world
        .set_block(DIMENSION, pos(0, 66, 0), state_id("minecraft:cactus"), 18);
    native.world.set_block(
        DIMENSION,
        pos(1, 66, 0),
        state_id("minecraft:wither_rose"),
        18,
    );
    assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
    assert_eq!(body.living().unwrap().health(), before - 1.0);
    let effect = body
        .living()
        .unwrap()
        .active_effect("minecraft:wither")
        .unwrap();
    assert_eq!((effect.duration(), effect.amplifier()), (40, 0));
}
#[test]
fn a_body_moved_past_its_recorded_segment_still_touches_the_final_cactus() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    let mut body = store.detach(id).unwrap();
    body.set_no_physics(true);
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        crate::entity::move_entity(
            &mut body,
            &mut store,
            level,
            crate::entity::MoverType::Self_,
            Vec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        )
    });
    assert_eq!(body.position(), Vec3d { x: 1.5, ..FEET });
    body.set_no_physics(false);
    let from = body.position();
    let to = Vec3d {
        x: from.x + 3.0,
        ..from
    };
    body.record_movement(crate::entity::inside_blocks::EntityMovement::direct(
        from,
        Vec3d {
            x: from.x + 1.0,
            ..from
        },
    ));
    body.set_pos(to);
    native.world.set_block(
        DIMENSION,
        pos(to.x.floor() as i32, 64, 0),
        state_id("minecraft:cactus"),
        18,
    );
    let before = body.living().unwrap().health();
    assert!(native.with_level(|level| apply_inside_for_test(&mut body, &mut store, level)));
    assert_eq!(body.living().unwrap().health(), before - 1.0);
}

#[test]
fn polar_bears_and_skeleton_horses_keep_more_speed_in_water() {
    for (resource_id, slow_down) in [
        ("minecraft:cow", 0.8_f32),
        ("minecraft:polar_bear", 0.98),
        ("minecraft:skeleton_horse", 0.96),
    ] {
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = living_at(&mut store, resource_id, FEET);
        store.get_mut(id).unwrap().set_delta_movement(Vec3d {
            x: 0.1,
            y: 0.0,
            z: 0.0,
        });
        let mut native = NativeLevel::new();
        for y in 64..=66 {
            native
                .world
                .set_block(DIMENSION, pos(0, y, 0), state_id("minecraft:water"), 18);
        }
        let mut body = store.detach(id).unwrap();
        native.with_level(|level| {
            crate::entity::living::travel_in_water(&mut body, &mut store, level, Vec3d::default())
                .unwrap();
        });
        assert_eq!(
            body.delta_movement().x,
            0.1 * f64::from(slow_down),
            "{resource_id}"
        );
    }
}
