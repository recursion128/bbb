use super::*;

mod poses;
mod textured;

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

use crate::entity_models::catalog::horse_markings_texture_ref;
use crate::DynamicEntityTextureImage;

#[test]
fn equine_textured_layer_passes_match_vanilla_renderer_order() {
    // Vanilla `HorseRenderer`: base model submit first, `HorseMarkingLayer.order(1)` second only
    // when the markings texture is not the sentinel invisible texture.
    let marked_horse =
        horse_textured_layer_passes(HorseColorVariant::White, false, HorseMarkings::WhiteDots);
    assert_eq!(marked_horse.len(), 2);
    assert_eq!(
        marked_horse[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(marked_horse[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(marked_horse[0].texture, HORSE_WHITE_TEXTURE_REF);
    assert_eq!(marked_horse[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(marked_horse[0].order, 0);
    assert_eq!(
        marked_horse[1].render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(
        marked_horse[1].render_type.vanilla_name(),
        "entityTranslucent"
    );
    assert_eq!(
        marked_horse[1].texture,
        HORSE_MARKINGS_WHITEDOTS_TEXTURE_REF
    );
    assert_eq!(marked_horse[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(marked_horse[1].order, 1);

    let plain_baby =
        horse_textured_layer_passes(HorseColorVariant::Creamy, true, HorseMarkings::None);
    assert_eq!(plain_baby.len(), 1);
    assert_eq!(plain_baby[0].texture, HORSE_CREAMY_BABY_TEXTURE_REF);

    // Vanilla `DonkeyRenderer` / `UndeadHorseRenderer`: the base body is one order-0 cutout submit;
    // saddle and body-equipment layers stay in their dedicated post-base helpers.
    let donkey = donkey_textured_layer_passes(DonkeyModelFamily::Donkey, false, true);
    assert_eq!(donkey.len(), 1);
    assert_eq!(donkey[0].texture, DONKEY_TEXTURE_REF);
    assert_eq!(donkey[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(donkey[0].order, 0);

    let mule_baby = donkey_textured_layer_passes(DonkeyModelFamily::Mule, true, false);
    assert_eq!(mule_baby[0].texture, MULE_BABY_TEXTURE_REF);

    let skeleton_horse =
        undead_horse_textured_layer_passes(UndeadHorseModelFamily::Skeleton, false);
    assert_eq!(skeleton_horse.len(), 1);
    assert_eq!(skeleton_horse[0].texture, SKELETON_HORSE_TEXTURE_REF);
    assert_eq!(skeleton_horse[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(skeleton_horse[0].order, 0);

    let zombie_baby = undead_horse_textured_layer_passes(UndeadHorseModelFamily::Zombie, true);
    assert_eq!(zombie_baby[0].texture, ZOMBIE_HORSE_BABY_TEXTURE_REF);
}

fn undead_horse_texture_images() -> Vec<EntityModelTextureImage> {
    [
        SKELETON_HORSE_TEXTURE_REF,
        SKELETON_HORSE_BABY_TEXTURE_REF,
        ZOMBIE_HORSE_TEXTURE_REF,
        ZOMBIE_HORSE_BABY_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

fn horse_texture_images() -> Vec<EntityModelTextureImage> {
    texture_refs_matching(|path| {
        (path.starts_with("textures/entity/horse/horse_"))
            && !path.contains("horse_zombie")
            && !path.contains("horse_skeleton")
    })
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn donkey_texture_images() -> Vec<EntityModelTextureImage> {
    [
        DONKEY_TEXTURE_REF,
        DONKEY_BABY_TEXTURE_REF,
        MULE_TEXTURE_REF,
        MULE_BABY_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

fn assert_equine_buckets(meshes: &EntityModelTexturedMeshes, instance: EntityModelInstance) {
    assert!(!meshes.cutout.vertices.is_empty());
    let has_markings = match instance.kind {
        EntityModelKind::Horse { markings, baby, .. } => {
            horse_markings_texture_ref(markings, baby).is_some()
        }
        _ => false,
    };
    assert_eq!(!meshes.translucent.vertices.is_empty(), has_markings);
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

fn assert_textured_vertex_state(
    vertices: &[EntityModelTexturedVertex],
    light: [f32; 2],
    overlay: [f32; 2],
) {
    assert!(vertices
        .iter()
        .all(|vertex| vertex.light == light && vertex.overlay == overlay));
}
