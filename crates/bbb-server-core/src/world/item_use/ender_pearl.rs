//! `EnderpearlItem.use` (`EnderpearlItem.java:22-41`).

use bbb_protocol::packets::SoundSource;

use crate::entity::spawn_ender_pearl;
use crate::world::{ServerWorld, WorldEvent};

use super::{ItemUseContext, ItemUseResult, ThrowShape, ThrowSound};

/// `EnderpearlItem.use` (`EnderpearlItem.java:22-41`): `SoundEvents.ENDER_PEARL_THROW` as
/// `SoundSource.NEUTRAL`, then `spawnProjectileFromRotation(ThrownEnderpearl::new,
/// ..., 0.0F, 1.5F, 1.0F)` (`:35`), the stat and the consume.
///
/// `ItemStack.use` (`ItemStack.java:378-389`) would then apply the stack's
/// `use_cooldown` component through `applyAfterUseComponentSideEffects`
/// (`:397-411`); this server keeps no `ItemCooldowns`, so that second
/// half is not here -- see `use_item_in_hand`'s cooldown note.
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    world.throw_item_projectile(
        context,
        ThrowSound {
            resource_id: "minecraft:entity.ender_pearl.throw",
            source: SoundSource::Neutral,
        },
        ThrowShape {
            y_offset: 0.0,
            pow: 1.5,
            uncertainty: 1.0,
        },
        spawn_ender_pearl,
        events,
    )
}
