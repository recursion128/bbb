//! Dead-target celebration and its transient dancing presentation.

use bbb_protocol::entity_synched_data::VANILLA_PIGLIN_DANCING_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID};

use crate::entity::ai::mob::target::TargetView;
use crate::entity::ai::MemoryModuleType;
use crate::entity::{Entity, EntityData, EntityStore, PendingEntitySound};
use crate::world::simulation_level::LevelCtx;

use super::{baby, celebration, dont_hunt, erase, is_admiring, is_avoiding, CelebrationState};

// RememberIfHoglinWasKilled.java:13-19; StartCelebratingIfTargetDead.java:19-35.
pub(super) fn before_ai(body: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    if body.entity_type_id() != VANILLA_ENTITY_TYPE_PIGLIN_ID {
        return;
    }
    let Some(target) = store
        .mob_ai(body.id())
        .and_then(|runtime| runtime.target_ref())
    else {
        return;
    };
    let (hoglin, location, forget_target) = {
        let sensing = Default::default();
        let view = TargetView::new(body, store, level, &sensing)
            .with_detached(level.detached_entity(target.entity_id));
        let Some(target) = view
            .facts_of(target)
            .filter(|target| target.dead_or_dying())
        else {
            return;
        };
        (
            target.entity_type_id() == VANILLA_ENTITY_TYPE_HOGLIN_ID,
            target.block_position(),
            !target.is_player() || store.target_level_facts().forgive_dead_players,
        )
    };
    if hoglin {
        dont_hunt(body, store.random());
    }
    if celebration(body).is_some() {
        return;
    }
    // PiglinAi.java:148; PiglinAi.java:458-460.
    set(
        body,
        Some(CelebrationState {
            location,
            ticks: 300,
            dancing: hoglin && store.random().next_float() < 0.1,
        }),
    );
    if forget_target {
        erase(body, MemoryModuleType::AngryAt);
        if let Some(runtime) = store.mob_ai_mut(body.id()) {
            let target = runtime.target_state_mut();
            target.target = None;
            target.retaliation_ticks = None;
            runtime.stop_navigation();
        }
    }
    // PiglinAi.java:628-629.
    if !is_admiring(body)
        && !is_avoiding(body, store)
        && store
            .mob_ai(body.id())
            .is_some_and(|runtime| !runtime.has_target())
        && body.is_alive_with_health()
        && !body.is_silent()
    {
        let pitch = crate::entity::living::voice_pitch(store, baby(body));
        store.play_entity_sound(PendingEntitySound {
            position: body.position(),
            resource_id: "minecraft:entity.piglin.celebrate".to_owned(),
            source: crate::entity::living::living_sound_source(body),
            volume: 1.0,
            pitch,
        });
    }
}

pub(super) fn tick(body: &mut Entity) {
    let Some(mut state) = celebration(body) else {
        return;
    };
    state.ticks = state.ticks.saturating_sub(1);
    set(body, (state.ticks > 0).then_some(state));
}

pub(super) fn clear(body: &mut Entity) {
    set(body, None);
}

fn set(body: &mut Entity, celebration: Option<CelebrationState>) {
    let EntityData::GeneralLiving(data) = &mut body.data else {
        return;
    };
    let Some(current) = data.piglin_celebration_mut() else {
        return;
    };
    let was_dancing = current.is_some_and(|state| state.dancing);
    *current = celebration;
    if was_dancing != celebration.is_some_and(|state| state.dancing) {
        body.mark_synched_dirty(VANILLA_PIGLIN_DANCING_DATA_ID);
    }
}
