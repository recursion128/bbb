use super::*;

impl WorldStore {
    pub(super) fn predict_local_trident_item_use(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> LocalItemUsePrediction {
        if self.item_stack_next_damage_will_break(stack) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
        }
        let enchantments = if stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
        {
            &[][..]
        } else {
            stack.component_patch.enchantments.as_slice()
        };
        if self.trident_spin_attack_strength(enchantments) > 0.0
            && !self.local_player_is_in_water_or_rain()
        {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
        }
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(true),
        ))
    }

    fn local_player_is_in_water_or_rain(&self) -> bool {
        let Some(pose) = self.local_player_pose() else {
            return false;
        };
        if crate::client::local_player_fluid::local_player_fluid_contact(self, pose).in_water() {
            return true;
        }
        let block_effect_world = crate::client::EntityBlockEffectWorld::new(
            self.dimension,
            &self.registries,
            &self.chunks,
            self.entity_block_effect_weather(),
        );
        block_effect_world.is_in_rain(
            crate::EntityVec3 {
                x: pose.position.x,
                y: pose.position.y,
                z: pose.position.z,
            },
            pose.body_height(),
        )
    }
}
