//! Block states whose model adds a special renderer (Vanilla 26.1
//! `BuiltInBlockModels.addDefaults`). Entity-held blocks resolved through
//! `BlockModelResolver` draw the block-state quads plus this special model; the
//! special argument is `null`, so the projection sees an empty stack.

use std::collections::BTreeMap;

use bbb_client_assets::{
    CopperGolemStatuePose, ItemSpecialBedPart, ItemSpecialChestType, ItemSpecialEndCubeEffect,
    ItemSpecialHangingAttachment, ItemSpecialModel, ItemSpecialSkullKind,
    ItemSpecialStandingAttachment,
};
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    ItemModelDrawTarget, ItemModelFoil, ItemModelResidencyFrame, SpecialItemInstanceRequest,
};
use bbb_client_world::WorldStore;
use bbb_protocol::packets::ItemStackSummary;
use glam::{Mat4, Quat, Vec3};

use super::special::project_special_item_model;

const DYE_COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

const WOOD_TYPES: [&str; 12] = [
    "oak", "spruce", "birch", "acacia", "cherry", "jungle", "dark_oak", "pale_oak", "mangrove",
    "bamboo", "crimson", "warped",
];

/// Pushes the special model of an entity-held block, posed by `transform`.
/// `BlockModelRenderState.submit` hands it to the special renderer, which
/// submits entity models like any special item, so it joins the world item
/// lane that draws every special-model stage.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_special_block_model(
    residency: &mut ItemModelResidencyFrame,
    item_runtime: &NativeItemRuntime,
    world: &WorldStore,
    block_id: &str,
    properties: &BTreeMap<String, String>,
    christmas: bool,
    transform: Mat4,
    light: [f32; 2],
    overlay: [f32; 2],
    outline_color: Option<u32>,
) {
    let Some((model, block_transform)) = special_block_model(block_id, properties, christmas)
    else {
        return;
    };
    let Some(model) = project_special_item_model(
        &model,
        &ItemStackSummary::empty(),
        item_runtime,
        Some(world),
        None,
    ) else {
        return;
    };
    let submit_order = u32::try_from(residency.instances.len() + residency.special_items.len())
        .expect("special item submit order overflow");
    residency.special_items.push(SpecialItemInstanceRequest {
        model,
        transform: transform * block_transform,
        light,
        overlay,
        foil: ItemModelFoil::None,
        target: ItemModelDrawTarget::World,
        outline_color,
        submit_order,
        scissor: None,
    });
}

/// The special model and its block-local transformation for one block state.
pub(super) fn special_block_model(
    block_id: &str,
    properties: &BTreeMap<String, String>,
    christmas: bool,
) -> Option<(ItemSpecialModel, Mat4)> {
    let name = block_id.strip_prefix("minecraft:")?;
    let property = |key: &str| properties.get(key).map(String::as_str);
    let rotation = || -> Option<f32> { Some(segment_degrees(property("rotation")?.parse().ok()?)) };
    let facing = || property("facing").and_then(Facing::parse);

    if let Some(kind) = skull_kind(name, false) {
        let model = head_model(kind);
        return Some((model, skull_ground_transform(rotation()?)));
    }
    if let Some(kind) = skull_kind(name, true) {
        return Some((head_model(kind), skull_wall_transform(facing()?)));
    }
    if let Some(color) = name.strip_suffix("_wall_banner").filter(|c| is_dye(c)) {
        return Some((
            ItemSpecialModel::Banner {
                color: color.to_string(),
                attachment: ItemSpecialStandingAttachment::Wall,
            },
            banner_transform(facing()?.y_rot()),
        ));
    }
    if let Some(color) = name.strip_suffix("_banner").filter(|c| is_dye(c)) {
        return Some((
            ItemSpecialModel::Banner {
                color: color.to_string(),
                attachment: ItemSpecialStandingAttachment::Ground,
            },
            banner_transform(rotation()?),
        ));
    }
    if let Some(color) = name.strip_suffix("_bed").filter(|c| is_dye(c)) {
        let part = match property("part")? {
            "head" => ItemSpecialBedPart::Head,
            "foot" => ItemSpecialBedPart::Foot,
            _ => return None,
        };
        return Some((
            ItemSpecialModel::Bed {
                texture: format!("minecraft:{color}"),
                part,
            },
            bed_transform(facing()?),
        ));
    }
    if name == "shulker_box" || name.strip_suffix("_shulker_box").is_some_and(is_dye) {
        let texture = match name.strip_suffix("_shulker_box") {
            Some(color) => format!("minecraft:shulker_{color}"),
            None => "minecraft:shulker".to_string(),
        };
        return Some((
            ItemSpecialModel::ShulkerBox {
                texture,
                openness: 0.0,
            },
            shulker_box_transform(facing()?),
        ));
    }
    if let Some(model) = sign_block_model(name, &property) {
        return model;
    }
    if let Some(prefix) = chest_texture_prefix(name, christmas) {
        let (chest_type, suffix) = match property("type").unwrap_or("single") {
            "single" => (ItemSpecialChestType::Single, ""),
            "left" => (ItemSpecialChestType::Left, "_left"),
            "right" => (ItemSpecialChestType::Right, "_right"),
            _ => return None,
        };
        return Some((
            ItemSpecialModel::Chest {
                texture: format!("minecraft:{prefix}{suffix}"),
                openness: 0.0,
                chest_type,
            },
            chest_transform(facing()?),
        ));
    }
    if let Some(weather) = copper_golem_statue_weather(name) {
        let pose = match property("copper_golem_pose")? {
            "standing" => CopperGolemStatuePose::Standing,
            "sitting" => CopperGolemStatuePose::Sitting,
            "running" => CopperGolemStatuePose::Running,
            "star" => CopperGolemStatuePose::Star,
            _ => return None,
        };
        return Some((
            ItemSpecialModel::CopperGolemStatue {
                pose,
                texture: format!(
                    "minecraft:textures/entity/copper_golem/copper_golem{weather}.png"
                ),
            },
            copper_golem_statue_transform(facing()?),
        ));
    }
    Some(match name {
        "bell" => (ItemSpecialModel::Bell, Mat4::IDENTITY),
        // `ConduitRenderer.DEFAULT_TRANSFORMATION`.
        "conduit" => (
            ItemSpecialModel::Conduit,
            Mat4::from_translation(Vec3::splat(0.5)),
        ),
        "decorated_pot" => (
            ItemSpecialModel::DecoratedPot,
            decorated_pot_transform(facing()?),
        ),
        // `BuiltInBlockModels.createEnchantingTable`.
        "enchanting_table" => (
            ItemSpecialModel::Book {
                open_angle: 0.0,
                page1: 0.0,
                page2: 0.0,
            },
            Mat4::from_translation(Vec3::new(0.5, 0.8125, 0.5))
                * Mat4::from_rotation_z(180f32.to_radians())
                * Mat4::from_rotation_x(90f32.to_radians()),
        ),
        "end_gateway" => (
            ItemSpecialModel::EndCube {
                effect: ItemSpecialEndCubeEffect::Gateway,
            },
            Mat4::IDENTITY,
        ),
        // `TheEndPortalRenderer.TRANSFORMATION`.
        "end_portal" => (
            ItemSpecialModel::EndCube {
                effect: ItemSpecialEndCubeEffect::Portal,
            },
            Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
                * Mat4::from_scale(Vec3::new(1.0, 0.375, 1.0)),
        ),
        _ => return None,
    })
}

fn sign_block_model<'a>(
    name: &str,
    property: &impl Fn(&str) -> Option<&'a str>,
) -> Option<Option<(ItemSpecialModel, Mat4)>> {
    let facing = || property("facing").and_then(Facing::parse);
    let rotation = || -> Option<f32> { Some(segment_degrees(property("rotation")?.parse().ok()?)) };
    for wood in WOOD_TYPES {
        let Some(kind) = name.strip_prefix(wood) else {
            continue;
        };
        let wood_type = wood.to_string();
        let model = match kind {
            "_sign" => rotation().map(|angle| {
                (
                    ItemSpecialModel::StandingSign {
                        wood_type,
                        attachment: ItemSpecialStandingAttachment::Ground,
                        texture: None,
                    },
                    standing_sign_body_transform(angle, false),
                )
            }),
            "_wall_sign" => facing().map(|facing| {
                (
                    ItemSpecialModel::StandingSign {
                        wood_type,
                        attachment: ItemSpecialStandingAttachment::Wall,
                        texture: None,
                    },
                    standing_sign_body_transform(facing.y_rot(), true),
                )
            }),
            // `CeilingHangingSignBlock.getAttachmentPoint`.
            "_hanging_sign" => rotation().map(|angle| {
                let attachment = if property("attached") == Some("true") {
                    ItemSpecialHangingAttachment::CeilingMiddle
                } else {
                    ItemSpecialHangingAttachment::Ceiling
                };
                (
                    ItemSpecialModel::HangingSign {
                        wood_type,
                        attachment,
                        texture: None,
                    },
                    hanging_sign_body_transform(angle),
                )
            }),
            "_wall_hanging_sign" => facing().map(|facing| {
                (
                    ItemSpecialModel::HangingSign {
                        wood_type,
                        attachment: ItemSpecialHangingAttachment::Wall,
                        texture: None,
                    },
                    hanging_sign_body_transform(facing.y_rot()),
                )
            }),
            _ => continue,
        };
        return Some(model);
    }
    None
}

fn is_dye(color: &str) -> bool {
    DYE_COLORS.contains(&color)
}

fn skull_kind(name: &str, wall: bool) -> Option<ItemSpecialSkullKind> {
    let ground = if wall {
        name.strip_suffix("_wall_skull")
            .map(|base| format!("{base}_skull"))
            .or_else(|| {
                name.strip_suffix("_wall_head")
                    .map(|base| format!("{base}_head"))
            })?
    } else {
        name.to_string()
    };
    Some(match ground.as_str() {
        "skeleton_skull" => ItemSpecialSkullKind::Skeleton,
        "wither_skeleton_skull" => ItemSpecialSkullKind::WitherSkeleton,
        "zombie_head" => ItemSpecialSkullKind::Zombie,
        "creeper_head" => ItemSpecialSkullKind::Creeper,
        "dragon_head" => ItemSpecialSkullKind::Dragon,
        "piglin_head" => ItemSpecialSkullKind::Piglin,
        "player_head" => ItemSpecialSkullKind::Player,
        _ => return None,
    })
}

fn head_model(kind: ItemSpecialSkullKind) -> ItemSpecialModel {
    match kind {
        ItemSpecialSkullKind::Player => ItemSpecialModel::PlayerHead,
        kind => ItemSpecialModel::Head {
            kind,
            texture: None,
            animation: 0.0,
        },
    }
}

/// `BuiltInBlockModels` chest families; the regular and trapped chests switch
/// to the Christmas set through `IsXmas`.
fn chest_texture_prefix(name: &str, christmas: bool) -> Option<&'static str> {
    let name = name.strip_prefix("waxed_").unwrap_or(name);
    Some(match name {
        "ender_chest" => "ender",
        "chest" | "trapped_chest" if christmas => "christmas",
        "chest" => "normal",
        "trapped_chest" => "trapped",
        "copper_chest" => "copper",
        "exposed_copper_chest" => "copper_exposed",
        "weathered_copper_chest" => "copper_weathered",
        "oxidized_copper_chest" => "copper_oxidized",
        _ => return None,
    })
}

fn copper_golem_statue_weather(name: &str) -> Option<&'static str> {
    let name = name.strip_prefix("waxed_").unwrap_or(name);
    Some(match name {
        "copper_golem_statue" => "",
        "exposed_copper_golem_statue" => "_exposed",
        "weathered_copper_golem_statue" => "_weathered",
        "oxidized_copper_golem_statue" => "_oxidized",
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Facing {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Facing {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "down" => Self::Down,
            "up" => Self::Up,
            "north" => Self::North,
            "south" => Self::South,
            "west" => Self::West,
            "east" => Self::East,
            _ => return None,
        })
    }

    /// `Direction.toYRot`.
    fn y_rot(self) -> f32 {
        match self {
            Self::South => 0.0,
            Self::West => 90.0,
            Self::North => 180.0,
            Self::East => 270.0,
            Self::Down | Self::Up => 0.0,
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    fn step(self) -> Vec3 {
        match self {
            Self::Down => Vec3::NEG_Y,
            Self::Up => Vec3::Y,
            Self::North => Vec3::NEG_Z,
            Self::South => Vec3::Z,
            Self::West => Vec3::NEG_X,
            Self::East => Vec3::X,
        }
    }

    /// `Direction.getRotation`.
    fn rotation(self) -> Quat {
        use std::f32::consts::{FRAC_PI_2, PI};
        let xyz = |x: f32, z: f32| Quat::from_rotation_x(x) * Quat::from_rotation_z(z);
        match self {
            Self::Down => Quat::from_rotation_x(PI),
            Self::Up => Quat::IDENTITY,
            Self::North => xyz(FRAC_PI_2, PI),
            Self::South => Quat::from_rotation_x(FRAC_PI_2),
            Self::West => xyz(FRAC_PI_2, FRAC_PI_2),
            Self::East => xyz(FRAC_PI_2, -FRAC_PI_2),
        }
    }
}

/// `RotationSegment.convertToDegrees` (16 segments, wrapped to `[-180, 180)`).
fn segment_degrees(segment: i32) -> f32 {
    let degrees = segment.rem_euclid(16) as f32 * 22.5;
    if degrees >= 180.0 {
        degrees - 360.0
    } else {
        degrees
    }
}

fn y_rot_around(degrees: f32, origin: Vec3) -> Mat4 {
    Mat4::from_translation(origin)
        * Mat4::from_rotation_y(degrees.to_radians())
        * Mat4::from_translation(-origin)
}

/// `SkullBlockRenderer.createGroundTransformation`.
fn skull_ground_transform(angle: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
        * Mat4::from_rotation_y((-angle).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

/// `SkullBlockRenderer.createWallTransformation`.
fn skull_wall_transform(wall: Facing) -> Mat4 {
    let step = wall.step();
    Mat4::from_translation(Vec3::new(0.5 - step.x * 0.25, 0.25, 0.5 - step.z * 0.25))
        * Mat4::from_rotation_y((-wall.opposite().y_rot()).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
}

/// `BannerRenderer.modelTransformation`.
fn banner_transform(angle: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
        * Mat4::from_rotation_y((-angle).to_radians())
        * Mat4::from_scale(Vec3::new(0.666_666_7, -0.666_666_7, -0.666_666_7))
}

/// `BedRenderer.createModelTransform`.
fn bed_transform(facing: Facing) -> Mat4 {
    let origin = Vec3::splat(0.5);
    Mat4::from_translation(Vec3::new(0.0, 0.5625, 0.0))
        * Mat4::from_rotation_x(90f32.to_radians())
        * Mat4::from_translation(origin)
        * Mat4::from_rotation_z((180.0 + facing.y_rot()).to_radians())
        * Mat4::from_translation(-origin)
}

/// `ShulkerBoxRenderer.createModelTransform`.
fn shulker_box_transform(facing: Facing) -> Mat4 {
    Mat4::from_translation(Vec3::splat(0.5))
        * Mat4::from_scale(Vec3::splat(0.9995))
        * Mat4::from_quat(facing.rotation())
        * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))
}

/// `StandingSignRenderer.bodyTransformation`.
fn standing_sign_body_transform(angle: f32, wall: bool) -> Mat4 {
    let mut base =
        Mat4::from_translation(Vec3::splat(0.5)) * Mat4::from_rotation_y((-angle).to_radians());
    if wall {
        base *= Mat4::from_translation(Vec3::new(0.0, -0.3125, -0.4375));
    }
    base * Mat4::from_scale(Vec3::new(0.666_666_7, -0.666_666_7, -0.666_666_7))
}

/// `HangingSignRenderer.bodyTransformation`.
fn hanging_sign_body_transform(angle: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.5, 0.9375, 0.5))
        * Mat4::from_rotation_y((-angle).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, -0.3125, 0.0))
        * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0))
}

/// `ChestRenderer.createModelTransformation`.
fn chest_transform(facing: Facing) -> Mat4 {
    y_rot_around(-facing.y_rot(), Vec3::new(0.5, 0.0, 0.5))
}

/// `CopperGolemStatueBlockRenderer.createModelTransformation`.
fn copper_golem_statue_transform(facing: Facing) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
        * Mat4::from_rotation_y((-facing.opposite().y_rot()).to_radians())
}

/// `DecoratedPotRenderer.createModelTransformation`.
fn decorated_pot_transform(facing: Facing) -> Mat4 {
    y_rot_around(180.0 - facing.y_rot(), Vec3::splat(0.5))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn chest_minecart_chest_is_a_north_facing_single_chest() {
        // `MinecartChest.getDefaultDisplayBlockState`: chest facing north.
        let (model, transform) = special_block_model(
            "minecraft:chest",
            &state(&[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ]),
            false,
        )
        .unwrap();
        assert_eq!(
            model,
            ItemSpecialModel::Chest {
                texture: "minecraft:normal".to_string(),
                openness: 0.0,
                chest_type: ItemSpecialChestType::Single,
            }
        );
        // North yaw (180) spins the model about the block's vertical center line.
        let corner = transform.transform_point3(Vec3::ZERO);
        assert!((corner - Vec3::new(1.0, 0.0, 1.0)).abs().max_element() < 1e-6);

        let (christmas, _) = special_block_model(
            "minecraft:trapped_chest",
            &state(&[("facing", "north"), ("type", "left")]),
            true,
        )
        .unwrap();
        assert!(matches!(
            christmas,
            ItemSpecialModel::Chest { ref texture, chest_type: ItemSpecialChestType::Left, .. }
                if texture == "minecraft:christmas_left"
        ));
    }

    #[test]
    fn special_block_families_resolve_and_plain_blocks_do_not() {
        for (block, properties) in [
            (
                "minecraft:red_bed",
                state(&[("facing", "east"), ("part", "head")]),
            ),
            ("minecraft:shulker_box", state(&[("facing", "up")])),
            ("minecraft:lime_shulker_box", state(&[("facing", "down")])),
            ("minecraft:creeper_wall_head", state(&[("facing", "west")])),
            ("minecraft:player_head", state(&[("rotation", "3")])),
            ("minecraft:blue_wall_banner", state(&[("facing", "south")])),
            (
                "minecraft:dark_oak_hanging_sign",
                state(&[("rotation", "8"), ("attached", "true")]),
            ),
            (
                "minecraft:waxed_oxidized_copper_golem_statue",
                state(&[("facing", "north"), ("copper_golem_pose", "star")]),
            ),
            ("minecraft:decorated_pot", state(&[("facing", "east")])),
            ("minecraft:bell", state(&[])),
        ] {
            assert!(
                special_block_model(block, &properties, false).is_some(),
                "{block}"
            );
        }
        assert!(special_block_model("minecraft:stone", &state(&[]), false).is_none());
        assert!(special_block_model("minecraft:oak_planks", &state(&[]), false).is_none());
    }
}
