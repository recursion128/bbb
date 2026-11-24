//! `RuinedPortalPiece` template placement plus netherrack/vine/leaf extras.

use std::sync::Arc;

use bbb_protocol::block_state_types::{
    vanilla_block_state_support_facts, VanillaBlockSupportDirection,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::structure_pool::StructureLiquidSettings;

use crate::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use crate::structure_processor::{
    block_match_replace, random_block_match_replace, ProcessorRule, StructureProcessor,
};
use crate::structure_template_placement::{
    place_materialized_template_with_palette, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
use crate::{block_is, block_set_property, catalog_default_state, WorldGenLevel};
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplateSet,
};
use bbb_server_types::block_state_parse::parse_block_state_prefix;

use super::{
    template_bounding_box, transform, RuinedPortalMirror, RuinedPortalPiece,
    RuinedPortalProperties, RuinedPortalStructureStart, RuinedPortalVerticalPlacement,
};

/// `Block.UPDATE_CLIENTS` (`Block.java:92`).
const UPDATE_CLIENTS: i32 = 2;
/// `Block.UPDATE_ALL` (`Block.java:104`), netherrack/vine/leaf extras.
const UPDATE_ALL: i32 = 3;
const GOLD_GONE: f32 = 0.3;
const MAGMA_FOR_NETHERRACK: f32 = 0.07;
const MAGMA_FOR_LAVA: f32 = 0.2;
const FEATURES_CANNOT_REPLACE: &str = "minecraft:features_cannot_replace";
const NETHERRACK_DISTANCE: [f32; 14] = [
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.9, 0.8, 0.7, 0.6, 0.4, 0.2,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuinedPortalChunkPlacement {
    pub attempted: bool,
    pub placed: bool,
}

impl RuinedPortalStructureStart {
    /// `StructureStart.placeInChunk` then `RuinedPortalPiece.postProcess`
    /// (`StructureStart.java:93-96`, `RuinedPortalPiece.java:156-173`,
    /// `TemplateStructurePiece.java:91-117`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        templates: &StructureTemplateSet,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> RuinedPortalChunkPlacement {
        let template_id = self.piece.template().to_string();
        let live_template = level.structure_template(&template_id);
        let Some(template) = live_template.as_ref().or_else(|| {
            templates
                .template(&template_id)
                .map(|element| &element.template)
        }) else {
            return RuinedPortalChunkPlacement::default();
        };
        let template = template.materialized();
        let piece_bounds = piece_template_bounds(&self.piece, template.size);
        if !piece_bounds.intersects(chunk_bounds) {
            return RuinedPortalChunkPlacement::default();
        }
        if !chunk_bounds.contains(bounding_box_center(piece_bounds)) {
            return RuinedPortalChunkPlacement {
                attempted: true,
                placed: false,
            };
        }
        let writable = encapsulate(chunk_bounds, piece_bounds);
        let placed = place_template(&self.piece, template, level, writable, random);
        spread_netherrack(&self.piece, piece_bounds, level, random);
        add_netherrack_drip_columns_below_portal(&self.piece, piece_bounds, level, random);
        if self.piece.properties().vines || self.piece.properties().overgrown {
            add_vines_and_leaves(&self.piece, piece_bounds, level, random);
        }
        RuinedPortalChunkPlacement {
            attempted: true,
            placed,
        }
    }
}

fn piece_template_bounds(piece: &RuinedPortalPiece, size: [i32; 3]) -> JigsawBoundingBox {
    let pivot = BlockPos {
        x: size[0] / 2,
        y: 0,
        z: size[2] / 2,
    };
    let bounds = template_bounding_box(
        piece.template_position(),
        piece.rotation(),
        pivot,
        piece.mirror(),
        size,
    );
    JigsawBoundingBox {
        min: BlockPos {
            x: bounds[0],
            y: bounds[1],
            z: bounds[2],
        },
        max: BlockPos {
            x: bounds[3],
            y: bounds[4],
            z: bounds[5],
        },
    }
}

/// `BoundingBox.getCenter` (`BoundingBox.java:237-239`).
fn bounding_box_center(bounds: JigsawBoundingBox) -> BlockPos {
    BlockPos {
        x: bounds.min.x + (bounds.max.x - bounds.min.x + 1) / 2,
        y: bounds.min.y + (bounds.max.y - bounds.min.y + 1) / 2,
        z: bounds.min.z + (bounds.max.z - bounds.min.z + 1) / 2,
    }
}

fn encapsulate(left: JigsawBoundingBox, right: JigsawBoundingBox) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: left.min.x.min(right.min.x),
            y: left.min.y.min(right.min.y),
            z: left.min.z.min(right.min.z),
        },
        max: BlockPos {
            x: left.max.x.max(right.max.x),
            y: left.max.y.max(right.max.y),
            z: left.max.z.max(right.max.z),
        },
    }
}

/// `RuinedPortalPiece.makeSettings` then `TemplateStructurePiece.postProcess`
/// (`RuinedPortalPiece.java:107-135`, `TemplateStructurePiece.java:91-117`).
fn place_template<L: WorldGenLevel>(
    piece: &RuinedPortalPiece,
    template: &MaterializedStructureTemplate,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> bool {
    let processors = make_settings(piece);
    let origin = piece.template_position();
    let pivot = BlockPos {
        x: template.size[0] / 2,
        y: 0,
        z: template.size[2] / 2,
    };
    let Some(palette) = place_materialized_template_with_palette(
        template,
        level,
        MaterializedTemplatePlacement {
            position: origin,
            reference_pos: origin,
            rotation: piece.rotation(),
            mirror: piece.mirror(),
            pivot,
            finalize_entities: false,
            ignore_entities: false,
            bounding_box: chunk_bounds,
            processors: &processors,
            liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
            update_mode: UPDATE_CLIENTS,
            known_shape: true,
        },
        random,
        TemplateSettingsRandom::Positional,
    ) else {
        return false;
    };
    replace_jigsaw_blocks(piece, palette, pivot, chunk_bounds, level);
    true
}

fn make_settings(piece: &RuinedPortalPiece) -> Vec<StructureProcessor> {
    let properties = piece.properties();
    let ignore = if properties.air_pocket {
        vec![catalog_default_state("minecraft:structure_block")]
    } else {
        vec![
            catalog_default_state("minecraft:air"),
            catalog_default_state("minecraft:structure_block"),
        ]
    };
    let mut rules = vec![random_block_match_replace(
        "minecraft:gold_block",
        GOLD_GONE,
        catalog_default_state("minecraft:air"),
    )];
    rules.push(lava_processor_rule(piece.vertical_placement(), properties));
    if !properties.cold {
        rules.push(random_block_match_replace(
            "minecraft:netherrack",
            MAGMA_FOR_NETHERRACK,
            catalog_default_state("minecraft:magma_block"),
        ));
    }
    let mut processors = vec![
        StructureProcessor::BlockIgnore { blocks: ignore },
        StructureProcessor::Rule { rules },
        StructureProcessor::BlockAge {
            mossiness: properties.mossiness.get(),
        },
        StructureProcessor::ProtectedBlocks {
            cannot_replace: FEATURES_CANNOT_REPLACE.to_string(),
        },
        StructureProcessor::LavaSubmerged,
    ];
    if properties.replace_with_blackstone {
        processors.push(StructureProcessor::BlackstoneReplace);
    }
    processors
}

fn lava_processor_rule(
    placement: RuinedPortalVerticalPlacement,
    properties: RuinedPortalProperties,
) -> ProcessorRule {
    if placement == RuinedPortalVerticalPlacement::OnOceanFloor {
        block_match_replace(
            "minecraft:lava",
            catalog_default_state("minecraft:magma_block"),
        )
    } else if properties.cold {
        block_match_replace(
            "minecraft:lava",
            catalog_default_state("minecraft:netherrack"),
        )
    } else {
        random_block_match_replace(
            "minecraft:lava",
            MAGMA_FOR_LAVA,
            catalog_default_state("minecraft:magma_block"),
        )
    }
}

fn replace_jigsaw_blocks<L: WorldGenLevel>(
    piece: &RuinedPortalPiece,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    pivot: BlockPos,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
        piece.mirror(),
        pivot,
        chunk_bounds,
        "minecraft:jigsaw",
    ) {
        let Some(nbt) = nbt.as_deref() else {
            continue;
        };
        let state_string =
            compound_string(nbt, "final_state").unwrap_or_else(|| "minecraft:air".into());
        // An unparseable `final_state` logs and places air (`TemplateStructurePiece.java:103-114`).
        let state = parse_block_state_prefix(&state_string)
            .unwrap_or_else(|| catalog_default_state("minecraft:air"));
        let _ = level.set_block(pos, state, UPDATE_ALL);
    }
}

fn filter_blocks(
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    origin: BlockPos,
    rotation: JigsawRotation,
    mirror: RuinedPortalMirror,
    pivot: BlockPos,
    chunk_bounds: JigsawBoundingBox,
    block_name: &str,
) -> Vec<(BlockPos, Option<Arc<NbtSummaryValue>>)> {
    palette
        .blocks
        .iter()
        .filter(|block| block_is(block.state, block_name))
        .filter_map(|block| {
            let pos = wrapping_add(transform(block.position, mirror, rotation, pivot), origin);
            chunk_bounds
                .contains(pos)
                .then_some((pos, block.nbt.clone()))
        })
        .collect()
}

/// `RuinedPortalPiece.spreadNetherrack` (`RuinedPortalPiece.java:226-261`).
fn spread_netherrack<L: WorldGenLevel>(
    piece: &RuinedPortalPiece,
    bounds: JigsawBoundingBox,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    let follow_ground = matches!(
        piece.vertical_placement(),
        RuinedPortalVerticalPlacement::OnLandSurface | RuinedPortalVerticalPlacement::OnOceanFloor
    );
    let center = bounding_box_center(bounds);
    let max_distance = NETHERRACK_DISTANCE.len() as i32;
    let average_width = (bounds.max.x - bounds.min.x + 1 + bounds.max.z - bounds.min.z + 1) / 2;
    let distance_adjustment = random.next_int((8 - average_width / 2).max(1));
    let mut x = center.x - max_distance;
    while x != center.x.wrapping_add(max_distance).wrapping_add(1) {
        let mut z = center.z - max_distance;
        while z != center.z.wrapping_add(max_distance).wrapping_add(1) {
            let distance = (x - center.x).abs() + (z - center.z).abs();
            let adjusted = (distance + distance_adjustment).max(0);
            if adjusted < max_distance
                && random.next_double() < f64::from(NETHERRACK_DISTANCE[adjusted as usize])
            {
                let surface_y = surface_y(level, x, z, piece.vertical_placement());
                let y = if follow_ground {
                    surface_y
                } else {
                    bounds.min.y.min(surface_y)
                };
                let pos = BlockPos { x, y, z };
                if (y - bounds.min.y).abs() <= 3
                    && can_replace_with_netherrack_or_magma(piece, level, pos)
                {
                    place_netherrack_or_magma(piece, level, pos, random);
                    if piece.properties().overgrown {
                        maybe_add_leaves_above(level, pos, random);
                    }
                    add_netherrack_drip_column(
                        piece,
                        level,
                        BlockPos {
                            x: pos.x,
                            y: pos.y.wrapping_sub(1),
                            z: pos.z,
                        },
                        random,
                    );
                }
            }
            z = z.wrapping_add(1);
        }
        x = x.wrapping_add(1);
    }
}

/// `RuinedPortalPiece.addNetherrackDripColumnsBelowPortal` (`RuinedPortalPiece.java:203-212`).
fn add_netherrack_drip_columns_below_portal<L: WorldGenLevel>(
    piece: &RuinedPortalPiece,
    bounds: JigsawBoundingBox,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    let mut x = bounds.min.x.wrapping_add(1);
    while x < bounds.max.x {
        let mut z = bounds.min.z.wrapping_add(1);
        while z < bounds.max.z {
            let pos = BlockPos {
                x,
                y: bounds.min.y,
                z,
            };
            if block_is(level.get_block(pos), "minecraft:netherrack") {
                add_netherrack_drip_column(
                    piece,
                    level,
                    BlockPos {
                        x,
                        y: pos.y.wrapping_sub(1),
                        z,
                    },
                    random,
                );
            }
            z = z.wrapping_add(1);
        }
        x = x.wrapping_add(1);
    }
}

fn add_netherrack_drip_column<L: WorldGenLevel>(
    piece: &RuinedPortalPiece,
    level: &mut L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
) {
    let mut current = pos;
    place_netherrack_or_magma(piece, level, current, random);
    let mut remaining = 8;
    while remaining > 0 && random.next_float() < 0.5 {
        current.y = current.y.wrapping_sub(1);
        remaining -= 1;
        place_netherrack_or_magma(piece, level, current, random);
    }
}

fn add_vines_and_leaves<L: WorldGenLevel>(
    piece: &RuinedPortalPiece,
    bounds: JigsawBoundingBox,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    let mut x = bounds.min.x;
    while x != bounds.max.x.wrapping_add(1) {
        let mut y = bounds.min.y;
        while y != bounds.max.y.wrapping_add(1) {
            let mut z = bounds.min.z;
            while z != bounds.max.z.wrapping_add(1) {
                let pos = BlockPos { x, y, z };
                if piece.properties().vines {
                    maybe_add_vines(level, pos, random);
                }
                if piece.properties().overgrown {
                    maybe_add_leaves_above(level, pos, random);
                }
                z = z.wrapping_add(1);
            }
            y = y.wrapping_add(1);
        }
        x = x.wrapping_add(1);
    }
}

/// `RuinedPortalPiece.maybeAddVines` (`RuinedPortalPiece.java:182-195`).
fn maybe_add_vines<L: WorldGenLevel>(level: &mut L, pos: BlockPos, random: &mut dyn RandomSource) {
    let state = level.get_block(pos);
    if block_is(state, "minecraft:air") || block_is(state, "minecraft:vine") {
        return;
    }
    let direction = HORIZONTAL[random.next_int(4) as usize];
    let neighbour = BlockPos {
        x: pos.x.wrapping_add(direction.step_x()),
        y: pos.y,
        z: pos.z.wrapping_add(direction.step_z()),
    };
    if !block_is(level.get_block(neighbour), "minecraft:air") {
        return;
    }
    if !collision_face_full(state, direction.support()) {
        return;
    }
    let vine = catalog_default_state("minecraft:vine");
    let vine = block_set_property(vine, direction.opposite_face(), "true");
    let _ = level.set_block(neighbour, vine, UPDATE_ALL);
}

/// `RuinedPortalPiece.maybeAddLeavesAbove` (`RuinedPortalPiece.java:197-201`).
fn maybe_add_leaves_above<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
) {
    if random.next_float() >= 0.5
        || !block_is(level.get_block(pos), "minecraft:netherrack")
        || !block_is(
            level.get_block(BlockPos {
                x: pos.x,
                y: pos.y.wrapping_add(1),
                z: pos.z,
            }),
            "minecraft:air",
        )
    {
        return;
    }
    let leaves = catalog_default_state("minecraft:jungle_leaves");
    let leaves = block_set_property(leaves, "persistent", "true");
    let _ = level.set_block(
        BlockPos {
            x: pos.x,
            y: pos.y.wrapping_add(1),
            z: pos.z,
        },
        leaves,
        UPDATE_ALL,
    );
}

fn can_replace_with_netherrack_or_magma<L: WorldGenLevel>(
    piece: &RuinedPortalPiece,
    level: &L,
    pos: BlockPos,
) -> bool {
    let state = level.get_block(pos);
    if block_is(state, "minecraft:air")
        || block_is(state, "minecraft:obsidian")
        || level.block_state_in_tag(state, FEATURES_CANNOT_REPLACE)
    {
        return false;
    }

    piece.vertical_placement() == RuinedPortalVerticalPlacement::InNether
        || !block_is(state, "minecraft:lava")
}

fn place_netherrack_or_magma<L: WorldGenLevel>(
    piece: &RuinedPortalPiece,
    level: &mut L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
) {
    let id = if !piece.properties().cold && random.next_float() < MAGMA_FOR_NETHERRACK {
        "minecraft:magma_block"
    } else {
        "minecraft:netherrack"
    };
    let state = catalog_default_state(id);
    let _ = level.set_block(pos, state, UPDATE_ALL);
}

fn surface_y<L: WorldGenLevel>(
    level: &L,
    x: i32,
    z: i32,
    placement: RuinedPortalVerticalPlacement,
) -> i32 {
    let height = if placement == RuinedPortalVerticalPlacement::OnOceanFloor {
        level.get_ocean_floor_wg_height(x, z)
    } else {
        level.get_world_surface_wg_height(x, z)
    };
    height.wrapping_sub(1)
}

fn collision_face_full(state: i32, direction: VanillaBlockSupportDirection) -> bool {
    vanilla_block_state_support_facts(state)
        .is_some_and(|facts| facts.collision_face_full(direction))
}

#[derive(Clone, Copy)]
enum Horizontal {
    North,
    East,
    South,
    West,
}

/// `Direction.Plane.HORIZONTAL` (`Direction.java:577`).
const HORIZONTAL: [Horizontal; 4] = [
    Horizontal::North,
    Horizontal::East,
    Horizontal::South,
    Horizontal::West,
];

impl Horizontal {
    fn step_x(self) -> i32 {
        match self {
            Self::East => 1,
            Self::West => -1,
            Self::North | Self::South => 0,
        }
    }

    fn step_z(self) -> i32 {
        match self {
            Self::North => -1,
            Self::South => 1,
            Self::East | Self::West => 0,
        }
    }

    fn support(self) -> VanillaBlockSupportDirection {
        match self {
            Self::North => VanillaBlockSupportDirection::North,
            Self::East => VanillaBlockSupportDirection::East,
            Self::South => VanillaBlockSupportDirection::South,
            Self::West => VanillaBlockSupportDirection::West,
        }
    }

    fn opposite_face(self) -> &'static str {
        match self {
            Self::North => "south",
            Self::East => "west",
            Self::South => "north",
            Self::West => "east",
        }
    }
}

fn wrapping_add(left: BlockPos, right: BlockPos) -> BlockPos {
    BlockPos {
        x: left.x.wrapping_add(right.x),
        y: left.y.wrapping_add(right.y),
        z: left.z.wrapping_add(right.z),
    }
}

fn compound_string(value: &NbtSummaryValue, name: &str) -> Option<String> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    fields.iter().rev().find_map(|entry| {
        (entry.name == name)
            .then_some(&entry.value)
            .and_then(|value| match value {
                NbtSummaryValue::String(text) => text.try_to_string().ok(),
                _ => None,
            })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::super::{RuinedPortalFloat, RuinedPortalGenerationStub};
    use super::*;
    use crate::block_is;
    use crate::random::WorldgenRandom;
    use bbb_server_datapack::structure_template::{
        StructureTemplate, StructureTemplateBlock, StructureTemplateElement,
        StructureTemplateEntity, StructureTemplatePalette, StructureTemplateSet,
    };

    struct PlacementLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        air: i32,
        surface: i32,
        writes: Vec<(BlockPos, i32)>,
        loaded_block_entities: Vec<(BlockPos, i32, Vec<bbb_protocol::NbtSummaryEntry>)>,
        template_entities: Vec<(
            [f64; 3],
            (u8, bbb_protocol::block_state_types::VanillaBlockStateMirror),
        )>,
    }

    impl PlacementLevel {
        fn new() -> Self {
            Self {
                cells: BTreeMap::new(),
                air: catalog_default_state("minecraft:air"),
                surface: 64,
                writes: Vec::new(),
                loaded_block_entities: Vec::new(),
                template_entities: Vec::new(),
            }
        }
    }

    impl WorldGenLevel for PlacementLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
            crate::tests::builtin_registry_tag_contains(registry_id, tag_id, element_id)
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.cells
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(self.air)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            self.writes.push((pos, state));
            if state == self.air {
                self.cells.remove(&(pos.x, pos.y, pos.z));
            } else {
                self.cells.insert((pos.x, pos.y, pos.z), state);
            }
            true
        }

        fn get_min_y(&self) -> i32 {
            -64
        }

        fn get_max_y(&self) -> i32 {
            319
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

        fn get_world_surface_wg_height(&self, _x: i32, _z: i32) -> i32 {
            self.surface
        }

        fn get_ocean_floor_wg_height(&self, _x: i32, _z: i32) -> i32 {
            self.surface
        }

        fn load_structure_block_entity(
            &mut self,
            pos: BlockPos,
            state: i32,
            record: &[bbb_protocol::NbtSummaryEntry],
        ) {
            self.loaded_block_entities
                .push((pos, state, record.to_vec()));
        }

        fn add_template_entity_with_finalize(
            &mut self,
            _nbt: &NbtSummaryValue,
            position: [f64; 3],
            _rotation: [f32; 2],
            _finalize: bool,
            transform: (u8, bbb_protocol::block_state_types::VanillaBlockStateMirror),
        ) {
            self.template_entities.push((position, transform));
        }
    }

    fn at(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn state(id: &'static str) -> i32 {
        catalog_default_state(id)
    }

    fn block(x: i32, y: i32, z: i32, state: i32) -> StructureTemplateBlock {
        StructureTemplateBlock {
            position: at(x, y, z),
            state,
            nbt: None,
        }
    }

    fn palette(blocks: Vec<StructureTemplateBlock>) -> StructureTemplatePalette {
        StructureTemplatePalette {
            blocks,
            jigsaws: Vec::new(),
        }
    }

    fn templates(
        id: &str,
        size: [i32; 3],
        blocks: Vec<StructureTemplateBlock>,
    ) -> StructureTemplateSet {
        templates_with_entities(id, size, blocks, Vec::new())
    }

    fn templates_with_entities(
        id: &str,
        size: [i32; 3],
        blocks: Vec<StructureTemplateBlock>,
        entities: Vec<StructureTemplateEntity>,
    ) -> StructureTemplateSet {
        StructureTemplateSet {
            templates: vec![StructureTemplateElement {
                id: id.to_string(),
                source_pack: "test".to_string(),
                template: StructureTemplate::from_materialized(
                    bbb_server_datapack::structure_template::MaterializedStructureTemplate {
                        size,
                        palettes: vec![palette(blocks)],
                        entities,
                    },
                )
                .unwrap(),
            }],
            warnings: Vec::new(),
        }
    }

    fn props() -> RuinedPortalProperties {
        RuinedPortalProperties {
            cold: false,
            mossiness: RuinedPortalFloat::from_f32(0.0),
            air_pocket: true,
            overgrown: false,
            vines: false,
            replace_with_blackstone: false,
        }
    }

    fn start(
        template: &str,
        origin: BlockPos,
        size: [i32; 3],
        properties: RuinedPortalProperties,
    ) -> RuinedPortalStructureStart {
        RuinedPortalStructureStart::from_generation_stub(
            RuinedPortalGenerationStub::new(
                [0, 0],
                origin,
                "minecraft:ruined_portal",
                RuinedPortalPiece::new(
                    template,
                    origin,
                    JigsawRotation::None,
                    RuinedPortalMirror::None,
                    RuinedPortalVerticalPlacement::OnLandSurface,
                    properties,
                )
                .with_template_size(size),
            ),
            1,
        )
    }

    fn wide_bounds() -> JigsawBoundingBox {
        JigsawBoundingBox {
            min: at(-1000, -1000, -1000),
            max: at(1000, 1000, 1000),
        }
    }

    #[test]
    fn place_writes_a_non_air_block_inside_the_chunk_bb() {
        let id = "minecraft:ruined_portal/portal_1";
        let templates = templates(
            id,
            [1, 1, 1],
            vec![block(0, 0, 0, state("minecraft:obsidian"))],
        );
        let mut start = start(id, at(4, 10, 8), [1, 1, 1], props());
        let mut level = PlacementLevel::new();
        let report = start.place_in_chunk(
            &templates,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        let placed = level.get_block(at(4, 10, 8));
        assert!(
            block_is(placed, "minecraft:obsidian") || block_is(placed, "minecraft:crying_obsidian")
        );
    }

    #[test]
    fn place_in_chunk_skips_when_the_center_misses_the_writable_area() {
        let id = "minecraft:ruined_portal/portal_1";
        let templates = templates(
            id,
            [11, 1, 1],
            vec![block(0, 0, 0, state("minecraft:obsidian"))],
        );
        let mut start = start(id, at(12, 10, 0), [11, 1, 1], props());
        let report = start.place_in_chunk(
            &templates,
            &mut PlacementLevel::new(),
            JigsawBoundingBox {
                min: at(0, -64, 0),
                max: at(15, 320, 15),
            },
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && !report.placed);
    }

    #[test]
    fn drip_column_writes_under_interior_netherrack() {
        let id = "minecraft:ruined_portal/portal_1";
        let netherrack = state("minecraft:netherrack");
        let templates = templates(
            id,
            [3, 1, 3],
            vec![
                block(0, 0, 0, netherrack),
                block(1, 0, 0, netherrack),
                block(2, 0, 0, netherrack),
                block(0, 0, 1, netherrack),
                block(1, 0, 1, netherrack),
                block(2, 0, 1, netherrack),
                block(0, 0, 2, netherrack),
                block(1, 0, 2, netherrack),
                block(2, 0, 2, netherrack),
            ],
        );
        let mut start = start(id, at(0, 10, 0), [3, 1, 3], props());
        let mut level = PlacementLevel::new();
        start.place_in_chunk(
            &templates,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        let below = level.get_block(at(1, 9, 1));
        assert!(
            block_is(below, "minecraft:netherrack") || block_is(below, "minecraft:magma_block"),
            "below={below}"
        );
    }

    #[test]
    fn ruined_portal_preserves_block_entity_nbt_and_mirrors_template_entities() {
        use bbb_protocol::block_state_types::VanillaBlockStateMirror;
        use bbb_protocol::NbtSummaryEntry;

        let id = "minecraft:ruined_portal/portal_1";
        let chest = state("minecraft:chest");
        let templates = templates_with_entities(
            id,
            [3, 1, 3],
            vec![StructureTemplateBlock {
                position: at(1, 0, 1),
                state: chest,
                nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "LootTable".into(),
                    value: NbtSummaryValue::string("minecraft:chests/ruined_portal"),
                }]))),
            }],
            vec![StructureTemplateEntity {
                block_position: at(0, 0, 2),
                position_bits: [0.5_f64.to_bits(), 0.0_f64.to_bits(), 2.5_f64.to_bits()],
                nbt: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::string("minecraft:armor_stand"),
                }]),
            }],
        );
        let mut start = RuinedPortalStructureStart::from_generation_stub(
            RuinedPortalGenerationStub::new(
                [0, 0],
                at(10, 10, 20),
                "minecraft:ruined_portal",
                RuinedPortalPiece::new(
                    id,
                    at(10, 10, 20),
                    JigsawRotation::None,
                    RuinedPortalMirror::LeftRight,
                    RuinedPortalVerticalPlacement::OnLandSurface,
                    props(),
                )
                .with_template_size([3, 1, 3]),
            ),
            1,
        );
        let mut level = PlacementLevel::new();
        let report = start.place_in_chunk(
            &templates,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(99),
        );
        assert!(report.attempted && report.placed);
        assert_eq!(level.loaded_block_entities.len(), 1);
        assert!(level.loaded_block_entities[0]
            .2
            .iter()
            .any(|e| e.name == "LootTable"
                && e.value == NbtSummaryValue::string("minecraft:chests/ruined_portal")));
        assert!(level.loaded_block_entities[0]
            .2
            .iter()
            .any(|e| e.name == "LootTableSeed"));
        assert_eq!(level.template_entities.len(), 1);
        assert_eq!(
            level.template_entities[0].1,
            (0, VanillaBlockStateMirror::LeftRight)
        );
        // Mirror::LeftRight negates Z about pivot (1, 0, 1): 2.5 -> -1.5 -> +20 = 18.5.
        assert!((level.template_entities[0].0[2] - 18.5).abs() < 1e-6);
    }
}
