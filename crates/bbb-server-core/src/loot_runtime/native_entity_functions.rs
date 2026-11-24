use super::*;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID as CUSTOM_NAME,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID as PROFILE,
};
use bbb_protocol::item_types::VANILLA_ITEM_PLAYER_HEAD_ID;

impl Eval<'_> {
    /// Only a `Nameable` source writes, a null name included.
    pub(super) fn apply_copy_name(
        &mut self,
        param: LootContextParam,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let name = if param == LootContextParam::BlockEntity {
            self.context.block_entity().and_then(|source| {
                source
                    .block_entity
                    .nameable_custom_name()
                    .map(|name| name.cloned())
            })
        } else {
            match self.context.entity(param) {
                LootEntity::Absent => None,
                LootEntity::Opaque => {
                    return Err(self.err(
                        path,
                        unsupported_field("copy_name", "source entity has no view"),
                    ));
                }
                entity => Some(entity.custom_name().ok_or_else(|| {
                    self.err(
                        path,
                        unsupported_field("copy_name", "source custom name is unknown"),
                    )
                })?),
            }
        };
        let Some(name) = name else {
            return Ok(stack);
        };
        self.write_component(stack, CUSTOM_NAME, name.as_ref(), path)
    }

    /// Only a player head filled from a `Player` changes.
    pub(super) fn apply_fill_player_head(
        &mut self,
        param: LootContextParam,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        if stack_count(&stack) == 0 || stack.item_id != Some(VANILLA_ITEM_PLAYER_HEAD_ID) {
            return Ok(stack);
        }
        let profile = match self.context.entity(param) {
            LootEntity::Absent => return Ok(stack),
            LootEntity::Opaque => {
                return Err(self.err(
                    path,
                    unsupported_field("fill_player_head", "entity has no view"),
                ));
            }
            entity => entity.player_profile().ok_or_else(|| {
                self.err(
                    path,
                    unsupported_field("fill_player_head", "Player profile is unknown"),
                )
            })?,
        };
        let LootEntityAnswer::Value(profile) = profile else {
            return Ok(stack);
        };
        self.write_component(stack, PROFILE, Some(&profile), path)
    }

    pub(super) fn write_component(
        &mut self,
        stack: LootStack,
        type_id: i32,
        value: Option<&bbb_protocol::NbtSummaryValue>,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        self.at(
            path,
            environment.with_component_runtime(stack, type_id, value),
        )
        .map(RuntimeItemStack::into_summary)
    }
}
