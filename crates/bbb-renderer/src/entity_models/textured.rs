use super::{
    boat_model_root_transform,
    catalog::{
        boat_texture_ref, chicken_texture_ref, cow_texture_ref, pig_texture_ref,
        player_texture_ref, sheep_wool_layer_color, wolf_texture_ref, BoatModelFamily,
        ChickenModelVariant, CowModelVariant, EntityDyeColor, EntityModelInstance, EntityModelKind,
        EntityModelTextureAtlasEntry, EntityModelTextureAtlasLayout, EntityModelTextureRef,
        PigModelVariant, SheepWoolColor,
    },
    entity_model_root_transform,
    geometry::{emit_textured_model_parts, EntityModelTexturedMesh, TexturedModelPartDesc},
    model_layers::*,
    player_model_root_transform,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityModelLayerKind {
    BoatBase,
    ChickenBase,
    CowBase,
    CreeperBase,
    PigBase,
    PlayerBase,
    SheepBase,
    SheepWool,
    SheepWoolUndercoat,
    WolfBase,
    WolfCollar,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EntityModelLayerPass {
    pub(super) kind: EntityModelLayerKind,
    pub(super) model_layer: &'static str,
    pub(super) texture: EntityModelTextureRef,
    pub(super) parts: &'static [TexturedModelPartDesc],
    pub(super) tint: [f32; 4],
    pub(super) collector_order: i32,
    pub(super) submit_sequence: u32,
}

pub(super) fn entity_model_textured_mesh(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMesh {
    let mut mesh = EntityModelTexturedMesh::new();
    for instance in instances {
        match instance.kind {
            EntityModelKind::Chicken { variant, baby } => {
                emit_chicken_textured_model(&mut mesh, *instance, variant, baby, atlas);
            }
            EntityModelKind::Pig { variant, baby } => {
                emit_pig_textured_model(&mut mesh, *instance, variant, baby, atlas);
            }
            EntityModelKind::Cow { variant, baby } => {
                emit_cow_textured_model(&mut mesh, *instance, variant, baby, atlas);
            }
            EntityModelKind::Creeper => {
                emit_creeper_textured_model(&mut mesh, *instance, atlas);
            }
            EntityModelKind::Player { slim } => {
                emit_player_textured_model(&mut mesh, *instance, slim, atlas);
            }
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
            } => {
                emit_sheep_textured_model(&mut mesh, *instance, baby, sheared, wool_color, atlas);
            }
            EntityModelKind::Wolf {
                baby,
                tame,
                angry,
                collar_color,
            } => {
                emit_wolf_textured_model(
                    &mut mesh,
                    *instance,
                    baby,
                    tame,
                    angry,
                    collar_color,
                    atlas,
                );
            }
            EntityModelKind::Boat { family, chest } => {
                emit_boat_textured_model(&mut mesh, *instance, family, chest, atlas);
            }
            _ => {}
        }
    }
    mesh
}

fn emit_boat_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    family: BoatModelFamily,
    chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = boat_model_root_transform(instance);
    for pass in boat_textured_layer_passes(family, chest) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_chicken_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    variant: ChickenModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in chicken_textured_layer_passes(variant, baby) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_pig_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    variant: PigModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in pig_textured_layer_passes(variant, baby) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_cow_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    variant: CowModelVariant,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in cow_textured_layer_passes(variant, baby) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_creeper_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in creeper_textured_layer_passes() {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_player_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    slim: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = player_model_root_transform(instance);
    for pass in player_textured_layer_passes(slim) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_sheep_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in sheep_textured_layer_passes(baby, sheared, wool_color) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

fn emit_wolf_textured_model(
    mesh: &mut EntityModelTexturedMesh,
    instance: EntityModelInstance,
    baby: bool,
    tame: bool,
    angry: bool,
    collar_color: Option<EntityDyeColor>,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let transform = entity_model_root_transform(instance);
    for pass in wolf_textured_layer_passes(baby, tame, angry, collar_color) {
        let Some(entry) = entity_model_texture_atlas_entry(atlas, pass.texture) else {
            continue;
        };
        emit_textured_model_parts(
            mesh,
            pass.parts,
            transform,
            pass.texture,
            entry.uv,
            pass.tint,
        );
    }
}

pub(super) fn boat_textured_layer_passes(
    family: BoatModelFamily,
    chest: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::BoatBase,
        model_layer: boat_model_layer(family, chest),
        texture: boat_texture_ref(family, chest),
        parts: boat_textured_model_parts(family, chest),
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
        model_layer: chicken_model_layer(variant, baby),
        texture: chicken_texture_ref(variant, baby),
        parts: chicken_textured_model_parts(variant, baby),
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
        model_layer: pig_model_layer(variant, baby),
        texture: pig_texture_ref(variant, baby),
        parts: pig_textured_model_parts(variant, baby),
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
        model_layer: cow_model_layer(variant, baby),
        texture: cow_texture_ref(variant, baby),
        parts: cow_textured_model_parts(variant, baby),
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn creeper_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::CreeperBase,
        model_layer: MODEL_LAYER_CREEPER,
        texture: CREEPER_TEXTURE_REF,
        parts: &CREEPER_TEXTURED_PARTS,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(super) fn player_textured_layer_passes(slim: bool) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::PlayerBase,
        model_layer: player_model_layer(slim),
        texture: player_texture_ref(slim),
        parts: player_textured_model_parts(slim),
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
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    });
    if !baby && wool_color != SheepWoolColor::White {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SheepWoolUndercoat,
            model_layer: MODEL_LAYER_SHEEP_WOOL_UNDERCOAT,
            texture: SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
            parts: &ADULT_SHEEP_TEXTURED_PARTS,
            tint: wool_tint,
            collector_order: 1,
            submit_sequence: 1,
        });
    }
    if !sheared {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::SheepWool,
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
        model_layer,
        texture: wolf_texture_ref(baby, tame, angry),
        parts,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    });
    if let Some(collar_color) = tame.then_some(collar_color).flatten() {
        passes.push(EntityModelLayerPass {
            kind: EntityModelLayerKind::WolfCollar,
            model_layer,
            texture: if baby {
                WOLF_BABY_COLLAR_TEXTURE_REF
            } else {
                WOLF_COLLAR_TEXTURE_REF
            },
            parts,
            tint: collar_color.texture_diffuse_color(),
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
