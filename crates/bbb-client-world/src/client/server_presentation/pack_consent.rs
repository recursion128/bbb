use super::*;
use crate::{DialogState, ScreenRequest};

impl WorldStore {
    pub fn resource_pack_preference(&self) -> Option<bool> {
        self.presentation.pack_consent
    }

    pub fn set_resource_pack_preference(&mut self, preference: Option<bool>) {
        self.presentation.pack_consent = preference;
    }

    pub fn request_resource_pack_consent(&mut self, id: Uuid) {
        let presentation = &mut *self.presentation;
        let pack = presentation
            .resource_packs
            .get_mut(&id)
            .expect("push precedes consent");
        if let Some(accept) = presentation
            .pack_consent
            .filter(|accept| *accept || !pack.required)
        {
            let action = consent_action(accept);
            presentation
                .pack_choices
                .push((id, pack.push_sequence, action));
        } else {
            pack.awaiting_consent = true;
            self.push_screen_request(ScreenRequest::RefreshResourcePackPrompt);
        }
    }

    pub fn resource_pack_prompt(
        &self,
        resolve_text: impl Fn(&bbb_protocol::NbtSummaryValue) -> Vec<bbb_protocol::StyledTextRun>,
    ) -> Option<DialogState> {
        let mut pending = self
            .resource_packs()
            .values()
            .filter(|pack| pack.awaiting_consent);
        let required = pending.clone().any(|pack| pack.required);
        let latest = pending.by_ref().max_by_key(|pack| pack.push_sequence)?;
        Some(DialogState::resource_pack_prompt(
            latest.push_sequence,
            required,
            latest.prompt_component.as_ref(),
            resolve_text,
        ))
    }

    pub fn answer_resource_pack_prompt(&mut self, latest_sequence: u64, accepted: bool) -> bool {
        let presentation = &mut *self.presentation;
        let mut pending: Vec<_> = presentation
            .resource_packs
            .values_mut()
            .filter(|pack| pack.awaiting_consent)
            .collect();
        pending.sort_unstable_by_key(|pack| pack.push_sequence);
        if pending.last().map(|pack| pack.push_sequence) != Some(latest_sequence) {
            return false;
        }
        if accepted || !pending.iter().any(|pack| pack.required) {
            presentation.pack_consent = Some(accepted);
        }
        for pack in pending {
            pack.awaiting_consent = false;
            presentation
                .pack_choices
                .push((pack.id, pack.push_sequence, consent_action(accepted)));
        }
        true
    }

    pub fn take_resource_pack_choices(
        &mut self,
    ) -> Vec<(Uuid, u64, ProtocolResourcePackResponseAction)> {
        std::mem::take(&mut self.presentation.pack_choices)
    }
}

fn consent_action(accepted: bool) -> ProtocolResourcePackResponseAction {
    if accepted {
        ProtocolResourcePackResponseAction::Accepted
    } else {
        ProtocolResourcePackResponseAction::Declined
    }
}
