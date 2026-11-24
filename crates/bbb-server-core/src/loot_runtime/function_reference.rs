use super::*;

impl Eval<'_> {
    pub(super) fn apply_set_item(
        &mut self,
        item_id: i32,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        self.at(path, environment.with_item_runtime(stack, item_id))
            .map(RuntimeItemStack::into_summary)
    }

    /// `FunctionReference`: a missing or already running modifier leaves the
    /// stack.
    pub(super) fn apply_function_reference(
        &mut self,
        name: &str,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        if self.visited_functions.iter().any(|seen| seen == name) {
            return Ok(stack);
        }
        let tables = self.tables;
        let Some(function) = tables.modifier(name) else {
            return Ok(stack);
        };
        self.visited_functions.push(name.to_string());
        let result = self.apply_loot_function(function, stack, path);
        self.visited_functions.pop();
        result
    }
}
