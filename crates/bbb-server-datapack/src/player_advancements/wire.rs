//! The typed `ClientboundUpdateAdvancementsPacket` value a flushed update
//! projects into (`ClientboundUpdateAdvancementsPacket.java:49-55`). The byte
//! encoder over it belongs to the protocol crate.

use bbb_protocol::packets::{
    AdvancementCriterionProgressSummary, AdvancementDisplaySummary, AdvancementFrameType,
    AdvancementIconSummary, AdvancementProgressSummary,
    AdvancementSummary as WireAdvancementSummary, UpdateAdvancements,
};
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::{read_item_stack, PersistentItemStackHolderResolver};

use super::display::{AdvancementDisplay, AdvancementFrame};
use super::progress::AdvancementProgress;
use super::store::AdvancementsUpdate;
use super::tree::AdvancementSummary;

/// The update as `ClientboundUpdateAdvancementsPacket.write` sends it. An
/// icon whose component patch the persistent projection cannot own yet is a
/// refusal of the whole update rather than a wrong packet.
pub fn advancements_update_to_wire(
    update: &AdvancementsUpdate,
    resolver: &impl PersistentItemStackHolderResolver,
) -> Result<UpdateAdvancements, String> {
    Ok(UpdateAdvancements {
        reset: update.reset,
        added: update
            .added
            .iter()
            .map(|summary| advancement_to_wire(summary, resolver))
            .collect::<Result<_, _>>()?,
        removed: update.removed.clone(),
        progress: update
            .progress
            .iter()
            .map(|(id, progress)| progress_to_wire(id, progress))
            .collect(),
        show_advancements: update.show_advancements,
    })
}

/// `Advancement.write` (`Advancement.java:83-88`).
fn advancement_to_wire(
    summary: &AdvancementSummary,
    resolver: &impl PersistentItemStackHolderResolver,
) -> Result<WireAdvancementSummary, String> {
    Ok(WireAdvancementSummary {
        id: summary.id.clone(),
        parent: summary.parent.clone(),
        display: summary
            .display
            .as_ref()
            .map(|display| display_to_wire(&summary.id, display, resolver))
            .transpose()?,
        requirements: summary.requirements.groups().to_vec(),
        sends_telemetry_event: summary.sends_telemetry_event,
    })
}

/// `DisplayInfo.serializeToNetwork` (`DisplayInfo.java:106-128`): the flag
/// bits are background, toast and hidden; `announce_to_chat` never goes out.
fn display_to_wire(
    id: &str,
    display: &AdvancementDisplay,
    resolver: &impl PersistentItemStackHolderResolver,
) -> Result<AdvancementDisplaySummary, String> {
    Ok(AdvancementDisplaySummary {
        title: display.title.clone(),
        description: display.description.clone(),
        icon: icon_to_wire(id, &display.icon, resolver)?,
        frame_type: match display.frame {
            AdvancementFrame::Task => AdvancementFrameType::Task,
            AdvancementFrame::Challenge => AdvancementFrameType::Challenge,
            AdvancementFrame::Goal => AdvancementFrameType::Goal,
        },
        show_toast: display.show_toast,
        hidden: display.hidden,
        background: display.background.clone(),
        x: display.x,
        y: display.y,
    })
}

/// `ItemStackTemplate.STREAM_CODEC` (`ItemStackTemplate.java:30-38`): the
/// item holder id, the count and the component patch. The template's fields
/// go back through the persistent stack codec the save path owns, which reads
/// the same `{id, count, components}` shape (`ItemStack.java:111-122`).
fn icon_to_wire(
    id: &str,
    icon: &NbtSummaryValue,
    resolver: &impl PersistentItemStackHolderResolver,
) -> Result<AdvancementIconSummary, String> {
    let stack =
        read_item_stack(icon, resolver).map_err(|error| format!("the icon of {id}: {error}"))?;
    Ok(AdvancementIconSummary {
        item_id: stack
            .item_id
            .ok_or_else(|| format!("the icon of {id} projected to an empty stack"))?,
        count: stack.count,
        component_patch: stack.component_patch,
    })
}

/// `AdvancementProgress.serializeToNetwork` (`AdvancementProgress.java:105-107`)
/// writes every criterion with its optional obtained instant.
fn progress_to_wire(id: &str, progress: &AdvancementProgress) -> AdvancementProgressSummary {
    AdvancementProgressSummary {
        id: id.to_string(),
        criteria: progress
            .criteria()
            .iter()
            .map(|(name, criterion)| AdvancementCriterionProgressSummary {
                name: name.clone(),
                obtained_epoch_millis: criterion.obtained_epoch_millis(),
            })
            .collect(),
    }
}
