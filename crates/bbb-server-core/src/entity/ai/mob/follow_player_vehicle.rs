//! Following players riding boats or nautiluses.

use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::directions::{horizontal_direction, opposite, relative};
use bbb_server_types::geometry::Vec3Ops;

#[derive(Debug, Clone, Default)]
pub(super) struct FollowPlayerVehicleGoal {
    player: Option<TargetRef>,
    leading: bool,
    next_repath: i32,
}

impl FollowPlayerVehicleGoal {
    fn moving_player(targets: &TargetView<'_, '_, '_>, player: TargetRef) -> bool {
        targets.facts_of(player).is_some()
            && targets
                .store()
                .player(player.entity_id)
                .is_some_and(|player| player.has_moved_horizontally_recently())
    }
}

impl Goal for FollowPlayerVehicleGoal {
    fn can_use(&mut self, _: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) -> bool {
        let reach = targets.body_entity().bounding_box().inflate(5.0, 5.0, 5.0);
        let tags = targets.level().capture_registry_tags();
        self.player = targets
            .store()
            .ids_in_box_sections(reach)
            .into_iter()
            .find_map(|id| {
                let vehicle = targets.store().get(id)?;
                // Dolphin: both vehicle families use the same following rules.
                let followed = crate::entity::boat::is_boat(vehicle.data())
                    || matches!(
                        vehicle.entity_type_id(),
                        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
                            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
                    );
                if !followed || !vehicle.bounding_box().intersects(reach) {
                    return None;
                }
                let controller = crate::entity::direct_controlling_player_with_registry_tags(
                    vehicle,
                    Some(targets.store()),
                    &tags,
                    |id| targets.level().player_by_entity_id(id),
                )??;
                let player = TargetRef {
                    entity_id: controller.entity_id,
                    uuid: controller.uuid,
                };
                Self::moving_player(targets, player).then_some(player)
            });
        self.player.is_some()
    }

    fn can_continue(&mut self, _: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) -> bool {
        self.player.is_some_and(|player| {
            Self::moving_player(targets, player)
                && targets
                    .store()
                    .external_player_vehicle(player.entity_id)
                    .is_some()
        })
    }

    fn start(&mut self, _: &mut MobAiContext, _: &mut TargetView<'_, '_, '_>) {
        self.leading = false;
        self.next_repath = 0;
    }

    fn stop(&mut self, _: &mut MobAiContext, _: &mut TargetView<'_, '_, '_>) {
        self.player = None;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if context.tick_count < self.next_repath {
            return;
        }
        let Some(player) = self.player.and_then(|player| targets.facts_of(player)) else {
            return;
        };
        let Some(direction) = targets
            .store()
            .player(player.entity_id())
            .and_then(|player| horizontal_direction(player.body().y_rot()))
        else {
            return;
        };
        self.next_repath = context.tick_count.wrapping_add(10);
        let mut destination = player.block_position();
        if self.leading {
            for _ in 0..10 {
                destination = relative(destination, direction);
            }
        } else {
            destination = relative(destination, opposite(direction));
        }
        context.navigate_to_position(
            Vec3d {
                x: f64::from(destination.x),
                y: f64::from(destination.y) - 1.0,
                z: f64::from(destination.z),
            },
            1.0,
        );
        let distance = context.body.position.distance_to_sqr(player.position());
        if !self.leading && distance < 16.0 || self.leading && distance > 144.0 {
            self.leading = !self.leading;
            self.next_repath = 0;
        }
    }
}
