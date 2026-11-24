use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BLAST_FURNACE_ID, VANILLA_ITEM_CAMPFIRE_ID, VANILLA_ITEM_CRAFTING_TABLE_ID,
    VANILLA_ITEM_FURNACE_ID, VANILLA_ITEM_SMITHING_TABLE_ID, VANILLA_ITEM_SMOKER_ID,
    VANILLA_ITEM_STONECUTTER_ID,
};
use bbb_protocol::packets::{
    IngredientSummary, ItemStackSummary, RecipeDisplayEntry, RecipeDisplayId, RecipeDisplaySummary,
    RecipePropertySetSummary, SlotDisplaySummary, SlotDisplayTrimPatternSummary, SlotDisplayValue,
    StonecutterSelectableRecipeSummary, UpdateRecipes,
};
use bbb_protocol::JavaString;

use crate::dynamic_registry::DatapackGeneration;

use super::ingredient::{crafting_remainder, item_stack, IngredientTerm};
use super::{
    CookingKind, Ingredient, ItemTagIndex, Recipe, RecipeKind, RecipeLoadWarning, RecipeSet,
};

const PROPERTY_SET_KEYS: [&str; 7] = [
    "minecraft:smithing_base",
    "minecraft:smithing_template",
    "minecraft:smithing_addition",
    "minecraft:furnace_input",
    "minecraft:blast_furnace_input",
    "minecraft:smoker_input",
    "minecraft:campfire_input",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeDisplayCatalog {
    all_displays: Vec<RecipeDisplayEntry>,
    parent_recipe_ids: Vec<String>,
    recipe_ranges: BTreeMap<String, Range<usize>>,
    update_recipes: UpdateRecipes,
}

impl RecipeDisplayCatalog {
    pub fn all_displays(&self) -> &[RecipeDisplayEntry] {
        &self.all_displays
    }

    pub fn displays_for_recipe(&self, recipe_id: &str) -> &[RecipeDisplayEntry] {
        self.recipe_ranges
            .get(recipe_id)
            .map(|range| &self.all_displays[range.clone()])
            .unwrap_or_default()
    }

    pub fn parent_recipe_id(&self, display_id: RecipeDisplayId) -> Option<&str> {
        usize::try_from(display_id.index)
            .ok()
            .and_then(|index| self.parent_recipe_ids.get(index))
            .map(String::as_str)
    }

    /// `RecipeManager.getRecipeFromDisplay` (`RecipeManager.java:179-182`): the
    /// published display an id names, with the recipe it was unpacked from.
    pub fn recipe_from_display(
        &self,
        display_id: RecipeDisplayId,
    ) -> Option<(&str, &RecipeDisplayEntry)> {
        let index = usize::try_from(display_id.index).ok()?;
        Some((
            self.parent_recipe_ids.get(index)?.as_str(),
            self.all_displays.get(index)?,
        ))
    }

    pub fn update_recipes(&self) -> &UpdateRecipes {
        &self.update_recipes
    }
}

impl RecipeSet {
    /// `RecipeManager.finalizeRecipeLoading` (`RecipeManager.java:88-120`) and
    /// `RecipeManager.unpackRecipeInfo` (`RecipeManager.java:216-246`).
    pub fn finalize_client_catalog(
        &mut self,
        item_tags: &ItemTagIndex,
        registries: &DatapackGeneration,
    ) -> RecipeDisplayCatalog {
        self.finalize_client_catalog_with_content_registry(
            item_tags,
            registries,
            ContentRegistry::vanilla(),
        )
    }

    /// Content-registry-aware form of [`Self::finalize_client_catalog`].
    pub fn finalize_client_catalog_with_content_registry(
        &mut self,
        item_tags: &ItemTagIndex,
        registries: &DatapackGeneration,
        content_registry: &ContentRegistry,
    ) -> RecipeDisplayCatalog {
        let mut compiled = Vec::with_capacity(self.recipes.len());
        for element in std::mem::take(&mut self.recipes) {
            if !element.recipe.is_special()
                && element
                    .recipe
                    .placement_info(item_tags)
                    .is_impossible_to_place()
            {
                self.warnings.push(RecipeLoadWarning {
                    element_id: element.id,
                    pack: element.source_pack,
                    message: "recipe can't be placed due to empty ingredients and was ignored"
                        .to_string(),
                });
                continue;
            }
            match compile_displays(&element.recipe, registries, content_registry) {
                Ok(displays) => compiled.push((element, displays)),
                Err(message) => self.warnings.push(RecipeLoadWarning {
                    element_id: element.id,
                    pack: element.source_pack,
                    message: format!("couldn't compile recipe displays: {message}"),
                }),
            }
        }
        self.recipes = compiled
            .iter()
            .map(|(element, _)| element.clone())
            .collect();

        let mut all_displays = Vec::new();
        let mut parent_recipe_ids = Vec::new();
        let mut recipe_ranges = BTreeMap::new();
        let mut group_ids = BTreeMap::<JavaString, i32>::new();
        for (element, displays) in &compiled {
            let start = all_displays.len();
            let group = if element.recipe.group().is_empty() {
                None
            } else if let Some(group) = group_ids.get(element.recipe.group()) {
                Some(*group)
            } else {
                let next = i32::try_from(group_ids.len()).expect("recipe group count fits i32");
                group_ids.insert(element.recipe.group().clone(), next);
                Some(next)
            };
            let crafting_requirements = placement_ingredients(&element.recipe, item_tags)
                .map(|ingredients| ingredients.into_iter().map(ingredient_summary).collect());
            for display in displays {
                let index =
                    i32::try_from(all_displays.len()).expect("recipe display count fits i32");
                all_displays.push(RecipeDisplayEntry {
                    id: RecipeDisplayId { index },
                    display: display.clone(),
                    group,
                    category_id: element.recipe.category_id(),
                    crafting_requirements: crafting_requirements.clone(),
                });
                parent_recipe_ids.push(element.id.clone());
            }
            recipe_ranges.insert(element.id.clone(), start..all_displays.len());
        }

        RecipeDisplayCatalog {
            all_displays,
            parent_recipe_ids,
            recipe_ranges,
            update_recipes: compile_update_recipes(&self.recipes, item_tags, content_registry),
        }
    }
}

fn compile_displays(
    recipe: &Recipe,
    registries: &DatapackGeneration,
    content_registry: &ContentRegistry,
) -> Result<Vec<RecipeDisplaySummary>, String> {
    let crafting_table = item_display(VANILLA_ITEM_CRAFTING_TABLE_ID, content_registry)?;
    let displays = match &recipe.kind {
        RecipeKind::Shaped(shaped) => vec![RecipeDisplaySummary::crafting_shaped(
            shaped.pattern.width,
            shaped.pattern.height,
            shaped
                .pattern
                .ingredients
                .iter()
                .map(|ingredient| match ingredient {
                    Some(ingredient) => ingredient_display(ingredient, content_registry),
                    None => slot_display(SlotDisplayValue::Empty, content_registry),
                })
                .collect::<Result<Vec<_>, _>>()?,
            stack_display(shaped.result.create(), content_registry)?,
            crafting_table,
        )
        .map_err(|error| error.to_string())?],
        RecipeKind::Shapeless(shapeless) => vec![RecipeDisplaySummary::crafting_shapeless(
            shapeless
                .ingredients
                .iter()
                .map(|ingredient| ingredient_display(ingredient, content_registry))
                .collect::<Result<Vec<_>, _>>()?,
            stack_display(shapeless.result.create(), content_registry)?,
            crafting_table,
        )
        .map_err(|error| error.to_string())?],
        RecipeKind::Transmute(transmute) => {
            let input = ingredient_display(&transmute.input, content_registry)?;
            let material = ingredient_display(&transmute.material, content_registry)?;
            let mut ingredients = vec![input];
            let mut displays = Vec::new();
            for material_count in transmute.material_min..=transmute.material_max {
                ingredients.push(material.clone());
                let result_count = if transmute.add_material_count_to_result {
                    transmute
                        .result
                        .count
                        .checked_add(material_count)
                        .ok_or_else(|| "transmute result count overflow".to_string())?
                } else {
                    transmute.result.count
                };
                displays.push(
                    RecipeDisplaySummary::crafting_shapeless(
                        ingredients.clone(),
                        stack_display(
                            item_stack(transmute.result.item_id, result_count),
                            content_registry,
                        )?,
                        crafting_table.clone(),
                    )
                    .map_err(|error| error.to_string())?,
                );
            }
            displays
        }
        RecipeKind::Dye(dye) => {
            let dye_display = slot_display(
                SlotDisplayValue::OnlyWithComponent {
                    source: Box::new(ingredient_value(&dye.dye)?),
                    component_type_id: VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                },
                content_registry,
            )?;
            let target_display = ingredient_display(&dye.target, content_registry)?;
            let result = slot_display(
                SlotDisplayValue::Dyed {
                    dye: Box::new(
                        dye_display
                            .decode_value_with_content_registry(content_registry)
                            .map_err(|error| error.to_string())?,
                    ),
                    target: Box::new(
                        target_display
                            .decode_value_with_content_registry(content_registry)
                            .map_err(|error| error.to_string())?,
                    ),
                },
                content_registry,
            )?;
            vec![RecipeDisplaySummary::crafting_shapeless(
                vec![target_display, dye_display],
                result,
                crafting_table,
            )
            .map_err(|error| error.to_string())?]
        }
        RecipeKind::Imbue(imbue) => {
            let material = ingredient_display(&imbue.material, content_registry)?;
            let source = slot_display(
                SlotDisplayValue::WithAnyPotion {
                    display: Box::new(ingredient_value(&imbue.source)?),
                },
                content_registry,
            )?;
            let mut ingredients = vec![material.clone(); 9];
            ingredients[4] = source;
            let result = slot_display(
                SlotDisplayValue::WithAnyPotion {
                    display: Box::new(SlotDisplayValue::ItemStack {
                        stack: Box::new(imbue.result.create()),
                    }),
                },
                content_registry,
            )?;
            vec![
                RecipeDisplaySummary::crafting_shaped(3, 3, ingredients, result, crafting_table)
                    .map_err(|error| error.to_string())?,
            ]
        }
        RecipeKind::Cooking(cooking) => {
            let (station, _) = cooking_station_and_property(cooking.kind);
            vec![RecipeDisplaySummary::furnace(
                ingredient_display(&cooking.ingredient, content_registry)?,
                slot_display(SlotDisplayValue::AnyFuel, content_registry)?,
                stack_display(cooking.result.create(), content_registry)?,
                item_display(station, content_registry)?,
                cooking.cooking_time,
                cooking.experience.to_bits(),
            )
            .map_err(|error| error.to_string())?]
        }
        RecipeKind::Stonecutting(stonecutter) => vec![RecipeDisplaySummary::stonecutter(
            ingredient_display(&stonecutter.ingredient, content_registry)?,
            stack_display(stonecutter.result.create(), content_registry)?,
            item_display(VANILLA_ITEM_STONECUTTER_ID, content_registry)?,
        )
        .map_err(|error| error.to_string())?],
        RecipeKind::SmithingTransform(smithing) => vec![RecipeDisplaySummary::smithing(
            optional_ingredient_display(smithing.template.as_ref(), content_registry)?,
            ingredient_display(&smithing.base, content_registry)?,
            optional_ingredient_display(smithing.addition.as_ref(), content_registry)?,
            stack_display(smithing.result.create(), content_registry)?,
            item_display(VANILLA_ITEM_SMITHING_TABLE_ID, content_registry)?,
        )
        .map_err(|error| error.to_string())?],
        RecipeKind::SmithingTrim(smithing) => {
            let pattern_index = registries
                .registry("minecraft:trim_pattern")
                .and_then(|registry| {
                    registry
                        .elements
                        .iter()
                        .position(|element| element.id == smithing.pattern)
                })
                .and_then(|index| i32::try_from(index).ok())
                .ok_or_else(|| format!("missing trim pattern {}", smithing.pattern))?;
            let template = ingredient_display(&smithing.template, content_registry)?;
            let base = ingredient_display(&smithing.base, content_registry)?;
            let addition = ingredient_display(&smithing.addition, content_registry)?;
            let result = slot_display(
                SlotDisplayValue::SmithingTrim {
                    base: Box::new(
                        base.decode_value_with_content_registry(content_registry)
                            .map_err(|error| error.to_string())?,
                    ),
                    material: Box::new(
                        addition
                            .decode_value_with_content_registry(content_registry)
                            .map_err(|error| error.to_string())?,
                    ),
                    pattern: SlotDisplayTrimPatternSummary {
                        registry_id: Some(pattern_index),
                        direct: None,
                    },
                },
                content_registry,
            )?;
            vec![RecipeDisplaySummary::smithing(
                template,
                base,
                addition,
                result,
                item_display(VANILLA_ITEM_SMITHING_TABLE_ID, content_registry)?,
            )
            .map_err(|error| error.to_string())?]
        }
        _ => Vec::new(),
    };
    Ok(displays)
}

fn slot_display(
    value: SlotDisplayValue,
    content_registry: &ContentRegistry,
) -> Result<SlotDisplaySummary, String> {
    SlotDisplaySummary::from_value_with_content_registry(value, content_registry)
        .map_err(|error| error.to_string())
}

fn item_display(
    item_id: i32,
    content_registry: &ContentRegistry,
) -> Result<SlotDisplaySummary, String> {
    slot_display(
        SlotDisplayValue::Item {
            stack: Box::new(item_stack(item_id, 1)),
        },
        content_registry,
    )
}

fn stack_display(
    stack: ItemStackSummary,
    content_registry: &ContentRegistry,
) -> Result<SlotDisplaySummary, String> {
    slot_display(
        SlotDisplayValue::ItemStack {
            stack: Box::new(stack),
        },
        content_registry,
    )
}

fn optional_ingredient_display(
    ingredient: Option<&Ingredient>,
    content_registry: &ContentRegistry,
) -> Result<SlotDisplaySummary, String> {
    ingredient.map_or_else(
        || slot_display(SlotDisplayValue::Empty, content_registry),
        |ingredient| ingredient_display(ingredient, content_registry),
    )
}

fn ingredient_display(
    ingredient: &Ingredient,
    content_registry: &ContentRegistry,
) -> Result<SlotDisplaySummary, String> {
    slot_display(ingredient_value(ingredient)?, content_registry)
}

fn ingredient_value(ingredient: &Ingredient) -> Result<SlotDisplayValue, String> {
    match ingredient.terms.as_slice() {
        [IngredientTerm::Tag(tag)] => Ok(SlotDisplayValue::Tag { tag: tag.clone() }),
        terms
            if terms
                .iter()
                .all(|term| matches!(term, IngredientTerm::Item(_))) =>
        {
            Ok(SlotDisplayValue::Composite {
                contents: terms
                    .iter()
                    .map(|term| {
                        let IngredientTerm::Item(item_id) = term else {
                            unreachable!()
                        };
                        let item = SlotDisplayValue::Item {
                            stack: Box::new(item_stack(*item_id, 1)),
                        };
                        match crafting_remainder(*item_id) {
                            Some(remainder) => SlotDisplayValue::WithRemainder {
                                input: Box::new(item),
                                remainder: Box::new(SlotDisplayValue::ItemStack {
                                    stack: Box::new(remainder),
                                }),
                            },
                            None => item,
                        }
                    })
                    .collect(),
            })
        }
        _ => Err("ingredient holder set is not a tag or direct item list".to_string()),
    }
}

fn ingredient_summary(ingredient: &Ingredient) -> IngredientSummary {
    match ingredient.terms.as_slice() {
        [IngredientTerm::Tag(tag)] => IngredientSummary {
            tag: Some(tag.clone()),
            item_ids: Vec::new(),
        },
        terms => IngredientSummary {
            tag: None,
            item_ids: terms
                .iter()
                .filter_map(|term| match term {
                    IngredientTerm::Item(item_id) => Some(*item_id),
                    IngredientTerm::Tag(_) => None,
                })
                .collect(),
        },
    }
}

fn resolved_items(ingredient: &Ingredient, item_tags: &ItemTagIndex) -> BTreeSet<i32> {
    let mut items = BTreeSet::new();
    for term in &ingredient.terms {
        match term {
            IngredientTerm::Item(item_id) => {
                items.insert(*item_id);
            }
            IngredientTerm::Tag(tag) => {
                items.extend(item_tags.members(tag));
            }
        }
    }
    items
}

/// `RecipeManager.unpackRecipeInfo` (`RecipeManager.java:229-234`): a special
/// recipe carries no placement check, every other kind carries its ingredients.
fn placement_ingredients<'a>(
    recipe: &'a Recipe,
    item_tags: &ItemTagIndex,
) -> Option<Vec<&'a Ingredient>> {
    if recipe.is_special() {
        return None;
    }
    Some(recipe.placement_info(item_tags).ingredients().to_vec())
}

fn compile_update_recipes(
    recipes: &[super::RecipeElement],
    item_tags: &ItemTagIndex,
    content_registry: &ContentRegistry,
) -> UpdateRecipes {
    let mut property_items = PROPERTY_SET_KEYS.map(|_| BTreeSet::<i32>::new());
    let mut stonecutter_recipes = Vec::new();
    for element in recipes {
        match &element.recipe.kind {
            RecipeKind::Cooking(recipe) => {
                let (_, property) = cooking_station_and_property(recipe.kind);
                property_items[property].extend(resolved_items(&recipe.ingredient, item_tags));
            }
            RecipeKind::SmithingTransform(recipe) => {
                property_items[0].extend(resolved_items(&recipe.base, item_tags));
                if let Some(template) = &recipe.template {
                    property_items[1].extend(resolved_items(template, item_tags));
                }
                if let Some(addition) = &recipe.addition {
                    property_items[2].extend(resolved_items(addition, item_tags));
                }
            }
            RecipeKind::SmithingTrim(recipe) => {
                property_items[0].extend(resolved_items(&recipe.base, item_tags));
                property_items[1].extend(resolved_items(&recipe.template, item_tags));
                property_items[2].extend(resolved_items(&recipe.addition, item_tags));
            }
            RecipeKind::Stonecutting(recipe) => {
                if let (Ok(input), Ok(option_display)) = (
                    Ok::<_, String>(ingredient_summary(&recipe.ingredient)),
                    stack_display(recipe.result.create(), content_registry),
                ) {
                    stonecutter_recipes.push(StonecutterSelectableRecipeSummary {
                        input,
                        option_display,
                    });
                }
            }
            _ => {}
        }
    }
    let property_sets = PROPERTY_SET_KEYS
        .into_iter()
        .zip(property_items)
        .map(|(key, items)| RecipePropertySetSummary {
            key: key.to_string(),
            item_ids: items.into_iter().collect(),
        })
        .collect();
    UpdateRecipes {
        property_sets,
        stonecutter_recipes,
    }
}

fn cooking_station_and_property(kind: CookingKind) -> (i32, usize) {
    match kind {
        CookingKind::Smelting => (VANILLA_ITEM_FURNACE_ID, 3),
        CookingKind::Blasting => (VANILLA_ITEM_BLAST_FURNACE_ID, 4),
        CookingKind::Smoking => (VANILLA_ITEM_SMOKER_ID, 5),
        CookingKind::Campfire => (VANILLA_ITEM_CAMPFIRE_ID, 6),
    }
}
