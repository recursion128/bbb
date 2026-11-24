use super::*;

impl Default for MobRetainedState {
    fn default() -> Self {
        // `Mob` field defaults and `DropChances.DEFAULT`
        // (`Mob.java:113-140`, `DropChances.java:9-16`).
        Self {
            leash: None,
            home_position: BlockPos { x: 0, y: 0, z: 0 },
            home_radius: -1,
            flags: 0,
            can_pick_up_loot: false,
            persistence_required: false,
            drop_chances: [DEFAULT_MOB_EQUIPMENT_DROP_CHANCE; VANILLA_EQUIPMENT_SLOTS.len()],
            death_loot_table: None,
            death_loot_table_seed: 0,
        }
    }
}

impl MobRetainedState {
    pub(super) fn restore_command_transients(&mut self, previous: &Self) {
        if self.home_radius < 0 {
            self.home_position = previous.home_position;
        }
        let persisted = VANILLA_MOB_FLAG_NO_AI | VANILLA_MOB_FLAG_LEFT_HANDED;
        self.flags = (self.flags & persisted) | (previous.flags & !persisted);
    }

    pub(crate) const fn flags(&self) -> i8 {
        self.flags
    }

    pub(crate) fn set_no_ai(&mut self, value: bool) {
        self.set_flag(VANILLA_MOB_FLAG_NO_AI, value);
    }

    pub(crate) fn set_left_handed(&mut self, value: bool) {
        self.set_flag(VANILLA_MOB_FLAG_LEFT_HANDED, value);
    }

    /// Answers whether the flag changed.
    pub(crate) fn set_aggressive(&mut self, value: bool) -> bool {
        let before = self.flags;
        self.set_flag(VANILLA_MOB_FLAG_AGGRESSIVE, value);
        before != self.flags
    }

    fn set_flag(&mut self, mask: i8, value: bool) {
        if value {
            self.flags |= mask;
        } else {
            self.flags &= !mask;
        }
    }

    pub(crate) const fn can_pick_up_loot(&self) -> bool {
        self.can_pick_up_loot
    }

    pub(crate) fn set_can_pick_up_loot(&mut self, value: bool) {
        self.can_pick_up_loot = value;
    }

    pub(crate) const fn persistence_required(&self) -> bool {
        self.persistence_required
    }

    pub(crate) fn set_persistence_required(&mut self, value: bool) {
        self.persistence_required = value;
    }

    pub(crate) fn drop_chance(&self, slot: EquipmentSlot) -> f32 {
        self.drop_chances[usize::from(slot.ordinal())]
    }

    pub(crate) fn set_drop_chance(&mut self, slot: EquipmentSlot, chance: f32) -> bool {
        if !chance.is_finite() || chance < 0.0 || (chance == 0.0 && chance.is_sign_negative()) {
            return false;
        }
        self.drop_chances[usize::from(slot.ordinal())] = chance;
        true
    }

    /// `Mob.setGuaranteedDrop` over `DropChances.withGuaranteedDrop`
    /// (`Mob.java:567-569`, `DropChances.java:28-29`).
    pub(crate) fn set_guaranteed_drop(&mut self, slot: EquipmentSlot) {
        self.drop_chances[usize::from(slot.ordinal())] = 2.0;
    }

    pub(crate) fn death_loot_table(&self) -> Option<&str> {
        self.death_loot_table.as_deref()
    }

    pub(crate) fn set_death_loot_table(&mut self, table: Option<String>) {
        self.death_loot_table = table;
    }

    pub(crate) const fn death_loot_table_seed(&self) -> i64 {
        self.death_loot_table_seed
    }

    pub(crate) const fn set_death_loot_table_seed(&mut self, seed: i64) {
        self.death_loot_table_seed = seed;
    }

    pub(crate) fn has_default_drop_chances(&self) -> bool {
        self.drop_chances
            .iter()
            .all(|chance| *chance == DEFAULT_MOB_EQUIPMENT_DROP_CHANCE)
    }
}

impl Entity {
    pub fn mob_home(&self) -> Option<(BlockPos, i32)> {
        let EntityData::GeneralLiving(data) = self.data() else {
            return None;
        };
        let mob = data.mob.as_ref()?;
        (mob.home_radius != -1).then_some((mob.home_position, mob.home_radius))
    }

    pub fn set_mob_home(&mut self, position: BlockPos, radius: i32) -> bool {
        let EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(mob) = data.mob.as_mut() else {
            return false;
        };
        mob.home_position = position;
        mob.home_radius = radius;
        true
    }
}
