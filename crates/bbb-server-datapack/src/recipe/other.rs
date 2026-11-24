//! Cooking, smithing and stonecutting bodies (typed, not crafting-grid matchers).

use super::ingredient::{Ingredient, ItemStackTemplate};
use super::RecipeDecodeError;
use bbb_registry_codec::{
    registry_codec::RegistryReferenceLookup, PersistentItemStackHolderResolver,
    RegistryElementValueRef,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookingKind {
    Smelting,
    Blasting,
    Smoking,
    Campfire,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CookingRecipe {
    pub kind: CookingKind,
    pub ingredient: Ingredient,
    pub result: ItemStackTemplate,
    pub experience: f32,
    pub cooking_time: i32,
}

impl CookingRecipe {
    /// `AbstractCookingRecipe.cookingMapCodec` (`AbstractCookingRecipe.java:74-86`):
    /// `experience` falls back to 0 and `cookingtime` to the default its own
    /// serializer passes.
    pub(super) fn parse_source(
        root: RegistryElementValueRef<'_>,
        kind: CookingKind,
        lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
    ) -> Result<Self, RecipeDecodeError> {
        use super::source::{optional_f32, optional_i32, required, template};
        Ok(Self {
            kind,
            ingredient: Ingredient::parse_source_with_content_registry(
                required(root, "ingredient")?,
                lookup.content_registry(),
            )?,
            result: template(required(root, "result")?, lookup)?,
            experience: optional_f32(root, "experience", 0.0)?,
            cooking_time: optional_i32(root, "cookingtime", kind.default_cooking_time())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmithingTransformRecipe {
    pub template: Option<Ingredient>,
    pub base: Ingredient,
    pub addition: Option<Ingredient>,
    pub result: ItemStackTemplate,
}

impl SmithingTransformRecipe {
    pub(super) fn parse_source(
        root: RegistryElementValueRef<'_>,
        lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
    ) -> Result<Self, RecipeDecodeError> {
        use super::source::{required, template};
        let content_registry = lookup.content_registry();
        Ok(Self {
            template: root
                .field("template")
                .map(|value| {
                    Ingredient::parse_source_with_content_registry(value, content_registry)
                })
                .transpose()?,
            base: Ingredient::parse_source_with_content_registry(
                required(root, "base")?,
                content_registry,
            )?,
            addition: root
                .field("addition")
                .map(|value| {
                    Ingredient::parse_source_with_content_registry(value, content_registry)
                })
                .transpose()?,
            result: template(required(root, "result")?, lookup)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmithingTrimRecipe {
    pub template: Ingredient,
    pub base: Ingredient,
    pub addition: Ingredient,
    pub pattern: String,
}

impl SmithingTrimRecipe {
    pub(super) fn parse_source(
        root: RegistryElementValueRef<'_>,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Result<Self, RecipeDecodeError> {
        use super::source::{required, resource};
        Ok(Self {
            template: Ingredient::parse_source_with_content_registry(
                required(root, "template")?,
                content_registry,
            )?,
            base: Ingredient::parse_source_with_content_registry(
                required(root, "base")?,
                content_registry,
            )?,
            addition: Ingredient::parse_source_with_content_registry(
                required(root, "addition")?,
                content_registry,
            )?,
            pattern: resource(required(root, "pattern")?, "pattern")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StonecuttingRecipe {
    pub ingredient: Ingredient,
    pub result: ItemStackTemplate,
}

impl StonecuttingRecipe {
    pub(super) fn parse_source(
        root: RegistryElementValueRef<'_>,
        lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
    ) -> Result<Self, RecipeDecodeError> {
        use super::source::{required, template};
        Ok(Self {
            ingredient: Ingredient::parse_source_with_content_registry(
                required(root, "ingredient")?,
                lookup.content_registry(),
            )?,
            result: template(required(root, "result")?, lookup)?,
        })
    }
}
