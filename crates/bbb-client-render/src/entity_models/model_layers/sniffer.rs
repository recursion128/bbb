use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::keyframe::{animate, animate_walk, AnimationDefinition};

use crate::entity_models::model_data_26_1::sniffer_animation;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// `Sniffer.State` ordinals (the synced `DATA_STATE` VarInt) whose one-shot keyframe the renderer
/// applies, matching the projected `sniffer_animation_id`. `IDLING(0)` and `SEARCHING(4)` carry no
/// one-shot here.
const SNIFFER_STATE_FEELING_HAPPY_ID: i32 = 1;
const SNIFFER_STATE_SCENTING_ID: i32 = 2;
const SNIFFER_STATE_SNIFFING_ID: i32 = 3;
const SNIFFER_STATE_DIGGING_ID: i32 = 5;
const SNIFFER_STATE_RISING_ID: i32 = 6;

/// Vanilla `SnifferModel`; `SnifferRenderer` feeds it the baby layer too.
pub(in crate::entity_models) fn sniffer_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::SNIFFER_BABY
    } else {
        ModelLayer::SNIFFER
    }
    .bake();
    let state = &instance.render_state;
    let head = root.part_mut("head").unwrap();
    head.pose.rotation[0] = state.head_pitch.to_radians();
    head.pose.rotation[1] = state.head_yaw.to_radians();
    let walk = if state.kind.sniffer().is_searching {
        &sniffer_animation::SNIFFER_SNIFF_SEARCH
    } else {
        &sniffer_animation::SNIFFER_WALK
    };
    animate_walk(
        &mut root,
        walk,
        state.walk_animation_pos,
        state.walk_animation_speed,
        9.0,
        100.0,
    );
    if let Some(definition) = sniffer_state_animation(state.kind.sniffer().animation_id) {
        animate(
            &mut root,
            definition,
            state.kind.sniffer().animation_seconds,
            1.0,
        );
    }
    root
}

/// Maps the projected `sniffer_animation_id` (the active `Sniffer.State` ordinal, or `-1` for none)
/// to the one-shot keyframe def `SnifferModel.setupAnim` applies. `None` for `-1` (idling/searching)
/// and any unknown id, so no one-shot is layered onto the walk.
fn sniffer_state_animation(animation_id: i32) -> Option<&'static AnimationDefinition> {
    match animation_id {
        SNIFFER_STATE_FEELING_HAPPY_ID => Some(&sniffer_animation::SNIFFER_HAPPY),
        SNIFFER_STATE_SCENTING_ID => Some(&sniffer_animation::SNIFFER_SNIFFSNIFF),
        SNIFFER_STATE_SNIFFING_ID => Some(&sniffer_animation::SNIFFER_LONGSNIFF),
        SNIFFER_STATE_DIGGING_ID => Some(&sniffer_animation::SNIFFER_DIG),
        SNIFFER_STATE_RISING_ID => Some(&sniffer_animation::SNIFFER_STAND_UP),
        _ => None,
    }
}
