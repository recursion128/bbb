//! The clientbound projection of a `LivingEntity`'s effect changes.
//!
//! `LivingEntity.onEffectAdded`, `onEffectUpdated` and `onEffectsRemoved`
//! (`LivingEntity.java:1054-1100`) each end in `sendEffectToPassengers`
//! (`:1062-1068`), which hands every `ServerPlayer` riding the entity a
//! `ClientboundUpdateMobEffectPacket` or `ClientboundRemoveMobEffectPacket`.
//! This module builds those two payloads; delivering them needs a connection,
//! which lives outside this crate.

use bbb_protocol::packets::{MobEffectFlags, RemoveMobEffect, UpdateMobEffect};

use super::ActiveMobEffect;

/// `ClientboundUpdateMobEffectPacket`'s flag bits
/// (`ClientboundUpdateMobEffectPacket.java:15-18`).
const FLAG_AMBIENT: u8 = 1;
const FLAG_VISIBLE: u8 = 2;
const FLAG_SHOW_ICON: u8 = 4;
const FLAG_BLEND: u8 = 8;

/// `new ClientboundUpdateMobEffectPacket(entityId, effect, blend)`
/// (`ClientboundUpdateMobEffectPacket.java:25-48`).
///
/// `blend` is true only where `ServerPlayer.onEffectAdded` sends a session its
/// own newly added effect (`ServerPlayer.java:1636-1645`); every send this
/// crate can reach is `sendEffectToPassengers`' `false`
/// (`LivingEntity.java:1062-1068`).
pub fn update_mob_effect_packet(
    entity_id: i32,
    effect: &ActiveMobEffect,
    blend: bool,
) -> UpdateMobEffect {
    let ambient = effect.ambient();
    let visible = effect.show_particles();
    let show_icon = effect.show_icon();
    let mut raw = 0_u8;
    if ambient {
        raw |= FLAG_AMBIENT;
    }
    if visible {
        raw |= FLAG_VISIBLE;
    }
    if show_icon {
        raw |= FLAG_SHOW_ICON;
    }
    if blend {
        raw |= FLAG_BLEND;
    }
    UpdateMobEffect {
        entity_id,
        effect_id: effect.effect_id,
        amplifier: effect.amplifier(),
        duration_ticks: effect.duration(),
        flags: MobEffectFlags {
            raw,
            ambient,
            visible,
            show_icon,
            blend,
        },
    }
}

/// `new ClientboundRemoveMobEffectPacket(this.getId(), effect.getEffect())`
/// (`LivingEntity.java:1091-1095`, `ClientboundRemoveMobEffectPacket.java:14-21`).
pub fn remove_mob_effect_packet(entity_id: i32, effect: &ActiveMobEffect) -> RemoveMobEffect {
    RemoveMobEffect {
        entity_id,
        effect_id: effect.effect_id,
    }
}

#[cfg(test)]
mod tests;
