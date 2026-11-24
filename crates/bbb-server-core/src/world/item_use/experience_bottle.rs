//! `ExperienceBottleItem.use` (`ExperienceBottleItem.java:22-41`).

use bbb_protocol::packets::SoundSource;

use crate::entity::spawn_experience_bottle;
use crate::world::{ServerWorld, WorldEvent};

use super::{ItemUseContext, ItemUseResult, ThrowShape, ThrowSound};

/// `ExperienceBottleItem.use` (`ExperienceBottleItem.java:22-41`): `SoundEvents.EXPERIENCE_BOTTLE_THROW` as
/// `SoundSource.NEUTRAL`, then `spawnProjectileFromRotation(ThrownExperienceBottle::new,
/// ..., -20.0F, 0.7F, 1.0F)` (`:35`), the stat and the consume.
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    world.throw_item_projectile(
        context,
        ThrowSound {
            resource_id: "minecraft:entity.experience_bottle.throw",
            source: SoundSource::Neutral,
        },
        ThrowShape {
            y_offset: -20.0,
            pow: 0.7,
            uncertainty: 1.0,
        },
        spawn_experience_bottle,
        events,
    )
}
