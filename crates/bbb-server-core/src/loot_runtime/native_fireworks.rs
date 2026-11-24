use super::*;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID as FIREWORKS,
    VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID as EXPLOSION,
};
use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::loot::ListOperation;

impl Eval<'_> {
    pub(super) fn apply_set_fireworks(
        &mut self,
        flight_duration: Option<i8>,
        explosions: Option<&(ListOperation, Vec<NbtSummaryValue>)>,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        self.edit_record(FIREWORKS, stack, path, |fields| {
            if let Some(flight) = flight_duration {
                replace_field(fields, "flight_duration", NbtSummaryValue::Byte(flight));
            }
            if let Some((operation, values)) = explosions {
                let current = match fields.iter().rev().find(|field| field.name == "explosions") {
                    Some(NbtSummaryEntry {
                        value: NbtSummaryValue::List(current),
                        ..
                    }) => current.clone(),
                    _ => Vec::new(),
                };
                let values = operation.apply(current, values.clone(), usize::MAX);
                replace_field(fields, "explosions", NbtSummaryValue::List(values));
            }
        })
    }

    /// Fields the function leaves out keep their held values.
    pub(super) fn apply_set_firework_explosion(
        &mut self,
        values: &[NbtSummaryEntry],
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        self.edit_record(EXPLOSION, stack, path, |fields| {
            for entry in values {
                replace_field(fields, &entry.name, entry.value.clone());
            }
        })
    }

    fn edit_record(
        &mut self,
        type_id: i32,
        stack: LootStack,
        path: &str,
        edit: impl FnOnce(&mut Vec<NbtSummaryEntry>),
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let original = self
            .at(path, environment.component_runtime(&stack, type_id))?
            .unwrap_or_else(|| default_component(type_id));
        let NbtSummaryValue::Compound(mut fields) = original else {
            return Err(self.err(path, malformed("fireworks", "component")));
        };
        edit(&mut fields);
        let value = NbtSummaryValue::Compound(fields);
        self.at(
            path,
            environment.with_component_runtime(stack, type_id, Some(&value)),
        )
        .map(RuntimeItemStack::into_summary)
    }
}

fn default_component(type_id: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(if type_id == EXPLOSION {
        vec![NbtSummaryEntry {
            name: "shape".into(),
            value: NbtSummaryValue::String(
                FireworkExplosionShapeSummary::SmallBall.as_str().into(),
            ),
        }]
    } else {
        Vec::new()
    })
}

fn replace_field(
    fields: &mut Vec<NbtSummaryEntry>,
    name: impl Into<bbb_protocol::JavaString>,
    value: NbtSummaryValue,
) {
    let name = name.into();
    fields.retain(|field| field.name != name);
    fields.push(NbtSummaryEntry { name, value });
}
