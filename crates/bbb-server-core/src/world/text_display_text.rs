//! A text display's `text` resolved as it loads (`Display.java:773-786`).

use bbb_protocol::RichTextComponent;

use super::{LevelId, ServerWorld};
use crate::commands::types::CommandDamageEntityRef;
use crate::commands::CommandStoredEntity;
use crate::entity::EntityData;

impl ServerWorld {
    /// `TextDisplay.readAdditionalSaveData` resolves the loaded text against
    /// the display itself; a failure keeps `previous` (`Display.java:783-785`).
    pub(in crate::world) fn resolve_text_display_text(
        &mut self,
        level: LevelId,
        entity_id: i32,
        previous: Option<RichTextComponent>,
    ) {
        let resolved = {
            let reads = self.entity_reads();
            let Some(store) = reads
                .levels
                .iter()
                .find(|read| read.key == self.level_key(level))
            else {
                return;
            };
            let Some(entity) = store.entities.get(entity_id) else {
                return;
            };
            let EntityData::TextDisplay(data) = entity.data() else {
                return;
            };
            let entity_ref = CommandDamageEntityRef::Stored(CommandStoredEntity {
                entity,
                dimension: store.key,
                registry_tags: store.tags,
            });
            crate::commands::resolve_entity_text(
                data.text().component(),
                entity_ref,
                store.key,
                &reads,
            )
            .ok()
            .and_then(|value| {
                RichTextComponent::try_from_component_nbt_with_registry(
                    value,
                    reads.content_registry(),
                )
                .ok()
            })
        };
        let Some(text) = resolved.or(previous) else {
            return;
        };
        if let Some(entity) = self
            .levels
            .get_mut(&level)
            .and_then(|level| level.entities.get_mut(entity_id))
        {
            crate::entity::display::set_text_display_text(entity, text);
        }
    }
}
