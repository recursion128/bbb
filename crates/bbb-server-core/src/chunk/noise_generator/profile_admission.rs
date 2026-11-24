use std::sync::Arc;

use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_persistence::Java26LevelStem;
use bbb_server_types::schema_support::{
    expect_compound, expect_resource_id, field, invalid_schema, required_field,
};
use bbb_server_worldgen::noise::biome::{
    resolve_multi_noise_parameter_list, MultiNoisePreset, ResolvedMultiNoisePreset,
};
use bbb_server_worldgen::noise::carver::BuiltInCarverProgram;

use super::{
    compile_noise_final_density, NoiseFinalDensityProgram, SaveResult, NOISE_GENERATOR_ID,
};

/// Compiles a `minecraft:noise` stem for any level key and dimension type.
///
/// The type holder and generator are independent fields (`LevelStem.java:11-15`):
/// the biome source picks the CARVERS/FEATURES owner, the noise settings drive
/// terrain, and the dimension type only bounds the generated height
/// (`NoiseSettings.clampToHeightAccessor`, `NoiseSettings.java:54-58`).
pub fn compile_noise_level_source(
    stem: &Java26LevelStem,
    startup: Arc<DatapackStartup>,
    world_seed: i64,
    path: &str,
) -> SaveResult<NoiseFinalDensityProgram> {
    let profile = OwnedBiomeSourceProfile::from_stem(stem, &startup, path)?;
    let program = compile_noise_final_density(stem, startup, world_seed, path)?;
    if !profile.owns_complete_pipeline(&program) {
        return Err(invalid_schema(
            path,
            "the biome-source profile did not compile CARVERS, FEATURES and structure owners",
        ));
    }
    Ok(program)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnedBiomeSourceProfile {
    Direct,
    End,
    NetherPreset,
    OverworldPreset,
    RegistryBacked,
}

impl OwnedBiomeSourceProfile {
    pub(super) fn from_stem(
        stem: &Java26LevelStem,
        startup: &DatapackStartup,
        path: &str,
    ) -> SaveResult<Self> {
        if stem.generator != NOISE_GENERATOR_ID {
            return Err(invalid_schema(
                &format!("{path}.type"),
                format!("{} is not {NOISE_GENERATOR_ID}", stem.generator),
            ));
        }
        let fields = expect_compound(&stem.raw_generator, path)?;
        let source_path = format!("{path}.biome_source");
        let source = expect_compound(required_field(fields, "biome_source", path)?, &source_path)?;
        let type_id = expect_resource_id(
            required_field(source, "type", &source_path)?,
            &format!("{source_path}.type"),
        )?;
        match type_id.as_str() {
            "minecraft:the_end" => Ok(Self::End),
            "minecraft:fixed" | "minecraft:checkerboard" => Ok(Self::RegistryBacked),
            "minecraft:multi_noise" => {
                let preset = field(source, "preset");
                let biomes = field(source, "biomes");
                if preset.is_some() == biomes.is_some() {
                    return Err(invalid_schema(
                        &source_path,
                        "multi_noise requires exactly one of preset or biomes",
                    ));
                }
                let Some(preset) = preset else {
                    return Ok(Self::Direct);
                };
                match resolve_multi_noise_parameter_list(
                    &startup.registries,
                    preset,
                    &format!("{source_path}.preset"),
                )
                .map_err(|error| invalid_schema(&source_path, error.to_string()))?
                {
                    ResolvedMultiNoisePreset::Builtin(MultiNoisePreset::Nether) => {
                        Ok(Self::NetherPreset)
                    }
                    ResolvedMultiNoisePreset::Builtin(MultiNoisePreset::Overworld) => {
                        Ok(Self::OverworldPreset)
                    }
                    ResolvedMultiNoisePreset::Custom(_) => Ok(Self::Direct),
                }
            }
            _ => Err(invalid_schema(
                &format!("{source_path}.type"),
                format!("biome source type {type_id} has no runtime owner"),
            )),
        }
    }

    pub(super) fn owns_complete_pipeline(self, program: &NoiseFinalDensityProgram) -> bool {
        let structures = program.structures.is_some();
        match self {
            Self::End => {
                ((program.carvers_are_verified_empty && program.carvers.is_none())
                    || (!program.carvers_are_verified_empty
                        && matches!(
                            program.carvers.as_ref(),
                            Some(BuiltInCarverProgram::Overworld(_))
                        )))
                    && program.feature_resources.is_some()
                    && program.nether_features.is_none()
                    && structures
            }
            Self::NetherPreset => {
                matches!(
                    program.carvers.as_ref(),
                    Some(BuiltInCarverProgram::Nether(_))
                ) && program.feature_resources.is_none()
                    && program.nether_features.is_some()
                    && structures
            }
            Self::Direct | Self::OverworldPreset | Self::RegistryBacked => {
                matches!(
                    program.carvers.as_ref(),
                    Some(BuiltInCarverProgram::Overworld(_))
                ) && program.feature_resources.is_some()
                    && program.nether_features.is_none()
                    && structures
            }
        }
    }
}
