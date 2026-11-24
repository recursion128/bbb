use super::*;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID as CUSTOM_DATA;
use bbb_protocol::NbtSummaryValue;
use bbb_server_datapack::loot::{LootNbtCopy, LootNbtCopyStrategy, LootNbtProvider};

impl Eval<'_> {
    pub(super) fn apply_native_nbt_copy(
        &mut self,
        copy: &LootNbtCopy,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = || self.environment(path);
        let source = match &copy.source {
            LootNbtProvider::Storage(id) => self.at(path, environment()?.nbt_storage(id))?,
            LootNbtProvider::Context(LootContextParam::BlockEntity) => {
                let Some(entity) = self.context.block_entity() else {
                    return Ok(stack);
                };
                self.at(path, environment()?.nbt_block_entity(entity.block_entity))?
            }
            LootNbtProvider::Context(param) => match self.context.entity(*param) {
                LootEntity::Absent => return Ok(stack),
                LootEntity::Opaque => {
                    return Err(self.err(
                        path,
                        LootRuntimeReason::Catalog("NBT entity view is unavailable".into()),
                    ))
                }
                entity => self.at(path, environment()?.nbt_entity(entity))?,
            },
        };
        let mut result = None;
        let mut native = None;
        for operation in &copy.operations {
            // A missing source path skips only this operation.
            let Ok(sources) = operation.source.get(&source) else {
                continue;
            };
            if sources.is_empty() {
                continue;
            }
            if result.is_none() {
                // The held custom data is read on the first source match only.
                let stack = RuntimeItemStack::from_summary(stack.clone());
                let value = self
                    .at(path, environment()?.component_runtime(&stack, CUSTOM_DATA))?
                    .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()));
                if !matches!(value, NbtSummaryValue::Compound(_)) {
                    return Err(self.err(path, malformed("copy_custom_data", "CUSTOM_DATA")));
                }
                result = Some(value);
                native = Some(stack);
            }
            let target = result
                .as_mut()
                .expect("matched source creates the target copy");
            // A target path error leaves earlier writes in place.
            match operation.strategy {
                LootNbtCopyStrategy::Replace => {
                    let _ = operation
                        .target
                        .set(target, sources.last().expect("nonempty").clone());
                }
                LootNbtCopyStrategy::Append => {
                    let _ = operation.target.modify_or_create(
                        target,
                        NbtSummaryValue::List(Vec::new()),
                        |target| {
                            if let NbtSummaryValue::List(target) = target {
                                target.extend(sources.iter().cloned());
                            }
                        },
                    );
                }
                LootNbtCopyStrategy::Merge => {
                    let _ = operation.target.modify_or_create(
                        target,
                        NbtSummaryValue::Compound(Vec::new()),
                        |target| {
                            if let NbtSummaryValue::Compound(target) = target {
                                for source in &sources {
                                    if let NbtSummaryValue::Compound(source) = source {
                                        bbb_protocol::merge_command_nbt_compound(target, source);
                                    }
                                }
                            }
                        },
                    );
                }
            }
        }
        let Some(NbtSummaryValue::Compound(fields)) = result else {
            return Ok(stack);
        };
        let value = (!fields.is_empty()).then_some(NbtSummaryValue::Compound(fields));
        self.at(
            path,
            environment()?.with_component_runtime(
                native.expect("target copy has a Native stack"),
                CUSTOM_DATA,
                value.as_ref(),
            ),
        )
        .map(RuntimeItemStack::into_summary)
    }
}
