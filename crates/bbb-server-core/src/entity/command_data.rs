use crate::world::simulation_level::LevelCtx;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use thiserror::Error;

use crate::entity::save::{
    entity_record_tree_without_id, load_entity_command_record, EntityNbtReadContext,
    EntitySaveError,
};
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// Errors raised while reading or replacing a stored entity's command data.
#[derive(Debug, Error)]
pub enum EntityCommandDataError {
    #[error("stored entity {0} does not exist")]
    UnknownEntity(i32),
    #[error(transparent)]
    Save(#[from] EntitySaveError),
}

impl Entity {
    pub(crate) fn take_loaded_team(&mut self) -> Option<bbb_protocol::JavaString> {
        let EntityData::GeneralLiving(data) = &mut self.data else {
            return None;
        };
        let fields = data.opaque_fields_mut();
        let index = fields.iter().position(|entry| entry.name == "Team")?;
        match fields.swap_remove(index).value {
            NbtSummaryValue::String(team) => Some(team),
            _ => None,
        }
    }
}

impl EntityStore {
    pub fn command_data(&self, entity_id: i32) -> Result<NbtSummaryValue, EntityCommandDataError> {
        let entity = self
            .get(entity_id)
            .ok_or(EntityCommandDataError::UnknownEntity(entity_id))?;
        Ok(entity_record_tree_without_id(self, entity)?)
    }

    /// Loads one complete command-data compound while retaining store identity.
    pub fn load_command_data(
        &mut self,
        entity_id: i32,
        entries: &[NbtSummaryEntry],
    ) -> Result<(), EntityCommandDataError> {
        self.load_command_data_inner(entity_id, entries, None, None)
    }

    #[cfg(test)]
    pub(crate) fn load_command_data_with_level(
        &mut self,
        entity_id: i32,
        entries: &[NbtSummaryEntry],
        level: &LevelCtx<'_, '_>,
    ) -> Result<(), EntityCommandDataError> {
        self.load_command_data_inner(entity_id, entries, Some(level), None)
    }

    pub(crate) fn load_command_data_with_context(
        &mut self,
        entity_id: i32,
        entries: &[NbtSummaryEntry],
        level: Option<&LevelCtx<'_, '_>>,
        context: EntityNbtReadContext<'_>,
    ) -> Result<(), EntityCommandDataError> {
        self.load_command_data_inner(entity_id, entries, level, Some(context))
    }

    fn load_command_data_inner(
        &mut self,
        entity_id: i32,
        entries: &[NbtSummaryEntry],
        level: Option<&LevelCtx<'_, '_>>,
        context: Option<EntityNbtReadContext<'_>>,
    ) -> Result<(), EntityCommandDataError> {
        let publication = self.item_publication();
        {
            let entity = self
                .get(entity_id)
                .ok_or(EntityCommandDataError::UnknownEntity(entity_id))?;
            let game_time = level.map_or(self.game_time(), LevelCtx::game_time);
            let mut loaded = load_entity_command_record(
                self,
                entity,
                entries,
                game_time,
                level,
                context,
                publication.resolver(),
            )?;
            let loaded_anger_target = level
                .filter(|_| {
                    crate::entity::ai::mob::target::anger::is_neutral_mob(loaded.entity_type_id())
                })
                .and_then(|level| {
                    crate::entity::ai::mob::target::sense::prepare_loaded_anger_target(
                        &loaded, self, level,
                    )
                });
            let previous_happy = match entity.data() {
                crate::entity::EntityData::GeneralLiving(data) => data.happy_ghast_state(),
                _ => None,
            };
            let previous_framed_map = crate::entity::hanging::framed_map_removal(entity);
            let loaded_framed_map = crate::entity::hanging::framed_map_removal(&loaded);
            let remove_framed_map = previous_framed_map.filter(|prev| {
                loaded_framed_map
                    .as_ref()
                    .is_none_or(|next| prev.map_id != next.map_id || prev.pos != next.pos)
            });
            if crate::entity::leash::is_leashed(entity)
                && crate::entity::leash::leash_data(&loaded).is_none()
            {
                let mut previous = entity.clone();
                crate::entity::leash::drop_leash_store_spawn(&mut previous, self, true, false);
            }
            if let (Some(previous), crate::entity::EntityData::GeneralLiving(data)) =
                (previous_happy, &mut loaded.data)
            {
                let state = data.happy_ghast_state_mut().unwrap();
                let target = state.still_timeout;
                state.still_timeout = previous.still_timeout;
                state.stays_still = previous.stays_still;
                if let Some(packet) =
                    crate::entity::happy_ghast::set_server_still_timeout(&mut loaded, target)
                {
                    self.broadcast_to_tracking(entity_id, packet);
                }
            }
            *self
                .get_mut(entity_id)
                .expect("the validated entity is still stored") = loaded;
            if let Some(removal) = remove_framed_map {
                self.queue_command(crate::entity::EntityCommand::FramedMapRemoval(removal));
            }
            if self.get(entity_id).is_some_and(|entity| {
                matches!(entity.data(), crate::entity::EntityData::Item(data) if data.item().is_empty())
            }) {
                self.refile(entity_id);
                self.remove(entity_id, crate::entity::RemovalReason::Discarded);
                return Ok(());
            }
            if let Some(runtime) = self.mob_ai_mut(entity_id) {
                runtime.invalidate_brain();
            }
            self.refresh_retained_mob_navigation(entity_id);
            self.refresh_retained_mob_targets(entity_id);
            if let Some(target) = loaded_anger_target {
                target.apply(self, entity_id);
            }
            self.refile(entity_id);
            crate::entity::save::resolve_saved_last_hurt_by_mob_references(self);
            Ok(())
        }
    }
}
