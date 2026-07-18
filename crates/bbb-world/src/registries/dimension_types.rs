use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::{RegistryPacketEntry, WorldCardinalLighting, WorldDimension, WorldSkybox};

use super::{
    decode_client_environment_attributes, ClientEnvironmentAttributes,
    EnvironmentAttributeProjectionError,
};

pub const DIMENSION_TYPE_REGISTRY: &str = "minecraft:dimension_type";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DimensionTypeClientProjection {
    pub(crate) dimension: Option<WorldDimension>,
    pub(crate) has_sky_light: Option<bool>,
    pub(crate) has_ceiling: Option<bool>,
    pub(crate) cardinal_lighting: Option<WorldCardinalLighting>,
    pub(crate) skybox: Option<WorldSkybox>,
    pub(crate) ambient_light: Option<f32>,
    pub(crate) default_clock_resource_id: Option<String>,
    pub(crate) environment_attributes: ClientEnvironmentAttributes,
    pub(crate) environment_error: Option<EnvironmentAttributeProjectionError>,
}

pub(crate) fn dimension_type_client_projection_from_registry(
    entry: Option<&RegistryPacketEntry>,
) -> Option<DimensionTypeClientProjection> {
    dimension_type_client_projection(&entry?.element_data()?.value)
}

fn dimension_type_client_projection(
    value: &NbtSummaryValue,
) -> Option<DimensionTypeClientProjection> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let dimension = dimension_type_shape_from_fields(fields);
    let has_sky_light = bool_field(fields, "has_skylight");
    let has_ceiling = bool_field(fields, "has_ceiling");
    let cardinal_lighting = match field(fields, "cardinal_light") {
        Some(NbtSummaryValue::String(name)) => cardinal_lighting_type_from_name(name),
        None => Some(WorldCardinalLighting::Default),
        _ => None,
    };
    let skybox = match field(fields, "skybox") {
        Some(NbtSummaryValue::String(name)) => skybox_from_name(name),
        None => Some(WorldSkybox::Overworld),
        _ => None,
    };
    let ambient_light = field(fields, "ambient_light").and_then(numeric_f32);
    let default_clock_resource_id = match field(fields, "default_clock") {
        Some(NbtSummaryValue::String(resource_id)) => Some(resource_id.clone()),
        _ => None,
    };
    let (environment_attributes, environment_error) = match field(fields, "attributes") {
        Some(NbtSummaryValue::Compound(entries)) => {
            match decode_client_environment_attributes(entries) {
                Ok(attributes) => (attributes, None),
                Err(err) => (ClientEnvironmentAttributes::default(), Some(err)),
            }
        }
        Some(_) => (
            ClientEnvironmentAttributes::default(),
            Some(EnvironmentAttributeProjectionError {
                field: "attributes",
                reason: "expected compound",
            }),
        ),
        None => (ClientEnvironmentAttributes::default(), None),
    };
    Some(DimensionTypeClientProjection {
        dimension,
        has_sky_light,
        has_ceiling,
        cardinal_lighting,
        skybox,
        ambient_light,
        default_clock_resource_id,
        environment_attributes,
        environment_error,
    })
}

fn dimension_type_shape_from_fields(fields: &[NbtSummaryEntry]) -> Option<WorldDimension> {
    // DimensionType.MIN_Y/MAX_Y/Y_SIZE with BlockPos.PACKED_Y_LENGTH = 12.
    const MIN_Y: i32 = -2032;
    const MAX_Y: i32 = 2031;
    const MAX_HEIGHT: i32 = 4064;

    let min_y = numeric_i32(field(fields, "min_y")?)?;
    let height = numeric_i32(field(fields, "height")?)?;
    let logical_height = numeric_i32(field(fields, "logical_height")?)?;
    if !(MIN_Y..=MAX_Y).contains(&min_y)
        || !(16..=MAX_HEIGHT).contains(&height)
        || !(0..=MAX_HEIGHT).contains(&logical_height)
        || logical_height > height
        || min_y.rem_euclid(16) != 0
        || height % 16 != 0
        || min_y.checked_add(height)? > MAX_Y + 1
    {
        return None;
    }
    Some(WorldDimension { min_y, height })
}

fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn bool_field(fields: &[NbtSummaryEntry], name: &str) -> Option<bool> {
    Some(match field(fields, name)? {
        NbtSummaryValue::Byte(value) => *value != 0,
        NbtSummaryValue::Short(value) => *value != 0,
        NbtSummaryValue::Int(value) => *value != 0,
        NbtSummaryValue::Long(value) => *value != 0,
        _ => return None,
    })
}

fn numeric_i32(value: &NbtSummaryValue) -> Option<i32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
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

fn cardinal_lighting_type_from_name(name: &str) -> Option<WorldCardinalLighting> {
    match name {
        "default" => Some(WorldCardinalLighting::Default),
        "nether" => Some(WorldCardinalLighting::Nether),
        _ => None,
    }
}

fn skybox_from_name(name: &str) -> Option<WorldSkybox> {
    match name {
        "none" => Some(WorldSkybox::None),
        "overworld" => Some(WorldSkybox::Overworld),
        "end" => Some(WorldSkybox::End),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_pack_value_uses_the_dimension_projection() {
        let known = super::super::element_value::decode_known_pack_element(
            br#"{
                "has_skylight": true,
                "has_ceiling": false,
                "min_y": -64,
                "height": 384,
                "logical_height": 384,
                "ambient_light": 0.25,
                "skybox": "end",
                "cardinal_light": "nether",
                "default_clock": "minecraft:the_end"
            }"#,
        )
        .unwrap();
        let projection = dimension_type_client_projection(&known.value).unwrap();

        assert_eq!(projection.dimension, Some(WorldDimension::default()));
        assert_eq!(projection.has_sky_light, Some(true));
        assert_eq!(projection.has_ceiling, Some(false));
        assert_eq!(projection.ambient_light, Some(0.25));
        assert_eq!(projection.skybox, Some(WorldSkybox::End));
        assert_eq!(
            projection.cardinal_lighting,
            Some(WorldCardinalLighting::Nether)
        );
        assert_eq!(
            projection.default_clock_resource_id.as_deref(),
            Some("minecraft:the_end")
        );
    }
}
