use super::*;

impl Eval<'_> {
    /// `CopyComponentsFunction.run`: a block entity source walks its
    /// collected components; any other source visits the admitted types in
    /// registry order.
    pub(super) fn copy_components(
        &self,
        source: LootContextParam,
        include: Option<&[&'static str]>,
        exclude: Option<&[&'static str]>,
        mut stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let admits = |id: &str| {
            exclude.is_none_or(|exclude| !exclude.contains(&id))
                && include.is_none_or(|include| include.contains(&id))
        };
        if source == LootContextParam::BlockEntity {
            let Some(block_entity) = self.context.block_entity() else {
                return Ok(stack);
            };
            let components = block_entity.collect_components();
            for component in components
                .entries()
                .iter()
                .filter(|component| admits(&component.id))
            {
                let value = component.value.as_ref().map_err(|detail| {
                    self.err(
                        path,
                        unsupported_field("copy_components", format!("{}: {detail}", component.id)),
                    )
                })?;
                let Some(component_type) =
                    vanilla_data_component_type_for_resource_id(&component.id)
                else {
                    continue;
                };
                stack = self.apply_copied_component(stack, component_type.type_id, value, path)?;
            }
            return Ok(stack);
        }
        let included: Vec<&str> = match include {
            Some(include) => include.to_vec(),
            None => bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TYPES
                .iter()
                .map(|component| component.resource_id)
                .collect(),
        };
        let included: Vec<&str> = included.into_iter().filter(|id| admits(id)).collect();
        if included.is_empty() {
            return Ok(stack);
        }
        let tool = if source == LootContextParam::Tool {
            let Some(tool) = self.context.tool() else {
                return Ok(stack);
            };
            if tool.is_empty() {
                return Ok(stack);
            }
            Some(&tool.stack)
        } else {
            None
        };
        let entity = if tool.is_none() {
            match self.context.entity(source) {
                LootEntity::Absent => return Ok(stack),
                LootEntity::Opaque => {
                    return Err(self.err(
                        path,
                        unsupported_field("copy_components", "actual entity getter unavailable"),
                    ))
                }
                entity => Some(entity),
            }
        } else {
            None
        };
        for id in included {
            let type_id = vanilla_data_component_type_for_resource_id(id)
                .expect("checked type")
                .type_id;
            let value = if let Some(tool) = tool {
                let environment = self.environment(path)?;
                self.at(path, environment.component_runtime(tool, type_id))?
            } else {
                self.at(path, entity.expect("entity source").component(type_id))?
            };
            if let Some(value) = value {
                stack = self.apply_copied_component(stack, type_id, &value, path)?;
            }
        }
        Ok(stack)
    }

    fn apply_copied_component(
        &self,
        stack: LootStack,
        type_id: i32,
        value: &bbb_protocol::NbtSummaryValue,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        self.at(
            path,
            environment.with_component_runtime(stack, type_id, Some(value)),
        )
        .map(RuntimeItemStack::into_summary)
    }
}
