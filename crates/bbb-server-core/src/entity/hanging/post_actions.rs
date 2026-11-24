use super::*;
use crate::level_event::LevelEventSink;
use crate::world::simulation_level::LevelCtx;

pub(crate) struct ResolvedHangingPostActions(Vec<ResolvedAction>);

impl ResolvedHangingPostActions {
    pub(super) fn empty() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

enum ResolvedAction {
    Sound {
        position: Vec3d,
        resource_id: &'static str,
    },
    Drop {
        position: Vec3d,
        stack: RuntimeItemStack,
    },
    BlockChange(BlockPos),
    RemoveFramedMap(crate::entity::store::FramedMapRemoval),
    UpdateOutputSignal(BlockPos),
}

pub(crate) fn hurt_server_with_post_actions(
    entity: &mut Entity,
    store: &mut EntityStore,
    entity_drops: bool,
    source: HangHurtSource,
) -> Option<ResolvedHangingHurtEffects> {
    let effects = hurt_server(entity, entity_drops, source);
    let actions = resolve(store, effects.actions)?;
    Some(ResolvedHangingHurtEffects {
        outcome: effects.outcome,
        actions,
        emits_entity_die: effects.emits_entity_die,
    })
}

pub(super) fn resolve(
    store: &mut EntityStore,
    actions: Vec<HangingPostAction>,
) -> Option<ResolvedHangingPostActions> {
    let mut resolved = Vec::with_capacity(actions.len());
    for action in actions {
        resolved.push(match action {
            HangingPostAction::Sound {
                position,
                resource_id,
            } => ResolvedAction::Sound {
                position,
                resource_id,
            },
            HangingPostAction::Drop { position, stack } => ResolvedAction::Drop { position, stack },
            HangingPostAction::DropWithChance {
                position,
                stack,
                chance,
            } => {
                if store.random().next_float() < chance {
                    ResolvedAction::Drop { position, stack }
                } else {
                    continue;
                }
            }
            HangingPostAction::BlockChange(pos) => ResolvedAction::BlockChange(pos),
            HangingPostAction::RemoveFramedMap(removal) => ResolvedAction::RemoveFramedMap(removal),
            HangingPostAction::UpdateOutputSignal(pos) => ResolvedAction::UpdateOutputSignal(pos),
        });
    }
    Some(ResolvedHangingPostActions(resolved))
}

enum PostActionSink<'a, 'l, 'w> {
    EventSink(&'a mut LevelEventSink),
    Level(&'a mut LevelCtx<'l, 'w>),
}

pub(crate) fn apply_hanging_post_actions(
    actions: ResolvedHangingPostActions,
    store: &mut EntityStore,
    event_sink: &mut LevelEventSink,
    game_event_source_id: Option<i32>,
) {
    execute(
        actions,
        store,
        PostActionSink::EventSink(event_sink),
        game_event_source_id,
    );
}

pub(crate) fn apply_hanging_post_actions_in_level(
    actions: ResolvedHangingPostActions,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    game_event_source_id: Option<i32>,
) {
    execute(
        actions,
        store,
        PostActionSink::Level(level),
        game_event_source_id,
    );
}

fn execute(
    actions: ResolvedHangingPostActions,
    store: &mut EntityStore,
    mut target: PostActionSink<'_, '_, '_>,
    game_event_source_id: Option<i32>,
) {
    for action in actions.0 {
        match action {
            ResolvedAction::Sound {
                position,
                resource_id,
            } => store.play_entity_sound(PendingEntitySound {
                position,
                resource_id: resource_id.to_owned(),
                source: SoundSource::Neutral,
                volume: 1.0,
                pitch: 1.0,
            }),
            ResolvedAction::Drop { position, stack } => {
                spawn_dropped_item_entity(store, position, stack);
            }
            ResolvedAction::BlockChange(pos) => match &mut target {
                PostActionSink::EventSink(sink) => sink.game_event_with_source(
                    VanillaGameEvent::BLOCK_CHANGE,
                    pos,
                    game_event_source_id,
                    None,
                ),
                PostActionSink::Level(level) => {
                    level.game_event(VanillaGameEvent::BLOCK_CHANGE, pos, game_event_source_id)
                }
            },
            ResolvedAction::RemoveFramedMap(removal) => {
                store.queue_command(crate::entity::EntityCommand::FramedMapRemoval(removal))
            }
            ResolvedAction::UpdateOutputSignal(pos) => match &mut target {
                PostActionSink::EventSink(_) => {
                    store.queue_command(crate::entity::EntityCommand::UpdateOutputSignal(pos));
                }
                PostActionSink::Level(level) => {
                    level.update_neighbour_for_output_signal(
                        pos,
                        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
                    );
                }
            },
        }
    }
}

#[cfg(test)]
mod tests;
