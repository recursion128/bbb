//! Typed Java 26.1 load support and geometry for multi-piece igloo starts.

use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;

use super::{
    template_pivot, transform_about_pivot, IglooPiece, IglooStructureStart, IGLOO_PIECE_ID,
};
use crate::noise::structure_nbt::{
    compound, entry, int, rotation, rotation_name, start_to_nbt, string, NbtFields, NbtRead,
};
use crate::noise::structure_pieces::JigsawRotation;

impl IglooStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        start_to_nbt(
            self.structure_id(),
            self.start_chunk(),
            self.references(),
            self.pieces().iter().map(igloo_piece_to_nbt).collect(),
        )
    }

    pub fn from_nbt(outer_id: &str, value: &NbtSummaryValue) -> Option<Self> {
        let fields = value.compound()?;
        let pieces = fields
            .field("Children")?
            .list()?
            .iter()
            .map(piece_from_nbt)
            .collect::<Option<Vec<_>>>()?;
        Some(Self::from_parts(
            outer_id,
            [
                fields.field("ChunkX")?.int()?,
                fields.field("ChunkZ")?.int()?,
            ],
            fields.field("references")?.int()?,
            pieces,
        ))
    }

    pub fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.bounding_box()
            .is_some_and(|bounds| bounds.intersects_chunk(chunk_x, chunk_z))
    }
}

fn igloo_piece_to_nbt(piece: &IglooPiece) -> NbtSummaryValue {
    let position = piece.template_position();
    let size = piece.template_size();
    let bounds = if size == [0, 0, 0] {
        [
            position.x, position.y, position.z, position.x, position.y, position.z,
        ]
    } else {
        piece_bounding_box(piece.template(), position, piece.rotation(), size)
    };
    compound(vec![
        string("id", IGLOO_PIECE_ID),
        entry("BB", NbtSummaryValue::IntArray(bounds.to_vec())),
        int("O", 2),
        int("GD", 0),
        int("TPX", position.x),
        int("TPY", position.y),
        int("TPZ", position.z),
        string("Template", piece.template()),
        string("Rot", rotation_name(piece.rotation())),
    ])
}

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<IglooPiece> {
    let fields = value.compound()?;
    let template = fields.field("Template")?.resource_id()?;
    template_pivot(&template)?;
    Some(IglooPiece::new(
        template,
        BlockPos {
            x: fields.field("TPX")?.int()?,
            y: fields.field("TPY")?.int()?,
            z: fields.field("TPZ")?.int()?,
        },
        rotation(fields.field("Rot")?)?,
    ))
}

/// `StructureTemplate.getBoundingBox` with `Mirror.NONE` and the piece pivot
/// (`StructureTemplate.java:619-628`, `IglooPieces.java:38-40`, `IglooPieces.java:89-96`).
pub(crate) fn piece_bounding_box(
    template: &str,
    position: BlockPos,
    rotation: JigsawRotation,
    size: [i32; 3],
) -> [i32; 6] {
    let pivot = template_pivot(template).unwrap_or(BlockPos { x: 0, y: 0, z: 0 });
    let corner1 = transform_about_pivot(BlockPos { x: 0, y: 0, z: 0 }, rotation, pivot);
    let corner2 = transform_about_pivot(
        BlockPos {
            x: size[0] - 1,
            y: size[1] - 1,
            z: size[2] - 1,
        },
        rotation,
        pivot,
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
