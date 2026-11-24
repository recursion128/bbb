use super::*;
use crate::entity::{slime::SlimeFluidContact, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(crate) struct PreparedMobAiStep {
    context: MobAiContext,
    brain_effects: Vec<crate::entity::ai::BrainWorldEffect>,
}

impl PreparedMobAiStep {
    fn bee_custom_step(
        &mut self,
        body: &mut Entity,
        store: &mut crate::entity::store::EntityStore,
        level: &mut LevelCtx<'_, '_>,
        contact: Option<bool>,
    ) -> Option<()> {
        if body.entity_type_id() != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID {
            return Some(());
        }
        let output = self.context.target.take_output();
        target::apply_target_output(body, store, level, output, body.position());
        if !crate::entity::bee::custom_server_ai_step(body, store, level, contact)
            || !target::sense::bee_custom_step(body, store, level)
        {
            return None;
        }
        let anger = target::anger::read(body);
        let current = store.mob_ai(body.id())?.target_ref();
        self.context.target.target = current;
        self.context.target.anger = anger;
        if !body.is_alive() {
            return Some(());
        }
        let runtime = store.mob_ai_mut(body.id())?;
        let busy = current.is_some()
            || anger.is_some_and(|anger| anger.is_angry(level.game_time()))
            || runtime
                .goal_selector
                .is_running(|goal| matches!(goal, MobGoal::Breed(_) | MobGoal::Tempt(_)));
        let mob = MobPathingSnapshot::from_context(&self.context);
        let navigation_done = runtime.navigation.is_done();
        let movement = {
            let bee = runtime.kind.bee_mut()?;
            let navigation = &mut runtime.navigation;
            bee.foraging_step(body, level, busy, navigation_done, |level, target| {
                navigation
                    .create_path(level, mob, &runtime.random, target, 1)
                    .is_some_and(|path| path.reached())
            })
        };
        use crate::entity::bee::foraging::ForagingMovement;
        match movement {
            ForagingMovement::Idle { towards } => {
                // `BeeWanderGoal` yields to every
                // other movement goal the bee has.
                let moving = current.is_some()
                    || runtime.goal_selector.is_running(|goal| {
                        matches!(
                            goal,
                            MobGoal::MeleeAttack(_)
                                | MobGoal::Breed(_)
                                | MobGoal::Tempt(_)
                                | MobGoal::FollowParent(_)
                        )
                    });
                if !moving && navigation_done && runtime.random.next_int(10) == 0 {
                    let direction = towards.unwrap_or_else(|| {
                        stroll::view_vector(self.context.body.x_rot, self.context.body.y_head_rot)
                    });
                    let wander = stroll::flying_position(
                        level,
                        &runtime.random,
                        random_pos::RandomPosMob::of(body, self.context.nav.malus),
                        direction,
                    );
                    if let Some(Some(wanted)) = wander {
                        runtime.navigate_to(
                            level,
                            mob,
                            navigation::containing_block(wanted),
                            1,
                            1.0,
                        );
                    }
                }
            }
            ForagingMovement::None => {}
            ForagingMovement::Stop => {
                runtime.navigation.stop();
                runtime.navigation.set_max_visited_nodes_multiplier(1.0);
            }
            ForagingMovement::Navigate(target, speed, multiplier) => {
                runtime
                    .navigation
                    .set_max_visited_nodes_multiplier(multiplier);
                runtime.navigate_to(level, mob, target, 1, speed);
            }
            ForagingMovement::Hover { wanted, look } => {
                if let Some(wanted) = wanted {
                    runtime.navigation.stop();
                    runtime.move_control.set_wanted_position(wanted, 0.35);
                }
                if let Some(look) = look {
                    self.context.set_look_at(look.x, look.y, look.z);
                }
            }
            ForagingMovement::Enter {
                pos,
                nectar,
                flower,
            } => {
                level.bee_enter_hive(body, store, pos, nectar, flower);
            }
        }
        Some(())
    }

    pub(crate) fn take_brain_effects(&mut self) -> Vec<crate::entity::ai::BrainWorldEffect> {
        std::mem::take(&mut self.brain_effects)
    }

    pub(crate) fn take_post_step_actions(&mut self) -> Vec<MobPostStepAction> {
        std::mem::take(&mut self.context.post_step)
    }

    pub(crate) fn apply_pre_control(
        &mut self,
        body: &mut Entity,
        store: &mut crate::entity::store::EntityStore,
        level: &mut LevelCtx<'_, '_>,
        bee_contact: Option<bool>,
    ) -> Option<()> {
        if let Some(position) = self.context.controls.teleport.take() {
            body.set_pos(position);
            store.refile_detached(body);
            body.set_old_pos_and_rot();
        }
        body.set_rot(self.context.body.y_rot, self.context.body.x_rot);
        for request in std::mem::take(&mut self.context.species.enderman_teleports) {
            if crate::entity::enderman::teleport(body, store, level, request.destination)? {
                self.context.stop_navigation();
                if request.reset_timer_on_success {
                    self.context.target.enderman_look.teleport_succeeded();
                }
            }
        }
        self.context.body.position = body.position();
        self.context.body.bounding_box = body.bounding_box();
        self.context.body.eye_y = body.position().y + f64::from(body.eye_height());
        if let Some(runtime) = store.mob_ai_mut(body.id()) {
            runtime.target_state_mut().enderman_look = self.context.target.enderman_look.clone();
        }
        if body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID {
            enderman::custom_step(&mut self.context, body, store, level)?;
        }
        self.bee_custom_step(body, store, level, bee_contact)
    }
}

pub(crate) fn apply_post_step_actions(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    actions: Vec<MobPostStepAction>,
) -> bool {
    for action in actions {
        match action {
            MobPostStepAction::MeleeAttack { target_id } => {
                if !super::melee::do_hurt_target(entity, store, level, target_id) {
                    return false;
                }
            }
            MobPostStepAction::RangedAttack { target, power } => {
                super::ranged::perform::ranged_attack(entity, store, level, target, power);
            }
            MobPostStepAction::GuardianAttack { target_id } => {
                if !crate::entity::guardian::attack(entity, store, level, target_id) {
                    return false;
                }
            }
            MobPostStepAction::SpearStab { strikes } => {
                super::spear_use::stab(entity, store, level, strikes);
            }
            MobPostStepAction::GhastShoot { origin, direction } => {
                super::ranged::perform::ghast_shoot(entity, store, origin, direction);
            }
            MobPostStepAction::ShulkerShoot {
                target,
                attach_axis,
            } => {
                super::ranged::perform::shulker_shoot(entity, store, level, target, attach_axis);
            }
            MobPostStepAction::CastSpell(cast) => {
                super::ranged::perform::cast_spell(entity, store, level, cast);
            }
            MobPostStepAction::GoatRamAttack {
                target_id,
                knockback,
                direction,
            } => {
                crate::entity::goat::ram::apply_ram_hit(
                    entity, store, level, target_id, knockback, direction,
                );
            }
            MobPostStepAction::WardenSonicFire { target_id } => {
                crate::entity::warden::sonic_boom::fire(entity, store, target_id);
            }
            MobPostStepAction::WardenSniffEnd => {
                crate::entity::warden::awareness::end_sniff(entity, store, level);
            }
            MobPostStepAction::AllayDeliverItem => {
                crate::entity::allay::delivery::deliver(entity, store, level);
            }
            MobPostStepAction::SpawnChildFromBreeding { partner_id } => {
                let random = store.random().clone();
                if !crate::entity::animal_breeding::spawn_child_from_breeding(
                    entity, store, level, partner_id, &random,
                ) {
                    return false;
                }
            }
        }
    }
    true
}

impl MobAiRuntime {
    /// The body's `Mob.target` and target-goal state as the step left them.
    fn sync_target_state(&mut self, context: &MobAiContext) {
        self.target_state.bee = context.target.bee.clone();
        self.target_state.enderman_look = context.target.enderman_look.clone();
        self.target_state.target = context.target.target;
        self.target_state.target_change_time = context.target.target_change_time;
        self.target_state.counters = context.target.counters;
        if let (Some(witch), Some(cooldown)) =
            (self.kind.witch_mut(), context.target.heal_raider_cooldown)
        {
            witch.set_heal_raider_cooldown(cooldown);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_server_ai_step(
        &mut self,
        level: &mut LevelCtx<'_, '_>,
        entity: &mut Entity,
        store: &mut EntityStore,
        inherits_malus: bool,
        slime_contact: Option<SlimeFluidContact>,
        mut delegated: Option<MobNavigationDelegate<'_>>,
        effective_malus: PathfindingMalus,
    ) -> Option<PreparedMobAiStep> {
        let entity_id = entity.id();
        let tick_count = entity.tick_count();
        self.sensing.tick();
        if let (Some(look), crate::entity::EntityData::GeneralLiving(data)) =
            (&mut self.look_control, entity.data())
        {
            if let Some(shulker) = data.shulker_state() {
                look.set_shulker_attach(shulker.attach_face());
            }
        }
        {
            let living = entity.living_mut().expect("mob has living state");
            living.no_action_time = living.no_action_time.wrapping_add(1);
        }
        self.vindicator_door_step(entity, store, level);
        let mut context = MobAiContext::from_living_body(
            entity_id,
            tick_count,
            entity.position(),
            entity.bounding_box(),
            crate::entity::slime::eye_y(entity)
                .unwrap_or_else(|| entity.position().y + f64::from(entity.eye_height())),
            entity.x_rot(),
            entity.y_rot(),
            entity.on_ground(),
            entity.is_in_water() || entity.is_in_lava(),
            store.vehicle(entity_id).is_some(),
            entity.living_mut().expect("mob has living state"),
            self.random.clone(),
        );
        if let EntityData::GeneralLiving(data) = entity.data() {
            context.species.creeper_swell_dir =
                data.creeper_retained().map(|creeper| creeper.swell_dir());
            context.species.blaze_charged = data
                .blaze_retained()
                .is_some_and(crate::entity::blaze::BlazeRetainedState::charged);
            if let Some(fox) = data.fox_retained() {
                context.species.fox_reset_pitch = fox.resets_pitch();
                context.species.fox_can_look = !fox.is_sleeping();
            }
        }
        context.species.raider_banner = self.raider_banner.take();
        context.nav.malus = effective_malus;
        context.nav.inherits_malus = inherits_malus;
        context.body.creaking_home_pos = entity.creaking_home_pos();
        context.body.in_water = entity.is_in_water();
        context.body.in_lava = entity.is_in_lava();
        context.body.on_fire = entity.is_on_fire();
        context.body.eye_in_water =
            entity.is_eye_in_fluid(crate::entity::environment::FluidTag::Water);
        let can_move = entity.creaking_can_move();
        let mut own_malus = self.pathfinding_malus;
        context.body.water_fluid_height =
            entity.fluid_height(crate::entity::environment::FluidTag::Water);
        context.difficulty = store.difficulty();
        context.ranged = self.ranged.with_saved_casting_ticks(entity);
        context.target = target::TargetPhase {
            bee: self.target_state.bee.clone(),
            enderman_look: self.target_state.enderman_look.clone(),
            target: self.target_state.target,
            target_change_time: self.target_state.target_change_time,
            heal_raider_cooldown: self.kind.witch().map(|witch| witch.heal_raider_cooldown()),
            anger: self.target_state.pending_anger.take(),
            counters: self.target_state.counters,
            ..target::TargetPhase::default()
        };
        context.body.delta_movement = entity.delta_movement();
        context.species.player_attack_allowed =
            self.kind.witch().is_none_or(|witch| witch.attack_players);
        context.species.slime_in_fluid = matches!(
            slime_contact,
            Some(SlimeFluidContact::Water(_) | SlimeFluidContact::Lava(_))
        );
        // CopperGolemAi: the idle stroll waits for the transport cooldown.
        context.nav.stroll_allowed = stroll::level_weighs_positions(level)
            && self
                .kind
                .copper_golem()
                .is_none_or(|transport| transport.resting(tick_count));
        let navigation = delegated
            .as_ref()
            .map_or(&self.navigation, |owner| &owner.runtime.navigation);
        context.nav.navigation_done = navigation.is_done();
        context.nav.last_navigation_failed = self.last_navigation_failed;
        context.nav.navigation_target = navigation.path().map(GroundPath::target);
        if let Some(squid) = self.kind.squid() {
            context.species.squid_movement_vector = Some(squid.movement_vector());
        }
        if let Some(vex) = self.kind.vex() {
            context.species.vex_wanted_position = vex.wanted_position();
        }
        if let Some(ghast) = self.kind.ghast().filter(|_| self.uses_ghast_move_control()) {
            ghast.prepare_step(&mut context, entity);
        }
        context.nav.patrol_cooldown_until = self.patrol_cooldown_until;
        context.species.breed_goal_running = self.breed_goal_running();
        let door_navigation_owned = self.kind.doors().is_none_or(|doors| doors.navigation_owned);
        context.nav.door = self
            .kind
            .doors()
            .filter(|doors| doors.can_break)
            .map(|doors| DoorContext::new(doors.state, entity.horizontal_collision()));
        let mut navigation_context_ready = true;
        let was_panicking = self.panic_goal_running();
        let mut door_effects = Vec::new();
        let piglin_sensing_due = self.piglin().is_some_and(|piglin| piglin.sensing_due());
        let piglin_avoid = self.piglin().and_then(|piglin| piglin.retreat_target());
        let piglin_zombified = self.piglin().and_then(|piglin| piglin.nearest_zombified());
        let piglin_repellent = self.piglin().and_then(|piglin| piglin.nearest_repellent());
        let piglin_ride_target = self.piglin().and_then(|piglin| piglin.ride_target());
        let mut resolve_navigation = |context: &mut MobAiContext,
                                      targets: &mut target::TargetView<'_, '_, '_>|
         -> bool {
            if targets.is_aborted() {
                navigation_context_ready = false;
                return false;
            }
            let mut admitted = true;
            for (path_type, malus) in std::mem::take(&mut context.nav.own_malus_writes) {
                own_malus.set(path_type, malus);
                if !context.nav.inherits_malus {
                    context.nav.malus.set(path_type, malus);
                }
            }
            if follow_owner::resolve_request(context, targets).is_none() {
                navigation_context_ready = false;
            }
            let (navigation, mob, factory_owned) = if let Some(owner) = delegated.as_mut() {
                if !owner.factory_owned
                    && (context.nav.navigation_request.is_some()
                        || context.nav.patrol_request.is_some())
                {
                    navigation_context_ready = false;
                    return admitted;
                }
                (
                    &mut owner.runtime.navigation,
                    Some(owner.mob),
                    owner.factory_owned,
                )
            } else {
                (&mut self.navigation, None, door_navigation_owned)
            };
            if let Some(door) = context.nav.door.as_mut() {
                navigation_context_ready &= !door.unavailable;
                door_effects.extend(std::mem::take(&mut door.effects));
            }
            let level = targets.level();
            let request_mob = mob.unwrap_or_else(|| MobPathingSnapshot::from_context(context));
            match context.nav.navigation_request.take() {
                Some(NavigationRequest::MoveToEntity {
                    target,
                    reach_range,
                    speed_modifier,
                }) => {
                    let moved = navigation.move_to_entity(
                        level,
                        request_mob,
                        &context.random,
                        target,
                        reach_range,
                        speed_modifier,
                    );
                    context.nav.last_navigation_failed = !moved;
                }
                Some(NavigationRequest::MoveToPosition {
                    target,
                    reach_range,
                    speed_modifier,
                }) => {
                    let moved = navigation.move_to(
                        level,
                        request_mob,
                        &context.random,
                        target,
                        reach_range,
                        speed_modifier,
                    );
                    context.nav.last_navigation_failed = !moved;
                }
                Some(NavigationRequest::MoveToPositionAfterStop {
                    target,
                    reach_range,
                    speed_modifier,
                }) => {
                    navigation.stop();
                    let moved = navigation.move_to(
                        level,
                        request_mob,
                        &context.random,
                        target,
                        reach_range,
                        speed_modifier,
                    );
                    context.nav.last_navigation_failed = !moved;
                }
                Some(NavigationRequest::TryMoveToPosition {
                    target,
                    reach_range,
                    speed_modifier,
                }) => {
                    let moved = navigation.try_move_to(
                        level,
                        request_mob,
                        &context.random,
                        target,
                        reach_range,
                        speed_modifier,
                    );
                    context.nav.last_navigation_failed = !moved;
                    admitted &= moved;
                }
                Some(NavigationRequest::RequirePath {
                    target,
                    reach_range,
                }) => {
                    admitted &= navigation
                        .create_path_to(
                            level,
                            request_mob,
                            &context.random,
                            target,
                            reach_range,
                            None,
                        )
                        .is_some();
                }
                Some(NavigationRequest::ThroughVillage {
                    target,
                    speed_modifier,
                    can_break_doors,
                }) => {
                    if let Some(ground) = navigation.ground_mut() {
                        ground.move_through_village(
                            level,
                            request_mob,
                            target,
                            speed_modifier,
                            can_break_doors,
                        );
                    }
                }
                Some(NavigationRequest::Stop) => {
                    navigation.stop();
                    context.nav.last_navigation_failed = false;
                }
                None => {}
            }
            if let Some(speed_modifier) = context.nav.navigation_speed_request.take() {
                navigation.set_speed_modifier(speed_modifier);
            }
            if let Some(avoid_sun) = context.nav.avoid_sun_request.take() {
                if let Some(ground) = navigation.ground_mut() {
                    ground.set_avoid_sun(avoid_sun);
                }
            }
            if let Some(ground) = navigation.ground_mut() {
                if let Some(reached) =
                    super::raider_banner::resolve_path(context, targets, ground, mob)
                {
                    admitted &= reached;
                }
                if let Some(reached) = super::goat::resolve_path(context, targets, ground, mob) {
                    admitted &= reached;
                }
                if let Some(reached) =
                    super::goat::long_jump::resolve_path(context, targets, ground, mob)
                {
                    admitted &= reached;
                }
                navigation_context_ready &=
                    patrol::resolve_navigation(context, targets, ground, mob).is_some();
                let level = targets.level();
                if let Some(door) = context.nav.door.as_mut() {
                    door.refresh(level, ground, context.body.position, factory_owned);
                }
            }
            if let Some(prepared) = navigation.take_amphibious_prepare() {
                prepared.apply(&mut own_malus);
                if !context.nav.inherits_malus {
                    prepared.apply(&mut context.nav.malus);
                }
            }
            admitted
        };
        let mut targets = target::TargetView::new_mut(entity, store, level, &self.sensing);
        if matches!(
            targets.body().entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
        ) {
            self.goal_selector
                .observe_nautilus_damage(&mut context, &targets);
        }
        if matches!(
            targets.body().entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
        ) {
            let sensing_due = piglin_sensing_due
                || (targets.body().entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
                    && (targets.body_entity().tick_count() <= 1
                        || targets.body_entity().tick_count().rem_euclid(20) == 0));
            let target = crate::entity::piglin::attack_target(
                &targets,
                context.target.target,
                sensing_due,
                piglin_avoid,
                piglin_zombified,
            );
            context.set_target(target, &targets);
        }
        // HoglinAi: a breeding hoglin drops and seeks no target.
        let pacified = crate::entity::hoglin::is_pacified(targets.body_entity())
            || (targets.body().entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID
                && context.species.breed_goal_running);
        let retreating =
            crate::entity::hoglin::is_fleeing(targets.body_entity(), context.target.target);
        if pacified || retreating {
            context.set_target(None, &targets);
        }
        let _ = resolve_navigation(&mut context, &mut targets);
        let reduced = tick_count.wrapping_add(entity_id) % 2 != 0
            && tick_count > 1
            && !crate::entity::axolotl::has_play_dead_ticks(targets.body_entity());
        let retaliating = !retreating
            && target::retaliation::retain_retaliation(
                &mut self.target_state,
                &mut context,
                &targets,
            );
        context.species.piglin_ride_target = piglin_ride_target.filter(|_| {
            context.target.target.is_none()
                && !crate::entity::piglin::is_admiring(targets.body_entity())
                && piglin_avoid.is_none()
                && crate::entity::piglin::celebration(targets.body_entity()).is_none()
        });
        context.species.piglin_idle_allowed =
            piglin::idle_eligible(&context, &targets, piglin_avoid, piglin_repellent);
        if reduced {
            if !retaliating && !pacified && !retreating {
                self.target_selector.tick_running(
                    &mut context,
                    false,
                    &mut resolve_navigation,
                    &mut targets,
                );
            }
            self.goal_selector.tick_running(
                &mut context,
                false,
                &mut resolve_navigation,
                &mut targets,
            );
        } else {
            if !retaliating && !pacified && !retreating {
                self.target_selector.tick_with_effects(
                    &mut context,
                    &mut resolve_navigation,
                    &mut targets,
                );
            }
            self.goal_selector.tick_with_effects(
                &mut context,
                &mut resolve_navigation,
                &mut targets,
            );
        }
        if hoglin::avoid_threat(&mut context, &targets) {
            let _ = resolve_navigation(&mut context, &mut targets);
        }
        if piglin::avoid_threat(&mut context, &targets, piglin_avoid, piglin_repellent) {
            let _ = resolve_navigation(&mut context, &mut targets);
        }
        if piglin::approach_celebration(&mut context, &targets, piglin_avoid) {
            let _ = resolve_navigation(&mut context, &mut targets);
        }
        if targets.body().entity_type_id()
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
        {
            target::anger::zombified_piglin_custom_step(&mut context, &mut targets);
        }
        if let Some(blaze) = self.kind.blaze_mut() {
            blaze.custom_server_ai_step(&mut context, &mut targets);
        }
        // `SpellcasterIllager.customServerAiStep`.
        self.ranged = context.ranged.after_goals();
        let aborted = targets.is_aborted();
        let _ = targets;
        drop(resolve_navigation);
        if aborted {
            return None;
        }
        crate::entity::raider::set_spell_casting(entity, self.ranged.spell());
        self.ranged.save_casting_ticks(entity);
        if let Some(step) = self.ranged.take_crossbow_charge() {
            super::ranged::perform::apply_crossbow_charge(
                entity,
                store,
                step,
                &level.value_effect_level(),
            );
        }
        self.raider_banner = context.species.raider_banner.take();
        if let Some(change) = context.species.caravan_change.take() {
            store.queue_command(crate::entity::EntityCommand::LlamaCaravan(
                crate::entity::llama::PendingLlamaCaravan {
                    follower: target::TargetRef {
                        entity_id,
                        uuid: entity.uuid(),
                    },
                    change,
                },
            ));
        }
        if let Some(lying) = context.species.cat_lying {
            cat::apply_lying(entity, lying);
        }
        if let Some(relaxing) = context.species.cat_relaxing {
            cat::apply_relaxing(entity, relaxing);
        }
        if let Some(sleeping) = context.species.fox_sleep {
            crate::entity::fox::set_sleeping(entity, sleeping);
        }
        if let Some(sitting) = context.species.cat_sitting {
            cat::apply_sitting(entity, sitting);
        }
        if let Some(item) = self
            .raider_banner
            .as_mut()
            .and_then(super::raider_banner::take_pickup)
        {
            crate::entity::mob_pickup::pick_up_raider_banner(entity, store, level, item)?;
        }
        if !was_panicking
            && self.panic_goal_running()
            && matches!(
                entity.entity_type_id(),
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            )
        {
            crate::entity::camel::stand_up_instantly(entity, store, level)?;
        }
        for effect in door_effects {
            super::door::apply_effect(effect, entity, store, level);
        }
        if !navigation_context_ready {
            return None;
        }
        if std::mem::take(&mut context.target.reset_no_action_time) {
            entity
                .living_mut()
                .expect("mob has living state")
                .no_action_time = 0;
        }
        self.pathfinding_malus = own_malus;
        if let (Some(door), Some(doors)) = (context.nav.door.take(), self.kind.doors_mut()) {
            doors.state = door.state;
        }
        self.patrol_cooldown_until = context.nav.patrol_cooldown_until;
        self.last_navigation_failed = context.nav.last_navigation_failed;
        self.sync_target_state(&context);
        if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DROWNED_ID {
            context.species.drowned_swim = self.drowned_swim_intent(entity, store, level);
        }
        if can_move {
            if delegated.is_none() {
                if let Some(movement) = self.navigation.tick(level, &mut context, false) {
                    self.move_control
                        .set_wanted_position(movement.wanted, movement.speed_modifier);
                }
            }
            if let Some(prepared) = self.navigation.take_amphibious_prepare() {
                prepared.apply(&mut self.pathfinding_malus);
                if !context.nav.inherits_malus {
                    prepared.apply(&mut context.nav.malus);
                }
            }
            if let Some(movement) = context.controls.move_control_request.take() {
                self.move_control
                    .set_direct_position(movement.wanted, movement.speed_modifier);
            }
            if let Some((forwards, right)) = context.controls.strafe_request.take() {
                if self.navigation.is_done() {
                    self.move_control.strafe(forwards, right);
                }
            }
        }
        if matches!(
            entity.entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        ) && self.move_control.has_move_to()
            && !crate::entity::leash::is_leashed(entity)
        {
            crate::entity::camel::stand_up_if_ready(entity, store, level)?;
        }
        let mut brain_effects = Vec::new();
        let mob = navigation::MobPathingSnapshot::from_context(&context);
        self.tick_brain(
            (entity, store, level.scoreboard_teams()),
            level,
            (mob, context.nav.inherits_malus),
            &context.random,
            &mut brain_effects,
        );
        if let Some(request) = self.take_brain_look_request() {
            context.set_look_at(request.x, request.y, request.z);
        }
        if self.take_brain_jump_request() {
            context.request_jump();
        }
        if can_move
            && context.body.on_ground
            && crate::entity::rabbit::is_evil(entity)
            && self.kind.rabbit().is_some_and(RabbitRuntime::ready_to_hop)
        {
            let target_position = {
                let targets = target::TargetView::new(entity, store, level, &self.sensing);
                context
                    .unchecked_target(&targets)
                    .map(|target| target.position())
            };
            if let Some(target) = target_position.filter(|target| {
                let dx = target.x - context.body.position.x;
                let dy = target.y - context.body.position.y;
                let dz = target.z - context.body.position.z;
                dx * dx + dy * dy + dz * dz < 16.0
            }) {
                let speed = self.move_control.wanted_movement().speed_modifier;
                self.move_control.set_wanted_position(target, speed);
            }
        }
        super::cat::update_movement_pose(
            entity,
            self.move_control
                .has_move_to()
                .then_some(self.move_control.wanted_movement().speed_modifier),
            store,
            level,
        )?;
        if context.species.vex_wanted_position.is_some() {
            let targets = target::TargetView::new(entity, store, level, &self.sensing);
            if let Some(target) = context.unchecked_target(&targets) {
                let position = target.position();
                context.controls.flight_y_rot = -(crate::entity::projectile_hit::mth_atan2(
                    position.x - context.body.position.x,
                    position.z - context.body.position.z,
                ) as f32)
                    * (180.0 / std::f32::consts::PI);
            }
        }
        context.apply(entity.living_mut().expect("mob has living state"));
        Some(PreparedMobAiStep {
            context,
            brain_effects,
        })
    }

    pub(crate) fn finish_server_ai_step(
        &mut self,
        prepared: PreparedMobAiStep,
        level: &LevelCtx<'_, '_>,
        entity: &mut Entity,
        can_move: bool,
        can_look: bool,
        can_tick_move_control: bool,
    ) -> MobAiStepResult {
        let mut context = prepared.context;
        let uses_ghast_move_control = self.uses_ghast_move_control();
        context.body.position = entity.position();
        context.body.bounding_box = entity.bounding_box();
        context.body.eye_y = crate::entity::slime::eye_y(entity)
            .unwrap_or(context.body.position.y + f64::from(entity.eye_height()));
        context.body.x_rot = entity.x_rot();
        context.body.y_rot = entity.y_rot();
        let living = entity.living().expect("prepared mob has living state");
        context.body.y_body_rot = living.y_body_rot;
        context.body.y_head_rot = living.y_head_rot;
        context.controls.xxa = living.xxa;
        context.controls.yya = living.yya;
        context.controls.zza = living.zza;
        context.controls.speed = living.speed();
        context.controls.jumping = living.jumping;
        context.body.immobile = living.is_immobile();
        context.body.health = living.health();
        context.body.max_health = living.max_health();
        context.body.movement_speed = living
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
        context.body.flying_speed = living
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FLYING_SPEED_ID);
        let living = entity.living_mut().expect("prepared mob has living state");
        if let Some(creepy) = context.target.writes.enderman_creepy {
            target::apply_enderman_speed_modifier(living, creepy);
        }
        if let Some(attacking) = context.target.writes.zombified_attacking_speed {
            target::apply_zombified_attacking_speed(living, attacking);
        }
        context.body.movement_speed = living
            .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
        let (mut kind_output, tick_generic_move_control) = match &mut self.kind {
            MobKind::Bat(bat) => (
                MobAiStepKindResult::Bat(bat.custom_server_ai_step(&mut context, entity, level)),
                true,
            ),
            MobKind::Vex(vex) => {
                let bound_origin = match entity.data() {
                    EntityData::GeneralLiving(data) => data
                        .vex_retained()
                        .and_then(crate::entity::general_living::VexRetainedState::bound_origin),
                    _ => None,
                };
                context.resolve_vex_random_movement(bound_origin, level);
                (
                    vex.move_control_tick(&mut context, entity.delta_movement()),
                    false,
                )
            }
            MobKind::Ghast(ghast) if uses_ghast_move_control => {
                (ghast.move_control_tick(&mut context, level), false)
            }
            MobKind::Rabbit(runtime) => {
                let movement = self.move_control.wanted_movement();
                let has_wanted = can_move && self.move_control.has_move_to();
                runtime.custom_server_ai_step(&mut context, entity, movement, has_wanted);
                if can_move {
                    context.nav.navigation_done = self.navigation.is_done();
                    self.move_control
                        .tick_rabbit(level, &mut context, runtime.wants_jump());
                }
                (MobAiStepKindResult::None, false)
            }
            MobKind::Slime(slime) => {
                slime::apply_requests(slime, &mut context);
                (slime.move_control_tick(&mut context, entity), false)
            }
            _ => (MobAiStepKindResult::None, true),
        };
        if can_move && tick_generic_move_control {
            context.nav.navigation_done = self.navigation.is_done();
            if can_tick_move_control {
                self.move_control.tick(level, &mut context);
            }
        }
        if let Some(look_control) = &mut self.look_control {
            if let Some(target) = context.controls.look_target.take() {
                match context.controls.look_limits.take() {
                    Some((y_max_rot_speed, x_max_rot_angle)) => look_control
                        .set_look_at_with_limits(target, y_max_rot_speed, x_max_rot_angle),
                    None => look_control.set_look_at(target),
                }
            }
            let fox_sleeping = context
                .species
                .fox_sleep
                .unwrap_or(!context.species.fox_can_look);
            if can_look && !fox_sleeping {
                look_control.tick(&mut context);
            }
        }
        if let Some(jump_control) = &mut self.jump_control {
            if context.controls.jump_requested {
                jump_control.jump();
            }
            if can_move {
                jump_control.tick(&mut context);
            } else {
                context.controls.jumping = false;
            }
        }
        if let MobKind::Rabbit(runtime) = &mut self.kind {
            if can_move {
                if let Some(pitch) = runtime.jump_control_tick(&mut context) {
                    kind_output = MobAiStepKindResult::RabbitJump(pitch);
                }
            } else {
                context.controls.jumping = false;
                context.controls.jump_requested = false;
            }
        }
        let living = entity.living_mut().expect("prepared mob has living state");
        context.apply(living);
        match &mut self.kind {
            MobKind::Squid(squid) => {
                if let Some(movement_vector) = context.species.squid_movement_vector {
                    squid.set_movement_vector(movement_vector);
                }
            }
            MobKind::Vex(vex) => {
                vex.set_wanted_position(context.species.vex_wanted_position);
            }
            MobKind::Ghast(ghast) if uses_ghast_move_control => {
                ghast.set_wanted_position(context.species.ghast_wanted_position);
            }
            MobKind::Blaze(_) => {
                kind_output = MobAiStepKindResult::Blaze(BlazeAiStepResult {
                    charged: context.species.blaze_charged,
                    delta_movement: context.body.delta_movement,
                    action: context.species.blaze_action,
                });
            }
            MobKind::Witch(_) => {
                if let Some(action) = context.species.witch_action {
                    kind_output = MobAiStepKindResult::Witch(action);
                }
            }
            _ => {}
        }
        self.sync_target_state(&context);
        self.target_state.output = Some(context.target.take_output());
        let result = MobAiStepResult {
            x_rot: context
                .controls
                .body_x_rot_override
                .unwrap_or(context.body.x_rot),
            y_rot: context
                .controls
                .body_y_rot_override
                .unwrap_or(context.body.y_rot),
            no_gravity: context.controls.no_gravity_request.take(),
            swimming_impulse: context.controls.swimming_impulse,
            patrol: context.nav.patrol_result.take(),
            horse_stand_requested: context.species.horse_stand_requested,
            run_around_outcome: context.species.run_around_outcome,
            kind: context
                .species
                .sheep_action
                .map_or(kind_output, MobAiStepKindResult::Graze),
            creeper_swell_dir: context.species.creeper_swell_dir,
            wolf_interested: context.species.wolf_interested,
        };
        self.navigation.update_pathfinder_max_visited_nodes(
            living
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID),
        );
        result
    }
}
