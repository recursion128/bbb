use super::*;

pub(super) enum ExpansionOutput<'o, 'p> {
    Weighted {
        valid: &'o mut Vec<(String, ValidEntry<'p>)>,
        total_weight: &'o mut i32,
    },
    Contents(&'o mut Vec<LootStack>),
}

impl Eval<'_> {
    pub(super) fn apply_modify_contents(
        &mut self,
        component: i32,
        modifier: &LootFunction,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        if stack_count(&stack) == 0 {
            return Ok(stack);
        }
        let stack = RuntimeItemStack::from_summary(stack);
        let environment = self.environment(path)?;
        let Some(contents) = self.at(path, environment.contents_runtime(&stack, component))? else {
            return Ok(stack.into_summary());
        };
        let mut modified = Vec::with_capacity(contents.len());
        for (index, item) in contents.into_iter().enumerate() {
            // Empty children bypass the modifier.
            if item.is_empty() {
                modified.push(item);
                continue;
            }
            let item_path = format!("{path}.modifier[{index}]");
            let item = self.apply_loot_function(modifier, item.into_summary(), &item_path)?;
            let item = RuntimeItemStack::from_summary(item);
            let limit = item
                .max_stack_size()
                .map_err(|error| self.catalog(&item_path, error))?;
            modified.push(if !item.is_empty() && item.count() > limit {
                item.with_count(limit)
            } else {
                item
            });
        }
        self.at(
            path,
            environment.with_contents_runtime(stack, component, modified),
        )
        .map(RuntimeItemStack::into_summary)
    }

    pub(super) fn apply_set_contents(
        &mut self,
        component: i32,
        entries: &[LootEntry],
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        if stack_count(&stack) == 0 {
            return Ok(stack);
        }
        let stack = RuntimeItemStack::from_summary(stack);
        let mut emitted = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            // Each entry expands straight into the contents, without weights.
            self.expand(
                entry,
                &format!("{path}.entries[{index}]"),
                &mut ExpansionOutput::Contents(&mut emitted),
            )?;
        }
        let contents = emitted
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect();
        let environment = self.environment(path)?;
        self.at(
            path,
            environment.with_contents_runtime(stack, component, contents),
        )
        .map(RuntimeItemStack::into_summary)
    }

    pub(super) fn accept_expanded<'p>(
        &mut self,
        entry: ValidEntry<'p>,
        path: &str,
        output: &mut ExpansionOutput<'_, 'p>,
    ) -> Result<()> {
        match output {
            ExpansionOutput::Weighted {
                valid,
                total_weight,
            } => {
                let weight = self.entry_weight(entry.entry());
                if weight > 0 {
                    valid.push((path.to_string(), entry));
                    **total_weight += weight;
                }
                Ok(())
            }
            ExpansionOutput::Contents(contents) => {
                self.create_item_stack(entry, path.to_string(), &[], &[], None, contents)
            }
        }
    }
}
