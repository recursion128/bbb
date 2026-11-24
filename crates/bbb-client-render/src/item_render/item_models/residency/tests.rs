use super::*;
use crate::item_render::{SpecialItemModel, SpecialItemTexture};

const SPECIAL_TRIDENT_TEXTURE: crate::item_render::EntityModelTextureRef =
    crate::item_render::EntityModelTextureRef {
        path: "textures/entity/test_special_trident.png",
        size: [64, 64],
    };
fn quad(translucent: bool) -> ItemModelQuad {
    ItemModelQuad {
        corners: [
            [0.0, 0.0, 16.0],
            [16.0, 0.0, 16.0],
            [16.0, 16.0, 16.0],
            [0.0, 16.0, 16.0],
        ],
        uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        tint: [1.0; 4],
        normal: [0.0, 0.0, 1.0],
        shade: 1.0,
        translucent,
    }
}

fn identity(name: &str) -> ItemModelAssetIdentity {
    ItemModelAssetIdentity::new(Arc::<str>::from(name), Arc::<str>::from("plain"), 0)
}

fn block_frame(transform: Mat4) -> ItemModelResidencyFrame {
    let identity = identity("minecraft:stone");
    ItemModelResidencyFrame {
        assets: vec![ItemModelAssetRequest {
            identity: identity.clone(),
            atlas: ItemModelAtlasKind::Blocks,
            geometry_class: ItemModelGeometryClass::Solid,
            source: ItemModelAssetSource::Quads(Arc::from([quad(false)])),
        }],
        instances: vec![ItemModelInstanceRequest {
            identity,
            atlas: ItemModelAtlasKind::Blocks,
            geometry_class: ItemModelGeometryClass::Solid,
            transform,
            tint: [1.0; 4],
            light: [1.0; 2],
            overlay: [0.0, 10.0],
            foil: ItemModelFoil::None,
            target: ItemModelDrawTarget::FirstPerson,
            phase: ItemModelDrawPhase::Solid,
            submit_order: 0,
            scissor: None,
        }],
        special_items: Vec::new(),
    }
}

#[test]
fn resident_geometry_filters_stably_and_keeps_decal_before_portal_metadata() {
    let mut first = quad(false);
    first.tint = [1.0, 0.5, 0.25, 0.75];
    first.normal = [0.25, 0.5, 0.75];
    first.shade = 0.4;
    let mut skipped = quad(true);
    skipped.tint = [0.25, 0.5, 1.0, 0.5];
    let mut last = quad(false);
    last.corners.reverse();
    for corner in &mut last.corners {
        corner[0] += 16.0;
    }
    last.normal = [0.0; 3];
    last.shade = 0.6;
    let quads = vec![first, skipped, last];
    let (plain, indices) = resident_geometry(quads.clone(), ItemModelGeometryClass::Solid, false);
    let (portal, portal_indices) =
        resident_geometry(quads.clone(), ItemModelGeometryClass::Solid, true);
    let (translucent, translucent_indices) =
        resident_geometry(quads, ItemModelGeometryClass::Translucent, false);
    assert_eq!(indices, [0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6]);
    assert_eq!(portal_indices, indices);
    assert_eq!(plain.len(), 8);
    assert_eq!(plain[0].position, [0.0, 0.0, 1.0]);
    assert_eq!(plain[4].position, [1.0, 1.0, 1.0]);
    assert!(plain[..4].iter().all(|vertex| vertex.color == first.tint));
    assert!(plain[4..]
        .iter()
        .all(|vertex| vertex.normal_diffuse == [0.0, 0.0, 1.0, 1.0]));
    assert!(portal[..4]
        .iter()
        .all(|vertex| vertex.normal_diffuse == [0.25, 0.5, 0.75, 0.4]));
    assert!(portal[4..]
        .iter()
        .all(|vertex| vertex.normal_diffuse == [0.0, 0.0, 0.0, 0.6]));
    for (plain, portal) in plain.iter().zip(&portal) {
        assert_eq!(portal.position, plain.position);
        assert_eq!(portal.uv, plain.uv);
        assert_eq!(portal.color, plain.color);
        assert_eq!(portal.decal_uv, plain.decal_uv);
    }
    assert_eq!(translucent_indices, [0, 1, 2, 0, 2, 3]);
    assert_eq!(translucent.len(), 4);
    assert!(translucent
        .iter()
        .all(|vertex| vertex.color == skipped.tint));
}

#[test]
fn block_feature_target_uses_the_world_depth_lane_without_gui_clipping() {
    assert!(!ItemModelDrawTarget::BlockFeature.is_gui());
    assert_ne!(
        ItemModelDrawTarget::BlockFeature,
        ItemModelDrawTarget::World
    );
}

#[test]
fn resident_item_model_entity_outline_target_and_phase_are_explicit_metadata() {
    assert!(!ItemModelDrawTarget::EntityOutline.is_gui());
    assert_ne!(
        ItemModelDrawTarget::EntityOutline,
        ItemModelDrawTarget::BlockFeature
    );
    assert_ne!(ItemModelDrawPhase::Outline, ItemModelDrawPhase::Solid);
    assert_ne!(ItemModelDrawPhase::Outline, ItemModelDrawPhase::Translucent);
}

#[test]
fn smithing_entity_preview_target_is_indexed_and_not_a_gui_scissor_lane() {
    let first = ItemModelDrawTarget::EntityPreview(0);
    let second = ItemModelDrawTarget::EntityPreview(1);
    assert!(!first.is_gui());
    assert_ne!(first, second);
    assert_ne!(first, ItemModelDrawTarget::FirstPerson);
    assert_ne!(first, ItemModelDrawTarget::Gui);
}

#[test]
fn resident_solid_no_cutout_shader_matches_vanilla_solid_block() {
    let shader = resident_no_alpha_cutout_shader();
    assert!(!shader.contains("discard"));
    assert!(shader.contains("textureSample(item_atlas, item_sampler, input.uv)"));
}

#[test]
fn resident_item_model_diffuse_shaders_split_cull_and_no_cull_normals() {
    assert!(ITEM_MODEL_RESIDENT_SHADER.contains(
        "let diffuse = mix(1.0, diffuse_light(input.normal_diffuse.xyz), input.normal_diffuse.w)"
    ));
    assert!(!ITEM_MODEL_RESIDENT_SHADER.contains("@builtin(front_facing)"));

    let no_cull = resident_per_face_shader();
    assert!(no_cull.contains("@builtin(front_facing) front_facing: bool"));
    assert!(no_cull.contains(
        "let face_normal = select(-input.normal_diffuse.xyz, input.normal_diffuse.xyz, front_facing)"
    ));
    assert!(no_cull
        .contains("let diffuse = mix(1.0, diffuse_light(face_normal), input.normal_diffuse.w)"));
    for shader in [ITEM_MODEL_RESIDENT_SHADER, no_cull.as_str()] {
        assert!(shader.contains("let light0 = normalize(camera.minecraft_light0.xyz)"));
        assert!(shader.contains("let light1 = normalize(camera.minecraft_light1.xyz)"));
        assert!(shader.contains("(light_value.x + light_value.y) * 0.6 + 0.4"));
    }
}

#[test]
fn resident_item_model_outline_shader_matches_vanilla_blocks_atlas_outline() {
    assert!(ITEM_MODEL_RESIDENT_OUTLINE_SHADER.contains("if (texel.a == 0.0)"));
    assert!(ITEM_MODEL_RESIDENT_OUTLINE_SHADER.contains("return vec4<f32>(input.tint.rgb, 1.0)"));
    assert!(!ITEM_MODEL_RESIDENT_OUTLINE_SHADER.contains("lightmap"));
    assert!(!ITEM_MODEL_RESIDENT_OUTLINE_SHADER.contains("overlay"));
    assert!(!ITEM_MODEL_RESIDENT_OUTLINE_SHADER.contains("fog"));
}

#[test]
fn resident_item_model_outline_pipeline_matches_vanilla_outline_no_cull_state() {
    assert_eq!(ITEM_MODEL_RESIDENT_OUTLINE_BLEND, None);
    assert_eq!(ITEM_MODEL_RESIDENT_OUTLINE_CULL_MODE, None);
    const {
        assert!(ITEM_MODEL_RESIDENT_OUTLINE_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ITEM_MODEL_RESIDENT_OUTLINE_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual
    );
}

#[test]
fn residency_frame_append_preserves_ordinary_special_submit_order() {
    let mut target = block_frame(Mat4::IDENTITY);
    let mut source = block_frame(Mat4::from_translation(glam::Vec3::X));
    source
        .special_items
        .push(super::super::SpecialItemInstanceRequest {
            model: SpecialItemModel::Trident {
                texture: SpecialItemTexture::Static(SPECIAL_TRIDENT_TEXTURE),
            },
            transform: Mat4::IDENTITY,
            light: [1.0; 2],
            overlay: [0.0, 10.0],
            foil: ItemModelFoil::None,
            target: ItemModelDrawTarget::World,
            outline_color: None,
            submit_order: 1,
            scissor: None,
        });

    target.append(source);

    assert_eq!(
        target
            .instances
            .iter()
            .map(|instance| instance.submit_order)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(target.special_items[0].submit_order, 2);
}

#[test]
fn resident_scissor_rejects_empty_and_clips_to_attachment_bounds() {
    assert_eq!(ItemModelScissorRect::new(0, 0, 0, 1), None);
    assert_eq!(ItemModelScissorRect::new(0, 0, 1, 0), None);
    assert_eq!(
        ItemModelScissorRect::new(6, 7, 8, 9).and_then(|rect| rect.clipped_to(10, 12)),
        ItemModelScissorRect::new(6, 7, 4, 5)
    );
    assert_eq!(
        ItemModelScissorRect::new(10, 0, 1, 1).and_then(|rect| rect.clipped_to(10, 12)),
        None
    );
}

#[test]
fn arena_coalesces_adjacent_frees_and_adjacent_writes() {
    let mut arena = ItemArenaBuffer::new("test", wgpu::BufferUsages::INDEX, 4, u32::MAX);
    let a = arena.allocate(6).unwrap();
    let b = arena.allocate(12).unwrap();
    let c = arena.allocate(6).unwrap();
    assert_eq!((a, b, c, arena.used_end), (0, 6, 18, 24));
    arena.queue_write(a, &[1; 24]);
    arena.queue_write(b, &[2; 48]);
    arena.queue_write(c, &[3; 24]);
    assert_eq!(arena.pending_writes.len(), 1);
    assert_eq!(arena.pending_writes[0].bytes.len(), 96);

    arena.free(a, 6);
    arena.free(b, 12);
    assert_eq!(arena.allocate(18), Some(0));
    arena.free(18, 6);
    arena.free(0, 18);
    assert_eq!(arena.used_end, 0);
    assert!(arena.free_by_start.is_empty());
}
