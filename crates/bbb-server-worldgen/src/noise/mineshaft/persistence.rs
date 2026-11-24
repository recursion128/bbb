//! Java 26.1 NBT load support for mineshaft starts and pieces.

use bbb_protocol::NbtSummaryValue;

use super::{MineshaftDirection, MineshaftPiece, MineshaftPieceKind, MineshaftStructureStart};
use crate::noise::structure_nbt::{
    bounding_box, bounding_box_to_nbt, byte, compound, entry, int, start_to_nbt, string, NbtFields,
    NbtRead,
};
use crate::noise::structures::{MineshaftType, NoiseStructurePlan};

const CORRIDOR_ID: &str = "minecraft:mscorridor";
const CROSSING_ID: &str = "minecraft:mscrossing";
const ROOM_ID: &str = "minecraft:msroom";
const STAIRS_ID: &str = "minecraft:msstairs";

impl MineshaftStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        start_to_nbt(
            &self.structure_id,
            self.start_chunk,
            self.references,
            self.pieces.iter().map(piece_to_nbt).collect(),
        )
    }

    pub fn from_nbt(
        outer_id: &str,
        value: &NbtSummaryValue,
        plan: &NoiseStructurePlan,
    ) -> Option<Self> {
        let fields = value.compound()?;
        let pieces = fields
            .field("Children")?
            .list()?
            .iter()
            .map(piece_from_nbt)
            .collect::<Option<Vec<_>>>()?;
        if pieces.is_empty() {
            return None;
        }
        Some(Self {
            structure_id: outer_id.to_string(),
            start_chunk: [
                fields.field("ChunkX")?.int()?,
                fields.field("ChunkZ")?.int()?,
            ],
            references: fields.field("references")?.int()?,
            terrain_adjustment: plan.structures().get(outer_id)?.terrain_adjustment,
            pieces,
            cached_bounding_box: crate::noise::structure_pieces::CachedStartBoundingBox::new(),
        })
    }
}

fn piece_to_nbt(piece: &MineshaftPiece) -> NbtSummaryValue {
    let mut fields = vec![
        string("id", piece.kind.id()),
        entry("BB", bounding_box_to_nbt(piece.bounding_box)),
        int(
            "O",
            piece.orientation.map_or(-1, MineshaftDirection::nbt_id),
        ),
        int("GD", piece.generation_depth),
        int(
            "MST",
            match piece.mineshaft_type {
                MineshaftType::Normal => 0,
                MineshaftType::Mesa => 1,
            },
        ),
    ];
    match &piece.kind {
        MineshaftPieceKind::Corridor {
            has_rails,
            spider_corridor,
            has_placed_spider,
            num_sections,
        } => {
            fields.push(byte("hr", i8::from(*has_rails)));
            fields.push(byte("sc", i8::from(*spider_corridor)));
            fields.push(byte("hps", i8::from(*has_placed_spider)));
            fields.push(int("Num", *num_sections));
        }
        MineshaftPieceKind::Crossing {
            direction,
            two_floored,
        } => {
            fields.push(byte("tf", i8::from(*two_floored)));
            fields.push(byte("D", direction.nbt_id() as i8));
        }
        MineshaftPieceKind::Room { entrances } => fields.push(entry(
            "Entrances",
            NbtSummaryValue::List(entrances.iter().copied().map(bounding_box_to_nbt).collect()),
        )),
        MineshaftPieceKind::Stairs => {}
    }
    compound(fields)
}

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<MineshaftPiece> {
    let fields = value.compound()?;
    let flag = |name: &str| fields.field(name)?.boolean();
    let orientation = match fields.field("O")?.int()? {
        -1 => None,
        id => Some(direction_from_id(id)?),
    };
    let kind = match fields.field("id")?.resource_id()?.as_str() {
        CORRIDOR_ID => MineshaftPieceKind::Corridor {
            has_rails: flag("hr")?,
            spider_corridor: flag("sc")?,
            has_placed_spider: flag("hps")?,
            num_sections: fields.field("Num")?.int()?,
        },
        CROSSING_ID => MineshaftPieceKind::Crossing {
            direction: direction_from_id(i32::from(fields.field("D")?.byte()?))?,
            two_floored: flag("tf")?,
        },
        ROOM_ID => MineshaftPieceKind::Room {
            entrances: fields
                .field("Entrances")?
                .list()?
                .iter()
                .map(bounding_box)
                .collect::<Option<Vec<_>>>()?,
        },
        STAIRS_ID => MineshaftPieceKind::Stairs,
        _ => return None,
    };
    Some(MineshaftPiece {
        kind,
        generation_depth: fields.field("GD")?.int()?,
        bounding_box: bounding_box(fields.field("BB")?)?,
        orientation,
        mineshaft_type: match fields.field("MST")?.int()? {
            0 => MineshaftType::Normal,
            1 => MineshaftType::Mesa,
            _ => return None,
        },
    })
}

fn direction_from_id(value: i32) -> Option<MineshaftDirection> {
    match value {
        0 => Some(MineshaftDirection::South),
        1 => Some(MineshaftDirection::West),
        2 => Some(MineshaftDirection::North),
        3 => Some(MineshaftDirection::East),
        _ => None,
    }
}
