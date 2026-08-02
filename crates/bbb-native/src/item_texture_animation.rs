use std::time::{Duration, Instant};

use bbb_item_model::NativeItemRuntime;
use bbb_renderer::Renderer;

const ITEM_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Default)]
pub(crate) struct ItemTextureAnimationState {
    resource_generation: Option<u64>,
    tick: u64,
    last_advance_at: Option<Instant>,
}

pub(crate) fn maybe_upload_item_texture_animation(
    renderer: &mut Renderer,
    state: &mut ItemTextureAnimationState,
    items: &NativeItemRuntime,
) {
    if !items.has_texture_animation() {
        return;
    }
    let Some((previous_tick, tick)) =
        advance_item_texture_animation_tick(state, items.resource_generation(), Instant::now())
    else {
        return;
    };
    let patches = match items.texture_animation_atlas_patches(previous_tick, tick) {
        Ok(Some(patches)) => patches,
        Ok(None) => return,
        Err(err) => {
            tracing::warn!(?err, "failed to build animated item texture atlas patches");
            return;
        }
    };
    if let Err(err) = renderer.update_hud_item_atlas_patches(&patches) {
        tracing::warn!(?err, "failed to update animated HUD item texture atlas");
    }
    if let Err(err) = renderer.update_item_entity_atlas_patches(&patches) {
        tracing::warn!(
            ?err,
            "failed to update animated model/entity item texture atlas"
        );
    }
}

fn advance_item_texture_animation_tick(
    state: &mut ItemTextureAnimationState,
    resource_generation: u64,
    now: Instant,
) -> Option<(u64, u64)> {
    if state.resource_generation != Some(resource_generation) {
        state.resource_generation = Some(resource_generation);
        state.tick = 0;
        state.last_advance_at = Some(now);
        return None;
    }
    let last = state.last_advance_at.get_or_insert(now);
    let elapsed = now.saturating_duration_since(*last);
    let ticks = elapsed.as_millis() / ITEM_TEXTURE_ANIMATION_INTERVAL.as_millis();
    if ticks == 0 {
        return None;
    }

    let ticks = u64::try_from(ticks).unwrap_or(u64::MAX);
    let previous_tick = state.tick;
    state.tick = state.tick.saturating_add(ticks);
    let advanced = Duration::from_millis(
        ticks.saturating_mul(ITEM_TEXTURE_ANIMATION_INTERVAL.as_millis() as u64),
    );
    state.last_advance_at = last.checked_add(advanced).or(Some(now));
    Some((previous_tick, state.tick))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_texture_animation_tick_uses_client_tick_interval_and_resets_on_reload() {
        let mut state = ItemTextureAnimationState::default();
        let start = Instant::now();

        assert_eq!(
            advance_item_texture_animation_tick(&mut state, 7, start),
            None
        );
        assert_eq!(
            advance_item_texture_animation_tick(&mut state, 7, start + Duration::from_millis(49)),
            None
        );
        assert_eq!(
            advance_item_texture_animation_tick(&mut state, 7, start + Duration::from_millis(50)),
            Some((0, 1))
        );
        assert_eq!(
            advance_item_texture_animation_tick(&mut state, 7, start + Duration::from_millis(199)),
            Some((1, 3))
        );
        assert_eq!(state.tick, 3);

        assert_eq!(
            advance_item_texture_animation_tick(&mut state, 8, start + Duration::from_millis(200)),
            None
        );
        assert_eq!(state.tick, 0);
        assert_eq!(
            advance_item_texture_animation_tick(&mut state, 8, start + Duration::from_millis(250)),
            Some((0, 1))
        );
    }
}
