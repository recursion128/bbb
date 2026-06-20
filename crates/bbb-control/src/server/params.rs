use crate::types::{
    CreativeModeItemStackControl, CreativeModeSlotControlRequest, NetControlRequest,
    RecipeBookTypeControl,
};

const SIGN_UPDATE_LINE_COUNT: usize = 4;
const SIGN_UPDATE_MAX_LINE_CHARS: usize = 384;
pub(super) const RENAME_ITEM_MAX_NAME_CHARS: usize = 32767;
pub(super) const EDIT_BOOK_MAX_PAGES: usize = 100;
pub(super) const EDIT_BOOK_MAX_PAGE_CHARS: usize = 1024;
pub(super) const EDIT_BOOK_MAX_TITLE_CHARS: usize = 32;

pub(super) fn i32_param(params: &serde_json::Value, key: &str) -> Option<i32> {
    params.get(key)?.as_i64()?.try_into().ok()
}

pub(super) fn optional_i32_param(
    params: &serde_json::Value,
    key: &str,
) -> Result<Option<i32>, String> {
    let Some(value) = params.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(value) = value.as_i64() else {
        return Err(format!(
            "net.set_beacon requires integer or null param {key}"
        ));
    };
    value
        .try_into()
        .map(Some)
        .map_err(|_| format!("net.set_beacon requires integer or null param {key}"))
}

pub(super) fn f32_param(params: &serde_json::Value, key: &str) -> Option<f32> {
    params
        .get(key)?
        .as_f64()
        .filter(|value| value.is_finite())
        .map(|value| value as f32)
}

pub(super) fn string_param<'a>(params: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    params.get(key)?.as_str()
}

pub(super) fn non_empty_string_param<'a>(
    params: &'a serde_json::Value,
    key: &str,
) -> Option<&'a str> {
    let value = string_param(params, key)?;
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub(super) fn is_resource_location(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    let mut parts = value.split(':');
    let first = parts.next().expect("split yields at least one part");
    let second = parts.next();
    if parts.next().is_some() {
        return false;
    }

    match second {
        Some(path) => is_resource_namespace(first) && is_resource_path(path),
        None => is_resource_path(first),
    }
}

fn is_resource_namespace(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
}

fn is_resource_path(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b'/')
        })
}

pub(super) fn recipe_book_type_param(
    params: &serde_json::Value,
    key: &str,
) -> Option<RecipeBookTypeControl> {
    match string_param(params, key)? {
        "crafting" => Some(RecipeBookTypeControl::Crafting),
        "furnace" => Some(RecipeBookTypeControl::Furnace),
        "blast_furnace" => Some(RecipeBookTypeControl::BlastFurnace),
        "smoker" => Some(RecipeBookTypeControl::Smoker),
        _ => None,
    }
}

pub(super) fn change_difficulty_request_param(
    params: &serde_json::Value,
) -> Result<NetControlRequest, String> {
    let Some(value) = string_param(params, "difficulty") else {
        return Err(format!(
            "net.change_difficulty requires string param difficulty: peaceful, easy, normal, or hard"
        ));
    };
    NetControlRequest::change_difficulty_named(value).ok_or_else(|| {
        format!("net.change_difficulty requires difficulty peaceful, easy, normal, or hard")
    })
}

pub(super) fn change_game_mode_request_param(
    params: &serde_json::Value,
) -> Result<NetControlRequest, String> {
    let Some(value) = params.get("game_mode") else {
        return Err("net.change_game_mode requires string param game_mode".to_string());
    };
    let Some(value) = value.as_str() else {
        return Err("net.change_game_mode param game_mode must be a string".to_string());
    };
    NetControlRequest::change_game_mode_named(value).ok_or_else(|| {
        "net.change_game_mode requires game_mode survival, creative, adventure, or spectator"
            .to_string()
    })
}

pub(super) fn sign_lines_param(
    params: &serde_json::Value,
    key: &str,
) -> Result<[String; SIGN_UPDATE_LINE_COUNT], String> {
    let lines = params
        .get(key)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "net.update_sign requires array param lines".to_string())?;
    if lines.len() != SIGN_UPDATE_LINE_COUNT {
        return Err(format!(
            "net.update_sign requires exactly {SIGN_UPDATE_LINE_COUNT} lines"
        ));
    }

    let mut parsed = Vec::with_capacity(SIGN_UPDATE_LINE_COUNT);
    for (index, line) in lines.iter().enumerate() {
        let line = line
            .as_str()
            .ok_or_else(|| format!("net.update_sign line {index} must be a string"))?;
        if line.chars().count() > SIGN_UPDATE_MAX_LINE_CHARS {
            return Err(format!(
                "net.update_sign line {index} exceeds {SIGN_UPDATE_MAX_LINE_CHARS} characters"
            ));
        }
        parsed.push(line.to_string());
    }

    parsed
        .try_into()
        .map_err(|_| "net.update_sign requires exactly 4 lines".to_string())
}

pub(super) fn edit_book_pages_param(
    params: &serde_json::Value,
    key: &str,
) -> Result<Vec<String>, String> {
    let pages = params
        .get(key)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "net.edit_book requires array param pages".to_string())?;
    if pages.len() > EDIT_BOOK_MAX_PAGES {
        return Err(format!(
            "net.edit_book requires at most {EDIT_BOOK_MAX_PAGES} pages"
        ));
    }

    let mut parsed = Vec::with_capacity(pages.len());
    for (index, page) in pages.iter().enumerate() {
        let page = page
            .as_str()
            .ok_or_else(|| format!("net.edit_book page {index} must be a string"))?;
        if page.chars().count() > EDIT_BOOK_MAX_PAGE_CHARS {
            return Err(format!(
                "net.edit_book page {index} exceeds {EDIT_BOOK_MAX_PAGE_CHARS} characters"
            ));
        }
        parsed.push(page.to_string());
    }

    Ok(parsed)
}

pub(super) fn edit_book_title_param(
    params: &serde_json::Value,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(title) = params.get(key) else {
        return Ok(None);
    };
    if title.is_null() {
        return Ok(None);
    }
    let title = title
        .as_str()
        .ok_or_else(|| "net.edit_book title must be a string or null".to_string())?;
    if title.chars().count() > EDIT_BOOK_MAX_TITLE_CHARS {
        return Err(format!(
            "net.edit_book title exceeds {EDIT_BOOK_MAX_TITLE_CHARS} characters"
        ));
    }
    Ok(Some(title.to_string()))
}

pub(super) fn bool_param(params: &serde_json::Value, key: &str) -> Option<bool> {
    params.get(key)?.as_bool()
}

pub(super) fn validate_creative_mode_slot_request(
    request: &CreativeModeSlotControlRequest,
) -> Result<(), String> {
    match &request.item {
        CreativeModeItemStackControl::Empty => Ok(()),
        CreativeModeItemStackControl::Item { item_id, count } => {
            if *item_id < 0 {
                return Err("net.set_creative_mode_slot requires item.item_id >= 0".to_string());
            }
            if *count <= 0 {
                return Err("net.set_creative_mode_slot requires item.count > 0".to_string());
            }
            Ok(())
        }
    }
}
