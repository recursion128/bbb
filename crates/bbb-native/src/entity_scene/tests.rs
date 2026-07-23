use super::*;
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
use bbb_world::{
    vanilla_entity_metadata_enderman_render_accessors_for_entity_type,
    vanilla_entity_metadata_happy_ghast_accessors_for_entity_type,
    ArmorMaterialKind as WorldArmorMaterialKind, ChunkColumn, ChunkPos, ChunkState,
    EntityPickBoundsState, EntityVec3, HappyGhastHarnessColor as WorldHappyGhastHarnessColor,
    ItemEquipmentSlot, LightData, LlamaBodyDecorColor as WorldLlamaBodyDecorColor,
    RegistryPacketEntry, VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
    VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
};
use uuid::Uuid;

fn insert_loaded_entity_chunk(world: &mut WorldStore, pos: ChunkPos) {
    world.insert_decoded_chunk(ChunkColumn {
        pos,
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: Vec::new(),
        light: LightData::default(),
    });
}

fn bee_metadata_accessors() -> bbb_world::VanillaBeeMetadataAccessors {
    bbb_world::vanilla_entity_metadata_bee_accessors_for_entity_type(VANILLA_ENTITY_TYPE_BEE_ID)
        .expect("generated Bee metadata accessors")
}

fn panda_metadata_accessors() -> bbb_world::VanillaPandaMetadataAccessors {
    vanilla_entity_metadata_panda_accessors_for_entity_type(VANILLA_ENTITY_TYPE_PANDA_ID)
        .expect("generated Panda metadata accessors must resolve")
}

fn armor_stand_metadata_accessors() -> bbb_world::VanillaArmorStandMetadataAccessors {
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
        bbb_world::vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
            .expect("generated nearest baby accessor must resolve");
    assert!(!default);
    protocol_bool_data(data_id, is_baby)
}

fn abstract_villager_unhappy_data(entity_type_id: i32, counter: i32) -> EntityDataValue {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
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
        bbb_world::vanilla_entity_metadata_bat_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_BAT_ID,
        )
        .expect("generated Bat flags accessor must resolve");
    assert_eq!(default, 0);
    protocol_byte_data(data_id, flags)
}

fn vex_flags_data(flags: i8) -> EntityDataValue {
    let (data_id, default, _charging_mask) =
        bbb_world::vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
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

fn end_crystal_metadata_accessors() -> bbb_world::VanillaEndCrystalMetadataAccessors {
    bbb_world::vanilla_entity_metadata_end_crystal_accessors_for_entity_type(
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

fn goat_metadata_accessors() -> bbb_world::VanillaGoatMetadataAccessors {
    bbb_world::vanilla_entity_metadata_goat_accessors_for_entity_type(VANILLA_ENTITY_TYPE_GOAT_ID)
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

fn turtle_metadata_accessors() -> bbb_world::VanillaTurtleMetadataAccessors {
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
        bbb_world::vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
            .expect("generated inherited Mob metadata accessors must resolve");
    assert_eq!(accessors.flags_default, 0);
    protocol_byte_data(accessors.flags_data_id, accessors.aggressive_mask)
}

fn llama_variant_data(entity_type_id: i32, variant: i32) -> EntityDataValue {
    let accessors = vanilla_entity_metadata_llama_accessors_for_entity_type(entity_type_id)
        .expect("generated Llama metadata accessors must resolve");
    assert_eq!(accessors.variant_default, 0);
    protocol_int_data(accessors.variant_data_id, variant)
}

fn abstract_boat_metadata_accessors() -> bbb_world::VanillaAbstractBoatMetadataAccessors {
    bbb_world::vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    )
    .expect("generated AbstractBoat metadata accessors must resolve")
}

fn vehicle_metadata_accessors(entity_type_id: i32) -> bbb_world::VanillaVehicleMetadataAccessors {
    bbb_world::vanilla_entity_metadata_vehicle_accessors_for_entity_type(entity_type_id)
        .expect("generated VehicleEntity metadata accessors must resolve")
}

fn creeper_metadata_accessors() -> bbb_world::VanillaCreeperMetadataAccessors {
    bbb_world::vanilla_entity_metadata_creeper_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_CREEPER_ID,
    )
    .expect("generated Creeper metadata accessors must resolve")
}

fn shulker_metadata_accessors() -> bbb_world::VanillaShulkerMetadataAccessors {
    vanilla_entity_metadata_shulker_accessors_for_entity_type(VANILLA_ENTITY_TYPE_SHULKER_ID)
        .expect("generated Shulker metadata accessors must resolve")
}

fn snow_golem_pumpkin_accessor() -> (u8, i8, i8) {
    bbb_world::vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
    )
    .expect("generated SnowGolem pumpkin accessor must resolve")
}

fn sheep_wool_data(color_id: u8, sheared: bool) -> EntityDataValue {
    let (data_id, _, sheared_mask, color_mask) =
        bbb_world::vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
        )
        .expect("generated Sheep wool accessor must resolve");
    assert_eq!(color_id & !color_mask, 0, "invalid Sheep wool color id");
    let value = color_id | if sheared { sheared_mask as u8 } else { 0 };
    protocol_byte_data(data_id, value as i8)
}

fn rabbit_type_data(variant_id: i32) -> EntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_rabbit_type_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_RABBIT_ID,
    )
    .expect("generated Rabbit variant accessor must resolve");
    protocol_int_data(data_id, variant_id)
}

fn pufferfish_puff_state_data(puff_state: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_world::vanilla_entity_metadata_pufferfish_puff_state_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        )
        .expect("generated Pufferfish puff-state accessor must resolve");
    protocol_int_data(data_id, puff_state)
}

fn slime_size_data(entity_type_id: i32, size: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_world::vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)
            .expect("generated Slime size accessor must resolve");
    protocol_int_data(data_id, size)
}

fn phantom_size_data(size: i32) -> EntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_phantom_size_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PHANTOM_ID,
    )
    .expect("generated Phantom size accessor must resolve");
    protocol_int_data(data_id, size)
}

fn salmon_variant_data(variant: i32) -> EntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_salmon_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SALMON_ID,
    )
    .expect("generated Salmon variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn tropical_fish_variant_data(packed: i32) -> EntityDataValue {
    let accessors =
        bbb_world::vanilla_entity_metadata_tropical_fish_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        )
        .expect("generated TropicalFish packed-variant accessor must resolve");
    protocol_int_data(accessors.variant_data_id, packed)
}

fn parrot_variant_data(variant: i32) -> EntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_parrot_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PARROT_ID,
    )
    .expect("generated Parrot variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn axolotl_variant_data(variant: i32) -> EntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_axolotl_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    )
    .expect("generated Axolotl variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn camel_dash_data(dashing: bool) -> EntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_camel_dash_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    )
    .expect("generated Camel dash accessor must resolve");
    protocol_bool_data(data_id, dashing)
}

fn camel_last_pose_change_tick_data(entity_type_id: i32, tick: i64) -> EntityDataValue {
    let (data_id, _) =
        bbb_world::vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated Camel last-pose-change accessor must resolve");
    protocol_long_data(data_id, tick)
}

fn polar_bear_standing_data(standing: bool) -> EntityDataValue {
    let (data_id, _) =
        bbb_world::vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        )
        .expect("generated PolarBear standing accessor must resolve");
    protocol_bool_data(data_id, standing)
}

fn mushroom_cow_variant_data(variant: i32) -> EntityDataValue {
    let (data_id, _) =
        bbb_world::vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        )
        .expect("generated MushroomCow variant accessor must resolve");
    protocol_int_data(data_id, variant)
}

fn bogged_sheared_data(sheared: bool) -> EntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_BOGGED_ID,
    )
    .expect("generated Bogged sheared accessor must resolve");
    protocol_bool_data(data_id, sheared)
}

fn allay_dancing_data(dancing: bool) -> EntityDataValue {
    let (data_id, _) = bbb_world::vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ALLAY_ID,
    )
    .expect("generated Allay dancing accessor must resolve");
    protocol_bool_data(data_id, dancing)
}

fn axolotl_playing_dead_data(playing_dead: bool) -> EntityDataValue {
    let (data_id, _) =
        bbb_world::vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        )
        .expect("generated Axolotl playing-dead accessor must resolve");
    protocol_bool_data(data_id, playing_dead)
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
        display_culling: None,
    }
}

#[test]
fn visibility_first_culling_applies_distance_frustum_and_leash_union() {
    let camera = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let frustum = CameraFrustum::from_pose(camera, 16.0 / 9.0, 70.0);
    let front = render_candidate(1, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [0.0, 0.0, 10.0]);
    let behind = render_candidate(2, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [0.0, 0.0, -10.0]);
    assert!(entity_renderer_should_render(
        front,
        &[front],
        &HashMap::from([(front.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));
    assert!(!entity_renderer_should_render(
        behind,
        &[behind],
        &HashMap::from([(behind.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    let holder = render_candidate(3, VANILLA_ENTITY_TYPE_COW_ID, [0.0, 0.0, 10.0]);
    let mut leashed = render_candidate(4, VANILLA_ENTITY_TYPE_COW_ID, [30.0, 0.0, 10.0]);
    leashed.leash_holder_id = Some(holder.entity_id);
    let candidates = [leashed, holder];
    assert!(entity_renderer_should_render(
        leashed,
        &candidates,
        &HashMap::from([(leashed.entity_id, 0), (holder.entity_id, 1)]),
        camera,
        frustum,
        12,
    ));

    let local_player = render_candidate(5, VANILLA_ENTITY_TYPE_PLAYER_ID, [0.0, 0.0, 0.0]);
    let mut first_vehicle = render_candidate(6, VANILLA_ENTITY_TYPE_COW_ID, [0.0, 0.0, -10.0]);
    let root_vehicle = render_candidate(7, VANILLA_ENTITY_TYPE_COW_ID, [0.0, 0.0, -10.0]);
    let mut riding_player = local_player;
    riding_player.vehicle_id = Some(first_vehicle.entity_id);
    first_vehicle.vehicle_id = Some(root_vehicle.entity_id);
    let candidates = [riding_player, first_vehicle, root_vehicle];
    let index = HashMap::from([
        (riding_player.entity_id, 0),
        (first_vehicle.entity_id, 1),
        (root_vehicle.entity_id, 2),
    ]);
    assert!(candidate_has_local_player_indirect_passenger(
        root_vehicle.entity_id,
        Some(riding_player.entity_id),
        &candidates,
        &index,
    ));
    assert!(!candidate_has_local_player_indirect_passenger(
        99,
        Some(riding_player.entity_id),
        &candidates,
        &index,
    ));

    let mut fishing_hook =
        render_candidate(8, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, [0.0, 0.0, 10.0]);
    assert!(!entity_renderer_should_render(
        fishing_hook,
        &[fishing_hook],
        &HashMap::from([(fishing_hook.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));
    fishing_hook.fishing_hook_owner_id = Some(local_player.entity_id);
    let candidates = [fishing_hook, local_player];
    assert!(entity_renderer_should_render(
        fishing_hook,
        &candidates,
        &HashMap::from([(fishing_hook.entity_id, 0), (local_player.entity_id, 1)]),
        camera,
        frustum,
        12,
    ));
    fishing_hook.position.z = -10.0;
    let candidates = [fishing_hook, local_player];
    assert!(!entity_renderer_should_render(
        fishing_hook,
        &candidates,
        &HashMap::from([(fishing_hook.entity_id, 0), (local_player.entity_id, 1)]),
        camera,
        frustum,
        12,
    ));
}

#[test]
fn visibility_first_culling_keeps_catalog_exceptions_and_custom_bounds() {
    let camera = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let frustum = CameraFrustum::from_pose(camera, 16.0 / 9.0, 70.0);
    let lightning = render_candidate(1, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, [0.0, 0.0, -10.0]);
    assert!(entity_renderer_should_render(
        lightning,
        &[lightning],
        &HashMap::from([(lightning.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    let sniffer = render_candidate(2, VANILLA_ENTITY_TYPE_SNIFFER_ID, [0.0, 4.0, 10.0]);
    let (sniffer_min, sniffer_max) = entity_renderer_culling_bounds(sniffer, true);
    for (actual, expected) in sniffer_min.into_iter().zip([-1.6, 2.9, 8.4]) {
        assert!((actual - expected).abs() < 1.0e-5);
    }
    for (actual, expected) in sniffer_max.into_iter().zip([1.6, 7.1, 11.6]) {
        assert!((actual - expected).abs() < 1.0e-5);
    }

    let mut zombie = render_candidate(3, VANILLA_ENTITY_TYPE_ZOMBIE_ID, [0.0, 4.0, 10.0]);
    zombie.wears_dragon_head = true;
    let (dragon_head_min, dragon_head_max) = entity_renderer_culling_bounds(zombie, true);
    assert_eq!(dragon_head_min, [-1.5, 3.0, 8.5]);
    assert_eq!(dragon_head_max, [1.5, 7.0, 11.5]);
}

#[test]
fn display_visibility_uses_view_range_and_metadata_owned_culling_box() {
    let camera = CameraPose {
        position: [0.0; 3],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: 0.0,
    };
    let frustum = CameraFrustum::from_pose(camera, 16.0 / 9.0, 70.0);
    let mut display = render_candidate(10, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID, [0.0, 0.0, -10.0]);
    display.display_culling = Some(DisplayEntityCullingState {
        view_range: 1.0,
        width: 0.0,
        height: 0.0,
    });
    assert!(entity_renderer_should_render(
        display,
        &[display],
        &HashMap::from([(display.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    display.display_culling = Some(DisplayEntityCullingState {
        view_range: 1.0,
        width: 4.0,
        height: 6.0,
    });
    assert!(!entity_renderer_should_render(
        display,
        &[display],
        &HashMap::from([(display.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    display.position = EntityVec3 {
        x: 0.0,
        y: 0.0,
        z: 100.0,
    };
    display.display_culling = Some(DisplayEntityCullingState {
        view_range: 1.0,
        width: 0.0,
        height: 0.0,
    });
    assert!(!entity_renderer_should_render(
        display,
        &[display],
        &HashMap::from([(display.entity_id, 0)]),
        camera,
        frustum,
        12,
    ));

    display.position = EntityVec3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };
    display.display_culling = Some(DisplayEntityCullingState {
        view_range: 2.0,
        width: 4.0,
        height: 6.0,
    });
    assert_eq!(
        entity_renderer_culling_bounds(display, true),
        ([-1.5, 1.5, 0.5], [3.5, 8.5, 5.5])
    );
}

fn tamable_animal_flags_data(entity_type_id: i32, tame: bool, sitting: bool) -> EntityDataValue {
    let accessors =
        bbb_world::vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)
            .expect("generated TamableAnimal metadata accessors must resolve");
    let flags = if tame { accessors.tame_mask } else { 0 }
        | if sitting { accessors.sitting_mask } else { 0 };
    protocol_byte_data(accessors.flags_data_id, flags)
}

#[test]
fn entity_hand_projection_does_not_restore_component_id_aliases() {
    let source = include_str!("hands.rs");
    for old_alias in [
        ["DATA_COMPONENT_CONSUMABLE_", "TYPE_ID"].concat(),
        ["DATA_COMPONENT_SWING_ANIMATION_", "TYPE_ID"].concat(),
        ["DATA_COMPONENT_BLOCKS_ATTACKS_", "TYPE_ID"].concat(),
        ["DATA_COMPONENT_TOOL_", "TYPE_ID"].concat(),
    ] {
        assert!(
            !source.contains(&format!("const {old_alias}")),
            "restored `{old_alias}` alias"
        );
    }
}

fn vanilla_kinetic_weapon_fixture(resource_ids: &[&str]) -> serde_json::Value {
    let catalog =
        serde_json::to_value(bbb_pack::ItemRegistryCatalog::vanilla_26_1().unwrap()).unwrap();
    serde_json::Value::Object(
        resource_ids
            .iter()
            .map(|resource_id| {
                (
                    (*resource_id).to_string(),
                    catalog["default_kinetic_weapons"][*resource_id].clone(),
                )
            })
            .collect(),
    )
}

#[test]
fn squid_scene_membership_uses_the_world_generated_catalog() {
    assert!(is_squid_entity_type(VANILLA_ENTITY_TYPE_SQUID_ID));
    assert!(is_squid_entity_type(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID));
    assert!(!is_squid_entity_type(VANILLA_ENTITY_TYPE_COD_ID));

    let source = include_str!("instance.rs");
    let body = source
        .split("fn is_squid_entity_type")
        .nth(1)
        .and_then(|source| source.split_once("\n}"))
        .map(|(body, _)| body)
        .expect("squid entity type query source");
    assert!(!body.contains("matches!"));
}

#[test]
fn entity_scene_outline_is_none_without_visible_entity_targets() {
    assert_eq!(
        entity_scene_outline_from_world_at_partial_tick(&WorldStore::new(), 1.0),
        None
    );
}

#[test]
fn entity_scene_outline_projects_missing_server_entity_labels_when_debug_flag_is_enabled() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 3.0],
    ));

    let default_outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected default client-side hitbox outline");
    assert!(default_outline.text_labels.is_empty());

    let outline =
        entity_scene_outline_from_world_at_partial_tick_with_server_details(&world, 1.0, true)
            .expect("expected local-server debug entity outline");

    assert_eq!(outline.text_labels.len(), 1);
    assert_eq!(
        outline.text_labels[0].text,
        ENTITY_MISSING_SERVER_LABEL_TEXT
    );
    assert_eq!(
        outline.text_labels[0].color,
        ENTITY_MISSING_SERVER_LABEL_COLOR
    );
    assert!(outline.text_labels[0].centered);
    assert_selection_box_close(outline.text_labels[0].position, [0.0, 3.2, 3.0]);
}

#[test]
fn entity_scene_outline_projects_pick_bounds_for_all_visible_targets() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        11,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [2.0, 1.0, 3.0],
    ));

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.5)
        .expect("expected entity scene outline");

    assert!(outline.boxes.is_empty());
    assert_eq!(outline.colored_boxes.len(), 2);
    assert_eq!(outline.lines.len(), 2);
    assert_eq!(outline.points.len(), 2);
    assert_selection_box_close(outline.colored_boxes[0].min, [-0.49, 1.0, 2.51]);
    assert_selection_box_close(outline.colored_boxes[0].max, [0.49, 1.7, 3.49]);
    assert_eq!(outline.colored_boxes[0].color, ENTITY_HITBOX_COLOR);
    assert_selection_box_close(outline.colored_boxes[1].min, [1.51, 1.0, 2.51]);
    assert_selection_box_close(outline.colored_boxes[1].max, [2.49, 1.7, 3.49]);
    assert_eq!(outline.colored_boxes[1].color, ENTITY_HITBOX_COLOR);
    assert_eq!(outline.lines[0].color, ENTITY_VIEW_VECTOR_COLOR);
    assert!((outline.lines[0].to[2] - outline.lines[0].from[2] - 2.0).abs() < 1.0e-5);
    assert_selection_box_close(outline.points[0].position, [0.0, 1.0, 3.0]);
    assert_eq!(outline.points[0].color, ENTITY_HITBOX_COLOR);
    assert_eq!(outline.points[0].size, ENTITY_POSITION_POINT_SIZE);
    assert_selection_box_close(outline.points[1].position, [2.0, 1.0, 3.0]);
}

#[test]
fn entity_scene_outline_projects_passenger_vehicle_slab() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        90,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        91,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 90,
        passenger_ids: vec![91],
    }));

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected passenger vehicle debug outline");

    assert_eq!(
        outline.colored_boxes[2].color,
        ENTITY_PASSENGER_VEHICLE_COLOR
    );
    assert_selection_box_close(outline.colored_boxes[2].min, [0.8, 64.1875, -2.2]);
    assert_selection_box_close(outline.colored_boxes[2].max, [1.2, 64.25, -1.8]);
}

#[test]
fn entity_scene_outline_uses_bounds_without_pick_radius_inflation() {
    let outline_box = entity_pick_target_box(EntityPickTargetState {
        entity_id: 7,
        position: EntityVec3 {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        },
        bounds: EntityPickBoundsState::from_centered_size(2.0, 4.0, 6.0, 1.5),
    });

    assert_selection_box_close(outline_box.min, [9.0, 18.0, 27.0]);
    assert_selection_box_close(outline_box.max, [11.0, 22.0, 33.0]);
}

#[test]
fn entity_scene_outline_filters_local_player_and_camera_entity() {
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(10));
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [0.0, 1.0, 3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        11,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [2.0, 1.0, 3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [4.0, 1.0, 3.0],
    ));
    assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected non-camera entity scene outline");

    assert_eq!(outline.colored_boxes.len(), 1);
    assert_eq!(outline.lines.len(), 1);
    assert_eq!(outline.points.len(), 1);
    assert_selection_box_close(outline.colored_boxes[0].min, [3.51, 1.0, 2.51]);
    assert_selection_box_close(outline.colored_boxes[0].max, [4.49, 1.7, 3.49]);
    assert_eq!(outline.colored_boxes[0].color, ENTITY_HITBOX_COLOR);
    assert_selection_box_close(outline.points[0].position, [4.0, 1.0, 3.0]);
}

#[test]
fn entity_scene_outline_projects_living_eye_height_box_and_view_vector() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_rotation(
        20,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        [0.0, 1.0, 3.0],
        90.0,
        -30.0,
        90.0,
    ));
    let eye_height = world.probe_entity_camera_pose(20).unwrap().eye_height;

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected living entity debug outline");

    assert_eq!(outline.colored_boxes.len(), 2);
    assert_eq!(outline.lines.len(), 1);
    assert_eq!(outline.points.len(), 1);
    assert_eq!(outline.colored_boxes[0].color, ENTITY_HITBOX_COLOR);
    assert_eq!(outline.colored_boxes[1].color, ENTITY_EYE_HEIGHT_COLOR);
    assert_selection_box_close(
        outline.colored_boxes[1].min,
        [-0.3, 1.0 + eye_height - 0.01, 2.7],
    );
    assert_selection_box_close(
        outline.colored_boxes[1].max,
        [0.3, 1.0 + eye_height + 0.01, 3.3],
    );
    assert_eq!(outline.lines[0].color, ENTITY_VIEW_VECTOR_COLOR);
    assert_selection_box_close(outline.lines[0].from, [0.0, 1.0 + eye_height, 3.0]);
    assert_selection_box_close(outline.lines[0].to, [-1.7320508, 2.0 + eye_height, 3.0]);
    assert_selection_box_close(outline.points[0].position, [0.0, 1.0, 3.0]);
    assert_eq!(outline.points[0].size, ENTITY_POSITION_POINT_SIZE);
}

#[test]
fn entity_scene_outline_projects_ender_dragon_parent_and_part_hitboxes() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        100,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        [0.0, 64.0, 0.0],
    ));
    world.advance_entity_client_animations(1);

    let outline = entity_scene_outline_from_world_at_partial_tick(&world, 1.0)
        .expect("expected dragon debug outline");

    assert!(outline.boxes.is_empty());
    assert_eq!(outline.colored_boxes.len(), 10);
    assert_eq!(outline.colored_boxes[0].color, ENTITY_HITBOX_COLOR);
    assert!(
        (outline.colored_boxes[0].max[0] - outline.colored_boxes[0].min[0] - 16.0).abs() < 1e-5
    );
    assert!((outline.colored_boxes[0].max[1] - outline.colored_boxes[0].min[1] - 8.0).abs() < 1e-5);
    assert_eq!(outline.colored_boxes[1].color, ENTITY_EYE_HEIGHT_COLOR);
    assert_selection_box_close(outline.colored_boxes[1].min, [-8.0, 70.79, -8.0]);
    assert_selection_box_close(outline.colored_boxes[1].max, [8.0, 70.81, 8.0]);
    assert_eq!(outline.points.len(), 1);
    assert_eq!(outline.points[0].position, [0.0, 64.0, 0.0]);
    assert_eq!(outline.lines.len(), 1);
    assert_eq!(outline.lines[0].color, ENTITY_VIEW_VECTOR_COLOR);
    assert_selection_box_close(outline.lines[0].from, [0.0, 70.8, 0.0]);
    assert_selection_box_close(outline.lines[0].to, [0.0, 70.8, 2.0]);
    for part_box in &outline.colored_boxes[2..] {
        assert_eq!(part_box.color, ENTITY_DRAGON_PART_HITBOX_COLOR);
    }
    assert!((outline.colored_boxes[2].max[0] - outline.colored_boxes[2].min[0] - 1.0).abs() < 1e-5);
    assert!((outline.colored_boxes[2].max[1] - outline.colored_boxes[2].min[1] - 1.0).abs() < 1e-5);
    assert!((outline.colored_boxes[8].max[0] - outline.colored_boxes[8].min[0] - 4.0).abs() < 1e-5);
    assert!((outline.colored_boxes[8].max[1] - outline.colored_boxes[8].min[1] - 2.0).abs() < 1e-5);
}

#[test]
fn entity_model_instances_project_chicken_adult_and_baby_models() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        26,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        27,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        85,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [5.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 27,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, true)],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![
                EntityModelInstance::chicken(26, [1.0, 64.0, -2.0], 0.0, false),
                EntityModelInstance::chicken(27, [3.0, 64.0, -2.0], 0.0, true).with_age_scale(0.5),
                EntityModelInstance::new(85, EntityModelKind::Minecart, [5.0, 64.0, -2.0], 0.0,),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_project_sheep_eat_grass_head_pose() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        70,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));

    // At rest both entities resolve to the resting head pose.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    assert_eq!(resting[0].render_state.head_eat, SheepHeadEatPose::NONE);
    assert_eq!(resting[1].render_state.head_eat, SheepHeadEatPose::NONE);

    // Vanilla SheepRenderer.extractRenderState projects the eat animation
    // through the partial tick; the chicken stays at rest.
    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 70,
        event_id: 10,
    }));
    let eating = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);
    assert_eq!(
        eating[0].render_state.head_eat,
        SheepHeadEatPose::from_eat_tick(40, 0.5)
    );
    assert_ne!(eating[0].render_state.head_eat, SheepHeadEatPose::NONE);
    assert_eq!(eating[1].render_state.head_eat, SheepHeadEatPose::NONE);

    // The pose follows the canonical countdown as it decrements.
    world.advance_entity_client_animations(20);
    let mid = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    assert_eq!(
        mid[0].render_state.head_eat,
        SheepHeadEatPose::from_eat_tick(20, 0.0)
    );
}

#[test]
fn entity_model_instances_project_warden_tendril_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        94,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
        [1.0, 64.0, -2.0],
    ));

    // A warden at rest projects no tendril pulse, so WardenModel.animateTendrils holds at bind.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(resting[0].render_state.tendril_animation, 0.0);

    // Vanilla Warden.handleEntityEvent(61) resets tendrilAnimation to 10; getTendrilAnimation
    // lerps (tendrilAnimationO, tendrilAnimation) / 10. After three client ticks the pair is
    // (8, 7), so at partialTick 1.0 the projected pulse is 7/10.
    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 94,
        event_id: 61,
    }));
    world.advance_entity_client_animations(3);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.tendril_animation,
        7.0 / 10.0,
        "the projected tendril pulse drives the WardenModel.animateTendrils antenna sway"
    );
}

#[test]
fn entity_model_instances_project_warden_heart_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        96,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
        [1.0, 64.0, -2.0],
    ));

    // A warden between heartbeats projects no heart pulse, so the heart overlay's alpha is 0.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(resting[0].render_state.heart_animation, 0.0);

    // With no synced anger, vanilla `Warden.getHeartBeatDelay()` is the calm 40, so the heartbeat
    // (`tickCount % 40 == 0`) first fires on the 40th client tick: `heartAnimation = 10`, then
    // `heartAnimationO = 10; heartAnimation--`, leaving the pair (10, 9). At partialTick 1.0
    // `getHeartAnimation` lerps to 9/10.
    world.advance_entity_client_animations(40);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.heart_animation,
        9.0 / 10.0,
        "the projected heartbeat pulse drives the warden heart emissive overlay's alpha"
    );
}

#[test]
fn entity_model_instances_project_squid_out_of_water_tentacle_and_body_tilt_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        95,
        VANILLA_ENTITY_TYPE_SQUID_ID,
        [1.0, 64.0, -2.0],
    ));
    // The out-of-water branch ignores horizontal swim velocity for pose, but the motion packet
    // keeps this test on the same projection path as the in-water branch.
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 95,
        position: Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: Vec3d {
            x: 0.2,
            y: -0.1,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: false,
    }));

    // A floating squid at rest projects the bind pose into the render state.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let resting = resting
        .iter()
        .find(|instance| instance.entity_id == 95)
        .unwrap();
    assert_eq!(resting.render_state.squid_tentacle_angle, 0.0);
    assert_eq!(resting.render_state.squid_x_body_rot, 0.0);
    assert_eq!(resting.render_state.squid_z_body_rot, 0.0);

    // One tick out of water uses the suffocating branch: tentacles flex with
    // `abs(sin(tentacleMovement))`, xBodyRot eases toward -90 degrees, and zBodyRot is untouched.
    world.advance_entity_client_animations(1);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let squid = instances
        .iter()
        .find(|instance| instance.entity_id == 95)
        .unwrap();
    assert!(
        squid.render_state.squid_tentacle_angle > 0.0,
        "the projected tentacle angle drives SquidModel.setupAnim: {}",
        squid.render_state.squid_tentacle_angle
    );
    assert!(
        squid.render_state.squid_x_body_rot < 0.0,
        "an out-of-water squid projects a negative body pitch: {}",
        squid.render_state.squid_x_body_rot
    );
    assert_eq!(
        squid.render_state.squid_z_body_rot, 0.0,
        "out of water leaves the swim roll untouched"
    );
}

#[test]
fn entity_model_instances_project_squid_body_yaw_from_world_animation() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_rotation(
        96,
        VANILLA_ENTITY_TYPE_SQUID_ID,
        [1.0, 64.0, -2.0],
        20.0,
        5.0,
        30.0,
    ));

    // Vanilla `LivingEntity.recreateFromPacket` seeds squid `yBodyRot` from
    // the head yaw. A dry squid does not refine that yaw in `Squid.aiStep`, so
    // the native instance must use 30 as `bodyRot` and keep the head yaw
    // relative to that projected body yaw, not the synced transform yaw 20.
    world.advance_entity_client_animations(1);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let squid = instances
        .iter()
        .find(|instance| instance.entity_id == 96)
        .unwrap();
    assert_eq!(squid.render_state.body_rot, 30.0);
    assert_eq!(squid.render_state.head_yaw, 0.0);
    assert_eq!(squid.render_state.head_pitch, 5.0);
    assert!(
        squid.render_state.squid_x_body_rot < 0.0,
        "the same world animation state still feeds SquidRenderer body pitch"
    );
}

#[test]
fn entity_model_instances_project_chicken_wing_flap_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        96,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    // Mark the chicken airborne so `Chicken.aiStep` builds flap speed and advances
    // the flap phase (vanilla `onGround()` false branch).
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 96,
        position: Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: -0.1,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: false,
    }));

    // An unticked chicken projects the bind pose (wings held) into the render state.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let resting = resting
        .iter()
        .find(|instance| instance.entity_id == 96)
        .unwrap();
    assert_eq!(resting.render_state.chicken_flap, 0.0);
    assert_eq!(resting.render_state.chicken_flap_speed, 0.0);

    // After ticking airborne, the world-side flap accumulator develops a non-zero
    // flap phase and a saturated flap speed, both of which flow through
    // EntityModelSourceState into the renderer EntityRenderState (`ChickenModel.setupAnim`
    // wing zRot).
    world.advance_entity_client_animations(3);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let chicken = instances
        .iter()
        .find(|instance| instance.entity_id == 96)
        .unwrap();
    assert!(
        chicken.render_state.chicken_flap > 0.0,
        "the projected flap phase drives ChickenModel.setupAnim: {}",
        chicken.render_state.chicken_flap
    );
    assert!(
        chicken.render_state.chicken_flap_speed > 0.0,
        "an airborne chicken projects a non-zero flap speed: {}",
        chicken.render_state.chicken_flap_speed
    );
}

#[test]
fn entity_model_instances_project_slime_squish_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_SLIME_ID,
        [1.0, 64.0, -2.0],
    ));
    // Mark the slime grounded so `Slime.tick` sees the air→ground transition and
    // seeds the landing squish target (vanilla `onGround()` true branch from rest).
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 97,
        position: Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: true,
    }));

    // An unticked slime projects the undeformed cube (squish 0) into the render state.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let resting = resting
        .iter()
        .find(|instance| instance.entity_id == 97)
        .unwrap();
    assert_eq!(resting.render_state.slime_squish, 0.0);

    // After ticking on the ground, the world-side squish accumulator eases toward
    // the negative landing target, and that flows through EntityModelSourceState
    // into the renderer EntityRenderState (`SlimeRenderer.scale` body stretch).
    world.advance_entity_client_animations(2);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let slime = instances
        .iter()
        .find(|instance| instance.entity_id == 97)
        .unwrap();
    assert!(
        slime.render_state.slime_squish < 0.0,
        "the projected landing squish drives SlimeRenderer.scale: {}",
        slime.render_state.slime_squish
    );
}

#[test]
fn entity_model_instances_project_evoker_fangs_bite_progress_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        98,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
        [1.0, 64.0, -2.0],
    ));

    let bite = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .iter()
            .find(|instance| instance.entity_id == 98)
            .map(|instance| instance.render_state.evoker_fangs_bite_progress)
    };

    // A fang that has not started its attack is hidden underground: biteProgress 0.
    assert_eq!(bite(&world), Some(0.0));

    // Vanilla `EvokerFangs.handleEntityEvent`: event 4 starts the attack, and the
    // `lifeTicks` countdown drives the biteProgress ramp above 0, flowing through
    // EntityModelSourceState into the renderer EntityRenderState
    // (`EvokerFangsModel.setupAnim` jaw snap / rise / vanish).
    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 98,
        event_id: 4,
    }));
    world.advance_entity_client_animations(3);
    let progress = bite(&world).expect("the attacking fang projects an instance");
    assert!(
        progress > 0.0,
        "the projected bite ramp drives EvokerFangsModel.setupAnim: {progress}"
    );
}

#[test]
fn entity_model_instances_project_camel_dash_seconds_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        99,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        [1.0, 64.0, -2.0],
    ));

    let dash = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .iter()
            .find(|instance| instance.entity_id == 99)
            .map(|instance| {
                (
                    instance.render_state.camel_dash_seconds,
                    instance.render_state.camel_idle_seconds,
                    instance.render_state.camel_jump_cooldown,
                )
            })
    };

    // A non-dashing camel projects the stopped-animation sentinel.
    assert_eq!(dash(&world), Some((-1.0, -1.0, 0.0)));

    // Vanilla `Camel.setupAnimationStates`: the synced `DASH` boolean starts `dashAnimationState`,
    // and the elapsed seconds flow through EntityModelSourceState into the renderer EntityRenderState
    // (`CamelModel.setupAnim` looping `CAMEL_DASH` gallop).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 99,
        values: vec![camel_dash_data(true)],
    }));
    world.advance_entity_client_animations(2);
    let progress = dash(&world).expect("the dashing camel projects an instance");
    assert!(
        progress.0 >= 0.0,
        "the projected dash timer drives CamelModel.setupAnim: {progress:?}"
    );
    assert!(
        (progress.1 - 0.1).abs() < 1.0e-6,
        "the projected idle timer drives CamelModel.setupAnim: {progress:?}"
    );
    assert_eq!(
        progress.2, 52.0,
        "the projected dash cooldown drives CamelModel.applyHeadRotation"
    );
}

#[test]
fn entity_model_instances_project_armadillo_peek_seconds_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        100,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID,
        [1.0, 64.0, -2.0],
    ));

    let peek = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .iter()
            .find(|instance| instance.entity_id == 100)
            .map(|instance| instance.render_state.armadillo_peek_seconds)
    };

    assert_eq!(peek(&world), Some(-1.0));

    // Vanilla `Armadillo.setupAnimationStates`: entering SCARED starts `peekAnimationState`
    // and fast-forwards it by 50 ticks; that elapsed value flows through EntityModelSourceState
    // into the renderer EntityRenderState (`ArmadilloModel.setupAnim` `ARMADILLO_PEEK`).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 100,
        values: vec![protocol_armadillo_state_data(2)],
    }));
    let fast_forwarded = peek(&world).expect("the scared armadillo projects an instance");
    assert!((fast_forwarded - 2.5).abs() < 1.0e-6);

    // Event 64 restarts the peek on the next client tick.
    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 100,
        event_id: 64,
    }));
    world.advance_entity_client_animations(1);
    let restarted = peek(&world).expect("the restarted armadillo projects an instance");
    assert!((restarted - 0.0).abs() < 1.0e-6);
}

#[test]
fn entity_model_instances_project_axolotl_play_dead_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        95,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        [1.0, 64.0, -2.0],
    ));

    let play_dead = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .iter()
            .find(|instance| instance.entity_id == 95)
            .map(|instance| instance.render_state.axolotl_playing_dead_factor)
    };

    // An awake axolotl projects no play-dead blend.
    assert_eq!(play_dead(&world), Some(0.0));

    // Vanilla `Axolotl.playingDeadAnimator`: the synced `DATA_PLAYING_DEAD` flag eases the
    // factor up, flowing through EntityModelSourceState into the renderer EntityRenderState
    // (`AdultAxolotlModel.setupPlayDeadAnimation` limp-on-its-side pose).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 95,
        values: vec![axolotl_playing_dead_data(true)],
    }));
    world.advance_entity_client_animations(3);
    let factor = play_dead(&world).expect("the play-dead axolotl projects an instance");
    assert!(
        factor > 0.0,
        "the projected play-dead factor drives AdultAxolotlModel.setupAnim: {factor}"
    );
}

#[test]
fn entity_model_instances_project_baby_axolotl_animation_states() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_AXOLOTL_ID, true)],
    }));
    world.advance_entity_client_animations(1);

    let timers = |world: &WorldStore, partial_tick: f32| {
        let instances =
            entity_model_instances_from_world_at_partial_tick(world, None, partial_tick);
        let state = &instances
            .iter()
            .find(|instance| instance.entity_id == 97)
            .expect("baby axolotl instance")
            .render_state;
        [
            state.axolotl_baby_swim_seconds,
            state.axolotl_baby_walk_seconds,
            state.axolotl_baby_walk_underwater_seconds,
            state.axolotl_baby_idle_underwater_seconds,
            state.axolotl_baby_idle_underwater_on_ground_seconds,
            state.axolotl_baby_idle_on_ground_seconds,
            state.axolotl_baby_play_dead_seconds,
        ]
    };
    assert_eq!(
        timers(&world, 0.5),
        [-1.0, -1.0, -1.0, -1.0, -1.0, 0.025, -1.0]
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![axolotl_playing_dead_data(true)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(
        timers(&world, 0.5),
        [-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, 0.025]
    );
}

#[test]
fn entity_model_instances_project_allay_dance_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        96,
        VANILLA_ENTITY_TYPE_ALLAY_ID,
        [1.0, 64.0, -2.0],
    ));

    let dance = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .iter()
            .find(|instance| instance.entity_id == 96)
            .map(|instance| {
                (
                    instance.render_state.allay_dancing,
                    instance.render_state.allay_spinning,
                    instance.render_state.allay_spinning_progress,
                )
            })
    };

    // A non-dancing allay projects the inert dance state (head-look pose, no spin).
    assert_eq!(dance(&world), Some((false, false, 0.0)));

    // Vanilla `Allay.tick`: the synced `DATA_DANCING` flag opens the dance, and the spin
    // sub-window state flows through EntityModelSourceState into the renderer EntityRenderState
    // (`AllayModel.setupAnim` dance branch: body spin/sway + head tilt).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 96,
        values: vec![allay_dancing_data(true)],
    }));
    world.advance_entity_client_animations(1);
    let (dancing, spinning, progress) =
        dance(&world).expect("the dancing allay projects an instance");
    assert!(
        dancing,
        "the synced flag drives AllayModel.setupAnim's dance branch"
    );
    assert!(spinning, "the dance opens in the spin sub-window");
    assert!(
        progress > 0.0,
        "the projected spin ramp drives the body spin: {progress}"
    );
}

#[test]
fn entity_model_instances_project_allay_holding_item_progress_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_ALLAY_ID,
        [1.0, 64.0, -2.0],
    ));

    let holding_progress = |world: &WorldStore, partial: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial)
            .iter()
            .find(|instance| instance.entity_id == 97)
            .map(|instance| instance.render_state.allay_holding_item_progress)
    };

    assert_eq!(holding_progress(&world, 1.0), Some(0.0));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 97,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));
    world.advance_entity_client_animations(3);

    let progress = holding_progress(&world, 1.0)
        .expect("the allay projects a renderer instance after ticking");
    assert!(
            (progress - 0.6).abs() < 1.0e-6,
            "native forwards Allay.getHoldingItemAnimationProgress into EntityModelRenderState: {progress}"
        );
}

#[test]
fn entity_model_instances_project_head_look_from_world() {
    let mut world = WorldStore::new();
    // Body yaw 30, head yaw 100, pitch -20: net head yaw =
    // wrapDegrees(100 - 30) = 70, head pitch = -20.
    world.apply_add_entity(protocol_add_entity_with_rotation(
        70,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
        30.0,
        -20.0,
        100.0,
    ));
    // Head aligned with body and level: no look turn.
    world.apply_add_entity(protocol_add_entity_with_rotation(
        71,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
        45.0,
        0.0,
        45.0,
    ));
    // Body yaw 10, head yaw 200: diff 190 wraps to -170 (shortest turn).
    world.apply_add_entity(protocol_add_entity_with_rotation(
        72,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [5.0, 64.0, -2.0],
        10.0,
        5.0,
        200.0,
    ));
    // After the spawn frame, the existing movement-owned body-yaw path uses
    // packet y_rot; the spawn-only LivingEntity head-yaw seed is cleared.
    world.advance_entity_client_animations(1);

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let find = |id: i32| {
        instances
            .iter()
            .find(|instance| instance.entity_id == id)
            .unwrap_or_else(|| panic!("missing entity {id}"))
    };

    let sheep = find(70).render_state;
    assert_eq!(sheep.head_yaw, 70.0);
    assert_eq!(sheep.head_pitch, -20.0);

    let chicken = find(71).render_state;
    assert_eq!(chicken.head_yaw, 0.0);
    assert_eq!(chicken.head_pitch, 0.0);

    let wrapped = find(72).render_state;
    assert_eq!(wrapped.head_yaw, -170.0);
    assert_eq!(wrapped.head_pitch, 5.0);
}

#[test]
fn entity_model_instances_project_polar_bear_standing_scale() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        80,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));

    // A polar bear on all fours and any other entity carry a zero scale.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(resting[0].render_state.polar_bear_stand_scale, 0.0);
    assert_eq!(resting[1].render_state.polar_bear_stand_scale, 0.0);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 80,
        values: vec![polar_bear_standing_data(true)],
    }));
    world.advance_entity_client_animations(1);

    // Vanilla PolarBearRenderer.extractRenderState reads
    // getStandingAnimationScale(partialTick); after one tick that is
    // lerp(0.5, 0, 1) / 6.
    let standing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);
    assert_eq!(standing[0].render_state.polar_bear_stand_scale, 0.5 / 6.0);
    assert_eq!(standing[1].render_state.polar_bear_stand_scale, 0.0);
}

#[test]
fn entity_model_instances_project_shulker_peek() {
    let accessors = shulker_metadata_accessors();
    // Vanilla Shulker.DATA_PEEK_ID (BYTE), a 0..=100 percentage; the client peek state
    // advances 0.05/tick toward raw·0.01 and the render state reads the partial-tick lerp
    // `Shulker.getClientPeekAmount` (`Mth.lerp(partialTick, currentPeekAmountO, current)`).
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        82,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        83,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));

    let peek = |world: &WorldStore, id: i32, partial: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .shulker_peek
    };

    // A closed shulker and every other entity carry a zero peek (the closed/bind pose).
    assert_eq!(peek(&world, 82, 1.0), 0.0);
    assert_eq!(peek(&world, 83, 1.0), 0.0);

    // Open the lid fully (raw 100 → target 1.0), then advance one tick: the client peek steps
    // 0.05 from 0. At partial-tick 0.5 the render state lerps `0 + 0.5·(0.05 − 0) = 0.025`.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![protocol_byte_data(accessors.peek_data_id, 100)],
    }));
    world.advance_entity_client_animations(1);
    assert!((peek(&world, 82, 0.5) - 0.025).abs() < 1.0e-6);
    // The chicken has no peek state, so it stays at the closed/bind pose.
    assert_eq!(peek(&world, 83, 0.5), 0.0);
}

#[test]
fn entity_model_instances_project_shulker_attach_face() {
    let accessors = shulker_metadata_accessors();
    // Vanilla Shulker.DATA_ATTACH_FACE_ID (DIRECTION) feeds
    // `ShulkerRenderState.attachFace`, whose default is `Direction.DOWN`.
    const DIRECTION_NORTH: i32 = 2;
    const DIRECTION_NEGATIVE_ONE: i32 = -1;

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        82,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        83,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));

    let attach_face = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .shulker_attach_face
    };

    assert_eq!(attach_face(&world, 82), EntityAttachmentFace::Down);
    assert_eq!(attach_face(&world, 83), EntityAttachmentFace::Down);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![protocol_direction_data(
            accessors.attach_face_data_id,
            DIRECTION_NORTH,
        )],
    }));
    assert_eq!(attach_face(&world, 82), EntityAttachmentFace::North);
    assert_eq!(attach_face(&world, 83), EntityAttachmentFace::Down);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![protocol_direction_data(
            accessors.attach_face_data_id,
            DIRECTION_NEGATIVE_ONE,
        )],
    }));
    // Vanilla `Direction.BY_ID` uses positive-modulo wrap, so -1 wraps to EAST.
    assert_eq!(attach_face(&world, 82), EntityAttachmentFace::East);
}

#[test]
fn entity_model_instances_project_death_animation_time() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        82,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));

    // A living entity at rest carries deathTime 0 and no red overlay.
    let alive = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    assert_eq!(alive[0].render_state.death_time, 0.0);
    assert!(!alive[0].render_state.has_red_overlay);

    // Vanilla isDeadOrDying(): health <= 0 starts the death counter; tickDeath
    // increments it each client tick, projected (plus the partial tick) as
    // LivingEntityRenderState.deathTime and driving the red overlay.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![living_entity_health_data(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            0.0,
        )],
    }));
    world.advance_entity_client_animations(2);
    let dying = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
    assert_eq!(dying[0].render_state.death_time, 2.25);
    assert!(dying[0].render_state.has_red_overlay);
}

#[test]
fn entity_model_instances_project_creaking_teardown_flicker_without_death_overlay() {
    let (tearing_down_data_id, tearing_down_default) =
        bbb_world::vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated Creaking tearing-down accessor");
    assert!(!tearing_down_default);

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        822,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
        [1.0, 64.0, -2.0],
    ));
    // Keep synced `isActive()` true to prove teardown ignores it in favor of the local flicker.
    // Health zero starts the shared death/red-overlay source state exactly as the server's
    // `CreakingHeartBlockEntity.removeProtector` does alongside `setTearingDown()`.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 822,
        values: vec![
            protocol_bool_data(creaking_active_data_id(), true),
            protocol_bool_data(tearing_down_data_id, true),
            living_entity_health_data(VANILLA_ENTITY_TYPE_CREAKING_ID, 0.0),
        ],
    }));

    // First teardown tick: deathTime 1 schedules the false-eye 0..=0 interval and toggles on.
    world.advance_entity_client_animations(1);
    let first = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 822)
        .unwrap();
    assert_eq!(first.kind, EntityModelKind::Creaking { eyes_glowing: true });
    assert_eq!(first.render_state.death_time, 0.0);
    assert!(!first.render_state.has_red_overlay);

    // Second tick schedules from the glowing 2..=8 interval and toggles off. Synced active remains
    // true, so a false kind here proves native projection selected `hasGlowingEyes()`.
    world.advance_entity_client_animations(1);
    let second = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 822)
        .unwrap();
    assert_eq!(
        second.kind,
        EntityModelKind::Creaking {
            eyes_glowing: false
        }
    );
    assert_eq!(second.render_state.death_time, 0.0);
    assert!(!second.render_state.has_red_overlay);
}

#[test]
fn entity_model_instances_project_ender_dragon_death_time() {
    // Vanilla `EnderDragonRenderer.extractRenderState` reads the dragon-specific
    // `dragonDeathTime`, not the generic 20-tick `LivingEntity.deathTime`, before selecting the
    // dying dissolve body render type.
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 83,
        "entity_type_id": VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
        "y_rot": 0.0,
        "ender_dragon_death_time": 3.5,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 0.0, None).unwrap();

    assert_eq!(instance.kind, EntityModelKind::EnderDragon);
    assert_eq!(instance.render_state.death_time, 0.0);
    assert_eq!(instance.render_state.ender_dragon_death_time, 3.5);
}

#[test]
fn entity_model_instances_fold_freeze_shake_into_body_rot() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        83,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));

    // A living entity that is not frozen solid has an unshaken body yaw.
    let warm = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    assert_eq!(warm[0].render_state.body_rot, 0.0);

    // Vanilla Entity.isFullyFrozen(): ticksFrozen >= 140. setupRotations then
    // adds cos(floor(ageInTicks) * 3.25) * π * 0.4 to the body yaw; the shake
    // uses the floored (integer) tick count, so it does not lerp with partial.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 83,
        values: vec![entity_ticks_frozen_data(140)],
    }));
    world.advance_entity_client_animations(2);
    let frozen = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
    let expected_shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
    assert!((frozen[0].render_state.body_rot - expected_shake).abs() < 1e-6);
    // The head turn relative to the body is unchanged by the shake.
    assert_eq!(frozen[0].render_state.head_yaw, 0.0);
}

#[test]
fn entity_model_instances_shake_zombie_family_while_converting() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        84,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        85,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        [3.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        86,
        VANILLA_ENTITY_TYPE_HUSK_ID,
        [5.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        87,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
        [7.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(2);

    let shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
    let body_rot = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .body_rot
    };

    // A non-converting zombie / zombie villager does not shake.
    assert_eq!(body_rot(&world, 84), 0.0);
    assert_eq!(body_rot(&world, 85), 0.0);

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 84,
        values: vec![protocol_zombie_villager_converting_data(true)],
    }));
    assert_eq!(
        body_rot(&world, 84),
        0.0,
        "Zombie does not own ZombieVillager.DATA_CONVERTING_ID"
    );

    // AbstractZombieRenderer.isShaking ORs in DATA_DROWNED_CONVERSION_ID (18) for the
    // registered Zombie, Husk, and Drowned renderers.
    for (id, entity_type_id) in [
        (84, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        (86, VANILLA_ENTITY_TYPE_HUSK_ID),
        (87, VANILLA_ENTITY_TYPE_DROWNED_ID),
    ] {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![protocol_zombie_drowned_conversion_data(
                entity_type_id,
                true
            )],
        }));
        assert!((body_rot(&world, id) - shake).abs() < 1e-6);
    }

    // ZombieVillagerRenderer does not inherit AbstractZombieRenderer, so inherited
    // Zombie.DATA_DROWNED_CONVERSION_ID does not affect its shake projection.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 85,
        values: vec![protocol_zombie_drowned_conversion_data(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            true,
        )],
    }));
    assert_eq!(body_rot(&world, 85), 0.0);

    // ZombieVillagerRenderer independently ORs in DATA_CONVERTING_ID (19).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 85,
        values: vec![protocol_zombie_villager_converting_data(true)],
    }));
    assert!((body_rot(&world, 85) - shake).abs() < 1e-6);
}

#[test]
fn entity_model_instances_shake_striders_while_suffocating() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        86,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        87,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [3.0, 64.0, -2.0],
    ));
    // Keep this renderer-only fixture's synchronized warm/cold state under
    // explicit test control. Vanilla Strider.tick recomputes suffocation in
    // air unless noAI is set, which is now part of the native client tick.
    let mob_accessors = bbb_world::vanilla_entity_metadata_mob_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_STRIDER_ID,
    )
    .expect("generated Strider Mob accessors");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 86,
        values: vec![protocol_byte_data(
            mob_accessors.flags_data_id,
            mob_accessors.no_ai_mask as i8,
        )],
    }));
    world.advance_entity_client_animations(2);

    let strider = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.25)
            .into_iter()
            .find(|instance| instance.entity_id == 86)
            .unwrap()
    };

    // A warm strider does not shake and keeps the non-cold texture variant.
    let warm = strider(&world);
    assert_eq!(warm.render_state.body_rot, 0.0);
    assert_eq!(
        warm.kind,
        EntityModelKind::Strider {
            baby: false,
            cold: false
        }
    );

    // StriderRenderer.isShaking ORs in StriderRenderState.isSuffocating,
    // which is extracted from DATA_SUFFOCATING (19). The same flag also
    // selects the cold texture.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 86,
        values: vec![strider_suffocating_data(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            true,
        )],
    }));
    let shaking = strider(&world);
    let expected_shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
    assert!((shaking.render_state.body_rot - expected_shake).abs() < 1e-6);
    assert_eq!(shaking.render_state.head_yaw, 0.0);
    assert_eq!(
        shaking.kind,
        EntityModelKind::Strider {
            baby: false,
            cold: true
        }
    );

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 87,
        values: vec![strider_suffocating_data(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            true,
        )],
    }));
    let pig_body_rot = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25)
        .into_iter()
        .find(|instance| instance.entity_id == 87)
        .unwrap()
        .render_state
        .body_rot;
    assert_eq!(
        pig_body_rot, 0.0,
        "Pig does not own Strider.DATA_SUFFOCATING"
    );
}

#[test]
fn entity_model_instances_shake_skeletons_while_freeze_converting() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        87,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        88,
        VANILLA_ENTITY_TYPE_STRAY_ID,
        [3.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(2);

    let body_rot = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 87)
            .unwrap()
            .render_state
            .body_rot
    };
    assert_eq!(body_rot(&world), 0.0);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 87,
        values: vec![skeleton_freeze_conversion_data(
            VANILLA_ENTITY_TYPE_SKELETON_ID,
            true,
        )],
    }));
    let expected_shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
    assert!((body_rot(&world) - expected_shake).abs() < 1e-6);

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![skeleton_freeze_conversion_data(
            VANILLA_ENTITY_TYPE_SKELETON_ID,
            true,
        )],
    }));
    let stray_body_rot = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 88)
        .unwrap()
        .render_state
        .body_rot;
    assert_eq!(
        stray_body_rot, 0.0,
        "Stray does not own Skeleton.DATA_STRAY_CONVERSION_ID"
    );
}

#[test]
fn entity_model_instances_shake_piglins_and_hoglins_while_zombifying() {
    fn dimension_world(dimension_type_id: i32, dimension: &str) -> WorldStore {
        let mut world = WorldStore::new();
        let mut login = protocol_play_login(10);
        login.levels = vec![dimension.to_string()];
        login.common_spawn_info.dimension_type_id = dimension_type_id;
        login.common_spawn_info.dimension = dimension.to_string();
        world.apply_login(&login);
        insert_loaded_entity_chunk(&mut world, ChunkPos { x: 0, z: -1 });
        world
    }

    fn add_piglin_family(world: &mut WorldStore) {
        for (id, entity_type_id, position) in [
            (87, VANILLA_ENTITY_TYPE_PIGLIN_ID, [1.0, 64.0, -2.0]),
            (88, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, [3.0, 64.0, -2.0]),
            (89, VANILLA_ENTITY_TYPE_HOGLIN_ID, [5.0, 64.0, -2.0]),
            (90, VANILLA_ENTITY_TYPE_ZOGLIN_ID, [7.0, 64.0, -2.0]),
            (
                91,
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
                [9.0, 64.0, -2.0],
            ),
        ] {
            world.apply_add_entity(protocol_add_entity(id, entity_type_id, position));
        }
        world.advance_entity_client_animations(2);
    }

    fn body_rot(world: &WorldStore, id: i32) -> f32 {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .body_rot
    }

    let shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;

    let mut overworld = dimension_world(0, "minecraft:overworld");
    add_piglin_family(&mut overworld);

    // Vanilla `PiglinRenderer` and `HoglinRenderer` OR `isConverting()` into
    // `isShaking`. The built-in Overworld keeps
    // `EnvironmentAttributes.PIGLINS_ZOMBIFY` at its default `true`.
    for id in [87, 88, 89] {
        assert!((body_rot(&overworld, id) - shake).abs() < 1e-6);
    }
    // ZoglinRenderer and ZombifiedPiglinRenderer do not expose a conversion override.
    for id in [90, 91] {
        assert_eq!(body_rot(&overworld, id), 0.0);
    }

    // Synced immune-to-zombification metadata suppresses the conversion shake.
    assert!(overworld.apply_set_entity_data(SetEntityData {
        id: 87,
        values: vec![zombification_immunity_data(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            true,
        )],
    }));
    assert!(overworld.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![zombification_immunity_data(
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
            true,
        )],
    }));
    assert!(overworld.apply_set_entity_data(SetEntityData {
        id: 89,
        values: vec![zombification_immunity_data(
            VANILLA_ENTITY_TYPE_HOGLIN_ID,
            true,
        )],
    }));
    for id in [87, 88, 89] {
        assert_eq!(body_rot(&overworld, id), 0.0);
    }

    let mut nether = dimension_world(1, "minecraft:the_nether");
    add_piglin_family(&mut nether);
    // Vanilla built-in Nether sets `PIGLINS_ZOMBIFY` to false.
    for id in [87, 88, 89, 90, 91] {
        assert_eq!(body_rot(&nether, id), 0.0);
    }

    let mut custom = WorldStore::new();
    custom.record_registry_entries(
        bbb_world::DIMENSION_TYPE_REGISTRY,
        0,
        vec![bbb_world::RegistryPacketEntry::with_raw_data(
            "example:custom",
            piglins_zombify_dimension_nbt(false),
        )],
    );
    custom.apply_login(&protocol_play_login(10));
    add_piglin_family(&mut custom);
    for id in [87, 88, 89, 90, 91] {
        assert_eq!(body_rot(&custom, id), 0.0);
    }
}

#[test]
fn entity_model_instances_project_auto_spin_attack() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        86,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    // A non-living entity never carries the living-entity flags byte.
    world.apply_add_entity(protocol_add_entity(
        87,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [3.0, 64.0, -2.0],
    ));

    let auto_spin = |world: &WorldStore, id: i32, partial: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .auto_spin_age_ticks
    };

    // A living entity at rest is not spinning.
    assert_eq!(auto_spin(&world, 86, 0.0), None);

    // Vanilla LivingEntity.isAutoSpinAttack(): DATA_LIVING_ENTITY_FLAGS & 4.
    // setupRotations then reads the lerped ageInTicks (tickCount + partial).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 86,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            true,
            false,
            true,
        )],
    }));
    world.advance_entity_client_animations(3);
    assert_eq!(auto_spin(&world, 86, 0.5), Some(3.5));

    // Clearing the spin bit (other living flags still set) stops the spin.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 86,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            true,
            false,
            false,
        )],
    }));
    assert_eq!(auto_spin(&world, 86, 0.5), None);

    // The living-entity gate keeps a non-living entity from ever spinning, even
    // if a stray flags byte is present.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 87,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            false,
            false,
            true,
        )],
    }));
    assert_eq!(auto_spin(&world, 87, 0.5), None);
}

#[test]
fn entity_model_instances_project_boat_rowing_and_damage_times() {
    const ADVANCE: f32 = std::f32::consts::PI / 8.0;
    let boat_accessors = abstract_boat_metadata_accessors();
    let vehicle_accessors = vehicle_metadata_accessors(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        90,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        91,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [2.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 90,
        passenger_ids: vec![91],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 90,
        values: vec![
            protocol_int_data(vehicle_accessors.hurt_time_data_id, 10),
            protocol_int_data(vehicle_accessors.hurt_dir_data_id, -1),
            protocol_float_data(vehicle_accessors.damage_data_id, 20.0),
            protocol_bool_data(boat_accessors.paddle_left_data_id, true),
            protocol_bool_data(boat_accessors.paddle_right_data_id, true),
            protocol_int_data(boat_accessors.bubble_time_data_id, 60),
        ],
    }));

    world.advance_entity_client_animations(2);
    let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
        .into_iter()
        .find(|instance| instance.entity_id == 90)
        .unwrap()
        .render_state;

    assert!((render_state.boat_rowing_time_left - ADVANCE * 1.5).abs() < 1.0e-6);
    assert!((render_state.boat_rowing_time_right - ADVANCE * 1.5).abs() < 1.0e-6);
    assert!((render_state.boat_hurt_time - 7.5).abs() < 1.0e-6);
    assert_eq!(render_state.boat_hurt_dir, -1);
    assert!((render_state.boat_damage_time - 17.5).abs() < 1.0e-6);
    let first_bubble_angle = 10.0 * (0.5_f32).sin() * 0.05;
    let second_bubble_angle = 10.0 * (1.0_f32).sin() * 0.1;
    let expected_bubble_angle =
        first_bubble_angle + (second_bubble_angle - first_bubble_angle) * 0.5;
    assert!((render_state.boat_bubble_angle - expected_bubble_angle).abs() < 1.0e-6);
}

#[test]
fn entity_model_instances_project_minecart_damage_times() {
    let accessors = vehicle_metadata_accessors(VANILLA_ENTITY_TYPE_MINECART_ID);

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        93,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 93,
        values: vec![
            protocol_int_data(accessors.hurt_time_data_id, 10),
            protocol_int_data(accessors.hurt_dir_data_id, -1),
            protocol_float_data(accessors.damage_data_id, 20.0),
        ],
    }));

    world.advance_entity_client_animations(2);
    let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
        .into_iter()
        .find(|instance| instance.entity_id == 93)
        .unwrap()
        .render_state;

    assert!((render_state.minecart_hurt_time - 7.5).abs() < 1.0e-6);
    assert_eq!(render_state.minecart_hurt_dir, -1);
    assert!((render_state.minecart_damage_time - 17.5).abs() < 1.0e-6);
}

#[test]
fn entity_model_instances_project_tnt_minecart_fuse_remaining_ticks() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        95,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 95,
        event_id: 10,
    }));
    world.advance_entity_client_animations(76);

    let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
        .into_iter()
        .find(|instance| instance.entity_id == 95)
        .unwrap()
        .render_state;

    assert_eq!(render_state.minecart_tnt_fuse_remaining_in_ticks, 4.5);
}

#[test]
fn entity_model_instances_project_minecart_new_render_from_track_steps() {
    let mut world = WorldStore::new();
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec![
            "minecraft:vanilla".to_string(),
            "minecraft:minecart_improvements".to_string(),
        ],
    });
    world.apply_add_entity(protocol_add_entity(
        94,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(
        world.apply_move_minecart_along_track(MoveMinecartAlongTrack {
            entity_id: 94,
            lerp_steps: vec![MinecartStep {
                position: Vec3d {
                    x: 1.75,
                    y: 64.2,
                    z: -2.75,
                },
                movement: Vec3d {
                    x: 0.4,
                    y: 0.0,
                    z: -0.4,
                },
                y_rot: 90.0,
                x_rot: 5.0,
                weight: 1.25,
            }],
        })
    );
    world.advance_entity_client_animations(1);

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
        .into_iter()
        .find(|instance| instance.entity_id == 94)
        .unwrap();
    assert!(instance.render_state.minecart_new_render);
    assert!((instance.position[0] - 1.125).abs() < 1.0e-6);
    assert!((instance.position[1] - 64.03333).abs() < 1.0e-5);
    assert!((instance.position[2] + 2.125).abs() < 1.0e-6);
    assert!((instance.render_state.body_rot - 15.0).abs() < 1.0e-6);
    assert!((instance.render_state.head_pitch - 0.8333333).abs() < 1.0e-6);
}

#[test]
fn entity_model_instance_projects_old_minecart_rail_render_points_from_source() {
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 96,
        "entity_type_id": VANILLA_ENTITY_TYPE_MINECART_ID,
        "position": { "x": 2.5, "y": 1.0, "z": 3.5 },
        "y_rot": 20.0,
        "x_rot": -10.0,
        "minecart_pos_on_rail": [2.5, 1.5625, 3.5],
        "minecart_front_pos": [2.8, 1.8625, 3.5],
        "minecart_back_pos": [2.2, 1.2625, 3.5],
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();

    assert_eq!(
        instance.render_state.minecart_pos_on_rail,
        Some([2.5, 1.5625, 3.5])
    );
    assert_eq!(
        instance.render_state.minecart_front_pos,
        Some([2.8, 1.8625, 3.5])
    );
    assert_eq!(
        instance.render_state.minecart_back_pos,
        Some([2.2, 1.2625, 3.5])
    );
}

#[test]
fn entity_model_instance_projects_boat_underwater_from_source() {
    // Vanilla `AbstractBoatRenderer.extractRenderState` copies `AbstractBoat.isUnderWater()`
    // into `BoatRenderState.isUnderWater`; the world layer owns that fluid projection and
    // native must preserve it when building the renderer instance.
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 92,
        "entity_type_id": VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
        "y_rot": 0.0,
        "boat_bubble_angle": 6.0,
        "boat_underwater": true,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();

    assert_eq!(
        instance.kind,
        EntityModelKind::Boat {
            family: BoatModelFamily::Oak,
            chest: false,
        }
    );
    assert_eq!(instance.render_state.boat_bubble_angle, 6.0);
    assert!(instance.render_state.boat_underwater);
}

#[test]
fn entity_model_instances_project_aggressive_for_zombie_family() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        90,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [1.0, 64.0, -2.0],
    ));

    let aggressive = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .is_aggressive
    };

    // A calm zombie projects is_aggressive = false.
    assert!(!aggressive(&world, 90));

    // Setting Mob.isAggressive through the generated accessor/mask projects through to the held-out
    // arm render state.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 90,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_ZOMBIE_ID)],
    }));
    assert!(aggressive(&world, 90));
}

#[test]
fn entity_model_instances_project_aggressive_for_pillager_attacking_pose() {
    // Vanilla `Pillager.getArmPose`: after crossbow charge/hold checks, an aggressive pillager returns
    // `ATTACKING`. Shared `IllagerRenderer.extractRenderState` still copies `isAggressive` for the
    // evoker, although the evoker arm-pose resolver ignores that field.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        91,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        92,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [2.0, 64.0, -2.0],
    ));

    let aggressive = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .is_aggressive
    };

    assert!(!aggressive(&world, 91));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 91,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_PILLAGER_ID)],
    }));
    assert!(aggressive(&world, 91));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 92,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_EVOKER_ID)],
    }));
    assert!(aggressive(&world, 92));
}

#[test]
fn entity_model_instances_project_enderman_carrying_and_creepy() {
    let accessors = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    )
    .expect("generated EnderMan render metadata accessors");

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        94,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        [1.0, 64.0, -2.0],
    ));

    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    // A freshly spawned enderman carries nothing and is not creepy.
    let calm = state(&world, 94);
    assert!(!calm.enderman_carrying);
    assert!(!calm.enderman_creepy);

    // A present carried block (`DATA_CARRY_STATE` set) and `DATA_CREEPY` project through
    // to the held-out arm pose and the creepy head/hat shift.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 94,
        values: vec![
            EntityDataValue {
                data_id: accessors.carried_block_state_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
                value: EntityDataValueKind::OptionalBlockState(Some(10)),
            },
            protocol_bool_data(accessors.creepy_data_id, true),
        ],
    }));
    let primed = state(&world, 94);
    assert!(primed.enderman_carrying);
    assert!(primed.enderman_creepy);
}

#[test]
fn entity_model_instances_project_bat_resting_and_flying_start_phase() {
    // Vanilla Bat.DATA_ID_FLAGS (BYTE) and its named resting bit.
    let (_, default, resting_mask) =
        bbb_world::vanilla_entity_metadata_bat_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_BAT_ID,
        )
        .expect("generated Bat flags accessor must resolve");
    assert_eq!(default, 0);
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        95,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [1.0, 64.0, -2.0],
    ));

    let state = |world: &WorldStore, id: i32, partial_tick: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial_tick)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    // Before the first client tick the flying AnimationState is stopped. The first tick starts it
    // at elapsed zero, and native forwards the partial-tick sample into renderer state.
    let initial = state(&world, 95, 0.0);
    assert!(!initial.bat_resting);
    assert_eq!(initial.bat_flying_seconds, -1.0);
    world.advance_entity_client_animations(1);
    let flying = state(&world, 95, 0.5);
    assert!(!flying.bat_resting);
    assert_eq!(flying.bat_flying_seconds, 0.025);

    // Setting Bat.isResting via the named mask projects through to the hanging-pose state.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 95,
        values: vec![bat_flags_data(resting_mask)],
    }));
    assert!(state(&world, 95, 0.0).bat_resting);
    world.advance_entity_client_animations(1);
    let resting = state(&world, 95, 0.0);
    assert!(resting.bat_resting);
    assert_eq!(resting.bat_flying_seconds, -1.0);

    // Taking off restarts the timer on the next bat tick instead of resuming the old absolute-age
    // phase.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 95,
        values: vec![bat_flags_data(default)],
    }));
    world.advance_entity_client_animations(1);
    let restarted = state(&world, 95, 0.0);
    assert!(!restarted.bat_resting);
    assert_eq!(restarted.bat_flying_seconds, 0.0);
}

#[test]
fn entity_model_instances_project_vex_charging() {
    // Vanilla Vex.DATA_FLAGS_ID (BYTE) and its named FLAG_IS_CHARGING bit.
    let (_, default, charging_mask) =
        bbb_world::vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VEX_ID,
        )
        .expect("generated Vex flags accessor must resolve");
    assert_eq!(default, 0);
    const PLAIN_ITEM_ID: i32 = 702;

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_VEX_ID,
        [1.0, 64.0, -2.0],
    ));
    // A bat reuses data id 16 for its OWN resting flag — used below to prove the vex
    // charging projection is gated to the vex and never leaks onto another type.
    world.apply_add_entity(protocol_add_entity(
        98,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [2.0, 64.0, -2.0],
    ));

    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };
    let equip =
        |entity_id: i32, slot: EquipmentSlot, item_id: Option<i32>, count: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot,
                item: ItemStackSummary {
                    item_id,
                    count,
                    component_patch: DataComponentPatchSummary::default(),
                },
            }],
        };

    // An idle vex projects vex_charging = false.
    let idle = state(&world, 97);
    assert!(!idle.vex_charging);
    assert!(!idle.vex_right_hand_item_non_empty);
    assert!(!idle.vex_left_hand_item_non_empty);

    // Setting Vex.isCharging through the named mask projects through to the charging pose.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![vex_flags_data(charging_mask)],
    }));
    assert!(state(&world, 97).vex_charging);

    // Vanilla `ArmedEntityRenderState` checks RIGHT/LEFT hand item-state emptiness. bbb's current
    // Vex projection maps default RIGHT main hand to RIGHT and offhand to LEFT.
    assert!(world.apply_set_equipment(equip(97, EquipmentSlot::MainHand, Some(PLAIN_ITEM_ID), 1)));
    let main_hand_item = state(&world, 97);
    assert!(main_hand_item.vex_right_hand_item_non_empty);
    assert!(!main_hand_item.vex_left_hand_item_non_empty);

    assert!(world.apply_set_equipment(equip(97, EquipmentSlot::OffHand, Some(PLAIN_ITEM_ID), 1)));
    let both_hands = state(&world, 97);
    assert!(both_hands.vex_right_hand_item_non_empty);
    assert!(both_hands.vex_left_hand_item_non_empty);

    assert!(world.apply_set_equipment(equip(97, EquipmentSlot::MainHand, None, 0)));
    let offhand_only = state(&world, 97);
    assert!(!offhand_only.vex_right_hand_item_non_empty);
    assert!(offhand_only.vex_left_hand_item_non_empty);

    // The same flag byte set on a non-vex (bat) does NOT project vex_charging — the
    // derivation is gated by the generated accessor's Vex membership. The same held items also do
    // not project Vex hand state.
    assert!(world.apply_set_equipment(equip(98, EquipmentSlot::MainHand, Some(PLAIN_ITEM_ID), 1)));
    assert!(world.apply_set_equipment(equip(98, EquipmentSlot::OffHand, Some(PLAIN_ITEM_ID), 1)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 98,
        values: vec![vex_flags_collision_data(charging_mask)],
    }));
    let bat = state(&world, 98);
    assert!(!bat.vex_charging);
    assert!(!bat.vex_right_hand_item_non_empty);
    assert!(!bat.vex_left_hand_item_non_empty);
}

#[test]
fn entity_model_instances_project_turtle_has_egg() {
    // Vanilla Turtle.HAS_EGG (AgeableMob 16/17 then Turtle's BOOLEAN data id 18) and
    // TurtleRenderer.extractRenderState: state.hasEgg = !entity.isBaby() && entity.hasEgg().
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        140,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        [1.0, 64.0, -2.0],
    ));
    // A second turtle (made a baby below), plus a non-turtle (bat) that reuses data id 18 for
    // its own flag — used to prove the egg projection is gated to adult turtles.
    world.apply_add_entity(protocol_add_entity(
        141,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        142,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [3.0, 64.0, -2.0],
    ));

    let has_egg = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .turtle_has_egg
    };

    // An adult turtle without the flag projects turtle_has_egg = false.
    assert!(!has_egg(&world, 140));

    // Setting Turtle.HAS_EGG (data id 18) on the adult projects the egg belly.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 140,
        values: vec![turtle_has_egg_data(true)],
    }));
    assert!(has_egg(&world, 140));

    // A baby turtle with HAS_EGG set stays false (gated on !isBaby()).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 141,
        values: vec![
            baby_data(VANILLA_ENTITY_TYPE_TURTLE_ID, true),
            turtle_has_egg_data(true),
        ],
    }));
    assert!(!has_egg(&world, 141));

    // The same flag on a non-turtle (bat) does NOT project turtle_has_egg.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 142,
        values: vec![turtle_has_egg_collision_data(true)],
    }));
    assert!(!has_egg(&world, 142));
}

#[test]
fn entity_model_instances_project_pillager_charging_crossbow() {
    // Vanilla Pillager.IS_CHARGING_CROSSBOW and Pillager.getArmPose: a charging pillager renders
    // CROSSBOW_CHARGE, suppressing CROSSBOW_HOLD. SpellcasterIllager owns a different accessor at
    // the reused wire slot, so generated Pillager membership must gate the projection.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        160,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        161,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [2.0, 64.0, -2.0],
    ));
    let (charging_data_id, charging_default) =
        bbb_world::vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
        )
        .expect("generated Pillager charging accessor");
    assert!(!charging_default);

    let charging = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .is_charging_crossbow
    };

    // A pillager without the flag is not charging.
    assert!(!charging(&world, 160));

    // Setting the generated IS_CHARGING_CROSSBOW accessor projects the charge state.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![protocol_bool_data(charging_data_id, true)],
    }));
    assert!(charging(&world, 160));

    // The same wire slot on an evoker does NOT project charging.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 161,
        values: vec![protocol_bool_data(charging_data_id, true)],
    }));
    assert!(!charging(&world, 161));
}

#[test]
fn entity_model_instances_project_pillager_crossbow_hold_from_either_hand() {
    // Vanilla `Pillager.getArmPose` uses `isHolding(Items.CROSSBOW)`, and `LivingEntity.isHolding`
    // checks both hands before the aggressive ATTACKING fallback.
    const CROSSBOW_ID: i32 = 0;

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": ["minecraft:crossbow"],
        "protocol_ids": { "minecraft:crossbow": CROSSBOW_ID }
    }))
    .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_pack::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(CROSSBOW_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    };
    let holds_crossbow = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .pillager_holds_crossbow
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        162,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        163,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        164,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [3.0, 64.0, -2.0],
    ));

    assert!(!holds_crossbow(&world, 162));
    assert!(world.apply_set_equipment(equip(162, EquipmentSlot::MainHand)));
    assert!(holds_crossbow(&world, 162));

    assert!(world.apply_set_equipment(equip(163, EquipmentSlot::OffHand)));
    assert!(holds_crossbow(&world, 163));

    assert!(world.apply_set_equipment(equip(164, EquipmentSlot::OffHand)));
    assert!(!holds_crossbow(&world, 164));
}

#[test]
fn illager_celebrating_matches_generated_accessor_and_arm_pose_catalogs() {
    // Raider.IS_CELEBRATING supplies the instance flag; the nearest getArmPose implementation
    // decides whether that flag can reach CELEBRATING. Compare their intersection over every type.
    for entry in VANILLA_ENTITY_TYPES {
        let accessor =
            bbb_world::vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type(
                entry.id,
            );
        let values = accessor
            .map(|(data_id, default)| {
                assert!(!default, "{} celebrating default", entry.resource_id);
                vec![protocol_bool_data(data_id, true)]
            })
            .unwrap_or_default();
        let expected = accessor.is_some()
            && bbb_world::vanilla_entity_type_illager_arm_pose_uses_celebrating(entry.id);
        assert_eq!(
            illager_celebrating(entry.id, &values),
            expected,
            "{}",
            entry.resource_id
        );
    }
    assert!(!illager_celebrating(-1, &[]));
}

#[test]
fn entity_model_instances_project_illager_main_hand_empty() {
    // Vanilla `IllagerModel.setupAnim` ATTACKING chooses empty-hand zombie arms vs armed weapon
    // swing from `state.getMainHandItemState().isEmpty()`. Native projects that from canonical
    // equipment so renderer can choose the right branch.
    const PLAIN_ITEM_ID: i32 = 701;

    let main_hand = |entity_id: i32, item_id: Option<i32>, count: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id,
                count,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    };
    let projected = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .illager_main_hand_empty
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        173,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        174,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        175,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [3.0, 64.0, -2.0],
    ));

    assert!(projected(&world, 173));
    assert!(projected(&world, 174));
    assert!(!projected(&world, 175));

    assert!(world.apply_set_equipment(main_hand(173, Some(PLAIN_ITEM_ID), 1)));
    assert!(!projected(&world, 173));

    assert!(world.apply_set_equipment(main_hand(173, None, 0)));
    assert!(projected(&world, 173));
}

#[test]
fn entity_model_instances_project_piglin_dancing() {
    // Vanilla Piglin.isDancing() (BOOLEAN data id 19) and Piglin.getArmPose → DANCING: a regular
    // piglin dances by a soul campfire. The brute and zombified piglin never return DANCING, so the
    // projection is gated to the regular piglin type.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        180,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        181,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        [2.0, 64.0, -3.0],
    ));

    let dancing = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .piglin_dancing
    };

    // No flag → not dancing.
    assert!(!dancing(&world, 180));

    // Piglin.DATA_IS_DANCING projects the dance for the regular piglin.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 180,
        values: vec![protocol_bool_data(piglin_dancing_data_id(), true)],
    }));
    assert!(dancing(&world, 180));

    // The same encoded value on a piglin brute does NOT project dancing because the generated
    // accessor is not defined on it.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 181,
        values: vec![protocol_bool_data(piglin_dancing_data_id(), true)],
    }));
    assert!(!dancing(&world, 181));
}

#[test]
fn piglin_is_charging_crossbow_is_gated_to_the_regular_piglin() {
    // Vanilla Piglin.DATA_IS_CHARGING_CROSSBOW (BOOLEAN): getArmPose returns CROSSBOW_CHARGE
    // (the pull-back draw) while true, suppressing CROSSBOW_HOLD. Only the regular piglin defines the
    // accessor, so the projection is type-gated.
    let (charging_data_id, charging_default) =
        bbb_world::vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin charging accessor");
    assert!(!charging_default);
    let charging = vec![protocol_bool_data(charging_data_id, true)];
    assert!(piglin_is_charging_crossbow(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        &charging
    ));
    assert!(!piglin_is_charging_crossbow(
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        &charging
    ));
    assert!(!piglin_is_charging_crossbow(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        &[]
    ));
}

#[test]
fn entity_model_instances_project_piglin_crossbow_charge() {
    // Vanilla Piglin.getArmPose → CROSSBOW_CHARGE while isChargingCrossbow(): the
    // regular piglin draws its crossbow. Unlike CROSSBOW_HOLD this needs no held-item resolution (the
    // flag alone drives it), so it projects without an item runtime. Gated to the regular piglin: the
    // brute never charges because the generated accessor is not defined on it.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        190,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [1.0, 64.0, -4.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        191,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        [2.0, 64.0, -4.0],
    ));
    let (charging_data_id, charging_default) =
        bbb_world::vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin charging accessor");
    assert!(!charging_default);

    let charging = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .piglin_crossbow_charge
    };

    // No flag → not drawing.
    assert!(!charging(&world, 190));

    // Piglin.DATA_IS_CHARGING_CROSSBOW (id 18) projects the draw for the regular piglin — no item
    // runtime needed.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 190,
        values: vec![protocol_bool_data(charging_data_id, true)],
    }));
    assert!(charging(&world, 190));

    // The same data id on a piglin brute does NOT project a draw.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 191,
        values: vec![protocol_bool_data(charging_data_id, true)],
    }));
    assert!(!charging(&world, 191));
}

#[test]
fn entity_model_instances_piglin_crossbow_hold_needs_a_resolved_held_item() {
    // The CROSSBOW_HOLD pose needs the held item resolved through the item registry to confirm a
    // charged crossbow; without an item runtime it can never level, so the projection defaults off.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        182,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [3.0, 64.0, -3.0],
    ));
    let crossbow_hold = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 182)
        .unwrap()
        .render_state
        .piglin_crossbow_hold;
    assert!(!crossbow_hold);
}

#[test]
fn entity_model_instances_drowned_throw_trident_needs_a_resolved_held_item() {
    // The THROW_TRIDENT pose needs the held item resolved through the item registry to confirm a
    // trident; without an item runtime it can never raise the trident, so the projection defaults off
    // even for an aggressive drowned.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        230,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
        [3.0, 64.0, -8.0],
    ));
    // Aggressive (so only the missing item runtime, not the flag, gates the pose off).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 230,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_DROWNED_ID)],
    }));
    let throwing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 230)
        .unwrap()
        .render_state
        .drowned_throw_trident;
    assert!(!throwing);
}

#[test]
fn entity_model_instance_projects_drowned_swim_amount_from_source() {
    // Vanilla `HumanoidMobRenderer.extractHumanoidRenderState` copies
    // `LivingEntity.getSwimAmount(partialTicks)` into `state.swimAmount`; `DrownedRenderer`
    // additionally needs `boundingBoxHeight` for its swim rotation pivot.

    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 231,
        "entity_type_id": VANILLA_ENTITY_TYPE_DROWNED_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
        "y_rot": 0.0,
        "swim_amount": 0.27,
        "bounding_box_height": 1.95,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();

    assert_eq!(
        instance.kind,
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false,
        }
    );
    assert!((instance.render_state.swim_amount - 0.27).abs() < 1.0e-6);
    assert!((instance.render_state.bounding_box_height - 1.95).abs() < 1.0e-6);
}

#[test]
fn entity_model_instance_projects_elytra_animation_state_from_source() {
    // Vanilla `HumanoidMobRenderer.extractHumanoidRenderState` copies
    // `LivingEntity.elytraAnimationState.getRotX/Y/Z(partialTicks)` into the
    // render state. The world layer owns those timers; native must preserve them
    // when building the renderer instance for `WingsLayer`.
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 232,
        "entity_type_id": VANILLA_ENTITY_TYPE_PLAYER_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
        "y_rot": 0.0,
        "elytra_rot_x": 0.42,
        "elytra_rot_y": 0.08,
        "elytra_rot_z": -0.64,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();

    assert!((instance.render_state.elytra_rot_x - 0.42).abs() < 1.0e-6);
    assert!((instance.render_state.elytra_rot_y - 0.08).abs() < 1.0e-6);
    assert!((instance.render_state.elytra_rot_z + 0.64).abs() < 1.0e-6);
}

#[test]
fn entity_model_instance_projects_player_flight_swim_state_without_rederiving_it() {
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 233,
        "entity_type_id": VANILLA_ENTITY_TYPE_PLAYER_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
        "y_rot": 0.0,
        "swim_amount": 0.61,
        "humanoid_speed_value": 19.5,
        "is_fall_flying": true,
        "is_visually_swimming": false,
        "player_fall_flying_time_in_ticks": 7.25,
        "player_should_apply_flying_y_rot": true,
        "player_flying_y_rot": -0.73,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();
    let state = instance.render_state;
    assert!((state.swim_amount - 0.61).abs() < 1.0e-6);
    assert!((state.humanoid_speed_value - 19.5).abs() < 1.0e-6);
    assert!(state.is_fall_flying);
    assert!(!state.is_visually_swimming);
    assert!((state.player_fall_flying_time_in_ticks - 7.25).abs() < 1.0e-6);
    assert!(state.player_should_apply_flying_y_rot);
    assert!((state.player_flying_y_rot + 0.73).abs() < 1.0e-6);
}

#[test]
fn entity_model_instances_project_witch_item_and_potion_state() {
    // `WitchRenderState.isHoldingItem` only needs a non-empty main hand, so it projects without the
    // item runtime. `isHoldingPotion` needs registry resolution (`Items.POTION`), so it stays false
    // without the runtime.
    const POTION_ID: i32 = 0;

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        244,
        VANILLA_ENTITY_TYPE_WITCH_ID,
        [3.0, 64.0, -4.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 244,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(POTION_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 244)
        .unwrap()
        .render_state;
    assert!(state.witch_holding_item);
    assert!(!state.witch_holding_potion);

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": ["minecraft:potion"],
        "protocol_ids": { "minecraft:potion": POTION_ID }
    }))
    .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_pack::EquipmentAssetCatalog::default(),
    );
    let state = entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 244)
        .unwrap()
        .render_state;
    assert!(state.witch_holding_item);
    assert!(state.witch_holding_potion);
}

#[test]
fn entity_model_instances_project_copper_golem_holding_item_from_either_hand() {
    // `CopperGolemModel.setupAnim` checks both rendered hand item states before clamping the arms into
    // the held-item pose, so an off-hand-only item is enough.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        245,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 245,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::OffHand,
            item: ItemStackSummary {
                item_id: Some(6),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 245)
        .unwrap()
        .render_state;
    assert!(state.copper_golem_holding_item);
}

#[test]
fn entity_model_instances_project_copper_golem_idle_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        247,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, -2.0],
    ));

    let idle_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 247)
            .unwrap()
            .render_state
            .copper_golem_idle_seconds
    };

    assert_eq!(idle_seconds(&world), -1.0);
    world.advance_entity_client_animations(240);
    let after_timeout = idle_seconds(&world);
    assert!(
        after_timeout >= 0.0,
        "native projection carries the world copper golem idle timer: {after_timeout}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 247,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_GETTING_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&world), -1.0);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 247,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_project_copper_golem_get_item_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        248,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, -1.0],
    ));

    let get_item_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 248)
            .unwrap()
            .render_state
            .copper_golem_get_item_seconds
    };

    assert_eq!(get_item_seconds(&world), -1.0);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 248,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_GETTING_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    let after_one = get_item_seconds(&world);
    assert!(
        after_one >= 0.0,
        "native projection carries the copper golem GETTING_ITEM interaction timer: {after_one}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 248,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(get_item_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_project_copper_golem_get_no_item_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        249,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, 0.0],
    ));

    let get_no_item_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 249)
            .unwrap()
            .render_state
            .copper_golem_get_no_item_seconds
    };

    assert_eq!(get_no_item_seconds(&world), -1.0);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 249,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    let after_one = get_no_item_seconds(&world);
    assert!(
        after_one >= 0.0,
        "native projection carries the copper golem GETTING_NO_ITEM interaction timer: {after_one}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 249,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(get_no_item_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_project_copper_golem_drop_item_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        250,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, 1.0],
    ));

    let drop_item_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 250)
            .unwrap()
            .render_state
            .copper_golem_drop_item_seconds
    };

    assert_eq!(drop_item_seconds(&world), -1.0);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 250,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_DROPPING_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    let after_one = drop_item_seconds(&world);
    assert!(
        after_one >= 0.0,
        "native projection carries the copper golem DROPPING_ITEM interaction timer: {after_one}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 250,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(drop_item_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_project_copper_golem_drop_no_item_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        251,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, 2.0],
    ));

    let drop_no_item_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 251)
            .unwrap()
            .render_state
            .copper_golem_drop_no_item_seconds
    };

    assert_eq!(drop_no_item_seconds(&world), -1.0);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 251,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    let after_one = drop_no_item_seconds(&world);
    assert!(
            after_one >= 0.0,
            "native projection carries the copper golem DROPPING_NO_ITEM interaction timer: {after_one}"
        );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 251,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(drop_no_item_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_custom_head_skull_needs_item_runtime() {
    // `LivingEntityRenderer.extractRenderState` needs the HEAD stack resolved as an AbstractSkullBlock.
    // Without the item registry runtime, bbb must not guess from the protocol id.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        246,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 246,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary {
                item_id: Some(7),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 246)
        .unwrap()
        .render_state;
    assert_eq!(state.custom_head_skull, None);
}

#[test]
fn entity_model_instances_stab_swing_needs_a_resolved_spear() {
    // The STAB swing type needs the held item resolved through the item registry to confirm a spear
    // (the STAB default lives on the item prototype, not the network patch); without an item runtime
    // it can never resolve, so the projection defaults off and the entity keeps the WHACK swing.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        240,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    let stab = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 240)
        .unwrap()
        .render_state
        .main_hand_swing_is_stab;
    assert!(!stab);
}

#[test]
fn entity_model_instances_project_stab_from_attack_arm_item() {
    // Vanilla `ArmedEntityRenderState.extractArmedEntityRenderState` reads
    // `entity.getItemHeldByArm(state.attackArm).getSwingAnimation().type()`: an
    // off-hand spear swing is STAB even when the main hand is a plain item.
    const WOODEN_SPEAR_ID: i32 = 0;
    const PLAIN_ITEM_ID: i32 = 1;

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:wooden_spear",
            "minecraft:stick"
        ],
        "protocol_ids": {
            "minecraft:wooden_spear": WOODEN_SPEAR_ID,
            "minecraft:stick": PLAIN_ITEM_ID
        },
        "default_kinetic_weapons": vanilla_kinetic_weapon_fixture(&[
            "minecraft:wooden_spear"
        ])
    }))
    .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_pack::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    };
    let state = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 240)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        240,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(240, EquipmentSlot::MainHand, PLAIN_ITEM_ID)));
    assert!(world.apply_set_equipment(equip(240, EquipmentSlot::OffHand, WOODEN_SPEAR_ID)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 240, action: 3 }));

    let off_hand_attack = state(&world);
    assert!(off_hand_attack.attack_arm_off_hand);
    assert!(off_hand_attack.main_hand_swing_is_stab);

    assert!(world.apply_entity_animation(EntityAnimation { id: 240, action: 0 }));
    let main_hand_attack = state(&world);
    assert!(!main_hand_attack.attack_arm_off_hand);
    assert!(
        !main_hand_attack.main_hand_swing_is_stab,
        "the main-hand plain item keeps WHACK even while the off hand holds a spear"
    );

    world.apply_add_entity(protocol_add_entity(
        241,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [2.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(241, EquipmentSlot::MainHand, WOODEN_SPEAR_ID,)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 241, action: 0 }));
    let zombie_attack =
        entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 241)
            .unwrap()
            .render_state;
    assert!(
        zombie_attack.main_hand_swing_is_stab,
        "ArmedEntityRenderState extracts attack-arm STAB for non-player humanoids too"
    );
}

#[test]
fn entity_model_instances_project_stab_from_stack_swing_animation_patch() {
    // Vanilla `ItemStack.getSwingAnimation()` reads the stack component before
    // the item prototype. A patch-granted STAB therefore drives the attack-arm
    // STAB render state even on a non-spear item and without an item runtime.
    const PLAIN_ITEM_ID: i32 = 31;

    let mut item = ItemStackSummary {
        item_id: Some(PLAIN_ITEM_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    item.component_patch.added = 1;
    item.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    item.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::Stab,
        duration: 17,
    });

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        242,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 242,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item,
        }],
    }));
    assert!(world.apply_entity_animation(EntityAnimation { id: 242, action: 0 }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 242)
        .unwrap()
        .render_state;
    assert!(state.main_hand_swing_is_stab);
    assert!(
            state.player_main_hand_spear_pose,
            "AvatarRenderer returns ArmPose.SPEAR for a swinging STAB stack even when the item is not tagged spear"
        );
}

#[test]
fn entity_model_instances_stack_swing_animation_overrides_default_spear_stab() {
    // Removing or overriding `SWING_ANIMATION` on a spear stack changes
    // `getSwingAnimation().type()` away from the item prototype's STAB default.
    const WOODEN_SPEAR_ID: i32 = 0;

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:wooden_spear"
        ],
        "protocol_ids": {
            "minecraft:wooden_spear": WOODEN_SPEAR_ID
        }
    }))
    .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_pack::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, item: ItemStackSummary| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item,
        }],
    };
    let state = |world: &WorldStore, entity_id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == entity_id)
            .unwrap()
            .render_state
    };

    let mut removed = ItemStackSummary {
        item_id: Some(WOODEN_SPEAR_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    removed.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];

    let mut whack = ItemStackSummary {
        item_id: Some(WOODEN_SPEAR_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    whack.component_patch.added = 1;
    whack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    whack.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::Whack,
        duration: 6,
    });

    let mut none = ItemStackSummary {
        item_id: Some(WOODEN_SPEAR_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    none.component_patch.added = 1;
    none.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    none.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::None,
        duration: 6,
    });

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        243,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(243, removed)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 243, action: 0 }));
    assert!(!state(&world, 243).main_hand_swing_is_stab);
    assert!(!state(&world, 243).main_hand_swing_is_none);

    world.apply_add_entity(protocol_add_entity(
        244,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(244, whack)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 244, action: 0 }));
    assert!(!state(&world, 244).main_hand_swing_is_stab);
    assert!(!state(&world, 244).main_hand_swing_is_none);

    world.apply_add_entity(protocol_add_entity(
        245,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(245, none)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 245, action: 0 }));
    assert!(!state(&world, 245).main_hand_swing_is_stab);
    assert!(state(&world, 245).main_hand_swing_is_none);
}

#[test]
fn entity_model_instances_project_player_spear_use_kinetic_state() {
    // Vanilla `AvatarRenderer.getArmPose` returns `SPEAR` while a player uses a spear, and
    // `SpearAnimations` reads the default `KineticWeapon` from the resolved item prototype. The same
    // pose has `affectsOffhandPose`, so an off-hand spear use suppresses the main-hand ITEM fallback.
    const WOODEN_SPEAR_ID: i32 = 0;
    const IRON_SPEAR_ID: i32 = 1;
    const PLAIN_ITEM_ID: i32 = 2;

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:wooden_spear",
            "minecraft:iron_spear",
            "minecraft:stick"
        ],
        "protocol_ids": {
            "minecraft:wooden_spear": WOODEN_SPEAR_ID,
            "minecraft:iron_spear": IRON_SPEAR_ID,
            "minecraft:stick": PLAIN_ITEM_ID
        },
        "default_kinetic_weapons": vanilla_kinetic_weapon_fixture(&[
            "minecraft:wooden_spear",
            "minecraft:iron_spear"
        ])
    }))
    .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_pack::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    };
    let using = |id: i32, off_hand: bool| SetEntityData {
        id,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            off_hand,
            false,
        )],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        241,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(241, EquipmentSlot::MainHand, WOODEN_SPEAR_ID)));
    assert!(world.apply_set_entity_data(using(241, false)));
    let main = state(&world, 241);
    assert_eq!(
        main.player_using_spear,
        Some(SpearKineticWeapon {
            delay_ticks: 15.0,
            dismount_duration_ticks: 100.0,
            knockback_duration_ticks: 200.0,
            damage_duration_ticks: 300.0,
            forward_movement: 0.38,
        })
    );
    assert!(!main.player_main_hand_item_pose);
    assert!(!main.player_main_hand_spear_pose);
    assert!(!main.player_off_hand_spear_pose);

    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 241,
        event_id: 2,
    }));
    world.advance_entity_client_animations(2);
    let hit_feedback =
        entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 0.5)
            .into_iter()
            .find(|instance| instance.entity_id == 241)
            .unwrap()
            .render_state;
    assert_eq!(hit_feedback.ticks_since_kinetic_hit_feedback, 2.5);

    world.apply_add_entity(protocol_add_entity(
        242,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(242, EquipmentSlot::MainHand, PLAIN_ITEM_ID)));
    assert!(world.apply_set_equipment(equip(242, EquipmentSlot::OffHand, IRON_SPEAR_ID)));
    assert!(world.apply_set_entity_data(using(242, true)));
    let off = state(&world, 242);
    assert!(off.use_item_off_hand);
    assert_eq!(
        off.player_using_spear,
        Some(SpearKineticWeapon {
            delay_ticks: 12.0,
            dismount_duration_ticks: 50.0,
            knockback_duration_ticks: 135.0,
            damage_duration_ticks: 225.0,
            forward_movement: 0.38,
        })
    );
    assert!(!off.player_main_hand_item_pose);
    assert!(!off.player_off_hand_item_pose);
    assert!(!off.player_main_hand_spear_pose);
    assert!(!off.player_off_hand_spear_pose);
}

#[test]
fn entity_model_instances_project_player_held_spear_arm_pose() {
    // Vanilla `AvatarRenderer.getArmPose` returns `SPEAR` for a held spear even when it is not being
    // used. In the normal right-handed `HumanoidModel.setupAnim` dispatch, the off-hand pose runs first;
    // an off-hand `SPEAR` has `affectsOffhandPose`, so it suppresses the main-hand ITEM pose. A charged
    // main-hand `CROSSBOW_HOLD` is the two-handed exception: it forces the off hand to ITEM first. A
    // main-hand spear does not suppress a plain off-hand ITEM because the off hand is posed first.
    const WOODEN_SPEAR_ID: i32 = 0;
    const IRON_SPEAR_ID: i32 = 1;
    const PLAIN_ITEM_ID: i32 = 2;
    const CROSSBOW_ID: i32 = 3;

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:wooden_spear",
            "minecraft:iron_spear",
            "minecraft:stick",
            "minecraft:crossbow"
        ],
        "protocol_ids": {
            "minecraft:wooden_spear": WOODEN_SPEAR_ID,
            "minecraft:iron_spear": IRON_SPEAR_ID,
            "minecraft:stick": PLAIN_ITEM_ID,
            "minecraft:crossbow": CROSSBOW_ID
        },
        "default_kinetic_weapons": vanilla_kinetic_weapon_fixture(&[
            "minecraft:wooden_spear",
            "minecraft:iron_spear"
        ])
    }))
    .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_pack::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32, charged: bool| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    charged_projectiles_items: if charged {
                        vec![bbb_protocol::packets::ItemStackTemplateSummary {
                            item_id: PLAIN_ITEM_ID,
                            count: 1,
                            component_patch: DataComponentPatchSummary::default(),
                        }]
                    } else {
                        Vec::new()
                    },
                    ..DataComponentPatchSummary::default()
                },
            },
        }],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        243,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(243, EquipmentSlot::MainHand, WOODEN_SPEAR_ID, false,)));
    assert!(world.apply_set_equipment(equip(243, EquipmentSlot::OffHand, PLAIN_ITEM_ID, false,)));
    let main_spear = state(&world, 243);
    assert!(main_spear.player_main_hand_spear_pose);
    assert!(!main_spear.player_off_hand_spear_pose);
    assert!(!main_spear.humanoid_mob_main_hand_spear_pose);
    assert!(!main_spear.humanoid_mob_off_hand_spear_pose);
    assert!(!main_spear.player_main_hand_item_pose);
    assert!(
        main_spear.player_off_hand_item_pose,
        "a main-hand spear does not suppress the plain off-hand ITEM pose"
    );

    world.apply_add_entity(protocol_add_entity(
        244,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [4.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(244, EquipmentSlot::MainHand, PLAIN_ITEM_ID, false,)));
    assert!(world.apply_set_equipment(equip(244, EquipmentSlot::OffHand, IRON_SPEAR_ID, false,)));
    let off_spear = state(&world, 244);
    assert!(!off_spear.player_main_hand_spear_pose);
    assert!(off_spear.player_off_hand_spear_pose);
    assert!(!off_spear.humanoid_mob_main_hand_spear_pose);
    assert!(!off_spear.humanoid_mob_off_hand_spear_pose);
    assert!(
        !off_spear.player_main_hand_item_pose,
        "off-hand SPEAR affectsOffhandPose skips the main-hand ITEM pose"
    );
    assert!(!off_spear.player_off_hand_item_pose);

    world.apply_add_entity(protocol_add_entity(
        245,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [5.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(245, EquipmentSlot::MainHand, CROSSBOW_ID, true,)));
    assert!(world.apply_set_equipment(equip(245, EquipmentSlot::OffHand, IRON_SPEAR_ID, false,)));
    let off_spear_over_crossbow = state(&world, 245);
    assert!(!off_spear_over_crossbow.player_off_hand_spear_pose);
    assert!(
        off_spear_over_crossbow.player_crossbow_hold,
        "main-hand CROSSBOW_HOLD isTwoHanded forces a non-empty off hand to ITEM first"
    );
    assert!(
            off_spear_over_crossbow.player_off_hand_item_pose,
            "the forced off-hand ITEM pose replaces the held SPEAR pose before CROSSBOW_HOLD overwrites it"
        );
}

#[test]
fn entity_model_instances_project_humanoid_mob_held_spear_arm_pose() {
    // Vanilla `HumanoidMobRenderer.getArmPose` returns `SPEAR` for a non-player humanoid mob holding an
    // item tagged `minecraft:spears`. `AbstractZombieRenderer` additionally checks the opposite hand's
    // STAB swing component, so a zombie-family mob with a spear in either hand marks both arm poses.
    const WOODEN_SPEAR_ID: i32 = 0;
    const IRON_SPEAR_ID: i32 = 1;
    const PLAIN_ITEM_ID: i32 = 2;

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:wooden_spear",
            "minecraft:iron_spear",
            "minecraft:stick"
        ],
        "protocol_ids": {
            "minecraft:wooden_spear": WOODEN_SPEAR_ID,
            "minecraft:iron_spear": IRON_SPEAR_ID,
            "minecraft:stick": PLAIN_ITEM_ID
        },
        "default_kinetic_weapons": vanilla_kinetic_weapon_fixture(&[
            "minecraft:wooden_spear",
            "minecraft:iron_spear"
        ])
    }))
    .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_pack::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        246,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        [6.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(246, EquipmentSlot::MainHand, WOODEN_SPEAR_ID)));
    let skeleton_main = state(&world, 246);
    assert!(skeleton_main.humanoid_mob_main_hand_spear_pose);
    assert!(!skeleton_main.humanoid_mob_off_hand_spear_pose);
    assert!(!skeleton_main.player_main_hand_spear_pose);

    world.apply_add_entity(protocol_add_entity(
        247,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        [7.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(247, EquipmentSlot::MainHand, PLAIN_ITEM_ID)));
    assert!(world.apply_set_equipment(equip(247, EquipmentSlot::OffHand, IRON_SPEAR_ID)));
    let skeleton_off = state(&world, 247);
    assert!(!skeleton_off.humanoid_mob_main_hand_spear_pose);
    assert!(skeleton_off.humanoid_mob_off_hand_spear_pose);

    world.apply_add_entity(protocol_add_entity(
        248,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [8.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(248, EquipmentSlot::MainHand, WOODEN_SPEAR_ID)));
    let zombie_main = state(&world, 248);
    assert!(zombie_main.humanoid_mob_main_hand_spear_pose);
    assert!(
        zombie_main.humanoid_mob_off_hand_spear_pose,
        "AbstractZombieRenderer checks the opposite main-hand STAB component for the off arm"
    );

    world.apply_add_entity(protocol_add_entity(
        249,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [9.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(249, EquipmentSlot::OffHand, IRON_SPEAR_ID)));
    let zombie_off = state(&world, 249);
    assert!(
        zombie_off.humanoid_mob_main_hand_spear_pose,
        "AbstractZombieRenderer checks the opposite off-hand STAB component for the main arm"
    );
    assert!(zombie_off.humanoid_mob_off_hand_spear_pose);

    world.apply_add_entity(protocol_add_entity(
        250,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [10.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(250, EquipmentSlot::MainHand, WOODEN_SPEAR_ID)));
    let piglin_main = state(&world, 250);
    assert!(piglin_main.humanoid_mob_main_hand_spear_pose);
    assert!(
            !piglin_main.humanoid_mob_off_hand_spear_pose,
            "PiglinRenderer inherits the base same-hand HumanoidMobRenderer pose, not the zombie opposite-hand override"
        );

    world.apply_add_entity(protocol_add_entity(
        251,
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        [11.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(251, EquipmentSlot::OffHand, IRON_SPEAR_ID)));
    let zombified_off = state(&world, 251);
    assert!(!zombified_off.humanoid_mob_main_hand_spear_pose);
    assert!(zombified_off.humanoid_mob_off_hand_spear_pose);
}

#[test]
fn entity_model_instances_spyglass_pose_needs_a_resolved_spyglass() {
    // The SPYGLASS use-item pose needs the using-hand item resolved through the item registry to
    // confirm a spyglass; without an item runtime it can never resolve, so the projection defaults
    // off even for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        250,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -10.0],
    ));
    // Flag the player as using an item (so only the missing runtime, not the flag, gates the pose off).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 250,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let spyglass = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 250)
        .unwrap()
        .render_state
        .player_using_spyglass;
    assert!(!spyglass);
}

#[test]
fn entity_model_instances_horn_pose_needs_a_resolved_goat_horn() {
    // The TOOT_HORN use-item pose needs the using-hand item resolved through the item registry to
    // confirm a goat horn; without an item runtime it can never resolve, so the projection defaults
    // off even for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        251,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 251,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let tooting = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 251)
        .unwrap()
        .render_state
        .player_tooting_horn;
    assert!(!tooting);
}

#[test]
fn entity_model_instances_brush_pose_needs_a_resolved_brush() {
    // The BRUSH use-item pose needs the using-hand item resolved through the item registry to confirm
    // a brush; without an item runtime it can never resolve, so the projection defaults off even for a
    // player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        252,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 252,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let brushing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 252)
        .unwrap()
        .render_state
        .player_brushing;
    assert!(!brushing);
}

#[test]
fn entity_model_instances_block_pose_uses_shield_or_blocks_attacks_component() {
    // The BLOCK use-item pose needs a non-consumable `BLOCKS_ATTACKS` item. Vanilla shields resolve from
    // the item registry, while datapack/patch-granted blockers are visible in `added_type_ids` and work
    // without an item runtime.
    const PLAIN_ITEM_ID: i32 = 730;

    let mut world = WorldStore::new();
    let equip = |entity_id: i32, added_type_ids: Vec<i32>| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(PLAIN_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids,
                    ..DataComponentPatchSummary::default()
                },
            },
        }],
    };
    let set_using = |id: i32| SetEntityData {
        id,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    };
    let blocking = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .player_blocking
    };

    world.apply_add_entity(protocol_add_entity(
        253,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [4.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(set_using(253)));
    assert!(!blocking(&world, 253));

    world.apply_add_entity(protocol_add_entity(
        254,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [5.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(
        254,
        vec![VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID]
    )));
    assert!(world.apply_set_entity_data(set_using(254)));
    assert!(blocking(&world, 254));

    // `Item.getUseAnimation` checks CONSUMABLE before BLOCKS_ATTACKS, so a stack adding both routes to
    // EAT/DRINK rather than the BLOCK pose.
    world.apply_add_entity(protocol_add_entity(
        255,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [6.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(
        255,
        vec![
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID
        ]
    )));
    assert!(world.apply_set_entity_data(set_using(255)));
    assert!(!blocking(&world, 255));
}

#[test]
fn entity_model_instances_throw_trident_pose_needs_a_resolved_trident() {
    // The THROW_TRIDENT use-item pose needs the using-hand item resolved through the item registry to
    // confirm a trident; without an item runtime it can never resolve, so the projection defaults off
    // even for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        254,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [5.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 254,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let throwing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 254)
        .unwrap()
        .render_state
        .player_throwing_trident;
    assert!(!throwing);
}

#[test]
fn entity_model_instances_bow_draw_pose_needs_a_resolved_bow() {
    // The BOW_AND_ARROW use-item pose needs the using-hand item resolved through the item registry to
    // confirm a bow; without an item runtime it can never resolve, so the projection defaults off even
    // for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        255,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [6.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 255,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let drawing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 255)
        .unwrap()
        .render_state
        .player_drawing_bow;
    assert!(!drawing);
}

#[test]
fn entity_model_instances_crossbow_charge_pose_needs_a_resolved_crossbow() {
    // The CROSSBOW_CHARGE use-item pose needs the using-hand item resolved through the item registry to
    // confirm a crossbow; without an item runtime it can never resolve, so the projection defaults off
    // even for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        256,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [7.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 256,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let charging = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 256)
        .unwrap()
        .render_state
        .player_charging_crossbow;
    assert!(!charging);
}

#[test]
fn entity_model_instances_crossbow_hold_pose_needs_a_resolved_charged_crossbow() {
    // The CROSSBOW_HOLD pose needs the held item resolved through the item registry to confirm a CHARGED
    // crossbow; without an item runtime neither the main-hand nor off-hand projection can resolve, so
    // both default off even for a (non-swinging) player.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        257,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [8.0, 64.0, -10.0],
    ));
    let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 257)
        .unwrap()
        .render_state;
    assert!(!render_state.player_crossbow_hold);
    assert!(!render_state.player_crossbow_hold_off_hand);
}

#[test]
fn entity_model_instances_project_off_hand_bow_and_crossbow_player_poses() {
    // Vanilla `AvatarRenderer.getArmPose` selects the BOW/CROSSBOW use pose from the used hand, and
    // `CROSSBOW_HOLD` before the use-item branch from either hand. The renderer owns the exact arm math;
    // this test proves native projects the correct using-hand/off-hand booleans from resolved items.
    const BOW_ID: i32 = 0;
    const CROSSBOW_ID: i32 = 1;

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:bow",
            "minecraft:crossbow"
        ],
        "protocol_ids": {
            "minecraft:bow": BOW_ID,
            "minecraft:crossbow": CROSSBOW_ID
        }
    }))
    .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_pack::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32, charged: bool| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    charged_projectiles_items: if charged {
                        vec![bbb_protocol::packets::ItemStackTemplateSummary {
                            item_id: BOW_ID,
                            count: 1,
                            component_patch: DataComponentPatchSummary::default(),
                        }]
                    } else {
                        Vec::new()
                    },
                    ..DataComponentPatchSummary::default()
                },
            },
        }],
    };
    let use_off_hand = |id: i32| SetEntityData {
        id,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            true,
            false,
        )],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();

    world.apply_add_entity(protocol_add_entity(
        258,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [9.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(258, EquipmentSlot::OffHand, BOW_ID, false)));
    assert!(world.apply_set_entity_data(use_off_hand(258)));
    let drawing_bow = state(&world, 258);
    assert!(drawing_bow.player_drawing_bow);
    assert!(drawing_bow.use_item_off_hand);
    assert!(!drawing_bow.player_off_hand_item_pose);

    world.apply_add_entity(protocol_add_entity(
        259,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [10.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(259, EquipmentSlot::OffHand, CROSSBOW_ID, false)));
    assert!(world.apply_set_entity_data(use_off_hand(259)));
    let charging_crossbow = state(&world, 259);
    assert!(charging_crossbow.player_charging_crossbow);
    assert!(charging_crossbow.use_item_off_hand);
    assert!(!charging_crossbow.player_crossbow_hold_off_hand);

    world.apply_add_entity(protocol_add_entity(
        260,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [11.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(260, EquipmentSlot::OffHand, CROSSBOW_ID, true)));
    let off_hand_hold = state(&world, 260);
    assert!(!off_hand_hold.player_crossbow_hold);
    assert!(off_hand_hold.player_crossbow_hold_off_hand);
    assert!(!off_hand_hold.player_off_hand_item_pose);

    world.apply_add_entity(protocol_add_entity(
        261,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [12.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(261, EquipmentSlot::MainHand, CROSSBOW_ID, true)));
    assert!(world.apply_set_equipment(equip(261, EquipmentSlot::OffHand, CROSSBOW_ID, true)));
    let main_hand_hold = state(&world, 261);
    assert!(main_hand_hold.player_crossbow_hold);
    assert!(!main_hand_hold.player_crossbow_hold_off_hand);

    world.apply_add_entity(protocol_add_entity(
        262,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [13.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(262, EquipmentSlot::MainHand, CROSSBOW_ID, true)));
    assert!(world.apply_set_equipment(equip(262, EquipmentSlot::OffHand, BOW_ID, false)));
    assert!(world.apply_set_entity_data(use_off_hand(262)));
    let main_hold_over_off_bow = state(&world, 262);
    assert!(main_hold_over_off_bow.player_crossbow_hold);
    assert!(
        !main_hold_over_off_bow.player_drawing_bow,
        "main-hand CROSSBOW_HOLD forces the using off-hand bow pose down to ITEM"
    );
    assert!(main_hold_over_off_bow.player_off_hand_item_pose);
}

#[test]
fn entity_model_instances_project_player_main_hand_item_pose() {
    // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM`: a player holding a plain main-hand item, not
    // using it, lowers/halves the arm. Gated to the player kind, a non-empty main hand, and not using an
    // item; an empty hand, a using-item player, or a non-player mob never reaches the `ITEM` fallback.
    // Any item id resolves the same way without a runtime — only "main hand non-empty" drives the gate.
    const PLAIN_ITEM_ID: i32 = 710;

    let plain_main_hand = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(PLAIN_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    };
    let posing = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .player_main_hand_item_pose
    };

    let mut world = WorldStore::new();
    // A player holding a plain item in the main hand reaches the ITEM fallback pose.
    world.apply_add_entity(protocol_add_entity(
        260,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, 8.0],
    ));
    assert!(world.apply_set_equipment(plain_main_hand(260)));
    assert!(posing(&world, 260));

    // The same player with an empty main hand has no item to pose.
    world.apply_add_entity(protocol_add_entity(
        261,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, 8.0],
    ));
    assert!(!posing(&world, 261));

    // A player USING a NON-special main-hand item (here a plain item — `EAT`/`DRINK` or any tool) still
    // falls through to the `ITEM` fallback (vanilla `getArmPose` only special-cases bow/crossbow/trident/
    // shield/spyglass/horn/brush; everything else -> `ITEM`). A SPECIAL using item would suppress it, but
    // that needs the item runtime to resolve, so the no-runtime case treats any using item as non-special.
    world.apply_add_entity(protocol_add_entity(
        262,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, 8.0],
    ));
    assert!(world.apply_set_equipment(plain_main_hand(262)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 262,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    assert!(posing(&world, 262));

    // A non-player mob holding an item never returns ITEM (`HumanoidMobRenderer.getArmPose`).
    world.apply_add_entity(protocol_add_entity(
        263,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [4.0, 64.0, 8.0],
    ));
    assert!(world.apply_set_equipment(plain_main_hand(263)));
    assert!(!posing(&world, 263));
}

#[test]
fn player_stuck_counts_flow_unchanged_from_world_to_renderer_state() {
    let accessors = bbb_world::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    )
    .expect("generated LivingEntity metadata accessors");
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        263,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, 8.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 263,
        values: vec![
            protocol_int_data(accessors.arrow_count_data_id, -7),
            protocol_int_data(accessors.stinger_count_data_id, 1_000),
        ],
    }));

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 263)
        .expect("player model instance");
    assert_eq!(instance.render_state.player_arrow_count, -7);
    assert_eq!(instance.render_state.player_stinger_count, 1_000);
}

#[test]
fn entity_model_instances_project_player_off_hand_item_pose() {
    // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM`: a player holding a plain off-hand
    // item lowers/halves the OFF arm. Gated to the player kind + a non-empty off hand, suppressed only
    // when USING the off hand (its use poses win); using the MAIN hand leaves the off hand on `ITEM`.
    const PLAIN_ITEM_ID: i32 = 720;

    let plain_off_hand = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::OffHand,
            item: ItemStackSummary {
                item_id: Some(PLAIN_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    };
    let posing = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .player_off_hand_item_pose
    };

    let mut world = WorldStore::new();
    // A player holding a plain off-hand item reaches the off-hand ITEM fallback pose.
    world.apply_add_entity(protocol_add_entity(
        270,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, 12.0],
    ));
    assert!(world.apply_set_equipment(plain_off_hand(270)));
    assert!(posing(&world, 270));

    // An empty off hand has no item to pose.
    world.apply_add_entity(protocol_add_entity(
        271,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, 12.0],
    ));
    assert!(!posing(&world, 271));

    // USING a NON-special off-hand item (here a plain item — `EAT`/`DRINK`) still falls through to the
    // off-hand `ITEM` fallback (only a special off-hand use item would route to its own pose, which needs
    // the runtime to resolve; the no-runtime case treats any using item as non-special).
    world.apply_add_entity(protocol_add_entity(
        272,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, 12.0],
    ));
    assert!(world.apply_set_equipment(plain_off_hand(272)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 272,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            true,
            false,
        )],
    }));
    assert!(posing(&world, 272));

    // USING the MAIN hand leaves the off hand on its ITEM fallback (the off hand is not the using hand).
    world.apply_add_entity(protocol_add_entity(
        273,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [4.0, 64.0, 12.0],
    ));
    assert!(world.apply_set_equipment(plain_off_hand(273)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 273,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    assert!(posing(&world, 273));

    // A non-player mob never returns ITEM for the off hand either.
    world.apply_add_entity(protocol_add_entity(
        274,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [5.0, 64.0, 12.0],
    ));
    assert!(world.apply_set_equipment(plain_off_hand(274)));
    assert!(!posing(&world, 274));
}

#[test]
fn entity_model_instances_project_piglin_melee_attack_pose() {
    // Vanilla Piglin/PiglinBrute.getArmPose ATTACKING_WITH_MELEE_WEAPON: aggressive (Mob.isAggressive,
    // resolved through the generated Mob accessor/mask) AND isHoldingMeleeWeapon (main-hand item with
    // DataComponents.TOOL, wire type 28). Gated to the regular piglin + brute (the zombified piglin
    // uses its renderer zombie-arm pose); the regular piglin is also suppressed while DANCING.
    // Any item id resolves the same way — only the TOOL component drives the pose, not the item type.
    const MELEE_ITEM_ID: i32 = 700;

    // A main-hand stack carrying the `minecraft:tool` data component (wire type 28).
    let tool_main_hand = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(MELEE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID],
                    ..DataComponentPatchSummary::default()
                },
            },
        }],
    };
    let set_aggressive = |id: i32, entity_type_id: i32| SetEntityData {
        id,
        values: vec![mob_aggressive_data(entity_type_id)],
    };
    let attacking = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .piglin_attacking_with_melee
    };

    let mut world = WorldStore::new();
    // Regular piglin (210), brute (211), zombified piglin (212): all aggressive, all holding a tool.
    for (id, type_id) in [
        (210, VANILLA_ENTITY_TYPE_PIGLIN_ID),
        (211, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID),
        (212, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID),
    ] {
        world.apply_add_entity(protocol_add_entity(id, type_id, [1.0, 64.0, -6.0]));
        assert!(world.apply_set_equipment(tool_main_hand(id)));
        assert!(world.apply_set_entity_data(set_aggressive(id, type_id)));
    }
    // The piglin and the brute raise/swing the melee weapon; the zombified piglin uses zombie arms.
    assert!(attacking(&world, 210));
    assert!(attacking(&world, 211));
    assert!(!attacking(&world, 212));

    // An aggressive piglin with an empty hand (no tool component) does not raise a weapon.
    world.apply_add_entity(protocol_add_entity(
        213,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [2.0, 64.0, -6.0],
    ));
    assert!(world.apply_set_entity_data(set_aggressive(213, VANILLA_ENTITY_TYPE_PIGLIN_ID,)));
    assert!(!attacking(&world, 213));

    // A non-aggressive piglin holding a tool does not attack.
    world.apply_add_entity(protocol_add_entity(
        214,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [3.0, 64.0, -6.0],
    ));
    assert!(world.apply_set_equipment(tool_main_hand(214)));
    assert!(!attacking(&world, 214));

    // A dancing piglin (higher priority) holding a tool while aggressive keeps DANCING, not the attack.
    world.apply_add_entity(protocol_add_entity(
        215,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [4.0, 64.0, -6.0],
    ));
    assert!(world.apply_set_equipment(tool_main_hand(215)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 215,
        values: vec![
            mob_aggressive_data(VANILLA_ENTITY_TYPE_PIGLIN_ID),
            protocol_bool_data(piglin_dancing_data_id(), true),
        ],
    }));
    assert!(!attacking(&world, 215));
}

#[test]
fn entity_model_instances_project_piglin_admiring_a_loved_offhand_item() {
    // Vanilla Piglin.getArmPose ADMIRING_ITEM = PiglinAi.isLovedItem(getOffhandItem()) =
    // offhand.is(ItemTags.PIGLIN_LOVED). Gated to the regular piglin (the brute has no admire branch);
    // higher priority than ATTACKING/CROSSBOW (suppresses them), below DANCING.
    const LOVED_ITEM_ID: i32 = 800; // stand-in for a `minecraft:piglin_loved` item (e.g. gold_ingot).
    const PLAIN_ITEM_ID: i32 = 801; // not in the tag.
    const TOOL_ITEM_ID: i32 = 802; // a melee weapon (TOOL component) for the suppression check.

    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32, tool: bool| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: if tool {
                        vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID]
                    } else {
                        Vec::new()
                    },
                    ..DataComponentPatchSummary::default()
                },
            },
        }],
    };
    let admiring = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .piglin_admiring
    };

    let mut world = WorldStore::new();
    // The `minecraft:piglin_loved` item tag arrives via UpdateTags (gold_ingot etc.).
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: PIGLIN_LOVED_ITEM_TAG.to_string(),
                entries: vec![LOVED_ITEM_ID, TOOL_ITEM_ID],
            }],
        }],
    });

    // A regular piglin with a loved item in its OFFHAND admires it.
    world.apply_add_entity(protocol_add_entity(
        220,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [1.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(220, EquipmentSlot::OffHand, LOVED_ITEM_ID, false)));
    assert!(admiring(&world, 220));

    // A non-loved offhand item → no admiring.
    world.apply_add_entity(protocol_add_entity(
        221,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [2.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(221, EquipmentSlot::OffHand, PLAIN_ITEM_ID, false)));
    assert!(!admiring(&world, 221));

    // The piglin brute has no ADMIRING_ITEM branch, even with a loved offhand item.
    world.apply_add_entity(protocol_add_entity(
        222,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        [3.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(222, EquipmentSlot::OffHand, LOVED_ITEM_ID, false)));
    assert!(!admiring(&world, 222));

    // A dancing piglin (higher priority) does not admire.
    world.apply_add_entity(protocol_add_entity(
        223,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [4.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(223, EquipmentSlot::OffHand, LOVED_ITEM_ID, false)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 223,
        values: vec![protocol_bool_data(piglin_dancing_data_id(), true)],
    }));
    assert!(!admiring(&world, 223));

    // ADMIRING suppresses ATTACKING: an aggressive piglin with a tool main hand AND a loved offhand
    // admires (vanilla precedence ADMIRING > ATTACKING), so it does not swing.
    world.apply_add_entity(protocol_add_entity(
        224,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [5.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(224, EquipmentSlot::MainHand, TOOL_ITEM_ID, true)));
    assert!(world.apply_set_equipment(equip(224, EquipmentSlot::OffHand, LOVED_ITEM_ID, false)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 224,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_PIGLIN_ID)],
    }));
    let attacking_224 = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 224)
        .unwrap()
        .render_state
        .piglin_attacking_with_melee;
    assert!(
        admiring(&world, 224),
        "the loved offhand item makes it admire"
    );
    assert!(
        !attacking_224,
        "admiring (higher priority) suppresses the melee swing"
    );
}

#[test]
fn entity_model_instances_project_panda_unhappy_and_sneezing() {
    // Vanilla PandaRenderState projects unhappy/eat/sneeze counters and named flags from the Panda
    // metadata family; worried-gene pandas additionally become scared in a thundering level.
    let accessors = panda_metadata_accessors();
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(1));
    insert_loaded_entity_chunk(&mut world, ChunkPos { x: 0, z: -1 });
    world.apply_add_entity(protocol_add_entity(
        190,
        VANILLA_ENTITY_TYPE_PANDA_ID,
        [1.0, 64.0, -4.0],
    ));

    let panda = |world: &WorldStore, partial_tick: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial_tick)
            .into_iter()
            .find(|instance| instance.entity_id == 190)
            .unwrap()
            .render_state
    };

    // No data → content panda.
    let rest = panda(&world, 0.0);
    assert!(!rest.panda_unhappy);
    assert!(!rest.panda_sneezing);
    assert_eq!(rest.panda_sneeze_time, 0);
    assert!(!rest.panda_eating);
    assert!(!rest.panda_sitting);
    assert!(!rest.panda_scared);
    assert_eq!(rest.panda_sit_amount, 0.0);
    assert_eq!(rest.panda_lie_on_back_amount, 0.0);
    assert_eq!(rest.panda_roll_amount, 0.0);
    assert_eq!(rest.panda_roll_time, 0.0);

    // UNHAPPY_COUNTER > 0 projects the unhappy shake; the sneeze flag + counter project the dip; the
    // sitting bit + EAT_COUNTER project the held-item layer state. The same flags byte also feeds
    // the world-side sit/on-back/roll animation amounts that native copies into render state.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 190,
        values: vec![
            protocol_int_data(accessors.unhappy_counter_data_id, 12),
            protocol_int_data(accessors.sneeze_counter_data_id, 9),
            protocol_int_data(accessors.eat_counter_data_id, 4),
            protocol_byte_data(
                accessors.flags_data_id,
                accessors.sneezing_mask
                    | accessors.rolling_mask
                    | accessors.sitting_mask
                    | accessors.on_back_mask,
            ),
        ],
    }));
    world.advance_entity_client_animations(1);
    let active = panda(&world, 0.5);
    assert!(active.panda_unhappy);
    assert!(active.panda_sneezing);
    assert_eq!(active.panda_sneeze_time, 9);
    assert!(active.panda_eating);
    assert!(active.panda_sitting);
    assert!(!active.panda_scared);
    assert!((active.panda_sit_amount - 0.075).abs() < 1.0e-6);
    assert!((active.panda_lie_on_back_amount - 0.075).abs() < 1.0e-6);
    assert!((active.panda_roll_amount - 0.075).abs() < 1.0e-6);
    assert!((active.panda_roll_time - 1.5).abs() < 1.0e-6);

    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 2,
        param: 0.0,
    });
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 8,
        param: 0.9,
    });
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 190,
        values: vec![
            protocol_byte_data(accessors.main_gene_data_id, 2),
            protocol_byte_data(accessors.hidden_gene_data_id, 0),
        ],
    }));
    assert!(!panda(&world, 0.0).panda_scared);
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 8,
        param: 1.0,
    });
    assert!(panda(&world, 0.0).panda_scared);

    // A zero unhappy counter is content again; clearing the flag stops the sneeze even with a counter.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 190,
        values: vec![
            protocol_int_data(accessors.unhappy_counter_data_id, 0),
            protocol_int_data(accessors.eat_counter_data_id, 0),
            protocol_byte_data(accessors.flags_data_id, 0),
        ],
    }));
    let calmed = panda(&world, 0.0);
    assert!(!calmed.panda_unhappy);
    assert!(!calmed.panda_sneezing);
    assert!(!calmed.panda_eating);
    assert!(!calmed.panda_sitting);
    assert!(calmed.panda_scared);
}

#[test]
fn entity_model_instances_project_villager_unhappy() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        191,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        [1.0, 64.0, -4.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        192,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        [2.0, 64.0, -4.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        193,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        [3.0, 64.0, -4.0],
    ));

    let villager_unhappy = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .villager_unhappy
    };

    assert!(!villager_unhappy(&world, 191));
    assert!(!villager_unhappy(&world, 192));

    for (id, entity_type_id) in [
        (191, VANILLA_ENTITY_TYPE_VILLAGER_ID),
        (192, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID),
    ] {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![abstract_villager_unhappy_data(entity_type_id, 12)],
        }));
    }
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 193,
        values: vec![abstract_villager_unhappy_collision_data(12)],
    }));
    assert!(villager_unhappy(&world, 191));
    assert!(villager_unhappy(&world, 192));
    assert!(
        !villager_unhappy(&world, 193),
        "zombie villagers do not use VillagerRenderState"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 191,
        values: vec![abstract_villager_unhappy_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            0,
        )],
    }));
    assert!(!villager_unhappy(&world, 191));
}

#[test]
fn entity_model_instances_project_goat_ramming_head_tilt() {
    use std::f32::consts::PI;

    // Vanilla Goat.getRammingXHeadRot() = lowerHeadTick/20 · (baby ? 52.5 : 30)° · π/180, driven by
    // the world-projected ram counter (entity events 58/59). The baby max head pitch is steeper.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        200,
        VANILLA_ENTITY_TYPE_GOAT_ID,
        [1.0, 64.0, -5.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        201,
        VANILLA_ENTITY_TYPE_GOAT_ID,
        [2.0, 64.0, -5.0],
    ));
    // Make 201 a baby goat through the generated inherited accessor.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 201,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_GOAT_ID, true)],
    }));

    let ramming = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .goat_ramming_x_head_rot
    };

    // At rest both goats project no head tilt.
    assert_eq!(ramming(&world, 200), 0.0);
    assert_eq!(ramming(&world, 201), 0.0);

    // Event 58 begins the ram; after 20 ticks the counter saturates at 20 (full tilt).
    for id in [200, 201] {
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: id,
            event_id: 58,
        }));
    }
    world.advance_entity_client_animations(20);
    // Adult: 30° → π/6; baby: 52.5°.
    assert!((ramming(&world, 200) - PI / 6.0).abs() < 1.0e-5);
    assert!((ramming(&world, 201) - 52.5_f32.to_radians()).abs() < 1.0e-5);
    assert!(
        ramming(&world, 201) > ramming(&world, 200),
        "the baby tilts its head further"
    );
}

#[test]
fn entity_model_instances_project_turtle_laying_egg() {
    // Vanilla Turtle.LAYING_EGG (BOOLEAN data id 19) and TurtleRenderer.extractRenderState:
    // state.isLayingEgg = entity.isLayingEgg(). Unlike hasEgg, this is NOT baby-gated (the
    // egg-laying amplitude lives in the shared TurtleModel).
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        150,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        [1.0, 64.0, -2.0],
    ));
    // A baby turtle (lays too), plus a non-turtle (bat) used to prove the type gating.
    world.apply_add_entity(protocol_add_entity(
        151,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        152,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [3.0, 64.0, -2.0],
    ));

    let laying = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .turtle_laying_egg
    };

    // A turtle that is not laying projects turtle_laying_egg = false.
    assert!(!laying(&world, 150));

    // Setting Turtle.LAYING_EGG (data id 19) projects through.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 150,
        values: vec![turtle_laying_egg_data(true)],
    }));
    assert!(laying(&world, 150));

    // A baby turtle DOES lay (no baby exclusion, unlike hasEgg).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 151,
        values: vec![
            baby_data(VANILLA_ENTITY_TYPE_TURTLE_ID, true),
            turtle_laying_egg_data(true),
        ],
    }));
    assert!(laying(&world, 151));

    // The same flag on a non-turtle (bat) does NOT project turtle_laying_egg.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 152,
        values: vec![turtle_laying_egg_collision_data(true)],
    }));
    assert!(!laying(&world, 152));
}

#[test]
fn entity_model_instances_project_end_crystal_shows_bottom() {
    // Vanilla EndCrystal.DATA_SHOW_BOTTOM (BOOLEAN, default true) and
    // EndCrystalRenderState.showsBottom = entity.showsBottom().
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        160,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        [1.0, 64.0, -2.0],
    ));
    // A non-crystal: the field defaults true (it is unused and never reads id 9).
    world.apply_add_entity(protocol_add_entity(
        161,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [2.0, 64.0, -2.0],
    ));

    let shows_bottom = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .end_crystal_shows_bottom
    };

    // No synced value → the vanilla default `true` (the bottom slab is shown).
    assert!(shows_bottom(&world, 160));

    // Clearing DATA_SHOW_BOTTOM hides the base slab.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![end_crystal_show_bottom_data(false)],
    }));
    assert!(!shows_bottom(&world, 160));

    // Re-setting it shows the base again.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![end_crystal_show_bottom_data(true)],
    }));
    assert!(shows_bottom(&world, 160));

    // A non-crystal keeps the default `true` (unused).
    assert!(shows_bottom(&world, 161));
}

#[test]
fn entity_model_instances_project_end_crystal_beam_target() {
    // Vanilla EndCrystal.DATA_BEAM_TARGET (OPTIONAL_BLOCK_POS) projects as
    // EndCrystalRenderState.beamOffset = target block center - crystal position.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        162,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        [10.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        163,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [10.0, 64.0, -3.0],
    ));

    let beam = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .end_crystal_beam
    };

    assert!(beam(&world, 162).is_none());

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 162,
        values: vec![end_crystal_beam_target_data(Some(
            bbb_protocol::packets::BlockPos {
                x: 14,
                y: 67,
                z: -10,
            },
        ))],
    }));
    assert_eq!(beam(&world, 162).unwrap().beam_offset, [4.5, 3.5, -6.5]);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 162,
        values: vec![end_crystal_beam_target_data(None)],
    }));
    assert!(beam(&world, 162).is_none());

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 163,
        values: vec![end_crystal_beam_target_collision_data(Some(
            bbb_protocol::packets::BlockPos {
                x: 14,
                y: 67,
                z: -10,
            },
        ))],
    }));
    assert!(beam(&world, 163).is_none());
}

#[test]
fn entity_model_instance_projects_ender_dragon_healing_beam_from_source() {
    // Vanilla `EnderDragonRenderer.extractRenderState` stores nullable `beamOffset`; native must
    // preserve the world-projected offset so the renderer can submit `endCrystalBeam` after
    // body+eyes.
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 164,
        "entity_type_id": VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
        "y_rot": 0.0,
        "ender_dragon_death_time": 44.5,
        "ender_dragon_beam": { "beam_offset": [6.0, -0.1, 8.0] },
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 0.0, None).unwrap();

    assert_eq!(instance.kind, EntityModelKind::EnderDragon);
    assert_eq!(instance.render_state.ender_dragon_death_time, 44.5);
    assert_eq!(
        instance
            .render_state
            .ender_dragon_beam
            .expect("dragon beam source maps to render state")
            .beam_offset,
        [6.0, -0.1, 8.0]
    );
}

#[test]
fn entity_model_instances_project_bee_stinger() {
    // Vanilla Bee.DATA_FLAGS_ID and FLAG_HAS_STUNG are generated from Bee.java.
    let accessors = bee_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        96,
        VANILLA_ENTITY_TYPE_BEE_ID,
        [1.0, 64.0, -2.0],
    ));

    let has_stinger = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .bee_has_stinger
    };

    // A bee that has not stung keeps its stinger.
    assert!(has_stinger(&world, 96));

    // Setting Bee.hasStung (DATA_FLAGS_ID & 4) hides the stinger cube.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 96,
        values: vec![protocol_byte_data(
            accessors.flags_data_id,
            accessors.has_stung_mask,
        )],
    }));
    assert!(!has_stinger(&world, 96));
}

#[test]
fn entity_model_instances_project_bee_angry_from_anger_end_time() {
    // Vanilla Bee.DATA_ANGER_END_TIME (LONG): isAngry = endTime > 0 && endTime - gameTime
    // > 0. The world has no time set here, so the game time defaults to 0.
    let accessors = bee_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_BEE_ID,
        [1.0, 64.0, -2.0],
    ));

    let angry = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .bee_angry
    };

    // A bee with no anger end time (default -1) is calm.
    assert!(!angry(&world, 97));

    // An anger end time in the future (game time 0) makes the bee angry.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![protocol_long_data(accessors.anger_end_time_data_id, 200)],
    }));
    assert!(angry(&world, 97));

    // A zero/past end time is calm again.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![protocol_long_data(accessors.anger_end_time_data_id, 0)],
    }));
    assert!(!angry(&world, 97));
}

#[test]
fn entity_model_instances_project_camel_class_sit_then_sit_pose_from_pose_change_tick() {
    // Vanilla Camel.LAST_POSE_CHANGE_TICK (LONG): the SIGN encodes sitting (< 0) and the
    // magnitude is the change tick. getPoseTime = gameTime - |LAST_POSE_CHANGE_TICK|. A camel that
    // sat at game tick 100 (so LAST_POSE_CHANGE_TICK = -100) plays CAMEL_SIT for getPoseTime < 40,
    // then CAMEL_SIT_POSE (which starts when the 40-tick sit-down window ends).
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        150,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        152,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        [2.0, 64.0, -2.0],
    ));
    // Sat down at game tick 100 (negative magnitude → sitting).
    for (id, entity_type_id) in [
        (150, VANILLA_ENTITY_TYPE_CAMEL_ID),
        (152, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
    ] {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![camel_last_pose_change_tick_data(entity_type_id, -100)],
        }));
    }

    let sit_seconds = |world: &WorldStore, id: i32| {
        let state = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state;
        (
            state.camel_sit_seconds,
            state.camel_sit_pose_seconds,
            state.camel_standup_seconds,
        )
    };

    // getPoseTime = 120 - 100 = 20 (< 40): inside the sit-down window, so CAMEL_SIT is active at
    // 20/20 = 1.0 s, and sit-pose / standup are the stopped sentinel.
    world.apply_world_time(PlayTime {
        game_time: 120,
        clock_updates: Vec::new(),
    });
    assert_eq!(sit_seconds(&world, 150), (1.0, -1.0, -1.0));
    assert_eq!(sit_seconds(&world, 152), (1.0, -1.0, -1.0));

    // getPoseTime = 160 - 100 = 60 (>= 40): past the sit-down window, so CAMEL_SIT_POSE takes over
    // at (60 - 40)/20 = 1.0 s, and sit / standup are stopped.
    world.apply_world_time(PlayTime {
        game_time: 160,
        clock_updates: Vec::new(),
    });
    assert_eq!(sit_seconds(&world, 150), (-1.0, 1.0, -1.0));
    assert_eq!(sit_seconds(&world, 152), (-1.0, 1.0, -1.0));

    // Standing back up at game tick 200 (positive magnitude → not sitting): getPoseTime = 210 -
    // 200 = 10 (< 52 STANDUP window, >= 0), so CAMEL_STANDUP is active at 10/20 = 0.5 s.
    for (id, entity_type_id) in [
        (150, VANILLA_ENTITY_TYPE_CAMEL_ID),
        (152, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
    ] {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![camel_last_pose_change_tick_data(entity_type_id, 200)],
        }));
    }
    world.apply_world_time(PlayTime {
        game_time: 210,
        clock_updates: Vec::new(),
    });
    assert_eq!(sit_seconds(&world, 150), (-1.0, -1.0, 0.5));
    assert_eq!(sit_seconds(&world, 152), (-1.0, -1.0, 0.5));

    // Long after standing up (getPoseTime = 300 - 200 = 100 >= 52): no transition, all stopped.
    world.apply_world_time(PlayTime {
        game_time: 300,
        clock_updates: Vec::new(),
    });
    assert_eq!(sit_seconds(&world, 150), (-1.0, -1.0, -1.0));
    assert_eq!(sit_seconds(&world, 152), (-1.0, -1.0, -1.0));
}

#[test]
fn camel_body_anchor_y_offset_is_exposed_for_native_attachment_projection() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        151,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 151,
        values: vec![
            entity_pose_data(VANILLA_ENTITY_POSE_SITTING_ID),
            camel_last_pose_change_tick_data(VANILLA_ENTITY_TYPE_CAMEL_ID, -100),
        ],
    }));
    world.apply_world_time(PlayTime {
        game_time: 160,
        clock_updates: Vec::new(),
    });
    let sitting_front = world
        .entity_body_anchor_y_offset(151, true, 0.0)
        .expect("camel body anchor y-offset");
    assert!(
        (sitting_front - 0.77).abs() < 1.0e-5,
        "sitting front anchor: {sitting_front}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 151,
        values: vec![
            entity_pose_data(VANILLA_ENTITY_POSE_STANDING_ID),
            camel_last_pose_change_tick_data(VANILLA_ENTITY_TYPE_CAMEL_ID, 200),
        ],
    }));
    world.apply_world_time(PlayTime {
        game_time: 224,
        clock_updates: Vec::new(),
    });
    let standing_front_flex = world
        .entity_body_anchor_y_offset(151, true, 0.0)
        .expect("camel body anchor y-offset");
    assert!(
        (standing_front_flex - 1.508).abs() < 1.0e-5,
        "stand-up front flex anchor: {standing_front_flex}"
    );
}

#[test]
fn entity_model_instances_project_player_crouch_pose() {
    // Vanilla Entity.isCrouching reads the generated Entity pose accessor.

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        98,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -2.0],
    ));

    let crouching = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .is_crouching
    };
    let set_pose = |world: &mut WorldStore, id: i32, pose: i32| {
        world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![entity_pose_data(pose)],
        })
    };

    // A standing player is not crouching.
    assert!(!crouching(&world, 98));
    // Pose.CROUCHING projects the sneaking pose; standing again clears it.
    assert!(set_pose(&mut world, 98, VANILLA_ENTITY_POSE_CROUCHING_ID));
    assert!(crouching(&world, 98));
    assert!(set_pose(&mut world, 98, VANILLA_ENTITY_POSE_STANDING_ID));
    assert!(!crouching(&world, 98));
}

#[test]
fn entity_model_instances_project_upside_down_dinnerbone() {
    let mut world = WorldStore::new();
    // A sheep with a head turn (body 30, head 100, pitch -20): net head yaw 70,
    // head pitch -20 while upright.
    world.apply_add_entity(protocol_add_entity_with_rotation(
        88,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
        30.0,
        -20.0,
        100.0,
    ));
    // A non-living entity (boat) is never flipped by LivingEntityRenderer.
    world.apply_add_entity(protocol_add_entity(
        89,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [3.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(1);

    let render_state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    // A normally-named living entity is upright; its head look is unnegated.
    let upright = render_state(&world, 88);
    assert_eq!(upright.upside_down_height, None);
    assert_eq!(upright.head_yaw, 70.0);
    assert_eq!(upright.head_pitch, -20.0);

    // Vanilla LivingEntityRenderer.isUpsideDownName: the "Dinnerbone" name tag
    // flips the model; extractRenderState then negates the head yaw and pitch.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![entity_custom_name_data(Some("Dinnerbone"))],
    }));
    let flipped = render_state(&world, 88);
    assert!(flipped
        .upside_down_height
        .is_some_and(|height| height > 0.0));
    assert_eq!(flipped.head_yaw, -70.0);
    assert_eq!(flipped.head_pitch, 20.0);

    // "Grumm" flips too; an unrelated name does not.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![entity_custom_name_data(Some("Grumm"))],
    }));
    assert!(render_state(&world, 88).upside_down_height.is_some());
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![entity_custom_name_data(Some("Steve"))],
    }));
    assert_eq!(render_state(&world, 88).upside_down_height, None);

    // A non-living entity named Dinnerbone is still never flipped.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 89,
        values: vec![entity_custom_name_data(Some("Dinnerbone"))],
    }));
    assert_eq!(render_state(&world, 89).upside_down_height, None);
}

#[test]
fn entity_model_instances_project_sleeping_pose() {
    let mut world = WorldStore::new();
    // A sheep facing body yaw 45 with no bed resolved (no chunk loaded).
    world.apply_add_entity(protocol_add_entity_with_rotation(
        93,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
        45.0,
        0.0,
        45.0,
    ));

    let sleeping = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .sleeping
    };

    // An awake entity is not laid down.
    assert_eq!(sleeping(&world, 93), None);

    // Vanilla Pose.SLEEPING with no resolvable bed falls back to the body yaw and
    // no head offset (setupRotations `angle = bodyRot`).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 93,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID)],
    }));
    assert_eq!(
        sleeping(&world, 93),
        Some(SleepingPose {
            yaw_angle: 45.0,
            bed_offset: [0.0, 0.0],
        })
    );
}

#[test]
fn entity_model_instances_project_scale_attribute() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));

    let scale = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 97)
            .unwrap()
            .render_state
            .scale
    };

    // Default size projects scale 1.0.
    assert_eq!(scale(&world), 1.0);

    // Vanilla getScale() (the SCALE attribute) flows through to the render state.
    assert!(world.apply_update_attributes(UpdateAttributes {
        entity_id: 97,
        attributes: vec![AttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 1.25,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(scale(&world), 1.25);
}

#[test]
fn entity_model_instances_project_walk_animation() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        98,
        VANILLA_ENTITY_TYPE_COW_ID,
        [0.0, 64.0, 0.0],
    ));

    let walk = |world: &WorldStore| -> (bool, f32, f32, f32) {
        let state = entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 98)
            .unwrap()
            .render_state;
        (
            state.is_riding,
            state.walk_animation_pos,
            state.walk_animation_speed,
            state.worn_head_animation_pos,
        )
    };
    let sync = |world: &mut WorldStore, id: i32, x: f64| {
        assert!(world.apply_entity_position_sync(EntityPositionSync {
            id,
            position: Vec3d { x, y: 64.0, z: 0.0 },
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        }));
    };

    // A standing cow projects no limb swing.
    world.advance_entity_client_animations(1);
    assert_eq!(walk(&world), (false, 0.0, 0.0, 0.0));

    // After one 0.5-block step, the WalkAnimationState reaches speed = 0.4 and
    // position = 0.4 (targetSpeed = min(0.5 * 4, 1) = 1.0), and both flow through
    // EntityModelSourceState to the renderer EntityRenderState. Vanilla also reuses the same
    // position for LivingEntityRenderState.wornHeadAnimationPos while not riding a living entity.
    sync(&mut world, 98, 0.5);
    world.advance_entity_client_animations(1);
    let (is_riding, pos, speed, worn_head_pos) = walk(&world);
    assert!(!is_riding);
    assert!((speed - 0.4).abs() < 1e-5, "walk speed: {speed}");
    assert!((pos - 0.4).abs() < 1e-5, "walk position: {pos}");
    assert!(
        (worn_head_pos - 0.4).abs() < 1e-5,
        "worn head animation position: {worn_head_pos}"
    );

    // While riding a living entity, vanilla keeps the passenger's limb swing stopped but drives
    // worn skull animation from the vehicle walk animation position.
    world.apply_add_entity(protocol_add_entity(
        99,
        VANILLA_ENTITY_TYPE_COW_ID,
        [0.0, 64.0, 0.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        100,
        VANILLA_ENTITY_TYPE_COW_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 99,
        passenger_ids: vec![100],
    }));
    sync(&mut world, 99, 0.0);
    world.advance_entity_client_animations(1);
    sync(&mut world, 99, 0.5);
    world.advance_entity_client_animations(1);
    let passenger = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 100)
        .unwrap()
        .render_state;
    assert!(passenger.is_riding);
    assert_eq!(
        (passenger.walk_animation_pos, passenger.walk_animation_speed),
        (0.0, 0.0)
    );
    assert!(
        (passenger.worn_head_animation_pos - 0.4).abs() < 1e-5,
        "passenger worn head animation position: {}",
        passenger.worn_head_animation_pos
    );
}

#[test]
fn entity_light_coords_packs_vanilla_block_and_sky_with_on_fire_override() {
    use bbb_world::TerrainLight;

    // A generic (non-special) entity type — no per-renderer block-light override.
    let generic = VANILLA_ENTITY_TYPE_CHICKEN_ID;
    // Daylight surface (block 0, sky 15) -> LightCoordsUtil.pack(0, 15).
    assert_eq!(
        entity_light_coords(generic, &[], TerrainLight { sky: 15, block: 0 }),
        15 << 20
    );
    // Full-bright fallback (block 15, sky 15) -> LightCoordsUtil.FULL_BRIGHT.
    assert_eq!(
        entity_light_coords(generic, &[], TerrainLight { sky: 15, block: 15 }),
        15_728_880
    );
    // Torch-lit cave (block 14, sky 0) -> pack(14, 0).
    assert_eq!(
        entity_light_coords(generic, &[], TerrainLight { sky: 0, block: 14 }),
        14 << 4
    );
    // EntityRenderer.getBlockLightLevel forces block light to 15 on fire,
    // leaving sky light untouched.
    let on_fire = vec![entity_on_fire_data()];
    assert_eq!(
        entity_light_coords(generic, &on_fire, TerrainLight { sky: 4, block: 0 }),
        (15 << 4) | (4 << 20)
    );
}

#[test]
fn on_fire_model_source_reaches_dispatcher_flame_dimensions() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        910,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 910,
        values: vec![entity_on_fire_data()],
    }));

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 910)
        .unwrap();
    assert_eq!(instance.render_state.flame_dimensions, Some([0.4, 0.7]));
}

#[test]
fn entity_light_coords_applies_vanilla_per_renderer_block_light_overrides() {
    use bbb_world::TerrainLight;

    let dark = TerrainLight { sky: 0, block: 0 };
    // Vanilla `BlazeRenderer`/`MagmaCubeRenderer.getBlockLightLevel` = 15 unconditionally: even in pitch
    // dark, the block light packs to 15 (sky stays 0).
    assert_eq!(
        entity_light_coords(VANILLA_ENTITY_TYPE_BLAZE_ID, &[], dark),
        15 << 4
    );
    assert_eq!(
        entity_light_coords(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, &[], dark),
        15 << 4
    );
    // The source-derived full set also covers the direct renderer overrides and the three
    // `ThrownItemRenderer(..., true)` registrations.
    for full_bright in [
        VANILLA_ENTITY_TYPE_WITHER_ID,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        VANILLA_ENTITY_TYPE_ALLAY_ID,
        VANILLA_ENTITY_TYPE_VEX_ID,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
    ] {
        assert_eq!(entity_light_coords(full_bright, &[], dark), 15 << 4);
    }

    // Vanilla `ItemFrameRenderer`: normal item frames use the sampled light; glow item frames clamp
    // only the block component to at least `GLOW_FRAME_BRIGHTNESS = 5`.
    assert_eq!(
        entity_light_coords(VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, &[], dark),
        0
    );
    assert_eq!(
        entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, &[], dark),
        5 << 4
    );
    assert_eq!(
        entity_light_coords(
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            &[],
            TerrainLight { sky: 3, block: 7 }
        ),
        (7 << 4) | (3 << 20)
    );

    // Vanilla `GlowSquidRenderer.getBlockLightLevel` = max(super, (int)clampedLerp(1 - darkTicks/10, 0,
    // 15)). Undamaged (no DARK_TICKS data, so 0) -> fully bright 15.
    assert_eq!(
        entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &[], dark),
        15 << 4
    );
    let dark_ticks = |ticks: i32| vec![glow_squid_dark_ticks_data(ticks)];
    // Just hurt (darkTicks 100): factor = 1 - 10 = -9 < 0 -> min 0, so the boost is 0 and the squid is as
    // dark as its surroundings (block 0 here).
    assert_eq!(
        entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &dark_ticks(100), dark),
        0
    );
    // Mid-ramp (darkTicks 5): factor = 0.5 -> (int) lerp(0.5, 0, 15) = (int) 7.5 = 7.
    assert_eq!(
        entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &dark_ticks(5), dark),
        7 << 4
    );
    // darkTicks 10: factor = 0 -> 0 boost.
    assert_eq!(
        entity_light_coords(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &dark_ticks(10), dark),
        0
    );
    // The boost only ever RAISES the block light: a torch-lit (block 12) hurt glow squid keeps 12.
    assert_eq!(
        entity_light_coords(
            VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
            &dark_ticks(100),
            TerrainLight { sky: 0, block: 12 }
        ),
        12 << 4
    );
}

#[test]
fn entity_model_instances_project_full_bright_light_without_chunk_data() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        90,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.light_coords,
        bbb_renderer::ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
}

#[test]
fn entity_model_instances_project_hurt_red_overlay_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        91,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));

    let calm = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(!calm[0].render_state.has_red_overlay);

    assert!(world.apply_hurt_animation(bbb_protocol::packets::HurtAnimation { id: 91, yaw: 0.0 }));
    let hurt = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(hurt[0].render_state.has_red_overlay);
}

#[test]
fn creeper_white_overlay_progress_matches_vanilla_strobe() {
    // (int)(step * 10) even -> 0.0, odd -> clamp(step, 0.5, 1.0).
    assert_eq!(creeper_white_overlay_progress(0.0), 0.0);
    assert_eq!(creeper_white_overlay_progress(0.15), 0.5); // bucket 1 (odd), clamped up
    assert_eq!(creeper_white_overlay_progress(0.55), 0.55); // bucket 5 (odd)
    assert_eq!(creeper_white_overlay_progress(0.6), 0.0); // bucket 6 (even)
}

#[test]
fn entity_model_instances_project_creeper_white_overlay_from_world() {
    let accessors = creeper_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        92,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        [1.0, 64.0, -2.0],
    ));
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(resting[0].render_state.white_overlay_progress, 0.0);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 92,
        values: vec![EntityDataValue {
            data_id: accessors.swell_dir_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    world.advance_entity_client_animations(5);

    // swell = 5, getSwelling(1.0) = 5/28; the strobe lands in an odd bucket
    // so the projected progress is the clamped swelling (>= 0.5).
    let swelling = 5.0 / 28.0;
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.white_overlay_progress,
        creeper_white_overlay_progress(swelling)
    );
    assert!(instances[0].render_state.white_overlay_progress >= 0.5);
}

#[test]
fn entity_model_instances_project_creeper_swelling_from_world() {
    let accessors = creeper_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        93,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        [1.0, 64.0, -2.0],
    ));
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        resting[0].render_state.creeper_swelling, 0.0,
        "a calm creeper carries no swell, so CreeperRenderer.scale is the identity"
    );

    // Prime the creeper: swell direction = 1 advances the swell counter each tick, and
    // vanilla `Creeper.getSwelling(partialTick) = swell / SWELL_MAX (28)`.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 93,
        values: vec![EntityDataValue {
            data_id: accessors.swell_dir_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    world.advance_entity_client_animations(5);

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.creeper_swelling,
        5.0 / 28.0,
        "the projected swell drives the renderer inflate-and-flicker scale"
    );
}

#[test]
fn entity_model_instances_project_charged_creeper_from_world() {
    let accessors = creeper_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        [1.0, 64.0, -2.0],
    ));
    // A plain creeper is not powered, so it wears no CreeperPowerLayer energy swirl.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(!resting[0].render_state.creeper_powered);

    // Vanilla `Creeper.DATA_IS_POWERED`: set true for a lightning-charged creeper.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![EntityDataValue {
            data_id: accessors.powered_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(
        instances[0].render_state.creeper_powered,
        "the charged creeper projects isPowered, gating the energy-swirl overlay"
    );
}

#[test]
fn entity_model_instances_project_powered_wither_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        145,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        [3.0, 64.0, 1.0],
    ));
    // A wither with no synced health defaults to full (maxHealth 300), so it is not powered.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(!resting[0].render_state.wither_powered);

    // A healthy wither (health 200/300 > 150) stays un-powered.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 145,
        values: vec![living_entity_health_data(
            VANILLA_ENTITY_TYPE_WITHER_ID,
            200.0
        )],
    }));
    let healthy = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(!healthy[0].render_state.wither_powered);

    // Vanilla `WitherBoss.isPowered() = getHealth() <= getMaxHealth() / 2`: at or below half
    // health (120 ≤ 150) the `WitherArmorLayer` energy swirl ignites.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 145,
        values: vec![living_entity_health_data(
            VANILLA_ENTITY_TYPE_WITHER_ID,
            120.0
        )],
    }));
    let powered = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(
        powered[0].render_state.wither_powered,
        "the half-health wither projects isPowered, gating the energy-swirl overlay"
    );
}

#[test]
fn entity_model_instances_project_wither_side_head_rotations() {
    // Vanilla WitherBoss.DATA_TARGET_B: the first side-head target id.

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        145,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        [0.0, 64.0, 0.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        26,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [0.0, 64.0, 0.0],
    ));
    let target_eye = f64::from(world.probe_entity_camera_pose(26).unwrap().eye_height);
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 26,
        position: Vec3d {
            x: 11.3,
            y: 66.2 - target_eye,
            z: 0.0,
        },
        delta_movement: Vec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: true,
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 145,
        values: vec![protocol_wither_target_b_data(26)],
    }));
    world.advance_entity_client_animations(1);

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);
    let wither = instances
        .iter()
        .find(|instance| instance.entity_id == 145)
        .expect("wither instance");
    assert!(wither.render_state.wither_x_head_rots[0].abs() < 1.0e-5);
    assert_eq!(wither.render_state.wither_x_head_rots[1], 0.0);
    assert_eq!(wither.render_state.wither_y_head_rots, [-10.0, 0.0]);
}

fn protocol_wither_target_b_data(target_id: i32) -> EntityDataValue {
    let accessor = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_WITHER_ID,
        "DATA_TARGET_B",
    )
    .expect("generated WitherBoss target-B accessor must resolve");
    assert_eq!(
        accessor.owner,
        "net.minecraft.world.entity.boss.wither.WitherBoss"
    );
    protocol_int_data(accessor.data_id, target_id)
}

#[test]
fn entity_model_kind_uses_vanilla_chicken_variant_metadata() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CHICKEN_ID, &[]),
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            &[protocol_chicken_variant_data(1)]
        ),
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Warm,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            &[
                protocol_chicken_variant_data(2),
                baby_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, true),
            ]
        ),
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Cold,
            baby: true
        }
    );
}

#[test]
fn entity_model_instances_project_chicken_variants_from_world_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:chicken_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    assert_eq!(
        entity_model_kind_with_registries(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            &[protocol_chicken_variant_data(99)],
            Some(&world),
        ),
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
    );
    world.apply_add_entity(protocol_add_entity(
        26,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        27,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 26,
        values: vec![protocol_chicken_variant_data(0)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 27,
        values: vec![
            protocol_chicken_variant_data(2),
            baby_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, true),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![
                EntityModelInstance::chicken_variant(
                    26,
                    [1.0, 64.0, -2.0],
                    0.0,
                    ChickenModelVariant::Cold,
                    false
                ),
                EntityModelInstance::chicken_variant(
                    27,
                    [3.0, 64.0, -2.0],
                    0.0,
                    ChickenModelVariant::Warm,
                    true
                )
                .with_age_scale(0.5),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_kind_uses_vanilla_cow_variant_metadata() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[]),
        EntityModelKind::Cow {
            variant: CowModelVariant::Temperate,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[protocol_cow_variant_data(1)]),
        EntityModelKind::Cow {
            variant: CowModelVariant::Warm,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_COW_ID,
            &[
                protocol_cow_variant_data(2),
                baby_data(VANILLA_ENTITY_TYPE_COW_ID, true),
            ]
        ),
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: true
        }
    );
}

#[test]
fn entity_model_instances_project_cow_variants_from_world_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:cow_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    assert_eq!(
        entity_model_kind_with_registries(
            VANILLA_ENTITY_TYPE_COW_ID,
            &[protocol_cow_variant_data(99)],
            Some(&world),
        ),
        EntityModelKind::Cow {
            variant: CowModelVariant::Temperate,
            baby: false
        }
    );
    world.apply_add_entity(protocol_add_entity(
        30,
        VANILLA_ENTITY_TYPE_COW_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        31,
        VANILLA_ENTITY_TYPE_COW_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 30,
        values: vec![protocol_cow_variant_data(0)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 31,
        values: vec![
            protocol_cow_variant_data(2),
            baby_data(VANILLA_ENTITY_TYPE_COW_ID, true),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![
                EntityModelInstance::cow_variant(
                    30,
                    [1.0, 64.0, -2.0],
                    0.0,
                    CowModelVariant::Cold,
                    false
                ),
                EntityModelInstance::cow_variant(
                    31,
                    [3.0, 64.0, -2.0],
                    0.0,
                    CowModelVariant::Warm,
                    true
                )
                .with_age_scale(0.5),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_kind_uses_vanilla_pig_variant_metadata() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIG_ID, &[]),
        EntityModelKind::Pig {
            variant: PigModelVariant::Temperate,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIG_ID, &[protocol_pig_variant_data(1)]),
        EntityModelKind::Pig {
            variant: PigModelVariant::Warm,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PIG_ID,
            &[
                protocol_pig_variant_data(2),
                baby_data(VANILLA_ENTITY_TYPE_PIG_ID, true),
            ]
        ),
        EntityModelKind::Pig {
            variant: PigModelVariant::Cold,
            baby: true
        }
    );
}

#[test]
fn entity_model_instances_project_pig_variants_from_world_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:pig_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    assert_eq!(
        entity_model_kind_with_registries(
            VANILLA_ENTITY_TYPE_PIG_ID,
            &[protocol_pig_variant_data(99)],
            Some(&world),
        ),
        EntityModelKind::Pig {
            variant: PigModelVariant::Temperate,
            baby: false
        }
    );
    world.apply_add_entity(protocol_add_entity(
        100,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        101,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 100,
        values: vec![protocol_pig_variant_data(0)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 101,
        values: vec![
            protocol_pig_variant_data(2),
            baby_data(VANILLA_ENTITY_TYPE_PIG_ID, true),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![
                EntityModelInstance::pig(100, [1.0, 64.0, -2.0], 0.0, PigModelVariant::Cold, false,),
                EntityModelInstance::pig(101, [3.0, 64.0, -2.0], 0.0, PigModelVariant::Warm, true,)
                    .with_age_scale(0.5),
            ],
            1.0,
        )
    );
}

struct ReadyFarmAnimalPresentationLookup {
    presentation: FarmAnimalVariantPresentation,
    ready_asset_id: &'static str,
    texture: EntityDynamicTexture,
}

struct ReadyArmorTrimLookup {
    asset_id: &'static str,
    texture: EntityDynamicTexture,
    missing_texture: Option<EntityDynamicTexture>,
}

impl EntityVariantRegistryLookup for ReadyArmorTrimLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        EntityDataRegistryRevision::default()
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        if asset_id == self.asset_id {
            Some(self.texture)
        } else if asset_id == "minecraft:missingno" {
            self.missing_texture
        } else {
            None
        }
    }
}

#[test]
fn armor_trim_crosses_native_boundary_only_after_texture_is_ready() {
    let trim = WorldArmorTrimSprite {
        sprite_id: "example:trims/entity/humanoid/spire_opal".to_string(),
        decal: true,
    };
    let texture = EntityDynamicTexture { handle: 42_4300 };
    let ready = ReadyArmorTrimLookup {
        asset_id: "example:trims/entity/humanoid/spire_opal",
        texture,
        missing_texture: None,
    };
    let missing_texture = EntityDynamicTexture { handle: 42_4301 };
    let missing = ReadyArmorTrimLookup {
        asset_id: "example:some_other_sprite",
        texture,
        missing_texture: Some(missing_texture),
    };

    assert_eq!(armor_trim(Some(&trim), None), None);
    assert_eq!(
        armor_trim(Some(&trim), Some(&ready)),
        Some(EntityArmorTrim {
            texture,
            decal: true,
        })
    );
    assert_eq!(
        armor_trim(Some(&trim), Some(&missing)),
        Some(EntityArmorTrim {
            texture: missing_texture,
            decal: true,
        }),
        "vanilla's atlas lookup preserves the trim layer with the ready missing sprite"
    );
}

impl EntityVariantRegistryLookup for ReadyFarmAnimalPresentationLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        EntityDataRegistryRevision::default()
    }

    fn farm_animal_variant_presentation(
        &self,
        _registry: &str,
        _holder_id: i32,
    ) -> Option<FarmAnimalVariantPresentation> {
        Some(self.presentation.clone())
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        (asset_id == self.ready_asset_id).then_some(self.texture)
    }
}

#[test]
fn ready_farm_animal_presentation_switches_model_and_age_texture_atomically() {
    let texture = EntityDynamicTexture { handle: 42_4242 };
    let mut lookup = ReadyFarmAnimalPresentationLookup {
        presentation: FarmAnimalVariantPresentation {
            model: FarmAnimalVariantModel::Cold,
            adult_asset_id: "example:entity/cow/highland".to_string(),
            adult_texture_path: "example:textures/entity/cow/highland.png".to_string(),
            baby_asset_id: "example:entity/cow/highland_baby".to_string(),
            baby_texture_path: "example:textures/entity/cow/highland_baby.png".to_string(),
        },
        ready_asset_id: "example:entity/cow/highland",
        texture,
    };
    let values = [protocol_cow_variant_data(0)];
    let mut adult_kind = EntityModelKind::Cow {
        variant: CowModelVariant::Temperate,
        baby: false,
    };
    assert_eq!(
        apply_ready_farm_animal_variant_presentation(
            &mut adult_kind,
            VANILLA_ENTITY_TYPE_COW_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );
    assert_eq!(
        adult_kind,
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: false,
        }
    );

    let mut baby_kind = EntityModelKind::Cow {
        variant: CowModelVariant::Warm,
        baby: true,
    };
    assert_eq!(
        apply_ready_farm_animal_variant_presentation(
            &mut baby_kind,
            VANILLA_ENTITY_TYPE_COW_ID,
            &values,
            Some(&lookup),
        ),
        None,
        "an adult upload must not expose the baby presentation"
    );
    assert_eq!(
        baby_kind,
        EntityModelKind::Cow {
            variant: CowModelVariant::Warm,
            baby: true,
        },
        "unready presentation must retain the complete built-in fallback"
    );

    lookup.ready_asset_id = "example:entity/cow/highland_baby";
    assert_eq!(
        apply_ready_farm_animal_variant_presentation(
            &mut baby_kind,
            VANILLA_ENTITY_TYPE_COW_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );
    assert_eq!(
        baby_kind,
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: true,
        }
    );
}

struct ReadyCatPresentationLookup {
    presentation: CatVariantPresentation,
    ready_asset_id: &'static str,
    texture: EntityDynamicTexture,
    revision: EntityDataRegistryRevision,
}

impl EntityVariantRegistryLookup for ReadyCatPresentationLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        self.revision
    }

    fn cat_variant_presentation(&self, _holder_id: i32) -> Option<CatVariantPresentation> {
        Some(self.presentation.clone())
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        (asset_id == self.ready_asset_id).then_some(self.texture)
    }
}

#[test]
fn ready_cat_presentation_selects_the_age_specific_network_texture() {
    let texture = EntityDynamicTexture { handle: 73_001 };
    let mut lookup = ReadyCatPresentationLookup {
        presentation: CatVariantPresentation {
            adult_asset_id: "example:entity/cat/lynx".to_string(),
            adult_texture_path: "example:textures/entity/cat/lynx.png".to_string(),
            baby_asset_id: "example:entity/cat/lynx_baby".to_string(),
            baby_texture_path: "example:textures/entity/cat/lynx_baby.png".to_string(),
        },
        ready_asset_id: "example:entity/cat/lynx",
        texture,
        revision: EntityDataRegistryRevision::default(),
    };
    let values = [protocol_cat_variant_data(0)];
    let adult_kind = EntityModelKind::Feline {
        cat: true,
        baby: false,
        cat_variant: CatModelVariant::Black,
        collar: None,
    };
    assert_eq!(
        ready_cat_variant_texture(
            &adult_kind,
            VANILLA_ENTITY_TYPE_CAT_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );

    let baby_kind = EntityModelKind::Feline {
        cat: true,
        baby: true,
        cat_variant: CatModelVariant::Black,
        collar: None,
    };
    assert_eq!(
        ready_cat_variant_texture(
            &baby_kind,
            VANILLA_ENTITY_TYPE_CAT_ID,
            &values,
            Some(&lookup),
        ),
        None,
        "an adult upload must not expose the baby presentation"
    );
    lookup.ready_asset_id = "example:entity/cat/lynx_baby";
    assert_eq!(
        ready_cat_variant_texture(
            &baby_kind,
            VANILLA_ENTITY_TYPE_CAT_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );
    assert_eq!(
        ready_cat_variant_texture(
            &baby_kind,
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            &values,
            Some(&lookup),
        ),
        None,
        "the cat holder never changes the ocelot texture"
    );

    lookup.revision.session = 1;
    assert_eq!(
        ready_cat_variant_texture(
            &adult_kind,
            VANILLA_ENTITY_TYPE_CAT_ID,
            &values,
            Some(&lookup),
        ),
        None,
        "a new registry generation must not reinterpret an old holder ordinal"
    );
}

struct ReadyWolfPresentationLookup {
    presentation: WolfVariantPresentation,
    ready_asset_id: &'static str,
    texture: EntityDynamicTexture,
}

impl EntityVariantRegistryLookup for ReadyWolfPresentationLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        EntityDataRegistryRevision::default()
    }

    fn wolf_variant_presentation(&self, _holder_id: i32) -> Option<WolfVariantPresentation> {
        Some(self.presentation.clone())
    }

    fn ready_texture(&self, asset_id: &str) -> Option<EntityDynamicTexture> {
        (asset_id == self.ready_asset_id).then_some(self.texture)
    }
}

#[test]
fn ready_wolf_presentation_selects_vanilla_age_and_state_texture() {
    let texture = EntityDynamicTexture { handle: 74_001 };
    let mut lookup = ReadyWolfPresentationLookup {
        presentation: WolfVariantPresentation {
            adult: wolf_test_asset_presentation("adult"),
            baby: wolf_test_asset_presentation("baby"),
        },
        ready_asset_id: "example:adult/wild",
        texture,
    };
    let values = [protocol_wolf_variant_data(0)];
    for (baby, tame, angry, expected_asset_id) in [
        (false, false, false, "example:adult/wild"),
        (false, true, false, "example:adult/tame"),
        (false, false, true, "example:adult/angry"),
        (true, false, false, "example:baby/wild"),
        (true, true, false, "example:baby/tame"),
        (true, false, true, "example:baby/angry"),
        (true, true, true, "example:baby/tame"),
    ] {
        lookup.ready_asset_id = expected_asset_id;
        let kind = EntityModelKind::Wolf {
            baby,
            tame,
            angry,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        };
        assert_eq!(
            ready_wolf_variant_texture(&kind, VANILLA_ENTITY_TYPE_WOLF_ID, &values, Some(&lookup),),
            Some(texture),
            "wrong selection for baby={baby}, tame={tame}, angry={angry}"
        );
    }
}

fn wolf_test_asset_presentation(age: &str) -> WolfVariantAssetPresentation {
    WolfVariantAssetPresentation {
        wild_asset_id: format!("example:{age}/wild"),
        wild_texture_path: format!("example:textures/{age}/wild.png"),
        tame_asset_id: format!("example:{age}/tame"),
        tame_texture_path: format!("example:textures/{age}/tame.png"),
        angry_asset_id: format!("example:{age}/angry"),
        angry_texture_path: format!("example:textures/{age}/angry.png"),
    }
}

struct ReadyZombieNautilusPresentationLookup {
    ready: bool,
    texture: EntityDynamicTexture,
}

impl EntityVariantRegistryLookup for ReadyZombieNautilusPresentationLookup {
    fn entity_data_registry_revision(&self) -> EntityDataRegistryRevision {
        EntityDataRegistryRevision::default()
    }

    fn zombie_nautilus_variant_presentation(
        &self,
        _holder_id: i32,
    ) -> Option<ZombieNautilusVariantPresentation> {
        Some(ZombieNautilusVariantPresentation {
            model: ZombieNautilusVariantModelType::Warm,
            asset_id: "example:entity/nautilus/coral".to_string(),
            texture_path: "example:textures/entity/nautilus/coral.png".to_string(),
        })
    }

    fn ready_texture(&self, _asset_id: &str) -> Option<EntityDynamicTexture> {
        self.ready.then_some(self.texture)
    }
}

#[test]
fn zombie_nautilus_network_model_waits_for_its_matching_texture() {
    let texture = EntityDynamicTexture { handle: 75_001 };
    let mut lookup = ReadyZombieNautilusPresentationLookup {
        ready: false,
        texture,
    };
    let values = [protocol_zombie_nautilus_variant_data(0)];
    let mut kind = EntityModelKind::ZombieNautilus {
        variant: ZombieNautilusModelVariant::Temperate,
    };

    assert_eq!(
        apply_ready_zombie_nautilus_variant_presentation(
            &mut kind,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &values,
            Some(&lookup),
        ),
        None
    );
    assert_eq!(
        kind,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate,
        },
        "unready custom texture must retain the complete built-in presentation"
    );

    lookup.ready = true;
    assert_eq!(
        apply_ready_zombie_nautilus_variant_presentation(
            &mut kind,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &values,
            Some(&lookup),
        ),
        Some(texture)
    );
    assert_eq!(
        kind,
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm,
        }
    );
}

#[test]
fn entity_model_instances_project_pig_saddle_render_state() {
    const SADDLE_ITEM_ID: i32 = 740;
    let trim_texture = EntityDynamicTexture { handle: 42_4313 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/pig_saddle/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
            },
        }],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        110,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [1.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(110)));
    let unresolved = state(&world, 110);
    assert!(
        !unresolved.pig_saddle,
        "without the item registry's saddle-slot map, a raw item id is not enough"
    );
    assert_eq!(unresolved.pig_saddle_trim, None);
    assert!(!unresolved.pig_saddle_foil);

    world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    let saddled = state(&world, 110);
    assert!(saddled.pig_saddle);
    assert!(saddled.pig_saddle_foil);
    assert_eq!(
        saddled.pig_saddle_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );
}

#[test]
fn entity_model_instances_project_snow_golem_pumpkin_render_state() {
    let (pumpkin_data_id, _pumpkin_default, _pumpkin_mask) = snow_golem_pumpkin_accessor();
    let pumpkin = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .snow_golem_pumpkin
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        111,
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        112,
        VANILLA_ENTITY_TYPE_COW_ID,
        [3.0, 64.0, -3.0],
    ));

    assert!(
        pumpkin(&world, 111),
        "SnowGolem.DATA_PUMPKIN_ID defaults to the generated pumpkin mask"
    );
    assert!(!pumpkin(&world, 112));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 111,
        values: vec![EntityDataValue {
            data_id: pumpkin_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(0),
        }],
    }));
    assert!(!pumpkin(&world, 111));
}

#[test]
fn entity_model_instances_project_equine_saddle_and_ridden_render_state() {
    const SADDLE_ITEM_ID: i32 = 741;
    let trim_texture = EntityDynamicTexture { handle: 42_4313 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/horse_saddle/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
            },
        }],
    };
    let equine_saddle = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.equine_saddle,
            render_state.equine_saddle_ridden,
            render_state.equine_saddle_trim,
            render_state.equine_saddle_foil,
        )
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        111,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        112,
        VANILLA_ENTITY_TYPE_COW_ID,
        [2.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(111)));
    assert_eq!(
        equine_saddle(&world, 111),
        (false, false, None, false),
        "without the item registry's saddle-slot map, a raw item id is not enough"
    );

    world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    assert_eq!(
        equine_saddle(&world, 111),
        (
            true,
            false,
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            true,
        )
    );

    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 111,
        passenger_ids: vec![112],
    }));
    assert_eq!(
        equine_saddle(&world, 111),
        (
            true,
            true,
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            true,
        )
    );
}

#[test]
fn entity_model_instance_projects_equine_tail_counter_from_source() {
    // Vanilla `AbstractHorseRenderer.extractRenderState` maps `tailCounter > 0`
    // to `EquineRenderState.animateTail`; the world layer owns the client-side
    // random counter and native must preserve that bool in the renderer state.
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 113,
        "entity_type_id": VANILLA_ENTITY_TYPE_HORSE_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -3.0 },
        "y_rot": 0.0,
        "equine_animate_tail": true,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();

    assert_eq!(
        instance.kind,
        EntityModelKind::Horse {
            baby: false,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None,
        }
    );
    assert!(instance.render_state.equine_animate_tail);
}

#[test]
fn entity_model_instance_projects_equine_pose_animations_from_source() {
    // Vanilla `AbstractHorseRenderer.extractRenderState` forwards the partial-lerped
    // eat / stand / mouth animation floats to `EquineRenderState`; native preserves
    // the world-owned source projection.
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 114,
        "entity_type_id": VANILLA_ENTITY_TYPE_HORSE_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -3.0 },
        "y_rot": 0.0,
        "equine_eat_animation": 0.25,
        "equine_stand_animation": 0.5,
        "equine_feeding_animation": 0.75,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();

    assert_eq!(instance.render_state.equine_eat_animation, 0.25);
    assert_eq!(instance.render_state.equine_stand_animation, 0.5);
    assert_eq!(instance.render_state.equine_feeding_animation, 0.75);
}

#[test]
fn entity_model_instances_project_strider_saddle_and_ridden_render_state() {
    const SADDLE_ITEM_ID: i32 = 742;
    let trim_texture = EntityDynamicTexture { handle: 42_4314 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/strider_saddle/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
            },
        }],
    };
    let strider_state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        113,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        114,
        VANILLA_ENTITY_TYPE_COW_ID,
        [2.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(113)));
    let unresolved = strider_state(&world, 113);
    assert!(
        !unresolved.strider_saddle,
        "without the item registry's saddle-slot map, a raw item id is not enough"
    );
    assert!(!unresolved.strider_ridden);
    assert_eq!(unresolved.strider_saddle_trim, None);
    assert!(!unresolved.strider_saddle_foil);

    world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    let saddled = strider_state(&world, 113);
    assert!(saddled.strider_saddle);
    assert!(!saddled.strider_ridden);
    assert!(saddled.strider_saddle_foil);
    assert_eq!(
        saddled.strider_saddle_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );

    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 113,
        passenger_ids: vec![114],
    }));
    let ridden = strider_state(&world, 113);
    assert!(ridden.strider_saddle);
    assert!(ridden.strider_ridden);
    assert!(ridden.strider_saddle_foil);
    assert_eq!(ridden.strider_saddle_trim, saddled.strider_saddle_trim);
}

#[test]
fn entity_model_instances_project_camel_saddle_and_ridden_render_state() {
    const SADDLE_ITEM_ID: i32 = 743;
    let camel_trim_texture = EntityDynamicTexture { handle: 42_4315 };
    let camel_trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/camel_saddle/sentry_iron",
        texture: camel_trim_texture,
        missing_texture: None,
    };
    let husk_trim_texture = EntityDynamicTexture { handle: 42_4316 };
    let husk_trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/camel_husk_saddle/sentry_iron",
        texture: husk_trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
            },
        }],
    };
    let camel_saddle = |world: &WorldStore, id: i32, lookup: &ReadyArmorTrimLookup| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        115,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        116,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        117,
        VANILLA_ENTITY_TYPE_COW_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(115)));
    let unresolved = camel_saddle(&world, 115, &camel_trim_lookup);
    assert!(
        !unresolved.camel_saddle,
        "without the item registry's saddle-slot map, a raw item id is not enough"
    );
    assert!(!unresolved.camel_saddle_ridden);
    assert_eq!(unresolved.camel_saddle_trim, None);
    assert!(!unresolved.camel_saddle_foil);

    world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    let saddled = camel_saddle(&world, 115, &camel_trim_lookup);
    assert!(saddled.camel_saddle);
    assert!(!saddled.camel_saddle_ridden);
    assert!(saddled.camel_saddle_foil);
    assert_eq!(
        saddled.camel_saddle_trim,
        Some(EntityArmorTrim {
            texture: camel_trim_texture,
            decal: false,
        })
    );

    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 115,
        passenger_ids: vec![117],
    }));
    let ridden = camel_saddle(&world, 115, &camel_trim_lookup);
    assert!(ridden.camel_saddle);
    assert!(ridden.camel_saddle_ridden);
    assert!(ridden.camel_saddle_foil);
    assert_eq!(ridden.camel_saddle_trim, saddled.camel_saddle_trim);

    assert!(world.apply_set_equipment(saddle(116)));
    let husk = camel_saddle(&world, 116, &husk_trim_lookup);
    assert!(husk.camel_saddle);
    assert!(!husk.camel_saddle_ridden);
    assert!(husk.camel_saddle_foil);
    assert_eq!(
        husk.camel_saddle_trim,
        Some(EntityArmorTrim {
            texture: husk_trim_texture,
            decal: false,
        })
    );
}

#[test]
fn entity_model_instances_project_nautilus_saddle_render_state() {
    const SADDLE_ITEM_ID: i32 = 744;
    let trim_texture = EntityDynamicTexture { handle: 42_4317 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/nautilus_saddle/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let saddle = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: ItemStackSummary {
                item_id: Some(SADDLE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
            },
        }],
    };
    let nautilus_saddle = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        118,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        119,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        [2.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(saddle(118)));
    let unresolved = nautilus_saddle(&world, 118);
    assert_eq!(
        (
            unresolved.nautilus_saddle,
            unresolved.nautilus_saddle_trim,
            unresolved.nautilus_saddle_foil,
        ),
        (false, None, false),
        "without the item registry's saddle-slot map, a raw item id is not enough"
    );

    world.set_default_item_equipment_slots(std::collections::BTreeMap::from([(
        SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    let living = nautilus_saddle(&world, 118);
    assert!(living.nautilus_saddle);
    assert!(living.nautilus_saddle_foil);
    assert_eq!(
        living.nautilus_saddle_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );

    assert!(world.apply_set_equipment(saddle(119)));
    let zombie = nautilus_saddle(&world, 119);
    assert!(zombie.nautilus_saddle);
    assert!(zombie.nautilus_saddle_foil);
    assert_eq!(zombie.nautilus_saddle_trim, living.nautilus_saddle_trim);
}

#[test]
fn entity_model_instances_project_nautilus_body_armor_render_state() {
    const IRON_NAUTILUS_ARMOR_ITEM_ID: i32 = 747;
    const NETHERITE_NAUTILUS_ARMOR_ITEM_ID: i32 = 748;
    let trim_texture = EntityDynamicTexture { handle: 42_4308 };
    let missing_texture = EntityDynamicTexture { handle: 42_4307 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/nautilus_body/sentry_iron_darker",
        texture: trim_texture,
        missing_texture: Some(missing_texture),
    };

    let body_armor = |entity_id: i32, item_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
            },
        }],
    };
    let nautilus_body_armor = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.nautilus_body_armor,
            render_state.nautilus_body_armor_trim,
            render_state.nautilus_body_armor_foil,
        )
    };

    let mut world = WorldStore::new();
    world.set_default_nautilus_body_armor_materials(std::collections::BTreeMap::from([
        (IRON_NAUTILUS_ARMOR_ITEM_ID, WorldArmorMaterialKind::Iron),
        (
            NETHERITE_NAUTILUS_ARMOR_ITEM_ID,
            WorldArmorMaterialKind::Netherite,
        ),
    ]));
    world.apply_add_entity(protocol_add_entity(
        123,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        124,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        125,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 125,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID, true)],
    }));

    assert!(world.apply_set_equipment(body_armor(123, IRON_NAUTILUS_ARMOR_ITEM_ID)));
    assert!(world.apply_set_equipment(body_armor(124, NETHERITE_NAUTILUS_ARMOR_ITEM_ID)));
    assert!(world.apply_set_equipment(body_armor(125, IRON_NAUTILUS_ARMOR_ITEM_ID)));

    assert_eq!(
        nautilus_body_armor(&world, 123),
        (
            Some(EntityArmorMaterial::Iron),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            true,
        )
    );
    assert_eq!(
        nautilus_body_armor(&world, 124),
        (
            Some(EntityArmorMaterial::Netherite),
            Some(EntityArmorTrim {
                texture: missing_texture,
                decal: false,
            }),
            true,
        ),
        "the absent netherite trim sprite keeps the layer through vanilla's missing sprite"
    );
    assert_eq!(
        nautilus_body_armor(&world, 125),
        (None, None, false),
        "baby living nautilus skip the body armor equipment layer"
    );
}

#[test]
fn entity_model_instances_project_horse_body_armor_render_state() {
    const LEATHER_HORSE_ARMOR_ITEM_ID: i32 = 749;
    const NETHERITE_HORSE_ARMOR_ITEM_ID: i32 = 750;
    const LEATHER_DYE: i32 = 0x0033_66CC;
    let trim_texture = EntityDynamicTexture { handle: 42_4309 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/horse_body/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let body_armor = |entity_id: i32, item_id: i32, dyed_color: Option<i32>| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    dyed_color,
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
            },
        }],
    };
    let horse_body_armor = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.equine_body_armor,
            render_state.equine_body_armor_trim,
            render_state.equine_body_armor_dye,
            render_state.equine_body_armor_foil,
        )
    };

    let mut world = WorldStore::new();
    world.set_default_horse_body_armor_materials(std::collections::BTreeMap::from([
        (LEATHER_HORSE_ARMOR_ITEM_ID, WorldArmorMaterialKind::Leather),
        (
            NETHERITE_HORSE_ARMOR_ITEM_ID,
            WorldArmorMaterialKind::Netherite,
        ),
    ]));
    world.apply_add_entity(protocol_add_entity(
        126,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        127,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        128,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        [3.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        129,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        [4.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 129,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_HORSE_ID, true)],
    }));

    assert!(world.apply_set_equipment(body_armor(
        126,
        LEATHER_HORSE_ARMOR_ITEM_ID,
        Some(LEATHER_DYE),
    )));
    assert!(world.apply_set_equipment(body_armor(127, NETHERITE_HORSE_ARMOR_ITEM_ID, None,)));
    assert!(world.apply_set_equipment(body_armor(128, NETHERITE_HORSE_ARMOR_ITEM_ID, None,)));
    assert!(world.apply_set_equipment(body_armor(
        129,
        LEATHER_HORSE_ARMOR_ITEM_ID,
        Some(LEATHER_DYE),
    )));

    assert_eq!(
        horse_body_armor(&world, 126),
        (
            Some(EntityArmorMaterial::Leather),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            Some(LEATHER_DYE as u32),
            true,
        )
    );
    assert_eq!(
        horse_body_armor(&world, 127),
        (
            Some(EntityArmorMaterial::Netherite),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            None,
            true,
        )
    );
    assert_eq!(
        horse_body_armor(&world, 128),
        (None, None, None, false),
        "skeleton horses are not in vanilla CAN_WEAR_HORSE_ARMOR"
    );
    assert_eq!(
        horse_body_armor(&world, 129),
        (None, None, None, false),
        "baby horses skip the body armor equipment layer"
    );
}

#[test]
fn entity_model_instances_project_wolf_body_armor_render_state() {
    const WOLF_ARMOR_ITEM_ID: i32 = 751;
    const WOLF_ARMOR_DYE: i32 = 0x0033_66CC;
    let trim_texture = EntityDynamicTexture { handle: 42_4310 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/wolf_body/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let body_armor =
        |entity_id: i32, damage: i32, enchantment_glint_override: Option<bool>| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: ItemStackSummary {
                    item_id: Some(WOLF_ARMOR_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        dyed_color: Some(WOLF_ARMOR_DYE),
                        damage: Some(damage),
                        enchantments: vec![ItemEnchantmentSummary {
                            holder_id: 12,
                            level: 1,
                        }],
                        enchantment_glint_override,
                        armor_trim_material_id: Some(1), // iron
                        armor_trim_pattern_id: Some(0),  // sentry
                        ..Default::default()
                    },
                },
            }],
        };
    let wolf_body_armor = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.wolf_body_armor,
            render_state.wolf_body_armor_trim,
            render_state.wolf_body_armor_dye,
            render_state.wolf_body_armor_crackiness,
            render_state.wolf_body_armor_foil,
        )
    };

    let mut world = WorldStore::new();
    world.set_default_wolf_body_armor_materials(std::collections::BTreeMap::from([(
        WOLF_ARMOR_ITEM_ID,
        WorldArmorMaterialKind::ArmadilloScute,
    )]));
    world.set_default_item_max_damage(std::collections::BTreeMap::from([(WOLF_ARMOR_ITEM_ID, 64)]));
    world.apply_add_entity(protocol_add_entity(
        130,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        131,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        132,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 131,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_WOLF_ID, true)],
    }));

    assert!(world.apply_set_equipment(body_armor(130, 24, None)));
    assert!(world.apply_set_equipment(body_armor(131, 44, None)));
    assert!(world.apply_set_equipment(body_armor(132, 24, Some(false))));

    assert_eq!(
        wolf_body_armor(&world, 130),
        (
            Some(EntityArmorMaterial::ArmadilloScute),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            Some(WOLF_ARMOR_DYE as u32),
            Some(WolfArmorCrackiness::Medium),
            true
        )
    );
    assert_eq!(
        wolf_body_armor(&world, 132),
        (
            Some(EntityArmorMaterial::ArmadilloScute),
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            }),
            Some(WOLF_ARMOR_DYE as u32),
            Some(WolfArmorCrackiness::Medium),
            false
        ),
        "enchantment_glint_override=false wins over non-empty enchantments"
    );
    assert_eq!(
        wolf_body_armor(&world, 131),
        (None, None, None, None, false),
        "baby wolves skip the adult-only WolfArmorLayer"
    );
}

#[test]
fn entity_model_instances_project_llama_body_decor_render_state() {
    const WHITE_CARPET_ITEM_ID: i32 = 745;
    const BLACK_CARPET_ITEM_ID: i32 = 746;
    let trim_texture = EntityDynamicTexture { handle: 42_4311 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/llama_body/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let body_item = |entity_id: i32, item_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1), // iron
                    armor_trim_pattern_id: Some(0),  // sentry
                    ..Default::default()
                },
            },
        }],
    };
    let llama_body_decor = |world: &WorldStore, id: i32| {
        let render_state = entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            None,
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state;
        (
            render_state.llama_body_decor,
            render_state.llama_body_decor_trim,
            render_state.llama_body_decor_foil,
        )
    };

    let mut world = WorldStore::new();
    world.set_default_llama_body_decor_colors(std::collections::BTreeMap::from([
        (WHITE_CARPET_ITEM_ID, WorldLlamaBodyDecorColor::White),
        (BLACK_CARPET_ITEM_ID, WorldLlamaBodyDecorColor::Black),
    ]));
    world.apply_add_entity(protocol_add_entity(
        120,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        121,
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
        [2.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        122,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 122,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true)],
    }));

    assert!(world.apply_set_equipment(body_item(120, WHITE_CARPET_ITEM_ID)));
    assert!(world.apply_set_equipment(body_item(121, BLACK_CARPET_ITEM_ID)));
    assert!(world.apply_set_equipment(body_item(122, WHITE_CARPET_ITEM_ID)));

    let expected_trim = Some(EntityArmorTrim {
        texture: trim_texture,
        decal: false,
    });
    assert_eq!(
        llama_body_decor(&world, 120),
        (Some(EntityDyeColor::White), expected_trim, true)
    );
    assert_eq!(
        llama_body_decor(&world, 121),
        (Some(EntityDyeColor::Black), expected_trim, true),
        "an explicit carpet wins over the trader-llama empty-stack fallback"
    );
    assert_eq!(
        llama_body_decor(&world, 122),
        (None, None, false),
        "baby llamas ignore body items; renderer handles trader baby fallback separately"
    );
}

#[test]
fn entity_model_instances_project_happy_ghast_harness_and_ropes_render_state() {
    const WHITE_HARNESS_ITEM_ID: i32 = 747;
    let trim_texture = EntityDynamicTexture { handle: 42_4312 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/happy_ghast_body/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };
    let harness = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                item_id: Some(WHITE_HARNESS_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    }],
                    armor_trim_material_id: Some(1),
                    armor_trim_pattern_id: Some(0),
                    ..Default::default()
                },
            },
        }],
    };

    let mut world = WorldStore::new();
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:harnesses".to_string(),
                entries: vec![WHITE_HARNESS_ITEM_ID],
            }],
        }],
    });
    world.set_default_happy_ghast_harness_colors(std::collections::BTreeMap::from([(
        WHITE_HARNESS_ITEM_ID,
        WorldHappyGhastHarnessColor::White,
    )]));
    world.apply_add_entity(protocol_add_entity(
        126,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        128,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(harness(126)));
    let accessors = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 126,
        values: vec![EntityDataValue {
            data_id: accessors.is_leash_holder_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 126,
        passenger_ids: vec![128],
    }));

    let instances = entity_model_instances_from_world_with_registry_at_partial_tick(
        &world,
        None,
        0.0,
        Some(&trim_lookup),
    );
    for (entity_id, baby, ridden) in [(126, false, true)] {
        let instance = instances
            .iter()
            .find(|instance| instance.entity_id == entity_id)
            .unwrap();
        assert_eq!(instance.kind, EntityModelKind::HappyGhast { baby });
        assert!(instance.render_state.happy_ghast_body_item);
        assert_eq!(
            instance.render_state.happy_ghast_harness,
            Some(EntityDyeColor::White)
        );
        assert_eq!(
            instance.render_state.happy_ghast_harness_trim,
            Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            })
        );
        assert!(instance.render_state.happy_ghast_harness_foil);
        assert!(instance.render_state.happy_ghast_body_item_is_harness);
        assert!(instance.render_state.happy_ghast_is_leash_holder);
        assert_eq!(instance.render_state.happy_ghast_ridden, ridden);
    }
}

#[test]
fn entity_model_instances_project_armor_stand_flags_and_pose_without_pose_lerp() {
    let mut world = WorldStore::new();
    let accessors = armor_stand_metadata_accessors();
    world.apply_add_entity(protocol_add_entity(
        5,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        [1.0, 64.0, -2.0],
    ));
    let mut pose = armor_stand_default_model_pose();
    pose.body = [0.0, 15.0, 0.0];
    pose.left_arm = [-30.0, 0.0, -20.0];
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 5,
        values: vec![
            protocol_byte_data(
                accessors.client_flags_data_id,
                accessors.small_mask
                    | accessors.show_arms_mask
                    | accessors.no_baseplate_mask
                    | accessors.marker_mask,
            ),
            protocol_rotations_data(accessors.body_pose_data_id, pose.body),
            protocol_rotations_data(accessors.left_arm_pose_data_id, pose.left_arm),
        ],
    }));

    let start_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        start_instances[0].kind, instances[0].kind,
        "vanilla copies each synced ArmorStand Rotations pose directly at every partial tick"
    );

    assert_eq!(
        instances,
        aged(
            vec![EntityModelInstance::armor_stand_with_marker(
                5,
                [1.0, 64.0, -2.0],
                0.0,
                true,
                true,
                true,
                false,
                pose,
            )
            .with_age_scale(0.5),],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_project_armor_stand_hit_wiggle() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        5,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        [1.0, 64.0, -2.0],
    ));

    fn armor_stand_wiggle(world: &WorldStore, partial_tick: f32) -> f32 {
        entity_model_instances_from_world_at_partial_tick(world, None, partial_tick)
            .into_iter()
            .find(|instance| instance.entity_id == 5)
            .unwrap()
            .render_state
            .armor_stand_wiggle
    }

    // Rest projects the first non-wobbling value: vanilla only applies the
    // setup rotation while `ArmorStandRenderState.wiggle < ArmorStand.WOBBLE_TIME`.
    assert_eq!(armor_stand_wiggle(&world, 0.0), 5.0);

    assert!(
        world.apply_entity_event(bbb_protocol::packets::EntityEvent {
            entity_id: 5,
            event_id: 32,
        })
    );
    assert_eq!(armor_stand_wiggle(&world, 0.0), 0.0);
    assert_eq!(armor_stand_wiggle(&world, 0.5), 0.5);

    world.advance_entity_client_animations(4);
    assert_eq!(armor_stand_wiggle(&world, 0.75), 4.75);

    world.advance_entity_client_animations(1);
    assert_eq!(armor_stand_wiggle(&world, 0.0), 5.0);
}

#[test]
fn entity_model_instances_project_avatar_model_part_visibility_from_world() {
    let mut world = WorldStore::new();
    let player = protocol_add_entity(1550, VANILLA_ENTITY_TYPE_PLAYER_ID, [1.0, 64.0, -2.0]);
    let player_uuid = player.uuid;
    world.apply_add_entity(player);
    world.apply_add_entity(protocol_add_entity(
        830,
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        [3.0, 64.0, -2.0],
    ));
    let player_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK
            | PlayerModelPartVisibility::JACKET_MASK
            | PlayerModelPartVisibility::RIGHT_PANTS_MASK,
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 1550,
        values: vec![protocol_byte_data(
            AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
            player_parts.vanilla_mask() as i8,
        )],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![
                EntityModelInstance::player_with_skin(
                    1550,
                    [1.0, 64.0, -2.0],
                    0.0,
                    EntityPlayerSkin::Default(default_player_skin_for_profile_id(
                        player_uuid.as_u128(),
                    )),
                    player_parts,
                )
                .with_player_fall_flying_time_in_ticks(1.0),
                EntityModelInstance::player_with_parts(
                    830,
                    [3.0, 64.0, -2.0],
                    0.0,
                    false,
                    PlayerModelPartVisibility::from_vanilla_mask(
                        PlayerModelPartVisibility::ALL_MASK,
                    ),
                ),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_use_uuid_default_skin_without_player_info() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_uuid(
        1551,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        Uuid::nil(),
        [2.0, 64.0, -2.0],
    ));

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1551)
        .unwrap();

    assert_eq!(
        instance.kind,
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::SlimAlex),
            parts: PlayerModelPartVisibility::from_vanilla_mask(0),
        }
    );
}

#[test]
fn entity_model_instances_use_player_info_profile_skin_for_players() {
    const SLIM_TEXTURES_PROPERTY: &str = "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=";

    let mut world = WorldStore::new();
    let player = protocol_add_entity(1552, VANILLA_ENTITY_TYPE_PLAYER_ID, [1.0, 64.0, -2.0]);
    let profile_id = player.uuid;
    world.apply_add_entity(player);
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: vec![PlayerInfoEntry {
            profile_id,
            profile: Some(GameProfile {
                uuid: profile_id,
                name: "Alex".to_string(),
                properties: vec![GameProfileProperty {
                    name: "textures".to_string(),
                    value: SLIM_TEXTURES_PROPERTY.to_string(),
                    signature: Some("signature".to_string()),
                }],
            }),
            listed: true,
            latency: 0,
            game_mode: GameType::Survival,
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }],
    });
    let runtime = NativeItemRuntime::empty_for_test();

    let instance = entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1552)
        .unwrap();

    let EntityModelKind::Player { skin, parts } = instance.kind else {
        panic!("player entity should use the player model");
    };
    let EntityPlayerSkin::Dynamic(skin) = skin else {
        panic!("player info textures property should produce a dynamic player skin");
    };
    assert_eq!(skin.model, EntityPlayerSkinModel::Slim);
    assert_eq!(skin.status, EntityDynamicPlayerSkinStatus::Loading);
    assert_ne!(skin.handle, 0);
    assert_eq!(parts, PlayerModelPartVisibility::from_vanilla_mask(0));
    let cape = instance
        .render_state
        .player_cape_texture
        .expect("profile cape texture");
    let elytra = instance
        .render_state
        .player_elytra_texture
        .expect("profile elytra texture");
    assert_eq!(cape.kind, EntityDynamicPlayerTextureKind::Cape);
    assert_eq!(elytra.kind, EntityDynamicPlayerTextureKind::Elytra);
    assert_ne!(cape.handle, 0);
    assert_ne!(elytra.handle, 0);
    assert_ne!(cape.handle, elytra.handle);
}

#[test]
fn entity_model_instances_forward_player_extra_ears_from_world_source() {
    let mut world = WorldStore::new();
    let deadmau5_uuid = Uuid::from_u128(0xCCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC_CCCC);
    let mixed_case_uuid = Uuid::from_u128(0xDDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD_DDDD);
    world.apply_add_entity(protocol_add_entity_with_uuid(
        1554,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        deadmau5_uuid,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity_with_uuid(
        1555,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        mixed_case_uuid,
        [2.0, 64.0, -2.0],
    ));
    world.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: vec![
            PlayerInfoEntry {
                profile_id: deadmau5_uuid,
                profile: Some(GameProfile {
                    uuid: deadmau5_uuid,
                    name: "deadmau5".to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: GameType::Survival,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            },
            PlayerInfoEntry {
                profile_id: mixed_case_uuid,
                profile: Some(GameProfile {
                    uuid: mixed_case_uuid,
                    name: "Deadmau5".to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: GameType::Survival,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            },
        ],
    });

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let deadmau5 = instances
        .iter()
        .find(|instance| instance.entity_id == 1554)
        .expect("deadmau5 player instance");
    let mixed_case = instances
        .iter()
        .find(|instance| instance.entity_id == 1555)
        .expect("mixed-case player instance");

    assert!(deadmau5.render_state.show_extra_ears);
    assert!(!mixed_case.render_state.show_extra_ears);
}

#[test]
fn entity_model_instances_forward_player_cape_cloak_state() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        1553,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 1553,
        position: Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: true,
    }));
    world.advance_entity_client_animations(1);
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 1553,
        position: Vec3d {
            x: 0.0,
            y: 65.0,
            z: 1.0,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: true,
    }));
    world.advance_entity_client_animations(1);

    let source = world
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 1553)
        .unwrap();
    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1553)
        .unwrap();

    assert_eq!(
        (
            instance.render_state.player_cape_flap,
            instance.render_state.player_cape_lean,
            instance.render_state.player_cape_lean2,
        ),
        (
            source.player_cape_flap,
            source.player_cape_lean,
            source.player_cape_lean2,
        )
    );
    assert_eq!(instance.render_state.player_cape_flap, -6.0);
    assert_eq!(instance.render_state.player_cape_lean, 74.25);
    assert_eq!(instance.render_state.player_cape_lean2, 0.0);
}

#[test]
fn entity_model_instances_forward_player_shoulder_parrots_from_world_source() {
    // Vanilla Player shoulder parrots use generated `OPTIONAL_UNSIGNED_INT` accessors. World keeps
    // the raw `Parrot.Variant` ids; native maps them to renderer `ParrotModelVariant`s.

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        1556,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 1556,
        values: vec![
            protocol_optional_unsigned_int_data(
                VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
                Some(4),
            ),
            protocol_optional_unsigned_int_data(
                VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
                Some(1),
            ),
        ],
    }));

    let source = world
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 1556)
        .unwrap();
    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 1556)
        .unwrap();

    assert_eq!(source.player_left_shoulder_parrot, Some(4));
    assert_eq!(source.player_right_shoulder_parrot, Some(1));
    assert_eq!(
        instance.render_state.player_left_shoulder_parrot,
        Some(ParrotModelVariant::Gray)
    );
    assert_eq!(
        instance.render_state.player_right_shoulder_parrot,
        Some(ParrotModelVariant::Blue)
    );
}

#[test]
fn entity_model_instances_project_chest_equipment_layers() {
    const CHESTPLATE_ID: i32 = 0;
    const ELYTRA_ID: i32 = 1;
    let trim_texture = EntityDynamicTexture { handle: 42_4312 };
    let trim_lookup = ReadyArmorTrimLookup {
        asset_id: "minecraft:trims/entity/wings/sentry_iron",
        texture: trim_texture,
        missing_texture: None,
    };

    let registry: bbb_pack::ItemRegistryCatalog = serde_json::from_value(serde_json::json!({
        "resource_ids": [
            "minecraft:diamond_chestplate",
            "minecraft:elytra"
        ],
        "protocol_ids": {
            "minecraft:diamond_chestplate": CHESTPLATE_ID,
            "minecraft:elytra": ELYTRA_ID
        },
        "default_equipment_slots": {
            "minecraft:diamond_chestplate": "chest",
            "minecraft:elytra": "chest"
        },
        "humanoid_armor_assets": {
            "minecraft:diamond_chestplate": "diamond"
        },
        "equippable_assets": {
            "minecraft:diamond_chestplate": "diamond",
            "minecraft:elytra": "elytra"
        }
    }))
    .unwrap();
    let equipment_assets: bbb_pack::EquipmentAssetCatalog =
            serde_json::from_value(serde_json::json!({
                "assets": {
                    "minecraft:diamond": {
                        "layers": {
                            "humanoid": [
                                {
                                    "texture": "minecraft:diamond",
                                    "texture_location": "minecraft:textures/entity/equipment/humanoid/diamond.png",
                                    "use_player_texture": false
                                }
                            ]
                        }
                    },
                    "minecraft:elytra": {
                        "layers": {
                            "wings": [
                                {
                                    "texture": "minecraft:elytra",
                                    "texture_location": "minecraft:textures/entity/equipment/wings/elytra.png",
                                    "use_player_texture": true
                                }
                            ]
                        }
                    }
                }
            }))
            .unwrap();
    let runtime =
        NativeItemRuntime::for_test_with_registry_and_equipment_assets(registry, equipment_assets);

    let mut world = WorldStore::new();
    world.set_item_armor_materials(runtime.item_armor_materials_by_protocol_id());
    world.set_default_item_wings_equipment_assets(
        runtime.item_wings_equipment_assets_by_protocol_id(),
    );
    world.apply_add_entity(protocol_add_entity(
        1553,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        1554,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [3.0, 64.0, -2.0],
    ));
    let equip_with_patch = |entity_id: i32,
                            item_id: Option<i32>,
                            count: i32,
                            component_patch: DataComponentPatchSummary|
     -> SetEquipment {
        SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Chest,
                item: ItemStackSummary {
                    item_id,
                    count,
                    component_patch,
                },
            }],
        }
    };
    let equip = |entity_id: i32, item_id: Option<i32>, count: i32| {
        equip_with_patch(
            entity_id,
            item_id,
            count,
            DataComponentPatchSummary::default(),
        )
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_with_registry_at_partial_tick(
            world,
            Some(&runtime),
            0.0,
            Some(&trim_lookup),
        )
        .into_iter()
        .find(|instance| instance.entity_id == id)
        .unwrap()
        .render_state
    };

    let trimmed_foil = DataComponentPatchSummary {
        enchantments: vec![ItemEnchantmentSummary {
            holder_id: 12,
            level: 1,
        }],
        armor_trim_material_id: Some(1), // iron
        armor_trim_pattern_id: Some(0),  // sentry
        ..Default::default()
    };
    assert!(world.apply_set_equipment(equip_with_patch(
        1553,
        Some(ELYTRA_ID),
        1,
        trimmed_foil.clone(),
    )));
    let with_elytra = state(&world, 1553);
    assert!(with_elytra.chest_equipment_has_wings);
    assert!(!with_elytra.chest_equipment_has_humanoid);
    assert_eq!(with_elytra.chest_armor, None);
    assert!(!with_elytra.chest_armor_foil);
    assert!(with_elytra.chest_wings_foil);
    assert_eq!(
        with_elytra.chest_wings_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );
    assert_eq!(
        with_elytra.chest_wings_layer,
        Some(EntityEquipmentLayerTexture {
            texture: bbb_renderer::EntityModelTextureRef {
                path: "textures/entity/equipment/wings/elytra.png",
                size: [64, 32],
            },
            use_player_texture: true,
        })
    );

    assert!(world.apply_set_equipment(equip_with_patch(
        1553,
        Some(CHESTPLATE_ID),
        1,
        DataComponentPatchSummary {
            enchantments: vec![ItemEnchantmentSummary {
                holder_id: 12,
                level: 1,
            }],
            ..Default::default()
        }
    )));
    let with_chestplate = state(&world, 1553);
    assert!(!with_chestplate.chest_equipment_has_wings);
    assert!(with_chestplate.chest_equipment_has_humanoid);
    assert_eq!(
        with_chestplate.chest_armor,
        Some(EntityArmorMaterial::Diamond)
    );
    assert!(with_chestplate.chest_armor_foil);
    assert!(!with_chestplate.chest_wings_foil);
    assert_eq!(with_chestplate.chest_wings_trim, None);
    assert_eq!(with_chestplate.chest_wings_layer, None);

    assert!(world.apply_set_equipment(equip(1553, None, 0)));
    let empty_chest = state(&world, 1553);
    assert!(!empty_chest.chest_equipment_has_wings);
    assert!(!empty_chest.chest_equipment_has_humanoid);
    assert_eq!(empty_chest.chest_armor, None);
    assert!(!empty_chest.chest_armor_foil);
    assert!(!empty_chest.chest_wings_foil);
    assert_eq!(empty_chest.chest_wings_trim, None);
    assert_eq!(empty_chest.chest_wings_layer, None);

    assert!(world.apply_set_equipment(equip_with_patch(1554, Some(ELYTRA_ID), 1, trimmed_foil,)));
    let zombie = state(&world, 1554);
    assert!(zombie.chest_equipment_has_wings);
    assert!(!zombie.chest_equipment_has_humanoid);
    assert_eq!(zombie.chest_armor, None);
    assert!(!zombie.chest_armor_foil);
    assert!(zombie.chest_wings_foil);
    assert_eq!(
        zombie.chest_wings_trim,
        Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        })
    );
    assert_eq!(
        zombie.chest_wings_layer,
        Some(EntityEquipmentLayerTexture {
            texture: bbb_renderer::EntityModelTextureRef {
                path: "textures/entity/equipment/wings/elytra.png",
                size: [64, 32],
            },
            use_player_texture: true,
        })
    );
}

#[test]
fn entity_model_instances_project_slime_and_magma_cube_size() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        117,
        VANILLA_ENTITY_TYPE_SLIME_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        80,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 117,
        values: vec![slime_size_data(VANILLA_ENTITY_TYPE_SLIME_ID, 4)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 80,
        values: vec![slime_size_data(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, 3)],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![
                EntityModelInstance::slime(117, [1.0, 64.0, -2.0], 0.0, 4),
                EntityModelInstance::magma_cube(80, [3.0, 64.0, -2.0], 0.0, 3),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_project_age_in_ticks_from_world_age_and_partial_tick() {
    // Vanilla `EntityRenderState.ageInTicks = entity.tickCount + partialTick`: the world
    // tracks the per-entity client-animation age and the scene lerps it with the partial
    // tick. After 7 client ticks at partial 0.25 the projected age is 7.25.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        70,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(7);

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

    assert_eq!(instances.len(), 1);
    assert!(
        (instances[0].render_state.age_in_ticks - 7.25).abs() < 1e-6,
        "{}",
        instances[0].render_state.age_in_ticks
    );
}

#[test]
fn entity_model_instances_forward_exact_age_scale_separately_from_scale_attribute() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        71,
        VANILLA_ENTITY_TYPE_GOAT_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 71,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_GOAT_ID, true)],
    }));
    assert!(world.apply_update_attributes(UpdateAttributes {
        entity_id: 71,
        attributes: vec![AttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));

    let source = world
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 71)
        .expect("baby goat model source");
    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 71)
        .expect("baby goat model instance");

    assert_eq!((source.scale, source.age_scale), (2.0, 0.55));
    assert_eq!(instance.render_state.scale, source.scale);
    assert_eq!(instance.render_state.age_scale, source.age_scale);
}

#[test]
fn entity_model_instances_do_not_drop_pick_target_entity_types() {
    let mut world = WorldStore::new();
    for (index, entity_type_id) in VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS
        .iter()
        .copied()
        .enumerate()
    {
        world.apply_add_entity(protocol_add_entity(
            1000 + index as i32,
            entity_type_id,
            [index as f64 * 2.0, 64.0, 0.0],
        ));
    }

    let sources = world.entity_model_sources_at_partial_tick(1.0);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        sources.len(),
        VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS.len()
    );
    assert_eq!(instances.len(), sources.len());
}

#[test]
fn entity_model_kind_matches_every_fixed_renderer_presentation_owner() {
    use bbb_world::VanillaEntityRendererPresentationOwner as Owner;

    for entry in VANILLA_ENTITY_TYPES {
        let owner = bbb_world::vanilla_entity_renderer_presentation_owner(entry.id);
        let kind = entity_model_kind(entry.id, &[]);
        match owner {
            Owner::AvatarModel => assert!(
                matches!(kind, EntityModelKind::Player { .. }),
                "{} avatar renderer projected {kind:?}",
                entry.resource_id
            ),
            Owner::EntityModel => assert!(
                !matches!(
                    kind,
                    EntityModelKind::NoRender
                        | EntityModelKind::Humanoid { .. }
                        | EntityModelKind::Quadruped { .. }
                        | EntityModelKind::Placeholder { .. }
                ),
                "{} model renderer projected unowned/raw fallback {kind:?}",
                entry.resource_id
            ),
            Owner::Noop
            | Owner::ThrownItem
            | Owner::BlockAttachment
            | Owner::FireworkItem
            | Owner::ItemFrame
            | Owner::ItemCluster
            | Owner::Lightning
            | Owner::Display
            | Owner::Painting => assert_eq!(
                kind,
                EntityModelKind::NoRender,
                "{} separately owned renderer projected {kind:?}",
                entry.resource_id
            ),
            Owner::Unknown => panic!("{} has no renderer presentation owner", entry.resource_id),
        }
    }
}

#[test]
fn painting_without_ready_variant_or_atlas_suppresses_raw_fallback_geometry() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:painting_variant",
        0,
        vec![RegistryPacketEntry::stub("minecraft:kebab")],
    );
    let mut spawn = protocol_add_entity(9300, VANILLA_ENTITY_TYPE_PAINTING_ID, [1.0, 64.0, -2.0]);
    spawn.data = 3;
    world.apply_add_entity(spawn);
    let data_id = bbb_world::vanilla_entity_metadata_painting_variant_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PAINTING_ID,
    )
    .expect("generated Painting variant accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 9300,
        values: vec![EntityDataValue {
            data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
            value: bbb_protocol::packets::EntityDataValueKind::PaintingVariant(
                bbb_protocol::packets::PaintingVariantData {
                    registry: Some(
                        bbb_protocol::packets::EntityDataRegistryReference::unresolved(0),
                    ),
                    direct: None,
                },
            ),
        }],
    }));

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 9300)
        .expect("not-ready painting presentation source");
    assert_eq!(instance.kind, EntityModelKind::NoRender);
}

#[test]
fn primed_tnt_uses_block_attachment_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_TNT_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn falling_block_uses_block_attachment_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn firework_rocket_uses_item_billboard_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn item_entity_uses_item_model_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ITEM_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn ominous_item_spawner_uses_item_cluster_renderer_not_placeholder_bounds() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn lightning_bolt_uses_weather_target_renderer_not_entity_placeholder() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_base_zombie_and_skeleton() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_ID, &[]),
        EntityModelKind::Zombie { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            &[protocol_zombie_baby_data(
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                true
            )]
        ),
        EntityModelKind::Zombie { baby: true }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SKELETON_ID, &[]),
        EntityModelKind::Skeleton
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_HUSK_ID, &[]),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HUSK_ID,
            &[protocol_zombie_baby_data(VANILLA_ENTITY_TYPE_HUSK_ID, true)]
        ),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_DROWNED_ID, &[]),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_DROWNED_ID,
            &[protocol_zombie_baby_data(
                VANILLA_ENTITY_TYPE_DROWNED_ID,
                true
            )]
        ),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, &[]),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            &[protocol_zombie_baby_data(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                true,
            )]
        ),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_ID, &[]),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            &[protocol_bool_data(piglin_baby_data_id(), true)]
        ),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, &[]),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID, &[]),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
            &[protocol_zombie_baby_data(
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
                true,
            )]
        ),
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_STRAY_ID, &[]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PARCHED_ID, &[]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, &[]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BOGGED_ID, &[]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: false }
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BOGGED_ID, &[bogged_sheared_data(true)]),
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: true }
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_armor_stands() {
    let accessors = armor_stand_metadata_accessors();
    let default_pose = armor_stand_default_model_pose();
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, &[]),
        EntityModelKind::ArmorStand {
            small: false,
            marker: false,
            show_arms: false,
            show_base_plate: true,
            pose: default_pose,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            &[protocol_byte_data(
                accessors.client_flags_data_id,
                accessors.small_mask
                    | accessors.show_arms_mask
                    | accessors.no_baseplate_mask
                    | accessors.marker_mask,
            )],
        ),
        EntityModelKind::ArmorStand {
            small: true,
            marker: true,
            show_arms: true,
            show_base_plate: false,
            pose: default_pose,
        }
    );

    let mut pose = default_pose;
    pose.head = [1.0, 2.0, 3.0];
    pose.right_arm = [-20.0, 5.0, 10.0];
    pose.left_leg = [4.0, 5.0, 6.0];
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            &[
                protocol_rotations_data(accessors.head_pose_data_id, pose.head),
                protocol_rotations_data(accessors.right_arm_pose_data_id, pose.right_arm),
                protocol_rotations_data(accessors.left_leg_pose_data_id, pose.left_leg),
            ],
        ),
        EntityModelKind::ArmorStand {
            small: false,
            marker: false,
            show_arms: false,
            show_base_plate: true,
            pose,
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_slime_and_magma_cube() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SLIME_ID, &[]),
        EntityModelKind::Slime { size: 1 }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SLIME_ID,
            &[slime_size_data(VANILLA_ENTITY_TYPE_SLIME_ID, 4)]
        ),
        EntityModelKind::Slime { size: 4 }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, &[]),
        EntityModelKind::MagmaCube { size: 1 }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
            &[slime_size_data(VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, 3)]
        ),
        EntityModelKind::MagmaCube { size: 3 }
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_ghast() {
    // The ghast was a placeholder render box; it now resolves to the real model. The `charging` flag
    // (vanilla `Ghast.DATA_IS_CHARGING`, BOOLEAN at index 16) swaps to the shooting texture.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_GHAST_ID, &[]),
        EntityModelKind::Ghast { charging: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_GHAST_ID,
            &[protocol_bool_data(ghast_charging_data_id(), true)]
        ),
        EntityModelKind::Ghast { charging: true }
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_happy_ghast() {
    // The happy ghast was a placeholder render box; it now resolves to the real model.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, &[]),
        EntityModelKind::HappyGhast { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, true)]
        ),
        EntityModelKind::HappyGhast { baby: true }
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_blaze() {
    // The blaze was a placeholder render box; it now resolves to the real model.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BLAZE_ID, &[]),
        EntityModelKind::Blaze
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_endermite() {
    // The endermite was a placeholder render box; it now resolves to the real model.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ENDERMITE_ID, &[]),
        EntityModelKind::Endermite
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_silverfish() {
    // The silverfish was a placeholder render box; it now resolves to the real model.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SILVERFISH_ID, &[]),
        EntityModelKind::Silverfish
    );
}

#[test]
fn entity_model_kind_projects_pufferfish_puff_state_from_data() {
    // The pufferfish was a placeholder render box; it now resolves to the real model and
    // projects its synced `PUFF_STATE` (whose generated default is deflated).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PUFFERFISH_ID, &[]),
        EntityModelKind::Pufferfish { puff_state: 0 }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
            &[pufferfish_puff_state_data(2)]
        ),
        EntityModelKind::Pufferfish { puff_state: 2 }
    );
}

#[test]
fn entity_model_kind_maps_vex_to_real_model() {
    // The vex resolves to the real `VexModel`. Its idle wing flap / arm bob / head look read the
    // projected age and look angles. The generated `Vex.DATA_FLAGS_ID` and named charging mask
    // project to `charging`, which vanilla `VexRenderer.getTextureLocation` swaps to
    // `vex_charging.png`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_VEX_ID, &[]),
        EntityModelKind::Vex { charging: false }
    );
    let (_, default, charging_mask) =
        bbb_world::vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VEX_ID,
        )
        .expect("generated Vex flags accessor must resolve");
    assert_eq!(default, 0);
    let charging_values = vec![vex_flags_data(charging_mask)];
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_VEX_ID, &charging_values),
        EntityModelKind::Vex { charging: true }
    );
}

#[test]
fn entity_model_kind_maps_dragon_fireball_to_billboard_quad() {
    // The dragon fireball was a placeholder render box; it now resolves to the camera-facing
    // `DragonFireballRenderer` full-sheet quad. Its constant block light 15 already rides the
    // generated static-full-bright renderer catalog.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID, &[]),
        EntityModelKind::DragonFireball
    );
    assert!(
        bbb_world::vanilla_entity_renderer_has_static_full_block_light(
            VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID
        )
    );
}

#[test]
fn entity_model_kind_maps_experience_orb_to_billboard_quad() {
    // The orb was a placeholder render box; it now resolves to the camera-facing
    // `ExperienceOrbRenderer` icon quad. The generated `ExperienceOrb.DATA_VALUE` projects
    // through vanilla `ExperienceOrb.getIcon()` thresholds to the 4×4 sheet cell.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID, &[]),
        EntityModelKind::ExperienceOrb { icon: 0 }
    );
    let (value_data_id, default) =
        bbb_world::vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        )
        .expect("generated ExperienceOrb value accessor must resolve");
    assert_eq!(default, 0);
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            &[protocol_int_data(value_data_id, 149)]
        ),
        EntityModelKind::ExperienceOrb { icon: 6 }
    );
}

#[test]
fn entity_model_kind_maps_allay_to_real_model() {
    // The allay was a placeholder render box; it now resolves to the real `AllayModel`. Its
    // idle/flying wing flap, arm bob, head look, vertical bob, and dance pose read the projected age,
    // walk animation, look angles, and dancing state. Held-item rendering is a separate equipment
    // concern.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ALLAY_ID, &[]),
        EntityModelKind::Allay
    );
}

#[test]
fn entity_model_kind_maps_bat_to_real_model() {
    // The bat was a placeholder render box; it now resolves to the real `BatModel`, the
    // first keyframe-animated entity. Its looping `BAT_FLYING` wing flap and the synced
    // `isResting` / `BAT_RESTING` branch read projected render state.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BAT_ID, &[]),
        EntityModelKind::Bat
    );
}

#[test]
fn entity_model_kind_projects_bee_baby_from_data() {
    // The bee was a placeholder render box; it now resolves to the real `AdultBeeModel` /
    // `BabyBeeModel`, keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default adult).
    // The procedural airborne flap / bob reads the projected age and ground state.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BEE_ID, &[]),
        EntityModelKind::Bee {
            baby: false,
            angry: false,
            has_nectar: false,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_BEE_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_BEE_ID, true)]
        ),
        EntityModelKind::Bee {
            baby: true,
            angry: false,
            has_nectar: false,
        }
    );
}

#[test]
fn entity_model_kind_projects_bee_nectar_and_angry_texture_flags() {
    // Vanilla `BeeRenderer.getTextureLocation` keys on `hasNectar` (the synced
    // `DATA_FLAGS_ID & 8`, index 18) and `isAngry` (the synced `DATA_ANGER_END_TIME`, index 19,
    // in the future). A bee carrying nectar swaps to the `*_nectar*` texture.
    let accessors = bee_metadata_accessors();
    assert_eq!(
        entity_model_kind_with_time_and_registries(
            VANILLA_ENTITY_TYPE_BEE_ID,
            &[protocol_byte_data(
                accessors.flags_data_id,
                accessors.has_nectar_mask,
            )],
            0.0,
            0,
            None,
        ),
        EntityModelKind::Bee {
            baby: false,
            angry: false,
            has_nectar: true,
        }
    );
    // An anger-end time past the current game time makes the bee angry (and the roll/stung
    // bits in the flags byte do not flip `hasNectar`).
    assert_eq!(
        entity_model_kind_with_time_and_registries(
            VANILLA_ENTITY_TYPE_BEE_ID,
            &[protocol_long_data(accessors.anger_end_time_data_id, 100)],
            0.0,
            10,
            None,
        ),
        EntityModelKind::Bee {
            baby: false,
            angry: true,
            has_nectar: false,
        }
    );
}

#[test]
fn entity_model_kind_projects_dolphin_baby_from_data() {
    // The dolphin was a placeholder render box; it now resolves to the real `DolphinModel`,
    // keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default adult). Its swim body
    // tilt / tail wave reads the projected `isMoving` (the synced velocity); the held-item carry
    // layer remains a separate equipment concern.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_DOLPHIN_ID, &[]),
        EntityModelKind::Dolphin { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_DOLPHIN_ID, true)]
        ),
        EntityModelKind::Dolphin { baby: true }
    );
}

#[test]
fn entity_model_kind_maps_guardian_and_elder_guardian_to_real_model() {
    // Both guardians were placeholder render boxes; they now resolve to the real
    // `GuardianModel`. The variant follows the registered renderer class (the elder is the same
    // mesh scaled 2.35×), with no synced data. Procedural spike withdrawal, eye tracking, tail sway,
    // and the attack beam are carried by the separate world-to-render-state projection.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_GUARDIAN_ID, &[]),
        EntityModelKind::Guardian { elder: false }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, &[]),
        EntityModelKind::Guardian { elder: true }
    );
}

#[test]
fn guardian_eye_tracking_projection_preserves_world_space_render_inputs() {
    let source = WorldGuardianEyeTrackingSource {
        eye_position: [1.0, 2.0, 3.0],
        look_direction: [-0.25, 0.5, 0.75],
        look_at_position: [4.0, 5.0, 6.0],
    };
    assert_eq!(
        guardian_eye_tracking(Some(source)),
        Some(GuardianEyeTrackingRenderState {
            eye_position: source.eye_position,
            look_direction: source.look_direction,
            look_at_position: source.look_at_position,
        })
    );
    assert_eq!(guardian_eye_tracking(None), None);
}

#[test]
fn guardian_instance_carries_world_projected_camera_eye_target() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_rotation(
        330,
        VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        [0.0, 64.0, 0.0],
        0.0,
        0.0,
        0.0,
    ));
    world.apply_add_entity(protocol_add_entity(
        331,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [5.0, 66.0, 0.0],
    ));
    assert!(world.apply_set_camera(SetCamera { camera_id: 331 }));

    let projected = world
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 330)
        .unwrap()
        .guardian_eye_tracking;
    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 330)
        .unwrap();

    assert!(projected.is_some());
    assert_eq!(
        instance.render_state.guardian_eye_tracking,
        guardian_eye_tracking(projected)
    );
}

#[test]
fn entity_model_kind_maps_creaking_to_real_model() {
    // The data-only selector resolves the real `CreakingModel` and uses synced `IS_ACTIVE` for the
    // ordinary emissive-eye state. `entity_model_instance` overrides this with the world-projected
    // client flicker during teardown, when the exact walk/combat/death states are already projected.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CREAKING_ID, &[]),
        EntityModelKind::Creaking {
            eyes_glowing: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            &[protocol_bool_data(creaking_active_data_id(), true)]
        ),
        EntityModelKind::Creaking { eyes_glowing: true }
    );
}

#[test]
fn entity_model_kind_maps_frog_to_real_model() {
    // The frog resolves to the real `FrogModel` at its rest pose, textured by temperature
    // variant. With no synced `DATA_VARIANT_ID` it defaults to TEMPERATE; otherwise the
    // `Holder<FrogVariant>` registry id selects the colour. Without a synced `frog_variant`
    // registry, the static `FrogVariants.bootstrap` order (TEMPERATE=0, WARM=1, COLD=2) applies.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FROG_ID, &[]),
        EntityModelKind::Frog {
            variant: FrogModelVariant::Temperate
        }
    );
    for (id, variant) in [
        (0, FrogModelVariant::Temperate),
        (1, FrogModelVariant::Warm),
        (2, FrogModelVariant::Cold),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_FROG_ID,
                &[protocol_frog_variant_data(id)]
            ),
            EntityModelKind::Frog { variant }
        );
    }

    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:frog_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:warm"),
            RegistryPacketEntry::stub("minecraft:temperate"),
        ],
    );
    assert_eq!(
        entity_model_kind_with_registries(
            VANILLA_ENTITY_TYPE_FROG_ID,
            &[bind_protocol_registry_data_to_resource(
                protocol_frog_variant_data(0),
                "minecraft:cold",
            )],
            Some(&world),
        ),
        EntityModelKind::Frog {
            variant: FrogModelVariant::Cold
        }
    );
}

#[test]
fn entity_model_kind_maps_breeze_to_real_model() {
    // The breeze was a placeholder render box; it now resolves to the real `BreezeModel`, the
    // second keyframe entity (and the first to use CATMULLROM). Its looping `IDLE` head bob / rod
    // spin, triggered actions, typed wind model, and eyes layer all read projected render state.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BREEZE_ID, &[]),
        EntityModelKind::Breeze
    );
}

#[test]
fn entity_model_kind_projects_turtle_baby_from_data() {
    // The turtle was a placeholder render box; it now resolves to the real
    // `AdultTurtleModel` / `BabyTurtleModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
    // (index 16, default adult). The head look, land-walk / water-swim branch, egg-laying amplitude,
    // and egg-belly shell read projected look, walk, environment, and synced turtle state.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_TURTLE_ID, &[]),
        EntityModelKind::Turtle { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_TURTLE_ID, true)]
        ),
        EntityModelKind::Turtle { baby: true }
    );
}

#[test]
fn entity_model_kind_projects_strider_baby_and_cold_from_data() {
    // The strider previously fell back to the horse quadruped; it now resolves to the real
    // `AdultStriderModel` / `BabyStriderModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
    // (index 16, default adult). The `cold` flag is the synced `DATA_SUFFOCATING` (19), swapping
    // to the `strider_cold` texture; ridden pose and saddle layer are generic render-state flags.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_STRIDER_ID, &[]),
        EntityModelKind::Strider {
            baby: false,
            cold: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_STRIDER_ID, true)]
        ),
        EntityModelKind::Strider {
            baby: true,
            cold: false
        }
    );
    // A suffocating strider carries the cold texture.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            &[strider_suffocating_data(
                VANILLA_ENTITY_TYPE_STRIDER_ID,
                true,
            )]
        ),
        EntityModelKind::Strider {
            baby: false,
            cold: true
        }
    );
}

#[test]
fn entity_model_kind_maps_cod_to_real_model() {
    // The cod was a placeholder render box; it now resolves to the real `CodModel`. Its
    // tail sway / out-of-water flop read the projected `in_water` render-state flag.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COD_ID, &[]),
        EntityModelKind::Cod
    );
}

#[test]
fn entity_model_kind_projects_salmon_size_from_variant_data() {
    // The salmon was a placeholder render box; it now resolves to the real `SalmonModel`
    // and projects its synced `DATA_TYPE` size variant (`Salmon.Variant` ids
    // SMALL=0/MEDIUM=1/LARGE=2 clamped, with a generated MEDIUM default).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[]),
        EntityModelKind::Salmon {
            size: SalmonModelSize::Medium,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[salmon_variant_data(0)]),
        EntityModelKind::Salmon {
            size: SalmonModelSize::Small,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[salmon_variant_data(2)]),
        EntityModelKind::Salmon {
            size: SalmonModelSize::Large,
        }
    );
    // Out-of-range ids clamp to the large body, matching `ByIdMap.continuous(CLAMP)`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SALMON_ID, &[salmon_variant_data(9)]),
        EntityModelKind::Salmon {
            size: SalmonModelSize::Large,
        }
    );
}

#[test]
fn entity_model_kind_projects_tropical_fish_shape_from_packed_variant() {
    // The tropical fish was a placeholder render box; it now resolves to the real model
    // and decodes the body shape from the synced packed variant (`DATA_ID_TYPE_VARIANT`,
    // index 17). The default 0 (KOB/white/white) is the small body; a LARGE-base pattern
    // selects the flopper body.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, &[]),
        EntityModelKind::TropicalFish {
            shape: TropicalFishModelShape::Small,
            base_color: EntityDyeColor::White,
            // pattern bits = 0 → Pattern.byId(0) = KOB; pattern color = (0 >> 24) = WHITE.
            pattern: TropicalFishPattern::Kob,
            pattern_color: EntityDyeColor::White,
        }
    );
    // FLOPPER (LARGE base, index 0) with arbitrary base/pattern color bytes → large body.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
            &[tropical_fish_variant_data(0x0405_0001)]
        ),
        EntityModelKind::TropicalFish {
            shape: TropicalFishModelShape::Large,
            // base byte = (0x0405_0001 >> 16) & 0xFF = 0x05 → DyeColor.byId(5) = LIME.
            base_color: EntityDyeColor::Lime,
            // pattern bits = 0x0405_0001 & 0xFFFF = 1 → Pattern.byId(1) = FLOPPER.
            pattern: TropicalFishPattern::Flopper,
            // pattern color = (0x0405_0001 >> 24) & 0xFF = 0x04 → DyeColor.byId(4) = YELLOW.
            pattern_color: EntityDyeColor::Yellow,
        }
    );
    // SPOTTY (SMALL base, index 5 → 0x0500) stays the small body.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
            &[tropical_fish_variant_data(0x0500)]
        ),
        EntityModelKind::TropicalFish {
            shape: TropicalFishModelShape::Small,
            // base byte = (0x0500 >> 16) & 0xFF = 0 → DyeColor.byId(0) = WHITE.
            base_color: EntityDyeColor::White,
            // pattern bits = 0x0500 = 1280 → Pattern.byId(1280) = SPOTTY (small, index 5).
            pattern: TropicalFishPattern::Spotty,
            pattern_color: EntityDyeColor::White,
        }
    );
}

#[test]
fn entity_model_kind_projects_tropical_fish_base_color_from_packed_variant() {
    // Vanilla `TropicalFish.getBaseColor(packedVariant) = DyeColor.byId(packedVariant >> 16
    // & 0xFF)`, surfaced by `TropicalFishRenderer.getModelTint = state.baseColor`. Each dye
    // id occupies bits 16..24 of the packed variant; the low 16 bits (pattern) and high 8
    // bits (pattern color) must not bleed into the base color.
    let base_color_of = |packed: i32| match entity_model_kind(
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        &[tropical_fish_variant_data(packed)],
    ) {
        EntityModelKind::TropicalFish { base_color, .. } => base_color,
        other => panic!("expected tropical fish, got {other:?}"),
    };
    // id 0 → WHITE, id 11 → BLUE, id 15 → BLACK, with noise in the other byte ranges.
    assert_eq!(
        base_color_of(0x00FF_FFFF & !0x00FF_0000),
        EntityDyeColor::White
    );
    assert_eq!(base_color_of(0x000B_0000), EntityDyeColor::Blue);
    assert_eq!(base_color_of(0xFF0F_FFFFu32 as i32), EntityDyeColor::Black);
    // Out-of-range base byte (16) falls back to WHITE like `DyeColor.byId` (ZERO strategy).
    assert_eq!(base_color_of(0x0010_0000), EntityDyeColor::White);
}

#[test]
fn entity_model_kind_projects_tropical_fish_pattern_and_pattern_color_from_packed_variant() {
    // Vanilla `getPattern(packed) = Pattern.byId(packed & 0xFFFF)` (sparse, default KOB) and
    // `getPatternColor(packed) = DyeColor.byId(packed >> 24 & 0xFF)`. The pattern occupies the
    // low 16 bits and the pattern color the top byte; the base color byte (bits 16..24) must
    // not bleed into either.
    let decode = |packed: i32| match entity_model_kind(
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        &[tropical_fish_variant_data(packed)],
    ) {
        EntityModelKind::TropicalFish {
            shape,
            pattern,
            pattern_color,
            ..
        } => (shape, pattern, pattern_color),
        other => panic!("expected tropical fish, got {other:?}"),
    };

    // All twelve patterns map from their `base.id | index << 8` packed id.
    for (packed_pattern, expected, shape) in [
        (
            0x0000,
            TropicalFishPattern::Kob,
            TropicalFishModelShape::Small,
        ),
        (
            0x0100,
            TropicalFishPattern::Sunstreak,
            TropicalFishModelShape::Small,
        ),
        (
            0x0200,
            TropicalFishPattern::Snooper,
            TropicalFishModelShape::Small,
        ),
        (
            0x0300,
            TropicalFishPattern::Dasher,
            TropicalFishModelShape::Small,
        ),
        (
            0x0400,
            TropicalFishPattern::Brinely,
            TropicalFishModelShape::Small,
        ),
        (
            0x0500,
            TropicalFishPattern::Spotty,
            TropicalFishModelShape::Small,
        ),
        (
            0x0001,
            TropicalFishPattern::Flopper,
            TropicalFishModelShape::Large,
        ),
        (
            0x0101,
            TropicalFishPattern::Stripey,
            TropicalFishModelShape::Large,
        ),
        (
            0x0201,
            TropicalFishPattern::Glitter,
            TropicalFishModelShape::Large,
        ),
        (
            0x0301,
            TropicalFishPattern::Blockfish,
            TropicalFishModelShape::Large,
        ),
        (
            0x0401,
            TropicalFishPattern::Betty,
            TropicalFishModelShape::Large,
        ),
        (
            0x0501,
            TropicalFishPattern::Clayfish,
            TropicalFishModelShape::Large,
        ),
    ] {
        // Mix in a non-zero base color byte (GRAY = 7) and pattern color byte (BLUE = 11) to
        // prove neither disturbs the pattern decode.
        let packed = packed_pattern | (0x07 << 16) | (0x0B << 24);
        let (got_shape, got_pattern, got_pattern_color) = decode(packed);
        assert_eq!(got_pattern, expected);
        assert_eq!(got_shape, shape);
        assert_eq!(got_shape, expected.shape(), "shape mirrors pattern.shape()");
        assert_eq!(got_pattern_color, EntityDyeColor::Blue);
    }

    // Unknown pattern id falls back to KOB (small body) like `ByIdMap.sparse(..., KOB)`.
    let (shape, pattern, _) = decode(0x00AB);
    assert_eq!(pattern, TropicalFishPattern::Kob);
    assert_eq!(shape, TropicalFishModelShape::Small);
}

#[test]
fn entity_model_kind_projects_squid_glow_and_baby_from_data() {
    // The squid and glow squid were placeholder render boxes; they now resolve to the
    // real `SquidModel`. The glow variant follows the registered renderer class and the baby
    // flag uses the generated inherited `AgeableMob.DATA_BABY_ID` accessor. The
    // tentacle sweep / body tilt are projected by the world-side squid animation accumulator.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SQUID_ID, &[]),
        EntityModelKind::Squid {
            glow: false,
            baby: false,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, &[]),
        EntityModelKind::Squid {
            glow: true,
            baby: false,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SQUID_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_SQUID_ID, true)]
        ),
        EntityModelKind::Squid {
            glow: false,
            baby: true,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, true)]
        ),
        EntityModelKind::Squid {
            glow: true,
            baby: true,
        }
    );
}

#[test]
fn entity_model_kind_projects_phantom_size_from_data() {
    // The phantom was a placeholder render box; it now resolves to the real model and
    // projects its synced `ID_SIZE` and generated default.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PHANTOM_ID, &[]),
        EntityModelKind::Phantom { size: 0 }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PHANTOM_ID, &[phantom_size_data(5)]),
        EntityModelKind::Phantom { size: 5 }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_base_cow_and_sheep() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[]),
        EntityModelKind::Cow {
            variant: CowModelVariant::Temperate,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_COW_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_COW_ID, true)]
        ),
        EntityModelKind::Cow {
            variant: CowModelVariant::Temperate,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SHEEP_ID, &[]),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_SHEEP_ID, true)]
        ),
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    // The mooshroom shares the cow body, so it renders through the dedicated `Mooshroom` model
    // (the real cow mesh) rather than the generic quadruped stand-in — adult and baby alike. The
    // default variant (no `DATA_TYPE`) is the vanilla `Red`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, &[]),
        EntityModelKind::Mooshroom {
            baby: false,
            variant: MooshroomVariant::Red,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_MOOSHROOM_ID, true)]
        ),
        EntityModelKind::Mooshroom {
            baby: true,
            variant: MooshroomVariant::Red,
        }
    );
    // The generated synced `MushroomCow.DATA_TYPE` accessor selects the brown coat (id 1;
    // `ByIdMap` CLAMP folds out-of-range ids to the nearest endpoint).
    for (id, variant) in [
        (-1, MooshroomVariant::Red),
        (1, MooshroomVariant::Brown),
        (2, MooshroomVariant::Brown),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
                &[mushroom_cow_variant_data(id)]
            ),
            EntityModelKind::Mooshroom {
                baby: false,
                variant,
            }
        );
    }
}

#[test]
fn entity_model_kind_uses_vanilla_sheep_wool_metadata() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SHEEP_ID, &[sheep_wool_data(14, true)]),
        EntityModelKind::Sheep {
            baby: false,
            sheared: true,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[
                sheep_wool_data(15, false),
                baby_data(VANILLA_ENTITY_TYPE_SHEEP_ID, true),
            ]
        ),
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::Black,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[sheep_wool_data(3, false), sheep_wool_data(5, true),]
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: true,
            wool_color: SheepWoolColor::Lime,
            jeb: false,
            age_ticks: 0.0,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[entity_invisible_data(), sheep_wool_data(14, false),]
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::Red,
            jeb: false,
            age_ticks: 0.0,
        }
    );
}

#[test]
fn entity_model_kind_projects_sheep_jeb_custom_name_and_age() {
    assert_eq!(
        entity_model_kind_with_time_and_registries(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[
                entity_custom_name_data(Some("jeb_")),
                sheep_wool_data(0, false),
            ],
            12.5,
            0,
            None,
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: true,
            age_ticks: 12.5,
        }
    );
    assert_eq!(
        entity_model_kind_with_time_and_registries(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[entity_custom_name_data(Some("Not jeb_"))],
            25.0,
            0,
            None,
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 25.0,
        }
    );
    assert_eq!(
        entity_model_kind_with_time_and_registries(
            VANILLA_ENTITY_TYPE_SHEEP_ID,
            &[entity_custom_name_data(None)],
            25.0,
            0,
            None,
        ),
        EntityModelKind::Sheep {
            baby: false,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 25.0,
        }
    );
}

#[test]
fn entity_model_instances_project_sheep_wool_metadata_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        111,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 111,
        values: vec![sheep_wool_data(14, true)],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![EntityModelInstance::sheep_render_state(
                111,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                true,
                SheepWoolColor::Red,
                false,
                false,
                1.0,
            )],
            1.0,
        )
    );
    // A visible entity (no invisible shared flag) projects `invisible == false`.
    assert!(!instances[0].render_state.invisible);
}

#[test]
fn entity_model_instances_project_sheep_jeb_custom_name_and_age_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        112,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 112,
        values: vec![entity_custom_name_data(Some("jeb_"))],
    }));
    world.advance_entity_client_animations(12);

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);

    assert_eq!(
        instances,
        aged(
            vec![EntityModelInstance::sheep_render_state(
                112,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                false,
                SheepWoolColor::White,
                false,
                true,
                12.5,
            )],
            12.5,
        )
    );
}

#[test]
fn entity_model_instances_project_sheep_invisible_shared_flag_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        113,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 113,
        values: vec![entity_invisible_data(), sheep_wool_data(14, false),],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

    assert_eq!(
        instances,
        aged(
            vec![EntityModelInstance::sheep_render_state(
                113,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                false,
                SheepWoolColor::Red,
                true,
                false,
                0.25,
            )],
            0.25,
        )
    );
    // The shared invisible flag is now projected uniformly into the render state.
    assert!(instances[0].render_state.invisible);
    assert!(instances[0].render_state.invisible_to_player);
}

#[test]
fn entity_model_instances_project_spectator_visible_invisible_sheep_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        113,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 113,
        values: vec![entity_invisible_data(), sheep_wool_data(14, false),],
    }));
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 3,
        param: 3.0,
    });

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

    assert_eq!(instances.len(), 1);
    assert!(instances[0].render_state.invisible);
    assert!(!instances[0].render_state.invisible_to_player);
}

#[test]
fn entity_model_instances_project_glowing_shared_flag_from_world() {
    let mut world = WorldStore::new();
    let sheep = protocol_add_entity(113, VANILLA_ENTITY_TYPE_SHEEP_ID, [1.0, 64.0, -2.0]);
    let sheep_uuid = sheep.uuid;
    world.apply_add_entity(sheep);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 113,
        values: vec![entity_invisible_glowing_data(), sheep_wool_data(14, false),],
    }));
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "green".to_string(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Green"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Green,
            player_prefix: bbb_protocol::RichTextComponent::plain(""),
            player_suffix: bbb_protocol::RichTextComponent::plain(""),
        }),
        players: vec![sheep_uuid.to_string()],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);

    assert_eq!(instances.len(), 1);
    assert!(instances[0].render_state.invisible);
    assert!(instances[0].render_state.invisible_to_player);
    assert!(instances[0].render_state.appears_glowing);
    assert_eq!(instances[0].render_state.outline_color, 0xff55_ff55);
}

#[test]
fn entity_model_kind_uses_exact_models_for_goats() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_GOAT_ID, &[]),
        EntityModelKind::Goat {
            baby: false,
            left_horn: true,
            right_horn: true,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_GOAT_ID,
            &[
                baby_data(VANILLA_ENTITY_TYPE_GOAT_ID, true),
                goat_left_horn_data(false),
                goat_right_horn_data(true),
            ]
        ),
        EntityModelKind::Goat {
            baby: true,
            left_horn: false,
            right_horn: true,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_GOAT_ID,
            &[goat_left_horn_data(false), goat_right_horn_data(false),]
        ),
        EntityModelKind::Goat {
            baby: false,
            left_horn: false,
            right_horn: false,
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_hoglins_and_zoglins() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_HOGLIN_ID, &[]),
        EntityModelKind::Hoglin {
            family: HoglinModelFamily::Hoglin,
            baby: false,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HOGLIN_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_HOGLIN_ID, true)]
        ),
        EntityModelKind::Hoglin {
            family: HoglinModelFamily::Hoglin,
            baby: true,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOGLIN_ID, &[]),
        EntityModelKind::Hoglin {
            family: HoglinModelFamily::Zoglin,
            baby: false,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOGLIN_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_ZOGLIN_ID, true)]
        ),
        EntityModelKind::Hoglin {
            family: HoglinModelFamily::Zoglin,
            baby: true,
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_ravagers() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_RAVAGER_ID, &[]),
        EntityModelKind::Ravager
    );
}

#[test]
fn entity_model_kind_maps_sniffer_to_real_model() {
    // The sniffer was approximated by the cow quadruped model; it now resolves to the real
    // `SnifferModel` on the adult `ModelLayers.SNIFFER` or baby `ModelLayers.SNIFFER_BABY`
    // baked layer. Vanilla keys the baby renderer off `AgeableMob.DATA_BABY_ID` (index 16) and
    // uses `snifflet.png` while still constructing a `SnifferModel`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SNIFFER_ID, &[]),
        EntityModelKind::Sniffer { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_SNIFFER_ID, true)]
        ),
        EntityModelKind::Sniffer { baby: true }
    );
}

#[test]
fn entity_model_kind_maps_warden_to_real_model() {
    // The warden was a placeholder bounds box; it now resolves to the real `WardenModel`. The
    // head look, walk, idle wobble, tendril sway, triggered keyframes, and emissive layers are driven
    // by projected render state rather than this kind mapping. The kind mapping itself reads no
    // synced data.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WARDEN_ID, &[]),
        EntityModelKind::Warden
    );
}

#[test]
fn entity_model_kind_projects_armadillo_baby_from_data() {
    // The armadillo was a placeholder bounds box; it now resolves to the real
    // `AdultArmadilloModel` / `BabyArmadilloModel`, keyed off the synced `AgeableMob.DATA_BABY_ID`
    // (index 16, default adult), as in the vanilla `AgeableMobRenderer`. The clamped head look,
    // `applyWalk` leg sway, and the roll-out / roll-up / peek keyframe transitions are projected
    // separately from world animation state.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ARMADILLO_ID, &[]),
        EntityModelKind::Armadillo {
            baby: false,
            rolled_up: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_ARMADILLO_ID, true)]
        ),
        EntityModelKind::Armadillo {
            baby: true,
            rolled_up: false
        }
    );
}

#[test]
fn entity_model_kind_projects_armadillo_rolled_up_from_state() {
    // Vanilla `Armadillo.ARMADILLO_STATE` is an `ArmadilloState` enum. Only the steady SCARED state
    // is `shouldHideInShell` for every tick, so it maps to `rolled_up`; the tick-gated ROLLING /
    // UNROLLING transitions and IDLE stay not-rolled-up.
    let kind = |id: i32| {
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            &[protocol_armadillo_state_data(id)],
        )
    };
    assert_eq!(
        kind(ARMADILLO_STATE_SCARED_ID),
        EntityModelKind::Armadillo {
            baby: false,
            rolled_up: true
        }
    );
    for non_scared in [
        bbb_world::VANILLA_ARMADILLO_STATE_IDLE_ID,
        bbb_world::VANILLA_ARMADILLO_STATE_ROLLING_ID,
        bbb_world::VANILLA_ARMADILLO_STATE_UNROLLING_ID,
    ] {
        assert_eq!(
            kind(non_scared),
            EntityModelKind::Armadillo {
                baby: false,
                rolled_up: false
            },
            "state {non_scared} is not the steady SCARED ball"
        );
    }

    // A baby armadillo can roll up too — the state and the baby flag compose.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARMADILLO_ID,
            &[
                baby_data(VANILLA_ENTITY_TYPE_ARMADILLO_ID, true),
                protocol_armadillo_state_data(ARMADILLO_STATE_SCARED_ID),
            ]
        ),
        EntityModelKind::Armadillo {
            baby: true,
            rolled_up: true
        }
    );
}

#[test]
fn entity_model_kind_projects_axolotl_baby_from_data() {
    // The axolotl was a placeholder bounds box; it now resolves to the real `AdultAxolotlModel`
    // / `BabyAxolotlModel`, keyed off the synced `AgeableMob.DATA_BABY_ID` (index 16, default
    // adult), as in the vanilla `AgeableMobRenderer`, and textured by the `Axolotl.Variant`
    // colour read from the generated `DATA_VARIANT` accessor. Adult procedural factors and baby
    // AnimationState elapsed clocks project independently through the selected model.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_AXOLOTL_ID, &[]),
        EntityModelKind::Axolotl {
            baby: false,
            variant: AxolotlModelVariant::Lucy
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_AXOLOTL_ID, true)]
        ),
        EntityModelKind::Axolotl {
            baby: true,
            variant: AxolotlModelVariant::Lucy
        }
    );
    // `DATA_VARIANT` selects the colour via `Axolotl.Variant.byId`.
    for (id, variant) in [
        (-1, AxolotlModelVariant::Lucy),
        (0, AxolotlModelVariant::Lucy),
        (1, AxolotlModelVariant::Wild),
        (2, AxolotlModelVariant::Gold),
        (3, AxolotlModelVariant::Cyan),
        (4, AxolotlModelVariant::Blue),
        (5, AxolotlModelVariant::Lucy),
    ] {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_AXOLOTL_ID, &[axolotl_variant_data(id)]),
            EntityModelKind::Axolotl {
                baby: false,
                variant
            }
        );
    }
}

#[test]
fn entity_model_kind_maps_tadpole_to_real_model() {
    // The tadpole was a placeholder bounds box; it now resolves to the real `TadpoleModel`; its tail
    // yaw sway reads projected age. No synced data is needed here (the tadpole is an `AbstractFish`,
    // not an `AgeableMob`, so it carries no baby flag).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_TADPOLE_ID, &[]),
        EntityModelKind::Tadpole
    );
}

#[test]
fn entity_model_kind_maps_parrot_to_real_model() {
    // The parrot resolves to the real `ParrotModel` at its STANDING rest pose, textured per the
    // five `Parrot.Variant` colours read from the generated synced `DATA_VARIANT_ID` via
    // `Parrot.Variant.byId`. Projected render state drives the head look, pose branch, wing flap,
    // and dance; the player shoulder layer constructs the separate ON_SHOULDER pose.
    // No synced data → the vanilla DEFAULT (RED_BLUE).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PARROT_ID, &[]),
        EntityModelKind::Parrot {
            variant: ParrotModelVariant::RedBlue,
        }
    );
    // Each id selects its colour; out-of-range values clamp to the nearest endpoint.
    for (id, variant) in [
        (-99, ParrotModelVariant::RedBlue),
        (0, ParrotModelVariant::RedBlue),
        (1, ParrotModelVariant::Blue),
        (2, ParrotModelVariant::Green),
        (3, ParrotModelVariant::YellowBlue),
        (4, ParrotModelVariant::Gray),
        (99, ParrotModelVariant::Gray),
    ] {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_PARROT_ID, &[parrot_variant_data(id)]),
            EntityModelKind::Parrot { variant }
        );
    }
}

#[test]
fn entity_model_kind_maps_shulker_to_real_model() {
    let accessors = shulker_metadata_accessors();
    // The shulker resolves to the real `ShulkerModel` at its closed rest pose, textured by the
    // dye colour read from `DATA_COLOR_ID`. With no synced colour it defaults to the uncolored
    // texture (`None`). Projected render state drives peek open/close, head look, and attach-face
    // rotation.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SHULKER_ID, &[]),
        EntityModelKind::Shulker { color: None }
    );
    // The generated vanilla default is the uncolored shulker; 0..=15 select a dye.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SHULKER_ID,
            &[protocol_byte_data(
                accessors.color_data_id,
                accessors.color_default,
            )]
        ),
        EntityModelKind::Shulker { color: None }
    );
    for (id, color) in [
        (0, EntityDyeColor::White),
        (4, EntityDyeColor::Yellow),
        (11, EntityDyeColor::Blue),
        (15, EntityDyeColor::Black),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_SHULKER_ID,
                &[protocol_byte_data(accessors.color_data_id, id)]
            ),
            EntityModelKind::Shulker { color: Some(color) }
        );
    }
}

#[test]
fn entity_model_kind_maps_wither_to_real_model() {
    // The wither was a placeholder bounds box; it now resolves to the real `WitherBossModel` at
    // its bind rest pose. Projected render state drives the procedural ribcage/tail breathing sway,
    // head look, spawn transform, and invulnerable/powered layers; no synced data is read by the kind
    // mapping itself.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_ID, &[]),
        EntityModelKind::Wither
    );
}

#[test]
fn entity_model_kind_maps_giant_to_real_model() {
    // The giant was a placeholder bounds box; it now resolves to the real `GiantZombieModel`
    // (the humanoid zombie body layer scaled 6×). The head look and limb swing read the
    // projected look angles and walk animation; renderer/native layer paths cover the zombie
    // texture, armor, and held items. The giant is never a baby, so no baby flag is read.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_GIANT_ID, &[]),
        EntityModelKind::Giant
    );
}

#[test]
fn entity_model_kind_maps_end_crystal_to_real_model() {
    // The end crystal was a placeholder bounds box; it now resolves to the real `EndCrystalModel`
    // at its rest pose. The model kind itself reads no synced data; age, `showsBottom`, and the
    // optional `DATA_BEAM_TARGET` custom beam are projected into render state separately.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_END_CRYSTAL_ID, &[]),
        EntityModelKind::EndCrystal
    );
}

#[test]
fn entity_model_kind_maps_evoker_fangs_to_real_model() {
    // The evoker fangs were a placeholder bounds box; they now resolve to the real
    // `EvokerFangsModel` at the closed-jaw rest pose. Projected age drives the bite, base drop, and
    // emerge scale, while the kind mapping itself reads no synced data.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID, &[]),
        EntityModelKind::EvokerFangs
    );
}

#[test]
fn entity_model_kind_maps_leash_knot_to_real_model() {
    // The leash knot was a placeholder bounds box; it now resolves to the real `LeashKnotModel`.
    // The model has no animation, so the geometry is complete; no synced data is read.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, &[]),
        EntityModelKind::LeashKnot
    );
}

#[test]
fn entity_model_kind_maps_arrows_to_real_model() {
    // The arrow and spectral arrow were placeholder boxes; they now resolve to the real
    // `ArrowModel`, sharing one model but binding different images. A plain arrow is `Normal`; a
    // tipped arrow (`ID_EFFECT_COLOR` 11 > 0) is `Tipped`; the spectral arrow type is `Spectral`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ARROW_ID, &[]),
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Normal
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARROW_ID,
            &[protocol_arrow_effect_color_data(0x385dc6)]
        ),
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Tipped
        }
    );
    // A potionless arrow (`getColor()` returns the `-1` sentinel) is not tipped.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ARROW_ID,
            &[protocol_arrow_effect_color_data(-1)]
        ),
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Normal
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID, &[]),
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Spectral
        }
    );
}

#[test]
fn entity_model_instances_project_arrow_impact_shake() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        60,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        [1.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(1);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 60,
        values: vec![protocol_abstract_arrow_in_ground_data(
            VANILLA_ENTITY_TYPE_ARROW_ID,
            true,
        )],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
    let arrow = instances
        .iter()
        .find(|instance| instance.entity_id == 60)
        .expect("arrow instance");
    assert_eq!(
        arrow.kind,
        EntityModelKind::Arrow {
            texture: ArrowModelTexture::Normal
        }
    );
    assert_eq!(arrow.render_state.arrow_shake, 6.75);
}

#[test]
fn entity_model_kind_maps_trident_to_real_model() {
    // The thrown trident was a placeholder box; it now resolves to the real `TridentModel`. The
    // model has no animation, so the geometry is complete; the foil flag is projected onto render
    // state separately, so model kind selection still reads no synced data.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_TRIDENT_ID, &[]),
        EntityModelKind::Trident
    );
}

#[test]
fn entity_model_instances_project_thrown_trident_foil_flag() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        135,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        [1.0, 64.0, -2.0],
    ));

    let default_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    let trident = default_instances
        .iter()
        .find(|instance| instance.entity_id == 135)
        .expect("trident instance");
    assert_eq!(trident.kind, EntityModelKind::Trident);
    assert!(!trident.render_state.trident_foil);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 135,
        values: vec![protocol_thrown_trident_foil_data(true)],
    }));

    let foiled_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    let trident = foiled_instances
        .iter()
        .find(|instance| instance.entity_id == 135)
        .expect("foiled trident instance");
    assert_eq!(trident.kind, EntityModelKind::Trident);
    assert!(trident.render_state.trident_foil);
}

#[test]
fn entity_model_kind_skips_thrown_item_projectiles_for_the_billboard_layer() {
    // The thrown-item projectiles (vanilla `ThrownItemRenderer`) render as a camera-facing item
    // sprite via the item-entity billboard layer, so the 3D model scene draws nothing for them — the
    // model kind is `NoRender` rather than the former placeholder box.
    for (type_id, _scale) in bbb_world::vanilla_entity_renderer_thrown_item_scales() {
        assert_eq!(
            entity_model_kind(type_id, &[]),
            EntityModelKind::NoRender,
            "thrown-item projectile type {type_id} should be NoRender",
        );
    }
}

#[test]
fn entity_model_kind_maps_wither_skull_to_real_model() {
    // The wither skull was a placeholder box; it now resolves to the real `SkullModel`. Its flight
    // facing comes from the projected yaw/pitch (a plain `EntityRenderer`). Vanilla
    // `WitherSkull.DATA_DANGEROUS` is the synced boolean at id 8 and swaps to
    // `wither_invulnerable.png`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, &[]),
        EntityModelKind::WitherSkull { dangerous: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
            &[wither_skull_dangerous_data(true)]
        ),
        EntityModelKind::WitherSkull { dangerous: true }
    );
}

#[test]
fn entity_model_kind_maps_llama_spit_to_real_model() {
    // The llama spit was a placeholder box; it now resolves to the real `LlamaSpitModel`. The
    // model has no `setupAnim`, so the geometry is complete; only the texture is deferred
    // entity-side state, so no synced data is read.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, &[]),
        EntityModelKind::LlamaSpit
    );
}

#[test]
fn entity_model_kind_maps_shulker_bullet_to_real_model() {
    // The shulker bullet was a placeholder box; it now resolves to the real `ShulkerBulletModel`.
    // Its facing and age-driven tumble come from projected render state; the textured path adds the
    // translucent outer-shell pass. The kind mapping itself reads no synced data.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, &[]),
        EntityModelKind::ShulkerBullet
    );
}

#[test]
fn entity_model_kind_maps_wind_charges_to_real_model() {
    // The wind charge and breeze wind charge were placeholder boxes; both share the real
    // `WindChargeModel` (vanilla registers `WindChargeRenderer` for both). Projected age drives the
    // counter-rotation and scrolling translucent texture; the kind mapping itself reads no synced
    // data.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WIND_CHARGE_ID, &[]),
        EntityModelKind::WindCharge
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID, &[]),
        EntityModelKind::WindCharge
    );
}

#[test]
fn entity_model_kind_maps_ender_dragon_to_real_model() {
    // The ender dragon was a placeholder bounds box; it now resolves to the real
    // `EnderDragonModel` at its bind layout. Projected world/render state drives the procedural
    // flight animation, dying dissolve/rays, nearest-crystal healing beam, and renderer-owned
    // emissive eyes; no synced data is read by the kind mapping itself.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, &[]),
        EntityModelKind::EnderDragon
    );
}

#[test]
fn entity_model_kind_renders_nothing_for_noop_renderer_entities() {
    // The area effect cloud, marker, and interaction use vanilla `NoopRenderer` — they render no
    // model — so they resolve to `EntityModelKind::NoRender`, replacing the former placeholder
    // boxes (which incorrectly drew a debug box where vanilla draws nothing).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, &[]),
        EntityModelKind::NoRender
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_INTERACTION_ID, &[]),
        EntityModelKind::NoRender
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_MARKER_ID, &[]),
        EntityModelKind::NoRender
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_polar_bears() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, &[]),
        EntityModelKind::PolarBear { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, true)]
        ),
        EntityModelKind::PolarBear { baby: true }
    );
    // The panda (adult and baby) renders through its dedicated `PandaModel` / `BabyPandaModel`;
    // with no gene metadata the displayed variant is the vanilla default `NORMAL`.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PANDA_ID, &[]),
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Normal
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PANDA_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_PANDA_ID, true)]
        ),
        EntityModelKind::Panda {
            baby: true,
            variant: PandaModelVariant::Normal
        }
    );
}

#[test]
fn entity_model_kind_projects_panda_gene_variant_from_data() {
    // Vanilla `Panda.getVariant()` = `Gene.getVariantFromGenes(mainGene, hiddenGene)` off the two
    // synced gene bytes. A dominant main gene always shows.
    let accessors = panda_metadata_accessors();
    assert_eq!(
        panda_model_kind(
            VANILLA_ENTITY_TYPE_PANDA_ID,
            &[
                protocol_byte_data(accessors.main_gene_data_id, 6),
                protocol_byte_data(accessors.hidden_gene_data_id, 0),
            ]
        ),
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Aggressive
        }
    );
    // A recessive main gene (BROWN=4) shows only when both genes match.
    assert_eq!(
        panda_model_kind(
            VANILLA_ENTITY_TYPE_PANDA_ID,
            &[
                protocol_byte_data(accessors.main_gene_data_id, 4),
                protocol_byte_data(accessors.hidden_gene_data_id, 4),
            ]
        ),
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Brown
        }
    );
    // An unmatched recessive main gene falls back to NORMAL.
    assert_eq!(
        panda_model_kind(
            VANILLA_ENTITY_TYPE_PANDA_ID,
            &[
                protocol_byte_data(accessors.main_gene_data_id, 4),
                protocol_byte_data(accessors.hidden_gene_data_id, 1),
            ]
        ),
        EntityModelKind::Panda {
            baby: false,
            variant: PandaModelVariant::Normal
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_villagers() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_VILLAGER_ID, &[]),
        EntityModelKind::Villager { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_VILLAGER_ID, true)]
        ),
        EntityModelKind::Villager { baby: true }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, &[]),
        EntityModelKind::WanderingTrader
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, true)]
        ),
        EntityModelKind::WanderingTrader
    );
}

#[test]
fn villager_model_data_reads_vanilla_serializer_and_static_fallback_order() {
    assert_eq!(
        villager_model_data(VANILLA_ENTITY_TYPE_VILLAGER_ID, &[], None),
        VillagerModelData::DEFAULT
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[protocol_villager_data_at_unhappy_counter(6, 14, 9)],
            None,
        ),
        VillagerModelData::DEFAULT,
        "id 18 is AbstractVillager.DATA_UNHAPPY_COUNTER, not Villager.DATA_VILLAGER_DATA"
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                6,
                14,
                9,
            )],
            None,
        ),
        VillagerModelData::new(
            VillagerModelType::Taiga,
            VillagerModelProfession::Weaponsmith,
            9,
        )
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                4,
                11,
                2
            )],
            None,
        ),
        VillagerModelData::new(VillagerModelType::Snow, VillagerModelProfession::Nitwit, 2,)
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                4,
                11,
                2,
            )],
            None,
        ),
        VillagerModelData::DEFAULT,
        "zombie villager data lives at id 20, not the villager id 19"
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                6,
                14,
                9,
            )],
            None,
        ),
        VillagerModelData::DEFAULT,
        "wandering traders do not own DATA_VILLAGER_DATA"
    );
}

#[test]
fn villager_model_data_uses_fixed_built_in_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:villager_type",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:swamp"),
            RegistryPacketEntry::stub("minecraft:desert"),
        ],
    );
    world.record_registry_entries(
        "minecraft:villager_profession",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:librarian"),
            RegistryPacketEntry::stub("minecraft:farmer"),
        ],
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                1,
                0,
                5,
            )],
            Some(&world),
        ),
        VillagerModelData::new(VillagerModelType::Jungle, VillagerModelProfession::None, 5,),
        "static villager registries are not remapped by configuration RegistryData"
    );
    assert_eq!(
        villager_model_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            &[protocol_villager_data(
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                99,
                -1,
                3,
            )],
            Some(&world),
        ),
        VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::None, 3,)
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_wolves() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WOLF_ID, &[]),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_WOLF_ID, true)]
        ),
        EntityModelKind::Wolf {
            baby: true,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[tamable_animal_flags_data(
                VANILLA_ENTITY_TYPE_WOLF_ID,
                true,
                false,
            )]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Red),
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[
                tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
                protocol_int_data(wolf_metadata_accessors().collar_color_data_id, 11),
            ]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Blue),
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[
                entity_invisible_data(),
                tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
                protocol_int_data(wolf_metadata_accessors().collar_color_data_id, 11),
            ]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: false,
            collar_color: Some(EntityDyeColor::Blue),
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[protocol_int_data(
                wolf_metadata_accessors().collar_color_data_id,
                11,
            )]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    // The adult cat, ocelot, and fox render through their dedicated models (cat = the shared
    // `AdultFelineModel` scaled 0.8, ocelot = the unscaled feline, fox = `AdultFoxModel`); each baby
    // now renders through its own dedicated vanilla mesh.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[]),
        EntityModelKind::Feline {
            cat: true,
            baby: false,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_OCELOT_ID, &[]),
        EntityModelKind::Feline {
            cat: false,
            baby: false,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FOX_ID, &[]),
        EntityModelKind::Fox {
            baby: false,
            variant: FoxModelVariant::Red
        }
    );
    // The cat/ocelot babies now render through the dedicated `BabyFelineModel` layout, as does the
    // fox baby through its own `BabyFoxModel`.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_CAT_ID, true)]
        ),
        EntityModelKind::Feline {
            cat: true,
            baby: true,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_OCELOT_ID, true)]
        ),
        EntityModelKind::Feline {
            cat: false,
            baby: true,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_FOX_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_FOX_ID, true)]
        ),
        EntityModelKind::Fox {
            baby: true,
            variant: FoxModelVariant::Red
        }
    );
    // The fox `DATA_TYPE_ID` (18, int) selects the RED/SNOW variant via `Fox.Variant.byId`.
    let fox_accessors =
        vanilla_entity_metadata_fox_accessors_for_entity_type(VANILLA_ENTITY_TYPE_FOX_ID)
            .expect("generated Fox metadata accessors must resolve");
    for (id, variant) in [
        (-1, FoxModelVariant::Red),
        (0, FoxModelVariant::Red),
        (1, FoxModelVariant::Snow),
        (2, FoxModelVariant::Red),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_FOX_ID,
                &[protocol_int_data(fox_accessors.type_data_id, id)]
            ),
            EntityModelKind::Fox {
                baby: false,
                variant
            }
        );
    }
    // The rabbit (adult and baby) renders through its dedicated `AdultRabbitModel` / `BabyRabbitModel`,
    // textured by the `Rabbit.Variant` colour (`DATA_TYPE_ID`) plus the Toast name override.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_RABBIT_ID, &[]),
        EntityModelKind::Rabbit {
            baby: false,
            variant: RabbitModelVariant::Brown,
            toast: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_RABBIT_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_RABBIT_ID, true)]
        ),
        EntityModelKind::Rabbit {
            baby: true,
            variant: RabbitModelVariant::Brown,
            toast: false
        }
    );
    // `DATA_TYPE_ID` selects the colour via `Rabbit.Variant.byId` (sparse; EVIL = 99).
    for (id, variant) in [
        (0, RabbitModelVariant::Brown),
        (1, RabbitModelVariant::White),
        (2, RabbitModelVariant::Black),
        (3, RabbitModelVariant::WhiteSplotched),
        (4, RabbitModelVariant::Gold),
        (5, RabbitModelVariant::Salt),
        (99, RabbitModelVariant::Evil),
        (7, RabbitModelVariant::Brown),
    ] {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_RABBIT_ID, &[rabbit_type_data(id)]),
            EntityModelKind::Rabbit {
                baby: false,
                variant,
                toast: false
            }
        );
    }
    // The custom name "Toast" flips the toast override; any other name does not.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_RABBIT_ID,
            &[entity_custom_name_data(Some("Toast"))]
        ),
        EntityModelKind::Rabbit {
            baby: false,
            variant: RabbitModelVariant::Brown,
            toast: true
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_RABBIT_ID,
            &[entity_custom_name_data(Some("toast"))]
        ),
        EntityModelKind::Rabbit {
            baby: false,
            variant: RabbitModelVariant::Brown,
            toast: false
        }
    );
}

#[test]
fn entity_model_kind_projects_wolf_variant_from_registry_and_fallback() {
    // Vanilla `WolfRenderer` keys the texture on the synced `Wolf.DATA_VARIANT_ID` (index 23)
    // registry holder. The dynamic `wolf_variant` registry order the server sent wins; without it
    // the static `WolfVariants.bootstrap` order is the fallback. The default is `Pale`.
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:wolf_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:striped"),
            RegistryPacketEntry::stub("minecraft:ashen"),
            RegistryPacketEntry::stub("minecraft:woods"),
        ],
    );
    // The world-bound metadata identity records that registry id 1 meant `ashen`; consumers do
    // not reinterpret the ordinal through a separately supplied registry generation.
    assert_eq!(
        entity_model_kind_with_registries(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[bind_protocol_registry_data_to_resource(
                protocol_wolf_variant_data(1),
                "minecraft:ashen",
            )],
            Some(&world),
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Ashen,
        }
    );

    // No dynamic registry → the static vanilla order: id 3 is `black`.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[protocol_wolf_variant_data(3)]
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Black,
        }
    );

    // No variant holder at all → the vanilla `WolfVariants.DEFAULT` (`Pale`).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WOLF_ID, &[]),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
}

#[test]
fn entity_model_kind_uses_vanilla_cat_variant_metadata() {
    // Without the dynamic `cat_variant` registry the bootstrap order (tabby=0..all_black=10) is
    // the static fallback; the vanilla default is BLACK. The ocelot has no breed.
    for (id, variant) in [
        (0, CatModelVariant::Tabby),
        (1, CatModelVariant::Black),
        (2, CatModelVariant::Red),
        (3, CatModelVariant::Siamese),
        (4, CatModelVariant::BritishShorthair),
        (5, CatModelVariant::Calico),
        (6, CatModelVariant::Persian),
        (7, CatModelVariant::Ragdoll),
        (8, CatModelVariant::White),
        (9, CatModelVariant::Jellie),
        (10, CatModelVariant::AllBlack),
        (99, CatModelVariant::Black),
    ] {
        assert_eq!(
            entity_model_kind(VANILLA_ENTITY_TYPE_CAT_ID, &[protocol_cat_variant_data(id)]),
            EntityModelKind::Feline {
                cat: true,
                baby: false,
                cat_variant: variant,
                collar: None
            }
        );
    }
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            &[protocol_cat_variant_data(0)]
        ),
        EntityModelKind::Feline {
            cat: false,
            baby: false,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
}

#[test]
fn entity_model_kind_projects_cat_collar_from_tame_and_color() {
    // Vanilla `CatRenderer`: `state.collarColor = isTame() ? getCollarColor() : null`, and
    // `getCollarColor() = DyeColor.byId(DATA_COLLAR_COLOR)` (default RED). The ocelot has no collar.
    fn collar_of(kind: &EntityModelKind) -> Option<EntityDyeColor> {
        match kind {
            EntityModelKind::Feline { collar, .. } => *collar,
            other => panic!("expected feline, got {other:?}"),
        }
    }

    // An untamed cat carries no collar even with a color set.
    assert_eq!(
        collar_of(&feline_model_kind(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[protocol_int_data(
                cat_metadata_accessors().collar_color_data_id,
                5,
            )],
            true,
            None,
        )),
        None
    );
    // A tame cat with no explicit color defaults to RED (14).
    assert_eq!(
        collar_of(&feline_model_kind(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[tamable_animal_flags_data(
                VANILLA_ENTITY_TYPE_CAT_ID,
                true,
                false,
            )],
            true,
            None,
        )),
        Some(EntityDyeColor::Red)
    );
    // A tame cat shows its dyed collar color.
    assert_eq!(
        collar_of(&feline_model_kind(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[
                tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, true, false),
                protocol_int_data(cat_metadata_accessors().collar_color_data_id, 5),
            ],
            true,
            None,
        )),
        Some(EntityDyeColor::Lime)
    );
    // A tame ocelot still has no collar.
    assert_eq!(
        collar_of(&feline_model_kind(
            VANILLA_ENTITY_TYPE_OCELOT_ID,
            &[tamable_animal_flags_data(
                VANILLA_ENTITY_TYPE_CAT_ID,
                true,
                false,
            )],
            false,
            None,
        )),
        None
    );
}

#[test]
fn entity_model_instances_project_cat_variants_from_world_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:cat_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:jellie"),
            RegistryPacketEntry::stub("minecraft:calico"),
            RegistryPacketEntry::stub("minecraft:white"),
        ],
    );
    assert_eq!(
        entity_model_kind_with_registries(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[protocol_cat_variant_data(99)],
            Some(&world),
        ),
        EntityModelKind::Feline {
            cat: true,
            baby: false,
            cat_variant: CatModelVariant::Black,
            collar: None
        }
    );
    world.apply_add_entity(protocol_add_entity(
        41,
        VANILLA_ENTITY_TYPE_CAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        42,
        VANILLA_ENTITY_TYPE_CAT_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 41,
        values: vec![protocol_cat_variant_data(0)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 42,
        values: vec![
            protocol_cat_variant_data(2),
            baby_data(VANILLA_ENTITY_TYPE_CAT_ID, true),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![
                EntityModelInstance::feline(
                    41,
                    [1.0, 64.0, -2.0],
                    0.0,
                    true,
                    false,
                    CatModelVariant::Jellie,
                    None,
                ),
                EntityModelInstance::feline(
                    42,
                    [3.0, 64.0, -2.0],
                    0.0,
                    true,
                    true,
                    CatModelVariant::White,
                    None,
                )
                .with_age_scale(0.5),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_project_feline_crouch_sprint_and_cat_sitting_from_world() {
    // Vanilla `CatRenderer` / `OcelotRenderer.extractRenderState` copy `Entity.isCrouching()`
    // (Pose.CROUCHING, ordinal 5) and `Entity.isSprinting()` (shared flags bit 3). Cat also copies
    // `TamableAnimal.isInSittingPose()`; ocelot leaves `isSitting` at the default false.
    let pose_data = |pose| entity_pose_data(pose);
    let feline_state = |world: &WorldStore, id: i32| {
        let instance = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap();
        (
            instance.render_state.feline_is_crouching,
            instance.render_state.feline_is_sprinting,
            instance.render_state.feline_is_sitting,
        )
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        43,
        VANILLA_ENTITY_TYPE_CAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        44,
        VANILLA_ENTITY_TYPE_OCELOT_ID,
        [3.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        45,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [5.0, 64.0, -2.0],
    ));

    assert_eq!(feline_state(&world, 43), (false, false, false));
    assert_eq!(feline_state(&world, 44), (false, false, false));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 43,
        values: vec![
            pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID),
            entity_sprinting_data(),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, false, true),
        ],
    }));
    assert_eq!(feline_state(&world, 43), (true, true, true));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 44,
        values: vec![pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID)],
    }));
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 44,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_CAT_ID,
            false,
            true,
        )],
    }));
    assert_eq!(feline_state(&world, 44), (true, false, false));

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 45,
        values: vec![
            pose_data(VANILLA_ENTITY_POSE_CROUCHING_ID),
            entity_sprinting_data(),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, false, true),
        ],
    }));
    assert_eq!(feline_state(&world, 45), (false, false, false));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 43,
        values: vec![
            pose_data(VANILLA_ENTITY_POSE_STANDING_ID),
            entity_default_shared_flags_data(),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_CAT_ID, false, false),
        ],
    }));
    assert_eq!(feline_state(&world, 43), (false, false, false));
}

#[test]
fn entity_model_instances_project_cat_lie_down_and_relax_amounts_from_world() {
    // Vanilla `CatRenderer.extractRenderState` forwards the partial-tick eased
    // `getLieDownAmount`, `getLieDownAmountTail`, and `getRelaxStateOneAmount` into
    // `FelineRenderState`; ocelots leave all three at zero.
    let feline_amounts = |world: &WorldStore, id: i32, partial_tick: f32| {
        let instance = entity_model_instances_from_world_at_partial_tick(world, None, partial_tick)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap();
        (
            instance.render_state.feline_lie_down_amount,
            instance.render_state.feline_lie_down_amount_tail,
            instance.render_state.feline_relax_state_one_amount,
        )
    };
    let assert_close = |actual: f32, expected: f32| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "expected {expected}, got {actual}"
        );
    };
    let assert_amounts = |actual: (f32, f32, f32), expected: (f32, f32, f32)| {
        assert_close(actual.0, expected.0);
        assert_close(actual.1, expected.1);
        assert_close(actual.2, expected.2);
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        46,
        VANILLA_ENTITY_TYPE_CAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        47,
        VANILLA_ENTITY_TYPE_OCELOT_ID,
        [3.0, 64.0, -2.0],
    ));

    assert_eq!(feline_amounts(&world, 46, 1.0), (0.0, 0.0, 0.0));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 46,
        values: vec![
            protocol_bool_data(cat_metadata_accessors().lying_data_id, true),
            protocol_bool_data(cat_metadata_accessors().relax_state_one_data_id, true),
        ],
    }));
    world.advance_entity_client_animations(2);
    assert_amounts(feline_amounts(&world, 46, 1.0), (0.3, 0.16, 0.2));
    assert_amounts(feline_amounts(&world, 46, 0.5), (0.225, 0.12, 0.15));

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 47,
        values: vec![
            protocol_bool_data(cat_metadata_accessors().lying_data_id, true),
            protocol_bool_data(cat_metadata_accessors().relax_state_one_data_id, true),
        ],
    }));
    world.advance_entity_client_animations(2);
    assert_eq!(feline_amounts(&world, 47, 1.0), (0.0, 0.0, 0.0));
}

#[test]
fn entity_model_instances_project_cat_lying_on_sleeping_player_from_world() {
    // Vanilla `CatRenderer.extractRenderState` forwards
    // `Cat.isLyingOnTopOfSleepingPlayer()`, which the world source derives from a lying cat and
    // nearby sleeping player.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        48,
        VANILLA_ENTITY_TYPE_CAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        49,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -1.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 48,
        values: vec![protocol_bool_data(
            cat_metadata_accessors().lying_data_id,
            true,
        )],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 49,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID)],
    }));

    let source = world
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 48)
        .unwrap();
    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 48)
        .unwrap();

    assert!(source.feline_is_lying_on_top_of_sleeping_player);
    assert!(
        instance
            .render_state
            .feline_is_lying_on_top_of_sleeping_player
    );
}

#[test]
fn entity_model_kind_uses_vanilla_wolf_anger_end_time_metadata() {
    assert_eq!(
        entity_model_kind_with_time_and_registries(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[protocol_long_data(
                wolf_metadata_accessors().anger_end_time_data_id,
                200,
            )],
            0.0,
            199,
            None,
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: true,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind_with_time_and_registries(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[protocol_long_data(
                wolf_metadata_accessors().anger_end_time_data_id,
                200,
            )],
            0.0,
            200,
            None,
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert_eq!(
        entity_model_kind_with_time_and_registries(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            &[
                tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
                protocol_long_data(wolf_metadata_accessors().anger_end_time_data_id, 200),
            ],
            0.0,
            199,
            None,
        ),
        EntityModelKind::Wolf {
            baby: false,
            tame: true,
            angry: true,
            collar_color: Some(EntityDyeColor::Red),
            variant: WolfModelVariant::Pale,
        }
    );
}

#[test]
fn entity_model_instances_project_wolf_anger_from_world_game_time() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        148,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![protocol_long_data(
            wolf_metadata_accessors().anger_end_time_data_id,
            130,
        )],
    }));
    world.apply_world_time(PlayTime {
        game_time: 120,
        clock_updates: Vec::new(),
    });

    let angry_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        angry_instances,
        aged(
            vec![EntityModelInstance::wolf_state(
                148,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                false,
                true,
                false,
                None,
            )
            // Vanilla `Wolf.getTailAngle()` angry branch raises the tail to 1.5393804.
            .with_wolf_tail_angle(1.5393804)],
            1.0,
        )
    );

    world.apply_world_time(PlayTime {
        game_time: 130,
        clock_updates: Vec::new(),
    });

    let calm_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        calm_instances,
        aged(
            vec![EntityModelInstance::wolf_state(
                148,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                false,
                false,
                false,
                None,
            )],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_project_wolf_invisible_shared_flag_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        148,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![
            entity_invisible_data(),
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
            protocol_int_data(wolf_metadata_accessors().collar_color_data_id, 11),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![EntityModelInstance::wolf_state(
                148,
                [1.0, 64.0, -2.0],
                0.0,
                false,
                true,
                false,
                true,
                Some(EntityDyeColor::Blue),
            )
            // A tame wolf with no synced health defaults to full (maxHealth 40), so
            // `Wolf.getTailAngle()` = (0.55 - 0) * π.
            .with_wolf_tail_angle(0.55 * std::f32::consts::PI)],
            1.0,
        )
    );
    // The shared invisible flag is now projected uniformly into the render state.
    assert!(instances[0].render_state.invisible);
    assert!(instances[0].render_state.invisible_to_player);
}

#[test]
fn entity_model_instances_project_wolf_tame_tail_angle_from_health() {
    // Vanilla `Wolf.getTailAngle()` for a tame wolf droops the tail with damage:
    // (0.55 - damageRatio * 0.4) * π, damageRatio = (maxHealth - health) / maxHealth,
    // with the tame maxHealth constant 40. A hurt tame wolf (health 8/40) lowers its
    // tail off the healthy raise.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        148,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![
            tamable_animal_flags_data(VANILLA_ENTITY_TYPE_WOLF_ID, true, false),
            living_entity_health_data(VANILLA_ENTITY_TYPE_WOLF_ID, 8.0),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let tail_angle = instances[0].render_state.wolf_tail_angle;
    let expected = (0.55 - 0.8 * 0.4) * std::f32::consts::PI; // damageRatio 0.8
    assert!(
        (tail_angle - expected).abs() < 1e-6,
        "tame wolf tail droops with health: {tail_angle} vs {expected}"
    );

    // An untamed wolf keeps the π/5 default no matter its health.
    let mut wild = WorldStore::new();
    wild.apply_add_entity(protocol_add_entity(
        149,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(wild.apply_set_entity_data(SetEntityData {
        id: 149,
        values: vec![living_entity_health_data(VANILLA_ENTITY_TYPE_WOLF_ID, 4.0)],
    }));
    let wild_instances = entity_model_instances_from_world_at_partial_tick(&wild, None, 1.0);
    assert_eq!(
        wild_instances[0].render_state.wolf_tail_angle,
        std::f32::consts::PI / 5.0
    );
}

#[test]
fn entity_model_instances_project_wolf_sitting_flag_from_world() {
    // Vanilla `WolfRenderState.isSitting = Wolf.isInSittingPose()` = `TamableAnimal`
    // generated `DATA_FLAGS_ID` sitting mask. A sitting (tame) wolf projects `wolf_sitting`; clearing the
    // bit projects `false`.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        148,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            true,
            true,
        )],
    }));
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(
        instances[0].render_state.wolf_sitting,
        "a sitting wolf projects wolf_sitting"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 148,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            true,
            false,
        )],
    }));
    let standing = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(
        !standing[0].render_state.wolf_sitting,
        "a standing wolf does not project wolf_sitting"
    );
}

#[test]
fn entity_model_instance_projects_wolf_wet_shade_from_source() {
    // Vanilla `WolfRenderer.extractRenderState` copies `Wolf.getWetShade(partialTicks)`
    // into `WolfRenderState.wetShade`, and `WolfRenderer.getModelTint` consumes that
    // render-state field. It also copies `Wolf.getShakeAnim(partialTicks)` for
    // `WolfRenderState.getBodyRollAngle`, and `Wolf.getHeadRollAngle(partialTicks)`
    // into `WolfRenderState.headRollAngle`. The world layer owns the timers; native must
    // preserve the projected values when building `EntityRenderState`.
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 148,
        "entity_type_id": VANILLA_ENTITY_TYPE_WOLF_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
        "y_rot": 0.0,
        "wolf_wet_shade": 0.75625,
        "wolf_shake_anim": 0.5,
        "wolf_head_roll_angle": 0.188,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();

    assert_eq!(
        instance.kind,
        EntityModelKind::Wolf {
            baby: false,
            tame: false,
            angry: false,
            collar_color: None,
            variant: WolfModelVariant::Pale,
        }
    );
    assert!(
        (instance.render_state.wolf_wet_shade - 0.75625).abs() < 1.0e-6,
        "native preserves world-projected WolfRenderState.wetShade: {}",
        instance.render_state.wolf_wet_shade
    );
    assert!(
        (instance.render_state.wolf_shake_anim - 0.5).abs() < 1.0e-6,
        "native preserves world-projected WolfRenderState.shakeAnim: {}",
        instance.render_state.wolf_shake_anim
    );
    assert!(
        (instance.render_state.wolf_head_roll_angle - 0.188).abs() < 1.0e-6,
        "native preserves world-projected WolfRenderState.headRollAngle: {}",
        instance.render_state.wolf_head_roll_angle
    );
}

#[test]
fn entity_model_instances_project_parrot_sitting_flag_from_world() {
    // Vanilla `ParrotModel.getPose == SITTING` = `Parrot.isInSittingPose()` = the same
    // The generated `TamableAnimal.DATA_FLAGS_ID` sitting bit inherited by the wolf. A sitting parrot projects
    // `parrot_sitting`; the projection is gated to the parrot, so the same flag byte on
    // another tamable (wolf) never sets `parrot_sitting`.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        150,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        151,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        [2.0, 64.0, -2.0],
    ));

    let parrot_sitting = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .parrot_sitting
    };

    // A standing parrot projects parrot_sitting = false.
    assert!(!parrot_sitting(&world, 150));

    // Setting the TamableAnimal sitting bit projects through to the perch pose.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 150,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_PARROT_ID,
            false,
            true,
        )],
    }));
    assert!(parrot_sitting(&world, 150));

    // The same sitting bit on a non-parrot (wolf) does NOT project parrot_sitting — the
    // derivation is gated to the parrot type.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 151,
        values: vec![tamable_animal_flags_data(
            VANILLA_ENTITY_TYPE_WOLF_ID,
            true,
            true,
        )],
    }));
    assert!(!parrot_sitting(&world, 151));
}

#[test]
fn entity_model_instances_preserve_parrot_party_from_world_source() {
    // Vanilla `ParrotRenderer.extractRenderState` copies `ParrotModel.getPose(entity)`, where
    // `isPartyParrot()` wins over sitting/flying. The world layer owns the jukebox proximity
    // projection, so native must preserve `parrot_party` when building `EntityRenderState`.
    let source: EntityModelSourceState = serde_json::from_value(serde_json::json!({
        "entity_id": 152,
        "entity_type_id": VANILLA_ENTITY_TYPE_PARROT_ID,
        "position": { "x": 1.0, "y": 64.0, "z": -2.0 },
        "y_rot": 0.0,
        "parrot_party": true,
        "data_values": []
    }))
    .unwrap();

    let instance = entity_model_instance(source, &WorldStore::new(), None, 0, 1.0, None).unwrap();

    assert_eq!(
        instance.kind,
        EntityModelKind::Parrot {
            variant: ParrotModelVariant::RedBlue,
        }
    );
    assert!(
        instance.render_state.parrot_party,
        "native preserves the world-projected ParrotRenderState PARTY pose"
    );
}

#[test]
fn entity_model_instances_project_illager_spellcasting_flag_from_world() {
    // Vanilla `SpellcasterIllager.isCastingSpell()` = the synced `DATA_SPELL_CASTING_ID`
    // byte > 0 (the byte holds the spell id). A casting evoker/illusioner projects
    // `illager_spellcasting`; the projection is gated to the spellcaster illagers, so the
    // same byte on a vindicator never sets it.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        160,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        161,
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        162,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        [3.0, 64.0, -2.0],
    ));

    let casting = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .illager_spellcasting
    };

    // An idle evoker/illusioner projects illager_spellcasting = false.
    assert!(!casting(&world, 160));
    assert!(!casting(&world, 161));

    // Setting the spell-casting byte > 0 (here 2 = FANGS) projects through to the cast pose.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![protocol_byte_data(
            spellcaster_casting_data_id(VANILLA_ENTITY_TYPE_EVOKER_ID),
            2,
        )],
    }));
    assert!(casting(&world, 160));
    // Any non-zero spell id (1 = SUMMON_VEX, the lowest) also counts as casting.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 161,
        values: vec![protocol_byte_data(
            spellcaster_casting_data_id(VANILLA_ENTITY_TYPE_ILLUSIONER_ID),
            1,
        )],
    }));
    assert!(casting(&world, 161));

    // The same byte on a non-spellcaster (vindicator) does NOT project illager_spellcasting.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 162,
        values: vec![protocol_byte_data(
            spellcaster_casting_data_id(VANILLA_ENTITY_TYPE_EVOKER_ID),
            2,
        )],
    }));
    assert!(!casting(&world, 162));
}

#[test]
fn entity_model_kind_uses_exact_models_for_horses() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_HORSE_ID, &[]),
        EntityModelKind::Horse {
            baby: false,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HORSE_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_HORSE_ID, true)]
        ),
        EntityModelKind::Horse {
            baby: true,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
    );
    // The packed `DATA_ID_TYPE_VARIANT` (id 19) carries the coat color in the low byte
    // (`& 0xFF`) and the markings in the next nibble (`>> 8`): id `4 | (2 << 8)` = black coat +
    // white-field markings.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_HORSE_ID,
            &[horse_variant_data(4 | (2 << 8))]
        ),
        EntityModelKind::Horse {
            baby: false,
            variant: HorseColorVariant::Black,
            markings: HorseMarkings::WhiteField
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_DONKEY_ID, &[]),
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: false,
            has_chest: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_DONKEY_ID,
            &[
                baby_data(VANILLA_ENTITY_TYPE_DONKEY_ID, true),
                chested_horse_chest_data(VANILLA_ENTITY_TYPE_DONKEY_ID, true),
            ]
        ),
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: true,
            has_chest: true
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_MULE_ID,
            &[chested_horse_chest_data(VANILLA_ENTITY_TYPE_MULE_ID, true)]
        ),
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: false,
            has_chest: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, &[]),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, true)]
        ),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, &[]),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false
        }
    );
    // The nautilus (adult and baby) renders through its dedicated `NautilusModel`
    // (`createBodyMesh` / `createBabyBodyLayer`). The zombie nautilus maps to the dedicated
    // `ZombieNautilus` kind, selected by the synced `ZombieNautilusVariant` holder: the default
    // TEMPERATE → the `NautilusModel` body + `zombie_nautilus.png`; WARM → the
    // `ZombieNautilusCoralModel` + `zombie_nautilus_coral.png`. It is a plain `MobRenderer`, so
    // always adult.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_NAUTILUS_ID, &[]),
        EntityModelKind::Nautilus { baby: false }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_NAUTILUS_ID, true)]
        ),
        EntityModelKind::Nautilus { baby: true }
    );
    // No variant data → the TEMPERATE default (no corals).
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, &[]),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[protocol_zombie_nautilus_variant_data(0)]
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate
        }
    );
    // WARM (registry id 1) → the coral model.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[protocol_zombie_nautilus_variant_data(1)]
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm
        }
    );
    // The zombie nautilus is never a baby, so the baby flag in its metadata is ignored.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID, true)]
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate
        }
    );
}

#[test]
fn entity_model_kind_projects_zombie_nautilus_variant_from_registry_and_fallback() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:zombie_nautilus_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:warm"),
            RegistryPacketEntry::stub("minecraft:temperate"),
        ],
    );

    assert_eq!(
        entity_model_kind_with_registries(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[bind_protocol_registry_data_to_resource(
                protocol_zombie_nautilus_variant_data(0),
                "minecraft:warm",
            )],
            Some(&world),
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Warm
        }
    );
    assert_eq!(
        entity_model_kind_with_registries(
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            &[protocol_zombie_nautilus_variant_data(99)],
            Some(&world),
        ),
        EntityModelKind::ZombieNautilus {
            variant: ZombieNautilusModelVariant::Temperate
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_camels() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAMEL_ID, &[]),
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_CAMEL_ID, true)]
        ),
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: true
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, &[]),
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            &[baby_data(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, true)]
        ),
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: false
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_llamas() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_LLAMA_ID, &[]),
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Creamy,
            baby: false,
            has_chest: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            &[
                llama_variant_data(VANILLA_ENTITY_TYPE_LLAMA_ID, 2),
                chested_horse_chest_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true),
            ]
        ),
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Brown,
            baby: false,
            has_chest: true
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_LLAMA_ID,
            &[
                baby_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true),
                chested_horse_chest_data(VANILLA_ENTITY_TYPE_LLAMA_ID, true),
                llama_variant_data(VANILLA_ENTITY_TYPE_LLAMA_ID, 3),
            ]
        ),
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Gray,
            baby: true,
            has_chest: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            &[llama_variant_data(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, 99)]
        ),
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Gray,
            baby: false,
            has_chest: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            &[llama_variant_data(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, -1)]
        ),
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Creamy,
            baby: false,
            has_chest: false
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_illagers_and_witch() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_WITCH_ID, &[]),
        EntityModelKind::Witch
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_EVOKER_ID, &[]),
        EntityModelKind::Illager {
            family: IllagerModelFamily::Evoker
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ILLUSIONER_ID, &[]),
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PILLAGER_ID, &[]),
        EntityModelKind::Illager {
            family: IllagerModelFamily::Pillager
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_VINDICATOR_ID, &[]),
        EntityModelKind::Illager {
            family: IllagerModelFamily::Vindicator
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_spiders() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SPIDER_ID, &[]),
        EntityModelKind::Spider
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, &[]),
        EntityModelKind::CaveSpider
    );
}

#[test]
fn entity_model_kind_uses_avatar_model_part_visibility_for_players_and_mannequins() {
    let hat_and_left_sleeve = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PLAYER_ID, &[]),
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            parts: PlayerModelPartVisibility::from_vanilla_mask(0),
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, &[]),
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            parts: PlayerModelPartVisibility::from_vanilla_mask(
                PlayerModelPartVisibility::ALL_MASK,
            ),
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            &[protocol_byte_data(
                AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                hat_and_left_sleeve.vanilla_mask() as i8,
            )],
        ),
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            parts: hat_and_left_sleeve,
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            &[protocol_byte_data(AVATAR_MODEL_CUSTOMIZATION_DATA_ID, 0)],
        ),
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            parts: PlayerModelPartVisibility::from_vanilla_mask(0),
        }
    );
}

#[test]
fn entity_model_kind_uses_exact_models_for_boats_and_rafts() {
    let cases = [
        (
            VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID,
            BoatModelFamily::Acacia,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID,
            BoatModelFamily::Acacia,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
            BoatModelFamily::Bamboo,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID,
            BoatModelFamily::Bamboo,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID,
            BoatModelFamily::Birch,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID,
            BoatModelFamily::Cherry,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID,
            BoatModelFamily::DarkOak,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID,
            BoatModelFamily::Jungle,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID,
            BoatModelFamily::Mangrove,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            BoatModelFamily::Oak,
            true,
        ),
        (
            VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID,
            BoatModelFamily::PaleOak,
            false,
        ),
        (
            VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID,
            BoatModelFamily::Spruce,
            true,
        ),
    ];

    for (entity_type_id, family, chest) in cases {
        assert_eq!(
            entity_model_kind(entity_type_id, &[]),
            EntityModelKind::Boat { family, chest }
        );
    }
}

#[test]
fn entity_model_kind_uses_exact_model_for_enderman() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_ENDERMAN_ID, &[]),
        EntityModelKind::Enderman
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, &[]),
        EntityModelKind::SnowGolem
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_iron_golem() {
    // No synced health → the default full-health golem is uncracked.
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, &[]),
        EntityModelKind::IronGolem {
            crackiness: IronGolemCrackiness::None,
        }
    );
    // Vanilla `IronGolem.getCrackiness()` = `Crackiness.GOLEM.byFraction(health / 100)`: at 40/100
    // (= 0.4) the medium cracks show.
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            &[living_entity_health_data(
                VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
                40.0,
            )]
        ),
        EntityModelKind::IronGolem {
            crackiness: IronGolemCrackiness::Medium,
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, &[]),
        EntityModelKind::SnowGolem
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_snow_golem() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, &[]),
        EntityModelKind::SnowGolem
    );
}

#[test]
fn entity_model_kind_uses_exact_model_for_copper_golem_weathering() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, &[]),
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        }
    );
    for (id, weathering) in [
        (-5, CopperGolemWeathering::Unaffected),
        (0, CopperGolemWeathering::Unaffected),
        (1, CopperGolemWeathering::Exposed),
        (2, CopperGolemWeathering::Weathered),
        (3, CopperGolemWeathering::Oxidized),
        (99, CopperGolemWeathering::Oxidized),
    ] {
        assert_eq!(
            entity_model_kind(
                VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
                &[protocol_copper_golem_weathering_data(id)],
            ),
            EntityModelKind::CopperGolem { weathering }
        );
    }
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
            &[protocol_armadillo_state_data(2)],
        ),
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        }
    );
}

#[test]
fn entity_model_kind_uses_explicit_unknown_future_fallback() {
    let kind = entity_model_kind(9999, &[]);

    assert_eq!(
        placeholder_name(kind),
        Some("todo_unknown_entity_type_bounds")
    );
}

#[test]
fn entity_model_instances_filter_local_player_and_camera_entity() {
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(10));
    world.apply_add_entity(protocol_add_entity(
        10,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [0.0, 64.0, 0.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        11,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, 0.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        12,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [2.0, 64.0, 0.0],
    ));
    assert!(world.apply_set_camera(SetCamera { camera_id: 11 }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        instances,
        aged(
            vec![EntityModelInstance::chicken(
                12,
                [2.0, 64.0, 0.0],
                0.0,
                false
            )],
            1.0,
        )
    );
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
        bbb_world::vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type(
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

fn glow_squid_dark_ticks_data(ticks: i32) -> EntityDataValue {
    let (data_id, default) =
        vanilla_entity_metadata_glow_squid_dark_ticks_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
        )
        .expect("generated GlowSquid dark-ticks accessor");
    assert_eq!(default, 0);
    protocol_int_data(data_id, ticks)
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

fn entity_metadata_accessors() -> bbb_world::VanillaEntityMetadataAccessors {
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
        bbb_world::vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated zombification-immunity accessor");
    assert!(!default);
    protocol_bool_data(data_id, immune)
}

fn piglin_baby_data_id() -> u8 {
    let (data_id, default) = bbb_world::vanilla_entity_metadata_baby_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
    )
    .expect("generated Piglin baby accessor");
    assert!(!default);
    data_id
}

fn piglin_dancing_data_id() -> u8 {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin dancing accessor");
    assert!(!default);
    data_id
}

fn spellcaster_casting_data_id(entity_type_id: i32) -> u8 {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
            entity_type_id,
        )
        .expect("generated SpellcasterIllager casting accessor");
    assert_eq!(default, 0);
    data_id
}

fn creaking_active_data_id() -> u8 {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_creaking_active_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated Creaking active accessor");
    assert!(!default);
    data_id
}

fn ghast_charging_data_id() -> u8 {
    let (data_id, default) =
        bbb_world::vanilla_entity_metadata_ghast_charging_accessor_for_entity_type(
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
    let accessor = bbb_world::vanilla_entity_metadata_accessor_for_entity_type(
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

fn wolf_metadata_accessors() -> bbb_world::VanillaWolfMetadataAccessors {
    vanilla_entity_metadata_wolf_accessors_for_entity_type(VANILLA_ENTITY_TYPE_WOLF_ID)
        .expect("generated Wolf metadata accessors must resolve")
}

fn cat_metadata_accessors() -> bbb_world::VanillaCatMetadataAccessors {
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

fn piglins_zombify_dimension_nbt(value: bool) -> Vec<u8> {
    let mut out = vec![10, 10];
    write_test_nbt_string(&mut out, "attributes");
    out.push(1);
    write_test_nbt_string(&mut out, "minecraft:gameplay/piglins_zombify");
    out.push(i8::from(value) as u8);
    out.push(0);
    out.push(0);
    out
}

fn write_test_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
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

#[test]
fn fishing_hook_third_person_hand_offset_matches_vanilla_projection() {
    assert_selection_box_close(
        fishing_hook_third_person_line_origin_offset(fishing_hook_render_source()),
        [1.65, 1.3, 4.8],
    );

    let mut source = fishing_hook_render_source();
    source.holding_arm_left = true;
    source.owner_is_crouching = true;
    assert_selection_box_close(
        fishing_hook_third_person_line_origin_offset(source),
        [2.35, 1.1125, 4.8],
    );
}

#[test]
fn fishing_hook_world_source_reaches_dedicated_instance_state() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        4,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, 3.0],
    ));
    let mut hook = protocol_add_entity(5, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, [6.0, 62.0, -1.0]);
    hook.data = 4;
    world.apply_add_entity(hook);

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 5)
        .expect("dedicated fishing-hook instance");
    assert_eq!(instance.kind, EntityModelKind::FishingHook);
    assert_selection_box_close(
        instance
            .render_state
            .fishing_line_origin_offset
            .expect("player-owner hand offset"),
        [-3.65, 2.92, 4.8],
    );
}

#[test]
fn fishing_hook_first_person_hand_offset_uses_camera_near_plane() {
    let pose = CameraPose {
        position: [10.0, 20.0, 30.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    assert_selection_box_close(
        fishing_hook_first_person_line_origin_offset(
            fishing_hook_render_source(),
            pose,
            [1920, 1080],
        ),
        [1.5518672, 1.7019858, 4.6857147],
    );

    let mut source = fishing_hook_render_source();
    source.holding_arm_left = true;
    assert_selection_box_close(
        fishing_hook_first_person_line_origin_offset(source, pose, [1920, 1080]),
        [2.4481325, 1.7019858, 4.6857147],
    );
}

#[test]
fn entity_model_kind_uses_dedicated_fishing_hook_renderer() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, &[]),
        EntityModelKind::FishingHook
    );
}

const VANILLA_PICK_TARGET_RENDER_MODEL_ENTITY_TYPE_IDS: &[i32] = &[
    0, 1, 2, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    29, 30, 31, 32, 33, 34, 35, 36, 38, 40, 41, 42, 45, 46, 51, 52, 54, 55, 56, 57, 58, 59, 61, 62,
    63, 64, 65, 66, 67, 68, 70, 74, 75, 78, 80, 81, 82, 83, 85, 86, 87, 88, 89, 90, 91, 94, 95, 96,
    97, 98, 99, 100, 101, 102, 103, 104, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
    119, 121, 122, 124, 125, 126, 127, 128, 129, 130, 132, 133, 134, 136, 137, 138, 139, 140, 141,
    142, 143, 144, 145, 146, 148, 149, 150, 151, 152, 153, 154, 155,
];

fn placeholder_name(kind: EntityModelKind) -> Option<&'static str> {
    match kind {
        EntityModelKind::Placeholder { name, .. } => Some(name),
        _ => None,
    }
}
