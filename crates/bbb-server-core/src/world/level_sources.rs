//! The generator and dimension type of every level a world declares, sized
//! by the `dimension_type` holder the session receives.

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::{decode_nbt_summary, NbtSummaryValue};
use bbb_registry_codec::registry_codec::DimensionTypeScalars;
use bbb_server_datapack::debug_generator::DebugLevelSource;
use bbb_server_datapack::dimension_type::bake_java_26_1_level_stems;
use bbb_server_datapack::dimension_type::DIMENSION_TYPE_REGISTRY;
use bbb_server_datapack::flat_generator::{served_level_sources_from_stems, FlatLevelSource};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_persistence::Java26WorldGenSettings;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::save::{Java26LevelKey, SaveError, SaveResult};
use bbb_server_types::{Result, ServerWorldError};

use super::ServerLevelSource;
use crate::chunk::noise_generator::compile_noise_level_source;
use crate::registry_sync::SynchronizedRegistries;

const NOISE_GENERATOR: &str = "minecraft:noise";

pub(super) struct DeclaredLevelSources {
    pub(super) served: BTreeMap<Java26LevelKey, ServerLevelSource>,
    pub(super) unserved: BTreeMap<Java26LevelKey, String>,
    /// `LevelStem.type`, the element a session's `CommonPlayerSpawnInfo` indexes.
    pub(super) dimension_types: BTreeMap<Java26LevelKey, String>,
}

/// Every level a save's world-gen settings declare. `require_owned_noise`
/// makes a noise stem without startup datapacks fail the world instead of
/// leaving that level unserved.
pub(super) fn level_sources_from_world_gen_settings(
    settings: &Java26WorldGenSettings,
    startup: Option<&Arc<DatapackStartup>>,
    require_owned_noise: bool,
) -> SaveResult<DeclaredLevelSources> {
    const PATH: &str = "world_gen_settings.data.dimensions";
    let mut dimensions = match startup {
        Some(startup) => bake_java_26_1_level_stems(settings, &startup.registries),
        None => settings.dimensions.clone(),
    };
    dimensions.retain(|_, stem| match startup {
        Some(startup) => bbb_server_datapack::dimension_type::dimension_type_runtime_facts(
            &startup.registries,
            &stem.dimension_type,
        )
        .is_ok(),
        None => {
            bbb_server_datapack::dimension_type::builtin_dimension_type_shape(&stem.dimension_type)
                .is_ok()
        }
    });
    let mut declared = served_level_sources_from_stems(
        &dimensions,
        startup.map(|startup| &startup.registries),
        PATH,
    )?;
    let mut served = declared
        .served
        .into_iter()
        .map(|(dimension, source)| (dimension, ServerLevelSource::Flat(source)))
        .collect::<BTreeMap<_, _>>();
    for dimension in dimensions
        .iter()
        .filter(|(_, stem)| stem.generator == "minecraft:debug")
        .map(|(dimension, _)| dimension)
    {
        served.insert(
            dimension.clone(),
            ServerLevelSource::Debug(DebugLevelSource::new(0, 384)?),
        );
        declared.unserved.remove(dimension);
    }
    // Every noise stem is compiled for its own key and dimension type; one that
    // cannot compile fails the load as `WorldDimensions` decoding does.
    for (key, stem) in dimensions
        .iter()
        .filter(|(_, stem)| stem.generator == NOISE_GENERATOR)
    {
        let path = format!("{PATH}.{}.generator", key.resource_id());
        let Some(startup) = startup else {
            if require_owned_noise {
                return Err(SaveError::InvalidNbt {
                    path: None,
                    detail: format!("{path} requires the startup datapack generation"),
                });
            }
            continue;
        };
        let program = compile_noise_level_source(stem, Arc::clone(startup), settings.seed, &path)?;
        program.preflight_structure_start_owners(settings.generate_structures)?;
        served.insert(
            key.clone(),
            ServerLevelSource::Noise {
                program: Arc::new(program),
                generate_structures: settings.generate_structures,
            },
        );
        declared.unserved.remove(key);
    }
    let dimension_types = dimensions
        .iter()
        .map(|(name, stem)| (name.clone(), stem.dimension_type.clone()))
        .collect();
    Ok(DeclaredLevelSources {
        served,
        unserved: declared.unserved,
        dimension_types,
    })
}

/// `min_y`/`height` of the published element (`DimensionType.java:92-93`).
fn published_dimension_type_height(
    registries: &SynchronizedRegistries,
    element_id: &str,
) -> Result<(i32, i32)> {
    let invalid = |detail: String| ServerWorldError::InvalidConfiguration(detail);
    let entry = registries
        .published()
        .iter()
        .find(|registry| registry.registry == DIMENSION_TYPE_REGISTRY)
        .ok_or_else(|| {
            invalid(
                "the synchronized catalog contains no minecraft:dimension_type registry"
                    .to_string(),
            )
        })?
        .entries
        .iter()
        .find(|entry| entry.id == element_id)
        .ok_or_else(|| {
            invalid(format!(
                "dimension type {element_id} is absent from the published transaction"
            ))
        })?;
    let payload = entry.raw_data.as_ref().ok_or_else(|| {
        invalid(format!(
            "published dimension type {element_id} carries no payload"
        ))
    })?;
    let value = decode_nbt_summary(payload)
        .map_err(|error| {
            invalid(format!(
                "published dimension type {element_id} is not readable NBT: {error}"
            ))
        })?
        .ok_or_else(|| {
            invalid(format!(
                "published dimension type {element_id} is empty NBT"
            ))
        })?;
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(invalid(format!(
            "published dimension type {element_id} is not a compound"
        )));
    };
    let int = |name: &str| match fields.iter().find(|entry| entry.name == name)?.value {
        NbtSummaryValue::Int(value) => Some(value),
        _ => None,
    };
    let min_y = int("min_y").ok_or_else(|| {
        invalid(format!(
            "published dimension type {element_id} has no min_y"
        ))
    })?;
    let height = int("height").ok_or_else(|| {
        invalid(format!(
            "published dimension type {element_id} has no height"
        ))
    })?;
    DimensionTypeScalars::check_shape(min_y, height, int("logical_height"))
        .map_err(|message| invalid(format!("published dimension type {element_id}: {message}")))?;
    Ok((min_y, height))
}

/// Sizes every served source whose dimension type the session receives,
/// returning the bound sources and their dimension type elements.
pub(super) fn bind_level_sources_to_published_dimension_types(
    sources: &BTreeMap<Java26LevelKey, ServerLevelSource>,
    dimension_types: &BTreeMap<Java26LevelKey, String>,
    registries: &SynchronizedRegistries,
) -> Result<(
    BTreeMap<Java26LevelKey, ServerLevelSource>,
    BTreeMap<Java26LevelKey, String>,
)> {
    let mut bound = BTreeMap::new();
    let mut level_dimension_types = BTreeMap::new();
    for (dimension, source) in sources {
        let Some(element_id) = dimension_types.get(dimension) else {
            continue;
        };
        if registries.dimension_type_index(element_id).is_none() {
            continue;
        }
        let (min_y, height) = published_dimension_type_height(registries, element_id)?;
        let invalid = |error: SaveError| ServerWorldError::InvalidConfiguration(error.to_string());
        let source = match source {
            ServerLevelSource::Flat(flat) => ServerLevelSource::Flat(
                FlatLevelSource::new(flat.settings().clone(), min_y, height).map_err(invalid)?,
            ),
            ServerLevelSource::Debug(_) => {
                ServerLevelSource::Debug(DebugLevelSource::new(min_y, height).map_err(invalid)?)
            }
            ServerLevelSource::Noise { program, .. } => {
                if program.dimension_type_id() != element_id {
                    return Err(ServerWorldError::InvalidConfiguration(format!(
                        "{} names dimension type {element_id}, but its noise program is {}",
                        dimension.resource_id(),
                        program.dimension_type_id()
                    )));
                }
                let shape = program.dimension_type_shape();
                if shape.min_y != min_y || shape.height != height {
                    return Err(ServerWorldError::InvalidConfiguration(format!(
                        "{} noise program is sized min_y={} height={} but published {element_id} is min_y={min_y} height={height}",
                        dimension.resource_id(),
                        shape.min_y,
                        shape.height
                    )));
                }
                source.clone()
            }
        };
        bound.insert(dimension.clone(), source);
        level_dimension_types.insert(dimension.clone(), element_id.clone());
    }
    Ok((bound, level_dimension_types))
}

#[cfg(test)]
impl super::ServerWorld {
    /// Published `min_y`/`height` for this loaded level's dimension type.
    pub(crate) fn dimension_type_height(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Result<(i32, i32)> {
        let element_id = self
            .cold
            .level_dimension_types
            .get(&super::LevelId::from(dimension))
            .ok_or_else(|| {
                ServerWorldError::InvalidConfiguration(format!(
                    "{} is not a loaded level",
                    dimension.resource_id()
                ))
            })?;
        published_dimension_type_height(&self.registries, element_id)
    }

    pub(crate) fn common_player_spawn_info(
        &self,
        dimension: Java26BuiltinDimension,
        game_type: bbb_protocol::game_types::GameType,
    ) -> bbb_protocol::packets::CommonPlayerSpawnInfo {
        self.common_spawn_info(&dimension.into(), game_type, None, 0, None)
    }
}

#[cfg(test)]
mod tests;
