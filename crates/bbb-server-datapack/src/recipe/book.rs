//! What a session's recipe book holds, on the library side
//! (`ServerRecipeBook.java:61-121`): which loaded recipes a book may take, the
//! display entries an unlock or a restore carries, and the flags each entry
//! gets. The known and highlight sets, their packets and their menus stay
//! outside.

use bbb_protocol::packets::{RecipeBookAddEntry, RecipeDisplayEntry};

use super::{RecipeDisplayCatalog, RecipeElement, RecipeSet};

/// Why the library puts no entry in a book for a named recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookRefusalReason {
    /// The validator `ServerPlayer` hands `loadUntrusted` (`ServerPlayer.java:391`):
    /// the loaded recipes answer no such id.
    NoSuchRecipe,
    /// `addRecipes` walks past a special recipe (`ServerRecipeBook.java:66`),
    /// which is also the set `Recipe.display` (`Recipe.java:42-44`) leaves
    /// empty, since `CustomRecipe` never overrides it.
    Special,
}

impl BookRefusalReason {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NoSuchRecipe => "the loaded recipes answer no such id",
            Self::Special => "a special recipe never enters a book",
        }
    }
}

/// One id a book add walked past, held with its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookRefusal {
    pub recipe_id: String,
    pub reason: BookRefusalReason,
}

/// The entries one `ClientboundRecipeBookAddPacket` (`ClientboundRecipeBookAddPacket.java:11-18`)
/// carries, with the ids the library refused counted beside them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BookAdd {
    pub entries: Vec<RecipeBookAddEntry>,
    pub refusals: Vec<BookRefusal>,
}

impl BookAdd {
    fn refuse(&mut self, recipe_id: &str, reason: BookRefusalReason) {
        self.refusals.push(BookRefusal {
            recipe_id: recipe_id.to_string(),
            reason,
        });
    }

    /// `ClientboundRecipeBookAddPacket.Entry` (`ClientboundRecipeBookAddPacket.java:29-50`)
    /// per published display.
    fn add(&mut self, displays: &[RecipeDisplayEntry], notification: bool, highlight: bool) {
        self.entries.extend(
            displays
                .iter()
                .map(|display| RecipeBookAddEntry::new(display.clone(), notification, highlight)),
        );
    }
}

/// The recipe library a book reads: the loaded recipes and the displays
/// `finalizeRecipeLoading` published for them (`RecipeManager.java:118-119`).
#[derive(Debug, Clone, Copy)]
pub struct RecipeBookLibrary<'a> {
    recipes: &'a RecipeSet,
    displays: &'a RecipeDisplayCatalog,
}

impl<'a> RecipeBookLibrary<'a> {
    pub fn new(recipes: &'a RecipeSet, displays: &'a RecipeDisplayCatalog) -> Self {
        Self { recipes, displays }
    }

    /// The gate `addRecipes` applies (`ServerRecipeBook.java:64-72`) beside the
    /// per-player `known` set, which stays with the caller.
    pub fn may_hold(&self, recipe_id: &str) -> Result<(), BookRefusalReason> {
        self.holder(recipe_id).map(|_| ())
    }

    fn holder(&self, recipe_id: &str) -> Result<&'a RecipeElement, BookRefusalReason> {
        match self.recipes.recipe(recipe_id) {
            None => Err(BookRefusalReason::NoSuchRecipe),
            Some(element) if element.recipe.is_special() => Err(BookRefusalReason::Special),
            Some(element) => Ok(element),
        }
    }

    /// Every loaded recipe a book may hold, in the `Identifier` order
    /// `RecipeMap.create` (`RecipeMap.java:24-34`) received.
    pub fn holdable_recipe_ids(&self) -> impl Iterator<Item = &'a str> {
        self.recipes
            .recipes
            .iter()
            .filter(|element| !element.recipe.is_special())
            .map(|element| element.id.as_str())
    }

    /// The `RecipeBookCategory` every display of this recipe carries
    /// (`RecipeManager.java:240` over `Recipe.recipeBookCategory`,
    /// `Recipe.java:46`).
    pub fn book_category_id(&self, recipe_id: &str) -> Option<i32> {
        self.recipes
            .recipe(recipe_id)
            .map(|element| element.recipe.category_id())
    }

    /// `ServerRecipeBook.addRecipes` (`ServerRecipeBook.java:61-80`): every
    /// display of a recipe the book did not already know, notified as the
    /// recipe itself declares and always highlighted.
    pub fn unlocked<I>(&self, recipe_ids: I) -> BookAdd
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut add = BookAdd::default();
        for recipe_id in recipe_ids {
            let recipe_id = recipe_id.as_ref();
            match self.holder(recipe_id) {
                Err(reason) => add.refuse(recipe_id, reason),
                Ok(element) => add.add(
                    self.displays.displays_for_recipe(recipe_id),
                    element.recipe.show_notification(),
                    true,
                ),
            }
        }
        add
    }

    /// `ServerRecipeBook.sendInitialRecipeBook` (`ServerRecipeBook.java:112-121`):
    /// every display a restored book already holds, never a notification and
    /// highlighted only where its own highlight set says.
    pub fn restored<I, S>(&self, known: I) -> BookAdd
    where
        I: IntoIterator<Item = (S, bool)>,
        S: AsRef<str>,
    {
        let mut add = BookAdd::default();
        for (recipe_id, highlight) in known {
            let recipe_id = recipe_id.as_ref();
            match self.may_hold(recipe_id) {
                Err(reason) => add.refuse(recipe_id, reason),
                Ok(()) => add.add(
                    self.displays.displays_for_recipe(recipe_id),
                    false,
                    highlight,
                ),
            }
        }
        add
    }
}
