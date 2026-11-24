use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;

use super::{EndCityPiece, EndCityStructureStart, END_CITY_PIECE_ID};
use crate::noise::structure_nbt::{
    bounding_box_to_nbt, byte, compound, entry, int, rotation, rotation_name, start_to_nbt, string,
    NbtFields, NbtRead,
};

impl EndCityStructureStart {
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
}

fn piece_to_nbt(piece: &EndCityPiece) -> NbtSummaryValue {
    let position = piece.template_position();
    compound(vec![
        string("id", END_CITY_PIECE_ID),
        entry("BB", bounding_box_to_nbt(piece.bounding_box())),
        int("O", 2),
        int("GD", piece.gen_depth()),
        int("TPX", position.x),
        int("TPY", position.y),
        int("TPZ", position.z),
        string("Template", piece.template()),
        string("Rot", rotation_name(piece.rotation())),
        byte("OW", i8::from(piece.overwrite())),
    ])
}

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<EndCityPiece> {
    let fields = value.compound()?;
    let mut piece = EndCityPiece::new(
        fields.field("Template")?.resource_id()?,
        BlockPos {
            x: fields.field("TPX")?.int()?,
            y: fields.field("TPY")?.int()?,
            z: fields.field("TPZ")?.int()?,
        },
        rotation(fields.field("Rot")?)?,
        fields.field("OW")?.boolean()?,
        [1, 1, 1],
    );
    piece.set_gen_depth(fields.field("GD")?.int()?);
    Some(piece)
}
