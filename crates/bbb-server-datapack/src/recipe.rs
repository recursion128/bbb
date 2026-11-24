//! `RecipeManager` surface (`RecipeManager.java:69-80`): the `recipe/` tree decoded
//! through the 26.1 `RecipeSerializer` dispatch, winner-take-all, warn-only per
//! recipe (`SimpleJsonResourceReloadListener.scanDirectory`).

use std::fmt;

use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::JavaString;
use bbb_registry_codec::{
    decode_direct_registry_json, registry_codec::RegistryReferenceLookup,
    PersistentItemStackHolderResolver, RegistryElementValueRef,
};

use super::registry_data_loader::DatapackLoadLimits;
use super::resource_overlay::{MergedServerDataView, PackOverlayInput};

pub mod book;
mod components;
pub mod cooking;
mod display_catalog;
pub mod fuel;
pub mod ingredient;
pub mod input;
mod other;
pub mod placement;
mod shaped;
mod shapeless;
mod source;
mod special;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod matcher_tests;

#[cfg(test)]
mod display_catalog_tests;

#[cfg(test)]
mod component_tests;

#[cfg(test)]
mod placement_tests;

#[cfg(test)]
mod cooking_tests;

#[cfg(test)]
mod fuel_tests;

#[cfg(test)]
mod book_tests;

use other::{SmithingTransformRecipe, SmithingTrimRecipe, StonecuttingRecipe};
use shaped::ShapedRecipe;
use shapeless::ShapelessRecipe;
use special::{
    BannerDuplicateRecipe, BookCloningRecipe, DecoratedPotRecipe, DyeRecipe, FireworkRocketRecipe,
    FireworkStarFadeRecipe, FireworkStarRecipe, ImbueRecipe, MapExtendingRecipe, RepairItemRecipe,
    ShieldDecorationRecipe, TransmuteRecipe,
};

pub use book::{BookAdd, BookRefusal, BookRefusalReason, RecipeBookLibrary};
pub use cooking::CookingCheck;
pub use display_catalog::RecipeDisplayCatalog;
pub use fuel::{
    BurnTime, FuelRefusal, FuelRefusalReason, FuelRow, FuelTable, FuelValues, FurnaceFuelKind,
    VANILLA_BURN_TIME_ROWS, VANILLA_FUEL_BASE_UNIT,
};
pub use ingredient::{crafting_remainder, Ingredient, ItemStackTemplate, ItemTagIndex};
pub use input::{CraftingInput, CraftingInputPositioned};
pub use other::{CookingKind, CookingRecipe};
pub use placement::{GridPlacement, PlacementInfo, PlacementRefusal, EMPTY_SLOT};

/// Actual saved data for the first MAP_ID in this crafting input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftingMapData {
    pub scale: i8,
    pub exploration: bool,
}

/// 26.1 `RecipeSerializers` bootstrap order (`RecipeSerializers.java:6-28`).
pub const RECIPE_SERIALIZER_TYPES_26_1: &[&str] = &[
    "minecraft:blasting",
    "minecraft:campfire_cooking",
    "minecraft:crafting_decorated_pot",
    "minecraft:crafting_dye",
    "minecraft:crafting_imbue",
    "minecraft:crafting_shaped",
    "minecraft:crafting_shapeless",
    "minecraft:crafting_special_bannerduplicate",
    "minecraft:crafting_special_bookcloning",
    "minecraft:crafting_special_firework_rocket",
    "minecraft:crafting_special_firework_star",
    "minecraft:crafting_special_firework_star_fade",
    "minecraft:crafting_special_mapextending",
    "minecraft:crafting_special_repairitem",
    "minecraft:crafting_special_shielddecoration",
    "minecraft:crafting_transmute",
    "minecraft:smelting",
    "minecraft:smithing_transform",
    "minecraft:smithing_trim",
    "minecraft:smoking",
    "minecraft:stonecutting",
];

/// A recipe decode failure, attributed with the recipe id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeDecodeError {
    pub message: String,
}

impl fmt::Display for RecipeDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RecipeDecodeError {}

/// Typed 26.1 recipe body.
#[derive(Debug, Clone, PartialEq)]
pub enum RecipeKind {
    Shaped(ShapedRecipe),
    Shapeless(ShapelessRecipe),
    Transmute(TransmuteRecipe),
    Dye(DyeRecipe),
    Imbue(ImbueRecipe),
    DecoratedPot(DecoratedPotRecipe),
    BookCloning(BookCloningRecipe),
    MapExtending(MapExtendingRecipe),
    FireworkRocket(FireworkRocketRecipe),
    FireworkStar(FireworkStarRecipe),
    FireworkStarFade(FireworkStarFadeRecipe),
    BannerDuplicate(BannerDuplicateRecipe),
    ShieldDecoration(ShieldDecorationRecipe),
    RepairItem,
    Cooking(CookingRecipe),
    SmithingTransform(SmithingTransformRecipe),
    SmithingTrim(SmithingTrimRecipe),
    Stonecutting(StonecuttingRecipe),
}

/// One decoded recipe: serializer id plus the typed body.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub type_id: String,
    pub kind: RecipeKind,
    group: JavaString,
    category_id: i32,
    show_notification: bool,
}

impl Recipe {
    pub fn is_crafting(&self) -> bool {
        !matches!(
            self.kind,
            RecipeKind::Cooking(_)
                | RecipeKind::SmithingTransform(_)
                | RecipeKind::SmithingTrim(_)
                | RecipeKind::Stonecutting(_)
        )
    }

    /// `CustomRecipe.isSpecial` (`CustomRecipe.java:5-6`).
    pub fn is_special(&self) -> bool {
        matches!(
            self.kind,
            RecipeKind::DecoratedPot(_)
                | RecipeKind::BookCloning(_)
                | RecipeKind::MapExtending(_)
                | RecipeKind::FireworkRocket(_)
                | RecipeKind::FireworkStar(_)
                | RecipeKind::FireworkStarFade(_)
                | RecipeKind::BannerDuplicate(_)
                | RecipeKind::ShieldDecoration(_)
                | RecipeKind::RepairItem
        )
    }

    pub fn show_notification(&self) -> bool {
        self.show_notification
    }

    pub fn group(&self) -> &JavaString {
        &self.group
    }

    pub fn category_id(&self) -> i32 {
        self.category_id
    }

    pub fn matches_crafting(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        input: &CraftingInput,
        tags: &ItemTagIndex,
        map: Option<CraftingMapData>,
    ) -> bool {
        match &self.kind {
            RecipeKind::Shaped(recipe) => recipe.matches(input, tags),
            RecipeKind::Shapeless(recipe) => recipe.matches(input, tags),
            RecipeKind::Transmute(recipe) => recipe.matches(resolver, input, tags),
            RecipeKind::Dye(recipe) => recipe.matches(input, tags),
            RecipeKind::Imbue(recipe) => recipe.matches(input, tags),
            RecipeKind::DecoratedPot(recipe) => recipe.matches(input, tags),
            RecipeKind::BookCloning(recipe) => recipe.matches(input, tags),
            RecipeKind::MapExtending(recipe) => recipe.matches(input, tags, map),
            RecipeKind::FireworkRocket(recipe) => recipe.matches(input, tags),
            RecipeKind::FireworkStar(recipe) => recipe.matches(input, tags),
            RecipeKind::FireworkStarFade(recipe) => recipe.matches(input, tags),
            RecipeKind::BannerDuplicate(recipe) => recipe.matches(input, tags),
            RecipeKind::ShieldDecoration(recipe) => recipe.matches(input, tags),
            RecipeKind::RepairItem => RepairItemRecipe.matches(input),
            _ => false,
        }
    }

    pub fn assemble_crafting(
        &self,
        input: &CraftingInput,
        tags: &ItemTagIndex,
    ) -> ItemStackSummary {
        match &self.kind {
            RecipeKind::Shaped(recipe) => recipe.result.create(),
            RecipeKind::Shapeless(recipe) => recipe.result.create(),
            RecipeKind::Transmute(recipe) => recipe.assemble(input, tags),
            RecipeKind::Dye(recipe) => recipe.assemble(input, tags),
            RecipeKind::Imbue(recipe) => recipe.assemble(input),
            RecipeKind::DecoratedPot(recipe) => recipe.assemble(input),
            RecipeKind::BookCloning(recipe) => recipe.assemble(input, tags),
            RecipeKind::MapExtending(recipe) => recipe.assemble(input),
            RecipeKind::FireworkRocket(recipe) => recipe.assemble(input, tags),
            RecipeKind::FireworkStar(recipe) => recipe.assemble(input, tags),
            RecipeKind::FireworkStarFade(recipe) => recipe.assemble(input, tags),
            RecipeKind::BannerDuplicate(recipe) => recipe.assemble(input),
            RecipeKind::ShieldDecoration(recipe) => recipe.assemble(input, tags),
            RecipeKind::RepairItem => special::RepairItemRecipe.assemble(input, tags),
            _ => ItemStackSummary::empty(),
        }
    }

    pub fn remaining_items(&self, input: &CraftingInput) -> Vec<ItemStackSummary> {
        match &self.kind {
            RecipeKind::BookCloning(recipe) => recipe.remaining_items(input),
            RecipeKind::BannerDuplicate(recipe) => recipe.remaining_items(input),
            _ => special::default_remainder_plan(input),
        }
    }
}

/// One loaded recipe, attributed to the pack that won the overlay merge.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeElement {
    pub id: String,
    pub source_pack: String,
    pub recipe: Recipe,
}

/// One warn-only recipe loading event (`scanDirectory` logs and skips the element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeLoadWarning {
    pub element_id: String,
    pub pack: String,
    pub message: String,
}

/// The recipe map for one datapack view (`RecipeMap.create` equivalent: holders in
/// sorted id order).
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeSet {
    /// Recipes in `Identifier` order, which is what `RecipeManager.prepare`
    /// (`RecipeManager.java:69-81`) hands `RecipeMap.create`.
    pub recipes: Vec<RecipeElement>,
    pub warnings: Vec<RecipeLoadWarning>,
}

impl RecipeSet {
    /// Looks up a recipe by id in O(log N) when recipes are in Vanilla
    /// `Identifier` order, falling back to a linear scan for small or
    /// custom-ordered synthetic sets.
    pub fn recipe(&self, id: &str) -> Option<&RecipeElement> {
        if self.recipes.len() > 16 {
            let target = identifier_order(id);
            if let Ok(index) = self
                .recipes
                .binary_search_by(|recipe| identifier_order(&recipe.id).cmp(&target))
            {
                if self.recipes[index].id == id {
                    return Some(&self.recipes[index]);
                }
            }
        }
        self.recipes.iter().find(|recipe| recipe.id == id)
    }

    /// Pre-partitions crafting and cooking recipe indices once so repeated
    /// lookups scan only recipes of the requested type.
    pub fn build_index(&self) -> RecipeIndex {
        RecipeIndex::from_recipes(self)
    }

    /// `RecipeManager.getRecipeFor` (`RecipeManager.java:131-146`): hint first,
    /// then type-filtered scan.
    pub fn get_recipe_for_crafting(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        input: &CraftingInput,
        hint: Option<&str>,
        tags: &ItemTagIndex,
        map: Option<CraftingMapData>,
    ) -> Option<&RecipeElement> {
        if input.is_empty() {
            return None;
        }
        if let Some(id) = hint {
            if let Some(element) = self.recipe(id) {
                if element.recipe.matches_crafting(resolver, input, tags, map) {
                    return Some(element);
                }
            }
        }
        self.recipes
            .iter()
            .find(|element| element.recipe.matches_crafting(resolver, input, tags, map))
    }
}

/// Type-partitioned index into a [`RecipeSet`], avoiding full-catalog scans
/// across non-matching recipe families on every crafting or smelting check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecipeIndex {
    crafting: Vec<usize>,
    smelting: Vec<usize>,
    blasting: Vec<usize>,
    smoking: Vec<usize>,
    campfire: Vec<usize>,
}

impl RecipeIndex {
    pub fn from_recipes(set: &RecipeSet) -> Self {
        let mut crafting = Vec::new();
        let mut smelting = Vec::new();
        let mut blasting = Vec::new();
        let mut smoking = Vec::new();
        let mut campfire = Vec::new();
        for (index, element) in set.recipes.iter().enumerate() {
            match &element.recipe.kind {
                RecipeKind::Cooking(cooking) => match cooking.kind {
                    CookingKind::Smelting => smelting.push(index),
                    CookingKind::Blasting => blasting.push(index),
                    CookingKind::Smoking => smoking.push(index),
                    CookingKind::Campfire => campfire.push(index),
                },
                RecipeKind::SmithingTransform(_)
                | RecipeKind::SmithingTrim(_)
                | RecipeKind::Stonecutting(_) => {}
                _ => crafting.push(index),
            }
        }
        Self {
            crafting,
            smelting,
            blasting,
            smoking,
            campfire,
        }
    }

    pub fn crafting_indices(&self) -> &[usize] {
        &self.crafting
    }

    pub fn cooking_indices(&self, kind: CookingKind) -> &[usize] {
        match kind {
            CookingKind::Smelting => &self.smelting,
            CookingKind::Blasting => &self.blasting,
            CookingKind::Smoking => &self.smoking,
            CookingKind::Campfire => &self.campfire,
        }
    }

    pub fn get_recipe_for_crafting<'a>(
        &self,
        set: &'a RecipeSet,
        resolver: &dyn PersistentItemStackHolderResolver,
        input: &CraftingInput,
        hint: Option<&str>,
        tags: &ItemTagIndex,
        map: Option<CraftingMapData>,
    ) -> Option<&'a RecipeElement> {
        if input.is_empty() {
            return None;
        }
        if let Some(id) = hint {
            if let Some(element) = set.recipe(id) {
                if element.recipe.matches_crafting(resolver, input, tags, map) {
                    return Some(element);
                }
            }
        }
        self.crafting.iter().find_map(|&index| {
            let element = set.recipes.get(index)?;
            element
                .recipe
                .matches_crafting(resolver, input, tags, map)
                .then_some(element)
        })
    }

    pub fn get_recipe_for_cooking<'a>(
        &self,
        set: &'a RecipeSet,
        kind: CookingKind,
        input: &ItemStackSummary,
        hint: Option<&str>,
        tags: &ItemTagIndex,
    ) -> Option<&'a RecipeElement> {
        if let Some(element) = hint.and_then(|id| set.recipe(id)) {
            if element
                .recipe
                .cooking_of_kind(kind)
                .is_some_and(|recipe| recipe.matches(input, tags))
            {
                return Some(element);
            }
        }
        if ingredient::stack_is_empty(input) {
            return None;
        }
        self.cooking_indices(kind).iter().find_map(|&index| {
            let element = set.recipes.get(index)?;
            element
                .recipe
                .cooking_of_kind(kind)
                .is_some_and(|recipe| recipe.matches(input, tags))
                .then_some(element)
        })
    }
}

pub fn parse_recipe_source_with_lookup(
    root: RegistryElementValueRef<'_>,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Result<Recipe, RecipeDecodeError> {
    source::parse_recipe_source(root, lookup)
}

fn is_special_type(type_id: &str) -> bool {
    matches!(
        type_id,
        "minecraft:crafting_decorated_pot"
            | "minecraft:crafting_special_bannerduplicate"
            | "minecraft:crafting_special_bookcloning"
            | "minecraft:crafting_special_firework_rocket"
            | "minecraft:crafting_special_firework_star"
            | "minecraft:crafting_special_firework_star_fade"
            | "minecraft:crafting_special_mapextending"
            | "minecraft:crafting_special_repairitem"
            | "minecraft:crafting_special_shielddecoration"
    )
}

/// `RecipeManager.prepare`: winner-take-all scan of the `recipe/` tree with
/// warn-only per-recipe decode (`scanDirectory`), held in sorted id order.
pub fn load_recipes_with_lookup(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> RecipeSet {
    let mut recipes = Vec::new();
    let mut warnings = Vec::new();
    for namespace in view.namespaces() {
        for path in view.list(namespace, "recipe") {
            let Some(element_path) = path
                .strip_prefix("recipe/")
                .and_then(|path| path.strip_suffix(".json"))
            else {
                continue;
            };
            let element_id = format!("{namespace}:{element_path}");
            let Some((pack_id, resources)) = view.winner(packs, namespace, path) else {
                continue;
            };
            let Some(bytes) = resources.read(namespace, path) else {
                continue;
            };
            if bytes.len() > limits.max_element_bytes {
                warnings.push(RecipeLoadWarning {
                    element_id,
                    pack: pack_id.to_string(),
                    message: format!("recipe file exceeds the budget ({} bytes)", bytes.len()),
                });
                continue;
            }
            let parsed = decode_direct_registry_json(&bytes)
                .map_err(|e| RecipeDecodeError {
                    message: format!("invalid JSON: {e}"),
                })
                .and_then(|value| parse_recipe_source_with_lookup(value.root(), lookup));
            match parsed {
                Ok(recipe) => recipes.push(RecipeElement {
                    id: element_id,
                    source_pack: pack_id.to_string(),
                    recipe,
                }),
                Err(decode) => warnings.push(RecipeLoadWarning {
                    element_id,
                    pack: pack_id.to_string(),
                    message: format!("couldn't parse data file: {decode}"),
                }),
            }
        }
    }
    recipes.sort_by(|a, b| identifier_order(&a.id).cmp(&identifier_order(&b.id)));
    RecipeSet { recipes, warnings }
}

/// `Identifier.compareTo` (`Identifier.java:142-149`), which is the order the
/// `TreeMap` of `RecipeManager.prepare` (`RecipeManager.java:70`) hands
/// `RecipeMap.create` and so the order every lookup scans.
fn identifier_order(id: &str) -> (&str, &str) {
    match id.split_once(':') {
        Some((namespace, path)) => (path, namespace),
        None => (id, ""),
    }
}
