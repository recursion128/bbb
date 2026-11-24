//! Shared STRUCTURE_STARTS NBT read and reference-window helpers.

use super::*;
use crate::noise::structure_nbt::{
    bounding_box, bounding_box_to_nbt, compound, entry, int, rotation, rotation_name, string,
};

pub fn piece_to_nbt<'a>(
    piece: &JigsawPiece,
    liquid_settings: StructureLiquidSettings,
    junctions: impl Iterator<Item = &'a JigsawGraphJunction>,
) -> NbtSummaryValue {
    let mut fields = vec![
        // `StructurePiece.java:84-92` saves the current registry id.
        string("id", JIGSAW_PIECE_ID),
        entry("BB", bounding_box_to_nbt(piece.bounding_box)),
        int("O", -1),
        int("GD", 0),
        int("PosX", piece.position.x),
        int("PosY", piece.position.y),
        int("PosZ", piece.position.z),
        int("ground_level_delta", piece.ground_level_delta),
        entry("pool_element", pool_element_to_nbt(&piece.element)),
        string("rotation", rotation_name(piece.rotation)),
        entry(
            "junctions",
            NbtSummaryValue::List(junctions.map(junction_to_nbt).collect()),
        ),
    ];
    if liquid_settings != StructureLiquidSettings::ApplyWaterlogging {
        fields.push(string(
            "liquid_settings",
            liquid_settings_name(liquid_settings),
        ));
    }
    compound(fields)
}

pub fn piece_from_nbt(
    value: &NbtSummaryValue,
    piece_index: usize,
    startup: &DatapackStartup,
) -> Option<(
    JigsawPiece,
    StructureLiquidSettings,
    Vec<JigsawGraphJunction>,
)> {
    let fields = value.compound()?;
    // `PiecesContainer.java:21-27,57-65` lowercases and renames legacy ids before lookup.
    let piece_id =
        normalize_resource_location(fields.field("id")?.string()?.to_lowercase()).ok()?;
    if !matches!(
        piece_id.as_str(),
        JIGSAW_PIECE_ID
            | "minecraft:nvi"
            | "minecraft:pcp"
            | "minecraft:bastionremnant"
            | "minecraft:runtime"
    ) {
        return None;
    }
    let liquid_settings = match fields.field("liquid_settings") {
        Some(value) => parse_liquid_settings_nbt(value)?,
        None => StructureLiquidSettings::ApplyWaterlogging,
    };
    let junctions = fields
        .field("junctions")?
        .list()?
        .iter()
        .map(|value| junction_from_nbt(value, piece_index))
        .collect::<Option<Vec<_>>>()?;
    let piece = JigsawPiece {
        element: pool_element_from_nbt(fields.field("pool_element")?, startup)?,
        position: BlockPos {
            x: fields.field("PosX")?.int()?,
            y: fields.field("PosY")?.int()?,
            z: fields.field("PosZ")?.int()?,
        },
        ground_level_delta: fields.field("ground_level_delta")?.int()?,
        rotation: rotation(fields.field("rotation")?)?,
        bounding_box: bounding_box(fields.field("BB")?)?,
    };
    Some((piece, liquid_settings, junctions))
}

pub fn pool_element_to_nbt(element: &StructurePoolElement) -> NbtSummaryValue {
    match element {
        StructurePoolElement::Empty => {
            compound(vec![string("element_type", "minecraft:empty_pool_element")])
        }
        StructurePoolElement::Feature {
            feature,
            projection,
        } => compound(vec![
            string("element_type", "minecraft:feature_pool_element"),
            entry(
                "feature",
                match feature {
                    bbb_server_datapack::structure_pool::PoolFeatureHolder::Named(id) => {
                        NbtSummaryValue::string(id.as_str())
                    }
                    bbb_server_datapack::structure_pool::PoolFeatureHolder::Inline {
                        raw_nbt,
                        ..
                    } => raw_nbt.clone(),
                },
            ),
            string("projection", projection_name(*projection)),
        ]),
        StructurePoolElement::Single {
            legacy,
            location,
            processors,
            projection,
            liquid_settings,
        } => {
            let mut fields = vec![
                string(
                    "element_type",
                    if *legacy {
                        "minecraft:legacy_single_pool_element"
                    } else {
                        "minecraft:single_pool_element"
                    },
                ),
                string("location", location),
                entry(
                    "processors",
                    match processors {
                        ProcessorListHolder::Named(id) => NbtSummaryValue::string(id.as_str()),
                        ProcessorListHolder::InlineEmpty => {
                            compound(vec![entry("processors", NbtSummaryValue::List(Vec::new()))])
                        }
                        ProcessorListHolder::Inline { raw_nbt, .. } => raw_nbt.clone(),
                    },
                ),
                string("projection", projection_name(*projection)),
            ];
            if let Some(liquids) = liquid_settings {
                fields.push(string(
                    "override_liquid_settings",
                    liquid_settings_name(*liquids),
                ));
            }
            compound(fields)
        }
        StructurePoolElement::List {
            elements,
            projection,
        } => compound(vec![
            string("element_type", "minecraft:list_pool_element"),
            entry(
                "elements",
                NbtSummaryValue::List(elements.iter().map(pool_element_to_nbt).collect()),
            ),
            string("projection", projection_name(*projection)),
        ]),
    }
}

pub fn pool_element_from_nbt(
    value: &NbtSummaryValue,
    startup: &DatapackStartup,
) -> Option<StructurePoolElement> {
    let fields = value.compound()?;
    let projection = || parse_projection(fields.field("projection")?);
    let type_id = fields.field("element_type")?.resource_id()?;
    Some(match type_id.as_str() {
        "minecraft:empty_pool_element" => StructurePoolElement::Empty,
        "minecraft:feature_pool_element" => {
            let feature = match fields.field("feature")? {
                NbtSummaryValue::String(_) => {
                    let id = fields.field("feature")?.resource_id()?;
                    startup.registries.element(PLACED_FEATURE_REGISTRY, &id)?;
                    bbb_server_datapack::structure_pool::PoolFeatureHolder::Named(id)
                }
                other @ NbtSummaryValue::Compound(_) => {
                    bbb_server_datapack::structure_pool::parse_inline_feature_nbt(other)?
                }
                _ => return None,
            };
            StructurePoolElement::Feature {
                feature,
                projection: projection()?,
            }
        }
        "minecraft:single_pool_element" | "minecraft:legacy_single_pool_element" => {
            let processors = match fields.field("processors")? {
                NbtSummaryValue::String(id) => {
                    let id = normalize_resource_location(id.try_to_string().ok()?).ok()?;
                    startup
                        .registries
                        .worldgen
                        .processor_lists
                        .contains_key(&id)
                        .then_some(())?;
                    ProcessorListHolder::Named(id)
                }
                other @ (NbtSummaryValue::Compound(_) | NbtSummaryValue::List(_)) => {
                    bbb_server_datapack::structure_pool::parse_inline_processors_nbt(other)?
                }
                _ => return None,
            };
            StructurePoolElement::Single {
                legacy: type_id == "minecraft:legacy_single_pool_element",
                location: fields.field("location")?.resource_id()?,
                processors,
                projection: projection()?,
                liquid_settings: match fields.field("override_liquid_settings") {
                    Some(value) => Some(parse_liquid_settings_nbt(value)?),
                    None => None,
                },
            }
        }
        "minecraft:list_pool_element" => {
            let elements = fields
                .field("elements")?
                .list()?
                .iter()
                .map(|value| pool_element_from_nbt(value, startup))
                .collect::<Option<Vec<_>>>()?;
            if elements.is_empty() {
                return None;
            }
            StructurePoolElement::List {
                elements,
                projection: projection()?,
            }
        }
        _ => return None,
    })
}

pub fn junction_to_nbt(junction: &JigsawGraphJunction) -> NbtSummaryValue {
    compound(vec![
        int("source_x", junction.source_x),
        int("source_ground_y", junction.source_ground_y),
        int("source_z", junction.source_z),
        int("delta_y", junction.delta_y),
        string(
            "dest_proj",
            projection_name(junction.destination_projection),
        ),
    ])
}

pub fn junction_from_nbt(
    value: &NbtSummaryValue,
    piece_index: usize,
) -> Option<JigsawGraphJunction> {
    let fields = value.compound()?;
    Some(JigsawGraphJunction {
        piece_index,
        source_x: fields.field("source_x")?.int()?,
        source_ground_y: fields.field("source_ground_y")?.int()?,
        source_z: fields.field("source_z")?.int()?,
        delta_y: fields.field("delta_y")?.int()?,
        destination_projection: parse_projection(fields.field("dest_proj")?)?,
    })
}

pub fn inflate(bounds: JigsawBoundingBox, amount: i32) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: bounds.min.x - amount,
            y: bounds.min.y - amount,
            z: bounds.min.z - amount,
        },
        max: BlockPos {
            x: bounds.max.x + amount,
            y: bounds.max.y + amount,
            z: bounds.max.z + amount,
        },
    }
}

pub fn chunk_block_bounds(chunk_x: i32, chunk_z: i32) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: chunk_x * 16,
            y: i32::MIN,
            z: chunk_z * 16,
        },
        max: BlockPos {
            x: chunk_x * 16 + 15,
            y: i32::MAX,
            z: chunk_z * 16 + 15,
        },
    }
}

/// Adds starts keyed by structure id, replacing an existing start of the same
/// structure as `ChunkAccess.setStartForStructure` does.
pub fn insert_starts<T>(
    current: &mut Vec<T>,
    starts: impl IntoIterator<Item = T>,
    structure_id: impl Fn(&T) -> &str,
) {
    for start in starts {
        current.retain(|existing| structure_id(existing) != structure_id(&start));
        current.push(start);
    }
    current.sort_by(|left, right| structure_id(left).cmp(structure_id(right)));
}

pub fn pack_chunk(x: i32, z: i32) -> i64 {
    (u64::from(x as u32) | (u64::from(z as u32) << 32)) as i64
}

pub fn unpack_chunk(packed: i64) -> [i32; 2] {
    let packed = packed as u64;
    [(packed as u32) as i32, ((packed >> 32) as u32) as i32]
}

pub const fn projection_name(projection: StructurePoolProjection) -> &'static str {
    match projection {
        StructurePoolProjection::TerrainMatching => "terrain_matching",
        StructurePoolProjection::Rigid => "rigid",
    }
}

pub fn parse_projection(value: &NbtSummaryValue) -> Option<StructurePoolProjection> {
    match value.string()?.as_str() {
        "terrain_matching" => Some(StructurePoolProjection::TerrainMatching),
        "rigid" => Some(StructurePoolProjection::Rigid),
        _ => None,
    }
}

pub const fn liquid_settings_name(settings: StructureLiquidSettings) -> &'static str {
    match settings {
        StructureLiquidSettings::IgnoreWaterlogging => "ignore_waterlogging",
        StructureLiquidSettings::ApplyWaterlogging => "apply_waterlogging",
    }
}

pub fn parse_liquid_settings_nbt(value: &NbtSummaryValue) -> Option<StructureLiquidSettings> {
    match value.string()?.as_str() {
        "ignore_waterlogging" => Some(StructureLiquidSettings::IgnoreWaterlogging),
        "apply_waterlogging" => Some(StructureLiquidSettings::ApplyWaterlogging),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;
    use crate::noise::structure_nbt::{compound, entry, int, string};
    use crate::noise::structure_pieces::JigsawRotation;

    fn startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    fn sample_piece() -> JigsawPiece {
        JigsawPiece {
            element: StructurePoolElement::Empty,
            position: BlockPos { x: 1, y: 2, z: 3 },
            ground_level_delta: 0,
            rotation: JigsawRotation::None,
            bounding_box: JigsawBoundingBox {
                min: BlockPos { x: 1, y: 2, z: 3 },
                max: BlockPos { x: 1, y: 2, z: 3 },
            },
        }
    }

    fn raw_piece_nbt(id: &str) -> NbtSummaryValue {
        compound(vec![
            string("id", id),
            entry("BB", NbtSummaryValue::IntArray(vec![1, 2, 3, 1, 2, 3])),
            int("O", -1),
            int("GD", 0),
            int("PosX", 1),
            int("PosY", 2),
            int("PosZ", 3),
            int("ground_level_delta", 0),
            entry(
                "pool_element",
                compound(vec![string("element_type", "minecraft:empty_pool_element")]),
            ),
            string("rotation", "NONE"),
            entry("junctions", NbtSummaryValue::List(Vec::new())),
        ])
    }

    #[test]
    fn legacy_jigsaw_piece_ids_load_cleanly() {
        let startup = startup();
        let expected = sample_piece();
        for legacy_id in [
            JIGSAW_PIECE_ID,
            "nvi",
            "PCP",
            "minecraft:BastionRemnant",
            "MINECRAFT:RUNTIME",
        ] {
            let encoded = raw_piece_nbt(legacy_id);
            let (decoded, liquid_settings, junctions) =
                piece_from_nbt(&encoded, 0, &startup).unwrap();
            assert_eq!(decoded, expected, "legacy id {legacy_id}");
            assert_eq!(liquid_settings, StructureLiquidSettings::ApplyWaterlogging);
            assert!(junctions.is_empty());
        }
    }
}
