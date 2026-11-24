//! Bed sleep state and the Player sleep lifecycle.

use crate::world::LevelId;
mod respawn;
pub(in crate::world) mod stand_up;
mod status;
pub(in crate::world) use status::SleepStatus;

use super::{
    block_position, send, stored_block_state, ClientboundPlayPacket, GameType, PlayerRespawnConfig,
    ServerWorld, TeleportTransition, WorldEvent,
};
use crate::block_behavior::{bed, direction_property};
use crate::connection::ConnectionId;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::packets::{BlockPos, EntityAnimation, PlayerRotation, SystemChat, Vec3d};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::environment::BedRule;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::stats::{custom_stat, CUSTOM_STAT_TIME_SINCE_REST};

impl super::PlayerState {
    pub(crate) fn sleep_timer(&self) -> i16 {
        self.cold.sleep_timer
    }

    pub(crate) fn sleeping_pos(&self) -> Option<BlockPos> {
        self.cold.sleeping_pos
    }
}

impl ServerWorld {
    // BedBlock.java:122-130: wake only the first sleeping Villager in the block box.
    pub(in crate::world) fn kick_sleeping_villager(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> bool {
        let bounds = bbb_server_types::geometry::Aabb {
            min_x: f64::from(pos.x),
            min_y: f64::from(pos.y),
            min_z: f64::from(pos.z),
            max_x: f64::from(pos.x) + 1.0,
            max_y: f64::from(pos.y) + 1.0,
            max_z: f64::from(pos.z) + 1.0,
        };
        let store = &self.level_by_id(dimension).entities;
        let id = store.ids_in_box_sections(bounds).into_iter().find(|id| {
            store.get(*id).is_some_and(|entity| {
                entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID
                    && entity.removal_reason().is_none()
                    && entity.sleeping_pos().is_some()
                    && entity.bounding_box().intersects(bounds)
            })
        });
        let Some(id) = id else {
            return false;
        };
        self.with_stored_hurt_level(dimension, |store, level| {
            let Some(mut entity) = store.detach(id) else {
                return false;
            };
            let stopped = level
                .stop_sleeping(
                    crate::entity::level::EntityBlockContext::new(Some(&mut *store), None),
                    &mut entity,
                )
                .is_some();
            store.attach(entity);
            stopped
        })
    }

    pub(in crate::world) fn sleep_message(
        &self,
        id: ConnectionId,
        message: bbb_protocol::component::RichTextComponent,
        overlay: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        events.push(send(
            id,
            ClientboundPlayPacket::SystemChat(SystemChat::from_component(message, overlay)),
        ));
    }

    pub(in crate::world) fn sleep_problem(
        &self,
        id: ConnectionId,
        key: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        self.sleep_message(id, translated(key, Vec::new()), true, events);
    }

    pub(in crate::world) fn bed_rule_problem(
        &self,
        id: ConnectionId,
        rule: &BedRule,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(message) = rule.error_message.as_ref().and_then(|message| {
            bbb_protocol::component::RichTextComponent::try_from_component_nbt(message.clone()).ok()
        }) {
            self.sleep_message(id, message, true, events);
        }
    }

    // ServerPlayer.java:1978-1984: same-position suppresses only the message.
    pub(in crate::world) fn set_sleep_respawn(
        &mut self,
        id: ConnectionId,
        respawn: PlayerRespawnConfig,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let player = self.levels.player_mut(id).expect("connected sleep actor");
        let changed = player
            .cold
            .respawn
            .as_ref()
            .is_none_or(|old| old.dimension != respawn.dimension || old.pos != respawn.pos);
        player.cold.respawn = Some(respawn);
        if changed {
            self.sleep_message(
                id,
                translated("block.minecraft.set_spawn", Vec::new()),
                false,
                events,
            );
        }
        changed
    }

    // ServerPlayer.java:1172-1232: setting spawn precedes sleep and monster refusal.
    pub(in crate::world) fn start_player_sleep(
        &mut self,
        id: ConnectionId,
        pos: BlockPos,
        rule: BedRule,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.start_player_sleep_checked(id, pos, rule, events);
    }

    pub(in crate::world) fn start_player_sleep_checked(
        &mut self,
        id: ConnectionId,
        pos: BlockPos,
        rule: BedRule,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player_at(id);
        if player.cold.sleeping_pos.is_some() || player.damage.is_dead_or_dying() {
            return Some(());
        }
        let dimension = player.dimension;
        let Some(is_dark) = self.is_dark_outside(&dimension) else {
            return Some(());
        };
        let can_sleep = rule.can_sleep.allows(is_dark);
        let can_set_spawn = rule.can_set_spawn.allows(is_dark);
        if !can_sleep && !can_set_spawn {
            self.bed_rule_problem(id, &rule, events);
            return Some(());
        }
        let state = stored_block_state(&self.level_by_id(&dimension).chunks, pos);
        let Some(facing) = direction_property(state, "facing") else {
            return Some(());
        };
        let foot = relative(pos, opposite(facing));
        let reachable = |bed: BlockPos| {
            (player.position().x - (f64::from(bed.x) + 0.5)).abs() <= 3.0
                && (player.position().y - f64::from(bed.y)).abs() <= 2.0
                && (player.position().z - (f64::from(bed.z) + 0.5)).abs() <= 3.0
        };
        if !reachable(pos) && !reachable(foot) {
            self.sleep_problem(id, "block.minecraft.bed.too_far_away", events);
            return Some(());
        }
        let free = |pos: BlockPos| {
            self.level_by_id(&dimension)
                .chunks
                .get_block_state(pos.x, pos.y, pos.z)
                .and_then(bbb_protocol::block_state_types::vanilla_block_state_support_facts)
                .is_some_and(|facts| !facts.suffocating())
        };
        if !free(BlockPos {
            y: pos.y + 1,
            ..pos
        }) || !free(BlockPos {
            y: foot.y + 1,
            ..foot
        }) {
            self.sleep_problem(id, "block.minecraft.bed.obstructed", events);
            return Some(());
        }
        let rotation = player.rotation();
        if can_set_spawn {
            self.set_sleep_respawn(
                id,
                PlayerRespawnConfig {
                    dimension: self.level_key(dimension).clone(),
                    pos,
                    yaw: rotation.y_rot,
                    pitch: rotation.x_rot,
                    forced: false,
                },
                events,
            );
        }
        if !can_sleep {
            self.bed_rule_problem(id, &rule, events);
            return Some(());
        }
        if self.levels[id].game_type != GameType::Creative && self.monster_prevents_rest(id, pos) {
            self.sleep_problem(id, "block.minecraft.bed.not_safe", events);
            return Some(());
        }
        self.levels[id].reset_stat(
            &mut self.scoreboard,
            custom_stat(CUSTOM_STAT_TIME_SINCE_REST),
        );
        self.stop_external_player_ride(
            &dimension,
            id,
            super::external_player_ride_events::DismountLanding::Search,
            events,
        )?;

        self.set_bed_occupied(&dimension, pos, true);
        let player = self.levels.player_mut(id).unwrap();
        player.set_pose(VANILLA_ENTITY_POSE_SLEEPING_ID);
        player.set_position(sleep_position(pos));
        player.cold.sleeping_pos = Some(pos);
        player.cold.sleeping_pos_dirty = true;
        player.set_delta_movement(Vec3d::default());
        player.needs_sync = true;
        player.cold.sleep_timer = 0;
        player.award_stat_once(&mut self.scoreboard, custom_stat("minecraft:sleep_in_bed"));
        self.update_player_query_section(id);
        self.fire_player_trigger(id, "minecraft:slept_in_bed", events);
        if self.sleep_percentage() > 100 {
            self.sleep_problem(id, "sleep.not_possible", events);
        }
        self.update_sleeping_player_list(&dimension, events);
        Some(())
    }

    pub(in crate::world) fn set_bed_occupied(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        occupied: bool,
    ) {
        let state = stored_block_state(&self.level_by_id(dimension).chunks, pos);
        if bed::is_bed(state) {
            if let Some(state) =
                with_property(state, "occupied", if occupied { "true" } else { "false" })
            {
                self.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL);
            }
        }
    }

    pub(in crate::world) fn sleep_wake_landing(
        &self,
        id: ConnectionId,
    ) -> Option<(Vec3d, PlayerRotation, Option<BlockPos>)> {
        let player = self.levels.player(id)?;
        let mut position = player.position();
        let mut rotation = player.rotation();
        let mut bed_pos = None;
        if let Some(pos) = player.cold.sleeping_pos {
            let chunks = &self.level_by_id(&player.dimension).chunks;
            let state = stand_up::checked_state(chunks, pos)?;
            if bed::is_bed(state) {
                let facing = direction_property(state, "facing");
                // Occupied does not change the bed collision shape; preflight before publication.
                position = self
                    .sleep_stand_up_position(
                        &player.dimension,
                        pos,
                        VANILLA_ENTITY_TYPE_PLAYER_ID,
                        facing,
                        rotation.y_rot,
                    )?
                    .unwrap_or(Vec3d {
                        x: f64::from(pos.x) + 0.5,
                        y: f64::from(pos.y) + 1.1,
                        z: f64::from(pos.z) + 0.5,
                    });
                rotation = PlayerRotation {
                    y_rot: look_at_bed_yaw(position, pos),
                    x_rot: 0.0,
                };
                bed_pos = Some(pos);
            }
        }
        Some((position, rotation, bed_pos))
    }

    // ServerPlayer.java:1253-1262; LivingEntity.java:3732-3760; Player.java:1318-1325.
    pub(in crate::world) fn stop_player_sleep(
        &mut self,
        id: ConnectionId,
        forceful: bool,
        update_list: bool,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        self.stop_player_sleep_inner(id, forceful, update_list, true, events)
    }

    pub(in crate::world) fn stop_player_sleep_for_teleport(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.stop_player_sleep_inner(id, true, true, false, events) {
            return;
        }
        // Fallback if the bed chunk is no longer loaded: still clear sleeping state.
        let Some(dimension) = self.levels.player(id).map(|player| player.dimension) else {
            return;
        };
        if let Some(player) = self.levels.player_mut(id) {
            if player.cold.sleeping_pos.take().is_some() {
                player.set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
                player.cold.sleeping_pos_dirty = true;
                player.cold.sleep_timer = 0;
            }
        }
        self.update_sleeping_player_list(&dimension, events);
    }

    fn stop_player_sleep_inner(
        &mut self,
        id: ConnectionId,
        forceful: bool,
        update_list: bool,
        send_teleport: bool,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some((mut position, mut rotation, bed_pos)) = self.sleep_wake_landing(id) else {
            return false;
        };
        let player = &self.levels.player(id).expect("the player is online");
        let dimension = player.dimension;
        let wake_yaw = player.rotation().y_rot;
        if player.cold.sleeping_pos.is_some() {
            let entity_id = player.entity_id();
            if let Some(pos) = bed_pos {
                let Some(landing) = self.with_stored_hurt_level(&dimension, |entities, level| {
                    level.bed_wake_position(
                        crate::entity::level::EntityBlockContext::new(Some(entities), None),
                        pos,
                        VANILLA_ENTITY_TYPE_PLAYER_ID,
                        wake_yaw,
                    )
                }) else {
                    return false;
                };
                position = landing;
                rotation = PlayerRotation {
                    y_rot: look_at_bed_yaw(position, pos),
                    x_rot: 0.0,
                };
            }
            self.send_to_player_tracking_and_self(
                id,
                entity_id,
                ClientboundPlayPacket::EntityAnimation(EntityAnimation {
                    id: entity_id,
                    action: 2,
                }),
                events,
            );
            let player = self.levels.player_mut(id).unwrap();
            player.set_position(position);
            player.set_rotation(rotation);
            player.set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
            player.cold.sleeping_pos = None;
            player.cold.sleeping_pos_dirty = true;
        }
        if update_list {
            self.update_sleeping_player_list(&dimension, events);
        }
        self.levels[id].cold.sleep_timer = if forceful { 0 } else { 100 };
        if send_teleport {
            self.teleport_set_position(
                id,
                &TeleportTransition {
                    new_level: dimension,
                    position,
                    y_rot: rotation.y_rot,
                    x_rot: rotation.x_rot,
                    relatives_mask: 0,
                },
                events,
            );
        }
        true
    }

    // Player.java:239-258; LivingEntity.java:2724-2726.
    pub(in crate::world) fn tick_player_sleep(&mut self, events: &mut Vec<WorldEvent>) {
        if !self.levels.players().any(|p| {
            p.body_tick_enabled && (p.cold.sleeping_pos.is_some() || p.cold.sleep_timer > 0)
        }) {
            return;
        }
        let ids: Vec<_> = self
            .levels
            .player_entries()
            .filter(|(_, p)| {
                p.body_tick_enabled && (p.cold.sleeping_pos.is_some() || p.cold.sleep_timer > 0)
            })
            .map(|(&id, _)| id)
            .collect();
        for id in ids {
            let player = self.levels.player_at(id);
            let Some(pos) = player.cold.sleeping_pos else {
                let player = self.levels.player_mut(id).unwrap();
                if player.cold.sleep_timer > 0 {
                    let timer = i32::from(player.cold.sleep_timer) + 1;
                    player.cold.sleep_timer = if timer >= 110 { 0 } else { timer as i16 };
                }
                continue;
            };
            let dimension = player.dimension;
            let sample_pos = block_position(player.position());
            let Some((rule, dark)) = self
                .bed_rule_at(&dimension, sample_pos)
                .zip(self.is_dark_outside(&dimension))
            else {
                continue;
            };
            let sleep_allowed = rule.can_sleep.allows(dark);
            self.levels[id].cold.sleep_timer =
                (i32::from(self.levels[id].cold.sleep_timer) + 1).min(100) as i16;
            if !sleep_allowed {
                self.stop_player_sleep(id, false, true, events);
            } else if !bed::is_bed(stored_block_state(
                &self.level_by_id(&dimension).chunks,
                pos,
            )) || self.levels[id].damage.is_dead_or_dying()
                || self.levels[id].game_type == GameType::Spectator
            {
                self.stop_player_sleep(id, true, true, events);
            } else {
                self.levels[id].set_x_rot(0.0);
            }
        }
    }
}

pub(super) fn sleep_position(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.6875,
        z: f64::from(pos.z) + 0.5,
    }
}

pub(super) fn look_at_bed_yaw(position: Vec3d, pos: BlockPos) -> f32 {
    let direction = Vec3d {
        x: f64::from(pos.x) + 0.5 - position.x,
        y: f64::from(pos.y) - position.y,
        z: f64::from(pos.z) + 0.5 - position.z,
    }
    .normalize();
    let degrees = crate::entity::projectile_hit::mth_atan2(direction.z, direction.x) * 180.0_f64
        / f64::from(std::f32::consts::PI)
        - 90.0;
    let mut degrees = degrees % 360.0;
    if degrees >= 180.0 {
        degrees -= 360.0;
    }
    if degrees < -180.0 {
        degrees += 360.0;
    }
    degrees as f32
}

fn translated(
    key: &str,
    args: Vec<bbb_protocol::NbtSummaryValue>,
) -> bbb_protocol::component::RichTextComponent {
    bbb_protocol::component::RichTextComponent::try_from_component_nbt(
        crate::commands::translatable(key, args),
    )
    .expect("canonical sleep message")
}

#[cfg(test)]
mod tests;
