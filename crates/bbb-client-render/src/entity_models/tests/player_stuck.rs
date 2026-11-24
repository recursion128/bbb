use super::*;

fn player_stuck_atlas() -> EntityModelTextureAtlasLayout {
    let images = [
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        PLAYER_SLIM_STEVE_TEXTURE_REF,
        ARROW_TEXTURE_REF,
        BEE_STINGER_TEXTURE_REF,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(texture, vec![index as u8; len])
    })
    .collect::<Vec<_>>();
    build_entity_model_texture_atlas(&images).unwrap().0
}

fn wide_player(entity_id: i32) -> EntityModelInstance {
    EntityModelInstance::player_with_skin(
        entity_id,
        [2.0, 65.0, -3.0],
        35.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
}

#[test]
fn player_stuck_layers_submit_deterministic_posed_arrow_and_surface_stinger() {
    let atlas = player_stuck_atlas();
    let instance = wide_player(3)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.25, 0.7))
        .with(|s| {
            s.age_in_ticks = 4.5;
            s.kind.player_mut().arrow_count = 2;
            s.kind.player_mut().stinger_count = 1;
            s.light_coords = (7_u32 << 4) | (9_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        })
        .with_outline_color(0xff12_3456);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let arrow = entity_model_textured_meshes(
        &[instance.with(|s| {
            s.kind.player_mut().arrow_count = 1;
            s.kind.player_mut().stinger_count = 0;
        })],
        &atlas,
    );
    let stinger = entity_model_textured_meshes(
        &[instance.with(|s| {
            s.kind.player_mut().arrow_count = 0;
            s.kind.player_mut().stinger_count = 1;
        })],
        &atlas,
    );
    let arrow_len = arrow.cutout_cull.vertices.len();
    assert!(arrow_len > 0 && !stinger.cutout_cull.vertices.is_empty());
    assert_eq!(
        meshes.cutout_cull.vertices.len(),
        arrow_len * 2 + stinger.cutout_cull.vertices.len()
    );
    assert_ne!(
        &meshes.cutout_cull.vertices[..arrow_len],
        &meshes.cutout_cull.vertices[arrow_len..arrow_len * 2]
    );
    assert!(meshes.cutout_cull.vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light()
            && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY
    }));
    for (vertices, texture) in [
        (
            &meshes.cutout_cull.vertices[..arrow_len * 2],
            ARROW_TEXTURE_REF,
        ),
        (
            &meshes.cutout_cull.vertices[arrow_len * 2..],
            BEE_STINGER_TEXTURE_REF,
        ),
    ] {
        let uv = atlas
            .entries
            .iter()
            .find(|entry| entry.texture == texture)
            .unwrap()
            .uv;
        assert!(vertices.iter().all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
    }

    let repeated = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(meshes.cutout_cull.vertices, repeated.cutout_cull.vertices);
}

#[test]
fn player_stuck_arrow_follows_the_selected_posed_left_arm() {
    // Entity id 3 selects bodyParts[2] = leftArm on its first Java nextInt(6).
    let atlas = player_stuck_atlas();
    let idle = wide_player(3).with(|s| s.kind.player_mut().arrow_count = 1);
    let walking = idle
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.25, 0.8))
        .with(|s| s.age_in_ticks = 4.5);
    let idle_meshes = entity_model_textured_meshes(&[idle], &atlas);
    let walking_meshes = entity_model_textured_meshes(&[walking], &atlas);
    assert!(!idle_meshes.cutout_cull.vertices.is_empty());
    assert_eq!(
        idle_meshes.cutout_cull.vertices.len(),
        walking_meshes.cutout_cull.vertices.len()
    );
    assert_ne!(
        idle_meshes.cutout_cull.vertices,
        walking_meshes.cutout_cull.vertices
    );
}

#[test]
fn player_stuck_layers_bound_untrusted_counts_and_ignore_non_positive_values() {
    let atlas = player_stuck_atlas();
    let hostile = entity_model_textured_meshes(
        &[wide_player(42).with(|s| {
            s.kind.player_mut().arrow_count = 1_000;
            s.kind.player_mut().stinger_count = 1_000;
        })],
        &atlas,
    );
    let single = entity_model_textured_meshes(
        &[wide_player(42).with(|s| {
            s.kind.player_mut().arrow_count = 1;
            s.kind.player_mut().stinger_count = 1;
        })],
        &atlas,
    );
    assert!(!single.cutout_cull.vertices.is_empty());
    assert_eq!(
        hostile.cutout_cull.vertices.len(),
        single.cutout_cull.vertices.len() * 64
    );

    let non_positive = entity_model_textured_meshes(
        &[wide_player(42).with(|s| {
            s.kind.player_mut().arrow_count = -1;
            s.kind.player_mut().stinger_count = 0;
        })],
        &atlas,
    );
    assert!(!non_positive.cutout.vertices.is_empty());
    assert!(non_positive.cutout_cull.vertices.is_empty());
}

#[test]
fn player_stuck_layers_skip_missing_atlas_geometry() {
    let images = [PLAYER_WIDE_STEVE_TEXTURE_REF]
        .into_iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(texture, vec![0; len])
        })
        .collect::<Vec<_>>();
    let atlas = build_entity_model_texture_atlas(&images).unwrap().0;
    let instance = wide_player(5).with(|s| {
        s.kind.player_mut().arrow_count = 1;
        s.kind.player_mut().stinger_count = 1;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout_cull.vertices.is_empty());
}

#[test]
fn player_stuck_layers_remain_visible_when_the_base_body_has_no_render_type() {
    let atlas = player_stuck_atlas();
    let hidden = wide_player(11)
        .with(|s| {
            s.kind.player_mut().arrow_count = 1;
            s.kind.player_mut().stinger_count = 1;
        })
        .with_invisible(true)
        .with(|s| s.invisible_to_player = true);
    let meshes = entity_model_textured_meshes(&[hidden], &atlas);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(!meshes.cutout_cull.vertices.is_empty());
    let visible = entity_model_textured_meshes(&[hidden.with_invisible(false)], &atlas);
    assert_eq!(meshes.cutout_cull.vertices, visible.cutout_cull.vertices);
}
