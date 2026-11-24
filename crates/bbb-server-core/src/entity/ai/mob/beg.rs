//! A wolf's interested pose while watching a player holding bone or food.

use bbb_protocol::item_types::{vanilla_item_type, VANILLA_ITEM_BONE_ID};
use bbb_server_types::geometry::Vec3Ops;

use super::look_at::{nearest_look_at_target, LookAtTargetKind};
use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(crate) struct BegGoal {
    player: Option<i32>,
    look_time: i32,
}

impl BegGoal {
    fn interesting(targets: &TargetView<'_, '_, '_>, id: i32) -> bool {
        targets
            .facts(id)
            .and_then(|target| target.player())
            .is_some_and(|player| {
                [player.main_hand_item_id, player.off_hand_item_id]
                    .into_iter()
                    .any(|item| {
                        item == Some(VANILLA_ITEM_BONE_ID)
                            || item.and_then(vanilla_item_type).is_some_and(|item| {
                                targets.level().registry_tag_membership(
                                    "minecraft:item",
                                    "minecraft:wolf_food",
                                    item.resource_id,
                                )
                            })
                    })
            })
    }
}

/// `BegGoal`.
impl Goal for BegGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.player = nearest_look_at_target(
            targets,
            LookAtTargetKind::Player,
            8.0,
            context.body.position,
            |_| true,
        )
        .map(|target| target.entity_id());
        self.player.is_some_and(|id| Self::interesting(targets, id))
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.player
            .and_then(|id| targets.facts(id))
            .and_then(|target| target.player())
            .is_some_and(|player| {
                !player.dead_or_dying
                    && context.body.position.distance_to_sqr(player.position) <= 64.0
                    && self.look_time > 0
                    && Self::interesting(targets, player.entity_id)
            })
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.species.wolf_interested = Some(true);
        self.look_time = reduced_tick_delay(40 + context.next_int(40));
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.species.wolf_interested = Some(false);
        self.player = None;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(player) = self
            .player
            .and_then(|id| targets.facts(id))
            .and_then(|target| target.player())
        else {
            return;
        };
        let crate::entity::EntityData::GeneralLiving(data) = targets.body_entity().data() else {
            unreachable!("begging is a wolf goal");
        };
        context.set_look_at_player_with_limits(*player, 10.0, data.tamable_max_head_x_rot() as f32);
        self.look_time -= 1;
    }
}
