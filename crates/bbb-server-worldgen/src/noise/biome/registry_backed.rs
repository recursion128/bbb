//! Registry-backed fixed and checkerboard biome-source runtimes.

use bbb_protocol::{normalize_resource_location, NbtSummaryValue};
use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::tag_loader::TagLoadReport;

use super::direct::{compound, reject_extra, required};
use super::{refuse, NoiseBiomeError, BIOME_REGISTRY};

enum RegistryBackedBiomeKind {
    Fixed { biome: i32 },
    Checkerboard { biomes: Vec<i32>, shift: u32 },
}

pub struct RegistryBackedBiomeSource {
    kind: RegistryBackedBiomeKind,
    possible_biomes: Vec<String>,
    registry_len: usize,
}

impl RegistryBackedBiomeSource {
    pub fn from_nbt(
        generation: &DatapackGeneration,
        tags: &TagLoadReport,
        value: &NbtSummaryValue,
        path: &str,
    ) -> Result<Self, NoiseBiomeError> {
        let fields = compound(value, path)?;
        let type_id = resource_id(required(fields, "type", path)?, &format!("{path}.type"))?;
        let biomes = generation
            .registry(BIOME_REGISTRY)
            .ok_or_else(|| refuse(BIOME_REGISTRY, "registry is absent"))?;
        let resolve = |id: &str, holder_path: &str| {
            biomes
                .elements
                .iter()
                .position(|element| element.id == id)
                .and_then(|index| i32::try_from(index).ok())
                .ok_or_else(|| refuse(holder_path, format!("has no published biome {id}")))
        };
        let (kind, source_biomes) = match type_id.as_str() {
            "minecraft:fixed" => {
                reject_extra(fields, &["type", "biome"], path)?;
                let holder_path = format!("{path}.biome");
                let id = named_holder(required(fields, "biome", path)?, &holder_path)?;
                let biome = resolve(&id, &holder_path)?;
                (RegistryBackedBiomeKind::Fixed { biome }, vec![id])
            }
            "minecraft:checkerboard" => {
                reject_extra(fields, &["type", "biomes", "scale"], path)?;
                let holders_path = format!("{path}.biomes");
                let source_biomes =
                    holder_set(required(fields, "biomes", path)?, tags, &holders_path)?;
                if source_biomes.is_empty() {
                    return Err(refuse(&holders_path, "checkerboard biome set is empty"));
                }
                let mut resolved = Vec::with_capacity(source_biomes.len());
                for (index, id) in source_biomes.iter().enumerate() {
                    resolved.push(resolve(id, &format!("{holders_path}[{index}]"))?);
                }
                let scale = fields
                    .iter()
                    .find(|entry| entry.name == "scale")
                    .map(|entry| integer(&entry.value, &format!("{path}.scale")))
                    .transpose()?
                    .unwrap_or(2);
                if !(0..=62).contains(&scale) {
                    return Err(refuse(
                        format!("{path}.scale"),
                        "checkerboard scale must be within [0,62]",
                    ));
                }
                // Java masks int shift distances; scale 62 therefore shifts by zero
                // (`CheckerboardColumnBiomeSource.java:19-37`).
                let shift = u32::try_from((scale + 2) & 31).expect("masked shift is nonnegative");
                (
                    RegistryBackedBiomeKind::Checkerboard {
                        biomes: resolved,
                        shift,
                    },
                    source_biomes,
                )
            }
            _ => {
                return Err(refuse(
                    format!("{path}.type"),
                    format!("biome source type {type_id} has no runtime owner"),
                ))
            }
        };
        let mut possible_biomes = Vec::new();
        for biome in source_biomes {
            if !possible_biomes.contains(&biome) {
                possible_biomes.push(biome);
            }
        }
        Ok(Self {
            kind,
            possible_biomes,
            registry_len: biomes.elements.len(),
        })
    }

    pub const fn registry_len(&self) -> usize {
        self.registry_len
    }

    pub fn possible_biomes(&self) -> Vec<&str> {
        self.possible_biomes.iter().map(String::as_str).collect()
    }

    pub const fn fixed_biome(&self) -> Option<i32> {
        match &self.kind {
            RegistryBackedBiomeKind::Fixed { biome } => Some(*biome),
            RegistryBackedBiomeKind::Checkerboard { .. } => None,
        }
    }

    pub fn sample(&self, quart_x: i32, quart_z: i32) -> i32 {
        match &self.kind {
            RegistryBackedBiomeKind::Fixed { biome } => *biome,
            RegistryBackedBiomeKind::Checkerboard { biomes, shift } => {
                let index = (quart_x >> *shift)
                    .wrapping_add(quart_z >> *shift)
                    .rem_euclid(i32::try_from(biomes.len()).expect("biome count fits i32"));
                biomes[usize::try_from(index).expect("floorMod is nonnegative")]
            }
        }
    }
}

fn holder_set(
    value: &NbtSummaryValue,
    tags: &TagLoadReport,
    path: &str,
) -> Result<Vec<String>, NoiseBiomeError> {
    match value {
        NbtSummaryValue::String(raw) if raw.units().first() == Some(&u16::from(b'#')) => {
            let tag = normalize_resource_location(
                String::from_utf16(&raw.units()[1..])
                    .map_err(|error| refuse(path, error.to_string()))?,
            )
            .map_err(|error| refuse(path, error.to_string()))?;
            tags.registry(BIOME_REGISTRY)
                .and_then(|registry| registry.tag(&tag))
                .map(<[String]>::to_vec)
                .ok_or_else(|| refuse(path, format!("missing biome tag #{tag}")))
        }
        NbtSummaryValue::String(_) => named_holder(value, path).map(|holder| vec![holder]),
        NbtSummaryValue::List(values) => {
            if values
                .iter()
                .any(|value| !matches!(value, NbtSummaryValue::String(_)))
            {
                return Err(refuse(
                    path,
                    "inline or mixed biome holders have no published registry identity",
                ));
            }
            values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let holder_path = format!("{path}[{index}]");
                    if matches!(value, NbtSummaryValue::String(raw) if raw.units().first() == Some(&u16::from(b'#'))) {
                        return Err(refuse(&holder_path, "nested biome tags are not holders"));
                    }
                    named_holder(value, &holder_path)
                })
                .collect()
        }
        NbtSummaryValue::Compound(_) => Err(refuse(
            path,
            "inline biome holder has no published registry identity",
        )),
        _ => Err(refuse(path, "must be a compact biome HolderSet")),
    }
}

fn named_holder(value: &NbtSummaryValue, path: &str) -> Result<String, NoiseBiomeError> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(refuse(
            path,
            "inline biome holder has no published registry identity",
        ));
    };
    if raw.units().first() == Some(&u16::from(b'#')) {
        return Err(refuse(path, "biome tag is not a named holder"));
    }
    normalize_resource_location(
        raw.try_to_string()
            .map_err(|error| refuse(path, error.to_string()))?,
    )
    .map_err(|error| refuse(path, error.to_string()))
}

fn resource_id(value: &NbtSummaryValue, path: &str) -> Result<String, NoiseBiomeError> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(refuse(path, "must be a resource id"));
    };
    normalize_resource_location(
        raw.try_to_string()
            .map_err(|error| refuse(path, error.to_string()))?,
    )
    .map_err(|error| refuse(path, error.to_string()))
}

fn integer(value: &NbtSummaryValue, path: &str) -> Result<i32, NoiseBiomeError> {
    let value = match value {
        NbtSummaryValue::Byte(value) => i64::from(*value),
        NbtSummaryValue::Short(value) => i64::from(*value),
        NbtSummaryValue::Int(value) => i64::from(*value),
        NbtSummaryValue::Long(value) => *value,
        _ => return Err(refuse(path, "must be an integer")),
    };
    i32::try_from(value).map_err(|_| refuse(path, "integer is outside the Java int range"))
}
