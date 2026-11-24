//! Typed Java 26.1 load support and geometry for one-piece shipwreck starts.

use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;

use super::{ShipwreckPiece, ShipwreckStructureStart, SHIPWRECK_PIECE_ID, SHIPWRECK_PIVOT};
use crate::noise::structure_nbt::{
    byte, compound, entry, int, rotation, rotation_name, start_to_nbt, string, NbtFields, NbtRead,
};
use crate::noise::structure_pieces::JigsawRotation;

impl ShipwreckStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        let piece = &self.piece;
        let position = piece.template_position();
        let size = piece.template_size();
        let bounds = if size == [0, 0, 0] {
            [
                position.x, position.y, position.z, position.x, position.y, position.z,
            ]
        } else {
            piece_bounding_box(position, piece.rotation(), size)
        };
        start_to_nbt(
            &self.structure_id,
            self.start_chunk,
            self.references,
            vec![compound(vec![
                string("id", SHIPWRECK_PIECE_ID),
                entry("BB", NbtSummaryValue::IntArray(bounds.to_vec())),
                int("O", 2),
                int("GD", 0),
                int("TPX", position.x),
                int("TPY", position.y),
                int("TPZ", position.z),
                string("Template", piece.template()),
                byte("isBeached", i8::from(piece.is_beached())),
                string("Rot", rotation_name(piece.rotation())),
                byte("height_adjusted", i8::from(piece.height_adjusted())),
            ])],
        )
    }

    pub fn from_parts(
        structure_id: impl Into<String>,
        start_chunk: [i32; 2],
        references: i32,
        piece: ShipwreckPiece,
    ) -> Self {
        Self {
            structure_id: structure_id.into(),
            start_chunk,
            references,
            terrain_adjustment: crate::noise::structures::StructureTerrainAdjustment::None,
            piece,
            cached_bounding_box: crate::noise::structure_pieces::CachedStartBoundingBox::new(),
        }
    }

    pub fn from_nbt(outer_id: &str, value: &NbtSummaryValue) -> Option<Self> {
        let fields = value.compound()?;
        let [piece] = fields.field("Children")?.list()? else {
            return None;
        };
        Some(Self::from_parts(
            outer_id,
            [
                fields.field("ChunkX")?.int()?,
                fields.field("ChunkZ")?.int()?,
            ],
            fields.field("references")?.int()?,
            piece_from_nbt(piece)?,
        ))
    }

    pub fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.bounding_box().intersects_chunk(chunk_x, chunk_z)
    }
}

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<ShipwreckPiece> {
    let fields = value.compound()?;
    Some(ShipwreckPiece::from_parts(
        fields.field("Template")?.resource_id()?,
        BlockPos {
            x: fields.field("TPX")?.int()?,
            y: fields.field("TPY")?.int()?,
            z: fields.field("TPZ")?.int()?,
        },
        rotation(fields.field("Rot")?)?,
        fields.field("isBeached")?.boolean()?,
        fields.field("height_adjusted")?.boolean()?,
    ))
}

/// `StructureTemplate.getBoundingBox` with `Mirror.NONE` and pivot `(4,0,15)`
/// (`StructureTemplate.java:615-629`, `ShipwreckPieces.java:33,118-123`).
pub(crate) fn piece_bounding_box(
    position: BlockPos,
    rotation: JigsawRotation,
    size: [i32; 3],
) -> [i32; 6] {
    let corner1 = transform(BlockPos { x: 0, y: 0, z: 0 }, rotation);
    let corner2 = transform(
        BlockPos {
            x: size[0] - 1,
            y: size[1] - 1,
            z: size[2] - 1,
        },
        rotation,
    );
    [
        corner1.x.min(corner2.x) + position.x,
        corner1.y.min(corner2.y) + position.y,
        corner1.z.min(corner2.z) + position.z,
        corner1.x.max(corner2.x) + position.x,
        corner1.y.max(corner2.y) + position.y,
        corner1.z.max(corner2.z) + position.z,
    ]
}

/// `StructureTemplate.transform` under `Mirror.NONE`
/// (`StructureTemplate.java:528-555`).
fn transform(pos: BlockPos, rotation: JigsawRotation) -> BlockPos {
    let pivot = SHIPWRECK_PIVOT;
    match rotation {
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: pivot.x - pivot.z + pos.z,
            y: pos.y,
            z: pivot.x + pivot.z - pos.x,
        },
        JigsawRotation::Clockwise90 => BlockPos {
            x: pivot.x + pivot.z - pos.z,
            y: pos.y,
            z: pivot.z - pivot.x + pos.x,
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: pivot.x + pivot.x - pos.x,
            y: pos.y,
            z: pivot.z + pivot.z - pos.z,
        },
        JigsawRotation::None => pos,
    }
}
