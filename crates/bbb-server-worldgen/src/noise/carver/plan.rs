//! Biome-selected configured-carver holders compiled for the CARVERS stage.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
    vanilla_block_state_identity,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::tag_loader::TagLoadReport;
use bbb_server_datapack::worldgen::{
    BiomeCarverHolder, BiomeCarvers, CarverDebugSettings, ConfiguredCarver, HolderSet,
    CONFIGURED_CARVER_REGISTRY,
};

use super::{error, normalize_id, NoiseDensityError, BIOME_REGISTRY, BLOCK_REGISTRY};

pub(super) struct CompiledCarver {
    pub(super) config: Arc<ConfiguredCarver>,
    pub(super) replaceable: BTreeSet<i32>,
    /// Present only when `debug_mode` is on; Vanilla then stamps these states
    /// instead of the carved ones.
    pub(super) debug: Option<DebugStates>,
}

#[derive(Clone, Copy)]
pub(super) struct DebugStates {
    pub(super) air: i32,
    pub(super) water: i32,
    pub(super) lava: i32,
    pub(super) barrier: i32,
}

pub(super) struct RegistryCarverPlan {
    by_biome: Vec<Option<Vec<Arc<CompiledCarver>>>>,
}

impl RegistryCarverPlan {
    pub(super) fn compile(
        generation: &DatapackGeneration,
        tags: &TagLoadReport,
        possible_biomes: &[&str],
    ) -> Result<Self, NoiseDensityError> {
        let biomes = generation
            .registry(BIOME_REGISTRY)
            .ok_or_else(|| error(BIOME_REGISTRY, "registry is absent"))?;
        let possible = possible_biomes
            .iter()
            .map(|id| normalize_id(id, "noise_generator.carvers.possible_biomes"))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let mut cache = BTreeMap::<String, Arc<CompiledCarver>>::new();
        let mut by_biome = vec![None; biomes.elements.len()];

        for (biome_index, biome) in biomes.elements.iter().enumerate() {
            if !possible.contains(&biome.id) {
                continue;
            }
            let path = format!("{BIOME_REGISTRY}.{}.carvers", biome.id);
            let decoded_owned;
            let carvers = match generation
                .worldgen
                .cached_biome_carvers(&biome.id, &biome.source_json)
            {
                Some(Ok(Some(carvers))) => carvers,
                Some(Ok(None)) => return Err(error(&path, "field is absent")),
                Some(Err(err)) => return Err(error(&path, err.to_string())),
                None => {
                    let source = biome.decoded_loaded_source(usize::MAX).map_err(|err| {
                        error(&path, format!("source JSON cannot be decoded: {err}"))
                    })?;
                    let json = source
                        .json()
                        .ok_or_else(|| error(&path, "parsed source JSON is absent"))?;
                    decoded_owned = BiomeCarvers::decode_optional(json)
                        .map_err(|err| error(&path, err.to_string()))?
                        .ok_or_else(|| error(&path, "field is absent"))?;
                    &decoded_owned
                }
            };
            let mut compiled = Vec::new();
            for holder in &carvers.holders {
                match holder {
                    BiomeCarverHolder::Tag(tag) => {
                        let entries = tags
                            .registry(CONFIGURED_CARVER_REGISTRY)
                            .and_then(|registry| registry.tag(tag))
                            .ok_or_else(|| {
                                error(&path, format!("unresolved configured-carver tag #{tag}"))
                            })?;
                        for id in entries {
                            compiled.push(resolve_named_carver(
                                generation, tags, &mut cache, id, &path,
                            )?);
                        }
                    }
                    BiomeCarverHolder::Named(id) => {
                        compiled.push(resolve_named_carver(
                            generation, tags, &mut cache, id, &path,
                        )?);
                    }
                    BiomeCarverHolder::Inline(config) => {
                        compiled.push(Arc::new(compile_carver(Arc::clone(config), tags, &path)?));
                    }
                }
            }
            by_biome[biome_index] = Some(compiled);
        }

        if let Some(missing) = possible.iter().find(|id| {
            biomes
                .elements
                .iter()
                .position(|biome| &biome.id == *id)
                .is_none_or(|index| by_biome[index].is_none())
        }) {
            return Err(error(
                "noise_generator.carvers.possible_biomes",
                format!("biome {missing} is absent"),
            ));
        }
        Ok(Self { by_biome })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.by_biome.iter().flatten().all(Vec::is_empty)
    }

    pub(super) fn for_biome(
        &self,
        biome_id: i32,
    ) -> Result<&[Arc<CompiledCarver>], NoiseDensityError> {
        usize::try_from(biome_id)
            .ok()
            .and_then(|index| self.by_biome.get(index))
            .and_then(Option::as_deref)
            .ok_or_else(|| {
                error(
                    "noise_generator.carvers.apply",
                    format!("sampled biome id {biome_id} has no compiled carver plan"),
                )
            })
    }
}

fn resolve_named_carver(
    generation: &DatapackGeneration,
    tags: &TagLoadReport,
    cache: &mut BTreeMap<String, Arc<CompiledCarver>>,
    id: &str,
    path: &str,
) -> Result<Arc<CompiledCarver>, NoiseDensityError> {
    if let Some(carver) = cache.get(id) {
        return Ok(Arc::clone(carver));
    }
    let config = generation
        .worldgen
        .configured_carvers
        .get(id)
        .cloned()
        .ok_or_else(|| error(path, format!("unresolved configured carver {id}")))?;
    let carver = Arc::new(compile_carver(config, tags, path)?);
    cache.insert(id.to_string(), Arc::clone(&carver));
    Ok(carver)
}

fn compile_carver(
    config: Arc<ConfiguredCarver>,
    tags: &TagLoadReport,
    path: &str,
) -> Result<CompiledCarver, NoiseDensityError> {
    let ids = match &config.replaceable {
        HolderSet::Tag(tag) => tags
            .registry(BLOCK_REGISTRY)
            .and_then(|registry| registry.tag(tag))
            .ok_or_else(|| error(path, format!("unresolved block tag #{tag}")))?
            .to_vec(),
        HolderSet::Direct(ids) => ids.clone(),
    };
    let replaceable = ids
        .iter()
        .map(|id| {
            vanilla_block_type_for_resource_id(id)
                .map(|block| block.id)
                .ok_or_else(|| error(path, format!("unknown block {id}")))
        })
        .collect::<Result<_, _>>()?;
    let debug = config
        .debug_settings
        .as_ref()
        .filter(|debug| debug.debug_mode)
        .map(debug_states);
    Ok(CompiledCarver {
        config,
        replaceable,
        debug,
    })
}

/// `CarverDebugSettings.CODEC` defaults every missing state to the default
/// air state (acacia button), and the constructor waterlogs the water state.
fn debug_states(settings: &CarverDebugSettings) -> DebugStates {
    let air = default_state("minecraft:acacia_button");
    let water = settings.water_state.unwrap_or(air);
    DebugStates {
        air: settings.air_state.unwrap_or(air),
        water: waterlogged(water),
        lava: settings.lava_state.unwrap_or(air),
        barrier: settings.barrier_state.unwrap_or(air),
    }
}

fn default_state(id: &str) -> i32 {
    vanilla_block_type_for_resource_id(id)
        .and_then(|block| vanilla_block_default_state_id(block.id))
        .expect("built-in debug block has a default state")
}

fn waterlogged(state: i32) -> i32 {
    let Some(identity) = vanilla_block_state_identity(state) else {
        return state;
    };
    if !identity.properties.contains_key("waterlogged") {
        return state;
    }
    let mut properties = identity.properties.clone();
    properties.insert("waterlogged".to_string(), "true".to_string());
    vanilla_block_state_id_by_name_and_properties(&identity.name, &properties).unwrap_or(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_states_default_to_air_and_waterlog_the_water_state() {
        let air = default_state("minecraft:acacia_button");
        let dry = vanilla_block_state_id_by_name_and_properties(
            "minecraft:mangrove_roots",
            &BTreeMap::from([("waterlogged".to_owned(), "false".to_owned())]),
        )
        .unwrap();
        let wet = vanilla_block_state_id_by_name_and_properties(
            "minecraft:mangrove_roots",
            &BTreeMap::from([("waterlogged".to_owned(), "true".to_owned())]),
        )
        .unwrap();
        let states = debug_states(&CarverDebugSettings {
            debug_mode: true,
            air_state: None,
            water_state: Some(dry),
            lava_state: None,
            barrier_state: None,
        });
        assert_eq!(
            [states.air, states.water, states.lava, states.barrier],
            [air, wet, air, air]
        );
    }
}
