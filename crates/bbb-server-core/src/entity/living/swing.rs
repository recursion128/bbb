use super::LivingEntityState;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::InteractionHand;

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SwingState {
    swinging: bool,
    time: i32,
    arm: Option<InteractionHand>,
    attack_anim: f32,
}

impl LivingEntityState {
    pub(crate) fn is_swinging(&self) -> bool {
        self.swing.swinging
    }

    // LivingEntity.java:1978-2007.
    pub(crate) fn swing(&mut self, hand: InteractionHand) -> bool {
        if !self.swing.swinging
            || self.swing.time >= self.current_swing_duration() / 2
            || self.swing.time < 0
        {
            self.swing.time = -1;
            self.swing.swinging = true;
            self.swing.arm = Some(hand);
            true
        } else {
            false
        }
    }

    // LivingEntity.java:2145-2157.
    pub(crate) fn update_swing_time(&mut self) {
        let duration = self.current_swing_duration();
        if self.swing.swinging {
            self.swing.time = self.swing.time.wrapping_add(1);
            if self.swing.time >= duration {
                self.swing.time = 0;
                self.swing.swinging = false;
            }
        } else {
            self.swing.time = 0;
        }
        self.swing.attack_anim = self.swing.time as f32 / duration as f32;
    }

    // LivingEntity.java:3353-3359.
    pub(crate) fn used_item_hand(&self) -> InteractionHand {
        if self.living_entity_flags & 2 != 0 {
            InteractionHand::OffHand
        } else {
            InteractionHand::MainHand
        }
    }

    #[cfg(test)]
    pub(crate) fn attack_anim(&self) -> f32 {
        self.swing.attack_anim
    }

    fn current_swing_duration(&self) -> i32 {
        let slot = match self.swing.arm.unwrap_or(InteractionHand::MainHand) {
            InteractionHand::MainHand => EquipmentSlot::MainHand,
            InteractionHand::OffHand => EquipmentSlot::OffHand,
        };
        let duration = self.item_by_slot(slot).map_or(6, swing_animation_duration);
        // MobEffectUtil.java:25-40: haste/conduit take precedence over fatigue.
        let haste = self.active_effect("minecraft:haste");
        let conduit = self.active_effect("minecraft:conduit_power");
        if haste.is_some() || conduit.is_some() {
            let amplifier = haste
                .map_or(0, |e| e.amplifier())
                .max(conduit.map_or(0, |e| e.amplifier()));
            duration.wrapping_sub(1_i32.wrapping_add(amplifier))
        } else if let Some(fatigue) = self.active_effect("minecraft:mining_fatigue") {
            duration.wrapping_add(1_i32.wrapping_add(fatigue.amplifier()).wrapping_mul(2))
        } else {
            duration
        }
    }
}

/// `ItemStack.getSwingAnimation().duration()` (`ItemStack.java:1033-1035`;
/// `SwingAnimation.java:11-18`).
pub(crate) fn swing_animation_duration(
    stack: &bbb_server_types::item_stack::RuntimeItemStack,
) -> i32 {
    bbb_server_types::item_stack::static_item_component(
        stack.summary(),
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
    )
    .ok()
    .flatten()
    .and_then(|patch| patch.swing_animation.map(|animation| animation.duration))
    .unwrap_or(6)
}

#[cfg(test)]
mod tests;
