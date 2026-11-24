//! `SpellcasterIllager`'s goals: the casting pose that holds the illager
//! still, and the evoker's and illusioner's spells.

use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_VEX_ID};
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;
use uuid::Uuid;

use super::super::target::TargetView;
use super::super::{reduced_tick_delay, Goal, MobAiContext, MobPostStepAction};
use super::RangedTarget;
use crate::entity::living::ActiveMobEffect;

/// `DyeColor.BLUE` / `DyeColor.RED`.
const BLUE: i8 = 11;
pub(crate) const RED: i8 = 14;

/// `SpellcasterIllager.spellCastingTickCount`, the evoker's `wololoTarget`
/// and the llama's `didSpit`, which several goals of one mob read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RangedState {
    casting_ticks: i32,
    /// `SpellcasterIllager.currentSpell`, by `IllagerSpell.id`.
    spell: i8,
    wololo_target: Option<i32>,
    pub(super) did_spit: bool,
    /// The crossbow write the goal phase leaves for the body.
    pub(super) crossbow_charge: Option<super::perform::CrossbowCharge>,
}

impl RangedState {
    pub(crate) fn after_goals(self) -> Self {
        Self {
            casting_ticks: (self.casting_ticks - 1).max(0),
            ..self
        }
    }

    pub(crate) const fn spell(self) -> i8 {
        self.spell
    }

    /// `spellCastingTickCount` lives on `RaiderRetainedState` and persists as
    /// `SpellTicks`, so a reload resumes the cast.
    pub(crate) fn with_saved_casting_ticks(self, entity: &crate::entity::Entity) -> Self {
        let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
            return self;
        };
        match data.raider_retained().and_then(|state| state.spell_ticks()) {
            Some(casting_ticks) => Self {
                casting_ticks,
                ..self
            },
            None => self,
        }
    }

    pub(crate) fn save_casting_ticks(self, entity: &mut crate::entity::Entity) {
        let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
            return;
        };
        if let Some(state) = data.raider_retained_mut() {
            state.set_spell_ticks(self.casting_ticks);
        }
    }

    pub(crate) fn take_crossbow_charge(&mut self) -> Option<super::perform::CrossbowCharge> {
        self.crossbow_charge.take()
    }

    pub(crate) fn take_did_spit(&mut self) -> bool {
        std::mem::take(&mut self.did_spit)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spell {
    SummonVex,
    Fangs,
    Wololo,
    Disappear,
    Blindness,
}

impl Spell {
    /// `getCastWarmupTime`, `getCastingTime`, `getCastingInterval`
    /// (`Evoker.java`, `Illusioner.java`).
    const fn timing(self) -> (i32, i32, i32) {
        match self {
            Self::SummonVex => (20, 100, 340),
            Self::Fangs => (20, 40, 100),
            Self::Wololo => (40, 60, 140),
            Self::Disappear => (20, 20, 340),
            Self::Blindness => (20, 20, 180),
        }
    }

    const fn prepare_sound(self) -> &'static str {
        match self {
            Self::SummonVex => "minecraft:entity.evoker.prepare_summon",
            Self::Fangs => "minecraft:entity.evoker.prepare_attack",
            Self::Wololo => "minecraft:entity.evoker.prepare_wololo",
            Self::Disappear => "minecraft:entity.illusioner.prepare_mirror",
            Self::Blindness => "minecraft:entity.illusioner.prepare_blindness",
        }
    }

    /// `SpellcasterIllager.IllagerSpell.id` (`SpellcasterIllager.java:123-128`).
    const fn id(self) -> i8 {
        match self {
            Self::SummonVex => 1,
            Self::Fangs => 2,
            Self::Wololo => 3,
            Self::Disappear => 4,
            Self::Blindness => 5,
        }
    }

    const fn cast_sound(self) -> &'static str {
        match self {
            Self::SummonVex | Self::Fangs | Self::Wololo => "minecraft:entity.evoker.cast_spell",
            Self::Disappear | Self::Blindness => "minecraft:entity.illusioner.cast_spell",
        }
    }
}

/// What a spell does once its warmup ends, replayed against the level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellCast {
    pub(crate) spell: Spell,
    pub(crate) target: Option<RangedTarget>,
    pub(crate) sheep: Option<i32>,
}

/// The world half of a spell: what reaches a session or spawns a mob.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingMobSpell {
    ElderGuardianPulse {
        source: Uuid,
        position: Vec3d,
        silent: bool,
    },
    /// `target.addEffect(effect, caster)` on a player.
    PlayerEffect {
        source_entity_id: i32,
        player: Uuid,
        effect: ActiveMobEffect,
    },
    /// `EvokerSummonSpellGoal.performSpellCasting`:
    /// each vex's block and limited life.
    SummonVexes {
        owner: Uuid,
        vexes: Vec<(BlockPos, i32)>,
    },
}

/// `SpellcasterIllager.SpellcasterCastingSpellGoal`
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SpellcasterCastingGoal;

impl Goal for SpellcasterCastingGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.ranged.casting_ticks > 0
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.ranged.spell = 0;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let look = context.unchecked_target(targets).or_else(|| {
            context
                .ranged
                .wololo_target
                .and_then(|id| targets.facts(id))
        });
        if let Some(target) = look {
            let position = target.position();
            context.set_look_at_with_limits(position.x, target.eye_y(), position.z, 75.0, 40.0);
        }
    }
}

/// `SpellcasterIllager.SpellcasterUseSpellGoal` and its subclasses.
#[derive(Debug, Clone)]
pub(crate) struct SpellGoal {
    spell: Spell,
    warmup: i32,
    next_attack_tick: i32,
    last_target: Option<i32>,
}

impl SpellGoal {
    pub(crate) fn new(spell: Spell) -> Self {
        Self {
            spell,
            warmup: 0,
            next_attack_tick: 0,
            last_target: None,
        }
    }

    /// `SpellcasterUseSpellGoal.canUse`.
    fn base_can_use(&self, context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
        context
            .unchecked_target(targets)
            .is_some_and(|target| target.alive())
            && context.ranged.casting_ticks <= 0
            && context.tick_count >= self.next_attack_tick
    }

    fn living_in_range(
        context: &MobAiContext,
        targets: &TargetView<'_, '_, '_>,
        entity_type_id: i32,
        vertical: f64,
    ) -> Vec<i32> {
        let search = context.body.bounding_box.inflate(16.0, vertical, 16.0);
        let position = context.position();
        targets
            .store()
            .ids_in_box_sections(search)
            .into_iter()
            .filter(|&id| {
                targets.store().get(id).is_some_and(|entity| {
                    entity.entity_type_id() == entity_type_id
                        && entity.is_alive()
                        && entity.bounding_box().intersects(search)
                        && entity.position().distance_to_sqr(position) <= 256.0
                })
            })
            .collect()
    }

    fn wololo_can_use(&self, context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
        if context.unchecked_target(targets).is_some()
            || context.ranged.casting_ticks > 0
            || context.tick_count < self.next_attack_tick
            || !targets.level().mob_griefing()
        {
            return false;
        }
        let sheep: Vec<i32> =
            Self::living_in_range(context, targets, VANILLA_ENTITY_TYPE_SHEEP_ID, 4.0)
                .into_iter()
                .filter(|&id| {
                    targets
                        .store()
                        .get(id)
                        .and_then(crate::entity::sheep::color_of)
                        == Some(BLUE)
                })
                .collect();
        if sheep.is_empty() {
            return false;
        }
        let index = context.next_int(sheep.len() as i32) as usize;
        context.ranged.wololo_target = Some(sheep[index]);
        true
    }
}

impl Goal for SpellGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        match self.spell {
            Spell::Wololo => self.wololo_can_use(context, targets),
            Spell::SummonVex => {
                self.base_can_use(context, targets) && {
                    let vexes =
                        Self::living_in_range(context, targets, VANILLA_ENTITY_TYPE_VEX_ID, 16.0)
                            .len() as i32;
                    context.next_int(8) + 1 > vexes
                }
            }
            Spell::Fangs => self.base_can_use(context, targets),
            Spell::Disappear => {
                self.base_can_use(context, targets)
                    && targets.body_entity().living().is_none_or(|living| {
                        living.active_effect("minecraft:invisibility").is_none()
                    })
            }
            // `Illusioner`: local difficulty above NORMAL's ordinal.
            Spell::Blindness => {
                self.base_can_use(context, targets)
                    && context
                        .unchecked_target(targets)
                        .is_some_and(|target| Some(target.entity_id()) != self.last_target)
                    && targets
                        .level()
                        .effective_difficulty_at(targets.body_entity().block_position())
                        .is_some_and(|difficulty| difficulty > 2.0)
            }
        }
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let subject = match self.spell {
            Spell::Wololo => context.ranged.wololo_target.is_some(),
            _ => context
                .unchecked_target(targets)
                .is_some_and(|target| target.alive()),
        };
        subject && self.warmup > 0
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let (warmup, casting, interval) = self.spell.timing();
        self.warmup = reduced_tick_delay(warmup);
        context.ranged.casting_ticks = casting;
        self.next_attack_tick = context.tick_count + interval;
        if self.spell == Spell::Blindness {
            self.last_target = context
                .unchecked_target(targets)
                .map(|target| target.entity_id());
        }
        targets.play_body_sound(self.spell.prepare_sound(), SoundSource::Hostile, 1.0, 1.0);
        context.ranged.spell = self.spell.id();
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        if self.spell == Spell::Wololo {
            context.ranged.wololo_target = None;
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.warmup -= 1;
        if self.warmup != 0 {
            return;
        }
        let target = context
            .unchecked_target(targets)
            .map(|target| RangedTarget::of(&target));
        context
            .post_step
            .push(MobPostStepAction::CastSpell(SpellCast {
                spell: self.spell,
                target,
                sheep: context.ranged.wololo_target,
            }));
        targets.play_body_sound(self.spell.cast_sound(), SoundSource::Hostile, 1.0, 1.0);
    }
}
