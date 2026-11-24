use super::*;

use crate::entity_models::model_layers::{bell_model, bell_shake_base_rotation};
use crate::entity_models::transforms::bell_model_root_transform;
use glam::{Mat4, Vec3};

fn bell_instance(
    position: [f32; 3],
    ticks: f32,
    direction: Option<BellShakeDirection>,
) -> EntityModelInstance {
    EntityModelInstance::bell(-1, position).with_block_entity(|s| {
        s.bell_ticks = ticks;
        s.bell_shake_direction = direction;
    })
}

#[test]
fn bell_shake_angle_matches_vanilla_formula() {
    // Vanilla `BellModel.setupAnim`: `Mth.sin(ticks / π) / (4 + ticks / 3)`, hand-computed:
    // ticks 0 -> 0; ticks 10 -> sin(3.1831...)/7.3333... = -0.00565831;
    // ticks 25 -> sin(7.9577...)/12.3333... = 0.08064496.
    assert_eq!(bell_shake_base_rotation(0.0), 0.0);
    assert!((bell_shake_base_rotation(10.0) - (-0.005_658_3)).abs() < 1e-6);
    assert!((bell_shake_base_rotation(25.0) - 0.080_645_0).abs() < 1e-6);

    // Axis mapping: NORTH/SOUTH swing xRot (∓/±), EAST/WEST swing zRot (∓/±).
    let body_rotation = |ticks: f32, direction: Option<BellShakeDirection>| {
        let model = bell_model(&bell_instance([0.0, 64.0, 0.0], ticks, direction));
        model.try_child("bell_body").unwrap().pose.rotation
    };
    let base_10 = bell_shake_base_rotation(10.0);
    let base_25 = bell_shake_base_rotation(25.0);
    assert_eq!(
        body_rotation(10.0, Some(BellShakeDirection::North)),
        [-base_10, 0.0, 0.0]
    );
    assert_eq!(
        body_rotation(10.0, Some(BellShakeDirection::South)),
        [base_10, 0.0, 0.0]
    );
    assert_eq!(
        body_rotation(25.0, Some(BellShakeDirection::East)),
        [0.0, 0.0, -base_25]
    );
    assert_eq!(
        body_rotation(25.0, Some(BellShakeDirection::West)),
        [0.0, 0.0, base_25]
    );
    // DOWN/UP are wire-representable but rotate nothing (the vanilla switch skips them),
    // and a resting bell (no shake direction) stays still regardless of ticks.
    assert_eq!(
        body_rotation(10.0, Some(BellShakeDirection::Down)),
        [0.0; 3]
    );
    assert_eq!(body_rotation(10.0, Some(BellShakeDirection::Up)), [0.0; 3]);
    assert_eq!(body_rotation(10.0, None), [0.0; 3]);
    // The base lip keeps its bind pose and swings only through its parent.
    let model = bell_model(&bell_instance(
        [0.0, 0.0, 0.0],
        10.0,
        Some(BellShakeDirection::North),
    ));
    let body = model.try_child("bell_body").unwrap();
    assert_eq!(body.try_child("bell_base").unwrap().pose.rotation, [0.0; 3]);
}

#[test]
fn bell_transform_is_the_block_corner_translation() {
    // Vanilla `BellRenderer.submit` applies no pose-stack transform: the bell body renders in
    // block-local space for every attachment (the support frame is part of the `bell_*` block
    // models). The swing pivot offset(8,12,8) sits at the block's (0.5, 0.75, 0.5).
    let instance = bell_instance([10.0, 5.0, -3.0], 0.0, None);
    let transform = bell_model_root_transform(&instance);
    assert_eq!(
        transform,
        Mat4::from_translation(Vec3::new(10.0, 5.0, -3.0))
    );
    assert_eq!(instance.render_state.body_rot, 0.0);
}

#[test]
fn bell_layer_passes_match_vanilla_renderer() {
    let passes = bell_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    // Vanilla `BellModel`'s constructor picks `RenderTypes::entitySolid`.
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntitySolid
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entitySolid");
    assert_eq!(passes[0].texture, BELL_BODY_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn bell_textured_mesh_bakes_two_boxes_into_the_solid_bucket() {
    let images: Vec<EntityModelTextureImage> = [BELL_BODY_TEXTURE_REF]
        .iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![7; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = bell_instance([3.0, 4.0, 5.0], 10.0, Some(BellShakeDirection::West))
        .with(|s| s.light_coords = (4_u32 << 4) | (12_u32 << 20));
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    // body + base: 2 boxes -> 12 faces / 48 vertices / 72 indices, all in the backface-culled
    // dedicated solid bucket (vanilla `entitySolid`).
    assert_eq!(meshes.solid.cutout_faces, 12);
    assert_eq!(meshes.solid.vertices.len(), 48);
    assert_eq!(meshes.solid.indices.len(), 72);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes
        .solid
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));
}
