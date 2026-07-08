use super::*;

use crate::entity_models::colored::skull_block_model_root_transform;
use crate::entity_models::model_layers::{
    ENDER_DRAGON_TEXTURE_REF, MODEL_LAYER_DRAGON_SKULL, MODEL_LAYER_SKELETON_SKULL,
    SKELETON_TEXTURE_REF,
};
use glam::{Mat4, Vec3};

fn atlas_with(texture: EntityModelTextureRef) -> EntityModelTextureAtlasLayout {
    let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
    build_entity_model_texture_atlas(&[EntityModelTextureImage::new(texture, vec![0; len])])
        .unwrap()
        .0
}

fn skull_instance(
    skull: EntityCustomHeadSkull,
    attachment: SkullBlockModelAttachment,
    y_rot: f32,
) -> EntityModelInstance {
    EntityModelInstance::skull_block(-1, [2.0, 3.0, 4.0], y_rot, skull, attachment)
        .with_worn_head_animation_pos(7.25)
}

#[test]
fn skull_block_model_key_and_texture_refs_match_vanilla_selection() {
    let skeleton = EntityModelKind::SkullBlock {
        skull: EntityCustomHeadSkull::Skeleton,
        attachment: SkullBlockModelAttachment::Ground,
    };
    assert_eq!(skeleton.model_key(), "skull_block");
    assert_eq!(skeleton.vanilla_texture_ref(), Some(SKELETON_TEXTURE_REF));

    let dragon = EntityModelKind::SkullBlock {
        skull: EntityCustomHeadSkull::Dragon,
        attachment: SkullBlockModelAttachment::Wall {
            facing: EntityAttachmentFace::North,
        },
    };
    assert_eq!(dragon.vanilla_texture_ref(), Some(ENDER_DRAGON_TEXTURE_REF));
}

#[test]
fn skull_block_root_transforms_match_vanilla_ground_and_wall() {
    let ground = skull_block_model_root_transform(
        skull_instance(
            EntityCustomHeadSkull::Skeleton,
            SkullBlockModelAttachment::Ground,
            -90.0,
        ),
        SkullBlockModelAttachment::Ground,
    );
    assert_eq!(
        ground,
        Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0))
            * Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
            * Mat4::from_rotation_y((-90.0_f32).to_radians())
            * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
    );

    let wall = skull_block_model_root_transform(
        skull_instance(
            EntityCustomHeadSkull::Skeleton,
            SkullBlockModelAttachment::Wall {
                facing: EntityAttachmentFace::North,
            },
            0.0,
        ),
        SkullBlockModelAttachment::Wall {
            facing: EntityAttachmentFace::North,
        },
    );
    assert_eq!(
        wall,
        Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0))
            * Mat4::from_translation(Vec3::new(0.5, 0.25, 0.75))
            * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
    );

    let east_wall = skull_block_model_root_transform(
        skull_instance(
            EntityCustomHeadSkull::Skeleton,
            SkullBlockModelAttachment::Wall {
                facing: EntityAttachmentFace::East,
            },
            0.0,
        ),
        SkullBlockModelAttachment::Wall {
            facing: EntityAttachmentFace::East,
        },
    );
    assert_eq!(
        east_wall,
        Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0))
            * Mat4::from_translation(Vec3::new(0.25, 0.25, 0.5))
            * Mat4::from_rotation_y((-90.0_f32).to_radians())
            * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
    );
}

#[test]
fn skull_block_textured_mesh_submits_no_overlay_skull_model() {
    let atlas = atlas_with(SKELETON_TEXTURE_REF);
    let instance = skull_instance(
        EntityCustomHeadSkull::Skeleton,
        SkullBlockModelAttachment::Ground,
        0.0,
    )
    .with_light_coords((5_u32 << 4) | (11_u32 << 20));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 1);
    let submission = meshes.submissions[0];
    let pass = custom_head_skull_layer_pass(EntityCustomHeadSkull::Skeleton, SKELETON_TEXTURE_REF);
    assert_eq!(pass.model_layer, MODEL_LAYER_SKELETON_SKULL);
    assert_eq!(submission.render_type.vanilla_name(), "entityCutoutZOffset");
    assert_eq!(submission.texture, SKELETON_TEXTURE_REF);
    assert_eq!(submission.overlay, [0.0, 10.0]);
    assert_eq!(submission.light, instance.render_state.shader_light());
    assert_eq!(meshes.cutout_z_offset.vertices.len(), 24);
    assert!(meshes.translucent.vertices.is_empty());
}

#[test]
fn dragon_skull_block_uses_specialized_model_layer_and_animation_position() {
    let atlas = atlas_with(ENDER_DRAGON_TEXTURE_REF);
    let instance = skull_instance(
        EntityCustomHeadSkull::Dragon,
        SkullBlockModelAttachment::Ground,
        0.0,
    );

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 1);
    let pass =
        custom_head_skull_layer_pass(EntityCustomHeadSkull::Dragon, ENDER_DRAGON_TEXTURE_REF);
    assert_eq!(pass.model_layer, MODEL_LAYER_DRAGON_SKULL);
    assert_eq!(meshes.submissions[0].texture, ENDER_DRAGON_TEXTURE_REF);
    assert!(!meshes.cutout_z_offset.vertices.is_empty());
}
