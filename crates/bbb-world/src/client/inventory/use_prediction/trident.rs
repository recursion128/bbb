use super::*;
use bbb_protocol::item_types::VANILLA_ITEM_TRIDENT_ID;

use crate::client::local_player_movement::move_local_player_self_upward;

const VANILLA_TRIDENT_THROW_THRESHOLD_TICKS: u32 = 10;
const VANILLA_TRIDENT_AUTO_SPIN_ATTACK_TICKS: u32 = 20;
const VANILLA_TRIDENT_BASE_DAMAGE: f32 = 8.0;
const VANILLA_TRIDENT_GROUND_LAUNCH_HEIGHT: f64 = 1.1999999_f32 as f64;

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

    pub(super) fn release_local_trident_using_item(
        &mut self,
    ) -> Option<LocalInteractionSoundAction> {
        if self.local_player().interaction.using_item_ticks < VANILLA_TRIDENT_THROW_THRESHOLD_TICKS
        {
            return None;
        }
        let hand = self.local_player().interaction.using_item_hand?;
        let stack = self.local_item_in_hand(hand)?.clone();
        if stack.item_id != Some(VANILLA_ITEM_TRIDENT_ID) {
            return None;
        }
        let effective = self.effective_item_stack(&stack);
        let enchantments = self
            .item_stack_has_effective_component(&stack, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
            .then(|| effective.component_patch.enchantments.clone())
            .unwrap_or_default();
        let riptide_strength = self.trident_spin_attack_strength(&enchantments);
        if riptide_strength > 0.0
            && (!self.local_player_is_in_water_or_rain()
                || self.local_player_vehicle_id().is_some())
        {
            return None;
        }
        if self.item_stack_next_damage_will_break(&stack) {
            return None;
        }

        // Vanilla resolves this before the server-only damage/throw branch.
        // The selected client holder is deliberately retained as evidence of
        // synchronized selection even though the local call is suppressed.
        let _sound = self.trident_release_sound(&enchantments);
        if !(riptide_strength > 0.0) {
            // Damage, stack consumption, and ThrownTrident creation are all
            // inside Vanilla's ServerLevel branch.
            return None;
        }

        let mut pose = self.local_player_pose()?;
        let y_rot = pose.y_rot.to_radians();
        let x_rot = pose.x_rot.to_radians();
        let mut xd = -y_rot.sin() * x_rot.cos();
        let mut yd = -x_rot.sin();
        let mut zd = y_rot.cos() * x_rot.cos();
        let distance = (xd * xd + yd * yd + zd * zd).sqrt();
        xd *= riptide_strength / distance;
        yd *= riptide_strength / distance;
        zd *= riptide_strength / distance;
        pose.delta_movement.x += f64::from(xd);
        pose.delta_movement.y += f64::from(yd);
        pose.delta_movement.z += f64::from(zd);
        self.start_local_player_auto_spin_attack(
            VANILLA_TRIDENT_AUTO_SPIN_ATTACK_TICKS,
            VANILLA_TRIDENT_BASE_DAMAGE,
            stack,
        );
        if pose.on_ground {
            pose = move_local_player_self_upward(self, pose, VANILLA_TRIDENT_GROUND_LAUNCH_HEIGHT);
        }
        self.local_player.pose = Some(pose);
        Some(LocalInteractionSoundAction::SuppressedTridentRiptide)
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
