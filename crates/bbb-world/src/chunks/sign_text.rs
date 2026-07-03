use crate::Result;

use super::nbt::{decode_nbt_root, find_entry, NbtValue};
use super::state::SignBlockEntityTextState;

pub(crate) fn decode_sign_block_entity_text(
    raw_nbt: &[u8],
) -> Result<Option<SignBlockEntityTextState>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };

    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let front = decode_sign_text_side(&entries, "front_text");
    let back = decode_sign_text_side(&entries, "back_text");
    if front.is_none() && back.is_none() {
        return Ok(None);
    }
    Ok(Some(SignBlockEntityTextState {
        front: front.unwrap_or_else(empty_lines),
        back: back.unwrap_or_else(empty_lines),
    }))
}

fn decode_sign_text_side(entries: &[(String, NbtValue)], key: &str) -> Option<[String; 4]> {
    let NbtValue::Compound(text_entries) = find_entry(entries, key)? else {
        return None;
    };
    let NbtValue::List(messages) = find_entry(text_entries, "messages")? else {
        return None;
    };
    if messages.len() != 4 {
        return None;
    }
    Some(std::array::from_fn(|index| {
        component_plain_text(&messages[index])
    }))
}

fn component_plain_text(value: &NbtValue) -> String {
    let mut out = String::new();
    append_component_text(value, &mut out);
    out
}

fn append_component_text(value: &NbtValue, out: &mut String) {
    match value {
        NbtValue::String(text) => out.push_str(text),
        NbtValue::List(items) => {
            for item in items {
                append_component_text(item, out);
            }
        }
        NbtValue::Compound(entries) => {
            append_primary_component_text(entries, out);
            if let Some(extra) = find_entry(entries, "extra") {
                append_component_text(extra, out);
            }
        }
        NbtValue::Double(_) | NbtValue::IntArray(_) | NbtValue::Other => {}
    }
}

fn append_primary_component_text(entries: &[(String, NbtValue)], out: &mut String) {
    if let Some(text) = find_entry(entries, "text") {
        append_component_text(text, out);
        return;
    }

    if let Some(fallback) = find_entry(entries, "fallback") {
        append_component_text(fallback, out);
    } else if let Some(translate) = find_entry(entries, "translate") {
        append_component_text(translate, out);
    } else if let Some(keybind) = find_entry(entries, "keybind") {
        append_component_text(keybind, out);
    } else if let Some(selector) = find_entry(entries, "selector") {
        append_component_text(selector, out);
    } else if let Some(nbt) = find_entry(entries, "nbt") {
        append_component_text(nbt, out);
    }

    if let Some(with) = find_entry(entries, "with") {
        if !out.is_empty() {
            out.push(' ');
        }
        append_component_text(with, out);
    }
}

fn empty_lines() -> [String; 4] {
    std::array::from_fn(|_| String::new())
}
