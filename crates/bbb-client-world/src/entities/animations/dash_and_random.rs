use super::*;

impl EntityClientAnimationState {
    pub(in crate::entities) fn start_local_arrow_shake(&mut self) {
        if let Some(slot) = self.arrow_kind_mut() {
            *slot = Some(ArrowShakeAnimationState::started());
        }
    }

    pub(in crate::entities) fn advance_arrow_shake_tick(&mut self) {
        if let Some(shake) = self.arrow_kind_mut().and_then(|slot| slot.as_mut()) {
            shake.advance_client_tick();
            if shake.is_settled() {
                if let Some(slot) = self.arrow_kind_mut() {
                    *slot = None;
                }
            }
        }
    }

    pub(crate) fn camel_dash_should_clear_before_tick(
        self,
        is_dashing: bool,
        on_ground: bool,
        in_liquid: bool,
        is_passenger: bool,
    ) -> bool {
        // A newly observed synced DASH edge has the same effective 55-tick
        // cooldown that `Camel.onSyncedDataUpdated` installs in Vanilla. The
        // state is created by this tick below, so it cannot clear on that edge.
        self.camel_kind()
            .and_then(|k| k.dash_cooldown)
            .is_some_and(|cooldown| {
                cooldown.should_clear_before_tick(is_dashing, on_ground, in_liquid, is_passenger)
            })
    }

    pub(crate) fn start_local_camel_dash_after_tick(&mut self) {
        let age_ticks = self.age_ticks;
        if let Some(camel) = self.camel_kind_mut() {
            camel
                .dash
                .get_or_insert_with(KeyframeAnimationState::default)
                .animate_when(true, age_ticks);
            camel
                .dash_cooldown
                .get_or_insert_with(Default::default)
                .start_local_dash_after_tick();
        }
    }

    pub(crate) fn stop_local_camel_dash(&mut self) {
        let age_ticks = self.age_ticks;
        let Some(camel) = self.camel_kind_mut() else {
            return;
        };
        if let Some(dash) = camel.dash.as_mut() {
            dash.animate_when(false, age_ticks);
        }
        if let Some(cooldown) = camel.dash_cooldown.as_mut() {
            cooldown.stop_local_dash();
        }
    }

    pub(crate) fn start_local_nautilus_dash(&mut self) {
        if let Some(nautilus) = self.nautilus_kind_mut() {
            nautilus
                .dash_cooldown
                .get_or_insert_with(Default::default)
                .start_local_dash();
        }
    }

    pub(crate) fn advance_item_based_steering_tick(&mut self, boost_time_total: i32) -> f32 {
        self.item_based_steering
            .as_mut()
            .map_or(1.0, |steering| steering.tick_and_factor(boost_time_total))
    }

    pub(crate) fn next_equine_feeding_particle_units(&mut self, entity_id: i32) -> [f64; 3] {
        let Some(equine) = self.equine_kind_mut() else {
            return [0.5; 3];
        };
        let equine = equine
            .tail
            .get_or_insert_with(|| EquineTailAnimationState::new(entity_id));
        [
            equine.next_double(),
            equine.next_double(),
            equine.next_double(),
        ]
    }

    pub(crate) fn next_local_interaction_random_float(&mut self, entity_id: i32) -> f32 {
        self.entity_random_mut(entity_id).next_float()
    }

    pub(crate) fn next_local_interaction_random_double(&mut self, entity_id: i32) -> f64 {
        self.entity_random_mut(entity_id).next_double()
    }

    pub(crate) fn next_local_interaction_random_int_bound(
        &mut self,
        entity_id: i32,
        bound: i32,
    ) -> i32 {
        if bound <= 1 {
            return 0;
        }
        ((self.entity_random_mut(entity_id).next_double() * f64::from(bound)) as i32)
            .clamp(0, bound - 1)
    }

    pub(crate) fn next_local_interaction_random_gaussian(&mut self, entity_id: i32) -> f64 {
        // Box-Muller transform using the entity's client RNG.
        let rng = self.entity_random_mut(entity_id);
        let u1 = rng.next_double().max(1.0e-12);
        let u2 = rng.next_double();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    /// Enforces `EnderMan.playStareSound`'s 400-tick cooldown (`EnderMan.java:173-180`).
    pub(crate) fn try_trigger_enderman_stare_sound(&mut self) -> bool {
        let age_ticks = self.age_ticks;
        let EntityKindAnimation::EnderAmbient {
            last_stare_sound_tick,
            ..
        } = &mut self.kind
        else {
            return false;
        };
        if last_stare_sound_tick.is_some_and(|last| age_ticks < last.saturating_add(400)) {
            return false;
        }
        *last_stare_sound_tick = Some(age_ticks);
        true
    }

    pub(crate) fn entity_random_mut(&mut self, entity_id: i32) -> &mut LevelEventSoundRandomState {
        self.entity_random.get_or_insert_with(|| {
            LevelEventSoundRandomState::with_seed(
                i64::from(entity_id).wrapping_add(ENTITY_CLIENT_RANDOM_SALT as i64),
            )
        })
    }

    pub(crate) fn needs_particle_trigger_bounds_snapshot(&self) -> bool {
        self.ravager_kind().is_some() || self.evoker_fangs_kind().is_some()
    }

    pub(crate) fn item_bob_offset(&self) -> f32 {
        match self.kind {
            EntityKindAnimation::Item(offset) => offset,
            _ => 0.0,
        }
    }

    pub(crate) fn end_crystal_age_ticks(&self) -> u32 {
        match self.kind {
            EntityKindAnimation::EndCrystal(offset) => offset,
            _ => 0,
        }
        .wrapping_add(self.age_ticks)
    }

    pub(crate) fn age_lock_particle_timer_ticks(&self) -> u8 {
        self.age_lock_particle_timer_ticks
    }

    pub(crate) fn start_age_lock_particle_timer(&mut self) {
        self.age_lock_particle_timer_ticks = AGE_LOCK_COOLDOWN_TICKS;
    }

    pub(crate) fn advance_age_lock_particle_timer_tick(&mut self) {
        self.age_lock_particle_timer_ticks = self.age_lock_particle_timer_ticks.saturating_sub(1);
    }
}
