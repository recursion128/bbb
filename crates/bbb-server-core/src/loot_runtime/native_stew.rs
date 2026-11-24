use super::*;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID as STEW_EFFECTS;
use bbb_protocol::item_types::VANILLA_ITEM_SUSPICIOUS_STEW_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

impl Eval<'_> {
    pub(super) fn apply_native_stew(
        &mut self,
        effects: &[(String, NumberProvider)],
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let stack = RuntimeItemStack::from_summary(stack);
        // Neither guard evaluates a duration or draws.
        if stack.is_empty()
            || stack.item_id() != Some(VANILLA_ITEM_SUSPICIOUS_STEW_ID)
            || effects.is_empty()
        {
            return Ok(stack.into_summary());
        }
        let selected = self.random.next_int(effects.len() as i32) as usize;
        let (id, provider) = &effects[selected];
        let duration =
            self.number_int(provider, &format!("{path}.effects[{selected}].duration"))?;
        let environment = self.environment(path)?;
        let instantaneous = self.at(path, environment.mob_effect_instantaneous(id))?;
        let duration = if instantaneous {
            duration
        } else {
            duration.wrapping_mul(20)
        };
        let mut values = match self.at(path, environment.component_runtime(&stack, STEW_EFFECTS))? {
            Some(NbtSummaryValue::List(values)) => values,
            None => Vec::new(),
            Some(_) => return Err(self.err(path, malformed("set_stew_effect", "component"))),
        };
        // An existing effect of the same type is not replaced.
        values.push(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string(id.clone()),
            },
            NbtSummaryEntry {
                name: "duration".into(),
                value: NbtSummaryValue::Int(duration),
            },
        ]));
        self.at(
            path,
            environment.with_component_runtime(
                stack,
                STEW_EFFECTS,
                Some(&NbtSummaryValue::List(values)),
            ),
        )
        .map(RuntimeItemStack::into_summary)
    }
}
