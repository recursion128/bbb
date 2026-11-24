use super::{apply_half_amplitude_leg_swing, apply_head_look};
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Mutable villager model, mirroring vanilla `VillagerModel`/`BabyVillagerModel`. The unified tree is
/// built for the selected `baby` layout with the vanilla child names. `setup_anim` looks the head
/// ([`apply_head_look`] on `head`), applies the unhappy head shake, and swings the legs at the
/// villager-family half amplitude ([`apply_half_amplitude_leg_swing`]). The combined `arms` part defers.
pub(in crate::entity_models) fn villager_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = if baby {
        ModelLayer::VILLAGER_BABY
    } else {
        ModelLayer::VILLAGER
    }
    .bake();
    let render_state = &instance.render_state;
    let head = root.child_mut("head");
    apply_head_look(head, render_state.head_yaw, render_state.head_pitch);
    if render_state.kind.villager().unhappy {
        head.pose.rotation[2] = 0.3 * (0.45 * render_state.age_in_ticks).sin();
        head.pose.rotation[0] = 0.4;
    } else {
        head.pose.rotation[2] = 0.0;
    }
    apply_half_amplitude_leg_swing(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
    root
}

/// Mutable wandering trader model, mirroring vanilla `WanderingTraderRenderer`, which reuses the adult
/// `VillagerModel` layer. The unified tree is built with the vanilla child names (`head`, `body`, the
/// combined `arms`, `right_leg`, `left_leg`). `setup_anim` looks the head ([`apply_head_look`] on
/// `head`) and swings the legs at the villager-family half amplitude
/// ([`apply_half_amplitude_leg_swing`]). The combined `arms` part is also used by
/// `villager_crossed_arms_item_transform` for `CrossedArmsItemLayer`.
pub(in crate::entity_models) fn wandering_trader_model(
    instance: &EntityModelInstance,
) -> BakedRoot {
    let mut root = ModelLayer::WANDERING_TRADER.bake();
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    apply_half_amplitude_leg_swing(
        &mut root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
    );
    root
}
