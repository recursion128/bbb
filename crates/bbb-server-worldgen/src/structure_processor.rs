//! Structure processor runtime over the datapack-decoded processor types.

use std::sync::Arc;

use crate::random::WorldgenRandom;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::random_source::RandomSource;

pub use bbb_server_datapack::worldgen::processor::{
    Axis, PosRuleTest, ProcessorRule, RuleBlockEntityModifier, StructureProcessor,
};
pub use bbb_server_datapack::worldgen::Heightmap;
use bbb_server_datapack::worldgen::{Holder, ProcessorList, ProcessorListHolder, RuleTest};
use bbb_server_types::block_state_parse::parse_block_state_prefix;

use super::multiface::is_collision_shape_full_block_at;
use super::value_providers::IntProviderExt;
use super::{block_in_holder_set, block_is, block_set_property, rule_test_matches, WorldGenLevel};

/// `StructureTemplate.StructureBlockInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureBlockInfo {
    pub pos: BlockPos,
    pub state: i32,
    pub nbt: Option<Arc<NbtSummaryValue>>,
}

/// Resolves a processor-list holder: a registry id or an inline list.
pub fn resolve_processor_list<L: WorldGenLevel>(
    holder: &ProcessorListHolder,
    level: &L,
) -> Option<Arc<ProcessorList>> {
    match holder {
        Holder::Named(id) => level.processor_list(id),
        Holder::Inline(list) => Some(list.clone()),
    }
}

fn process_block<L: WorldGenLevel>(
    processor: &StructureProcessor,
    level: &L,
    reference_pos: BlockPos,
    original: &StructureBlockInfo,
    mut processed: StructureBlockInfo,
    settings_random: &mut dyn RandomSource,
) -> Option<Option<StructureBlockInfo>> {
    match processor {
        StructureProcessor::BlockIgnore { blocks } => Some(
            (!blocks
                .iter()
                .any(|state| same_block(*state, processed.state)))
            .then_some(processed),
        ),
        StructureProcessor::BlockRot {
            rottable_blocks,
            integrity,
        } => {
            let rottable = match rottable_blocks {
                None => true,
                Some(set) => block_in_holder_set(level, original.state, set),
            };
            Some((!rottable || settings_random.next_float() <= *integrity).then_some(processed))
        }
        StructureProcessor::Gravity { heightmap, offset } => {
            let height = gravity_height(level, *heightmap, processed.pos.x, processed.pos.z)
                .wrapping_add(*offset);
            processed.pos.y = height.wrapping_add(original.pos.y);
            Some(Some(processed))
        }
        StructureProcessor::JigsawReplacement => {
            if !block_is(processed.state, "minecraft:jigsaw") {
                return Some(Some(processed));
            }
            let Some(nbt) = &processed.nbt else {
                return Some(Some(processed));
            };
            let final_state = compound_string_or(nbt, "final_state", "minecraft:air");
            let Some(state) = parse_block_state_prefix(&final_state) else {
                return Some(None);
            };
            if block_is(state, "minecraft:structure_void") {
                return Some(None);
            }
            processed.state = state;
            processed.nbt = None;
            Some(Some(processed))
        }
        StructureProcessor::Rule { rules } => {
            let mut random = WorldgenRandom::with_seed(block_pos_seed(processed.pos));
            let location_state = level.get_block(processed.pos);
            for rule in rules {
                if rule_matches(
                    rule,
                    level,
                    original.pos,
                    processed.pos,
                    reference_pos,
                    processed.state,
                    location_state,
                    &mut random,
                ) {
                    processed.state = rule.output_state;
                    processed.nbt = apply_block_entity_modifier(
                        &rule.block_entity_modifier,
                        &mut random,
                        processed.nbt.as_deref(),
                    )?;
                    break;
                }
            }
            Some(Some(processed))
        }
        StructureProcessor::Nop | StructureProcessor::Capped { .. } => Some(Some(processed)),
        StructureProcessor::BlockAge { mossiness } => {
            if let Some(state) = age_state(level, processed.state, *mossiness, settings_random)? {
                processed.state = state;
            }
            Some(Some(processed))
        }
        StructureProcessor::BlackstoneReplace => {
            if let Some(target) = blackstone_replacement(processed.state) {
                let mut state = default_state(target);
                for property in ["facing", "half", "type"] {
                    if let Some(value) = bbb_facts::block_state_types::vanilla_block_state_property(
                        processed.state,
                        property,
                    ) {
                        state = block_set_property(state, property, value);
                    }
                }
                processed.state = state;
            }
            Some(Some(processed))
        }
        StructureProcessor::LavaSubmerged => {
            if block_is(level.get_block(processed.pos), "minecraft:lava")
                && !is_collision_shape_full_block_at(level, processed.pos, processed.state)
            {
                processed.state = default_state("minecraft:lava");
            }
            Some(Some(processed))
        }
        StructureProcessor::ProtectedBlocks { cannot_replace } => {
            let state = level.get_block(processed.pos);
            Some((!level.block_state_in_tag(state, cannot_replace)).then_some(processed))
        }
    }
}

fn finalize_processing<L: WorldGenLevel>(
    processor: &StructureProcessor,
    level: &L,
    position: BlockPos,
    reference_pos: BlockPos,
    originals: &[StructureBlockInfo],
    processed: &mut [StructureBlockInfo],
    settings_random: &mut StructureProcessorRandom<'_>,
) -> Option<()> {
    let StructureProcessor::Capped { delegate, limit } = processor else {
        return Some(());
    };
    if limit.max_inclusive() == 0 || processed.is_empty() || originals.len() != processed.len() {
        return Some(());
    }
    let mut random = WorldgenRandom::at(level.get_seed(), position.x, position.y, position.z);
    let max_to_replace = (limit.sample(&mut random) as usize).min(processed.len());
    if max_to_replace == 0 {
        return Some(());
    }
    let mut indices = (0..processed.len()).collect::<Vec<_>>();
    for size in (2..=indices.len()).rev() {
        let swap_to = random.next_int(size as i32) as usize;
        indices.swap(size - 1, swap_to);
    }
    let mut replaced = 0;
    for index in indices {
        if replaced >= max_to_replace {
            break;
        }
        let before = processed[index].clone();
        let after = settings_random.with_random(before.pos, |random| {
            process_block(
                delegate,
                level,
                reference_pos,
                &originals[index],
                before.clone(),
                random,
            )
        })?;
        if let Some(after) = after {
            if after != before {
                processed[index] = after;
                replaced += 1;
            }
        }
    }
    Some(())
}

/// `StructureTemplate.processBlockInfos` (`StructureTemplate.java:438-469`).
pub fn process_block_infos<L, F>(
    level: &L,
    position: BlockPos,
    reference_pos: BlockPos,
    processors: &[StructureProcessor],
    blocks: &[StructureBlockInfo],
    settings_random: &mut dyn RandomSource,
    calculate_relative_position: F,
) -> Option<Vec<StructureBlockInfo>>
where
    L: WorldGenLevel,
    F: Fn(BlockPos) -> BlockPos,
{
    process_block_infos_with_settings_random(
        level,
        position,
        reference_pos,
        processors,
        blocks,
        &mut StructureProcessorRandom::Shared(settings_random),
        calculate_relative_position,
    )
}

/// `StructurePlaceSettings.getRandom(pos)` when no shared random was installed
/// (`StructurePlaceSettings.java:110-115`): every processor invocation starts a
/// fresh legacy stream from the processed block position.
pub fn process_block_infos_with_positional_random<L, F>(
    level: &L,
    position: BlockPos,
    reference_pos: BlockPos,
    processors: &[StructureProcessor],
    blocks: &[StructureBlockInfo],
    calculate_relative_position: F,
) -> Option<Vec<StructureBlockInfo>>
where
    L: WorldGenLevel,
    F: Fn(BlockPos) -> BlockPos,
{
    process_block_infos_with_settings_random(
        level,
        position,
        reference_pos,
        processors,
        blocks,
        &mut StructureProcessorRandom::Positional,
        calculate_relative_position,
    )
}

enum StructureProcessorRandom<'a> {
    Shared(&'a mut dyn RandomSource),
    Positional,
}

impl StructureProcessorRandom<'_> {
    fn with_random<T>(
        &mut self,
        position: BlockPos,
        use_random: impl FnOnce(&mut dyn RandomSource) -> T,
    ) -> T {
        match self {
            Self::Shared(random) => use_random(*random),
            Self::Positional => {
                let mut random = WorldgenRandom::with_seed(block_pos_seed(position));
                use_random(&mut random)
            }
        }
    }
}

fn process_block_infos_with_settings_random<L, F>(
    level: &L,
    position: BlockPos,
    reference_pos: BlockPos,
    processors: &[StructureProcessor],
    blocks: &[StructureBlockInfo],
    settings_random: &mut StructureProcessorRandom<'_>,
    calculate_relative_position: F,
) -> Option<Vec<StructureBlockInfo>>
where
    L: WorldGenLevel,
    F: Fn(BlockPos) -> BlockPos,
{
    let needs_originals = processors
        .iter()
        .any(|processor| matches!(processor, StructureProcessor::Capped { .. }));
    let mut originals = if needs_originals {
        Vec::with_capacity(blocks.len())
    } else {
        Vec::new()
    };
    let mut processed = Vec::with_capacity(blocks.len());
    for original in blocks {
        let mut current = Some(StructureBlockInfo {
            pos: calculate_relative_position(original.pos),
            state: original.state,
            nbt: original.nbt.clone(),
        });
        for processor in processors {
            let Some(info) = current else {
                break;
            };
            current = settings_random.with_random(info.pos, |random| {
                process_block(processor, level, reference_pos, original, info, random)
            })?;
        }
        if let Some(info) = current {
            if needs_originals {
                originals.push(original.clone());
            }
            processed.push(info);
        }
    }
    if needs_originals {
        for processor in processors {
            finalize_processing(
                processor,
                level,
                position,
                reference_pos,
                &originals,
                &mut processed,
                settings_random,
            )?;
        }
    }
    Some(processed)
}

fn gravity_height<L: WorldGenLevel>(level: &L, heightmap: Heightmap, x: i32, z: i32) -> i32 {
    match heightmap {
        Heightmap::WorldSurfaceWg if level.is_server_level() => {
            level.get_world_surface_height(x, z)
        }
        Heightmap::WorldSurfaceWg => level.get_world_surface_wg_height(x, z),
        Heightmap::WorldSurface => level.get_world_surface_height(x, z),
        Heightmap::OceanFloorWg if level.is_server_level() => level.get_ocean_floor_height(x, z),
        Heightmap::OceanFloorWg => level.get_ocean_floor_wg_height(x, z),
        Heightmap::OceanFloor => level.get_ocean_floor_height(x, z),
        Heightmap::MotionBlocking => level.get_motion_blocking_height(x, z),
        Heightmap::MotionBlockingNoLeaves => level.get_motion_blocking_no_leaves_height(x, z),
    }
}

fn replace_rule(
    input_predicate: RuleTest,
    output_state: i32,
    block_entity_modifier: RuleBlockEntityModifier,
) -> ProcessorRule {
    ProcessorRule {
        input_predicate,
        location_predicate: RuleTest::AlwaysTrue,
        position_predicate: PosRuleTest::Always,
        output_state,
        block_entity_modifier,
    }
}

/// `OceanRuinPieces.archyRuleProcessor`.
pub fn block_match_replace_with_loot(
    input_block: &str,
    output_state: i32,
    loot_table: &str,
) -> ProcessorRule {
    replace_rule(
        RuleTest::BlockMatch(input_block.to_string()),
        output_state,
        RuleBlockEntityModifier::AppendLoot(loot_table.to_string()),
    )
}

/// `RuinedPortalPiece.getBlockReplaceRule(Block, Block)`.
pub fn block_match_replace(input_block: &str, output_state: i32) -> ProcessorRule {
    replace_rule(
        RuleTest::BlockMatch(input_block.to_string()),
        output_state,
        RuleBlockEntityModifier::Passthrough,
    )
}

/// `RuinedPortalPiece.getBlockReplaceRule(Block, float, Block)`.
pub fn random_block_match_replace(
    input_block: &str,
    probability: f32,
    output_state: i32,
) -> ProcessorRule {
    replace_rule(
        RuleTest::RandomBlockMatch {
            block: input_block.to_string(),
            probability,
        },
        output_state,
        RuleBlockEntityModifier::Passthrough,
    )
}

/// `ProcessorRule.test`.
#[allow(clippy::too_many_arguments)]
fn rule_matches<L: WorldGenLevel>(
    rule: &ProcessorRule,
    level: &L,
    template_pos: BlockPos,
    world_pos: BlockPos,
    reference_pos: BlockPos,
    input_state: i32,
    location_state: i32,
    random: &mut dyn RandomSource,
) -> bool {
    rule_test_matches(level, &rule.input_predicate, input_state, random)
        && rule_test_matches(level, &rule.location_predicate, location_state, random)
        && pos_rule_matches(
            &rule.position_predicate,
            template_pos,
            world_pos,
            reference_pos,
            random,
        )
}

/// `PosRuleTest.test` for the linear families.
fn pos_rule_matches(
    test: &PosRuleTest,
    _template_pos: BlockPos,
    world_pos: BlockPos,
    reference_pos: BlockPos,
    random: &mut dyn RandomSource,
) -> bool {
    let (chance, draw) = match *test {
        PosRuleTest::Always => return true,
        PosRuleTest::Linear {
            min_chance,
            max_chance,
            min_dist,
            max_dist,
        } => {
            let distance = world_pos
                .x
                .wrapping_sub(reference_pos.x)
                .wrapping_abs()
                .wrapping_add(world_pos.y.wrapping_sub(reference_pos.y).wrapping_abs())
                .wrapping_add(world_pos.z.wrapping_sub(reference_pos.z).wrapping_abs());
            (
                clamped_linear_chance(distance, min_dist, max_dist, min_chance, max_chance),
                random.next_float(),
            )
        }
        PosRuleTest::AxisAlignedLinear {
            min_chance,
            max_chance,
            min_dist,
            max_dist,
            axis,
        } => {
            let distance = match axis {
                Axis::X => world_pos.x.wrapping_sub(reference_pos.x).wrapping_abs(),
                Axis::Y => world_pos.y.wrapping_sub(reference_pos.y).wrapping_abs(),
                Axis::Z => world_pos.z.wrapping_sub(reference_pos.z).wrapping_abs(),
            };
            (
                clamped_linear_chance(distance, min_dist, max_dist, min_chance, max_chance),
                random.next_float(),
            )
        }
    };
    draw <= chance
}

fn apply_block_entity_modifier(
    modifier: &RuleBlockEntityModifier,
    random: &mut dyn RandomSource,
    existing: Option<&NbtSummaryValue>,
) -> Option<Option<Arc<NbtSummaryValue>>> {
    match modifier {
        RuleBlockEntityModifier::Clear => {
            Some(Some(Arc::new(NbtSummaryValue::Compound(Vec::new()))))
        }
        RuleBlockEntityModifier::Passthrough => Some(existing.cloned().map(Arc::new)),
        RuleBlockEntityModifier::AppendStatic(data) => {
            let merged = match existing {
                Some(existing) => merge_compounds(existing, data)?,
                None => data.clone(),
            };
            Some(Some(Arc::new(merged)))
        }
        RuleBlockEntityModifier::AppendLoot(loot_table) => {
            let mut value = match existing {
                Some(NbtSummaryValue::Compound(fields)) => fields.clone(),
                Some(_) => return None,
                None => Vec::new(),
            };
            put_compound(
                &mut value,
                "LootTable",
                NbtSummaryValue::string(loot_table.clone()),
            );
            put_compound(
                &mut value,
                "LootTableSeed",
                NbtSummaryValue::Long(random.next_long()),
            );
            Some(Some(Arc::new(NbtSummaryValue::Compound(value))))
        }
    }
}

fn clamped_linear_chance(
    distance: i32,
    min_dist: i32,
    max_dist: i32,
    min_chance: f32,
    max_chance: f32,
) -> f32 {
    let factor =
        ((distance as f32 - min_dist as f32) / (max_dist as f32 - min_dist as f32)).clamp(0.0, 1.0);
    min_chance + factor * (max_chance - min_chance)
}

fn same_block(left: i32, right: i32) -> bool {
    matches!(
        (
            vanilla_block_state_block_type_id(left),
            vanilla_block_state_block_type_id(right)
        ),
        (Some(left), Some(right)) if left == right
    )
}

fn default_state(resource_id: &'static str) -> i32 {
    crate::catalog_default_state(resource_id)
}

fn with_properties_of(target: &'static str, source: i32) -> i32 {
    let mut state = default_state(target);
    for (name, value) in bbb_facts::block_state_types::vanilla_block_state_properties(source) {
        state = block_set_property(state, name, value);
    }
    state
}

fn random_stairs(random: &mut dyn RandomSource, block: &'static str) -> i32 {
    // `BlockAgeProcessor.java:97-100`; `Direction.Plane.HORIZONTAL` is
    // north, east, south, west and `Half.values()` is top, bottom.
    let facing = ["north", "east", "south", "west"][random.next_int(4) as usize];
    let half = ["top", "bottom"][random.next_int(2) as usize];
    let state = block_set_property(default_state(block), "facing", facing);
    block_set_property(state, "half", half)
}

fn age_state<L: WorldGenLevel>(
    level: &L,
    state: i32,
    mossiness: f32,
    random: &mut dyn RandomSource,
) -> Option<Option<i32>> {
    if block_is(state, "minecraft:stone_bricks")
        || block_is(state, "minecraft:stone")
        || block_is(state, "minecraft:chiseled_stone_bricks")
    {
        if random.next_float() >= 0.5 {
            return Some(None);
        }
        let non_mossy = [
            default_state("minecraft:cracked_stone_bricks"),
            random_stairs(random, "minecraft:stone_brick_stairs"),
        ];
        let mossy = [
            default_state("minecraft:mossy_stone_bricks"),
            random_stairs(random, "minecraft:mossy_stone_brick_stairs"),
        ];
        let choices = if random.next_float() < mossiness {
            mossy
        } else {
            non_mossy
        };
        return Some(Some(choices[random.next_int(2) as usize]));
    }
    if level.block_state_in_tag(state, "minecraft:stairs") {
        if random.next_float() >= 0.5 {
            return Some(None);
        }
        let non_mossy = [
            default_state("minecraft:stone_slab"),
            default_state("minecraft:stone_brick_slab"),
        ];
        let mossy = [
            with_properties_of("minecraft:mossy_stone_brick_stairs", state),
            default_state("minecraft:mossy_stone_brick_slab"),
        ];
        let choices = if random.next_float() < mossiness {
            mossy
        } else {
            non_mossy
        };
        return Some(Some(choices[random.next_int(2) as usize]));
    }
    if level.block_state_in_tag(state, "minecraft:slabs") {
        return Some(
            (random.next_float() < mossiness)
                .then(|| with_properties_of("minecraft:mossy_stone_brick_slab", state)),
        );
    }
    if level.block_state_in_tag(state, "minecraft:walls") {
        return Some(
            (random.next_float() < mossiness)
                .then(|| with_properties_of("minecraft:mossy_stone_brick_wall", state)),
        );
    }
    if block_is(state, "minecraft:obsidian") {
        return Some(
            (random.next_float() < 0.15).then(|| default_state("minecraft:crying_obsidian")),
        );
    }
    Some(None)
}

fn blackstone_replacement(state: i32) -> Option<&'static str> {
    // `BlackstoneReplaceProcessor.java:18-42`.
    const REPLACEMENTS: [(&str, &str); 23] = [
        ("minecraft:cobblestone", "minecraft:blackstone"),
        ("minecraft:mossy_cobblestone", "minecraft:blackstone"),
        ("minecraft:stone", "minecraft:polished_blackstone"),
        (
            "minecraft:stone_bricks",
            "minecraft:polished_blackstone_bricks",
        ),
        (
            "minecraft:mossy_stone_bricks",
            "minecraft:polished_blackstone_bricks",
        ),
        (
            "minecraft:cobblestone_stairs",
            "minecraft:blackstone_stairs",
        ),
        (
            "minecraft:mossy_cobblestone_stairs",
            "minecraft:blackstone_stairs",
        ),
        (
            "minecraft:stone_stairs",
            "minecraft:polished_blackstone_stairs",
        ),
        (
            "minecraft:stone_brick_stairs",
            "minecraft:polished_blackstone_brick_stairs",
        ),
        (
            "minecraft:mossy_stone_brick_stairs",
            "minecraft:polished_blackstone_brick_stairs",
        ),
        ("minecraft:cobblestone_slab", "minecraft:blackstone_slab"),
        (
            "minecraft:mossy_cobblestone_slab",
            "minecraft:blackstone_slab",
        ),
        (
            "minecraft:smooth_stone_slab",
            "minecraft:polished_blackstone_slab",
        ),
        ("minecraft:stone_slab", "minecraft:polished_blackstone_slab"),
        (
            "minecraft:stone_brick_slab",
            "minecraft:polished_blackstone_brick_slab",
        ),
        (
            "minecraft:mossy_stone_brick_slab",
            "minecraft:polished_blackstone_brick_slab",
        ),
        (
            "minecraft:stone_brick_wall",
            "minecraft:polished_blackstone_brick_wall",
        ),
        (
            "minecraft:mossy_stone_brick_wall",
            "minecraft:polished_blackstone_brick_wall",
        ),
        ("minecraft:cobblestone_wall", "minecraft:blackstone_wall"),
        (
            "minecraft:mossy_cobblestone_wall",
            "minecraft:blackstone_wall",
        ),
        (
            "minecraft:chiseled_stone_bricks",
            "minecraft:chiseled_polished_blackstone",
        ),
        (
            "minecraft:cracked_stone_bricks",
            "minecraft:cracked_polished_blackstone_bricks",
        ),
        ("minecraft:iron_bars", "minecraft:iron_chain"),
    ];
    REPLACEMENTS
        .iter()
        .find_map(|(from, to)| block_is(state, from).then_some(*to))
}

fn compound_string_or(value: &NbtSummaryValue, name: &str, default: &str) -> String {
    let NbtSummaryValue::Compound(fields) = value else {
        return default.into();
    };
    fields
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| match &entry.value {
            NbtSummaryValue::String(value) => {
                value.try_to_string().unwrap_or_else(|_| default.into())
            }
            _ => default.into(),
        })
        .unwrap_or_else(|| default.into())
}

fn merge_compounds(existing: &NbtSummaryValue, added: &NbtSummaryValue) -> Option<NbtSummaryValue> {
    let (NbtSummaryValue::Compound(existing), NbtSummaryValue::Compound(added)) = (existing, added)
    else {
        return None;
    };
    let mut merged = existing.clone();
    for entry in added {
        if let Some(old) = merged.iter_mut().find(|old| old.name == entry.name) {
            old.value = match (&old.value, &entry.value) {
                (NbtSummaryValue::Compound(_), NbtSummaryValue::Compound(_)) => {
                    merge_compounds(&old.value, &entry.value)?
                }
                _ => entry.value.clone(),
            };
        } else {
            merged.push(entry.clone());
        }
    }
    Some(NbtSummaryValue::Compound(merged))
}

fn put_compound(fields: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(entry) = fields.iter_mut().find(|entry| entry.name == name) {
        entry.value = value;
    } else {
        fields.push(NbtSummaryEntry {
            name: name.into(),
            value,
        });
    }
}

pub fn block_pos_seed(pos: BlockPos) -> i64 {
    crate::random::position_seed(pos.x, pos.y, pos.z)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::block_state_types::{
        vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
    };

    use super::*;

    #[derive(Default)]
    struct TestLevel {
        blocks: BTreeMap<(i32, i32, i32), i32>,
        default_block: i32,
        seed: i64,
        world_surface_wg: i32,
        world_surface: i32,
        server_level: bool,
    }

    impl WorldGenLevel for TestLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
            crate::tests::builtin_registry_tag_contains(registry_id, tag_id, element_id)
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.blocks
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(self.default_block)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            self.blocks.insert((pos.x, pos.y, pos.z), state);
            true
        }

        fn get_min_y(&self) -> i32 {
            -64
        }

        fn get_max_y(&self) -> i32 {
            320
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

        fn get_world_surface_wg_height(&self, _x: i32, _z: i32) -> i32 {
            self.world_surface_wg
        }

        fn get_world_surface_height(&self, _x: i32, _z: i32) -> i32 {
            self.world_surface
        }

        fn is_server_level(&self) -> bool {
            self.server_level
        }

        fn get_seed(&self) -> i64 {
            self.seed
        }
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn string(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::string(value)
    }

    fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
        let properties = properties
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        vanilla_block_state_id_by_name_and_properties(name, &properties).expect("state exists")
    }

    fn info(pos: BlockPos, state: i32) -> StructureBlockInfo {
        StructureBlockInfo {
            pos,
            state,
            nbt: None,
        }
    }

    fn run(
        level: &TestLevel,
        processors: &[StructureProcessor],
        blocks: &[StructureBlockInfo],
        seed: i64,
    ) -> Option<Vec<StructureBlockInfo>> {
        process_block_infos(
            level,
            BlockPos { x: 0, y: 0, z: 0 },
            BlockPos { x: 0, y: 0, z: 0 },
            processors,
            blocks,
            &mut WorldgenRandom::with_seed(seed),
            |pos| pos,
        )
    }

    #[test]
    fn position_tests_and_block_entity_modifiers_cover_every_family() {
        let linear = PosRuleTest::Linear {
            min_chance: 1.0,
            max_chance: 0.0,
            min_dist: 0,
            max_dist: 10,
        };
        let axis = PosRuleTest::AxisAlignedLinear {
            min_chance: 1.0,
            max_chance: 0.0,
            min_dist: 0,
            max_dist: 10,
            axis: Axis::X,
        };
        let zero = BlockPos { x: 0, y: 0, z: 0 };
        assert!(pos_rule_matches(
            &linear,
            zero,
            zero,
            zero,
            &mut WorldgenRandom::with_seed(1)
        ));
        assert!(!pos_rule_matches(
            &axis,
            zero,
            BlockPos { x: 10, y: 0, z: 0 },
            zero,
            &mut WorldgenRandom::with_seed(1)
        ));

        let existing = compound(vec![
            entry("keep", NbtSummaryValue::Int(1)),
            entry(
                "nested",
                compound(vec![entry("left", NbtSummaryValue::Int(2))]),
            ),
        ]);
        let clear = RuleBlockEntityModifier::Clear;
        let passthrough = RuleBlockEntityModifier::Passthrough;
        let append = RuleBlockEntityModifier::AppendStatic(compound(vec![
            entry("added", NbtSummaryValue::Int(3)),
            entry(
                "nested",
                compound(vec![entry("right", NbtSummaryValue::Int(4))]),
            ),
        ]));
        let loot = RuleBlockEntityModifier::AppendLoot("minecraft:chests/test".to_string());
        let mut random = WorldgenRandom::with_seed(42);
        assert_eq!(
            apply_block_entity_modifier(&clear, &mut random, Some(&existing)).unwrap(),
            Some(Arc::new(compound(Vec::new())))
        );
        assert_eq!(
            apply_block_entity_modifier(&passthrough, &mut random, Some(&existing)).unwrap(),
            Some(Arc::new(existing.clone()))
        );
        let appended = apply_block_entity_modifier(&append, &mut random, Some(&existing))
            .unwrap()
            .unwrap();
        let NbtSummaryValue::Compound(appended) = appended.as_ref() else {
            panic!("append_static result")
        };
        assert_eq!(appended.len(), 3);
        let NbtSummaryValue::Compound(nested) = &appended[1].value else {
            panic!("nested append_static result")
        };
        assert_eq!(
            nested,
            &vec![
                entry("left", NbtSummaryValue::Int(2)),
                entry("right", NbtSummaryValue::Int(4))
            ]
        );
        let looted = apply_block_entity_modifier(&loot, &mut random, None)
            .unwrap()
            .unwrap();
        let NbtSummaryValue::Compound(looted) = looted.as_ref() else {
            panic!("append_loot result")
        };
        assert_eq!(
            looted[0],
            entry("LootTable", string("minecraft:chests/test"))
        );
        assert_eq!(looted[1].name, "LootTableSeed");
        assert!(matches!(looted[1].value, NbtSummaryValue::Long(_)));
    }

    #[test]
    fn rule_processor_wires_position_test_and_modifier_into_first_match() {
        let processor = StructureProcessor::Rule {
            rules: vec![ProcessorRule {
                input_predicate: RuleTest::AlwaysTrue,
                location_predicate: RuleTest::AlwaysTrue,
                position_predicate: PosRuleTest::Linear {
                    min_chance: 1.0,
                    max_chance: 0.0,
                    min_dist: 0,
                    max_dist: 10,
                },
                output_state: state("minecraft:polished_blackstone", &[]),
                block_entity_modifier: RuleBlockEntityModifier::AppendStatic(compound(vec![
                    entry("processed", NbtSummaryValue::Byte(1)),
                ])),
            }],
        };
        let original = StructureBlockInfo {
            pos: BlockPos { x: 0, y: 0, z: 0 },
            state: state("minecraft:stone", &[]),
            nbt: Some(Arc::new(compound(vec![entry(
                "original",
                NbtSummaryValue::Byte(1),
            )]))),
        };
        let result = run(&TestLevel::default(), &[processor], &[original], 1).unwrap();
        assert!(block_is(result[0].state, "minecraft:polished_blackstone"));
        let NbtSummaryValue::Compound(nbt) = result[0].nbt.as_deref().unwrap() else {
            panic!("rule modifier nbt")
        };
        assert_eq!(
            nbt,
            &vec![
                entry("original", NbtSummaryValue::Byte(1)),
                entry("processed", NbtSummaryValue::Byte(1))
            ]
        );
    }

    #[test]
    fn jigsaw_replacement_parses_state_prefix_and_consumes_nbt() {
        let jigsaw = state("minecraft:jigsaw", &[("orientation", "north_up")]);
        let original = StructureBlockInfo {
            pos: BlockPos { x: 1, y: 2, z: 3 },
            state: jigsaw,
            nbt: Some(Arc::new(compound(vec![entry(
                "final_state",
                string("minecraft:oak_log[axis=x]{ignored:1}"),
            )]))),
        };
        let result = run(
            &TestLevel::default(),
            &[StructureProcessor::JigsawReplacement],
            &[original],
            1,
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].state,
            state("minecraft:oak_log", &[("axis", "x")])
        );
        assert_eq!(result[0].nbt, None);

        let invalid = StructureBlockInfo {
            pos: BlockPos { x: 0, y: 0, z: 0 },
            state: jigsaw,
            nbt: Some(Arc::new(compound(vec![entry(
                "final_state",
                string("minecraft:not_a_block"),
            )]))),
        };
        assert!(run(
            &TestLevel::default(),
            &[StructureProcessor::JigsawReplacement],
            &[invalid],
            1
        )
        .unwrap()
        .is_empty());
        let malformed_nbt = StructureBlockInfo {
            pos: BlockPos { x: 0, y: 0, z: 0 },
            state: jigsaw,
            nbt: Some(Arc::new(compound(vec![entry(
                "final_state",
                string("minecraft:stone{unterminated:"),
            )]))),
        };
        assert!(run(
            &TestLevel::default(),
            &[StructureProcessor::JigsawReplacement],
            &[malformed_nbt],
            1
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn processors_apply_gravity_tags_shapes_age_and_properties() {
        let stone = state("minecraft:stone", &[]);
        let mut level = TestLevel {
            default_block: stone,
            world_surface_wg: 70,
            ..TestLevel::default()
        };
        let gravity = StructureProcessor::Gravity {
            heightmap: Heightmap::WorldSurfaceWg,
            offset: 2,
        };
        let result = process_block_infos(
            &level,
            BlockPos { x: 0, y: 0, z: 0 },
            BlockPos { x: 0, y: 0, z: 0 },
            &[gravity],
            &[info(BlockPos { x: 4, y: 3, z: 5 }, stone)],
            &mut WorldgenRandom::with_seed(1),
            |pos| BlockPos {
                x: pos.x + 10,
                y: pos.y,
                z: pos.z + 20,
            },
        )
        .unwrap();
        assert_eq!(
            result[0].pos,
            BlockPos {
                x: 14,
                y: 75,
                z: 25
            }
        );
        level.server_level = true;
        level.world_surface = 81;
        assert_eq!(gravity_height(&level, Heightmap::WorldSurfaceWg, 4, 5), 81);
        level.server_level = false;

        assert!(run(
            &level,
            &[StructureProcessor::BlockIgnore {
                blocks: vec![stone]
            }],
            &[info(BlockPos { x: 0, y: 0, z: 0 }, stone)],
            1
        )
        .unwrap()
        .is_empty());
        assert_eq!(
            run(
                &level,
                &[StructureProcessor::BlockRot {
                    rottable_blocks: None,
                    integrity: 1.0,
                }],
                &[info(BlockPos { x: 0, y: 0, z: 0 }, stone)],
                1
            )
            .unwrap()
            .len(),
            1
        );
        assert!(run(
            &level,
            &[StructureProcessor::BlockRot {
                rottable_blocks: None,
                integrity: 0.0,
            }],
            &[info(BlockPos { x: 0, y: 0, z: 0 }, stone)],
            1
        )
        .unwrap()
        .is_empty());

        level.default_block = state("minecraft:bedrock", &[]);
        assert!(run(
            &level,
            &[StructureProcessor::ProtectedBlocks {
                cannot_replace: "minecraft:features_cannot_replace".to_string(),
            }],
            &[info(BlockPos { x: 0, y: 0, z: 0 }, stone)],
            1
        )
        .unwrap()
        .is_empty());

        let stair = state(
            "minecraft:stone_brick_stairs",
            &[
                ("facing", "west"),
                ("half", "top"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ],
        );
        let replaced = run(
            &level,
            &[StructureProcessor::BlackstoneReplace],
            &[info(BlockPos { x: 0, y: 0, z: 0 }, stair)],
            1,
        )
        .unwrap()[0]
            .state;
        assert!(block_is(
            replaced,
            "minecraft:polished_blackstone_brick_stairs"
        ));
        let identity = vanilla_block_state_identity(replaced).unwrap();
        assert_eq!(identity.properties["facing"], "west");
        assert_eq!(identity.properties["half"], "top");

        let slab = state(
            "minecraft:stone_brick_slab",
            &[("type", "top"), ("waterlogged", "false")],
        );
        let aged = run(
            &level,
            &[StructureProcessor::BlockAge { mossiness: 1.0 }],
            &[info(BlockPos { x: 0, y: 0, z: 0 }, slab)],
            1,
        )
        .unwrap()[0]
            .state;
        assert!(block_is(aged, "minecraft:mossy_stone_brick_slab"));
        assert_eq!(
            vanilla_block_state_identity(aged).unwrap().properties["type"],
            "top"
        );
    }

    #[test]
    fn capped_finalize_changes_only_its_sampled_limit() {
        let stone = state("minecraft:stone", &[]);
        let level = TestLevel {
            default_block: stone,
            seed: 42,
            ..TestLevel::default()
        };
        let blocks = (0..5)
            .map(|x| info(BlockPos { x, y: 0, z: 0 }, stone))
            .collect::<Vec<_>>();
        let result = run(
            &level,
            &[StructureProcessor::Capped {
                delegate: Box::new(StructureProcessor::BlackstoneReplace),
                limit: bbb_server_datapack::worldgen::IntProvider::Constant(2),
            }],
            &blocks,
            99,
        )
        .unwrap();
        assert_eq!(
            result
                .iter()
                .filter(|info| block_is(info.state, "minecraft:polished_blackstone"))
                .count(),
            2
        );
        assert_eq!(
            result
                .iter()
                .filter(|info| block_is(info.state, "minecraft:polished_blackstone"))
                .map(|info| info.pos.x)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }
}
