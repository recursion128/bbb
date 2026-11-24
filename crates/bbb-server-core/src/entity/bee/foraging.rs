//! Pollination, crop dusting and travel to native hives.

use super::*;
use crate::block_behavior::{bonemeal, crop, stem};
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::block_queries::{bool_property, int_property};

pub(crate) enum ForagingMovement {
    /// No foraging goal runs; `BeeWanderGoal` may, heading for the hive when
    /// `towards` is set (`Bee.java:1268-1294`).
    Idle {
        towards: Option<Vec3d>,
    },
    /// A foraging goal holds the movement without a new order.
    None,
    Stop,
    /// A target, speed and `setMaxVisitedNodesMultiplier` factor.
    Navigate(BlockPos, f64, f32),
    Hover {
        wanted: Option<Vec3d>,
        look: Option<Vec3d>,
    },
    Enter {
        pos: BlockPos,
        nectar: bool,
        flower: Option<BlockPos>,
    },
}

fn center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.6,
        z: f64::from(pos.z) + 0.5,
    }
}
fn distance(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)
}
/// `Bee.closerThan` (`Bee.java:655-657`): block distance, not centres.
fn closer_than(a: BlockPos, b: BlockPos, distance: i32) -> bool {
    let d = |a: i32, b: i32| f64::from(a - b).powi(2);
    d(a.x, b.x) + d(a.y, b.y) + d(a.z, b.z) < f64::from(distance * distance)
}
fn attracts(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let state = level.block_state(pos);
    level.block_state_in_tag(state, "minecraft:bee_attractive")
        && bool_property(state, "waterlogged") != Some(true)
        && (!crate::block_behavior::state_is_block(
            state,
            bbb_protocol::block_types::VANILLA_BLOCK_SUNFLOWER_ID,
        ) || bbb_protocol::block_state_types::vanilla_block_state_property(state, "half")
            == Some("upper"))
}

impl BeeRuntime {
    /// Pollination and hive goals (`Bee.java:745-773`, `Bee.java:1021-1058`,
    /// `Bee.java:1078-1248`).
    pub(crate) fn foraging_step(
        &mut self,
        entity: &mut Entity,
        level: &mut LevelCtx<'_, '_>,
        busy: bool,
        navigation_done: bool,
        mut reachable: impl FnMut(&LevelCtx<'_, '_>, BlockPos) -> bool,
    ) -> ForagingMovement {
        let position = entity.position();
        let pos = entity.block_position();
        let tick = entity.tick_count();
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            return ForagingMovement::None;
        };
        let idle = ForagingMovement::Idle { towards: None };
        let bee = data.bee_retained_mut().expect("Bee retained state");
        if tick % 20 == 0
            && bee.hive_pos.is_some_and(|hive| {
                distance(center(hive), position) >= 48.0 * 48.0 || !level.bee_hive_valid(hive)
            })
        {
            bee.hive_pos = None;
        }
        if busy || bee.has_stung() {
            self.pollination_ticks = 0;
            self.successful_pollination_ticks = 0;
            self.known_flower_ticks = 0;
            return idle;
        }
        let nectar = bee.has_flag(VANILLA_BEE_FLAG_HAS_NECTAR);
        if nectar
            && bee.hive_pos.is_some_and(|hive| level.bee_hive_valid(hive))
            && bee.crops_grown_since_pollination < 10
            && level.level_random().next_float() >= 0.3
            && level.level_random().next_int(30) == 0
        {
            for depth in 1..=2 {
                if bee.crops_grown_since_pollination >= 10 {
                    break;
                }
                let below = BlockPos {
                    y: pos.y - depth,
                    ..pos
                };
                if grow_crop(level, below) {
                    bee.crops_grown_since_pollination += 1;
                }
            }
        }
        let wants_hive = bee.stay_out_of_hive_countdown <= 0
            && self.pollination_ticks == 0
            && (nectar || bee.ticks_without_nectar > 3600 || level.bees_stay_in_hive(pos));
        if wants_hive {
            if bee.hive_pos.is_none() && self.remaining_hive_cooldown == 0 {
                self.remaining_hive_cooldown = 200;
                bee.hive_pos = level
                    .bee_hives_near(pos)
                    .into_iter()
                    .find(|candidate| reachable(level, *candidate));
                self.travelling_ticks = 0;
            }
            if let Some(hive) = bee.hive_pos {
                if level.bee_hive_near_fire(hive) {
                    return idle;
                }
                self.known_flower_ticks = 0;
                if !level.bee_hive_has_space(hive) {
                    bee.hive_pos = None;
                    self.remaining_hive_cooldown = 200;
                    return ForagingMovement::Stop;
                }
                if distance(
                    position,
                    Vec3d {
                        y: f64::from(hive.y) + 0.5,
                        ..center(hive)
                    },
                ) < 4.0
                {
                    return ForagingMovement::Enter {
                        pos: hive,
                        nectar,
                        flower: bee.flower_pos,
                    };
                }
                self.travelling_ticks += 1;
                if self.travelling_ticks > 2400 {
                    bee.hive_pos = None;
                    self.remaining_hive_cooldown = 200;
                    return ForagingMovement::Stop;
                }
                return if tick % 20 == 0 {
                    // Bee.java:853.
                    ForagingMovement::Navigate(hive, 1.0, 10.0)
                } else {
                    ForagingMovement::None
                };
            }
        }
        // Bee.java:1268-1274: wander back towards a valid hive left too far behind.
        let towards = bee
            .hive_pos
            .filter(|hive| {
                level.bee_hive_valid(*hive)
                    && closer_than(*hive, pos, 48)
                    && !closer_than(*hive, pos, 24)
            })
            .map(|hive| {
                let to = Vec3d {
                    x: f64::from(hive.x) + 0.5 - position.x,
                    y: f64::from(hive.y) + 0.5 - position.y,
                    z: f64::from(hive.z) + 0.5 - position.z,
                };
                let length = (to.x * to.x + to.y * to.y + to.z * to.z).sqrt();
                Vec3d {
                    x: to.x / length,
                    y: to.y / length,
                    z: to.z / length,
                }
            });
        let idle = ForagingMovement::Idle { towards };
        if nectar {
            return idle;
        }
        if level.is_raining() {
            if self.pollination_ticks > 0 {
                self.pollination_ticks = 0;
                self.successful_pollination_ticks = 0;
                self.remaining_flower_cooldown = 200;
                return ForagingMovement::Stop;
            }
            return self.go_to_known_flower(bee, level, pos, navigation_done, idle);
        }
        if self.pollination_ticks == 0 {
            if self.remaining_flower_cooldown > 0 {
                return self.go_to_known_flower(bee, level, pos, navigation_done, idle);
            }
            let mut flowers = Vec::new();
            for x in -5..=5 {
                for y in -5..=5 {
                    for z in -5..=5 {
                        let candidate = BlockPos {
                            x: pos.x + x,
                            y: pos.y + y,
                            z: pos.z + z,
                        };
                        if Some(candidate) != self.failed_flower
                            && !bbb_server_types::fluid::block_state_is_air(
                                level.block_state(candidate),
                            )
                            && attracts(level, candidate)
                        {
                            flowers.push(candidate);
                        }
                    }
                }
            }
            flowers.sort_by(|a, b| {
                distance(center(*a), position).total_cmp(&distance(center(*b), position))
            });
            let flower = flowers
                .into_iter()
                .find(|candidate| reachable(level, *candidate));
            let Some(flower) = flower else {
                self.failed_flower = None;
                self.remaining_flower_cooldown = level.level_random().next_int(41) + 20;
                return self.go_to_known_flower(bee, level, pos, navigation_done, idle);
            };
            bee.flower_pos = Some(flower);
            bee.ticks_without_nectar = 0;
            self.pollination_ticks = 1;
            self.successful_pollination_ticks = 0;
            self.known_flower_ticks = 0;
            self.hover_pos = None;
            // Bee.java:1111.
            return ForagingMovement::Navigate(flower, 1.2, 1.0);
        }
        self.pollination_ticks += 1;
        let Some(flower) = bee.flower_pos.filter(|flower| attracts(level, *flower)) else {
            self.pollination_ticks = 0;
            self.remaining_flower_cooldown = 200;
            return ForagingMovement::Stop;
        };
        if self.pollination_ticks > 600 {
            self.failed_flower = Some(flower);
            bee.flower_pos = None;
            self.pollination_ticks = 0;
            self.remaining_flower_cooldown = 200;
            return ForagingMovement::Stop;
        }
        let target = center(flower);
        if distance(position, target) <= 1.0 {
            // Bee.java:1176-1192: drift between spots a third of a block apart.
            let hover = *self.hover_pos.get_or_insert(target);
            let mut movement = ForagingMovement::Hover {
                wanted: Some(hover),
                look: None,
            };
            if distance(position, hover) <= 0.1 * 0.1 {
                let random = level.level_random();
                let wanted = (random.next_int(25) == 0).then(|| {
                    let offset = || f64::from((random.next_float() * 2.0 - 1.0) / 3.0);
                    Vec3d {
                        x: target.x + offset(),
                        y: target.y,
                        z: target.z + offset(),
                    }
                });
                if wanted.is_some() {
                    self.hover_pos = wanted;
                }
                movement = ForagingMovement::Hover {
                    wanted,
                    look: Some(target),
                };
            }
            self.successful_pollination_ticks += 1;
            if self.successful_pollination_ticks > 400 {
                bee.set_flag(VANILLA_BEE_FLAG_HAS_NECTAR, true);
                bee.ticks_without_nectar = 0;
                self.pollination_ticks = 0;
                self.remaining_flower_cooldown = 200;
                entity.mark_synched_dirty(VANILLA_BEE_FLAGS_DATA_ID);
                return ForagingMovement::Stop;
            }
            if self.successful_pollination_ticks % 60 == 0 {
                level.play_block_sound(flower, "minecraft:entity.bee.pollinate");
            }
            movement
        } else {
            self.hover_pos = None;
            if navigation_done && distance(position, target) < 4.0 {
                ForagingMovement::Hover {
                    wanted: Some(target),
                    look: None,
                }
            } else if tick % 20 == 0 {
                ForagingMovement::Navigate(flower, 1.2, 1.0)
            } else {
                ForagingMovement::None
            }
        }
    }

    /// `BeeGoToKnownFlowerGoal` (`Bee.java:892-941`).
    fn go_to_known_flower(
        &mut self,
        bee: &mut BeeRetainedState,
        level: &mut LevelCtx<'_, '_>,
        pos: BlockPos,
        navigation_done: bool,
        idle: ForagingMovement,
    ) -> ForagingMovement {
        let Some(flower) = bee
            .flower_pos
            .filter(|flower| bee.ticks_without_nectar > 600 && !closer_than(*flower, pos, 2))
        else {
            if self.known_flower_ticks > 0 {
                self.known_flower_ticks = 0;
                return ForagingMovement::Stop;
            }
            return idle;
        };
        self.known_flower_ticks += 1;
        if self.known_flower_ticks > 2400 || (navigation_done && !closer_than(flower, pos, 48)) {
            bee.flower_pos = None;
            self.remaining_flower_cooldown = level.level_random().next_int(41) + 20;
            self.known_flower_ticks = 0;
            return ForagingMovement::Stop;
        }
        if navigation_done {
            ForagingMovement::Navigate(flower, 1.0, 1.0)
        } else {
            ForagingMovement::None
        }
    }
}

/// `BeeGrowCropGoal.tick` (`Bee.java:944-1000`).
fn grow_crop(level: &mut LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    let state = level.block_state(pos);
    if !level.block_state_in_tag(state, "minecraft:bee_growables") {
        return false;
    }
    let grown = if crop::is_crop(state) {
        (!crop::is_max_age(state))
            .then(|| crop::state_for_age(state, crop::age(state)? + 1))
            .flatten()
    } else if stem::is_stem(state) {
        int_property(state, "age")
            .filter(|age| *age < 7)
            .and_then(|age| with_property(state, "age", &(age + 1).to_string()))
    } else {
        use bbb_protocol::block_types::{
            VANILLA_BLOCK_CAVE_VINES_ID, VANILLA_BLOCK_CAVE_VINES_PLANT_ID,
            VANILLA_BLOCK_SWEET_BERRY_BUSH_ID,
        };
        let block = vanilla_block_state_block_type_id(state);
        if block == Some(VANILLA_BLOCK_SWEET_BERRY_BUSH_ID) {
            int_property(state, "age")
                .filter(|age| *age < 3)
                .and_then(|age| with_property(state, "age", &(age + 1).to_string()))
        } else if matches!(
            block,
            Some(VANILLA_BLOCK_CAVE_VINES_ID | VANILLA_BLOCK_CAVE_VINES_PLANT_ID)
        ) && bonemeal::grow_crop(level, pos)
        {
            Some(level.block_state(pos))
        } else {
            None
        }
    };
    let Some(grown) = grown else {
        return false;
    };
    let changed =
        level.block_state(pos) == grown || level.set_block(pos, grown, SET_BLOCK_UPDATE_ALL, 512);
    if changed {
        level.level_event(2011, pos, 15);
    }
    changed
}
