use super::*;

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_worldgen::noise::biome::NoiseBiomeSource;
use bbb_server_worldgen::noise::carver::nether::NetherCarverProgram;
use bbb_server_worldgen::noise::carver::nether::NETHER_BIOMES;
use bbb_server_worldgen::noise::carver::BuiltInCarverProgram;
use bbb_server_worldgen::noise::density::NoiseDensitySource;
use bbb_server_worldgen::noise::surface::NoiseSurfaceProgram;

fn install_custom_carver(
    startup: &mut Arc<DatapackStartup>,
    id: &str,
    source_id: &str,
    probability: f32,
) {
    let mut source = configured_carver_json(startup, source_id);
    source["config"]["probability"] = serde_json::json!(probability);
    source["config"]["replaceable"] = serde_json::json!(["minecraft:netherrack"]);
    put_configured_carver(startup, id, source);
}

fn set_biome_carvers(
    startup: &mut Arc<DatapackStartup>,
    biome_id: &str,
    carvers: serde_json::Value,
) {
    let element = Arc::get_mut(startup)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == biome_id)
        .unwrap();
    let mut source: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
    source["carvers"] = carvers;
    element.source_pack = "file/custom_nether_carvers".to_string();
    element.source_json = serde_json::to_vec(&source).unwrap().into();
    element.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(source)).unwrap())
            .unwrap(),
    );
    element.codec_value = None;
}

struct CompiledNetherCarvers {
    source: NoiseDensitySource,
    biome_source: NoiseBiomeSource,
    surface: NoiseSurfaceProgram,
    carvers: BuiltInCarverProgram,
    biome_ids: BTreeMap<String, i32>,
}

impl CompiledNetherCarvers {
    fn apply_to_empty_chunk(&self, chunk_x: i32, chunk_z: i32) -> Result<Vec<i64>, String> {
        let biome = self.biome_ids["minecraft:nether_wastes"];
        let mut column = ChunkColumn::new_with_biome_registry(
            0,
            8,
            self.source.air_block_state_id(),
            biome,
            self.biome_source.registry_len(),
        );
        let mut ocean_floor_wg = [0; 256];
        let mut world_surface_wg = [0; 256];
        let mut postprocessing = Vec::new();
        self.carvers
            .apply(
                &self.source,
                &self.biome_source,
                &self.surface,
                chunk_x,
                chunk_z,
                &mut column,
                &mut ocean_floor_wg,
                &mut world_surface_wg,
                &mut postprocessing,
            )
            .map_err(|error| error.to_string())
    }

    fn apply_to_netherrack_chunk(&self, chunk_x: i32, chunk_z: i32) -> Result<Vec<i64>, String> {
        let biome = self.biome_ids["minecraft:nether_wastes"];
        let mut column = ChunkColumn::new_with_biome_registry(
            0,
            8,
            self.source.air_block_state_id(),
            biome,
            self.biome_source.registry_len(),
        );
        let netherrack = vanilla_block_type_for_resource_id("minecraft:netherrack")
            .and_then(|block| vanilla_block_default_state_id(block.id))
            .expect("Vanilla netherrack default state");
        for y in 0..128 {
            for local_x in 0..16 {
                for local_z in 0..16 {
                    let _ = column.set_block_state(local_x, y, local_z, netherrack);
                }
            }
        }
        let mut ocean_floor_wg = [128; 256];
        let mut world_surface_wg = [128; 256];
        let mut postprocessing = Vec::new();
        self.carvers
            .apply(
                &self.source,
                &self.biome_source,
                &self.surface,
                chunk_x,
                chunk_z,
                &mut column,
                &mut ocean_floor_wg,
                &mut world_surface_wg,
                &mut postprocessing,
            )
            .map_err(|error| error.to_string())
    }
}

fn compile_nether(
    startup: Arc<DatapackStartup>,
    seed: i64,
) -> Result<CompiledNetherCarvers, String> {
    let generation = &startup.registries;
    let source = NoiseDensitySource::from_registry(generation, "minecraft:nether", seed)
        .map_err(|error| error.to_string())?;
    let biome_source =
        NoiseBiomeSource::from_registry(generation, "minecraft:nether", "minecraft:nether", seed)
            .map_err(|error| error.to_string())?;
    let surface = NoiseSurfaceProgram::from_registry(generation, "minecraft:nether", seed, &source)
        .map_err(|error| error.to_string())?;
    let carvers = BuiltInCarverProgram::Nether(
        NetherCarverProgram::from_registry(generation, &startup.tags, seed, &source, &biome_source)
            .map_err(|error| error.to_string())?,
    );
    let biome_ids = generation
        .registry("minecraft:worldgen/biome")
        .ok_or_else(|| "biome registry is absent".to_string())?
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| (element.id.clone(), index as i32))
        .collect();
    Ok(CompiledNetherCarvers {
        source,
        biome_source,
        surface,
        carvers,
        biome_ids,
    })
}

fn set_all_nether_biome_carvers(startup: &mut Arc<DatapackStartup>, carvers: serde_json::Value) {
    for biome in NETHER_BIOMES {
        set_biome_carvers(startup, biome, carvers.clone());
    }
}

#[test]
fn custom_nether_carver_types_execute() {
    let mut startup = Arc::new(fresh_vanilla_startup());
    install_custom_carver(&mut startup, "example:nether", "minecraft:nether_cave", 1.0);
    install_custom_carver(&mut startup, "example:cave", "minecraft:cave", 1.0);
    install_custom_carver(&mut startup, "example:canyon", "minecraft:canyon", 1.0);
    set_all_nether_biome_carvers(&mut startup, serde_json::json!([]));
    set_biome_carvers(
        &mut startup,
        "minecraft:nether_wastes",
        serde_json::json!(["example:nether", "example:canyon", "example:nether"]),
    );
    set_biome_carvers(
        &mut startup,
        "minecraft:soul_sand_valley",
        serde_json::json!("example:cave"),
    );

    let program = compile_nether(startup, 71).unwrap();
    assert!(!program.apply_to_empty_chunk(0, 0).unwrap().is_empty());
}

#[test]
fn source_exact_nether_carver_plan_runs_the_canonical_nether_runtime() {
    let program = compile_nether(vanilla_startup(), 0).unwrap();
    let mask = program.apply_to_netherrack_chunk(0, 0).unwrap();
    assert!(!mask.is_empty());
}
