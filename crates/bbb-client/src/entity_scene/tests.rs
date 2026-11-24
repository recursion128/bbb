use super::*;
use bbb_client_world::{
    vanilla_entity_metadata_enderman_render_accessors_for_entity_type,
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type, ChunkColumn, ChunkPos,
    ChunkState, EntityPickBoundsState, EntityVec3, LightData, RegistryPacketEntry,
    VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID, VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_data_serializers::*;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_SITTING_ID,
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::packets::{
    AddEntity, AttributeSnapshot, ChatFormatting, CommonPlayerSpawnInfo, DataComponentPatchSummary,
    EntityAnimation, EntityDataValue, EntityEvent, EntityPositionSync, EquipmentSlot,
    EquipmentSlotUpdate, GameProfile, GameProfileProperty, GameType, ItemEnchantmentSummary,
    ItemStackSummary, MinecartStep, MoveMinecartAlongTrack, PlayLogin, PlayTime, PlayerInfoAction,
    PlayerInfoEntry, PlayerInfoUpdate, PlayerTeamMethod, PlayerTeamParameters, RegistryTags,
    SetCamera, SetEntityData, SetEquipment, SetPassengers, SetPlayerTeam, SwingAnimationSummary,
    SwingAnimationTypeSummary, TagNetworkPayload, TeamCollisionRule, TeamVisibility,
    UpdateAttributes, UpdateTags, Vec3d,
};
use uuid::Uuid;

mod body_rotation;
mod player_arm_poses;
mod registry_fixtures;

use registry_fixtures::{crossbow_charge_time_enchantment_nbt, install_piglins_zombify_dimension};

fn install_swing_animation_defaults(world: &mut WorldStore, defaults: &[(i32, &str, i32)]) {
    world.set_default_item_component_codec_values(
        defaults
            .iter()
            .map(|&(item_id, animation_type, duration)| {
                (
                    item_id,
                    std::collections::BTreeMap::from([(
                        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                        serde_json::json!({"type": animation_type, "duration": duration}),
                    )]),
                )
            })
            .collect(),
    );
    world.rebuild_item_component_defaults().unwrap();
}

fn insert_loaded_entity_chunk(world: &mut WorldStore, pos: ChunkPos) {
    world.insert_decoded_chunk(ChunkColumn {
        pos,
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
}

fn bee_metadata_accessors() -> bbb_client_world::VanillaBeeMetadataAccessors {
    bbb_client_world::vanilla_entity_metadata_bee_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_BEE_ID,
    )
    .expect("generated Bee metadata accessors")
}

fn panda_metadata_accessors() -> bbb_client_world::VanillaPandaMetadataAccessors {
    vanilla_entity_metadata_panda_accessors_for_entity_type(VANILLA_ENTITY_TYPE_PANDA_ID)
        .expect("generated Panda metadata accessors must resolve")
}

fn armor_stand_metadata_accessors() -> bbb_client_world::VanillaArmorStandMetadataAccessors {
    vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    )
    .expect("generated ArmorStand metadata accessors must resolve")
}

fn horse_variant_data(packed_variant: i32) -> EntityDataValue {
    let (data_id, default) = vanilla_entity_metadata_horse_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_HORSE_ID,
    )
    .expect("generated Horse variant accessor must resolve");
    assert_eq!(default, 0);
    protocol_int_data(data_id, packed_variant)
}

fn chested_horse_chest_data(entity_type_id: i32, has_chest: bool) -> EntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_chested_horse_chest_accessor_for_entity_type(entity_type_id)
            .expect("generated AbstractChestedHorse chest accessor must resolve");
    assert!(!default);
    protocol_bool_data(data_id, has_chest)
}

fn baby_data(entity_type_id: i32, is_baby: bool) -> EntityDataValue {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
            .expect("generated nearest baby accessor must resolve");
    assert!(!default);
    protocol_bool_data(data_id, is_baby)
}

fn abstract_villager_unhappy_data(entity_type_id: i32, counter: i32) -> EntityDataValue {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated inherited AbstractVillager unhappy accessor must resolve");
    assert_eq!(default, 0);
    protocol_int_data(data_id, counter)
}

fn abstract_villager_unhappy_collision_data(counter: i32) -> EntityDataValue {
    abstract_villager_unhappy_data(VANILLA_ENTITY_TYPE_VILLAGER_ID, counter)
}

fn bat_flags_data(flags: i8) -> EntityDataValue {
    let (data_id, default, _resting_mask) =
        bbb_client_world::vanilla_entity_metadata_bat_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_BAT_ID,
        )
        .expect("generated Bat flags accessor must resolve");
    assert_eq!(default, 0);
    protocol_byte_data(data_id, flags)
}

fn vex_flags_data(flags: i8) -> EntityDataValue {
    let (data_id, default, _charging_mask) =
        bbb_client_world::vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VEX_ID,
        )
        .expect("generated Vex flags accessor must resolve");
    assert_eq!(default, 0);
    protocol_byte_data(data_id, flags)
}

/// Deliberately sends the Vex flags field to a non-Vex to cover data-id/type gating.
fn vex_flags_collision_data(flags: i8) -> EntityDataValue {
    vex_flags_data(flags)
}

fn end_crystal_metadata_accessors() -> bbb_client_world::VanillaEndCrystalMetadataAccessors {
    bbb_client_world::vanilla_entity_metadata_end_crystal_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
    )
    .expect("generated EndCrystal metadata accessors must resolve")
}

fn end_crystal_show_bottom_data(show_bottom: bool) -> EntityDataValue {
    let accessors = end_crystal_metadata_accessors();
    assert!(accessors.show_bottom_default);
    protocol_bool_data(accessors.show_bottom_data_id, show_bottom)
}

fn end_crystal_beam_target_data(
    target: Option<bbb_protocol::packets::BlockPos>,
) -> EntityDataValue {
    protocol_optional_block_pos_data(end_crystal_metadata_accessors().beam_target_data_id, target)
}

/// Deliberately sends the EndCrystal beam field to a Bat to cover data-id/type gating.
fn end_crystal_beam_target_collision_data(
    target: Option<bbb_protocol::packets::BlockPos>,
) -> EntityDataValue {
    end_crystal_beam_target_data(target)
}

fn goat_metadata_accessors() -> bbb_client_world::VanillaGoatMetadataAccessors {
    bbb_client_world::vanilla_entity_metadata_goat_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_GOAT_ID,
    )
    .expect("generated Goat metadata accessors must resolve")
}

fn goat_left_horn_data(has_left_horn: bool) -> EntityDataValue {
    let accessors = goat_metadata_accessors();
    assert!(accessors.left_horn_default);
    protocol_bool_data(accessors.left_horn_data_id, has_left_horn)
}

fn goat_right_horn_data(has_right_horn: bool) -> EntityDataValue {
    let accessors = goat_metadata_accessors();
    assert!(accessors.right_horn_default);
    protocol_bool_data(accessors.right_horn_data_id, has_right_horn)
}

fn turtle_metadata_accessors() -> bbb_client_world::VanillaTurtleMetadataAccessors {
    vanilla_entity_metadata_turtle_accessors_for_entity_type(VANILLA_ENTITY_TYPE_TURTLE_ID)
        .expect("generated Turtle metadata accessors must resolve")
}

fn turtle_has_egg_data(has_egg: bool) -> EntityDataValue {
    let accessors = turtle_metadata_accessors();
    assert!(!accessors.has_egg_default);
    protocol_bool_data(accessors.has_egg_data_id, has_egg)
}

/// Deliberately sends the Turtle egg field to a Bat to cover data-id/type gating.
fn turtle_has_egg_collision_data(has_egg: bool) -> EntityDataValue {
    turtle_has_egg_data(has_egg)
}

fn turtle_laying_egg_data(laying_egg: bool) -> EntityDataValue {
    let accessors = turtle_metadata_accessors();
    assert!(!accessors.laying_egg_default);
    protocol_bool_data(accessors.laying_egg_data_id, laying_egg)
}

/// Deliberately sends the Turtle laying-egg field to a Bat to cover data-id/type gating.
fn turtle_laying_egg_collision_data(laying_egg: bool) -> EntityDataValue {
    turtle_laying_egg_data(laying_egg)
}

fn mob_aggressive_data(entity_type_id: i32) -> EntityDataValue {
    let accessors =
        bbb_client_world::vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
            .expect("generated inherited Mob metadata accessors must resolve");
    assert_eq!(accessors.flags_default, 0);
    protocol_byte_data(accessors.flags_data_id, accessors.aggressive_mask)
}

fn mob_left_handed_data(entity_type_id: i32) -> EntityDataValue {
    let accessors =
        bbb_client_world::vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
            .expect("generated inherited Mob metadata accessors must resolve");
    assert_eq!(accessors.flags_default, 0);
    protocol_byte_data(accessors.flags_data_id, accessors.left_handed_mask)
}

fn llama_variant_data(entity_type_id: i32, variant: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_llama_accessors_for_entity_type(entity_type_id)
        .expect("generated Llama metadata accessors must resolve");
    assert_eq!(accessors.variant_default, 0);
    protocol_int_data(accessors.variant_data_id, variant)
}

fn abstract_boat_metadata_accessors() -> bbb_client_world::VanillaAbstractBoatMetadataAccessors {
    bbb_client_world::vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    )
    .expect("generated AbstractBoat metadata accessors must resolve")
}

fn vehicle_metadata_accessors(
    entity_type_id: i32,
) -> bbb_client_world::VanillaVehicleMetadataAccessors {
    bbb_client_world::vanilla_entity_metadata_vehicle_accessors_for_entity_type(entity_type_id)
        .expect("generated VehicleEntity metadata accessors must resolve")
}

fn creeper_metadata_accessors() -> bbb_client_world::VanillaCreeperMetadataAccessors {
    bbb_client_world::vanilla_entity_metadata_creeper_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CREEPER_ID,
    )
    .expect("generated Creeper metadata accessors must resolve")
}

fn shulker_metadata_accessors() -> bbb_client_world::VanillaShulkerMetadataAccessors {
    vanilla_entity_metadata_shulker_accessors_for_entity_type(VANILLA_ENTITY_TYPE_SHULKER_ID)
        .expect("generated Shulker metadata accessors must resolve")
}

fn snow_golem_pumpkin_accessor() -> (u8, i8, i8) {
    bbb_client_world::vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
    )
    .expect("generated SnowGolem pumpkin accessor must resolve")
}

fn sheep_wool_data(color_id: u8, sheared: bool) -> EntityDataValue {
    let (data_id, _, sheared_mask, color_mask) =
        bbb_client_world::vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
        )
        .expect("generated Sheep wool accessor must resolve");
    assert_eq!(color_id & !color_mask, 0, "invalid Sheep wool color id");
    let value = color_id | if sheared { sheared_mask as u8 } else { 0 };
    protocol_byte_data(data_id, value as i8)
}

fn rabbit_type_data(variant_id: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_RABBIT_ID,
        )
        .expect("generated Rabbit variant accessor must resolve");
    protocol_int_data(data_id, variant_id)
}

fn pufferfish_puff_state_data(puff_state: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        )
        .expect("generated Pufferfish puff-state accessor must resolve");
    protocol_int_data(data_id, puff_state)
}

fn slime_size_data(entity_type_id: i32, size: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_slime_size_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated Slime size accessor must resolve");
    protocol_int_data(data_id, size)
}

fn phantom_size_data(size: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_phantom_size_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PHANTOM_ID,
        )
        .expect("generated Phantom size accessor must resolve");
    protocol_int_data(data_id, size)
}

fn salmon_variant_data(variant: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_SALMON_ID,
        )
        .expect("generated Salmon variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn tropical_fish_variant_data(packed: i32) -> EntityDataValue {
    let accessors =
        bbb_client_world::vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        )
        .expect("generated TropicalFish packed-variant accessor must resolve");
    protocol_int_data(accessors.variant_data_id, packed)
}

fn parrot_variant_data(variant: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PARROT_ID,
        )
        .expect("generated Parrot variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn axolotl_variant_data(variant: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        )
        .expect("generated Axolotl variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn camel_dash_data(dashing: bool) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_camel_dash_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
        )
        .expect("generated Camel dash accessor must resolve");
    protocol_bool_data(data_id, dashing)
}

fn camel_last_pose_change_tick_data(entity_type_id: i32, tick: i64) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated Camel last-pose-change accessor must resolve");
    protocol_long_data(data_id, tick)
}

fn polar_bear_standing_data(standing: bool) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        )
        .expect("generated PolarBear standing accessor must resolve");
    protocol_bool_data(data_id, standing)
}

fn mushroom_cow_variant_data(variant: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        )
        .expect("generated MushroomCow variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn bogged_sheared_data(sheared: bool) -> EntityDataValue {
    let (data_id, _) =
        bbb_client_world::vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_BOGGED_ID,
        )
        .expect("generated Bogged sheared accessor must resolve");
    protocol_bool_data(data_id, sheared)
}

fn armor_stand_default_model_pose() -> ArmorStandModelPose {
    let accessors = armor_stand_metadata_accessors();
    ArmorStandModelPose {
        head: accessors.head_pose_default,
        body: accessors.body_pose_default,
        left_arm: accessors.left_arm_pose_default,
        right_arm: accessors.right_arm_pose_default,
        left_leg: accessors.left_leg_pose_default,
        right_leg: accessors.right_leg_pose_default,
    }
}

fn render_candidate(
    entity_id: i32,
    entity_type_id: i32,
    position: [f64; 3],
) -> EntityRenderCandidateState {
    EntityRenderCandidateState {
        entity_id,
        uuid: Uuid::from_u128(entity_id as u128),
        entity_type_id,
        position: EntityVec3 {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        current_position: EntityVec3 {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        bounds: EntityPickBoundsState::from_base_size(1.0, 2.0, 0.0),
        vehicle_id: None,
        leash_holder_id: None,
        fishing_hook_owner_id: None,
        is_spectator: false,
        is_invisible: false,
        is_sleeping: false,
        wears_dragon_head: false,
        has_glowing_flag: false,
        has_always_visible_name: false,
        display_culling: None,
        special_culling: EntityRendererSpecialCullingState::None,
    }
}

fn tamable_animal_flags_data(entity_type_id: i32, tame: bool, sitting: bool) -> EntityDataValue {
    let accessors =
        bbb_client_world::vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
            entity_type_id,
        )
        .expect("generated TamableAnimal metadata accessors must resolve");
    let flags = if tame { accessors.tame_mask } else { 0 }
        | if sitting { accessors.sitting_mask } else { 0 };
    protocol_byte_data(accessors.flags_data_id, flags)
}

fn install_kinetic_runtime_defaults(world: &mut WorldStore, runtime: &NativeItemRuntime) {
    world.set_default_item_component_codec_values(
        runtime.default_component_codec_values_by_protocol_id(),
    );
    world.rebuild_item_component_defaults().unwrap();
}

fn vanilla_kinetic_weapon_fixture(resource_ids: &[&str]) -> serde_json::Value {
    let catalog =
        serde_json::to_value(bbb_client_assets::ItemRegistryCatalog::vanilla_26_1().unwrap())
            .unwrap();
    serde_json::Value::Object(
        resource_ids
            .iter()
            .map(|resource_id| {
                (
                    (*resource_id).to_string(),
                    serde_json::Value::Object([
                        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                    ].into_iter().map(|type_id| {
                        let key = type_id.to_string();
                        (key.clone(), catalog["default_component_codec_values"][*resource_id][&key].clone())
                    }).collect()),
                )
            })
            .collect(),
    )
}

fn protocol_add_entity(id: i32, entity_type_id: i32, position: [f64; 3]) -> AddEntity {
    protocol_add_entity_with_rotation(id, entity_type_id, position, 0.0, 0.0, 0.0)
}

fn protocol_add_entity_with_uuid(
    id: i32,
    entity_type_id: i32,
    uuid: Uuid,
    position: [f64; 3],
) -> AddEntity {
    let mut entity = protocol_add_entity(id, entity_type_id, position);
    entity.uuid = uuid;
    entity
}

/// Stamps the projected `ageInTicks` (= entity `age_ticks` + partial tick) onto every
/// expected instance, so model-selection assertions need not repeat it per instance.
/// Drops the world-projected AABB height that hand-built expected instances leave at zero.
fn without_bounds(instances: &[EntityModelInstance]) -> Vec<EntityModelInstance> {
    let mut instances = instances.to_vec();
    for instance in &mut instances {
        instance.render_state.bounding_box_height = 0.0;
    }
    instances
}

fn aged(mut instances: Vec<EntityModelInstance>, age_in_ticks: f32) -> Vec<EntityModelInstance> {
    for instance in &mut instances {
        instance.render_state.age_in_ticks = age_in_ticks;
    }
    instances
}

fn protocol_add_entity_with_rotation(
    id: i32,
    entity_type_id: i32,
    position: [f64; 3],
    y_rot: f32,
    x_rot: f32,
    y_head_rot: f32,
) -> AddEntity {
    AddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678 + id as u128),
        entity_type_id,
        position: Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        delta_movement: Vec3d::default(),
        x_rot,
        y_rot,
        y_head_rot,
        data: 0,
    }
}

fn skeleton_freeze_conversion_data(entity_type_id: i32, converting: bool) -> EntityDataValue {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated Skeleton freeze-conversion accessor");
    assert!(!default);
    protocol_bool_data(data_id, converting)
}

fn strider_suffocating_data(entity_type_id: i32, suffocating: bool) -> EntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(entity_type_id)
            .expect("generated Strider suffocating accessor");
    assert!(!default);
    protocol_bool_data(data_id, suffocating)
}

fn wither_skull_dangerous_data(dangerous: bool) -> EntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        )
        .expect("generated WitherSkull dangerous accessor");
    assert!(!default);
    protocol_bool_data(data_id, dangerous)
}

fn living_entity_health_data(entity_type_id: i32, health: f32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .expect("generated LivingEntity metadata accessors");
    assert_eq!(accessors.health_default, 1.0);
    protocol_float_data(accessors.health_data_id, health)
}

fn living_entity_flags_data(
    entity_type_id: i32,
    is_using: bool,
    off_hand: bool,
    spin_attack: bool,
) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .expect("generated LivingEntity metadata accessors");
    assert_eq!(accessors.flags_default, 0);
    let mut flags = 0;
    if is_using {
        flags |= accessors.is_using_mask;
    }
    if off_hand {
        flags |= accessors.off_hand_mask;
    }
    if spin_attack {
        flags |= accessors.spin_attack_mask;
    }
    protocol_byte_data(accessors.flags_data_id, flags)
}

fn entity_metadata_accessors() -> bbb_client_world::VanillaEntityMetadataAccessors {
    vanilla_entity_metadata_entity_accessors().expect("generated Entity metadata accessors")
}

fn entity_pose_data(pose: i32) -> EntityDataValue {
    let accessors = entity_metadata_accessors();
    assert_eq!(accessors.pose_default, VANILLA_ENTITY_POSE_STANDING_ID);
    protocol_pose_data(accessors.pose_data_id, pose)
}

fn entity_custom_name_data(value: Option<&str>) -> EntityDataValue {
    protocol_optional_component_data(entity_metadata_accessors().custom_name_data_id, value)
}

fn entity_shared_flags_data(flags: i8) -> EntityDataValue {
    protocol_byte_data(entity_metadata_accessors().shared_flags_data_id, flags)
}

fn entity_on_fire_data() -> EntityDataValue {
    entity_shared_flags_data(entity_metadata_accessors().on_fire_mask)
}

fn entity_invisible_data() -> EntityDataValue {
    entity_shared_flags_data(entity_metadata_accessors().invisible_mask)
}

fn entity_invisible_glowing_data() -> EntityDataValue {
    let accessors = entity_metadata_accessors();
    entity_shared_flags_data(accessors.invisible_mask | accessors.glowing_mask)
}

fn entity_sprinting_data() -> EntityDataValue {
    entity_shared_flags_data(entity_metadata_accessors().sprinting_mask)
}

fn entity_default_shared_flags_data() -> EntityDataValue {
    entity_shared_flags_data(entity_metadata_accessors().shared_flags_default)
}

fn entity_ticks_frozen_data(ticks: i32) -> EntityDataValue {
    let accessors = entity_metadata_accessors();
    assert_eq!(accessors.ticks_frozen_default, 0);
    protocol_int_data(accessors.ticks_frozen_data_id, ticks)
}

fn zombification_immunity_data(entity_type_id: i32, immune: bool) -> EntityDataValue {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated zombification-immunity accessor");
    assert!(!default);
    protocol_bool_data(data_id, immune)
}

fn piglin_baby_data_id() -> u8 {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin baby accessor");
    assert!(!default);
    data_id
}

fn piglin_dancing_data_id() -> u8 {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin dancing accessor");
    assert!(!default);
    data_id
}

fn spellcaster_casting_data_id(entity_type_id: i32) -> u8 {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated SpellcasterIllager casting accessor");
    assert_eq!(default, 0);
    data_id
}

fn creaking_active_data_id() -> u8 {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_creaking_active_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated Creaking active accessor");
    assert!(!default);
    data_id
}

fn ghast_charging_data_id() -> u8 {
    let (data_id, default) =
        bbb_client_world::vanilla_entity_metadata_ghast_charging_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_GHAST_ID,
        )
        .expect("generated Ghast charging accessor");
    assert!(!default);
    data_id
}

fn protocol_bool_data(data_id: u8, value: bool) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    }
}

fn protocol_optional_block_pos_data(
    data_id: u8,
    value: Option<bbb_protocol::packets::BlockPos>,
) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
        value: EntityDataValueKind::OptionalBlockPos(value),
    }
}

fn protocol_chicken_variant_data(id: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    )
    .expect("generated Chicken variant metadata accessors must resolve");
    EntityDataValue {
        data_id: accessors.variant_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_CHICKEN_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::ChickenVariant,
            value: resolved_entity_data_registry_reference(
                EntityDataRegistryHolder::ChickenVariant,
                id,
                ChickenModelVariant::for_vanilla_registry_id(id)
                    .map(ChickenModelVariant::resource_id),
            ),
        },
    }
}

fn protocol_armadillo_state_data(id: i32) -> EntityDataValue {
    // Vanilla `EntityDataSerializers.ARMADILLO_STATE` is serializer id 36.
    let (data_id, _) = vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMADILLO_ID,
    )
    .expect("generated Armadillo state accessor must resolve");
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ARMADILLO_STATE_ID,
        value: EntityDataValueKind::EnumId {
            serializer: EntityDataEnumSerializer::ArmadilloState,
            id,
        },
    }
}

fn protocol_zombie_baby_data(entity_type_id: i32, baby: bool) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_zombie_accessors_for_entity_type(entity_type_id)
        .expect("generated Zombie metadata accessors");
    protocol_bool_data(accessors.baby_data_id, baby)
}

fn protocol_zombie_drowned_conversion_data(
    entity_type_id: i32,
    converting: bool,
) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_zombie_accessors_for_entity_type(entity_type_id)
        .expect("generated Zombie metadata accessors");
    protocol_bool_data(accessors.drowned_conversion_data_id, converting)
}

fn protocol_zombie_villager_converting_data(converting: bool) -> EntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        )
        .expect("generated ZombieVillager converting accessor");
    assert!(!default);
    protocol_bool_data(data_id, converting)
}

fn protocol_abstract_arrow_in_ground_data(entity_type_id: i32, in_ground: bool) -> EntityDataValue {
    let accessors =
        vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
            .expect("generated AbstractArrow metadata accessors");
    protocol_bool_data(accessors.in_ground_data_id, in_ground)
}

fn protocol_arrow_effect_color_data(color: i32) -> EntityDataValue {
    let accessors =
        vanilla_entity_metadata_arrow_accessors_for_entity_type(VANILLA_ENTITY_TYPE_ARROW_ID)
            .expect("generated Arrow metadata accessors");
    protocol_int_data(accessors.effect_color_data_id, color)
}

fn protocol_thrown_trident_foil_data(foil: bool) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .expect("generated ThrownTrident metadata accessors");
    protocol_bool_data(accessors.foil_data_id, foil)
}

fn protocol_copper_golem_weathering_data(id: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    )
    .expect("generated CopperGolem metadata accessors");
    // Vanilla `EntityDataSerializers.WEATHERING_COPPER_STATE` is serializer id 38.
    EntityDataValue {
        data_id: accessors.weather_state_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_WEATHERING_COPPER_STATE_ID,
        value: EntityDataValueKind::EnumId {
            serializer: EntityDataEnumSerializer::WeatheringCopperState,
            id,
        },
    }
}

fn protocol_copper_golem_state_data(id: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_copper_golem_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    )
    .expect("generated CopperGolem metadata accessors");
    // Vanilla `EntityDataSerializers.COPPER_GOLEM_STATE` is serializer id 37.
    EntityDataValue {
        data_id: accessors.state_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COPPER_GOLEM_STATE_ID,
        value: EntityDataValueKind::EnumId {
            serializer: EntityDataEnumSerializer::CopperGolemState,
            id,
        },
    }
}

fn protocol_villager_data(
    entity_type_id: i32,
    villager_type: i32,
    profession: i32,
    level: i32,
) -> EntityDataValue {
    let data_id = vanilla_entity_metadata_villager_data_accessor_for_entity_type(entity_type_id)
        .expect("generated villager-data accessor must resolve");
    protocol_villager_data_at(data_id, villager_type, profession, level)
}

fn protocol_villager_data_at_unhappy_counter(
    villager_type: i32,
    profession: i32,
    level: i32,
) -> EntityDataValue {
    let accessor = bbb_client_world::vanilla_entity_metadata_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        "DATA_UNHAPPY_COUNTER",
    )
    .expect("generated AbstractVillager unhappy-counter accessor must resolve");
    assert_eq!(accessor.serializer_name, "INT");
    protocol_villager_data_at(accessor.data_id, villager_type, profession, level)
}

fn protocol_villager_data_at(
    data_id: u8,
    villager_type: i32,
    profession: i32,
    level: i32,
) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_VILLAGER_DATA_ID,
        value: EntityDataValueKind::VillagerData {
            villager_type: resolved_entity_data_registry_reference_for_registry(
                "minecraft:villager_type",
                villager_type,
                VillagerModelType::for_vanilla_registry_id(villager_type)
                    .map(VillagerModelType::resource_id),
            ),
            profession: resolved_entity_data_registry_reference_for_registry(
                "minecraft:villager_profession",
                profession,
                VillagerModelProfession::for_vanilla_registry_id(profession)
                    .map(VillagerModelProfession::resource_id),
            ),
            level,
        },
    }
}

fn protocol_cow_variant_data(id: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_COW_ID,
    )
    .expect("generated Cow variant metadata accessors must resolve");
    EntityDataValue {
        data_id: accessors.variant_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_COW_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::CowVariant,
            value: resolved_entity_data_registry_reference(
                EntityDataRegistryHolder::CowVariant,
                id,
                CowModelVariant::for_vanilla_registry_id(id).map(CowModelVariant::resource_id),
            ),
        },
    }
}

fn protocol_pig_variant_data(id: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_farm_animal_variant_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_PIG_ID,
    )
    .expect("generated Pig variant metadata accessors must resolve");
    EntityDataValue {
        data_id: accessors.variant_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PIG_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::PigVariant,
            value: resolved_entity_data_registry_reference(
                EntityDataRegistryHolder::PigVariant,
                id,
                PigModelVariant::for_vanilla_registry_id(id).map(PigModelVariant::resource_id),
            ),
        },
    }
}

fn wolf_metadata_accessors() -> bbb_client_world::VanillaWolfMetadataAccessors {
    vanilla_entity_metadata_wolf_accessors_for_entity_type(VANILLA_ENTITY_TYPE_WOLF_ID)
        .expect("generated Wolf metadata accessors must resolve")
}

fn cat_metadata_accessors() -> bbb_client_world::VanillaCatMetadataAccessors {
    vanilla_entity_metadata_cat_accessors_for_entity_type(VANILLA_ENTITY_TYPE_CAT_ID)
        .expect("generated Cat metadata accessors must resolve")
}

fn protocol_zombie_nautilus_variant_data(id: i32) -> EntityDataValue {
    // Vanilla `EntityDataSerializers.ZOMBIE_NAUTILUS_VARIANT` is serializer id 32.
    let (data_id, _) = vanilla_entity_metadata_zombie_nautilus_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    )
    .expect("generated ZombieNautilus variant metadata accessor must resolve");
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ZOMBIE_NAUTILUS_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::ZombieNautilusVariant,
            value: resolved_entity_data_registry_reference(
                EntityDataRegistryHolder::ZombieNautilusVariant,
                id,
                ZombieNautilusModelVariant::for_vanilla_registry_id(id)
                    .map(ZombieNautilusModelVariant::resource_id),
            ),
        },
    }
}

fn protocol_frog_variant_data(id: i32) -> EntityDataValue {
    // Vanilla `EntityDataSerializers.FROG_VARIANT` is serializer id 27.
    let (data_id, _) =
        vanilla_entity_metadata_frog_variant_accessor_for_entity_type(VANILLA_ENTITY_TYPE_FROG_ID)
            .expect("generated Frog variant metadata accessor must resolve");
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FROG_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::FrogVariant,
            value: resolved_entity_data_registry_reference(
                EntityDataRegistryHolder::FrogVariant,
                id,
                FrogModelVariant::for_vanilla_registry_id(id).map(FrogModelVariant::resource_id),
            ),
        },
    }
}

fn protocol_cat_variant_data(id: i32) -> EntityDataValue {
    // Vanilla `EntityDataSerializers.CAT_VARIANT` is serializer id 21.
    let accessors = vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CAT_ID,
    )
    .expect("generated Cat variant metadata accessors must resolve");
    EntityDataValue {
        data_id: accessors.variant_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_CAT_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::CatVariant,
            value: resolved_entity_data_registry_reference(
                EntityDataRegistryHolder::CatVariant,
                id,
                CatModelVariant::for_vanilla_registry_id(id).map(CatModelVariant::resource_id),
            ),
        },
    }
}

fn protocol_wolf_variant_data(id: i32) -> EntityDataValue {
    // Vanilla `EntityDataSerializers.WOLF_VARIANT` is serializer id 25.
    let accessors = vanilla_entity_metadata_companion_variant_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_WOLF_ID,
    )
    .expect("generated Wolf variant metadata accessors must resolve");
    EntityDataValue {
        data_id: accessors.variant_data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_WOLF_VARIANT_ID,
        value: EntityDataValueKind::RegistryId {
            serializer: EntityDataRegistryHolder::WolfVariant,
            value: resolved_entity_data_registry_reference(
                EntityDataRegistryHolder::WolfVariant,
                id,
                WolfModelVariant::for_vanilla_registry_id(id).map(WolfModelVariant::resource_id),
            ),
        },
    }
}

fn resolved_entity_data_registry_reference(
    serializer: EntityDataRegistryHolder,
    holder_id: i32,
    resource_id: Option<&str>,
) -> EntityDataRegistryReference {
    resolved_entity_data_registry_reference_for_registry(
        serializer.registry(),
        holder_id,
        resource_id,
    )
}

fn resolved_entity_data_registry_reference_for_registry(
    registry: &str,
    holder_id: i32,
    resource_id: Option<&str>,
) -> EntityDataRegistryReference {
    let mut value = EntityDataRegistryReference::unresolved(holder_id);
    if let Some(resource_id) = resource_id {
        value.bind(EntityDataRegistryIdentity::new(
            registry,
            resource_id,
            EntityDataRegistrySource::VanillaFallback,
            EntityDataRegistryRevision::default(),
        ));
    }
    value
}

fn bind_protocol_registry_data_to_resource(
    mut data: EntityDataValue,
    resource_id: &str,
) -> EntityDataValue {
    let EntityDataValueKind::RegistryId { serializer, value } = &mut data.value else {
        panic!("test helper requires registry-holder metadata");
    };
    value.bind(EntityDataRegistryIdentity::new(
        serializer.registry(),
        resource_id,
        EntityDataRegistrySource::Server,
        EntityDataRegistryRevision::default(),
    ));
    data
}

fn protocol_int_data(data_id: u8, value: i32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(value),
    }
}

fn protocol_optional_unsigned_int_data(data_id: u8, value: Option<i32>) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
        value: EntityDataValueKind::OptionalUnsignedInt(value),
    }
}

fn protocol_long_data(data_id: u8, value: i64) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
        value: EntityDataValueKind::Long(value),
    }
}

fn protocol_pose_data(data_id: u8, pose: i32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
        value: EntityDataValueKind::Pose(pose),
    }
}

fn protocol_float_data(data_id: u8, value: f32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
        value: EntityDataValueKind::Float(value),
    }
}

fn protocol_optional_component_data(data_id: u8, value: Option<&str>) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
        value: EntityDataValueKind::OptionalComponent(
            value.map(bbb_protocol::RichTextComponent::plain),
        ),
    }
}

fn protocol_byte_data(data_id: u8, value: i8) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(value),
    }
}

fn protocol_direction_data(data_id: u8, value: i32) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
        value: EntityDataValueKind::Direction(value),
    }
}

fn protocol_rotations_data(data_id: u8, value: [f32; 3]) -> EntityDataValue {
    EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
        value: EntityDataValueKind::Rotations {
            x: value[0],
            y: value[1],
            z: value[2],
        },
    }
}

fn protocol_play_login(player_id: i32) -> PlayLogin {
    PlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: false,
    }
}

fn assert_selection_box_close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "expected {expected}, got {actual}"
        );
    }
}

fn fishing_hook_render_source() -> FishingHookRenderSourceState {
    FishingHookRenderSourceState {
        hook_position: EntityVec3 {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        },
        owner_id: 4,
        owner_eye_position: EntityVec3 {
            x: 12.0,
            y: 22.0,
            z: 34.0,
        },
        owner_body_y_rot: 0.0,
        owner_scale: 1.0,
        owner_is_crouching: false,
        holding_arm_left: false,
        owner_attack_anim: 0.0,
    }
}

fn install_equipment_defaults(
    world: &mut WorldStore,
    generation: u64,
    equipment: &[(i32, &str, &str)],
) {
    let mut names = Vec::new();
    let codecs = equipment
        .iter()
        .map(|&(item_id, slot, asset)| {
            if !names.iter().any(|name| name == asset) {
                names.push(asset.to_string());
            }
            (
                item_id,
                std::collections::BTreeMap::from([(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    serde_json::json!({"slot": slot, "asset_id": asset}),
                )]),
            )
        })
        .collect();
    world
        .replace_item_component_defaults(codecs, generation, names)
        .unwrap();
    world.rebuild_item_component_defaults().unwrap();
}

mod combat_and_living_states;
mod culling_and_outlines;
mod mob_animations_and_poses;
mod model_kinds_aquatic_and_passive;
mod model_kinds_wolves_equines_and_hooks;
mod variants_saddles_and_players;

use model_kinds_wolves_equines_and_hooks::VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS;

#[test]
fn modded_entity_resolves_data_driven_rig_and_enraged_slot_from_content_registry() {
    use bbb_protocol::content_registry::{
        ContentRegistry, ContentRegistryBuilder, ModEntityRegistration, ModSynchedFieldSpec,
        BBB_CONTENT_REGISTRY_CHANNEL,
    };
    let mut builder = ContentRegistryBuilder::new();
    builder.register_entity(ModEntityRegistration {
        resource_id: "bbb_demo:inferno_warden".to_string(),
        synched_fields: vec![ModSynchedFieldSpec {
            data_id: 16,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        }],
        rig_id: Some("bbb_demo:inferno_warden".to_string()),
        dynamic_light_rgb: Some([1.0, 0.42, 0.08]),
        ..Default::default()
    });
    let registry = builder.freeze().expect("freeze demo content registry");
    let raw_payload = registry.encode_wire();

    let mut world = WorldStore::new();
    world.apply_custom_payload(bbb_protocol::packets::CustomPayload {
        id: BBB_CONTENT_REGISTRY_CHANNEL.to_string(),
        payload: bbb_protocol::packets::CustomPayloadBody::Unknown { raw_payload },
    });
    let mod_type_id = ContentRegistry::entity_base_id();
    assert_eq!(
        world
            .content_registry()
            .entity_by_id(mod_type_id)
            .map(|entry| entry.resource_id.as_str()),
        Some("bbb_demo:inferno_warden")
    );

    world.apply_add_entity(protocol_add_entity(157, mod_type_id, [4.0, 64.0, 4.0]));
    world.apply_set_entity_data(SetEntityData {
        id: 157,
        values: vec![protocol_byte_data(16, 1)],
    });

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let boss = instances
        .iter()
        .find(|instance| instance.entity_id == 157)
        .expect("modded boss render instance");
    let expected_rig_id = bbb_client_assets::EntityRigCatalog::builtin_default()
        .rig_index("bbb_demo:inferno_warden")
        .expect("builtin rig");
    assert_eq!(
        boss.kind,
        EntityModelKind::DataDriven {
            rig_id: expected_rig_id,
            anim_state: 1,
        }
    );

    let lights = crate::dynamic_lights::extract_dynamic_point_lights(
        bbb_client_render::DynamicLightsMode::Fancy,
        &world,
        None,
        &instances,
        None,
    );
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].color, [1.0, 0.42, 0.08]);
    assert_eq!(lights[0].intensity, 1.0);
}
