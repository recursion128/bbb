use super::*;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID as CUSTOM_MODEL_DATA;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::loot::ListOperation;

const MAX_LIST_SIZE: usize = i32::MAX as usize;

type Edit<T> = Option<(ListOperation, Vec<T>)>;

impl Eval<'_> {
    pub(super) fn apply_native_model_data(
        &mut self,
        floats: &Edit<NumberProvider>,
        flags: &Edit<NbtSummaryValue>,
        strings: &Edit<NbtSummaryValue>,
        colors: &Edit<NumberProvider>,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let original = self
            .at(
                path,
                environment.component_runtime(&stack, CUSTOM_MODEL_DATA),
            )?
            .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()));
        let current = |field: &str| match nbt_field(&original, field) {
            Some(NbtSummaryValue::List(values)) => values.clone(),
            _ => Vec::new(),
        };
        let mut fields = Vec::new();
        let mut push = |field: &str, values: Vec<NbtSummaryValue>| {
            if !values.is_empty() {
                fields.push(NbtSummaryEntry {
                    name: field.into(),
                    value: NbtSummaryValue::List(values),
                });
            }
        };
        let mut floats_out = current("floats");
        if let Some((operation, providers)) = floats {
            let mut mapped = Vec::with_capacity(providers.len());
            for (index, provider) in providers.iter().enumerate() {
                let value = self.number_float(provider, &format!("{path}.floats[{index}]"))?;
                mapped.push(NbtSummaryValue::Float(value.to_bits()));
            }
            floats_out = operation.apply(floats_out, mapped, MAX_LIST_SIZE);
        }
        push("floats", floats_out);
        for (field, edit) in [("flags", flags), ("strings", strings)] {
            let mut values = current(field);
            if let Some((operation, replacement)) = edit {
                values = operation.apply(values, replacement.clone(), MAX_LIST_SIZE);
            }
            push(field, values);
        }
        let mut colors_out = current("colors");
        if let Some((operation, providers)) = colors {
            let mut mapped = Vec::with_capacity(providers.len());
            for (index, provider) in providers.iter().enumerate() {
                let value = self.number_int(provider, &format!("{path}.colors[{index}]"))?;
                mapped.push(NbtSummaryValue::Int(value));
            }
            colors_out = operation.apply(colors_out, mapped, MAX_LIST_SIZE);
        }
        push("colors", colors_out);
        let value = NbtSummaryValue::Compound(fields);
        self.at(
            path,
            environment.with_component_runtime(stack, CUSTOM_MODEL_DATA, Some(&value)),
        )
        .map(RuntimeItemStack::into_summary)
    }
}
