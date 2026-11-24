//! Typed Java 26.1 load support for the one-piece buried-treasure start.

use bbb_protocol::NbtSummaryValue;

use super::{BuriedTreasurePiece, BuriedTreasureStructureStart};
use crate::noise::structure_nbt::{compound, entry, int, start_to_nbt, string, NbtFields, NbtRead};

const BURIED_TREASURE_PIECE_ID: &str = "minecraft:btp";

impl BuriedTreasureStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        let p = self.piece.position;
        start_to_nbt(
            &self.structure_id,
            self.start_chunk,
            self.references,
            vec![compound(vec![
                string("id", BURIED_TREASURE_PIECE_ID),
                entry(
                    "BB",
                    NbtSummaryValue::IntArray(vec![p.x, p.y, p.z, p.x, p.y, p.z]),
                ),
                int("O", -1),
                int("GD", 0),
            ])],
        )
    }

    pub fn from_nbt(outer_id: &str, value: &NbtSummaryValue) -> Option<Self> {
        let fields = value.compound()?;
        let [piece] = fields.field("Children")?.list()? else {
            return None;
        };
        let bounds = crate::noise::structure_nbt::bounding_box(piece.compound()?.field("BB")?)?;
        Some(Self {
            structure_id: outer_id.to_string(),
            start_chunk: [
                fields.field("ChunkX")?.int()?,
                fields.field("ChunkZ")?.int()?,
            ],
            references: fields.field("references")?.int()?,
            terrain_adjustment: crate::noise::structures::StructureTerrainAdjustment::None,
            piece: BuriedTreasurePiece {
                position: bounds.min,
            },
            cached_bounding_box: crate::noise::structure_pieces::CachedStartBoundingBox::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::BlockPos;

    use super::*;
    use crate::noise::buried_treasure::{BuriedTreasureGenerationStub, BURIED_TREASURE_ID};
    use crate::noise::structure_nbt::{compound, entry, int, string};

    #[test]
    fn buried_treasure_start_loads_vanilla_one_piece_envelope() {
        let chunk = [-3, 5];
        let expected = BuriedTreasureStructureStart::from_generation_stub(
            BuriedTreasureGenerationStub::new(
                chunk,
                BlockPos {
                    x: chunk[0] * 16 + 8,
                    y: 47,
                    z: chunk[1] * 16 + 8,
                },
                BlockPos {
                    x: chunk[0] * 16 + 9,
                    y: 47,
                    z: chunk[1] * 16 + 9,
                },
            ),
            3,
        );
        let encoded = compound(vec![
            string("id", BURIED_TREASURE_ID),
            int("ChunkX", chunk[0]),
            int("ChunkZ", chunk[1]),
            int("references", 3),
            entry(
                "Children",
                NbtSummaryValue::List(vec![compound(vec![
                    string("id", "minecraft:btp"),
                    entry(
                        "BB",
                        NbtSummaryValue::IntArray(vec![-39, 47, 89, -39, 47, 89]),
                    ),
                    int("O", -1),
                    int("GD", 0),
                ])]),
            ),
        ]);
        let decoded = BuriedTreasureStructureStart::from_nbt(BURIED_TREASURE_ID, &encoded).unwrap();
        assert_eq!(decoded, expected);
    }
}
