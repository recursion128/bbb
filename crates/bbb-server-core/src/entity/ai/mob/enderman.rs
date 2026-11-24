use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;

use super::MobAiContext;
use crate::entity::living::random_teleport::block_position;
use crate::entity::{store::EntityStore, Entity};

pub(super) fn custom_step(
    context: &mut MobAiContext,
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    if context.tick_count < context.target.target_change_time.wrapping_add(600) {
        return Some(());
    }
    if store.target_level_facts().bright_outside != Some(true) {
        return Some(());
    }
    let pos = body.block_position();
    let brightness = if level.has_ticking_column_at(pos) {
        crate::entity::level::light_level_dependent_magic_value(
            level,
            block_position(Vec3d {
                y: context.body.eye_y,
                ..body.position()
            }),
        )?
    } else {
        0.0
    };
    if brightness > 0.5
        && level.can_see_sky(pos)
        && context.random.next_float() * 30.0 < (brightness - 0.4) * 2.0
    {
        {
            let sensing = super::sensing::Sensing::default();
            let view = super::target::TargetView::new(body, store, level, &sensing);
            context.set_target(None, &view);
        }
        crate::entity::enderman::random_teleport(body, store, level)?;
        context.body.position = body.position();
        context.body.bounding_box = body.bounding_box();
        context.body.eye_y = body.position().y + f64::from(body.eye_height());
    }
    Some(())
}

pub(super) mod blocks;
pub(super) mod look_for_player;
pub(super) mod teleport;
