//! Dolphin swimming, air seeking, and return to water.

use super::navigation::NavigationMove;
use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};
use crate::block_behavior::weathering::within_manhattan;
use crate::entity::level::fluid_state_tag_membership;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_path_facts,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_BUBBLE_COLUMN_ID, VANILLA_BLOCK_WATER_ID};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::directions::{horizontal_direction, relative};
use bbb_server_types::fluid::{
    block_state_blocks_motion, block_state_fluid_state, block_state_is_air,
};
use bbb_server_types::geometry::Vec3Ops;

use super::body_rotation::{base_body_rotation_max_yaw, base_max_head_x_rot};
use super::target::conditions::TargetingConditions;
use super::target::TargetRef;
use crate::entity::living::mob_effects::ActiveMobEffect;

#[derive(Debug, Clone, Copy)]
pub(super) struct BreathAirGoal;

impl Goal for BreathAirGoal {
    fn can_use(&mut self, _: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) -> bool {
        targets.body_entity().air_supply() < 140
    }

    // `BreathAirGoal.isInterruptable` (`BreathAirGoal.java:33-35`).
    fn is_interruptable(&self) -> bool {
        false
    }

    fn start(&mut self, context: &mut MobAiContext, _: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let feet = targets.body_entity().block_position();
        let mut destination = BlockPos {
            y: feet.y + 8,
            ..feet
        };
        'search: for z in feet.z - 1..=feet.z + 1 {
            for y in feet.y..=feet.y + 8 {
                for x in feet.x - 1..=feet.x + 1 {
                    let pos = BlockPos { x, y, z };
                    let state = targets.level().block_state(pos);
                    let Some(facts) = vanilla_block_state_path_facts(state) else {
                        continue;
                    };
                    if facts.land_pathfindable()
                        && (block_state_fluid_state(state).is_empty()
                            || vanilla_block_state_block_type_id(state)
                                == Some(VANILLA_BLOCK_BUBBLE_COLUMN_ID))
                    {
                        destination = pos;
                        break 'search;
                    }
                }
            }
        }
        context.navigate_to_position(
            Vec3d {
                x: f64::from(destination.x),
                y: f64::from(destination.y) + 1.0,
                z: f64::from(destination.z),
            },
            1.0,
        );
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct TryFindWaterGoal {
    next_scan: i64,
    wanted: Option<BlockPos>,
}

impl Goal for TryFindWaterGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
            let feet = targets.body_entity().block_position();
            if context.target.target.is_some()
                || !context.nav.navigation_done
                || targets.game_time() < self.next_scan
                || !targets.level().has_ticking_column_at(feet)
                || fluid_state_tag_membership(
                    targets.level(),
                    block_state_fluid_state(targets.level().block_state(feet)),
                    "minecraft:water",
                )
            {
                return false;
            }
            self.next_scan = targets.game_time() + 40;
            self.wanted = axolotl_water_position(targets);
            return self.wanted.is_some();
        }
        targets.body_entity().on_ground()
            && !fluid_state_tag_membership(
                targets.level(),
                block_state_fluid_state(
                    targets
                        .level()
                        .block_state(targets.body_entity().block_position()),
                ),
                "minecraft:water",
            )
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
            if let Some(pos) = self.wanted {
                let center = water_center(pos);
                context.set_look_at(center.x, center.y, center.z);
                context.try_navigate_to_position(center, 0, 0.15);
            }
            return;
        }
        let feet = targets.body_entity().block_position();
        for z in feet.z - 2..=feet.z + 2 {
            for y in feet.y - 2..=feet.y {
                for x in feet.x - 2..=feet.x + 2 {
                    let pos = BlockPos { x, y, z };
                    if fluid_state_tag_membership(
                        targets.level(),
                        block_state_fluid_state(targets.level().block_state(pos)),
                        "minecraft:water",
                    ) {
                        context.controls.move_control_request = Some(NavigationMove {
                            wanted: Vec3d {
                                x: f64::from(x),
                                y: f64::from(y),
                                z: f64::from(z),
                            },
                            speed_modifier: 1.0,
                        });
                        return;
                    }
                }
            }
        }
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
            context.target.target.is_none() && !context.nav.navigation_done
        } else {
            self.can_use(context, targets)
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
            if let Some(pos) = self.wanted {
                let center = water_center(pos);
                context.set_look_at(center.x, center.y, center.z);
            }
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
            self.wanted = None;
            context.stop_navigation();
        }
    }
}

// `TryFindWater.create`; `AxolotlAi.initIdleActivity` uses range six and speed .15.
fn axolotl_water_position(targets: &TargetView<'_, '_, '_>) -> Option<BlockPos> {
    let body = targets.body_entity();
    let feet = body.block_position();
    let mut alternate = None;
    for pos in within_manhattan(feet, 6, 6, 6) {
        if pos.x == feet.x && pos.z == feet.z {
            continue;
        }
        let above = BlockPos {
            y: pos.y + 1,
            ..pos
        };
        if !targets.level().has_ticking_column_at(pos)
            || !targets.level().has_ticking_column_at(above)
        {
            return None;
        }
        let state = targets.level().block_state(pos);
        if vanilla_block_state_block_type_id(state)? != VANILLA_BLOCK_WATER_ID {
            continue;
        }
        let above_state = targets.level().block_state(above);
        vanilla_block_state_path_facts(above_state)?;
        if block_state_is_air(above_state) {
            return Some(pos);
        }
        if alternate.is_none() && body.position().distance_to_sqr(water_center(pos)) >= 2.25 {
            alternate = Some(pos);
        }
    }
    alternate
}

fn water_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct SwimWithPlayerGoal {
    player: Option<TargetRef>,
}

impl SwimWithPlayerGoal {
    fn give_grace(&self, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(player) = self.player {
            let source_entity_id = targets.body_entity().id();
            targets.queue_command(crate::entity::EntityCommand::MobSpell(
                super::ranged::PendingMobSpell::PlayerEffect {
                    source_entity_id,
                    player: player.uuid,
                    effect: ActiveMobEffect::new("minecraft:dolphins_grace", 100, 0)
                        .expect("dolphin's grace is a built-in effect"),
                },
            ));
        }
    }
}

impl Goal for SwimWithPlayerGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let conditions = TargetingConditions::for_non_combat()
            .range(10.0)
            .ignore_line_of_sight();
        self.player = targets
            .local_players()
            .filter(|player| conditions.test(targets, player, |_| true))
            .min_by(|a, b| {
                context
                    .body
                    .position
                    .distance_to_sqr(a.position())
                    .total_cmp(&context.body.position.distance_to_sqr(b.position()))
            })
            .map(|player| player.target_ref());
        self.player.is_some_and(|player| {
            targets
                .store()
                .player(player.entity_id)
                .is_some_and(|state| state.is_swimming())
                && context.target.target != Some(player)
        })
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.player
            .and_then(|player| targets.facts_of(player))
            .is_some_and(|player| {
                targets
                    .store()
                    .player(player.entity_id())
                    .is_some_and(|state| state.is_swimming())
                    && context.body.position.distance_to_sqr(player.position()) < 256.0
            })
    }

    fn start(&mut self, _: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.give_grace(targets);
    }

    fn stop(&mut self, context: &mut MobAiContext, _: &mut TargetView<'_, '_, '_>) {
        self.player = None;
        context.stop_navigation();
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((player_id, player_pos, player_eye_y)) = self
            .player
            .and_then(|player| targets.facts_of(player))
            .map(|player| (player.entity_id(), player.position(), player.eye_y()))
        else {
            return;
        };
        let kind = targets.body_entity().entity_type_id();
        context.set_look_at_with_limits(
            player_pos.x,
            player_eye_y,
            player_pos.z,
            base_body_rotation_max_yaw(kind).expect("dolphin head rotation") + 20.0,
            base_max_head_x_rot(kind).expect("dolphin head rotation"),
        );
        if context.body.position.distance_to_sqr(player_pos) < 6.25 {
            context.stop_navigation();
        } else {
            context.navigate_to_position(player_pos, 4.0);
        }
        if targets
            .store()
            .player(player_id)
            .is_some_and(|state| state.is_swimming())
            && context.next_int(6) == 0
        {
            self.give_grace(targets);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SwimToTreasureGoal;

fn reached_treasure(position: Vec3d, treasure: BlockPos) -> bool {
    position.distance_to_sqr(Vec3d {
        x: f64::from(treasure.x) + 0.5,
        y: position.y.floor() + 0.5,
        z: f64::from(treasure.z) + 0.5,
    }) < 16.0
}

impl Goal for SwimToTreasureGoal {
    fn can_use(&mut self, _: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) -> bool {
        targets.body_entity().air_supply() >= 100
            && crate::entity::dolphin::treasure(targets.body_entity()).is_some()
    }

    // `DolphinSwimToTreasureGoal.isInterruptable` (`Dolphin.java:379-381`).
    fn is_interruptable(&self) -> bool {
        false
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.can_use(context, targets)
            && crate::entity::dolphin::treasure(targets.body_entity())
                .is_some_and(|treasure| !reached_treasure(context.body.position, treasure))
    }

    fn start(&mut self, context: &mut MobAiContext, _: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if crate::entity::dolphin::treasure(targets.body_entity())
            .is_none_or(|treasure| reached_treasure(context.body.position, treasure))
        {
            if let Some((entity, _, _)) = targets.world_mut() {
                crate::entity::dolphin::set_got_fish(entity, false);
            }
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(treasure) = crate::entity::dolphin::treasure(targets.body_entity()) else {
            return;
        };
        if !context.nav.navigation_done
            && !context.nav.navigation_target.is_some_and(|target| {
                context.body.position.distance_to_sqr(Vec3d {
                    x: f64::from(target.x) + 0.5,
                    y: f64::from(target.y) + 0.5,
                    z: f64::from(target.z) + 0.5,
                }) < 144.0
            })
        {
            return;
        }
        let destination = Vec3d {
            x: f64::from(treasure.x) + 0.5,
            y: f64::from(treasure.y) + 0.5,
            z: f64::from(treasure.z) + 0.5,
        };
        let mob = super::random_pos::RandomPosMob::of(targets.body_entity(), context.nav.malus);
        let sample = |horizontal, vertical, angle| {
            super::random_pos::default_random_pos_towards(
                targets.level(),
                &context.random,
                mob,
                horizontal,
                vertical,
                destination,
                angle,
            )
        };
        let Some(mut next) = sample(16, 1, std::f64::consts::FRAC_PI_8) else {
            return;
        };
        if next.is_none() {
            let Some(fallback) = sample(8, 4, std::f64::consts::FRAC_PI_2) else {
                return;
            };
            next = fallback;
        }
        if let Some(pos) = next {
            let state = targets.level().block_state(BlockPos {
                x: pos.x.floor() as i32,
                y: pos.y.floor() as i32,
                z: pos.z.floor() as i32,
            });
            if !fluid_state_tag_membership(
                targets.level(),
                block_state_fluid_state(state),
                "minecraft:water",
            ) || crate::entity::level::block_state_water_pathfindable(targets.level(), state)
                != Some(true)
            {
                let Some(fallback) = sample(8, 5, std::f64::consts::FRAC_PI_2) else {
                    return;
                };
                next = fallback;
            }
        }
        let Some(next) = next else {
            if let Some((entity, _, _)) = targets.world_mut() {
                crate::entity::dolphin::set_got_fish(entity, false);
            }
            return;
        };
        let kind = targets.body_entity().entity_type_id();
        context.set_look_at_with_limits(
            next.x,
            next.y,
            next.z,
            base_body_rotation_max_yaw(kind).expect("dolphin head rotation") + 20.0,
            base_max_head_x_rot(kind).expect("dolphin head rotation"),
        );
        context.navigate_to_position(next, 1.3);
        if context.next_int(super::reduced_tick_delay(80)) == 0 {
            targets.body_entity_event(38);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct PlayWithItemsGoal {
    cooldown: i32,
}

fn held_item(targets: &TargetView<'_, '_, '_>) -> bool {
    targets.body_entity().living().is_some_and(|living| {
        living
            .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
            .is_some()
    })
}

fn play_item(targets: &TargetView<'_, '_, '_>) -> Option<Vec3d> {
    let reach = targets.body_entity().bounding_box().inflate(8.0, 8.0, 8.0);
    targets
        .store()
        .ids_in_box_sections(reach)
        .into_iter()
        .find_map(|id| {
            let entity = targets.store().get(id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            (entity.is_alive()
                && !item.has_pickup_delay()
                && entity.is_in_water()
                && entity.bounding_box().intersects(reach))
            .then_some(entity.position())
        })
}

impl Goal for PlayWithItemsGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.cooldown <= context.tick_count && (held_item(targets) || play_item(targets).is_some())
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(position) = play_item(targets) {
            context.navigate_to_position(position, 1.2);
            if !targets.body_entity().is_silent() {
                let source = crate::entity::living::living_sound_source(targets.body_entity());
                targets.play_body_sound("minecraft:entity.dolphin.play", source, 1.0, 1.0);
            }
        }
        self.cooldown = 0;
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if held_item(targets) {
            if let Some((entity, store, level)) = targets.world_mut() {
                crate::entity::dolphin::throw_item(entity, store, level);
            }
            self.cooldown = context.tick_count.wrapping_add(context.next_int(100));
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if held_item(targets) {
            if let Some((entity, store, level)) = targets.world_mut() {
                crate::entity::dolphin::throw_item(entity, store, level);
            }
        } else if let Some(position) = play_item(targets) {
            context.navigate_to_position(position, 1.2);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct JumpGoal;

impl Goal for JumpGoal {
    // `DolphinJumpGoal.isInterruptable` (`DolphinJumpGoal.java:61-63`).
    fn is_interruptable(&self) -> bool {
        false
    }

    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if context.next_int(reduced_tick_delay(10)) != 0 {
            return false;
        }
        let Some(direction) = horizontal_direction(targets.body_entity().y_rot()) else {
            return false;
        };
        let feet = targets.body_entity().block_position();
        let facing = relative(feet, direction);
        for distance in [0, 1, 4, 5, 6, 7] {
            let at = BlockPos {
                x: feet.x + (facing.x - feet.x) * distance,
                z: feet.z + (facing.z - feet.z) * distance,
                ..feet
            };
            let Some(state) = targets.level().block_state_checked(at) else {
                return false;
            };
            if !fluid_state_tag_membership(
                targets.level(),
                block_state_fluid_state(state),
                "minecraft:water",
            ) || block_state_blocks_motion(state)
                || ![1, 2].into_iter().all(|height| {
                    targets
                        .level()
                        .block_state_checked(BlockPos {
                            y: at.y + height,
                            ..at
                        })
                        .is_some_and(block_state_is_air)
                })
            {
                return false;
            }
        }
        true
    }

    fn can_continue(&mut self, _: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) -> bool {
        let body = targets.body_entity();
        let y = body.delta_movement().y;
        let settled = y * y < f64::from(0.03_f32)
            && body.x_rot() != 0.0
            && body.x_rot().abs() < 10.0
            && body.is_in_water();
        !body.on_ground() && !settled
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let body = targets.body_entity();
        let Some(direction) = horizontal_direction(body.y_rot()) else {
            return;
        };
        let feet = body.block_position();
        let facing = relative(feet, direction);
        let movement = body.delta_movement().plus(Vec3d {
            x: f64::from(facing.x - feet.x) * 0.6,
            y: 0.7,
            z: f64::from(facing.z - feet.z) * 0.6,
        });
        let silent = body.is_silent();
        let sound_source = crate::entity::living::living_sound_source(body);
        context.stop_navigation();
        targets.set_body_delta_movement(movement);
        if !silent {
            targets.play_body_sound("minecraft:entity.dolphin.jump", sound_source, 1.0, 1.0);
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, _: &mut TargetView<'_, '_, '_>) {
        context.set_body_x_rot(0.0);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let body = targets.body_entity();
        let movement = body.delta_movement();
        let pitch = body.x_rot();
        let x_rot = if movement.y * movement.y < f64::from(0.03_f32) && pitch != 0.0 {
            pitch + 0.2 * (0.0 - pitch)
        } else if movement.length_sqr().sqrt() > f64::from(1.0e-5_f32) {
            (-movement.y)
                .atan2(movement.horizontal_distance_sqr().sqrt())
                .to_degrees() as f32
        } else {
            return;
        };
        context.set_body_x_rot(x_rot);
    }
}
