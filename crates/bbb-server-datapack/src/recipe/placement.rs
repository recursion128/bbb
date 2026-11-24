//! What `ServerPlaceRecipe` asks the recipe library for once a menu has chosen a
//! recipe: the `PlacementInfo` (`PlacementInfo.java:9-73`) that recipe compiles
//! its ingredients into, and the `PlaceRecipeHelper` walk
//! (`PlaceRecipeHelper.java:9-62`) turning it into one ingredient index per menu
//! grid slot. The slots themselves, the stack moves and the packet reply stay
//! with the menu.

use super::ingredient::{Ingredient, ItemTagIndex};
use super::{Recipe, RecipeKind};

/// `PlacementInfo.EMPTY_SLOT` (`PlacementInfo.java:10`).
pub const EMPTY_SLOT: i32 = -1;

/// `PlacementInfo` (`PlacementInfo.java:9-73`).
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementInfo<'a> {
    ingredients: Vec<&'a Ingredient>,
    slots_to_ingredient_index: Vec<i32>,
}

impl<'a> PlacementInfo<'a> {
    /// `PlacementInfo.NOT_PLACEABLE` (`PlacementInfo.java:11`).
    fn not_placeable() -> Self {
        Self {
            ingredients: Vec::new(),
            slots_to_ingredient_index: Vec::new(),
        }
    }

    /// `PlacementInfo.create(Ingredient)` (`PlacementInfo.java:20-22`).
    fn create_single(ingredient: &'a Ingredient, tags: &ItemTagIndex) -> Self {
        if ingredient.is_empty(tags) {
            return Self::not_placeable();
        }
        Self {
            ingredients: vec![ingredient],
            slots_to_ingredient_index: vec![0],
        }
    }

    /// `PlacementInfo.create(List)` (`PlacementInfo.java:47-61`).
    fn create(ingredients: Vec<&'a Ingredient>, tags: &ItemTagIndex) -> Self {
        let mut slots_to_ingredient_index = Vec::with_capacity(ingredients.len());
        for (index, ingredient) in ingredients.iter().enumerate() {
            if ingredient.is_empty(tags) {
                return Self::not_placeable();
            }
            slots_to_ingredient_index
                .push(i32::try_from(index).expect("placement ingredient index fits i32"));
        }
        Self {
            ingredients,
            slots_to_ingredient_index,
        }
    }

    /// `PlacementInfo.createFromOptionals` (`PlacementInfo.java:24-45`).
    fn create_from_optionals(
        slots: impl IntoIterator<Item = Option<&'a Ingredient>>,
        tags: &ItemTagIndex,
    ) -> Self {
        let mut ingredients = Vec::new();
        let mut slots_to_ingredient_index = Vec::new();
        let mut placement_index = 0;
        for slot in slots {
            match slot {
                Some(ingredient) => {
                    if ingredient.is_empty(tags) {
                        return Self::not_placeable();
                    }
                    ingredients.push(ingredient);
                    slots_to_ingredient_index.push(placement_index);
                    placement_index += 1;
                }
                None => slots_to_ingredient_index.push(EMPTY_SLOT),
            }
        }
        Self {
            ingredients,
            slots_to_ingredient_index,
        }
    }

    /// `PlacementInfo.ingredients` (`PlacementInfo.java:67-69`).
    pub fn ingredients(&self) -> &[&'a Ingredient] {
        &self.ingredients
    }

    /// `PlacementInfo.slotsToIngredientIndex` (`PlacementInfo.java:63-65`).
    pub fn slots_to_ingredient_index(&self) -> &[i32] {
        &self.slots_to_ingredient_index
    }

    /// `PlacementInfo.isImpossibleToPlace` (`PlacementInfo.java:71-73`).
    pub fn is_impossible_to_place(&self) -> bool {
        self.slots_to_ingredient_index.is_empty()
    }
}

/// Why a chosen recipe has no grid placement, counted rather than laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlacementRefusal {
    /// `CustomRecipe.placementInfo` is `NOT_PLACEABLE` (`CustomRecipe.java:25-27`),
    /// which `handlePlaceRecipe` drops before any menu sees it
    /// (`ServerGamePacketListenerImpl.java:1966-1969`).
    Special,
    /// Only `AbstractCraftingMenu` (`AbstractCraftingMenu.java:38-62`) and
    /// `AbstractFurnaceMenu` (`AbstractFurnaceMenu.java:173-193`) reach
    /// `ServerPlaceRecipe`, so no menu ever places a smithing or stonecutting
    /// recipe.
    NoPlacingMenu,
    /// `PlacementInfo.isImpossibleToPlace` (`PlacementInfo.java:71-73`): an
    /// ingredient named a tag that resolved to no item.
    ImpossibleToPlace,
}

impl PlacementRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Special => "a special recipe's placement info is NOT_PLACEABLE",
            Self::NoPlacingMenu => "no recipe-book menu places this recipe type",
            Self::ImpossibleToPlace => "an ingredient resolved to no item",
        }
    }
}

/// One filled menu grid: the ingredient index each grid slot takes, `EMPTY_SLOT`
/// where the walk placed nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct GridPlacement<'a> {
    ingredients: Vec<&'a Ingredient>,
    slots: Vec<i32>,
}

impl<'a> GridPlacement<'a> {
    /// The ingredients the placed indices name, in `PlacementInfo` order.
    pub fn ingredients(&self) -> &[&'a Ingredient] {
        &self.ingredients
    }

    /// One ingredient index per menu grid slot, in `getInputGridSlots` order.
    pub fn slots(&self) -> &[i32] {
        &self.slots
    }

    /// The ingredient `ServerPlaceRecipe` moves into one grid slot.
    pub fn ingredient_at(&self, grid_slot: usize) -> Option<&'a Ingredient> {
        let index = usize::try_from(*self.slots.get(grid_slot)?).ok()?;
        self.ingredients.get(index).copied()
    }
}

impl Recipe {
    /// `Recipe.placementInfo` (`Recipe.java:40`), per kind.
    pub fn placement_info(&self, tags: &ItemTagIndex) -> PlacementInfo<'_> {
        match &self.kind {
            // `ShapedRecipe.createPlacementInfo` (`ShapedRecipe.java:62-64`).
            RecipeKind::Shaped(recipe) => PlacementInfo::create_from_optionals(
                recipe.pattern.ingredients.iter().map(Option::as_ref),
                tags,
            ),
            // `ShapelessRecipe.createPlacementInfo` (`ShapelessRecipe.java:56-58`).
            RecipeKind::Shapeless(recipe) => {
                PlacementInfo::create(recipe.ingredients.iter().collect(), tags)
            }
            // `TransmuteRecipe.createPlacementInfo` (`TransmuteRecipe.java:208-214`).
            RecipeKind::Transmute(recipe) => PlacementInfo::create(
                std::iter::once(&recipe.input)
                    .chain(std::iter::repeat_n(
                        &recipe.material,
                        usize::try_from(recipe.material_max).unwrap_or_default(),
                    ))
                    .collect(),
                tags,
            ),
            // `DyeRecipe.createPlacementInfo` (`DyeRecipe.java:133-135`).
            RecipeKind::Dye(recipe) => {
                PlacementInfo::create(vec![&recipe.target, &recipe.dye], tags)
            }
            // `ImbueRecipe.createPlacementInfo` (`ImbueRecipe.java:94-98`).
            RecipeKind::Imbue(recipe) => PlacementInfo::create(
                vec![
                    &recipe.material,
                    &recipe.material,
                    &recipe.material,
                    &recipe.material,
                    &recipe.source,
                    &recipe.material,
                    &recipe.material,
                    &recipe.material,
                    &recipe.material,
                ],
                tags,
            ),
            // `SingleItemRecipe.placementInfo` (`SingleItemRecipe.java:48-54`).
            RecipeKind::Cooking(recipe) => PlacementInfo::create_single(&recipe.ingredient, tags),
            RecipeKind::Stonecutting(recipe) => {
                PlacementInfo::create_single(&recipe.ingredient, tags)
            }
            // `SmithingTransformRecipe.createPlacementInfo`
            // (`SmithingTransformRecipe.java:85-87`).
            RecipeKind::SmithingTransform(recipe) => PlacementInfo::create_from_optionals(
                [
                    recipe.template.as_ref(),
                    Some(&recipe.base),
                    recipe.addition.as_ref(),
                ],
                tags,
            ),
            // `SmithingTrimRecipe.createPlacementInfo` (`SmithingTrimRecipe.java:103-105`).
            RecipeKind::SmithingTrim(recipe) => {
                PlacementInfo::create(vec![&recipe.template, &recipe.base, &recipe.addition], tags)
            }
            // `CustomRecipe.placementInfo` (`CustomRecipe.java:25-27`).
            _ => PlacementInfo::not_placeable(),
        }
    }

    /// Which ingredient every slot of a `gridWidth` x `gridHeight` menu grid
    /// takes, the way `ServerPlaceRecipe.placeRecipe`
    /// (`ServerPlaceRecipe.java:114-133`) fills one.
    pub fn grid_placement(
        &self,
        grid_width: i32,
        grid_height: i32,
        tags: &ItemTagIndex,
    ) -> Result<GridPlacement<'_>, PlacementRefusal> {
        if self.is_special() {
            return Err(PlacementRefusal::Special);
        }
        if matches!(
            self.kind,
            RecipeKind::SmithingTransform(_)
                | RecipeKind::SmithingTrim(_)
                | RecipeKind::Stonecutting(_)
        ) {
            return Err(PlacementRefusal::NoPlacingMenu);
        }
        let info = self.placement_info(tags);
        if info.is_impossible_to_place() {
            return Err(PlacementRefusal::ImpossibleToPlace);
        }
        // `PlaceRecipeHelper.placeRecipe` (`PlaceRecipeHelper.java:9-17`): a
        // shaped recipe carries its own extent into the grid, every other kind
        // takes the grid's own.
        let (recipe_width, recipe_height) = match &self.kind {
            RecipeKind::Shaped(recipe) => (recipe.pattern.width, recipe.pattern.height),
            _ => (grid_width, grid_height),
        };
        let cells = usize::try_from(grid_width.max(0) * grid_height.max(0)).unwrap_or_default();
        let mut slots = vec![EMPTY_SLOT; cells];
        place_recipe(
            grid_width,
            grid_height,
            recipe_width,
            recipe_height,
            info.slots_to_ingredient_index(),
            |ingredient_index, grid_index, _grid_x, _grid_y| {
                if let Some(slot) = usize::try_from(grid_index)
                    .ok()
                    .and_then(|index| slots.get_mut(index))
                {
                    *slot = *ingredient_index;
                }
            },
        );
        Ok(GridPlacement {
            ingredients: info.ingredients,
            slots,
        })
    }
}

/// `PlaceRecipeHelper.placeRecipe` (`PlaceRecipeHelper.java:19-62`).
pub fn place_recipe<T>(
    grid_width: i32,
    grid_height: i32,
    recipe_width: i32,
    recipe_height: i32,
    entries: &[T],
    mut output: impl FnMut(&T, i32, i32, i32),
) {
    let mut entries = entries.iter().peekable();
    let mut grid_index = 0;
    let mut grid_y = 0;
    while grid_y < grid_height {
        let center_rows = (recipe_height as f32) < grid_height as f32 / 2.0;
        let start_row = (grid_height as f32 / 2.0 - recipe_height as f32 / 2.0).floor() as i32;
        if center_rows && start_row > grid_y {
            grid_index += grid_width;
            grid_y += 1;
        }
        let mut grid_x = 0;
        while grid_x < grid_width {
            if entries.peek().is_none() {
                return;
            }
            let center_columns = (recipe_width as f32) < grid_width as f32 / 2.0;
            let start_column = (grid_width as f32 / 2.0 - recipe_width as f32 / 2.0).floor() as i32;
            let mut total_recipe_width_in_grid = recipe_width;
            let mut add_ingredient_to_slot = grid_x < recipe_width;
            if center_columns {
                total_recipe_width_in_grid = start_column + recipe_width;
                add_ingredient_to_slot =
                    start_column <= grid_x && grid_x < start_column + recipe_width;
            }
            if add_ingredient_to_slot {
                let entry = entries.next().expect("entry presence checked");
                output(entry, grid_index, grid_x, grid_y);
            } else if total_recipe_width_in_grid == grid_x {
                grid_index += grid_width - grid_x;
                break;
            }
            grid_index += 1;
            grid_x += 1;
        }
        grid_y += 1;
    }
}
