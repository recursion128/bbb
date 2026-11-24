//! `BonemealableBlock`: the three questions bone meal asks the clicked block,
//! answered per generated owner class in [`target`].

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::block_types::{
    vanilla_block_type, vanilla_block_type_for_resource_id, VanillaBonemealTargetOverride,
    VANILLA_BLOCK_BROWN_MUSHROOM_ID, VANILLA_BLOCK_CRIMSON_FUNGUS_ID,
    VANILLA_BLOCK_CRIMSON_NYLIUM_ID, VANILLA_BLOCK_FERN_ID, VANILLA_BLOCK_HANGING_ROOTS_ID,
    VANILLA_BLOCK_LARGE_FERN_ID, VANILLA_BLOCK_MANGROVE_PROPAGULE_ID, VANILLA_BLOCK_MOSS_BLOCK_ID,
    VANILLA_BLOCK_SEAGRASS_ID, VANILLA_BLOCK_SEA_PICKLE_ID, VANILLA_BLOCK_SHORT_DRY_GRASS_ID,
    VANILLA_BLOCK_SHORT_GRASS_ID, VANILLA_BLOCK_TALL_DRY_GRASS_ID, VANILLA_BLOCK_TALL_GRASS_ID,
    VANILLA_BLOCK_TALL_SEAGRASS_ID, VANILLA_BLOCK_WARPED_NYLIUM_ID, VANILLA_BLOCK_WATER_ID,
};
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{BlockPos, Direction, ItemStackSummary};
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{air_block_state_id, block_state_is_air};
use bbb_server_worldgen::multiface as spreading;
use bbb_server_worldgen::placed_feature::place_placed_feature;
use bbb_server_worldgen::{place_configured_feature, WorldGenLevel};

use super::level::SET_BLOCK_UPDATE_CLIENTS;
use super::{
    bamboo, big_dripleaf, block_type_of, bool_property, can_survive_at, cocoa, crop,
    default_block_state_id, growing_plant, int_property, light_dampening, multiface, offset,
    pitcher_crop, sapling, state_is_block, stem, sweet_berry_bush,
};
use crate::fluid::{above, below, with_property, SET_BLOCK_UPDATE_ALL};
use crate::world::simulation_level::LevelCtx;

type Valid = fn(&LevelCtx<'_, '_>, BlockPos, i32) -> bool;
type Success = fn(&LevelCtx<'_, '_>, i32) -> bool;
type Perform = fn(&mut LevelCtx<'_, '_>, BlockPos, i32);

/// `isValidBonemealTarget`, `isBonemealSuccess` and `performBonemeal` of one
/// owner class.
struct BonemealTarget {
    valid: Valid,
    success: Success,
    perform: Perform,
}

/// `BoneMealItem.growCrop` (`BoneMealItem.java:64-79`) without the shrink:
/// true when the block is a valid target, whether or not it grew.
pub(crate) fn grow_crop(level: &mut LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let state = level.block_state(pos);
    let Some(target) = block_type_of(state)
        .and_then(vanilla_block_type)
        .and_then(|block| block.bonemeal_target_override)
        .and_then(target)
    else {
        return false;
    };
    if !(target.valid)(level, pos, state) {
        return false;
    }
    if (target.success)(level, state) {
        (target.perform)(level, pos, state);
    }
    true
}

/// `BoneMealItem.growWaterPlant` (`BoneMealItem.java:81-142`) without consumption.
pub(crate) fn grow_water_plant(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    clicked_face: Option<Direction>,
) -> bool {
    if !state_is_block(level.block_state(pos), VANILLA_BLOCK_WATER_ID)
        || !level.fluid_state(pos).is_full()
    {
        return false;
    }
    let random = level.level_random();
    let mut underwater_bonemeals: Option<Vec<i32>> = None;
    'walk: for j in 0..128 {
        let mut at = pos;
        for _ in 0..j / 16 {
            at = offset(
                at,
                random.next_int(3) - 1,
                (random.next_int(3) - 1) * random.next_int(3) / 2,
                random.next_int(3) - 1,
            );
            if super::collision_shape_full_block_at(level.block_state(at), at) {
                continue 'walk;
            }
        }
        let mut state = default_block_state_id(VANILLA_BLOCK_SEAGRASS_ID);
        if level.biome_resource_id(at).is_some_and(|biome| {
            level.registry_tag_membership(
                "minecraft:worldgen/biome",
                "minecraft:produces_corals_from_bonemeal",
                biome,
            )
        }) {
            if j == 0 && clicked_face.is_some_and(|face| HORIZONTAL.contains(&face)) {
                let wall_corals = resolve_water_plant_states(level, WALL_CORALS);
                state = pick_random_state(&wall_corals, random).unwrap_or(state);
                state = with_property(
                    state,
                    "facing",
                    super::direction_serialized_name(clicked_face.unwrap()),
                )
                .unwrap_or(state);
            } else if random.next_int(4) == 0 {
                let plants = underwater_bonemeals.get_or_insert_with(|| {
                    resolve_water_plant_states(level, "minecraft:underwater_bonemeals")
                });
                state = pick_random_state(plants, random).unwrap_or(state);
            }
        }
        if level.block_state_in_tag(state, WALL_CORALS)
            && super::direction_property(state, "facing").is_some()
        {
            for _ in 0..4 {
                if can_survive_at(state, at, level) {
                    break;
                }
                state = with_property(
                    state,
                    "facing",
                    super::direction_serialized_name(HORIZONTAL[random.next_int(4) as usize]),
                )
                .unwrap_or(state);
            }
        }
        if can_survive_at(state, at, level) {
            let current = level.block_state(at);
            if state_is_block(current, VANILLA_BLOCK_WATER_ID) && level.fluid_state(at).is_full() {
                set(level, at, state, SET_BLOCK_UPDATE_ALL);
            } else if state_is_block(current, VANILLA_BLOCK_SEAGRASS_ID)
                && state_is_block(level.block_state(above(at)), VANILLA_BLOCK_WATER_ID)
                && random.next_int(10) == 0
            {
                place_double_plant(
                    level,
                    at,
                    default_block_state_id(VANILLA_BLOCK_TALL_SEAGRASS_ID),
                );
            }
        }
    }
    true
}

const GLOW_LICHEN_SPREAD: spreading::SpreadConfig = spreading::SpreadConfig::Default {
    block: "minecraft:glow_lichen",
};

const WALL_CORALS: &str = "minecraft:wall_corals";

fn resolve_water_plant_states(level: &LevelCtx<'_, '_>, tag: &str) -> Vec<i32> {
    let mut members = level.registry_tag_members("minecraft:block", tag);
    let fallback;
    if members.is_empty() {
        fallback = vanilla_persistent_registry_tag_members_26_1("minecraft:block", tag);
        if let PersistentRegistryTagMembers::Present(ref builtin) = fallback {
            members.extend(
                builtin
                    .iter()
                    .map(String::as_str)
                    .filter(|id| level.registry_tag_membership("minecraft:block", tag, id)),
            );
        }
    }
    members
        .into_iter()
        .filter_map(|id| {
            vanilla_block_type_for_resource_id(id).map(|block| default_block_state_id(block.id))
        })
        .collect()
}

fn pick_random_state(states: &[i32], random: &crate::random::GameRandom) -> Option<i32> {
    if states.is_empty() {
        return None;
    }
    Some(states[random.next_int(states.len() as i32) as usize])
}

fn target(owner: VanillaBonemealTargetOverride) -> Option<BonemealTarget> {
    use VanillaBonemealTargetOverride as Owner;
    let (valid, success, perform): (Valid, Success, Perform) = match owner {
        Owner::CropBlock => (
            |_, _, s| crop::is_valid_bonemeal_target(s),
            always,
            grow_crop_age,
        ),
        Owner::StemBlock => (|_, _, s| int_property(s, AGE) != Some(7), always, grow_stem),
        Owner::PitcherCropBlock => (
            pitcher_crop::is_valid_bonemeal_target,
            always,
            pitcher_crop::perform_bonemeal,
        ),
        Owner::SweetBerryBushBlock => (
            |l, p, s| {
                sweet_berry_bush::age(s).is_some_and(|age| age < sweet_berry_bush::MAX_AGE)
                    && air_in_build_height(l, above(p))
            },
            always,
            |l, p, s| {
                let age = sweet_berry_bush::age(s).unwrap_or(0);
                if let Some(grown) =
                    sweet_berry_bush::with_age(s, (age + 1).min(sweet_berry_bush::MAX_AGE))
                {
                    set(l, p, grown, SET_BLOCK_UPDATE_CLIENTS);
                }
            },
        ),
        Owner::CocoaBlock => (
            |_, _, s| int_property(s, AGE).is_some_and(|age| age < cocoa::MAX_AGE),
            always,
            |l, p, s| set_int(l, p, s, AGE, int_property(s, AGE).unwrap_or(0) + 1),
        ),
        // SaplingBlock.java:60-67.
        Owner::SaplingBlock => (
            |l, p, s| in_build_height(l, offset_up(p, sapling::minimum_height(l, s))),
            chance_45,
            |l, p, s| sapling::advance_tree(l, p, s),
        ),
        Owner::MangrovePropaguleBlock => (
            |_, _, s| bool_property(s, "hanging") != Some(true) || hanging_propagule_growing(s),
            |l, s| {
                if bool_property(s, "hanging") == Some(true) {
                    hanging_propagule_growing(s)
                } else {
                    chance_45(l, s)
                }
            },
            |l, p, s| {
                if hanging_propagule_growing(s) {
                    set_int(l, p, s, AGE, int_property(s, AGE).unwrap_or(0) + 1);
                } else {
                    sapling::advance_tree(l, p, s);
                }
            },
        ),
        Owner::AzaleaBlock => (
            // AzaleaBlock.java:43-50.
            |l, p, s| {
                in_build_height(l, offset_up(p, sapling::minimum_height(l, s) + 2))
                    && l.fluid_state(above(p)).is_empty()
            },
            chance_45,
            |l, p, s| {
                sapling::grow_tree(l, p, s);
            },
        ),
        Owner::BambooSaplingBlock => (
            |l, p, _| bamboo::sapling_is_valid_bonemeal_target(l, p),
            always,
            |l, p, _| bamboo::sapling_perform_bonemeal(l, p),
        ),
        Owner::BambooStalkBlock => (
            |l, p, _| bamboo::stalk_is_valid_bonemeal_target(l, p),
            always,
            |l, p, _| bamboo::stalk_perform_bonemeal(l, p),
        ),
        Owner::GrowingPlantHeadBlock | Owner::GrowingPlantBodyBlock => (
            |l, p, s| growing_plant::bonemeal_head(l, p, s).is_some(),
            always,
            |l, p, s| {
                if let Some((head, kind)) = growing_plant::bonemeal_head(l, p, s) {
                    growing_plant::perform_head_bonemeal(l, head, kind);
                }
            },
        ),
        Owner::CaveVinesBlock | Owner::CaveVinesPlantBlock => (
            |_, _, s| !growing_plant::has_berries(s),
            always,
            |l, p, s| {
                if let Some(berries) = growing_plant::with_berries(s, true) {
                    set(l, p, berries, SET_BLOCK_UPDATE_CLIENTS);
                }
            },
        ),
        Owner::GlowLichenBlock => (
            |l, p, s| spreading::can_spread_from_any_face(GLOW_LICHEN_SPREAD, s, l, p),
            always,
            |l, p, s| {
                let random = l.level_random();
                spreading::spread_from_random_face_toward_random_direction(
                    GLOW_LICHEN_SPREAD,
                    s,
                    l,
                    p,
                    &mut &*random,
                );
            },
        ),
        Owner::GrassBlock => (
            |l, p, _| air_in_build_height(l, above(p)),
            always,
            grow_grass,
        ),
        Owner::NyliumBlock => (
            |l, p, _| air_in_build_height(l, above(p)),
            always,
            grow_nylium,
        ),
        Owner::BonemealableFeaturePlacerBlock => (
            |l, p, _| block_state_is_air(l.block_state(above(p))),
            always,
            |l, p, s| {
                let feature = if state_is_block(s, VANILLA_BLOCK_MOSS_BLOCK_ID) {
                    "minecraft:moss_patch_bonemeal"
                } else {
                    "minecraft:pale_moss_patch_bonemeal"
                };
                place_feature(l, feature, above(p));
            },
        ),
        Owner::MushroomBlock => (mushroom_has_room, chance_40, grow_mushroom),
        Owner::NetherFungusBlock => (
            |l, p, s| {
                let nylium = if state_is_block(s, VANILLA_BLOCK_CRIMSON_FUNGUS_ID) {
                    VANILLA_BLOCK_CRIMSON_NYLIUM_ID
                } else {
                    VANILLA_BLOCK_WARPED_NYLIUM_ID
                };
                state_is_block(l.block_state(below(p)), nylium) && in_build_height(l, above(p))
            },
            chance_40,
            |l, p, s| {
                let feature = if state_is_block(s, VANILLA_BLOCK_CRIMSON_FUNGUS_ID) {
                    "minecraft:crimson_fungus_planted"
                } else {
                    "minecraft:warped_fungus_planted"
                };
                place_feature(l, feature, p);
            },
        ),
        Owner::TallGrassBlock => (
            |l, p, s| {
                let lower = double_plant_half(grown_tall_grass(s), "lower");
                can_survive_at(lower, p, l) && air_in_build_height(l, above(p))
            },
            always,
            |l, p, s| place_double_plant(l, p, grown_tall_grass(s)),
        ),
        Owner::TallFlowerBlock => (|_, _, _| true, always, |l, p, s| pop_self(l, p, s)),
        Owner::FlowerBedBlock => (
            |_, _, _| true,
            always,
            |l, p, s| match int_property(s, FLOWER_AMOUNT) {
                Some(amount) if amount < 4 => set_int(l, p, s, FLOWER_AMOUNT, amount + 1),
                _ => pop_self(l, p, s),
            },
        ),
        Owner::SeagrassBlock => (
            |l, p, _| state_is_block(l.block_state(above(p)), VANILLA_BLOCK_WATER_ID),
            always,
            |l, p, _| {
                place_double_plant(l, p, default_block_state_id(VANILLA_BLOCK_TALL_SEAGRASS_ID))
            },
        ),
        Owner::SeaPickleBlock => (
            |l, p, s| {
                bool_property(s, "waterlogged") == Some(true)
                    && l.block_state_in_tag(l.block_state(below(p)), CORAL_BLOCKS)
            },
            always,
            grow_sea_pickles,
        ),
        Owner::RootedDirtBlock => (
            |l, p, _| air_in_build_height(l, below(p)),
            always,
            |l, p, _| {
                let roots = default_block_state_id(VANILLA_BLOCK_HANGING_ROOTS_ID);
                set(l, below(p), roots, SET_BLOCK_UPDATE_ALL);
            },
        ),
        Owner::MangroveLeavesBlock => (
            |l, p, _| block_state_is_air(l.block_state(below(p))),
            always,
            |l, p, _| {
                let propagule = default_block_state_id(VANILLA_BLOCK_MANGROVE_PROPAGULE_ID);
                if let Some(hanging) = with_property(propagule, "hanging", "true")
                    .and_then(|state| with_property(state, AGE, "0"))
                {
                    set(l, below(p), hanging, SET_BLOCK_UPDATE_CLIENTS);
                }
            },
        ),
        Owner::NetherrackBlock => (
            |l, p, _| {
                light_dampening(l.block_state(above(p))) == 0
                    && cube(p).any(|at| l.block_state_in_tag(l.block_state(at), NYLIUM))
            },
            always,
            spread_nylium,
        ),
        Owner::BushBlock | Owner::FireflyBushBlock => (
            |l, p, s| spreadable_neighbour(l, p, own_default(s)).is_some(),
            always,
            |l, p, s| {
                let state = own_default(s);
                if let Some(at) = shuffled_spreadable_neighbour(l, p, state) {
                    set(l, at, state, SET_BLOCK_UPDATE_ALL);
                }
            },
        ),
        Owner::ShortDryGrassBlock => (
            |l, p, _| air_in_build_height(l, above(p)),
            always,
            |l, p, _| {
                set(
                    l,
                    p,
                    default_block_state_id(VANILLA_BLOCK_TALL_DRY_GRASS_ID),
                    SET_BLOCK_UPDATE_ALL,
                );
            },
        ),
        Owner::TallDryGrassBlock => (
            |l, p, _| {
                let short = default_block_state_id(VANILLA_BLOCK_SHORT_DRY_GRASS_ID);
                spreadable_neighbour(l, p, short).is_some() && air_in_build_height(l, above(p))
            },
            always,
            |l, p, _| {
                let short = default_block_state_id(VANILLA_BLOCK_SHORT_DRY_GRASS_ID);
                if let Some(at) = shuffled_spreadable_neighbour(l, p, short) {
                    set(l, at, short, SET_BLOCK_UPDATE_ALL);
                }
            },
        ),
        Owner::HangingMossBlock => (
            |l, p, s| air_in_build_height(l, hanging_moss_growth(l, p, s)),
            always,
            |l, p, s| {
                let growth = hanging_moss_growth(l, p, s);
                if block_state_is_air(l.block_state(growth)) {
                    if let Some(tip) = with_property(s, "tip", "true") {
                        set(l, growth, tip, SET_BLOCK_UPDATE_ALL);
                    }
                }
            },
        ),
        Owner::MossyCarpetBlock => (
            multiface::mossy_carpet_is_valid_bonemeal_target,
            always,
            multiface::mossy_carpet_perform_bonemeal,
        ),
        Owner::SmallDripleafBlock => (|_, _, _| true, always, big_dripleaf::small_perform_bonemeal),
        Owner::BigDripleafBlock | Owner::BigDripleafStemBlock => (
            big_dripleaf::is_valid_bonemeal_target,
            always,
            big_dripleaf::perform_bonemeal,
        ),
    };
    Some(BonemealTarget {
        valid,
        success,
        perform,
    })
}

const AGE: &str = "age";
const FLOWER_AMOUNT: &str = "flower_amount";
const CORAL_BLOCKS: &str = "minecraft:coral_blocks";
const NYLIUM: &str = "minecraft:nylium";

fn always(_: &LevelCtx<'_, '_>, _: i32) -> bool {
    true
}

/// `SaplingBlock.isBonemealSuccess` (`SaplingBlock.java:70-72`) and
/// `AzaleaBlock`'s (`AzaleaBlock.java:53-55`).
fn chance_45(level: &LevelCtx<'_, '_>, _: i32) -> bool {
    f64::from(level.level_random().next_float()) < 0.45
}

/// `MushroomBlock` (`MushroomBlock.java:128-130`) and `NetherFungusBlock`
/// (`NetherFungusBlock.java:77-79`).
fn chance_40(level: &LevelCtx<'_, '_>, _: i32) -> bool {
    f64::from(level.level_random().next_float()) < 0.4
}

fn set(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32, flags: i32) {
    level.set_block(pos, state, flags, crate::neighbor_updater::UPDATE_LIMIT);
}

fn set_int(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32, property: &str, value: i32) {
    if let Some(next) = with_property(state, property, &value.to_string()) {
        set(level, pos, next, SET_BLOCK_UPDATE_CLIENTS);
    }
}

fn in_build_height(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    pos.y >= level.get_min_y() && pos.y <= level.get_max_y()
}

fn air_in_build_height(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    in_build_height(level, pos) && block_state_is_air(level.block_state(pos))
}

fn offset_up(pos: BlockPos, height: i32) -> BlockPos {
    BlockPos {
        y: pos.y + height,
        ..pos
    }
}

/// `MushroomBlock.isValidBonemealTarget` (`MushroomBlock.java:105-125`).
fn mushroom_has_room(level: &LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> bool {
    use bbb_server_datapack::worldgen::feature::ConfiguredFeature;
    let feature = if state_is_block(state, VANILLA_BLOCK_BROWN_MUSHROOM_ID) {
        "minecraft:huge_brown_mushroom"
    } else {
        "minecraft:huge_red_mushroom"
    };
    match level.configured_feature(feature).as_deref() {
        Some(ConfiguredFeature::HugeRedMushroom(config))
        | Some(ConfiguredFeature::HugeBrownMushroom(config)) => {
            in_build_height(level, offset_up(pos, 4 + config.foliage_radius))
        }
        _ => false,
    }
}

/// `CropBlock.growCrops` (`CropBlock.java:91-94`) with each class's
/// `getBonemealAgeIncrease`.
fn grow_crop_age(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let (Some(kind), Some(age)) = (crop::crop_kind(state), crop::age(state)) else {
        return;
    };
    let increase = match kind {
        crop::CropKind::Plain => 2 + level.level_random().next_int(4),
        crop::CropKind::Beetroot => (2 + level.level_random().next_int(4)) / 3,
        crop::CropKind::Torchflower => 1,
    };
    if let Some(grown) = crop::state_for_age(state, (age + increase).min(kind.max_age())) {
        set(level, pos, grown, SET_BLOCK_UPDATE_CLIENTS);
    }
}

/// `StemBlock.performBonemeal` (`StemBlock.java:125-132`).
fn grow_stem(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let age = (int_property(state, AGE).unwrap_or(0) + 2 + level.level_random().next_int(4)).min(7);
    let Some(grown) = with_property(state, AGE, &age.to_string()) else {
        return;
    };
    set(level, pos, grown, SET_BLOCK_UPDATE_CLIENTS);
    if age == 7 {
        stem::random_tick(level, pos, grown);
    }
}

fn hanging_propagule_growing(state: i32) -> bool {
    bool_property(state, "hanging") == Some(true)
        && int_property(state, AGE).is_some_and(|age| age < 4)
}

fn place_feature(level: &mut LevelCtx<'_, '_>, id: &str, pos: BlockPos) -> bool {
    let Some(feature) = level.configured_feature(id) else {
        return false;
    };
    let random = level.level_random();
    place_configured_feature(&feature, level, pos, &mut &*random)
}

/// `MushroomBlock.growMushroom` (`MushroomBlock.java:89-102`).
fn grow_mushroom(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let feature = if state_is_block(state, VANILLA_BLOCK_BROWN_MUSHROOM_ID) {
        "minecraft:huge_brown_mushroom"
    } else {
        "minecraft:huge_red_mushroom"
    };
    if level.configured_feature(feature).is_none() {
        return;
    }
    set(level, pos, air_block_state_id(), SET_BLOCK_UPDATE_ALL);
    if !place_feature(level, feature, pos) {
        set(level, pos, state, SET_BLOCK_UPDATE_ALL);
    }
}

/// `NyliumBlock.performBonemeal` (`NyliumBlock.java:57-71`).
fn grow_nylium(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let above_pos = above(pos);
    if state_is_block(state, VANILLA_BLOCK_CRIMSON_NYLIUM_ID) {
        place_feature(
            level,
            "minecraft:crimson_forest_vegetation_bonemeal",
            above_pos,
        );
    } else if state_is_block(state, VANILLA_BLOCK_WARPED_NYLIUM_ID) {
        place_feature(
            level,
            "minecraft:warped_forest_vegetation_bonemeal",
            above_pos,
        );
        place_feature(level, "minecraft:nether_sprouts_bonemeal", above_pos);
        if level.level_random().next_int(8) == 0 {
            place_feature(level, "minecraft:twisting_vines_bonemeal", above_pos);
        }
    }
}

/// `GrassBlock.performBonemeal` (`GrassBlock.java:44-86`).
fn grow_grass(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let grass_feature = level.placed_feature("minecraft:grass_bonemeal");
    let Some(grass_block) = block_type_of(state) else {
        return;
    };
    let random = level.level_random();
    'walk: for j in 0..128 {
        let mut test = above(pos);
        for _ in 0..j / 16 {
            test = offset(
                test,
                random.next_int(3) - 1,
                (random.next_int(3) - 1) * random.next_int(3) / 2,
                random.next_int(3) - 1,
            );
            if !state_is_block(level.block_state(below(test)), grass_block)
                || super::collision_shape_full_block_at(level.block_state(test), test)
            {
                continue 'walk;
            }
        }
        let test_state = level.block_state(test);
        if state_is_block(test_state, VANILLA_BLOCK_SHORT_GRASS_ID) && random.next_int(10) == 0 {
            let tall =
                double_plant_half(default_block_state_id(VANILLA_BLOCK_TALL_GRASS_ID), "lower");
            if can_survive_at(tall, test, level) && air_in_build_height(level, above(test)) {
                place_double_plant(
                    level,
                    test,
                    default_block_state_id(VANILLA_BLOCK_TALL_GRASS_ID),
                );
            }
        }
        if block_state_is_air(test_state) && in_build_height(level, test) {
            if random.next_int(8) == 0 {
                if let Some(features) = level
                    .biome_bonemeal_features(test)
                    .filter(|features| !features.is_empty())
                {
                    let feature = &features[random.next_int(features.len() as i32) as usize];
                    place_configured_feature(feature, level, test, &mut &*random);
                }
            } else if let Some(grass_feature) = &grass_feature {
                place_placed_feature(grass_feature, level, &mut &*random, test);
            }
        }
    }
}

/// `TallGrassBlock.getGrownBlock` (`TallGrassBlock.java:48-50`).
fn grown_tall_grass(state: i32) -> i32 {
    let block = if state_is_block(state, VANILLA_BLOCK_FERN_ID) {
        VANILLA_BLOCK_LARGE_FERN_ID
    } else {
        VANILLA_BLOCK_TALL_GRASS_ID
    };
    default_block_state_id(block)
}

fn double_plant_half(state: i32, half: &str) -> i32 {
    with_property(state, "half", half).unwrap_or(state)
}

/// `DoublePlantBlock.placeAt` (`DoublePlantBlock.java:86-90`) with flags 2.
fn place_double_plant(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    set(
        level,
        pos,
        double_plant_half(state, "lower"),
        SET_BLOCK_UPDATE_CLIENTS,
    );
    set(
        level,
        above(pos),
        double_plant_half(state, "upper"),
        SET_BLOCK_UPDATE_CLIENTS,
    );
}

/// `Block.popResource(level, pos, new ItemStack(this))`.
fn pop_self(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let Some(item) = block_type_of(state)
        .and_then(vanilla_block_type)
        .and_then(|block| vanilla_item_type_for_resource_id(block.resource_id))
    else {
        return;
    };
    let stack = ItemStackSummary {
        item_id: Some(item.id),
        count: 1,
        ..ItemStackSummary::empty()
    };
    level.pop_resource(pos, &stack);
}

/// `SeaPickleBlock.performBonemeal` (`SeaPickleBlock.java:134-169`).
fn grow_sea_pickles(level: &mut LevelCtx<'_, '_>, pos: BlockPos, state: i32) {
    let random = level.level_random();
    let mut z_span = 1;
    let mut z_offset = 0;
    for x in 0..5 {
        for z in 0..z_span {
            for y in pos.y - 1..pos.y + 1 {
                let at = BlockPos {
                    x: pos.x - 2 + x,
                    y,
                    z: pos.z - z_offset + z,
                };
                if at != pos
                    && random.next_int(6) == 0
                    && state_is_block(level.block_state(at), VANILLA_BLOCK_WATER_ID)
                    && level.block_state_in_tag(level.block_state(below(at)), CORAL_BLOCKS)
                {
                    let pickles = (random.next_int(4) + 1).to_string();
                    if let Some(pickle) = with_property(
                        default_block_state_id(VANILLA_BLOCK_SEA_PICKLE_ID),
                        "pickles",
                        &pickles,
                    ) {
                        set(level, at, pickle, SET_BLOCK_UPDATE_ALL);
                    }
                }
            }
        }
        if x < 2 {
            z_span += 2;
            z_offset += 1;
        } else {
            z_span -= 2;
            z_offset -= 1;
        }
    }
    if let Some(full) = with_property(state, "pickles", "4") {
        set(level, pos, full, SET_BLOCK_UPDATE_CLIENTS);
    }
}

fn cube(pos: BlockPos) -> impl Iterator<Item = BlockPos> {
    (-1..=1)
        .flat_map(move |x| (-1..=1).flat_map(move |y| (-1..=1).map(move |z| offset(pos, x, y, z))))
}

/// `NetherrackBlock.performBonemeal` (`NetherrackBlock.java:46-72`).
fn spread_nylium(level: &mut LevelCtx<'_, '_>, pos: BlockPos, _: i32) {
    let mut warped = false;
    let mut crimson = false;
    for at in cube(pos) {
        let state = level.block_state(at);
        warped |= state_is_block(state, VANILLA_BLOCK_WARPED_NYLIUM_ID);
        crimson |= state_is_block(state, VANILLA_BLOCK_CRIMSON_NYLIUM_ID);
    }
    let nylium = match (warped, crimson) {
        (true, true) if level.level_random().next_boolean() => VANILLA_BLOCK_WARPED_NYLIUM_ID,
        (true, true) => VANILLA_BLOCK_CRIMSON_NYLIUM_ID,
        (true, false) => VANILLA_BLOCK_WARPED_NYLIUM_ID,
        (false, true) => VANILLA_BLOCK_CRIMSON_NYLIUM_ID,
        (false, false) => return,
    };
    set(
        level,
        pos,
        default_block_state_id(nylium),
        SET_BLOCK_UPDATE_ALL,
    );
}

fn own_default(state: i32) -> i32 {
    block_type_of(state).map_or(state, default_block_state_id)
}

const HORIZONTAL: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

/// `BonemealableBlock.getSpreadableNeighbourPos` over the horizontal plane.
fn spreadable_neighbour(level: &LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> Option<BlockPos> {
    HORIZONTAL
        .into_iter()
        .map(|direction| relative(pos, direction))
        .find(|&at| block_state_is_air(level.block_state(at)) && can_survive_at(state, at, level))
}

/// `findSpreadableNeighbourPos`, which shuffles the directions first.
fn shuffled_spreadable_neighbour(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
) -> Option<BlockPos> {
    let random = level.level_random();
    let mut directions = HORIZONTAL;
    for index in (1..directions.len()).rev() {
        let swap = random.next_int(index as i32 + 1) as usize;
        directions.swap(index, swap);
    }
    directions
        .into_iter()
        .map(|direction| relative(pos, direction))
        .find(|&at| block_state_is_air(level.block_state(at)) && can_survive_at(state, at, level))
}

/// `HangingMossBlock.getTip(pos).below()` (`HangingMossBlock.java:110-120`).
fn hanging_moss_growth(level: &LevelCtx<'_, '_>, pos: BlockPos, state: i32) -> BlockPos {
    let block = block_type_of(state);
    let mut at = below(pos);
    while block.is_some() && block_type_of(level.block_state(at)) == block {
        at = below(at);
    }
    at
}
