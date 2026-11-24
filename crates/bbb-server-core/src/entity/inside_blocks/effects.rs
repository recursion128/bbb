//! Ordered block and fluid effects shared by stored and session bodies.

use bbb_protocol::block_types;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::fluid::FluidFamily;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsideBlockEffectType {
    Freeze,
    ClearFreeze,
    FireIgnite,
    LavaIgnite,
    Extinguish,
}

impl InsideBlockEffectType {
    const ALL: [Self; 5] = [
        Self::Freeze,
        Self::ClearFreeze,
        Self::FireIgnite,
        Self::LavaIgnite,
        Self::Extinguish,
    ];

    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InsideBlockEffect {
    Type(InsideBlockEffectType),
    Damage {
        damage_type: &'static str,
        amount: f32,
    },
    LavaDamage {
        amount: f32,
    },
    DestroyPowderSnow(BlockPos),
    LowerCauldron {
        pos: BlockPos,
        state: i32,
    },
}

/// `InsideBlockEffectApplier.StepBasedCollector`
/// (`InsideBlockEffectApplier.java`).
#[derive(Default)]
pub struct BlockEffectCollector {
    effects: [bool; 5],
    before: [Vec<InsideBlockEffect>; 5],
    after: [Vec<InsideBlockEffect>; 5],
    final_effects: Vec<InsideBlockEffect>,
    last_step: Option<i32>,
}

impl BlockEffectCollector {
    pub fn advance_step(&mut self, step: i32) {
        if self.last_step != Some(step) {
            self.last_step = Some(step);
            self.flush_step();
        }
    }

    fn apply(&mut self, effect: InsideBlockEffectType) {
        self.effects[effect.index()] = true;
    }

    fn run_before(&mut self, effect: InsideBlockEffectType, action: InsideBlockEffect) {
        self.before[effect.index()].push(action);
    }

    fn run_after(&mut self, effect: InsideBlockEffectType, action: InsideBlockEffect) {
        self.after[effect.index()].push(action);
    }

    fn flush_step(&mut self) {
        for effect in InsideBlockEffectType::ALL {
            let index = effect.index();
            self.final_effects.append(&mut self.before[index]);
            if std::mem::take(&mut self.effects[index]) {
                self.final_effects.push(InsideBlockEffect::Type(effect));
            }
            self.final_effects.append(&mut self.after[index]);
        }
    }

    pub fn take_effects(&mut self) -> Vec<InsideBlockEffect> {
        self.flush_step();
        self.last_step = None;
        std::mem::take(&mut self.final_effects)
    }

    pub fn collect_block(&mut self, kind: EnvironmentBlockKind, pos: BlockPos, state: i32) {
        match kind {
            EnvironmentBlockKind::Fire { damage } => {
                self.apply(InsideBlockEffectType::ClearFreeze);
                self.apply(InsideBlockEffectType::FireIgnite);
                self.run_after(
                    InsideBlockEffectType::FireIgnite,
                    InsideBlockEffect::Damage {
                        damage_type: "minecraft:in_fire",
                        amount: damage,
                    },
                );
            }
            EnvironmentBlockKind::PowderSnow => {
                self.run_before(
                    InsideBlockEffectType::Extinguish,
                    InsideBlockEffect::DestroyPowderSnow(pos),
                );
                self.apply(InsideBlockEffectType::Freeze);
                self.apply(InsideBlockEffectType::Extinguish);
            }
            EnvironmentBlockKind::LavaCauldron => self.collect_fluid(FluidFamily::Lava),
            EnvironmentBlockKind::LayeredCauldron => {
                self.run_before(
                    InsideBlockEffectType::Extinguish,
                    InsideBlockEffect::LowerCauldron { pos, state },
                );
                self.apply(InsideBlockEffectType::Extinguish);
            }
            EnvironmentBlockKind::Other => {}
        }
    }

    pub fn collect_fluid(&mut self, family: FluidFamily) {
        match family {
            FluidFamily::Water => self.apply(InsideBlockEffectType::Extinguish),
            FluidFamily::Lava => {
                self.apply(InsideBlockEffectType::ClearFreeze);
                self.apply(InsideBlockEffectType::LavaIgnite);
                self.run_after(
                    InsideBlockEffectType::LavaIgnite,
                    InsideBlockEffect::LavaDamage { amount: 4.0 },
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvironmentBlockKind {
    Fire { damage: f32 },
    PowderSnow,
    LavaCauldron,
    LayeredCauldron,
    Other,
}

pub(super) fn environment_block_kind(block_type_id: i32) -> EnvironmentBlockKind {
    match block_type_id {
        block_types::VANILLA_BLOCK_FIRE_ID => EnvironmentBlockKind::Fire { damage: 1.0 },
        block_types::VANILLA_BLOCK_SOUL_FIRE_ID => EnvironmentBlockKind::Fire { damage: 2.0 },
        block_types::VANILLA_BLOCK_POWDER_SNOW_ID => EnvironmentBlockKind::PowderSnow,
        block_types::VANILLA_BLOCK_LAVA_CAULDRON_ID => EnvironmentBlockKind::LavaCauldron,
        block_types::VANILLA_BLOCK_WATER_CAULDRON_ID
        | block_types::VANILLA_BLOCK_POWDER_SNOW_CAULDRON_ID => {
            EnvironmentBlockKind::LayeredCauldron
        }
        _ => EnvironmentBlockKind::Other,
    }
}
