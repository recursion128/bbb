use crate::registry_codec::{
    recursive_values::RegistryCodecDecodeContext, RegistryCodecError, RegistryElementSource,
    RegistryElementValueRef,
};
use bbb_facts::data_component_types::{
    vanilla_data_component_type, VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::NbtSummaryValue;

pub fn decode_loot_book_parameter_source(
    type_id: i32,
    input: RegistryElementValueRef<'_>,
    source: RegistryElementSource,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let mut context =
        RegistryCodecDecodeContext::new("minecraft:loot_function", "BookContent.CODEC", source);
    if !matches!(
        type_id,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID
            | VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID
    ) {
        return Err(context.codec_error("book", "not a book content component"));
    }
    let descriptor = vanilla_data_component_type(type_id).expect("generated book component");
    super::decode_component_value_source(descriptor, input, "book", &mut context)
        .map(|value| value.into_parts().0)
}
