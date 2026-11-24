//! `ShipwreckPiece.postProcess` plus `TemplateStructurePiece.postProcess`.

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
use crate::{block_is, catalog_default_state, WorldGenLevel};
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplateSet,
};
use bbb_server_types::block_state_parse::parse_block_state_prefix;

use super::{
    ShipwreckPiece, ShipwreckStructureStart, SHIPWRECK_MAP_LOOT, SHIPWRECK_PIVOT,
    SHIPWRECK_SUPPLY_LOOT, SHIPWRECK_TREASURE_LOOT,
};

/// `Block.UPDATE_CLIENTS` (`Block.java:92`), the `placeInWorld` update flag.
const UPDATE_CLIENTS: i32 = 2;
/// `Block.UPDATE_NEIGHBORS | UPDATE_CLIENTS` (`TemplateStructurePiece.java:114`).
const UPDATE_JIGSAW_FINAL: i32 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipwreckChunkPlacement {
    pub attempted: bool,
    pub placed: bool,
}

impl ShipwreckStructureStart {
    /// `ShipwreckPieces.ShipwreckPiece.postProcess` then
    /// `TemplateStructurePiece.postProcess` (`ShipwreckPieces.java:137-171`,
    /// `TemplateStructurePiece.java:82-117`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        templates: &StructureTemplateSet,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> ShipwreckChunkPlacement {
        let template_id = self.piece.template().to_string();
        let live_template = level.structure_template(&template_id);
        let Some(template) = live_template.as_ref().or_else(|| {
            templates
                .template(&template_id)
                .map(|element| &element.template)
        }) else {
            return ShipwreckChunkPlacement::default();
        };
        let template = template.materialized();
        maybe_adjust_height(&mut self.piece, template.size, level, random);
        self.cached_bounding_box.invalidate();
        let placed = place_template(&self.piece, template, level, chunk_bounds, random);
        ShipwreckChunkPlacement {
            attempted: true,
            placed,
        }
    }
}

/// `ShipwreckPieces.java:146-166`.
fn maybe_adjust_height<L: WorldGenLevel>(
    piece: &mut ShipwreckPiece,
    size: [i32; 3],
    level: &L,
    random: &mut dyn RandomSource,
) {
    if piece.height_adjusted() || ShipwreckPiece::is_too_big_to_fit_in_world_gen_region(size) {
        return;
    }
    let height = sample_template_height(piece, size, level, random);
    piece.adjust_position_height(height);
}

fn sample_template_height<L: WorldGenLevel>(
    piece: &ShipwreckPiece,
    size: [i32; 3],
    level: &L,
    random: &mut dyn RandomSource,
) -> i32 {
    let origin = piece.template_position();
    let mut min_y = level.get_max_y().wrapping_add(1);
    let mut mean = 0;
    let base_size = size[0].wrapping_mul(size[2]);
    if base_size == 0 {
        mean = sample_height(level, piece.is_beached(), origin.x, origin.z);
    } else {
        let max_x = origin.x.wrapping_add(size[0].wrapping_sub(1));
        let max_z = origin.z.wrapping_add(size[2].wrapping_sub(1));
        let mut z = origin.z;
        while z != max_z.wrapping_add(1) {
            let mut x = origin.x;
            while x != max_x.wrapping_add(1) {
                let height = sample_height(level, piece.is_beached(), x, z);
                mean = mean.wrapping_add(height);
                min_y = min_y.min(height);
                x = x.wrapping_add(1);
            }
            z = z.wrapping_add(1);
        }
        mean /= base_size;
    }
    if piece.is_beached() {
        ShipwreckPiece::calculate_beached_position(min_y, size[1], random.next_int(3))
    } else {
        mean
    }
}

fn sample_height<L: WorldGenLevel>(level: &L, beached: bool, x: i32, z: i32) -> i32 {
    if beached {
        level.get_world_surface_wg_height(x, z)
    } else {
        level.get_ocean_floor_wg_height(x, z)
    }
}

/// `TemplateStructurePiece.postProcess` (`TemplateStructurePiece.java:91-117`)
/// with `ShipwreckPieces.makeSettings` (`ShipwreckPieces.java:118-124`).
fn place_template<L: WorldGenLevel>(
    piece: &ShipwreckPiece,
    template: &MaterializedStructureTemplate,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> bool {
    let processors = structure_and_air_processors();
    let origin = piece.template_position();
    let Some(palette) = place_materialized_template_with_palette(
        template,
        level,
        MaterializedTemplatePlacement {
            position: origin,
            reference_pos: origin,
            rotation: piece.rotation(),
            mirror: RuinedPortalMirror::None,
            pivot: SHIPWRECK_PIVOT,
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
    handle_data_markers(piece, palette, chunk_bounds, level, random);
    replace_jigsaw_blocks(piece, palette, chunk_bounds, level);
    true
}

fn structure_and_air_processors() -> Vec<StructureProcessor> {
    vec![StructureProcessor::BlockIgnore {
        blocks: vec![
            catalog_default_state("minecraft:air"),
            catalog_default_state("minecraft:structure_block"),
        ],
    }]
}

/// `ShipwreckPieces.handleDataMarker` (`ShipwreckPieces.java:127-134`).
fn handle_data_markers<L: WorldGenLevel>(
    piece: &ShipwreckPiece,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
        chunk_bounds,
        "minecraft:structure_block",
    ) {
        let Some(nbt) = nbt.as_deref() else {
            continue;
        };
        if !compound_string(nbt, "mode").is_some_and(|mode| mode.eq_ignore_ascii_case("data")) {
            continue;
        }
        let marker = compound_string(nbt, "metadata").unwrap_or_default();
        let Some(loot_table) = loot_table_for_marker(&marker) else {
            continue;
        };
        let below = BlockPos {
            x: pos.x,
            y: pos.y.wrapping_sub(1),
            z: pos.z,
        };
        let seed = random.next_long();
        level.set_block_entity_loot_table(below, loot_table, seed);
    }
}

fn replace_jigsaw_blocks<L: WorldGenLevel>(
    piece: &ShipwreckPiece,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
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
        let _ = level.set_block(pos, state, UPDATE_JIGSAW_FINAL);
    }
}

/// `StructureTemplate.filterBlocks` (`StructureTemplate.java:220-236`).
fn filter_blocks(
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    origin: BlockPos,
    rotation: JigsawRotation,
    chunk_bounds: JigsawBoundingBox,
    block_name: &str,
) -> Vec<(BlockPos, Option<Arc<NbtSummaryValue>>)> {
    palette
        .blocks
        .iter()
        .filter(|block| block_is(block.state, block_name))
        .filter_map(|block| {
            let pos = wrapping_add(transform(block.position, rotation, SHIPWRECK_PIVOT), origin);
            chunk_bounds
                .contains(pos)
                .then_some((pos, block.nbt.clone()))
        })
        .collect()
}

fn loot_table_for_marker(marker: &str) -> Option<&'static str> {
    match marker {
        "map_chest" => Some(SHIPWRECK_MAP_LOOT),
        "treasure_chest" => Some(SHIPWRECK_TREASURE_LOOT),
        "supply_chest" => Some(SHIPWRECK_SUPPLY_LOOT),
        _ => None,
    }
}

/// `StructureTemplate.transform` with `Mirror.NONE` (`StructureTemplate.java:528-556`).
fn transform(pos: BlockPos, rotation: JigsawRotation, pivot: BlockPos) -> BlockPos {
    let BlockPos { x, y, z } = pos;
    let (pivot_x, pivot_z) = (pivot.x, pivot.z);
    match rotation {
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: pivot_x - pivot_z + z,
            y,
            z: pivot_x + pivot_z - x,
        },
        JigsawRotation::Clockwise90 => BlockPos {
            x: pivot_x + pivot_z - z,
            y,
            z: pivot_z - pivot_x + x,
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: pivot_x + pivot_x - x,
            y,
            z: pivot_z + pivot_z - z,
        },
        JigsawRotation::None => pos,
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
    use std::sync::Arc;

    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::random::WorldgenRandom;
    use bbb_server_datapack::structure_template::{
        StructureTemplate, StructureTemplateBlock, StructureTemplateElement,
        StructureTemplateEntity, StructureTemplatePalette, StructureTemplateSet,
    };

    use super::*;

    struct PlacementLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        air: i32,
        max_y: i32,
        surface: BTreeMap<(i32, i32), i32>,
        ocean: BTreeMap<(i32, i32), i32>,
        writes: Vec<(BlockPos, i32, i32)>,
        loot: Vec<(BlockPos, &'static str, i64)>,
        loot_available: bool,
    }

    impl PlacementLevel {
        fn new() -> Self {
            Self {
                cells: BTreeMap::new(),
                air: catalog_default_state("minecraft:air"),
                max_y: 319,
                surface: BTreeMap::new(),
                ocean: BTreeMap::new(),
                writes: Vec::new(),
                loot: Vec::new(),
                loot_available: false,
            }
        }

        fn set_surface(&mut self, x: i32, z: i32, height: i32) {
            self.surface.insert((x, z), height);
        }

        fn set_ocean(&mut self, x: i32, z: i32, height: i32) {
            self.ocean.insert((x, z), height);
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

        fn get_world_surface_wg_height(&self, x: i32, z: i32) -> i32 {
            self.surface.get(&(x, z)).copied().unwrap_or(self.max_y)
        }

        fn get_ocean_floor_wg_height(&self, x: i32, z: i32) -> i32 {
            self.ocean.get(&(x, z)).copied().unwrap_or(self.max_y)
        }

        fn set_block_entity_loot_table(
            &mut self,
            pos: BlockPos,
            loot_table: &'static str,
            seed: i64,
        ) {
            self.loot.push((pos, loot_table, seed));
        }
    }

    fn at(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn state(name: &'static str) -> i32 {
        catalog_default_state(name)
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
                template: StructureTemplate::from_materialized(MaterializedStructureTemplate {
                    size,
                    palettes: vec![palette(blocks)],
                    entities,
                })
                .unwrap(),
            }],
            warnings: Vec::new(),
        }
    }

    fn start(template: &str, origin: BlockPos, beached: bool) -> ShipwreckStructureStart {
        start_with_flags(template, origin, beached, false, JigsawRotation::None)
    }

    fn start_with_flags(
        template: &str,
        origin: BlockPos,
        beached: bool,
        height_adjusted: bool,
        rotation: JigsawRotation,
    ) -> ShipwreckStructureStart {
        ShipwreckStructureStart::from_generation_stub(
            super::super::ShipwreckGenerationStub::new(
                [0, 0],
                origin,
                if beached {
                    super::super::SHIPWRECK_BEACHED_ID
                } else {
                    super::super::SHIPWRECK_ID
                },
                ShipwreckPiece::from_parts(template, origin, rotation, beached, height_adjusted),
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

    fn place(
        start: &mut ShipwreckStructureStart,
        templates: &StructureTemplateSet,
        level: &mut PlacementLevel,
        random: &mut dyn RandomSource,
    ) -> ShipwreckChunkPlacement {
        start.place_in_chunk(templates, level, wide_bounds(), random)
    }

    #[test]
    fn ocean_height_uses_mean_ocean_floor_wg() {
        let id = "minecraft:shipwreck/rightsideup_full";
        let templates = templates(
            id,
            [2, 4, 2],
            vec![block(0, 0, 0, state("minecraft:oak_planks"))],
        );
        let origin = at(10, 90, 20);
        let mut level = PlacementLevel::new();
        level.set_ocean(10, 20, 5);
        level.set_ocean(11, 20, 7);
        level.set_ocean(10, 21, 9);
        level.set_ocean(11, 21, 11);
        let mut start = start(id, origin, false);
        let report = place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        assert_eq!(start.piece().template_position().y, 8);
        assert!(start.piece().height_adjusted());
    }

    #[test]
    fn beached_height_uses_min_surface_minus_half_size_and_random() {
        let id = "minecraft:shipwreck/with_mast";
        let templates = templates(
            id,
            [2, 6, 2],
            vec![block(0, 0, 0, state("minecraft:oak_planks"))],
        );
        let origin = at(4, 90, 8);
        let mut level = PlacementLevel::new();
        level.set_surface(4, 8, 12);
        level.set_surface(5, 8, 20);
        level.set_surface(4, 9, 18);
        level.set_surface(5, 9, 16);
        let mut expected = WorldgenRandom::with_seed(17);
        let extra = expected.next_int(3);
        let mut start = start(id, origin, true);
        place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(17),
        );
        assert_eq!(
            start.piece().template_position().y,
            ShipwreckPiece::calculate_beached_position(12, 6, extra)
        );
        assert!(start.piece().height_adjusted());
    }

    #[test]
    fn already_adjusted_skips_height_resample() {
        let id = "minecraft:shipwreck/sideways_full";
        let templates = templates(
            id,
            [2, 2, 2],
            vec![block(0, 0, 0, state("minecraft:oak_planks"))],
        );
        let origin = at(3, 44, 5);
        let mut level = PlacementLevel::new();
        level.set_ocean(3, 5, 1);
        level.set_ocean(4, 5, 1);
        level.set_ocean(3, 6, 1);
        level.set_ocean(4, 6, 1);
        let mut start = start_with_flags(id, origin, false, true, JigsawRotation::None);
        place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(start.piece().template_position().y, 44);
        assert!(start.piece().height_adjusted());
    }

    #[test]
    fn too_big_template_skips_height_adjust() {
        let id = "minecraft:shipwreck/too_big";
        let templates = templates(
            id,
            [33, 2, 2],
            vec![block(0, 0, 0, state("minecraft:oak_planks"))],
        );
        let origin = at(0, 90, 0);
        let mut level = PlacementLevel::new();
        level.set_ocean(0, 0, 4);
        let mut start = start(id, origin, false);
        place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(start.piece().template_position().y, 90);
        assert!(!start.piece().height_adjusted());
    }

    #[test]
    fn marker_loot_maps_onto_the_block_below() {
        assert_eq!(loot_table_for_marker("map_chest"), Some(SHIPWRECK_MAP_LOOT));
        assert_eq!(
            loot_table_for_marker("treasure_chest"),
            Some(SHIPWRECK_TREASURE_LOOT)
        );
        assert_eq!(
            loot_table_for_marker("supply_chest"),
            Some(SHIPWRECK_SUPPLY_LOOT)
        );
        assert_eq!(loot_table_for_marker("unused"), None);

        let id = "minecraft:shipwreck/markers";
        let oak = state("minecraft:oak_planks");
        let structure = state("minecraft:structure_block");
        let marker = |x: i32, metadata: &str| StructureTemplateBlock {
            position: at(x, 1, 0),
            state: structure,
            nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "mode".into(),
                    value: NbtSummaryValue::string("DATA"),
                },
                NbtSummaryEntry {
                    name: "metadata".into(),
                    value: NbtSummaryValue::string(metadata),
                },
            ]))),
        };
        let templates = templates(
            id,
            [4, 2, 1],
            vec![
                block(0, 0, 0, oak),
                marker(0, "map_chest"),
                marker(1, "treasure_chest"),
                marker(2, "supply_chest"),
                marker(3, "other"),
            ],
        );
        let origin = at(5, 10, 7);
        let mut start = start_with_flags(id, origin, false, true, JigsawRotation::None);
        let mut level = PlacementLevel::new();
        let mut random = WorldgenRandom::with_seed(9);
        place(&mut start, &templates, &mut level, &mut random);
        let mut expected = WorldgenRandom::with_seed(9);
        assert_eq!(
            level.loot,
            vec![
                (at(5, 10, 7), SHIPWRECK_MAP_LOOT, expected.next_long()),
                (at(6, 10, 7), SHIPWRECK_TREASURE_LOOT, expected.next_long()),
                (at(7, 10, 7), SHIPWRECK_SUPPLY_LOOT, expected.next_long()),
            ]
        );
        assert!(!level
            .writes
            .iter()
            .any(|(_, state, _)| block_is(*state, "minecraft:structure_block")));
    }

    #[test]
    fn builtin_shipwreck_template_writes_inside_the_chunk_bb() {
        use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
        use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

        let Ok(startup) = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        }) else {
            return;
        };
        let Some(template) = startup
            .structures
            .templates
            .iter()
            .find(|template| template.id.starts_with("minecraft:shipwreck/"))
        else {
            return;
        };
        if !template.template.materialized().entities.is_empty() {
            return;
        }
        let origin = at(0, 64, 0);
        let mut start = start_with_flags(&template.id, origin, false, true, JigsawRotation::None);
        let mut level = PlacementLevel::new();
        let report = start.place_in_chunk(
            &startup.structures,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted);
        assert!(report.placed);
        assert!(
            level.writes.iter().any(|(_, block, flags)| {
                *flags == UPDATE_CLIENTS && !block_is(*block, "minecraft:air")
            }),
            "{}",
            template.id
        );
    }
}
