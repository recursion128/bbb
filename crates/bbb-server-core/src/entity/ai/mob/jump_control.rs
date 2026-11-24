use super::spec::mob_spec;
use super::MobAiContext;

#[derive(Debug, Clone, Default)]
pub(super) struct JumpControl {
    jump: bool,
}

impl JumpControl {
    pub(super) fn for_entity(entity_type_id: i32) -> Option<Self> {
        mob_spec(entity_type_id)
            .is_none_or(|spec| spec.jump_control)
            .then(Self::default)
    }

    // `JumpControl` latches requests for exactly one controller tick
    pub(super) fn jump(&mut self) {
        self.jump = true;
    }

    pub(super) fn tick(&mut self, context: &mut MobAiContext) {
        context.controls.jumping = self.jump;
        self.jump = false;
        context.controls.jump_requested = false;
    }
}
