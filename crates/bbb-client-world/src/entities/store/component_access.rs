use super::*;

impl EntityStore {
    #[cfg(test)]
    pub(crate) fn metadata(&self, id: i32) -> Option<EntityMetadata> {
        self.record(id).map(|entity| entity.metadata.clone())
    }

    #[cfg(test)]
    pub(crate) fn equipment(&self, id: i32) -> Option<EntityEquipment> {
        self.record(id).map(|entity| entity.equipment.clone())
    }

    #[cfg(test)]
    pub(crate) fn attributes(&self, id: i32) -> Option<EntityAttributes> {
        self.record(id).map(|entity| entity.attributes.clone())
    }

    #[cfg(test)]
    pub(crate) fn transient_events(&self, id: i32) -> Option<EntityTransientEvents> {
        self.record(id).map(|entity| entity.events)
    }

    #[cfg(test)]
    pub(crate) fn experience_orb_movement(
        &self,
        id: i32,
    ) -> Option<super::super::record::EntityExperienceOrbMovement> {
        self.record(id)?.family.experience_orb().copied()
    }

    #[cfg(test)]
    pub(crate) fn fishing_hook_movement(
        &self,
        id: i32,
    ) -> Option<super::super::record::EntityFishingHookMovement> {
        self.record(id)?.family.fishing_hook().copied()
    }

    #[cfg(test)]
    pub(crate) fn abstract_arrow_runtime(&self, id: i32) -> Option<EntityAbstractArrowRuntime> {
        self.record(id)?.family.abstract_arrow().cloned()
    }

    pub(crate) fn with_transform_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransform) -> R,
    ) -> Option<R> {
        self.record_mut(id)
            .map(|entity| update(&mut entity.transform))
    }

    pub(crate) fn boat_movement(&self, id: i32) -> Option<EntityBoatMovement> {
        self.record(id)?.family.boat().copied()
    }

    pub(crate) fn with_boat_movement_transform_and_support_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(
            &mut EntityBoatMovement,
            &mut EntityTransform,
            &mut EntitySupportingBlock,
        ) -> R,
    ) -> Option<R> {
        let entity = self.record_mut(id)?;
        Some(update(
            entity.family.boat_mut()?,
            &mut entity.transform,
            &mut entity.supporting_block,
        ))
    }

    pub(crate) fn with_transform_and_support_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransform, &mut EntitySupportingBlock) -> R,
    ) -> Option<R> {
        let entity = self.record_mut(id)?;
        Some(update(&mut entity.transform, &mut entity.supporting_block))
    }

    pub(super) fn with_client_animations_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityClientAnimationState) -> R,
    ) -> Option<R> {
        self.record_mut(id)
            .map(|entity| update(&mut entity.animations))
    }

    pub(crate) fn with_metadata_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMetadata) -> R,
    ) -> Option<R> {
        self.record_mut(id)
            .map(|entity| update(&mut entity.metadata))
    }

    pub(crate) fn with_equipment_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityEquipment) -> R,
    ) -> Option<R> {
        self.record_mut(id)
            .map(|entity| update(&mut entity.equipment))
    }

    pub(crate) fn with_attributes_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityAttributes) -> R,
    ) -> Option<R> {
        self.record_mut(id)
            .map(|entity| update(&mut entity.attributes))
    }

    pub(crate) fn with_transient_events_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransientEvents) -> R,
    ) -> Option<R> {
        self.record_mut(id).map(|entity| update(&mut entity.events))
    }

    pub(crate) fn with_mount_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMount) -> R,
    ) -> Option<R> {
        self.record_mut(id).map(|entity| update(&mut entity.mount))
    }

    pub(crate) fn with_leash_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityLeash) -> R,
    ) -> Option<R> {
        self.record_mut(id).map(|entity| update(&mut entity.leash))
    }

    pub(crate) fn with_mob_effects_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMobEffects) -> R,
    ) -> Option<R> {
        self.record_mut(id)
            .map(|entity| update(&mut entity.effects))
    }

    pub(crate) fn supporting_block(&self, id: i32) -> Option<EntitySupportingBlock> {
        self.record(id).map(|entity| entity.supporting_block)
    }

    pub(crate) fn with_supporting_block_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntitySupportingBlock) -> R,
    ) -> Option<R> {
        self.record_mut(id)
            .map(|entity| update(&mut entity.supporting_block))
    }

    pub(crate) fn with_minecart_lerp_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMinecartLerp) -> R,
    ) -> Option<R> {
        self.record_mut(id)?.family.minecart_lerp_mut().map(update)
    }

    pub(crate) fn with_hurting_projectile_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut super::super::HurtingProjectileState) -> R,
    ) -> Option<R> {
        self.record_mut(id)?
            .family
            .hurting_projectile_mut()
            .map(update)
    }
}
