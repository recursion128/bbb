//! `DATA_EFFECT_PARTICLES` and `DATA_EFFECT_AMBIENCE_ID`
//! (`LivingEntity.java:180-183`) as the `SynchedEntityData` of a body the store
//! does not hold keeps them, each with its own dirty bit.

use bbb_protocol::packets::ParticlePayload;
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID;

use super::ActiveMobEffects;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SynchedEffectParticles {
    particles: Vec<ParticlePayload>,
    ambience: bool,
    particles_dirty: bool,
    ambience_dirty: bool,
}

impl ActiveMobEffects {
    /// The list `updateSynchronizedMobEffectParticles` builds
    /// (`LivingEntity.java:881-886`).
    #[must_use]
    pub fn visible_particles(&self) -> Vec<ParticlePayload> {
        self.iter()
            .filter(|effect| effect.show_particles)
            .map(super::ActiveMobEffect::particle)
            .collect()
    }

    /// `LivingEntity.areAllEffectsAmbient` (`LivingEntity.java:939-947`).
    #[must_use]
    pub fn are_all_ambient(&self) -> bool {
        self.iter()
            .all(|effect| !effect.show_particles || effect.ambient)
    }
}

/// `SynchedEntityData.set`'s `ObjectUtils.notEqual`
/// (`SynchedEntityData.java:60-68`) for `DATA_EFFECT_PARTICLES`.
/// `ColorParticleOption` keeps `Object.equals` (`ColorParticleOption.java:10-55`),
/// so a list holding a freshly built `entity_effect` option never equals the one
/// it replaces.
pub(crate) fn particles_write_marks_dirty(
    current: &[ParticlePayload],
    next: &[ParticlePayload],
) -> bool {
    next.iter()
        .any(|particle| particle.particle_type_id == VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID)
        || next != current
}

impl SynchedEffectParticles {
    /// `removeEffectParticles` (`LivingEntity.java:949-951`).
    pub fn remove(&mut self) {
        self.set_particles(Vec::new());
    }

    /// `updateSynchronizedMobEffectParticles` (`LivingEntity.java:880-889`).
    pub fn update(&mut self, effects: &ActiveMobEffects) {
        self.set_particles(effects.visible_particles());
        let ambience = effects.are_all_ambient();
        if ambience != self.ambience {
            self.ambience = ambience;
            self.ambience_dirty = true;
        }
    }

    fn set_particles(&mut self, particles: Vec<ParticlePayload>) {
        if particles_write_marks_dirty(&self.particles, &particles) {
            self.particles = particles;
            self.particles_dirty = true;
        }
    }

    #[must_use]
    pub fn particles(&self) -> &[ParticlePayload] {
        &self.particles
    }

    #[must_use]
    pub const fn ambience(&self) -> bool {
        self.ambience
    }

    /// `SynchedEntityData.packDirty`'s read-and-clear
    /// (`SynchedEntityData.java:74-90`) of the two items, as
    /// `(particles, ambience)`.
    pub fn take_dirty(&mut self) -> (bool, bool) {
        (
            std::mem::take(&mut self.particles_dirty),
            std::mem::take(&mut self.ambience_dirty),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ActiveMobEffect, MobEffectBody};
    use super::*;
    use crate::entity::attributes::{AttributeMap, AttributeSupplier};

    fn holding(effects: &[ActiveMobEffect]) -> ActiveMobEffects {
        let mut attributes = AttributeMap::new(
            AttributeSupplier::for_entity_type("minecraft:player").expect("a player row"),
        );
        let (mut health, mut absorption_amount) = (20.0, 0.0);
        let mut body = MobEffectBody {
            attributes: &mut attributes,
            health: &mut health,
            absorption_amount: &mut absorption_amount,
            food: None,
            abilities_invulnerable: false,
        };
        let mut map = ActiveMobEffects::default();
        for effect in effects {
            map.add(&mut body, effect.clone())
                .expect("a vanilla effect");
        }
        map
    }

    fn speed(ambient: bool, visible: bool) -> ActiveMobEffect {
        ActiveMobEffect::with_visibility("minecraft:speed", 200, 0, ambient, visible, true)
            .expect("speed is a vanilla effect")
    }

    #[test]
    fn ambience_is_vacuously_true_and_survives_the_particle_removal() {
        let hidden = holding(&[speed(false, false)]);
        assert!(hidden.visible_particles().is_empty());
        assert!(hidden.are_all_ambient(), "no visible effect is non-ambient");

        let mut rows = SynchedEffectParticles::default();
        rows.update(&holding(&[speed(true, true)]));
        assert!(rows.ambience());
        assert_eq!(rows.take_dirty(), (true, true));

        rows.remove();
        assert!(rows.particles().is_empty());
        assert!(
            rows.ambience(),
            "`removeEffectParticles` writes only the particle item"
        );
        assert_eq!(rows.take_dirty(), (true, false));
    }
}
