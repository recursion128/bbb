//! Typed Java 26.1 load support for multi-piece stronghold starts.

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::{
    SmallDoorType, StrongholdPiece, StrongholdPieceExtra, StrongholdPieceKind,
    StrongholdStructureStart,
};
use crate::noise::scattered::HorizontalDirection;
use crate::noise::structure_nbt::{
    byte, compound, entry, int, start_to_nbt, string, NbtFields, NbtRead,
};

impl StrongholdStructureStart {
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
        Some(Self::from_generation(
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

fn piece_to_nbt(piece: &StrongholdPiece) -> NbtSummaryValue {
    let mut fields = vec![
        string("id", piece.piece_id()),
        entry(
            "BB",
            NbtSummaryValue::IntArray(piece.bounding_box().to_vec()),
        ),
        int("O", piece.orientation().to_2d()),
        int("GD", piece.gen_depth()),
        string("EntryDoor", piece.entry_door().legacy_name()),
    ];
    match piece.extra() {
        StrongholdPieceExtra::None => {}
        StrongholdPieceExtra::ChestCorridor { chest } => {
            fields.push(byte("Chest", i8::from(*chest)));
        }
        StrongholdPieceExtra::FillerCorridor { steps } => {
            fields.push(int("Steps", *steps));
        }
        StrongholdPieceExtra::FiveCrossing {
            left_low,
            left_high,
            right_low,
            right_high,
        } => {
            fields.push(byte("leftLow", i8::from(*left_low)));
            fields.push(byte("leftHigh", i8::from(*left_high)));
            fields.push(byte("rightLow", i8::from(*right_low)));
            fields.push(byte("rightHigh", i8::from(*right_high)));
        }
        StrongholdPieceExtra::Library { tall } => {
            fields.push(byte("Tall", i8::from(*tall)));
        }
        StrongholdPieceExtra::PortalRoom { mob } => {
            fields.push(byte("Mob", i8::from(*mob)));
        }
        StrongholdPieceExtra::RoomCrossing { room_type } => {
            fields.push(int("Type", *room_type));
        }
        StrongholdPieceExtra::StairsDown { source } => {
            fields.push(byte("Source", i8::from(*source)));
        }
        StrongholdPieceExtra::Straight { left, right } => {
            fields.push(byte("Left", i8::from(*left)));
            fields.push(byte("Right", i8::from(*right)));
        }
    }
    compound(fields)
}

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<StrongholdPiece> {
    let fields = value.compound()?;
    let kind = StrongholdPieceKind::from_piece_id(&fields.field("id")?.resource_id()?)?;
    Some(StrongholdPiece::new(
        kind,
        fields.field("BB")?.int_array()?.try_into().ok()?,
        HorizontalDirection::from_2d(fields.field("O")?.int()?)?,
        fields.field("GD")?.int()?,
        SmallDoorType::from_legacy_name(&fields.field("EntryDoor")?.string()?)?,
        extra_from_nbt(kind, fields)?,
    ))
}

fn extra_from_nbt(
    kind: StrongholdPieceKind,
    fields: &[NbtSummaryEntry],
) -> Option<StrongholdPieceExtra> {
    let flag = |name: &str| fields.field(name)?.boolean();
    let int = |name: &str| fields.field(name)?.int();
    Some(match kind {
        StrongholdPieceKind::ChestCorridor => StrongholdPieceExtra::ChestCorridor {
            chest: flag("Chest")?,
        },
        StrongholdPieceKind::FillerCorridor => StrongholdPieceExtra::FillerCorridor {
            steps: int("Steps")?,
        },
        StrongholdPieceKind::FiveCrossing => StrongholdPieceExtra::FiveCrossing {
            left_low: flag("leftLow")?,
            left_high: flag("leftHigh")?,
            right_low: flag("rightLow")?,
            right_high: flag("rightHigh")?,
        },
        StrongholdPieceKind::Library => StrongholdPieceExtra::Library {
            tall: flag("Tall")?,
        },
        StrongholdPieceKind::PortalRoom => StrongholdPieceExtra::PortalRoom { mob: flag("Mob")? },
        StrongholdPieceKind::RoomCrossing => StrongholdPieceExtra::RoomCrossing {
            room_type: int("Type")?,
        },
        StrongholdPieceKind::StairsDown | StrongholdPieceKind::Start => {
            StrongholdPieceExtra::StairsDown {
                source: flag("Source")?,
            }
        }
        StrongholdPieceKind::Straight => StrongholdPieceExtra::Straight {
            left: flag("Left")?,
            right: flag("Right")?,
        },
        StrongholdPieceKind::LeftTurn
        | StrongholdPieceKind::RightTurn
        | StrongholdPieceKind::PrisonHall
        | StrongholdPieceKind::StraightStairsDown => StrongholdPieceExtra::None,
    })
}
