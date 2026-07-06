//! Sign block-entity NBT -> [`SignBlockEntityTextState`] projection.
//!
//! The NBT structure and the styled-component flattening live in
//! `bbb_protocol::decode_sign_block_entity_nbt` (the single styled-run
//! decoder, shared with chat/item components); this module only maps the
//! protocol shape onto the world store's serde state (dye-name string ->
//! [`SignTextDyeColor`], defaults per vanilla `SignText.DIRECT_CODEC`
//! `orElse` fallbacks).

use bbb_protocol::{decode_sign_block_entity_nbt, SignTextNbt};

use crate::Result;

use super::state::{SignBlockEntityTextState, SignTextDyeColor, SignTextSideState};

pub(crate) fn decode_sign_block_entity_text(
    raw_nbt: &[u8],
) -> Result<Option<SignBlockEntityTextState>> {
    let Some(sign) = decode_sign_block_entity_nbt(raw_nbt)? else {
        return Ok(None);
    };
    Ok(Some(SignBlockEntityTextState {
        // Vanilla `loadAdditional`: a missing/malformed side falls back to an
        // empty `SignText` (`orElseGet(SignText::new)` — empty lines, black,
        // not glowing).
        front: sign.front_text.map(sign_text_side).unwrap_or_default(),
        back: sign.back_text.map(sign_text_side).unwrap_or_default(),
        is_waxed: sign.is_waxed,
    }))
}

fn sign_text_side(side: SignTextNbt) -> SignTextSideState {
    SignTextSideState {
        color: side
            .color
            .as_deref()
            .map(SignTextDyeColor::from_name)
            .unwrap_or_default(),
        has_glowing_text: side.has_glowing_text,
        lines: side.messages,
    }
}
