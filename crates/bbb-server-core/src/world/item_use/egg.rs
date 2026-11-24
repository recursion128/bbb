//! `EggItem.use` (`EggItem.java:24-43`).

use bbb_protocol::packets::SoundSource;

use crate::entity::spawn_egg;
use crate::world::{ServerWorld, WorldEvent};

use super::{ItemUseContext, ItemUseResult, ThrowShape, ThrowSound};

/// `EggItem.use` (`EggItem.java:24-43`): `SoundEvents.EGG_THROW` as
/// `SoundSource.PLAYERS`, then `spawnProjectileFromRotation(ThrownEgg::new,
/// ..., 0.0F, 1.5F, 1.0F)` (`:37`), the stat and the consume.
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    world.throw_item_projectile(
        context,
        ThrowSound {
            resource_id: "minecraft:entity.egg.throw",
            source: SoundSource::Players,
        },
        ThrowShape {
            y_offset: 0.0,
            pow: 1.5,
            uncertainty: 1.0,
        },
        spawn_egg,
        events,
    )
}
