//! Vanilla `FossilFeature` (`Feature.FOSSIL`, `Feature.java:72`): a fossil
//! structure template and its ore overlay lowered below the ocean floor.

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use bbb_server_datapack::structure_pool::StructureLiquidSettings;
use bbb_server_datapack::structure_template::StructureTemplate;
use bbb_server_datapack::worldgen::FossilConfig;

use super::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use super::structure_processor::resolve_processor_list;
use super::structure_template_placement::{
    place_materialized_template, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
use super::{block_is, is_empty_block, WorldGenLevel, UPDATE_NONE};

const ZERO: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

/// `Rotation` (`Rotation.java:18-21`) in `values()` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rotation {
    None,
    Clockwise90,
    Clockwise180,
    Counterclockwise90,
}

const ROTATIONS: [Rotation; 4] = [
    Rotation::None,
    Rotation::Clockwise90,
    Rotation::Clockwise180,
    Rotation::Counterclockwise90,
];

impl Rotation {
    /// `Rotation.getRandom` (`Rotation.java:112-114`) through `Util.getRandom`
    /// (`Util.java:742-744`).
    fn get_random(random: &mut dyn RandomSource) -> Rotation {
        ROTATIONS[random.next_int(ROTATIONS.len() as i32) as usize]
    }

    fn jigsaw(self) -> JigsawRotation {
        match self {
            Self::None => JigsawRotation::None,
            Self::Clockwise90 => JigsawRotation::Clockwise90,
            Self::Clockwise180 => JigsawRotation::Clockwise180,
            Self::Counterclockwise90 => JigsawRotation::CounterClockwise90,
        }
    }
}

/// `BoundingBox` (`BoundingBox.java:48-64`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoundingBox {
    min_x: i32,
    min_y: i32,
    min_z: i32,
    max_x: i32,
    max_y: i32,
    max_z: i32,
}

impl BoundingBox {
    /// Inverted bounds are re-sorted (`BoundingBox.java:55-63`).
    fn new(min_x: i32, min_y: i32, min_z: i32, max_x: i32, max_y: i32, max_z: i32) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            min_z: min_z.min(max_z),
            max_x: min_x.max(max_x),
            max_y: min_y.max(max_y),
            max_z: min_z.max(max_z),
        }
    }

    /// `BoundingBox.fromCorners` (`BoundingBox.java:66-75`).
    fn from_corners(pos0: BlockPos, pos1: BlockPos) -> Self {
        Self::new(
            pos0.x.min(pos1.x),
            pos0.y.min(pos1.y),
            pos0.z.min(pos1.z),
            pos0.x.max(pos1.x),
            pos0.y.max(pos1.y),
            pos0.z.max(pos1.z),
        )
    }

    /// `BoundingBox.move` (`BoundingBox.java:184-192`).
    fn moved(self, amount: BlockPos) -> Self {
        Self {
            min_x: self.min_x + amount.x,
            min_y: self.min_y + amount.y,
            min_z: self.min_z + amount.z,
            max_x: self.max_x + amount.x,
            max_y: self.max_y + amount.y,
            max_z: self.max_z + amount.z,
        }
    }

    /// `BoundingBox.forAllCorners` order (`BoundingBox.java:241-251`).
    fn corners(&self) -> [BlockPos; 8] {
        let at = |x, y, z| BlockPos { x, y, z };
        [
            at(self.max_x, self.max_y, self.max_z),
            at(self.min_x, self.max_y, self.max_z),
            at(self.max_x, self.min_y, self.max_z),
            at(self.min_x, self.min_y, self.max_z),
            at(self.max_x, self.max_y, self.min_z),
            at(self.min_x, self.max_y, self.min_z),
            at(self.max_x, self.min_y, self.min_z),
            at(self.min_x, self.min_y, self.min_z),
        ]
    }
}

/// `StructureTemplateManager.getOrCreate` (`StructureTemplateManager.java:83-92`):
/// an unknown id answers `new StructureTemplate()`.
fn get_or_create_template<L: WorldGenLevel>(
    level: &L,
    resource_id: &str,
) -> Option<StructureTemplate> {
    level.structure_template(resource_id)
}

/// `StructureTemplate.getSize(rotation)` (`StructureTemplate.java:518-526`).
fn template_size(template: Option<&StructureTemplate>, rotation: Rotation) -> [i32; 3] {
    let size = template
        .map(|template| template.materialized().size)
        .unwrap_or([0, 0, 0]);
    match rotation {
        Rotation::Counterclockwise90 | Rotation::Clockwise90 => [size[2], size[1], size[0]],
        _ => size,
    }
}

/// `StructureTemplate.transform` with `Mirror.NONE` (`StructureTemplate.java:528-556`).
fn transform(pos: BlockPos, rotation: Rotation, pivot: BlockPos) -> BlockPos {
    let BlockPos { x, y, z } = pos;
    let (pivot_x, pivot_z) = (pivot.x, pivot.z);
    match rotation {
        Rotation::Counterclockwise90 => BlockPos {
            x: pivot_x - pivot_z + z,
            y,
            z: pivot_x + pivot_z - x,
        },
        Rotation::Clockwise90 => BlockPos {
            x: pivot_x + pivot_z - z,
            y,
            z: pivot_z - pivot_x + x,
        },
        Rotation::Clockwise180 => BlockPos {
            x: pivot_x + pivot_x - x,
            y,
            z: pivot_z + pivot_z - z,
        },
        Rotation::None => pos,
    }
}

/// `StructureTemplate.getZeroPositionWithTransform` with `Mirror.NONE`
/// (`StructureTemplate.java:592-613`).
fn zero_position_with_transform(
    zero_pos: BlockPos,
    rotation: Rotation,
    size_x: i32,
    size_z: i32,
) -> BlockPos {
    let size_x = size_x - 1;
    let size_z = size_z - 1;
    match rotation {
        Rotation::Counterclockwise90 => offset(zero_pos, 0, 0, size_x),
        Rotation::Clockwise90 => offset(zero_pos, size_z, 0, 0),
        Rotation::Clockwise180 => offset(zero_pos, size_x, 0, size_z),
        Rotation::None => zero_pos,
    }
}

/// `StructureTemplate.getBoundingBox` with `Mirror.NONE`
/// (`StructureTemplate.java:624-629`).
fn template_bounding_box(
    position: BlockPos,
    rotation: Rotation,
    pivot: BlockPos,
    size: [i32; 3],
) -> BoundingBox {
    let corner1 = transform(ZERO, rotation, pivot);
    let corner2 = transform(
        BlockPos {
            x: size[0] - 1,
            y: size[1] - 1,
            z: size[2] - 1,
        },
        rotation,
        pivot,
    );
    BoundingBox::from_corners(corner1, corner2).moved(position)
}

fn offset(pos: BlockPos, dx: i32, dy: i32, dz: i32) -> BlockPos {
    BlockPos {
        x: pos.x + dx,
        y: pos.y + dy,
        z: pos.z + dz,
    }
}

/// `FossilFeature.place` (`FossilFeature.java:26-69`).
pub fn place_fossil<L: WorldGenLevel>(
    config: &FossilConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    let rotation = Rotation::get_random(random);
    let fossil_index = random.next_int(config.fossil_structures.len() as i32) as usize;
    let fossil_base = get_or_create_template(level, &config.fossil_structures[fossil_index]);
    let fossil_overlay = get_or_create_template(level, &config.overlay_structures[fossil_index]);
    // `ChunkPos.containing` and its block bounds (`ChunkPos.java:48-50`,
    // `:111-125`; `SectionPos.java:80-82`, `:127-133`).
    let chunk_min_x = (origin.x >> 4) << 4;
    let chunk_min_z = (origin.z >> 4) << 4;
    let bounding_box = BoundingBox::new(
        chunk_min_x - 16,
        level.get_min_y(),
        chunk_min_z - 16,
        chunk_min_x + 15 + 16,
        level.get_max_y(),
        chunk_min_z + 15 + 16,
    );
    let base_size = fossil_base
        .as_ref()
        .map(|template| template.materialized().size)
        .unwrap_or([0, 0, 0]);
    let size = template_size(fossil_base.as_ref(), rotation);
    let low_corner = offset(origin, -size[0] / 2, 0, -size[2] / 2);
    let mut lowest_surface_y = origin.y;
    for xscan in 0..size[0] {
        for zscan in 0..size[2] {
            lowest_surface_y = lowest_surface_y
                .min(level.get_ocean_floor_wg_height(low_corner.x + xscan, low_corner.z + zscan));
        }
    }
    let target_y = (lowest_surface_y - 15 - random.next_int(10)).max(level.get_min_y() + 10);
    let target_pos = zero_position_with_transform(
        BlockPos {
            x: low_corner.x,
            y: target_y,
            z: low_corner.z,
        },
        rotation,
        base_size[0],
        base_size[2],
    );
    if count_empty_corners(
        level,
        template_bounding_box(target_pos, rotation, ZERO, base_size),
    ) > config.max_empty_corners_allowed
    {
        return Some(false);
    }
    let placement_bounds = JigsawBoundingBox {
        min: BlockPos {
            x: bounding_box.min_x,
            y: bounding_box.min_y,
            z: bounding_box.min_z,
        },
        max: BlockPos {
            x: bounding_box.max_x,
            y: bounding_box.max_y,
            z: bounding_box.max_z,
        },
    };
    let processors = resolve_processor_list(&config.fossil_processors, level)?;
    if let Some(template) = &fossil_base {
        place_materialized_template(
            template.materialized(),
            level,
            MaterializedTemplatePlacement {
                position: target_pos,
                reference_pos: target_pos,
                rotation: rotation.jigsaw(),
                mirror: crate::noise::ruined_portal::RuinedPortalMirror::None,
                pivot: BlockPos { x: 0, y: 0, z: 0 },
                finalize_entities: false,
                ignore_entities: false,
                bounding_box: placement_bounds,
                processors: &processors.0,
                liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
                update_mode: UPDATE_NONE,
                known_shape: false,
            },
            random,
            TemplateSettingsRandom::Shared,
        );
    }
    let processors = resolve_processor_list(&config.overlay_processors, level)?;
    if let Some(template) = &fossil_overlay {
        place_materialized_template(
            template.materialized(),
            level,
            MaterializedTemplatePlacement {
                position: target_pos,
                reference_pos: target_pos,
                rotation: rotation.jigsaw(),
                mirror: crate::noise::ruined_portal::RuinedPortalMirror::None,
                pivot: BlockPos { x: 0, y: 0, z: 0 },
                finalize_entities: false,
                ignore_entities: false,
                bounding_box: placement_bounds,
                processors: &processors.0,
                liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
                update_mode: UPDATE_NONE,
                known_shape: false,
            },
            random,
            TemplateSettingsRandom::Shared,
        );
    }
    Some(true)
}

/// `FossilFeature.countEmptyCorners` (`FossilFeature.java:71-80`).
fn count_empty_corners<L: WorldGenLevel>(level: &L, structure_bounds: BoundingBox) -> i32 {
    structure_bounds
        .corners()
        .into_iter()
        .filter(|pos| {
            let state = level.get_block(*pos);
            is_empty_block(state)
                || block_is(state, "minecraft:lava")
                || block_is(state, "minecraft:water")
        })
        .count() as i32
}
