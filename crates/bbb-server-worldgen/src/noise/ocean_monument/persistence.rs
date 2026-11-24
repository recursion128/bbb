//! Typed Java 26.1 load support for the one-piece ocean-monument start.

use bbb_protocol::NbtSummaryValue;

use super::{OceanMonumentPiece, OceanMonumentStructureStart, OCEAN_MONUMENT_PIECE_ID};
use crate::noise::scattered::HorizontalDirection;
use crate::noise::structure_nbt::{compound, entry, int, start_to_nbt, string, NbtFields, NbtRead};

impl OceanMonumentStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        let piece = self.piece();
        start_to_nbt(
            self.structure_id(),
            self.start_chunk(),
            self.references(),
            vec![compound(vec![
                string("id", OCEAN_MONUMENT_PIECE_ID),
                entry(
                    "BB",
                    NbtSummaryValue::IntArray(piece.bounding_box().to_vec()),
                ),
                int("O", piece.orientation().to_2d()),
                int("GD", 0),
            ])],
        )
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

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<OceanMonumentPiece> {
    let fields = value.compound()?;
    Some(OceanMonumentPiece::new(
        fields.field("BB")?.int_array()?.try_into().ok()?,
        HorizontalDirection::from_2d(fields.field("O")?.int()?)?,
    ))
}
