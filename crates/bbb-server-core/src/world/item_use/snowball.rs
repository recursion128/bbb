//! `SnowballItem.use` (`SnowballItem.java:24-43`).

use bbb_protocol::packets::SoundSource;

use crate::entity::spawn_snowball;
use crate::world::{ServerWorld, WorldEvent};

use super::{ItemUseContext, ItemUseResult, ThrowShape, ThrowSound};

/// `SnowballItem.use` (`SnowballItem.java:24-43`): `SoundEvents.SNOWBALL_THROW` as
/// `SoundSource.NEUTRAL`, then `spawnProjectileFromRotation(Snowball::new,
/// ..., 0.0F, 1.5F, 1.0F)` (`:37`), the stat and the consume.
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    world.throw_item_projectile(
        context,
        ThrowSound {
            resource_id: "minecraft:entity.snowball.throw",
            source: SoundSource::Neutral,
        },
        ThrowShape {
            y_offset: 0.0,
            pow: 1.5,
            uncertainty: 1.0,
        },
        spawn_snowball,
        events,
    )
}
