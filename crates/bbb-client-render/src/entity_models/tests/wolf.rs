use super::*;
use crate::entity_models::model_layers::wolf_model;
use crate::entity_models::wolf_variants::VANILLA_WOLF_VARIANT_TEXTURE_REFS;

const WOLF_TEXTURE_REF: EntityModelTextureRef = VANILLA_WOLF_VARIANT_TEXTURE_REFS[0];
const WOLF_TAME_TEXTURE_REF: EntityModelTextureRef = VANILLA_WOLF_VARIANT_TEXTURE_REFS[1];
const WOLF_ANGRY_TEXTURE_REF: EntityModelTextureRef = VANILLA_WOLF_VARIANT_TEXTURE_REFS[2];
const WOLF_TAME_BABY_TEXTURE_REF: EntityModelTextureRef = VANILLA_WOLF_VARIANT_TEXTURE_REFS[4];

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

use crate::DynamicEntityTextureImage;

// The adult wolf tail bind pose, mirrored from the model file so the tail pose-math tests can run
// without the deleted `ADULT_WOLF_PARTS` const tree. The layer rests the tail at the π/5 wild droop.
const ADULT_WOLF_TAIL_POSE: PartPose = PartPose {
    offset: [-1.0, 12.0, 8.0],
    rotation: [0.62831855, 0.0, 0.0],
};

#[test]
fn wolf_textured_mesh_uses_vanilla_uvs_and_collar_tint() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let wolf = wolf_submission_probe(EntityModelInstance::wolf_state(
        305,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        false,
        Some(EntityDyeColor::Blue),
    ));
    let meshes = entity_model_textured_meshes(&[wolf], &atlas);
    assert_wolf_vertices_match_vanilla(&meshes, wolf);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 132);
    assert_eq!(mesh.vertices.len(), 528);
    assert_eq!(mesh.indices.len(), 792);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        mesh.vertices[264].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
    assert_eq!(mesh.vertices[0].light, wolf.render_state.shader_light());
    assert_eq!(mesh.vertices[264].light, wolf.render_state.shader_light());
    assert_eq!(mesh.vertices[0].overlay, wolf.render_state.overlay_coords());
    assert_eq!(mesh.vertices[264].overlay, [0.0, 10.0]);
    assert_ne!(
        mesh.vertices[264].overlay,
        wolf.render_state.overlay_coords()
    );

    let untamed = EntityModelInstance::wolf_state(
        306,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        false,
        false,
        Some(EntityDyeColor::Red),
    );
    let untamed_with_collar_metadata = entity_model_textured_meshes(&[untamed], &atlas);
    assert_wolf_vertices_match_vanilla(&untamed_with_collar_metadata, untamed);
    assert_eq!(untamed_with_collar_metadata.cutout.cutout_faces, 66);
    assert!(untamed_with_collar_metadata
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let invisible = EntityModelInstance::wolf_state(
        307,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        true,
        Some(EntityDyeColor::Blue),
    );
    let invisible_tame = entity_model_textured_meshes(&[invisible], &atlas);
    assert_wolf_vertices_match_vanilla(&invisible_tame, invisible);
    // An invisible wolf without body armor renders nothing: the base body has no render type and
    // `WolfCollarLayer` gates on `state.isInvisible`.
    assert_eq!(invisible_tame.cutout.cutout_faces, 0);
    assert!(invisible_tame.cutout.vertices.is_empty());

    // Vanilla `LivingEntityRenderer.getRenderType`: an invisible wolf that is still invisible to
    // this client but `appearsGlowing()` submits only the base body with `RenderTypes.outline`.
    // The folded outline bucket uses `outlineColor` for vertex tint, matching
    // `OutlineBufferSource.EntityOutlineGenerator`; the submission still records the original
    // model tint.
    let glowing_invisible = invisible.with_appears_glowing(true).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.has_red_overlay = true;
    });
    let glowing = entity_model_textured_meshes(&[glowing_invisible], &atlas);

    assert!(glowing.cutout.vertices.is_empty());
    assert!(glowing.translucent.vertices.is_empty());
    assert!(glowing.eyes.vertices.is_empty());
    assert_eq!(glowing.outline.cutout_faces, 66);
    assert_eq!(glowing.outline.vertices.len(), 264);
    assert_eq!(glowing.outline.indices.len(), 396);
    assert!(glowing.outline.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == glowing_invisible.render_state.shader_light()
            && vertex.overlay == glowing_invisible.render_state.overlay_coords()
    }));

    // Vanilla `LivingEntityRenderer.getRenderType`: an invisible wolf that remains visible to this
    // client submits only the base body as `entityTranslucentCullItemTarget` with the
    // force-transparent `0x26ffffff` alpha. `WolfCollarLayer` still gates on `state.isInvisible`, so
    // the tame collar is skipped.
    let self_visible_invisible = invisible.with(|s| {
        s.invisible_to_player = false;
        s.light_coords = (6_u32 << 4) | (8_u32 << 20);
        s.has_red_overlay = true;
    });
    let self_visible = entity_model_textured_meshes(&[self_visible_invisible], &atlas);

    assert!(self_visible.cutout.vertices.is_empty());
    assert!(self_visible.translucent.vertices.is_empty());
    assert_eq!(self_visible.item_entity_translucent_cull.cutout_faces, 66);
    assert_eq!(
        self_visible.item_entity_translucent_cull.vertices.len(),
        264
    );
    assert!(self_visible
        .item_entity_translucent_cull
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 38.0 / 255.0]
            && vertex.light == self_visible_invisible.render_state.shader_light()
            && vertex.overlay == self_visible_invisible.render_state.overlay_coords()));
}

#[test]
fn self_visible_invisible_wolf_submission_survives_missing_texture_atlas_entry() {
    // Vanilla `LivingEntityRenderer.getRenderType` records the force-transparent base body submit;
    // missing texture data suppresses only folded translucent geometry.
    let len = usize::try_from(WOLF_TEXTURE_REF.size[0] * WOLF_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        WOLF_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::wolf_state(
        307,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        true,
        Some(EntityDyeColor::Blue),
    )
    .with(|s| {
        s.invisible_to_player = false;
        s.light_coords = (6_u32 << 4) | (8_u32 << 20);
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.item_entity_translucent_cull.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn wolf_collar_submission_survives_missing_texture_atlas_entry() {
    // Vanilla `WolfCollarLayer` records `RenderTypes.entityCutout(collar)` at order(1) with no
    // overlay; missing atlas data suppresses only the folded collar geometry.
    let images = [WOLF_TAME_TEXTURE_REF, WOLF_TAME_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect::<Vec<_>>();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    for (id, baby, collar_texture) in [
        (311, false, WOLF_COLLAR_TEXTURE_REF),
        (312, true, WOLF_BABY_COLLAR_TEXTURE_REF),
    ] {
        let bare = wolf_submission_probe(EntityModelInstance::wolf_state(
            id,
            [0.0, 64.0, 0.0],
            0.0,
            baby,
            true,
            false,
            false,
            None,
        ));
        let bare_meshes = entity_model_textured_meshes(&[bare], &atlas);
        assert_wolf_vertices_match_vanilla(&bare_meshes, bare);

        let collared = wolf_submission_probe(EntityModelInstance::wolf_state(
            id,
            [0.0, 64.0, 0.0],
            0.0,
            baby,
            true,
            false,
            false,
            Some(EntityDyeColor::Yellow),
        ));
        let meshes = entity_model_textured_meshes(&[collared], &atlas);
        assert_wolf_vertices_match_vanilla(&meshes, collared);

        assert_eq!(
            meshes.cutout.vertices,
            bare_meshes.cutout.vertices,
            "missing {path} suppresses only folded collar geometry",
            path = collar_texture.path
        );
        assert_eq!(meshes.cutout.indices, bare_meshes.cutout.indices);
        assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
            == collared.render_state.shader_light()
            && vertex.overlay == collared.render_state.overlay_coords()));
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());
    }
}

#[test]
fn wolf_biome_textures_join_global_atlas() {
    // Every biome face joins the global entity atlas, so a variant wolf resolves at runtime.
    for path in [
        "textures/entity/wolf/wolf_spotted.png",
        "textures/entity/wolf/wolf_striped_angry_baby.png",
        "textures/entity/wolf/wolf_chestnut_tame.png",
    ] {
        assert!(
            entity_model_texture_refs()
                .iter()
                .any(|texture| texture.path == path),
            "missing {path} from the global atlas"
        );
    }
}

#[test]
fn wolf_textured_layer_passes_match_vanilla_renderer_layers() {
    // The vestigial `parts` slices are nulled; every pass reads the unified `WolfModel` tree.
    let wild = wolf_textured_layer_passes(false, false, false, None, WolfModelVariant::Pale, 1.0);
    assert_eq!(wild.len(), 1);
    assert_eq!(wild[0].texture, WOLF_TEXTURE_REF);
    assert_eq!(wild[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(wild[0].order, 0);

    let tame_blue = wolf_textured_layer_passes(
        false,
        true,
        false,
        Some(EntityDyeColor::Blue),
        WolfModelVariant::Pale,
        1.0,
    );
    assert_eq!(tame_blue.len(), 2);
    assert_eq!(tame_blue[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(tame_blue[1].texture, WOLF_COLLAR_TEXTURE_REF);
    assert_eq!(
        tame_blue[1].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
    assert_eq!(tame_blue[1].order, 1);

    // An untamed wolf carrying collar metadata still emits only the base layer: the collar pass
    // is gated on `tame`, so a wild wolf renders no collar.
    let untamed_with_collar = wolf_textured_layer_passes(
        false,
        false,
        false,
        Some(EntityDyeColor::Blue),
        WolfModelVariant::Pale,
        1.0,
    );
    assert_eq!(untamed_with_collar.len(), 1);
    assert_eq!(untamed_with_collar[0].texture, WOLF_TEXTURE_REF);

    let angry = wolf_textured_layer_passes(false, false, true, None, WolfModelVariant::Pale, 1.0);
    assert_eq!(angry[0].texture, WOLF_ANGRY_TEXTURE_REF);
    assert_eq!(angry.len(), 1);

    let tame_angry = wolf_textured_layer_passes(
        false,
        true,
        true,
        Some(EntityDyeColor::Red),
        WolfModelVariant::Pale,
        1.0,
    );
    assert_eq!(tame_angry[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(tame_angry.len(), 2);

    let baby_tame = wolf_textured_layer_passes(
        true,
        true,
        false,
        Some(EntityDyeColor::Red),
        WolfModelVariant::Pale,
        1.0,
    );
    assert_eq!(baby_tame[0].texture, WOLF_TAME_BABY_TEXTURE_REF);
    assert_eq!(baby_tame[1].texture, WOLF_BABY_COLLAR_TEXTURE_REF);

    assert_eq!(ModelLayer::WOLF, ModelLayer::WOLF);
    assert_eq!(ModelLayer::WOLF_BABY, ModelLayer::WOLF_BABY);
}

#[test]
fn wet_wolf_textured_base_tints_like_vanilla_model_tint_without_shading_collar() {
    // Vanilla `WolfRenderer.getModelTint`: `wetShade == 1 ? -1 :
    // ARGB.colorFromFloat(1, wetShade, wetShade, wetShade)`. `LivingEntityRenderer.submit`
    // multiplies that tint into the base model submission; `WolfCollarLayer` is a later
    // order-1 layer with its own dye color.
    let wet_shade = 0.75;
    let passes = wolf_textured_layer_passes(
        false,
        true,
        false,
        Some(EntityDyeColor::Blue),
        WolfModelVariant::Pale,
        wet_shade,
    );
    assert_eq!(passes.len(), 2);
    assert_eq!(passes[0].texture, WOLF_TAME_TEXTURE_REF);
    assert_eq!(passes[0].tint, [wet_shade, wet_shade, wet_shade, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert_eq!(passes[1].texture, WOLF_COLLAR_TEXTURE_REF);
    assert_eq!(passes[1].tint, EntityDyeColor::Blue.texture_diffuse_color());
    assert_eq!(passes[1].order, 1);

    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let meshes = entity_model_textured_meshes(
        &[EntityModelInstance::wolf_state(
            306,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            true,
            false,
            false,
            Some(EntityDyeColor::Blue),
        )
        .with(|s| s.kind.wolf_mut().wet_shade = wet_shade)],
        &atlas,
    );

    assert_eq!(
        meshes.cutout.vertices[0].tint,
        [wet_shade, wet_shade, wet_shade, 1.0]
    );
    assert_eq!(
        meshes.cutout.vertices[264].tint,
        EntityDyeColor::Blue.texture_diffuse_color()
    );
}

#[test]
fn wolf_body_armor_submissions_match_vanilla_equipment_layers() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_armor_texture_images()).unwrap();
    let trim_texture = EntityDynamicTexture { handle: 91_101 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let dye = 0x0033_66CC;
    let dyed_tint = [
        0x33 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xCC as f32 / 255.0,
        1.0,
    ];
    let wolf = EntityModelInstance::wolf_state(
        308,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        false,
        Some(EntityDyeColor::Blue),
    )
    .with(|s| {
        s.kind.wolf_mut().body_equipment_asset = Some(test_wolf_equipment_asset_id());
        s.kind.wolf_mut().body_armor_dye = Some(dye);
        s.kind.wolf_mut().body_armor_crackiness = WolfArmorCrackiness::Medium;
        s.kind.wolf_mut().body_armor_foil = true;
        s.kind.wolf_mut().body_armor_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: false,
        });
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[wolf],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_wolf_armor_vertices(&meshes, wolf, dyed_tint);
    assert_eq!(
        meshes.dynamic_player_texture_armor_trim.vertices.len(),
        264,
        "the trim reuses the posed adult WOLF_ARMOR model"
    );
    assert!(meshes
        .dynamic_player_texture_armor_trim_decal
        .vertices
        .is_empty());

    assert!(
        !meshes.armor_translucent.vertices.is_empty(),
        "armorTranslucent cracks should emit into the dedicated armor translucent bucket"
    );
    assert!(meshes.translucent.vertices.is_empty());

    let undyed = entity_model_textured_meshes(
        &[EntityModelInstance::wolf_state(
            309,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            false,
            false,
            None,
        )
        .with(|s| s.kind.wolf_mut().body_equipment_asset = Some(test_wolf_equipment_asset_id()))],
        &atlas,
    );

    assert_eq!(undyed.armor_cutout.vertices.len(), 264);
    assert!(undyed
        .armor_cutout
        .vertices
        .iter()
        .all(|vertex| { vertex.tint == [1.0; 4] && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY }));
    assert!(undyed.translucent.vertices.is_empty());

    let baby = entity_model_textured_meshes_with_dynamic_textures(
        &[EntityModelInstance::wolf_state(
            310,
            [0.0, 64.0, 0.0],
            0.0,
            true,
            true,
            false,
            false,
            Some(EntityDyeColor::Red),
        )
        .with(|s| {
            s.kind.wolf_mut().body_equipment_asset = Some(test_wolf_equipment_asset_id());
            s.kind.wolf_mut().body_armor_trim = Some(EntityArmorTrim {
                texture: trim_texture,
                decal: false,
            });
            s.kind.wolf_mut().body_armor_dye = Some(dye);
            s.kind.wolf_mut().body_armor_crackiness = WolfArmorCrackiness::High;
        })],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );
    for mesh in [
        &baby.armor_cutout,
        &baby.armor_translucent,
        &baby.dynamic_player_texture_armor_trim,
        &baby.dynamic_player_texture_armor_trim_decal,
    ] {
        assert!(mesh.vertices.is_empty());
    }
    assert!(baby.armor_entity_glint.vertices.is_empty());
}

#[test]
fn invisible_wolf_body_armor_keeps_decal_trim_submission() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_armor_texture_images()).unwrap();
    let trim_texture = EntityDynamicTexture { handle: 91_102 };
    let dynamic_atlas = build_dynamic_player_texture_atlas(&[DynamicEntityTextureImage {
        handle: trim_texture.handle,
        size: [64, 32],
        standalone_sampling: None,
        rgba: vec![0xff; 64 * 32 * 4],
    }])
    .unwrap()
    .0;
    let invisible = EntityModelInstance::wolf_state(
        311,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        false,
        true,
        None,
    )
    .with(|s| {
        s.kind.wolf_mut().body_equipment_asset = Some(test_wolf_equipment_asset_id());
        s.kind.wolf_mut().body_armor_trim = Some(EntityArmorTrim {
            texture: trim_texture,
            decal: true,
        });
    });

    let meshes = entity_model_textured_meshes_with_dynamic_textures(
        &[invisible],
        &atlas,
        None,
        Some(&dynamic_atlas),
    );

    assert_eq!(
        meshes
            .dynamic_player_texture_armor_trim_decal
            .vertices
            .len(),
        264
    );
    assert!(meshes.cutout.vertices.is_empty());
}

#[test]
fn wolf_armor_crack_submission_survives_missing_texture_atlas_entry() {
    // Vanilla `WolfArmorLayer.maybeRenderCracks` submits `armorTranslucent` with no overlay after
    // the armor layers through the root collector's order 0; a missing crack texture suppresses only
    // the folded translucent geometry.
    let images: Vec<_> = wolf_armor_texture_images()
        .into_iter()
        .filter(|image| image.texture != WOLF_ARMOR_CRACKINESS_MEDIUM_TEXTURE_REF)
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let dye = 0x0033_66CC;
    let wolf = EntityModelInstance::wolf_state(
        308,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        false,
        Some(EntityDyeColor::Blue),
    )
    .with(|s| {
        s.kind.wolf_mut().body_equipment_asset = Some(test_wolf_equipment_asset_id());
        s.kind.wolf_mut().body_armor_dye = Some(dye);
        s.kind.wolf_mut().body_armor_crackiness = WolfArmorCrackiness::Medium;
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[wolf], &atlas);

    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.armor_translucent.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}

#[test]
fn invisible_wolf_body_armor_keeps_vanilla_layer_submissions() {
    // Vanilla `WolfArmorLayer` does not gate on `state.isInvisible`, unlike `WolfCollarLayer`.
    // Therefore an invisible adult wolf with body armor keeps the armor equipment/crack submissions:
    // hidden-to-player wolves have no base/collar submit, self-visible wolves add only the translucent
    // base, and glowing hidden wolves add only the base outline.
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_armor_texture_images()).unwrap();
    let dye = 0x00AA_5500;
    let dyed_tint = [0xAA as f32 / 255.0, 0x55 as f32 / 255.0, 0.0, 1.0];
    let invisible = EntityModelInstance::wolf_state(
        320,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        true,
        Some(EntityDyeColor::Blue),
    )
    .with(|s| {
        s.kind.wolf_mut().body_equipment_asset = Some(test_wolf_equipment_asset_id());
        s.kind.wolf_mut().body_armor_dye = Some(dye);
        s.kind.wolf_mut().body_armor_crackiness = WolfArmorCrackiness::High;
        s.kind.wolf_mut().body_armor_foil = true;
        s.light_coords = (4_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.75;
        s.has_red_overlay = true;
    });

    let hidden = entity_model_textured_meshes(&[invisible], &atlas);
    assert_wolf_armor_vertices(&hidden, invisible, dyed_tint);

    assert!(hidden.armor_cutout.vertices.iter().all(|vertex| {
        vertex.overlay == [0.0, 10.0] && vertex.light == invisible.render_state.shader_light()
    }));
    assert!(!hidden.armor_cutout.vertices.is_empty());
    assert!(!hidden.armor_translucent.vertices.is_empty());
    assert!(hidden.cutout.vertices.is_empty());
    assert!(hidden.translucent.vertices.is_empty());

    let self_visible = invisible.with(|s| s.invisible_to_player = false);
    let self_visible_meshes = entity_model_textured_meshes(&[self_visible], &atlas);
    assert_wolf_armor_vertices(&self_visible_meshes, self_visible, dyed_tint);

    assert!(!self_visible_meshes
        .item_entity_translucent_cull
        .vertices
        .is_empty());

    let glowing_hidden = invisible.with_outline_color(0xff33_66cc);
    let glowing = entity_model_textured_meshes(&[glowing_hidden], &atlas);
    assert_wolf_armor_vertices(&glowing, glowing_hidden, dyed_tint);

    assert_eq!(glowing.outline.cutout_faces, 264);
    assert_eq!(glowing.outline.vertices.len(), 1056);
    assert_eq!(glowing.outline.indices.len(), 1584);
    let outline_tint = [
        0x33 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xcc as f32 / 255.0,
        1.0,
    ];
    assert!(glowing.outline.vertices[0..264].iter().all(|vertex| {
        vertex.tint == outline_tint
            && vertex.light == glowing_hidden.render_state.shader_light()
            && vertex.overlay == glowing_hidden.render_state.overlay_coords()
    }));
    assert!(glowing.outline.vertices[264..].iter().all(|vertex| {
        vertex.tint == outline_tint
            && vertex.light == glowing_hidden.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]
    }));
}

#[test]
fn wolf_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    for base in [
        EntityModelInstance::wolf(480, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::wolf(481, [0.0, 64.0, 0.0], 0.0, true),
    ] {
        let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
        let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_wolf_vertices_match_vanilla(&resting, base);
        assert_wolf_vertices_match_vanilla(&yawed, yawed_instance);
        assert_wolf_vertices_match_vanilla(&pitched, pitched_instance);
        let resting = &resting.cutout;
        let yawed = &yawed.cutout;
        let pitched = &pitched.cutout;
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "{:?}", base.kind);
        assert_ne!(yawed.vertices, pitched.vertices, "{:?}", base.kind);
    }
}

#[test]
fn wolf_textured_mesh_swings_its_legs_when_walking() {
    // The real wolf render path (texture-backed) swings the same legs. A standing wolf is
    // byte-identical however far the swing has advanced; a walking one differs, splays
    // along Z (measured over the leg region so the resting tail droop does not mask it),
    // and (for the adult) lifts its feet, while keeping the vertex count.
    let z_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[2]);
            hi = hi.max(vertex.position[2]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    for (base, adult_size, legs) in [
        (
            EntityModelInstance::wolf(482, [0.0, 64.0, 0.0], 0.0, false),
            true,
            wolf_part(false, "right_front_leg"),
        ),
        (
            EntityModelInstance::wolf(483, [0.0, 64.0, 0.0], 0.0, true),
            false,
            wolf_part(true, "right_front_leg"),
        ),
    ] {
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_wolf_vertices_match_vanilla(&resting, base);
        assert_wolf_vertices_match_vanilla(&still, still_instance);
        assert_wolf_vertices_match_vanilla(&walking, walking_instance);
        let resting = &resting.cutout;
        let still = &still.cutout;
        let walking = &walking.cutout;

        assert_eq!(resting.vertices, still.vertices, "{:?} is inert", base.kind);
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{:?} leg swing keeps the vertex count",
            base.kind
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );

        assert!(
            z_extent(&walking.vertices[legs.clone()])
                > z_extent(&resting.vertices[legs.clone()]) + 0.02,
            "{:?} legs should splay along Z",
            base.kind
        );
        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(resting);
            let (walk_min, walk_max) = textured_mesh_extents(walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "an adult wolf's feet should lift off the ground (textured)"
            );
        }
    }
}

fn wolf_texture_images() -> Vec<EntityModelTextureImage> {
    texture_refs_matching(|path| {
        path.starts_with("textures/entity/wolf/wolf.png")
            || path.starts_with("textures/entity/wolf/wolf_tame.png")
            || path.starts_with("textures/entity/wolf/wolf_angry.png")
            || path.starts_with("textures/entity/wolf/wolf_baby.png")
            || path.starts_with("textures/entity/wolf/wolf_tame_baby.png")
            || path.starts_with("textures/entity/wolf/wolf_angry_baby.png")
            || path.starts_with("textures/entity/wolf/wolf_collar")
    })
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

fn wolf_armor_texture_images() -> Vec<EntityModelTextureImage> {
    let mut images = wolf_texture_images();
    let start = images.len();
    for (offset, texture) in [
        WOLF_BODY_ARMADILLO_SCUTE_TEXTURE_REF,
        WOLF_BODY_ARMADILLO_SCUTE_OVERLAY_TEXTURE_REF,
        WOLF_ARMOR_CRACKINESS_LOW_TEXTURE_REF,
        WOLF_ARMOR_CRACKINESS_MEDIUM_TEXTURE_REF,
        WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        images.push(EntityModelTextureImage::new(
            *texture,
            vec![(start + offset) as u8; len],
        ));
    }
    images
}

#[test]
fn wolf_tail_swing_pose_wags_with_the_quadruped_amplitude() {
    // Vanilla WolfModel.setupAnim (non-angry branch): tail.yRot = cos(pos * 0.6662) *
    // 1.4 * speed (the same QuadrupedModel amplitude as the legs, no phase offset), then
    // tail.xRot = state.tailAngle. The base tail pose carries the layer's resting xRot
    // droop (0.62831855 = π/5, the untamed tailAngle); the wag sets yRot and xRot and
    // preserves the offset and zRot.
    let base = ADULT_WOLF_TAIL_POSE;
    let wild = std::f32::consts::PI / 5.0;
    assert!(
        (base.rotation[0] - wild).abs() < 1e-6,
        "adult tail rests with the layer xRot droop: {}",
        base.rotation[0]
    );
    let tail = wolf_tail_swing_pose(base, wild, 0.0, 1.0);
    assert!(
        (tail.rotation[1] - 1.4).abs() < 1e-6,
        "tail wags to cos(0) * 1.4 * 1 = 1.4: {}",
        tail.rotation[1]
    );
    assert_eq!(
        tail.rotation[0], wild,
        "untamed tailAngle sets the π/5 droop"
    );
    assert_eq!(tail.rotation[2], base.rotation[2], "zRot preserved");
    assert_eq!(tail.offset, base.offset, "offset preserved");

    // A general (pos, speed) reproduces cos(pos * 0.6662) * 1.4 * speed.
    let phase = 2.0_f32 * 0.6662;
    let tail = wolf_tail_swing_pose(base, wild, 2.0, 0.5);
    assert!((tail.rotation[1] - phase.cos() * 1.4 * 0.5).abs() < 1e-6);

    // At rest, an untamed wolf (tailAngle = π/5 = the layer xRot) is byte-identical to the
    // base pose, so the borrow fast paths still apply.
    let tail = wolf_tail_swing_pose(base, wild, 3.0, 0.0);
    assert_eq!(tail.rotation[1], 0.0);
    assert_eq!(
        tail, base,
        "untamed wolf at rest matches the layer pose exactly"
    );

    // A tame wolf's health droop SETS a different xRot even when standing still.
    let droop = (0.55 - 0.8 * 0.4) * std::f32::consts::PI; // damageRatio 0.8 (health 8/40)
    let tail = wolf_tail_swing_pose(base, droop, 0.0, 0.0);
    assert_eq!(tail.rotation[0], droop, "tame tailAngle droop sets xRot");
    assert_eq!(tail.rotation[1], 0.0, "no wag at rest");
    assert_ne!(
        tail, base,
        "a drooping tame tail differs from the layer rest pose"
    );
}

#[test]
fn wolf_water_shake_roll_matches_vanilla_body_roll_angle() {
    // Vanilla `WolfRenderState.getBodyRollAngle(offset)` clamps `(shakeAnim + offset) / 1.8` and
    // applies two sine waves. Adult `AdultWolfModel.shakeOffWater` rolls the real head, body,
    // upper-body mane, and real tail; baby `BabyWolfModel.shakeOffWater` rolls head/body/tail.
    // Both models add `headRollAngle` to only the head roll for the interested/begging tilt.
    let shake_anim = 0.9;
    let head_roll_angle = 0.17;
    assert_eq!(wolf_body_roll_angle(0.0, -0.16), 0.0);
    assert!(wolf_body_roll_angle(2.5, 0.0).abs() < 1.0e-6);

    let adult_instance = EntityModelInstance::wolf(170, [0.0, 64.0, 0.0], 0.0, false).with(|s| {
        s.kind.wolf_mut().shake_anim = shake_anim;
        s.kind.wolf_mut().head_roll_angle = head_roll_angle;
    });
    let mut adult = wolf_model(false, false, &adult_instance);
    let adult_root = &mut adult;
    assert!(
        (adult_root.child_mut("body").pose.rotation[2] - wolf_body_roll_angle(shake_anim, -0.16))
            .abs()
            < 1.0e-6
    );
    assert!(
        (adult_root
            .child_mut("head")
            .child_mut("real_head")
            .pose
            .rotation[2]
            - (head_roll_angle + wolf_body_roll_angle(shake_anim, 0.0)))
        .abs()
            < 1.0e-6
    );
    assert!(
        (adult_root.child_mut("upper_body").pose.rotation[2]
            - wolf_body_roll_angle(shake_anim, -0.08))
        .abs()
            < 1.0e-6
    );
    assert!(
        (adult_root
            .child_mut("tail")
            .child_mut("real_tail")
            .pose
            .rotation[2]
            - wolf_body_roll_angle(shake_anim, -0.2))
        .abs()
            < 1.0e-6
    );

    let baby_instance = EntityModelInstance::wolf(171, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.kind.wolf_mut().shake_anim = shake_anim;
        s.kind.wolf_mut().head_roll_angle = head_roll_angle;
    });
    let mut baby = wolf_model(true, false, &baby_instance);
    let baby_root = &mut baby;
    assert!(
        (baby_root.child_mut("body").pose.rotation[2] - wolf_body_roll_angle(shake_anim, -0.16))
            .abs()
            < 1.0e-6
    );
    assert!(
        (baby_root.child_mut("head").pose.rotation[2]
            - (head_roll_angle + wolf_body_roll_angle(shake_anim, 0.0)))
        .abs()
            < 1.0e-6
    );
    assert!(
        (baby_root.child_mut("tail").pose.rotation[2] - wolf_body_roll_angle(shake_anim, -0.2))
            .abs()
            < 1.0e-6
    );
}

#[test]
fn wolf_textured_mesh_applies_water_shake_roll_to_base_and_collar() {
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let base = EntityModelInstance::wolf_state(
        172,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        false,
        false,
        Some(EntityDyeColor::Blue),
    );
    let dry = entity_model_textured_meshes(&[base], &atlas);
    let shaking = entity_model_textured_meshes(
        &[base.with(|s| {
            s.kind.wolf_mut().shake_anim = 0.9;
            s.kind.wolf_mut().head_roll_angle = 0.17;
        })],
        &atlas,
    );

    assert_eq!(dry.cutout.vertices.len(), shaking.cutout.vertices.len());
    assert_ne!(
        dry.cutout.vertices[..264],
        shaking.cutout.vertices[..264],
        "base wolf pass rolls while shaking water off"
    );
    assert_ne!(
        dry.cutout.vertices[264..],
        shaking.cutout.vertices[264..],
        "collar pass reuses the same rolled wolf pose"
    );
}

#[test]
fn wolf_textured_mesh_wags_its_tail_when_walking() {
    // The texture-backed wolf base layer runs the same tail wag, emitting the parts in the
    // same order, so the adult tail occupies textured vertices [240, 264). A standing wolf
    // is byte-identical; a walking one wags its tail sideways.
    let x_extent = |verts: &[EntityModelTexturedVertex]| -> f32 {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for vertex in verts {
            lo = lo.min(vertex.position[0]);
            hi = hi.max(vertex.position[0]);
        }
        hi - lo
    };
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let base = EntityModelInstance::wolf(151, [0.0, 64.0, 0.0], 0.0, false);
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
    assert_wolf_vertices_match_vanilla(&resting, base);
    assert_wolf_vertices_match_vanilla(&walking, walking_instance);
    let resting = &resting.cutout;
    let walking = &walking.cutout;
    assert_ne!(
        resting.vertices[wolf_part(false, "tail")],
        walking.vertices[wolf_part(false, "tail")],
        "the tail wags when walking"
    );
    let rest_tail_x = x_extent(&resting.vertices[wolf_part(false, "tail")]);
    let walk_tail_x = x_extent(&walking.vertices[wolf_part(false, "tail")]);
    assert!(
        walk_tail_x > rest_tail_x + 0.1,
        "the textured tail wags sideways when walking: {rest_tail_x} -> {walk_tail_x}"
    );
}

#[test]
fn tame_wolf_droops_its_tail_with_damage_textured() {
    // The texture-backed base layer SETS the same `wolf_tail_angle` droop on the tail.
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let wild = EntityModelInstance::wolf(151, [0.0, 64.0, 0.0], 0.0, false);
    let full = 0.55 * std::f32::consts::PI;
    let hurt = (0.55 - 0.8 * 0.4) * std::f32::consts::PI;
    let healthy_instance = wild.with(|s| s.kind.wolf_mut().tail_angle = full);
    let damaged_instance = wild.with(|s| s.kind.wolf_mut().tail_angle = hurt);
    let wild_meshes = entity_model_textured_meshes(&[wild], &atlas);
    let healthy_meshes = entity_model_textured_meshes(&[healthy_instance], &atlas);
    let damaged_meshes = entity_model_textured_meshes(&[damaged_instance], &atlas);
    assert_wolf_vertices_match_vanilla(&wild_meshes, wild);
    assert_wolf_vertices_match_vanilla(&healthy_meshes, healthy_instance);
    assert_wolf_vertices_match_vanilla(&damaged_meshes, damaged_instance);
    let wild_mesh = &wild_meshes.cutout;
    let healthy_mesh = &healthy_meshes.cutout;
    let damaged_mesh = &damaged_meshes.cutout;
    let tail = wolf_part(false, "tail");
    let outside_tail = |vertices: &[EntityModelTexturedVertex]| {
        let mut rest = vertices[..tail.start].to_vec();
        rest.extend_from_slice(&vertices[tail.end..]);
        rest
    };
    assert_eq!(
        outside_tail(&wild_mesh.vertices),
        outside_tail(&damaged_mesh.vertices),
        "only the tail bends with the tail angle"
    );
    assert_ne!(
        wild_mesh.vertices[tail.clone()],
        healthy_mesh.vertices[tail.clone()],
        "a healthy tame wolf's tail differs from the π/5 wild rest droop"
    );
    assert_ne!(
        healthy_mesh.vertices[tail.clone()],
        damaged_mesh.vertices[tail.clone()],
        "the tail bends further as health drops"
    );
}

#[test]
fn wolf_sitting_pose_matches_vanilla_set_sitting_pose() {
    // Vanilla `WolfModel.setSittingPose` (ageScale 1.0 adult / 0.5 baby; rotations SET):
    //   body:     y += 4*as,  z -= 2*as,  xRot = π/4  (baby: a further −π/2, → −π/4)
    //   hindLeg:  y += 6.7*as, z -= 5*as, xRot = 3π/2
    //   frontLeg: xRot = 5.811947, x += ±0.01*as (right +, left −), y += 1*as
    //   tail:     y += 9*as,  z -= 2*as  (offset only; xRot/yRot come from the tail pose)
    // The roles are now resolved by child name (the adult and baby trees name the same parts).
    assert_eq!(WOLF_SIT_FRONT_LEG_X_ROT, 5.811947);
    assert_eq!(
        wolf_sitting_part_roles(),
        [
            ("body", WolfSitPart::Body),
            ("right_hind_leg", WolfSitPart::HindLeg),
            ("left_hind_leg", WolfSitPart::HindLeg),
            ("right_front_leg", WolfSitPart::RightFrontLeg),
            ("left_front_leg", WolfSitPart::LeftFrontLeg),
            ("tail", WolfSitPart::Tail),
        ]
    );

    let base = PartPose {
        offset: [1.0, 2.0, 3.0],
        rotation: [0.1, 0.2, 0.3],
    };

    let mut body = base;
    apply_wolf_sitting_pose(&mut body, WolfSitPart::Body, false, 1.0);
    assert_eq!(body.offset, [1.0, 6.0, 1.0]);
    assert_eq!(body.rotation, [std::f32::consts::FRAC_PI_4, 0.2, 0.3]);

    let mut baby_body = base;
    apply_wolf_sitting_pose(&mut baby_body, WolfSitPart::Body, true, 0.5);
    assert_eq!(baby_body.offset, [1.0, 4.0, 2.0]);
    assert!(
        (baby_body.rotation[0] - (std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_2)).abs()
            < 1e-6
    );

    let mut hind = base;
    apply_wolf_sitting_pose(&mut hind, WolfSitPart::HindLeg, false, 1.0);
    assert_eq!(hind.offset, [1.0, 8.7, -2.0]);
    assert!((hind.rotation[0] - std::f32::consts::PI * 1.5).abs() < 1e-6);

    let mut right = base;
    apply_wolf_sitting_pose(&mut right, WolfSitPart::RightFrontLeg, false, 1.0);
    assert!((right.offset[0] - 1.01).abs() < 1e-6);
    assert_eq!(right.offset[1], 3.0);
    assert_eq!(right.rotation[0], WOLF_SIT_FRONT_LEG_X_ROT);
    let mut left = base;
    apply_wolf_sitting_pose(&mut left, WolfSitPart::LeftFrontLeg, false, 1.0);
    assert!((left.offset[0] - 0.99).abs() < 1e-6);

    // The baby front-leg x nudge scales by ageScale 0.5.
    let mut baby_right = base;
    apply_wolf_sitting_pose(&mut baby_right, WolfSitPart::RightFrontLeg, true, 0.5);
    assert!((baby_right.offset[0] - 1.005).abs() < 1e-6);

    let mut tail = base;
    apply_wolf_sitting_pose(&mut tail, WolfSitPart::Tail, false, 1.0);
    assert_eq!(tail.offset, [1.0, 11.0, 1.0]);
    assert_eq!(
        tail.rotation, base.rotation,
        "the tail rotation is left to the tail pose"
    );
}

#[test]
fn wolf_textured_mesh_sits_folds_legs_and_tilts_body() {
    // The texture-backed render path folds into the same sitting pose.
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    for (baby, head_end) in [(false, 96), (true, 72)] {
        let standing = EntityModelInstance::wolf(161, [0.0, 64.0, 0.0], 0.0, baby);
        let sitting = standing.with(|s| s.kind.wolf_mut().sitting = true);
        let stand_meshes = entity_model_textured_meshes(&[standing], &atlas);
        let sit_meshes = entity_model_textured_meshes(&[sitting], &atlas);
        assert_wolf_vertices_match_vanilla(&stand_meshes, standing);
        assert_wolf_vertices_match_vanilla(&sit_meshes, sitting);
        let stand_mesh = &stand_meshes.cutout;
        let sit_mesh = &sit_meshes.cutout;
        assert_eq!(
            stand_mesh.vertices.len(),
            sit_mesh.vertices.len(),
            "baby={baby}"
        );
        assert_eq!(
            stand_mesh.vertices[..head_end],
            sit_mesh.vertices[..head_end],
            "baby={baby}: the head is unchanged by sitting"
        );
        assert_ne!(
            stand_mesh.vertices[head_end..],
            sit_mesh.vertices[head_end..],
            "baby={baby}: the body, legs, and tail fold when sitting"
        );
    }
}

#[test]
fn wolf_angry_tail_pose_matches_vanilla_get_tail_angle() {
    // Vanilla `WolfModel.setupAnim` for an angry wolf: `tail.yRot = 0` (no wag) and
    // `tail.xRot = getTailAngle() = 1.5393804` (the angry constant), overriding the layer's
    // π/5 wild rest droop. The offset and zRot are preserved.
    let base = ADULT_WOLF_TAIL_POSE;
    assert!(
        (base.rotation[0] - 0.62831855).abs() < 1e-6,
        "adult tail rests at the π/5 wild droop: {}",
        base.rotation[0]
    );
    assert_eq!(WOLF_ANGRY_TAIL_X_ROT, 1.5393804);

    let angry = wolf_angry_tail_pose(base);
    assert!(
        (angry.rotation[0] - 1.5393804).abs() < 1e-6,
        "angry tail raises to 1.5393804: {}",
        angry.rotation[0]
    );
    assert_eq!(angry.rotation[1], 0.0, "angry tail does not wag");
    assert_eq!(angry.rotation[2], base.rotation[2], "zRot preserved");
    assert_eq!(angry.offset, base.offset, "offset preserved");
}

#[test]
fn angry_wolf_textured_mesh_raises_and_holds_its_tail_still() {
    // The texture-backed angry wolf runs the same tail branch: the tail is raised and held
    // still (no wag) while the legs swing. Positions ignore the differing angry-texture UVs.
    let tail_positions = |verts: &[EntityModelTexturedVertex]| -> Vec<[f32; 3]> {
        verts[wolf_part(false, "tail")]
            .iter()
            .map(|v| v.position)
            .collect()
    };
    let (atlas, _) = build_entity_model_texture_atlas(&wolf_texture_images()).unwrap();
    let calm = EntityModelInstance::wolf(150, [0.0, 64.0, 0.0], 0.0, false);
    let angry = EntityModelInstance::wolf_state(
        151,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
        true,
        false,
        None,
    );

    let calm_rest_meshes = entity_model_textured_meshes(&[calm], &atlas);
    let angry_rest_meshes = entity_model_textured_meshes(&[angry], &atlas);
    assert_wolf_vertices_match_vanilla(&calm_rest_meshes, calm);
    assert_wolf_vertices_match_vanilla(&angry_rest_meshes, angry);
    let calm_rest = &calm_rest_meshes.cutout;
    let angry_rest = &angry_rest_meshes.cutout;
    assert_ne!(
        tail_positions(&calm_rest.vertices),
        tail_positions(&angry_rest.vertices),
        "the angry wolf raises its textured tail"
    );

    let angry_walking_instance =
        angry.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let angry_walking_meshes = entity_model_textured_meshes(&[angry_walking_instance], &atlas);
    assert_wolf_vertices_match_vanilla(&angry_walking_meshes, angry_walking_instance);
    let angry_walking = &angry_walking_meshes.cutout;
    assert_eq!(
        angry_rest.vertices[wolf_part(false, "tail")],
        angry_walking.vertices[wolf_part(false, "tail")],
        "the angry textured tail is held still when walking (no wag)"
    );
    assert_ne!(
        angry_rest.vertices[wolf_part(false, "right_front_leg")],
        angry_walking.vertices[wolf_part(false, "right_front_leg")],
        "the angry wolf still swings its legs in the textured path"
    );
}

fn assert_wolf_vertices_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_wolf_folded_meshes(meshes, !instance.render_state.invisible);
    let collar = matches!(
        instance.kind,
        EntityModelKind::Wolf {
            tame: true,
            collar_color: Some(_),
            ..
        }
    );
    assert!(meshes.cutout.vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light()
            && (vertex.overlay == instance.render_state.overlay_coords()
                || collar && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY)
    }));
}

fn assert_wolf_armor_vertices(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    dyed_tint: [f32; 4],
) {
    assert_eq!(meshes.armor_cutout.vertices.len(), 528);
    assert!(meshes.armor_cutout.vertices[..264]
        .iter()
        .all(|vertex| vertex.tint == [1.0; 4]));
    assert!(meshes.armor_cutout.vertices[264..]
        .iter()
        .all(|vertex| vertex.tint == dyed_tint));
    assert!(meshes.armor_cutout.vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light()
            && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY
    }));
    assert_eq!(meshes.armor_entity_glint.vertices.len(), 264);
    assert_eq!(meshes.armor_translucent.vertices.len(), 264);
    assert!(meshes.armor_entity_glint.vertices.iter().all(|vertex| {
        vertex.tint == [1.0; 4]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY
    }));
    assert!(meshes.armor_translucent.vertices.iter().all(|vertex| {
        vertex.tint == [1.0; 4]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY
    }));
}

fn wolf_submission_probe(instance: EntityModelInstance) -> EntityModelInstance {
    instance.with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
}

fn assert_wolf_folded_meshes(meshes: &EntityModelTexturedMeshes, visible: bool) {
    assert_eq!(meshes.cutout.vertices.is_empty(), !visible);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.outline.vertices.is_empty());
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

fn wolf_part(baby: bool, name: &str) -> std::ops::Range<usize> {
    let layer = if baby {
        ModelLayer::WOLF_BABY
    } else {
        ModelLayer::WOLF
    };
    layer.bake().part_vertex_range(name).unwrap()
}
