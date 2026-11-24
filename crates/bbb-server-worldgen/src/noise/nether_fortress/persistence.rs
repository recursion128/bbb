//! Java 26.1 `NetherFortressPieces` load support.

use bbb_protocol::NbtSummaryValue;

use super::{FortressPiece, FortressPieceKind, FortressStructureStart};
use crate::noise::scattered::HorizontalDirection;
use crate::noise::structure_nbt::{
    bounding_box, bounding_box_to_nbt, byte, compound, entry, int, start_to_nbt, string, NbtFields,
    NbtRead,
};
use crate::noise::structures::NoiseStructurePlan;

impl FortressStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        start_to_nbt(
            self.structure_id(),
            self.start_chunk(),
            self.references(),
            self.pieces().iter().map(piece_to_nbt).collect(),
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
        Some(Self::from_parts_for(
            outer_id,
            [
                fields.field("ChunkX")?.int()?,
                fields.field("ChunkZ")?.int()?,
            ],
            fields.field("references")?.int()?,
            plan.structures().get(outer_id)?.terrain_adjustment,
            pieces,
        ))
    }
}

fn piece_to_nbt(piece: &FortressPiece) -> NbtSummaryValue {
    let mut fields = vec![
        string("id", piece.kind.piece_id()),
        entry("BB", bounding_box_to_nbt(piece.bounding_box)),
        int("O", piece.orientation.to_2d()),
        int("GD", piece.generation_depth),
    ];
    if piece.kind == FortressPieceKind::EndFiller {
        fields.push(int("Seed", piece.random_seed));
    }
    if matches!(
        piece.kind,
        FortressPieceKind::SmallLeftTurn | FortressPieceKind::SmallRightTurn
    ) {
        fields.push(byte("Chest", i8::from(piece.chest_pending)));
    }
    if piece.kind == FortressPieceKind::MonsterThrone {
        fields.push(byte("Mob", i8::from(piece.spawner_placed)));
    }
    compound(fields)
}

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<FortressPiece> {
    let fields = value.compound()?;
    let flag = |name: &str| fields.field(name).and_then(NbtRead::boolean);
    Some(FortressPiece {
        kind: kind_from_id(&fields.field("id")?.resource_id()?)?,
        generation_depth: fields.field("GD")?.int()?,
        bounding_box: bounding_box(fields.field("BB")?)?,
        orientation: HorizontalDirection::from_2d(fields.field("O")?.int()?)?,
        random_seed: fields.field("Seed").and_then(NbtRead::int).unwrap_or(0),
        chest_pending: flag("Chest").unwrap_or(false),
        spawner_placed: flag("Mob").unwrap_or(false),
    })
}

fn kind_from_id(id: &str) -> Option<FortressPieceKind> {
    use FortressPieceKind::*;
    Some(match id {
        "minecraft:nestart" => Start,
        "minecraft:nebcr" => BridgeCrossing,
        "minecraft:nebs" => BridgeStraight,
        "minecraft:nerc" => RoomCrossing,
        "minecraft:nesr" => StairsRoom,
        "minecraft:nemt" => MonsterThrone,
        "minecraft:nece" => CastleEntrance,
        "minecraft:nesc" => SmallCorridor,
        "minecraft:nescsc" => SmallCrossing,
        "minecraft:nesclt" => SmallLeftTurn,
        "minecraft:nescrt" => SmallRightTurn,
        "minecraft:neccs" => CorridorStairs,
        "minecraft:nectb" => TBalcony,
        "minecraft:necsr" => StalkRoom,
        "minecraft:nebef" => EndFiller,
        _ => return None,
    })
}
