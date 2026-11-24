//! Typed End City starts, recursive template pieces, placement, and persistence.

use bbb_protocol::packets::BlockPos;

use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox, JigsawRotation};
use super::structures::StructureTerrainAdjustment;

pub mod persistence;
pub mod pieces;
pub mod placement;
pub mod start;
pub use placement::EndCityChunkPlacement;

pub const END_CITY_ID: &str = "minecraft:end_city";
pub const END_CITY_SET_ID: &str = "minecraft:end_cities";
pub const END_CITY_TYPE_ID: &str = "minecraft:end_city";
pub const END_CITY_PIECE_ID: &str = "minecraft:ecp";
pub const END_CITY_LOOT: &str = "minecraft:chests/end_city_treasure";
pub const END_CITY_TEMPLATE_PREFIX: &str = "minecraft:end_city/";

pub const END_CITY_TEMPLATES: [&str; 20] = [
    "base_floor",
    "base_roof",
    "bridge_end",
    "bridge_gentle_stairs",
    "bridge_piece",
    "bridge_steep_stairs",
    "fat_tower_base",
    "fat_tower_middle",
    "fat_tower_top",
    "second_floor_1",
    "second_floor_2",
    "second_roof",
    "ship",
    "third_floor_1",
    "third_floor_2",
    "third_roof",
    "tower_base",
    "tower_floor",
    "tower_piece",
    "tower_top",
];

pub fn qualify_template(short_name: &str) -> String {
    format!("{END_CITY_TEMPLATE_PREFIX}{short_name}")
}

pub fn short_template_name(template: &str) -> Option<&str> {
    template.strip_prefix(END_CITY_TEMPLATE_PREFIX)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndCityPiece {
    template: String,
    template_position: BlockPos,
    rotation: JigsawRotation,
    overwrite: bool,
    gen_depth: i32,
    template_size: [i32; 3],
}

impl EndCityPiece {
    pub fn new(
        template: impl Into<String>,
        template_position: BlockPos,
        rotation: JigsawRotation,
        overwrite: bool,
        template_size: [i32; 3],
    ) -> Self {
        Self {
            template: template.into(),
            template_position,
            rotation,
            overwrite,
            gen_depth: 0,
            template_size,
        }
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

    pub const fn overwrite(&self) -> bool {
        self.overwrite
    }

    pub const fn gen_depth(&self) -> i32 {
        self.gen_depth
    }

    pub fn set_gen_depth(&mut self, gen_depth: i32) {
        self.gen_depth = gen_depth;
    }

    pub const fn template_size(&self) -> [i32; 3] {
        self.template_size
    }

    pub fn set_template_size(&mut self, size: [i32; 3]) {
        self.template_size = size;
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        piece_bounding_box(self.template_position, self.rotation, self.template_size)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndCityStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    pieces: Vec<EndCityPiece>,
    cached_bounding_box: CachedStartBoundingBox,
}

impl EndCityStructureStart {
    pub fn from_parts(
        structure_id: impl Into<String>,
        start_chunk: [i32; 2],
        references: i32,
        pieces: Vec<EndCityPiece>,
    ) -> Self {
        Self {
            structure_id: structure_id.into(),
            start_chunk,
            references,
            terrain_adjustment: StructureTerrainAdjustment::None,
            pieces,
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

    pub fn pieces(&self) -> &[EndCityPiece] {
        &self.pieces
    }

    pub fn pieces_mut(&mut self) -> &mut [EndCityPiece] {
        self.cached_bounding_box.invalidate();
        &mut self.pieces
    }

    pub fn bounding_box(&self) -> Option<JigsawBoundingBox> {
        self.cached_bounding_box.get_or_init(|| {
            JigsawBoundingBox::adjusted_union(
                self.pieces.iter().map(EndCityPiece::bounding_box),
                self.terrain_adjustment,
            )
        })
    }

    pub fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.bounding_box()
            .is_some_and(|bounds| bounds.intersects_chunk(chunk_x, chunk_z))
    }
}

pub fn piece_bounding_box(
    position: BlockPos,
    rotation: JigsawRotation,
    size: [i32; 3],
) -> JigsawBoundingBox {
    let delta = BlockPos {
        x: size[0] - 1,
        y: size[1] - 1,
        z: size[2] - 1,
    };
    let rotated = super::woodland_mansion::direction::rotate_block_pos(delta, rotation);
    let far = BlockPos {
        x: position.x + rotated.x,
        y: position.y + rotated.y,
        z: position.z + rotated.z,
    };
    JigsawBoundingBox {
        min: BlockPos {
            x: position.x.min(far.x),
            y: position.y.min(far.y),
            z: position.z.min(far.z),
        },
        max: BlockPos {
            x: position.x.max(far.x),
            y: position.y.max(far.y),
            z: position.z.max(far.z),
        },
    }
}
