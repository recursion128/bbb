use crate::chunks::{decode_nbt_root, find_nbt_entry, NbtValue};

use super::environment_attributes::{decode_client_environment_attributes, parse_rgb};
use super::RegistryPacketEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeTemperatureModifier {
    None,
    Frozen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeGrassColorModifier {
    None,
    DarkForest,
    Swamp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BiomeClientProjection {
    pub has_precipitation: bool,
    pub temperature: f32,
    pub temperature_modifier: BiomeTemperatureModifier,
    pub downfall: f32,
    pub water_color: [u8; 3],
    pub foliage_color: Option<[u8; 3]>,
    pub dry_foliage_color: Option<[u8; 3]>,
    pub grass_color: Option<[u8; 3]>,
    pub grass_color_modifier: BiomeGrassColorModifier,
    pub fog_color: Option<[u8; 3]>,
    pub sky_color: Option<[u8; 3]>,
    pub water_fog_color: Option<[u8; 3]>,
    pub water_fog_end_distance: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiomeProjectionError {
    pub field: String,
    pub reason: &'static str,
}

pub(super) fn decode_biome_client_projection(
    entry: &RegistryPacketEntry,
) -> Result<Option<BiomeClientProjection>, BiomeProjectionError> {
    let Some(raw) = entry.raw_data() else {
        return Ok(None);
    };
    let root = decode_nbt_root(raw)
        .map_err(|_| error("<root>", "invalid NBT"))?
        .ok_or_else(|| error("<root>", "missing root compound"))?;
    let NbtValue::Compound(fields) = root else {
        return Err(error("<root>", "expected compound"));
    };
    let has_precipitation = required_bool(&fields, "has_precipitation")?;
    let temperature = required_finite_float(&fields, "temperature")?;
    let downfall = required_finite_float(&fields, "downfall")?;
    let temperature_modifier = match optional_string(&fields, "temperature_modifier")? {
        None | Some("none") => BiomeTemperatureModifier::None,
        Some("frozen") => BiomeTemperatureModifier::Frozen,
        Some(_) => return Err(error("temperature_modifier", "unknown enum value")),
    };
    let effects = required_compound(&fields, "effects")?;
    let water_color = required_rgb(effects, "water_color")?;
    let foliage_color = optional_rgb(effects, "foliage_color")?;
    let dry_foliage_color = optional_rgb(effects, "dry_foliage_color")?;
    let grass_color = optional_rgb(effects, "grass_color")?;
    let grass_color_modifier = match optional_string(effects, "grass_color_modifier")? {
        None | Some("none") => BiomeGrassColorModifier::None,
        Some("dark_forest") => BiomeGrassColorModifier::DarkForest,
        Some("swamp") => BiomeGrassColorModifier::Swamp,
        Some(_) => return Err(error("effects.grass_color_modifier", "unknown enum value")),
    };
    let attributes = optional_compound(&fields, "attributes")?
        .map(decode_client_environment_attributes)
        .transpose()
        .map_err(|err| error(format!("attributes.{}", err.field), err.reason))?
        .unwrap_or_default();

    Ok(Some(BiomeClientProjection {
        has_precipitation,
        temperature,
        temperature_modifier,
        downfall,
        water_color,
        foliage_color,
        dry_foliage_color,
        grass_color,
        grass_color_modifier,
        fog_color: attributes.fog_color,
        sky_color: attributes.sky_color,
        water_fog_color: attributes.water_fog_color,
        water_fog_end_distance: attributes.water_fog_end_distance,
    }))
}

fn required_bool(
    entries: &[(String, NbtValue)],
    field: &str,
) -> Result<bool, BiomeProjectionError> {
    match find_nbt_entry(entries, field) {
        Some(NbtValue::Byte(value)) => Ok(*value != 0),
        Some(_) => Err(error(field, "expected byte boolean")),
        None => Err(error(field, "missing required field")),
    }
}

fn required_finite_float(
    entries: &[(String, NbtValue)],
    field: &str,
) -> Result<f32, BiomeProjectionError> {
    match find_nbt_entry(entries, field) {
        Some(NbtValue::Float(value)) if value.is_finite() => Ok(*value),
        Some(NbtValue::Float(_)) => Err(error(field, "float must be finite")),
        Some(_) => Err(error(field, "expected float")),
        None => Err(error(field, "missing required field")),
    }
}

fn required_compound<'a>(
    entries: &'a [(String, NbtValue)],
    field: &str,
) -> Result<&'a [(String, NbtValue)], BiomeProjectionError> {
    optional_compound(entries, field)?.ok_or_else(|| error(field, "missing required field"))
}

fn optional_compound<'a>(
    entries: &'a [(String, NbtValue)],
    field: &str,
) -> Result<Option<&'a [(String, NbtValue)]>, BiomeProjectionError> {
    match find_nbt_entry(entries, field) {
        Some(NbtValue::Compound(value)) => Ok(Some(value)),
        Some(_) => Err(error(field, "expected compound")),
        None => Ok(None),
    }
}

fn optional_string<'a>(
    entries: &'a [(String, NbtValue)],
    field: &str,
) -> Result<Option<&'a str>, BiomeProjectionError> {
    match find_nbt_entry(entries, field) {
        Some(NbtValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(error(field, "expected string")),
        None => Ok(None),
    }
}

fn required_rgb(
    entries: &[(String, NbtValue)],
    field: &str,
) -> Result<[u8; 3], BiomeProjectionError> {
    optional_rgb(entries, field)?.ok_or_else(|| error(field, "missing required field"))
}

fn optional_rgb(
    entries: &[(String, NbtValue)],
    field: &str,
) -> Result<Option<[u8; 3]>, BiomeProjectionError> {
    match find_nbt_entry(entries, field) {
        Some(value) => parse_rgb(value)
            .map(Some)
            .ok_or_else(|| error(field, "expected #RRGGBB string or RGB int")),
        None => Ok(None),
    }
}

fn error(field: impl Into<String>, reason: &'static str) -> BiomeProjectionError {
    BiomeProjectionError {
        field: field.into(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_network_biome_client_projection() {
        let entry = RegistryPacketEntry::with_raw_data("example:custom", biome_nbt(false));
        let projection = decode_biome_client_projection(&entry).unwrap().unwrap();

        assert!(projection.has_precipitation);
        assert_eq!(projection.temperature, 0.35);
        assert_eq!(
            projection.temperature_modifier,
            BiomeTemperatureModifier::Frozen
        );
        assert_eq!(projection.downfall, 0.8);
        assert_eq!(projection.water_color, [0x11, 0x22, 0x33]);
        assert_eq!(projection.foliage_color, Some([0x22, 0x44, 0x66]));
        assert_eq!(projection.grass_color, Some([0x33, 0x55, 0x77]));
        assert_eq!(
            projection.grass_color_modifier,
            BiomeGrassColorModifier::Swamp
        );
        assert_eq!(projection.fog_color, Some([0x44, 0x66, 0x88]));
        assert_eq!(projection.sky_color, Some([0x55, 0x77, 0x99]));
        assert_eq!(projection.water_fog_color, Some([0x66, 0x88, 0xaa]));
        assert_eq!(projection.water_fog_end_distance, Some(48.0));
    }

    #[test]
    fn rejects_relevant_attribute_modifier_without_guessing() {
        let entry = RegistryPacketEntry::with_raw_data("example:custom", biome_nbt(true));
        let error = decode_biome_client_projection(&entry).unwrap_err();
        assert_eq!(
            error,
            BiomeProjectionError {
                field: "attributes.minecraft:visual/water_fog_end_distance".to_string(),
                reason: "attribute modifiers are not yet supported",
            }
        );
        assert_eq!(
            decode_biome_client_projection(&RegistryPacketEntry::stub("example:stub")).unwrap(),
            None
        );
    }

    fn biome_nbt(modified_distance: bool) -> Vec<u8> {
        let mut out = vec![10];
        write_byte(&mut out, "has_precipitation", 1);
        write_float(&mut out, "temperature", 0.35);
        write_string(&mut out, "temperature_modifier", "frozen");
        write_float(&mut out, "downfall", 0.8);
        begin_compound(&mut out, "effects");
        write_string(&mut out, "water_color", "#112233");
        write_int(&mut out, "foliage_color", 0x224466);
        write_string(&mut out, "grass_color", "#335577");
        write_string(&mut out, "grass_color_modifier", "swamp");
        out.push(0);
        begin_compound(&mut out, "attributes");
        write_string(&mut out, "minecraft:visual/fog_color", "#446688");
        write_string(&mut out, "minecraft:visual/sky_color", "#557799");
        write_int(&mut out, "minecraft:visual/water_fog_color", 0x6688aa);
        if modified_distance {
            begin_compound(&mut out, "minecraft:visual/water_fog_end_distance");
            write_string(&mut out, "modifier", "override");
            write_float(&mut out, "argument", 48.0);
            out.push(0);
        } else {
            write_float(&mut out, "minecraft:visual/water_fog_end_distance", 48.0);
        }
        out.push(0);
        out.push(0);
        out
    }

    fn begin_compound(out: &mut Vec<u8>, name: &str) {
        out.push(10);
        write_utf(out, name);
    }

    fn write_byte(out: &mut Vec<u8>, name: &str, value: i8) {
        out.push(1);
        write_utf(out, name);
        out.push(value as u8);
    }

    fn write_int(out: &mut Vec<u8>, name: &str, value: i32) {
        out.push(3);
        write_utf(out, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn write_float(out: &mut Vec<u8>, name: &str, value: f32) {
        out.push(5);
        write_utf(out, name);
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }

    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_utf(out, name);
        write_utf(out, value);
    }

    fn write_utf(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
