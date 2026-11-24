//! Noise-program fixtures shared with the crates above.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

#[cfg(test)]
use crate::enchantment_facts::builtin_enchantment_registry;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use bbb_server_persistence::Java26LevelStem;

use super::{compile_noise_final_density, compile_noise_level_source, NoiseFinalDensityProgram};

pub(crate) fn publish_worldgen_element(
    generation: &mut bbb_server_datapack::dynamic_registry::DatapackGeneration,
    registry_id: &str,
    mut element: bbb_server_datapack::dynamic_registry::RawRegistryElement,
) {
    element.source = None;
    let source = bbb_registry_codec::parse_registry_json(&element.source_json).unwrap();
    let typed = bbb_server_datapack::worldgen::WorldgenElement::decode(registry_id, &source)
        .unwrap()
        .unwrap();
    generation.worldgen.insert(element.id.clone(), typed);
    let elements = &mut generation
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == registry_id)
        .unwrap()
        .elements;
    if let Some(previous) = elements.iter_mut().find(|value| value.id == element.id) {
        *previous = element;
    } else {
        elements.push(element);
    }
}

pub fn compound(entries: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

pub fn noise_stem(settings: NbtSummaryValue) -> Java26LevelStem {
    Java26LevelStem {
        dimension_type: "minecraft:the_nether".to_string(),
        generator: "minecraft:noise".to_string(),
        raw_generator: compound(vec![
            ("type", NbtSummaryValue::string("minecraft:noise")),
            (
                "biome_source",
                compound(vec![
                    ("type", NbtSummaryValue::string("minecraft:multi_noise")),
                    ("preset", NbtSummaryValue::string("minecraft:nether")),
                ]),
            ),
            ("settings", settings),
        ]),
    }
}

pub fn overworld_noise_stem() -> Java26LevelStem {
    let mut stem = noise_stem(NbtSummaryValue::string("minecraft:overworld"));
    stem.dimension_type = "minecraft:overworld".to_string();
    let NbtSummaryValue::Compound(generator) = &mut stem.raw_generator else {
        unreachable!()
    };
    let biome_source = generator
        .iter_mut()
        .find(|entry| entry.name == "biome_source")
        .unwrap();
    let NbtSummaryValue::Compound(fields) = &mut biome_source.value else {
        unreachable!()
    };
    fields
        .iter_mut()
        .find(|entry| entry.name == "preset")
        .unwrap()
        .value = NbtSummaryValue::string("minecraft:overworld");
    stem
}

pub fn nether_noise_stem() -> Java26LevelStem {
    noise_stem(NbtSummaryValue::string("minecraft:nether"))
}

pub fn ephemeral_options() -> DatapackStartupOptions {
    DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    }
}

/// The embedded vanilla datapack, loaded once per process and shared.
pub fn vanilla_startup() -> Arc<DatapackStartup> {
    static STARTUP: OnceLock<Arc<DatapackStartup>> = OnceLock::new();
    Arc::clone(
        STARTUP.get_or_init(|| Arc::new(DatapackStartup::load(&ephemeral_options()).unwrap())),
    )
}

/// A private deep copy for tests that edit registry elements through
/// `Arc::get_mut`; the shared generation is never mutated.
pub fn fresh_vanilla_startup() -> DatapackStartup {
    (*vanilla_startup()).clone()
}

/// The shared Overworld startup per `ore_veins_enabled` value.
pub fn overworld_startup(ore_veins_enabled: bool) -> Arc<DatapackStartup> {
    static STARTUPS: [OnceLock<Arc<DatapackStartup>>; 2] = [OnceLock::new(), OnceLock::new()];
    Arc::clone(
        STARTUPS[usize::from(ore_veins_enabled)]
            .get_or_init(|| Arc::new(fresh_overworld_startup(ore_veins_enabled))),
    )
}

/// A private, mutable Overworld startup (see [`fresh_vanilla_startup`]).
pub fn fresh_overworld_startup(ore_veins_enabled: bool) -> DatapackStartup {
    let mut startup = fresh_vanilla_startup();
    let element = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:overworld")
        .unwrap();
    let mut settings: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
    settings["ore_veins_enabled"] = serde_json::json!(ore_veins_enabled);
    element.source_json = serde_json::to_vec(&settings).unwrap().into();
    let element = element.clone();
    publish_worldgen_element(
        &mut startup.registries,
        "minecraft:worldgen/noise_settings",
        element,
    );
    startup
}

/// The built-in Overworld program for one `(ore_veins_enabled, seed)`,
/// compiled once per process from the shared startup.
pub fn overworld_program(
    ore_veins_enabled: bool,
    world_seed: i64,
) -> Arc<NoiseFinalDensityProgram> {
    type OverworldProgramMap = Mutex<BTreeMap<(bool, i64), Arc<NoiseFinalDensityProgram>>>;
    static PROGRAMS: OnceLock<OverworldProgramMap> = OnceLock::new();
    let mut programs = PROGRAMS.get_or_init(Mutex::default).lock().unwrap();
    Arc::clone(
        programs
            .entry((ore_veins_enabled, world_seed))
            .or_insert_with(|| fresh_overworld_program(ore_veins_enabled, world_seed)),
    )
}

/// A separately compiled program (its own owner token) over the shared startup.
/// Compiled through `compile_noise_final_density`: the CARVERS-storage entry
/// refuses the `ore_veins_enabled = false` variant.
pub fn fresh_overworld_program(
    ore_veins_enabled: bool,
    world_seed: i64,
) -> Arc<NoiseFinalDensityProgram> {
    Arc::new(
        compile_noise_final_density(
            &overworld_noise_stem(),
            overworld_startup(ore_veins_enabled),
            world_seed,
            "dimensions.minecraft:overworld.generator",
        )
        .unwrap(),
    )
}

pub fn nether_program(world_seed: i64) -> Arc<NoiseFinalDensityProgram> {
    static PROGRAMS: OnceLock<Mutex<BTreeMap<i64, Arc<NoiseFinalDensityProgram>>>> =
        OnceLock::new();
    let mut programs = PROGRAMS.get_or_init(Mutex::default).lock().unwrap();
    Arc::clone(programs.entry(world_seed).or_insert_with(|| {
        Arc::new(
            compile_noise_level_source(
                &nether_noise_stem(),
                vanilla_startup(),
                world_seed,
                "dimensions.minecraft:the_nether.generator",
            )
            .unwrap(),
        )
    }))
}

/// The built-in `minecraft:enchantment` generation, which is what a level with
/// no extra datapack publishes.
#[cfg(test)]
pub struct BuiltinEnchantmentPublication;

#[cfg(test)]
impl bbb_protocol::PersistentRegistryResolver for BuiltinEnchantmentPublication {
    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> bbb_protocol::PersistentRegistryMembership {
        if registry_id != "minecraft:enchantment" {
            return bbb_protocol::PersistentRegistryMembership::Unavailable;
        }
        if builtin_enchantment_registry()
            .iter()
            .any(|entry| entry == resource_id)
        {
            bbb_protocol::PersistentRegistryMembership::Present
        } else {
            bbb_protocol::PersistentRegistryMembership::Absent
        }
    }

    fn tag_members(
        &self,
        _registry_id: &str,
        _tag_id: &str,
    ) -> bbb_protocol::PersistentRegistryTagMembers {
        bbb_protocol::PersistentRegistryTagMembers::Unavailable
    }
}

#[cfg(test)]
impl bbb_registry_codec::PersistentItemStackHolderResolver for BuiltinEnchantmentPublication {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        if registry_id != "minecraft:enchantment" {
            return None;
        }
        let index = builtin_enchantment_registry()
            .iter()
            .position(|entry| entry == resource_id)?;
        i32::try_from(index).ok()
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        if registry_id != "minecraft:enchantment" {
            return None;
        }
        builtin_enchantment_registry()
            .get(usize::try_from(holder_id).ok()?)
            .cloned()
    }
}

impl NoiseFinalDensityProgram {
    /// The chunk nearest the origin (by square rings) where `set_id` attempts
    /// a start and `accept` agrees, so tests need not pin seed-specific chunks.
    pub fn find_structure_candidate(
        &self,
        set_id: &str,
        radius: i32,
        mut accept: impl FnMut([i32; 2]) -> bool,
    ) -> Option<[i32; 2]> {
        let plan = self.structures.as_ref()?;
        (0..=radius)
            .flat_map(|ring| {
                (-ring..=ring).flat_map(move |x| {
                    let edge = x.abs() == ring;
                    let zs: Vec<i32> = if edge {
                        (-ring..=ring).collect()
                    } else {
                        vec![-ring, ring]
                    };
                    zs.into_iter().map(move |z| [x, z])
                })
            })
            .find(|&[x, z]| {
                plan.random_spread_attempt_order(set_id, self.world_seed, x, z)
                    .is_some_and(|attempts| !attempts.is_empty())
                    && accept([x, z])
            })
    }

    /// The nearest chunk whose jigsaw starts include `structure_id`.
    pub fn jigsaw_start_chunk(&self, set_id: &str, structure_id: &str) -> [i32; 2] {
        self.find_structure_candidate(set_id, 256, |[x, z]| {
            self.generate_jigsaw_structure_starts_component(x, z, &[])
                .unwrap()
                .starts()
                .iter()
                .any(|start| start.structure_id() == structure_id)
        })
        .unwrap_or_else(|| panic!("no {structure_id} start near the origin"))
    }

    /// The nearest chunk whose buried-treasure candidate passes its biome gate.
    pub fn buried_treasure_chunk(&self) -> [i32; 2] {
        self.find_structure_candidate("minecraft:buried_treasures", 256, |[x, z]| {
            self.generate_buried_treasure_start_component(x, z, false)
                .unwrap()
                .is_some()
        })
        .expect("a buried treasure near the origin")
    }
}
