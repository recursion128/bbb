use super::*;
use crate::entity::ai::mob::MobAiStepKindResult;
use crate::world::simulation_level::LevelCtx;
use bbb_server_types::geometry::Vec3Ops;

pub(super) fn ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    water_contact: Option<FluidContact>,
    ghast_contact: Option<crate::entity::ghast::GhastFluidContact>,
    slime_contact: Option<crate::entity::slime::SlimeFluidContact>,
    bee_contact: Option<bool>,
) -> bool {
    apply_movement_dead_band(entity);
    crate::entity::raider::ai_step(entity, store, level);
    let runs_mob_ai = matches!(
        entity.data(),
        EntityData::GeneralLiving(data) if data.is_mob() && !data.is_no_ai()
    );
    if runs_mob_ai {
        crate::entity::armadillo::scare::tick(entity, store, level);
    }
    let horse_immobile = match entity.data() {
        EntityData::GeneralLiving(data) => data.abstract_horse_retained().map(|horse| {
            if matches!(
                entity.entity_type_id(),
                VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            ) {
                entity
                    .living()
                    .is_none_or(LivingEntityState::is_dead_or_dying)
                    || horse.is_eating()
            } else {
                entity.living().is_none_or(LivingEntityState::is_immobile)
                    || horse.is_eating()
                    || horse.is_standing()
            }
        }),
        _ => None,
    };
    let ravager_immobile = crate::entity::ravager::is_immobile(entity);
    let immobile = {
        let Some(living) = entity.living_mut() else {
            return true;
        };
        apply_input(living);
        let immobile = horse_immobile.unwrap_or_else(|| living.is_immobile()) || ravager_immobile;
        if immobile {
            apply_immobile(living);
        }
        immobile
    };

    let wither_invulnerable = matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.wither_retained().is_some_and(|state| state.invulnerable_ticks() > 0));
    if !immobile && runs_mob_ai && wither_invulnerable {
        crate::entity::wither::custom_server_ai_step(entity, store, level);
    }
    if !immobile && runs_mob_ai && !wither_invulnerable {
        crate::entity::happy_ghast::update_home(entity, store);
        crate::entity::happy_ghast::before_server_ai(entity, level);
        crate::entity::axolotl::before_ai(entity, store, level);
        crate::entity::goat::before_ai(entity);
        crate::entity::allay::before_ai(entity, level);
        crate::entity::warden::anger::before_ai(entity, store, level);
        crate::entity::warden::custom_server_ai_step(entity, store);
        crate::entity::hoglin::tick_ai(entity, store, level);
        crate::entity::fish::tick_schooling(entity, store, level);
        crate::entity::guardian::custom_server_ai_step(entity, store);
        crate::entity::dolphin::custom_server_ai_step(entity, store, level);
        let id = entity.id();
        let position = entity.position();
        let difficulty = store.difficulty();
        if let Some(pos) = store
            .mob_ai_mut(id)
            .and_then(|runtime| runtime.refresh_retained_navigation(entity))
        {
            level.destroy_block_progress(id, pos, -1);
        }
        let Some(control_owner) = store.mob_control_owner(entity, level) else {
            return false;
        };
        let Some((effective_malus, inherits_malus)) =
            store.effective_pathfinding_malus_with_provenance(entity, level)
        else {
            return false;
        };
        let delegate_snapshot = if control_owner != id {
            let has_target = store
                .mob_ai_mut(control_owner)
                .is_some_and(|runtime| runtime.has_target());
            let Some(owner) = store.get(control_owner) else {
                return false;
            };
            let Some(owner_malus) = store.effective_pathfinding_malus(owner, level) else {
                return false;
            };
            let Some(snapshot) =
                crate::entity::ai::mob::navigation::MobPathingSnapshot::from_entity(
                    owner,
                    has_target,
                    store.vehicle(control_owner).is_some(),
                    difficulty,
                    owner_malus,
                )
            else {
                return false;
            };
            Some((
                snapshot,
                owned_ground_delegate_factory(owner.entity_type_id()),
            ))
        } else {
            None
        };
        // The level the body's target goals, its `getTarget()` readers and its
        // anger read while it is out of the store.
        let anger = crate::entity::ai::mob::target::anger::read(entity);
        if let Some(runtime) = store.mob_ai_mut(id) {
            runtime.ensure_brain(entity);
        }
        let prepared =
            store.with_mob_ai_control(id, control_owner, |runtime, delegated_runtime, store| {
                runtime.target_state_mut().pending_anger = anger;
                runtime.prepare_server_ai_step(
                    level,
                    entity,
                    store,
                    inherits_malus,
                    slime_contact,
                    delegated_runtime.zip(delegate_snapshot).map(
                        |(runtime, (mob, factory_owned))| {
                            crate::entity::ai::mob::MobNavigationDelegate {
                                runtime,
                                mob,
                                factory_owned,
                            }
                        },
                    ),
                    effective_malus,
                )
            });
        let Some(mut prepared) = prepared.flatten() else {
            return false;
        };
        if prepared
            .apply_pre_control(entity, store, level, bee_contact)
            .is_none()
        {
            return false;
        }
        let brain_effects = prepared.take_brain_effects();
        let post_step = prepared.take_post_step_actions();
        if !crate::entity::ai::mob::brain_runtime::apply_brain_effects(
            entity,
            store,
            level,
            brain_effects,
        ) {
            return false;
        }
        crate::entity::ai::mob::villager::hero::tick(entity, store, level);
        crate::entity::axolotl::after_ai(entity);
        let can_move =
            entity.creaking_can_move() && !crate::entity::axolotl::is_playing_dead(entity);
        // `Camel.java:674-685` leaves rider rotation to the controller.
        let can_look = if matches!(
            entity.entity_type_id(),
            VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        ) {
            let Some(controller) =
                crate::entity::passenger::direct_controlling_living_with_registry_tags(
                    entity,
                    Some(store),
                    &level.capture_registry_tags(),
                    |id| level.player_by_entity_id(id),
                )
            else {
                return false;
            };
            controller.is_none()
        } else {
            can_move
        };
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!("prepared mob retained living state");
        };
        if data.ageable.is_some_and(|ageable| ageable.age() != 0)
            && animal_custom_server_ai_step_calls_super(data.entity_resource_id)
        {
            if let Some(animal) = data.animal.as_mut() {
                animal.reset_love();
            }
        }
        let can_tick_move_control = data
            .panda_retained()
            .is_none_or(|panda| panda.can_perform_action(level.is_thundering()))
            && data
                .fox_retained()
                .is_none_or(crate::entity::fox::FoxRetainedState::can_move);
        let runtime = store
            .mob_ai_mut(id)
            .expect("prepared mob runtime retained during Brain replay");
        let result = runtime.finish_server_ai_step(
            prepared,
            level,
            entity,
            can_move,
            can_look,
            can_tick_move_control,
        );
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!("finished mob retained living state");
        };
        let creeper_direction_changed = result
            .creeper_swell_dir
            .zip(data.creeper_retained_mut())
            .is_some_and(|(direction, creeper)| creeper.set_swell_dir(direction));
        let wolf_interested_changed = result
            .wolf_interested
            .is_some_and(|interested| data.set_wolf_interested(interested));
        entity.set_rot(result.y_rot, result.x_rot);
        if let Some(no_gravity) = result.no_gravity {
            entity.set_no_gravity(no_gravity);
        }
        if wolf_interested_changed {
            entity.mark_synched_dirty(
                bbb_protocol::entity_synched_data::VANILLA_WOLF_INTERESTED_DATA_ID,
            );
        }
        if creeper_direction_changed {
            entity.mark_synched_dirty(VANILLA_CREEPER_SWELL_DIR_DATA_ID);
        }
        if !crate::entity::ai::mob::apply_post_step_actions(entity, store, level, post_step) {
            return false;
        }
        if let Some(output) = store
            .mob_ai_mut(id)
            .and_then(|runtime| runtime.target_state_mut().output.take())
        {
            crate::entity::ai::mob::target::apply_target_output(
                entity, store, level, output, position,
            );
        }
        if let Some(moving) = store
            .mob_ai(id)
            .and_then(crate::entity::ai::mob::MobAiRuntime::guardian_moving)
        {
            crate::entity::guardian::set_moving(entity, moving);
        }
        if result.swimming_impulse != Vec3d::default() {
            let movement = entity.delta_movement();
            entity.set_delta_movement(Vec3d {
                x: movement.x + result.swimming_impulse.x,
                y: movement.y + result.swimming_impulse.y,
                z: movement.z + result.swimming_impulse.z,
            });
        }
        if let Some(result) = result.patrol {
            crate::entity::raider::apply_patrol_step(entity, store, result);
        }
        match result.kind {
            MobAiStepKindResult::None => {}
            MobAiStepKindResult::Bat(output) => apply_bat_ai_step(entity, level, output),
            MobAiStepKindResult::Motion { movement, y_rot } => {
                entity.set_delta_movement(movement);
                entity.set_rot(y_rot, entity.x_rot());
            }
            MobAiStepKindResult::RabbitJump(pitch) => {
                crate::entity::rabbit::play_jump_sound(entity, store, pitch);
            }
            MobAiStepKindResult::Slime { y_rot, jump_sound } => {
                crate::entity::slime::apply_control_rotation(entity, y_rot);
                if let Some(sound) = jump_sound {
                    crate::entity::slime::apply_ai_step(entity, store, sound);
                }
            }
            MobAiStepKindResult::Blaze(output) => {
                crate::entity::blaze::apply_ai_step(entity, store, level, output);
            }
            MobAiStepKindResult::Witch(action) => {
                crate::entity::witch::apply_ranged_attack(entity, store, action);
            }
            MobAiStepKindResult::Graze(action) => {
                if !crate::entity::sheep::apply_grazing(entity, store, level, action) {
                    return false;
                }
            }
        }
        if result.horse_stand_requested {
            apply_abstract_horse_random_stand(entity, store);
        }
        if let Some(outcome) = result.run_around_outcome {
            if !apply_run_around_like_crazy(entity, store, level, outcome) {
                return false;
            }
        }
        crate::entity::wither::custom_server_ai_step(entity, store, level);
        crate::entity::gift_loot::armadillo_custom_server_ai_step(entity, store, level);
        crate::entity::ai::mob::villager::raid::tick_raid_actions(entity, store, level);
        crate::entity::turtle::custom_server_ai_step(entity, store, level);
        crate::entity::sniffer::custom_server_ai_step(entity, store, level);
    }
    if crate::entity::armadillo::is_scared(entity) || crate::entity::mannequin::is_immovable(entity)
    {
        store.stop_mob_navigation(entity.id());
        if let Some(living) = entity.living_mut() {
            apply_immobile(living);
        }
    }
    if !crate::entity::rabbit::jump_from_ground(entity, store, level)
        && !slime_contact
            .is_some_and(|contact| crate::entity::slime::tick_jump(entity, level, contact))
    {
        tick_jump(entity, level);
    }
    // Mob.java:1278-1279; LivingEntity.java:3061-3064; Mannequin.java:115-122.
    if matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_mob() && data.is_no_ai())
        || crate::entity::mannequin::is_immovable(entity)
    {
        return true;
    }
    if let Some(contact) = ghast_contact {
        return crate::entity::ghast::travel(entity, store, level, contact);
    }
    if slime_contact.is_some_and(crate::entity::slime::blocks_unowned_fluid_travel) {
        return true;
    }
    if matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
    ) {
        let movement = entity.delta_movement();
        return crate::entity::movement::move_living_entity(
            entity,
            store,
            level,
            MoverType::Self_,
            movement,
        )
        .is_ok();
    }
    let Some(living) = entity.living() else {
        return true;
    };
    let mut input = Vec3d {
        x: f64::from(living.xxa),
        y: f64::from(living.yya),
        z: f64::from(living.zza),
    };
    crate::entity::camel::apply_travel_gate(entity, level.game_time(), &mut input);
    // Allay.java:149-151.
    if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID {
        let contact = if entity.is_in_water() {
            living::FlyingContact::Water
        } else if entity.is_in_lava() {
            living::FlyingContact::Lava
        } else {
            living::FlyingContact::Air
        };
        let speed = entity.living().expect("allay living state").speed();
        return living::travel_flying(entity, store, level, input, contact, speed, 0.02).is_ok();
    }
    match crate::entity::turtle::travel(entity, store, level, input) {
        Ok(true) => return true,
        Ok(false) => {}
        Err(()) => return false,
    }
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_FROG_ID && entity.is_in_water() {
        return super::frog_travel::travel_in_water(entity, store, level, input).is_ok();
    }
    if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DROWNED_ID
        && entity.is_in_water()
    {
        return drowned_water_travel(entity, store, level, input).is_ok();
    }
    let guardian_moving = match entity.data() {
        EntityData::GeneralLiving(data) => data.guardian_state().map(|state| state.moving),
        _ => None,
    };
    let nautilus = matches!(
        entity.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    );
    let axolotl =
        entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
    if nautilus
        || axolotl
        || is_water_animal_entity_type(entity.entity_type_id())
        || entity.entity_type_id() == VANILLA_ENTITY_TYPE_DOLPHIN_ID
        || guardian_moving.is_some()
    {
        match water_contact
            .or_else(|| (nautilus || axolotl).then(|| FluidContact::Known(entity.is_in_water())))
        {
            Some(FluidContact::Known(true)) => {
                // Dolphin.java:355-361; AbstractFish.java:105-110; Guardian.java:335-342; AbstractNautilus.java:218-223.
                let speed = if guardian_moving.is_some() {
                    0.1
                } else if nautilus
                    || axolotl
                    || entity.entity_type_id() == VANILLA_ENTITY_TYPE_DOLPHIN_ID
                {
                    entity.living().expect("living travel").speed()
                } else {
                    0.01
                };
                move_relative(entity, speed, input);
                let movement = entity.delta_movement();
                if crate::entity::movement::move_living_entity(
                    entity,
                    store,
                    level,
                    MoverType::Self_,
                    movement,
                )
                .is_err()
                {
                    return false;
                }
                let movement = entity.delta_movement();
                // Axolotl.java:533-536: current speed, .9 drag, no sinking impulse.
                let sink = !nautilus
                    && !axolotl
                    && guardian_moving.is_none_or(|moving| !moving)
                    && !store
                        .mob_ai(entity.id())
                        .is_some_and(|runtime| runtime.has_target());
                entity.set_delta_movement(Vec3d {
                    x: movement.x * 0.9,
                    y: movement.y * 0.9 - if sink { 0.005 } else { 0.0 },
                    z: movement.z * 0.9,
                });
            }
            Some(FluidContact::Known(false)) => {
                let Ok(()) = travel_in_air(entity, store, level, input) else {
                    return false;
                };
            }
            Some(FluidContact::Unknown) | None => {}
        }
        return true;
    }
    if entity.is_in_water() {
        return crate::entity::living::travel_in_water(entity, store, level, input).is_ok();
    }
    if entity.is_in_lava() {
        let stands_on_lava = entity.entity_type_id()
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID
            && bbb_server_types::fluid::block_state_fluid_state(
                level.block_state(entity.block_position()),
            )
            .family()
                == Some(bbb_server_types::fluid::FluidFamily::Lava);
        if !stands_on_lava {
            return crate::entity::living::travel_in_lava(entity, store, level, input).is_ok();
        }
    }
    let Ok(()) = travel_in_air(entity, store, level, input) else {
        return false;
    };
    true
}

fn drowned_water_travel(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    input: Vec3d,
) -> Result<(), ()> {
    // Drowned.java:241-248.
    if entity.is_swimming() {
        move_relative(entity, 0.01, input);
        let movement = entity.delta_movement();
        crate::entity::movement::move_living_entity(
            entity,
            store,
            level,
            MoverType::Self_,
            movement,
        )?;
        entity.set_delta_movement(entity.delta_movement().scale(0.9));
        return Ok(());
    }
    crate::entity::living::travel_in_water(entity, store, level, input)
}

fn owned_ground_delegate_factory(entity_type_id: i32) -> bool {
    crate::entity::ai::mob::has_owned_navigation(entity_type_id)
}

pub(super) fn update_control_flags(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    if entity.tick_count() % 5 != 0 {
        return true;
    }
    let Some(controller) = store.controlling_mob_passenger(entity, level) else {
        return false;
    };
    let ravager =
        entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RAVAGER_ID;
    let no_controller = if let Some(id) = controller {
        if ravager {
            let Some(passenger) = store.get(id) else {
                return false;
            };
            level.registry_tag_membership(
                "minecraft:entity_type",
                "minecraft:raiders",
                passenger.data().entity_resource_id(),
            )
        } else {
            false
        }
    } else {
        true
    };
    let no_controller = no_controller && !crate::entity::armadillo::is_scared(entity);
    let not_in_boat = store
        .vehicle(entity.id())
        .and_then(|id| store.get(id))
        .is_none_or(|vehicle| !crate::entity::boat::is_boat(vehicle.data()));
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.after_living_tick(entity.tick_count(), no_controller, not_in_boat, ravager);
    }
    true
}
