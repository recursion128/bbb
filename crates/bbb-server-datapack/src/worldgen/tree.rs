//! `TreeConfiguration` with its trunk, foliage and root placers, feature size
//! and decorators.

use bbb_protocol::packets::Direction;
use bbb_registry_codec::RegistryJsonValue as Json;

use super::block::{BlockPredicate, BlockStateProvider, HolderSet};
use super::feature::{positive_int, unit_float};
use super::json::{self, fail, At, Obj, Result};
use super::value::{self, IntProvider};
use super::Refs;

const BLOCK: &str = "minecraft:block";

#[derive(Debug, Clone, PartialEq)]
pub struct TreeConfiguration {
    pub trunk_provider: BlockStateProvider,
    pub trunk_placer: TrunkPlacer,
    pub foliage_provider: BlockStateProvider,
    pub foliage_placer: FoliagePlacer,
    pub root_placer: Option<RootPlacer>,
    pub minimum_size: FeatureSize,
    pub decorators: Vec<TreeDecorator>,
    pub ignore_vines: bool,
    pub below_trunk_provider: BlockStateProvider,
}

impl TreeConfiguration {
    pub fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        Ok(Self {
            trunk_provider: c.req("trunk_provider", BlockStateProvider::decode)?,
            trunk_placer: c.req("trunk_placer", TrunkPlacer::decode)?,
            foliage_provider: c.req("foliage_provider", BlockStateProvider::decode)?,
            foliage_placer: c.req("foliage_placer", FoliagePlacer::decode)?,
            root_placer: c.opt("root_placer", RootPlacer::decode)?,
            minimum_size: c.req("minimum_size", FeatureSize::decode)?,
            decorators: c.req("decorators", |v| json::list(v, TreeDecorator::decode))?,
            ignore_vines: c.or_else("ignore_vines", false, json::boolean),
            below_trunk_provider: c
                .or_else("below_trunk_provider", None, |v| {
                    BlockStateProvider::decode(v).map(Some)
                })
                .unwrap_or_else(place_below_overworld_trunks),
        })
    }

    pub(super) fn refs(&self, refs: &mut Refs) {
        self.trunk_provider.refs(refs);
        if let TrunkPlacerKind::UpwardsBranching {
            can_grow_through, ..
        } = &self.trunk_placer.kind
        {
            can_grow_through.refs(BLOCK, refs);
        }
        self.foliage_provider.refs(refs);
        if let Some(root) = &self.root_placer {
            root.root_provider.refs(refs);
            if let Some(above) = &root.above_root_placement {
                above.above_root_provider.refs(refs);
            }
            let mangrove = &root.mangrove_root_placement;
            mangrove.can_grow_through.refs(BLOCK, refs);
            mangrove.muddy_roots_in.refs(BLOCK, refs);
            mangrove.muddy_roots_provider.refs(refs);
        }
        for decorator in &self.decorators {
            decorator.refs(refs);
        }
        self.below_trunk_provider.refs(refs);
    }
}

/// `TreeConfiguration.PLACE_BELOW_OVERWORLD_TRUNKS`: dirt, except where the
/// block below cannot be replaced by it.
fn place_below_overworld_trunks() -> BlockStateProvider {
    let dirt = bbb_facts::block_types::vanilla_block_type_for_resource_id("minecraft:dirt")
        .and_then(|block| bbb_facts::block_state_types::vanilla_block_default_state_id(block.id))
        .expect("dirt has a default state");
    BlockStateProvider::RuleBased {
        fallback: None,
        rules: vec![(
            BlockPredicate::Not(Box::new(BlockPredicate::MatchingBlockTag {
                offset: (0, 0, 0),
                tag: "minecraft:cannot_replace_below_tree_trunk".to_string(),
            })),
            BlockStateProvider::Simple(dirt),
        )],
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrunkPlacerKind {
    Straight,
    Forking,
    Giant,
    MegaJungle,
    DarkOak,
    Fancy,
    Bending {
        min_height_for_leaves: i32,
        bend_length: IntProvider,
    },
    UpwardsBranching {
        extra_branch_steps: IntProvider,
        place_branch_per_log_probability: f32,
        extra_branch_length: IntProvider,
        can_grow_through: HolderSet,
    },
    Cherry {
        branch_count: IntProvider,
        branch_horizontal_length: IntProvider,
        /// `UniformInt` bounds.
        branch_start_offset_from_top: (i32, i32),
        /// `UniformInt.of(min, max - 1)`.
        second_branch_start_offset_from_top: (i32, i32),
        branch_end_offset_from_top: IntProvider,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrunkPlacer {
    pub base_height: i32,
    pub height_rand_a: i32,
    pub height_rand_b: i32,
    pub kind: TrunkPlacerKind,
}

impl TrunkPlacer {
    fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        let ints =
            |name: &str, min, max| c.req(name, |v| value::decode_int_provider_in(v, min, max));
        let kind = match c.type_id("type")?.as_str() {
            "minecraft:straight_trunk_placer" => TrunkPlacerKind::Straight,
            "minecraft:forking_trunk_placer" => TrunkPlacerKind::Forking,
            "minecraft:giant_trunk_placer" => TrunkPlacerKind::Giant,
            "minecraft:mega_jungle_trunk_placer" => TrunkPlacerKind::MegaJungle,
            "minecraft:dark_oak_trunk_placer" => TrunkPlacerKind::DarkOak,
            "minecraft:fancy_trunk_placer" => TrunkPlacerKind::Fancy,
            "minecraft:bending_trunk_placer" => TrunkPlacerKind::Bending {
                min_height_for_leaves: c.or("min_height_for_leaves", 1, positive_int)?,
                bend_length: ints("bend_length", 1, 64)?,
            },
            "minecraft:upwards_branching_trunk_placer" => TrunkPlacerKind::UpwardsBranching {
                extra_branch_steps: ints("extra_branch_steps", 1, i32::MAX)?,
                place_branch_per_log_probability: c
                    .req("place_branch_per_log_probability", unit_float)?,
                extra_branch_length: ints("extra_branch_length", 0, i32::MAX)?,
                can_grow_through: c.req("can_grow_through", HolderSet::blocks)?,
            },
            "minecraft:cherry_trunk_placer" => {
                let (min, max) = c.req("branch_start_offset_from_top", |v| {
                    let start = Obj::of(v)?;
                    let min = start.req("min_inclusive", json::int)?;
                    let max = start.req("max_inclusive", json::int)?;
                    if max < min {
                        return fail("Max must be at least min");
                    }
                    if max - min < 1 {
                        return fail("Need at least 2 blocks variation for the branch starts to fit both branches");
                    }
                    if min < -16 || max > 0 {
                        return fail("Value provider out of range [-16, 0]");
                    }
                    Ok((min, max))
                })?;
                TrunkPlacerKind::Cherry {
                    branch_count: ints("branch_count", 1, 3)?,
                    branch_horizontal_length: ints("branch_horizontal_length", 2, 16)?,
                    branch_start_offset_from_top: (min, max),
                    second_branch_start_offset_from_top: (min, max - 1),
                    branch_end_offset_from_top: ints("branch_end_offset_from_top", -16, 16)?,
                }
            }
            other => return fail(format!("unknown trunk placer type `{other}`")).at("type"),
        };
        Ok(Self {
            base_height: c.req("base_height", |v| json::int_in(v, 0, 32))?,
            height_rand_a: c.req("height_rand_a", |v| json::int_in(v, 0, 24))?,
            height_rand_b: c.req("height_rand_b", |v| json::int_in(v, 0, 24))?,
            kind,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FoliagePlacerKind {
    Blob {
        height: i32,
    },
    Bush {
        height: i32,
    },
    Fancy {
        height: i32,
    },
    Spruce {
        trunk_height: IntProvider,
    },
    Pine {
        height: IntProvider,
    },
    Acacia,
    DarkOak,
    MegaJungle {
        height: i32,
    },
    MegaPine {
        crown_height: IntProvider,
    },
    RandomSpread {
        foliage_height: IntProvider,
        leaf_placement_attempts: i32,
    },
    Cherry {
        height: IntProvider,
        wide_bottom_layer_hole_chance: f32,
        corner_hole_chance: f32,
        hanging_leaves_chance: f32,
        hanging_leaves_extension_chance: f32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FoliagePlacer {
    pub radius: IntProvider,
    pub offset: IntProvider,
    pub kind: FoliagePlacerKind,
}

impl FoliagePlacer {
    fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        let ints =
            |name: &str, min, max| c.req(name, |v| value::decode_int_provider_in(v, min, max));
        let height = || c.req("height", |v| json::int_in(v, 0, 16));
        let kind = match c.type_id("type")?.as_str() {
            "minecraft:blob_foliage_placer" => FoliagePlacerKind::Blob { height: height()? },
            "minecraft:bush_foliage_placer" => FoliagePlacerKind::Bush { height: height()? },
            "minecraft:fancy_foliage_placer" => FoliagePlacerKind::Fancy { height: height()? },
            "minecraft:spruce_foliage_placer" => FoliagePlacerKind::Spruce {
                trunk_height: ints("trunk_height", 0, 24)?,
            },
            "minecraft:pine_foliage_placer" => FoliagePlacerKind::Pine {
                height: ints("height", 0, 24)?,
            },
            "minecraft:acacia_foliage_placer" => FoliagePlacerKind::Acacia,
            "minecraft:dark_oak_foliage_placer" => FoliagePlacerKind::DarkOak,
            "minecraft:jungle_foliage_placer" => {
                FoliagePlacerKind::MegaJungle { height: height()? }
            }
            "minecraft:mega_pine_foliage_placer" => FoliagePlacerKind::MegaPine {
                crown_height: ints("crown_height", 0, 24)?,
            },
            "minecraft:random_spread_foliage_placer" => FoliagePlacerKind::RandomSpread {
                foliage_height: ints("foliage_height", 1, 512)?,
                leaf_placement_attempts: c
                    .req("leaf_placement_attempts", |v| json::int_in(v, 0, 256))?,
            },
            "minecraft:cherry_foliage_placer" => FoliagePlacerKind::Cherry {
                height: ints("height", 4, 16)?,
                wide_bottom_layer_hole_chance: c
                    .req("wide_bottom_layer_hole_chance", unit_float)?,
                corner_hole_chance: c.req("corner_hole_chance", unit_float)?,
                hanging_leaves_chance: c.req("hanging_leaves_chance", unit_float)?,
                hanging_leaves_extension_chance: c
                    .req("hanging_leaves_extension_chance", unit_float)?,
            },
            other => return fail(format!("unknown foliage placer type `{other}`")).at("type"),
        };
        Ok(Self {
            radius: ints("radius", 0, 16)?,
            offset: ints("offset", 0, 16)?,
            kind,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AboveRootPlacement {
    pub above_root_provider: BlockStateProvider,
    pub above_root_placement_chance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MangroveRootPlacement {
    pub can_grow_through: HolderSet,
    pub muddy_roots_in: HolderSet,
    pub muddy_roots_provider: BlockStateProvider,
    pub max_root_width: i32,
    pub max_root_length: i32,
    pub random_skew_chance: f32,
}

/// The one 26.1 root placer, `MangroveRootPlacer`.
#[derive(Debug, Clone, PartialEq)]
pub struct RootPlacer {
    pub trunk_offset_y: IntProvider,
    pub root_provider: BlockStateProvider,
    pub above_root_placement: Option<AboveRootPlacement>,
    pub mangrove_root_placement: MangroveRootPlacement,
}

impl RootPlacer {
    fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        match c.type_id("type")?.as_str() {
            "minecraft:mangrove_root_placer" => {}
            other => return fail(format!("unknown root placer type `{other}`")).at("type"),
        }
        Ok(Self {
            trunk_offset_y: c.req("trunk_offset_y", value::decode_int_provider)?,
            root_provider: c.req("root_provider", BlockStateProvider::decode)?,
            above_root_placement: c.opt("above_root_placement", |v| {
                let above = Obj::of(v)?;
                Ok(AboveRootPlacement {
                    above_root_provider: above
                        .req("above_root_provider", BlockStateProvider::decode)?,
                    above_root_placement_chance: above
                        .req("above_root_placement_chance", unit_float)?,
                })
            })?,
            mangrove_root_placement: c.req("mangrove_root_placement", |v| {
                let m = Obj::of(v)?;
                Ok(MangroveRootPlacement {
                    can_grow_through: m.req("can_grow_through", HolderSet::blocks)?,
                    muddy_roots_in: m.req("muddy_roots_in", HolderSet::blocks)?,
                    muddy_roots_provider: m
                        .req("muddy_roots_provider", BlockStateProvider::decode)?,
                    max_root_width: m.req("max_root_width", |v| json::int_in(v, 1, 12))?,
                    max_root_length: m.req("max_root_length", |v| json::int_in(v, 1, 64))?,
                    random_skew_chance: m.req("random_skew_chance", unit_float)?,
                })
            })?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureSizeKind {
    TwoLayers {
        limit: i32,
        lower_size: i32,
        upper_size: i32,
    },
    ThreeLayers {
        limit: i32,
        upper_limit: i32,
        lower_size: i32,
        middle_size: i32,
        upper_size: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSize {
    pub kind: FeatureSizeKind,
    pub min_clipped_height: Option<i32>,
}

impl FeatureSize {
    fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        let int = |name, default, max| c.or_else(name, default, |v| json::int_in(v, 0, max));
        let kind = match c.type_id("type")?.as_str() {
            "minecraft:two_layers_feature_size" => FeatureSizeKind::TwoLayers {
                limit: int("limit", 1, 81),
                lower_size: int("lower_size", 0, 16),
                upper_size: int("upper_size", 1, 16),
            },
            "minecraft:three_layers_feature_size" => FeatureSizeKind::ThreeLayers {
                limit: int("limit", 1, 80),
                upper_limit: int("upper_limit", 1, 80),
                lower_size: int("lower_size", 0, 16),
                middle_size: int("middle_size", 1, 16),
                upper_size: int("upper_size", 1, 16),
            },
            other => return fail(format!("unknown feature size type `{other}`")).at("type"),
        };
        Ok(Self {
            kind,
            min_clipped_height: c.opt("min_clipped_height", |v| json::int_in(v, 0, 80))?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeDecorator {
    TrunkVine,
    LeaveVine {
        probability: f32,
    },
    Cocoa {
        probability: f32,
    },
    Beehive {
        probability: f32,
    },
    AlterGround {
        provider: BlockStateProvider,
    },
    AttachedToLeaves {
        probability: f32,
        exclusion_radius_xz: i32,
        exclusion_radius_y: i32,
        block_provider: BlockStateProvider,
        required_empty_blocks: i32,
        directions: Vec<Direction>,
    },
    AttachedToLogs {
        probability: f32,
        block_provider: BlockStateProvider,
        directions: Vec<Direction>,
    },
    CreakingHeart {
        probability: f32,
    },
    PaleMoss {
        leaves_probability: f32,
        trunk_probability: f32,
        ground_probability: f32,
    },
    PlaceOnGround {
        tries: i32,
        radius: i32,
        height: i32,
        block_state_provider: BlockStateProvider,
    },
}

impl TreeDecorator {
    pub fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        let probability = |name: &str| c.req(name, unit_float);
        let directions = || c.req("directions", |v| json::nonempty_list(v, value::direction));
        let non_negative =
            |name, default| c.or_else(name, default, |v| json::int_in(v, 0, i32::MAX));
        Ok(match c.type_id("type")?.as_str() {
            "minecraft:trunk_vine" => Self::TrunkVine,
            "minecraft:leave_vine" => Self::LeaveVine {
                probability: probability("probability")?,
            },
            "minecraft:cocoa" => Self::Cocoa {
                probability: probability("probability")?,
            },
            "minecraft:beehive" => Self::Beehive {
                probability: probability("probability")?,
            },
            "minecraft:alter_ground" => Self::AlterGround {
                provider: c.req("provider", BlockStateProvider::decode)?,
            },
            "minecraft:attached_to_leaves" => Self::AttachedToLeaves {
                probability: probability("probability")?,
                exclusion_radius_xz: c.req("exclusion_radius_xz", |v| json::int_in(v, 0, 16))?,
                exclusion_radius_y: c.req("exclusion_radius_y", |v| json::int_in(v, 0, 16))?,
                block_provider: c.req("block_provider", BlockStateProvider::decode)?,
                required_empty_blocks: c
                    .req("required_empty_blocks", |v| json::int_in(v, 1, 16))?,
                directions: directions()?,
            },
            "minecraft:attached_to_logs" => Self::AttachedToLogs {
                probability: probability("probability")?,
                block_provider: c.req("block_provider", BlockStateProvider::decode)?,
                directions: directions()?,
            },
            "minecraft:creaking_heart" => Self::CreakingHeart {
                probability: probability("probability")?,
            },
            "minecraft:pale_moss" => Self::PaleMoss {
                leaves_probability: probability("leaves_probability")?,
                trunk_probability: probability("trunk_probability")?,
                ground_probability: probability("ground_probability")?,
            },
            "minecraft:place_on_ground" => Self::PlaceOnGround {
                tries: c.or_else("tries", 128, positive_int),
                radius: non_negative("radius", 2),
                height: non_negative("height", 1),
                block_state_provider: c.req("block_state_provider", BlockStateProvider::decode)?,
            },
            other => return fail(format!("unknown tree decorator type `{other}`")).at("type"),
        })
    }

    pub(super) fn refs(&self, refs: &mut Refs) {
        match self {
            Self::AlterGround { provider }
            | Self::AttachedToLeaves {
                block_provider: provider,
                ..
            }
            | Self::AttachedToLogs {
                block_provider: provider,
                ..
            }
            | Self::PlaceOnGround {
                block_state_provider: provider,
                ..
            } => provider.refs(refs),
            _ => {}
        }
    }
}
