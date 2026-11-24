use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID;
use bbb_protocol::packets::inventory::ResolvableProfileSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::{read_item_component, UnavailableHolderResolver};

pub(crate) fn decode_profile(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<ResolvableProfileSummary, String> {
    let patch = read_item_component(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        value,
        &UnavailableHolderResolver,
    )
    .map_err(|error| format!("invalid {what}: {error}"))?;
    Ok(patch
        .profile
        .expect("the profile reader populates its field"))
}
