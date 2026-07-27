//! Native projection of parsed `minecraft:special` item leaves.
//!
//! `bbb-item-model` owns tree selection and keeps the parsed pack value
//! renderer-neutral. This module owns the one conversion boundary from pack
//! identifiers plus effective stack components into renderer model values.

use std::{collections::BTreeSet, sync::Arc};

use bbb_item_model::{ItemModelLeafTransformation, NativeItemRuntime};
use bbb_pack::{
    CopperGolemStatuePose, ItemSpecialBedPart, ItemSpecialChestType, ItemSpecialEndCubeEffect,
    ItemSpecialHangingAttachment, ItemSpecialModel, ItemSpecialSkullKind,
    ItemSpecialStandingAttachment, ResourceLocation,
};
use bbb_protocol::packets::{BannerPatternLayerSummary, ItemStackSummary};
use bbb_renderer::{
    entity_model_texture_refs, BannerPatternKind, BedModelPart, ChestModelHalf,
    DecoratedPotPattern, EntityDefaultPlayerSkin, EntityDyeColor, EntityDynamicPlayerSkinStatus,
    EntityModelOwnedTextureRef, EntityModelTextureRef, EntityPlayerSkin, SignModelAttachment,
    SpecialItemBannerAttachment, SpecialItemCopperGolemPose, SpecialItemEndCubeEffect,
    SpecialItemHeadKind, SpecialItemModel as RendererSpecialItemModel, SpecialItemPatternLayer,
    SpecialItemTexture,
};
use bbb_world::WorldStore;
use glam::Mat4;

use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;

const BED_LOGICAL_SIZE: [u32; 2] = [64, 64];
const CHEST_LOGICAL_SIZE: [u32; 2] = [64, 64];
const COPPER_GOLEM_LOGICAL_SIZE: [u32; 2] = [64, 64];
const SIGN_LOGICAL_SIZE: [u32; 2] = [64, 32];
const SHULKER_LOGICAL_SIZE: [u32; 2] = [64, 64];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SpecialItemPackTextureDescriptor {
    pub path: String,
    pub logical_size: [u32; 2],
}

/// Exact explicit pack textures referenced by parsed special model payloads.
/// Built-in fixed textures and component-driven atlas sprites are owned by
/// their existing static/dynamic catalogs and therefore are not returned.
pub(crate) fn special_item_pack_texture_descriptors<'a>(
    models: impl IntoIterator<Item = &'a ItemSpecialModel>,
) -> Vec<SpecialItemPackTextureDescriptor> {
    let mut descriptors = BTreeSet::new();
    for model in models {
        let descriptor = match model {
            ItemSpecialModel::Bed { texture, .. } => {
                mapped_texture_descriptor(texture, "entity/bed", BED_LOGICAL_SIZE)
            }
            ItemSpecialModel::Chest { texture, .. } => {
                mapped_texture_descriptor(texture, "entity/chest", CHEST_LOGICAL_SIZE)
            }
            ItemSpecialModel::CopperGolemStatue { texture, .. } => {
                direct_texture_descriptor(texture, COPPER_GOLEM_LOGICAL_SIZE)
            }
            ItemSpecialModel::Head {
                kind,
                texture: Some(texture),
                ..
            } => mapped_texture_descriptor(texture, "entity", head_logical_size(*kind)),
            ItemSpecialModel::ShulkerBox { texture, .. } => {
                mapped_texture_descriptor(texture, "entity/shulker", SHULKER_LOGICAL_SIZE)
            }
            ItemSpecialModel::StandingSign {
                texture: Some(texture),
                ..
            } => mapped_texture_descriptor(texture, "entity/signs", SIGN_LOGICAL_SIZE),
            ItemSpecialModel::HangingSign {
                texture: Some(texture),
                ..
            } => mapped_texture_descriptor(texture, "entity/signs/hanging", SIGN_LOGICAL_SIZE),
            ItemSpecialModel::Bell
            | ItemSpecialModel::Banner { .. }
            | ItemSpecialModel::Book { .. }
            | ItemSpecialModel::Conduit
            | ItemSpecialModel::Head { texture: None, .. }
            | ItemSpecialModel::PlayerHead
            | ItemSpecialModel::Shield
            | ItemSpecialModel::Trident
            | ItemSpecialModel::DecoratedPot
            | ItemSpecialModel::StandingSign { texture: None, .. }
            | ItemSpecialModel::HangingSign { texture: None, .. }
            | ItemSpecialModel::EndCube { .. } => None,
        };
        if let Some(descriptor) = descriptor {
            if static_texture_ref(&descriptor.path).is_none() {
                descriptors.insert(descriptor);
            }
        }
    }
    descriptors.into_iter().collect()
}

pub(crate) fn special_item_leaf_matrix(transformation: ItemModelLeafTransformation) -> Mat4 {
    Mat4::from_cols_array_2d(&transformation.columns)
}

pub(crate) fn project_special_item_model(
    model: &ItemSpecialModel,
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
    world: Option<&WorldStore>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
) -> Option<RendererSpecialItemModel> {
    Some(match model {
        ItemSpecialModel::Bed { texture, part } => RendererSpecialItemModel::Bed {
            texture: mapped_special_texture(texture, "entity/bed", BED_LOGICAL_SIZE)?,
            part: match part {
                ItemSpecialBedPart::Head => BedModelPart::Head,
                ItemSpecialBedPart::Foot => BedModelPart::Foot,
            },
        },
        ItemSpecialModel::Bell => RendererSpecialItemModel::Bell {
            texture: fixed_special_texture("textures/entity/bell/bell_body.png", [32, 32])?,
        },
        ItemSpecialModel::Banner { color, attachment } => RendererSpecialItemModel::Banner {
            attachment: match attachment {
                ItemSpecialStandingAttachment::Ground => SpecialItemBannerAttachment::Ground,
                ItemSpecialStandingAttachment::Wall => SpecialItemBannerAttachment::Wall,
            },
            base_texture: fixed_special_texture(
                "textures/entity/banner/banner_base.png",
                [64, 64],
            )?,
            pattern_base_texture: fixed_special_texture(
                "textures/entity/banner/base.png",
                [64, 64],
            )?,
            base_color: dye_color_name(color)?,
            patterns: project_banner_patterns(
                &stack.component_patch.banner_pattern_layers,
                false,
                world,
                variant_textures,
            )
            .into(),
        },
        ItemSpecialModel::Book {
            open_angle,
            page1,
            page2,
        } => RendererSpecialItemModel::Book {
            texture: fixed_special_texture(
                "textures/entity/enchantment/enchanting_table_book.png",
                [64, 32],
            )?,
            open_angle_radians: open_angle.to_radians(),
            page1: *page1,
            page2: *page2,
        },
        ItemSpecialModel::Conduit => RendererSpecialItemModel::Conduit {
            texture: fixed_special_texture("textures/entity/conduit/base.png", [32, 16])?,
        },
        ItemSpecialModel::Chest {
            texture,
            openness,
            chest_type,
        } => RendererSpecialItemModel::Chest {
            texture: mapped_special_texture(texture, "entity/chest", CHEST_LOGICAL_SIZE)?,
            openness: *openness,
            half: match chest_type {
                ItemSpecialChestType::Single => ChestModelHalf::Single,
                ItemSpecialChestType::Left => ChestModelHalf::Left,
                ItemSpecialChestType::Right => ChestModelHalf::Right,
            },
        },
        ItemSpecialModel::CopperGolemStatue { pose, texture } => {
            RendererSpecialItemModel::CopperGolemStatue {
                texture: direct_special_texture(texture, COPPER_GOLEM_LOGICAL_SIZE)?,
                pose: match pose {
                    CopperGolemStatuePose::Standing => SpecialItemCopperGolemPose::Standing,
                    CopperGolemStatuePose::Sitting => SpecialItemCopperGolemPose::Sitting,
                    CopperGolemStatuePose::Star => SpecialItemCopperGolemPose::Star,
                    CopperGolemStatuePose::Running => SpecialItemCopperGolemPose::Running,
                },
            }
        }
        ItemSpecialModel::Head {
            kind,
            texture,
            animation,
        } => RendererSpecialItemModel::Head {
            texture: match texture {
                Some(texture) => {
                    mapped_special_texture(texture, "entity", head_logical_size(*kind))?
                }
                None => fixed_special_texture(
                    default_head_texture_path(*kind)?,
                    head_logical_size(*kind),
                )?,
            },
            kind: renderer_head_kind(*kind),
            animation: *animation,
        },
        ItemSpecialModel::PlayerHead => {
            let skin = stack
                .component_patch
                .profile
                .as_ref()
                .map(|profile| item_runtime.player_skin_for_profile(profile))
                .unwrap_or(EntityPlayerSkin::Default(
                    EntityDefaultPlayerSkin::SlimSteve,
                ));
            let dynamic = match skin {
                EntityPlayerSkin::Dynamic(dynamic)
                    if dynamic.status == EntityDynamicPlayerSkinStatus::Ready =>
                {
                    Some(dynamic)
                }
                _ => None,
            };
            RendererSpecialItemModel::PlayerHead {
                skin: dynamic,
                fallback_texture: default_player_skin_texture_ref(skin.fallback())?,
            }
        }
        ItemSpecialModel::ShulkerBox { texture, openness } => {
            RendererSpecialItemModel::ShulkerBox {
                texture: mapped_special_texture(texture, "entity/shulker", SHULKER_LOGICAL_SIZE)?,
                openness: *openness,
            }
        }
        ItemSpecialModel::Shield => {
            let patterns = project_banner_patterns(
                &stack.component_patch.banner_pattern_layers,
                true,
                world,
                variant_textures,
            );
            let base_color = stack.component_patch.base_color_id.and_then(dye_color_id);
            let decorated = base_color.is_some() || !patterns.is_empty();
            RendererSpecialItemModel::Shield {
                base_texture: fixed_special_texture(
                    if decorated {
                        "textures/entity/shield/shield_base.png"
                    } else {
                        "textures/entity/shield/shield_base_nopattern.png"
                    },
                    [64, 64],
                )?,
                pattern_base_texture: if decorated {
                    Some(fixed_special_texture(
                        "textures/entity/shield/base.png",
                        [64, 64],
                    )?)
                } else {
                    None
                },
                base_color,
                patterns: patterns.into(),
            }
        }
        ItemSpecialModel::Trident => RendererSpecialItemModel::Trident {
            texture: fixed_special_texture("textures/entity/trident/trident.png", [32, 32])?,
        },
        ItemSpecialModel::DecoratedPot => {
            let mut decorations = [None; 4];
            for (target, item_id) in decorations
                .iter_mut()
                .zip(&stack.component_patch.pot_decorations_item_ids)
            {
                *target = item_runtime
                    .item_resource_id(*item_id)
                    .and_then(DecoratedPotPattern::for_sherd_item_resource_id);
            }
            let sides = decorations.map(|pattern| {
                let path = pattern.map_or_else(
                    || "textures/entity/decorated_pot/decorated_pot_side.png".to_string(),
                    decorated_pot_pattern_texture_path,
                );
                fixed_special_texture(&path, [16, 16])
            });
            RendererSpecialItemModel::DecoratedPot {
                base_texture: fixed_special_texture(
                    "textures/entity/decorated_pot/decorated_pot_base.png",
                    [32, 32],
                )?,
                sides: [
                    sides[0].clone()?,
                    sides[1].clone()?,
                    sides[2].clone()?,
                    sides[3].clone()?,
                ],
                decorations,
            }
        }
        ItemSpecialModel::StandingSign {
            wood_type,
            attachment,
            texture,
        } => RendererSpecialItemModel::StandingSign {
            texture: match texture {
                Some(texture) => {
                    mapped_special_texture(texture, "entity/signs", SIGN_LOGICAL_SIZE)?
                }
                None => fixed_special_texture(
                    &format!("textures/entity/signs/{wood_type}.png"),
                    SIGN_LOGICAL_SIZE,
                )?,
            },
            attachment: match attachment {
                ItemSpecialStandingAttachment::Ground => SignModelAttachment::Standing,
                ItemSpecialStandingAttachment::Wall => SignModelAttachment::Wall,
            },
        },
        ItemSpecialModel::HangingSign {
            wood_type,
            attachment,
            texture,
        } => RendererSpecialItemModel::HangingSign {
            texture: match texture {
                Some(texture) => {
                    mapped_special_texture(texture, "entity/signs/hanging", SIGN_LOGICAL_SIZE)?
                }
                None => fixed_special_texture(
                    &format!("textures/entity/signs/hanging/{wood_type}.png"),
                    SIGN_LOGICAL_SIZE,
                )?,
            },
            attachment: match attachment {
                ItemSpecialHangingAttachment::Wall => SignModelAttachment::HangingWall,
                ItemSpecialHangingAttachment::Ceiling => SignModelAttachment::HangingCeiling,
                ItemSpecialHangingAttachment::CeilingMiddle => {
                    SignModelAttachment::HangingCeilingMiddle
                }
            },
        },
        ItemSpecialModel::EndCube { effect } => RendererSpecialItemModel::EndCube {
            sky_texture: fixed_special_texture("textures/environment/end_sky.png", [128, 128])?,
            portal_texture: fixed_special_texture(
                "textures/entity/end_portal/end_portal.png",
                [256, 256],
            )?,
            effect: match effect {
                ItemSpecialEndCubeEffect::Portal => SpecialItemEndCubeEffect::Portal,
                ItemSpecialEndCubeEffect::Gateway => SpecialItemEndCubeEffect::Gateway,
            },
        },
    })
}

fn project_banner_patterns(
    layers: &[BannerPatternLayerSummary],
    shield: bool,
    world: Option<&WorldStore>,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
) -> Vec<SpecialItemPatternLayer> {
    layers
        .iter()
        .take(16)
        .filter_map(|layer| {
            let asset_id = layer.asset_id.clone().or_else(|| {
                let holder_id = layer.registry_id?;
                world
                    .and_then(|world| {
                        world
                            .synchronized_banner_pattern_presentation(holder_id)
                            .ok()
                            .flatten()
                            .map(|presentation| presentation.asset_id)
                    })
                    .or_else(|| {
                        BannerPatternKind::for_vanilla_registry_id(holder_id)
                            .map(|pattern| pattern.resource_id().to_string())
                    })
            })?;
            let mapper = if shield {
                "entity/shield"
            } else {
                "entity/banner"
            };
            let sprite_id = mapped_sprite_id(&asset_id, mapper)?;
            let dynamic = variant_textures.and_then(|textures| {
                if shield {
                    textures.ready_shield_pattern_texture(&sprite_id)
                } else {
                    textures.ready_banner_pattern_texture(&sprite_id)
                }
            });
            let texture = dynamic.map_or_else(
                || mapped_special_texture(&asset_id, mapper, [64, 64]),
                |texture| {
                    Some(SpecialItemTexture::Dynamic {
                        texture,
                        logical_size: [64, 64],
                    })
                },
            )?;
            Some(SpecialItemPatternLayer::new(
                texture,
                dye_color_id(layer.color_id)?,
            ))
        })
        .collect()
}

fn mapped_sprite_id(asset_id: &str, prefix: &str) -> Option<String> {
    let asset = ResourceLocation::parse(asset_id).ok()?;
    Some(format!("{}:{prefix}/{}", asset.namespace(), asset.path()))
}

fn mapped_texture_descriptor(
    asset_id: &str,
    prefix: &str,
    logical_size: [u32; 2],
) -> Option<SpecialItemPackTextureDescriptor> {
    let asset = ResourceLocation::parse(asset_id).ok()?;
    Some(SpecialItemPackTextureDescriptor {
        path: canonical_texture_path(
            asset.namespace(),
            &format!("textures/{prefix}/{}.png", asset.path()),
        ),
        logical_size,
    })
}

fn direct_texture_descriptor(
    texture_id: &str,
    logical_size: [u32; 2],
) -> Option<SpecialItemPackTextureDescriptor> {
    let texture = ResourceLocation::parse(texture_id).ok()?;
    Some(SpecialItemPackTextureDescriptor {
        path: canonical_texture_path(texture.namespace(), texture.path()),
        logical_size,
    })
}

fn canonical_texture_path(namespace: &str, path: &str) -> String {
    if namespace == "minecraft" {
        path.to_string()
    } else {
        format!("{namespace}:{path}")
    }
}

fn mapped_special_texture(
    asset_id: &str,
    prefix: &str,
    logical_size: [u32; 2],
) -> Option<SpecialItemTexture> {
    let descriptor = mapped_texture_descriptor(asset_id, prefix, logical_size)?;
    Some(special_texture(descriptor.path, descriptor.logical_size))
}

fn direct_special_texture(texture_id: &str, logical_size: [u32; 2]) -> Option<SpecialItemTexture> {
    let descriptor = direct_texture_descriptor(texture_id, logical_size)?;
    Some(special_texture(descriptor.path, descriptor.logical_size))
}

fn fixed_special_texture(path: &str, logical_size: [u32; 2]) -> Option<SpecialItemTexture> {
    Some(special_texture(path.to_string(), logical_size))
}

fn special_texture(path: String, logical_size: [u32; 2]) -> SpecialItemTexture {
    static_texture_ref(&path).map_or_else(
        || {
            SpecialItemTexture::Pack(EntityModelOwnedTextureRef::new(
                Arc::<str>::from(path),
                logical_size,
            ))
        },
        SpecialItemTexture::Static,
    )
}

fn static_texture_ref(path: &str) -> Option<EntityModelTextureRef> {
    entity_model_texture_refs()
        .iter()
        .copied()
        .find(|texture| texture.path == path)
}

fn default_player_skin_texture_ref(skin: EntityDefaultPlayerSkin) -> Option<EntityModelTextureRef> {
    let (model, name) = match skin {
        EntityDefaultPlayerSkin::SlimAlex => ("slim", "alex"),
        EntityDefaultPlayerSkin::SlimAri => ("slim", "ari"),
        EntityDefaultPlayerSkin::SlimEfe => ("slim", "efe"),
        EntityDefaultPlayerSkin::SlimKai => ("slim", "kai"),
        EntityDefaultPlayerSkin::SlimMakena => ("slim", "makena"),
        EntityDefaultPlayerSkin::SlimNoor => ("slim", "noor"),
        EntityDefaultPlayerSkin::SlimSteve => ("slim", "steve"),
        EntityDefaultPlayerSkin::SlimSunny => ("slim", "sunny"),
        EntityDefaultPlayerSkin::SlimZuri => ("slim", "zuri"),
        EntityDefaultPlayerSkin::WideAlex => ("wide", "alex"),
        EntityDefaultPlayerSkin::WideAri => ("wide", "ari"),
        EntityDefaultPlayerSkin::WideEfe => ("wide", "efe"),
        EntityDefaultPlayerSkin::WideKai => ("wide", "kai"),
        EntityDefaultPlayerSkin::WideMakena => ("wide", "makena"),
        EntityDefaultPlayerSkin::WideNoor => ("wide", "noor"),
        EntityDefaultPlayerSkin::WideSteve => ("wide", "steve"),
        EntityDefaultPlayerSkin::WideSunny => ("wide", "sunny"),
        EntityDefaultPlayerSkin::WideZuri => ("wide", "zuri"),
    };
    static_texture_ref(&format!("textures/entity/player/{model}/{name}.png"))
}

fn renderer_head_kind(kind: ItemSpecialSkullKind) -> SpecialItemHeadKind {
    match kind {
        ItemSpecialSkullKind::Skeleton => SpecialItemHeadKind::Skeleton,
        ItemSpecialSkullKind::WitherSkeleton => SpecialItemHeadKind::WitherSkeleton,
        ItemSpecialSkullKind::Player => SpecialItemHeadKind::Player,
        ItemSpecialSkullKind::Zombie => SpecialItemHeadKind::Zombie,
        ItemSpecialSkullKind::Creeper => SpecialItemHeadKind::Creeper,
        ItemSpecialSkullKind::Dragon => SpecialItemHeadKind::Dragon,
        ItemSpecialSkullKind::Piglin => SpecialItemHeadKind::Piglin,
    }
}

fn default_head_texture_path(kind: ItemSpecialSkullKind) -> Option<&'static str> {
    match kind {
        ItemSpecialSkullKind::Skeleton => Some("textures/entity/skeleton/skeleton.png"),
        ItemSpecialSkullKind::WitherSkeleton => {
            Some("textures/entity/skeleton/wither_skeleton.png")
        }
        ItemSpecialSkullKind::Player => Some("textures/entity/player/slim/steve.png"),
        ItemSpecialSkullKind::Zombie => Some("textures/entity/zombie/zombie.png"),
        ItemSpecialSkullKind::Creeper => Some("textures/entity/creeper/creeper.png"),
        ItemSpecialSkullKind::Dragon => Some("textures/entity/enderdragon/dragon.png"),
        ItemSpecialSkullKind::Piglin => Some("textures/entity/piglin/piglin.png"),
    }
}

fn head_logical_size(kind: ItemSpecialSkullKind) -> [u32; 2] {
    match kind {
        ItemSpecialSkullKind::Skeleton
        | ItemSpecialSkullKind::WitherSkeleton
        | ItemSpecialSkullKind::Creeper => [64, 32],
        ItemSpecialSkullKind::Dragon => [256, 256],
        ItemSpecialSkullKind::Player
        | ItemSpecialSkullKind::Zombie
        | ItemSpecialSkullKind::Piglin => [64, 64],
    }
}

fn decorated_pot_pattern_texture_path(pattern: DecoratedPotPattern) -> String {
    let name = pattern
        .resource_id()
        .split_once(':')
        .map_or(pattern.resource_id(), |(_, path)| path);
    format!("textures/entity/decorated_pot/{name}_pottery_pattern.png")
}

fn dye_color_name(name: &str) -> Option<EntityDyeColor> {
    match name {
        "white" => Some(EntityDyeColor::White),
        "orange" => Some(EntityDyeColor::Orange),
        "magenta" => Some(EntityDyeColor::Magenta),
        "light_blue" => Some(EntityDyeColor::LightBlue),
        "yellow" => Some(EntityDyeColor::Yellow),
        "lime" => Some(EntityDyeColor::Lime),
        "pink" => Some(EntityDyeColor::Pink),
        "gray" => Some(EntityDyeColor::Gray),
        "light_gray" => Some(EntityDyeColor::LightGray),
        "cyan" => Some(EntityDyeColor::Cyan),
        "purple" => Some(EntityDyeColor::Purple),
        "blue" => Some(EntityDyeColor::Blue),
        "brown" => Some(EntityDyeColor::Brown),
        "green" => Some(EntityDyeColor::Green),
        "red" => Some(EntityDyeColor::Red),
        "black" => Some(EntityDyeColor::Black),
        _ => None,
    }
}

fn dye_color_id(id: i32) -> Option<EntityDyeColor> {
    match id {
        0 => Some(EntityDyeColor::White),
        1 => Some(EntityDyeColor::Orange),
        2 => Some(EntityDyeColor::Magenta),
        3 => Some(EntityDyeColor::LightBlue),
        4 => Some(EntityDyeColor::Yellow),
        5 => Some(EntityDyeColor::Lime),
        6 => Some(EntityDyeColor::Pink),
        7 => Some(EntityDyeColor::Gray),
        8 => Some(EntityDyeColor::LightGray),
        9 => Some(EntityDyeColor::Cyan),
        10 => Some(EntityDyeColor::Purple),
        11 => Some(EntityDyeColor::Blue),
        12 => Some(EntityDyeColor::Brown),
        13 => Some(EntityDyeColor::Green),
        14 => Some(EntityDyeColor::Red),
        15 => Some(EntityDyeColor::Black),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_pack_textures_follow_each_vanilla_mapper_and_logical_size() {
        let models = [
            ItemSpecialModel::Bed {
                texture: "example:scarlet".to_string(),
                part: ItemSpecialBedPart::Head,
            },
            ItemSpecialModel::Chest {
                texture: "example:ancient".to_string(),
                openness: 0.0,
                chest_type: ItemSpecialChestType::Single,
            },
            ItemSpecialModel::CopperGolemStatue {
                pose: CopperGolemStatuePose::Standing,
                texture: "example:textures/entity/statue/copper.png".to_string(),
            },
            ItemSpecialModel::Head {
                kind: ItemSpecialSkullKind::Dragon,
                texture: Some("example:obsidian_dragon".to_string()),
                animation: 0.0,
            },
            ItemSpecialModel::StandingSign {
                wood_type: "oak".to_string(),
                attachment: ItemSpecialStandingAttachment::Ground,
                texture: Some("example:ashen".to_string()),
            },
            ItemSpecialModel::HangingSign {
                wood_type: "oak".to_string(),
                attachment: ItemSpecialHangingAttachment::Ceiling,
                texture: Some("example:ashen".to_string()),
            },
        ];

        assert_eq!(
            special_item_pack_texture_descriptors(&models),
            vec![
                SpecialItemPackTextureDescriptor {
                    path: "example:textures/entity/bed/scarlet.png".to_string(),
                    logical_size: [64, 64],
                },
                SpecialItemPackTextureDescriptor {
                    path: "example:textures/entity/chest/ancient.png".to_string(),
                    logical_size: [64, 64],
                },
                SpecialItemPackTextureDescriptor {
                    path: "example:textures/entity/obsidian_dragon.png".to_string(),
                    logical_size: [256, 256],
                },
                SpecialItemPackTextureDescriptor {
                    path: "example:textures/entity/signs/ashen.png".to_string(),
                    logical_size: [64, 32],
                },
                SpecialItemPackTextureDescriptor {
                    path: "example:textures/entity/signs/hanging/ashen.png".to_string(),
                    logical_size: [64, 32],
                },
                SpecialItemPackTextureDescriptor {
                    path: "example:textures/entity/statue/copper.png".to_string(),
                    logical_size: [64, 64],
                },
            ]
        );
    }

    #[test]
    fn player_head_without_override_uses_vanilla_default_skin() {
        assert_eq!(
            default_head_texture_path(ItemSpecialSkullKind::Player),
            Some("textures/entity/player/slim/steve.png")
        );
        assert_eq!(head_logical_size(ItemSpecialSkullKind::Player), [64, 64]);
        assert!(static_texture_ref("textures/entity/player/slim/steve.png").is_some());
    }

    #[test]
    fn banner_component_layers_use_the_matching_banner_or_shield_mapper() {
        let component = [BannerPatternLayerSummary {
            registry_id: None,
            asset_id: Some("minecraft:stripe_top".to_string()),
            translation_key: None,
            color_id: 14,
        }];

        let banner = project_banner_patterns(&component, false, None, None);
        let shield = project_banner_patterns(&component, true, None, None);

        assert_eq!(banner.len(), 1);
        assert_eq!(shield.len(), 1);
        assert_eq!(banner[0].color, EntityDyeColor::Red);
        assert_eq!(shield[0].color, EntityDyeColor::Red);
        assert!(matches!(
            banner[0].texture,
            SpecialItemTexture::Static(texture)
                if texture.path == "textures/entity/banner/stripe_top.png"
        ));
        assert!(matches!(
            shield[0].texture,
            SpecialItemTexture::Pack(ref texture)
                if texture.path.as_ref() == "textures/entity/shield/stripe_top.png"
        ));
    }

    #[test]
    fn leaf_matrix_preserves_item_model_column_order() {
        let columns = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [5.0, 6.0, 7.0, 1.0],
        ];
        let matrix = special_item_leaf_matrix(ItemModelLeafTransformation { columns });
        assert_eq!(matrix.to_cols_array_2d(), columns);
    }
}
