//! `WoodlandMansionPiece` template placement plus `afterPlace` cobblestone.

use std::sync::Arc;

use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::structure_pool::StructureLiquidSettings;

use crate::noise::ruined_portal::RuinedPortalMirror;
use crate::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use crate::structure_processor::StructureProcessor;
use crate::structure_template_placement::{
    place_materialized_template_with_palette, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
use crate::{block_is, block_set_property, catalog_default_state, WorldGenLevel};
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplateSet,
};
use bbb_server_types::block_state_parse::parse_block_state_prefix;
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};

use super::direction::{rotate_direction, MansionDirection};
use super::persistence::{piece_bounding_box, transform_zero_pivot};
use super::{WoodlandMansionPiece, WoodlandMansionStructureStart, WOODLAND_MANSION_LOOT};

/// `Block.UPDATE_CLIENTS` (`Block.java:92`), the `placeInWorld` update flag.
const UPDATE_CLIENTS: i32 = 2;
/// `Block.UPDATE_NEIGHBORS | UPDATE_CLIENTS` (`TemplateStructurePiece.java:114`).
const UPDATE_JIGSAW_FINAL: i32 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WoodlandMansionChunkPlacement {
    pub attempted: bool,
    pub placed: bool,
    pub foundation: bool,
}

impl WoodlandMansionStructureStart {
    /// `StructureStart.placeInChunk`, `TemplateStructurePiece.postProcess`,
    /// then `WoodlandMansionStructure.afterPlace` (`StructureStart.java:93-99`,
    /// `TemplateStructurePiece.java:83-117`,
    /// `WoodlandMansionStructure.java:46-75`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        templates: &StructureTemplateSet,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> WoodlandMansionChunkPlacement {
        let mut report = WoodlandMansionChunkPlacement::default();
        let mut sizes = Vec::with_capacity(self.pieces.len());
        for index in 0..self.pieces.len() {
            let template_id = self.pieces[index].template().to_string();
            let live_template = level.structure_template(&template_id);
            let Some(template) = live_template.as_ref().or_else(|| {
                templates
                    .template(&template_id)
                    .map(|element| &element.template)
            }) else {
                sizes.push([0; 3]);
                continue;
            };
            let template = template.materialized();
            sizes.push(template.size);
            if !piece_intersects_writable_area(&self.pieces[index], template.size, chunk_bounds) {
                continue;
            }
            report.attempted = true;
            if place_template(&self.pieces[index], template, level, chunk_bounds, random) {
                report.placed = true;
            }
        }
        report.foundation = place_cobblestone_foundation(&self.pieces, &sizes, level, chunk_bounds);
        report
    }
}

fn piece_intersects_writable_area(
    piece: &WoodlandMansionPiece,
    size: [i32; 3],
    chunk_bounds: JigsawBoundingBox,
) -> bool {
    let bounds = piece_bounding_box(
        piece.template_position(),
        piece.rotation(),
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
    .intersects(chunk_bounds)
}

/// `WoodlandMansionPieces.makeSettings` (`WoodlandMansionPieces.java:1242-1244`)
/// then `TemplateStructurePiece.postProcess` (`TemplateStructurePiece.java:91-117`).
fn place_template<L: WorldGenLevel>(
    piece: &WoodlandMansionPiece,
    template: &MaterializedStructureTemplate,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> bool {
    let processors = make_settings();
    let origin = piece.template_position();
    let Some(palette) = place_materialized_template_with_palette(
        template,
        level,
        MaterializedTemplatePlacement {
            position: origin,
            reference_pos: origin,
            rotation: piece.rotation(),
            mirror: piece.mirror(),
            pivot: BlockPos { x: 0, y: 0, z: 0 },
            finalize_entities: false,
            ignore_entities: true,
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
    handle_data_markers(piece, palette, chunk_bounds, level, random);
    replace_jigsaw_blocks(piece, palette, chunk_bounds, level);
    true
}

fn make_settings() -> Vec<StructureProcessor> {
    vec![StructureProcessor::BlockIgnore {
        blocks: vec![catalog_default_state("minecraft:structure_block")],
    }]
}

/// `WoodlandMansionPieces.handleDataMarker` (`WoodlandMansionPieces.java:1255-1300`).
fn handle_data_markers<L: WorldGenLevel>(
    piece: &WoodlandMansionPiece,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
        piece.mirror(),
        chunk_bounds,
        "minecraft:structure_block",
    ) {
        let Some(nbt) = nbt.as_deref() else {
            continue;
        };
        let Some(mode) = compound_string(nbt, "mode") else {
            continue;
        };
        if !mode.eq_ignore_ascii_case("data") {
            continue;
        }
        let marker = compound_string(nbt, "metadata").unwrap_or_default();
        if let Some(facing) = chest_marker_facing(&marker) {
            place_chest(
                pos,
                rotate_direction(facing, piece.rotation()),
                level,
                random,
            );
            continue;
        }
        handle_mob_marker(&marker, pos, level);
    }
}

/// `WoodlandMansionPieces.handleDataMarker` mob arm (`WoodlandMansionPieces.java:1272-1299`).
fn handle_mob_marker<L: WorldGenLevel>(marker: &str, pos: BlockPos, level: &mut L) {
    let (resource_id, count) = match marker {
        "Mage" => ("minecraft:evoker", 1),
        "Warrior" => ("minecraft:vindicator", 1),
        "Group of Allays" => {
            // `level.getRandom().nextInt(3) + 1` (`:1280-1285`); fail closed, no guessed count.
            let extra = level.next_level_random_int(3);
            ("minecraft:allay", extra + 1)
        }
        _ => return,
    };
    // `Entity.snapTo(BlockPos)` (`Entity.java:1705-1706`) = `getBottomCenter`.
    let position = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y),
        f64::from(pos.z) + 0.5,
    ];
    let air = catalog_default_state("minecraft:air");
    for _ in 0..count {
        level.add_structure_mob(resource_id, position);
        level.set_block(pos, air, UPDATE_CLIENTS);
    }
}

fn chest_marker_facing(marker: &str) -> Option<MansionDirection> {
    match marker {
        "ChestWest" => Some(MansionDirection::West),
        "ChestEast" => Some(MansionDirection::East),
        "ChestSouth" => Some(MansionDirection::South),
        "ChestNorth" => Some(MansionDirection::North),
        marker if marker.starts_with("Chest") => Some(MansionDirection::North),
        _ => None,
    }
}

/// `StructurePiece.createChest` (`StructurePiece.java:461-484`).
fn place_chest<L: WorldGenLevel>(
    pos: BlockPos,
    facing: MansionDirection,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    if block_is(level.get_block(pos), "minecraft:chest") {
        return;
    }
    let chest = catalog_default_state("minecraft:chest");
    let chest = block_set_property(chest, "facing", facing_name(facing));
    let _ = level.set_block(pos, chest, UPDATE_CLIENTS);
    let seed = random.next_long();
    level.set_block_entity_loot_table(pos, WOODLAND_MANSION_LOOT, seed);
}

fn facing_name(direction: MansionDirection) -> &'static str {
    match direction {
        MansionDirection::North => "north",
        MansionDirection::South => "south",
        MansionDirection::West => "west",
        MansionDirection::East => "east",
        MansionDirection::Up => "north",
    }
}

fn replace_jigsaw_blocks<L: WorldGenLevel>(
    piece: &WoodlandMansionPiece,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
        piece.mirror(),
        chunk_bounds,
        "minecraft:jigsaw",
    ) {
        let Some(nbt) = nbt.as_deref() else {
            continue;
        };
        let state_string =
            compound_string(nbt, "final_state").unwrap_or_else(|| "minecraft:air".into());
        // `TemplateStructurePiece.java:105-114`: unparseable states place air.
        let state = parse_block_state_prefix(&state_string)
            .unwrap_or_else(|| catalog_default_state("minecraft:air"));
        let _ = level.set_block(pos, state, UPDATE_JIGSAW_FINAL);
    }
}

/// `StructureTemplate.filterBlocks` (`StructureTemplate.java:220-236`).
fn filter_blocks(
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    origin: BlockPos,
    rotation: JigsawRotation,
    mirror: RuinedPortalMirror,
    chunk_bounds: JigsawBoundingBox,
    block_name: &str,
) -> Vec<(BlockPos, Option<Arc<NbtSummaryValue>>)> {
    palette
        .blocks
        .iter()
        .filter(|block| block_is(block.state, block_name))
        .filter_map(|block| {
            let pos = wrapping_add(
                transform_zero_pivot(block.position, mirror, rotation),
                origin,
            );
            chunk_bounds
                .contains(pos)
                .then_some((pos, block.nbt.clone()))
        })
        .collect()
}

/// `WoodlandMansionStructure.afterPlace` (`WoodlandMansionStructure.java:46-75`).
fn place_cobblestone_foundation<L: WorldGenLevel>(
    pieces: &[WoodlandMansionPiece],
    sizes: &[[i32; 3]],
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) -> bool {
    let Some(union) = pieces_union_bounds(pieces, sizes) else {
        return false;
    };
    let cobble = catalog_default_state("minecraft:cobblestone");
    let y_start = union.min.y;
    let min_y = level.get_min_y();
    let mut placed = false;
    let mut x = chunk_bounds.min.x;
    while x != chunk_bounds.max.x.wrapping_add(1) {
        let mut z = chunk_bounds.min.z;
        while z != chunk_bounds.max.z.wrapping_add(1) {
            let floor = BlockPos { x, y: y_start, z };
            if !block_state_is_air(level.get_block(floor))
                && contains_box(union, floor)
                && pieces_contain(pieces, sizes, floor)
            {
                let mut y = y_start.wrapping_sub(1);
                while y > min_y {
                    let pos = BlockPos { x, y, z };
                    let state = level.get_block(pos);
                    if !block_state_is_air(state)
                        && block_state_fluid_state(state).family().is_none()
                    {
                        break;
                    }
                    let _ = level.set_block(pos, cobble, UPDATE_CLIENTS);
                    placed = true;
                    y = y.wrapping_sub(1);
                }
            }
            z = z.wrapping_add(1);
        }
        x = x.wrapping_add(1);
    }
    placed
}

fn pieces_union_bounds(
    pieces: &[WoodlandMansionPiece],
    sizes: &[[i32; 3]],
) -> Option<JigsawBoundingBox> {
    let mut union: Option<JigsawBoundingBox> = None;
    for (piece, size) in pieces.iter().zip(sizes) {
        let bounds = piece_bounding_box(
            piece.template_position(),
            piece.rotation(),
            piece.mirror(),
            *size,
        );
        let box_ = JigsawBoundingBox {
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
        };
        union = Some(match union {
            None => box_,
            Some(current) => JigsawBoundingBox {
                min: BlockPos {
                    x: current.min.x.min(box_.min.x),
                    y: current.min.y.min(box_.min.y),
                    z: current.min.z.min(box_.min.z),
                },
                max: BlockPos {
                    x: current.max.x.max(box_.max.x),
                    y: current.max.y.max(box_.max.y),
                    z: current.max.z.max(box_.max.z),
                },
            },
        });
    }
    union
}

fn pieces_contain(pieces: &[WoodlandMansionPiece], sizes: &[[i32; 3]], pos: BlockPos) -> bool {
    pieces.iter().zip(sizes).any(|(piece, size)| {
        let bounds = piece_bounding_box(
            piece.template_position(),
            piece.rotation(),
            piece.mirror(),
            *size,
        );
        contains_box(
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
            },
            pos,
        )
    })
}

fn contains_box(bounds: JigsawBoundingBox, pos: BlockPos) -> bool {
    pos.x >= bounds.min.x
        && pos.x <= bounds.max.x
        && pos.y >= bounds.min.y
        && pos.y <= bounds.max.y
        && pos.z >= bounds.min.z
        && pos.z <= bounds.max.z
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
    use std::sync::Arc;

    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::noise::ruined_portal::RuinedPortalMirror;
    use crate::random::WorldgenRandom;
    use bbb_server_datapack::structure_template::{
        StructureTemplate, StructureTemplateBlock, StructureTemplateElement,
        StructureTemplatePalette, StructureTemplateSet,
    };

    use super::super::{
        WoodlandMansionGenerationStub, WoodlandMansionPiece, WOODLAND_MANSION_ID,
        WOODLAND_MANSION_LOOT,
    };
    use super::*;

    struct PlacementLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        air: i32,
        max_y: i32,
        writes: Vec<(BlockPos, i32, i32)>,
        loot: Vec<(BlockPos, &'static str, i64)>,
        loot_available: bool,
        mobs: Vec<(String, [f64; 3])>,
        /// `WorldGenRegion.getRandom()` stream; `None` is the default seam miss.
        level_random: Option<WorldgenRandom>,
    }

    impl PlacementLevel {
        fn new() -> Self {
            Self {
                cells: BTreeMap::new(),
                air: catalog_default_state("minecraft:air"),
                max_y: 319,
                writes: Vec::new(),
                loot: Vec::new(),
                loot_available: true,
                mobs: Vec::new(),
                level_random: Some(WorldgenRandom::with_seed(0)),
            }
        }

        fn with_level_seed(seed: i64) -> Self {
            let mut level = Self::new();
            level.level_random = Some(WorldgenRandom::with_seed(seed));
            level
        }
    }

    impl WorldGenLevel for PlacementLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.cells
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(self.air)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
            self.writes.push((pos, state, update_flags));
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
            self.max_y
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

        fn set_block_entity_loot_table(
            &mut self,
            pos: BlockPos,
            loot_table: &'static str,
            seed: i64,
        ) {
            self.loot.push((pos, loot_table, seed));
        }

        fn next_level_random_int(&mut self, bound: i32) -> i32 {
            self.level_random
                .as_mut()
                .expect("level random")
                .next_int(bound)
        }

        fn add_structure_mob(&mut self, resource_id: &str, position: [f64; 3]) {
            self.mobs.push((resource_id.to_string(), position));
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
        StructureTemplateSet {
            templates: vec![StructureTemplateElement {
                id: id.to_string(),
                source_pack: "test".to_string(),
                template: StructureTemplate::from_materialized(MaterializedStructureTemplate {
                    size,
                    palettes: vec![palette(blocks)],
                    entities: Vec::new(),
                })
                .unwrap(),
            }],
            warnings: Vec::new(),
        }
    }

    fn start(template: &str, origin: BlockPos) -> WoodlandMansionStructureStart {
        WoodlandMansionStructureStart::from_generation_stub(
            WoodlandMansionGenerationStub::new(
                [0, 0],
                origin,
                WOODLAND_MANSION_ID,
                vec![WoodlandMansionPiece::new(
                    template,
                    origin,
                    JigsawRotation::None,
                    RuinedPortalMirror::None,
                )],
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

    fn data_marker(marker: &str) -> StructureTemplateBlock {
        StructureTemplateBlock {
            position: at(0, 0, 0),
            state: state("minecraft:structure_block"),
            nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "mode".into(),
                    value: NbtSummaryValue::string("DATA"),
                },
                NbtSummaryEntry {
                    name: "metadata".into(),
                    value: NbtSummaryValue::string(marker),
                },
            ]))),
        }
    }

    fn bottom_center(pos: BlockPos) -> [f64; 3] {
        [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y),
            f64::from(pos.z) + 0.5,
        ]
    }

    fn assert_marker_cleared_to_air(level: &PlacementLevel, pos: BlockPos) {
        assert!(
            level.writes.iter().any(|(written, written_state, flags)| {
                *written == pos && *written_state == level.air && *flags == UPDATE_CLIENTS
            }),
            "{:?}",
            level.writes
        );
        assert!(block_is(level.get_block(pos), "minecraft:air"));
    }

    #[test]
    fn place_writes_a_non_air_block_inside_the_chunk_bb() {
        let id = "minecraft:woodland_mansion/1x1_a1";
        let templates = templates(
            id,
            [1, 1, 1],
            vec![block(0, 0, 0, state("minecraft:oak_planks"))],
        );
        let mut start = start(id, at(4, 10, 8));
        let mut level = PlacementLevel::new();
        let report = start.place_in_chunk(
            &templates,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        assert!(block_is(
            level.get_block(at(4, 10, 8)),
            "minecraft:oak_planks"
        ));
    }

    #[test]
    fn chest_marker_places_loot_on_the_marker() {
        let id = "minecraft:woodland_mansion/chest";
        let structure = state("minecraft:structure_block");
        let templates = templates(
            id,
            [1, 1, 1],
            vec![StructureTemplateBlock {
                position: at(0, 0, 0),
                state: structure,
                nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "mode".into(),
                        value: NbtSummaryValue::string("DATA"),
                    },
                    NbtSummaryEntry {
                        name: "metadata".into(),
                        value: NbtSummaryValue::string("ChestWest"),
                    },
                ]))),
            }],
        );
        let mut start = start(id, at(2, 9, 3));
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(1);
        let seed = {
            let mut peek = WorldgenRandom::with_seed(1);
            peek.next_long()
        };
        start.place_in_chunk(&templates, &mut level, wide_bounds(), &mut random);
        assert!(block_is(level.get_block(at(2, 9, 3)), "minecraft:chest"));
        assert_eq!(level.loot, vec![(at(2, 9, 3), WOODLAND_MANSION_LOOT, seed)]);
    }

    #[test]
    fn mage_marker_spawns_one_evoker_and_clears_the_block() {
        let id = "minecraft:woodland_mansion/mage";
        let origin = at(0, 8, 0);
        let templates = templates(id, [1, 1, 1], vec![data_marker("Mage")]);
        let mut start = start(id, origin);
        let mut level = PlacementLevel::new();
        let report = start.place_in_chunk(
            &templates,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        assert_eq!(
            level.mobs,
            vec![("minecraft:evoker".to_string(), bottom_center(origin))]
        );
        assert_marker_cleared_to_air(&level, origin);
    }

    #[test]
    fn warrior_marker_spawns_one_vindicator_and_clears_the_block() {
        let id = "minecraft:woodland_mansion/warrior";
        let origin = at(3, 6, 1);
        let templates = templates(id, [1, 1, 1], vec![data_marker("Warrior")]);
        let mut start = start(id, origin);
        let mut level = PlacementLevel::new();
        let report = start.place_in_chunk(
            &templates,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        assert_eq!(
            level.mobs,
            vec![("minecraft:vindicator".to_string(), bottom_center(origin))]
        );
        assert_marker_cleared_to_air(&level, origin);
    }

    #[test]
    fn group_of_allays_uses_level_random_and_clears_the_block() {
        const SEED: i64 = 11;
        let expected = {
            let mut peek = WorldgenRandom::with_seed(SEED);
            peek.next_int(3) + 1
        };
        let id = "minecraft:woodland_mansion/allays";
        let origin = at(5, 12, 7);
        let templates = templates(id, [1, 1, 1], vec![data_marker("Group of Allays")]);
        let mut start = start(id, origin);
        let mut level = PlacementLevel::with_level_seed(SEED);
        let report = start.place_in_chunk(
            &templates,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        assert!((1..=3).contains(&expected), "{expected}");
        assert_eq!(
            level.mobs,
            vec![("minecraft:allay".to_string(), bottom_center(origin)); expected as usize]
        );
        assert_marker_cleared_to_air(&level, origin);
    }

    #[test]
    fn place_in_chunk_skips_pieces_that_miss_the_writable_area() {
        let near_id = "minecraft:woodland_mansion/1x1_a1";
        let far_id = "minecraft:woodland_mansion/1x1_a2";
        let templates = StructureTemplateSet {
            templates: vec![
                templates(
                    near_id,
                    [1, 1, 1],
                    vec![block(0, 0, 0, state("minecraft:oak_planks"))],
                )
                .templates
                .remove(0),
                templates(
                    far_id,
                    [1, 1, 1],
                    vec![block(0, 0, 0, state("minecraft:oak_planks"))],
                )
                .templates
                .remove(0),
            ],
            warnings: Vec::new(),
        };
        let mut start = WoodlandMansionStructureStart::from_generation_stub(
            WoodlandMansionGenerationStub::new(
                [0, 0],
                at(4, 10, 8),
                WOODLAND_MANSION_ID,
                vec![
                    WoodlandMansionPiece::new(
                        near_id,
                        at(4, 10, 8),
                        JigsawRotation::None,
                        RuinedPortalMirror::None,
                    ),
                    WoodlandMansionPiece::new(
                        far_id,
                        at(80, 10, 80),
                        JigsawRotation::None,
                        RuinedPortalMirror::None,
                    ),
                ],
            ),
            1,
        );
        let report = start.place_in_chunk(
            &templates,
            &mut PlacementLevel::new(),
            JigsawBoundingBox {
                min: at(0, -64, 0),
                max: at(15, 320, 15),
            },
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
    }

    #[test]
    fn after_place_fills_cobblestone_under_the_floor() {
        let id = "minecraft:woodland_mansion/1x1_a1";
        let templates = templates(
            id,
            [1, 1, 1],
            vec![block(0, 0, 0, state("minecraft:oak_planks"))],
        );
        let mut start = start(id, at(4, 10, 8));
        let mut level = PlacementLevel::new();
        start.place_in_chunk(
            &templates,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(block_is(
            level.get_block(at(4, 9, 8)),
            "minecraft:cobblestone"
        ));
        assert!(block_is(
            level.get_block(at(4, -63, 8)),
            "minecraft:cobblestone"
        ));
    }
}
