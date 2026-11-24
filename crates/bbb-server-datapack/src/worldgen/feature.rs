//! `ConfiguredFeature.DIRECT_CODEC` and every 26.1 feature configuration.

use bbb_facts::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::RegistryJsonValue as Json;

use super::block::{block_id, BlockPredicate, BlockStateProvider, HolderSet, RuleTest};
use super::json::{self, fail, At, Obj, Result};
use super::placement::{PlacedFeatureHolder, PLACED_FEATURE_REGISTRY};
use super::processor::ProcessorListHolder;
use super::tree::{TreeConfiguration, TreeDecorator};
use super::value::{self, FloatProvider, IntProvider};
use super::Refs;

const BLOCK: &str = "minecraft:block";

/// One configured feature: the `Feature` registry id selects the variant and
/// its configuration codec.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfiguredFeature {
    NoOp,
    Tree(Box<TreeConfiguration>),
    FallenTree(FallenTreeConfig),
    BlockPile(BlockStateProvider),
    Spring(SpringConfig),
    ChorusPlant,
    ReplaceSingleBlock(Vec<OreTarget>),
    VoidStartPlatform,
    DesertWell,
    Fossil(FossilConfig),
    HugeRedMushroom(HugeMushroomConfig),
    HugeBrownMushroom(HugeMushroomConfig),
    Spike(SpikeConfig),
    GlowstoneBlob,
    FreezeTopLayer,
    Vines,
    BlockColumn(BlockColumnConfig),
    VegetationPatch(VegetationPatchConfig),
    WaterloggedVegetationPatch(VegetationPatchConfig),
    RootSystem(RootSystemConfig),
    MultifaceGrowth(MultifaceGrowthConfig),
    UnderwaterMagma(UnderwaterMagmaConfig),
    MonsterRoom,
    BlueIce,
    /// `BlockStateConfiguration.state`.
    Iceberg(i32),
    BlockBlob(BlockBlobConfig),
    Disk(DiskConfig),
    Lake {
        fluid: BlockStateProvider,
        barrier: BlockStateProvider,
    },
    Ore(OreConfig),
    EndPlatform,
    EndSpike(EndSpikeConfig),
    EndIsland,
    EndGateway(EndGatewayConfig),
    /// `ProbabilityFeatureConfiguration.probability`.
    Seagrass(f32),
    Kelp,
    CoralTree,
    CoralMushroom,
    CoralClaw,
    /// `CountConfiguration.count`.
    SeaPickle(IntProvider),
    SimpleBlock {
        to_place: BlockStateProvider,
        schedule_tick: bool,
    },
    /// `ProbabilityFeatureConfiguration.probability`.
    Bamboo(f32),
    HugeFungus(HugeFungusConfig),
    NetherForestVegetation(NetherForestVegetationConfig),
    WeepingVines,
    TwistingVines {
        spread_width: i32,
        spread_height: i32,
        max_height: i32,
    },
    BasaltColumns {
        reach: IntProvider,
        height: IntProvider,
    },
    DeltaFeature(DeltaConfig),
    NetherrackReplaceBlobs(ReplaceSphereConfig),
    FillLayer {
        height: i32,
        state: i32,
    },
    BonusChest,
    BasaltPillar,
    ScatteredOre(OreConfig),
    RandomSelector(RandomSelectorConfig),
    SimpleRandomSelector(PlacedFeatureSet),
    RandomBooleanSelector {
        feature_true: PlacedFeatureHolder,
        feature_false: PlacedFeatureHolder,
    },
    Geode(Box<GeodeConfig>),
    DripstoneCluster(DripstoneClusterConfig),
    LargeDripstone(LargeDripstoneConfig),
    PointedDripstone(PointedDripstoneConfig),
    SculkPatch(SculkPatchConfig),
}

impl ConfiguredFeature {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        let type_id = obj.type_id("type")?;
        obj.req("config", |config| Self::decode_config(&type_id, config))
    }

    fn decode_config(type_id: &str, config: &Json) -> Result<Self> {
        let c = || Obj::of(config);
        Ok(match type_id {
            "minecraft:no_op" => Self::NoOp,
            "minecraft:tree" => Self::Tree(Box::new(TreeConfiguration::decode(config)?)),
            "minecraft:fallen_tree" => Self::FallenTree(FallenTreeConfig::decode(&c()?)?),
            "minecraft:block_pile" => {
                Self::BlockPile(c()?.req("state_provider", BlockStateProvider::decode)?)
            }
            "minecraft:spring_feature" => Self::Spring(SpringConfig::decode(&c()?)?),
            "minecraft:chorus_plant" => Self::ChorusPlant,
            "minecraft:replace_single_block" => {
                Self::ReplaceSingleBlock(c()?.req("targets", ore_targets)?)
            }
            "minecraft:void_start_platform" => Self::VoidStartPlatform,
            "minecraft:desert_well" => Self::DesertWell,
            "minecraft:fossil" => Self::Fossil(FossilConfig::decode(&c()?)?),
            "minecraft:huge_red_mushroom" => {
                Self::HugeRedMushroom(HugeMushroomConfig::decode(&c()?)?)
            }
            "minecraft:huge_brown_mushroom" => {
                Self::HugeBrownMushroom(HugeMushroomConfig::decode(&c()?)?)
            }
            "minecraft:spike" => Self::Spike(SpikeConfig::decode(&c()?)?),
            "minecraft:glowstone_blob" => Self::GlowstoneBlob,
            "minecraft:freeze_top_layer" => Self::FreezeTopLayer,
            "minecraft:vines" => Self::Vines,
            "minecraft:block_column" => Self::BlockColumn(BlockColumnConfig::decode(&c()?)?),
            "minecraft:vegetation_patch" => {
                Self::VegetationPatch(VegetationPatchConfig::decode(&c()?)?)
            }
            "minecraft:waterlogged_vegetation_patch" => {
                Self::WaterloggedVegetationPatch(VegetationPatchConfig::decode(&c()?)?)
            }
            "minecraft:root_system" => Self::RootSystem(RootSystemConfig::decode(&c()?)?),
            "minecraft:multiface_growth" => {
                Self::MultifaceGrowth(MultifaceGrowthConfig::decode(&c()?)?)
            }
            "minecraft:underwater_magma" => {
                let c = c()?;
                Self::UnderwaterMagma(UnderwaterMagmaConfig {
                    floor_search_range: c.req("floor_search_range", |v| json::int_in(v, 0, 512))?,
                    placement_radius_around_floor: c
                        .req("placement_radius_around_floor", |v| json::int_in(v, 0, 64))?,
                    placement_probability_per_valid_position: c
                        .req("placement_probability_per_valid_position", unit_float)?,
                })
            }
            "minecraft:monster_room" => Self::MonsterRoom,
            "minecraft:blue_ice" => Self::BlueIce,
            "minecraft:iceberg" => Self::Iceberg(c()?.req("state", value::block_state)?),
            "minecraft:block_blob" => {
                let c = c()?;
                Self::BlockBlob(BlockBlobConfig {
                    state: c.req("state", value::block_state)?,
                    can_place_on: c.req("can_place_on", BlockPredicate::decode)?,
                })
            }
            "minecraft:disk" => {
                let c = c()?;
                Self::Disk(DiskConfig {
                    state_provider: c.req("state_provider", BlockStateProvider::decode)?,
                    target: c.req("target", BlockPredicate::decode)?,
                    radius: c.req("radius", |v| value::decode_int_provider_in(v, 0, 8))?,
                    half_height: c.req("half_height", |v| json::int_in(v, 0, 4))?,
                })
            }
            "minecraft:lake" => {
                let c = c()?;
                Self::Lake {
                    fluid: c.req("fluid", BlockStateProvider::decode)?,
                    barrier: c.req("barrier", BlockStateProvider::decode)?,
                }
            }
            "minecraft:ore" => Self::Ore(OreConfig::decode(&c()?)?),
            "minecraft:end_platform" => Self::EndPlatform,
            "minecraft:end_spike" => Self::EndSpike(EndSpikeConfig::decode(&c()?)?),
            "minecraft:end_island" => Self::EndIsland,
            "minecraft:end_gateway" => {
                let c = c()?;
                Self::EndGateway(EndGatewayConfig {
                    exit: c.opt("exit", value::block_pos)?,
                    exact: c.req("exact", json::boolean)?,
                })
            }
            "minecraft:seagrass" => Self::Seagrass(c()?.req("probability", unit_float)?),
            "minecraft:kelp" => Self::Kelp,
            "minecraft:coral_tree" => Self::CoralTree,
            "minecraft:coral_mushroom" => Self::CoralMushroom,
            "minecraft:coral_claw" => Self::CoralClaw,
            "minecraft:sea_pickle" => {
                Self::SeaPickle(c()?.req("count", |v| value::decode_int_provider_in(v, 0, 256))?)
            }
            "minecraft:simple_block" => {
                let c = c()?;
                Self::SimpleBlock {
                    to_place: c.req("to_place", BlockStateProvider::decode)?,
                    schedule_tick: c.or("schedule_tick", false, json::boolean)?,
                }
            }
            "minecraft:bamboo" => Self::Bamboo(c()?.req("probability", unit_float)?),
            "minecraft:huge_fungus" => {
                let c = c()?;
                Self::HugeFungus(HugeFungusConfig {
                    valid_base_state: c.req("valid_base_block", value::block_state)?,
                    stem_state: c.req("stem_state", value::block_state)?,
                    hat_state: c.req("hat_state", value::block_state)?,
                    decor_state: c.req("decor_state", value::block_state)?,
                    replaceable_blocks: c.req("replaceable_blocks", BlockPredicate::decode)?,
                    planted: c.or_else("planted", false, json::boolean),
                })
            }
            "minecraft:nether_forest_vegetation" => {
                let c = c()?;
                Self::NetherForestVegetation(NetherForestVegetationConfig {
                    state_provider: c.req("state_provider", BlockStateProvider::decode)?,
                    spread_width: c.req("spread_width", positive_int)?,
                    spread_height: c.req("spread_height", positive_int)?,
                })
            }
            "minecraft:weeping_vines" => Self::WeepingVines,
            "minecraft:twisting_vines" => {
                let c = c()?;
                Self::TwistingVines {
                    spread_width: c.req("spread_width", positive_int)?,
                    spread_height: c.req("spread_height", positive_int)?,
                    max_height: c.req("max_height", positive_int)?,
                }
            }
            "minecraft:basalt_columns" => {
                let c = c()?;
                Self::BasaltColumns {
                    reach: c.req("reach", |v| value::decode_int_provider_in(v, 0, 3))?,
                    height: c.req("height", |v| value::decode_int_provider_in(v, 1, 10))?,
                }
            }
            "minecraft:delta_feature" => {
                let c = c()?;
                Self::DeltaFeature(DeltaConfig {
                    contents: c.req("contents", value::block_state)?,
                    rim: c.req("rim", value::block_state)?,
                    size: c.req("size", |v| value::decode_int_provider_in(v, 0, 16))?,
                    rim_size: c.req("rim_size", |v| value::decode_int_provider_in(v, 0, 16))?,
                })
            }
            "minecraft:netherrack_replace_blobs" => {
                let c = c()?;
                Self::NetherrackReplaceBlobs(ReplaceSphereConfig {
                    target: c.req("target", value::block_state)?,
                    state: c.req("state", value::block_state)?,
                    radius: c.req("radius", |v| value::decode_int_provider_in(v, 0, 12))?,
                })
            }
            "minecraft:fill_layer" => {
                let c = c()?;
                Self::FillLayer {
                    // `DimensionType.Y_SIZE`.
                    height: c.req("height", |v| json::int_in(v, 0, 4064))?,
                    state: c.req("state", value::block_state)?,
                }
            }
            "minecraft:bonus_chest" => Self::BonusChest,
            "minecraft:basalt_pillar" => Self::BasaltPillar,
            "minecraft:scattered_ore" => Self::ScatteredOre(OreConfig::decode(&c()?)?),
            "minecraft:random_selector" => {
                let c = c()?;
                Self::RandomSelector(RandomSelectorConfig {
                    features: c.req("features", |v| {
                        json::list(v, |entry| {
                            let entry = Obj::of(entry)?;
                            Ok((
                                entry.req("feature", PlacedFeatureHolder::decode_placed)?,
                                entry.req("chance", unit_float)?,
                            ))
                        })
                    })?,
                    default: c.req("default", PlacedFeatureHolder::decode_placed)?,
                })
            }
            "minecraft:simple_random_selector" => {
                Self::SimpleRandomSelector(c()?.req("features", PlacedFeatureSet::decode)?)
            }
            "minecraft:random_boolean_selector" => {
                let c = c()?;
                Self::RandomBooleanSelector {
                    feature_true: c.req("feature_true", PlacedFeatureHolder::decode_placed)?,
                    feature_false: c.req("feature_false", PlacedFeatureHolder::decode_placed)?,
                }
            }
            "minecraft:geode" => Self::Geode(Box::new(GeodeConfig::decode(&c()?)?)),
            "minecraft:dripstone_cluster" => {
                Self::DripstoneCluster(DripstoneClusterConfig::decode(&c()?)?)
            }
            "minecraft:large_dripstone" => {
                Self::LargeDripstone(LargeDripstoneConfig::decode(&c()?)?)
            }
            "minecraft:pointed_dripstone" => {
                let c = c()?;
                let chance = |name, default| c.or_else(name, default, unit_float);
                Self::PointedDripstone(PointedDripstoneConfig {
                    chance_of_taller_dripstone: chance("chance_of_taller_dripstone", 0.2),
                    chance_of_directional_spread: chance("chance_of_directional_spread", 0.7),
                    chance_of_spread_radius2: chance("chance_of_spread_radius2", 0.5),
                    chance_of_spread_radius3: chance("chance_of_spread_radius3", 0.5),
                })
            }
            "minecraft:sculk_patch" => {
                let c = c()?;
                Self::SculkPatch(SculkPatchConfig {
                    charge_count: c.req("charge_count", |v| json::int_in(v, 1, 32))?,
                    amount_per_charge: c.req("amount_per_charge", |v| json::int_in(v, 1, 500))?,
                    spread_attempts: c.req("spread_attempts", |v| json::int_in(v, 1, 64))?,
                    growth_rounds: c.req("growth_rounds", |v| json::int_in(v, 0, 8))?,
                    spread_rounds: c.req("spread_rounds", |v| json::int_in(v, 0, 8))?,
                    extra_rare_growths: c.req("extra_rare_growths", value::decode_int_provider)?,
                    catalyst_chance: c.req("catalyst_chance", unit_float)?,
                })
            }
            other => return fail(format!("unknown feature type `{other}`")).at("type"),
        })
    }

    pub fn refs(&self, refs: &mut Refs) {
        match self {
            Self::Tree(config) => config.refs(refs),
            Self::FallenTree(config) => {
                config.trunk_provider.refs(refs);
                for decorator in config.stump_decorators.iter().chain(&config.log_decorators) {
                    decorator.refs(refs);
                }
            }
            Self::BlockPile(provider) => provider.refs(refs),
            Self::Spring(config) => config.valid_blocks.refs(BLOCK, refs),
            Self::Fossil(config) => {
                config.fossil_processors.refs(refs);
                config.overlay_processors.refs(refs);
            }
            Self::HugeRedMushroom(config) | Self::HugeBrownMushroom(config) => {
                config.cap_provider.refs(refs);
                config.stem_provider.refs(refs);
                config.can_place_on.refs(refs);
            }
            Self::Spike(config) => {
                config.can_place_on.refs(refs);
                config.can_replace.refs(refs);
            }
            Self::BlockColumn(config) => {
                for layer in &config.layers {
                    layer.provider.refs(refs);
                }
                config.allowed_placement.refs(refs);
            }
            Self::VegetationPatch(config) | Self::WaterloggedVegetationPatch(config) => {
                config.ground_state.refs(refs);
                config.vegetation_feature.refs(refs);
            }
            Self::RootSystem(config) => {
                config.feature.refs(refs);
                config.root_state_provider.refs(refs);
                config.hanging_root_state_provider.refs(refs);
                config.allowed_tree_position.refs(refs);
            }
            Self::MultifaceGrowth(config) => config.can_be_placed_on.refs(BLOCK, refs),
            Self::BlockBlob(config) => config.can_place_on.refs(refs),
            Self::Disk(config) => {
                config.state_provider.refs(refs);
                config.target.refs(refs);
            }
            Self::Lake { fluid, barrier } => {
                fluid.refs(refs);
                barrier.refs(refs);
            }
            Self::SimpleBlock { to_place, .. } => to_place.refs(refs),
            Self::HugeFungus(config) => config.replaceable_blocks.refs(refs),
            Self::NetherForestVegetation(config) => config.state_provider.refs(refs),
            Self::RandomSelector(config) => {
                for (feature, _) in &config.features {
                    feature.refs(refs);
                }
                config.default.refs(refs);
            }
            Self::SimpleRandomSelector(set) => set.refs(refs),
            Self::RandomBooleanSelector {
                feature_true,
                feature_false,
            } => {
                feature_true.refs(refs);
                feature_false.refs(refs);
            }
            Self::Geode(config) => {
                for provider in [
                    &config.filling_provider,
                    &config.inner_layer_provider,
                    &config.alternate_inner_layer_provider,
                    &config.middle_layer_provider,
                    &config.outer_layer_provider,
                ] {
                    provider.refs(refs);
                }
            }
            _ => {}
        }
    }
}

/// `ExtraCodecs.POSITIVE_INT`.
pub(super) fn positive_int(value: &Json) -> Result<i32> {
    json::int_in(value, 1, i32::MAX)
}

/// `Codec.floatRange(0, 1)`.
pub(super) fn unit_float(value: &Json) -> Result<f32> {
    json::float_in(value, 0.0, 1.0)
}

/// `TagKey.hashedCodec`: a `#`-prefixed tag id, kept without the `#`.
pub(super) fn hashed_tag(value: &Json) -> Result<String> {
    match json::string(value)?.strip_prefix('#') {
        Some(tag) => json::id(&Json::String(tag.into())),
        None => fail("Not a tag id"),
    }
}

/// `OreConfiguration.TargetBlockState`.
#[derive(Debug, Clone, PartialEq)]
pub struct OreTarget {
    pub target: RuleTest,
    pub state: i32,
}

fn ore_targets(value: &Json) -> Result<Vec<OreTarget>> {
    json::list(value, |target| {
        let target = Obj::of(target)?;
        Ok(OreTarget {
            target: target.req("target", RuleTest::decode)?,
            state: target.req("state", value::block_state)?,
        })
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct OreConfig {
    pub targets: Vec<OreTarget>,
    pub size: i32,
    pub discard_chance_on_air_exposure: f32,
}

impl OreConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        Ok(Self {
            targets: c.req("targets", ore_targets)?,
            size: c.req("size", |v| json::int_in(v, 0, 64))?,
            discard_chance_on_air_exposure: c.req("discard_chance_on_air_exposure", unit_float)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpringConfig {
    /// `FluidState.createLegacyBlock` of a source water or lava state; other
    /// fluid states place nothing.
    pub fluid_block: Option<i32>,
    pub requires_block_below: bool,
    pub rock_count: i32,
    pub hole_count: i32,
    pub valid_blocks: HolderSet,
}

impl SpringConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        Ok(Self {
            fluid_block: c.req("state", fluid_state_legacy_block)?,
            requires_block_below: c.or_else("requires_block_below", true, json::boolean),
            rock_count: c.or_else("rock_count", 4, json::int),
            hole_count: c.or_else("hole_count", 1, json::int),
            valid_blocks: c.req("valid_blocks", HolderSet::blocks)?,
        })
    }
}

/// `FluidState.CODEC` for a spring: a source water or lava state is the
/// level-0 liquid block (`FlowingFluid.getLegacyLevel`).
fn fluid_state_legacy_block(value: &Json) -> Result<Option<i32>> {
    use bbb_facts::registry_data_types::VanillaFluidIdentity as Fluid;
    let name = Obj::of(value)?.req("Name", json::id)?;
    let block = match Fluid::for_resource_id(&name) {
        Some(fluid @ (Fluid::Water | Fluid::Lava)) => fluid.resource_id(),
        Some(_) => return Ok(None),
        None => return fail(format!("unknown fluid `{name}`")).at("Name"),
    };
    let properties = [("level".to_string(), "0".to_string())]
        .into_iter()
        .collect();
    Ok(vanilla_block_state_id_by_name_and_properties(
        block,
        &properties,
    ))
}

#[derive(Debug, Clone, PartialEq)]
pub struct FossilConfig {
    pub fossil_structures: Vec<String>,
    pub overlay_structures: Vec<String>,
    pub fossil_processors: ProcessorListHolder,
    pub overlay_processors: ProcessorListHolder,
    pub max_empty_corners_allowed: i32,
}

impl FossilConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        let ids = |v: &Json| json::list(v, json::id);
        let config = Self {
            fossil_structures: c.req("fossil_structures", ids)?,
            overlay_structures: c.req("overlay_structures", ids)?,
            fossil_processors: c.req("fossil_processors", ProcessorListHolder::decode_list)?,
            overlay_processors: c.req("overlay_processors", ProcessorListHolder::decode_list)?,
            max_empty_corners_allowed: c
                .req("max_empty_corners_allowed", |v| json::int_in(v, 0, 7))?,
        };
        if config.fossil_structures.is_empty() {
            return fail("Fossil structure lists cannot be empty");
        }
        if config.fossil_structures.len() != config.overlay_structures.len() {
            return fail("Fossil structure lists must be equal lengths");
        }
        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HugeMushroomConfig {
    pub cap_provider: BlockStateProvider,
    pub stem_provider: BlockStateProvider,
    pub foliage_radius: i32,
    pub can_place_on: BlockPredicate,
}

impl HugeMushroomConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        Ok(Self {
            cap_provider: c.req("cap_provider", BlockStateProvider::decode)?,
            stem_provider: c.req("stem_provider", BlockStateProvider::decode)?,
            foliage_radius: c.or_else("foliage_radius", 2, json::int),
            can_place_on: c.req("can_place_on", BlockPredicate::decode)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpikeConfig {
    pub state: i32,
    pub can_place_on: BlockPredicate,
    pub can_replace: BlockPredicate,
}

impl SpikeConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        Ok(Self {
            state: c.req("state", value::block_state)?,
            can_place_on: c.req("can_place_on", BlockPredicate::decode)?,
            can_replace: c.req("can_replace", BlockPredicate::decode)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockColumnLayer {
    pub height: IntProvider,
    pub provider: BlockStateProvider,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockColumnConfig {
    pub layers: Vec<BlockColumnLayer>,
    pub direction: Direction,
    pub allowed_placement: BlockPredicate,
    pub prioritize_tip: bool,
}

impl BlockColumnConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        Ok(Self {
            layers: c.req("layers", |v| {
                json::list(v, |layer| {
                    let layer = Obj::of(layer)?;
                    Ok(BlockColumnLayer {
                        height: layer
                            .req("height", |v| value::decode_int_provider_in(v, 0, i32::MAX))?,
                        provider: layer.req("provider", BlockStateProvider::decode)?,
                    })
                })
            })?,
            direction: c.req("direction", value::direction)?,
            allowed_placement: c.req("allowed_placement", BlockPredicate::decode)?,
            prioritize_tip: c.req("prioritize_tip", json::boolean)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VegetationPatchConfig {
    pub replaceable: String,
    pub ground_state: BlockStateProvider,
    pub vegetation_feature: PlacedFeatureHolder,
    /// `CaveSurface.getDirection()`: up for `ceiling`, down for `floor`.
    pub surface: Direction,
    pub depth: IntProvider,
    pub extra_bottom_block_chance: f32,
    pub vertical_range: i32,
    pub vegetation_chance: f32,
    pub xz_radius: IntProvider,
    pub extra_edge_column_chance: f32,
}

impl VegetationPatchConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        Ok(Self {
            replaceable: c.req("replaceable", hashed_tag)?,
            ground_state: c.req("ground_state", BlockStateProvider::decode)?,
            vegetation_feature: c.req("vegetation_feature", PlacedFeatureHolder::decode_placed)?,
            surface: c.req("surface", |v| match json::string(v)?.as_str() {
                "ceiling" => Ok(Direction::Up),
                "floor" => Ok(Direction::Down),
                other => fail(format!("unknown cave surface `{other}`")),
            })?,
            depth: c.req("depth", |v| value::decode_int_provider_in(v, 1, 128))?,
            extra_bottom_block_chance: c.req("extra_bottom_block_chance", unit_float)?,
            vertical_range: c.req("vertical_range", |v| json::int_in(v, 1, 256))?,
            vegetation_chance: c.req("vegetation_chance", unit_float)?,
            xz_radius: c.req("xz_radius", value::decode_int_provider)?,
            extra_edge_column_chance: c.req("extra_edge_column_chance", unit_float)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RootSystemConfig {
    pub feature: PlacedFeatureHolder,
    pub required_vertical_space_for_tree: i32,
    pub root_radius: i32,
    pub root_replaceable: String,
    pub root_state_provider: BlockStateProvider,
    pub root_placement_attempts: i32,
    pub root_column_max_height: i32,
    pub hanging_root_radius: i32,
    pub hanging_roots_vertical_span: i32,
    pub hanging_root_state_provider: BlockStateProvider,
    pub hanging_root_placement_attempts: i32,
    pub allowed_vertical_water_for_tree: i32,
    pub allowed_tree_position: BlockPredicate,
}

impl RootSystemConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        let int = |name: &str, max| c.req(name, |v| json::int_in(v, 1, max));
        Ok(Self {
            feature: c.req("feature", PlacedFeatureHolder::decode_placed)?,
            required_vertical_space_for_tree: int("required_vertical_space_for_tree", 64)?,
            root_radius: int("root_radius", 64)?,
            root_replaceable: c.req("root_replaceable", hashed_tag)?,
            root_state_provider: c.req("root_state_provider", BlockStateProvider::decode)?,
            root_placement_attempts: int("root_placement_attempts", 256)?,
            root_column_max_height: int("root_column_max_height", 4096)?,
            hanging_root_radius: int("hanging_root_radius", 64)?,
            hanging_roots_vertical_span: int("hanging_roots_vertical_span", 16)?,
            hanging_root_state_provider: c
                .req("hanging_root_state_provider", BlockStateProvider::decode)?,
            hanging_root_placement_attempts: int("hanging_root_placement_attempts", 256)?,
            allowed_vertical_water_for_tree: int("allowed_vertical_water_for_tree", 64)?,
            allowed_tree_position: c.req("allowed_tree_position", BlockPredicate::decode)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultifaceGrowthConfig {
    /// A `MultifaceSpreadeableBlock`: glow lichen or sculk vein.
    pub block: &'static str,
    pub search_range: i32,
    pub chance_of_spreading: f32,
    pub can_be_placed_on: HolderSet,
    /// UP when ceilings are allowed, DOWN when floors are, then the
    /// horizontal plane when walls are.
    pub valid_directions: Vec<Direction>,
}

impl MultifaceGrowthConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        let flag = |name| c.or_else(name, false, json::boolean);
        let mut valid_directions = Vec::with_capacity(6);
        if flag("can_place_on_ceiling") {
            valid_directions.push(Direction::Up);
        }
        if flag("can_place_on_floor") {
            valid_directions.push(Direction::Down);
        }
        if flag("can_place_on_wall") {
            valid_directions.extend([
                Direction::North,
                Direction::East,
                Direction::South,
                Direction::West,
            ]);
        }
        Ok(Self {
            block: c.or_else("block", "minecraft:glow_lichen", |v| {
                match block_id(v)?.as_str() {
                    "minecraft:glow_lichen" => Ok("minecraft:glow_lichen"),
                    "minecraft:sculk_vein" => Ok("minecraft:sculk_vein"),
                    _ => fail("Growth block should be a multiface spreadeable block"),
                }
            }),
            search_range: c.or_else("search_range", 10, |v| json::int_in(v, 1, 64)),
            chance_of_spreading: c.or_else("chance_of_spreading", 0.5, unit_float),
            can_be_placed_on: c.req("can_be_placed_on", HolderSet::blocks)?,
            valid_directions,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnderwaterMagmaConfig {
    pub floor_search_range: i32,
    pub placement_radius_around_floor: i32,
    pub placement_probability_per_valid_position: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockBlobConfig {
    pub state: i32,
    pub can_place_on: BlockPredicate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiskConfig {
    pub state_provider: BlockStateProvider,
    pub target: BlockPredicate,
    pub radius: IntProvider,
    pub half_height: i32,
}

/// `EndSpikeFeature.EndSpike`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndSpike {
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,
    pub height: i32,
    pub guarded: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EndSpikeConfig {
    pub crystal_invulnerable: bool,
    pub spikes: Vec<EndSpike>,
    pub crystal_beam_target: Option<BlockPos>,
}

impl EndSpikeConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        Ok(Self {
            crystal_invulnerable: c.or_else("crystal_invulnerable", false, json::boolean),
            spikes: c.req("spikes", |v| {
                json::list(v, |spike| {
                    let spike = Obj::of(spike)?;
                    let int = |name| spike.or_else(name, 0, json::int);
                    Ok(EndSpike {
                        center_x: int("centerX"),
                        center_z: int("centerZ"),
                        radius: int("radius"),
                        height: int("height"),
                        guarded: spike.or_else("guarded", false, json::boolean),
                    })
                })
            })?,
            crystal_beam_target: c.opt("crystal_beam_target", value::block_pos)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EndGatewayConfig {
    pub exit: Option<BlockPos>,
    pub exact: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HugeFungusConfig {
    pub valid_base_state: i32,
    pub stem_state: i32,
    pub hat_state: i32,
    pub decor_state: i32,
    pub replaceable_blocks: BlockPredicate,
    pub planted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetherForestVegetationConfig {
    pub state_provider: BlockStateProvider,
    pub spread_width: i32,
    pub spread_height: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeltaConfig {
    pub contents: i32,
    pub rim: i32,
    pub size: IntProvider,
    pub rim_size: IntProvider,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceSphereConfig {
    pub target: i32,
    pub state: i32,
    pub radius: IntProvider,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RandomSelectorConfig {
    pub features: Vec<(PlacedFeatureHolder, f32)>,
    pub default: PlacedFeatureHolder,
}

/// `ExtraCodecs.nonEmptyHolderSet(PlacedFeature.LIST_CODEC)`.
#[derive(Debug, Clone, PartialEq)]
pub enum PlacedFeatureSet {
    Tag(String),
    Direct(Vec<PlacedFeatureHolder>),
}

impl PlacedFeatureSet {
    fn decode(value: &Json) -> Result<Self> {
        if let Json::String(raw) = value {
            if let Some(tag) = raw.strip_prefix('#') {
                return Ok(Self::Tag(json::id(&Json::String(tag))?));
            }
        }
        let features = json::one_or_list(value, PlacedFeatureHolder::decode_placed)?;
        if features.is_empty() {
            return fail("Holder set must not be empty");
        }
        Ok(Self::Direct(features))
    }

    fn refs(&self, refs: &mut Refs) {
        match self {
            Self::Tag(tag) => refs.tag(PLACED_FEATURE_REGISTRY, tag),
            Self::Direct(features) => features.iter().for_each(|feature| feature.refs(refs)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeodeConfig {
    pub filling_provider: BlockStateProvider,
    pub inner_layer_provider: BlockStateProvider,
    pub alternate_inner_layer_provider: BlockStateProvider,
    pub middle_layer_provider: BlockStateProvider,
    pub outer_layer_provider: BlockStateProvider,
    pub inner_placements: Vec<i32>,
    pub cannot_replace: String,
    pub invalid_blocks: String,
    pub filling: f64,
    pub inner_layer: f64,
    pub middle_layer: f64,
    pub outer_layer: f64,
    pub generate_crack_chance: f64,
    pub base_crack_size: f64,
    pub crack_point_offset: i32,
    pub use_potential_placements_chance: f64,
    pub use_alternate_layer0_chance: f64,
    pub placements_require_layer0_alternate: bool,
    pub outer_wall_distance: IntProvider,
    pub distribution_points: IntProvider,
    pub point_offset: IntProvider,
    pub min_gen_offset: i32,
    pub max_gen_offset: i32,
    pub noise_multiplier: f64,
    pub invalid_blocks_threshold: i32,
}

impl GeodeConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        let chance = |obj: &Obj<'_>, name, default| {
            obj.or_else(name, default, |v| json::double_in(v, 0.0, 1.0))
        };
        let blocks = c.req("blocks", Obj::of)?;
        let provider = |name: &str| blocks.req(name, BlockStateProvider::decode).at("blocks");
        let layers = c.req("layers", Obj::of)?;
        let layer =
            |name, default| layers.or_else(name, default, |v| json::double_in(v, 0.01, 50.0));
        let crack = c.req("crack", Obj::of)?;
        let uniform = |min_inclusive, max_inclusive| IntProvider::Uniform {
            min_inclusive,
            max_inclusive,
        };
        Ok(Self {
            filling_provider: provider("filling_provider")?,
            inner_layer_provider: provider("inner_layer_provider")?,
            alternate_inner_layer_provider: provider("alternate_inner_layer_provider")?,
            middle_layer_provider: provider("middle_layer_provider")?,
            outer_layer_provider: provider("outer_layer_provider")?,
            inner_placements: blocks
                .req("inner_placements", |v| {
                    json::nonempty_list(v, value::block_state)
                })
                .at("blocks")?,
            cannot_replace: blocks.req("cannot_replace", hashed_tag).at("blocks")?,
            invalid_blocks: blocks.req("invalid_blocks", hashed_tag).at("blocks")?,
            filling: layer("filling", 1.7),
            inner_layer: layer("inner_layer", 2.2),
            middle_layer: layer("middle_layer", 3.2),
            outer_layer: layer("outer_layer", 4.2),
            generate_crack_chance: chance(&crack, "generate_crack_chance", 1.0),
            base_crack_size: crack
                .or_else("base_crack_size", 2.0, |v| json::double_in(v, 0.0, 5.0)),
            crack_point_offset: crack.or_else("crack_point_offset", 2, |v| json::int_in(v, 0, 10)),
            use_potential_placements_chance: chance(c, "use_potential_placements_chance", 0.35),
            use_alternate_layer0_chance: chance(c, "use_alternate_layer0_chance", 0.0),
            placements_require_layer0_alternate: c.or_else(
                "placements_require_layer0_alternate",
                true,
                json::boolean,
            ),
            outer_wall_distance: c.or_else("outer_wall_distance", uniform(4, 5), |v| {
                value::decode_int_provider_in(v, 1, 20)
            }),
            distribution_points: c.or_else("distribution_points", uniform(3, 4), |v| {
                value::decode_int_provider_in(v, 1, 20)
            }),
            point_offset: c.or_else("point_offset", uniform(1, 2), |v| {
                value::decode_int_provider_in(v, 0, 10)
            }),
            min_gen_offset: c.or_else("min_gen_offset", -16, json::int),
            max_gen_offset: c.or_else("max_gen_offset", 16, json::int),
            noise_multiplier: chance(c, "noise_multiplier", 0.05),
            invalid_blocks_threshold: c.req("invalid_blocks_threshold", json::int)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DripstoneClusterConfig {
    pub floor_to_ceiling_search_range: i32,
    pub height: IntProvider,
    pub radius: IntProvider,
    pub max_stalagmite_stalactite_height_diff: i32,
    pub height_deviation: i32,
    pub dripstone_block_layer_thickness: IntProvider,
    pub density: FloatProvider,
    pub wetness: FloatProvider,
    pub chance_of_dripstone_column_at_max_distance_from_center: f32,
    pub max_distance_from_edge_affecting_chance_of_dripstone_column: i32,
    pub max_distance_from_center_affecting_height_bias: i32,
}

impl DripstoneClusterConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        let int = |name: &str, min, max| c.req(name, |v| json::int_in(v, min, max));
        let ints =
            |name: &str, min, max| c.req(name, |v| value::decode_int_provider_in(v, min, max));
        let floats = |name: &str| c.req(name, |v| value::decode_float_provider_in(v, 0.0, 2.0));
        Ok(Self {
            floor_to_ceiling_search_range: int("floor_to_ceiling_search_range", 1, 512)?,
            height: ints("height", 1, 128)?,
            radius: ints("radius", 1, 128)?,
            max_stalagmite_stalactite_height_diff: int(
                "max_stalagmite_stalactite_height_diff",
                0,
                64,
            )?,
            height_deviation: int("height_deviation", 1, 64)?,
            dripstone_block_layer_thickness: ints("dripstone_block_layer_thickness", 0, 128)?,
            density: floats("density")?,
            wetness: floats("wetness")?,
            chance_of_dripstone_column_at_max_distance_from_center: c.req(
                "chance_of_dripstone_column_at_max_distance_from_center",
                unit_float,
            )?,
            max_distance_from_edge_affecting_chance_of_dripstone_column: int(
                "max_distance_from_edge_affecting_chance_of_dripstone_column",
                1,
                64,
            )?,
            max_distance_from_center_affecting_height_bias: int(
                "max_distance_from_center_affecting_height_bias",
                1,
                64,
            )?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LargeDripstoneConfig {
    pub floor_to_ceiling_search_range: i32,
    pub column_radius: IntProvider,
    pub height_scale: FloatProvider,
    pub max_column_radius_to_cave_height_ratio: f32,
    pub stalactite_bluntness: FloatProvider,
    pub stalagmite_bluntness: FloatProvider,
    pub wind_speed: FloatProvider,
    pub min_radius_for_wind: i32,
    pub min_bluntness_for_wind: f32,
}

impl LargeDripstoneConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        let floats =
            |name: &str, min, max| c.req(name, |v| value::decode_float_provider_in(v, min, max));
        Ok(Self {
            floor_to_ceiling_search_range: c.or_else("floor_to_ceiling_search_range", 30, |v| {
                json::int_in(v, 1, 512)
            }),
            column_radius: c.req("column_radius", |v| value::decode_int_provider_in(v, 1, 60))?,
            height_scale: floats("height_scale", 0.0, 20.0)?,
            max_column_radius_to_cave_height_ratio: c
                .req("max_column_radius_to_cave_height_ratio", |v| {
                    json::float_in(v, 0.1, 1.0)
                })?,
            stalactite_bluntness: floats("stalactite_bluntness", 0.1, 10.0)?,
            stalagmite_bluntness: floats("stalagmite_bluntness", 0.1, 10.0)?,
            wind_speed: floats("wind_speed", 0.0, 2.0)?,
            min_radius_for_wind: c.req("min_radius_for_wind", |v| json::int_in(v, 0, 100))?,
            min_bluntness_for_wind: c
                .req("min_bluntness_for_wind", |v| json::float_in(v, 0.0, 5.0))?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointedDripstoneConfig {
    pub chance_of_taller_dripstone: f32,
    pub chance_of_directional_spread: f32,
    pub chance_of_spread_radius2: f32,
    pub chance_of_spread_radius3: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SculkPatchConfig {
    pub charge_count: i32,
    pub amount_per_charge: i32,
    pub spread_attempts: i32,
    pub growth_rounds: i32,
    pub spread_rounds: i32,
    pub extra_rare_growths: IntProvider,
    pub catalyst_chance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FallenTreeConfig {
    pub trunk_provider: BlockStateProvider,
    pub log_length: IntProvider,
    pub stump_decorators: Vec<TreeDecorator>,
    pub log_decorators: Vec<TreeDecorator>,
}

impl FallenTreeConfig {
    fn decode(c: &Obj<'_>) -> Result<Self> {
        let decorators = |v: &Json| json::list(v, TreeDecorator::decode);
        Ok(Self {
            trunk_provider: c.req("trunk_provider", BlockStateProvider::decode)?,
            log_length: c.req("log_length", |v| value::decode_int_provider_in(v, 0, 16))?,
            stump_decorators: c.req("stump_decorators", decorators)?,
            log_decorators: c.req("log_decorators", decorators)?,
        })
    }
}
