//! Typed Java 26.1 load support for scattered-feature starts.

use bbb_protocol::NbtSummaryValue;

use super::{
    HorizontalDirection, ScatteredFeatureExtra, ScatteredFeatureKind, ScatteredFeaturePiece,
    ScatteredFeatureStructureStart,
};
use crate::noise::structure_nbt::{
    bounding_box, byte, compound, entry, int, start_to_nbt, string, NbtFields, NbtRead,
};

impl ScatteredFeatureStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        start_to_nbt(
            self.structure_id(),
            self.start_chunk(),
            self.references(),
            vec![piece_to_nbt(self.piece())],
        )
    }

    pub fn from_nbt(
        outer_id: &str,
        value: &NbtSummaryValue,
        kind: ScatteredFeatureKind,
    ) -> Option<Self> {
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
            piece_from_nbt(piece, kind)?,
        ))
    }

    pub fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.bounding_box().intersects_chunk(chunk_x, chunk_z)
    }
}

fn piece_to_nbt(piece: &ScatteredFeaturePiece) -> NbtSummaryValue {
    let mut fields = vec![
        string("id", piece.kind().piece_id()),
        entry(
            "BB",
            NbtSummaryValue::IntArray(piece.bounding_box().to_vec()),
        ),
        int("O", piece.orientation().to_2d()),
        int("GD", 0),
        int("Width", piece.width()),
        int("Height", piece.height()),
        int("Depth", piece.depth()),
        int("HPos", piece.height_position()),
    ];
    match piece.extra() {
        ScatteredFeatureExtra::DesertPyramid {
            has_placed_chest0,
            has_placed_chest1,
            has_placed_chest2,
            has_placed_chest3,
        } => {
            fields.push(byte("hasPlacedChest0", i8::from(has_placed_chest0)));
            fields.push(byte("hasPlacedChest1", i8::from(has_placed_chest1)));
            fields.push(byte("hasPlacedChest2", i8::from(has_placed_chest2)));
            fields.push(byte("hasPlacedChest3", i8::from(has_placed_chest3)));
        }
        ScatteredFeatureExtra::JungleTemple {
            placed_main_chest,
            placed_hidden_chest,
            placed_trap1,
            placed_trap2,
        } => {
            fields.push(byte("placedMainChest", i8::from(placed_main_chest)));
            fields.push(byte("placedHiddenChest", i8::from(placed_hidden_chest)));
            fields.push(byte("placedTrap1", i8::from(placed_trap1)));
            fields.push(byte("placedTrap2", i8::from(placed_trap2)));
        }
        ScatteredFeatureExtra::SwampHut { witch, cat } => {
            fields.push(byte("Witch", i8::from(witch)));
            fields.push(byte("Cat", i8::from(cat)));
        }
    }
    compound(fields)
}

fn piece_from_nbt(
    value: &NbtSummaryValue,
    kind: ScatteredFeatureKind,
) -> Option<ScatteredFeaturePiece> {
    let fields = value.compound()?;
    let int = |name: &str| fields.field(name)?.int();
    let flag = |name: &str| fields.field(name)?.boolean();
    let bounds = bounding_box(fields.field("BB")?)?;
    let extra = match kind {
        ScatteredFeatureKind::DesertPyramid => ScatteredFeatureExtra::DesertPyramid {
            has_placed_chest0: flag("hasPlacedChest0")?,
            has_placed_chest1: flag("hasPlacedChest1")?,
            has_placed_chest2: flag("hasPlacedChest2")?,
            has_placed_chest3: flag("hasPlacedChest3")?,
        },
        ScatteredFeatureKind::JungleTemple => ScatteredFeatureExtra::JungleTemple {
            placed_main_chest: flag("placedMainChest")?,
            placed_hidden_chest: flag("placedHiddenChest")?,
            placed_trap1: flag("placedTrap1")?,
            placed_trap2: flag("placedTrap2")?,
        },
        ScatteredFeatureKind::SwampHut => ScatteredFeatureExtra::SwampHut {
            witch: flag("Witch")?,
            cat: flag("Cat")?,
        },
    };
    Some(ScatteredFeaturePiece::new(
        kind,
        int("Width")?,
        int("Height")?,
        int("Depth")?,
        int("HPos")?,
        HorizontalDirection::from_2d(int("O")?)?,
        [
            bounds.min.x,
            bounds.min.y,
            bounds.min.z,
            bounds.max.x,
            bounds.max.y,
            bounds.max.z,
        ],
        extra,
    ))
}
