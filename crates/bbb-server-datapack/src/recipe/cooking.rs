//! What a furnace, blast furnace, smoker and campfire ask the recipe library
//! for: the `RecipeManager` lookup each runs over its own cooking type with one
//! input stack (`RecipeManager.java:131-146`), and the cooking time and
//! experience the chosen body declares. The block entity, its menu and the burn
//! tick stay outside.

use bbb_protocol::packets::ItemStackSummary;

use super::ingredient::{stack_is_empty, ItemTagIndex};
use super::other::{CookingKind, CookingRecipe};
use super::{Recipe, RecipeElement, RecipeKind, RecipeSet};

impl CookingKind {
    /// The `RecipeType` each concrete body returns (`RecipeType.java:9-12`).
    pub fn recipe_type_id(self) -> &'static str {
        match self {
            Self::Smelting => "minecraft:smelting",
            Self::Blasting => "minecraft:blasting",
            Self::Smoking => "minecraft:smoking",
            Self::Campfire => "minecraft:campfire_cooking",
        }
    }

    /// The `cookingMapCodec` default `cookingtime` each kind passes
    /// (`AbstractCookingRecipe.java:82`, `SmeltingRecipe.java:11`,
    /// `BlastingRecipe.java:11`, `SmokingRecipe.java:11`,
    /// `CampfireCookingRecipe.java:11`).
    pub fn default_cooking_time(self) -> i32 {
        match self {
            Self::Smelting => 200,
            Self::Blasting | Self::Smoking | Self::Campfire => 100,
        }
    }
}

impl CookingRecipe {
    /// `SingleItemRecipe.matches` (`SingleItemRecipe.java:30-32`).
    pub fn matches(&self, input: &ItemStackSummary, tags: &ItemTagIndex) -> bool {
        self.ingredient.test(input, tags)
    }

    /// `SingleItemRecipe.assemble` (`SingleItemRecipe.java:56-58`).
    pub fn assemble(&self) -> ItemStackSummary {
        self.result.create()
    }
}

impl Recipe {
    /// The cooking body this recipe carries, if any.
    pub fn cooking(&self) -> Option<&CookingRecipe> {
        match &self.kind {
            RecipeKind::Cooking(recipe) => Some(recipe),
            _ => None,
        }
    }

    /// The type filter of `RecipeMap.byType` (`RecipeMap.java:36-38`) and of
    /// `RecipeManager.byKeyTyped` (`RecipeManager.java:152-155`).
    pub fn cooking_of_kind(&self, kind: CookingKind) -> Option<&CookingRecipe> {
        self.cooking().filter(|recipe| recipe.kind == kind)
    }
}

impl RecipeSet {
    /// `RecipeManager.getRecipeFor` (`RecipeManager.java:131-146`) over a
    /// cooking type: a hint answers only when `byKeyTyped` (`:152-155`) admits
    /// it as this type and it still matches, otherwise
    /// `RecipeMap.getRecipesFor` (`RecipeMap.java:48-50`) takes the first match
    /// of that type alone, in the order `RecipeMap.create` (`:24-34`) received.
    pub fn get_recipe_for_cooking(
        &self,
        kind: CookingKind,
        input: &ItemStackSummary,
        hint: Option<&str>,
        tags: &ItemTagIndex,
    ) -> Option<&RecipeElement> {
        if let Some(element) = hint.and_then(|id| self.recipe(id)) {
            if element
                .recipe
                .cooking_of_kind(kind)
                .is_some_and(|recipe| recipe.matches(input, tags))
            {
                return Some(element);
            }
        }
        if stack_is_empty(input) {
            return None;
        }
        self.recipes.iter().find(|element| {
            element
                .recipe
                .cooking_of_kind(kind)
                .is_some_and(|recipe| recipe.matches(input, tags))
        })
    }
}

/// `RecipeManager.createCheck` (`RecipeManager.java:197-214`): the per-block
/// hint a furnace or campfire carries, replaced on every match and left alone
/// on a miss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookingCheck {
    kind: CookingKind,
    last_recipe: Option<String>,
}

impl CookingCheck {
    pub fn new(kind: CookingKind) -> Self {
        Self {
            kind,
            last_recipe: None,
        }
    }

    pub fn kind(&self) -> CookingKind {
        self.kind
    }

    pub fn last_recipe(&self) -> Option<&str> {
        self.last_recipe.as_deref()
    }

    pub fn get_recipe_for<'a>(
        &mut self,
        recipes: &'a RecipeSet,
        input: &ItemStackSummary,
        tags: &ItemTagIndex,
    ) -> Option<&'a RecipeElement> {
        let found =
            recipes.get_recipe_for_cooking(self.kind, input, self.last_recipe.as_deref(), tags)?;
        self.last_recipe = Some(found.id.clone());
        Some(found)
    }
}
