//! Special crafting serializers (`RecipeSerializers.java:9-20`).

use bbb_protocol::builtin_enum_component_values::vanilla_dye_color_firework_color;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
};
use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
use bbb_protocol::packets::{
    DataComponentPatchSummary, FireworkExplosionSummary, ItemEnchantmentSummary, ItemStackSummary,
    WrittenBookContentSummary,
};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::item_stack_template::{codec_int, patch_declares, prototype_component_value};

use super::components::{
    apply_dyes, apply_template, banner_patterns, copy_component, dye_color_or_white, dyed_color,
    firework_explosion_or_default, set_component, set_component_to_null, single_component_patch,
    written_book_content, ComponentValue, DYE_COLOR_WHITE_ID, WRITTEN_BOOK_MAX_GENERATION,
};
use super::ingredient::{
    banner_item_color, crafting_remainder, is_banner_item, item_stack, stack_has_component,
    stack_is_empty, Ingredient, ItemStackTemplate, ItemTagIndex,
};
use super::input::CraftingInput;
use super::shaped::ShapedRecipePattern;

#[derive(Debug, Clone, PartialEq)]
pub struct TransmuteRecipe {
    pub input: Ingredient,
    pub material: Ingredient,
    pub material_min: i32,
    pub material_max: i32,
    pub result: ItemStackTemplate,
    pub add_material_count_to_result: bool,
}

impl TransmuteRecipe {
    /// `TransmuteRecipe.matches` (`TransmuteRecipe.java:97-139`).
    pub fn matches(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
        input: &CraftingInput,
        tags: &ItemTagIndex,
    ) -> bool {
        let Some((slot, material_count)) = self.matching_input(input, tags) else {
            return false;
        };
        if self.compute_result_size(material_count) != 1 {
            return true;
        }
        let found_input = input.get_item(slot);
        let result = self.compute_result(found_input, 0);
        !stack_is_empty(&result)
            && RuntimeItemStack::from_summary(found_input.clone())
                .same_item_same_effective_components(
                    &RuntimeItemStack::from_summary(result),
                    resolver,
                )
                .is_ok_and(|same| !same)
    }

    fn matching_input(&self, input: &CraftingInput, tags: &ItemTagIndex) -> Option<(i32, i32)> {
        let mut found_input = None;
        let mut material_count = 0;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.input.test(stack, tags) {
                if found_input.is_some() {
                    return None;
                }
                found_input = Some(slot);
            } else if self.material.test(stack, tags) {
                material_count += 1;
                if material_count > self.material_max {
                    return None;
                }
            } else {
                return None;
            }
        }
        let found_input = found_input?;
        if material_count < self.material_min || material_count > self.material_max {
            return None;
        }
        Some((found_input, material_count))
    }

    /// `TransmuteRecipe.computeResultSize` (`TransmuteRecipe.java:89-91`).
    fn compute_result_size(&self, material_count: i32) -> i32 {
        if self.add_material_count_to_result {
            material_count + self.result.count
        } else {
            self.result.count
        }
    }

    /// `TransmuteRecipe.computeResult` (`TransmuteRecipe.java:93-95`).
    fn compute_result(&self, input: &ItemStackSummary, material_count: i32) -> ItemStackSummary {
        create_with_original_components(&self.result, input, material_count)
    }

    /// `TransmuteRecipe.assemble` (`TransmuteRecipe.java:141-168`).
    pub fn assemble(&self, input: &CraftingInput, tags: &ItemTagIndex) -> ItemStackSummary {
        if self.add_material_count_to_result {
            let mut material_count = 0;
            let mut input_ingredient = ItemStackSummary::empty();
            for slot in 0..input.size() {
                let stack = input.get_item(slot);
                if stack_is_empty(stack) {
                    continue;
                }
                if self.input.test(stack, tags) {
                    input_ingredient = stack.clone();
                } else if self.material.test(stack, tags) {
                    material_count += 1;
                }
            }
            return self.compute_result(&input_ingredient, material_count);
        }
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if !stack_is_empty(stack) && self.input.test(stack, tags) {
                return self.compute_result(stack, 0);
            }
        }
        ItemStackSummary::empty()
    }
}

/// `TransmuteRecipe.createWithOriginalComponents` (`TransmuteRecipe.java:85-87`):
/// the target template applied over the input stack's own component patch.
pub fn create_with_original_components(
    target: &ItemStackTemplate,
    input: &ItemStackSummary,
    extra_count: i32,
) -> ItemStackSummary {
    target
        .apply(target.count + extra_count, &input.component_patch)
        .unwrap_or_else(|_| ItemStackSummary::empty())
}

#[derive(Debug, Clone, PartialEq)]
pub struct DyeRecipe {
    pub target: Ingredient,
    pub dye: Ingredient,
    pub result: ItemStackTemplate,
}

impl DyeRecipe {
    /// `DyeRecipe.matches` (`DyeRecipe.java:62-90`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.ingredient_count() < 2 {
            return false;
        }
        let mut has_target = false;
        let mut has_dyes = false;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.target.test(stack, tags) {
                if has_target {
                    return false;
                }
                has_target = true;
            } else if self.dye.test(stack, tags) && has_dye_component(stack) {
                has_dyes = true;
            } else {
                return false;
            }
        }
        has_target && has_dyes
    }

    /// `DyeRecipe.assemble` (`DyeRecipe.java:92-125`).
    pub fn assemble(&self, input: &CraftingInput, tags: &ItemTagIndex) -> ItemStackSummary {
        let mut dyes = Vec::new();
        let mut target = ItemStackSummary::empty();
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.target.test(stack, tags) {
                if !stack_is_empty(&target) {
                    return ItemStackSummary::empty();
                }
                target = stack.clone();
            } else {
                if !self.dye.test(stack, tags) {
                    return ItemStackSummary::empty();
                }
                dyes.push(dye_color_or_white(stack));
            }
        }
        if stack_is_empty(&target) || dyes.is_empty() {
            return ItemStackSummary::empty();
        }
        // `:117-120`: the new colour averages the target's own dye with the
        // ones on the grid, so it is read before the result is rebuilt.
        let new_color = apply_dyes(dyed_color(&target), &dyes);
        let mut result = create_with_original_components(&self.result, &target, 0);
        if result.item_id.is_some() {
            set_component(
                &mut result.component_patch,
                self.result.prototype_defaults(),
                ComponentValue::DyedColor(new_color),
            );
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImbueRecipe {
    pub source: Ingredient,
    pub material: Ingredient,
    pub result: ItemStackTemplate,
}

impl ImbueRecipe {
    /// `ImbueRecipe.matches` (`ImbueRecipe.java:59-78`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.width() != 3 || input.height() != 3 || input.ingredient_count() != 9 {
            return false;
        }
        for y in 0..3 {
            for x in 0..3 {
                let stack = input.get_item_at(x, y);
                let ingredient = if x == 1 && y == 1 {
                    &self.source
                } else {
                    &self.material
                };
                if !ingredient.test(stack, tags) {
                    return false;
                }
            }
        }
        true
    }

    /// `ImbueRecipe.assemble` (`ImbueRecipe.java:81-86`).
    pub fn assemble(&self, input: &CraftingInput) -> ItemStackSummary {
        let mut result = self.result.create();
        if result.item_id.is_none() {
            return result;
        }
        copy_component(
            &mut result.component_patch,
            self.result.prototype_defaults(),
            input.get_item_at(1, 1),
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        );
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecoratedPotRecipe {
    pub back: Ingredient,
    pub left: Ingredient,
    pub right: Ingredient,
    pub front: Ingredient,
    pub result: ItemStackTemplate,
}

impl DecoratedPotRecipe {
    /// `DecoratedPotRecipe.matches` (`DecoratedPotRecipe.java:75-82`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        input.width() == 3
            && input.height() == 3
            && input.ingredient_count() == 4
            && self.back.test(input.get_item_at(1, 0), tags)
            && self.left.test(input.get_item_at(0, 1), tags)
            && self.right.test(input.get_item_at(2, 1), tags)
            && self.front.test(input.get_item_at(1, 2), tags)
    }

    /// `DecoratedPotRecipe.assemble` (`DecoratedPotRecipe.java:84-88`).
    pub fn assemble(&self, input: &CraftingInput) -> ItemStackSummary {
        let side = |x, y| input.get_item_at(x, y).item_id.unwrap_or(0);
        let mut additional = single_component_patch(VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID);
        additional.pot_decorations_item_ids = vec![side(1, 0), side(0, 1), side(2, 1), side(1, 2)];
        apply_template(&self.result, &additional)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BookCloningRecipe {
    pub source: Ingredient,
    pub material: Ingredient,
    pub generation_min: i32,
    pub generation_max: i32,
    pub result: ItemStackTemplate,
}

impl BookCloningRecipe {
    /// `BookCloningRecipe.canCraftCopy` (`BookCloningRecipe.java:54-56`) over
    /// the written content `matches` reads at `:70`.
    fn can_craft_copy(&self, stack: &ItemStackSummary) -> bool {
        let Some(generation) = written_book_generation(stack) else {
            return false;
        };
        generation >= self.generation_min && generation <= self.generation_max
    }

    /// `BookCloningRecipe.matches` (`BookCloningRecipe.java:58-91`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.ingredient_count() < 2 {
            return false;
        }
        let mut has_material = false;
        let mut has_source = false;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.source.test(stack, tags) {
                // `:70-73`: no written content, or a generation outside
                // `allowed_generations`, refuses outright -- a copy of a copy
                // cannot be cloned again.
                if !self.can_craft_copy(stack) || has_source {
                    return false;
                }
                has_source = true;
            } else if self.material.test(stack, tags) {
                has_material = true;
            } else {
                return false;
            }
        }
        has_source && has_material
    }

    /// `BookCloningRecipe.assemble` (`BookCloningRecipe.java:93-125`).
    pub fn assemble(&self, input: &CraftingInput, tags: &ItemTagIndex) -> ItemStackSummary {
        let mut copies = 0;
        let mut source = ItemStackSummary::empty();
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.source.test(stack, tags) && has_written_book_content(stack) {
                if !stack_is_empty(&source) {
                    return ItemStackSummary::empty();
                }
                source = stack.clone();
            } else {
                if !self.material.test(stack, tags) {
                    return ItemStackSummary::empty();
                }
                copies += 1;
            }
        }
        let Some(content) = written_book_content(&source) else {
            return ItemStackSummary::empty();
        };
        let Some(copied) = craft_copy(content) else {
            return ItemStackSummary::empty();
        };
        let mut result = create_with_original_components(&self.result, &source, copies - 1);
        if result.item_id.is_none() {
            return result;
        }
        set_component(
            &mut result.component_patch,
            self.result.prototype_defaults(),
            ComponentValue::WrittenBookContent {
                content: copied,
                pages_nbt: source.component_patch.written_book_pages_nbt.clone(),
                page_filters_nbt: source.component_patch.written_book_page_filters_nbt.clone(),
            },
        );
        result
    }

    /// `BookCloningRecipe.getRemainingItems` (`BookCloningRecipe.java:128-143`).
    ///
    /// `:137` returns the source book itself, so the written content rides back
    /// into the grid; a bare item id would hand the player a blank book.
    pub fn remaining_items(&self, input: &CraftingInput) -> Vec<ItemStackSummary> {
        self.remainder_plan(input)
    }

    pub(super) fn remainder_plan(&self, input: &CraftingInput) -> Vec<ItemStackSummary> {
        let mut remainders = (0..input.size())
            .map(|_| ItemStackSummary::empty())
            .collect::<Vec<_>>();
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if let Some(remainder) = stack.item_id.and_then(crafting_remainder) {
                remainders[slot as usize] = remainder;
            } else if has_written_book_content(stack) {
                remainders[slot as usize] =
                    super::ingredient::copy_with_count(input.get_item(slot), 1);
                break;
            }
        }
        remainders
    }
}

/// `WrittenBookContent.craftCopy` (`WrittenBookContent.java:88-90`): the copy is
/// one generation on. The record constructor (`:68-78`) throws past
/// `MAX_GENERATION`, which the recipe reaches only because `assemble` re-reads
/// the content without the `allowed_generations` gate `matches` applied; no book
/// is handed out instead.
fn craft_copy(content: &WrittenBookContentSummary) -> Option<WrittenBookContentSummary> {
    let generation = content.generation + 1;
    (generation <= WRITTEN_BOOK_MAX_GENERATION).then(|| WrittenBookContentSummary {
        generation,
        ..content.clone()
    })
}

/// `writtenBookContent.generation()` as `matches` reads it at
/// `BookCloningRecipe.java:70-71` before handing it to `canCraftCopy`.
fn written_book_generation(stack: &ItemStackSummary) -> Option<i32> {
    written_book_content(stack).map(|content| content.generation)
}

/// `itemStack.has(DataComponents.WRITTEN_BOOK_CONTENT)`
/// (`BookCloningRecipe.java:100`, `:136`).
fn has_written_book_content(stack: &ItemStackSummary) -> bool {
    stack_has_component(
        stack,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapExtendingRecipe {
    pub pattern: ShapedRecipePattern,
    pub result: ItemStackTemplate,
}

impl MapExtendingRecipe {
    // MapExtendingRecipe.java:47-63: actual saved data decides eligibility; locked maps can scale.
    pub fn matches(
        &self,
        input: &CraftingInput,
        tags: &ItemTagIndex,
        map: Option<super::CraftingMapData>,
    ) -> bool {
        self.pattern.matches(input, tags)
            && input
                .items()
                .iter()
                .any(|item| item.component_patch.map_id.is_some())
            && map.is_some_and(|map| map.scale < 4 && !map.exploration)
    }

    // MapExtendingRecipe.java:65-81: the first MAP_ID supplies the original components.
    pub fn assemble(&self, input: &CraftingInput) -> ItemStackSummary {
        let Some(source) = input
            .items()
            .iter()
            .find(|item| item.component_patch.map_id.is_some())
        else {
            return ItemStackSummary::empty();
        };
        let mut result = create_with_original_components(&self.result, source, 0);
        if result.item_id.is_some() {
            set_component(
                &mut result.component_patch,
                self.result.prototype_defaults(),
                ComponentValue::MapPostProcessing(
                    bbb_protocol::packets::MapPostProcessingSummary::Scale,
                ),
            );
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FireworkRocketRecipe {
    pub shell: Ingredient,
    pub fuel: Ingredient,
    pub star: Ingredient,
    pub result: ItemStackTemplate,
}

impl FireworkRocketRecipe {
    /// `FireworkRocketRecipe.matches` (`FireworkRocketRecipe.java:51-79`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.ingredient_count() < 2 {
            return false;
        }
        let mut has_shell = false;
        let mut fuel_count = 0;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.shell.test(stack, tags) {
                if has_shell {
                    return false;
                }
                has_shell = true;
            } else if self.fuel.test(stack, tags) {
                fuel_count += 1;
                if fuel_count > 3 {
                    return false;
                }
            } else if !self.star.test(stack, tags) {
                return false;
            }
        }
        has_shell && fuel_count >= 1
    }

    /// `FireworkRocketRecipe.assemble` (`FireworkRocketRecipe.java:81-101`).
    pub fn assemble(&self, input: &CraftingInput, tags: &ItemTagIndex) -> ItemStackSummary {
        let mut fuel_count = 0;
        let mut explosions = Vec::new();
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.fuel.test(stack, tags) {
                fuel_count += 1;
            } else if self.star.test(stack, tags)
                && stack_has_component(stack, VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID)
            {
                explosions.push(firework_explosion_or_default(stack));
            }
        }
        let mut additional = single_component_patch(VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID);
        additional.fireworks_flight_duration = Some(fuel_count);
        additional.fireworks_explosions = explosions;
        apply_template(&self.result, &additional)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FireworkStarRecipe {
    pub shapes: Vec<(FireworkExplosionShapeSummary, Ingredient)>,
    pub trail: Ingredient,
    pub twinkle: Ingredient,
    pub fuel: Ingredient,
    pub dye: Ingredient,
    pub result: ItemStackTemplate,
}

impl FireworkStarRecipe {
    fn find_shape(
        &self,
        stack: &ItemStackSummary,
        tags: &ItemTagIndex,
    ) -> Option<FireworkExplosionShapeSummary> {
        self.shapes
            .iter()
            .find_map(|(shape, ingredient)| ingredient.test(stack, tags).then_some(*shape))
    }

    /// `FireworkStarRecipe.matches` (`FireworkStarRecipe.java:86-136`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.ingredient_count() < 2 {
            return false;
        }
        let mut has_fuel = false;
        let mut has_dye = false;
        let mut has_shape = false;
        let mut has_trail = false;
        let mut has_twinkle = false;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.twinkle.test(stack, tags) {
                if has_twinkle {
                    return false;
                }
                has_twinkle = true;
            } else if self.trail.test(stack, tags) {
                if has_trail {
                    return false;
                }
                has_trail = true;
            } else if self.fuel.test(stack, tags) {
                if has_fuel {
                    return false;
                }
                has_fuel = true;
            } else if self.dye.test(stack, tags) && has_dye_component(stack) {
                has_dye = true;
            } else if self.find_shape(stack, tags).is_some() {
                if has_shape {
                    return false;
                }
                has_shape = true;
            } else {
                return false;
            }
        }
        has_fuel && has_dye
    }

    /// `FireworkStarRecipe.assemble` (`FireworkStarRecipe.java:138-164`).
    pub fn assemble(&self, input: &CraftingInput, tags: &ItemTagIndex) -> ItemStackSummary {
        let mut shape = FireworkExplosionShapeSummary::SmallBall;
        let mut has_twinkle = false;
        let mut has_trail = false;
        let mut colors = Vec::new();
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if let Some(found) = self.find_shape(stack, tags) {
                shape = found;
            } else if self.twinkle.test(stack, tags) {
                has_twinkle = true;
            } else if self.trail.test(stack, tags) {
                has_trail = true;
            } else if self.dye.test(stack, tags) {
                colors.push(vanilla_dye_color_firework_color(dye_color_or_white(stack)));
            }
        }
        let mut result = self.result.create();
        if result.item_id.is_none() {
            return result;
        }
        set_component(
            &mut result.component_patch,
            self.result.prototype_defaults(),
            ComponentValue::FireworkExplosion(FireworkExplosionSummary {
                shape,
                colors,
                fade_colors: Vec::new(),
                has_trail,
                has_twinkle,
            }),
        );
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FireworkStarFadeRecipe {
    pub target: Ingredient,
    pub dye: Ingredient,
    pub result: ItemStackTemplate,
}

impl FireworkStarFadeRecipe {
    /// `FireworkStarFadeRecipe.matches` (`FireworkStarFadeRecipe.java:45-73`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.ingredient_count() < 2 {
            return false;
        }
        let mut has_dye = false;
        let mut has_target = false;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.dye.test(stack, tags) && has_dye_component(stack) {
                has_dye = true;
            } else if self.target.test(stack, tags) {
                if has_target {
                    return false;
                }
                has_target = true;
            } else {
                return false;
            }
        }
        has_target && has_dye
    }

    /// `FireworkStarFadeRecipe.assemble` (`FireworkStarFadeRecipe.java:75-96`).
    pub fn assemble(&self, input: &CraftingInput, tags: &ItemTagIndex) -> ItemStackSummary {
        let mut colors = Vec::new();
        let mut target = None;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if self.dye.test(stack, tags) {
                colors.push(vanilla_dye_color_firework_color(dye_color_or_white(stack)));
            } else if self.target.test(stack, tags) {
                target = Some(stack);
            }
        }
        let Some(target) = target.filter(|_| !colors.is_empty()) else {
            return ItemStackSummary::empty();
        };
        let mut result = create_with_original_components(&self.result, target, 0);
        if result.item_id.is_none() {
            return result;
        }
        // `:91` reads the explosion off the rebuilt result, not off the target,
        // so a result template that declared its own explosion wins.
        let mut explosion = firework_explosion_or_default(&result);
        explosion.fade_colors = colors;
        set_component(
            &mut result.component_patch,
            self.result.prototype_defaults(),
            ComponentValue::FireworkExplosion(explosion),
        );
        result
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BannerDuplicateRecipe {
    pub banner: Ingredient,
    pub result: ItemStackTemplate,
}

impl BannerDuplicateRecipe {
    /// `BannerDuplicateRecipe.matches` (`BannerDuplicateRecipe.java:33-77`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        if input.ingredient_count() != 2 {
            return false;
        }
        let mut color = None;
        let mut has_target = false;
        let mut has_source = false;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            let Some(item_id) = stack.item_id else {
                return false;
            };
            if !self.banner.test(stack, tags) || !is_banner_item(item_id) {
                return false;
            }
            match color {
                None => color = Some(item_id),
                Some(expected) if expected != item_id => return false,
                Some(_) => {}
            }
            let patterned = banner_pattern_count(stack) > 0;
            if patterned {
                if has_source || banner_pattern_count(stack) > 6 {
                    return false;
                }
                has_source = true;
            } else if has_target {
                return false;
            } else {
                has_target = true;
            }
        }
        has_source && has_target
    }

    /// `BannerDuplicateRecipe.assemble` (`BannerDuplicateRecipe.java:79-91`):
    /// `:85` rebuilds the result from the patterned banner's own patch, so the
    /// copy comes out patterned.
    pub fn assemble(&self, input: &CraftingInput) -> ItemStackSummary {
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            let count = banner_pattern_count(stack);
            if count > 0 && count <= 6 {
                return create_with_original_components(&self.result, stack, 0);
            }
        }
        ItemStackSummary::empty()
    }

    /// `BannerDuplicateRecipe.getRemainingItems` (`BannerDuplicateRecipe.java:94-110`).
    ///
    /// `:104` returns the patterned banner itself, so its layers ride back into
    /// the grid; a bare item id would hand the player a blank banner.
    pub fn remaining_items(&self, input: &CraftingInput) -> Vec<ItemStackSummary> {
        self.remainder_plan(input)
    }

    pub(super) fn remainder_plan(&self, input: &CraftingInput) -> Vec<ItemStackSummary> {
        let mut remainders = (0..input.size())
            .map(|_| ItemStackSummary::empty())
            .collect::<Vec<_>>();
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if let Some(remainder) = stack.item_id.and_then(crafting_remainder) {
                remainders[slot as usize] = remainder;
            } else if banner_pattern_count(stack) > 0 {
                remainders[slot as usize] =
                    super::ingredient::copy_with_count(input.get_item(slot), 1);
            }
        }
        remainders
    }
}

fn banner_pattern_count(stack: &ItemStackSummary) -> usize {
    stack.component_patch.banner_pattern_layers.len()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShieldDecorationRecipe {
    pub banner: Ingredient,
    pub target: Ingredient,
    pub result: ItemStackTemplate,
}

impl ShieldDecorationRecipe {
    /// `ShieldDecorationRecipe.matches` (`ShieldDecorationRecipe.java:44-81`).
    pub fn matches(&self, input: &CraftingInput, tags: &ItemTagIndex) -> bool {
        let mut has_clear_target = false;
        let mut has_pattern_banner = false;
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            if self.banner.test(stack, tags) && stack.item_id.is_some_and(is_banner_item) {
                if has_pattern_banner {
                    return false;
                }
                has_pattern_banner = true;
            } else if self.target.test(stack, tags) {
                if has_clear_target || banner_pattern_count(stack) > 0 {
                    return false;
                }
                has_clear_target = true;
            } else {
                return false;
            }
        }
        has_clear_target && has_pattern_banner
    }

    /// `ShieldDecorationRecipe.assemble` (`ShieldDecorationRecipe.java:83-104`).
    pub fn assemble(&self, input: &CraftingInput, tags: &ItemTagIndex) -> ItemStackSummary {
        let mut patterns = None;
        let mut base_color = DYE_COLOR_WHITE_ID;
        let mut target = ItemStackSummary::empty();
        for slot in 0..input.size() {
            let stack = input.get_item(slot);
            if stack_is_empty(stack) {
                continue;
            }
            let banner_color = stack
                .item_id
                .filter(|_| self.banner.test(stack, tags))
                .and_then(banner_item_color);
            if let Some(color) = banner_color {
                patterns = banner_patterns(stack);
                base_color = color;
            } else if self.target.test(stack, tags) {
                target = stack.clone();
            }
        }
        let mut result = create_with_original_components(&self.result, &target, 0);
        if result.item_id.is_none() {
            return result;
        }
        let defaults = self.result.prototype_defaults();
        match patterns {
            // `:101` sets whatever the banner carried, so a grid without one
            // writes the null `ItemStack.set` turns into a removal.
            Some(patterns) => set_component(
                &mut result.component_patch,
                defaults,
                ComponentValue::BannerPatterns(patterns),
            ),
            None => set_component_to_null(
                &mut result.component_patch,
                defaults,
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            ),
        }
        set_component(
            &mut result.component_patch,
            defaults,
            ComponentValue::BaseColor(base_color),
        );
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairItemRecipe;

impl RepairItemRecipe {
    /// `RepairItemRecipe.matches` (`RepairItemRecipe.java:55-57`).
    pub fn matches(&self, input: &CraftingInput) -> bool {
        items_to_combine(input).is_some()
    }

    /// `RepairItemRecipe.assemble` (`RepairItemRecipe.java:59-85`).
    pub fn assemble(&self, input: &CraftingInput, tags: &ItemTagIndex) -> ItemStackSummary {
        let Some((first, second)) = items_to_combine(input) else {
            return ItemStackSummary::empty();
        };
        let item_id = first.item_id.expect("combined stacks are non-empty");
        let (max1, max2) = (max_damage(first), max_damage(second));
        let durability = max1.max(max2);
        let remaining =
            (max1 - damage_value(first)) + (max2 - damage_value(second)) + durability * 5 / 100;
        let mut result = item_stack(item_id, 1);
        let patch = &mut result.component_patch;
        if Some(durability) != prototype_int(item_id, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID) {
            put_patch_entry(patch, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID);
            patch.max_damage = Some(durability);
        }
        let damage = (durability - remaining).clamp(0, durability.max(0));
        if Some(damage) != prototype_int(item_id, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID) {
            put_patch_entry(patch, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
            patch.damage = Some(damage);
        }
        let mut curses: Vec<ItemEnchantmentSummary> = Vec::new();
        for enchantment in enchantments(first).iter().chain(enchantments(second)) {
            if !tags.is_curse_enchantment(enchantment.holder_id) {
                continue;
            }
            match curses
                .iter_mut()
                .find(|kept| kept.holder_id == enchantment.holder_id)
            {
                Some(kept) => kept.level = kept.level.max(enchantment.level),
                None => curses.push(enchantment.clone()),
            }
        }
        if !curses.is_empty() {
            put_patch_entry(patch, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
            patch.enchantments = curses;
        }
        result
    }
}

fn prototype_int(item_id: i32, type_id: i32) -> Option<i32> {
    prototype_component_value(item_id, type_id).and_then(codec_int)
}

/// `getOrDefault(type, 0)` over an int component.
fn int_component(stack: &ItemStackSummary, type_id: i32, value: Option<i32>) -> i32 {
    match patch_declares(&stack.component_patch, type_id) {
        Some(true) => value.unwrap_or(0),
        Some(false) => 0,
        None => stack
            .item_id
            .and_then(|item_id| prototype_int(item_id, type_id))
            .unwrap_or(0),
    }
}

/// `ItemStack.getMaxDamage` (`ItemStack.java:433-435`).
fn max_damage(stack: &ItemStackSummary) -> i32 {
    int_component(
        stack,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        stack.component_patch.max_damage,
    )
}

/// `ItemStack.getDamageValue` (`ItemStack.java:425-427`).
fn damage_value(stack: &ItemStackSummary) -> i32 {
    int_component(
        stack,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        stack.component_patch.damage,
    )
    .clamp(0, max_damage(stack).max(0))
}

/// `EnchantmentHelper.getEnchantmentsForCrafting` for a damageable item; every
/// 26.1 prototype default for `ENCHANTMENTS` is empty.
fn enchantments(stack: &ItemStackSummary) -> &[ItemEnchantmentSummary] {
    match patch_declares(
        &stack.component_patch,
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    ) {
        Some(true) => &stack.component_patch.enchantments,
        _ => &[],
    }
}

fn put_patch_entry(patch: &mut DataComponentPatchSummary, type_id: i32) {
    patch.removed_type_ids.retain(|id| *id != type_id);
    if !patch.added_type_ids.contains(&type_id) {
        patch.added_type_ids.push(type_id);
    }
}

fn items_to_combine(input: &CraftingInput) -> Option<(&ItemStackSummary, &ItemStackSummary)> {
    if input.ingredient_count() != 2 {
        return None;
    }
    let mut first: Option<&ItemStackSummary> = None;
    for slot in 0..input.size() {
        let stack = input.get_item(slot);
        if stack_is_empty(stack) {
            continue;
        }
        let Some(first) = first else {
            first = Some(stack);
            continue;
        };
        // `RepairItemRecipe.canCombine` (`RepairItemRecipe.java:45-53`) asks
        // the stack, so a patch that adds or removes either component decides
        // it rather than the item's registration.
        let combine = stack.item_id == first.item_id
            && first.count == 1
            && stack.count == 1
            && is_repairable(first)
            && is_repairable(stack);
        return combine.then_some((first, stack));
    }
    None
}

/// `itemStack.has(DataComponents.DYE)` as `DyeRecipe.matches` (`:80`),
/// `FireworkStarRecipe.matches` (`FireworkStarRecipe.java:118`) and
/// `FireworkStarFadeRecipe.matches` (`FireworkStarFadeRecipe.java:56`) all ask
/// it, which a patch can add to a non-dye or take off a dye.
fn has_dye_component(stack: &ItemStackSummary) -> bool {
    stack_has_component(
        stack,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    )
}

fn is_repairable(stack: &ItemStackSummary) -> bool {
    stack_has_component(
        stack,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    ) && stack_has_component(
        stack,
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    )
}

/// Default `CraftingRecipe.getRemainingItems` (`CraftingRecipe.java:22-36`).
pub(super) fn default_remainder_plan(input: &CraftingInput) -> Vec<ItemStackSummary> {
    input
        .items()
        .iter()
        .map(|stack| {
            stack
                .item_id
                .and_then(crafting_remainder)
                .unwrap_or_else(ItemStackSummary::empty)
        })
        .collect()
}
