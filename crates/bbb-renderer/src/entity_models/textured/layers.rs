use super::super::{
    catalog::{
        boat_texture_ref, chicken_texture_ref, cow_texture_ref, pig_texture_ref,
        player_texture_ref, sheep_wool_render_color, wolf_texture_ref, BoatModelFamily,
        ChickenModelVariant, CowModelVariant, EntityDyeColor, EntityModelTextureRef,
        HoglinModelFamily, PigModelVariant, PlayerModelPartVisibility, SheepWoolColor,
        SkeletonModelFamily,
    },
    geometry::TexturedModelPartDesc,
    model_layers::*,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerKind {
    BoatBase,
    ChickenBase,
    CowBase,
    CreeperBase,
    EndermanBase,
    EndermanEyes,
    GoatBase,
    HoglinBase,
    IronGolemBase,
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
    GhastBase,
    BlazeBase,
    EndermiteBase,
    SilverfishBase,
    PolarBearBase,
    RavagerBase,
    SnowGolemBase,
    SpiderBase,
    SpiderEyes,
    VillagerBase,
    WanderingTraderBase,
    WitchBase,
    WolfBase,
    WolfCollar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerRenderType {
    Cutout,
    Translucent,
    Eyes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::entity_models) enum EntityModelLayerVisibility {
    All,
    PlayerParts(PlayerModelPartVisibility),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::entity_models) struct EntityModelLayerPass {
    pub(in crate::entity_models) kind: EntityModelLayerKind,
    pub(in crate::entity_models) render_type: EntityModelLayerRenderType,
    pub(in crate::entity_models) model_layer: &'static str,
    pub(in crate::entity_models) texture: EntityModelTextureRef,
    pub(in crate::entity_models) parts: &'static [TexturedModelPartDesc],
    pub(in crate::entity_models) visibility: EntityModelLayerVisibility,
    pub(in crate::entity_models) tint: [f32; 4],
    pub(in crate::entity_models) collector_order: i32,
    pub(in crate::entity_models) submit_sequence: u32,
}

pub(in crate::entity_models) fn boat_textured_layer_passes(
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

pub(in crate::entity_models) fn chicken_textured_layer_passes(
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

pub(in crate::entity_models) fn pig_textured_layer_passes(
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

pub(in crate::entity_models) fn cow_textured_layer_passes(
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

pub(in crate::entity_models) fn creeper_textured_layer_passes() -> Vec<EntityModelLayerPass> {
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

pub(in crate::entity_models) fn spider_textured_layer_passes(
    cave: bool,
) -> Vec<EntityModelLayerPass> {
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

pub(in crate::entity_models) fn enderman_textured_layer_passes() -> Vec<EntityModelLayerPass> {
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

pub(in crate::entity_models) fn iron_golem_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::IronGolemBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_IRON_GOLEM,
        texture: IRON_GOLEM_TEXTURE_REF,
        parts: &IRON_GOLEM_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn snow_golem_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::SnowGolemBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_SNOW_GOLEM,
        texture: SNOW_GOLEM_TEXTURE_REF,
        parts: &SNOW_GOLEM_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn witch_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::WitchBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_WITCH,
        texture: WITCH_TEXTURE_REF,
        parts: &WITCH_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn slime_textured_layer_passes() -> Vec<EntityModelLayerPass> {
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

pub(in crate::entity_models) fn magma_cube_textured_layer_passes() -> Vec<EntityModelLayerPass> {
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

pub(in crate::entity_models) fn ghast_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::GhastBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_GHAST,
        texture: GHAST_TEXTURE_REF,
        parts: &GHAST_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn blaze_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::BlazeBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_BLAZE,
        texture: BLAZE_TEXTURE_REF,
        parts: &BLAZE_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn endermite_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::EndermiteBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_ENDERMITE,
        texture: ENDERMITE_TEXTURE_REF,
        parts: &ENDERMITE_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn silverfish_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::SilverfishBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_SILVERFISH,
        texture: SILVERFISH_TEXTURE_REF,
        parts: &SILVERFISH_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn polar_bear_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::PolarBearBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: if baby {
            MODEL_LAYER_POLAR_BEAR_BABY
        } else {
            MODEL_LAYER_POLAR_BEAR
        },
        texture: if baby {
            POLAR_BEAR_BABY_TEXTURE_REF
        } else {
            POLAR_BEAR_TEXTURE_REF
        },
        parts: if baby {
            &BABY_POLAR_BEAR_TEXTURED_PARTS
        } else {
            &ADULT_POLAR_BEAR_TEXTURED_PARTS
        },
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn ravager_textured_layer_passes() -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::RavagerBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_RAVAGER,
        texture: RAVAGER_TEXTURE_REF,
        parts: &RAVAGER_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn villager_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::VillagerBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: if baby {
            MODEL_LAYER_VILLAGER_BABY
        } else {
            MODEL_LAYER_VILLAGER
        },
        texture: if baby {
            VILLAGER_BABY_TEXTURE_REF
        } else {
            VILLAGER_TEXTURE_REF
        },
        parts: if baby {
            &BABY_VILLAGER_TEXTURED_PARTS
        } else {
            &ADULT_VILLAGER_TEXTURED_PARTS
        },
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn wandering_trader_textured_layer_passes() -> Vec<EntityModelLayerPass>
{
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::WanderingTraderBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: MODEL_LAYER_WANDERING_TRADER,
        texture: WANDERING_TRADER_TEXTURE_REF,
        parts: &ADULT_VILLAGER_TEXTURED_PARTS,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn hoglin_textured_layer_passes(
    family: HoglinModelFamily,
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    let (model_layer, texture, parts) = match (family, baby) {
        (HoglinModelFamily::Hoglin, false) => (
            MODEL_LAYER_HOGLIN,
            HOGLIN_TEXTURE_REF,
            ADULT_HOGLIN_TEXTURED_PARTS.as_slice(),
        ),
        (HoglinModelFamily::Hoglin, true) => (
            MODEL_LAYER_HOGLIN_BABY,
            HOGLIN_BABY_TEXTURE_REF,
            BABY_HOGLIN_TEXTURED_PARTS.as_slice(),
        ),
        (HoglinModelFamily::Zoglin, false) => (
            MODEL_LAYER_ZOGLIN,
            ZOGLIN_TEXTURE_REF,
            ADULT_HOGLIN_TEXTURED_PARTS.as_slice(),
        ),
        (HoglinModelFamily::Zoglin, true) => (
            MODEL_LAYER_ZOGLIN_BABY,
            ZOGLIN_BABY_TEXTURE_REF,
            BABY_HOGLIN_TEXTURED_PARTS.as_slice(),
        ),
    };
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::HoglinBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer,
        texture,
        parts,
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn player_textured_layer_passes(
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

pub(in crate::entity_models) fn sheep_textured_layer_passes(
    baby: bool,
    sheared: bool,
    wool_color: SheepWoolColor,
    invisible: bool,
    jeb: bool,
    age_ticks: f32,
) -> Vec<EntityModelLayerPass> {
    let wool_tint = sheep_wool_render_color(wool_color, jeb, age_ticks);
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
    if !invisible && !baby && (jeb || wool_color != SheepWoolColor::White) {
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
    if !invisible && !sheared {
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

pub(in crate::entity_models) fn wolf_textured_layer_passes(
    baby: bool,
    tame: bool,
    angry: bool,
    invisible: bool,
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
    if !invisible {
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
    }
    passes
}

pub(in crate::entity_models) fn goat_textured_layer_passes(
    baby: bool,
) -> Vec<EntityModelLayerPass> {
    vec![EntityModelLayerPass {
        kind: EntityModelLayerKind::GoatBase,
        render_type: EntityModelLayerRenderType::Cutout,
        model_layer: goat_model_layer(baby),
        texture: goat_texture_ref(baby),
        parts: goat_textured_model_parts(baby).as_slice(),
        visibility: EntityModelLayerVisibility::All,
        tint: [1.0, 1.0, 1.0, 1.0],
        collector_order: 0,
        submit_sequence: 0,
    }]
}

pub(in crate::entity_models) fn skeleton_textured_layer_passes(
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

pub(super) fn player_visible_textured_model_parts(
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

fn goat_model_layer(baby: bool) -> &'static str {
    if baby {
        MODEL_LAYER_GOAT_BABY
    } else {
        MODEL_LAYER_GOAT
    }
}

fn goat_texture_ref(baby: bool) -> EntityModelTextureRef {
    if baby {
        GOAT_BABY_TEXTURE_REF
    } else {
        GOAT_TEXTURE_REF
    }
}

fn goat_textured_model_parts(baby: bool) -> &'static [TexturedModelPartDesc; 6] {
    if baby {
        &BABY_GOAT_TEXTURED_PARTS
    } else {
        &ADULT_GOAT_TEXTURED_PARTS
    }
}

pub(super) fn goat_visible_textured_model_parts(
    baby: bool,
    left_horn: bool,
    right_horn: bool,
) -> [TexturedModelPartDesc; 6] {
    let mut parts = *goat_textured_model_parts(baby);
    let head_index = if baby {
        BABY_GOAT_HEAD_INDEX
    } else {
        ADULT_GOAT_HEAD_INDEX
    };
    parts[head_index].children = if baby {
        baby_goat_head_children(left_horn, right_horn)
    } else {
        adult_goat_head_children(left_horn, right_horn)
    };
    parts
}

fn adult_goat_head_children(left_horn: bool, right_horn: bool) -> &'static [TexturedModelPartDesc] {
    match (left_horn, right_horn) {
        (true, true) => ADULT_GOAT_TEXTURED_HEAD_CHILDREN.as_slice(),
        (true, false) => ADULT_GOAT_TEXTURED_HEAD_CHILDREN_LEFT_ONLY.as_slice(),
        (false, true) => ADULT_GOAT_TEXTURED_HEAD_CHILDREN_RIGHT_ONLY.as_slice(),
        (false, false) => ADULT_GOAT_TEXTURED_HEAD_CHILDREN_NO_HORNS.as_slice(),
    }
}

fn baby_goat_head_children(left_horn: bool, right_horn: bool) -> &'static [TexturedModelPartDesc] {
    match (left_horn, right_horn) {
        (true, true) => BABY_GOAT_TEXTURED_HEAD_CHILDREN.as_slice(),
        (true, false) => BABY_GOAT_TEXTURED_HEAD_CHILDREN_LEFT_ONLY.as_slice(),
        (false, true) => BABY_GOAT_TEXTURED_HEAD_CHILDREN_RIGHT_ONLY.as_slice(),
        (false, false) => BABY_GOAT_TEXTURED_HEAD_CHILDREN_NO_HORNS.as_slice(),
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
