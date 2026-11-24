use super::target::{TargetRef, TargetView};
use super::{Goal, MobAiContext};
#[derive(Clone, Default)]
pub(crate) struct EndermanFreezeWhenLookedAt {
    target: Option<TargetRef>,
}
impl Goal for EndermanFreezeWhenLookedAt {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.target = context
            .valid_target(targets)
            .map(|target| target.target_ref());
        let Some(player) = context
            .valid_target(targets)
            .and_then(|target| target.player())
        else {
            return false;
        };
        let dx = player.position.x - context.body.position.x;
        let dy = player.position.y - context.body.position.y;
        let dz = player.position.z - context.body.position.z;
        if dx * dx + dy * dy + dz * dz > 256.0
            || !crate::entity::living::gaze::not_wearing_disguise(targets.level(), player)
        {
            return false;
        }
        crate::entity::living::gaze::is_looking_at_me(
            targets.level(),
            context.body.position,
            &[context.body.eye_y],
            player,
            0.025,
            true,
            false,
        ) == Some(true)
    }
    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
    }
    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(player) = self
            .target
            .and_then(|target| targets.facts_of(target))
            .and_then(|target| target.player())
        {
            context.set_look_at(
                player.position.x,
                player.position.y + player.eye_height,
                player.position.z,
            );
        }
    }
}
