use super::GeneralLivingData;

impl GeneralLivingData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.living.restore_command_transients(&previous.living);
        if let (Some(current), Some(previous)) = (&mut self.mob, &previous.mob) {
            current.restore_command_transients(previous);
        }
        self.kind.restore_command_transients(&previous.kind);
    }
}
