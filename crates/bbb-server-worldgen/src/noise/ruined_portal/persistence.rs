//! Typed Java 26.1 load support for ruined-portal starts.

use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;

use super::{
    template_bounding_box, RuinedPortalMirror, RuinedPortalPiece, RuinedPortalProperties,
    RuinedPortalStructureStart, RuinedPortalVerticalPlacement, RUINED_PORTAL_PIECE_ID,
};
use crate::noise::structure_nbt::{
    byte, compound, entry, int, rotation, rotation_name, start_to_nbt, string, NbtFields, NbtRead,
};

impl RuinedPortalStructureStart {
    pub fn to_nbt(&self) -> NbtSummaryValue {
        start_to_nbt(
            self.structure_id(),
            self.start_chunk(),
            self.references(),
            vec![piece_to_nbt(self.piece())],
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

fn piece_to_nbt(piece: &RuinedPortalPiece) -> NbtSummaryValue {
    let position = piece.template_position();
    let size = piece.template_size();
    let bounds = if size == [0, 0, 0] {
        [
            position.x, position.y, position.z, position.x, position.y, position.z,
        ]
    } else {
        let pivot = BlockPos {
            x: size[0] / 2,
            y: 0,
            z: size[2] / 2,
        };
        template_bounding_box(position, piece.rotation(), pivot, piece.mirror(), size)
    };
    let properties = piece.properties();
    compound(vec![
        string("id", RUINED_PORTAL_PIECE_ID),
        entry("BB", NbtSummaryValue::IntArray(bounds.to_vec())),
        int("O", 2),
        int("GD", 0),
        int("TPX", position.x),
        int("TPY", position.y),
        int("TPZ", position.z),
        string("Template", piece.template()),
        string("Rotation", rotation_name(piece.rotation())),
        string("Mirror", piece.mirror().legacy_name()),
        string("VerticalPlacement", piece.vertical_placement().as_str()),
        entry(
            "Properties",
            compound(vec![
                byte("cold", i8::from(properties.cold)),
                entry(
                    "mossiness",
                    NbtSummaryValue::Float(properties.mossiness.bits()),
                ),
                byte("air_pocket", i8::from(properties.air_pocket)),
                byte("overgrown", i8::from(properties.overgrown)),
                byte("vines", i8::from(properties.vines)),
                byte(
                    "replace_with_blackstone",
                    i8::from(properties.replace_with_blackstone),
                ),
            ]),
        ),
    ])
}

fn piece_from_nbt(value: &NbtSummaryValue) -> Option<RuinedPortalPiece> {
    let fields = value.compound()?;
    let properties = fields.field("Properties")?.compound()?;
    let flag = |name: &str| properties.field(name)?.boolean();
    Some(RuinedPortalPiece::new(
        fields.field("Template")?.resource_id()?,
        BlockPos {
            x: fields.field("TPX")?.int()?,
            y: fields.field("TPY")?.int()?,
            z: fields.field("TPZ")?.int()?,
        },
        rotation(fields.field("Rotation")?)?,
        RuinedPortalMirror::from_legacy_name(&fields.field("Mirror")?.string()?)?,
        RuinedPortalVerticalPlacement::from_serialized_name(
            &fields.field("VerticalPlacement")?.string()?,
        )?,
        RuinedPortalProperties {
            cold: flag("cold")?,
            mossiness: super::RuinedPortalFloat::from_f32(properties.field("mossiness")?.float()?),
            air_pocket: flag("air_pocket")?,
            overgrown: flag("overgrown")?,
            vines: flag("vines")?,
            replace_with_blackstone: flag("replace_with_blackstone")?,
        },
    ))
}
