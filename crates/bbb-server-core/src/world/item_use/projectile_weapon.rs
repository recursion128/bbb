//! The session half of `ProjectileWeaponItem`: `Player.getProjectile`
//! (`Player.java:1854-1875`), drawing from the slot it names, and the volley
//! loop of `ProjectileWeaponItem.shoot` (`ProjectileWeaponItem.java:43-77`)
//! with the weapon worn down after each projectile.

use crate::item_durability::ValueContext;
use crate::loot_runtime::LootEntity;
use bbb_protocol::packets::InteractionHand;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::connection::ConnectionId;
use crate::entity::projectile_shoot::{
    movement_to_shoot, shoot_from_rotation, ProjectileShot, ShotParameters, ShotSource,
};
use crate::entity::projectile_weapon::{
    crossbow_shot_direction, crossbow_shot_pitch, draw, durability_use, is_arrow, is_firework,
    spawn_projectile, volley_angles, volley_spread, Shooter,
};
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{ServerWorld, WorldEvent};

/// Where `Player.getProjectile` found its stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectileSlot {
    Hand(InteractionHand),
    Inventory(usize),
    /// `new ItemStack(Items.ARROW)` for a player with infinite materials.
    Conjured,
}

/// How a volley is aimed and loosed.
#[derive(Debug, Clone, Copy)]
pub(super) struct VolleyShape {
    pub(super) crossbow: bool,
    pub(super) power: f32,
    pub(super) uncertainty: f32,
    pub(super) crit: bool,
}

fn supported(stack: &RuntimeItemStack, crossbow_held: bool) -> bool {
    is_arrow(stack) || (crossbow_held && is_firework(stack))
}

impl ServerWorld {
    fn player_projectile_slot(
        &self,
        connection_id: ConnectionId,
        crossbow: bool,
    ) -> Option<ProjectileSlot> {
        let player = self.levels.player(connection_id)?;
        for hand in [InteractionHand::OffHand, InteractionHand::MainHand] {
            if supported(runtime_item_in_hand(player, hand), crossbow) {
                return Some(ProjectileSlot::Hand(hand));
            }
        }
        if let Some(index) = player
            .inventory
            .inventory_slots()
            .iter()
            .position(|stack| supported(stack, false))
        {
            return Some(ProjectileSlot::Inventory(index));
        }
        player
            .abilities
            .instabuild
            .then_some(ProjectileSlot::Conjured)
    }

    /// `!player.getProjectile(weapon).isEmpty()`.
    pub(super) fn player_has_projectile(
        &self,
        connection_id: ConnectionId,
        crossbow: bool,
    ) -> bool {
        self.player_projectile_slot(connection_id, crossbow)
            .is_some()
    }

    /// `ProjectileWeaponItem.draw(weapon, player.getProjectile(weapon), player)`,
    /// writing the used ammo back to its slot.
    pub(super) fn player_draw_projectiles(
        &mut self,
        connection_id: ConnectionId,
        weapon: &RuntimeItemStack,
        crossbow: bool,
    ) -> Vec<RuntimeItemStack> {
        let Some(slot) = self.player_projectile_slot(connection_id, crossbow) else {
            return Vec::new();
        };
        let player = &self.levels[connection_id];
        let infinite = player.abilities.instabuild;
        let mut projectile = match slot {
            ProjectileSlot::Hand(hand) => runtime_item_in_hand(player, hand).clone(),
            ProjectileSlot::Inventory(index) => player
                .inventory
                .slot(index)
                .cloned()
                .unwrap_or_else(RuntimeItemStack::empty),
            ProjectileSlot::Conjured => crate::entity::item::plain_stack("minecraft:arrow", 1)
                .unwrap_or_else(RuntimeItemStack::empty),
        };
        let dimension = player.dimension;
        let origin = player.position();
        let view = self.session_loot_view(connection_id);
        let level = self.level_by_id(&dimension);
        let drawn = draw(
            level.entities.weapon_effects(),
            &self.random,
            ValueContext::Entity {
                this: view.as_ref().map_or(LootEntity::Opaque, LootEntity::Facts),
                origin,
            },
            weapon,
            &mut projectile,
            infinite,
            &self.value_effect_level(&dimension),
        );
        let player = &mut self.levels[connection_id];
        match slot {
            ProjectileSlot::Hand(hand) => player.inventory.set_item_in_hand(hand, projectile),
            ProjectileSlot::Inventory(index) => {
                player.inventory.set_inventory_slot(index, projectile);
            }
            ProjectileSlot::Conjured => {}
        }
        drawn
    }

    /// `ProjectileWeaponItem.shoot` for a session's weapon in `hand`; answers
    /// whether the weapon is still there.
    pub(super) fn player_shoot_volley(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        projectiles: Vec<RuntimeItemStack>,
        shape: VolleyShape,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let player = &self.levels[connection_id];
        let weapon = runtime_item_in_hand(player, hand).clone();
        let dimension = player.dimension;
        let position = player.position();
        let Ok(eye) = player.eye_position() else {
            return true;
        };
        let rotation = player.rotation();
        let source = ShotSource {
            known_movement: player.known_movement,
            on_ground: player.on_ground(),
        };
        let shooter = Shooter {
            uuid: player.profile.uuid,
            player: true,
            position,
            eye_y: eye.y,
        };
        let view = self.session_loot_view(connection_id);
        let level = self.level_by_id(&dimension);
        let spread = volley_spread(
            level.entities.weapon_effects(),
            &self.random,
            ValueContext::Entity {
                this: view.as_ref().map_or(LootEntity::Opaque, LootEntity::Facts),
                origin: position,
            },
            &weapon,
            &self.value_effect_level(&dimension),
        );
        let angles = volley_angles(projectiles.len(), spread);
        for (index, (projectile, angle)) in projectiles.into_iter().zip(angles).enumerate() {
            if projectile.is_empty() {
                continue;
            }
            let durability = durability_use(shape.crossbow, &projectile);
            let (value_level, store) = self.value_effect_level_and_entities(&dimension);
            let spawn = |store: &mut _, shot: ProjectileShot| {
                spawn_projectile(
                    store,
                    shooter,
                    &weapon,
                    projectile,
                    shape.crit,
                    shape.crossbow,
                    shot,
                    &value_level,
                    |_| {},
                )
            };
            if shape.crossbow {
                // `CrossbowItem.shootProjectile` (`CrossbowItem.java:107-136`).
                let direction =
                    crossbow_shot_direction(rotation.x_rot, rotation.y_rot, None, angle);
                let movement = movement_to_shoot(store, direction, shape.power, shape.uncertainty);
                let _ = spawn(store, ProjectileShot::new(movement));
                let pitch = crossbow_shot_pitch(&self.random, index);
                let _ = self.play_entity_sound(
                    connection_id,
                    None,
                    "minecraft:item.crossbow.shoot",
                    1.0,
                    pitch,
                    events,
                );
            } else {
                // `BowItem.shootProjectile` (`BowItem.java:62-72`).
                let _ = shoot_from_rotation(
                    store,
                    source,
                    ShotParameters {
                        x_rot: rotation.x_rot,
                        y_rot: rotation.y_rot + angle,
                        y_offset: 0.0,
                        pow: shape.power,
                        uncertainty: shape.uncertainty,
                    },
                    spawn,
                );
            }
            if !self.hurt_and_break_in_hand(connection_id, hand, durability, events) {
                return false;
            }
        }
        true
    }
}
