use super::*;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID as ATTRIBUTE_MODIFIERS;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::loot::AttributeModifierTemplate;

impl Eval<'_> {
    pub(super) fn apply_native_attributes(
        &mut self,
        modifiers: &[AttributeModifierTemplate],
        replace: bool,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let mut entries = Vec::new();
        // `replace` starts from no modifiers without reading the held ones.
        if !replace {
            let old = match self.at(
                path,
                environment.component_runtime(&stack, ATTRIBUTE_MODIFIERS),
            )? {
                Some(NbtSummaryValue::List(old)) => old,
                None => Vec::new(),
                Some(_) => return Err(self.err(path, malformed("set_attributes", "component"))),
            };
            for entry in old {
                let field = |name| {
                    nbt_string(&entry, name)
                        .ok_or_else(|| self.err(path, malformed("set_attributes", "component")))
                };
                entries.push((field("type")?, field("id")?, entry));
            }
        }
        for (index, modifier) in modifiers.iter().enumerate() {
            // Even a single slot draws before the amount.
            let slot = self.random.next_int(modifier.slots.len() as i32) as usize;
            let amount = self.number_float(
                &modifier.amount,
                &format!("{path}.modifiers[{index}].amount"),
            )?;
            // A modifier is identified by attribute and id together.
            entries.retain(|(attribute, id, _)| {
                attribute != &modifier.attribute || id != &modifier.id
            });
            let field = |name: &str, value| NbtSummaryEntry {
                name: name.into(),
                value,
            };
            let value = NbtSummaryValue::Compound(vec![
                field("type", NbtSummaryValue::string(modifier.attribute.clone())),
                field("id", NbtSummaryValue::string(modifier.id.clone())),
                field(
                    "amount",
                    NbtSummaryValue::Double(f64::from(amount).to_bits()),
                ),
                field(
                    "operation",
                    NbtSummaryValue::string(modifier.operation.clone()),
                ),
                field(
                    "slot",
                    NbtSummaryValue::string(modifier.slots[slot].clone()),
                ),
            ]);
            entries.push((modifier.attribute.clone(), modifier.id.clone(), value));
        }
        let value = NbtSummaryValue::List(entries.into_iter().map(|(_, _, entry)| entry).collect());
        self.at(
            path,
            environment.with_component_runtime(stack, ATTRIBUTE_MODIFIERS, Some(&value)),
        )
        .map(RuntimeItemStack::into_summary)
    }
}
