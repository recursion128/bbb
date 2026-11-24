//! `WorldStem.load` startup wiring (goal.md D6): build the pack repository from
//! the embedded vanilla snapshot plus `<world>/datapacks`, select per the saved
//! `Java26DataPacks` (safe mode forces the vanilla-only selection,
//! `Main.java:153-158`), and load every datapack surface in `WorldLoader` order
//! (`WorldLoader.java:25-107`): dynamic registries + tags, then the reloadable
//! loot / recipe / advancement / function surfaces and the structure templates.
//!
//! Registry load failures abort startup with attribution (`Main.java:185`); the
//! `scanDirectory`-family surfaces (loot, recipe, advancement, function,
//! structure) collect warn-only entries that the runtime logs.

use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use super::advancement::{load_advancements, AdvancementSet};
use super::dynamic_registry::SharedDatapackGeneration;
use super::frozen_lookup::validate_frozen_registry_codec_references_with_content_registry;
use super::function::{load_function_library, FunctionLibrary};
use super::loot::{load_loot_registries, LootRegistries};
use super::mob_spawn::{DatapackMobSpawnCatalog, MobSpawnCatalogError};
use super::pack::ServerPackResources;
use super::pack_repository::{
    DatapackListing, PackRepositoryError, PackSelection, ServerPackRepository,
};
use super::recipe::{
    load_recipes_with_lookup, ItemTagIndex, RecipeDisplayCatalog, RecipeIndex, RecipeSet,
};
use super::registry_data_loader::{
    load_dynamic_registries_with_content_registry, DatapackLoadError, DatapackLoadLimits,
};
use super::resource_overlay::{LayeredPackResources, MergedServerDataView, PackOverlayInput};
use super::structure_pool::{StructurePoolError, StructurePoolSet};
use super::structure_template::{
    load_structure_templates_with_content_registry, StructureTemplateSet,
};
use super::tag_loader::{load_tags_for_startup_with_content_registry, TagLoadReport};
use super::vanilla_snapshot::VanillaDatapackSnapshot;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_server_types::save::Java26DataPacks;

pub(crate) mod registry_scope;

/// `FeatureFlags.DEFAULT_FLAGS`: the vanilla feature set only
/// (`enableForcedFeaturePacks` stays out of scope, see `ServerPackRepository::select`).
pub fn default_enabled_features_26_1() -> BTreeSet<String> {
    DEFAULT_ENABLED_FEATURES_26_1
        .iter()
        .map(|feature| (*feature).to_string())
        .collect()
}

/// The same set, borrowed, for the readers that only ask whether something is
/// in it. It is the one declaration; [`default_enabled_features_26_1`] builds
/// its owned form from it rather than repeating the names.
pub const DEFAULT_ENABLED_FEATURES_26_1: &[&str] = &["minecraft:vanilla"];

/// Startup inputs for the datapack pipeline (`WorldLoader.PackConfig`
/// equivalent, `Main.java:285`).
#[derive(Debug, Clone)]
pub struct DatapackStartupOptions {
    /// `<world>/datapacks`; `None` is the ephemeral flat-fixture mode, which
    /// loads the built-in packs only through an empty scratch directory.
    pub datapacks_dir: Option<PathBuf>,
    /// The saved `level.dat` DataPacks selection; `None` (fresh or ephemeral
    /// world) selects the vanilla default (`Main.java:275-285`).
    pub saved_data_packs: Option<Java26DataPacks>,
    /// `--safeMode` equivalent: force the vanilla-only selection.
    pub safe_mode: bool,
    pub limits: DatapackLoadLimits,
}

/// The aggregate fail-closed startup error (`Main.java:185`: datapack load
/// failure aborts the server start).
#[derive(Debug)]
pub enum DatapackStartupError {
    Repository(PackRepositoryError),
    Registries(DatapackLoadError),
}

impl fmt::Display for DatapackStartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repository(error) => write!(f, "pack repository failed: {error}"),
            Self::Registries(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DatapackStartupError {}

/// Which selection a reload applies: `/reload` re-discovers over the effective
/// config (`ReloadCommand.discoverNewPacks`, `ReloadCommand.java:26-38`) while
/// `/datapack enable` and `disable` hand `reloadPacks` the list they built
/// (`DataPackCommand.java:218`, `:227`).
enum ReloadSelection<'a> {
    Rediscover(&'a Java26DataPacks),
    Named(&'a [String]),
}

#[derive(Debug)]
struct CachedMobSpawnCatalog {
    biome_sources: Vec<(String, Arc<[u8]>)>,
    structure_sources: Vec<(String, Arc<[u8]>)>,
    content_registry: Arc<ContentRegistry>,
    catalog: Result<Arc<DatapackMobSpawnCatalog>, MobSpawnCatalogError>,
}

/// The fully loaded datapack surface held by the server world for the later
/// gameplay and MCP consumers.
#[derive(Debug, Clone)]
pub struct DatapackStartup {
    pub(crate) registry_scope: registry_scope::FrozenRegistryScope,
    /// `<world>/datapacks` as this load resolved it, which is where
    /// `/datapack create` writes (`LevelResource.DATAPACK_DIR`); `None` in the
    /// ephemeral mode that keeps no save directory.
    pub datapacks_dir: Option<PathBuf>,
    pub selection: PackSelection,
    /// `PackRepository.getAvailablePacks` as of this load, kept so `/datapack
    /// list` can still name packs after the open repository is dropped.
    pub available: Vec<DatapackListing>,
    /// The world's enabled feature set this selection was computed against
    /// (`WorldDataConfiguration.enabledFeatures`).
    pub enabled_features: BTreeSet<String>,
    pub registries: SharedDatapackGeneration,
    pub content_registry: Arc<ContentRegistry>,
    pub trades: std::collections::BTreeMap<String, crate::trades::VillagerTrade>,
    pub tags: TagLoadReport,
    pub loot: LootRegistries,
    /// Shared, so every player's crafting catalog holds it without a copy.
    pub recipes: Arc<RecipeSet>,
    /// Pre-partitioned crafting/cooking recipe indices for `self.recipes`.
    pub recipe_index: Arc<RecipeIndex>,
    pub recipe_displays: RecipeDisplayCatalog,
    pub advancements: AdvancementSet,
    pub functions: FunctionLibrary,
    pub structures: StructureTemplateSet,
    mob_spawn_cache: Arc<OnceLock<CachedMobSpawnCatalog>>,
}

impl DatapackStartup {
    /// Run the whole startup pipeline eagerly; on success nothing references the
    /// pack files any more (all surfaces are owned, in-memory data).
    pub fn load(options: &DatapackStartupOptions) -> Result<Self, DatapackStartupError> {
        Self::load_with_content_registry(options, ContentRegistry::vanilla())
    }

    pub fn load_with_content_registry(
        options: &DatapackStartupOptions,
        content_registry: &ContentRegistry,
    ) -> Result<Self, DatapackStartupError> {
        Self::load_with_registries(options, None, None, Some(content_registry))
    }

    /// Reload tags and reloadable resources against the registry generation the
    /// running server already owns.
    ///
    /// Vanilla's `/reload` passes `this.registries` to
    /// `ReloadableServerResources.loadResources` and calls
    /// `TagLoader.loadTagsForExistingRegistries`; it does not run
    /// `RegistryDataLoader` again (`MinecraftServer.java:1504-1544`). Taking
    /// the previous startup retains both frozen holder identities and tag views:
    /// an empty built tag map supplies no replacement lookup.
    pub fn reload(
        options: &DatapackStartupOptions,
        previous: &Self,
    ) -> Result<Self, DatapackStartupError> {
        let current = options
            .saved_data_packs
            .clone()
            .unwrap_or_else(|| previous.selection.data_packs().clone());
        Self::reload_with_data_packs(options, &current, previous)
    }

    /// Reload from the running world's effective enabled and disabled pack ids.
    pub fn reload_with_data_packs(
        options: &DatapackStartupOptions,
        current: &Java26DataPacks,
        previous: &Self,
    ) -> Result<Self, DatapackStartupError> {
        Self::load_with_registries(
            options,
            Some(previous),
            Some(ReloadSelection::Rediscover(current)),
            None,
        )
    }

    /// Reload the set `/datapack enable` or `disable` named, in the order it
    /// named them: `reloadResources` opens exactly `packsToEnable`
    /// (`MinecraftServer.java:1505-1508`) and only then makes it the selection
    /// (`:1534`), with no re-discovery pass.
    pub fn reload_with_selected_ids(
        options: &DatapackStartupOptions,
        selected: &[String],
        previous: &Self,
    ) -> Result<Self, DatapackStartupError> {
        Self::load_with_registries(
            options,
            Some(previous),
            Some(ReloadSelection::Named(selected)),
            None,
        )
    }

    fn load_with_registries(
        options: &DatapackStartupOptions,
        previous: Option<&Self>,
        reload_data_packs: Option<ReloadSelection<'_>>,
        content_registry: Option<&ContentRegistry>,
    ) -> Result<Self, DatapackStartupError> {
        let content_registry = match content_registry {
            Some(registry) if !registry.is_vanilla_only() => Arc::new(registry.clone()),
            Some(_) => Arc::clone(ContentRegistry::vanilla()),
            None => previous
                .map(|prev| Arc::clone(&prev.content_registry))
                .unwrap_or_else(|| Arc::clone(ContentRegistry::vanilla())),
        };
        // Ephemeral mode still goes through the one repository code path, against
        // an empty scratch directory that is removed once loading finishes.
        let scratch;
        let datapacks_dir = match &options.datapacks_dir {
            Some(dir) => dir.clone(),
            None => {
                scratch = std::env::temp_dir().join(format!(
                    "bbb-ephemeral-datapacks-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                scratch
            }
        };
        let result = Self::load_from_dir(
            &datapacks_dir,
            options,
            previous,
            reload_data_packs,
            content_registry,
        );
        if options.datapacks_dir.is_none() {
            let _ = std::fs::remove_dir_all(&datapacks_dir);
        }
        result
    }

    fn load_from_dir(
        datapacks_dir: &std::path::Path,
        options: &DatapackStartupOptions,
        previous: Option<&Self>,
        reload_data_packs: Option<ReloadSelection<'_>>,
        content_registry: Arc<ContentRegistry>,
    ) -> Result<Self, DatapackStartupError> {
        let snapshot = VanillaDatapackSnapshot::embedded_shared().expect("embedded snapshot");
        let feature_packs: Vec<(String, Box<dyn ServerPackResources>)> = snapshot
            .feature_pack_ids()
            .into_iter()
            .map(|id| {
                (
                    id.clone(),
                    Box::new(snapshot.feature_pack(&id).expect("declared feature pack"))
                        as Box<dyn ServerPackResources>,
                )
            })
            .collect();
        let repository = ServerPackRepository::new(
            datapacks_dir,
            Box::new(snapshot.vanilla_pack()),
            feature_packs,
        )
        .map_err(DatapackStartupError::Repository)?;
        let selection = match reload_data_packs {
            Some(ReloadSelection::Rediscover(current)) => repository.select_reload(current),
            Some(ReloadSelection::Named(ids)) => repository.select_ids(ids),
            None => {
                let data_packs = options.saved_data_packs.clone().unwrap_or_else(|| {
                    Java26DataPacks::new(vec!["vanilla".to_string()], Vec::new())
                });
                // A Vanilla level's `enabled_features` is what its selected
                // packs request (`MinecraftServer.java:1610-1618`), so the
                // saved selection allows its own feature packs.
                let mut allowed = default_enabled_features_26_1();
                for id in data_packs.enabled() {
                    if let Some(pack) = id.with_utf8(|id| repository.get(id)).flatten() {
                        allowed.extend(pack.meta.requested_features.iter().cloned());
                    }
                }
                repository.select(&data_packs, options.safe_mode, &allowed)
            }
        };
        let opened = repository.open_selected(&selection);
        // `configureRepositoryWithSelection` (`MinecraftServer.java:1610-1618`);
        // a reload keeps the world's set.
        let enabled_features = match previous {
            Some(previous) => previous.enabled_features.clone(),
            None => {
                let mut features = default_enabled_features_26_1();
                for pack in &opened {
                    features.extend(pack.meta.requested_features.iter().cloned());
                }
                features
            }
        };
        let layered_packs: Vec<LayeredPackResources<'_>> = opened
            .iter()
            .map(|pack| LayeredPackResources::from_server_pack(pack))
            .collect();
        let inputs: Vec<PackOverlayInput<'_>> = opened
            .iter()
            .zip(&layered_packs)
            .map(|(pack, resources)| PackOverlayInput {
                pack_id: &pack.id,
                resources,
                filter: Some(&pack.meta.filter),
            })
            .collect();
        let view = MergedServerDataView::merge(&inputs);
        // Startup runs the fail-closed `RegistryDataLoader` layer first. A
        // reload instead retains the generation supplied above via `Arc::clone`
        // and only builds tags/resources against it, matching `reloadResources`'
        // existing registry access.
        let mut registries = match previous {
            Some(previous) => previous.registries.clone(),
            None => SharedDatapackGeneration::new(
                load_dynamic_registries_with_content_registry(
                    &view,
                    &inputs,
                    options.limits,
                    &content_registry,
                )
                .map_err(DatapackStartupError::Registries)?,
            ),
        };
        let mut tags = load_tags_for_startup_with_content_registry(
            &inputs,
            &registries,
            options.limits,
            &content_registry,
        );
        let registry_scope = match previous {
            Some(previous) => {
                registry_scope::retain_unreloaded_tags(&mut tags, &previous.tags, &registries);
                previous.registry_scope.clone()
            }
            None => {
                validate_frozen_registry_codec_references_with_content_registry(
                    &registries,
                    &tags,
                    &content_registry,
                )
                .map_err(DatapackStartupError::Registries)?;
                registry_scope::FrozenRegistryScope::new(&tags)
            }
        };
        let bonemeal_features = registries
            .worldgen
            .compute_bonemeal_features(&tags)
            .map_err(|error| {
                DatapackStartupError::Registries(DatapackLoadError {
                    errors: vec![super::registry_data_loader::DatapackElementError {
                        registry_id: "minecraft:worldgen/biome".into(),
                        element_id: None,
                        pack: String::new(),
                        message: error.to_string(),
                    }],
                })
            })?;
        if registries.worldgen.biome_bonemeal_features != bonemeal_features {
            registries.worldgen.biome_bonemeal_features = bonemeal_features;
        }
        let lookup = crate::frozen_lookup::FrozenDatapackRegistryLookup::new_with_content_registry(
            &registries,
            &tags,
            &content_registry,
        );
        let loot = load_loot_registries(&view, &inputs, options.limits, Some(&lookup));
        let trades = match previous {
            Some(previous) => previous.trades.clone(),
            None => crate::trades::load_trade_catalog(&registries, &lookup)
                .map_err(DatapackStartupError::Registries)?,
        };
        let mut recipes = load_recipes_with_lookup(&view, &inputs, options.limits, &lookup);
        let item_tags = tags
            .registry("minecraft:item")
            .map(|loaded| {
                ItemTagIndex::from_loaded_with_content_registry(loaded, &content_registry)
            })
            .unwrap_or_default();
        let recipe_displays = recipes.finalize_client_catalog_with_content_registry(
            &item_tags,
            &registries,
            &content_registry,
        );
        let recipe_index = Arc::new(recipes.build_index());
        let advancements = load_advancements(&view, &inputs, options.limits, Some(&lookup));
        let functions = load_function_library(&view, &inputs, options.limits);
        let structures = load_structure_templates_with_content_registry(
            &view,
            &inputs,
            options.limits,
            &content_registry,
        );
        let available = repository.listings();
        if previous.is_none() {
            registries.trim_startup_sources();
        }
        let mob_spawn_cache = match previous {
            Some(previous) => Arc::clone(&previous.mob_spawn_cache),
            None => Arc::new(OnceLock::new()),
        };
        Ok(Self {
            registry_scope,
            datapacks_dir: options.datapacks_dir.clone(),
            selection,
            available,
            enabled_features,
            registries,
            content_registry,
            trades,
            tags,
            loot,
            recipes: Arc::new(recipes),
            recipe_index,
            recipe_displays,
            advancements,
            functions,
            structures,
            mob_spawn_cache,
        })
    }

    pub fn content_registry(&self) -> &ContentRegistry {
        &self.content_registry
    }

    /// Returns a shared `Arc<StructurePoolSet>` reused across all dimensions
    /// without re-parsing template pools.
    pub fn structure_pools_shared(&self) -> Result<Arc<StructurePoolSet>, StructurePoolError> {
        StructurePoolSet::from_generation_shared(&self.registries)
    }

    /// Returns a shared `Arc<DatapackMobSpawnCatalog>` built once per startup
    /// generation and reused across dimensions and `/reload`.
    pub fn mob_spawn_catalog_shared(
        &self,
    ) -> Result<Arc<DatapackMobSpawnCatalog>, MobSpawnCatalogError> {
        let biome_elements = self
            .registries
            .registry("minecraft:worldgen/biome")
            .map(|r| r.elements.as_slice())
            .unwrap_or(&[]);
        let structure_elements = self
            .registries
            .registry("minecraft:worldgen/structure")
            .map(|r| r.elements.as_slice())
            .unwrap_or(&[]);
        let sources_match =
            |stamps: &[(String, Arc<[u8]>)],
             elements: &[super::dynamic_registry::RawRegistryElement]| {
                stamps.len() == elements.len()
                    && stamps
                        .iter()
                        .zip(elements)
                        .all(|((id, src), el)| id == &el.id && Arc::ptr_eq(src, &el.source_json))
            };
        let cached = self.mob_spawn_cache.get_or_init(|| CachedMobSpawnCatalog {
            biome_sources: biome_elements
                .iter()
                .map(|el| (el.id.clone(), Arc::clone(&el.source_json)))
                .collect(),
            structure_sources: structure_elements
                .iter()
                .map(|el| (el.id.clone(), Arc::clone(&el.source_json)))
                .collect(),
            content_registry: Arc::clone(&self.content_registry),
            catalog: DatapackMobSpawnCatalog::from_generation_with_registry(
                &self.registries,
                &self.content_registry,
            )
            .map(Arc::new),
        });
        if Arc::ptr_eq(&cached.content_registry, &self.content_registry)
            && sources_match(&cached.biome_sources, biome_elements)
            && sources_match(&cached.structure_sources, structure_elements)
        {
            return cached.catalog.clone();
        }
        DatapackMobSpawnCatalog::from_generation_with_registry(
            &self.registries,
            &self.content_registry,
        )
        .map(Arc::new)
    }

    /// `PackRepository.getSelectedPacks` in load order
    /// (`DataPackCommand.listEnabledPacks`, DataPackCommand.java:258-274).
    pub fn listed_enabled(&self) -> Vec<&DatapackListing> {
        self.selection
            .ordered_ids
            .iter()
            .filter_map(|id| self.available.iter().find(|pack| pack.id == *id))
            .collect()
    }

    /// Unselected packs whose requested features are a subset of the world's
    /// enabled set (`DataPackCommand.listAvailablePacks`,
    /// DataPackCommand.java:235-256).
    pub fn listed_available(&self) -> Vec<&DatapackListing> {
        self.available
            .iter()
            .filter(|pack| {
                !self.selection.ordered_ids.contains(&pack.id)
                    && pack.requested_features.is_subset(&self.enabled_features)
            })
            .collect()
    }

    /// One attributed line per warn-only event across every surface, for the
    /// startup log (`scanDirectory` logs and skips).
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        for notice in &self.selection.notices {
            warnings.push(format!("pack selection: {notice:?}"));
        }
        for warning in &self.tags.warnings {
            warnings.push(format!(
                "tag {} (registry {}, pack {:?}): {}",
                warning.tag_id.as_deref().unwrap_or("<file>"),
                warning.registry_id,
                warning.pack,
                warning.message
            ));
        }
        for warning in &self.loot.warnings {
            warnings.push(format!(
                "loot {} (pack {:?}): {}",
                warning.element_id.as_deref().unwrap_or("<file>"),
                warning.pack,
                warning.message
            ));
        }
        for warning in &self.recipes.warnings {
            warnings.push(format!(
                "recipe {} (pack {}): {}",
                warning.element_id, warning.pack, warning.message
            ));
        }
        for warning in &self.advancements.warnings {
            warnings.push(format!(
                "advancement {} (pack {}): {}",
                warning.element_id, warning.pack, warning.message
            ));
        }
        for warning in &self.functions.warnings {
            warnings.push(format!(
                "function {} (pack {}): {}",
                warning.element_id, warning.pack, warning.message
            ));
        }
        for warning in &self.structures.warnings {
            warnings.push(format!(
                "structure {} (pack {}): {}",
                warning.element_id, warning.pack, warning.message
            ));
        }
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "bbb-datapack-startup-test-{}-{}",
                std::process::id(),
                tag
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const VALID_MCMETA: &str =
        "{\"pack\":{\"description\":\"test\",\"pack_format\":1000,\"supported_formats\":[0,10000]}}";

    fn ephemeral_options() -> DatapackStartupOptions {
        DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        }
    }

    #[test]
    fn ephemeral_mode_loads_the_builtin_vanilla_surface_clean() {
        let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
        assert_eq!(startup.selection.ordered_ids, ["vanilla".to_string()]);
        assert_eq!(startup.selection.data_packs().enabled(), ["vanilla"]);
        assert_eq!(
            startup.selection.data_packs().disabled(),
            [
                "minecart_improvements",
                "redstone_experiments",
                "trade_rebalance"
            ]
        );
        assert_eq!(startup.registries.registries.len(), 46);
        assert_eq!(startup.recipes.recipes.len(), 1515);
        assert_eq!(startup.recipe_displays.all_displays().len(), 1498);
        assert_eq!(
            startup
                .recipe_displays
                .update_recipes()
                .stonecutter_recipes
                .len(),
            275
        );
        assert_eq!(startup.advancements.advancements.len(), 1617);
        assert_eq!(startup.structures.templates.len(), 1202);
        assert_eq!(startup.warnings(), Vec::<String>::new());
    }

    #[test]
    fn list_helpers_project_selected_and_feature_filtered_available() {
        let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
        assert_eq!(
            startup
                .listed_enabled()
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            ["vanilla"]
        );
        for pack in startup.listed_available() {
            assert!(!startup.selection.ordered_ids.contains(&pack.id));
            assert!(pack.requested_features.is_subset(&startup.enabled_features));
        }
        for pack in &startup.available {
            let selected = startup.selection.ordered_ids.contains(&pack.id);
            let allowed = pack.requested_features.is_subset(&startup.enabled_features);
            assert_eq!(
                !selected && allowed,
                startup
                    .listed_available()
                    .iter()
                    .any(|listed| listed.id == pack.id)
            );
        }
    }

    #[test]
    fn a_saved_experiment_pack_stays_selected_and_enables_its_feature() {
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            saved_data_packs: Some(Java26DataPacks::new(
                vec!["vanilla".to_string(), "trade_rebalance".to_string()],
                vec![
                    "minecart_improvements".to_string(),
                    "redstone_experiments".to_string(),
                ],
            )),
            ..ephemeral_options()
        })
        .unwrap();
        assert_eq!(
            startup.selection.data_packs().enabled(),
            ["vanilla", "trade_rebalance"]
        );
        assert!(startup
            .enabled_features
            .contains("minecraft:trade_rebalance"));
        let trade = startup
            .registries
            .element(
                "minecraft:villager_trade",
                "minecraft:armorer/5/emerald_chainmail_boots_swamp",
            )
            .expect("the experiment pack's trade is loaded");
        assert_eq!(trade.source_pack, "trade_rebalance");
    }

    fn world_pack_options(dir: &Path, safe_mode: bool) -> DatapackStartupOptions {
        DatapackStartupOptions {
            datapacks_dir: Some(dir.join("datapacks")),
            saved_data_packs: None,
            safe_mode,
            limits: DatapackLoadLimits::default(),
        }
    }

    #[test]
    fn biome_bonemeal_features_follow_nested_selectors_and_loaded_tags() {
        use crate::worldgen::ConfiguredFeature;
        use serde_json::json;

        let vanilla = DatapackStartup::load(&ephemeral_options()).unwrap();
        let world = TempDir::new("biome-bonemeal");
        let pack = world.0.join("datapacks/tweaks");
        std::fs::create_dir_all(&pack).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        let mut biome = serde_json::from_slice::<serde_json::Value>(
            &(vanilla
                .registries
                .element("minecraft:worldgen/biome", "minecraft:plains")
                .unwrap())
            .source_json,
        )
        .unwrap();
        biome["features"] = json!(["#example:flowers"]);
        let leaf = json!({"feature":"minecraft:flower_swamp", "placement":[]});
        let simple = json!({"feature":{"type":"minecraft:simple_random_selector","config":{"features":"#example:leaves"}},"placement":[]});
        let boolean = json!({"feature":{"type":"minecraft:random_boolean_selector","config":{"feature_true":leaf,"feature_false":leaf}},"placement":[]});
        for (path, value) in [
            ("data/minecraft/worldgen/biome/plains.json", biome),
            (
                "data/example/worldgen/placed_feature/root.json",
                json!({"feature":"example:root","placement":[]}),
            ),
            ("data/example/worldgen/placed_feature/leaf.json", leaf),
            (
                "data/example/worldgen/configured_feature/root.json",
                json!({"type":"minecraft:random_selector","config":{"features":[{"feature":simple,"chance":0.5}],"default":boolean}}),
            ),
            (
                "data/example/tags/worldgen/placed_feature/flowers.json",
                json!({"values":["example:root"]}),
            ),
            (
                "data/example/tags/worldgen/placed_feature/leaves.json",
                json!({"values":["example:leaf"]}),
            ),
            (
                "data/minecraft/tags/worldgen/configured_feature/can_spawn_from_bone_meal.json",
                json!({"replace":true,"values":["example:root","minecraft:flower_swamp"]}),
            ),
        ] {
            let file = pack.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, serde_json::to_vec(&value).unwrap()).unwrap();
        }
        let mut options = world_pack_options(&world.0, false);
        options.saved_data_packs = Some(Java26DataPacks::new(
            vec!["vanilla".into(), "file/tweaks".into()],
            Vec::new(),
        ));
        let startup = DatapackStartup::load(&options).unwrap();
        let features = &startup.registries.worldgen.biome_bonemeal_features["minecraft:plains"];
        assert_eq!(features.len(), 4);
        assert_eq!(
            features
                .iter()
                .filter(|feature| matches!(&***feature, ConfiguredFeature::RandomSelector(_)))
                .count(),
            1
        );
        assert_eq!(
            features
                .iter()
                .filter(|feature| ***feature
                    == *startup.registries.worldgen.configured_features["minecraft:flower_swamp"])
                .count(),
            3
        );
    }

    #[test]
    fn particle_environment_loads_typed_holder_values_and_ordered_layers() {
        use crate::environment_attributes::discrete::{
            DimensionAmbientParticlesEnvironment, DimensionParticleEnvironment,
        };
        use crate::loot::{EnvironmentAttributeExpectedValue, LootCondition};
        use bbb_protocol::environment_particles::ParticleOptionValue;
        use serde_json::json;

        const PARTICLE: &str = "minecraft:visual/default_dripstone_particle";
        const AMBIENT: &str = "minecraft:visual/ambient_particles";
        let vanilla = DatapackStartup::load(&ephemeral_options()).unwrap();
        let world = TempDir::new("particle-environment");
        let pack = world.0.join("datapacks/tweaks");
        std::fs::create_dir_all(&pack).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        let item = json!({"type":"item","item":{"id":"goat_horn","count":2,
            "components":{"minecraft:instrument":"example:horn","minecraft:custom_data":{"seal":7}}}});
        let dust = |color| json!({"type":"dust","color":color,"scale":2.0});
        let ambient = |first, second| {
            json!([
                {"particle":first,"probability":0.25},
                {"particle":second,"probability":0.75}
            ])
        };
        let predicate =
            json!({"condition":"environment_attribute_check","attribute":PARTICLE,"value":item});
        let mut dimension = serde_json::from_slice::<serde_json::Value>(
            &(vanilla
                .registries
                .element("minecraft:dimension_type", "minecraft:overworld")
                .unwrap())
            .source_json,
        )
        .unwrap();
        dimension["attributes"] =
            json!({PARTICLE:dust(0x123456),AMBIENT:ambient(dust(0x123456),item.clone())});
        dimension["timelines"] = json!([]);
        let mut timed = dimension.clone();
        timed["timelines"] = json!(["example:first", "example:last"]);
        let mut biome = serde_json::from_slice::<serde_json::Value>(
            &(vanilla
                .registries
                .element("minecraft:worldgen/biome", "minecraft:plains")
                .unwrap())
            .source_json,
        )
        .unwrap();
        biome["attributes"] = json!({PARTICLE:item,AMBIENT:ambient(item.clone(),dust(0xabcdef))});
        let mut bare = biome.clone();
        bare["attributes"] = json!({});
        let instrument = serde_json::from_slice::<serde_json::Value>(
            &(vanilla
                .registries
                .registry("minecraft:instrument")
                .unwrap()
                .elements[0])
                .source_json,
        )
        .unwrap();
        for (path, value) in [
            ("data/example/instrument/horn.json", instrument),
            ("data/example/dimension_type/plain.json", dimension),
            ("data/example/dimension_type/timed.json", timed),
            ("data/minecraft/worldgen/biome/plains.json", biome),
            ("data/example/worldgen/biome/bare.json", bare),
            ("data/example/predicate/particle.json", predicate.clone()),
            (
                "data/example/predicate/ambient.json",
                json!({"condition":"environment_attribute_check",
                "attribute":AMBIENT,"value":ambient(item.clone(),dust(0xabcdef))}),
            ),
            (
                "data/example/villager_trade/particle.json",
                json!({
                    "wants":{"id":"emerald"},"gives":"stone","merchant_predicate":predicate,
                    "given_item_modifiers":[{"function":"set_count","count":2,"conditions":[predicate]}]
                }),
            ),
            (
                "data/example/timeline/first.json",
                json!({"clock":"minecraft:overworld","tracks":{
                    PARTICLE:{"keyframes":[{"ticks":0,"value":dust(0x654321)}]},
                    AMBIENT:{"keyframes":[{"ticks":0,"value":ambient(dust(0x654321),item.clone())}]}
                }}),
            ),
            (
                "data/example/timeline/last.json",
                json!({"clock":"minecraft:overworld","tracks":{
                    PARTICLE:{"keyframes":[{"ticks":0,"value":item},{"ticks":10,"value":dust(0xabcdef)}]},
                    AMBIENT:{"keyframes":[{"ticks":0,"value":ambient(item.clone(),dust(0xabcdef))},
                        {"ticks":10,"value":ambient(dust(0xabcdef),item.clone())}]}
                }}),
            ),
        ] {
            let file = pack.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, serde_json::to_vec(&value).unwrap()).unwrap();
        }
        let mut options = world_pack_options(&world.0, false);
        options.saved_data_packs = Some(Java26DataPacks::new(
            vec!["vanilla".into(), "file/tweaks".into()],
            Vec::new(),
        ));
        let startup = DatapackStartup::load(&options).unwrap();
        let expected = startup.loot.predicate("example:particle").unwrap();
        let LootCondition::EnvironmentAttributeCheck {
            value: EnvironmentAttributeExpectedValue::Particle(expected_particle),
            ..
        } = expected
        else {
            panic!("typed particle predicate");
        };
        let ParticleOptionValue::Item { stack } = &expected_particle.option else {
            panic!("item particle");
        };
        assert_eq!(stack.count, 2);
        let holder_id = startup
            .registries
            .registry("minecraft:instrument")
            .unwrap()
            .elements
            .iter()
            .position(|element| element.id == "example:horn")
            .unwrap() as i32;
        let patch = &stack.component_patch;
        assert_eq!(patch.instrument.registry_id, Some(holder_id));
        assert!(patch.custom_data.is_some());
        let trade = &startup.trades["example:particle"];
        assert_eq!(trade.merchant_predicate.as_ref(), Some(expected));
        assert_eq!(
            trade.given_item_modifiers[0].conditions,
            vec![expected.clone()]
        );
        let sources =
            crate::environment_attributes::EnvironmentAttributeSources::new(&startup).unwrap();
        let plain = DimensionParticleEnvironment::from_sources(&sources, "example:plain").unwrap();
        assert_eq!(
            plain.sample("minecraft:plains", &|_| None),
            Some(expected_particle)
        );
        assert!(matches!(
            &plain.sample("example:bare", &|_| None).unwrap().option,
            ParticleOptionValue::Dust {
                color: 0x123456,
                ..
            }
        ));
        let timed = DimensionParticleEnvironment::from_sources(&sources, "example:timed").unwrap();
        assert_eq!(
            timed.sample("minecraft:plains", &|clock| {
                assert_eq!(clock, "minecraft:overworld");
                Some(5)
            }),
            Some(expected_particle)
        );
        assert!(matches!(
            &timed
                .sample("minecraft:plains", &|_| Some(10))
                .unwrap()
                .option,
            ParticleOptionValue::Dust {
                color: 0xabcdef,
                ..
            }
        ));
        let plain_ambient =
            DimensionAmbientParticlesEnvironment::from_sources(&sources, "example:plain").unwrap();
        let biome_ambient = plain_ambient.sample("minecraft:plains", &|_| None).unwrap();
        let LootCondition::EnvironmentAttributeCheck {
            value: EnvironmentAttributeExpectedValue::AmbientParticles(expected_ambient),
            ..
        } = startup.loot.predicate("example:ambient").unwrap()
        else {
            panic!("typed ambient predicate");
        };
        assert_eq!(biome_ambient, expected_ambient);
        assert_eq!(&biome_ambient[0].particle, expected_particle);
        assert_eq!(biome_ambient[0].probability_bits, 0.25_f32.to_bits());
        assert_eq!(biome_ambient[1].probability_bits, 0.75_f32.to_bits());
        let timed_ambient =
            DimensionAmbientParticlesEnvironment::from_sources(&sources, "example:timed").unwrap();
        let at_zero = timed_ambient
            .sample("minecraft:plains", &|_| Some(0))
            .unwrap();
        let at_ten = timed_ambient
            .sample("minecraft:plains", &|_| Some(10))
            .unwrap();
        assert_eq!(at_zero, biome_ambient);
        assert_eq!(&at_ten[1].particle, expected_particle);
        assert_eq!(at_ten[1].probability_bits, 0.75_f32.to_bits());
    }

    #[test]
    fn world_datapack_overrides_and_warns() {
        let world = TempDir::new("world-pack");
        let pack = world.0.join("datapacks/tweaks");
        std::fs::create_dir_all(pack.join("data/minecraft/recipe")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(
            pack.join("data/minecraft/recipe/modded.json"),
            "{\"type\":\"modded:machine\"}",
        )
        .unwrap();

        let startup = DatapackStartup::load(&world_pack_options(&world.0, false)).unwrap();
        // The world pack auto-adds after vanilla (`PackSource.WORLD`).
        assert_eq!(
            startup.selection.ordered_ids,
            ["vanilla".to_string(), "file/tweaks".to_string()]
        );
        assert_eq!(startup.recipes.recipes.len(), 1515);
        assert_eq!(startup.recipes.warnings.len(), 1);
        let warnings = startup.warnings();
        // The world-pack auto-add notice plus the attributed recipe warning.
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("AutoAdded"));
        assert!(warnings[1].contains("modded:machine"));
    }

    #[test]
    fn safe_mode_forces_vanilla_only() {
        let world = TempDir::new("safe-mode");
        let pack = world.0.join("datapacks/tweaks");
        std::fs::create_dir_all(pack.join("data/minecraft/recipe")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(
            pack.join("data/minecraft/recipe/extra.json"),
            r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:stick"}}"#,
        )
        .unwrap();

        let startup = DatapackStartup::load(&world_pack_options(&world.0, true)).unwrap();
        assert_eq!(startup.selection.ordered_ids, ["vanilla".to_string()]);
        assert_eq!(
            startup.selection.data_packs(),
            &Java26DataPacks::new(vec!["vanilla".to_string()], Vec::new())
        );
        assert_eq!(startup.recipes.recipes.len(), 1515);
        assert_eq!(startup.warnings(), Vec::<String>::new());
    }

    #[test]
    fn world_datapack_overlays_fixed_registry_tags() {
        let world = TempDir::new("fixed-tags");
        let pack = world.0.join("datapacks/tweaks");
        std::fs::create_dir_all(pack.join("data/minecraft/tags/block/mineable")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(
            pack.join("data/minecraft/tags/block/mineable/pickaxe.json"),
            r#"{"replace":true,"values":["minecraft:stone"]}"#,
        )
        .unwrap();

        std::fs::create_dir_all(pack.join("data/minecraft/dimension_type")).unwrap();
        std::fs::create_dir_all(pack.join("data/test/tags/timeline")).unwrap();
        std::fs::write(
            pack.join("data/test/tags/timeline/custom.json"),
            r#"{"values":[]}"#,
        )
        .unwrap();
        let snapshot = crate::vanilla_snapshot::VanillaDatapackSnapshot::embedded_shared().unwrap();
        let mut dimension: serde_json::Value =
            serde_json::from_slice(snapshot.read_path("dimension_type/overworld.json").unwrap())
                .unwrap();
        dimension["attributes"] = serde_json::json!({});
        dimension["timelines"] = serde_json::json!("#test:custom");
        std::fs::write(
            pack.join("data/minecraft/dimension_type/overworld.json"),
            serde_json::to_vec(&dimension).unwrap(),
        )
        .unwrap();

        let startup = DatapackStartup::load(&world_pack_options(&world.0, false)).unwrap();
        assert!(startup.tags.warnings.is_empty());
        assert_eq!(
            startup
                .tags
                .registry("minecraft:block")
                .and_then(|registry| registry.tag("minecraft:mineable/pickaxe"))
                .unwrap(),
            ["minecraft:stone"]
        );
        let sources =
            crate::environment_attributes::EnvironmentAttributeSources::new(&startup).unwrap();
        let sky = crate::environment_attributes::DimensionFloatAttribute::from_attribute(
            &sources,
            "minecraft:overworld",
            bbb_facts::environment_attribute_types::vanilla_environment_attribute(
                "minecraft:gameplay/sky_light_level",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(sky.sample_unclamped(None, |_| None), Some(15.0));
    }

    #[test]
    fn reload_reuses_existing_registries_and_resolves_tags_against_them() {
        let current = DatapackStartup::load(&ephemeral_options()).unwrap();
        let current_registries = current.registries.clone();
        let mut current_data_packs = current.selection.data_packs().clone();
        current_data_packs.disabled.push("file/blocked".into());
        let world = TempDir::new("reload-existing-registries");
        let pack = world.0.join("datapacks/tweaks");
        std::fs::create_dir_all(pack.join("data/minecraft/damage_type")).unwrap();
        std::fs::create_dir_all(pack.join("data/minecraft/tags/damage_type")).unwrap();
        std::fs::create_dir_all(pack.join("data/minecraft/recipe")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        // Startup would reject this registry element. Reload never opens it,
        // because the running registry access is already frozen.
        std::fs::write(
            pack.join("data/minecraft/damage_type/reload_only.json"),
            "{not-json",
        )
        .unwrap();
        std::fs::write(
            pack.join("data/minecraft/tags/damage_type/reload_only.json"),
            r#"{"values":["minecraft:reload_only"]}"#,
        )
        .unwrap();
        std::fs::write(
            pack.join("data/minecraft/recipe/reload_only.json"),
            r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:stone"}}"#,
        )
        .unwrap();
        let blocked = world.0.join("datapacks/blocked");
        std::fs::create_dir_all(&blocked).unwrap();
        std::fs::write(blocked.join("pack.mcmeta"), VALID_MCMETA).unwrap();

        let mut reload_options = world_pack_options(&world.0, true);
        reload_options.saved_data_packs = Some(current_data_packs);
        let reloaded = DatapackStartup::reload(&reload_options, &current).unwrap();

        assert!(reloaded
            .selection
            .ordered_ids
            .contains(&"file/tweaks".to_string()));
        assert!(!reloaded
            .selection
            .ordered_ids
            .contains(&"file/blocked".into()));
        assert!(reloaded
            .selection
            .data_packs()
            .disabled()
            .contains(&"file/blocked".into()));
        assert_eq!(reloaded.registries, current_registries);
        assert!(reloaded
            .registries
            .registry("minecraft:damage_type")
            .unwrap()
            .elements
            .iter()
            .all(|element| element.id != "minecraft:reload_only"));
        assert!(reloaded
            .tags
            .registry("minecraft:damage_type")
            .and_then(|registry| registry.tag("minecraft:reload_only"))
            .is_none());
        assert!(reloaded.tags.warnings.iter().any(|warning| {
            warning.registry_id == "minecraft:damage_type"
                && warning.tag_id.as_deref() == Some("minecraft:reload_only")
        }));
        assert!(reloaded.recipes.recipe("minecraft:reload_only").is_some());
    }

    /// `reloadResources` opens exactly the ids it is handed
    /// (`MinecraftServer.java:1505-1508`) and only then makes them the
    /// selection (`:1534`), so the order `/datapack enable first` chose
    /// survives the reload and every unnamed pack lands disabled.
    #[test]
    fn a_reload_of_a_named_selection_keeps_its_order() {
        let previous = DatapackStartup::load(&ephemeral_options()).unwrap();
        let world = TempDir::new("reload-named-selection");
        for name in ["aaa", "zzz"] {
            let pack = world.0.join("datapacks").join(name);
            std::fs::create_dir_all(&pack).unwrap();
            std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        }

        let options = world_pack_options(&world.0, false);
        let reloaded = DatapackStartup::reload_with_selected_ids(
            &options,
            &["file/zzz".to_string(), "vanilla".to_string()],
            &previous,
        )
        .unwrap();
        assert_eq!(
            reloaded.selection.ordered_ids,
            ["file/zzz".to_string(), "vanilla".to_string()]
        );
        // `getSelectedPacks(repository, true)` (`MinecraftServer.java:1649-1653`)
        // writes every available id the new selection left out, which is the
        // unnamed world pack and the three feature packs.
        assert_eq!(
            reloaded.selection.data_packs().disabled(),
            [
                "file/aaa".to_string(),
                "minecart_improvements".to_string(),
                "redstone_experiments".to_string(),
                "trade_rebalance".to_string(),
            ]
        );
    }

    #[test]
    fn saved_selection_is_respected() {
        let world = TempDir::new("saved-selection");
        let pack = world.0.join("datapacks/tweaks");
        std::fs::create_dir_all(pack.join("data/minecraft/recipe")).unwrap();
        std::fs::write(pack.join("pack.mcmeta"), VALID_MCMETA).unwrap();
        std::fs::write(
            pack.join("data/minecraft/recipe/extra.json"),
            r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:stick"}}"#,
        )
        .unwrap();

        let mut options = world_pack_options(&world.0, false);
        options.saved_data_packs = Some(Java26DataPacks::new(
            vec!["vanilla".to_string(), "file/tweaks".to_string()],
            Vec::new(),
        ));
        let startup = DatapackStartup::load(&options).unwrap();
        assert_eq!(
            startup.selection.ordered_ids,
            ["vanilla".to_string(), "file/tweaks".to_string()]
        );
        assert_eq!(startup.recipes.recipes.len(), 1516);

        // A saved selection naming a pack absent from `datapacks/` is dropped from
        // the load order with a notice (`MinecraftServer` warn-and-drop).
        let empty_world = TempDir::new("saved-selection-missing");
        let mut options = world_pack_options(&empty_world.0, false);
        options.saved_data_packs = Some(Java26DataPacks::new(
            vec!["vanilla".to_string(), "file/removed".to_string()],
            Vec::new(),
        ));
        let startup = DatapackStartup::load(&options).unwrap();
        assert_eq!(startup.selection.ordered_ids, ["vanilla".to_string()]);
        assert!(!startup.selection.notices.is_empty());
    }

    #[test]
    fn startup_shares_embedded_json_bytes_and_trims_transient_asts() {
        let snapshot = VanillaDatapackSnapshot::embedded_shared().unwrap();
        let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
        for trimmed in [
            "minecraft:worldgen/biome",
            "minecraft:worldgen/template_pool",
            "minecraft:villager_trade",
            "minecraft:damage_type",
        ] {
            assert!(
                startup
                    .registries
                    .registry(trimmed)
                    .unwrap()
                    .elements
                    .iter()
                    .all(|element| element.source.is_none()),
                "{trimmed} retained transient AST"
            );
        }
        assert!(startup
            .registries
            .registry("minecraft:enchantment")
            .unwrap()
            .elements
            .iter()
            .all(|element| element.source.is_some()));
        let plains = startup
            .registries
            .element("minecraft:worldgen/biome", "minecraft:plains")
            .unwrap();
        let shared = snapshot
            .read_shared_path("worldgen/biome/plains.json")
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(&plains.source_json, &shared));
    }

    #[test]
    fn world_datapack_evaluates_pack_mcmeta_overlays_in_folder_and_zip_packs() {
        use std::io::{Cursor, Write};

        let mcmeta = r#"{
            "pack": {
                "description": "overlay pack",
                "min_format": [101, 1],
                "max_format": [101, 1]
            },
            "overlays": {
                "entries": [
                    { "min_format": [99, 0], "max_format": [99, 9], "directory": "legacy" },
                    { "min_format": [101, 0], "max_format": [101, 1], "directory": "v101_a" },
                    { "min_format": [101, 1], "max_format": [101, 1], "directory": "v101_b" }
                ]
            }
        }"#;
        let recipe_base = r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:stick","count":1}}"#;
        let recipe_v101_a = r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:stick","count":2}}"#;
        let recipe_v101_b = r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:stick","count":3}}"#;
        let recipe_legacy = r#"{"type":"minecraft:crafting_shapeless","ingredients":["minecraft:oak_planks"],"result":{"id":"minecraft:stick","count":9}}"#;

        // Folder pack with active and inactive overlays.
        let world_dir = TempDir::new("startup-overlays-folder");
        let pack_dir = world_dir.0.join("datapacks/overlay_dir");
        std::fs::create_dir_all(pack_dir.join("data/minecraft/recipe")).unwrap();
        std::fs::create_dir_all(pack_dir.join("v101_a/data/minecraft/recipe")).unwrap();
        std::fs::create_dir_all(pack_dir.join("v101_b/data/minecraft/recipe")).unwrap();
        std::fs::create_dir_all(pack_dir.join("legacy/data/minecraft/recipe")).unwrap();
        std::fs::write(pack_dir.join("pack.mcmeta"), mcmeta).unwrap();
        std::fs::write(
            pack_dir.join("data/minecraft/recipe/overlay_base.json"),
            recipe_base,
        )
        .unwrap();
        std::fs::write(
            pack_dir.join("v101_a/data/minecraft/recipe/overlay_base.json"),
            recipe_v101_a,
        )
        .unwrap();
        std::fs::write(
            pack_dir.join("v101_b/data/minecraft/recipe/overlay_base.json"),
            recipe_v101_b,
        )
        .unwrap();
        std::fs::write(
            pack_dir.join("v101_a/data/minecraft/recipe/overlay_only_a.json"),
            recipe_v101_a,
        )
        .unwrap();
        std::fs::write(
            pack_dir.join("legacy/data/minecraft/recipe/overlay_legacy.json"),
            recipe_legacy,
        )
        .unwrap();

        let startup = DatapackStartup::load(&world_pack_options(&world_dir.0, false)).unwrap();
        let base_recipe = startup.recipes.recipe("minecraft:overlay_base").unwrap();
        assert_eq!(base_recipe.source_pack, "file/overlay_dir");
        assert!(startup.recipes.recipe("minecraft:overlay_only_a").is_some());
        assert!(startup.recipes.recipe("minecraft:overlay_legacy").is_none());
        let crate::recipe::RecipeKind::Shapeless(base_shapeless) = &base_recipe.recipe.kind else {
            panic!("expected shapeless recipe");
        };
        assert_eq!(base_shapeless.result.count, 3);

        // Zip pack with identical overlay metadata and files.
        let world_zip = TempDir::new("startup-overlays-zip");
        let zip_path = world_zip.0.join("datapacks/overlay_zip.zip");
        std::fs::create_dir_all(zip_path.parent().unwrap()).unwrap();
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            let options = zip::write::SimpleFileOptions::default();
            for (name, body) in [
                ("pack.mcmeta", mcmeta),
                ("data/minecraft/recipe/overlay_base.json", recipe_base),
                (
                    "v101_a/data/minecraft/recipe/overlay_base.json",
                    recipe_v101_a,
                ),
                (
                    "v101_b/data/minecraft/recipe/overlay_base.json",
                    recipe_v101_b,
                ),
                (
                    "v101_a/data/minecraft/recipe/overlay_only_a.json",
                    recipe_v101_a,
                ),
                (
                    "legacy/data/minecraft/recipe/overlay_legacy.json",
                    recipe_legacy,
                ),
            ] {
                writer.start_file(name, options).unwrap();
                writer.write_all(body.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        std::fs::write(&zip_path, cursor.into_inner()).unwrap();

        let zip_startup = DatapackStartup::load(&world_pack_options(&world_zip.0, false)).unwrap();
        let zip_base = zip_startup
            .recipes
            .recipe("minecraft:overlay_base")
            .unwrap();
        assert_eq!(zip_base.source_pack, "file/overlay_zip.zip");
        assert!(zip_startup
            .recipes
            .recipe("minecraft:overlay_only_a")
            .is_some());
        assert!(zip_startup
            .recipes
            .recipe("minecraft:overlay_legacy")
            .is_none());
        let crate::recipe::RecipeKind::Shapeless(zip_shapeless) = &zip_base.recipe.kind else {
            panic!("expected shapeless recipe");
        };
        assert_eq!(zip_shapeless.result.count, 3);
    }

    #[test]
    fn reload_shares_immutable_generation_and_catalogs_via_arc() {
        let world = TempDir::new("startup-reload-arc");
        let options = world_pack_options(&world.0, false);
        let first = DatapackStartup::load(&options).unwrap();
        let pools_a = first.structure_pools_shared().unwrap();
        let pools_b = first.structure_pools_shared().unwrap();
        assert!(Arc::ptr_eq(&pools_a, &pools_b));

        let spawns_a = first.mob_spawn_catalog_shared().unwrap();
        let spawns_b = first.mob_spawn_catalog_shared().unwrap();
        assert!(Arc::ptr_eq(&spawns_a, &spawns_b));

        let reloaded = DatapackStartup::reload(&options, &first).unwrap();
        assert!(Arc::ptr_eq(
            first.registries.as_arc(),
            reloaded.registries.as_arc()
        ));
        assert!(Arc::ptr_eq(
            &pools_a,
            &reloaded.structure_pools_shared().unwrap()
        ));
        assert!(Arc::ptr_eq(
            &spawns_a,
            &reloaded.mob_spawn_catalog_shared().unwrap()
        ));
        assert!(first.recipes.recipe("minecraft:crafting_table").is_some());
        assert!(!first.recipe_index.crafting_indices().is_empty());
        assert!(first.recipe_index.crafting_indices().len() < first.recipes.recipes.len());
    }

    #[test]
    fn startup_with_content_registry_loads_custom_registries_tags_loot_recipes_trades_advancements_and_structures(
    ) {
        use crate::frozen_lookup::FrozenDatapackRegistryLookup;
        use crate::player_advancements::{
            advancements_update_to_wire, AdvancementTree, PlayerAdvancements,
        };
        use crate::recipe::CraftingInput;
        use crate::structure_template::{
            MaterializedStructureTemplate, StructureTemplate, StructureTemplateBlock,
            StructureTemplatePalette,
        };
        use bbb_protocol::content_registry::{
            ContentRegistryBuilder, ModBlockRegistration, ModEntityRegistration,
            ModItemRegistration, ModParticleRegistration,
        };
        use bbb_protocol::packets::{BlockPos, ItemStackSummary};

        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "example:obsidian_brick".to_string(),
            state_count: 2,
            light_emission: 0,
            destroy_speed: 5.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_item(ModItemRegistration {
            resource_id: "example:ruby_ingot".to_string(),
            max_stack_size: 16,
            is_block_item: false,
        });
        builder.register_entity(ModEntityRegistration {
            resource_id: "example:inferno_warden".to_string(),
            ..Default::default()
        });
        builder.register_sound_event("example:warden_roar");
        builder.register_particle(ModParticleRegistration {
            resource_id: "example:ember".to_string(),
            override_limiter: false,
        });
        let content_registry = builder.freeze_for_session().unwrap();
        let ruby_id = content_registry
            .item_by_resource_id("example:ruby_ingot")
            .unwrap()
            .id;
        let brick = content_registry
            .block_by_resource_id("example:obsidian_brick")
            .unwrap();

        let template_bytes = StructureTemplate::from_materialized_with_content_registry(
            MaterializedStructureTemplate {
                size: [2, 1, 1],
                palettes: vec![StructureTemplatePalette {
                    blocks: vec![
                        StructureTemplateBlock {
                            position: BlockPos { x: 0, y: 0, z: 0 },
                            state: brick.first_state_id,
                            nbt: None,
                        },
                        StructureTemplateBlock {
                            position: BlockPos { x: 1, y: 0, z: 0 },
                            state: brick.first_state_id + 1,
                            nbt: None,
                        },
                    ],
                    jigsaws: Vec::new(),
                }],
                entities: Vec::new(),
            },
            &content_registry,
        )
        .unwrap()
        .raw()
        .to_vec();

        let world = TempDir::new("startup-content-registry");
        let pack = world.0.join("datapacks/mod_pack");
        std::fs::create_dir_all(&pack).unwrap();
        std::fs::write(
            pack.join("pack.mcmeta"),
            r#"{"pack":{"description":"mod test","min_format":101,"max_format":101}}"#,
        )
        .unwrap();

        let write_file = |rel: &str, bytes: &[u8]| {
            let full = pack.join(rel);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, bytes).unwrap();
        };

        // Dynamic synchronized registry referencing custom sound event (R16-04 / R10-11).
        write_file(
            "data/example/jukebox_song/warden_theme.json",
            br#"{"comparator_output":7,"description":{"text":"Warden Theme"},"length_in_seconds":42.0,"sound_event":"example:warden_roar"}"#,
        );
        // Static registry tags containing custom item, block, and entity types.
        write_file(
            "data/example/tags/item/gems.json",
            br#"{"values":["example:ruby_ingot"]}"#,
        );
        write_file(
            "data/example/tags/block/bricks.json",
            br#"{"values":["example:obsidian_brick"]}"#,
        );
        write_file(
            "data/example/tags/entity_type/bosses.json",
            br#"{"values":["example:inferno_warden"]}"#,
        );
        // Loot table dropping the custom item and checking custom block state property.
        write_file(
            "data/example/loot_table/warden_drop.json",
            br#"{"pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"example:ruby_ingot","conditions":[{"condition":"minecraft:block_state_property","block":"example:obsidian_brick","properties":{"bbb_state_offset":"1"}}]}]}]}"#,
        );
        // Shaped recipe using custom item tag and producing count=4 of the custom item.
        write_file(
            "data/example/recipe/ruby_quad.json",
            br##"{"type":"minecraft:crafting_shaped","pattern":["#"],"key":{"#":"#example:gems"},"result":{"id":"example:ruby_ingot","count":4}}"##,
        );
        // Villager trade giving the custom item.
        write_file(
            "data/example/villager_trade/ruby_offer.json",
            br#"{"wants":{"id":"minecraft:emerald","count":2},"gives":{"id":"example:ruby_ingot","count":1}}"#,
        );
        // Advancement with custom block contact, bee nest trigger, and custom item icon.
        write_file(
            "data/example/advancement/ruby_found.json",
            br#"{"display":{"icon":{"id":"example:ruby_ingot"},"title":"Ruby","description":"Found"},"criteria":{"step":{"trigger":"minecraft:enter_block","conditions":{"block":"example:obsidian_brick","state":{"bbb_state_offset":"1"}}},"hive":{"trigger":"minecraft:bee_nest_destroyed","conditions":{"block":"example:obsidian_brick"}}}}"#,
        );
        // Structure template with custom multi-state block palette.
        write_file("data/example/structure/shrine.nbt", &template_bytes);

        let startup = DatapackStartup::load_with_content_registry(
            &world_pack_options(&world.0, false),
            &content_registry,
        )
        .unwrap();

        // 1. Dynamic registry + frozen references.
        assert!(startup
            .registries
            .element("minecraft:jukebox_song", "example:warden_theme")
            .is_some());
        startup.validate_frozen_registry_references().unwrap();

        // 2. Tags bind custom elements without dropping them.
        assert_eq!(
            startup
                .tags
                .registry("minecraft:item")
                .unwrap()
                .tag("example:gems"),
            Some(&["example:ruby_ingot".to_string()][..])
        );
        assert_eq!(
            startup
                .tags
                .registry("minecraft:block")
                .unwrap()
                .tag("example:bricks"),
            Some(&["example:obsidian_brick".to_string()][..])
        );
        assert_eq!(
            startup
                .tags
                .registry("minecraft:entity_type")
                .unwrap()
                .tag("example:bosses"),
            Some(&["example:inferno_warden".to_string()][..])
        );

        // 3. Loot table loaded with custom item id.
        let loot_table = startup.loot.table("example:warden_drop").unwrap();
        assert_eq!(
            loot_table.pools[0].entries[0].kind,
            crate::loot::LootEntryKind::Item(ruby_id)
        );

        // 4. Recipe + display catalog + ItemStackTemplate prototype defaults (max_stack_size=16).
        let recipe = startup.recipes.recipe("example:ruby_quad").unwrap();
        let item_tags = ItemTagIndex::from_loaded_with_content_registry(
            startup.tags.registry("minecraft:item").unwrap(),
            startup.content_registry(),
        );
        let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
        let input = CraftingInput::of(
            1,
            1,
            vec![ItemStackSummary {
                item_id: Some(ruby_id),
                count: 1,
                component_patch: Default::default(),
            }],
        );
        assert!(recipe
            .recipe
            .matches_crafting(&lookup, &input, &item_tags, None));
        let crafted = recipe.recipe.assemble_crafting(&input, &item_tags);
        assert_eq!(crafted.item_id, Some(ruby_id));
        assert_eq!(crafted.count, 4);
        assert!(!startup
            .recipe_displays
            .displays_for_recipe("example:ruby_quad")
            .is_empty());

        // 5. Villager trade catalog loaded with custom item.
        let trade = startup.trades.get("example:ruby_offer").unwrap();
        assert_eq!(trade.gives.item_id, "example:ruby_ingot");
        assert_eq!(trade.gives.count, 1);

        // 6. Advancement loaded, decoded into AdvancementTree, and projected to wire with custom icon.
        assert!(startup
            .advancements
            .advancement("example:ruby_found")
            .is_some());
        let tree = AdvancementTree::from_loaded_with_content_registry(
            &startup.advancements,
            startup.content_registry(),
        );
        let node_index = tree.index_of("example:ruby_found").unwrap();
        let mut player = PlayerAdvancements::created(&tree);
        player.award(&tree, node_index, &"step".into(), 1000);
        player.award(&tree, node_index, &"hive".into(), 1001);
        let update = player.flush_dirty(&tree, true).unwrap();
        let wire = advancements_update_to_wire(&update, &lookup).unwrap();
        let added = wire
            .added
            .iter()
            .find(|entry| entry.id == "example:ruby_found")
            .unwrap();
        assert_eq!(added.display.as_ref().unwrap().icon.item_id, ruby_id);

        // 7. Structure template round-tripped custom block states with offsets 0 and 1.
        let shrine = startup.structures.template("example:shrine").unwrap();
        let blocks = &shrine.template.materialized().palettes[0].blocks;
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].state, brick.first_state_id);
        assert_eq!(blocks[1].state, brick.first_state_id + 1);
    }
}
