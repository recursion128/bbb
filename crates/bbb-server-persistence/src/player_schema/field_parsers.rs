//! Lenient readers for `playerdata` fields: a missing or mistyped value
//! takes the Vanilla default (`CompoundTag.getXOr`, codec `orElse`).

use super::{
    Java26PlayerAbilities, Java26PlayerRecipeBook, Java26PlayerRespawn, Java26WardenSpawnTracker,
    DEFAULT_LOCATOR_STYLE, RECIPE_BOOK_SETTING_NAMES,
};
pub use bbb_protocol::nbt_summary::{
    codec_double_value as number_f64, codec_int_value as number_i32,
};
use bbb_protocol::{
    packets::{BlockPos, GlobalPos, WaypointIcon},
    NbtSummaryEntry, NbtSummaryValue,
};
use std::collections::HashSet;

pub use crate::schema_support::{field, int_or};

pub fn number_f32(value: &NbtSummaryValue) -> Option<f32> {
    match *value {
        NbtSummaryValue::Float(bits) => Some(f32::from_bits(bits)),
        _ => number_f64(value).map(|value| value as f32),
    }
}

pub fn short_or(entries: &[NbtSummaryEntry], name: &str, default: i16) -> i16 {
    field(entries, name)
        .and_then(number_i32)
        .map_or(default, |value| value as i16)
}

pub fn float_or(entries: &[NbtSummaryEntry], name: &str, default: f32) -> f32 {
    field(entries, name).and_then(number_f32).unwrap_or(default)
}

pub fn double_or(entries: &[NbtSummaryEntry], name: &str, default: f64) -> f64 {
    field(entries, name).and_then(number_f64).unwrap_or(default)
}

pub fn bool_or(entries: &[NbtSummaryEntry], name: &str, default: bool) -> bool {
    field(entries, name)
        .and_then(number_i32)
        .map_or(default, |value| value as i8 != 0)
}

/// `Vec3.CODEC`: exactly three doubles.
pub fn double_vec3(value: Option<&NbtSummaryValue>) -> Option<[f64; 3]> {
    let Some(NbtSummaryValue::List(values)) = value else {
        return None;
    };
    let mut result = [0.0; 3];
    if values.len() != 3 {
        return None;
    }
    for (slot, value) in result.iter_mut().zip(values) {
        let NbtSummaryValue::Double(bits) = value else {
            return None;
        };
        *slot = f64::from_bits(*bits);
    }
    Some(result)
}

pub fn resource_id(value: &NbtSummaryValue) -> Option<String> {
    let NbtSummaryValue::String(raw) = value else {
        return None;
    };
    raw.try_to_string()
        .ok()
        .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())
}

pub fn int_vec3(value: Option<&NbtSummaryValue>) -> Option<[i32; 3]> {
    match value {
        Some(NbtSummaryValue::IntArray(values)) => <[i32; 3]>::try_from(values.as_slice()).ok(),
        _ => None,
    }
}

/// `Waypoint.Icon.CODEC` (`Waypoint.java:36-42`, `ExtraCodecs.java:139`); a
/// failed read is the default icon.
pub fn parse_locator_bar_icon(value: Option<&NbtSummaryValue>) -> WaypointIcon {
    let default = || WaypointIcon {
        style: DEFAULT_LOCATOR_STYLE.to_string(),
        color_rgb: None,
    };
    let Some(NbtSummaryValue::Compound(entries)) = value else {
        return default();
    };
    let Some(style) = field(entries, "style").and_then(resource_id) else {
        return default();
    };
    let color_rgb = match field(entries, "color") {
        None => None,
        Some(raw) => match parse_rgb_color_codec(raw) {
            Some(color) => Some(color),
            None => return default(),
        },
    };
    WaypointIcon { style, color_rgb }
}

/// `ExtraCodecs.RGB_COLOR_CODEC` (`ExtraCodecs.java:139`, `ARGB.java:68-70,209-211,225-227`):
/// accepts either a packed `TAG_Int` or a 3-float `VECTOR3F` list and returns the
/// 24-bit RGB channels carried by `ByteBufCodecs.RGB_COLOR`.
fn parse_rgb_color_codec(value: &NbtSummaryValue) -> Option<u32> {
    match value {
        NbtSummaryValue::Int(color) => Some((*color as u32) & 0x00ff_ffff),
        NbtSummaryValue::List(values) if values.len() == 3 => {
            let mut channels = [0_u32; 3];
            for (slot, entry) in channels.iter_mut().zip(values) {
                let channel = match entry {
                    NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
                    NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
                    NbtSummaryValue::Byte(v) => f32::from(*v),
                    NbtSummaryValue::Short(v) => f32::from(*v),
                    NbtSummaryValue::Int(v) => *v as f32,
                    NbtSummaryValue::Long(v) => *v as f32,
                    _ => return None,
                };
                if !channel.is_finite() {
                    return None;
                }
                *slot = ((channel * 255.0_f32).floor() as i32 as u32) & 0xff;
            }
            Some((channels[0] << 16) | (channels[1] << 8) | channels[2])
        }
        _ => None,
    }
}

/// `Abilities.Packed.CODEC` (`Abilities.java:53-64`): every field `orElse`.
pub fn parse_abilities(value: Option<&NbtSummaryValue>) -> Java26PlayerAbilities {
    let entries = match value {
        Some(NbtSummaryValue::Compound(entries)) => entries.as_slice(),
        _ => &[],
    };
    Java26PlayerAbilities {
        invulnerable: bool_or(entries, "invulnerable", false),
        flying: bool_or(entries, "flying", false),
        may_fly: bool_or(entries, "mayfly", false),
        instant_build: bool_or(entries, "instabuild", false),
        may_build: bool_or(entries, "mayBuild", true),
        fly_speed: float_or(entries, "flySpeed", 0.05),
        walk_speed: float_or(entries, "walkSpeed", 0.1),
    }
}

/// `ServerRecipeBook.Packed.CODEC` (`ServerRecipeBook.java:150-158`). Invalid
/// ids are skipped, and a highlight that is not a known recipe is dropped so
/// `ServerRecipeBook.apply` can take the rest.
pub fn parse_recipe_book(value: &NbtSummaryValue) -> Java26PlayerRecipeBook {
    let NbtSummaryValue::Compound(entries) = value else {
        return Java26PlayerRecipeBook::default();
    };
    let ids = |name: &str| -> Vec<String> {
        let mut seen = HashSet::new();
        match field(entries, name) {
            Some(NbtSummaryValue::List(values)) => values
                .iter()
                .filter_map(resource_id)
                .filter(|id| seen.insert(id.clone()))
                .collect(),
            _ => Vec::new(),
        }
    };
    let recipes = ids("recipes");
    let known: HashSet<&str> = recipes.iter().map(String::as_str).collect();
    let to_be_displayed = ids("toBeDisplayed")
        .into_iter()
        .filter(|id| known.contains(id.as_str()))
        .collect();
    let mut settings = [false; 8];
    for (setting, name) in settings.iter_mut().zip(RECIPE_BOOK_SETTING_NAMES) {
        *setting = bool_or(entries, name, false);
    }
    Java26PlayerRecipeBook {
        recipes,
        to_be_displayed,
        settings,
    }
}

/// `WardenSpawnTracker.CODEC` (`WardenSpawnTracker.java:18-25`).
pub fn parse_warden(value: Option<&NbtSummaryValue>) -> Java26WardenSpawnTracker {
    let entries = match value {
        Some(NbtSummaryValue::Compound(entries)) => entries.as_slice(),
        _ => &[],
    };
    Java26WardenSpawnTracker::new(
        int_or(entries, "ticks_since_last_warning", 0),
        int_or(entries, "warning_level", 0),
        int_or(entries, "cooldown_ticks", 0),
    )
}

/// `ServerPlayer.RespawnConfig.CODEC` (`ServerPlayer.java:2270-2277`); any
/// field failure, including an out-of-range angle, drops the whole config.
/// `GlobalPos.MAP_CODEC` (`GlobalPos.java:13-16`).
pub fn parse_global_pos(value: &NbtSummaryValue) -> Option<GlobalPos> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let dimension = field(entries, "dimension").and_then(resource_id)?;
    let [x, y, z] = int_vec3(field(entries, "pos"))?;
    Some(GlobalPos {
        dimension,
        pos: BlockPos { x, y, z },
    })
}

pub fn parse_respawn(value: &NbtSummaryValue) -> Option<Java26PlayerRespawn> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let dimension = field(entries, "dimension").and_then(resource_id)?;
    let [x, y, z] = int_vec3(field(entries, "pos"))?;
    let yaw = field(entries, "yaw").and_then(number_f32)?;
    let pitch = field(entries, "pitch").and_then(number_f32)?;
    if !(-180.0..=180.0).contains(&yaw) || !(-90.0..=90.0).contains(&pitch) {
        return None;
    }
    Some(Java26PlayerRespawn {
        global_pos: GlobalPos {
            dimension,
            pos: BlockPos { x, y, z },
        },
        yaw,
        pitch,
        forced: bool_or(entries, "forced", false),
    })
}
