use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `EvokerFangsModel`.
pub(in crate::entity_models) fn evoker_fangs_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::EVOKER_FANGS.bake();
    use std::f32::consts::PI;

    // Vanilla `EvokerFangsRenderer` skips the whole model while `biteProgress == 0`
    // (the fang is still underground); hide the subtree to match exactly.
    let bite_progress = instance.render_state.kind.evoker_fangs().bite_progress;
    root.visible = bite_progress != 0.0;
    if !root.visible {
        return root;
    }

    // Vanilla `EvokerFangsModel.setupAnim`: a cubic ease-out `biteAmount` snaps the
    // jaws shut over the first half of the bite (`biteAmount` runs `1 → 0` as
    // `biteProgress` runs `0 → 0.5`), `base.y` lifts the fang out of the ground, and
    // a `preScale` shrinks the whole model to nothing over the final 10%.
    let mut bite_amount = (bite_progress * 2.0).min(1.0);
    bite_amount = 1.0 - bite_amount * bite_amount * bite_amount;
    let base = root.child_mut("base");
    base.child_mut("upper_jaw").pose.rotation[2] = PI - bite_amount * 0.35 * PI;
    base.child_mut("lower_jaw").pose.rotation[2] = PI + bite_amount * 0.35 * PI;
    base.pose.offset[1] -= (bite_progress + (bite_progress * 2.7).sin()) * 7.2;

    let pre_scale = if bite_progress > 0.9 {
        (1.0 - bite_progress) / 0.1
    } else {
        1.0
    };
    root.pose.offset[1] = 24.0 - 20.0 * pre_scale;
    root.scale = [pre_scale; 3];
    root
}
