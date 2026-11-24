use super::*;
use crate::entity_models::model_layers::player_model;

mod layers;
mod poses;

fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_fixture(instances, atlas)
}

fn entity_model_textured_meshes_with_dynamic_textures(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    skins: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    textures: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    super::entity_model_textured_meshes_with_static_equipment_and_dynamic_textures_fixture(
        instances, atlas, skins, textures,
    )
}

use crate::entity_models::gpu::build_dynamic_player_texture_atlas_with_limits;
use crate::player_skin::{DynamicPlayerSkinImage, DynamicPlayerTextureImage};
use crate::StandaloneTextureSampling;
use glam::{Mat4, Vec3};

/// The wide-player limb rest poses, for the desc-level arm-swing/bob reference-formula tests (the
/// player now builds a named tree, so it has no `*_PARTS` desc const). Right arm `x = -5`, left arm
/// `x = +5`, right leg `x = -1.9` — the vanilla `PlayerModel.createMesh` offsets.
const PLAYER_FIXTURE_RIGHT_ARM_POSE: PartPose = PartPose {
    offset: [-5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_FIXTURE_LEFT_ARM_POSE: PartPose = PartPose {
    offset: [5.0, 2.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};
const PLAYER_FIXTURE_RIGHT_LEG_POSE: PartPose = PartPose {
    offset: [-1.9, 12.0, 0.0],
    rotation: [0.0, 0.0, 0.0],
};

fn assert_player_mat4_close(actual: Mat4, expected: Mat4) {
    for (actual, expected) in actual
        .to_cols_array()
        .into_iter()
        .zip(expected.to_cols_array())
    {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "expected matrix component {expected}, got {actual}"
        );
    }
}

#[test]
fn player_model_part_visibility_masks_match_vanilla_player_model_part_bits() {
    assert_eq!(PlayerModelPartVisibility::CAPE_MASK, 1 << 0);
    assert_eq!(PlayerModelPartVisibility::JACKET_MASK, 1 << 1);
    assert_eq!(PlayerModelPartVisibility::LEFT_SLEEVE_MASK, 1 << 2);
    assert_eq!(PlayerModelPartVisibility::RIGHT_SLEEVE_MASK, 1 << 3);
    assert_eq!(PlayerModelPartVisibility::LEFT_PANTS_MASK, 1 << 4);
    assert_eq!(PlayerModelPartVisibility::RIGHT_PANTS_MASK, 1 << 5);
    assert_eq!(PlayerModelPartVisibility::HAT_MASK, 1 << 6);
    assert_eq!(PlayerModelPartVisibility::ALL_MASK, 0x7f);
    assert_eq!(
        PLAYER_MODEL_PARTS_ALL_VISIBLE.vanilla_mask(),
        PlayerModelPartVisibility::ALL_MASK
    );
    assert_eq!(PLAYER_MODEL_PARTS_ALL_HIDDEN.vanilla_mask(), 0);

    let mask = PlayerModelPartVisibility::HAT_MASK
        | PlayerModelPartVisibility::JACKET_MASK
        | PlayerModelPartVisibility::LEFT_SLEEVE_MASK
        | PlayerModelPartVisibility::RIGHT_PANTS_MASK;
    let parts = PlayerModelPartVisibility::from_vanilla_mask(mask);
    assert!(parts.hat);
    assert!(parts.jacket);
    assert!(parts.left_sleeve);
    assert!(!parts.right_sleeve);
    assert!(!parts.left_pants);
    assert!(parts.right_pants);
    assert!(!parts.cape);
    assert_eq!(parts.vanilla_mask(), mask);
}

#[test]
fn player_textured_mesh_uses_vanilla_uvs_tints_and_avatar_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let wide_instance = EntityModelInstance::player(901, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let slim_instance = EntityModelInstance::player(902, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.6;
        s.has_red_overlay = true;
    });
    let wide_meshes = entity_model_textured_meshes(&[wide_instance], &atlas);
    let slim_meshes = entity_model_textured_meshes(&[slim_instance], &atlas);
    assert_player_vertices_match_vanilla(&wide_meshes, wide_instance);
    assert_player_vertices_match_vanilla(&slim_meshes, slim_instance);
    let wide = &wide_meshes.cutout;
    let slim = &slim_meshes.cutout;

    for mesh in [&wide, &slim] {
        assert_eq!(mesh.cutout_faces, 72);
        assert_eq!(mesh.vertices.len(), 288);
        assert_eq!(mesh.indices.len(), 432);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
    assert_close2(wide.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(slim.vertices[0].uv, [16.0 / 64.0, 0.5]);

    let (wide_min, wide_max) = textured_mesh_extents(wide);
    let (slim_min, slim_max) = textured_mesh_extents(slim);
    assert!(wide_max[1] - wide_min[1] > 1.8);
    assert!(wide_max[1] - wide_min[1] < 2.0);
    assert!(wide_max[0] - wide_min[0] > slim_max[0] - slim_min[0]);
    assert_ne!(wide.vertices, slim.vertices);
}

#[test]
fn player_using_a_spyglass_raises_it_to_the_eye() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `SPYGLASS`: the holding arm raises along the
    // head look — `xRot = clamp(head.xRot − 1.9198622 − (crouch?π/12), −2.4, 3.3)`, `yRot = head.yRot ∓
    // π/12` — and skips the idle bob (`zRot` back to bind). Applied before crouch, so a crouching player's
    // `arm.xRot += 0.4` still lands on top.
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let base = EntityModelInstance::player(920, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch));

    // Main-hand spyglass raises the RIGHT arm and removes its bob roll.
    let mut main = player_model(
        false,
        &base.with(|s| s.kind.player_mut().using_spyglass = true),
    );
    let right = main.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (pitch_rad - 1.9198622).clamp(-2.4, 3.3)).abs() < 1e-6,
        "the right arm pitches up to the eye: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (yaw_rad - PI / 12.0)).abs() < 1e-6,
        "the right arm yaws −π/12 off the head: {}",
        right.rotation[1]
    );
    assert_eq!(
        right.rotation[2], 0.0,
        "the spyglass arm skips the idle bob"
    );

    // Off-hand spyglass raises the LEFT arm with the mirrored yaw.
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().using_spyglass = true;
            s.use_item_off_hand = true;
        }),
    );
    let left = off.child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (pitch_rad - 1.9198622).clamp(-2.4, 3.3)).abs() < 1e-6,
        "the left arm pitches up to the eye: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (yaw_rad + PI / 12.0)).abs() < 1e-6,
        "the left arm yaws +π/12 off the head"
    );

    // Crouching adds the crouch term to the clamp AND the crouch block's +0.4 on top (pose runs first).
    let mut crouch = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().using_spyglass = true;
            s.is_crouching = true;
        }),
    );
    let crouch_right_x = crouch.child_mut("right_arm").pose.rotation[0];
    let expected = (pitch_rad - 1.9198622 - PI / 12.0).clamp(-2.4, 3.3) + 0.4;
    assert!(
        (crouch_right_x - expected).abs() < 1e-6,
        "a crouching spyglass arm clamps with the crouch term, then gains +0.4: {crouch_right_x} vs {expected}"
    );
}

#[test]
fn player_holding_an_item_lowers_the_main_arm() {
    use std::f32::consts::PI;

    // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM` (`HumanoidModel.poseRightArm` ITEM case): a player
    // holding a plain main-hand item lowers/halves the walk swing, then the shared idle bob is added:
    // `xRot = walkXRot · 0.5 − π/10 + bobX`. A nonzero age proves bob is not incorrectly halved.
    let age = 11.0_f32;
    let walk_pos = 0.7_f32;
    let walk_speed = 0.8_f32;
    let base = EntityModelInstance::player(930, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -10.0))
        .with(|s| s.age_in_ticks = age)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (walk_pos, walk_speed));
    let walk_x = (walk_pos * 0.6662 + PI).cos() * walk_speed;
    let bob_x = (age * 0.067).sin() * 0.05;
    let expected_x = walk_x * 0.5 - PI / 10.0 + bob_x;
    let mut held = player_model(
        false,
        &base.with(|s| s.kind.player_mut().main_hand_item_pose = true),
    );
    let right = held.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - expected_x).abs() < 1e-6,
        "the right arm halves walk before adding bob: {} vs {expected_x}",
        right.rotation[0]
    );
    assert_eq!(right.rotation[1], 0.0, "the ITEM pose zeroes the arm yaw");

    // The empty-handed arm keeps the full walk swing before the same bob, proving ITEM performed the
    // half-and-lower transform rather than merely inheriting the unposed arm.
    let mut idle = player_model(false, &base);
    let idle_x = idle.child_mut("right_arm").pose.rotation[0];
    assert!(
        (idle_x - (walk_x + bob_x)).abs() < 1e-6,
        "the empty hand keeps the full walk swing before bob: {idle_x}"
    );
    assert!((idle_x - right.rotation[0]).abs() > 1e-3);

    // The main-hand ITEM pose is right-arm only; the left arm is untouched.
    let mut held_left = player_model(
        false,
        &base.with(|s| s.kind.player_mut().main_hand_item_pose = true),
    );
    let left = held_left.child_mut("left_arm").pose;
    let mut bare_left = player_model(false, &base);
    assert_eq!(
        left.rotation,
        bare_left.child_mut("left_arm").pose.rotation,
        "the main-hand ITEM pose leaves the off (left) arm alone"
    );
}

fn player_texture_images() -> Vec<EntityModelTextureImage> {
    texture_refs_matching(|path| path.starts_with("textures/entity/player/"))
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn steve_player_texture_images() -> Vec<EntityModelTextureImage> {
    [PLAYER_WIDE_STEVE_TEXTURE_REF, PLAYER_SLIM_STEVE_TEXTURE_REF]
        .into_iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(texture, vec![index as u8; len])
        })
        .collect()
}

fn steve_and_elytra_texture_images() -> Vec<EntityModelTextureImage> {
    [
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        PLAYER_SLIM_STEVE_TEXTURE_REF,
        ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect()
}

fn steve_and_riptide_texture_images() -> Vec<EntityModelTextureImage> {
    [
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        PLAYER_SLIM_STEVE_TEXTURE_REF,
        TRIDENT_RIPTIDE_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect()
}

fn steve_and_parrot_texture_images() -> Vec<EntityModelTextureImage> {
    [PLAYER_WIDE_STEVE_TEXTURE_REF, PLAYER_SLIM_STEVE_TEXTURE_REF]
        .into_iter()
        .chain(
            [
                PARROT_RED_BLUE_TEXTURE_REF,
                PARROT_BLUE_TEXTURE_REF,
                PARROT_GREEN_TEXTURE_REF,
                PARROT_YELLOW_BLUE_TEXTURE_REF,
                PARROT_GRAY_TEXTURE_REF,
            ]
            .iter()
            .copied(),
        )
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_player_vertices_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_player_folded_meshes_are_static_cutout_only(meshes);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0; 4]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == instance.render_state.overlay_coords()));
}

fn assert_player_folded_meshes_are_static_cutout_only(meshes: &EntityModelTexturedMeshes) {
    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn dynamic_player_texture_image(
    handle: u64,
    size: [u32; 2],
    base: u8,
) -> DynamicPlayerTextureImage {
    let mut rgba = Vec::new();
    for y in 0..size[1] {
        for x in 0..size[0] {
            rgba.extend_from_slice(&[base + x as u8, x as u8, y as u8, 255]);
        }
    }
    DynamicPlayerTextureImage {
        handle,
        size,
        standalone_sampling: None,
        rgba,
    }
}

fn atlas_pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = rgba_offset(width, y, x, "test atlas pixel").unwrap();
    [
        rgba[offset],
        rgba[offset + 1],
        rgba[offset + 2],
        rgba[offset + 3],
    ]
}

#[test]
fn player_model_part_masks_match_the_protocol_owner() {
    // The renderer does not depend on `bbb-protocol` at build time, so this
    // projection is kept beside the models it drives. `bbb-protocol` owns the
    // same layout for `ClientInformation.displayed_skin_parts`, which the
    // dedicated server reads; this guard keeps the two from drifting apart
    // instead of letting the layering boundary create a second truth.
    type Owner = bbb_protocol::player_model_parts::PlayerModelPartVisibility;
    assert_eq!(PlayerModelPartVisibility::CAPE_MASK, Owner::CAPE_MASK);
    assert_eq!(PlayerModelPartVisibility::JACKET_MASK, Owner::JACKET_MASK);
    assert_eq!(
        PlayerModelPartVisibility::LEFT_SLEEVE_MASK,
        Owner::LEFT_SLEEVE_MASK
    );
    assert_eq!(
        PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
        Owner::RIGHT_SLEEVE_MASK
    );
    assert_eq!(
        PlayerModelPartVisibility::LEFT_PANTS_MASK,
        Owner::LEFT_PANTS_MASK
    );
    assert_eq!(
        PlayerModelPartVisibility::RIGHT_PANTS_MASK,
        Owner::RIGHT_PANTS_MASK
    );
    assert_eq!(PlayerModelPartVisibility::HAT_MASK, Owner::HAT_MASK);
    assert_eq!(PlayerModelPartVisibility::ALL_MASK, Owner::ALL_MASK);
    for mask in 0..=u8::MAX {
        let renderer = PlayerModelPartVisibility::from_vanilla_mask(mask);
        let owner = Owner::from_vanilla_mask(mask);
        assert_eq!(renderer.hat, owner.hat, "mask {mask:#04x}");
        assert_eq!(
            renderer.vanilla_mask(),
            owner.vanilla_mask(),
            "mask {mask:#04x}"
        );
    }
}
