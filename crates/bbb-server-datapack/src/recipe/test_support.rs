use std::sync::OnceLock;

use serde_json::Value;

use crate::dynamic_registry::DatapackGeneration;
use crate::frozen_lookup::FrozenDatapackRegistryLookup;
use crate::registry_data_loader::DatapackLoadLimits;
use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
use crate::startup::{DatapackStartup, DatapackStartupOptions};
use crate::tag_loader::TagLoadReport;

use super::{
    load_recipes_with_lookup, parse_recipe_source_with_lookup, Recipe, RecipeDecodeError, RecipeSet,
};

pub(super) fn builtin_publication() -> &'static DatapackStartup {
    static PUBLICATION: OnceLock<DatapackStartup> = OnceLock::new();
    PUBLICATION.get_or_init(|| {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .expect("vanilla publication loads")
    })
}

pub(super) fn builtin_generation() -> &'static DatapackGeneration {
    &builtin_publication().registries
}

pub(super) fn builtin_tags() -> &'static TagLoadReport {
    &builtin_publication().tags
}

pub(crate) fn builtin_lookup() -> FrozenDatapackRegistryLookup<'static> {
    FrozenDatapackRegistryLookup::from_startup(builtin_publication())
}

/// Runs `with` over `generation` frozen the way startup freezes it, with the
/// built-in item prototypes.
pub(super) fn with_generation_lookup<T>(
    generation: &DatapackGeneration,
    with: impl FnOnce(&FrozenDatapackRegistryLookup<'_>) -> T,
) -> T {
    let tags = TagLoadReport::default();
    with(&FrozenDatapackRegistryLookup::new(generation, &tags))
}

pub(super) fn parse_recipe_with_builtin_lookup(value: &Value) -> Result<Recipe, RecipeDecodeError> {
    let bytes = serde_json::to_vec(value).expect("test recipe serializes");
    let decoded = bbb_registry_codec::decode_direct_registry_json(&bytes)
        .expect("test recipe source decodes");
    parse_recipe_source_with_lookup(decoded.root(), &builtin_lookup())
}

pub(super) fn load_recipes_with_builtin_lookup(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
) -> RecipeSet {
    load_recipes_with_lookup(view, packs, limits, &builtin_lookup())
}
