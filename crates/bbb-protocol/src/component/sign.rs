use super::*;

/// One sign face decoded from the sign block entity's NBT — vanilla
/// `SignText.DIRECT_CODEC` (`SignText.java`): `messages` is a list of exactly
/// four chat components (`LINES_CODEC` enforces `Util.fixedSize(_, 4)`), each
/// flattened here into styled runs by the shared component traversal;
/// `filtered_messages` is a lenient optional list of exactly four components
/// and falls back to `messages` when absent or malformed;
/// `color` is the raw `DyeColor.CODEC` name string (`"black"` when absent —
/// `fieldOf("color").orElse(DyeColor.BLACK)`); `has_glowing_text` is the
/// `Codec.BOOL` byte (`orElse(false)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignTextNbt {
    pub messages: [Vec<StyledTextRun>; 4],
    pub filtered_messages: Option<[Vec<StyledTextRun>; 4]>,
    pub color: Option<String>,
    pub has_glowing_text: bool,
}

/// The sign block entity's render-relevant NBT — vanilla
/// `SignBlockEntity.loadAdditional`: the `front_text` / `back_text`
/// `SignText.DIRECT_CODEC` compounds plus the root `is_waxed` boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignBlockEntityNbt {
    pub front_text: Option<SignTextNbt>,
    pub back_text: Option<SignTextNbt>,
    pub is_waxed: bool,
}

/// Decodes a sign block entity's NBT (`SignBlockEntity.loadAdditional`
/// fields). Returns `Ok(None)` when the payload carries neither `front_text`
/// nor `back_text` — every non-sign block entity's NBT flows through here, so
/// absence of the sign keys is the (cheap) sign gate, mirroring how the codec
/// only ever sees sign-shaped data in vanilla. Message components reuse the
/// single styled-component traversal ([`decode_styled_component_summary`]'s
/// `append_component_runs`), so sign lines carry the same inherited-style
/// runs as every other decoded component in the workspace.
pub fn decode_sign_block_entity_nbt(raw_nbt: &[u8]) -> Result<Option<SignBlockEntityNbt>> {
    let mut decoder = Decoder::new(raw_nbt);
    let root = decode_nbt_summary_from_decoder(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after block entity nbt".to_string(),
        ));
    }
    let Some(NbtSummaryValue::Compound(entries)) = root else {
        return Ok(None);
    };
    let front_text = decode_sign_text_nbt(&entries, "front_text");
    let back_text = decode_sign_text_nbt(&entries, "back_text");
    if front_text.is_none() && back_text.is_none() {
        return Ok(None);
    }
    Ok(Some(SignBlockEntityNbt {
        front_text,
        back_text,
        is_waxed: matches!(find_entry(&entries, "is_waxed"), Some(NbtSummaryValue::Byte(value)) if *value != 0),
    }))
}

fn decode_sign_text_nbt(entries: &[NbtSummaryEntry], key: &str) -> Option<SignTextNbt> {
    let NbtSummaryValue::Compound(text_entries) = find_entry(entries, key)? else {
        return None;
    };
    let messages = decode_sign_lines(find_entry(text_entries, "messages")?)?;
    // Vanilla wraps `filtered_messages` in `ExtraCodecs.lenientOptionalFieldOf`:
    // absence or any codec failure is the same as no filtered value, after
    // which `SignText` copies the raw messages for filtered reads.
    let filtered_messages =
        find_entry(text_entries, "filtered_messages").and_then(decode_sign_lines);
    let color = match find_entry(text_entries, "color") {
        Some(NbtSummaryValue::String(color)) => color.try_to_string().ok(),
        _ => None,
    };
    Some(SignTextNbt {
        messages,
        filtered_messages,
        color,
        has_glowing_text: matches!(
            find_entry(text_entries, "has_glowing_text"),
            Some(NbtSummaryValue::Byte(value)) if *value != 0
        ),
    })
}

fn decode_sign_lines(value: &NbtSummaryValue) -> Option<[Vec<StyledTextRun>; 4]> {
    let NbtSummaryValue::List(lines) = value else {
        return None;
    };
    // Vanilla `SignText.LINES_CODEC` rejects any list that is not exactly four
    // components (`Util.fixedSize(_, 4)`).
    if lines.len() != 4 {
        return None;
    }
    Some(std::array::from_fn(|index| {
        let mut runs = RunCollector::default();
        append_component_runs(&lines[index], &ComponentStyle::default(), &mut runs);
        runs.runs
    }))
}
