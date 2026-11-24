//! Typed Java 26.1 load support and geometry for woodland-mansion starts.

use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;

use super::{WoodlandMansionPiece, WoodlandMansionStructureStart, WOODLAND_MANSION_PIECE_ID};
use crate::noise::ruined_portal::RuinedPortalMirror;
use crate::noise::structure_nbt::{
    compound, entry, int, rotation, rotation_name, start_to_nbt, string, NbtFields, NbtRead,
};
use crate::noise::structure_pieces::JigsawRotation;

impl WoodlandMansionStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        start_to_nbt(
            self.structure_id(),
            self.start_chunk(),
            self.references(),
            self.pieces().iter().map(piece_to_nbt).collect(),
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
        if pieces.is_empty() {
            return None;
        }
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

fn piece_to_nbt(piece: &WoodlandMansionPiece) -> NbtSummaryValue {
    let position = piece.template_position();
    let size = piece.template_size();
    let bounds = if size == [0, 0, 0] {
        [
            position.x, position.y, position.z, position.x, position.y, position.z,
        ]
    } else {
        piece_bounding_box(position, piece.rotation(), piece.mirror(), size)
    };
    compound(vec![
        string("id", WOODLAND_MANSION_PIECE_ID),
        entry("BB", NbtSummaryValue::IntArray(bounds.to_vec())),
        int("O", 2),
        int("GD", 0),
        int("TPX", position.x),
        int("TPY", position.y),
        int("TPZ", position.z),
        string("Template", piece.template()),
        string("Rot", rotation_name(piece.rotation())),
        string("Mi", piece.mirror().legacy_name()),
    ])
}

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<WoodlandMansionPiece> {
    let fields = value.compound()?;
    Some(WoodlandMansionPiece::new(
        fields.field("Template")?.resource_id()?,
        BlockPos {
            x: fields.field("TPX")?.int()?,
            y: fields.field("TPY")?.int()?,
            z: fields.field("TPZ")?.int()?,
        },
        rotation(fields.field("Rot")?)?,
        RuinedPortalMirror::from_legacy_name(&fields.field("Mi")?.string()?)?,
    ))
}

/// `StructureTemplate.transform` (`StructureTemplate.java:528-556`) with the
/// pivot `BlockPos.ZERO` that `WoodlandMansionPieces.makeSettings` always uses
/// (`WoodlandMansionPieces.java:1242-1244`).
pub fn transform_zero_pivot(
    pos: BlockPos,
    mirror: RuinedPortalMirror,
    rotation: JigsawRotation,
) -> BlockPos {
    let (mut x, y, mut z) = (pos.x, pos.y, pos.z);
    match mirror {
        RuinedPortalMirror::LeftRight => z = z.wrapping_neg(),
        RuinedPortalMirror::FrontBack => x = x.wrapping_neg(),
        RuinedPortalMirror::None => {}
    }
    match rotation {
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: z,
            y,
            z: x.wrapping_neg(),
        },
        JigsawRotation::Clockwise90 => BlockPos {
            x: z.wrapping_neg(),
            y,
            z: x,
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: x.wrapping_neg(),
            y,
            z: z.wrapping_neg(),
        },
        JigsawRotation::None => BlockPos { x, y, z },
    }
}

/// `StructureTemplate.getBoundingBox` (`StructureTemplate.java:619-629`).
pub fn piece_bounding_box(
    position: BlockPos,
    rotation: JigsawRotation,
    mirror: RuinedPortalMirror,
    size: [i32; 3],
) -> [i32; 6] {
    let delta = BlockPos {
        x: size[0] - 1,
        y: size[1] - 1,
        z: size[2] - 1,
    };
    let corner1 = transform_zero_pivot(BlockPos { x: 0, y: 0, z: 0 }, mirror, rotation);
    let corner2 = transform_zero_pivot(delta, mirror, rotation);
    [
        corner1.x.min(corner2.x) + position.x,
        corner1.y.min(corner2.y) + position.y,
        corner1.z.min(corner2.z) + position.z,
        corner1.x.max(corner2.x) + position.x,
        corner1.y.max(corner2.y) + position.y,
        corner1.z.max(corner2.z) + position.z,
    ]
}
