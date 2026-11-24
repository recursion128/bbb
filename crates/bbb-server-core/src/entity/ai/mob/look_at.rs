use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_GUARDIAN_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::{Entity, EntityData};

use super::target::conditions::TargetingConditions;
use super::target::{Target, TargetRef, TargetView};
use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookAtTargetKind {
    Player,
    AnyMob,
    Guardian,
    Strider,
}

impl LookAtTargetKind {
    fn matches(self, entity: &Entity) -> bool {
        match self {
            Self::Player => false,
            Self::AnyMob => {
                matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_mob())
            }
            Self::Guardian => matches!(
                entity.entity_type_id(),
                VANILLA_ENTITY_TYPE_GUARDIAN_ID | VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID
            ),
            Self::Strider => entity.entity_type_id() == VANILLA_ENTITY_TYPE_STRIDER_ID,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LookAtSpec {
    pub(crate) target_kind: LookAtTargetKind,
    pub(crate) distance: f64,
    pub(crate) probability: f32,
    pub(crate) only_horizontal: bool,
}

pub(super) fn nearest_look_at_target<'a, 'l, 'w>(
    targets: &'a TargetView<'_, 'l, 'w>,
    kind: LookAtTargetKind,
    range: f64,
    nearest_origin: Vec3d,
    admitted: impl Fn(i32) -> bool,
) -> Option<Target<'a, 'l, 'w>> {
    let conditions = TargetingConditions::for_non_combat().range(range);
    let mut nearest = None;
    let mut nearest_distance = f64::INFINITY;
    let mut consider = |target: Target<'a, 'l, 'w>| {
        if conditions.test(targets, &target, |target| admitted(target.entity_id())) {
            let distance = nearest_origin.distance_to_sqr(target.position());
            if distance < nearest_distance {
                nearest = Some(target);
                nearest_distance = distance;
            }
        }
    };
    if kind == LookAtTargetKind::Player {
        for target in targets.local_players() {
            consider(target);
        }
    } else {
        let search = targets
            .body_entity()
            .bounding_box()
            .inflate(range, 3.0, range);
        targets.store().for_each_box_section_id(search, |id| {
            let Some(entity) = targets.store().get(id) else {
                return;
            };
            if kind.matches(entity) && entity.bounding_box().intersects(search) {
                consider(targets.facts(id).expect("stored look targets are living"));
            }
        });
    }
    nearest
}

#[derive(Debug, Clone)]
enum LookAtMode {
    Standard(LookAtSpec),
    Trading,
    PiglinAttention {
        next_attempt: Option<i64>,
    },
    IdleAttention {
        players_only: bool,
        max_distance: f64,
        approach_speed: f64,
        next_attempt: Option<i64>,
        last_navigation_target: Option<BlockPos>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PiglinAttentionMode {
    Idle,
    BruteIdle,
    Celebrate,
    Ride,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum PiglinPeer {
    Piglin,
    Brute,
}

impl PiglinPeer {
    const fn entity_type_id(self) -> i32 {
        match self {
            Self::Piglin => VANILLA_ENTITY_TYPE_PIGLIN_ID,
            Self::Brute => VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LookAtPlayerGoal {
    mode: LookAtMode,
    target: Option<TargetRef>,
    look_time: i32,
}

impl LookAtPlayerGoal {
    pub(super) const fn new(spec: LookAtSpec) -> Self {
        Self {
            mode: LookAtMode::Standard(spec),
            target: None,
            look_time: 0,
        }
    }

    pub(super) const fn trading() -> Self {
        Self {
            mode: LookAtMode::Trading,
            target: None,
            look_time: 0,
        }
    }

    pub(super) const fn hoglin() -> Self {
        Self::idle_attention(false, 8.0, 0.4)
    }

    pub(super) const fn piglin() -> Self {
        Self {
            mode: LookAtMode::PiglinAttention { next_attempt: None },
            target: None,
            look_time: 0,
        }
    }

    pub(super) const fn idle_attention(
        players_only: bool,
        max_distance: f64,
        approach_speed: f64,
    ) -> Self {
        Self {
            mode: LookAtMode::IdleAttention {
                players_only,
                max_distance,
                approach_speed,
                next_attempt: None,
                last_navigation_target: None,
            },
            target: None,
            look_time: 0,
        }
    }

    const fn spec(&self) -> Option<LookAtSpec> {
        match self.mode {
            LookAtMode::Standard(spec) => Some(spec),
            LookAtMode::Trading => Some(LookAtSpec {
                target_kind: LookAtTargetKind::Player,
                distance: 8.0,
                probability: 1.0,
                only_horizontal: false,
            }),
            LookAtMode::IdleAttention { .. } | LookAtMode::PiglinAttention { .. } => None,
        }
    }

    fn can_use_idle_attention(
        &mut self,
        context: &mut MobAiContext,
        targets: &TargetView<'_, '_, '_>,
    ) -> bool {
        let LookAtMode::IdleAttention {
            players_only,
            max_distance,
            next_attempt,
            ..
        } = &mut self.mode
        else {
            return false;
        };
        if !*players_only && context.target.target.is_some() {
            return false;
        }
        let now = targets.game_time();
        match *next_attempt {
            Some(due) if now < due => return false,
            None => {
                *next_attempt = Some(now.saturating_add(i64::from(30 + context.next_int(31))));
                return false;
            }
            Some(_) => {}
        }
        *next_attempt = Some(now.saturating_add(i64::from(30 + context.next_int(31))));
        // Sensor: visibility scales follow range separately from the look radius.
        let conditions = TargetingConditions::for_non_combat().range(context.body.follow_range);
        let radius = *max_distance;
        let search_box = bbb_server_types::geometry::Aabb {
            min_x: context.body.position.x - radius,
            min_y: context.body.position.y - radius,
            min_z: context.body.position.z - radius,
            max_x: context.body.position.x + radius,
            max_y: context.body.position.y + radius,
            max_z: context.body.position.z + radius,
        };
        self.target = targets
            .local_players()
            .chain(
                (!*players_only)
                    .then(|| targets.local_stored_in(search_box))
                    .into_iter()
                    .flatten()
                    .filter(|target| !target.is_player()),
            )
            .filter(|target| {
                context.body.position.distance_to_sqr(target.position())
                    <= *max_distance * *max_distance
                    && conditions.test(targets, target, |_| true)
            })
            .min_by(|a, b| {
                context
                    .body
                    .position
                    .distance_to_sqr(a.position())
                    .total_cmp(&context.body.position.distance_to_sqr(b.position()))
            })
            .map(Target::target_ref);
        self.target.is_some()
    }

    fn idle_attention_target<'a, 'l, 'w>(
        &self,
        context: &MobAiContext,
        targets: &'a TargetView<'_, 'l, 'w>,
    ) -> Option<Target<'a, 'l, 'w>> {
        let LookAtMode::IdleAttention { players_only, .. } = self.mode else {
            return None;
        };
        if !players_only && context.target.target.is_some() {
            return None;
        }
        let target = self.target?;
        targets.facts_of(target).filter(|target| {
            target.is_local()
                && (!players_only || target.is_player())
                && TargetingConditions::for_non_combat()
                    .range(context.body.follow_range)
                    .test(targets, target, |_| true)
        })
    }
}

impl LookAtPlayerGoal {
    pub(super) fn tick_piglin_attention(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        mode: PiglinAttentionMode,
    ) -> bool {
        if !matches!(self.mode, LookAtMode::PiglinAttention { .. }) {
            return false;
        }
        if context.target.target.is_some() {
            self.target = None;
            self.look_time = 0;
            return false;
        }
        let approaching_ride = mode == PiglinAttentionMode::Ride
            && targets.store().vehicle(context.entity_id).is_none();
        if approaching_ride {
            self.target = None;
            self.look_time = 0;
        }
        let mut loved = None;
        let mut interaction = None;
        for player in targets
            .local_players()
            .filter(|target| piglin_visible(context, targets, *target))
        {
            let distance = context.body.position.distance_to_sqr(player.position());
            // PiglinAi: the walk gate uses every sensor-visible loved player.
            if crate::entity::piglin::player_holds_loved_item(targets, player)
                && loved.is_none_or(|(_, best)| distance < best)
            {
                loved = Some((player.target_ref(), distance));
            }
            // SetLookAndInteract: interaction within four blocks only sets gaze.
            if matches!(
                mode,
                PiglinAttentionMode::Idle | PiglinAttentionMode::BruteIdle
            ) && distance <= 16.0
                && interaction.is_none_or(|(_, best)| distance < best)
            {
                interaction = Some((player.target_ref(), distance));
            }
        }
        let loved_range_squared = if mode == PiglinAttentionMode::Ride {
            64.0
        } else {
            196.0
        };
        let preferred = loved
            .filter(|(_, distance)| {
                mode != PiglinAttentionMode::BruteIdle && *distance <= loved_range_squared
            })
            .or(interaction)
            .map(|(target, _)| target);
        if let Some(target) = preferred {
            self.set_piglin_target(context, target);
        } else if !approaching_ride && self.current_idle_target(context, targets).is_none() {
            self.target = None;
            self.look_time = 0;
            let LookAtMode::PiglinAttention { next_attempt } = &mut self.mode else {
                return loved.is_some();
            };
            let now = targets.game_time();
            if next_attempt.is_none_or(|due| now >= due) {
                *next_attempt = Some(now.saturating_add(i64::from(30 + context.next_int(31))));
                let choice = match mode {
                    PiglinAttentionMode::Idle | PiglinAttentionMode::Ride => context.next_int(4),
                    PiglinAttentionMode::BruteIdle => context.next_int(5),
                    PiglinAttentionMode::Celebrate => 1,
                };
                let search_box = bbb_server_types::geometry::Aabb {
                    min_x: context.body.position.x - 8.0,
                    min_y: context.body.position.y - 8.0,
                    min_z: context.body.position.z - 8.0,
                    max_x: context.body.position.x + 8.0,
                    max_y: context.body.position.y + 8.0,
                    max_z: context.body.position.z + 8.0,
                };
                let selected = targets
                    .local_players()
                    .chain(
                        targets
                            .local_stored_in(search_box)
                            .filter(|target| !target.is_player()),
                    )
                    .filter(|target| {
                        context.body.position.distance_to_sqr(target.position()) <= 64.0
                            && piglin_visible(context, targets, *target)
                            && match (mode, choice) {
                                (_, 0) => target.is_player(),
                                (_, 1) => target.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_ID,
                                (PiglinAttentionMode::BruteIdle, 2) => {
                                    target.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
                                }
                                (PiglinAttentionMode::Idle | PiglinAttentionMode::Ride, 2)
                                | (PiglinAttentionMode::BruteIdle, 3) => true,
                                _ => false,
                            }
                    })
                    .min_by(|a, b| {
                        context
                            .body
                            .position
                            .distance_to_sqr(a.position())
                            .total_cmp(&context.body.position.distance_to_sqr(b.position()))
                    })
                    .map(|target| target.target_ref());
                if let Some(target) = selected {
                    self.set_piglin_target(context, target);
                }
            }
        }
        self.tick_piglin_gaze(context, targets);
        loved.is_some()
    }

    pub(super) fn current_idle_target(
        &self,
        context: &MobAiContext,
        targets: &TargetView<'_, '_, '_>,
    ) -> Option<TargetRef> {
        if !matches!(self.mode, LookAtMode::PiglinAttention { .. })
            || self.look_time <= 0
            || context.target.target.is_some()
        {
            return None;
        }
        let target = self.target?;
        targets
            .facts_of(target)
            .filter(|target| piglin_visible(context, targets, *target))
            .map(|target| target.target_ref())
    }

    // InteractWith: the movement owner consumes this same gaze immediately.
    pub(super) fn look_at_peer(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        peer: PiglinPeer,
    ) -> Option<TargetRef> {
        if !matches!(self.mode, LookAtMode::PiglinAttention { .. })
            || context.target.target.is_some()
        {
            return None;
        }
        let search_box = bbb_server_types::geometry::Aabb {
            min_x: context.body.position.x - 8.0,
            min_y: context.body.position.y - 8.0,
            min_z: context.body.position.z - 8.0,
            max_x: context.body.position.x + 8.0,
            max_y: context.body.position.y + 8.0,
            max_z: context.body.position.z + 8.0,
        };
        let target = targets
            .local_stored_in(search_box)
            .filter(|target| {
                target.entity_type_id() == peer.entity_type_id()
                    && context.body.position.distance_to_sqr(target.position()) <= 64.0
                    && piglin_visible(context, targets, *target)
            })
            .min_by(|a, b| {
                context
                    .body
                    .position
                    .distance_to_sqr(a.position())
                    .total_cmp(&context.body.position.distance_to_sqr(b.position()))
            })?
            .target_ref();
        self.set_piglin_target(context, target);
        self.tick_piglin_gaze(context, targets);
        Some(target)
    }

    fn set_piglin_target(&mut self, context: &mut MobAiContext, target: TargetRef) {
        if self.target != Some(target) || self.look_time <= 0 {
            self.target = Some(target);
            self.look_time = 45 + context.next_int(46);
        }
    }

    fn tick_piglin_gaze(&mut self, context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) {
        if let Some(target) = self
            .current_idle_target(context, targets)
            .and_then(|target| targets.facts_of(target))
        {
            let position = target.position();
            context.set_look_at(position.x, target.eye_y(), position.z);
            self.look_time -= 1;
        }
    }

    pub(super) fn tick_idle_attention(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        approach: bool,
    ) {
        if self.look_time <= 0 || self.idle_attention_target(context, targets).is_none() {
            if self.target.is_some() {
                self.stop(context, targets);
            }
            if !self.can_use(context, targets) {
                return;
            }
            self.start(context, targets);
            // This entry point runs twice as often as the selector's ordinary goal ticks.
            self.look_time *= 2;
        }
        self.tick_attention(context, targets, approach);
    }

    fn tick_attention(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        approach: bool,
    ) {
        let Some(target) = self.idle_attention_target(context, targets) else {
            return;
        };
        let position = target.position();
        context.set_look_at(position.x, target.eye_y(), position.z);
        let LookAtMode::IdleAttention {
            approach_speed,
            last_navigation_target,
            ..
        } = &mut self.mode
        else {
            return;
        };
        let feet_block = super::navigation::containing_block(position);
        let mut speed = *approach_speed;
        let mut approach = approach;
        // AxolotlAi: only towards a target in the same medium.
        if targets.body_entity().entity_type_id()
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID
        {
            let water_at_target = crate::entity::level::fluid_state_tag_membership(
                targets.level(),
                bbb_server_types::fluid::block_state_fluid_state(
                    targets.level().block_state(feet_block),
                ),
                "minecraft:water",
            );
            approach &= water_at_target == context.body.in_water;
            if context.body.in_water {
                speed = 0.5;
            }
        }
        if approach
            && !crate::entity::camel::refuses_to_move(targets.body_entity(), targets.game_time())
            && context.navigation_done()
            && context.body.position.distance_to_sqr(position) > 9.0
            && *last_navigation_target != Some(feet_block)
        {
            context.try_navigate_to_position(position, 3, speed);
            *last_navigation_target = Some(feet_block);
        }
        self.look_time -= 1;
    }
}

impl Goal for LookAtPlayerGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if matches!(self.mode, LookAtMode::Trading) {
            self.target = targets
                .body_entity()
                .merchant_trading_player()
                .and_then(|id| targets.facts(id))
                .filter(|target| target.is_local() && target.is_player())
                .map(Target::target_ref);
            return self.target.is_some();
        }
        let Some(spec) = self.spec() else {
            return self.can_use_idle_attention(context, targets);
        };
        if !fox_attention_allowed(targets) {
            return false;
        }
        if context.next_float() >= spec.probability {
            return false;
        }
        self.target = nearest_look_at_target(
            targets,
            spec.target_kind,
            spec.distance,
            Vec3d {
                y: context.body.eye_y,
                ..context.body.position
            },
            |id| {
                spec.target_kind != LookAtTargetKind::Player
                    || !targets
                        .store()
                        .has_indirect_passenger(context.entity_id, id)
            },
        )
        .map(Target::target_ref);
        self.target.is_some()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(spec) = self.spec() else {
            return self.look_time > 0 && self.idle_attention_target(context, targets).is_some();
        };
        let Some(target) = self.target.and_then(|target| targets.facts_of(target)) else {
            return false;
        };
        fox_attention_allowed(targets)
            && target.alive()
            && context.body.position.distance_to_sqr(target.position())
                <= spec.distance * spec.distance
            && self.look_time > 0
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.look_time = if self.spec().is_some() {
            (40 + context.next_int(40) + 1) / 2
        } else {
            // GoatAi: a finite look releases MOVE for idle strolling.
            (45 + context.next_int(46) + 1) / 2
        };
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let LookAtMode::IdleAttention {
            next_attempt,
            last_navigation_target,
            ..
        } = &mut self.mode
        {
            if last_navigation_target
                .take()
                .is_some_and(|target| context.nav.navigation_target == Some(target))
            {
                context.stop_navigation();
            }
            *next_attempt = Some(
                targets
                    .game_time()
                    .saturating_add(i64::from(30 + context.next_int(31))),
            );
        }
        self.target = None;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if matches!(self.mode, LookAtMode::PiglinAttention { .. }) {
            self.tick_piglin_gaze(context, targets);
            return;
        }
        let Some(spec) = self.spec() else {
            self.tick_attention(context, targets, true);
            return;
        };
        let target = self
            .target
            .and_then(|target| targets.facts_of(target))
            .filter(|target| target.alive())
            .map(|target| {
                let y = if spec.only_horizontal {
                    context.body.eye_y
                } else {
                    target.eye_y()
                };
                let position = target.position();
                (position.x, y, position.z)
            });
        if let Some((x, y, z)) = target {
            context.set_look_at(x, y, z);
            self.look_time -= 1;
        }
    }
}

fn fox_attention_allowed(targets: &TargetView<'_, '_, '_>) -> bool {
    targets.body_entity().entity_type_id() != VANILLA_ENTITY_TYPE_FOX_ID
        || matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.fox_retained().is_some_and(|fox| !fox.is_faceplanted() && !fox.is_interested()))
}

fn piglin_visible<'w>(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, 'w>,
    target: Target<'_, '_, 'w>,
) -> bool {
    crate::entity::piglin::visible(targets, target)
        && !targets
            .store()
            .has_indirect_passenger(context.entity_id, target.entity_id())
}

#[cfg(test)]
mod tests;
