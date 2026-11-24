use super::*;

#[test]
fn axolotl_adult_body_turns_toward_the_look_yaw() {
    // Vanilla `AdultAxolotlModel.setupAnim` turns the whole body toward the look:
    // `body.yRot += yRot·π/180`, unconditionally and before the procedural land/water sways. The body
    // is the root part, so the yaw rotates every vertex about the body pivot.
    let adult_rest = entity_model_mesh(&[EntityModelInstance::axolotl(
        82,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        AxolotlModelVariant::Lucy,
    )]);
    let adult_yawed = entity_model_mesh(&[EntityModelInstance::axolotl(
        83,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        AxolotlModelVariant::Lucy,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (40.0, 0.0))]);
    assert_eq!(adult_rest.vertices.len(), adult_yawed.vertices.len());
    assert_ne!(
        adult_rest.vertices, adult_yawed.vertices,
        "the adult body turns with the look yaw"
    );

    // The pitch feeds only the swimming sub-animation's body tilt, which is gated off here (the
    // in-water / moving factors default to 0), so a pure-pitch look leaves the body at rest.
    let adult_pitched = entity_model_mesh(&[EntityModelInstance::axolotl(
        84,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        AxolotlModelVariant::Lucy,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -25.0))]);
    assert_eq!(
        adult_rest.vertices, adult_pitched.vertices,
        "pitch alone does not turn the adult body"
    );

    // The baby model (entirely keyframe-driven, no `body.yRot += yRot`) ignores the look.
    let baby_rest = entity_model_mesh(&[EntityModelInstance::axolotl(
        85,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        AxolotlModelVariant::Lucy,
    )]);
    let baby_looked = entity_model_mesh(&[EntityModelInstance::axolotl(
        86,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        AxolotlModelVariant::Lucy,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (40.0, -25.0))]);
    assert_eq!(
        baby_rest.vertices, baby_looked.vertices,
        "the baby axolotl ignores the look"
    );
}

#[test]
fn axolotl_textured_render_matches_vanilla_renderer() {
    // `AxolotlRenderer.TEXTURE_BY_TYPE`: the five `Axolotl.Variant` colours × {adult, baby}.
    let variants = [
        AxolotlModelVariant::Lucy,
        AxolotlModelVariant::Wild,
        AxolotlModelVariant::Gold,
        AxolotlModelVariant::Cyan,
        AxolotlModelVariant::Blue,
    ];
    for variant in variants {
        for baby in [false, true] {
            let texture = axolotl_texture_ref(variant, baby);
            let passes = axolotl_textured_layer_passes(variant, baby);
            assert_eq!(passes.len(), 1);
            assert_eq!(
                passes[0].render_type,
                EntityModelLayerRenderType::EntityCutout
            );
            assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
            assert_eq!(passes[0].texture, texture);
            assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
            assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
            assert_eq!(passes[0].order, 0);
            assert!(entity_model_texture_refs().contains(&texture));
        }
    }
    assert_eq!(AxolotlModelVariant::from_id(-1), AxolotlModelVariant::Lucy);
    assert_eq!(AxolotlModelVariant::from_id(5), AxolotlModelVariant::Lucy);
    for (id, variant) in variants.into_iter().enumerate() {
        assert_eq!(variant.vanilla_id(), id as i32);
    }

    let images: Vec<EntityModelTextureImage> =
        texture_refs_matching(|path| path.starts_with("textures/entity/axolotl/"))
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    for baby in [false, true] {
        let instance = EntityModelInstance::axolotl(
            900,
            [0.0, 64.0, 0.0],
            0.0,
            baby,
            AxolotlModelVariant::Cyan,
        )
        .with(|s| {
            s.light_coords = (4_u32 << 4) | (13_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());

        let mesh = &meshes.cutout;

        assert!(
            !mesh.vertices.is_empty(),
            "baby={baby} emits textured geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        assert!(mesh.vertices.iter().all(|vertex| vertex.light
            == instance.render_state.shader_light()
            && vertex.overlay == instance.render_state.overlay_coords()));
    }
}
