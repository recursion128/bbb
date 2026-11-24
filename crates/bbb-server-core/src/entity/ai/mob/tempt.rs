use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_GOAT_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
};
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::{Entity, EntityData, NearbyPlayer};

use super::body_rotation::{base_body_rotation_max_yaw, base_max_head_x_rot};
use super::random_look::HEAD_ROT_SPEED;
use super::target::{conditions::TargetingConditions, TargetView};
use super::{reduced_tick_delay, Goal, MobAiContext};

// `TemptGoal.DEFAULT_STOP_DISTANCE` (`TemptGoal.java:17`).
const DEFAULT_STOP_DISTANCE: f64 = 2.5;

/// The `TemptGoal` item predicate: an item tag or one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemptItems {
    Tag(&'static str),
    Item(i32),
    HappyGhast,
}

/// Which `TemptGoal` subclass decides `canScare`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemptScare {
    Never,
    /// `Cat.CatTemptGoal`: untamed only, and never
    /// scared by a player it has picked.
    Cat,
    /// `Ocelot.OcelotTemptGoal`: scared until trusting.
    Ocelot,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TemptSpec {
    items: TemptItems,
    scare: TemptScare,
    speed_modifier: f64,
    look_y_max_rot_speed: f32,
    look_x_max_rot_angle: f32,
}

impl TemptSpec {
    pub(super) fn new(
        items: TemptItems,
        scare: TemptScare,
        speed_modifier: f64,
        entity_type_id: i32,
    ) -> Option<Self> {
        // Brain `FollowTemptation` uses `LookControl.setLookAt(Vec3)`, which
        // reads `Mob.getHeadRotSpeed` and `Mob.getMaxHeadXRot`.
        let look_y_max_rot_speed = if uses_follow_temptation(entity_type_id)
            || entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        {
            HEAD_ROT_SPEED
        } else {
            base_body_rotation_max_yaw(entity_type_id)? + 20.0
        };
        Some(Self {
            items,
            scare,
            speed_modifier,
            look_y_max_rot_speed,
            look_x_max_rot_angle: base_max_head_x_rot(entity_type_id)?,
        })
    }
}

// An empty hand holds `Items.AIR`, which no tempting tag contains.
fn follows_items(
    source: &Entity,
    level: &LevelCtx<'_, '_>,
    items: TemptItems,
    item_type_id: Option<i32>,
) -> bool {
    let tag = match items {
        TemptItems::Item(wanted) => return item_type_id == Some(wanted),
        TemptItems::Tag(tag) => tag,
        TemptItems::HappyGhast => {
            let Some(wearing) =
                crate::entity::living::valid_equipment::has_valid_item_with_resolver(
                    source,
                    bbb_protocol::equipment_slots::EquipmentSlot::Body,
                    level.item_stack_resolver(),
                )
            else {
                return false;
            };
            let baby = matches!(source.data(), EntityData::GeneralLiving(data) if data.is_baby());
            if wearing || baby {
                "minecraft:happy_ghast_food"
            } else {
                "minecraft:happy_ghast_tempt_items"
            }
        }
    };
    let Some(item) = item_type_id.and_then(vanilla_item_type) else {
        return false;
    };
    level.registry_tag_membership("minecraft:item", tag, item.resource_id)
}

#[derive(Debug, Clone)]
pub(crate) struct TemptGoal {
    spec: TemptSpec,
    calm_down: i32,
    target_id: Option<i32>,
    /// The player's position and rotation `canScare` compares against.
    seen: (Vec3d, f32, f32),
    cat_selected: Option<i32>,
}

impl TemptGoal {
    pub(super) const fn new(spec: TemptSpec) -> Self {
        Self {
            spec,
            calm_down: 0,
            target_id: None,
            seen: (
                Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                0.0,
                0.0,
            ),
            cat_selected: None,
        }
    }

    fn can_scare(&self, targets: &crate::entity::ai::mob::target::TargetView<'_, '_, '_>) -> bool {
        match self.spec.scare {
            TemptScare::Never => false,
            TemptScare::Cat => self.cat_selected.is_none() || self.cat_selected != self.target_id,
            TemptScare::Ocelot => {
                !matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                if data.ocelot_trusting().unwrap_or(false))
            }
        }
    }

    fn target<'a>(&self, targets: &'a TargetView<'_, '_, '_>) -> Option<&'a NearbyPlayer> {
        let id = self.target_id?;
        targets
            .level()
            .players()
            .iter()
            .find(|player| player.entity_id == id)
    }
}

pub(super) fn nearest_tempting_player(
    context: &MobAiContext,
    targets: &TargetView<'_, '_, '_>,
    items: TemptItems,
    brain_temptation: bool,
) -> Option<i32> {
    let conditions = TargetingConditions::for_non_combat()
        .range(context.body.tempt_range)
        .ignore_line_of_sight();
    let mut nearest: Option<(f64, i32)> = None;
    for candidate in targets.local_players() {
        if !conditions.test(targets, &candidate, |_| true) {
            continue;
        }
        let player = candidate.player().expect("local player");
        if !(follows_items(
            targets.body_entity(),
            targets.level(),
            items,
            player.main_hand_item_id,
        ) || follows_items(
            targets.body_entity(),
            targets.level(),
            items,
            player.off_hand_item_id,
        )) {
            continue;
        }
        // `TemptingSensor.doTick` excludes the body's own passengers.
        if brain_temptation
            && targets
                .store()
                .passengers(targets.body_entity().id())
                .contains(&player.entity_id)
        {
            continue;
        }
        let distance = context.body.position.distance_to_sqr(player.position);
        if nearest.is_none_or(|(best, _)| distance < best) {
            nearest = Some((distance, player.entity_id));
        }
    }
    nearest.map(|(_, id)| id)
}

impl Goal for TemptGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        let brain_temptation = uses_follow_temptation(targets.body_entity().entity_type_id())
            || (targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
                && self.spec.items != TemptItems::HappyGhast);
        if self.calm_down > 0 {
            self.calm_down -= 1;
            return false;
        }
        self.target_id =
            nearest_tempting_player(context, targets, self.spec.items, brain_temptation);
        self.target_id.is_some()
            && !(self.spec.scare == TemptScare::Cat
                && matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                    if data.is_tame()))
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        if self.can_scare(targets) {
            if let Some(player) = self.target(targets) {
                let (seen, x_rot, y_rot) = self.seen;
                if context.body.position.distance_to_sqr(player.position) < 36.0 {
                    if player.position.distance_to_sqr(seen) > 0.010000000000000002
                        || (player.x_rot - x_rot).abs() > 5.0
                        || (player.y_rot - y_rot).abs() > 5.0
                    {
                        return false;
                    }
                } else {
                    self.seen.0 = player.position;
                }
                self.seen.1 = player.x_rot;
                self.seen.2 = player.y_rot;
            }
        }
        self.can_use(context, targets)
    }

    fn start(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        if let Some(player) = self.target(targets) {
            self.seen.0 = player.position;
        }
    }

    fn stop(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.target_id = None;
        if self.spec.items == TemptItems::HappyGhast {
            context.species.ghast_wanted_position = None;
        } else {
            context.stop_navigation();
        }
        self.calm_down = reduced_tick_delay(100);
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        let Some(target) = self.target(targets) else {
            return;
        };
        if self.spec.scare == TemptScare::Cat {
            if self.cat_selected.is_none() && context.next_int(reduced_tick_delay(600)) == 0 {
                self.cat_selected = self.target_id;
            } else if context.next_int(reduced_tick_delay(500)) == 0 {
                self.cat_selected = None;
            }
        }
        let eye_y = target.position.y + target.eye_height;
        if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            context.set_look_at(target.position.x, eye_y, target.position.z);
        } else {
            context.set_look_at_with_limits(
                target.position.x,
                eye_y,
                target.position.z,
                self.spec.look_y_max_rot_speed,
                self.spec.look_x_max_rot_angle,
            );
        }
        let stop_distance = stop_distance(targets.body_entity(), self.spec.items);
        if self.spec.items == TemptItems::HappyGhast {
            context.species.ghast_wanted_position =
                if context.body.position.distance_to_sqr(target.position)
                    < stop_distance * stop_distance
                {
                    None
                } else {
                    let eye = Vec3d {
                        y: eye_y,
                        ..target.position
                    };
                    let fraction = context.next_double();
                    Some(
                        context
                            .body
                            .position
                            .plus(eye.plus(context.body.position.scale(-1.0)).scale(fraction)),
                    )
                };
        } else if context.body.position.distance_to_sqr(target.position)
            < stop_distance * stop_distance
        {
            context.stop_navigation();
        } else if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            context.try_navigate_to_position(
                Vec3d {
                    y: eye_y,
                    ..target.position
                },
                2,
                self.spec.speed_modifier,
            );
        } else if targets.body_entity().entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
            // AxolotlAi.getSpeedModifier; FollowTemptation.
            context.try_navigate_to_position(
                target.position,
                2,
                if targets.body_entity().is_in_water() {
                    0.5
                } else {
                    0.15
                },
            );
        } else {
            context.navigate_to(target.position, self.spec.speed_modifier);
        }
    }
}

fn uses_follow_temptation(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ARMADILLO_ID
            | VANILLA_ENTITY_TYPE_AXOLOTL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_GOAT_ID
            | VANILLA_ENTITY_TYPE_SNIFFER_ID
    )
}

fn stop_distance(body: &Entity, items: TemptItems) -> f64 {
    let baby = matches!(body.data(), EntityData::GeneralLiving(data)
        if data.is_baby());
    match body.entity_type_id() {
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => {
            if items == TemptItems::HappyGhast {
                7.0
            } else {
                3.0
            }
        }
        // `CamelAi.initIdleActivity` and `ArmadilloAi.initIdleActivity`.
        VANILLA_ENTITY_TYPE_CAMEL_ID
        | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        | VANILLA_ENTITY_TYPE_SNIFFER_ID => {
            if baby {
                2.5
            } else {
                3.5
            }
        }
        VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
            if baby {
                1.0
            } else {
                2.0
            }
        }
        _ => DEFAULT_STOP_DISTANCE,
    }
}

#[cfg(test)]
mod tests;
