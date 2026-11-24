//! `CopperGolemStatueBlockEntity` (`CopperGolemStatueBlockEntity.java:16-54`).
//!
//! Pose lives on the block state. Custom name rides the shared component map.

/// `CopperGolemStatueBlockEntity` (`:16-19`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CopperGolemStatueBlockEntity {}

impl super::BlockEntity {
    pub(crate) fn copper_golem_statue_custom_name(
        &self,
    ) -> Option<Option<bbb_protocol::component::RichTextComponent>> {
        if !matches!(
            self.payload,
            super::BlockEntityPayload::CopperGolemStatue(_)
        ) {
            return None;
        }
        Some(
            self.components
                .iter()
                .find(|component| component.name == "minecraft:custom_name")
                .and_then(|component| {
                    bbb_protocol::component::RichTextComponent::try_from_component_nbt(
                        component.value.clone(),
                    )
                    .ok()
                }),
        )
    }

    /// `CopperGolemStatueBlockEntity.createStatue` (`CopperGolemStatueBlockEntity.java:21-23`).
    pub(crate) fn create_copper_golem_statue(
        &mut self,
        custom_name: Option<&bbb_protocol::component::RichTextComponent>,
    ) -> bool {
        if !matches!(
            self.payload,
            super::BlockEntityPayload::CopperGolemStatue(_)
        ) {
            return false;
        }
        self.components
            .retain(|component| component.name != "minecraft:custom_name");
        if let Some(name) = custom_name {
            self.components.push(bbb_protocol::NbtSummaryEntry {
                name: "minecraft:custom_name".into(),
                value: name.component().clone(),
            });
        }
        true
    }
}
