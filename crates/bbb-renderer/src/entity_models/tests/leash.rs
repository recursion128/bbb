use super::*;

use glam::{Mat4, Vec3};

fn blank_texture(texture: EntityModelTextureRef) -> EntityModelTextureImage {
    let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
    EntityModelTextureImage::new(texture, vec![0; len])
}

fn single_texture_atlas() -> EntityModelTextureAtlasLayout {
    let (atlas, _) =
        build_entity_model_texture_atlas(&[blank_texture(GUARDIAN_TEXTURE_REF)]).unwrap();
    atlas
}

#[test]
fn leashed_entity_emits_vanilla_two_strip_rope_mesh() {
    // Vanilla `LeashFeatureRenderer.renderLeash`: the rope is one `RenderTypes.leash()`
    // `TRIANGLE_STRIP` of two 25-pair passes over the entity pose translated by the leash offset.
    // The mesh unrolls that strip into an indexed triangle list over the same 100-vertex stream.
    let atlas = single_texture_atlas();
    let position = [10.0, 64.0, -5.0];
    let rope = LeashRopeRenderState {
        offset: [0.0, 0.459, 0.4],
        delta: [4.0, 1.0, 0.0],
        start_block_light: 3,
        start_sky_light: 10,
        end_block_light: 9,
        end_sky_light: 14,
        slack: true,
    };
    let instance = EntityModelInstance::pig(700, position, 0.0, PigModelVariant::Temperate, false)
        .with_leash_ropes(EntityLeashRopesRenderState::from_ropes(&[rope]));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.custom_submissions.len(), 1);
    let submit = meshes.custom_submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::Leash);
    assert_eq!(submit.render_type.vanilla_name(), "leash");
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert_eq!(
        submit.transform,
        Mat4::from_translation(Vec3::from_array(position) + Vec3::from_array(rope.offset))
    );

    // 2 passes × 25 pairs × 2 vertices; strip triangle `i` unrolls to `(i, i+1, i+2)`.
    assert_eq!(meshes.leash.vertices.len(), 100);
    assert_eq!(meshes.leash.indices.len(), 98 * 3);
    assert_eq!(&meshes.leash.indices[0..6], &[0, 1, 2, 1, 2, 3]);
    assert_eq!(&meshes.leash.indices[97 * 3..], &[97, 98, 99]);

    // `invSqrt(4² + 0²) * 0.05 / 2 = 0.00625`; `dxOff = dz·f = 0`, `dzOff = dx·f = 0.025`.
    let base = [10.0, 64.459, -4.6];
    // Forward pass, step 0: first vertex `(x − dxOff, y + 0.05, z + dzOff)`.
    assert_close3(
        meshes.leash.vertices[0].position,
        [base[0], base[1] + 0.05, base[2] + 0.025],
    );
    // Second vertex `(x + dxOff, y + 0.05 − 0.05, z − dzOff)`.
    assert_close3(
        meshes.leash.vertices[1].position,
        [base[0], base[1], base[2] - 0.025],
    );
    // Slack sag with `dy > 0`: `y = dy·t²`, so the halfway step (k=12) sits at a quarter height.
    assert_close3(
        meshes.leash.vertices[24].position,
        [base[0] + 2.0, base[1] + 0.25 + 0.05, base[2] + 0.025],
    );
    // Forward pass end (k=24): full delta plus the forward fudge.
    assert_close3(
        meshes.leash.vertices[48].position,
        [base[0] + 4.0, base[1] + 1.0 + 0.05, base[2] + 0.025],
    );
    // Backward pass starts at k=24 with fudge 0: the first vertex drops the width offset.
    assert_close3(
        meshes.leash.vertices[50].position,
        [base[0] + 4.0, base[1] + 1.0, base[2] + 0.025],
    );

    // Alternating colour: forward even steps take the 0.7 modifier, backward ODD steps do.
    assert_eq!(
        meshes.leash.vertices[0].color,
        [0.5 * 0.7, 0.4 * 0.7, 0.3 * 0.7, 1.0]
    );
    assert_eq!(meshes.leash.vertices[2].color, [0.5, 0.4, 0.3, 1.0]);
    assert_eq!(meshes.leash.vertices[50].color, [0.5, 0.4, 0.3, 1.0]);

    // Endpoint lights lerp with vanilla's `(int)` truncation before the `/15` normalization.
    assert_close2(meshes.leash.vertices[0].light, [3.0 / 15.0, 10.0 / 15.0]);
    assert_close2(meshes.leash.vertices[48].light, [9.0 / 15.0, 14.0 / 15.0]);
    // Step 1: `trunc(3 + 6/24) = 3`, `trunc(10 + 4/24) = 10`.
    assert_close2(meshes.leash.vertices[2].light, [3.0 / 15.0, 10.0 / 15.0]);
}

#[test]
fn quad_leash_emits_four_taut_ropes_and_survives_vertical_spans() {
    // A ghast-to-boat quad connection carries four slack-free ropes; a perfectly vertical span
    // (vanilla `invSqrt(0)`) must not poison the cross-section offsets with NaN.
    let atlas = single_texture_atlas();
    let rope = LeashRopeRenderState {
        offset: [0.0, 0.0, 0.0],
        delta: [0.0, 2.0, 0.0],
        start_block_light: 15,
        start_sky_light: 15,
        end_block_light: 15,
        end_sky_light: 15,
        slack: false,
    };
    let ropes = EntityLeashRopesRenderState::from_ropes(&[rope; 4]).unwrap();
    assert_eq!(ropes.ropes().len(), 4);
    let instance =
        EntityModelInstance::boat(701, [0.0, 70.0, 0.0], 0.0, BoatModelFamily::Oak, false)
            .with_leash_ropes(Some(ropes));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.custom_submissions.len(), 4);
    assert!(meshes
        .custom_submissions
        .iter()
        .all(|submit| submit.render_type == EntityModelLayerRenderType::Leash));
    assert_eq!(meshes.leash.vertices.len(), 400);
    assert_eq!(meshes.leash.indices.len(), 4 * 98 * 3);
    // Taut rope: `y = dy·t` (linear), and the degenerate cross-section keeps x/z at the origin.
    assert!(meshes
        .leash
        .vertices
        .iter()
        .all(|vertex| vertex.position[0] == 0.0 && vertex.position[2] == 0.0));
    assert_close3(meshes.leash.vertices[24].position, [0.0, 71.0 + 0.05, 0.0]);
    assert!(meshes
        .leash
        .vertices
        .iter()
        .all(|vertex| vertex.position.iter().all(|value| value.is_finite())));
}

#[test]
fn hidden_invisible_entity_still_emits_its_leash_rope() {
    // Vanilla `EntityRenderer.submit` submits leash states before any model work and without an
    // invisibility gate, so a fully hidden invisible entity keeps its rope.
    let atlas = single_texture_atlas();
    let rope = LeashRopeRenderState {
        offset: [0.0, 0.5, 0.0],
        delta: [1.0, 0.0, 1.0],
        start_block_light: 0,
        start_sky_light: 15,
        end_block_light: 0,
        end_sky_light: 15,
        slack: true,
    };
    let instance = EntityModelInstance::pig(
        702,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    )
    .with_invisible(true)
    .with_invisible_to_player(true)
    .with_leash_ropes(EntityLeashRopesRenderState::from_ropes(&[rope]));

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.leash.vertices.len(), 100);
    assert_eq!(meshes.custom_submissions.len(), 1);
    assert_eq!(
        meshes.custom_submissions[0].render_type,
        EntityModelLayerRenderType::Leash
    );
}
