//! Direct multi-noise parameter-list owner.

use super::climate::{ParameterPoint, ParameterTree};
use super::{refuse, NoiseBiomeError, BIOME_REGISTRY};
use crate::noise::density::ClimateSampler;
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::dynamic_registry::DatapackGeneration;

pub struct DirectBiomeSearch {
    pub last_result: Option<usize>,
}

impl DirectBiomeSearch {
    pub const fn new() -> Self {
        Self { last_result: None }
    }
}

impl Default for DirectBiomeSearch {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DirectBiomeSource {
    points: Vec<ParameterPoint>,
    tree: ParameterTree,
    registry_len: usize,
    possible_biomes: Vec<String>,
}

impl DirectBiomeSource {
    pub fn from_nbt(
        generation: &DatapackGeneration,
        value: &NbtSummaryValue,
        path: &str,
    ) -> Result<Self, NoiseBiomeError> {
        let entries = list(value, path)?;
        if entries.is_empty() {
            return Err(refuse(path, "direct multi-noise biomes must be non-empty"));
        }
        let biomes = generation
            .registry(BIOME_REGISTRY)
            .ok_or_else(|| refuse(BIOME_REGISTRY, "registry is absent"))?;
        let resolve = |id: &str, biome_path: &str| {
            biomes
                .elements
                .iter()
                .position(|element| element.id == id)
                .and_then(|i| i32::try_from(i).ok())
                .ok_or_else(|| refuse(biome_path, format!("has no published biome {id}")))
        };
        // Direct and preset forms are the two `mapEither` codec branches
        // (`MultiNoiseBiomeSource.java:18-24`; `Climate.java:156-169`).
        let mut points = Vec::with_capacity(entries.len());
        // `BiomeSource.java:29-41` retains the source's distinct possible-biome order.
        let mut possible_biomes = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            let entry_path = format!("{path}[{index}]");
            let fields = compound(entry, &entry_path)?;
            reject_extra(fields, &["biome", "parameters"], &entry_path)?;
            let biome_path = format!("{entry_path}.biome");
            let raw_biome = string_field(fields, "biome", &entry_path)?;
            if raw_biome.starts_with('#') {
                return Err(refuse(&biome_path, "tags are not biome holders"));
            }
            let biome = normalize_resource_location(raw_biome)
                .map_err(|error| refuse(&biome_path, error.to_string()))?;
            let biome_index = resolve(&biome, &biome_path)?;
            if !possible_biomes.iter().any(|possible| possible == &biome) {
                possible_biomes.push(biome.clone());
            }
            let parameters_path = format!("{entry_path}.parameters");
            let parameters = compound(
                required(fields, "parameters", &entry_path)?,
                &parameters_path,
            )?;
            let names = [
                "temperature",
                "humidity",
                "continentalness",
                "erosion",
                "depth",
                "weirdness",
            ];
            let mut ranges = [[0_i64; 2]; 6];
            reject_extra(
                parameters,
                &[
                    "temperature",
                    "humidity",
                    "continentalness",
                    "erosion",
                    "depth",
                    "weirdness",
                    "offset",
                ],
                &parameters_path,
            )?;
            for (dimension, name) in names.iter().enumerate() {
                ranges[dimension] = parse_parameter(
                    required(parameters, name, &parameters_path)?,
                    &format!("{parameters_path}.{name}"),
                    -2.0,
                    2.0,
                )?;
            }
            // Offset is a scalar while the six climate dimensions use interval codecs
            // (`Climate.java:208-228`).
            let offset = parse_scalar(
                required(parameters, "offset", &parameters_path)?,
                &format!("{parameters_path}.offset"),
                0.0,
                1.0,
            )?;
            points.push(ParameterPoint::new(ranges, offset, biome_index));
        }
        let tree = ParameterTree::new(&points);
        Ok(Self {
            points,
            tree,
            registry_len: biomes.elements.len(),
            possible_biomes,
        })
    }

    pub fn possible_biomes(&self) -> &[String] {
        &self.possible_biomes
    }

    pub const fn registry_len(&self) -> usize {
        self.registry_len
    }
    pub fn sample(
        &self,
        climate: &mut impl ClimateSampler,
        x: i32,
        y: i32,
        z: i32,
        search: &mut DirectBiomeSearch,
    ) -> i32 {
        let target = climate
            .climate_target(x, y, z)
            .expect("direct source has six climate functions");
        let winner = self
            .tree
            .search(&self.points, &target, &mut search.last_result);
        self.points[winner].biome
    }
}

pub fn list<'a>(
    value: &'a NbtSummaryValue,
    path: &str,
) -> Result<&'a [NbtSummaryValue], NoiseBiomeError> {
    match value {
        NbtSummaryValue::List(v) => Ok(v),
        _ => Err(refuse(path, "must be a list")),
    }
}

pub fn compound<'a>(
    value: &'a NbtSummaryValue,
    path: &str,
) -> Result<&'a [NbtSummaryEntry], NoiseBiomeError> {
    match value {
        NbtSummaryValue::Compound(v) => Ok(v),
        _ => Err(refuse(path, "must be a compound")),
    }
}

pub fn required<'a>(
    fields: &'a [NbtSummaryEntry],
    name: &str,
    path: &str,
) -> Result<&'a NbtSummaryValue, NoiseBiomeError> {
    fields
        .iter()
        .find(|e| e.name == name)
        .map(|e| &e.value)
        .ok_or_else(|| refuse(format!("{path}.{name}"), "is required"))
}

pub fn reject_extra(
    fields: &[NbtSummaryEntry],
    allowed: &[&str],
    path: &str,
) -> Result<(), NoiseBiomeError> {
    if let Some(e) = fields
        .iter()
        .find(|e| !allowed.iter().any(|name| e.name == *name))
    {
        return Err(refuse(
            format!("{path}.{}", e.name.to_display_string()),
            "unknown field",
        ));
    }
    for (index, entry) in fields.iter().enumerate() {
        if fields[..index]
            .iter()
            .any(|previous| previous.name == entry.name)
        {
            return Err(refuse(
                format!("{path}.{}", entry.name.to_display_string()),
                "duplicate field",
            ));
        }
    }
    Ok(())
}

pub fn string_field(
    fields: &[NbtSummaryEntry],
    name: &str,
    path: &str,
) -> Result<String, NoiseBiomeError> {
    match required(fields, name, path)? {
        NbtSummaryValue::String(v) => v
            .try_to_string()
            .map_err(|error| refuse(format!("{path}.{name}"), error.to_string())),
        _ => Err(refuse(
            format!("{path}.{name}"),
            "must be a named biome holder",
        )),
    }
}

pub fn number(value: &NbtSummaryValue) -> Option<f32> {
    match value {
        NbtSummaryValue::Byte(v) => Some(*v as f32),
        NbtSummaryValue::Short(v) => Some(*v as f32),
        NbtSummaryValue::Int(v) => Some(*v as f32),
        NbtSummaryValue::Long(v) => Some(*v as f32),
        NbtSummaryValue::Float(v) => Some(f32::from_bits(*v)),
        NbtSummaryValue::Double(v) => Some(f64::from_bits(*v) as f32),
        _ => None,
    }
}

pub fn validate_number(
    value: f32,
    path: &str,
    lower: f32,
    upper: f32,
) -> Result<i64, NoiseBiomeError> {
    if !value.is_finite() || value < lower || value > upper {
        return Err(refuse(
            path,
            format!("parameter must be within [{lower},{upper}]"),
        ));
    }
    Ok((value * 10_000.0_f32) as i64)
}

pub fn parse_scalar(
    value: &NbtSummaryValue,
    path: &str,
    lower: f32,
    upper: f32,
) -> Result<i64, NoiseBiomeError> {
    let value = number(value).ok_or_else(|| refuse(path, "parameter must be a number"))?;
    validate_number(value, path, lower, upper)
}

// `intervalCodec` accepts a point, a two-element list, or a min/max object
// (`ExtraCodecs.java:271-295`).
pub fn parse_parameter(
    value: &NbtSummaryValue,
    path: &str,
    lower: f32,
    upper: f32,
) -> Result<[i64; 2], NoiseBiomeError> {
    let values = match value {
        value if number(value).is_some() => {
            let v = number(value).expect("checked above");
            [v, v]
        }
        NbtSummaryValue::List(v) if v.len() == 2 => [
            number(&v[0]).ok_or_else(|| refuse(path, "range endpoints must be numbers"))?,
            number(&v[1]).ok_or_else(|| refuse(path, "range endpoints must be numbers"))?,
        ],
        NbtSummaryValue::List(_) => {
            return Err(refuse(path, "range must contain exactly two numbers"))
        }
        NbtSummaryValue::Compound(fields) => {
            reject_extra(fields, &["min", "max"], path)?;
            [
                number(required(fields, "min", path)?)
                    .ok_or_else(|| refuse(format!("{path}.min"), "must be a number"))?,
                number(required(fields, "max", path)?)
                    .ok_or_else(|| refuse(format!("{path}.max"), "must be a number"))?,
            ]
        }
        _ => {
            return Err(refuse(
                path,
                "parameter must be a number, two-number list, or min/max object",
            ))
        }
    };
    if values[0] > values[1] {
        return Err(refuse(
            path,
            format!("parameter must be ordered and within [{lower},{upper}]"),
        ));
    }
    Ok([
        validate_number(values[0], path, lower, upper)?,
        validate_number(values[1], path, lower, upper)?,
    ])
}

#[cfg(test)]
mod tests {
    use super::parse_parameter;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    #[test]
    fn parameters_quantize_as_f32_and_accept_points_or_intervals() {
        let point = parse_parameter(
            &NbtSummaryValue::Double(0.123456789_f64.to_bits()),
            "point",
            -2.0,
            2.0,
        )
        .unwrap();
        assert_eq!(point, [1234, 1234]);
        let interval = NbtSummaryValue::List(vec![
            NbtSummaryValue::Float((-0.25_f32).to_bits()),
            NbtSummaryValue::Float(0.5_f32.to_bits()),
        ]);
        assert_eq!(
            parse_parameter(&interval, "interval", -2.0, 2.0).unwrap(),
            [-2500, 5000]
        );
        let object = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "min".into(),
                value: NbtSummaryValue::Double((-0.125_f64).to_bits()),
            },
            NbtSummaryEntry {
                name: "max".into(),
                value: NbtSummaryValue::Float(0.75_f32.to_bits()),
            },
        ]);
        assert_eq!(
            parse_parameter(&object, "object", -2.0, 2.0).unwrap(),
            [-1250, 7500]
        );
    }
}
