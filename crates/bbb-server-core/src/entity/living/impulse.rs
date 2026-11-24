use bbb_protocol::packets::Vec3d;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct FallImpulseContext {
    pub reset_grace_time: i32,
    pub impact_position: Option<Vec3d>,
}

impl FallImpulseContext {
    // LivingEntity.java:1780-1786: false clears only the grace time.
    pub fn set_ignore_fall_damage(&mut self, ignore: bool, position: Vec3d) {
        if ignore {
            self.apply_grace(40);
            self.impact_position = Some(position);
        } else {
            self.reset_grace_time = 0;
        }
    }

    // LivingEntity.java:1789-1791.
    pub fn apply_grace(&mut self, ticks: i32) {
        self.reset_grace_time = self.reset_grace_time.max(ticks);
    }

    pub fn tick(&mut self) {
        tick_impulse_grace(&mut self.reset_grace_time);
    }

    // LivingEntity.java:1803-1805.
    pub fn is_in_grace(&self) -> bool {
        self.reset_grace_time > 0
    }

    // LivingEntity.java:1807-1811: the adapter also clears its runtime cause.
    pub fn reset(&mut self) {
        self.reset_grace_time = 0;
        self.impact_position = None;
    }

    // LivingEntity.java:1797-1801: reports execution, including an empty context.
    pub fn try_reset(&mut self) -> bool {
        if self.reset_grace_time == 0 {
            self.reset();
            true
        } else {
            false
        }
    }

    // LivingEntity.java:1754-1765: the result tells the adapter to clear its cause.
    pub fn adjust_fall_distance(&mut self, distance: f64, current_y: f64) -> (f64, bool) {
        let Some(impact) = self.impact_position else {
            return (distance, false);
        };
        let effective = java_min(distance, impact.y - current_y);
        let reset_executed = if effective <= 0.0 {
            self.reset();
            true
        } else {
            self.try_reset()
        };
        (effective, reset_executed)
    }
}

// LivingEntity.java:2802-2804: reaching zero does not clear the impact or cause.
pub(crate) fn tick_impulse_grace(ticks: &mut i32) {
    if *ticks > 0 {
        *ticks -= 1;
    }
}

fn java_min(first: f64, second: f64) -> f64 {
    // Math.min propagates NaN and prefers negative zero; f64::min ignores one NaN.
    if first.is_nan() {
        first
    } else if first == 0.0 && second == 0.0 && second.is_sign_negative() {
        second
    } else if first <= second {
        first
    } else {
        second
    }
}

#[cfg(test)]
mod tests;
