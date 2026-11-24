use bbb_protocol::nbt_summary::{
    codec_float_value, compound_field as field_value, integer_bool_value,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::environment_attributes::{
    decode_client_environment_attribute_map, parse_rgb, ClientEnvironmentAttributeMap,
    ClientEnvironmentAttributes,
};
use super::state::RegistryTypedValues;
#[cfg(test)]
use super::RegistryPacketEntry;
use crate::FrozenRegistryAccess;

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

impl RegistryTypedValues {
    pub(super) fn insert_biome_projection(
        &mut self,
        resource_id: &str,
        value: &NbtSummaryValue,
        access: FrozenRegistryAccess<'_>,
    ) -> Result<(), String> {
        let projection = decode_biome_client_projection_value(value, access)
            .map_err(|error| format!("{}: {}", error.field, error.reason))?;
        self.biomes.insert(resource_id.to_string(), projection);
        Ok(())
    }
}

#[cfg(test)]
fn decode_biome_client_projection(
    entry: &RegistryPacketEntry,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<Option<BiomeClientProjection>, BiomeProjectionError> {
    let Some(data) = entry.element_data() else {
        return Ok(None);
    };
    let value =
        super::revision::exact_network_value("minecraft:worldgen/biome", data, registry_access)
            .map_err(|_| error("<root>", "invalid biome codec value"))?;
    decode_biome_client_projection_value(&value, registry_access).map(Some)
}

fn decode_biome_client_projection_value(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<BiomeClientProjection, BiomeProjectionError> {
    let NbtSummaryValue::Compound(fields) = value else {
        unreachable!("canonical biome compound");
    };
    let has_precipitation = integer_bool_value(
        field_value(fields, "has_precipitation").expect("canonical biome precipitation"),
    )
    .expect("canonical biome boolean");
    let temperature = required_finite_float(fields, "temperature")?;
    let downfall = required_finite_float(fields, "downfall")?;
    // The canonical codec omits the default `none` modifier.
    let temperature_modifier = if field_value(fields, "temperature_modifier").is_some() {
        BiomeTemperatureModifier::Frozen
    } else {
        BiomeTemperatureModifier::None
    };
    let Some(NbtSummaryValue::Compound(effects)) = field_value(fields, "effects") else {
        unreachable!("canonical biome effects");
    };
    let water_color =
        parse_rgb(field_value(effects, "water_color").expect("canonical biome water color"))
            .expect("canonical biome color");
    let foliage_color = field_value(effects, "foliage_color")
        .map(|value| parse_rgb(value).expect("canonical biome color"));
    let dry_foliage_color = field_value(effects, "dry_foliage_color")
        .map(|value| parse_rgb(value).expect("canonical biome color"));
    let grass_color = field_value(effects, "grass_color")
        .map(|value| parse_rgb(value).expect("canonical biome color"));
    let grass_color_modifier = match field_value(effects, "grass_color_modifier") {
        None => BiomeGrassColorModifier::None,
        Some(NbtSummaryValue::String(value)) if value == "dark_forest" => {
            BiomeGrassColorModifier::DarkForest
        }
        Some(NbtSummaryValue::String(value)) if value == "swamp" => BiomeGrassColorModifier::Swamp,
        _ => unreachable!("canonical biome grass modifier"),
    };
    let environment_attribute_modifiers = match field_value(fields, "attributes") {
        Some(NbtSummaryValue::Compound(entries)) => {
            decode_client_environment_attribute_map(entries, true, registry_access)
                .map_err(|err| error(format!("attributes.{}", err.field), err.reason))?
        }
        None => ClientEnvironmentAttributeMap::default(),
        _ => unreachable!("canonical biome attributes"),
    };
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

fn required_finite_float(
    entries: &[NbtSummaryEntry],
    field: &str,
) -> Result<f32, BiomeProjectionError> {
    let value = field_value(entries, field)
        .and_then(codec_float_value)
        .expect("canonical biome float");
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error(field, "number must be finite"))
    }
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
    use crate::RegistrySet;

    #[test]
    fn decodes_network_biome_client_projection() {
        let registries = RegistrySet::vanilla_26_1();
        let entry = RegistryPacketEntry::with_raw_data("example:custom", biome_nbt(false));
        let projection = decode_biome_client_projection(&entry, registries.frozen_access())
            .unwrap()
            .unwrap();

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
        let registries = RegistrySet::vanilla_26_1();
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

        let value = super::super::revision::exact_network_value(
            "minecraft:worldgen/biome",
            &data,
            registries.frozen_access(),
        )
        .unwrap();
        let projection =
            decode_biome_client_projection_value(&value, registries.frozen_access()).unwrap();
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
        let registries = RegistrySet::vanilla_26_1();
        let entry = RegistryPacketEntry::with_raw_data("example:custom", biome_nbt(true));
        let projection = decode_biome_client_projection(&entry, registries.frozen_access())
            .unwrap()
            .unwrap();
        let mut attributes = ClientEnvironmentAttributes::default();
        projection
            .environment_attribute_modifiers
            .apply_to(&mut attributes)
            .unwrap();
        assert!((attributes.water_fog_end_distance.unwrap() - 81.6).abs() < 0.0001);
        assert_eq!(
            decode_biome_client_projection(
                &RegistryPacketEntry::stub("example:stub"),
                registries.frozen_access()
            )
            .unwrap(),
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
