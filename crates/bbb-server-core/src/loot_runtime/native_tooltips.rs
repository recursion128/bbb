use super::*;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID as TOOLTIP_DISPLAY;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

impl Eval<'_> {
    pub(super) fn apply_native_tooltips(
        &mut self,
        toggles: &[(&'static str, bool)],
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let display = self
            .at(path, environment.component_runtime(&stack, TOOLTIP_DISPLAY))?
            .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()));
        let NbtSummaryValue::Compound(mut fields) = display else {
            return Err(self.err(path, malformed("toggle_tooltips", "tooltip_display")));
        };
        let mut hidden: Vec<String> = match fields
            .iter()
            .rev()
            .find(|field| field.name == "hidden_components")
            .map(|field| &field.value)
        {
            Some(NbtSummaryValue::List(values)) => values
                .iter()
                .map(|value| match value {
                    NbtSummaryValue::String(id) => id.try_to_string().map_err(|_| {
                        self.err(path, malformed("toggle_tooltips", "tooltip_display"))
                    }),
                    _ => Err(self.err(path, malformed("toggle_tooltips", "tooltip_display"))),
                })
                .collect::<Result<_>>()?,
            Some(_) => return Err(self.err(path, malformed("toggle_tooltips", "tooltip_display"))),
            None => Vec::new(),
        };
        // `TooltipDisplay.withHidden` keeps insertion order.
        for &(component, shown) in toggles {
            if shown {
                hidden.retain(|id| id != component);
            } else if !hidden.iter().any(|id| id == component) {
                hidden.push(component.to_owned());
            }
        }
        fields.retain(|field| field.name != "hidden_components");
        if !hidden.is_empty() {
            fields.push(NbtSummaryEntry {
                name: "hidden_components".into(),
                value: NbtSummaryValue::List(
                    hidden.into_iter().map(NbtSummaryValue::string).collect(),
                ),
            });
        }
        let display = NbtSummaryValue::Compound(fields);
        self.at(
            path,
            environment.with_component_runtime(stack, TOOLTIP_DISPLAY, Some(&display)),
        )
        .map(RuntimeItemStack::into_summary)
    }
}
