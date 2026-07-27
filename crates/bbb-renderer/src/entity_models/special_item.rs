use std::sync::Arc;

use anyhow::{bail, Result};
use glam::Mat4;

use crate::item_models::{
    ItemModelAtlasKind, ItemModelDrawPhase, ItemModelQuad, SpecialItemBannerAttachment,
    SpecialItemEndCubeEffect, SpecialItemHeadKind, SpecialItemModel, SpecialItemTexture,
};

use super::{
    geometry::{EntityModelTexturedMesh, PART_POSE_ZERO},
    model::{EntityModel, ModelCube, ModelPart},
    model_layers::{
        copper_golem_statue_model, BannerModel, BedModel, BellModel, BookModel, ChestModel,
        ConduitModel, CustomHeadDragonSkullModel, CustomHeadPiglinSkullModel, CustomHeadSkullModel,
        DecoratedPotModel, ShulkerBoxModel, SignModel, TridentModel, BANNER_FLAG_PARTS,
        STANDING_BANNER_FRAME_PARTS, WALL_BANNER_FRAME_PARTS,
    },
    ConduitModelPart, EntityModelInstance, EntityModelKind, EntityModelUvRect,
};

const DECORATED_POT_BASE_PARTS: &[&str] = &["neck", "top", "bottom"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedSpecialItemTexture {
    pub(crate) atlas: ItemModelAtlasKind,
    pub(crate) logical_size: [u32; 2],
    pub(crate) uv: EntityModelUvRect,
    pub(crate) identity: Arc<str>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpecialItemGeometryPart {
    pub(crate) semantic_part: Arc<str>,
    pub(crate) texture_identity: Arc<str>,
    pub(crate) atlas: ItemModelAtlasKind,
    pub(crate) quads: Vec<ItemModelQuad>,
    pub(crate) tint: [f32; 4],
    pub(crate) phase: ItemModelDrawPhase,
    pub(crate) accepts_foil: bool,
}

pub(crate) fn bake_special_item_geometry(
    model: &SpecialItemModel,
    mut resolve: impl FnMut(&SpecialItemTexture) -> Result<ResolvedSpecialItemTexture>,
) -> Result<Vec<SpecialItemGeometryPart>> {
    validate_special_item_parameters(model)?;
    let mut parts = Vec::new();
    match model {
        SpecialItemModel::Bed { texture, part } => {
            let model = BedModel::new(*part);
            push_model_part(
                &mut parts,
                "bed",
                texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                &mut resolve,
            )?;
        }
        SpecialItemModel::Bell { texture } => {
            let mut model = BellModel::new();
            model.prepare(&resting_instance());
            push_model_part(
                &mut parts,
                "bell",
                texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                &mut resolve,
            )?;
        }
        SpecialItemModel::Banner {
            attachment,
            base_texture,
            pattern_base_texture,
            base_color,
            patterns,
        } => {
            let wall = matches!(attachment, SpecialItemBannerAttachment::Wall);
            let mut model = BannerModel::new(wall);
            model.prepare(&resting_instance());
            let frame = if wall {
                WALL_BANNER_FRAME_PARTS
            } else {
                STANDING_BANNER_FRAME_PARTS
            };
            push_model_part(
                &mut parts,
                "banner_frame",
                base_texture,
                model.root(),
                Some(frame),
                [1.0; 4],
                false,
                &mut resolve,
            )?;
            push_model_part(
                &mut parts,
                "banner_flag",
                base_texture,
                model.root(),
                Some(BANNER_FLAG_PARTS),
                [1.0; 4],
                false,
                &mut resolve,
            )?;
            push_model_part_with_phase(
                &mut parts,
                "banner_pattern_base",
                pattern_base_texture,
                model.root(),
                Some(BANNER_FLAG_PARTS),
                base_color.texture_diffuse_color(),
                false,
                ItemModelDrawPhase::Pattern,
                &mut resolve,
            )?;
            for (index, layer) in patterns.iter().take(16).enumerate() {
                push_model_part_with_phase(
                    &mut parts,
                    format!("banner_pattern_{index}"),
                    &layer.texture,
                    model.root(),
                    Some(BANNER_FLAG_PARTS),
                    layer.color.texture_diffuse_color(),
                    false,
                    ItemModelDrawPhase::Pattern,
                    &mut resolve,
                )?;
            }
        }
        SpecialItemModel::Book {
            texture,
            open_angle_radians,
            page1,
            page2,
        } => {
            let mut model = BookModel::new();
            model.apply_special_state(*open_angle_radians, *page1, *page2);
            push_model_part(
                &mut parts,
                "book",
                texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                &mut resolve,
            )?;
        }
        SpecialItemModel::Conduit { texture } => {
            let model = ConduitModel::new(ConduitModelPart::Shell);
            push_model_part(
                &mut parts,
                "conduit",
                texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                &mut resolve,
            )?;
        }
        SpecialItemModel::Chest {
            texture,
            openness,
            half,
        } => {
            let mut model = ChestModel::new(*half);
            model.apply_special_openness(*openness);
            push_model_part(
                &mut parts,
                "chest",
                texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                &mut resolve,
            )?;
        }
        SpecialItemModel::CopperGolemStatue { texture, pose } => {
            let model = copper_golem_statue_model(*pose);
            push_model_part_with_phase(
                &mut parts,
                "copper_golem_statue",
                texture,
                &model,
                None,
                [1.0; 4],
                false,
                ItemModelDrawPhase::CutoutNoCull,
                &mut resolve,
            )?;
        }
        SpecialItemModel::Head {
            texture,
            kind,
            animation,
        } => {
            let instance = resting_instance().with_worn_head_animation_pos(*animation);
            match kind {
                SpecialItemHeadKind::Dragon => {
                    let mut model = CustomHeadDragonSkullModel::new();
                    model.prepare(&instance);
                    push_model_part_with_phase(
                        &mut parts,
                        "dragon_head",
                        texture,
                        model.root(),
                        None,
                        [1.0; 4],
                        false,
                        ItemModelDrawPhase::CutoutZOffset,
                        &mut resolve,
                    )?;
                }
                SpecialItemHeadKind::Piglin => {
                    let mut model = CustomHeadPiglinSkullModel::new();
                    model.prepare(&instance);
                    push_model_part_with_phase(
                        &mut parts,
                        "piglin_head",
                        texture,
                        model.root(),
                        None,
                        [1.0; 4],
                        false,
                        ItemModelDrawPhase::CutoutZOffset,
                        &mut resolve,
                    )?;
                }
                SpecialItemHeadKind::Skeleton
                | SpecialItemHeadKind::WitherSkeleton
                | SpecialItemHeadKind::Zombie
                | SpecialItemHeadKind::Creeper => {
                    let model = CustomHeadSkullModel::new(false);
                    push_model_part_with_phase(
                        &mut parts,
                        "mob_head",
                        texture,
                        model.root(),
                        None,
                        [1.0; 4],
                        false,
                        ItemModelDrawPhase::CutoutZOffset,
                        &mut resolve,
                    )?;
                }
            }
        }
        SpecialItemModel::PlayerHead {
            skin,
            fallback_texture,
        } => {
            let texture = match skin {
                Some(skin) if skin.status == super::EntityDynamicPlayerSkinStatus::Ready => {
                    SpecialItemTexture::DynamicPlayerSkin(*skin)
                }
                _ => SpecialItemTexture::Static(*fallback_texture),
            };
            let model = CustomHeadSkullModel::new(true);
            push_model_part_with_phase(
                &mut parts,
                "player_head",
                &texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                ItemModelDrawPhase::TranslucentNoCull,
                &mut resolve,
            )?;
        }
        SpecialItemModel::ShulkerBox { texture, openness } => {
            let mut model = ShulkerBoxModel::new();
            let instance = resting_instance().with_shulker_box_progress(*openness);
            model.prepare(&instance);
            push_model_part_with_phase(
                &mut parts,
                "shulker_box",
                texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                ItemModelDrawPhase::CutoutNoCull,
                &mut resolve,
            )?;
        }
        SpecialItemModel::Shield {
            base_texture,
            pattern_base_texture,
            base_color,
            patterns,
        } => {
            let model = shield_model();
            push_model_part(
                &mut parts,
                "shield",
                base_texture,
                &model,
                None,
                [1.0; 4],
                true,
                &mut resolve,
            )?;
            if let Some(pattern_base_texture) = pattern_base_texture {
                push_model_part_with_phase(
                    &mut parts,
                    "shield_pattern_base",
                    pattern_base_texture,
                    &model,
                    None,
                    base_color
                        .unwrap_or(super::EntityDyeColor::White)
                        .texture_diffuse_color(),
                    false,
                    ItemModelDrawPhase::Pattern,
                    &mut resolve,
                )?;
                for (index, layer) in patterns.iter().take(16).enumerate() {
                    push_model_part_with_phase(
                        &mut parts,
                        format!("shield_pattern_{index}"),
                        &layer.texture,
                        &model,
                        None,
                        layer.color.texture_diffuse_color(),
                        false,
                        ItemModelDrawPhase::Pattern,
                        &mut resolve,
                    )?;
                }
            }
        }
        SpecialItemModel::Trident { texture } => {
            let model = TridentModel::new();
            push_model_part(
                &mut parts,
                "trident",
                texture,
                model.root(),
                None,
                [1.0; 4],
                true,
                &mut resolve,
            )?;
        }
        SpecialItemModel::DecoratedPot {
            base_texture,
            sides,
            ..
        } => {
            let model = DecoratedPotModel::new();
            push_model_part(
                &mut parts,
                "decorated_pot_base",
                base_texture,
                model.root(),
                Some(DECORATED_POT_BASE_PARTS),
                [1.0; 4],
                false,
                &mut resolve,
            )?;
            // Vanilla submit order is front, back, left, right, while component
            // identity remains back, left, right, front.
            for (name, index) in [("front", 3usize), ("back", 0), ("left", 1), ("right", 2)] {
                push_model_part(
                    &mut parts,
                    format!("decorated_pot_{name}"),
                    &sides[index],
                    model.root(),
                    Some(&[name]),
                    [1.0; 4],
                    false,
                    &mut resolve,
                )?;
            }
        }
        SpecialItemModel::StandingSign {
            texture,
            attachment,
        } => {
            if attachment.is_hanging() {
                bail!("standing-sign special model requires a standing or wall attachment");
            }
            let model = SignModel::new(*attachment);
            push_model_part_with_phase(
                &mut parts,
                "standing_sign",
                texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                ItemModelDrawPhase::CutoutNoCull,
                &mut resolve,
            )?;
        }
        SpecialItemModel::HangingSign {
            texture,
            attachment,
        } => {
            if !attachment.is_hanging() {
                bail!("hanging-sign special model requires a hanging attachment");
            }
            let model = SignModel::new(*attachment);
            push_model_part_with_phase(
                &mut parts,
                "hanging_sign",
                texture,
                model.root(),
                None,
                [1.0; 4],
                false,
                ItemModelDrawPhase::CutoutNoCull,
                &mut resolve,
            )?;
        }
        SpecialItemModel::EndCube {
            sky_texture,
            portal_texture,
            effect,
        } => {
            let sky = resolve(sky_texture)?;
            let portal = resolve(portal_texture)?;
            if sky.atlas != portal.atlas {
                bail!("end-cube sky and portal textures must share one atlas");
            }
            parts.push(SpecialItemGeometryPart {
                semantic_part: Arc::from("end_cube"),
                texture_identity: Arc::from(format!("{}+{}", sky.identity, portal.identity)),
                atlas: portal.atlas,
                quads: end_cube_quads(sky.uv, portal.uv),
                tint: [1.0; 4],
                phase: match effect {
                    SpecialItemEndCubeEffect::Portal => ItemModelDrawPhase::EndPortal,
                    SpecialItemEndCubeEffect::Gateway => ItemModelDrawPhase::EndGateway,
                },
                accepts_foil: false,
            });
        }
    }
    Ok(parts)
}

fn validate_special_item_parameters(model: &SpecialItemModel) -> Result<()> {
    let finite = match model {
        SpecialItemModel::Book {
            open_angle_radians,
            page1,
            page2,
            ..
        } => [*open_angle_radians, *page1, *page2]
            .into_iter()
            .all(f32::is_finite),
        SpecialItemModel::Chest { openness, .. }
        | SpecialItemModel::ShulkerBox { openness, .. } => openness.is_finite(),
        SpecialItemModel::Head { animation, .. } => animation.is_finite(),
        _ => true,
    };
    if !finite {
        bail!(
            "{} special item has a non-finite static parameter",
            model.registered_type_name()
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn push_model_part(
    parts: &mut Vec<SpecialItemGeometryPart>,
    semantic_part: impl Into<Arc<str>>,
    texture: &SpecialItemTexture,
    root: &ModelPart,
    retained: Option<&[&str]>,
    tint: [f32; 4],
    accepts_foil: bool,
    resolve: &mut impl FnMut(&SpecialItemTexture) -> Result<ResolvedSpecialItemTexture>,
) -> Result<()> {
    push_model_part_with_phase(
        parts,
        semantic_part,
        texture,
        root,
        retained,
        tint,
        accepts_foil,
        ItemModelDrawPhase::Solid,
        resolve,
    )
}

#[allow(clippy::too_many_arguments)]
fn push_model_part_with_phase(
    parts: &mut Vec<SpecialItemGeometryPart>,
    semantic_part: impl Into<Arc<str>>,
    texture: &SpecialItemTexture,
    root: &ModelPart,
    retained: Option<&[&str]>,
    tint: [f32; 4],
    accepts_foil: bool,
    phase: ItemModelDrawPhase,
    resolve: &mut impl FnMut(&SpecialItemTexture) -> Result<ResolvedSpecialItemTexture>,
) -> Result<()> {
    let resolved = resolve(texture)?;
    let mut mesh = EntityModelTexturedMesh::new();
    match retained {
        Some(retained) => root.render_textured_retained_with_size(
            &mut mesh,
            Mat4::IDENTITY,
            resolved.logical_size,
            resolved.uv,
            [1.0; 4],
            "",
            retained,
        ),
        None => root.render_textured_with_size(
            &mut mesh,
            Mat4::IDENTITY,
            resolved.logical_size,
            resolved.uv,
            [1.0; 4],
        ),
    }
    let mut quads = textured_mesh_quads(mesh)?;
    if quads.is_empty() {
        bail!("special item geometry part is empty");
    }
    let translucent = matches!(
        phase,
        ItemModelDrawPhase::Pattern
            | ItemModelDrawPhase::Translucent
            | ItemModelDrawPhase::TranslucentNoCull
    );
    for quad in &mut quads {
        quad.translucent = translucent;
    }
    parts.push(SpecialItemGeometryPart {
        semantic_part: semantic_part.into(),
        texture_identity: resolved.identity,
        atlas: resolved.atlas,
        quads,
        tint,
        phase,
        accepts_foil,
    });
    Ok(())
}

fn textured_mesh_quads(mesh: EntityModelTexturedMesh) -> Result<Vec<ItemModelQuad>> {
    if mesh.vertices.len() % 4 != 0 || mesh.indices.len() != mesh.vertices.len() / 4 * 6 {
        bail!("entity model mesh is not composed of independent textured quads");
    }
    let quads = mesh
        .vertices
        .chunks_exact(4)
        .map(|vertices| ItemModelQuad {
            corners: std::array::from_fn(|index| {
                vertices[index].position.map(|coordinate| coordinate * 16.0)
            }),
            uvs: std::array::from_fn(|index| vertices[index].uv),
            tint: [1.0; 4],
            normal: vertices[0].normal,
            shade: 1.0,
            translucent: false,
        })
        .collect::<Vec<_>>();
    if quads.iter().any(|quad| {
        quad.corners
            .iter()
            .flatten()
            .chain(quad.uvs.iter().flatten())
            .chain(quad.normal.iter())
            .any(|value| !value.is_finite())
    }) {
        bail!("special item model produced non-finite geometry");
    }
    Ok(quads)
}

fn resting_instance() -> EntityModelInstance {
    EntityModelInstance::new(0, EntityModelKind::Bell, [0.0; 3], 0.0)
}

fn shield_model() -> ModelPart {
    // Vanilla 26.1 `ShieldModel.createLayer` (64×64).
    ModelPart::new(
        PART_POSE_ZERO,
        Vec::new(),
        vec![
            (
                "plate",
                ModelPart::leaf(
                    PART_POSE_ZERO,
                    vec![ModelCube::new(
                        [-6.0, -11.0, -2.0],
                        [12.0, 22.0, 1.0],
                        [1.0; 4],
                        [12.0, 22.0, 1.0],
                        [0.0, 0.0],
                        false,
                    )],
                ),
            ),
            (
                "handle",
                ModelPart::leaf(
                    PART_POSE_ZERO,
                    vec![ModelCube::new(
                        [-1.0, -3.0, -1.0],
                        [2.0, 6.0, 6.0],
                        [1.0; 4],
                        [2.0, 6.0, 6.0],
                        [26.0, 0.0],
                        false,
                    )],
                ),
            ),
        ],
    )
}

fn end_cube_quads(sky: EntityModelUvRect, portal: EntityModelUvRect) -> Vec<ItemModelQuad> {
    let sky_size = [sky.max[0] - sky.min[0], sky.max[1] - sky.min[1]];
    let portal_size = [portal.max[0] - portal.min[0], portal.max[1] - portal.min[1]];
    let metadata_tint = [sky.min[0], sky.min[1], sky_size[0], sky_size[1]];
    let metadata_normal = [portal.min[0], portal.min[1], portal_size[0]];
    let uvs = [
        [portal.min[0], portal.max[1]],
        [portal.max[0], portal.max[1]],
        [portal.max[0], portal.min[1]],
        [portal.min[0], portal.min[1]],
    ];
    let p = |x, y, z| [x, y, z];
    [
        [
            p(0.0, 0.0, 0.0),
            p(16.0, 0.0, 0.0),
            p(16.0, 0.0, 16.0),
            p(0.0, 0.0, 16.0),
        ],
        [
            p(0.0, 16.0, 16.0),
            p(16.0, 16.0, 16.0),
            p(16.0, 16.0, 0.0),
            p(0.0, 16.0, 0.0),
        ],
        [
            p(0.0, 0.0, 0.0),
            p(0.0, 0.0, 16.0),
            p(0.0, 16.0, 16.0),
            p(0.0, 16.0, 0.0),
        ],
        [
            p(16.0, 0.0, 16.0),
            p(16.0, 0.0, 0.0),
            p(16.0, 16.0, 0.0),
            p(16.0, 16.0, 16.0),
        ],
        [
            p(16.0, 0.0, 0.0),
            p(0.0, 0.0, 0.0),
            p(0.0, 16.0, 0.0),
            p(16.0, 16.0, 0.0),
        ],
        [
            p(0.0, 0.0, 16.0),
            p(16.0, 0.0, 16.0),
            p(16.0, 16.0, 16.0),
            p(0.0, 16.0, 16.0),
        ],
    ]
    .into_iter()
    .map(|corners| ItemModelQuad {
        corners,
        uvs,
        tint: metadata_tint,
        normal: metadata_normal,
        shade: portal_size[1],
        translucent: false,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BedModelPart, ChestModelHalf, EntityDyeColor, EntityModelTextureRef, SignModelAttachment,
        SpecialItemCopperGolemPose, SpecialItemPatternLayer,
    };

    const TEST_TEXTURE: EntityModelTextureRef = EntityModelTextureRef {
        path: "textures/entity/test_special.png",
        size: [64, 64],
    };
    const TEST_TEXTURE_2: EntityModelTextureRef = EntityModelTextureRef {
        path: "textures/entity/test_special_2.png",
        size: [64, 64],
    };

    fn texture() -> SpecialItemTexture {
        SpecialItemTexture::Static(TEST_TEXTURE)
    }

    fn resolve(texture: &SpecialItemTexture) -> Result<ResolvedSpecialItemTexture> {
        let (logical_size, identity, atlas) = match texture {
            SpecialItemTexture::Static(texture) => (
                texture.size,
                Arc::from(format!("static:{}", texture.path)),
                ItemModelAtlasKind::Entity,
            ),
            SpecialItemTexture::Pack(texture) => (
                texture.size,
                Arc::from(format!("pack:{}", texture.path)),
                ItemModelAtlasKind::Entity,
            ),
            SpecialItemTexture::Dynamic {
                texture,
                logical_size,
            } => (
                *logical_size,
                Arc::from(format!("dynamic:{}", texture.handle)),
                ItemModelAtlasKind::DynamicEntity,
            ),
            SpecialItemTexture::DynamicPlayerSkin(skin) => (
                [64, 64],
                Arc::from(format!("skin:{}", skin.handle)),
                ItemModelAtlasKind::DynamicPlayerSkin,
            ),
        };
        Ok(ResolvedSpecialItemTexture {
            atlas,
            logical_size,
            uv: EntityModelUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            identity,
        })
    }

    fn all_registered_models() -> Vec<SpecialItemModel> {
        let pattern = SpecialItemPatternLayer::new(texture(), EntityDyeColor::Red);
        vec![
            SpecialItemModel::Bed {
                texture: texture(),
                part: BedModelPart::Head,
            },
            SpecialItemModel::Bell { texture: texture() },
            SpecialItemModel::Banner {
                attachment: SpecialItemBannerAttachment::Ground,
                base_texture: texture(),
                pattern_base_texture: texture(),
                base_color: EntityDyeColor::Blue,
                patterns: Arc::from([pattern.clone()]),
            },
            SpecialItemModel::Book {
                texture: texture(),
                open_angle_radians: 0.7,
                page1: 0.2,
                page2: 0.8,
            },
            SpecialItemModel::Conduit { texture: texture() },
            SpecialItemModel::Chest {
                texture: texture(),
                openness: 0.5,
                half: ChestModelHalf::Single,
            },
            SpecialItemModel::CopperGolemStatue {
                texture: texture(),
                pose: SpecialItemCopperGolemPose::Running,
            },
            SpecialItemModel::Head {
                texture: texture(),
                kind: SpecialItemHeadKind::Dragon,
                animation: 1.5,
            },
            SpecialItemModel::PlayerHead {
                skin: None,
                fallback_texture: TEST_TEXTURE,
            },
            SpecialItemModel::ShulkerBox {
                texture: texture(),
                openness: 0.25,
            },
            SpecialItemModel::Shield {
                base_texture: texture(),
                pattern_base_texture: Some(texture()),
                base_color: Some(EntityDyeColor::White),
                patterns: Arc::from([pattern]),
            },
            SpecialItemModel::Trident { texture: texture() },
            SpecialItemModel::DecoratedPot {
                base_texture: texture(),
                sides: std::array::from_fn(|_| texture()),
                decorations: [None; 4],
            },
            SpecialItemModel::StandingSign {
                texture: texture(),
                attachment: SignModelAttachment::Standing,
            },
            SpecialItemModel::HangingSign {
                texture: texture(),
                attachment: SignModelAttachment::HangingCeiling,
            },
            SpecialItemModel::EndCube {
                sky_texture: texture(),
                portal_texture: SpecialItemTexture::Static(TEST_TEXTURE_2),
                effect: SpecialItemEndCubeEffect::Portal,
            },
        ]
    }

    #[test]
    fn special_item_builders_cover_exact_vanilla_registration_set() {
        let models = all_registered_models();
        assert_eq!(models.len(), 16);
        assert_eq!(
            models
                .iter()
                .map(SpecialItemModel::registered_type_name)
                .collect::<Vec<_>>(),
            vec![
                "bed",
                "bell",
                "banner",
                "book",
                "conduit",
                "chest",
                "copper_golem_statue",
                "head",
                "player_head",
                "shulker_box",
                "shield",
                "trident",
                "decorated_pot",
                "standing_sign",
                "hanging_sign",
                "end_cube",
            ]
        );
        for model in &models {
            let parts = bake_special_item_geometry(model, resolve).unwrap();
            assert!(
                !parts.is_empty(),
                "{} produced no parts",
                model.registered_type_name()
            );
            assert!(parts.iter().all(|part| !part.quads.is_empty()));
        }
    }

    #[test]
    fn special_item_layers_keep_vanilla_pattern_pot_and_foil_lanes() {
        let models = all_registered_models();
        for index in 0..=5 {
            assert_eq!(
                bake_special_item_geometry(&models[index], resolve).unwrap()[0].phase,
                ItemModelDrawPhase::Solid
            );
        }
        assert_eq!(
            bake_special_item_geometry(&models[6], resolve).unwrap()[0].phase,
            ItemModelDrawPhase::CutoutNoCull
        );
        assert_eq!(
            bake_special_item_geometry(&models[7], resolve).unwrap()[0].phase,
            ItemModelDrawPhase::CutoutZOffset
        );
        assert_eq!(
            bake_special_item_geometry(&models[8], resolve).unwrap()[0].phase,
            ItemModelDrawPhase::TranslucentNoCull
        );
        assert_eq!(
            bake_special_item_geometry(&models[9], resolve).unwrap()[0].phase,
            ItemModelDrawPhase::CutoutNoCull
        );
        for index in [11usize, 12] {
            assert_eq!(
                bake_special_item_geometry(&models[index], resolve).unwrap()[0].phase,
                ItemModelDrawPhase::Solid
            );
        }
        for index in [13usize, 14] {
            assert_eq!(
                bake_special_item_geometry(&models[index], resolve).unwrap()[0].phase,
                ItemModelDrawPhase::CutoutNoCull
            );
        }

        let banner = bake_special_item_geometry(&models[2], resolve).unwrap();
        assert_eq!(banner.len(), 4);
        assert_eq!(banner[2].tint, EntityDyeColor::Blue.texture_diffuse_color());
        assert_eq!(banner[3].tint, EntityDyeColor::Red.texture_diffuse_color());
        assert!(banner[..2]
            .iter()
            .all(|part| part.phase == ItemModelDrawPhase::Solid));
        assert!(banner[2..]
            .iter()
            .all(|part| part.phase == ItemModelDrawPhase::Pattern));

        let shield = bake_special_item_geometry(&models[10], resolve).unwrap();
        assert_eq!(shield.len(), 3);
        assert!(shield[0].accepts_foil);
        assert!(shield[1..].iter().all(|part| !part.accepts_foil));
        assert_eq!(shield[0].phase, ItemModelDrawPhase::Solid);
        assert!(shield[1..]
            .iter()
            .all(|part| part.phase == ItemModelDrawPhase::Pattern));

        let trident = bake_special_item_geometry(&models[11], resolve).unwrap();
        assert!(trident[0].accepts_foil);
        assert!(models
            .iter()
            .enumerate()
            .filter(|(index, _)| !matches!(index, 10 | 11))
            .flat_map(|(_, model)| bake_special_item_geometry(model, resolve).unwrap())
            .all(|part| !part.accepts_foil));

        let pot = bake_special_item_geometry(&models[12], resolve).unwrap();
        assert_eq!(
            pot.iter()
                .map(|part| part.semantic_part.as_ref())
                .collect::<Vec<_>>(),
            vec![
                "decorated_pot_base",
                "decorated_pot_front",
                "decorated_pot_back",
                "decorated_pot_left",
                "decorated_pot_right",
            ]
        );
    }

    #[test]
    fn special_item_end_cube_preserves_both_atlas_rects_and_effect_phase() {
        let model = SpecialItemModel::EndCube {
            sky_texture: texture(),
            portal_texture: SpecialItemTexture::Static(TEST_TEXTURE_2),
            effect: SpecialItemEndCubeEffect::Gateway,
        };
        let parts = bake_special_item_geometry(&model, |texture| {
            let mut resolved = resolve(texture)?;
            if matches!(texture, SpecialItemTexture::Static(TEST_TEXTURE_2)) {
                resolved.uv = EntityModelUvRect {
                    min: [0.5, 0.25],
                    max: [0.75, 0.75],
                };
            } else {
                resolved.uv = EntityModelUvRect {
                    min: [0.0, 0.0],
                    max: [0.25, 0.5],
                };
            }
            Ok(resolved)
        })
        .unwrap();
        assert_eq!(parts[0].phase, ItemModelDrawPhase::EndGateway);
        assert_eq!(parts[0].quads.len(), 6);
        assert_eq!(parts[0].quads[0].tint, [0.0, 0.0, 0.25, 0.5]);
        assert_eq!(parts[0].quads[0].normal, [0.5, 0.25, 0.25]);
        assert_eq!(parts[0].quads[0].shade, 0.5);
    }

    #[test]
    fn special_item_book_and_statue_parameters_change_static_geometry() {
        let closed = SpecialItemModel::Book {
            texture: texture(),
            open_angle_radians: 0.0,
            page1: 0.0,
            page2: 0.0,
        };
        let open = SpecialItemModel::Book {
            texture: texture(),
            open_angle_radians: 1.0,
            page1: 0.25,
            page2: 0.75,
        };
        assert_ne!(
            bake_special_item_geometry(&closed, resolve).unwrap()[0].quads,
            bake_special_item_geometry(&open, resolve).unwrap()[0].quads
        );

        let statue_quads = [
            SpecialItemCopperGolemPose::Standing,
            SpecialItemCopperGolemPose::Sitting,
            SpecialItemCopperGolemPose::Star,
            SpecialItemCopperGolemPose::Running,
        ]
        .map(|pose| {
            bake_special_item_geometry(
                &SpecialItemModel::CopperGolemStatue {
                    texture: texture(),
                    pose,
                },
                resolve,
            )
            .unwrap()[0]
                .quads
                .clone()
        });
        for left in 0..statue_quads.len() {
            for right in left + 1..statue_quads.len() {
                assert_ne!(statue_quads[left], statue_quads[right]);
            }
        }
    }
}
