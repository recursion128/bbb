use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::environment_attributes::{
    decode_client_environment_attribute_map, parse_rgb, ClientEnvironmentAttributeMap,
    ClientEnvironmentAttributes,
};
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
    pub(crate) environment_attribute_modifiers: ClientEnvironmentAttributeMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiomeProjectionError {
    pub field: String,
    pub reason: &'static str,
}

pub(super) fn decode_biome_client_projection(
    entry: &RegistryPacketEntry,
) -> Result<Option<BiomeClientProjection>, BiomeProjectionError> {
    let Some(data) = entry.element_data() else {
        return Ok(None);
    };
    decode_biome_client_projection_value(&data.value).map(Some)
}

fn decode_biome_client_projection_value(
    value: &NbtSummaryValue,
) -> Result<BiomeClientProjection, BiomeProjectionError> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(error("<root>", "expected compound"));
    };
    let has_precipitation = required_bool(fields, "has_precipitation")?;
    let temperature = required_finite_float(fields, "temperature")?;
    let downfall = required_finite_float(fields, "downfall")?;
    let temperature_modifier = match optional_string(fields, "temperature_modifier")? {
        None | Some("none") => BiomeTemperatureModifier::None,
        Some("frozen") => BiomeTemperatureModifier::Frozen,
        Some(_) => return Err(error("temperature_modifier", "unknown enum value")),
    };
    let effects = required_compound(fields, "effects")?;
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
    let environment_attribute_modifiers = optional_compound(fields, "attributes")?
        .map(|entries| {
            decode_client_environment_attribute_map(entries, true)
                .map_err(|err| error(format!("attributes.{}", err.field), err.reason))
        })
        .transpose()?
        .unwrap_or_default();
    let mut attributes = ClientEnvironmentAttributes::default();
    environment_attribute_modifiers
        .apply_to(&mut attributes)
        .map_err(|err| error(format!("attributes.{}", err.field), err.reason))?;

    Ok(BiomeClientProjection {
        has_precipitation,
        temperature,
        temperature_modifier,
        downfall,
        water_color,
        foliage_color,
        dry_foliage_color,
        grass_color,
        grass_color_modifier,
        environment_attribute_modifiers,
    })
}

fn required_bool(entries: &[NbtSummaryEntry], field: &str) -> Result<bool, BiomeProjectionError> {
    match field_value(entries, field) {
        Some(value) => numeric_bool(value).ok_or_else(|| error(field, "expected numeric boolean")),
        None => Err(error(field, "missing required field")),
    }
}

fn required_finite_float(
    entries: &[NbtSummaryEntry],
    field: &str,
) -> Result<f32, BiomeProjectionError> {
    match field_value(entries, field) {
        Some(value) => match numeric_f32(value) {
            Some(value) if value.is_finite() => Ok(value),
            Some(_) => Err(error(field, "number must be finite")),
            None => Err(error(field, "expected number")),
        },
        None => Err(error(field, "missing required field")),
    }
}

fn required_compound<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
) -> Result<&'a [NbtSummaryEntry], BiomeProjectionError> {
    optional_compound(entries, field)?.ok_or_else(|| error(field, "missing required field"))
}

fn optional_compound<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
) -> Result<Option<&'a [NbtSummaryEntry]>, BiomeProjectionError> {
    match field_value(entries, field) {
        Some(NbtSummaryValue::Compound(value)) => Ok(Some(value)),
        Some(_) => Err(error(field, "expected compound")),
        None => Ok(None),
    }
}

fn optional_string<'a>(
    entries: &'a [NbtSummaryEntry],
    field: &str,
) -> Result<Option<&'a str>, BiomeProjectionError> {
    match field_value(entries, field) {
        Some(NbtSummaryValue::String(value)) => Ok(Some(value)),
        Some(_) => Err(error(field, "expected string")),
        None => Ok(None),
    }
}

fn required_rgb(entries: &[NbtSummaryEntry], field: &str) -> Result<[u8; 3], BiomeProjectionError> {
    optional_rgb(entries, field)?.ok_or_else(|| error(field, "missing required field"))
}

fn optional_rgb(
    entries: &[NbtSummaryEntry],
    field: &str,
) -> Result<Option<[u8; 3]>, BiomeProjectionError> {
    match field_value(entries, field) {
        Some(value) => parse_rgb(value)
            .map(Some)
            .ok_or_else(|| error(field, "expected #RRGGBB string or RGB int")),
        None => Ok(None),
    }
}

fn field_value<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn numeric_bool(value: &NbtSummaryValue) -> Option<bool> {
    Some(match value {
        NbtSummaryValue::Byte(value) => *value != 0,
        NbtSummaryValue::Short(value) => *value != 0,
        NbtSummaryValue::Int(value) => *value != 0,
        NbtSummaryValue::Long(value) => *value != 0,
        _ => return None,
    })
}

fn numeric_f32(value: &NbtSummaryValue) -> Option<f32> {
    Some(match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return None,
    })
}

fn error(field: impl Into<String>, reason: &'static str) -> BiomeProjectionError {
    BiomeProjectionError {
        field: field.into(),
        reason,
    }
}

/// Minimal network-NBT biome element for precipitation-focused fixtures:
/// exactly the required projection fields plus the climate knobs under test.
#[cfg(test)]
pub(crate) fn test_biome_element_nbt(temperature: f32, has_precipitation: bool) -> Vec<u8> {
    fn write_utf(out: &mut Vec<u8>, value: &str) {
        out.extend(
            u16::try_from(value.len())
                .expect("short name")
                .to_be_bytes(),
        );
        out.extend(value.as_bytes());
    }
    let mut out = vec![10];
    out.push(1);
    write_utf(&mut out, "has_precipitation");
    out.push(u8::from(has_precipitation));
    out.push(5);
    write_utf(&mut out, "temperature");
    out.extend(temperature.to_be_bytes());
    out.push(5);
    write_utf(&mut out, "downfall");
    out.extend(0.4_f32.to_be_bytes());
    out.push(10);
    write_utf(&mut out, "effects");
    out.push(3);
    write_utf(&mut out, "water_color");
    out.extend(0x3f_76_e4_i32.to_be_bytes());
    out.push(0);
    out.push(0);
    out
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
        let mut attributes = ClientEnvironmentAttributes::default();
        projection
            .environment_attribute_modifiers
            .apply_to(&mut attributes)
            .unwrap();
        assert_eq!(attributes.fog_color, Some([0x44, 0x66, 0x88]));
        assert_eq!(attributes.sky_color, Some([0x55, 0x77, 0x99]));
        assert_eq!(attributes.water_fog_color, Some([0x66, 0x88, 0xaa]));
        assert_eq!(attributes.water_fog_end_distance, Some(48.0));
    }

    #[test]
    fn decodes_known_pack_json_through_the_same_biome_projection() {
        let data = super::super::element_value::decode_known_pack_element(
            br##"{
                "has_precipitation": true,
                "temperature": 0.35,
                "downfall": 0.8,
                "effects": {"water_color": "#112233"},
                "attributes": {
                    "minecraft:visual/fog_color": "#446688",
                    "minecraft:visual/water_fog_end_distance": {
                        "argument": 0.85,
                        "modifier": "multiply"
                    }
                }
            }"##,
        )
        .unwrap();

        let projection = decode_biome_client_projection_value(&data.value).unwrap();
        assert!(projection.has_precipitation);
        assert_eq!(projection.water_color, [0x11, 0x22, 0x33]);
        let mut dimension_attributes = ClientEnvironmentAttributes {
            water_fog_end_distance: Some(200.0),
            ..ClientEnvironmentAttributes::default()
        };
        projection
            .environment_attribute_modifiers
            .apply_to(&mut dimension_attributes)
            .unwrap();
        assert!((dimension_attributes.water_fog_end_distance.unwrap() - 170.0).abs() < 0.0001);
    }

    #[test]
    fn decodes_relevant_attribute_modifier_through_the_shared_codec() {
        let entry = RegistryPacketEntry::with_raw_data("example:custom", biome_nbt(true));
        let projection = decode_biome_client_projection(&entry).unwrap().unwrap();
        let mut attributes = ClientEnvironmentAttributes::default();
        projection
            .environment_attribute_modifiers
            .apply_to(&mut attributes)
            .unwrap();
        assert!((attributes.water_fog_end_distance.unwrap() - 81.6).abs() < 0.0001);
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
            write_string(&mut out, "modifier", "multiply");
            write_float(&mut out, "argument", 0.85);
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
