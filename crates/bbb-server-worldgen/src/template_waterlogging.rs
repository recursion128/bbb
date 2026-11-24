use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_name,
};
use bbb_protocol::block_types::{vanilla_block_type, VanillaLiquidBlockContainerOverride};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily, FluidState};

use crate::fast_hash::FastHashSet;
use crate::{block_set_property, offset_block_pos, WorldGenLevel};

const LIQUID_UPDATE_FLAGS: i32 = 3;
const WATER_TICK_DELAY: i32 = 5;

#[derive(Debug)]
enum PreparedFluid {
    None,
    Locked,
    Container { waterlogged_state: Option<i32> },
}

#[derive(Debug)]
pub(crate) struct PreparedTemplateWaterlogging {
    pos: BlockPos,
    fluid: PreparedFluid,
}

#[derive(Debug, Default)]
pub(crate) struct StructureTemplateWaterlogging {
    to_fill: Vec<BlockPos>,
    locked_fluids: FastHashSet<BlockPos>,
}

impl StructureTemplateWaterlogging {
    pub(crate) fn prepare(
        &self,
        pos: BlockPos,
        state: i32,
        apply: bool,
    ) -> PreparedTemplateWaterlogging {
        let fluid = if !apply {
            PreparedFluid::None
        } else if block_state_fluid_state(state).is_source() {
            PreparedFluid::Locked
        } else if let Some(owner) = liquid_block_container(state) {
            PreparedFluid::Container {
                waterlogged_state: place_water_state(state, owner),
            }
        } else {
            PreparedFluid::None
        };
        PreparedTemplateWaterlogging { pos, fluid }
    }

    // `StructureTemplate.placeInWorld` captures fluid immediately before the write,
    // then locks sources or queues prior non-sources after success
    // (`StructureTemplate.java:281-320`).
    pub(crate) fn capture_previous<L: WorldGenLevel>(
        &self,
        level: &L,
        prepared: &PreparedTemplateWaterlogging,
    ) -> Option<FluidState> {
        match &prepared.fluid {
            PreparedFluid::Container { .. } => {
                Some(block_state_fluid_state(level.get_block(prepared.pos)))
            }
            PreparedFluid::None | PreparedFluid::Locked => None,
        }
    }

    pub(crate) fn record_success<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        prepared: PreparedTemplateWaterlogging,
        previous: Option<FluidState>,
    ) {
        match prepared.fluid {
            PreparedFluid::None => {}
            PreparedFluid::Locked => {
                self.locked_fluids.insert(prepared.pos);
            }
            PreparedFluid::Container { waterlogged_state } => {
                let previous = previous.expect("container placement captured its prior fluid");
                if previous.family() == Some(FluidFamily::Water) && previous.is_source() {
                    if let Some(waterlogged_state) = waterlogged_state {
                        let _ =
                            level.set_block(prepared.pos, waterlogged_state, LIQUID_UPDATE_FLAGS);
                        level.schedule_fluid_tick(prepared.pos, WATER_TICK_DELAY);
                    }
                }
                if !previous.is_source() {
                    self.to_fill.push(prepared.pos);
                }
            }
        }
    }

    pub(crate) fn refill<L: WorldGenLevel>(&mut self, level: &mut L) {
        // `StructureTemplate.placeInWorld` excludes DOWN and keeps this exact order
        // while repeatedly walking `toFill` in insertion order (`StructureTemplate.java:325-354`).
        const DIRECTIONS: [Direction; 5] = [
            Direction::Up,
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ];
        let mut filled = true;
        while filled && !self.to_fill.is_empty() {
            filled = false;
            let mut write = 0;
            for read in 0..self.to_fill.len() {
                let pos = self.to_fill[read];
                let mut source = block_state_fluid_state(level.get_block(pos));
                for direction in DIRECTIONS {
                    if source.is_source() {
                        break;
                    }
                    let neighbour_pos = offset_block_pos(pos, direction);
                    let neighbour = block_state_fluid_state(level.get_block(neighbour_pos));
                    if neighbour.is_source() && !self.locked_fluids.contains(&neighbour_pos) {
                        source = neighbour;
                    }
                }
                if source.is_source() {
                    let state = level.get_block(pos);
                    if let Some(owner) = liquid_block_container(state) {
                        if source.family() == Some(FluidFamily::Water) {
                            if let Some(waterlogged) = place_water_state(state, owner) {
                                let _ = level.set_block(pos, waterlogged, LIQUID_UPDATE_FLAGS);
                                level.schedule_fluid_tick(pos, WATER_TICK_DELAY);
                            }
                        }
                        filled = true;
                        continue;
                    }
                }
                self.to_fill[write] = pos;
                write += 1;
            }
            self.to_fill.truncate(write);
        }
    }
}

fn liquid_block_container(state: i32) -> Option<VanillaLiquidBlockContainerOverride> {
    vanilla_block_state_block_type_id(state)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.liquid_block_container_override)
}

fn place_water_state(state: i32, owner: VanillaLiquidBlockContainerOverride) -> Option<i32> {
    // `SimpleWaterloggedBlock.placeLiquid`, the slab guard, and the two lit-state
    // overrides define the observable state write (`SimpleWaterloggedBlock.java:25-36`,
    // `SlabBlock.java:106-108`, `CandleBlock.java:150-164`, `CampfireBlock.java:203-220`).
    match owner {
        VanillaLiquidBlockContainerOverride::SimpleWaterloggedBlock => {}
        VanillaLiquidBlockContainerOverride::SlabBlock if !is_double_slab(state) => {}
        VanillaLiquidBlockContainerOverride::BarrierBlock
        | VanillaLiquidBlockContainerOverride::KelpBlock
        | VanillaLiquidBlockContainerOverride::KelpPlantBlock
        | VanillaLiquidBlockContainerOverride::SeagrassBlock
        | VanillaLiquidBlockContainerOverride::SlabBlock
        | VanillaLiquidBlockContainerOverride::TallSeagrassBlock => return None,
    }
    let waterlogged = block_set_property(state, "waterlogged", "true");
    if waterlogged == state {
        return None;
    }
    if extinguishes_when_waterlogged(state) {
        return Some(block_set_property(waterlogged, "lit", "false"));
    }
    // DriedGhast uses the same state mutation; its placement sound is not a
    // worldgen state owner (`DriedGhastBlock.java:181-192`).
    Some(waterlogged)
}

fn is_double_slab(state: i32) -> bool {
    bbb_facts::block_state_types::vanilla_block_state_property(state, "type") == Some("double")
}

fn extinguishes_when_waterlogged(state: i32) -> bool {
    vanilla_block_state_name(state).is_some_and(|name| {
        name == "minecraft:campfire"
            || name == "minecraft:soul_campfire"
            || name == "minecraft:candle"
            || name.ends_with("_candle")
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::block_state_types::{
        vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
    };

    use super::*;
    use crate::test_support::SeamLevel;
    use crate::tests::RecordingLevel;

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn state(block: &'static str) -> i32 {
        crate::catalog_default_state(block)
    }

    fn dry_stairs() -> i32 {
        block_set_property(state("minecraft:oak_stairs"), "waterlogged", "false")
    }

    fn property(state: i32, name: &str) -> String {
        vanilla_block_state_identity(state)
            .and_then(|identity| identity.properties.get(name))
            .cloned()
            .unwrap()
    }

    fn flowing_water() -> i32 {
        vanilla_block_state_id_by_name_and_properties(
            "minecraft:water",
            &BTreeMap::from([("level".to_string(), "1".to_string())]),
        )
        .unwrap()
    }

    fn place_candidate(
        session: &mut StructureTemplateWaterlogging,
        level: &mut SeamLevel,
        at: BlockPos,
        placed: i32,
    ) {
        let prepared = session.prepare(at, placed, true);
        let previous = session.capture_previous(level, &prepared);
        assert!(level.set_block(at, placed, 2));
        session.record_success(level, prepared, previous);
    }

    #[test]
    fn locked_sources_do_not_refill_and_down_is_not_searched() {
        let mut level = SeamLevel::wrap(RecordingLevel::filled(state("minecraft:air")));
        let candidate = pos(0, 64, 0);
        let locked = pos(0, 65, 0);
        let below = pos(0, 63, 0);
        let mut session = StructureTemplateWaterlogging::default();
        place_candidate(&mut session, &mut level, locked, state("minecraft:water"));
        place_candidate(&mut session, &mut level, candidate, dry_stairs());
        level.set_cell(below, state("minecraft:water"));

        session.refill(&mut level);

        assert!(!block_state_fluid_state(level.get_block(candidate)).is_source());
        assert_eq!(session.to_fill, vec![candidate]);
    }

    #[test]
    fn first_source_direction_and_multiple_passes_match_vanilla() {
        let mut direction_level = SeamLevel::wrap(RecordingLevel::filled(state("minecraft:air")));
        let candidate = pos(0, 64, 0);
        direction_level.set_cell(pos(0, 65, 0), state("minecraft:lava"));
        direction_level.set_cell(pos(0, 64, -1), state("minecraft:water"));
        let mut direction_session = StructureTemplateWaterlogging::default();
        place_candidate(
            &mut direction_session,
            &mut direction_level,
            candidate,
            dry_stairs(),
        );
        direction_session.refill(&mut direction_level);
        assert!(!block_state_fluid_state(direction_level.get_block(candidate)).is_source());
        assert!(direction_session.to_fill.is_empty());

        let mut chain_level = SeamLevel::wrap(RecordingLevel::filled(state("minecraft:air")));
        let first = pos(0, 64, 0);
        let second = pos(-1, 64, 0);
        chain_level.set_cell(pos(1, 64, 0), state("minecraft:water"));
        let mut chain_session = StructureTemplateWaterlogging::default();
        place_candidate(&mut chain_session, &mut chain_level, second, dry_stairs());
        place_candidate(&mut chain_session, &mut chain_level, first, dry_stairs());
        chain_session.refill(&mut chain_level);
        assert!(block_state_fluid_state(chain_level.get_block(first)).is_source());
        assert!(block_state_fluid_state(chain_level.get_block(second)).is_source());
        assert!(chain_session.to_fill.is_empty());
    }

    #[test]
    fn flowing_previous_fluid_is_restored_from_an_adjacent_source() {
        let mut level = SeamLevel::wrap(RecordingLevel::filled(state("minecraft:air")));
        let candidate = pos(0, 64, 0);
        level.set_cell(candidate, flowing_water());
        level.set_cell(pos(0, 64, -1), state("minecraft:water"));
        let mut session = StructureTemplateWaterlogging::default();

        place_candidate(&mut session, &mut level, candidate, dry_stairs());
        session.refill(&mut level);

        assert!(block_state_fluid_state(level.get_block(candidate)).is_source());
        assert!(session.to_fill.is_empty());
    }

    #[test]
    fn actual_container_overrides_reject_double_slab_and_extinguish_lit_blocks() {
        let mut level = SeamLevel::wrap(RecordingLevel::filled(state("minecraft:air")));
        let source = state("minecraft:water");
        let mut session = StructureTemplateWaterlogging::default();

        let double_slab = block_set_property(
            block_set_property(state("minecraft:oak_slab"), "waterlogged", "false"),
            "type",
            "double",
        );
        let slab_pos = pos(0, 64, 0);
        level.set_cell(slab_pos, source);
        place_candidate(&mut session, &mut level, slab_pos, double_slab);
        assert_eq!(level.get_block(slab_pos), double_slab);

        let barrier_pos = pos(1, 64, 0);
        level.set_cell(barrier_pos, source);
        place_candidate(
            &mut session,
            &mut level,
            barrier_pos,
            state("minecraft:barrier"),
        );
        assert_eq!(level.get_block(barrier_pos), state("minecraft:barrier"));

        for (index, block) in ["minecraft:candle", "minecraft:campfire"]
            .iter()
            .enumerate()
        {
            let at = pos(index as i32 + 2, 64, 0);
            let lit = block_set_property(state(block), "lit", "true");
            level.set_cell(at, source);
            place_candidate(&mut session, &mut level, at, lit);
            let placed = level.get_block(at);
            assert_eq!(property(placed, "waterlogged"), "true");
            assert_eq!(property(placed, "lit"), "false");
        }

        let dried_ghast_pos = pos(4, 64, 0);
        level.set_cell(dried_ghast_pos, source);
        place_candidate(
            &mut session,
            &mut level,
            dried_ghast_pos,
            state("minecraft:dried_ghast"),
        );
        assert_eq!(
            property(level.get_block(dried_ghast_pos), "waterlogged"),
            "true"
        );
    }
}
