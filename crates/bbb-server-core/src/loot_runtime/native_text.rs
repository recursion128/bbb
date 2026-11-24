use super::*;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID as LORE;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::loot::ListOperation;

const MAX_LORE_LINES: usize = 256;

impl Eval<'_> {
    /// An absent name leaves the stack untouched.
    pub(super) fn apply_set_name(
        &mut self,
        name: Option<&NbtSummaryValue>,
        entity: Option<LootContextParam>,
        component: i32,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let Some(name) = name else {
            return Ok(stack);
        };
        let name = match entity {
            Some(entity) => self.context.resolve_text(entity, name),
            None => name.clone(),
        };
        self.write_component(stack, component, Some(&name), path)
    }

    /// Every line resolves before the list bounds apply.
    pub(super) fn apply_set_lore(
        &mut self,
        lore: &[NbtSummaryValue],
        mode: ListOperation,
        entity: Option<LootContextParam>,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let held = match self.at(path, environment.component_runtime(&stack, LORE))? {
            Some(NbtSummaryValue::List(held)) => held,
            None => Vec::new(),
            Some(_) => return Err(self.err(path, malformed("set_lore", "component"))),
        };
        let mut resolved = Vec::with_capacity(lore.len());
        for line in lore {
            resolved.push(match entity {
                Some(entity) => self.context.resolve_text(entity, line),
                None => line.clone(),
            });
        }
        let value = NbtSummaryValue::List(mode.apply(held, resolved, MAX_LORE_LINES));
        self.at(
            path,
            environment.with_component_runtime(stack, LORE, Some(&value)),
        )
        .map(RuntimeItemStack::into_summary)
    }
}
