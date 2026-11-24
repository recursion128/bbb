//! Typed generation and persistence for ruined-portal StructureStarts.

#![allow(
    dead_code,
    reason = "typed components remain inputs to the later world-level status owner"
)]

use bbb_protocol::packets::BlockPos;
use bbb_server_datapack::worldgen::structure::DecodedRuinedPortalSetup;

use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox, JigsawRotation};
use super::structures::StructureTerrainAdjustment;

pub mod persistence;
pub mod placement;
pub mod start;

pub use placement::RuinedPortalChunkPlacement;

pub const RUINED_PORTAL_TYPE_ID: &str = "minecraft:ruined_portal";
pub const RUINED_PORTAL_SET_ID: &str = "minecraft:ruined_portals";
pub const RUINED_PORTAL_STANDARD_ID: &str = "minecraft:ruined_portal";
pub const RUINED_PORTAL_DESERT_ID: &str = "minecraft:ruined_portal_desert";
pub const RUINED_PORTAL_JUNGLE_ID: &str = "minecraft:ruined_portal_jungle";
pub const RUINED_PORTAL_SWAMP_ID: &str = "minecraft:ruined_portal_swamp";
pub const RUINED_PORTAL_MOUNTAIN_ID: &str = "minecraft:ruined_portal_mountain";
pub const RUINED_PORTAL_OCEAN_ID: &str = "minecraft:ruined_portal_ocean";
pub const RUINED_PORTAL_NETHER_ID: &str = "minecraft:ruined_portal_nether";

pub fn ruined_portal_structure_id(id: &str) -> Option<&'static str> {
    Some(match id {
        RUINED_PORTAL_STANDARD_ID => RUINED_PORTAL_STANDARD_ID,
        RUINED_PORTAL_DESERT_ID => RUINED_PORTAL_DESERT_ID,
        RUINED_PORTAL_JUNGLE_ID => RUINED_PORTAL_JUNGLE_ID,
        RUINED_PORTAL_SWAMP_ID => RUINED_PORTAL_SWAMP_ID,
        RUINED_PORTAL_MOUNTAIN_ID => RUINED_PORTAL_MOUNTAIN_ID,
        RUINED_PORTAL_OCEAN_ID => RUINED_PORTAL_OCEAN_ID,
        RUINED_PORTAL_NETHER_ID => RUINED_PORTAL_NETHER_ID,
        _ => return None,
    })
}
/// `StructurePieceType.java:62,86-96` lowercases the registered `RUPO` id.
pub const RUINED_PORTAL_PIECE_ID: &str = "minecraft:rupo";

/// `RuinedPortalStructure.java:34-45`.
pub const PORTAL_TEMPLATES: &[&str] = &[
    "minecraft:ruined_portal/portal_1",
    "minecraft:ruined_portal/portal_2",
    "minecraft:ruined_portal/portal_3",
    "minecraft:ruined_portal/portal_4",
    "minecraft:ruined_portal/portal_5",
    "minecraft:ruined_portal/portal_6",
    "minecraft:ruined_portal/portal_7",
    "minecraft:ruined_portal/portal_8",
    "minecraft:ruined_portal/portal_9",
    "minecraft:ruined_portal/portal_10",
];

/// `RuinedPortalStructure.java:46-48`.
pub const GIANT_PORTAL_TEMPLATES: &[&str] = &[
    "minecraft:ruined_portal/giant_portal_1",
    "minecraft:ruined_portal/giant_portal_2",
    "minecraft:ruined_portal/giant_portal_3",
];

/// `RuinedPortalStructure.java:49`.
pub const GIANT_PORTAL_PROBABILITY: f32 = 0.05;
/// `RuinedPortalStructure.java:50`.
pub const MIN_Y_INDEX: i32 = 15;

/// Bit-exact f32 so settings and piece mossiness stay `Eq`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuinedPortalFloat {
    bits: u32,
}

impl RuinedPortalFloat {
    pub const fn from_f32(value: f32) -> Self {
        Self {
            bits: value.to_bits(),
        }
    }

    pub const fn get(self) -> f32 {
        f32::from_bits(self.bits)
    }

    pub const fn bits(self) -> u32 {
        self.bits
    }
}

/// `RuinedPortalPiece.VerticalPlacement` (`RuinedPortalPiece.java:329-351`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuinedPortalVerticalPlacement {
    OnLandSurface,
    PartlyBuried,
    OnOceanFloor,
    InMountain,
    Underground,
    InNether,
}

impl RuinedPortalVerticalPlacement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OnLandSurface => "on_land_surface",
            Self::PartlyBuried => "partly_buried",
            Self::OnOceanFloor => "on_ocean_floor",
            Self::InMountain => "in_mountain",
            Self::Underground => "underground",
            Self::InNether => "in_nether",
        }
    }

    pub fn from_serialized_name(value: &str) -> Option<Self> {
        match value {
            "on_land_surface" => Some(Self::OnLandSurface),
            "partly_buried" => Some(Self::PartlyBuried),
            "on_ocean_floor" => Some(Self::OnOceanFloor),
            "in_mountain" => Some(Self::InMountain),
            "underground" => Some(Self::Underground),
            "in_nether" => Some(Self::InNether),
            _ => None,
        }
    }
}

/// `Mirror` used by ruined-portal place settings (`RuinedPortalStructure.java:110`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuinedPortalMirror {
    None,
    LeftRight,
    FrontBack,
}

impl RuinedPortalMirror {
    pub const fn legacy_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::LeftRight => "LEFT_RIGHT",
            Self::FrontBack => "FRONT_BACK",
        }
    }

    pub fn from_legacy_name(value: &str) -> Option<Self> {
        match value {
            "NONE" => Some(Self::None),
            "LEFT_RIGHT" => Some(Self::LeftRight),
            "FRONT_BACK" => Some(Self::FrontBack),
            _ => None,
        }
    }
}

/// `RuinedPortalStructure.Setup` (`RuinedPortalStructure.java:237-258`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuinedPortalSetup {
    pub placement: RuinedPortalVerticalPlacement,
    pub air_pocket_probability: RuinedPortalFloat,
    pub mossiness: RuinedPortalFloat,
    pub overgrown: bool,
    pub vines: bool,
    pub can_be_cold: bool,
    pub replace_with_blackstone: bool,
    pub weight: RuinedPortalFloat,
}

/// `RuinedPortalPiece.Properties` (`RuinedPortalPiece.java:295-326`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuinedPortalProperties {
    pub cold: bool,
    pub mossiness: RuinedPortalFloat,
    pub air_pocket: bool,
    pub overgrown: bool,
    pub vines: bool,
    pub replace_with_blackstone: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuinedPortalStructureSettings {
    pub setups: Vec<RuinedPortalSetup>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuinedPortalPiece {
    template: String,
    template_position: BlockPos,
    rotation: JigsawRotation,
    mirror: RuinedPortalMirror,
    vertical_placement: RuinedPortalVerticalPlacement,
    properties: RuinedPortalProperties,
    template_size: [i32; 3],
}

impl RuinedPortalPiece {
    pub fn new(
        template: impl Into<String>,
        template_position: BlockPos,
        rotation: JigsawRotation,
        mirror: RuinedPortalMirror,
        vertical_placement: RuinedPortalVerticalPlacement,
        properties: RuinedPortalProperties,
    ) -> Self {
        Self {
            template: template.into(),
            template_position,
            rotation,
            mirror,
            vertical_placement,
            properties,
            template_size: [0, 0, 0],
        }
    }

    pub fn with_template_size(mut self, size: [i32; 3]) -> Self {
        self.template_size = size;
        self
    }

    pub const fn template_size(&self) -> [i32; 3] {
        self.template_size
    }

    pub fn set_template_size(&mut self, size: [i32; 3]) {
        self.template_size = size;
    }

    pub fn template(&self) -> &str {
        &self.template
    }

    pub const fn template_position(&self) -> BlockPos {
        self.template_position
    }

    pub const fn rotation(&self) -> JigsawRotation {
        self.rotation
    }

    pub const fn mirror(&self) -> RuinedPortalMirror {
        self.mirror
    }

    pub const fn vertical_placement(&self) -> RuinedPortalVerticalPlacement {
        self.vertical_placement
    }

    pub const fn properties(&self) -> RuinedPortalProperties {
        self.properties
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuinedPortalGenerationStub {
    source_chunk: [i32; 2],
    generation_anchor: BlockPos,
    structure_id: String,
    piece: RuinedPortalPiece,
}

impl RuinedPortalGenerationStub {
    pub fn new(
        source_chunk: [i32; 2],
        generation_anchor: BlockPos,
        structure_id: impl Into<String>,
        piece: RuinedPortalPiece,
    ) -> Self {
        Self {
            source_chunk,
            generation_anchor,
            structure_id: structure_id.into(),
            piece,
        }
    }

    pub const fn source_chunk(&self) -> [i32; 2] {
        self.source_chunk
    }

    pub const fn generation_anchor(&self) -> BlockPos {
        self.generation_anchor
    }

    pub fn structure_id(&self) -> &str {
        &self.structure_id
    }

    pub fn piece(&self) -> &RuinedPortalPiece {
        &self.piece
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuinedPortalStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    piece: RuinedPortalPiece,
    cached_bounding_box: CachedStartBoundingBox,
}

impl RuinedPortalStructureStart {
    pub fn from_generation_stub(stub: RuinedPortalGenerationStub, references: i32) -> Self {
        Self {
            structure_id: stub.structure_id,
            start_chunk: stub.source_chunk,
            references,
            terrain_adjustment: StructureTerrainAdjustment::None,
            piece: stub.piece,
            cached_bounding_box: CachedStartBoundingBox::new(),
        }
    }

    pub fn from_parts(
        structure_id: impl Into<String>,
        start_chunk: [i32; 2],
        references: i32,
        piece: RuinedPortalPiece,
    ) -> Self {
        let structure_id = structure_id.into();
        Self {
            structure_id,
            start_chunk,
            references,
            terrain_adjustment: StructureTerrainAdjustment::None,
            piece,
            cached_bounding_box: CachedStartBoundingBox::new(),
        }
    }

    pub fn with_terrain_adjustment(
        mut self,
        terrain_adjustment: StructureTerrainAdjustment,
    ) -> Self {
        self.terrain_adjustment = terrain_adjustment;
        self.cached_bounding_box.invalidate();
        self
    }

    pub const fn terrain_adjustment(&self) -> StructureTerrainAdjustment {
        self.terrain_adjustment
    }

    pub fn structure_id(&self) -> &str {
        &self.structure_id
    }

    pub const fn start_chunk(&self) -> [i32; 2] {
        self.start_chunk
    }

    pub const fn references(&self) -> i32 {
        self.references
    }

    pub(super) fn try_add_reference(&mut self) -> bool {
        if self.references >= 1 {
            return false;
        }
        self.references = self.references.wrapping_add(1);
        true
    }

    pub fn piece(&self) -> &RuinedPortalPiece {
        &self.piece
    }

    pub fn piece_mut(&mut self) -> &mut RuinedPortalPiece {
        self.cached_bounding_box.invalidate();
        &mut self.piece
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                let size = self.piece.template_size();
                let position = self.piece.template_position();
                let raw = if size == [0, 0, 0] {
                    JigsawBoundingBox::point(position)
                } else {
                    let pivot = BlockPos {
                        x: size[0] / 2,
                        y: 0,
                        z: size[2] / 2,
                    };
                    JigsawBoundingBox::from_array(template_bounding_box(
                        position,
                        self.piece.rotation(),
                        pivot,
                        self.piece.mirror(),
                        size,
                    ))
                };
                Some(raw.adjusted_for_terrain(self.terrain_adjustment))
            })
            .expect("ruined portal start owns one piece")
    }
}

/// `RuinedPortalStructure.CODEC` setups (`RuinedPortalStructure.java:52-55,237-258`).
pub fn ruined_portal_settings_from_decoded(
    decoded: &[DecodedRuinedPortalSetup],
) -> Option<RuinedPortalStructureSettings> {
    let setups = decoded
        .iter()
        .map(|setup| {
            Some(RuinedPortalSetup {
                placement: RuinedPortalVerticalPlacement::from_serialized_name(&setup.placement)?,
                air_pocket_probability: RuinedPortalFloat::from_f32(setup.air_pocket_probability),
                mossiness: RuinedPortalFloat::from_f32(setup.mossiness),
                overgrown: setup.overgrown,
                vines: setup.vines,
                can_be_cold: setup.can_be_cold,
                replace_with_blackstone: setup.replace_with_blackstone,
                weight: RuinedPortalFloat::from_f32(setup.weight),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    (!setups.is_empty()).then_some(RuinedPortalStructureSettings { setups })
}

/// `StructureTemplate.transform` (`StructureTemplate.java:528-555`).
pub fn transform(
    pos: BlockPos,
    mirror: RuinedPortalMirror,
    rotation: JigsawRotation,
    pivot: BlockPos,
) -> BlockPos {
    let mut x = pos.x;
    let mut z = pos.z;
    let mut was_mirrored = true;
    match mirror {
        RuinedPortalMirror::LeftRight => z = z.wrapping_neg(),
        RuinedPortalMirror::FrontBack => x = x.wrapping_neg(),
        RuinedPortalMirror::None => was_mirrored = false,
    }
    match rotation {
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: pivot.x.wrapping_sub(pivot.z).wrapping_add(z),
            y: pos.y,
            z: pivot.x.wrapping_add(pivot.z).wrapping_sub(x),
        },
        JigsawRotation::Clockwise90 => BlockPos {
            x: pivot.x.wrapping_add(pivot.z).wrapping_sub(z),
            y: pos.y,
            z: pivot.z.wrapping_sub(pivot.x).wrapping_add(x),
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: pivot.x.wrapping_add(pivot.x).wrapping_sub(x),
            y: pos.y,
            z: pivot.z.wrapping_add(pivot.z).wrapping_sub(z),
        },
        JigsawRotation::None => {
            if was_mirrored {
                BlockPos { x, y: pos.y, z }
            } else {
                pos
            }
        }
    }
}

/// `StructureTemplate.getBoundingBox` (`StructureTemplate.java:624-628`).
pub fn template_bounding_box(
    position: BlockPos,
    rotation: JigsawRotation,
    pivot: BlockPos,
    mirror: RuinedPortalMirror,
    size: [i32; 3],
) -> [i32; 6] {
    let corner1 = transform(BlockPos { x: 0, y: 0, z: 0 }, mirror, rotation, pivot);
    let corner2 = transform(
        BlockPos {
            x: size[0] - 1,
            y: size[1] - 1,
            z: size[2] - 1,
        },
        mirror,
        rotation,
        pivot,
    );
    [
        corner1.x.min(corner2.x).wrapping_add(position.x),
        corner1.y.min(corner2.y).wrapping_add(position.y),
        corner1.z.min(corner2.z).wrapping_add(position.z),
        corner1.x.max(corner2.x).wrapping_add(position.x),
        corner1.y.max(corner2.y).wrapping_add(position.y),
        corner1.z.max(corner2.z).wrapping_add(position.z),
    ]
}

pub fn known_ruined_portal_template(template: &str) -> bool {
    PORTAL_TEMPLATES.contains(&template) || GIANT_PORTAL_TEMPLATES.contains(&template)
}

#[cfg(test)]
mod tests {
    use crate::noise::biome::NoiseBiomeSource;
    use crate::noise::structures::NoiseStructurePlan;
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    fn startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    fn overworld_plan(startup: &DatapackStartup) -> NoiseStructurePlan {
        let source = NoiseBiomeSource::from_registry(
            &startup.registries,
            "minecraft:overworld",
            "minecraft:overworld",
            0,
        )
        .unwrap();
        NoiseStructurePlan::from_startup(startup, source.overworld_biome_order().unwrap()).unwrap()
    }

    #[test]
    fn built_in_ruined_portal_variants_keep_exact_setups() {
        let plan = overworld_plan(&startup());
        let expected = [
            (
                "minecraft:ruined_portal_desert",
                RuinedPortalVerticalPlacement::PartlyBuried,
                0.0,
                0.0,
                false,
                false,
                false,
            ),
            (
                "minecraft:ruined_portal_jungle",
                RuinedPortalVerticalPlacement::OnLandSurface,
                0.5,
                0.8,
                true,
                true,
                false,
            ),
            (
                "minecraft:ruined_portal_swamp",
                RuinedPortalVerticalPlacement::OnOceanFloor,
                0.0,
                0.5,
                false,
                true,
                false,
            ),
            (
                "minecraft:ruined_portal_ocean",
                RuinedPortalVerticalPlacement::OnOceanFloor,
                0.0,
                0.8,
                false,
                false,
                true,
            ),
            (
                "minecraft:ruined_portal_nether",
                RuinedPortalVerticalPlacement::InNether,
                0.5,
                0.0,
                false,
                false,
                false,
            ),
        ];
        for (id, placement, air, moss, overgrown, vines, can_be_cold) in expected {
            let settings = plan.structures()[id].ruined_portal.as_ref().unwrap();
            assert_eq!(settings.setups.len(), 1, "{id}");
            assert_eq!(settings.setups[0].placement, placement, "{id}");
            assert_eq!(settings.setups[0].air_pocket_probability.get(), air, "{id}");
            assert_eq!(settings.setups[0].mossiness.get(), moss, "{id}");
            assert_eq!(settings.setups[0].overgrown, overgrown, "{id}");
            assert_eq!(settings.setups[0].vines, vines, "{id}");
            assert_eq!(settings.setups[0].can_be_cold, can_be_cold, "{id}");
        }
        let standard = plan.structures()["minecraft:ruined_portal"]
            .ruined_portal
            .as_ref()
            .unwrap();
        assert_eq!(standard.setups.len(), 2);
        assert_eq!(
            standard.setups[0].placement,
            RuinedPortalVerticalPlacement::Underground
        );
        assert_eq!(
            standard.setups[1].placement,
            RuinedPortalVerticalPlacement::OnLandSurface
        );
        let mountain = plan.structures()["minecraft:ruined_portal_mountain"]
            .ruined_portal
            .as_ref()
            .unwrap();
        assert_eq!(mountain.setups.len(), 2);
        assert_eq!(
            mountain.setups[0].placement,
            RuinedPortalVerticalPlacement::InMountain
        );
        assert!(plan
            .structures()
            .values()
            .filter(|definition| definition.type_id != RUINED_PORTAL_TYPE_ID)
            .all(|definition| definition.ruined_portal.is_none()));
    }
}
