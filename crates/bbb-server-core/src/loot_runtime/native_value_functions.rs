use super::*;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID as CUSTOM_DATA,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID as DAMAGE,
};
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::rules::damage_and_max;

impl<'a> Eval<'a> {
    pub(super) fn environment(&self, path: &str) -> Result<&'a dyn LootItemEnvironment> {
        self.context.item_environment().ok_or_else(|| {
            self.err(
                path,
                LootRuntimeReason::Catalog(
                    "Native function requires actual item environment".into(),
                ),
            )
        })
    }
}

impl Eval<'_> {
    /// `CustomData.update` merges `tag` into the held compound; an empty
    /// result removes the component.
    pub(super) fn apply_set_custom_data(
        &mut self,
        tag: &[NbtSummaryEntry],
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let value = self
            .at(path, environment.component_runtime(&stack, CUSTOM_DATA))?
            .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()));
        let NbtSummaryValue::Compound(mut fields) = value else {
            return Err(self.err(path, malformed("set_custom_data", "tag")));
        };
        bbb_protocol::merge_command_nbt_compound(&mut fields, tag);
        let value = (!fields.is_empty()).then_some(NbtSummaryValue::Compound(fields));
        self.at(
            path,
            environment.with_component_runtime(stack, CUSTOM_DATA, value.as_ref()),
        )
        .map(RuntimeItemStack::into_summary)
    }

    /// A stack that cannot take damage evaluates no provider.
    pub(super) fn apply_set_damage(
        &mut self,
        damage: &NumberProvider,
        add: bool,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let Some((old, max)) = damage_and_max(stack.summary()) else {
            return Ok(stack.into_summary());
        };
        let base = if add {
            1.0 - old as f32 / max as f32
        } else {
            0.0
        };
        let value = self.number_float(damage, &format!("{path}.damage"))?;
        let damage =
            mth_floor((1.0 - (value + base).clamp(0.0, 1.0)) * max as f32).clamp(0, max.max(0));
        stack
            .with_component_patch(
                &DataComponentPatchSummary {
                    added_type_ids: vec![DAMAGE],
                    damage: Some(damage),
                    ..Default::default()
                },
                environment.item_resolver(),
            )
            .map(RuntimeItemStack::into_summary)
            .map_err(|error| self.catalog(path, error))
    }
}
