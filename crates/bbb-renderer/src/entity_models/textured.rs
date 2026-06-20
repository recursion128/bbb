use super::{
    boat_model_root_transform,
    catalog::{
        boat_texture_ref, chicken_texture_ref, cow_texture_ref, pig_texture_ref,
        player_texture_ref, sheep_wool_layer_color, wolf_texture_ref, BoatModelFamily,
        ChickenModelVariant, CowModelVariant, EntityDyeColor, EntityModelKind,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureRef,
        PigModelVariant, PlayerModelPartVisibility, SheepWoolColor, SkeletonModelFamily,
    },
    cave_spider_model_root_transform, entity_model_root_transform,
    geometry::{emit_textured_model_parts, EntityModelTexturedMesh, TexturedModelPartDesc},
    instances::EntityModelInstance,
    magma_cube_model_root_transform,
    model_layers::*,
    player_model_root_transform, slime_model_root_transform, wither_skeleton_model_root_transform,
};
use glam::Mat4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityModelLayerKind {
    BoatBase,
    ChickenBase,
    CowBase,
    CreeperBase,
    EndermanBase,
    EndermanEyes,
    PigBase,
    PlayerBase,
    SheepBase,
    SheepWool,
    SheepWoolUndercoat,
    SkeletonBase,
    SkeletonClothing,
    SlimeBase,
    SlimeOuter,
    MagmaCubeBase,
    SpiderBase,
    SpiderEyes,
    WolfBase,
    WolfCollar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityModelLayerRenderType {
    Cutout,
    Translucent,
    Eyes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityModelLayerVisibility {
    All,
    PlayerParts(PlayerModelPartVisibility),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EntityModelLayerPass {
    pub(super) kind: EntityModelLayerKind,
    pub(super) render_type: EntityModelLayerRenderType,
    pub(super) model_layer: &'static str,
    pub(super) texture: EntityModelTextureRef,
    pub(super) parts: &'static [TexturedModelPartDesc],
    pub(super) visibility: EntityModelLayerVisibility,
    pub(super) tint: [f32; 4],
    pub(super) collector_order: i32,
    pub(super) submit_sequence: u32,
}

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

pub(super) fn boat_textured_layer_passes(
    family: BoatModelFamily,
    chest: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::BoatBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: boat_model_layer(family, chest),
        texture: boat_texture_ref(family, chest),
        parts: boat_textured_model_parts(family, chest),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn chicken_textured_layer_passes(
    variant: ChickenModelVariant,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::ChickenBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: chicken_model_layer(variant, baby),
        texture: chicken_texture_ref(variant, baby),
        parts: chicken_textured_model_parts(variant, baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn pig_textured_layer_passes(
    variant: PigModelVariant,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::PigBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: pig_model_layer(variant, baby),
        texture: pig_texture_ref(variant, baby),
        parts: pig_textured_model_parts(variant, baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn cow_textured_layer_passes(
    variant: CowModelVariant,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::CowBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: cow_model_layer(variant, baby),
        texture: cow_texture_ref(variant, baby),
        parts: cow_textured_model_parts(variant, baby),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn creeper_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::CreeperBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_CREEPER,
        texture: CREEPER_TEXTURE_REF,
        parts: &CREEPER_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn spider_textured_layer_passes(cave: bool) -> Vec<EntityModelLayerPass> {
    let model_layer = if cave {
        MODEL_LAYER_CAVE_SPIDER
    } else {
        MODEL_LAYER_SPIDER
    };
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::SpiderBase,
            render_type: EntityModelLayerRenderType::Cutout,
            model_layer,
            texture: if cave {
                CAVE_SPIDER_TEXTURE_REF
            } else {
                SPIDER_TEXTURE_REF
            },
            parts: &SPIDER_TEXTURED_PARTS,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            collector_order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::SpiderEyes,
            render_type: EntityModelLayerRenderType::Eyes,
            model_layer,
            texture: SPIDER_EYES_TEXTURE_REF,
            parts: &SPIDER_TEXTURED_PARTS,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            collector_order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(super) fn enderman_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::EndermanBase,
            render_type: EntityModelLayerRenderType::Cutout,
            model_layer: MODEL_LAYER_ENDERMAN,
            texture: ENDERMAN_TEXTURE_REF,
            parts: &ENDERMAN_TEXTURED_PARTS,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            collector_order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::EndermanEyes,
            render_type: EntityModelLayerRenderType::Eyes,
            model_layer: MODEL_LAYER_ENDERMAN,
            texture: ENDERMAN_EYES_TEXTURE_REF,
            parts: &ENDERMAN_TEXTURED_PARTS,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            collector_order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(super) fn slime_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![
        EntityModelLayerPass {
            kind: EntityModelLayerKind::SlimeBase,
            render_type: EntityModelLayerRenderType::Cutout,
            model_layer: MODEL_LAYER_SLIME,
            texture: SLIME_TEXTURE_REF,
            parts: &SLIME_INNER_TEXTURED_PARTS,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            collector_order: 0,
            submit_sequence: 0,
        },
        EntityModelLayerPass {
            kind: EntityModelLayerKind::SlimeOuter,
            render_type: EntityModelLayerRenderType::Translucent,
            model_layer: MODEL_LAYER_SLIME_OUTER,
            texture: SLIME_TEXTURE_REF,
            parts: &SLIME_OUTER_TEXTURED_PARTS,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            collector_order: 1,
            submit_sequence: 1,
        },
    ]
}

pub(super) fn magma_cube_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::MagmaCubeBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_MAGMA_CUBE,
        texture: MAGMA_CUBE_TEXTURE_REF,
        parts: &MAGMA_CUBE_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn player_textured_layer_passes(
    slim: bool,
    parts: PlayerModelPartVisibility,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::PlayerBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: player_model_layer(slim),
        texture: player_texture_ref(slim),
        parts: player_textured_model_parts(slim),
        visibility: EntityModelLayerVisibility::PlayerParts(parts),
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn sheep_textured_layer_passes(
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
) -> Vec<EntityModelLayerPass> {
    let wool_tint = sheep_wool_layer_color(wool_color);
    let mut passes = Vec::with_capacity(3);
    passes.push(EntityModelLayerPass {
        kind: EntityModelLayerKind::SheepBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: if baby {
            MODEL_LAYER_SHEEP_BABY
        } else {
            MODEL_LAYER_SHEEP
        },
        texture: if baby {
            SHEEP_BABY_TEXTURE_REF
        } else {
            SHEEP_TEXTURE_REF
        },
        parts: if baby {
            &BABY_SHEEP_TEXTURED_PARTS
        } else {
            &ADULT_SHEEP_TEXTURED_PARTS
        },
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    });
    if !baby && wool_color != SheepWoolColor::White {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SheepWoolUndercoat,
            render_type: EntityModelLayerRenderType::Cutout,
            model_layer: MODEL_LAYER_SHEEP_WOOL_UNDERCOAT,
            texture: SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
            parts: &ADULT_SHEEP_TEXTURED_PARTS,
            visibility: EntityModelLayerVisibility::All,
            tint: wool_tint,
            collector_order: 1,
            submit_sequence: 1,
        });
    }
    if !sheared {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SheepWool,
            render_type: EntityModelLayerRenderType::Cutout,
            model_layer: if baby {
                MODEL_LAYER_SHEEP_BABY_WOOL
            } else {
                MODEL_LAYER_SHEEP_WOOL
            },
            texture: if baby {
                SHEEP_WOOL_BABY_TEXTURE_REF
            } else {
                SHEEP_WOOL_TEXTURE_REF
            },
            parts: if baby {
                &BABY_SHEEP_TEXTURED_PARTS
            } else {
                &ADULT_SHEEP_WOOL_TEXTURED_PARTS
            },
            visibility: EntityModelLayerVisibility::All,
            tint: wool_tint,
            collector_order: if baby { 1 } else { 0 },
            submit_sequence: 2,
        });
    }
    passes.sort_by_key(|pass| (pass.collector_order, pass.submit_sequence));
    passes
}

pub(super) fn wolf_textured_layer_passes(
    baby: bool,
    tame: bool,
    angry: bool,
    collar_color: Option<EntityDyeColor>,
) -> Vec<EntityModelLayerPass> {
    let parts = if baby {
        BABY_WOLF_TEXTURED_PARTS.as_slice()
    } else {
        ADULT_WOLF_TEXTURED_PARTS.as_slice()
    };
    let model_layer = if baby {
        MODEL_LAYER_WOLF_BABY
    } else {
        MODEL_LAYER_WOLF
    };
    let mut passes = Vec::with_capacity(2);
    passes.push(EntityModelLayerPass {
        kind: EntityModelLayerKind::WolfBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer,
        texture: wolf_texture_ref(baby, tame, angry),
        parts,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    });
    if let Some(collar_color) = tame.then_some(collar_color).flatten() {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::WolfCollar,
            render_type: EntityModelLayerRenderType::Cutout,
            model_layer,
            texture: if baby {
                WOLF_BABY_COLLAR_TEXTURE_REF
            } else {
                WOLF_COLLAR_TEXTURE_REF
            },
            parts,
            visibility: EntityModelLayerVisibility::All,
            tint: collar_color.texture_diffuse_color(),
            collector_order: 1,
            submit_sequence: 1,
        });
    }
    passes
}

pub(super) fn skeleton_textured_layer_passes(
    family: Option<SkeletonModelFamily>,
) -> Vec<EntityModelLayerPass> {
    let mut passes = vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::SkeletonBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: skeleton_model_layer(family),
        texture: skeleton_texture_ref(family),
        parts: skeleton_textured_model_parts(family),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }];
    if let Some((model_layer, texture, parts)) = skeleton_clothing_layer_pass_parts(family) {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SkeletonClothing,
            render_type: EntityModelLayerRenderType::Cutout,
            model_layer,
            texture,
            parts,
            visibility: EntityModelLayerVisibility::All,
            tint: [1.0, 1.0, 1.0, 1.0],
            collector_order: 1,
            submit_sequence: 1,
        });
    }
    passes
}

fn boat_model_layer(family: BoatModelFamily, chest: bool) -> &'static str {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => MODEL_LAYER_ACACIA_BOAT,
        (BoatModelFamily::Acacia, true) => MODEL_LAYER_ACACIA_CHEST_BOAT,
        (BoatModelFamily::Bamboo, false) => MODEL_LAYER_BAMBOO_RAFT,
        (BoatModelFamily::Bamboo, true) => MODEL_LAYER_BAMBOO_CHEST_RAFT,
        (BoatModelFamily::Birch, false) => MODEL_LAYER_BIRCH_BOAT,
        (BoatModelFamily::Birch, true) => MODEL_LAYER_BIRCH_CHEST_BOAT,
        (BoatModelFamily::Cherry, false) => MODEL_LAYER_CHERRY_BOAT,
        (BoatModelFamily::Cherry, true) => MODEL_LAYER_CHERRY_CHEST_BOAT,
        (BoatModelFamily::DarkOak, false) => MODEL_LAYER_DARK_OAK_BOAT,
        (BoatModelFamily::DarkOak, true) => MODEL_LAYER_DARK_OAK_CHEST_BOAT,
        (BoatModelFamily::Jungle, false) => MODEL_LAYER_JUNGLE_BOAT,
        (BoatModelFamily::Jungle, true) => MODEL_LAYER_JUNGLE_CHEST_BOAT,
        (BoatModelFamily::Mangrove, false) => MODEL_LAYER_MANGROVE_BOAT,
        (BoatModelFamily::Mangrove, true) => MODEL_LAYER_MANGROVE_CHEST_BOAT,
        (BoatModelFamily::Oak, false) => MODEL_LAYER_OAK_BOAT,
        (BoatModelFamily::Oak, true) => MODEL_LAYER_OAK_CHEST_BOAT,
        (BoatModelFamily::PaleOak, false) => MODEL_LAYER_PALE_OAK_BOAT,
        (BoatModelFamily::PaleOak, true) => MODEL_LAYER_PALE_OAK_CHEST_BOAT,
        (BoatModelFamily::Spruce, false) => MODEL_LAYER_SPRUCE_BOAT,
        (BoatModelFamily::Spruce, true) => MODEL_LAYER_SPRUCE_CHEST_BOAT,
    }
}

fn boat_textured_model_parts(
    family: BoatModelFamily,
    chest: bool,
) -> &'static [TexturedModelPartDesc] {
    match (family, chest) {
        (BoatModelFamily::Bamboo, false) => &RAFT_TEXTURED_PARTS,
        (BoatModelFamily::Bamboo, true) => &RAFT_CHEST_TEXTURED_PARTS,
        (_, false) => &BOAT_TEXTURED_PARTS,
        (_, true) => &BOAT_CHEST_TEXTURED_PARTS,
    }
}

fn player_model_layer(slim: bool) -> &'static str {
    if slim {
        MODEL_LAYER_PLAYER_SLIM
    } else {
        MODEL_LAYER_PLAYER
    }
}

fn player_textured_model_parts(slim: bool) -> &'static [TexturedModelPartDesc] {
    if slim {
        &PLAYER_SLIM_TEXTURED_PARTS
    } else {
        &PLAYER_WIDE_TEXTURED_PARTS
    }
}

fn player_visible_textured_model_parts(
    slim: bool,
    parts: PlayerModelPartVisibility,
) -> [TexturedModelPartDesc; 6] {
    let source = player_textured_model_parts(slim);
    [
        TexturedModelPartDesc {
            children: if parts.hat { source[0].children } else { &[] },
            ..source[0]
        },
        TexturedModelPartDesc {
            children: if parts.jacket {
                source[1].children
            } else {
                &[]
            },
            ..source[1]
        },
        TexturedModelPartDesc {
            children: if parts.right_sleeve {
                source[2].children
            } else {
                &[]
            },
            ..source[2]
        },
        TexturedModelPartDesc {
            children: if parts.left_sleeve {
                source[3].children
            } else {
                &[]
            },
            ..source[3]
        },
        TexturedModelPartDesc {
            children: if parts.right_pants {
                source[4].children
            } else {
                &[]
            },
            ..source[4]
        },
        TexturedModelPartDesc {
            children: if parts.left_pants {
                source[5].children
            } else {
                &[]
            },
            ..source[5]
        },
    ]
}

fn chicken_model_layer(variant: ChickenModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (_, true) => MODEL_LAYER_CHICKEN_BABY,
        (ChickenModelVariant::Cold, false) => MODEL_LAYER_COLD_CHICKEN,
        (_, false) => MODEL_LAYER_CHICKEN,
    }
}

fn chicken_textured_model_parts(
    variant: ChickenModelVariant,
    baby: bool,
) -> &'static [TexturedModelPartDesc] {
    match (variant, baby) {
        (_, true) => &BABY_CHICKEN_TEXTURED_PARTS,
        (ChickenModelVariant::Cold, false) => &COLD_CHICKEN_TEXTURED_PARTS,
        (_, false) => &ADULT_CHICKEN_TEXTURED_PARTS,
    }
}

fn pig_model_layer(variant: PigModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (_, true) => MODEL_LAYER_PIG_BABY,
        (PigModelVariant::Cold, false) => MODEL_LAYER_COLD_PIG,
        (_, false) => MODEL_LAYER_PIG,
    }
}

fn pig_textured_model_parts(
    variant: PigModelVariant,
    baby: bool,
) -> &'static [TexturedModelPartDesc] {
    match (variant, baby) {
        (_, true) => &BABY_PIG_TEXTURED_PARTS,
        (PigModelVariant::Cold, false) => &COLD_PIG_TEXTURED_PARTS,
        (_, false) => &ADULT_PIG_TEXTURED_PARTS,
    }
}

fn cow_model_layer(variant: CowModelVariant, baby: bool) -> &'static str {
    match (variant, baby) {
        (CowModelVariant::Temperate, false) => MODEL_LAYER_COW,
        (CowModelVariant::Temperate, true) => MODEL_LAYER_COW_BABY,
        (CowModelVariant::Warm, false) => MODEL_LAYER_WARM_COW,
        (CowModelVariant::Warm, true) => MODEL_LAYER_WARM_COW_BABY,
        (CowModelVariant::Cold, false) => MODEL_LAYER_COLD_COW,
        (CowModelVariant::Cold, true) => MODEL_LAYER_COLD_COW_BABY,
    }
}

fn cow_textured_model_parts(
    variant: CowModelVariant,
    baby: bool,
) -> &'static [TexturedModelPartDesc] {
    match (variant, baby) {
        (_, true) => &BABY_COW_TEXTURED_PARTS,
        (CowModelVariant::Warm, false) => &WARM_COW_TEXTURED_PARTS,
        (CowModelVariant::Cold, false) => &COLD_COW_TEXTURED_PARTS,
        (CowModelVariant::Temperate, false) => &ADULT_COW_TEXTURED_PARTS,
    }
}

fn skeleton_model_layer(family: Option<SkeletonModelFamily>) -> &'static str {
    match family {
        None => MODEL_LAYER_SKELETON,
        Some(SkeletonModelFamily::Stray) => MODEL_LAYER_STRAY,
        Some(SkeletonModelFamily::Parched) => MODEL_LAYER_PARCHED,
        Some(SkeletonModelFamily::WitherSkeleton) => MODEL_LAYER_WITHER_SKELETON,
        Some(SkeletonModelFamily::Bogged { .. }) => MODEL_LAYER_BOGGED,
    }
}

fn skeleton_texture_ref(family: Option<SkeletonModelFamily>) -> EntityModelTextureRef {
    match family {
        None => SKELETON_TEXTURE_REF,
        Some(SkeletonModelFamily::Stray) => STRAY_TEXTURE_REF,
        Some(SkeletonModelFamily::Parched) => PARCHED_TEXTURE_REF,
        Some(SkeletonModelFamily::WitherSkeleton) => WITHER_SKELETON_TEXTURE_REF,
        Some(SkeletonModelFamily::Bogged { .. }) => BOGGED_TEXTURE_REF,
    }
}

fn skeleton_textured_model_parts(
    family: Option<SkeletonModelFamily>,
) -> &'static [TexturedModelPartDesc] {
    match family {
        None | Some(SkeletonModelFamily::Stray) | Some(SkeletonModelFamily::WitherSkeleton) => {
            &SKELETON_TEXTURED_PARTS
        }
        Some(SkeletonModelFamily::Parched) => &PARCHED_TEXTURED_PARTS,
        Some(SkeletonModelFamily::Bogged { sheared: false }) => &BOGGED_TEXTURED_PARTS,
        Some(SkeletonModelFamily::Bogged { sheared: true }) => &BOGGED_SHEARED_TEXTURED_PARTS,
    }
}

fn skeleton_clothing_layer_pass_parts(
    family: Option<SkeletonModelFamily>,
) -> Option<(
    &'static str,
    EntityModelTextureRef,
    &'static [TexturedModelPartDesc],
)> {
    match family {
        Some(SkeletonModelFamily::Stray) => Some((
            MODEL_LAYER_STRAY_OUTER_LAYER,
            STRAY_OVERLAY_TEXTURE_REF,
            &STRAY_OUTER_TEXTURED_PARTS,
        )),
        Some(SkeletonModelFamily::Bogged { .. }) => Some((
            MODEL_LAYER_BOGGED_OUTER_LAYER,
            BOGGED_OVERLAY_TEXTURE_REF,
            &BOGGED_OUTER_TEXTURED_PARTS,
        )),
        None | Some(SkeletonModelFamily::Parched) | Some(SkeletonModelFamily::WitherSkeleton) => {
            None
        }
    }
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
