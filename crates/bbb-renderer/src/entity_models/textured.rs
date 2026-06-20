use super::{
    boat_model_root_transform,
    catalog::{
        BoatModelFamily, ChickenModelVariant, CowModelVariant, EntityDyeColor, EntityModelKind,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureRef,
        HoglinModelFamily, PigModelVariant, PlayerModelPartVisibility, SheepWoolColor,
        SkeletonModelFamily,
    },
    cave_spider_model_root_transform, entity_model_root_transform,
    geometry::{emit_textured_model_parts, EntityModelTexturedMesh, TexturedModelPartDesc},
    instances::EntityModelInstance,
    magma_cube_model_root_transform, player_model_root_transform, polar_bear_model_root_transform,
    slime_model_root_transform, wither_skeleton_model_root_transform,
};
use glam::Mat4;

mod layers;

pub(super) use layers::{
    boat_textured_layer_passes, chicken_textured_layer_passes, cow_textured_layer_passes,
    creeper_textured_layer_passes, enderman_textured_layer_passes, goat_textured_layer_passes,
    hoglin_textured_layer_passes, magma_cube_textured_layer_passes, pig_textured_layer_passes,
    player_textured_layer_passes, polar_bear_textured_layer_passes, sheep_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, spider_textured_layer_passes,
    wolf_textured_layer_passes, EntityModelLayerPass, EntityModelLayerRenderType,
};
use layers::{goat_visible_textured_model_parts, player_visible_textured_model_parts};
#[cfg(test)]
pub(super) use layers::{EntityModelLayerKind, EntityModelLayerVisibility};

pub(super) struct EntityModelTexturedMeshes {
    pub(super) cutout: EntityModelTexturedMesh,
    pub(super) translucent: EntityModelTexturedMesh,
    pub(super) eyes: EntityModelTexturedMesh,
}

impl EntityModelTexturedMeshes {
    fn new() -> Self {
        Self {
            cutout: EntityModelTexturedMesh::new(),
            translucent: EntityModelTexturedMesh::new(),
            eyes: EntityModelTexturedMesh::new(),
        }
    }

    fn mesh_mut(
        &mut self,
        render_type: EntityModelLayerRenderType,
    ) -> &mut EntityModelTexturedMesh {
        match render_type {
            EntityModelLayerRenderType::Cutout => &mut self.cutout,
            EntityModelLayerRenderType::Translucent => &mut self.translucent,
            EntityModelLayerRenderType::Eyes => &mut self.eyes,
        }
    }
}

#[cfg(test)]
pub(super) fn entity_model_textured_mesh(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMesh {
    entity_model_textured_meshes(instances, atlas).cutout
}

pub(super) fn entity_model_textured_meshes(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    let mut meshes = EntityModelTexturedMeshes::new();
    for instance in instances {
        match instance.kind {
            EntityModelKind::Chicken { variant, baby } => {
                emit_chicken_textured_model(&mut meshes, *instance, variant, baby, atlas);
            }
            EntityModelKind::Pig { variant, baby } => {
                emit_pig_textured_model(&mut meshes, *instance, variant, baby, atlas);
            }
            EntityModelKind::Cow { variant, baby } => {
                emit_cow_textured_model(&mut meshes, *instance, variant, baby, atlas);
            }
            EntityModelKind::Creeper => {
                emit_creeper_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Spider => {
                emit_spider_textured_model(&mut meshes, *instance, false, atlas);
            }
            EntityModelKind::CaveSpider => {
                emit_spider_textured_model(&mut meshes, *instance, true, atlas);
            }
            EntityModelKind::Enderman => {
                emit_enderman_textured_model(&mut meshes, *instance, atlas);
            }
            EntityModelKind::Slime { size } => {
                emit_slime_textured_model(&mut meshes, *instance, size, atlas);
            }
            EntityModelKind::MagmaCube { size } => {
                emit_magma_cube_textured_model(&mut meshes, *instance, size, atlas);
            }
            EntityModelKind::PolarBear { baby } => {
                emit_polar_bear_textured_model(&mut meshes, *instance, baby, atlas);
            }
            EntityModelKind::Hoglin { family, baby } => {
                emit_hoglin_textured_model(&mut meshes, *instance, family, baby, atlas);
            }
            EntityModelKind::Player { slim, parts } => {
                emit_player_textured_model(&mut meshes, *instance, slim, parts, atlas);
            }
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
            } => {
                emit_sheep_textured_model(&mut meshes, *instance, baby, sheared, wool_color, atlas);
            }
            EntityModelKind::Wolf {
                baby,
                tame,
                angry,
                collar_color,
            } => {
                emit_wolf_textured_model(
                    &mut meshes,
                    *instance,
                    baby,
                    tame,
                    angry,
                    collar_color,
                    atlas,
                );
            }
            EntityModelKind::Goat {
                baby,
                left_horn,
                right_horn,
            } => {
                emit_goat_textured_model(
                    &mut meshes,
                    *instance,
                    baby,
                    left_horn,
                    right_horn,
                    atlas,
                );
            }
            EntityModelKind::Skeleton => {
                emit_skeleton_textured_model(&mut meshes, *instance, None, atlas);
            }
            EntityModelKind::SkeletonVariant { family } => {
                emit_skeleton_textured_model(&mut meshes, *instance, Some(family), atlas);
            }
            EntityModelKind::Boat { family, chest } => {
                emit_boat_textured_model(&mut meshes, *instance, family, chest, atlas);
            }
            _ => {}
        }
    }
    meshes
}

fn emit_boat_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: BoatModelFamily,
    chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = boat_model_root_transform(instance);
    for pass in boat_textured_layer_passes(family, chest) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_chicken_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: ChickenModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in chicken_textured_layer_passes(variant, baby) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_pig_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: PigModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in pig_textured_layer_passes(variant, baby) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_cow_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    variant: CowModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in cow_textured_layer_passes(variant, baby) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_creeper_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in creeper_textured_layer_passes() {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_spider_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    cave: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = if cave {
        cave_spider_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    };
    for pass in spider_textured_layer_passes(cave) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_enderman_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in enderman_textured_layer_passes() {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_slime_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = slime_model_root_transform(instance, size);
    for pass in slime_textured_layer_passes() {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_magma_cube_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    size: i32,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = magma_cube_model_root_transform(instance, size);
    for pass in magma_cube_textured_layer_passes() {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_polar_bear_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = if baby {
        entity_model_root_transform(instance)
    } else {
        polar_bear_model_root_transform(instance)
    };
    for pass in polar_bear_textured_layer_passes(baby) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_hoglin_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: HoglinModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in hoglin_textured_layer_passes(family, baby) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_player_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    slim: bool,
    parts: PlayerModelPartVisibility,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = player_model_root_transform(instance);
    let visible_parts = player_visible_textured_model_parts(slim, parts);
    for pass in player_textured_layer_passes(slim, parts) {
        emit_textured_layer_pass_with_parts(
            meshes,
            &pass,
            visible_parts.as_slice(),
            transform,
            atlas,
        );
    }
}

fn emit_sheep_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in sheep_textured_layer_passes(baby, sheared, wool_color) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_wolf_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    tame: bool,
    angry: bool,
    collar_color: Option<EntityDyeColor>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in wolf_textured_layer_passes(baby, tame, angry, collar_color) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_goat_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    baby: bool,
    left_horn: bool,
    right_horn: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    let visible_parts = goat_visible_textured_model_parts(baby, left_horn, right_horn);
    for pass in goat_textured_layer_passes(baby) {
        emit_textured_layer_pass_with_parts(
            meshes,
            &pass,
            visible_parts.as_slice(),
            transform,
            atlas,
        );
    }
}

fn emit_skeleton_textured_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    family: Option<SkeletonModelFamily>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = if matches!(family, Some(SkeletonModelFamily::WitherSkeleton)) {
        wither_skeleton_model_root_transform(instance)
    } else {
        entity_model_root_transform(instance)
    };
    for pass in skeleton_textured_layer_passes(family) {
        emit_textured_layer_pass(meshes, &pass, transform, atlas);
    }
}

fn emit_textured_layer_pass(
    meshes: &mut EntityModelTexturedMeshes,
    pass: &EntityModelLayerPass,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
) {
    emit_textured_layer_pass_with_parts(meshes, pass, pass.parts, transform, atlas);
}

fn emit_textured_layer_pass_with_parts(
    meshes: &mut EntityModelTexturedMeshes,
    pass: &EntityModelLayerPass,
    parts: &[TexturedModelPartDesc],
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
        return;
    };
    emit_textured_model_parts(
        meshes.mesh_mut(pass.render_type),
        parts,
        transform,
        pass.texture,
        entry.uv,
        pass.tint,
    );
}

fn entity_model_texture_atlas_entry(
    atlas: &EntityModelTextureAtlasLayout,
    texture: EntityModelTextureRef,
) -> Option<EntityModelTextureAtlasEntry> {
    atlas
        .entries
        .iter()
        .copied()
        .find(|entry| entry.texture == texture)
}
