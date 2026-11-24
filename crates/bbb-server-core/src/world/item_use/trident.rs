//! `TridentItem` (`TridentItem.java:31-143`): held to charge, then thrown,
//! or with riptide launched with the player in water or rain.

use bbb_protocol::packets::{InteractionHand, SoundSource, Vec3d};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::{ItemUseContext, ItemUseResult};
use crate::connection::ConnectionId;
use crate::enchantment_weapon_effects::{pick_highest_level, process};
use crate::entity::abstract_arrow::ArrowPickup;
use crate::entity::projectile_shoot::{shoot_from_rotation, ShotParameters, ShotSource};
use crate::entity::FluidTag;
use crate::item_durability::{hurt_without_breaking, next_damage_will_break};
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{ServerWorld, WorldEvent};

/// `TridentItem.getUseDuration` (`TridentItem.java:57-59`).
pub(in crate::world) const USE_DURATION: i32 = 72000;

/// `TridentItem.THROW_THRESHOLD_TIME` (`TridentItem.java:32`).
const THROW_THRESHOLD_TIME: i32 = 10;

impl ServerWorld {
    /// `EnchantmentHelper.getTridentSpinAttackStrength`
    /// (`EnchantmentHelper.java:429-433`).
    fn trident_spin_attack_strength(
        &self,
        connection_id: ConnectionId,
        stack: &RuntimeItemStack,
    ) -> f32 {
        let dimension = self.levels[connection_id].dimension;
        let level = self.level_by_id(&dimension);
        process(
            level.entities.weapon_effects(),
            stack.summary(),
            crate::item_durability::ValueContext::Item(stack.summary()),
            |effects| effects.trident_spin_attack_strength.as_ref(),
            0.0,
            &self.random,
            &self.value_effect_level(&dimension),
        )
    }

    /// `Entity.isInWaterOrRain` (`Entity.java:1514-1516`), rain checked at the feet and
    /// at the top of the box as `isInRain` does.
    fn player_in_water_or_rain(&self, connection_id: ConnectionId) -> bool {
        let player = &self.levels[connection_id];
        if player.fluid_interaction.is_in_fluid(FluidTag::Water) {
            return true;
        }
        let feet = crate::world::player_packets::block_position(player.position());
        let top = player
            .bounding_box()
            .map_or(feet.y, |bounds| bounds.max_y.floor() as i32);
        let level = self.level_by_id(&player.dimension);
        level.weather.is_raining_at(&level.chunks, feet)
            || level.weather.is_raining_at(
                &level.chunks,
                bbb_protocol::packets::BlockPos { y: top, ..feet },
            )
    }

    /// `ItemStack.hurtWithoutBreaking(1, player)` on the stack in `hand`.
    fn hurt_without_breaking_in_hand(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) {
        let infinite = self.player_has_infinite_materials(connection_id);
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_item_damage);
        let old_stack = runtime_item_in_hand(&self.levels[connection_id], hand).clone();
        let mut stack = old_stack.summary().clone();
        let enchantments = Self::resolve_value_effects(&stack, &table);
        let raw_new_damage = hurt_without_breaking(
            &mut stack,
            1,
            infinite,
            &enchantments,
            &self.random,
            &self.value_effect_level(&self.player_level_id(connection_id)),
        );
        drop(enchantments);
        if let Some(raw_new_damage) = raw_new_damage {
            self.trigger_item_durability_changed(connection_id, &old_stack, raw_new_damage, events);
        }
        self.update_hand_stack(connection_id, hand, RuntimeItemStack::from_summary(stack));
    }
}

/// `TridentItem.use` (`TridentItem.java:123-135`).
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    _events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    let stack = runtime_item_in_hand(&world.levels[context.connection_id], context.hand).clone();
    if next_damage_will_break(stack.summary()) {
        return ItemUseResult::Fail;
    }
    if world.trident_spin_attack_strength(context.connection_id, &stack) > 0.0
        && !world.player_in_water_or_rain(context.connection_id)
    {
        return ItemUseResult::Fail;
    }
    world.start_using_item(context.connection_id, context.hand);
    ItemUseResult::Consume
}

/// `TridentItem.releaseUsing` (`TridentItem.java:62-120`).
pub(in crate::world) fn release_using(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    hand: InteractionHand,
    remaining: i32,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if USE_DURATION - remaining < THROW_THRESHOLD_TIME {
        return false;
    }
    let stack = runtime_item_in_hand(&world.levels[connection_id], hand).clone();
    let riptide = world.trident_spin_attack_strength(connection_id, &stack);
    let dimension = world.levels[connection_id].dimension;
    let riding = world
        .level_by_id(&dimension)
        .entities
        .external_player_vehicle(world.levels[connection_id].entity_id())
        .is_some();
    if riptide > 0.0 && !(world.player_in_water_or_rain(connection_id) && !riding) {
        return false;
    }
    if next_damage_will_break(stack.summary()) {
        return false;
    }
    let sound = pick_highest_level(
        world.level_by_id(&dimension).entities.weapon_effects(),
        stack.summary(),
        |effects| &effects.trident_sounds,
    )
    .cloned()
    .unwrap_or_else(|| {
        crate::entity::sound_variants::registered_sound("minecraft:item.trident.throw")
            .expect("trident throw sound is published")
    });
    if let Some(item_id) = stack.item_id() {
        world.levels[connection_id].award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    world.hurt_without_breaking_in_hand(connection_id, hand, events);
    let Some(player) = world.levels.player(connection_id) else {
        return false;
    };
    let stack = runtime_item_in_hand(player, hand).clone();
    let position = player.position();
    let rotation = player.rotation();
    if riptide == 0.0 {
        let infinite = player.abilities.instabuild;
        let owner = player.profile.uuid;
        let source = ShotSource {
            known_movement: player.known_movement,
            on_ground: player.on_ground(),
        };
        let Ok(eye) = player.eye_position() else {
            return false;
        };
        // `ItemStack.consumeAndReturn(1, player)`.
        let mut thrown = stack.clone();
        thrown.set_count(1);
        if !infinite {
            let mut left = stack;
            left.set_count(left.count() - 1);
            world.update_hand_stack(connection_id, hand, left);
        }
        let pickup = if infinite {
            ArrowPickup::CreativeOnly
        } else {
            ArrowPickup::Allowed
        };
        let spawn_at = Vec3d {
            y: eye.y - f64::from(0.1_f32),
            ..position
        };
        let (value_level, store) = world.value_effect_level_and_entities(&dimension);
        let spawned = shoot_from_rotation(
            store,
            source,
            ShotParameters {
                x_rot: rotation.x_rot,
                y_rot: rotation.y_rot,
                y_offset: 0.0,
                pow: 2.5,
                uncertainty: 1.0,
            },
            |store, shot| {
                crate::entity::trident::spawn_thrown(
                    store,
                    owner,
                    spawn_at,
                    thrown,
                    pickup,
                    shot,
                    &value_level,
                )
            },
        );
        // `:87`: `playSound(null, trident, ..)` follows the trident.
        if let Some(id) = spawned {
            store.play_sound_holder_from_entity_at(
                id,
                spawn_at,
                sound,
                SoundSource::Players,
                1.0,
                1.0,
            );
        }
        return true;
    }
    // `:92-110`: the push the client predicts for itself; the server's copy of
    // the body follows the next movement packet.
    let to_radians = std::f32::consts::PI / 180.0;
    let (y_rot, x_rot) = (rotation.y_rot * to_radians, rotation.x_rot * to_radians);
    let mut xd = -y_rot.sin() * x_rot.cos();
    let mut yd = -x_rot.sin();
    let mut zd = y_rot.cos() * x_rot.cos();
    let dist = (xd * xd + yd * yd + zd * zd).sqrt();
    xd *= riptide / dist;
    yd *= riptide / dist;
    zd *= riptide / dist;
    let player = &mut world.levels[connection_id];
    let delta = player.delta_movement();
    player.set_delta_movement(Vec3d {
        x: delta.x + f64::from(xd),
        y: delta.y + f64::from(yd),
        z: delta.z + f64::from(zd),
    });
    player.using_item.start_auto_spin_attack(20, 8.0, stack);
    world.remove_player_shoulders(connection_id, events);
    // `:109`: `playSound(null, player, ..)` follows the player.
    let player = &world.levels[connection_id];
    let (entity_id, position) = (player.entity_id(), player.position());
    world
        .level_mut_by_id(&dimension)
        .entities
        .play_sound_holder_from_entity_at(
            entity_id,
            position,
            sound,
            SoundSource::Players,
            1.0,
            1.0,
        );
    true
}
