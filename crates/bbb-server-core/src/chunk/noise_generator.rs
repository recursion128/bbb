//! Saved-stem projection for the executable part of `minecraft:noise`.
//!
//! Generating a fresh base chunk does not make the stem serveable. The caller
//! keeps the stem in `unserved_levels` until surface, carver, persistence and
//! serving owners consume this program through a complete level source.

use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::biome_climate::height_adjusted_temperature;
use bbb_server_datapack::worldgen::BiomeClimate;
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use bbb_server_datapack::dimension_type::{dimension_type_shape, DimensionTypeShape};
use bbb_server_datapack::dynamic_registry::LoadedDynamicRegistry;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::chunk_data::{ChunkColumn, ChunkGenerationState};
use bbb_server_types::fluid::{
    block_state_blocks_motion, block_state_fluid_state, block_state_is_air,
};
use bbb_server_worldgen::noise::aquifer::Aquifer;
use bbb_server_worldgen::noise::beardifier::JigsawBeardifier;
use bbb_server_worldgen::noise::biome::{
    resolve_multi_noise_parameter_list, MultiNoisePreset, NoiseBiomeSource,
    ResolvedMultiNoisePreset,
};
use bbb_server_worldgen::noise::carver::nether::NetherCarverProgram;
use bbb_server_worldgen::noise::carver::{BuiltInCarverProgram, OverworldCarverProgram};
use bbb_server_worldgen::noise::density::{NoiseChunkSampler, NoiseDensitySource};
use bbb_server_worldgen::noise::igloo::IglooStructureStart;
use bbb_server_worldgen::noise::mineshaft::MineshaftStructureStart;
use bbb_server_worldgen::noise::ocean_monument::OceanMonumentStructureStart;
use bbb_server_worldgen::noise::ocean_ruin::{OceanRuinStructureStart, OCEAN_RUIN_TYPE_ID};
use bbb_server_worldgen::noise::ore_vein::OreVein;
use bbb_server_worldgen::noise::ruined_portal::{
    RuinedPortalStructureStart, RUINED_PORTAL_TYPE_ID,
};
use bbb_server_worldgen::noise::scattered::{ScatteredFeatureStructureStart, SWAMP_HUT_ID};
use bbb_server_worldgen::noise::shipwreck::{ShipwreckStructureStart, SHIPWRECK_TYPE_ID};
#[cfg(test)]
use bbb_server_worldgen::noise::stronghold::STRONGHOLD_SET_ID;
use bbb_server_worldgen::noise::stronghold::{StrongholdStructureStart, STRONGHOLD_ID};
#[cfg(test)]
use bbb_server_worldgen::noise::structure_starts::JigsawStructureStart;
use bbb_server_worldgen::noise::structure_starts::{
    JigsawStructureChunkData, NoiseStructureChunkData,
};
use bbb_server_worldgen::noise::structures::{
    NoiseStructureDecorationInvocation, NoiseStructurePlan, StructureDecorationStep,
    StructureStartSetOwner,
};
use bbb_server_worldgen::noise::surface::NoiseSurfaceProgram;
use bbb_server_worldgen::noise::woodland_mansion::WoodlandMansionStructureStart;
#[cfg(test)]
use bbb_server_worldgen::noise::woodland_mansion::{
    WOODLAND_MANSION_ID, WOODLAND_MANSION_SET_ID, WOODLAND_MANSION_TYPE_ID,
};

use crate::chunk::blending::BlendingResolver;

use bbb_server_persistence::Java26LevelStem;
use bbb_server_types::save::SaveResult;
use bbb_server_types::schema_support::{
    expect_compound, expect_resource_id, field, invalid_schema, required_field,
};

pub const NOISE_GENERATOR_ID: &str = "minecraft:noise";

type NoiseFillSummary = ([i32; 256], [i32; 256], Vec<(i32, i32, i32)>);

/// Collects exactly `N` items straight into the heap; a `[T; N]` of chunks
/// would be tens of KiB of stack per frame.
pub fn boxed_array<T, const N: usize>(items: impl IntoIterator<Item = T>) -> Box<[T; N]> {
    let items: Vec<T> = items.into_iter().collect();
    let len = items.len();
    items
        .try_into()
        .unwrap_or_else(|_| panic!("expected {N} items, found {len}"))
}

/// Drains a boxed array without moving it back onto the stack.
pub fn drain_boxed<T, const N: usize>(items: Box<[T; N]>) -> std::vec::IntoIter<T> {
    let items: Box<[T]> = items;
    items.into_vec().into_iter()
}

mod buried_treasure;
mod end_features;
mod end_structures;
pub mod features;
mod inline_settings;
mod nether_features;
mod nether_structures;
mod profile_admission;
mod registered_structure_starts;
mod reloadable_resources;
mod retrogen;
pub mod structure_status;
use end_features::compile_feature_resources;
use end_structures::compile_builtin_structure_plan;
use nether_features::NetherFeatureResources;
pub use profile_admission::compile_noise_level_source;
use reloadable_resources::compile_end_carvers;
pub use reloadable_resources::NoiseReloadableResources;
pub use structure_status::NoiseStructureRingPositions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsDrivenStructureFamily {
    Shipwreck,
    OceanRuin,
    RuinedPortal,
}

pub struct NoiseFinalDensityProgram {
    source: Arc<NoiseDensitySource>,
    world_seed: i64,
    biome_source: Option<Arc<NoiseBiomeSource>>,
    biome_names: Vec<String>,
    surface: Option<Arc<NoiseSurfaceProgram>>,
    carvers: Option<BuiltInCarverProgram>,
    carvers_are_verified_empty: bool,
    structures: Option<NoiseStructurePlan>,
    feature_resources: Option<features::GenericFeatureResources>,
    nether_features: Option<NetherFeatureResources>,
    dimension_type_id: String,
    dimension_shape: DimensionTypeShape,
    /// Concentric-ring positions are a pure function of the frozen plan and
    /// `world_seed`; the biome search behind them is computed once.
    structure_rings: OnceLock<NoiseStructureRingPositions>,
    /// `Level.registryAccess()` for the level this program generates
    /// (`StructureTemplate.java:305-306`), set once by the owner that builds
    /// the level around it, exactly as `ServerLevel`'s constructor is handed
    /// one (`ServerLevel.java:236`). Unset reads block-entity item stacks the
    /// way a level with no publication must: a stack needing a named holder is
    /// refused rather than silently emptied.
    item_publication: OnceLock<crate::chunk::generation_publication::GenerationItemPublication>,
}

impl NoiseFinalDensityProgram {
    /// Hands this program the level's registry publication, once.
    ///
    /// `false` when the level already published one, which is a caller that
    /// built two levels over one generator rather than a state to merge.
    pub fn publish_item_registry(
        &self,
        publication: crate::chunk::generation_publication::GenerationItemPublication,
    ) -> bool {
        self.item_publication.set(publication).is_ok()
    }

    pub(crate) fn item_publication(
        &self,
    ) -> Option<&crate::chunk::generation_publication::GenerationItemPublication> {
        self.item_publication.get()
    }

    pub(crate) fn settings_driven_structure_family(
        &self,
        structure_id: &str,
    ) -> Option<SettingsDrivenStructureFamily> {
        let definition = self.structures.as_ref()?.structures().get(structure_id)?;
        if definition.step != StructureDecorationStep::SurfaceStructures {
            return None;
        }
        if definition.type_id == SHIPWRECK_TYPE_ID && definition.shipwreck.is_some() {
            Some(SettingsDrivenStructureFamily::Shipwreck)
        } else if definition.type_id == OCEAN_RUIN_TYPE_ID && definition.ocean_ruin.is_some() {
            Some(SettingsDrivenStructureFamily::OceanRuin)
        } else if definition.type_id == RUINED_PORTAL_TYPE_ID && definition.ruined_portal.is_some()
        {
            Some(SettingsDrivenStructureFamily::RuinedPortal)
        } else {
            None
        }
    }

    pub fn dimension_type_id(&self) -> &str {
        &self.dimension_type_id
    }

    pub const fn dimension_type_shape(&self) -> DimensionTypeShape {
        self.dimension_shape
    }

    pub fn settings_id(&self) -> &str {
        self.source.settings_id()
    }

    pub fn disable_mob_generation(&self) -> bool {
        self.source.disable_mob_generation()
    }

    pub(crate) fn find_spawn_position(&self) -> [i32; 2] {
        self.source.find_spawn_position()
    }

    pub fn biome_name(&self, id: i32) -> Option<String> {
        let id = usize::try_from(id).ok()?;
        self.biome_names.get(id).cloned()
    }

    pub(crate) fn uncached_noise_biome(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        let source = self.biome_source.as_ref()?;
        Some(source.sample(&self.source, x, y, z, &mut source.create_search()))
    }

    pub(crate) fn find_closest_biome_3d(
        &self,
        origin: BlockPos,
        allowed_registry_ids: &BTreeSet<i32>,
        search_radius: i32,
        horizontal_step: i32,
        vertical_step: i32,
    ) -> Option<(BlockPos, i32)> {
        let biome_source = self.biome_source.as_ref()?;
        let possible = biome_source.possible_biome_order();
        let candidates = allowed_registry_ids
            .iter()
            .copied()
            .filter(|id| {
                usize::try_from(*id)
                    .ok()
                    .and_then(|index| self.biome_names.get(index))
                    .is_some_and(|name| possible.contains(&name.as_str()))
            })
            .collect::<BTreeSet<_>>();
        let shape = self.dimension_shape;
        biome_source.find_closest_biome_3d(
            &self.source,
            origin,
            &candidates,
            search_radius,
            horizontal_step,
            vertical_step,
            shape.min_y + 1,
            shape.max_y_exclusive(),
        )
    }

    pub fn restored_structure_chunk_data(
        &self,
        value: &NbtSummaryValue,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<NoiseStructureChunkData> {
        let path = format!(
            "noise_settings.{}.restored_structures[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let owns_end_city = self.structures.as_ref().is_some_and(|plan| {
            plan.sets()
                .iter()
                .any(|set| plan.structure_start_owner(set) == StructureStartSetOwner::EndCity)
        });
        if owns_end_city && !crate::chunk::structure_generation::is_empty_structure_document(value)
        {
            self.validate_builtin_end_city_plan(&path)?;
        }
        let Some(plan) = self.structures.as_ref() else {
            if !crate::chunk::structure_generation::is_empty_structure_document(value) {
                return Err(invalid_schema(
                    &path,
                    "a structure-disabled noise source may restore only empty starts and references",
                ));
            }
            return Ok(NoiseStructureChunkData::from_supplied_starts(
                chunk_x,
                chunk_z,
                [],
                [],
            ));
        };
        let startup = self.structure_startup().ok_or_else(|| {
            invalid_schema(
                &path,
                "structure restore requires the frozen datapack generation",
            )
        })?;
        let mut decoded = NoiseStructureChunkData::from_nbt(value, chunk_x, chunk_z, plan, startup);
        if decoded.has_bbb_starts() {
            if let Ok(rings) = self.structure_start_ring_positions() {
                if let Ok(natural) =
                    self.generate_registered_structure_starts_status_task(chunk_x, chunk_z, &rings)
                {
                    decoded.hydrate_bbb_starts(natural);
                }
            }
        }
        Ok(decoded)
    }

    pub fn min_y(&self) -> i32 {
        self.source.min_y()
    }

    pub fn height(&self) -> i32 {
        self.source.height()
    }

    pub fn sea_level(&self) -> i32 {
        self.source.sea_level()
    }

    pub fn cell_width(&self) -> i32 {
        self.source.cell_width()
    }

    pub fn cell_height(&self) -> i32 {
        self.source.cell_height()
    }

    pub fn compute_single_point(&self, x: i32, y: i32, z: i32) -> f64 {
        self.source.sample_uninterpolated(x, y, z)
    }

    /// The final density after `NoiseChunk` has applied interpolation markers.
    pub fn compute_interpolated_point(&self, x: i32, y: i32, z: i32) -> f64 {
        self.source.sample_interpolated(x, y, z)
    }

    /// Computes a concentric-ring structure-set component without claiming a
    /// generated or persisted `StructureStart`.
    pub fn structure_ring_positions_component(
        &self,
        structure_set: &str,
    ) -> SaveResult<Vec<(i32, i32)>> {
        let path = format!(
            "noise_settings.{}.structure_rings.{structure_set}",
            self.source.settings_id()
        );
        let structures = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "structure rings require the built-in Overworld structure-state plan",
            )
        })?;
        let biome_source = self.biome_source.as_ref().ok_or_else(|| {
            invalid_schema(&path, "structure rings require the compiled biome source")
        })?;
        structures
            .ring_positions(structure_set, self.world_seed, biome_source, &self.source)
            .ok_or_else(|| {
                invalid_schema(
                    &path,
                    "structure set is absent or does not use concentric rings",
                )
            })
    }

    #[cfg(test)]
    pub(crate) fn woodland_mansion_structure_decoration_invocation(
        &self,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        self.woodland_mansion_structure_decoration_invocation_for(WOODLAND_MANSION_ID, target_chunk)
    }

    #[cfg(test)]
    pub(crate) fn owns_woodland_mansion_structure(&self, structure_id: &str) -> bool {
        self.structures.as_ref().is_some_and(|plan| {
            plan.structures()
                .get(structure_id)
                .is_some_and(|definition| {
                    definition.type_id == WOODLAND_MANSION_TYPE_ID
                        && definition.step == StructureDecorationStep::SurfaceStructures
                })
                && plan.sets().iter().any(|set| {
                    plan.structure_start_owner(set) == StructureStartSetOwner::WoodlandMansion
                        && set
                            .structures
                            .iter()
                            .any(|selection| selection.structure == structure_id)
                })
        })
    }

    #[cfg(test)]
    pub(crate) fn woodland_mansion_structure_decoration_invocation_for(
        &self,
        structure_id: &str,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = format!(
            "noise_settings.{}.woodland_mansion_structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1]
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "woodland-mansion placement requires the frozen structure plan",
            )
        })?;
        if !self.owns_woodland_mansion_structure(structure_id) {
            return Err(invalid_schema(
                &path,
                "structure is not owned by the woodland-mansion placement family",
            ));
        }
        let invocation = plan
            .decoration_invocations_for_chunk(self.world_seed, target_chunk[0], target_chunk[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from the decoration order")
            })?;
        let expected_step = StructureDecorationStep::SurfaceStructures.ordinal();
        if invocation.step != expected_step {
            return Err(invalid_schema(
                &path,
                format!(
                    "woodland-mansion decoration step is {}, expected SURFACE_STRUCTURES ordinal {expected_step}",
                    invocation.step
                ),
            ));
        }
        Ok(invocation)
    }

    pub fn ruined_portal_structure_decoration_invocation(
        &self,
        target_chunk: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = format!(
            "noise_settings.{}.ruined_portal_structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1]
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "ruined-portal placement requires the frozen structure plan",
            )
        })?;
        if self.settings_driven_structure_family(structure_id)
            != Some(SettingsDrivenStructureFamily::RuinedPortal)
        {
            return Err(invalid_schema(
                &path,
                "structure is not owned by the ruined-portal placement family",
            ));
        }
        let invocation = plan
            .decoration_invocations_for_chunk(self.world_seed, target_chunk[0], target_chunk[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from the decoration order")
            })?;
        let expected_step = StructureDecorationStep::SurfaceStructures.ordinal();
        if invocation.step != expected_step {
            return Err(invalid_schema(
                &path,
                format!(
                    "ruined-portal decoration step is {}, expected SURFACE_STRUCTURES ordinal {expected_step}",
                    invocation.step
                ),
            ));
        }
        Ok(invocation)
    }

    pub fn swamp_hut_structure_decoration_invocation(
        &self,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        self.swamp_hut_structure_decoration_invocation_for(target_chunk, SWAMP_HUT_ID)
    }

    pub fn swamp_hut_structure_decoration_invocation_for(
        &self,
        target_chunk: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = format!(
            "noise_settings.{}.swamp_hut_structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1]
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "swamp-hut placement requires the frozen structure plan",
            )
        })?;
        if !self.owns_swamp_hut_structure(structure_id) {
            return Err(invalid_schema(
                &path,
                "structure is not owned by the swamp-hut placement family",
            ));
        }
        let invocation = plan
            .decoration_invocations_for_chunk(self.world_seed, target_chunk[0], target_chunk[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from the decoration order")
            })?;
        let expected_step = StructureDecorationStep::SurfaceStructures.ordinal();
        if invocation.step != expected_step {
            return Err(invalid_schema(
                &path,
                format!(
                    "swamp-hut decoration step is {}, expected SURFACE_STRUCTURES ordinal {expected_step}",
                    invocation.step
                ),
            ));
        }
        Ok(invocation)
    }

    pub(crate) fn owns_swamp_hut_structure(&self, structure_id: &str) -> bool {
        let Some(plan) = self.structures.as_ref() else {
            return false;
        };
        plan.sets().iter().any(|set| {
            plan.structure_start_owner(set) == StructureStartSetOwner::SwampHut
                && set
                    .structures
                    .iter()
                    .any(|selection| selection.structure == structure_id)
        })
    }

    pub fn stronghold_structure_decoration_invocation(
        &self,
        target_chunk: [i32; 2],
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        self.stronghold_structure_decoration_invocation_for(target_chunk, STRONGHOLD_ID)
    }

    pub fn stronghold_structure_decoration_invocation_for(
        &self,
        target_chunk: [i32; 2],
        structure_id: &str,
    ) -> SaveResult<NoiseStructureDecorationInvocation> {
        let path = format!(
            "noise_settings.{}.stronghold_structure_invocation[{},{}].{structure_id}",
            self.source.settings_id(),
            target_chunk[0],
            target_chunk[1]
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "stronghold placement requires the frozen structure plan",
            )
        })?;
        if !plan.sets().iter().any(|set| {
            plan.structure_start_owner(set) == StructureStartSetOwner::Stronghold
                && set
                    .structures
                    .iter()
                    .any(|selection| selection.structure == structure_id)
        }) {
            return Err(invalid_schema(
                &path,
                "structure is not owned by the stronghold placement family",
            ));
        }
        let invocation = plan
            .decoration_invocations_for_chunk(self.world_seed, target_chunk[0], target_chunk[1])
            .into_iter()
            .find(|invocation| invocation.structure == structure_id)
            .ok_or_else(|| {
                invalid_schema(&path, "structure is absent from the decoration order")
            })?;
        let expected_step = StructureDecorationStep::SurfaceStructures.ordinal();
        if invocation.step != expected_step {
            return Err(invalid_schema(
                &path,
                format!(
                    "stronghold decoration step is {}, expected SURFACE_STRUCTURES ordinal {expected_step}",
                    invocation.step
                ),
            ));
        }
        Ok(invocation)
    }

    #[cfg(test)]
    pub(crate) fn owns_stronghold_structure(&self, structure_id: &str) -> bool {
        let Some(plan) = self.structures.as_ref() else {
            return false;
        };
        plan.sets().iter().any(|set| {
            plan.structure_start_owner(set) == StructureStartSetOwner::Stronghold
                && set
                    .structures
                    .iter()
                    .any(|selection| selection.structure == structure_id)
        })
    }

    /// Generates the jigsaw-owned random-spread half of STRUCTURE_STARTS with
    /// this program's frozen biome source and base-height owner
    /// (`ChunkStatusTasks.java:40-57`, `ChunkGenerator.java:464-589`).
    #[cfg(test)]
    pub(crate) fn generate_jigsaw_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[JigsawStructureStart],
    ) -> SaveResult<JigsawStructureChunkData> {
        let path = format!(
            "noise_settings.{}.structure_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "jigsaw starts require the built-in Overworld structure plan",
            )
        })?;
        self.generate_jigsaw_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    #[cfg(test)]
    pub(super) fn generate_jigsaw_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[JigsawStructureStart],
    ) -> SaveResult<JigsawStructureChunkData> {
        Ok(self
            .structure_generation_context()?
            .generate_jigsaw_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    /// Generates biome-gated mineshaft starts without publishing the status.
    pub fn generate_mineshaft_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[MineshaftStructureStart],
    ) -> SaveResult<Vec<MineshaftStructureStart>> {
        let path = format!(
            "noise_settings.{}.mineshaft_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "mineshaft starts require the frozen Overworld structure plan",
            )
        })?;
        self.generate_mineshaft_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_mineshaft_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[MineshaftStructureStart],
    ) -> SaveResult<Vec<MineshaftStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_mineshaft_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    #[cfg(test)]
    pub(crate) fn owns_mineshaft_structure(&self, structure_id: &str) -> bool {
        let Some(plan) = self.structures.as_ref() else {
            return false;
        };
        plan.sets().iter().any(|set| {
            plan.structure_start_owner(set) == StructureStartSetOwner::Mineshaft
                && set
                    .structures
                    .iter()
                    .any(|selection| selection.structure == structure_id)
        })
    }

    #[cfg(test)]
    pub(crate) fn first_mineshaft_structure_id(&self) -> Option<&str> {
        let plan = self.structures.as_ref()?;
        plan.structure_order()
            .iter()
            .map(String::as_str)
            .find(|structure_id| self.owns_mineshaft_structure(structure_id))
    }

    pub fn preflight_structure_start_owners(&self, generate_structures: bool) -> SaveResult<()> {
        // Disabled world options bypass `createStructures`; `ChunkStatusTasks.java:40-57`.
        if !generate_structures {
            return Ok(());
        }
        let path = format!(
            "noise_settings.{}.structure_status.plan",
            self.source.settings_id()
        );
        let plan = self
            .structures
            .as_ref()
            .ok_or_else(|| invalid_schema(&path, "structure status requires the frozen plan"))?;
        self.preflight_registered_structure_start_owners(plan, &path)
    }

    /// Generates biome-gated shipwreck starts without publishing the status.
    pub fn generate_shipwreck_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[ShipwreckStructureStart],
    ) -> SaveResult<Vec<ShipwreckStructureStart>> {
        let path = format!(
            "noise_settings.{}.shipwreck_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "shipwreck starts require the built-in Overworld structure plan",
            )
        })?;
        self.generate_shipwreck_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_shipwreck_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[ShipwreckStructureStart],
    ) -> SaveResult<Vec<ShipwreckStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_shipwreck_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    /// Generates biome-gated ocean-ruin starts without publishing the status.
    pub fn generate_ocean_ruin_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[OceanRuinStructureStart],
    ) -> SaveResult<Vec<OceanRuinStructureStart>> {
        let path = format!(
            "noise_settings.{}.ocean_ruin_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "ocean-ruin starts require the built-in Overworld structure plan",
            )
        })?;
        self.generate_ocean_ruin_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_ocean_ruin_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[OceanRuinStructureStart],
    ) -> SaveResult<Vec<OceanRuinStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_ocean_ruin_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    /// Generates biome-gated igloo starts without publishing the status.
    pub fn generate_igloo_structure_starts_component(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[IglooStructureStart],
    ) -> SaveResult<Vec<IglooStructureStart>> {
        let path = format!(
            "noise_settings.{}.igloo_starts[{chunk_x},{chunk_z}]",
            self.source.settings_id()
        );
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "igloo starts require the built-in Overworld structure plan",
            )
        })?;
        self.generate_igloo_structure_starts_for_plan_component(
            plan,
            chunk_x,
            chunk_z,
            existing_starts,
        )
    }

    pub(super) fn generate_igloo_structure_starts_for_plan_component(
        &self,
        plan: &NoiseStructurePlan,
        chunk_x: i32,
        chunk_z: i32,
        existing_starts: &[IglooStructureStart],
    ) -> SaveResult<Vec<IglooStructureStart>> {
        Ok(self
            .structure_generation_context()?
            .generate_igloo_structure_starts_for_plan_component(
                plan,
                chunk_x,
                chunk_z,
                existing_starts,
            ))
    }

    #[cfg(test)]
    pub fn first_overworld_woodland_mansion_start_in_window(
        &self,
        radius: i32,
    ) -> SaveResult<Option<([i32; 2], Vec<WoodlandMansionStructureStart>)>> {
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                "noise_settings.woodland_mansion_structure_invocation.search",
                "woodland-mansion search requires the frozen structure plan",
            )
        })?;
        for ring in 0..=radius {
            let chunks = if ring == 0 {
                vec![[0, 0]]
            } else {
                let mut ring_chunks = Vec::with_capacity((ring as usize) * 8);
                for chunk_x in -ring..=ring {
                    ring_chunks.push([chunk_x, -ring]);
                    ring_chunks.push([chunk_x, ring]);
                }
                for chunk_z in (-ring + 1)..ring {
                    ring_chunks.push([-ring, chunk_z]);
                    ring_chunks.push([ring, chunk_z]);
                }
                ring_chunks
            };
            for [chunk_x, chunk_z] in chunks {
                let Some(attempts) = plan.random_spread_attempt_order(
                    WOODLAND_MANSION_SET_ID,
                    self.world_seed,
                    chunk_x,
                    chunk_z,
                ) else {
                    continue;
                };
                if attempts.is_empty() {
                    continue;
                }
                let starts = self.generate_woodland_mansion_structure_starts_component(
                    chunk_x,
                    chunk_z,
                    &[],
                )?;
                if !starts.is_empty() {
                    return Ok(Some(([chunk_x, chunk_z], starts)));
                }
            }
        }
        Ok(None)
    }

    #[cfg(test)]
    pub fn first_overworld_stronghold_start_in_window(
        &self,
        radius: i32,
    ) -> SaveResult<Option<([i32; 2], Vec<StrongholdStructureStart>)>> {
        let plan = self.structures.as_ref().ok_or_else(|| {
            invalid_schema(
                "noise_settings.stronghold_start_search",
                "stronghold search requires the frozen structure plan",
            )
        })?;
        let rings = self.structure_start_ring_positions()?;
        let Some(positions) = rings.get(STRONGHOLD_SET_ID) else {
            return Ok(None);
        };
        const PINNED: [i32; 2] = [-13, -106];
        let mut candidates = positions
            .iter()
            .copied()
            .filter(|pos| *pos == PINNED || (pos[0].abs() <= radius && pos[1].abs() <= radius))
            .collect::<Vec<_>>();
        candidates.sort_by_key(|pos| (*pos != PINNED, pos[0].abs().max(pos[1].abs())));
        let context = self.structure_generation_context()?;
        for pos in candidates {
            let starts = context.generate_stronghold_structure_starts_for_plan_component(
                plan,
                pos[0],
                pos[1],
                &[],
                &rings,
            );
            if !starts.is_empty() {
                return Ok(Some((pos, starts)));
            }
        }
        Ok(None)
    }

    pub(crate) fn owns_ocean_monument_structure(&self, structure_id: &str) -> bool {
        let Some(plan) = self.structures.as_ref() else {
            return false;
        };
        plan.sets().iter().any(|set| {
            plan.structure_start_owner(set) == StructureStartSetOwner::OceanMonument
                && set
                    .structures
                    .iter()
                    .any(|selection| selection.structure == structure_id)
        })
    }

    /// Generates the fresh base column owned by `fillFromNoise` and the
    /// built-in multi-noise biome fill, before surface, carver and feature stages.
    pub fn generate_fresh_base_chunk(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<NoiseGeneratedChunk> {
        self.generate_fresh_base_chunk_with_beardifier(
            chunk_x,
            chunk_z,
            &JigsawBeardifier::default(),
        )
    }

    fn generate_fresh_base_chunk_with_beardifier(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        beardifier: &JigsawBeardifier,
    ) -> SaveResult<NoiseGeneratedChunk> {
        self.generate_fresh_base_chunk_with_beardifier_and_biome_blender(
            chunk_x, chunk_z, beardifier, None,
        )
    }

    fn generate_fresh_base_chunk_with_beardifier_and_biome_blender(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        beardifier: &JigsawBeardifier,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<NoiseGeneratedChunk> {
        self.generate_fresh_base_chunk_with_generation_shape(
            chunk_x,
            chunk_z,
            beardifier,
            biome_blender,
            None,
        )
    }

    pub(crate) fn fill_loading_biomes(
        &self,
        chunk: (i32, i32),
        column: &mut ChunkColumn,
        beardifier: &JigsawBeardifier,
        blender: &BlendingResolver,
    ) -> SaveResult<()> {
        let start_x = chunk
            .0
            .checked_mul(16)
            .ok_or_else(|| invalid_schema("loading_biomes", "chunk x overflows"))?;
        let start_z = chunk
            .1
            .checked_mul(16)
            .ok_or_else(|| invalid_schema("loading_biomes", "chunk z overflows"))?;
        let min_y = self.source.min_y().max(self.dimension_shape.min_y);
        let max_y = self
            .source
            .min_y()
            .saturating_add(self.source.height())
            .min(self.dimension_shape.max_y_exclusive());
        let mut sampler = NoiseChunkSampler::for_chunk_with_beardifier_and_blender(
            &self.source,
            start_x,
            start_z,
            min_y,
            max_y - min_y,
            beardifier.clone(),
            Arc::new(blender.clone()),
        );
        self.fill_column_biomes(chunk.0, chunk.1, column, &mut sampler, Some(blender))
    }

    fn fill_column_biomes(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        column: &mut ChunkColumn,
        sampler: &mut NoiseChunkSampler<'_>,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<()> {
        let path = "noise_biomes";
        let biome_source = self
            .biome_source
            .as_ref()
            .ok_or_else(|| invalid_schema(path, "biome source is absent"))?;
        let min_quart_y = column.min_y().div_euclid(4);
        let chunk_quart_x = chunk_x
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(path, "chunk x overflows"))?;
        let chunk_quart_z = chunk_z
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(path, "chunk z overflows"))?;
        let mut search = biome_source.create_search();
        // ChunkAccess.fillBiomesFromNoise and LevelChunkSection.fillBiomesFromNoise
        // (`ChunkAccess.java:442-452`, `LevelChunkSection.java:200-214`).
        for section in 0..column.section_count() {
            let quart_min_y = min_quart_y + section as i32 * 4;
            for local_x in 0..4 {
                for local_y in 0..4 {
                    for local_z in 0..4 {
                        let quart_x = chunk_quart_x + local_x;
                        let quart_y = quart_min_y + local_y;
                        let quart_z = chunk_quart_z + local_z;
                        let biome = biome_blender
                            .and_then(|blender| blender.blend(quart_x, quart_y, quart_z))
                            .unwrap_or_else(|| {
                                biome_source.sample_with(
                                    sampler,
                                    quart_x,
                                    quart_y,
                                    quart_z,
                                    &mut search,
                                )
                            });
                        column
                            .set_noise_biome(local_x, quart_min_y + local_y, local_z, biome)
                            .map_err(|error| invalid_schema(path, error.to_string()))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_fresh_base_chunk_with_generation_shape(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        beardifier: &JigsawBeardifier,
        biome_blender: Option<&BlendingResolver>,
        generation_shape: Option<(i32, i32)>,
    ) -> SaveResult<NoiseGeneratedChunk> {
        let path = format!("noise_settings.{}.fresh_base", self.source.settings_id());
        let biome_source = self.biome_source.as_ref().ok_or_else(|| {
            invalid_schema(
                &format!("{path}.biome_source"),
                "fresh chunk generation only owns the built-in Nether and Overworld multi-noise presets and The End source",
            )
        })?;
        let chunk_start_x = chunk_x
            .checked_mul(16)
            .ok_or_else(|| invalid_schema(&path, "chunk x is outside block-coordinate range"))?;
        let chunk_start_z = chunk_z
            .checked_mul(16)
            .ok_or_else(|| invalid_schema(&path, "chunk z is outside block-coordinate range"))?;
        // `fillSlice` samples the trailing cell corner at block +16 (`NoiseChunk.java:235-266`)
        // and the aquifer grid starts at block -5 (`Aquifer.java:120-129`); a wrapped
        // window would make Vanilla's grid size negative.
        for (axis, start) in [("x", chunk_start_x), ("z", chunk_start_z)] {
            start.checked_add(16).ok_or_else(|| {
                invalid_schema(
                    &path,
                    format!("chunk {axis} noise-corner range overflows block coordinates"),
                )
            })?;
            start.checked_sub(5).ok_or_else(|| {
                invalid_schema(
                    &path,
                    format!("chunk {axis} aquifer range underflows block coordinates"),
                )
            })?;
        }
        let full_noise_min_y = self.source.min_y().max(self.dimension_shape.min_y);
        let full_noise_max_y = self
            .source
            .min_y()
            .saturating_add(self.source.height())
            .min(self.dimension_shape.max_y_exclusive());
        let (noise_min_y, noise_height) = generation_shape.unwrap_or((
            full_noise_min_y,
            full_noise_max_y.saturating_sub(full_noise_min_y),
        ));
        let noise_max_y = noise_min_y.saturating_add(noise_height);
        if noise_min_y < full_noise_min_y || noise_max_y > full_noise_max_y {
            return Err(invalid_schema(
                &path,
                "generation accessor is outside the clamped noise shape",
            ));
        }
        let cell_width = self.source.cell_width();
        let cell_height = self.source.cell_height();
        if noise_height <= 0
            || noise_min_y.rem_euclid(cell_height) != 0
            || noise_height.rem_euclid(cell_height) != 0
            || 16_i32.rem_euclid(cell_width) != 0
        {
            return Err(invalid_schema(
                &path,
                "clamped noise shape must contain complete generation cells",
            ));
        }
        let section_count = usize::try_from(self.dimension_shape.section_count())
            .map_err(|_| invalid_schema(&path, "dimension section count is invalid"))?;
        let min_quart_y = self.dimension_shape.min_y.div_euclid(4);
        let chunk_quart_x = chunk_x
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(&path, "chunk x is outside quart-coordinate range"))?;
        let chunk_quart_z = chunk_z
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(&path, "chunk z is outside quart-coordinate range"))?;
        let mut sampler = if let Some(blender) = biome_blender {
            NoiseChunkSampler::for_chunk_with_beardifier_and_blender(
                &self.source,
                chunk_start_x,
                chunk_start_z,
                noise_min_y,
                noise_height,
                beardifier.clone(),
                Arc::new(blender.clone()),
            )
        } else {
            NoiseChunkSampler::for_chunk_with_beardifier(
                &self.source,
                chunk_start_x,
                chunk_start_z,
                noise_min_y,
                noise_height,
                beardifier.clone(),
            )
        };
        // The same Blender wraps the router before the first biome/aquifer
        // sample (`NoiseChunk.java:141-175`).
        let mut aquifer = Aquifer::for_noise_chunk(
            &mut sampler,
            chunk_start_x,
            chunk_start_z,
            noise_min_y,
            noise_height,
        );
        let mut search = biome_source.create_search();
        let initial_biome = biome_blender
            .and_then(|blender| blender.blend(chunk_quart_x, min_quart_y, chunk_quart_z))
            .unwrap_or_else(|| {
                biome_source.sample_with(
                    &mut sampler,
                    chunk_quart_x,
                    min_quart_y,
                    chunk_quart_z,
                    &mut search,
                )
            });
        let mut column = ChunkColumn::new_with_biome_registry(
            self.dimension_shape.min_y,
            section_count,
            self.source.air_block_state_id(),
            initial_biome,
            biome_source.registry_len(),
        );
        self.fill_column_biomes(chunk_x, chunk_z, &mut column, &mut sampler, biome_blender)?;
        let (ocean_floor_wg, world_surface_wg, postprocessing_positions) = self
            .fill_column_noise_with_sampler(
                &mut sampler,
                &mut aquifer,
                [chunk_start_x, chunk_start_z],
                noise_min_y,
                noise_height,
                beardifier,
                &mut column,
            );

        Ok(NoiseGeneratedChunk {
            chunk_x,
            chunk_z,
            column,
            ocean_floor_wg,
            world_surface_wg,
            postprocessing_positions,
            carving_mask: None,
        })
    }

    /// Generates through Vanilla's SURFACE status for the three built-in noise
    /// settings. Carvers, features, persistence and serving remain outside
    /// this owner.
    pub fn generate_fresh_surface_chunk(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> SaveResult<NoiseGeneratedChunk> {
        self.generate_fresh_surface_chunk_with_beardifier(
            chunk_x,
            chunk_z,
            &JigsawBeardifier::default(),
        )
    }

    fn generate_fresh_surface_chunk_with_beardifier(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        beardifier: &JigsawBeardifier,
    ) -> SaveResult<NoiseGeneratedChunk> {
        self.generate_fresh_surface_chunk_with_beardifier_and_biome_blender(
            chunk_x, chunk_z, beardifier, None,
        )
    }

    fn generate_fresh_surface_chunk_with_beardifier_and_biome_blender(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        beardifier: &JigsawBeardifier,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<NoiseGeneratedChunk> {
        let mut chunk = self.generate_fresh_base_chunk_with_beardifier_and_biome_blender(
            chunk_x,
            chunk_z,
            beardifier,
            biome_blender,
        )?;
        self.apply_surface_with_biome_blender(&mut chunk, biome_blender)?;
        Ok(chunk)
    }
    fn generate_fresh_carved_chunk_with_beardifier(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        beardifier: &JigsawBeardifier,
    ) -> SaveResult<NoiseGeneratedChunk> {
        self.generate_fresh_carved_chunk_with_beardifier_and_biome_blender(
            chunk_x, chunk_z, beardifier, None,
        )
    }

    fn generate_fresh_carved_chunk_with_beardifier_and_biome_blender(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        beardifier: &JigsawBeardifier,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<NoiseGeneratedChunk> {
        let mut chunk = self.generate_fresh_surface_chunk_with_beardifier_and_biome_blender(
            chunk_x,
            chunk_z,
            beardifier,
            biome_blender,
        )?;
        self.apply_carvers_to_chunk(&mut chunk)?;
        Ok(chunk)
    }

    fn apply_carvers_to_chunk(&self, chunk: &mut NoiseGeneratedChunk) -> SaveResult<()> {
        let path = format!("noise_settings.{}.carvers", self.source.settings_id());
        if self.carvers_are_verified_empty {
            chunk.carving_mask = Some(Vec::new());
            return Ok(());
        }
        let carvers = self.carvers.as_ref().ok_or_else(|| {
            invalid_schema(&path, "carver generation requires a frozen carver program")
        })?;
        carvers
            .validate_chunk(chunk.chunk_x, chunk.chunk_z)
            .map_err(|error| invalid_schema(&path, error.to_string()))?;
        let surface = self.surface.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "carver generation requires the built-in surface program",
            )
        })?;
        let biome_source = self.biome_source.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "carver generation requires the built-in biome source",
            )
        })?;
        chunk.carving_mask = Some(
            carvers
                .apply(
                    &self.source,
                    biome_source,
                    surface,
                    chunk.chunk_x,
                    chunk.chunk_z,
                    &mut chunk.column,
                    &mut chunk.ocean_floor_wg,
                    &mut chunk.world_surface_wg,
                    &mut chunk.postprocessing_positions,
                )
                .map_err(|error| invalid_schema(&path, error.to_string()))?,
        );
        Ok(())
    }
}

pub(crate) struct SavedChunkResume<'a> {
    pub(crate) chunk: [i32; 2],
    pub(crate) saved_status: bbb_server_persistence::Java26ChunkStatus,
    pub(crate) saved_column: &'a ChunkColumn,
    pub(crate) saved_postprocessing: Vec<(i32, i32, i32)>,
}

impl NoiseFinalDensityProgram {
    pub(crate) fn resume_saved_chunk_to_carvers(
        &self,
        saved: SavedChunkResume<'_>,
        beardifier: &JigsawBeardifier,
        biome_blender: &BlendingResolver,
    ) -> SaveResult<NoiseGeneratedChunk> {
        self.resume_saved_chunk_to_status(
            saved,
            bbb_server_persistence::Java26ChunkStatus::Carvers,
            beardifier,
            biome_blender,
        )
    }

    pub(crate) fn resume_saved_chunk_to_status(
        &self,
        saved: SavedChunkResume<'_>,
        target_status: bbb_server_persistence::Java26ChunkStatus,
        beardifier: &JigsawBeardifier,
        biome_blender: &BlendingResolver,
    ) -> SaveResult<NoiseGeneratedChunk> {
        use bbb_server_persistence::Java26ChunkStatus;
        let SavedChunkResume {
            chunk: [chunk_x, chunk_z],
            saved_status,
            saved_column,
            saved_postprocessing,
        } = saved;
        // ChunkStatusTasks.java:76-129: only statuses after the persisted status execute.
        let mut chunk = if saved_status < Java26ChunkStatus::Noise {
            let mut generated = self.generate_fresh_base_chunk_with_beardifier_and_biome_blender(
                chunk_x,
                chunk_z,
                beardifier,
                Some(biome_blender),
            )?;
            if saved_status >= Java26ChunkStatus::Biomes {
                let min_quart_y = saved_column.min_y().div_euclid(4);
                let max_quart_y = min_quart_y + saved_column.section_count() as i32 * 4;
                for y in min_quart_y..max_quart_y {
                    for x in 0..4 {
                        for z in 0..4 {
                            let biome = saved_column
                                .get_noise_biome(x, y, z)
                                .expect("saved biome cell");
                            generated
                                .column
                                .set_noise_biome(x, y, z, biome)
                                .map_err(|error| {
                                    invalid_schema("saved generation.biomes", error.to_string())
                                })?;
                        }
                    }
                }
            }
            let mut positions = saved_postprocessing;
            positions.append(&mut generated.postprocessing_positions);
            generated.postprocessing_positions = positions;
            generated
        } else {
            let (ocean_floor_wg, world_surface_wg) = prime_worldgen_heightmaps(saved_column);
            NoiseGeneratedChunk {
                chunk_x,
                chunk_z,
                column: saved_column.clone(),
                ocean_floor_wg,
                world_surface_wg,
                postprocessing_positions: saved_postprocessing,
                carving_mask: None,
            }
        };
        if target_status >= Java26ChunkStatus::Surface && saved_status < Java26ChunkStatus::Surface
        {
            self.apply_surface_with_biome_blender(&mut chunk, Some(biome_blender))?;
        }
        if target_status >= Java26ChunkStatus::Carvers {
            self.apply_carvers_to_chunk(&mut chunk)?;
        }
        Ok(chunk)
    }

    fn apply_surface_with_biome_blender(
        &self,
        chunk: &mut NoiseGeneratedChunk,
        biome_blender: Option<&BlendingResolver>,
    ) -> SaveResult<()> {
        let path = format!("noise_settings.{}.fresh_surface", self.source.settings_id());
        let surface = self.surface.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "surface generation owns only the fully compiled built-in Overworld, Nether and End programs",
            )
        })?;
        let biome_source = self.biome_source.as_ref().ok_or_else(|| {
            invalid_schema(
                &path,
                "surface generation requires an owned built-in biome source",
            )
        })?;
        let min_quart_y = chunk.column.min_y().div_euclid(4);
        let quart_height = i32::try_from(chunk.column.section_count())
            .map_err(|_| invalid_schema(&path, "column section count is outside i32"))?
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(&path, "column quart height overflows"))?;
        let max_quart_y = min_quart_y
            .checked_add(quart_height)
            .ok_or_else(|| invalid_schema(&path, "column quart range overflows"))?;
        let chunk_quart_x = chunk
            .chunk_x
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(&path, "surface chunk quart x overflows"))?;
        let chunk_quart_z = chunk
            .chunk_z
            .checked_mul(4)
            .ok_or_else(|| invalid_schema(&path, "surface chunk quart z overflows"))?;
        let mut assigned = Vec::with_capacity(
            usize::try_from(quart_height).expect("validated positive column quart height") * 16,
        );
        for quart_y in min_quart_y..max_quart_y {
            for local_x in 0..4 {
                for local_z in 0..4 {
                    assigned.push(
                        chunk
                            .column
                            .get_noise_biome(local_x, quart_y, local_z)
                            .ok_or_else(|| {
                                invalid_schema(
                                    &path,
                                    "assigned BIOMES column is incomplete before SURFACE",
                                )
                            })?,
                    );
                }
            }
        }
        surface.apply_with_biome_override(
            &self.source,
            biome_source,
            chunk.chunk_x,
            chunk.chunk_z,
            &mut chunk.column,
            &mut chunk.ocean_floor_wg,
            &mut chunk.world_surface_wg,
            &mut chunk.postprocessing_positions,
            |quart_x, quart_y, quart_z| {
                let local_x = quart_x - chunk_quart_x;
                let local_z = quart_z - chunk_quart_z;
                if (0..4).contains(&local_x) && (0..4).contains(&local_z) {
                    let clamped_y = quart_y.clamp(min_quart_y, max_quart_y - 1);
                    let index = usize::try_from(clamped_y - min_quart_y).unwrap() * 16
                        + usize::try_from(local_x).unwrap() * 4
                        + usize::try_from(local_z).unwrap();
                    Some(assigned[index])
                } else {
                    biome_blender.and_then(|blender| blender.blend(quart_x, quart_y, quart_z))
                }
            },
        );
        Ok(())
    }

    // `NoiseBasedChunkGenerator.java:119-120` delegates to its column scan at
    // `NoiseBasedChunkGenerator.java:191-264`; predicates are `Heightmap.java:144-155`.
    pub(crate) fn fresh_base_height(&self, x: i32, z: i32, heightmap_kind_id: i32) -> Option<i32> {
        let noise_min_y = self.source.min_y().max(self.dimension_shape.min_y);
        let noise_max_y = self
            .source
            .min_y()
            .checked_add(self.source.height())?
            .min(self.dimension_shape.max_y_exclusive());
        let noise_height = noise_max_y.checked_sub(noise_min_y)?;
        if noise_height <= 0 {
            return Some(self.dimension_shape.min_y);
        }
        let cell_height = self.source.cell_height();
        if noise_min_y.rem_euclid(cell_height) != 0 || noise_height.rem_euclid(cell_height) != 0 {
            return None;
        }
        // Single-column height probes use disabled aquifers so structure placement does
        // not construct an 81-point preliminary surface grid per (x, z) query.
        let mut aquifer = Aquifer::Disabled;
        let matches: fn(i32) -> bool = match heightmap_kind_id {
            VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID => |state| !block_state_is_air(state),
            VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID => block_state_blocks_motion,
            _ => return None,
        };
        let mut sampler =
            NoiseChunkSampler::for_column(&self.source, x, z, noise_min_y, noise_height);
        let beardifier = JigsawBeardifier::default();
        sampler
            .iterate_column(x, z, &beardifier, |_sampler, y, density| {
                let aquifer_decision = aquifer.compute(&self.source, x, y, z, density);
                let state = aquifer_decision
                    .state
                    .unwrap_or(self.source.default_block_state_id());
                matches(state).then(|| y.checked_add(1))
            })
            .unwrap_or(Some(self.dimension_shape.min_y))
    }

    /// Samples one full `(x, z)` base noise column in a single cell-interpolated pass.
    pub(crate) fn fresh_base_column(&self, x: i32, z: i32) -> Option<(i32, Vec<i32>)> {
        let noise_min_y = self.source.min_y().max(self.dimension_shape.min_y);
        let noise_max_y = self
            .source
            .min_y()
            .checked_add(self.source.height())?
            .min(self.dimension_shape.max_y_exclusive());
        let noise_height = noise_max_y.checked_sub(noise_min_y)?;
        if noise_height <= 0 {
            return Some((noise_min_y, Vec::new()));
        }
        let cell_height = self.source.cell_height();
        if noise_min_y.rem_euclid(cell_height) != 0 || noise_height.rem_euclid(cell_height) != 0 {
            return None;
        }
        let mut states =
            vec![self.source.air_block_state_id(); usize::try_from(noise_height).ok()?];
        let mut aquifer = Aquifer::Disabled;
        let mut sampler =
            NoiseChunkSampler::for_column(&self.source, x, z, noise_min_y, noise_height);
        let beardifier = JigsawBeardifier::default();
        let default_block = self.source.default_block_state_id();
        let _ = sampler.iterate_column(x, z, &beardifier, |_sampler, y, density| {
            let decision = aquifer.compute(&self.source, x, y, z, density);
            let state = decision.state.unwrap_or(default_block);
            if let Ok(index) = usize::try_from(y - noise_min_y) {
                if let Some(slot) = states.get_mut(index) {
                    *slot = state;
                }
            }
            None::<()>
        });
        Some((noise_min_y, states))
    }

    #[cfg(test)]
    pub fn generate_material_chunk(
        &self,
        chunk_start_x: i32,
        chunk_start_z: i32,
        noise_min_y: i32,
        noise_height: i32,
    ) -> NoiseMaterialChunk {
        self.generate_material_chunk_with_beardifier(
            chunk_start_x,
            chunk_start_z,
            noise_min_y,
            noise_height,
            &JigsawBeardifier::default(),
        )
    }

    #[cfg(test)]
    pub fn generate_material_chunk_with_beardifier(
        &self,
        chunk_start_x: i32,
        chunk_start_z: i32,
        noise_min_y: i32,
        noise_height: i32,
        beardifier: &JigsawBeardifier,
    ) -> NoiseMaterialChunk {
        let mut sampler = NoiseChunkSampler::for_chunk_with_beardifier(
            &self.source,
            chunk_start_x,
            chunk_start_z,
            noise_min_y,
            noise_height,
            beardifier.clone(),
        );
        let mut aquifer = Aquifer::for_noise_chunk(
            &mut sampler,
            chunk_start_x,
            chunk_start_z,
            noise_min_y,
            noise_height,
        );
        self.generate_material_chunk_with_sampler(
            &mut sampler,
            &mut aquifer,
            [chunk_start_x, chunk_start_z],
            noise_min_y,
            noise_height,
            beardifier,
        )
    }

    /// `NoiseBasedChunkGenerator.doFill` driving one `NoiseChunk`.
    #[allow(clippy::too_many_arguments)]
    fn fill_column_noise_with_sampler(
        &self,
        sampler: &mut NoiseChunkSampler<'_>,
        aquifer: &mut Aquifer,
        [chunk_start_x, chunk_start_z]: [i32; 2],
        noise_min_y: i32,
        noise_height: i32,
        beardifier: &JigsawBeardifier,
        column: &mut ChunkColumn,
    ) -> NoiseFillSummary {
        use bbb_server_types::chunk_data::{section_index, SectionBlocks, SECTION_VOLUME};

        enum SectionBuilder {
            Uniform(i32),
            Indirect {
                palette: Vec<i32>,
                indices: Box<[u16; SECTION_VOLUME]>,
                last_state: i32,
                last_index: u16,
            },
        }

        impl SectionBuilder {
            #[inline]
            fn set(&mut self, lx: usize, ly: usize, lz: usize, state: i32) {
                match self {
                    Self::Uniform(current) => {
                        if state != *current {
                            let mut indices = Box::new([0_u16; SECTION_VOLUME]);
                            indices[section_index(lx, ly, lz)] = 1;
                            *self = Self::Indirect {
                                palette: vec![*current, state],
                                indices,
                                last_state: state,
                                last_index: 1,
                            };
                        }
                    }
                    Self::Indirect {
                        palette,
                        indices,
                        last_state,
                        last_index,
                    } => {
                        let palette_index = if state == *last_state {
                            *last_index
                        } else {
                            let idx = match palette.iter().position(|&entry| entry == state) {
                                Some(existing) => existing as u16,
                                None => {
                                    let idx = palette.len() as u16;
                                    palette.push(state);
                                    idx
                                }
                            };
                            *last_state = state;
                            *last_index = idx;
                            idx
                        };
                        indices[section_index(lx, ly, lz)] = palette_index;
                    }
                }
            }
        }

        let cell_width = self.source.cell_width();
        let cell_height = self.source.cell_height();
        let cell_count_xz = 16 / cell_width;
        let cell_min_y = noise_min_y.div_euclid(cell_height);
        let cell_count_y = noise_height / cell_height;
        let ore = OreVein::new(&self.source);
        let air = self.source.air_block_state_id();
        let default_block = self.source.default_block_state_id();
        let column_min_y = column.min_y();
        let mut builders = (0..column.section_count())
            .map(|_| SectionBuilder::Uniform(air))
            .collect::<Vec<_>>();
        let mut ocean_floor_wg = [self.dimension_shape.min_y; 256];
        let mut world_surface_wg = [self.dimension_shape.min_y; 256];
        let mut postprocessing_positions = Vec::new();

        // `NoiseChunk` installs aquifer then ore before default block, and
        // `doFill` visits in this order (`NoiseChunk.java:141-165`,
        // `NoiseBasedChunkGenerator.java:399-449`).
        sampler.initialize_for_first_cell_x();
        for cell_x in 0..cell_count_xz {
            sampler.advance_cell_x(cell_x);
            for cell_z in 0..cell_count_xz {
                for cell_y in (0..cell_count_y).rev() {
                    sampler.select_cell_yz(cell_y, cell_z, beardifier);
                    for in_y in (0..cell_height).rev() {
                        let y = (cell_min_y + cell_y) * cell_height + in_y;
                        sampler.update_for_y(y, f64::from(in_y) / f64::from(cell_height));
                        let section_idx = usize::try_from((y - column_min_y).div_euclid(16))
                            .expect("validated noise y stays within column sections");
                        let local_y = (y - column_min_y).rem_euclid(16) as usize;
                        let builder = &mut builders[section_idx];
                        for in_x in 0..cell_width {
                            let local_x = cell_x * cell_width + in_x;
                            let x = chunk_start_x + local_x;
                            sampler.update_for_x(x, f64::from(in_x) / f64::from(cell_width));
                            for in_z in 0..cell_width {
                                let local_z = cell_z * cell_width + in_z;
                                let z = chunk_start_z + local_z;
                                sampler.update_for_z(z, f64::from(in_z) / f64::from(cell_width));
                                let density = sampler.interpolated_density();
                                let aquifer_decision =
                                    aquifer.compute_with_sampler(sampler, x, y, z, density);
                                let state = aquifer_decision
                                    .state
                                    .or_else(|| {
                                        ore.as_ref()
                                            .and_then(|ore| ore.compute_with(sampler, x, y, z))
                                    })
                                    .unwrap_or(default_block);
                                builder.set(local_x as usize, local_y, local_z as usize, state);
                                let height_index = (local_x + local_z * 16) as usize;
                                let first_available = y + 1;
                                if block_state_blocks_motion(state) {
                                    ocean_floor_wg[height_index] =
                                        ocean_floor_wg[height_index].max(first_available);
                                }
                                if !block_state_is_air(state) {
                                    world_surface_wg[height_index] =
                                        world_surface_wg[height_index].max(first_available);
                                }
                                if aquifer_decision.schedule_fluid_update
                                    && !block_state_fluid_state(state).is_empty()
                                {
                                    // `NoiseBasedChunkGenerator.java:430-445` marks every
                                    // nonempty fluid state, including waterlogged blocks.
                                    postprocessing_positions.push((x, y, z));
                                }
                            }
                        }
                    }
                }
            }
            sampler.swap_slices();
        }
        sampler.stop_interpolation();

        for (section_idx, builder) in builders.into_iter().enumerate() {
            match builder {
                SectionBuilder::Uniform(state) if state == air => {}
                SectionBuilder::Uniform(state) => {
                    column.replace_section_blocks(section_idx, SectionBlocks::Single(state));
                }
                SectionBuilder::Indirect {
                    palette, indices, ..
                } => {
                    column.replace_section_blocks(
                        section_idx,
                        SectionBlocks::from_palette_indices(palette, &indices),
                    );
                }
            }
        }
        (ocean_floor_wg, world_surface_wg, postprocessing_positions)
    }

    #[cfg(test)]
    fn generate_material_chunk_with_sampler(
        &self,
        sampler: &mut NoiseChunkSampler<'_>,
        aquifer: &mut Aquifer,
        [chunk_start_x, chunk_start_z]: [i32; 2],
        noise_min_y: i32,
        noise_height: i32,
        beardifier: &JigsawBeardifier,
    ) -> NoiseMaterialChunk {
        let cell_width = self.source.cell_width();
        let cell_height = self.source.cell_height();
        let cell_count_xz = 16 / cell_width;
        let cell_min_y = noise_min_y.div_euclid(cell_height);
        let cell_count_y = noise_height / cell_height;
        let ore = OreVein::new(&self.source);
        let mut material = NoiseMaterialChunk {
            min_y: noise_min_y,
            states: vec![
                self.source.air_block_state_id();
                usize::try_from(noise_height * 16 * 16)
                    .expect("validated noise height fits a material chunk")
            ],
            ocean_floor_wg: [self.dimension_shape.min_y; 256],
            world_surface_wg: [self.dimension_shape.min_y; 256],
            postprocessing_positions: Vec::new(),
            pressure_barrier_count: 0,
        };

        // `NoiseChunk` installs aquifer then ore before default block, and
        // `doFill` visits in this order (`NoiseChunk.java:141-165`,
        // `NoiseBasedChunkGenerator.java:399-449`).
        sampler.initialize_for_first_cell_x();
        for cell_x in 0..cell_count_xz {
            sampler.advance_cell_x(cell_x);
            for cell_z in 0..cell_count_xz {
                for cell_y in (0..cell_count_y).rev() {
                    sampler.select_cell_yz(cell_y, cell_z, beardifier);
                    for in_y in (0..cell_height).rev() {
                        let y = (cell_min_y + cell_y) * cell_height + in_y;
                        sampler.update_for_y(y, f64::from(in_y) / f64::from(cell_height));
                        for in_x in 0..cell_width {
                            let local_x = cell_x * cell_width + in_x;
                            let x = chunk_start_x + local_x;
                            sampler.update_for_x(x, f64::from(in_x) / f64::from(cell_width));
                            for in_z in 0..cell_width {
                                let local_z = cell_z * cell_width + in_z;
                                let z = chunk_start_z + local_z;
                                sampler.update_for_z(z, f64::from(in_z) / f64::from(cell_width));
                                let density = sampler.interpolated_density();
                                let aquifer_decision =
                                    aquifer.compute_with_sampler(sampler, x, y, z, density);
                                let material_rule = aquifer_decision.state.or_else(|| {
                                    ore.as_ref()
                                        .and_then(|ore| ore.compute_with(sampler, x, y, z))
                                });
                                if density <= 0.0 && material_rule.is_none() {
                                    material.pressure_barrier_count += 1;
                                }
                                let state =
                                    material_rule.unwrap_or(self.source.default_block_state_id());
                                material.set_state(local_x, y, local_z, state);
                                let height_index = (local_x + local_z * 16) as usize;
                                let first_available = y + 1;
                                if block_state_blocks_motion(state) {
                                    material.ocean_floor_wg[height_index] =
                                        material.ocean_floor_wg[height_index].max(first_available);
                                }
                                if !block_state_is_air(state) {
                                    material.world_surface_wg[height_index] = material
                                        .world_surface_wg[height_index]
                                        .max(first_available);
                                }
                                if aquifer_decision.schedule_fluid_update
                                    && !block_state_fluid_state(state).is_empty()
                                {
                                    // `NoiseBasedChunkGenerator.java:430-445` marks every
                                    // nonempty fluid state, including waterlogged blocks.
                                    material.postprocessing_positions.push((x, y, z));
                                }
                            }
                        }
                    }
                }
            }
            sampler.swap_slices();
        }
        sampler.stop_interpolation();
        material
    }
}

/// `Biome.coldEnoughToSnow` (`Biome.java:172-178`).
pub(crate) fn biome_cold_enough_to_snow(
    registry: &LoadedDynamicRegistry,
    climates: &[BiomeClimate],
    biome_id: &str,
    pos: BlockPos,
    sea_level: i32,
) -> Option<bool> {
    let index = registry
        .elements
        .iter()
        .position(|element| element.id == biome_id)?;
    let climate = climates.get(index)?;
    Some(
        height_adjusted_temperature(
            climate.temperature,
            climate.modifier,
            pos.x,
            pos.y,
            pos.z,
            sea_level,
        ) < 0.15,
    )
}

#[cfg(test)]
pub struct NoiseMaterialChunk {
    min_y: i32,
    states: Vec<i32>,
    ocean_floor_wg: [i32; 256],
    world_surface_wg: [i32; 256],
    postprocessing_positions: Vec<(i32, i32, i32)>,
    pressure_barrier_count: usize,
}

#[cfg(test)]
impl NoiseMaterialChunk {
    fn state(&self, x: i32, y: i32, z: i32) -> i32 {
        self.states[self.index(x, y, z)]
    }

    fn set_state(&mut self, x: i32, y: i32, z: i32, state: i32) {
        let index = self.index(x, y, z);
        self.states[index] = state;
    }

    fn index(&self, x: i32, y: i32, z: i32) -> usize {
        usize::try_from(((y - self.min_y) * 16 + z) * 16 + x)
            .expect("material coordinates stay inside the generated chunk")
    }
}

/// One fresh noise base column plus the two heightmaps `fillFromNoise` primes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoiseGeneratedChunk {
    chunk_x: i32,
    chunk_z: i32,
    column: ChunkColumn,
    ocean_floor_wg: [i32; 256],
    world_surface_wg: [i32; 256],
    postprocessing_positions: Vec<(i32, i32, i32)>,
    carving_mask: Option<Vec<i64>>,
}

impl NoiseGeneratedChunk {
    pub const fn chunk_x(&self) -> i32 {
        self.chunk_x
    }

    pub const fn chunk_z(&self) -> i32 {
        self.chunk_z
    }

    pub const fn column(&self) -> &ChunkColumn {
        &self.column
    }

    pub fn into_column(self) -> ChunkColumn {
        self.column
    }

    pub fn into_persistence_parts(
        self,
    ) -> SaveResult<(i32, i32, ChunkColumn, ChunkGenerationState)> {
        let path = format!(
            "noise chunk ({},{}) CARVERS persistence",
            self.chunk_x, self.chunk_z
        );
        let carving_mask = self.carving_mask.ok_or_else(|| {
            invalid_schema(&path, "the chunk has not completed the CARVERS stage")
        })?;
        Ok((
            self.chunk_x,
            self.chunk_z,
            self.column,
            ChunkGenerationState::Carvers {
                postprocessing_positions: self.postprocessing_positions,
                carving_mask,
            },
        ))
    }

    /// `Heightmap.getFirstAvailable` for the two world-generation maps.
    pub fn heightmap_first_available(&self, heightmap_kind_id: i32, x: i32, z: i32) -> Option<i32> {
        if !(0..16).contains(&x) || !(0..16).contains(&z) {
            return None;
        }
        let values = match heightmap_kind_id {
            VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID => &self.ocean_floor_wg,
            VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID => &self.world_surface_wg,
            _ => return None,
        };
        Some(values[(x + z * 16) as usize])
    }

    pub fn postprocessing_positions(&self) -> &[(i32, i32, i32)] {
        &self.postprocessing_positions
    }

    /// Canonical `BitSet.toLongArray`; `None` before the CARVERS stage.
    pub fn carving_mask(&self) -> Option<&[i64]> {
        self.carving_mask.as_deref()
    }
}

// `Heightmap.Types` WORLD_SURFACE_WG / OCEAN_FLOOR_WG share the final kinds' predicates.
fn prime_worldgen_heightmaps(column: &ChunkColumn) -> ([i32; 256], [i32; 256]) {
    let primed = bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps::prime(column);
    let values = |kind| *primed.values(kind).expect("a final heightmap kind");
    (
        values(bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID),
        values(bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID),
    )
}

/// Compiles the `NoiseRouter.finalDensity` program named by one saved stem.
///
/// `NoiseBasedChunkGenerator.CODEC` reads `biome_source` then a registry-file
/// `settings` holder (`NoiseBasedChunkGenerator.java:51-67`). Save-associated
/// Registry ops emit either a named holder or the direct inline codec branch.
/// The shared startup token keeps feature registries and templates on the same
/// immutable load generation; reload must compile a replacement program.
pub fn compile_noise_final_density(
    stem: &Java26LevelStem,
    startup: Arc<DatapackStartup>,
    world_seed: i64,
    path: &str,
) -> SaveResult<NoiseFinalDensityProgram> {
    if stem.generator != NOISE_GENERATOR_ID {
        return Err(invalid_schema(
            &format!("{path}.type"),
            format!("{} is not {NOISE_GENERATOR_ID}", stem.generator),
        ));
    }
    let fields = expect_compound(&stem.raw_generator, path)?;
    let settings_path = format!("{path}.settings");
    let (startup, settings_id) = inline_settings::resolve_noise_settings(
        startup,
        required_field(fields, "settings", path)?,
        &settings_path,
    )?;
    let generation = &startup.registries;
    let biome_source_path = format!("{path}.biome_source");
    let biome_source = expect_compound(
        required_field(fields, "biome_source", path)?,
        &biome_source_path,
    )?;
    let biome_source_type = expect_resource_id(
        required_field(biome_source, "type", &biome_source_path)?,
        &format!("{biome_source_path}.type"),
    )?;
    let registry_backed_biome_source = matches!(
        biome_source_type.as_str(),
        "minecraft:fixed" | "minecraft:checkerboard"
    );
    let preset = if biome_source_type == "minecraft:multi_noise" {
        let preset = field(biome_source, "preset");
        let biomes = field(biome_source, "biomes");
        if preset.is_some() == biomes.is_some() {
            return Err(invalid_schema(
                &biome_source_path,
                "multi_noise requires exactly one of preset or biomes",
            ));
        }
        let allowed = if preset.is_some() { "preset" } else { "biomes" };
        if biome_source
            .iter()
            .any(|entry| entry.name != "type" && entry.name != allowed)
        {
            return Err(invalid_schema(
                &biome_source_path,
                "multi_noise contains an unknown field",
            ));
        }
        preset
            .map(|value| {
                resolve_multi_noise_parameter_list(
                    generation,
                    value,
                    &format!("{biome_source_path}.preset"),
                )
                .map_err(|error| invalid_schema(&biome_source_path, error.to_string()))
            })
            .transpose()?
    } else {
        None
    };
    let direct_biomes = if biome_source_type == "minecraft:multi_noise" && preset.is_none() {
        field(biome_source, "biomes").cloned()
    } else {
        None
    };
    let end_biome_source = biome_source_type == "minecraft:the_end";
    if end_biome_source
        && (biome_source.len() != 1
            || biome_source
                .first()
                .is_none_or(|entry| entry.name != "type"))
    {
        return Err(invalid_schema(
            &biome_source_path,
            "minecraft:the_end biome source accepts only its type field",
        ));
    }
    let biome_source = match &preset {
        Some(preset) => Some(
            NoiseBiomeSource::from_resolved_preset(
                generation,
                &settings_id,
                preset,
                world_seed,
                &format!("{biome_source_path}.preset"),
            )
            .map_err(|error| invalid_schema(&biome_source_path, error.to_string()))?,
        ),
        None if end_biome_source => Some(
            NoiseBiomeSource::end_from_registry(generation)
                .map_err(|error| invalid_schema(&biome_source_path, error.to_string()))?,
        ),
        None if direct_biomes.is_some() => Some(
            NoiseBiomeSource::direct_from_nbt(
                generation,
                direct_biomes.as_ref().expect("direct source present"),
                &format!("{biome_source_path}.biomes"),
            )
            .map_err(|error| invalid_schema(&biome_source_path, error.to_string()))?,
        ),
        None if registry_backed_biome_source => Some(
            NoiseBiomeSource::registry_backed_from_nbt(
                generation,
                &startup.tags,
                required_field(fields, "biome_source", path)?,
                &biome_source_path,
            )
            .map_err(|error| invalid_schema(&biome_source_path, error.to_string()))?,
        ),
        _ => {
            return Err(invalid_schema(
                &format!("{biome_source_path}.type"),
                format!("biome source type {biome_source_type} has no runtime owner"),
            ))
        }
    };
    let uses_climate_sampler = direct_biomes.is_some()
        || matches!(
            preset,
            Some(
                ResolvedMultiNoisePreset::Builtin(MultiNoisePreset::Overworld)
                    | ResolvedMultiNoisePreset::Custom(_)
            )
        );
    let source = if uses_climate_sampler || end_biome_source {
        NoiseDensitySource::from_registry_with_climate(generation, &settings_id, world_seed)
    } else {
        NoiseDensitySource::from_registry(generation, &settings_id, world_seed)
    }
    .map_err(|error| invalid_schema(&settings_path, error.to_string()))?;
    let dimension_shape = dimension_type_shape(generation, &stem.dimension_type)
        .map_err(|error| invalid_schema(&format!("{path}.type"), error.to_string()))?;
    // `NoiseBasedChunkGenerator.java:267-295` resolves SURFACE from the settings holder after
    // BIOMES/NOISE; holder identity does not select a separate surface implementation.
    let surface = if biome_source.is_some() {
        Some(
            NoiseSurfaceProgram::from_registry(generation, &settings_id, world_seed, &source)
                .map_err(|error| invalid_schema(&settings_path, error.to_string()))?,
        )
    } else {
        None
    };
    let (carvers, carvers_are_verified_empty) = if let Some(biome_source) = biome_source
        .as_ref()
        .filter(|_| (uses_climate_sampler || registry_backed_biome_source) && surface.is_some())
    {
        let possible_biomes = biome_source.possible_biome_order();
        (
            Some(BuiltInCarverProgram::Overworld(
                OverworldCarverProgram::from_registry(
                    generation,
                    &startup.tags,
                    world_seed,
                    &possible_biomes,
                )
                .map_err(|error| invalid_schema(&settings_path, error.to_string()))?,
            )),
            false,
        )
    } else if matches!(
        preset,
        Some(ResolvedMultiNoisePreset::Builtin(MultiNoisePreset::Nether))
    ) && surface.is_some()
    {
        (
            Some(BuiltInCarverProgram::Nether(
                NetherCarverProgram::from_registry(
                    generation,
                    &startup.tags,
                    world_seed,
                    &source,
                    biome_source
                        .as_ref()
                        .expect("the checked Nether biome source is present"),
                )
                .map_err(|error| invalid_schema(&settings_path, error.to_string()))?,
            )),
            false,
        )
    } else if end_biome_source && surface.is_some() {
        let end_biomes = biome_source
            .as_ref()
            .and_then(NoiseBiomeSource::end_biome_order)
            .expect("the built-in End biome order is present");
        compile_end_carvers(
            generation,
            &startup.tags,
            world_seed,
            end_biomes,
            &settings_path,
        )?
    } else {
        (None, false)
    };
    let structures =
        compile_builtin_structure_plan(&startup, biome_source.as_ref(), &settings_path)?;
    let (feature_resources, nether_features) =
        compile_feature_resources(&startup, &source, biome_source.as_ref(), carvers.as_ref())?;
    let biome_names = generation
        .registry("minecraft:worldgen/biome")
        .expect("every compiled biome source has the biome registry")
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect();
    Ok(NoiseFinalDensityProgram {
        source: Arc::new(source),
        world_seed,
        biome_source: biome_source.map(Arc::new),
        biome_names,
        surface: surface.map(Arc::new),
        carvers,
        carvers_are_verified_empty,
        structures,
        feature_resources,
        nether_features,
        dimension_type_id: stem.dimension_type.clone(),
        dimension_shape,
        structure_rings: OnceLock::new(),
        item_publication: OnceLock::new(),
    })
}

#[cfg(test)]
pub mod test_support;
#[cfg(test)]
pub mod tests;
#[cfg(test)]
pub use test_support::{
    fresh_overworld_program, overworld_noise_stem, overworld_program, overworld_startup,
};
