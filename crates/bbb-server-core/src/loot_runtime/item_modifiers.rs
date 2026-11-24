use super::*;
use bbb_server_datapack::loot::{ComponentPatchParameter, HolderSet, ItemPredicate};

impl Eval<'_> {
    pub(super) fn apply_enchant_randomly(
        &mut self,
        options: Option<&HolderSet>,
        only_compatible: bool,
        include_additional_cost_component: bool,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let allowed =
            include_additional_cost_component && self.context.additional_cost_component_allowed();
        let candidates = self.at(
            path,
            environment.random_enchantment_candidates(&stack, options, only_compatible),
        )?;
        if candidates.is_empty() {
            return Ok(stack);
        }
        let count = i32::try_from(candidates.len()).unwrap_or(i32::MAX);
        let selected = &candidates[self.random.next_int(count) as usize];
        // `Mth.nextInt`.
        let level = if selected.min_level >= selected.max_level {
            selected.min_level
        } else {
            self.random
                .next_int(
                    selected
                        .max_level
                        .wrapping_sub(selected.min_level)
                        .wrapping_add(1),
                )
                .wrapping_add(selected.min_level)
        };
        let mut stack = self.at(
            path,
            environment.with_random_enchantment(stack, selected, level),
        )?;
        if allowed {
            let cost = 2_i32
                .wrapping_add(
                    self.random
                        .next_int(5_i32.wrapping_add(level.wrapping_mul(10))),
                )
                .wrapping_add(level.wrapping_mul(3));
            stack = RuntimeItemStack::from_summary(stack)
                .with_additional_trade_cost(Some(cost))
                .into_summary();
        }
        Ok(stack)
    }

    pub(super) fn apply_enchant_with_levels(
        &mut self,
        levels: &NumberProvider,
        options: Option<&HolderSet>,
        include_additional_cost_component: bool,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let cost = self.number_int(levels, &format!("{path}.levels"))?;
        let allowed =
            include_additional_cost_component && self.context.additional_cost_component_allowed();
        let result = environment.enchant_with_levels(stack, cost, options, self.random);
        let mut stack = self.at(path, result)?;
        if allowed && stack_count(&stack) > 0 && cost > 0 {
            stack = RuntimeItemStack::from_summary(stack)
                .with_additional_trade_cost(Some(cost))
                .into_summary();
        }
        Ok(stack)
    }

    pub(super) fn apply_filtered(
        &mut self,
        item_filter: &ItemPredicate,
        on_pass: &Option<Box<LootFunction>>,
        on_fail: &Option<Box<LootFunction>>,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let matches = self.at(path, environment.matches_item(item_filter, &stack))?;
        let (field, nested) = if matches {
            ("on_pass", on_pass)
        } else {
            ("on_fail", on_fail)
        };
        match nested {
            Some(nested) => self.apply_loot_function(nested, stack, &format!("{path}.{field}")),
            None => Ok(stack),
        }
    }

    pub(super) fn apply_set_components(
        &mut self,
        components: &ComponentPatchParameter,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let resolver = environment.item_resolver();
        let patch = self.at(path, prepare_component_patch(components, resolver))?;
        let stack = RuntimeItemStack::from_summary(stack);
        self.at(path, apply_prepared_component_patch(stack, patch, resolver))
            .map(RuntimeItemStack::into_summary)
    }

    /// `use_input_count` multiplies the result by the input count.
    pub(super) fn apply_furnace_smelt(
        &mut self,
        use_input_count: bool,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        if stack_count(&stack) == 0 {
            return Ok(stack);
        }
        let environment = self.environment(path)?;
        let input = ItemStackSummary {
            item_id: stack.item_id,
            count: stack_count(&stack),
            component_patch: Default::default(),
        };
        let Some(result) = self.at(path, environment.smelt_result(&input))? else {
            return Ok(stack);
        };
        if result.count() == 0 {
            return Ok(stack);
        }
        let input = if use_input_count {
            stack_count(&stack)
        } else {
            1
        };
        let limit = result
            .max_stack_size()
            .map_err(|error| self.catalog(path, error))?;
        let count = input.wrapping_mul(result.count()).min(limit);
        Ok(result.with_count(count).into_summary())
    }
}

#[cfg(test)]
mod tests;
