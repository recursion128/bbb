//! Session adaptation of the shared swept block-contact owner.

use super::{PlayerState, ServerWorld, WorldEvent};
use crate::connection::ConnectionId;
use crate::entity::{
    take_body_effect_movements, BlockContactGeometry, BlockEffectCollector, BodyBlockContact,
    BodyContactCursor, EntityMovement, EnvironmentBlockKind, FluidTag, InsideBlockEffect,
    InsideBlockEffectType,
};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_BURNING_TIME_ID;
use bbb_protocol::equipment_slots::{EquipmentSlotType, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{BlockPos, SoundEventHolder, SoundSource, Vec3d};
use bbb_server_types::interaction::player_pose_bounding_box;
use std::collections::VecDeque;

mod honey;
mod inside_callbacks;

#[derive(Clone, Default)]
pub(in crate::world) struct PlayerBlockContact {
    pub(in crate::world) old_position: Vec3d,
    pub(in crate::world) pending: VecDeque<EntityMovement>,
    final_movements: Vec<EntityMovement>,
    pub(in crate::world) fall_distance: f64,
    pub(in crate::world) starting_to_fall_position: Option<Vec3d>,
    pub(in crate::world) last_climbable_pos: Option<BlockPos>,
    pub(in crate::world) supporting: Option<BlockPos>,
    pub(in crate::world) on_ground_no_blocks: bool,
    pub(in crate::world) stuck_multiplier: Vec3d,
    pub(in crate::world) in_powder_snow: bool,
    pub(in crate::world) movement_emission: crate::entity::BodyMovementEmissionState,
    pub(in crate::world) crystal_sound: crate::entity::BodyCrystalSoundState,
    pub(in crate::world) piston_deltas: [f64; 3],
    pub(in crate::world) piston_deltas_game_time: i64,
}

impl PlayerBlockContact {
    pub(in crate::world) fn new(position: Vec3d, fall_distance: f64) -> Self {
        Self {
            old_position: position,
            fall_distance,
            ..Self::default()
        }
    }
}

struct SessionContact<'a> {
    world: &'a mut ServerWorld,
    id: ConnectionId,
    events: &'a mut Vec<WorldEvent>,
    collector: BlockEffectCollector,
}

impl SessionContact<'_> {
    fn block_state(&self, pos: BlockPos) -> i32 {
        self.world
            .level_by_id(&self.world.levels[self.id].dimension)
            .block_state(pos)
    }

    fn visit_contact(&mut self, contact: BodyBlockContact) -> bool {
        if contact.inside_block {
            self.world
                .trigger_enter_block(self.id, contact.state, self.events);
            use crate::entity::portal::PortalKind;
            let portal = match bbb_protocol::block_state_types::vanilla_block_state_block_type_id(
                contact.state,
            ) {
                Some(bbb_protocol::block_types::VANILLA_BLOCK_NETHER_PORTAL_ID) => {
                    Some(PortalKind::Nether)
                }
                Some(bbb_protocol::block_types::VANILLA_BLOCK_END_PORTAL_ID) => {
                    Some(PortalKind::End)
                }
                Some(bbb_protocol::block_types::VANILLA_BLOCK_END_GATEWAY_ID) => {
                    Some(PortalKind::EndGateway)
                }
                _ => None,
            };
            if let Some(kind) = portal {
                self.world
                    .record_player_portal_contact(self.id, kind, contact.pos, self.events);
            }
            self.collector.advance_step(contact.step);
            if self
                .world
                .player_block_trigger_inside(self.id, contact.pos, contact.state)
                .is_none()
            {
                return false;
            }
            if self.immediate_contact(contact.state, contact.pos).is_none() {
                return false;
            }
            if contact.precise
                && bbb_protocol::block_state_types::vanilla_block_state_block_type_id(contact.state)
                    == Some(bbb_protocol::block_types::VANILLA_BLOCK_BUBBLE_COLUMN_ID)
            {
                self.bubble_column(contact.pos);
            }
            if contact.environment_kind == EnvironmentBlockKind::PowderSnow {
                let player = self.world.levels.player(self.id).unwrap();
                let pos = super::super::block_position(player.position());
                let state = self.block_state(pos);
                if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
                    == Some(bbb_protocol::block_types::VANILLA_BLOCK_POWDER_SNOW_ID)
                    && self
                        .make_stuck(Vec3d {
                            x: f64::from(0.9_f32),
                            y: 1.5,
                            z: f64::from(0.9_f32),
                        })
                        .is_none()
                {
                    return false;
                }
            }
            self.collector
                .collect_block(contact.environment_kind, contact.pos, contact.state);
        }
        if let Some(fluid) = contact.fluid {
            self.collector.advance_step(contact.step);
            self.collector.collect_fluid(fluid);
        }
        true
    }
}

impl SessionContact<'_> {
    fn immediate_contact(&mut self, state: i32, pos: BlockPos) -> Option<()> {
        use crate::block_behavior::inside_contact::{contact, ContactBody, ImmediateContact};
        use crate::block_behavior::sweet_berry_bush;
        let player = self.world.levels.player(self.id).unwrap();
        let body = ContactBody {
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            living: true,
            weaving: player
                .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_WEAVING_ID),
        };
        match contact(state, body)? {
            ImmediateContact::Unhandled
            | ImmediateContact::Resolved
            | ImmediateContact::RavagerCrop
            | ImmediateContact::LilyPad
            | ImmediateContact::DestroyWithoutDrops => Some(()),
            ImmediateContact::Stuck(multiplier) => self.make_stuck(multiplier),
            ImmediateContact::Hurt(damage_type, amount) => self.hurt(damage_type, amount),
            ImmediateContact::SweetBerry { hurts_when_moving } => {
                self.make_stuck(sweet_berry_bush::stuck_multiplier())?;
                if hurts_when_moving
                    && sweet_berry_bush::movement_hurts(self.world.player_known_movement(self.id)?)
                {
                    self.hurt("minecraft:sweet_berry_bush", 1.0)?;
                }
                Some(())
            }
            ImmediateContact::WitherRose(duration) => self.wither_rose(duration),
            ImmediateContact::Honey => honey::apply(self, pos),
            ImmediateContact::BeeEyeblossom(_) => None,
        }
    }
    // Player.java:309-313 over BubbleColumnBlock.java:53-64; the client owns the push.
    fn bubble_column(&mut self, pos: BlockPos) {
        let player = self.world.levels.player(self.id).unwrap();
        if player.abilities.flying {
            return;
        }
        let above = BlockPos {
            y: pos.y + 1,
            ..pos
        };
        let chunks = &self.world.level_by_id(&player.dimension).chunks;
        let open = crate::world::movement_collision::block_collision_shape(
            chunks,
            above,
            crate::world::movement_collision::current_piston_no_clip(),
        )
        .is_empty()
            && bbb_server_types::fluid::block_state_fluid_state(self.block_state(above)).is_empty();
        if !open {
            return;
        }
        for packet in crate::entity::inside_blocks::bubble_column_particles(&self.world.random, pos)
        {
            self.world
                .publish_player_particles(self.id, packet, self.events);
        }
    }
    fn make_stuck(&mut self, multiplier: Vec3d) -> Option<()> {
        if !self.world.levels.player(self.id)?.abilities.flying {
            self.world.reset_player_fall_distance(self.id, self.events);
            self.world
                .levels
                .player_mut(self.id)?
                .block_contact
                .stuck_multiplier = multiplier;
        }
        self.world
            .levels
            .player_mut(self.id)?
            .try_reset_fall_impulse();
        Some(())
    }
    fn hurt(&mut self, damage_type: &'static str, amount: f32) -> Option<()> {
        self.world.hurt_connected_player_checked(
            self.id,
            damage_type,
            amount,
            None,
            None,
            None,
            self.events,
        )?;
        Some(())
    }
    fn wither_rose(&mut self, duration: i32) -> Option<()> {
        use bbb_server_datapack::damage::{player_is_invulnerable_to, HurtSource, HurtTarget};
        if self.world.admin.settings().difficulty
            == bbb_protocol::difficulties::Difficulty::Peaceful
        {
            return Some(());
        }
        let player = self.world.levels.player(self.id)?;
        let damage_types = std::sync::Arc::clone(&self.world.cold.damage_types);
        let facts = damage_types
            .iter()
            .find(|facts| facts.resource_id == "minecraft:wither")?;
        if player_is_invulnerable_to(
            &HurtTarget {
                state: player.damage,
                abilities_invulnerable: player.abilities.invulnerable,
                entity_invulnerable: player.entity_invulnerable,
                client_loaded: player.client_loaded(),
                changing_dimension: player.is_changing_dimension,
                max_health: player.max_health(),
                max_absorption: player.max_absorption(),
            },
            &HurtSource {
                damage_type: facts,
                causing_player: None,
                direct_player: None,
                source_position: None,
                arrow_owner_player: false,
                team_refuses_harm: false,
            },
            self.world.damage_game_rules(),
        ) {
            return Some(());
        }
        let source = crate::world::combat::PlayerHurtSource {
            damage_type: facts,
            causing: None,
            direct: None,
        };
        match self.world.player_immune_to_damage(self.id, source) {
            Ok(false) => {}
            Ok(true) => return Some(()),
            Err(detail) => {
                tracing::warn!("wither rose damage immunity: {detail}");
                return None;
            }
        }
        let effect = crate::entity::ActiveMobEffect::new("minecraft:wither", duration, 0)?;
        self.world
            .add_player_mob_effect(self.id, effect, self.events)
            .map(|_| ())
    }
}

impl PlayerState {
    pub(in crate::world) fn ignite_environment(&mut self, ticks: i32) {
        let multiplier = self
            .attributes
            .value(VANILLA_ATTRIBUTE_BURNING_TIME_ID)
            .unwrap_or(1.0);
        let ticks = (f64::from(ticks) * multiplier).ceil() as i32;
        if self.remaining_fire_ticks < ticks {
            self.set_remaining_fire_ticks(ticks);
        }
        self.set_ticks_frozen(0);
    }
}

impl ServerWorld {
    // ServerPlayer.java:832-839.
    pub(in crate::world) fn reset_player_fall_distance(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player_mut(id) else {
            return;
        };
        let start = player.block_contact.starting_to_fall_position.take();
        if player.damage.health > 0.0 {
            if let Some(start) = start {
                self.trigger_distance(id, "minecraft:fall_from_height", start, events);
            }
        }
        if let Some(player) = self.levels.player_mut(id) {
            player.block_contact.fall_distance = 0.0;
        }
    }

    fn player_can_freeze(&self, id: ConnectionId) -> Option<bool> {
        let player = self.levels.player(id).unwrap();
        let tags = self.player_registry_tags(id);
        let items: Vec<_> = player
            .inventory
            .equipment_slots()
            .filter_map(|(slot, index)| {
                let kind = VANILLA_EQUIPMENT_SLOTS
                    .iter()
                    .find(|fact| fact.slot == slot)?
                    .slot_type;
                if !matches!(
                    kind,
                    EquipmentSlotType::HumanoidArmor | EquipmentSlotType::AnimalArmor
                ) {
                    return None;
                }
                player
                    .inventory
                    .slot(index)?
                    .item_id()
                    .and_then(bbb_protocol::item_types::vanilla_item_type)
                    .map(|item| item.resource_id)
            })
            .collect();
        crate::entity::can_body_freeze(
            player.game_type == GameType::Spectator,
            "minecraft:player",
            items,
            |registry, tag, resource| {
                tags.as_ref()
                    .is_some_and(|tags| tags.contains(registry, tag, resource))
            },
        )
    }

    pub(in crate::world) fn tick_player_block_contacts(&mut self, events: &mut Vec<WorldEvent>) {
        let ids: Vec<_> = self
            .levels
            .player_entries()
            .filter(|(_, p)| p.body_tick_enabled)
            .map(|(&id, _)| id)
            .collect();
        for id in ids {
            let _ = self.tick_player_block_contacts_checked(id, events);
        }
    }

    fn tick_player_block_contacts_checked(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player_mut(id)?;
        let position = player.position();
        take_body_effect_movements(
            &mut player.block_contact.pending,
            &mut player.block_contact.final_movements,
            player.block_contact.old_position,
            position,
        );
        let movements = player.block_contact.final_movements.clone();
        player.block_contact.in_powder_snow = false;
        self.apply_player_block_effect_movements_checked(id, &movements, events)?;
        self.tick_player_freeze_suffix(id, events)?;
        Some(())
    }

    pub(in crate::world) fn apply_player_block_effect_movements_checked(
        &mut self,
        id: ConnectionId,
        movements: &[EntityMovement],
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player(id)?;
        if player.game_type != GameType::Spectator && !player.avatar_removed {
            if player.on_ground() && self.player_block_step_on(id, events).is_none() {
                return None;
            }
            let player = self.levels.player(id).unwrap();
            let Ok([width, height]) = bbb_server_types::interaction::player_pose_dimensions(
                player.pose_id,
                player.pose_scale(),
            ) else {
                return None;
            };
            let geometry = BlockContactGeometry { width, height };
            let was_on_fire = player.remaining_fire_ticks > 0;
            let was_freezing = player.ticks_frozen > 0;
            let previous_fire_ticks = player.remaining_fire_ticks;
            let mut visitor = SessionContact {
                world: self,
                id,
                events,
                collector: BlockEffectCollector::default(),
            };
            let mut cursor = BodyContactCursor::new(geometry, movements.to_vec());
            loop {
                let player = visitor.world.levels.player(id).unwrap();
                if player.avatar_removed || player.damage.is_dead_or_dying() {
                    break;
                }
                let chunks = &visitor.world.level_by_id(&player.dimension).chunks;
                match cursor.next_contact(chunks, player.block_contact.fall_distance)? {
                    Some(contact) if visitor.visit_contact(contact) => {}
                    Some(_) => return None,
                    None => break,
                }
            }
            let effects = visitor.collector.take_effects();
            let mut resolved = true;
            for effect in effects {
                let Some(player) = self.levels.player(id) else {
                    return Some(());
                };
                if player.avatar_removed || player.damage.is_dead_or_dying() {
                    break;
                }
                if !self.apply_player_contact_effect(id, effect, events) {
                    resolved = false;
                    break;
                }
            }
            if !resolved {
                return None;
            }
            let Some(player) = self.levels.player(id) else {
                return Some(());
            };
            let bounds =
                player_pose_bounding_box(player.position(), player.pose_id, player.pose_scale())
                    .ok()?;
            let level = self.level_by_id(&player.dimension);
            let feet = super::super::block_position(player.position());
            let rain = level.weather.is_raining_at(&level.chunks, feet)
                || level.weather.is_raining_at(
                    &level.chunks,
                    BlockPos {
                        y: bounds.max_y.floor() as i32,
                        ..feet
                    },
                );
            let Some(player) = self.levels.player_mut(id) else {
                return Some(());
            };
            if rain {
                player.remaining_fire_ticks = player.remaining_fire_ticks.min(0);
            }
            let extinguished = (was_on_fire && player.remaining_fire_ticks <= 0)
                || (was_freezing && player.ticks_frozen <= 0);
            if extinguished {
                let pitch =
                    1.6_f32 + (self.random.next_float() - self.random.next_float()) * 0.4_f32;
                let position = player.position();
                let registry_id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    "minecraft:entity.generic.extinguish_fire",
                )
                .unwrap();
                self.play_player_sound(
                    id,
                    None,
                    SoundEventHolder::Reference { registry_id },
                    SoundSource::Players,
                    position,
                    0.7,
                    pitch,
                    events,
                );
            }
            let Some(player) = self.levels.player_mut(id) else {
                return Some(());
            };
            if player.remaining_fire_ticks <= 0
                && player.remaining_fire_ticks <= previous_fire_ticks
            {
                player.set_remaining_fire_ticks(-20);
            }
        }
        Some(())
    }

    fn apply_player_contact_effect(
        &mut self,
        id: ConnectionId,
        effect: InsideBlockEffect,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        match effect {
            InsideBlockEffect::Type(InsideBlockEffectType::Freeze) => {
                let Some(player) = self.levels.player_mut(id) else {
                    return true;
                };
                player.block_contact.in_powder_snow = true;
                let Some(can_freeze) = self.player_can_freeze(id) else {
                    return false;
                };
                if can_freeze {
                    let Some(player) = self.levels.player_mut(id) else {
                        return true;
                    };
                    player.set_ticks_frozen(
                        player
                            .ticks_frozen
                            .wrapping_add(1)
                            .min(crate::entity::TICKS_REQUIRED_TO_FREEZE),
                    );
                }
            }
            InsideBlockEffect::Type(InsideBlockEffectType::ClearFreeze) => {
                if let Some(player) = self.levels.player_mut(id) {
                    player.set_ticks_frozen(0);
                }
            }
            InsideBlockEffect::Type(InsideBlockEffectType::FireIgnite) => {
                let Some(remaining) = self
                    .levels
                    .player(id)
                    .map(|player| player.remaining_fire_ticks)
                else {
                    return true;
                };
                let added = if remaining >= 0 {
                    self.random.next_int(2) + 1
                } else {
                    1
                };
                let Some(player) = self.levels.player_mut(id) else {
                    return true;
                };
                player.set_remaining_fire_ticks(player.remaining_fire_ticks.wrapping_add(added));
                if player.remaining_fire_ticks >= 0 {
                    player.ignite_environment(160);
                }
            }
            InsideBlockEffect::Type(InsideBlockEffectType::LavaIgnite) => {
                if let Some(player) = self.levels.player_mut(id) {
                    player.ignite_environment(300);
                }
            }
            InsideBlockEffect::Type(InsideBlockEffectType::Extinguish) => {
                if let Some(player) = self.levels.player_mut(id) {
                    player.remaining_fire_ticks = player.remaining_fire_ticks.min(0);
                }
            }
            InsideBlockEffect::Damage {
                damage_type,
                amount,
            } => {
                if self
                    .hurt_connected_player_checked(
                        id,
                        damage_type,
                        amount,
                        None,
                        None,
                        None,
                        events,
                    )
                    .is_none()
                {
                    return false;
                }
            }
            InsideBlockEffect::LavaDamage { amount } => {
                let Some(succeeded) = self.hurt_connected_player_checked(
                    id,
                    "minecraft:lava",
                    amount,
                    None,
                    None,
                    None,
                    events,
                ) else {
                    return false;
                };
                if succeeded && self.levels.player(id).is_some_and(|player| !player.silent) {
                    let Some(position) = self.levels.player(id).map(PlayerState::position) else {
                        return true;
                    };
                    let pitch = 2.0_f32 + self.random.next_float() * 0.4_f32;
                    let registry_id =
                        bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                            "minecraft:entity.generic.burn",
                        )
                        .unwrap();
                    self.play_player_sound(
                        id,
                        None,
                        SoundEventHolder::Reference { registry_id },
                        SoundSource::Players,
                        position,
                        0.4,
                        pitch,
                        events,
                    );
                }
            }
            InsideBlockEffect::LowerCauldron { pos, state } => {
                let player = self.levels.player(id).unwrap();
                if player.remaining_fire_ticks > 0 && self.player_may_interact_block(id, pos) {
                    let dimension = player.dimension;
                    self.with_level(dimension, |level| {
                        crate::block_behavior::cauldron::lower_fill_level_on_fire(
                            level, pos, state,
                        );
                    });
                }
            }
            InsideBlockEffect::DestroyPowderSnow(pos) => {
                // PowderSnowBlock.java:84-93; ServerPlayer.java:2054-2056.
                let player = self.levels.player(id).unwrap();
                let dimension = player.dimension;
                if player.remaining_fire_ticks > 0 && self.player_may_interact_block(id, pos) {
                    self.with_level(dimension, |level| {
                        LevelCtx::destroy_block(level, pos, false);
                    });
                }
            }
        }
        true
    }
}

impl ServerWorld {
    pub(super) fn player_legacy_support_pos(&self, id: ConnectionId) -> Option<BlockPos> {
        let player = self.levels.player(id)?;
        self.player_legacy_support_pos_at(id, player.position(), player.block_contact.supporting)
    }

    pub(in crate::world) fn player_legacy_support_pos_at(
        &self,
        id: ConnectionId,
        position: Vec3d,
        supporting: Option<BlockPos>,
    ) -> Option<BlockPos> {
        self.player_support_pos_at(id, position, supporting, 0.2_f32)
    }

    pub(in crate::world) fn player_support_pos_at(
        &self,
        id: ConnectionId,
        position: Vec3d,
        supporting: Option<BlockPos>,
        offset: f32,
    ) -> Option<BlockPos> {
        let player = self.levels.player(id).expect("the player is online");
        let level = self.level_by_id(&player.dimension);
        let mut known = true;
        let pos = crate::entity::body_on_pos(position, supporting, offset, |pos| {
            let state_id = if level.chunks.is_inside_build_height(pos.y) {
                let Some(state) = level.chunks.get_block_state(pos.x, pos.y, pos.z) else {
                    known = false;
                    return crate::entity::SupportingBlockFacts {
                        state_id: level.chunks.air_state_id(),
                        is_fence: false,
                        is_wall: false,
                    };
                };
                state
            } else {
                level.chunks.air_state_id()
            };
            let block_id =
                bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state_id);
            let tags = level.chunks.registry_tags();
            let membership =
                |tag| block_id.and_then(|id| tags.map(|tags| tags.contains_block(tag, id)));
            let fence = membership("minecraft:fences");
            let wall = membership("minecraft:walls");
            known &= matches!(fence, Some(true | false)) && matches!(wall, Some(true | false));
            crate::entity::SupportingBlockFacts {
                state_id,
                is_fence: fence == Some(true),
                is_wall: wall == Some(true),
            }
        });
        known.then_some(pos)
    }

    fn tick_player_freeze_suffix(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player(id).unwrap();
        let ticks = crate::entity::tick_body_freeze(
            player.ticks_frozen,
            player.block_contact.in_powder_snow,
            || self.player_can_freeze(id),
        );
        let ticks = ticks?;
        self.levels[id].set_ticks_frozen(ticks);
        if let Some(pos) = self.player_legacy_support_pos(id) {
            let player = self.levels.player(id).unwrap();
            let state = self.level_by_id(&player.dimension).block_state(pos);
            if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state).is_some() {
                crate::entity::update_body_frost(
                    &mut self.levels[id].attributes,
                    ticks,
                    bbb_server_types::fluid::block_state_is_air(state),
                );
            }
        }
        let player = self.levels.player(id).unwrap();
        let damage_due = crate::entity::body_freeze_damage_due(ticks, player.tick_count, || {
            self.player_can_freeze(id)
        })?;
        if damage_due {
            self.hurt_connected_player_checked(
                id,
                "minecraft:freeze",
                1.0,
                None,
                None,
                None,
                events,
            )?;
        }
        Some(())
    }

    pub(in crate::world) fn player_support_for_movement(
        &self,
        id: ConnectionId,
        position: Vec3d,
        movement: Vec3d,
        on_ground: bool,
    ) -> Option<(Option<BlockPos>, bool)> {
        let player = self.levels.player(id)?;
        let bounds =
            player_pose_bounding_box(position, player.pose_id, player.pose_scale()).ok()?;
        let mut supporting = player.block_contact.supporting;
        let mut no_blocks = player.block_contact.on_ground_no_blocks;
        let unavailable = std::cell::Cell::new(false);
        let level = self.level_by_id(&player.dimension);
        crate::entity::check_body_supporting_block_with_moving_pistons(
            position,
            bounds,
            on_ground,
            Some(movement),
            &mut supporting,
            &mut no_blocks,
            &|pos| {
                super::super::movement_collision::player_block_collision_shape_with_context(
                    &level.chunks,
                    player,
                    pos,
                    position.y,
                    false,
                )
                .unwrap_or_else(|| {
                    unavailable.set(true);
                    Vec::new()
                })
            },
            &|pos| {
                super::super::movement_collision::moving_piston_at(&level.chunks, pos)
                    .unwrap_or_else(|| {
                        unavailable.set(true);
                        false
                    })
            },
        );
        (!unavailable.get()).then_some((supporting, no_blocks))
    }

    fn set_player_contact_support(
        &mut self,
        id: ConnectionId,
        movement: Vec3d,
        on_ground: bool,
    ) -> Option<()> {
        let position = self.levels.player(id)?.position();
        let (supporting, no_blocks) =
            self.player_support_for_movement(id, position, movement, on_ground)?;
        let player = self.levels.player_mut(id)?;
        player.block_contact.supporting = supporting;
        player.block_contact.on_ground_no_blocks = no_blocks;
        Some(())
    }

    pub(in crate::world) fn player_contact_state(
        &self,
        id: ConnectionId,
        pos: BlockPos,
    ) -> Option<i32> {
        let player = self.levels.player(id)?;
        let level = self.level_by_id(&player.dimension);
        let state = if level.chunks.is_inside_build_height(pos.y) {
            level.chunks.get_block_state(pos.x, pos.y, pos.z)?
        } else {
            level.chunks.air_state_id()
        };
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state).is_none() {
            level.chunks.content_registry().block_by_state_id(state)?;
        }
        Some(state)
    }

    #[cfg(test)]
    pub(in crate::world) fn check_player_fall_state(
        &mut self,
        id: ConnectionId,
        movement: Vec3d,
        on_ground: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.check_player_fall_state_checked(id, movement, on_ground, events);
    }

    pub(in crate::world) fn check_player_fall_state_checked(
        &mut self,
        id: ConnectionId,
        movement: Vec3d,
        on_ground: bool,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player(id)?;
        let Ok(bounds) =
            player_pose_bounding_box(player.position(), player.pose_id, player.pose_scale())
        else {
            return None;
        };
        let expanded = bounds.inflate(1.0, 1.0, 1.0);
        let level = self.level_by_id(&player.dimension);
        let loaded = ((expanded.min_x.floor() as i32 >> 4)..=(expanded.max_x.ceil() as i32 >> 4))
            .all(|x| {
                ((expanded.min_z.floor() as i32 >> 4)..=(expanded.max_z.ceil() as i32 >> 4))
                    .all(|z| level.chunks.is_ticking_full(x, z))
            });
        if !loaded {
            return Some(());
        }
        self.set_player_contact_support(id, movement, on_ground)?;
        let pos = self.player_legacy_support_pos(id)?;
        let state = self.player_contact_state(id, pos)?;
        if on_ground {
            self.emit_player_extra_fall_particles(id, state, pos, events)?;
        }
        let player = self.levels.player(id)?;
        if !player.fluid_interaction.is_in_fluid(FluidTag::Water) {
            self.refresh_player_fluid_contact(id, events)?;
        }
        if on_ground && self.levels.player(id)?.block_contact.fall_distance > 0.0 {
            self.player_on_changed_block_checked(id, events)?;
        }
        if on_ground {
            self.emit_player_fall_particles(id, state, pos, events)?;
        }
        let player = self.levels.player_mut(id)?;
        player.block_contact.fall_distance = crate::entity::accumulate_body_fall_distance(
            player.block_contact.fall_distance,
            movement.y,
            player
                .fluid_interaction
                .is_in_fluid(crate::entity::FluidTag::Water),
        );
        if on_ground {
            let distance = player.block_contact.fall_distance;
            if distance > 0.0 {
                self.player_block_fall_on(id, state, pos, distance, events)?;
                let player = self.levels.player(id)?;
                let position = player.position();
                let affected = match player.block_contact.supporting {
                    Some(pos) => self.player_contact_state(id, pos)?,
                    None => state,
                };
                self.player_game_event_at(
                    id,
                    crate::level_event::VanillaGameEvent::HIT_GROUND,
                    position,
                    Some(affected),
                );
            }
            self.reset_player_fall_distance(id, events);
            self.levels.player_mut(id)?.block_contact.last_climbable_pos = None;
        }
        Some(())
    }

    #[cfg(test)]
    pub(in crate::world) fn update_player_contact_support(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.update_player_contact_support_checked(id, events);
    }

    pub(in crate::world) fn update_player_contact_support_checked(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player(id)?;
        let movement = player.known_movement;
        let on_ground = player.on_ground();
        self.check_player_fall_state_checked(id, movement, on_ground, events)?;
        if movement.y > 0.0 {
            self.reset_player_fall_distance(id, events);
        }
        Some(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod hazard_tests;

#[cfg(test)]
mod cobweb_tests;

#[cfg(test)]
mod ground_history_packet_tests;

#[cfg(test)]
mod ground_history_tests;
