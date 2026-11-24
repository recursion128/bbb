//! `SplashPotionItem.use` / `LingeringPotionItem.use` and the
//! `ThrowablePotionItem.use` they defer to (`ThrowablePotionItem.java:23-32`).

use bbb_protocol::packets::SoundSource;

use super::{ItemUseContext, ItemUseResult, ThrowShape, ThrowSound, ThrowSpawner};
use crate::world::{ServerWorld, WorldEvent};

fn throw(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    sound: ThrowSound,
    spawn: ThrowSpawner,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    // `spawnProjectileFromRotation(..., -20.0F, 0.5F, 1.0F)` (`:26`).
    let shape = ThrowShape {
        y_offset: -20.0,
        pow: 0.5,
        uncertainty: 1.0,
    };
    world.throw_item_projectile(context, sound, shape, spawn, events)
}

/// `SplashPotionItem.use` (`SplashPotionItem.java:21-33`).
pub(super) mod splash_potion {
    use super::*;

    pub(in crate::world::item_use) fn use_item(
        world: &mut ServerWorld,
        context: &ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> ItemUseResult {
        let sound = ThrowSound {
            resource_id: "minecraft:entity.splash_potion.throw",
            source: SoundSource::Players,
        };
        throw(
            world,
            context,
            sound,
            crate::entity::spawn_splash_potion,
            events,
        )
    }
}

/// `LingeringPotionItem.use` (`LingeringPotionItem.java:21-33`).
pub(super) mod lingering_potion {
    use super::*;

    pub(in crate::world::item_use) fn use_item(
        world: &mut ServerWorld,
        context: &ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> ItemUseResult {
        let sound = ThrowSound {
            resource_id: "minecraft:entity.lingering_potion.throw",
            source: SoundSource::Neutral,
        };
        throw(
            world,
            context,
            sound,
            crate::entity::spawn_lingering_potion,
            events,
        )
    }
}
