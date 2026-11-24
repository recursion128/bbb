use std::collections::BTreeMap;

use bbb_protocol::packets::BlockPos;

use crate::noise::biome::NoiseBiomeSource;
use crate::noise::ocean_monument::graph::regenerate_monument_children;
use crate::noise::ocean_monument::{
    required_surrounding_biomes, OceanMonumentRoomKind, OceanMonumentStructureStart,
    RegeneratedMonumentChildren,
};
use crate::noise::structures::{NoiseStructurePlan, StructureHeightmap};
use crate::{catalog_default_state, WorldGenLevel};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

pub struct PlacementLevel {
    pub cells: BTreeMap<(i32, i32, i32), i32>,
    pub proto_entities: Vec<(String, [f64; 3])>,
    air: i32,
    seed: i64,
}

impl PlacementLevel {
    pub fn new(seed: i64) -> Self {
        Self {
            cells: BTreeMap::new(),
            proto_entities: Vec::new(),
            air: catalog_default_state("minecraft:air"),
            seed,
        }
    }

    pub fn get_block(&self, pos: BlockPos) -> i32 {
        WorldGenLevel::get_block(self, pos)
    }
}

impl WorldGenLevel for PlacementLevel {
    fn ensure_can_write(&self, _origin: BlockPos) -> bool {
        true
    }

    fn get_block(&self, pos: BlockPos) -> i32 {
        self.cells
            .get(&(pos.x, pos.y, pos.z))
            .copied()
            .unwrap_or(self.air)
    }

    fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
        if state == self.air {
            self.cells.remove(&(pos.x, pos.y, pos.z));
        } else {
            self.cells.insert((pos.x, pos.y, pos.z), state);
        }
        true
    }

    fn get_min_y(&self) -> i32 {
        -64
    }

    fn get_max_y(&self) -> i32 {
        319
    }

    fn get_sea_level(&self) -> i32 {
        63
    }

    fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

    fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

    fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

    fn get_seed(&self) -> i64 {
        self.seed
    }

    fn add_structure_mob(&mut self, resource_id: &str, position: [f64; 3]) {
        if !position.iter().all(|coord| coord.is_finite()) {
            return;
        }
        self.proto_entities
            .push((resource_id.to_string(), position));
    }
}

pub fn startup() -> DatapackStartup {
    DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap()
}

pub fn overworld_plan(startup: &DatapackStartup) -> NoiseStructurePlan {
    let source = NoiseBiomeSource::from_registry(
        &startup.registries,
        "minecraft:overworld",
        "minecraft:overworld",
        0,
    )
    .unwrap();
    NoiseStructurePlan::from_startup(startup, source.overworld_biome_order().unwrap()).unwrap()
}

pub fn monument_start() -> OceanMonumentStructureStart {
    let startup = startup();
    let plan = overworld_plan(&startup);
    let chunk = plan.first_candidate(crate::noise::ocean_monument::OCEAN_MONUMENT_SET_ID, 0);
    let surrounding = required_surrounding_biomes(&startup).unwrap();
    plan.generate_missing_random_spread_ocean_monument_starts(
        0,
        chunk[0],
        chunk[1],
        &[],
        63,
        &surrounding,
        |_, _, heightmap| {
            assert_eq!(heightmap, StructureHeightmap::OceanFloorWg);
            Some(40)
        },
        |_| Some("minecraft:deep_ocean".to_string()),
    )
    .into_iter()
    .next()
    .expect("the first monument candidate has a monument")
}

/// The first level seed whose regenerated monument layout has every room
/// kind in `kinds`, with that layout.
pub fn monument_children_with(
    kinds: &[OceanMonumentRoomKind],
) -> (OceanMonumentStructureStart, RegeneratedMonumentChildren) {
    let start = monument_start();
    let [chunk_x, chunk_z] = start.start_chunk();
    let children = (0..256)
        .map(|seed| regenerate_monument_children(seed, chunk_x, chunk_z, start.piece()))
        .find(|children| {
            kinds
                .iter()
                .all(|kind| children.pieces.iter().any(|room| room.kind() == *kind))
        })
        .expect("some layout has every requested room kind");
    (start, children)
}
